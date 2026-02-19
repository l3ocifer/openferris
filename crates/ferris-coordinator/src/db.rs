use std::path::Path;

use ferris_common::{FerrisError, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

pub async fn init_coordinator_pool(db_path: &Path) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new().filename(db_path).create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

    // Enable WAL mode for better concurrent read/write
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

    Ok(pool)
}

pub async fn run_coordinator_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("../../migrations/coordinator")
        .run(pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;
    Ok(())
}
