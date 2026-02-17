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

use crate::registry::AgentRegistry;
use crate::router::InferenceRouter;

// ── State ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<AgentRegistry>,
    pub router: Arc<InferenceRouter>,
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
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
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
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn heartbeat(
    State(s): State<AppState>,
    Json(req): Json<HeartbeatRequest>,
) -> impl IntoResponse {
    match s.registry.heartbeat(&req).await {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response(),
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
    axum::extract::Query(params): axum::extract::Query<AgentQuery>,
) -> impl IntoResponse {
    let agent_id = match params.agent_id {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "agent_id required").into_response(),
    };

    match s.registry.ledger().get_balance(&agent_id).await {
        Ok(balance) => {
            (StatusCode::OK, Json(serde_json::to_value(balance).unwrap())).into_response()
        }
        Err(ferris_common::FerrisError::NotFound(msg)) => {
            (StatusCode::NOT_FOUND, msg).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn wallet_history(
    State(s): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<HistoryQuery>,
) -> impl IntoResponse {
    let agent_id = match params.agent_id {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "agent_id required").into_response(),
    };
    let limit = params.limit.unwrap_or(20);

    match s.registry.ledger().get_history(&agent_id, limit).await {
        Ok(history) => {
            (StatusCode::OK, Json(serde_json::to_value(history).unwrap())).into_response()
        }
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
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    // Step 1: Route to best provider
    let candidate = match s.router.route(&req.model, None).await {
        Ok(c) => c,
        Err(ferris_common::FerrisError::NotFound(msg)) => {
            return (StatusCode::NOT_FOUND, msg).into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Step 2: Proxy request to the provider node
    let client = reqwest::Client::new();
    let proxy_result = client
        .post(format!("{}/v1/chat/completions", candidate.endpoint_url))
        .json(&req)
        .send()
        .await;

    match proxy_result {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<ChatCompletionResponse>().await {
                Ok(completion) => {
                    // Step 3: Record settlement (fire-and-forget for now)
                    let job_id = uuid::Uuid::now_v7().to_string();
                    tracing::info!(
                        job_id,
                        model = %req.model,
                        provider = %candidate.agent_id,
                        tokens_in = completion.usage.prompt_tokens,
                        tokens_out = completion.usage.completion_tokens,
                        "inference routed"
                    );

                    (StatusCode::OK, Json(serde_json::to_value(completion).unwrap()))
                        .into_response()
                }
                Err(e) => (
                    StatusCode::BAD_GATEWAY,
                    format!("provider response parse error: {e}"),
                )
                    .into_response(),
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            (
                StatusCode::BAD_GATEWAY,
                format!("provider error ({status}): {text}"),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("provider unreachable: {e}"),
        )
            .into_response(),
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
