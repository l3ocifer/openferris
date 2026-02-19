use ferris_core::config::{load_config, resolve_data_dir, save_default_config};
use ferris_core::identity::Identity;

#[test]
fn resolve_data_dir_uses_env_var() {
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().join("custom-ferris");
    std::env::set_var("FERRIS_DATA_DIR", path.display().to_string());
    let resolved = resolve_data_dir(None);
    std::env::remove_var("FERRIS_DATA_DIR");
    assert_eq!(resolved, path);
}

#[test]
fn resolve_data_dir_prefers_cli_override() {
    std::env::set_var("FERRIS_DATA_DIR", "/should/not/use");
    let resolved = resolve_data_dir(Some("/cli/override"));
    std::env::remove_var("FERRIS_DATA_DIR");
    assert_eq!(resolved, std::path::PathBuf::from("/cli/override"));
}

#[test]
fn load_config_returns_defaults_when_no_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let config = load_config(tmp.path()).unwrap();
    assert_eq!(config.memory.max_entries, 1000);
    assert_eq!(config.storage.max_mb, 100);
    assert_eq!(config.tasks.max_scheduled, 10);
    assert_eq!(config.inference.ollama_url, "http://localhost:11434");
}

#[test]
fn save_and_load_config_roundtrip() {
    let tmp = tempfile::TempDir::new().unwrap();
    save_default_config(tmp.path(), "test-agent").unwrap();
    assert!(tmp.path().join("config.toml").exists());

    let config = load_config(tmp.path()).unwrap();
    assert_eq!(config.agent.name, "test-agent");
}

#[test]
fn save_default_config_is_idempotent() {
    let tmp = tempfile::TempDir::new().unwrap();
    save_default_config(tmp.path(), "agent-1").unwrap();
    save_default_config(tmp.path(), "agent-2").unwrap();
    let config = load_config(tmp.path()).unwrap();
    assert_eq!(config.agent.name, "agent-1"); // first write wins
}

#[test]
fn identity_generate_creates_unique_ids() {
    let id1 = Identity::generate();
    let id2 = Identity::generate();
    assert_ne!(id1.agent_id, id2.agent_id);
    assert_ne!(id1.signing_key.to_bytes(), id2.signing_key.to_bytes());
}

#[test]
fn identity_public_key_is_32_bytes() {
    let id = Identity::generate();
    let pk = id.public_key_bytes();
    assert_eq!(pk.len(), 32);
}

#[tokio::test]
async fn identity_save_and_load_roundtrip() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let pool =
        sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display())).await.unwrap();

    sqlx::query(
        "CREATE TABLE identity (
            agent_id TEXT PRIMARY KEY,
            public_key BLOB NOT NULL,
            secret_key_bytes BLOB NOT NULL,
            created_at INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let original = Identity::generate();
    let original_id = original.agent_id.clone();
    let original_pk = original.public_key_bytes();
    original.save(&pool).await.unwrap();

    let loaded = Identity::load(&pool).await.unwrap().unwrap();
    assert_eq!(loaded.agent_id, original_id);
    assert_eq!(loaded.public_key_bytes(), original_pk);
}

#[tokio::test]
async fn identity_load_returns_none_when_empty() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let pool =
        sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display())).await.unwrap();

    sqlx::query(
        "CREATE TABLE identity (
            agent_id TEXT PRIMARY KEY,
            public_key BLOB NOT NULL,
            secret_key_bytes BLOB NOT NULL,
            created_at INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = Identity::load(&pool).await.unwrap();
    assert!(result.is_none());
}
