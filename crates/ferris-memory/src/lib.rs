use ferris_common::{unix_timestamp, FerrisError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[cfg(feature = "semantic")]
use std::sync::Arc;

#[cfg(feature = "semantic")]
use std::sync::Mutex;

#[cfg(feature = "encryption")]
use ferris_crypto::Cipher;

// ── Embedding helpers ──────────────────────────────────────────────────────

#[cfg(feature = "semantic")]
struct Embedder {
    model: fastembed::TextEmbedding,
}

#[cfg(feature = "semantic")]
impl Embedder {
    fn try_new() -> std::result::Result<Self, fastembed::Error> {
        let model = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true),
        )?;
        Ok(Self { model })
    }

    fn embed(&mut self, text: &str) -> std::result::Result<Vec<f32>, fastembed::Error> {
        let results = self.model.embed(vec![text], None)?;
        Ok(results.into_iter().next().unwrap_or_default())
    }
}

#[cfg(any(feature = "semantic", test))]
fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

#[cfg(any(feature = "semantic", test))]
fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

#[cfg(any(feature = "semantic", test))]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── Data types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

// ── MemoryStore ────────────────────────────────────────────────────────────

pub struct MemoryStore {
    pool: SqlitePool,
    max_entries: u32,
    #[cfg(feature = "semantic")]
    embedder: Arc<Mutex<Option<Embedder>>>,
    #[cfg(feature = "encryption")]
    cipher: Option<Cipher>,
}

impl MemoryStore {
    pub fn new(pool: SqlitePool, max_entries: u32) -> Self {
        Self {
            pool,
            max_entries,
            #[cfg(feature = "semantic")]
            embedder: Arc::new(Mutex::new(None)),
            #[cfg(feature = "encryption")]
            cipher: None,
        }
    }

    #[cfg(feature = "encryption")]
    pub fn with_cipher(mut self, cipher: Cipher) -> Self {
        self.cipher = Some(cipher);
        self
    }

    /// Access the underlying connection pool (for status queries).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Ensure the embedding model is loaded. Returns true if available.
    #[cfg(feature = "semantic")]
    fn ensure_embedder(&self) -> bool {
        let mut guard = match self.embedder.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.is_some() {
            return true;
        }
        match Embedder::try_new() {
            Ok(e) => {
                tracing::info!("Semantic search model loaded (AllMiniLM-L6-V2)");
                *guard = Some(e);
                true
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to load embedding model, falling back to text search: {err}"
                );
                false
            }
        }
    }

    /// Generate an embedding for the given text (returns None if unavailable).
    #[cfg(feature = "semantic")]
    fn embed_text(&self, text: &str) -> Option<Vec<f32>> {
        let mut guard = match self.embedder.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.as_mut().and_then(|e| match e.embed(text) {
            Ok(v) => Some(v),
            Err(err) => {
                tracing::warn!("Embedding generation failed: {err}");
                None
            }
        })
    }

    /// Encrypt a value if a cipher is configured.
    #[cfg(feature = "encryption")]
    fn encrypt_value(&self, plaintext: &str) -> String {
        match &self.cipher {
            Some(c) => base64_encode(&c.encrypt(plaintext.as_bytes())),
            None => plaintext.to_string(),
        }
    }

    /// Decrypt a value if a cipher is configured.
    #[cfg(feature = "encryption")]
    fn decrypt_value(&self, stored: &str) -> String {
        match &self.cipher {
            Some(c) => match base64_decode(stored) {
                Some(bytes) => match c.decrypt(&bytes) {
                    Ok(plain) => String::from_utf8(plain).unwrap_or_else(|_| stored.to_string()),
                    Err(_) => stored.to_string(),
                },
                None => stored.to_string(),
            },
            None => stored.to_string(),
        }
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

        // Generate embedding from plaintext before encryption
        #[cfg(feature = "semantic")]
        let embedding_blob: Option<Vec<u8>> = {
            self.ensure_embedder();
            let embed_text = format!("{key} {value}");
            self.embed_text(&embed_text).map(|v| embedding_to_bytes(&v))
        };

        // Encrypt value if cipher is available
        #[cfg(feature = "encryption")]
        let stored_value = self.encrypt_value(value);
        #[cfg(not(feature = "encryption"))]
        let stored_value = value.to_string();

        let existing: Option<String> = sqlx::query_scalar("SELECT id FROM memories WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        let id = if let Some(existing_id) = existing {
            #[cfg(feature = "semantic")]
            {
                sqlx::query(
                    "UPDATE memories SET value = ?, metadata = ?, updated_at = ?, embedding = ? WHERE id = ?",
                )
                .bind(&stored_value)
                .bind(&meta_json)
                .bind(now)
                .bind(&embedding_blob)
                .bind(&existing_id)
                .execute(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;
            }
            #[cfg(not(feature = "semantic"))]
            {
                sqlx::query(
                    "UPDATE memories SET value = ?, metadata = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&stored_value)
                .bind(&meta_json)
                .bind(now)
                .bind(&existing_id)
                .execute(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;
            }
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
            #[cfg(feature = "semantic")]
            {
                sqlx::query(
                    "INSERT INTO memories (id, key, value, metadata, created_at, updated_at, embedding)
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(key)
                .bind(&stored_value)
                .bind(&meta_json)
                .bind(now)
                .bind(now)
                .bind(&embedding_blob)
                .execute(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;
            }
            #[cfg(not(feature = "semantic"))]
            {
                sqlx::query(
                    "INSERT INTO memories (id, key, value, metadata, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(key)
                .bind(&stored_value)
                .bind(&meta_json)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;
            }
            id
        };

        Ok(MemoryEntry {
            id,
            key: key.into(),
            value: value.into(),
            metadata,
            created_at: now,
            updated_at: now,
            score: None,
        })
    }

    /// Search memories using hybrid text + semantic search.
    ///
    /// When the semantic feature is enabled and the embedder is available,
    /// queries are embedded and matched against stored embeddings using
    /// cosine similarity. Results are ranked by similarity score.
    /// Falls back to LIKE-based text search if embedding is unavailable.
    pub async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        // Try semantic search first
        #[cfg(feature = "semantic")]
        {
            self.ensure_embedder();
            if let Some(query_embedding) = self.embed_text(query) {
                return self.semantic_recall(&query_embedding, query, limit).await;
            }
        }

        // Fallback: text-based LIKE search
        self.text_recall(query, limit).await
    }

    /// Pure text LIKE search (fallback).
    async fn text_recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
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

        Ok(rows.iter().map(|r| self.row_to_entry(r)).collect())
    }

    /// Semantic search: embed query, load all memory embeddings, compute cosine similarity.
    #[cfg(feature = "semantic")]
    async fn semantic_recall(
        &self,
        query_embedding: &[f32],
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>> {
        let rows = sqlx::query(
            "SELECT id, key, value, metadata, created_at, updated_at, embedding
             FROM memories
             WHERE embedding IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        let text_pattern = format!("%{query_text}%");
        let mut scored: Vec<(f32, MemoryEntry)> = rows
            .iter()
            .map(|row| {
                let embedding_bytes: Vec<u8> = row.get("embedding");
                let stored_embedding = bytes_to_embedding(&embedding_bytes);
                let vector_score = cosine_similarity(query_embedding, &stored_embedding);

                let mut entry = self.row_to_entry(row);
                let text_match = entry.key.contains(query_text)
                    || entry.value.contains(query_text)
                    || entry
                        .key
                        .to_lowercase()
                        .contains(&text_pattern.to_lowercase().replace('%', ""))
                    || entry
                        .value
                        .to_lowercase()
                        .contains(&text_pattern.to_lowercase().replace('%', ""));

                // Boost score if there's also a text match
                let combined_score =
                    if text_match { (vector_score + 0.3).min(1.0) } else { vector_score };

                entry.score = Some(combined_score);
                (combined_score, entry)
            })
            .collect();

        // Also include text-only matches (memories without embeddings)
        let text_only_rows = sqlx::query(
            "SELECT id, key, value, metadata, created_at, updated_at
             FROM memories
             WHERE embedding IS NULL AND (key LIKE ?1 OR value LIKE ?1)",
        )
        .bind(&text_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        for row in &text_only_rows {
            let mut entry = self.row_to_entry(row);
            entry.score = Some(0.5);
            scored.push((0.5, entry));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(_, entry)| entry).collect())
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

    fn row_to_entry(&self, row: &sqlx::sqlite::SqliteRow) -> MemoryEntry {
        let meta_str: Option<String> = row.get("metadata");

        #[cfg(feature = "encryption")]
        let value = self.decrypt_value(row.get::<String, _>("value").as_str());
        #[cfg(not(feature = "encryption"))]
        let value: String = row.get("value");

        MemoryEntry {
            id: row.get("id"),
            key: row.get("key"),
            value,
            metadata: meta_str.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            score: None,
        }
    }
}

// ── Base64 helpers for encryption ──────────────────────────────────────────

#[cfg(feature = "encryption")]
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[cfg(feature = "encryption")]
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).ok()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(":memory:").create_if_missing(true))
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                value TEXT NOT NULL,
                metadata TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                embedding BLOB
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
        assert!(!results.is_empty());
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
        let entry = store.remember("k", "v", Some(meta.clone())).await.unwrap();
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
        assert!(!by_key.is_empty());
        assert!(by_key.iter().any(|e| e.key == "greeting"));

        let by_value = store.recall("goodbye", 10).await.unwrap();
        assert!(!by_value.is_empty());
        assert!(by_value.iter().any(|e| e.key == "farewell"));
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

    #[test]
    fn cosine_similarity_identical_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let score = cosine_similarity(&a, &a);
        assert!((score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let score = cosine_similarity(&a, &b);
        assert!(score.abs() < 1e-5);
    }

    #[test]
    fn embedding_round_trip() {
        let original = vec![0.1_f32, 0.2, 0.3, -0.5];
        let bytes = embedding_to_bytes(&original);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(original.len(), restored.len());
        for (a, b) in original.iter().zip(&restored) {
            assert!((a - b).abs() < 1e-7);
        }
    }
}
