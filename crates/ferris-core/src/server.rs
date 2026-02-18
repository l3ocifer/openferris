use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ferris_common::FerrisConfig;
use ferris_crypto::Cipher;
use ferris_inference::{ChatCompletionRequest, OllamaProxy};
use ferris_memory::MemoryStore;
use ferris_storage::ObjectStore;
use ferris_tasks::TaskScheduler;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

// ── State ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    memory: Arc<MemoryStore>,
    storage: Arc<ObjectStore>,
    tasks: Arc<TaskScheduler>,
    inference: Arc<OllamaProxy>,
    agent_id: String,
}

// ── Request / Response types ────────────────────────────────────────────

#[derive(Deserialize)]
struct RememberReq {
    key: String,
    value: String,
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RecallReq {
    query: String,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct StoreReq {
    name: String,
    data_base64: String,
}

#[derive(Serialize)]
struct StoreResp {
    file_id: String,
    content_hash: String,
    size_bytes: u64,
}

#[derive(Deserialize)]
struct ListQuery {
    prefix: Option<String>,
}

#[derive(Serialize)]
struct FileEntry {
    file_id: String,
    name: String,
    size_bytes: u64,
    content_hash: String,
}

#[derive(Serialize)]
struct FileResp {
    file_id: String,
    name: String,
    data_base64: String,
}

#[derive(Deserialize)]
struct ScheduleReq {
    schedule: String,
    action: String,
}

#[derive(Serialize)]
struct ScheduleResp {
    task_id: String,
}

#[derive(Serialize)]
struct HealthResp {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct StatusResp {
    status: &'static str,
    memories: i64,
    objects: i64,
    active_tasks: i64,
}

// ── Server ──────────────────────────────────────────────────────────────

pub async fn run_server(
    config: &FerrisConfig,
    pool: SqlitePool,
    agent_id: &str,
    host: &str,
    port: u16,
    cipher: Option<Cipher>,
) -> ferris_common::Result<()> {
    let objects_dir = PathBuf::from(&config.agent.data_dir).join("objects");

    let mut memory = MemoryStore::new(pool.clone(), config.memory.max_entries);
    let mut storage = ObjectStore::new(pool.clone(), objects_dir, config.storage.max_mb);
    if let Some(c) = cipher {
        memory = memory.with_cipher(c.clone());
        storage = storage.with_cipher(c);
    }
    let memory = Arc::new(memory);
    let storage = Arc::new(storage);
    let tasks = Arc::new(TaskScheduler::new(pool.clone(), config.tasks.max_scheduled));

    // Start the background task executor
    let _task_executor = TaskScheduler::start_executor(pool, 60);
    let inference = Arc::new(OllamaProxy::new(
        &config.inference.ollama_url,
        config.inference.max_concurrent_requests,
    ));

    let app = build_app(memory, storage, tasks, inference, agent_id);

    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| ferris_common::FerrisError::Config(format!("invalid address: {e}")))?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("ferris server listening on http://{addr}");

    axum::serve(listener, app)
        .await
        .map_err(|e| ferris_common::FerrisError::Config(format!("server error: {e}")))?;

    Ok(())
}

/// Build the router (exposed for integration tests).
pub fn build_app(
    memory: Arc<MemoryStore>,
    storage: Arc<ObjectStore>,
    tasks: Arc<TaskScheduler>,
    inference: Arc<OllamaProxy>,
    agent_id: &str,
) -> Router {
    let state = AppState {
        memory,
        storage,
        tasks,
        inference,
        agent_id: agent_id.into(),
    };

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/memory/remember", post(remember))
        .route("/api/v1/memory/recall", post(recall))
        .route("/api/v1/memory/{key}", delete(forget))
        .route("/api/v1/storage/store", post(store))
        .route("/api/v1/storage", get(list_files))
        .route("/api/v1/storage/{file_id}", get(retrieve))
        .route("/api/v1/tasks", post(schedule_task).get(list_tasks))
        .route("/api/v1/tasks/{task_id}", delete(cancel_task))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .with_state(state)
}

// ── Handlers ────────────────────────────────────────────────────────────

async fn health() -> Json<HealthResp> {
    Json(HealthResp {
        status: "ok",
        service: "openferris",
    })
}

async fn status(State(s): State<AppState>) -> impl IntoResponse {
    let memories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
        .fetch_one(s.memory.pool())
        .await
        .unwrap_or(0);
    let objects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects")
        .fetch_one(s.storage.pool())
        .await
        .unwrap_or(0);
    let active_tasks: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE enabled = 1")
            .fetch_one(s.tasks.pool())
            .await
            .unwrap_or(0);

    Json(StatusResp {
        status: "ok",
        memories,
        objects,
        active_tasks,
    })
}

async fn remember(
    State(s): State<AppState>,
    Json(req): Json<RememberReq>,
) -> impl IntoResponse {
    match s.memory.remember(&req.key, &req.value, req.metadata).await {
        Ok(entry) => match serde_json::to_value(&entry) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn recall(
    State(s): State<AppState>,
    Json(req): Json<RecallReq>,
) -> impl IntoResponse {
    let limit = req.limit.unwrap_or(10);
    match s.memory.recall(&req.query, limit).await {
        Ok(entries) => match serde_json::to_value(&entries) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn forget(
    State(s): State<AppState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    match s.memory.forget(&key).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ferris_common::FerrisError::NotFound(_)) => {
            (StatusCode::NOT_FOUND, "memory key not found").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn store(
    State(s): State<AppState>,
    Json(req): Json<StoreReq>,
) -> impl IntoResponse {
    let bytes = match STANDARD.decode(&req.data_base64) {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid base64").into_response(),
    };
    match s.storage.store(&req.name, &bytes).await {
        Ok(info) => (
            StatusCode::OK,
            Json(StoreResp {
                file_id: info.file_id,
                content_hash: info.content_hash,
                size_bytes: info.size_bytes,
            }),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_files(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    match s.storage.list_files(q.prefix.as_deref()).await {
        Ok(files) => {
            let entries: Vec<FileEntry> = files
                .into_iter()
                .map(|f| FileEntry {
                    file_id: f.file_id,
                    name: f.name,
                    size_bytes: f.size_bytes,
                    content_hash: f.content_hash,
                })
                .collect();
            (StatusCode::OK, Json(entries)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn retrieve(
    State(s): State<AppState>,
    Path(file_id): Path<String>,
) -> impl IntoResponse {
    match s.storage.retrieve(&file_id).await {
        Ok((info, data)) => (
            StatusCode::OK,
            Json(FileResp {
                file_id: info.file_id,
                name: info.name,
                data_base64: STANDARD.encode(&data),
            }),
        )
            .into_response(),
        Err(ferris_common::FerrisError::NotFound(_)) => {
            (StatusCode::NOT_FOUND, "file not found").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn schedule_task(
    State(s): State<AppState>,
    Json(req): Json<ScheduleReq>,
) -> impl IntoResponse {
    match s.tasks.schedule_task(&req.schedule, &req.action).await {
        Ok(task) => (StatusCode::OK, Json(ScheduleResp { task_id: task.task_id })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_tasks(State(s): State<AppState>) -> impl IntoResponse {
    match s.tasks.list_tasks().await {
        Ok(tasks) => match serde_json::to_value(&tasks) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn cancel_task(
    State(s): State<AppState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    match s.tasks.cancel_task(&task_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ferris_common::FerrisError::NotFound(_)) => {
            (StatusCode::NOT_FOUND, "task not found").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn chat_completions(
    State(s): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    match s.inference.chat_completion(&s.agent_id, &req).await {
        Ok(result) => match serde_json::to_value(&result.response) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(ferris_common::FerrisError::Inference(msg)) if msg.contains("at capacity") => {
            (StatusCode::SERVICE_UNAVAILABLE, msg).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

async fn list_models(State(s): State<AppState>) -> impl IntoResponse {
    match s.inference.list_models().await {
        Ok(models) => {
            let response = serde_json::json!({
                "object": "list",
                "data": models.iter().map(|m| serde_json::json!({
                    "id": m.model_name,
                    "object": "model",
                    "owned_by": "local",
                })).collect::<Vec<_>>(),
            });
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(_) => {
            let empty = serde_json::json!({"object": "list", "data": []});
            (StatusCode::OK, Json(empty)).into_response()
        }
    }
}
