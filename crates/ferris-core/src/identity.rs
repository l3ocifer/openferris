use ed25519_dalek::SigningKey;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use ferris_common::{unix_timestamp, FerrisError, Result};

/// Ed25519 node identity with UUID v7 agent id.
pub struct Identity {
    pub agent_id: String,
    pub signing_key: SigningKey,
}

impl Identity {
    /// Generate a fresh identity (keypair + agent id).
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut rand_core::OsRng);
        let agent_id = Uuid::now_v7().to_string();
        Self { agent_id, signing_key }
    }

    /// Public key bytes (Ed25519 verifying key).
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    /// Persist identity to the node database.
    /// Uses INSERT OR IGNORE so first-write wins.
    pub async fn save(&self, pool: &SqlitePool) -> Result<()> {
        let now = unix_timestamp();
        let pubkey = self.public_key_bytes().to_vec();
        let secret = self.signing_key.to_bytes().to_vec();

        sqlx::query(
            "INSERT OR IGNORE INTO identity (agent_id, public_key, secret_key_bytes, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&self.agent_id)
        .bind(&pubkey)
        .bind(&secret)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(())
    }

    /// Load the node's identity from the database (returns None if uninitialized).
    pub async fn load(pool: &SqlitePool) -> Result<Option<Self>> {
        let row = sqlx::query("SELECT agent_id, secret_key_bytes FROM identity LIMIT 1")
            .fetch_optional(pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let agent_id: String = row.get("agent_id");
        let secret_bytes: Vec<u8> = row.get("secret_key_bytes");

        let secret_array: [u8; 32] = secret_bytes
            .try_into()
            .map_err(|_| FerrisError::Identity("corrupt secret key (expected 32 bytes)".into()))?;

        let signing_key = SigningKey::from_bytes(&secret_array);

        Ok(Some(Self { agent_id, signing_key }))
    }
}
