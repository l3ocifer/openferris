use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use ferris_common::{HeartbeatRequest, RegisterRequest};
use ferris_inference::{ChatCompletionRequest, ChatCompletionResponse};
use serde::Serialize;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;

use crate::auth::verify_agent_signature;
use crate::registry::AgentRegistry;
use crate::router::InferenceRouter;
use crate::storage_router::StorageRouter;

// ── State ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<AgentRegistry>,
    pub router: Arc<InferenceRouter>,
    pub storage_router: Arc<StorageRouter>,
}

// ── Response Types ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct CoordinatorStatus {
    status: &'static str,
    active_agents: i64,
    available_models: usize,
}

// ── Router ──────────────────────────────────────────────────────────────

pub fn build_coordinator_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/register", post(register))
        .route("/api/v1/heartbeat", post(heartbeat))
        .route("/api/v1/status", get(coordinator_status))
        .route("/api/v1/wallet/balance", get(wallet_balance))
        .route("/api/v1/wallet/history", get(wallet_history))
        .route("/api/v1/directory", get(directory))
        .route("/dashboard/stats", get(dashboard_stats))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/api/v1/network/store", post(network_store))
        .route("/api/v1/network/files", get(network_list_files))
        .route("/api/v1/network/files/{object_id}", get(network_retrieve))
        .layer(ConcurrencyLimitLayer::new(256))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB
        .with_state(state)
}

pub async fn run_coordinator(state: AppState, host: &str, port: u16) -> ferris_common::Result<()> {
    let app = build_coordinator_app(state);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| ferris_common::FerrisError::Config(format!("invalid address: {e}")))?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("coordinator listening on http://{addr}");

    axum::serve(listener, app)
        .await
        .map_err(|e| ferris_common::FerrisError::Config(format!("server error: {e}")))?;

    Ok(())
}

// ── Handlers ────────────────────────────────────────────────────────────

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "service": "openferris-coordinator"}))
}

async fn register(
    State(s): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    match s.registry.register(&req).await {
        Ok(resp) => match serde_json::to_value(&resp) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn heartbeat(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let agent_id = match headers.get("X-Agent-Id").and_then(|v| v.to_str().ok()) {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "X-Agent-Id header required").into_response(),
    };
    let signature = match headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return (StatusCode::BAD_REQUEST, "X-Signature header required").into_response(),
    };

    if let Err(e) = verify_agent_signature(s.registry.pool(), agent_id, signature, &body).await {
        return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
    }

    let req: HeartbeatRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    match s.registry.heartbeat(&req).await {
        Ok(resp) => match serde_json::to_value(&resp) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(ferris_common::FerrisError::NotFound(msg)) => {
            (StatusCode::NOT_FOUND, msg).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn coordinator_status(State(s): State<AppState>) -> impl IntoResponse {
    let active_agents = s.registry.active_agent_count().await.unwrap_or(0);
    let models = s.router.list_models().await.unwrap_or_default();

    Json(CoordinatorStatus {
        status: "ok",
        active_agents,
        available_models: models.len(),
    })
}

async fn wallet_balance(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<AgentQuery>,
) -> impl IntoResponse {
    let agent_id = match headers
        .get("X-Agent-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .or(params.agent_id)
    {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "agent_id required").into_response(),
    };

    if let Some(sig) = headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
        let body = headers
            .get("X-Timestamp")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if let Err(e) =
            verify_agent_signature(s.registry.pool(), &agent_id, sig, body.as_bytes()).await
        {
            return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
        }
    }

    match s.registry.ledger().get_balance(&agent_id).await {
        Ok(balance) => match serde_json::to_value(&balance) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(ferris_common::FerrisError::NotFound(msg)) => {
            (StatusCode::NOT_FOUND, msg).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn wallet_history(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<HistoryQuery>,
) -> impl IntoResponse {
    let agent_id = match headers
        .get("X-Agent-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .or(params.agent_id)
    {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "agent_id required").into_response(),
    };

    if let Some(sig) = headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
        let body = headers
            .get("X-Timestamp")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if let Err(e) =
            verify_agent_signature(s.registry.pool(), &agent_id, sig, body.as_bytes()).await
        {
            return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
        }
    }

    let limit = params.limit.unwrap_or(20);

    match s.registry.ledger().get_history(&agent_id, limit).await {
        Ok(history) => match serde_json::to_value(&history) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_models(State(s): State<AppState>) -> impl IntoResponse {
    match s.router.list_models().await {
        Ok(models) => {
            let response = serde_json::json!({
                "object": "list",
                "data": models,
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn chat_completions(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let consumer_id = headers
        .get("X-Agent-Id")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    if let Some(agent_id) = &consumer_id {
        if let Some(sig) = headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
            if let Err(e) =
                verify_agent_signature(s.registry.pool(), agent_id, sig, &body).await
            {
                return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
            }
        }
    }

    let req: ChatCompletionRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    // Step 1: Get ranked candidates (up to 3 for fallback)
    let candidates = match s.router.score_candidates(&req.model, None).await {
        Ok(c) if c.is_empty() => {
            return (
                StatusCode::NOT_FOUND,
                format!("no active provider for model: {}", req.model),
            )
                .into_response();
        }
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let max_attempts = candidates.len().min(3);
    let client = reqwest::Client::new();

    // Step 2: Try each candidate in ranked order
    for (attempt, candidate) in candidates.into_iter().take(max_attempts).enumerate() {
        let proxy_result = client
            .post(format!("{}/v1/chat/completions", candidate.endpoint_url))
            .json(&req)
            .send()
            .await;

        let resp = match proxy_result {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                tracing::warn!(
                    provider = %candidate.agent_id,
                    attempt = attempt + 1,
                    "provider error ({status}): {text}, trying next candidate"
                );
                if let Err(e) = s.registry.adjust_reputation(&candidate.agent_id, -1.0).await {
                    tracing::warn!(error = %e, "reputation penalty failed");
                }
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    provider = %candidate.agent_id,
                    attempt = attempt + 1,
                    "provider unreachable: {e}, trying next candidate"
                );
                if let Err(e) = s.registry.adjust_reputation(&candidate.agent_id, -1.0).await {
                    tracing::warn!(error = %e, "reputation penalty failed");
                }
                continue;
            }
        };

        let completion = match resp.json::<ChatCompletionResponse>().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    provider = %candidate.agent_id,
                    attempt = attempt + 1,
                    "provider response parse error: {e}, trying next candidate"
                );
                if let Err(e) = s.registry.adjust_reputation(&candidate.agent_id, -1.0).await {
                    tracing::warn!(error = %e, "reputation penalty failed");
                }
                continue;
            }
        };

        // Success — settle credits and return
        let job_id = uuid::Uuid::now_v7().to_string();

        if let Some(consumer) = &consumer_id {
            let total_tokens =
                completion.usage.prompt_tokens + completion.usage.completion_tokens;
            let amount_mc = total_tokens as i64;

            match s
                .registry
                .ledger()
                .settle_inference(
                    consumer,
                    &candidate.agent_id,
                    amount_mc,
                    &req.model,
                    completion.usage.prompt_tokens,
                    completion.usage.completion_tokens,
                    &job_id,
                )
                .await
            {
                Ok(tx) => {
                    if let Err(e) = s
                        .registry
                        .adjust_reputation(&candidate.agent_id, 0.1)
                        .await
                    {
                        tracing::warn!(error = %e, "reputation adjustment failed");
                    }

                    tracing::info!(
                        job_id,
                        tx_id = %tx.tx_id,
                        model = %req.model,
                        consumer = %consumer,
                        provider = %candidate.agent_id,
                        amount_mc,
                        fee_mc = tx.platform_fee_mc,
                        attempt = attempt + 1,
                        "inference settled"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        job_id,
                        error = %e,
                        "settlement failed (inference still served)"
                    );
                }
            }
        } else {
            tracing::info!(
                job_id,
                model = %req.model,
                provider = %candidate.agent_id,
                tokens_in = completion.usage.prompt_tokens,
                tokens_out = completion.usage.completion_tokens,
                attempt = attempt + 1,
                "inference routed (anonymous, no settlement)"
            );
        }

        return match serde_json::to_value(&completion) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        };
    }

    (
        StatusCode::BAD_GATEWAY,
        format!(
            "all {} providers failed for model: {}",
            max_attempts, req.model
        ),
    )
        .into_response()
}

async fn directory(State(s): State<AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, DirectoryEntry>(
        "SELECT a.agent_id, a.status, a.reputation, a.tier, a.region,
                a.gpu_model, a.cpu_cores, a.ram_mb,
                (SELECT GROUP_CONCAT(m.model_name, ', ')
                 FROM models m WHERE m.agent_id = a.agent_id) as models
         FROM agents a
         WHERE a.status = 'active'
         ORDER BY a.reputation DESC",
    )
    .fetch_all(s.registry.pool())
    .await;

    match rows {
        Ok(entries) => match serde_json::to_value(&entries) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn dashboard_stats(State(s): State<AppState>) -> impl IntoResponse {
    let active_agents: i64 = s.registry.active_agent_count().await.unwrap_or(0);
    let models = s.router.list_models().await.unwrap_or_default();

    let total_transactions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
            .fetch_one(s.registry.ledger().pool())
            .await
            .unwrap_or(0);

    let total_volume_mc: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(amount_mc), 0) FROM transactions")
            .fetch_one(s.registry.ledger().pool())
            .await
            .unwrap_or(0);

    let total_fees_mc: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(platform_fee_mc), 0) FROM transactions")
            .fetch_one(s.registry.ledger().pool())
            .await
            .unwrap_or(0);

    let active_escrows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM escrow WHERE status = 'held'")
            .fetch_one(s.registry.ledger().pool())
            .await
            .unwrap_or(0);

    let stats = serde_json::json!({
        "active_agents": active_agents,
        "available_models": models.len(),
        "total_transactions": total_transactions,
        "total_volume_credits": total_volume_mc as f64 / 1000.0,
        "total_fees_credits": total_fees_mc as f64 / 1000.0,
        "active_escrows": active_escrows,
    });

    (StatusCode::OK, Json(stats)).into_response()
}

// ── Query params ────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct AgentQuery {
    agent_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct HistoryQuery {
    agent_id: Option<String>,
    limit: Option<usize>,
}

#[derive(sqlx::FromRow, Serialize)]
struct DirectoryEntry {
    agent_id: String,
    status: String,
    reputation: f64,
    tier: String,
    region: Option<String>,
    gpu_model: Option<String>,
    cpu_cores: i64,
    ram_mb: i64,
    models: Option<String>,
}

// ── Network Storage Types ────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct NetworkStoreRequest {
    name: String,
    data_base64: String,
}

// ── Network Storage Handlers ─────────────────────────────────────────────

async fn network_store(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let agent_id = match headers.get("X-Agent-Id").and_then(|v| v.to_str().ok()) {
        Some(id) => id.to_string(),
        None => return (StatusCode::BAD_REQUEST, "X-Agent-Id header required").into_response(),
    };
    let signature = match headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s.to_string(),
        None => return (StatusCode::BAD_REQUEST, "X-Signature header required").into_response(),
    };

    if let Err(e) =
        verify_agent_signature(s.registry.pool(), &agent_id, &signature, &body).await
    {
        return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
    }

    let req: NetworkStoreRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    let data = match STANDARD.decode(&req.data_base64) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("invalid base64 data: {e}"),
            )
                .into_response()
        }
    };

    let size_bytes = data.len() as i64;

    // Find a storage node
    let candidate = match s.storage_router.find_storage_node(&agent_id).await {
        Ok(c) => c,
        Err(ferris_common::FerrisError::NotFound(msg)) => {
            return (StatusCode::NOT_FOUND, msg).into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Proxy the store request to the storage node
    let client = reqwest::Client::new();
    let proxy_body = serde_json::json!({
        "name": req.name,
        "data_base64": req.data_base64,
    });

    let proxy_result = client
        .post(format!(
            "{}/api/v1/storage/store",
            candidate.endpoint_url
        ))
        .json(&proxy_body)
        .send()
        .await;

    match proxy_result {
        Ok(resp) if resp.status().is_success() => {
            let content_hash = blake3::hash(&data).to_hex().to_string();

            let object_id = match s
                .storage_router
                .record_object(
                    &agent_id,
                    &candidate.agent_id,
                    &req.name,
                    size_bytes,
                    &content_hash,
                )
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            };

            // 1 millicredit per KB (minimum 1mc)
            let size_kb = (size_bytes + 1023) / 1024;
            let amount_mc = size_kb.max(1);

            match s
                .registry
                .ledger()
                .settle_storage(
                    &agent_id,
                    &candidate.agent_id,
                    amount_mc,
                    &object_id,
                    size_bytes,
                )
                .await
            {
                Ok(tx) => {
                    if let Err(e) =
                        s.registry.adjust_reputation(&candidate.agent_id, 0.1).await
                    {
                        tracing::warn!(error = %e, "reputation adjustment failed");
                    }

                    tracing::info!(
                        object_id,
                        tx_id = %tx.tx_id,
                        owner = %agent_id,
                        storage_node = %candidate.agent_id,
                        size_bytes,
                        amount_mc,
                        fee_mc = tx.platform_fee_mc,
                        "network storage settled"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        object_id,
                        error = %e,
                        "storage settlement failed (file still stored)"
                    );
                }
            }

            let response = serde_json::json!({
                "object_id": object_id,
                "storage_node": candidate.agent_id,
                "name": req.name,
                "size_bytes": size_bytes,
                "content_hash": content_hash,
            });

            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (
                StatusCode::BAD_GATEWAY,
                format!("storage node error ({status}): {text}"),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("storage node unreachable: {e}"),
        )
            .into_response(),
    }
}

async fn network_list_files(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let agent_id = match headers.get("X-Agent-Id").and_then(|v| v.to_str().ok()) {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "X-Agent-Id header required").into_response(),
    };

    match s.storage_router.list_objects(agent_id).await {
        Ok(objects) => match serde_json::to_value(&objects) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn network_retrieve(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(object_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let agent_id = match headers.get("X-Agent-Id").and_then(|v| v.to_str().ok()) {
        Some(id) => id.to_string(),
        None => return (StatusCode::BAD_REQUEST, "X-Agent-Id header required").into_response(),
    };

    let obj = match s.storage_router.find_object(&object_id, &agent_id).await {
        Ok(o) => o,
        Err(ferris_common::FerrisError::NotFound(msg)) => {
            return (StatusCode::NOT_FOUND, msg).into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Look up the storage node's endpoint
    let endpoint_url: Option<String> =
        match sqlx::query_scalar("SELECT endpoint_url FROM agents WHERE agent_id = ?")
            .bind(&obj.storage_agent)
            .fetch_optional(s.registry.pool())
            .await
        {
            Ok(url) => url,
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        };

    let endpoint_url = match endpoint_url {
        Some(url) => url,
        None => {
            return (
                StatusCode::BAD_GATEWAY,
                "storage node endpoint not found",
            )
                .into_response();
        }
    };

    // Proxy retrieval to the storage node
    let client = reqwest::Client::new();
    let proxy_result = client
        .get(format!(
            "{}/api/v1/storage/{}",
            endpoint_url, object_id
        ))
        .send()
        .await;

    match proxy_result {
        Ok(resp) if resp.status().is_success() => {
            let body = match resp.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    return (
                        StatusCode::BAD_GATEWAY,
                        format!("failed to read storage node response: {e}"),
                    )
                        .into_response();
                }
            };

            (StatusCode::OK, body).into_response()
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (
                StatusCode::BAD_GATEWAY,
                format!("storage node error ({status}): {text}"),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("storage node unreachable: {e}"),
        )
            .into_response(),
    }
}
