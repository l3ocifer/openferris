use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use ferris_common::{
    FerrisError, HeartbeatRequest, HeartbeatResponse, RegisterRequest, RegisterResponse, Result,
    WalletBalance,
};
use tokio::sync::watch;
use tracing::{error, info, warn};

// ── Coordinator Client ──────────────────────────────────────────────────

/// HTTP client for node → coordinator communication.
#[derive(Clone)]
pub struct CoordinatorClient {
    base_url: String,
    agent_id: String,
    signing_key: Arc<SigningKey>,
    http: reqwest::Client,
}

impl CoordinatorClient {
    pub fn new(base_url: &str, agent_id: &str, signing_key: SigningKey) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| FerrisError::Network(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            agent_id: agent_id.into(),
            signing_key: Arc::new(signing_key),
            http,
        })
    }

    /// Sign a JSON payload with Ed25519 and return the base64-encoded signature.
    #[cfg(test)]
    pub(crate) fn sign_payload_for_test(&self, body: &[u8]) -> String {
        self.sign_payload(body)
    }

    fn sign_payload(&self, body: &[u8]) -> String {
        let signature = self.signing_key.sign(body);
        STANDARD.encode(signature.to_bytes())
    }

    /// Register this node with the coordinator.
    pub async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse> {
        let body = serde_json::to_vec(req).map_err(|e| FerrisError::Network(e.to_string()))?;
        let sig = self.sign_payload(&body);

        let resp = self
            .http
            .post(format!("{}/api/v1/register", self.base_url))
            .header("X-Agent-Id", &self.agent_id)
            .header("X-Signature", sig)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| FerrisError::Network(format!("register failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(FerrisError::Network(format!("register rejected ({status}): {text}")));
        }

        resp.json().await.map_err(|e| FerrisError::Network(format!("register parse: {e}")))
    }

    /// Send a heartbeat to the coordinator.
    pub async fn heartbeat(&self, req: &HeartbeatRequest) -> Result<HeartbeatResponse> {
        let body = serde_json::to_vec(req).map_err(|e| FerrisError::Network(e.to_string()))?;
        let sig = self.sign_payload(&body);

        let resp = self
            .http
            .post(format!("{}/api/v1/heartbeat", self.base_url))
            .header("X-Agent-Id", &self.agent_id)
            .header("X-Signature", sig)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| FerrisError::Network(format!("heartbeat failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(FerrisError::Network(format!("heartbeat rejected ({status}): {text}")));
        }

        resp.json().await.map_err(|e| FerrisError::Network(format!("heartbeat parse: {e}")))
    }

    /// Query wallet balance.
    pub async fn get_balance(&self) -> Result<WalletBalance> {
        let timestamp = ferris_common::unix_timestamp().to_string();
        let sig = self.sign_payload(timestamp.as_bytes());

        let resp = self
            .http
            .get(format!("{}/api/v1/wallet/balance", self.base_url))
            .header("X-Agent-Id", &self.agent_id)
            .header("X-Signature", sig)
            .header("X-Timestamp", timestamp)
            .send()
            .await
            .map_err(|e| FerrisError::Network(format!("balance query failed: {e}")))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(FerrisError::Network(format!("balance query: {text}")));
        }

        resp.json().await.map_err(|e| FerrisError::Network(format!("balance parse: {e}")))
    }
}

// ── Heartbeat Loop ──────────────────────────────────────────────────────

/// Runs the heartbeat loop in the background. Returns a shutdown sender.
pub async fn start_heartbeat_loop(
    client: CoordinatorClient,
    make_request: impl Fn() -> HeartbeatRequest + Send + 'static,
    interval_secs: u64,
) -> watch::Sender<bool> {
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let req = make_request();
                    match client.heartbeat(&req).await {
                        Ok(resp) => {
                            if resp.status != "ok" {
                                warn!(status = %resp.status, "coordinator heartbeat status");
                            }
                            if !resp.queued_messages.is_empty() {
                                info!(count = resp.queued_messages.len(), "received queued messages");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "heartbeat failed");
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("heartbeat loop shutting down");
                        break;
                    }
                }
            }
        }
    });

    shutdown_tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    #[test]
    fn sign_payload_produces_valid_signature() {
        let signing_key = SigningKey::generate(&mut rand_core::OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let client =
            CoordinatorClient::new("http://localhost:9999", "test-agent", signing_key).unwrap();

        let payload = b"hello, ferris!";
        let sig_b64 = client.sign_payload_for_test(payload);

        let sig_bytes = STANDARD.decode(&sig_b64).expect("valid base64");
        let signature =
            Signature::from_bytes(&sig_bytes.try_into().expect("signature should be 64 bytes"));

        verifying_key.verify(payload, &signature).expect("signature should be valid");
    }
}
