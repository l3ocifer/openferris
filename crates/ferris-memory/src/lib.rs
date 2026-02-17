use ferris_common::{unix_timestamp, FerrisError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct MemoryStore {
    pool: SqlitePool,
    max_entries: u32,
}

impl MemoryStore {
    pub fn new(pool: SqlitePool, max_entries: u32) -> Self {
        Self { pool, max_entries }
    }

    /// Access the underlying connection pool (for status queries).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Store or update a memory entry (upsert by key).
    pub async fn remember(
        &self,
        key: &str,
        value: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<MemoryEntry> {
        let now = unix_timestamp();
        let meta_json = metadata.as_ref().map(|m| m.to_string());

        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM memories WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;

        let id = if let Some(existing_id) = existing {
            sqlx::query(
                "UPDATE memories SET value = ?, metadata = ?, updated_at = ? WHERE id = ?",
            )
            .bind(value)
            .bind(&meta_json)
            .bind(now)
            .bind(&existing_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;
            existing_id
        } else {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;

            if count >= self.max_entries as i64 {
                return Err(FerrisError::CapacityExceeded(format!(
                    "memory limit reached ({count}/{})",
                    self.max_entries
                )));
            }

            let id = Uuid::now_v7().to_string();
            sqlx::query(
                "INSERT INTO memories (id, key, value, metadata, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(key)
            .bind(value)
            .bind(&meta_json)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;
            id
        };

        Ok(MemoryEntry {
            id,
            key: key.into(),
            value: value.into(),
            metadata,
            created_at: now,
            updated_at: now,
        })
    }

    /// Search memories by substring match on key or value.
    pub async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let pattern = format!("%{query}%");
        let rows = sqlx::query(
            "SELECT id, key, value, metadata, created_at, updated_at
             FROM memories
             WHERE key LIKE ?1 OR value LIKE ?1
             ORDER BY updated_at DESC
             LIMIT ?2",
        )
        .bind(&pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(rows.iter().map(row_to_entry).collect())
    }

    /// Delete a memory by key.
    pub async fn forget(&self, key: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM memories WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(FerrisError::NotFound(format!("memory key: {key}")));
        }
        Ok(())
    }
}

fn row_to_entry(row: &sqlx::sqlite::SqliteRow) -> MemoryEntry {
    let meta_str: Option<String> = row.get("metadata");
    MemoryEntry {
        id: row.get("id"),
        key: row.get("key"),
        value: row.get("value"),
        metadata: meta_str.and_then(|s| serde_json::from_str(&s).ok()),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                value TEXT NOT NULL,
                metadata TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn remember_and_recall() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 100);

        let entry = store.remember("color", "blue", None).await.unwrap();
        assert_eq!(entry.key, "color");
        assert_eq!(entry.value, "blue");
        assert!(entry.metadata.is_none());

        let results = store.recall("color", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "color");
    }

    #[tokio::test]
    async fn remember_upsert_updates_value() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 100);

        let first = store.remember("k", "v1", None).await.unwrap();
        let second = store.remember("k", "v2", None).await.unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.value, "v2");

        let results = store.recall("k", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "v2");
    }

    #[tokio::test]
    async fn remember_with_metadata() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 100);

        let meta = serde_json::json!({"source": "test"});
        let entry = store
            .remember("k", "v", Some(meta.clone()))
            .await
            .unwrap();
        assert_eq!(entry.metadata, Some(meta));
    }

    #[tokio::test]
    async fn forget_removes_entry() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 100);

        store.remember("k", "v", None).await.unwrap();
        store.forget("k").await.unwrap();

        let results = store.recall("k", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn forget_missing_returns_not_found() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 100);

        let err = store.forget("nope").await.unwrap_err();
        assert!(matches!(err, FerrisError::NotFound(_)));
    }

    #[tokio::test]
    async fn capacity_enforcement() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 2);

        store.remember("a", "1", None).await.unwrap();
        store.remember("b", "2", None).await.unwrap();
        let err = store.remember("c", "3", None).await.unwrap_err();
        assert!(matches!(err, FerrisError::CapacityExceeded(_)));
    }

    #[tokio::test]
    async fn recall_searches_key_and_value() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 100);

        store.remember("greeting", "hello world", None).await.unwrap();
        store.remember("farewell", "goodbye", None).await.unwrap();

        let by_key = store.recall("greet", 10).await.unwrap();
        assert_eq!(by_key.len(), 1);
        assert_eq!(by_key[0].key, "greeting");

        let by_value = store.recall("goodbye", 10).await.unwrap();
        assert_eq!(by_value.len(), 1);
        assert_eq!(by_value[0].key, "farewell");
    }

    #[tokio::test]
    async fn recall_respects_limit() {
        let pool = test_pool().await;
        let store = MemoryStore::new(pool, 100);

        for i in 0..5 {
            store.remember(&format!("item{i}"), "data", None).await.unwrap();
        }

        let results = store.recall("item", 3).await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
