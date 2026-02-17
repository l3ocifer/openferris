use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use ferris_core::server::build_app;
use ferris_inference::OllamaProxy;
use ferris_memory::MemoryStore;
use ferris_storage::ObjectStore;
use ferris_tasks::TaskScheduler;
use http_body_util::BodyExt;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tempfile::TempDir;
use tower::ServiceExt;

async fn setup() -> (axum::Router, TempDir) {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true),
        )
        .await
        .unwrap();

    run_schema(&pool).await;

    let objects_dir = tmp.path().join("objects");
    let memory = Arc::new(MemoryStore::new(pool.clone(), 100));
    let storage = Arc::new(ObjectStore::new(pool.clone(), objects_dir, 100));
    let tasks = Arc::new(TaskScheduler::new(pool, 10));
    let inference = Arc::new(OllamaProxy::new("http://localhost:11434", 4));

    let app = build_app(memory, storage, tasks, inference, "test-agent");
    (app, tmp)
}

async fn run_schema(pool: &SqlitePool) {
    let ddl = [
        "CREATE TABLE IF NOT EXISTS identity (
            agent_id TEXT PRIMARY KEY,
            public_key BLOB NOT NULL,
            secret_key_bytes BLOB NOT NULL,
            created_at INTEGER NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            value TEXT NOT NULL,
            metadata TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS objects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            local_path TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_objects_name ON objects(name)",
        "CREATE INDEX IF NOT EXISTS idx_objects_hash ON objects(content_hash)",
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            schedule TEXT NOT NULL,
            action TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL
        )",
    ];
    for sql in ddl {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

async fn json_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_endpoint() {
    let (app, _tmp) = setup().await;
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn status_endpoint_counts() {
    let (app, _tmp) = setup().await;
    let resp = app
        .clone()
        .oneshot(Request::get("/api/v1/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["memories"], 0);
    assert_eq!(body["objects"], 0);
    assert_eq!(body["active_tasks"], 0);

    // Add a memory and re-check
    let req = json_request(
        Method::POST,
        "/api/v1/memory/remember",
        serde_json::json!({"key": "k", "value": "v"}),
    );
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(Request::get("/api/v1/status").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["memories"], 1);
}

#[tokio::test]
async fn memory_round_trip() {
    let (app, _tmp) = setup().await;

    // Remember
    let req = json_request(
        Method::POST,
        "/api/v1/memory/remember",
        serde_json::json!({"key": "color", "value": "blue"}),
    );
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["key"], "color");
    assert_eq!(body["value"], "blue");

    // Recall
    let req = json_request(
        Method::POST,
        "/api/v1/memory/recall",
        serde_json::json!({"query": "color"}),
    );
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = json_body(resp).await;
    let results = body.as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["value"], "blue");

    // Forget
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/v1/memory/color")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn storage_round_trip() {
    let (app, _tmp) = setup().await;

    // Store
    let req = json_request(
        Method::POST,
        "/api/v1/storage/store",
        serde_json::json!({"name": "test.txt", "data_base64": "SGVsbG8="}),
    );
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let file_id = body["file_id"].as_str().unwrap().to_string();
    assert!(!file_id.is_empty());

    // List
    let resp = app
        .clone()
        .oneshot(Request::get("/api/v1/storage").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Retrieve
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/storage/{file_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["data_base64"], "SGVsbG8=");
}

#[tokio::test]
async fn tasks_round_trip() {
    let (app, _tmp) = setup().await;

    // Schedule
    let req = json_request(
        Method::POST,
        "/api/v1/tasks",
        serde_json::json!({"schedule": "0 * * * *", "action": "ping"}),
    );
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // List
    let resp = app
        .clone()
        .oneshot(Request::get("/api/v1/tasks").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Cancel
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/tasks/{task_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn forget_nonexistent_returns_404() {
    let (app, _tmp) = setup().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/api/v1/memory/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn retrieve_nonexistent_returns_404() {
    let (app, _tmp) = setup().await;
    let resp = app
        .oneshot(
            Request::get("/api/v1/storage/no-such-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
