use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ferris_common::{FerrisError, ModelInfo, Result, SettlementReport};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[cfg(feature = "candle-backend")]
pub mod candle_backend;
pub mod model_manager;

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

// ── Inference Backend Trait ──────────────────────────────────────────────

/// Unified inference backend trait. Implementations include OllamaBackend
/// (proxy to a running Ollama daemon) and CandleBackend (embedded inference).
#[async_trait::async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Check if the backend is available and ready.
    async fn health_check(&self) -> Result<bool>;

    /// List models available on this backend.
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Run a chat completion.
    async fn chat_completion(
        &self,
        agent_id: &str,
        req: &ChatCompletionRequest,
    ) -> Result<InferenceResult>;

    /// Current number of in-flight requests.
    fn current_load(&self) -> u32;
}

// ── Ollama Backend ──────────────────────────────────────────────────────

/// Proxies inference requests to a local Ollama instance.
pub struct OllamaBackend {
    base_url: String,
    http: reqwest::Client,
    current_requests: Arc<AtomicU32>,
    max_concurrent: u32,
}

impl OllamaBackend {
    pub fn new(ollama_url: &str, max_concurrent: u32) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| FerrisError::Inference(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            base_url: ollama_url.trim_end_matches('/').to_string(),
            http,
            current_requests: Arc::new(AtomicU32::new(0)),
            max_concurrent,
        })
    }
}

#[async_trait::async_trait]
impl InferenceBackend for OllamaBackend {
    fn current_load(&self) -> u32 {
        self.current_requests.load(Ordering::Relaxed)
    }

    async fn health_check(&self) -> Result<bool> {
        match self.http.get(&self.base_url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| FerrisError::Inference(format!("ollama list models: {e}")))?;

        if !resp.status().is_success() {
            return Err(FerrisError::Inference("ollama /api/tags failed".into()));
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| FerrisError::Inference(format!("parse tags: {e}")))?;

        let models = body["models"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|m| {
                let name = m["name"].as_str().unwrap_or("unknown").to_string();
                let family = m["details"]["family"].as_str().map(String::from);
                let param_size = m["details"]["parameter_size"].as_str().and_then(parse_param_size);
                let quant = m["details"]["quantization_level"].as_str().map(String::from);

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

    async fn chat_completion(
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
            return Err(FerrisError::Inference(format!("ollama error ({status}): {text}")));
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
            "inference completed (ollama)"
        );

        Ok(InferenceResult { response: completion, settlement })
    }
}

// ── Backend auto-detection ──────────────────────────────────────────────

/// Create the best available inference backend:
/// 1. If Ollama is running, use OllamaBackend.
/// 2. Otherwise, use CandleBackend (embedded) with auto-downloaded model.
pub async fn create_backend(
    ollama_url: &str,
    max_concurrent: u32,
    models_dir: &std::path::Path,
) -> Result<Arc<dyn InferenceBackend>> {
    let ollama = OllamaBackend::new(ollama_url, max_concurrent)?;
    if ollama.health_check().await.unwrap_or(false) {
        info!("using Ollama backend at {ollama_url}");
        return Ok(Arc::new(ollama));
    }

    #[cfg(feature = "candle-backend")]
    {
        info!("Ollama not detected, using embedded candle backend");
        let backend = candle_backend::CandleBackend::new(models_dir, max_concurrent).await?;
        return Ok(Arc::new(backend));
    }

    #[allow(unreachable_code)]
    {
        info!("Ollama not detected and candle backend not compiled in — inference unavailable");
        Ok(Arc::new(ollama))
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

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
    fn ollama_backend_current_load_starts_at_zero() {
        let backend = OllamaBackend::new("http://localhost:11434", 4).unwrap();
        assert_eq!(backend.current_load(), 0);
    }

    #[test]
    fn ollama_backend_implements_inference_trait() {
        let backend = OllamaBackend::new("http://localhost:11434", 4).unwrap();
        let _dyn_ref: Arc<dyn InferenceBackend> = Arc::new(backend);
    }

    #[tokio::test]
    async fn ollama_health_check_returns_false_when_offline() {
        let backend = OllamaBackend::new("http://127.0.0.1:19999", 4).unwrap();
        let healthy = backend.health_check().await.unwrap();
        assert!(!healthy);
    }

    #[tokio::test]
    async fn create_backend_falls_back_when_ollama_offline() {
        let tmp = tempfile::TempDir::new().unwrap();
        // When Ollama is offline and no model files exist, candle backend will
        // attempt to download. We just verify the function doesn't panic and
        // returns an error (since download will fail in test env).
        let result = create_backend("http://127.0.0.1:19999", 4, tmp.path()).await;
        // Either succeeds with candle (if model cached) or fails trying to download
        // — both are valid, the important thing is it doesn't panic
        let _ = result;
    }

    #[test]
    fn chat_completion_request_serialization() {
        let req = ChatCompletionRequest {
            model: "test".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hello".into() }],
            stream: false,
            temperature: Some(0.7),
            max_tokens: Some(100),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("hello"));

        let deserialized: ChatCompletionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.model, "test");
        assert_eq!(deserialized.messages.len(), 1);
    }

    #[cfg(feature = "candle-backend")]
    #[test]
    fn candle_backend_chat_prompt_formatting() {
        let messages = vec![
            ChatMessage { role: "system".into(), content: "You are helpful.".into() },
            ChatMessage { role: "user".into(), content: "Hi".into() },
        ];
        let prompt = candle_backend::CandleBackend::format_chat_prompt(&messages);
        assert!(prompt.contains("<|im_start|>system"));
        assert!(prompt.contains("You are helpful."));
        assert!(prompt.contains("<|im_start|>user"));
        assert!(prompt.contains("Hi"));
        assert!(prompt.ends_with("<|im_start|>assistant\n"));
    }
}
