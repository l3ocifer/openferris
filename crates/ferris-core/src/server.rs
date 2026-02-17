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
    host: &str,
    port: u16,
) -> ferris_common::Result<()> {
    let objects_dir = PathBuf::from(&config.agent.data_dir).join("objects");

    let state = AppState {
        memory: Arc::new(MemoryStore::new(pool.clone(), config.memory.max_entries)),
        storage: Arc::new(ObjectStore::new(pool.clone(), objects_dir, config.storage.max_mb)),
        tasks: Arc::new(TaskScheduler::new(pool, config.tasks.max_scheduled)),
    };

    let app = Router::new()
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
        .with_state(state);

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
) -> Router {
    let state = AppState {
        memory,
        storage,
        tasks,
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
        Ok(entry) => (StatusCode::OK, Json(serde_json::to_value(entry).unwrap())).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn recall(
    State(s): State<AppState>,
    Json(req): Json<RecallReq>,
) -> impl IntoResponse {
    let limit = req.limit.unwrap_or(10);
    match s.memory.recall(&req.query, limit).await {
        Ok(entries) => {
            (StatusCode::OK, Json(serde_json::to_value(entries).unwrap())).into_response()
        }
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
        Ok(tasks) => {
            (StatusCode::OK, Json(serde_json::to_value(tasks).unwrap())).into_response()
        }
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
