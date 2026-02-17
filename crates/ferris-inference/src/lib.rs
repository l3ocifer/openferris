use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ferris_common::{FerrisError, ModelInfo, Result, SettlementReport};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

// ── Types ───────────────────────────────────────────────────────────────

/// OpenAI-compatible chat completion request (subset).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI-compatible chat completion response (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Result of a local inference execution (for settlement reporting).
#[derive(Debug)]
pub struct InferenceResult {
    pub response: ChatCompletionResponse,
    pub settlement: SettlementReport,
}

// ── Ollama Client ───────────────────────────────────────────────────────

/// Proxies inference requests to a local Ollama instance.
pub struct OllamaProxy {
    base_url: String,
    http: reqwest::Client,
    current_requests: Arc<AtomicU32>,
    max_concurrent: u32,
}

impl OllamaProxy {
    pub fn new(ollama_url: &str, max_concurrent: u32) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build HTTP client");

        Self {
            base_url: ollama_url.trim_end_matches('/').to_string(),
            http,
            current_requests: Arc::new(AtomicU32::new(0)),
            max_concurrent,
        }
    }

    /// Current number of in-flight requests.
    pub fn current_load(&self) -> u32 {
        self.current_requests.load(Ordering::Relaxed)
    }

    /// Check if the local Ollama server is reachable.
    pub async fn health_check(&self) -> Result<bool> {
        match self.http.get(&self.base_url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// List models available on the local Ollama instance.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| FerrisError::Inference(format!("ollama list models: {e}")))?;

        if !resp.status().is_success() {
            return Err(FerrisError::Inference("ollama /api/tags failed".into()));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| FerrisError::Inference(format!("parse tags: {e}")))?;

        let models = body["models"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|m| {
                let name = m["name"].as_str().unwrap_or("unknown").to_string();
                let family = m["details"]["family"].as_str().map(String::from);
                let param_size = m["details"]["parameter_size"]
                    .as_str()
                    .and_then(parse_param_size);
                let quant = m["details"]["quantization_level"]
                    .as_str()
                    .map(String::from);

                ModelInfo {
                    model_name: name,
                    model_family: family,
                    parameter_count_b: param_size,
                    quantization: quant,
                    is_hot: false,
                    avg_tokens_sec: None,
                }
            })
            .collect();

        Ok(models)
    }

    /// Execute a chat completion via Ollama's OpenAI-compatible endpoint.
    pub async fn chat_completion(
        &self,
        agent_id: &str,
        req: &ChatCompletionRequest,
    ) -> Result<InferenceResult> {
        let current = self.current_requests.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_concurrent {
            self.current_requests.fetch_sub(1, Ordering::SeqCst);
            return Err(FerrisError::Inference(format!(
                "at capacity ({}/{})",
                current, self.max_concurrent
            )));
        }

        let _guard = RequestGuard(self.current_requests.clone());
        let job_id = Uuid::now_v7().to_string();
        let start = std::time::Instant::now();

        // Forward to Ollama's OpenAI-compat endpoint (non-streaming for now)
        let mut forward_req = req.clone();
        forward_req.stream = false;

        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&forward_req)
            .send()
            .await
            .map_err(|e| FerrisError::Inference(format!("ollama request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(FerrisError::Inference(format!(
                "ollama error ({status}): {text}"
            )));
        }

        let completion: ChatCompletionResponse = resp
            .json()
            .await
            .map_err(|e| FerrisError::Inference(format!("parse completion: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let settlement = SettlementReport {
            job_id,
            agent_id: agent_id.into(),
            model_name: req.model.clone(),
            tokens_in: completion.usage.prompt_tokens,
            tokens_out: completion.usage.completion_tokens,
            duration_ms,
        };

        info!(
            model = %req.model,
            tokens_in = completion.usage.prompt_tokens,
            tokens_out = completion.usage.completion_tokens,
            duration_ms,
            "inference completed"
        );

        Ok(InferenceResult {
            response: completion,
            settlement,
        })
    }
}

/// RAII guard that decrements the request counter on drop.
struct RequestGuard(Arc<AtomicU32>);

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Parse Ollama parameter size strings like "7B", "13B", "70B" into f64.
fn parse_param_size(s: &str) -> Option<f64> {
    let s = s.trim().to_uppercase();
    if let Some(num) = s.strip_suffix('B') {
        num.parse().ok()
    } else {
        s.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn parse_param_size_known_inputs() {
        assert_eq!(parse_param_size("7B"), Some(7.0));
        assert_eq!(parse_param_size("13B"), Some(13.0));
        assert_eq!(parse_param_size("70B"), Some(70.0));
        assert_eq!(parse_param_size("0.5B"), Some(0.5));
        assert_eq!(parse_param_size("abc"), None);
        assert_eq!(parse_param_size(""), None);
    }

    #[test]
    fn request_guard_decrements_on_drop() {
        let counter = Arc::new(AtomicU32::new(1));
        {
            let _guard = RequestGuard(counter.clone());
            assert_eq!(counter.load(Ordering::SeqCst), 1);
        }
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ollama_proxy_current_load_starts_at_zero() {
        let proxy = OllamaProxy::new("http://localhost:11434", 4);
        assert_eq!(proxy.current_load(), 0);
    }
}
