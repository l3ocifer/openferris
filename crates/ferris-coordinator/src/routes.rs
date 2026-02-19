use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use ferris_common::{
    AgentMessage, HeartbeatRequest, RegisterRequest, SendMessageRequest, SettlementRequest,
};
use ferris_inference::{ChatCompletionRequest, ChatCompletionResponse};
use serde::Serialize;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;

use crate::auth::verify_agent_signature;
use crate::registry::AgentRegistry;
use crate::router::InferenceRouter;
use crate::storage_router::StorageRouter;

// ── State ───────────────────────────────────────────────────────────────

/// Shared application state for the coordinator HTTP server.
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

/// Build the Axum router with all coordinator API routes and middleware.
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
        .route("/api/v1/settle", post(settle))
        .route("/v1/embeddings", post(embeddings))
        .route("/api/v1/messages", get(poll_messages))
        .route("/api/v1/messages/send", post(send_message))
        .layer(ConcurrencyLimitLayer::new(256))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB
        .with_state(state)
}

/// Start the coordinator HTTP server on the given host and port.
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

    Json(CoordinatorStatus { status: "ok", active_agents, available_models: models.len() })
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
        let body = headers.get("X-Timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");
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
        let body = headers.get("X-Timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");
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
    let consumer_id = headers.get("X-Agent-Id").and_then(|v| v.to_str().ok()).map(String::from);

    if let Some(agent_id) = &consumer_id {
        if let Some(sig) = headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
            if let Err(e) = verify_agent_signature(s.registry.pool(), agent_id, sig, &body).await {
                return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
            }
        }
    }

    let req: ChatCompletionRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let is_streaming = req.stream;

    let candidates = match s.router.score_candidates(&req.model, None).await {
        Ok(c) if c.is_empty() => {
            return (StatusCode::NOT_FOUND, format!("no active provider for model: {}", req.model))
                .into_response();
        }
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let max_attempts = candidates.len().min(3);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_default();

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

        if is_streaming {
            return stream_sse_passthrough(s, resp, candidate, consumer_id, req.model.clone())
                .await;
        }

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

        let job_id = uuid::Uuid::now_v7().to_string();
        settle_inference_credits(
            &s,
            &consumer_id,
            &candidate.agent_id,
            &req.model,
            &completion,
            &job_id,
            attempt,
        )
        .await;

        return match serde_json::to_value(&completion) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        };
    }

    (
        StatusCode::BAD_GATEWAY,
        format!("all {} providers failed for model: {}", max_attempts, req.model),
    )
        .into_response()
}

/// Proxy an SSE stream from the upstream provider to the client, capturing
/// usage data from the final chunk for credit settlement.
async fn stream_sse_passthrough(
    s: AppState,
    resp: reqwest::Response,
    candidate: crate::router::RouteCandidate,
    consumer_id: Option<String>,
    model: String,
) -> axum::response::Response {
    let provider_id = candidate.agent_id.clone();
    let upstream = resp.bytes_stream();

    let s_clone = s.clone();
    let consumer_clone = consumer_id.clone();
    let model_clone = model.clone();
    let provider_clone = provider_id.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(32);

    let forward_handle = tokio::spawn(async move {
        use futures::StreamExt;
        let mut stream = Box::pin(upstream);
        let mut final_buf = String::new();

        while let Some(item) = stream.next().await {
            let to_send: Result<axum::body::Bytes, std::io::Error> = match item {
                Ok(bytes) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        final_buf.push_str(text);
                        if final_buf.len() > 4096 {
                            let trim_at = final_buf.len() - 2048;
                            final_buf = final_buf[trim_at..].to_string();
                        }
                    }
                    Ok(bytes)
                }
                Err(e) => Err(std::io::Error::other(e)),
            };
            if tx.send(to_send).await.is_err() {
                break;
            }
        }

        // Stream complete — extract usage from the tail and settle
        if let Err(e) = s_clone.registry.adjust_reputation(&provider_clone, 0.1).await {
            tracing::warn!(error = %e, "reputation adjustment failed");
        }

        let usage = extract_stream_usage(&final_buf);
        if let (Some(consumer), Some((prompt_tokens, completion_tokens))) = (&consumer_clone, usage)
        {
            let job_id = uuid::Uuid::now_v7().to_string();
            let total_tokens = prompt_tokens + completion_tokens;
            let amount_mc = total_tokens as i64;
            if let Err(e) = s_clone
                .registry
                .ledger()
                .settle_inference(
                    consumer,
                    &provider_clone,
                    amount_mc,
                    &model_clone,
                    prompt_tokens,
                    completion_tokens,
                    &job_id,
                )
                .await
            {
                tracing::warn!(job_id, error = %e, "streaming settlement failed");
            } else {
                tracing::info!(
                    job_id,
                    model = %model_clone,
                    consumer = %consumer,
                    provider = %provider_clone,
                    amount_mc,
                    "streaming inference settled"
                );
            }
        } else {
            tracing::info!(
                model = %model_clone,
                provider = %provider_clone,
                "streaming inference completed (no settlement)"
            );
        }
    });

    // Don't block on the settlement task — it runs in the background
    drop(forward_handle);

    let body_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    let body = axum::body::Body::from_stream(body_stream);

    match axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
    {
        Ok(resp) => resp.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Extract usage data from the tail of an SSE stream.
/// Looks for `"usage":{"prompt_tokens":N,"completion_tokens":N,...}` in the final chunks.
fn extract_stream_usage(tail: &str) -> Option<(u32, u32)> {
    for line in tail.lines().rev() {
        let data = line.strip_prefix("data: ")?;
        if data == "[DONE]" {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(usage) = v.get("usage") {
                let prompt = usage.get("prompt_tokens")?.as_u64()? as u32;
                let completion = usage.get("completion_tokens")?.as_u64()? as u32;
                return Some((prompt, completion));
            }
        }
    }
    None
}

async fn settle_inference_credits(
    s: &AppState,
    consumer_id: &Option<String>,
    provider_id: &str,
    model: &str,
    completion: &ChatCompletionResponse,
    job_id: &str,
    attempt: usize,
) {
    if let Some(consumer) = consumer_id {
        let total_tokens = completion.usage.prompt_tokens + completion.usage.completion_tokens;
        let amount_mc = total_tokens as i64;

        match s
            .registry
            .ledger()
            .settle_inference(
                consumer,
                provider_id,
                amount_mc,
                model,
                completion.usage.prompt_tokens,
                completion.usage.completion_tokens,
                job_id,
            )
            .await
        {
            Ok(tx) => {
                if let Err(e) = s.registry.adjust_reputation(provider_id, 0.1).await {
                    tracing::warn!(error = %e, "reputation adjustment failed");
                }

                tracing::info!(
                    job_id,
                    tx_id = %tx.tx_id,
                    model = %model,
                    consumer = %consumer,
                    provider = %provider_id,
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
            model = %model,
            provider = %provider_id,
            tokens_in = completion.usage.prompt_tokens,
            tokens_out = completion.usage.completion_tokens,
            attempt = attempt + 1,
            "inference routed (anonymous, no settlement)"
        );
    }
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

    let total_transactions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions")
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

// ── Settlement ──────────────────────────────────────────────────────────

async fn settle(
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

    if let Err(e) = verify_agent_signature(s.registry.pool(), &agent_id, &signature, &body).await {
        return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
    }

    let req: SettlementRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let total_tokens = req.tokens_in + req.tokens_out;
    let amount_mc = total_tokens as i64;

    match s
        .registry
        .ledger()
        .settle_inference(
            &req.consumer_agent,
            &agent_id,
            amount_mc,
            &req.model_name,
            req.tokens_in,
            req.tokens_out,
            &req.job_id,
        )
        .await
    {
        Ok(tx) => {
            if let Err(e) = s.registry.adjust_reputation(&agent_id, 0.1).await {
                tracing::warn!(error = %e, "reputation adjustment failed");
            }

            tracing::info!(
                job_id = %req.job_id,
                tx_id = %tx.tx_id,
                model = %req.model_name,
                consumer = %req.consumer_agent,
                provider = %agent_id,
                amount_mc,
                fee_mc = tx.platform_fee_mc,
                "node-reported settlement"
            );

            let response = serde_json::json!({
                "settled": true,
                "tx_id": tx.tx_id,
                "amount_mc": amount_mc,
                "platform_fee_mc": tx.platform_fee_mc,
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(ferris_common::FerrisError::InsufficientCredits(msg)) => {
            (StatusCode::PAYMENT_REQUIRED, msg).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Embeddings ──────────────────────────────────────────────────────────

async fn embeddings(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let consumer_id = headers.get("X-Agent-Id").and_then(|v| v.to_str().ok()).map(String::from);

    if let Some(agent_id) = &consumer_id {
        if let Some(sig) = headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
            if let Err(e) = verify_agent_signature(s.registry.pool(), agent_id, sig, &body).await {
                return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
            }
        }
    }

    let req: ferris_common::EmbeddingRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let candidates = match s.router.score_candidates(&req.model, None).await {
        Ok(c) if c.is_empty() => {
            return (StatusCode::NOT_FOUND, format!("no provider for model: {}", req.model))
                .into_response();
        }
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default();

    let max_attempts = candidates.len().min(3);
    for (attempt, candidate) in candidates.into_iter().take(max_attempts).enumerate() {
        let proxy_result = client
            .post(format!("{}/v1/embeddings", candidate.endpoint_url))
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
                    "embedding provider error ({status}): {text}"
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
                    "embedding provider unreachable: {e}"
                );
                if let Err(e) = s.registry.adjust_reputation(&candidate.agent_id, -1.0).await {
                    tracing::warn!(error = %e, "reputation penalty failed");
                }
                continue;
            }
        };

        let response_body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    provider = %candidate.agent_id,
                    attempt = attempt + 1,
                    "embedding parse error: {e}"
                );
                continue;
            }
        };

        if let Some(consumer) = &consumer_id {
            let input_count = match &req.input {
                ferris_common::EmbeddingInput::Single(_) => 1u32,
                ferris_common::EmbeddingInput::Batch(v) => v.len() as u32,
            };
            let amount_mc = input_count as i64;
            let job_id = uuid::Uuid::now_v7().to_string();
            if let Err(e) = s
                .registry
                .ledger()
                .settle_inference(
                    consumer,
                    &candidate.agent_id,
                    amount_mc,
                    &req.model,
                    input_count,
                    0,
                    &job_id,
                )
                .await
            {
                tracing::warn!(error = %e, "embedding settlement failed");
            }
        }

        return (StatusCode::OK, Json(response_body)).into_response();
    }

    (
        StatusCode::BAD_GATEWAY,
        format!("all {} providers failed for embedding model: {}", max_attempts, req.model),
    )
        .into_response()
}

// ── Agent Messaging ─────────────────────────────────────────────────────

async fn send_message(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let from_agent = match headers.get("X-Agent-Id").and_then(|v| v.to_str().ok()) {
        Some(id) => id.to_string(),
        None => return (StatusCode::BAD_REQUEST, "X-Agent-Id header required").into_response(),
    };
    let signature = match headers.get("X-Signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s.to_string(),
        None => return (StatusCode::BAD_REQUEST, "X-Signature header required").into_response(),
    };

    if let Err(e) = verify_agent_signature(s.registry.pool(), &from_agent, &signature, &body).await
    {
        return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
    }

    let req: SendMessageRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    // Verify recipient exists
    let exists: Option<String> =
        match sqlx::query_scalar("SELECT agent_id FROM agents WHERE agent_id = ?")
            .bind(&req.to_agent)
            .fetch_optional(s.registry.pool())
            .await
        {
            Ok(r) => r,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        };

    if exists.is_none() {
        return (StatusCode::NOT_FOUND, format!("recipient not found: {}", req.to_agent))
            .into_response();
    }

    let now = ferris_common::unix_timestamp();
    let message_id = uuid::Uuid::now_v7().to_string();
    let expires_at = now + 86400; // 24 hours
    let payload_str = req.payload.to_string();

    let result = sqlx::query(
        "INSERT INTO message_queue (message_id, from_agent, to_agent, payload, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&message_id)
    .bind(&from_agent)
    .bind(&req.to_agent)
    .bind(&payload_str)
    .bind(now)
    .bind(expires_at)
    .execute(s.registry.pool())
    .await;

    match result {
        Ok(_) => {
            tracing::info!(
                message_id,
                from = %from_agent,
                to = %req.to_agent,
                "message queued"
            );
            let response = serde_json::json!({
                "message_id": message_id,
                "expires_at": expires_at,
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn poll_messages(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<MessageQuery>,
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
        let body = headers.get("X-Timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");
        if let Err(e) =
            verify_agent_signature(s.registry.pool(), &agent_id, sig, body.as_bytes()).await
        {
            return (StatusCode::UNAUTHORIZED, e.to_string()).into_response();
        }
    }

    let now = ferris_common::unix_timestamp();

    // Clean expired messages
    let _ = sqlx::query("DELETE FROM message_queue WHERE expires_at < ?")
        .bind(now)
        .execute(s.registry.pool())
        .await;

    let limit = params.limit.unwrap_or(50);

    let rows: Vec<MessageRow> = match sqlx::query_as(
        "SELECT message_id, from_agent, to_agent, payload, created_at, expires_at, delivered_at
         FROM message_queue
         WHERE to_agent = ? AND delivered_at IS NULL AND expires_at >= ?
         ORDER BY created_at ASC
         LIMIT ?",
    )
    .bind(&agent_id)
    .bind(now)
    .bind(limit as i64)
    .fetch_all(s.registry.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // Mark as delivered
    let message_ids: Vec<String> = rows.iter().map(|r| r.message_id.clone()).collect();
    if !message_ids.is_empty() {
        let placeholders: Vec<&str> = message_ids.iter().map(|_| "?").collect();
        let query = format!(
            "UPDATE message_queue SET delivered_at = ? WHERE message_id IN ({})",
            placeholders.join(", ")
        );
        let mut q = sqlx::query(&query).bind(now);
        for id in &message_ids {
            q = q.bind(id);
        }
        let _ = q.execute(s.registry.pool()).await;
    }

    let messages: Vec<AgentMessage> = rows
        .into_iter()
        .map(|r| AgentMessage {
            message_id: r.message_id,
            from_agent: r.from_agent,
            to_agent: r.to_agent,
            payload: serde_json::from_str(&r.payload).unwrap_or(serde_json::Value::Null),
            created_at: r.created_at,
            expires_at: r.expires_at,
            delivered_at: Some(now),
        })
        .collect();

    match serde_json::to_value(&messages) {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
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

#[derive(serde::Deserialize)]
struct MessageQuery {
    agent_id: Option<String>,
    limit: Option<usize>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct MessageRow {
    message_id: String,
    from_agent: String,
    to_agent: String,
    payload: String,
    created_at: i64,
    expires_at: i64,
    delivered_at: Option<i64>,
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

    if let Err(e) = verify_agent_signature(s.registry.pool(), &agent_id, &signature, &body).await {
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
            return (StatusCode::BAD_REQUEST, format!("invalid base64 data: {e}")).into_response()
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
        .post(format!("{}/api/v1/storage/store", candidate.endpoint_url))
        .json(&proxy_body)
        .send()
        .await;

    match proxy_result {
        Ok(resp) if resp.status().is_success() => {
            let content_hash = blake3::hash(&data).to_hex().to_string();

            let object_id = match s
                .storage_router
                .record_object(&agent_id, &candidate.agent_id, &req.name, size_bytes, &content_hash)
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
                .settle_storage(&agent_id, &candidate.agent_id, amount_mc, &object_id, size_bytes)
                .await
            {
                Ok(tx) => {
                    if let Err(e) = s.registry.adjust_reputation(&candidate.agent_id, 0.1).await {
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
            (StatusCode::BAD_GATEWAY, format!("storage node error ({status}): {text}"))
                .into_response()
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, format!("storage node unreachable: {e}")).into_response()
        }
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
            return (StatusCode::BAD_GATEWAY, "storage node endpoint not found").into_response();
        }
    };

    // Proxy retrieval to the storage node
    let client = reqwest::Client::new();
    let proxy_result =
        client.get(format!("{}/api/v1/storage/{}", endpoint_url, object_id)).send().await;

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
            (StatusCode::BAD_GATEWAY, format!("storage node error ({status}): {text}"))
                .into_response()
        }
        Err(e) => {
            (StatusCode::BAD_GATEWAY, format!("storage node unreachable: {e}")).into_response()
        }
    }
}
