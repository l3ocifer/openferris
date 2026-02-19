use std::path::PathBuf;

use ferris_common::{unix_timestamp, FerrisError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[cfg(feature = "encryption")]
use ferris_crypto::Cipher;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub file_id: String,
    pub name: String,
    pub size_bytes: u64,
    pub content_hash: String,
    pub created_at: i64,
}

pub struct ObjectStore {
    pool: SqlitePool,
    objects_dir: PathBuf,
    max_mb: u64,
    #[cfg(feature = "encryption")]
    cipher: Option<Cipher>,
}

impl ObjectStore {
    pub fn new(pool: SqlitePool, objects_dir: PathBuf, max_mb: u64) -> Self {
        Self {
            pool,
            objects_dir,
            max_mb,
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

    /// Store a file with content-addressed deduplication (blake3).
    /// When encryption is enabled, file contents are encrypted before writing to disk.
    pub async fn store(&self, name: &str, data: &[u8]) -> Result<FileInfo> {
        let used: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size_bytes), 0) FROM objects")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        let max_bytes = self.max_mb * 1024 * 1024;
        if used as u64 + data.len() as u64 > max_bytes {
            return Err(FerrisError::CapacityExceeded(format!(
                "storage quota exceeded ({} MB limit)",
                self.max_mb
            )));
        }

        // Hash plaintext for content-addressed deduplication
        let hash_hex = blake3::hash(data).to_hex().to_string();

        // Encrypt data before writing to disk
        #[cfg(feature = "encryption")]
        let disk_data: Vec<u8> = match &self.cipher {
            Some(c) => c.encrypt(data),
            None => data.to_vec(),
        };
        #[cfg(not(feature = "encryption"))]
        let disk_data: &[u8] = data;

        let subdir = &hash_hex[..2];
        let file_path = self.objects_dir.join(subdir).join(&hash_hex);
        if !file_path.exists() {
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&file_path, &disk_data).await?;
        }

        let id = Uuid::now_v7().to_string();
        let now = unix_timestamp();

        sqlx::query(
            "INSERT INTO objects (id, name, size_bytes, local_path, content_hash, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(data.len() as i64)
        .bind(file_path.to_string_lossy().as_ref())
        .bind(&hash_hex)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(FileInfo {
            file_id: id,
            name: name.into(),
            size_bytes: data.len() as u64,
            content_hash: hash_hex,
            created_at: now,
        })
    }

    /// Retrieve a file by id — returns metadata + raw bytes.
    /// When encryption is enabled, file contents are decrypted after reading from disk.
    pub async fn retrieve(&self, file_id: &str) -> Result<(FileInfo, Vec<u8>)> {
        let row = sqlx::query(
            "SELECT id, name, size_bytes, local_path, content_hash, created_at
             FROM objects WHERE id = ?",
        )
        .bind(file_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .ok_or_else(|| FerrisError::NotFound(format!("object: {file_id}")))?;

        let local_path: String = row.get("local_path");
        let raw = tokio::fs::read(&local_path).await?;

        // Decrypt if cipher is available
        #[cfg(feature = "encryption")]
        let data = match &self.cipher {
            Some(c) => c.decrypt(&raw).map_err(|e| FerrisError::Storage(e.to_string()))?,
            None => raw,
        };
        #[cfg(not(feature = "encryption"))]
        let data = raw;

        Ok((row_to_info(&row), data))
    }

    /// List stored files, optionally filtered by name prefix.
    pub async fn list_files(&self, prefix: Option<&str>) -> Result<Vec<FileInfo>> {
        let rows = if let Some(p) = prefix {
            let pattern = format!("{p}%");
            sqlx::query(
                "SELECT id, name, size_bytes, local_path, content_hash, created_at
                 FROM objects WHERE name LIKE ? ORDER BY created_at DESC",
            )
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                "SELECT id, name, size_bytes, local_path, content_hash, created_at
                 FROM objects ORDER BY created_at DESC",
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(rows.iter().map(row_to_info).collect())
    }
}

fn row_to_info(row: &sqlx::sqlite::SqliteRow) -> FileInfo {
    FileInfo {
        file_id: row.get("id"),
        name: row.get("name"),
        size_bytes: row.get::<i64, _>("size_bytes") as u64,
        content_hash: row.get("content_hash"),
        created_at: row.get("created_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::TempDir;

    async fn test_setup() -> (SqlitePool, TempDir) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&db_path).create_if_missing(true))
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS objects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                local_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_objects_name ON objects(name)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_objects_hash ON objects(content_hash)")
            .execute(&pool)
            .await
            .unwrap();

        (pool, tmp)
    }

    #[tokio::test]
    async fn store_and_retrieve() {
        let (pool, tmp) = test_setup().await;
        let objects_dir = tmp.path().join("objects");
        let store = ObjectStore::new(pool, objects_dir, 100);

        let info = store.store("hello.txt", b"hello world").await.unwrap();
        assert_eq!(info.name, "hello.txt");
        assert_eq!(info.size_bytes, 11);
        assert!(!info.content_hash.is_empty());

        let (retrieved, data) = store.retrieve(&info.file_id).await.unwrap();
        assert_eq!(retrieved.name, "hello.txt");
        assert_eq!(data, b"hello world");
    }

    #[tokio::test]
    async fn store_deduplicates_content() {
        let (pool, tmp) = test_setup().await;
        let objects_dir = tmp.path().join("objects");
        let store = ObjectStore::new(pool, objects_dir.clone(), 100);

        let a = store.store("file_a.txt", b"same content").await.unwrap();
        let b = store.store("file_b.txt", b"same content").await.unwrap();

        assert_eq!(a.content_hash, b.content_hash);
        assert_ne!(a.file_id, b.file_id);

        // Only one blob on disk (hash-addressed path)
        let subdir = &a.content_hash[..2];
        let blob_path = objects_dir.join(subdir).join(&a.content_hash);
        assert!(blob_path.exists());
    }

    #[tokio::test]
    async fn list_files_all_and_filtered() {
        let (pool, tmp) = test_setup().await;
        let store = ObjectStore::new(pool, tmp.path().join("objects"), 100);

        store.store("notes/a.md", b"aaa").await.unwrap();
        store.store("notes/b.md", b"bbb").await.unwrap();
        store.store("data.csv", b"x,y").await.unwrap();

        let all = store.list_files(None).await.unwrap();
        assert_eq!(all.len(), 3);

        let notes = store.list_files(Some("notes/")).await.unwrap();
        assert_eq!(notes.len(), 2);

        let csv = store.list_files(Some("data")).await.unwrap();
        assert_eq!(csv.len(), 1);
    }

    #[tokio::test]
    async fn retrieve_missing_returns_not_found() {
        let (pool, tmp) = test_setup().await;
        let store = ObjectStore::new(pool, tmp.path().join("objects"), 100);

        let err = store.retrieve("nonexistent-id").await.unwrap_err();
        assert!(matches!(err, FerrisError::NotFound(_)));
    }

    #[tokio::test]
    async fn capacity_enforcement() {
        let (pool, tmp) = test_setup().await;
        // 1 MB limit
        let store = ObjectStore::new(pool, tmp.path().join("objects"), 1);

        // Store ~500KB — should succeed
        let data = vec![0u8; 500 * 1024];
        store.store("small.bin", &data).await.unwrap();

        // Store another ~600KB — should exceed 1MB limit
        let big = vec![0u8; 600 * 1024];
        let err = store.store("too_big.bin", &big).await.unwrap_err();
        assert!(matches!(err, FerrisError::CapacityExceeded(_)));
    }

    #[tokio::test]
    async fn blake3_hash_is_correct() {
        let (pool, tmp) = test_setup().await;
        let store = ObjectStore::new(pool, tmp.path().join("objects"), 100);

        let data = b"deterministic content";
        let expected = blake3::hash(data).to_hex().to_string();

        let info = store.store("hash_test.bin", data).await.unwrap();
        assert_eq!(info.content_hash, expected);
    }
}
