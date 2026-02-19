use std::sync::Arc;

use ferris_inference::OllamaBackend;
use ferris_memory::MemoryStore;
use ferris_storage::ObjectStore;
use ferris_tasks::TaskScheduler;
use rmcp::ServerHandler;

async fn setup() -> ferris_mcp::FerrisMcpServer {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    let pool =
        sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display())).await.unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS memories (
            key TEXT PRIMARY KEY, value TEXT NOT NULL, metadata TEXT,
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, embedding BLOB
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS objects (
            file_id TEXT PRIMARY KEY, name TEXT NOT NULL, size_bytes INTEGER NOT NULL,
            content_hash TEXT NOT NULL, created_at INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            task_id TEXT PRIMARY KEY, schedule TEXT NOT NULL, action TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active', created_at INTEGER NOT NULL,
            next_run_at INTEGER, last_run_at INTEGER
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let memory = Arc::new(MemoryStore::new(pool.clone(), 100));
    let objects_dir = tmp.path().join("objects");
    let storage = Arc::new(ObjectStore::new(pool.clone(), objects_dir, 100));
    let tasks = Arc::new(TaskScheduler::new(pool, 10));
    let inference: Arc<dyn ferris_inference::InferenceBackend> =
        Arc::new(OllamaBackend::new("http://localhost:11434", 4).unwrap());

    std::mem::forget(tmp);

    ferris_mcp::FerrisMcpServer::new(
        "test-agent-123".to_string(),
        memory,
        storage,
        tasks,
        inference,
        None,
    )
}

#[tokio::test]
async fn server_info_returns_capabilities() {
    let server = setup().await;
    let info = server.get_info();
    assert!(info.instructions.is_some());
    assert!(info.instructions.unwrap().contains("OpenFerris"));
    assert!(info.capabilities.tools.is_some());
}

#[tokio::test]
async fn server_can_be_constructed() {
    let server = setup().await;
    let info = server.get_info();
    assert!(info.capabilities.tools.is_some());
}
