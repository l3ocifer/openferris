use ed25519_dalek::{Signature, VerifyingKey};
use ferris_common::FerrisError;
use sqlx::SqlitePool;

/// Verify an Ed25519 signature from an agent against the stored public key.
///
/// Looks up the agent's `public_key` in the `agents` table, decodes the
/// base64 signature, and verifies it against the provided body bytes.
pub async fn verify_agent_signature(
    pool: &SqlitePool,
    agent_id: &str,
    signature_b64: &str,
    body: &[u8],
) -> Result<(), FerrisError> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use ed25519_dalek::Verifier;

    let public_key_bytes: Vec<u8> =
        sqlx::query_scalar("SELECT public_key FROM agents WHERE agent_id = ?")
            .bind(agent_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?
            .ok_or_else(|| FerrisError::Auth(format!("unknown agent: {agent_id}")))?;

    let key_bytes: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| FerrisError::Auth("invalid public key length".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| FerrisError::Auth(format!("invalid public key: {e}")))?;

    let sig_bytes = STANDARD
        .decode(signature_b64)
        .map_err(|e| FerrisError::Auth(format!("invalid signature encoding: {e}")))?;

    let sig_array: [u8; 64] =
        sig_bytes.try_into().map_err(|_| FerrisError::Auth("invalid signature length".into()))?;

    let signature = Signature::from_bytes(&sig_array);

    verifying_key
        .verify(body, &signature)
        .map_err(|_| FerrisError::Auth("signature verification failed".into()))
}
