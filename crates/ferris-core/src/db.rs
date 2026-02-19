use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use ferris_common::{FerrisError, Result};

/// Open (or create) the node-local SQLite database.
pub async fn init_pool(db_path: &Path) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new().filename(db_path).create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

    Ok(pool)
}

/// Run embedded migrations (compiled into the binary).
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("../../migrations/node")
        .run(pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;
    Ok(())
}
