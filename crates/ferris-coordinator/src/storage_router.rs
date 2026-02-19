use ferris_common::{unix_timestamp, FerrisError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

// ── Types ───────────────────────────────────────────────────────────────

/// A candidate storage node with capacity and reputation info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCandidate {
    pub agent_id: String,
    pub endpoint_url: String,
    pub storage_avail_mb: i64,
    pub reputation: f64,
}

/// Metadata for a file stored on the network (owner, location, hash).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkObject {
    pub object_id: String,
    pub owner_agent: String,
    pub storage_agent: String,
    pub name: String,
    pub size_bytes: i64,
    pub content_hash: String,
    pub created_at: i64,
}

// ── Storage Router ──────────────────────────────────────────────────────

/// Routes file storage requests to available storage nodes on the network.
pub struct StorageRouter {
    pool: SqlitePool,
}

impl StorageRouter {
    /// Create a new storage router backed by the given database pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Find the best storage node (most available space, active, with endpoint).
    pub async fn find_storage_node(&self, exclude_agent: &str) -> Result<StorageCandidate> {
        let row = sqlx::query(
            "SELECT agent_id, endpoint_url, storage_avail_mb, reputation
             FROM agents
             WHERE status = 'active'
               AND contribute_storage = 1
               AND endpoint_url IS NOT NULL
               AND agent_id != ?
             ORDER BY storage_avail_mb DESC, reputation DESC
             LIMIT 1",
        )
        .bind(exclude_agent)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .ok_or_else(|| FerrisError::NotFound("no storage nodes available".into()))?;

        Ok(StorageCandidate {
            agent_id: row.get("agent_id"),
            endpoint_url: row.get("endpoint_url"),
            storage_avail_mb: row.get("storage_avail_mb"),
            reputation: row.get("reputation"),
        })
    }

    /// Record a network object after successful storage.
    pub async fn record_object(
        &self,
        owner_agent: &str,
        storage_agent: &str,
        name: &str,
        size_bytes: i64,
        content_hash: &str,
    ) -> Result<String> {
        let object_id = Uuid::now_v7().to_string();
        let now = unix_timestamp();

        sqlx::query(
            "INSERT INTO network_objects (object_id, owner_agent, storage_agent, name, size_bytes, content_hash, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&object_id)
        .bind(owner_agent)
        .bind(storage_agent)
        .bind(name)
        .bind(size_bytes)
        .bind(content_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(object_id)
    }

    /// List network objects owned by an agent.
    pub async fn list_objects(&self, owner_agent: &str) -> Result<Vec<NetworkObject>> {
        let rows = sqlx::query(
            "SELECT object_id, owner_agent, storage_agent, name, size_bytes, content_hash, created_at
             FROM network_objects
             WHERE owner_agent = ? AND status = 'active'
             ORDER BY created_at DESC",
        )
        .bind(owner_agent)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| NetworkObject {
                object_id: row.get("object_id"),
                owner_agent: row.get("owner_agent"),
                storage_agent: row.get("storage_agent"),
                name: row.get("name"),
                size_bytes: row.get("size_bytes"),
                content_hash: row.get("content_hash"),
                created_at: row.get("created_at"),
            })
            .collect())
    }

    /// Look up which node stores a given object.
    pub async fn find_object(&self, object_id: &str, owner_agent: &str) -> Result<NetworkObject> {
        let row = sqlx::query(
            "SELECT object_id, owner_agent, storage_agent, name, size_bytes, content_hash, created_at
             FROM network_objects
             WHERE object_id = ? AND owner_agent = ? AND status = 'active'",
        )
        .bind(object_id)
        .bind(owner_agent)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .ok_or_else(|| FerrisError::NotFound(format!("network object not found: {object_id}")))?;

        Ok(NetworkObject {
            object_id: row.get("object_id"),
            owner_agent: row.get("owner_agent"),
            storage_agent: row.get("storage_agent"),
            name: row.get("name"),
            size_bytes: row.get("size_bytes"),
            content_hash: row.get("content_hash"),
            created_at: row.get("created_at"),
        })
    }
}
