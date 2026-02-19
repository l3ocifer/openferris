use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use candle_core::Device;
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_llama as qlm;
use ferris_common::{FerrisError, ModelInfo, Result, SettlementReport};
use tokenizers::Tokenizer;
use tracing::info;
use uuid::Uuid;

use crate::model_manager;
use crate::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Choice, InferenceBackend,
    InferenceResult, Usage,
};

/// Embedded inference backend using candle (pure Rust).
/// Loads quantized GGUF models and runs inference without any external daemon.
pub struct CandleBackend {
    model: Arc<tokio::sync::Mutex<qlm::ModelWeights>>,
    tokenizer: Arc<Tokenizer>,
    model_name: String,
    current_requests: Arc<AtomicU32>,
    max_concurrent: u32,
}

impl CandleBackend {
    /// Initialize the candle backend. Downloads a model if none is available locally.
    pub async fn new(models_dir: &Path, max_concurrent: u32) -> Result<Self> {
        let model_path = match model_manager::find_local_model(models_dir) {
            Some(p) => p,
            None => model_manager::download_model(models_dir).await?,
        };

        let model_name =
            model_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        info!(model = %model_name, path = %model_path.display(), "loading GGUF model");

        let device = Device::Cpu;

        let mut file = std::fs::File::open(&model_path)
            .map_err(|e| FerrisError::Inference(format!("open model file: {e}")))?;
        let gguf = candle_core::quantized::gguf_file::Content::read(&mut file)
            .map_err(|e| FerrisError::Inference(format!("parse GGUF: {e}")))?;
        let model = qlm::ModelWeights::from_gguf(gguf, &mut file, &device)
            .map_err(|e| FerrisError::Inference(format!("load model weights: {e}")))?;

        let tokenizer = Self::load_tokenizer(&model_path).await?;

        info!(model = %model_name, "candle backend ready");

        Ok(Self {
            model: Arc::new(tokio::sync::Mutex::new(model)),
            tokenizer: Arc::new(tokenizer),
            model_name,
            current_requests: Arc::new(AtomicU32::new(0)),
            max_concurrent,
        })
    }

    async fn load_tokenizer(model_path: &Path) -> Result<Tokenizer> {
        let dir = model_path.parent().unwrap_or(Path::new("."));
        let tokenizer_path = dir.join("tokenizer.json");

        if tokenizer_path.exists() {
            return Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| FerrisError::Inference(format!("load tokenizer: {e}")));
        }

        let rec = model_manager::recommend_model();
        let base_repo = rec.repo_id.replace("-GGUF", "");

        info!(repo = %base_repo, "downloading tokenizer from HuggingFace");

        let api = hf_hub::api::tokio::Api::new()
            .map_err(|e| FerrisError::Inference(format!("HF API: {e}")))?;
        let repo = api.model(base_repo);
        let tokenizer_file = repo
            .get("tokenizer.json")
            .await
            .map_err(|e| FerrisError::Inference(format!("download tokenizer: {e}")))?;

        if let Err(e) = std::fs::copy(&tokenizer_file, &tokenizer_path) {
            tracing::warn!(error = %e, "failed to cache tokenizer locally");
        }

        Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| FerrisError::Inference(format!("load tokenizer: {e}")))
    }

    pub(crate) fn format_chat_prompt(messages: &[ChatMessage]) -> String {
        let mut prompt = String::new();
        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    prompt.push_str("<|im_start|>system\n");
                    prompt.push_str(&msg.content);
                    prompt.push_str("<|im_end|>\n");
                }
                "user" => {
                    prompt.push_str("<|im_start|>user\n");
                    prompt.push_str(&msg.content);
                    prompt.push_str("<|im_end|>\n");
                }
                "assistant" => {
                    prompt.push_str("<|im_start|>assistant\n");
                    prompt.push_str(&msg.content);
                    prompt.push_str("<|im_end|>\n");
                }
                _ => {
                    prompt.push_str(&msg.content);
                    prompt.push('\n');
                }
            }
        }
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }
}

#[async_trait::async_trait]
impl InferenceBackend for CandleBackend {
    fn current_load(&self) -> u32 {
        self.current_requests.load(Ordering::Relaxed)
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            model_name: self.model_name.clone(),
            model_family: Some("qwen2".into()),
            parameter_count_b: None,
            quantization: Some("Q4_K_M".into()),
            is_hot: true,
            avg_tokens_sec: None,
        }])
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

        let model = self.model.clone();
        let tokenizer = self.tokenizer.clone();
        let messages = req.messages.clone();
        let temperature = req.temperature.unwrap_or(0.7);
        let max_tokens = req.max_tokens.unwrap_or(512) as usize;
        let model_name = self.model_name.clone();
        let req_model = req.model.clone();
        let agent_id_owned = agent_id.to_string();
        let counter = self.current_requests.clone();

        let result = tokio::task::spawn_blocking(move || {
            let _guard = RequestGuardSync(counter);
            let start = std::time::Instant::now();

            let prompt = Self::format_chat_prompt(&messages);
            let encoding = tokenizer
                .encode(prompt.as_str(), true)
                .map_err(|e| FerrisError::Inference(format!("tokenize: {e}")))?;
            let prompt_tokens = encoding.get_ids();
            let prompt_token_count = prompt_tokens.len() as u32;

            let device = Device::Cpu;
            let mut logits_processor = LogitsProcessor::new(42, Some(temperature), Some(0.9));

            let mut model_guard = model.blocking_lock();
            let mut generated_tokens: Vec<u32> = Vec::new();

            // EOS token IDs for Qwen2.5 ChatML
            let eos_tokens: &[u32] = &[151643, 151645];

            // Process the full prompt
            let input = candle_core::Tensor::new(prompt_tokens, &device)
                .map_err(|e| FerrisError::Inference(format!("tensor: {e}")))?
                .unsqueeze(0)
                .map_err(|e| FerrisError::Inference(format!("unsqueeze: {e}")))?;
            let logits = model_guard
                .forward(&input, 0)
                .map_err(|e| FerrisError::Inference(format!("forward: {e}")))?;
            let logits =
                logits.squeeze(0).map_err(|e| FerrisError::Inference(format!("squeeze: {e}")))?;
            let mut next_token = logits_processor
                .sample(&logits)
                .map_err(|e| FerrisError::Inference(format!("sample: {e}")))?;

            if !eos_tokens.contains(&next_token) {
                generated_tokens.push(next_token);
            }

            // Auto-regressive generation
            for i in 1..max_tokens {
                if eos_tokens.contains(&next_token) {
                    break;
                }

                let input = candle_core::Tensor::new(&[next_token], &device)
                    .map_err(|e| FerrisError::Inference(format!("tensor: {e}")))?
                    .unsqueeze(0)
                    .map_err(|e| FerrisError::Inference(format!("unsqueeze: {e}")))?;
                let logits = model_guard
                    .forward(&input, prompt_tokens.len() + i)
                    .map_err(|e| FerrisError::Inference(format!("forward: {e}")))?;
                let logits = logits
                    .squeeze(0)
                    .map_err(|e| FerrisError::Inference(format!("squeeze: {e}")))?;
                next_token = logits_processor
                    .sample(&logits)
                    .map_err(|e| FerrisError::Inference(format!("sample: {e}")))?;

                if eos_tokens.contains(&next_token) {
                    break;
                }
                generated_tokens.push(next_token);
            }

            let completion_token_count = generated_tokens.len() as u32;

            let output_text = tokenizer
                .decode(&generated_tokens, true)
                .map_err(|e| FerrisError::Inference(format!("decode: {e}")))?;

            let duration_ms = start.elapsed().as_millis() as u64;

            let response = ChatCompletionResponse {
                id: format!("chatcmpl-{}", Uuid::now_v7()),
                object: "chat.completion".into(),
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                model: model_name.clone(),
                choices: vec![Choice {
                    index: 0,
                    message: ChatMessage { role: "assistant".into(), content: output_text },
                    finish_reason: "stop".into(),
                }],
                usage: Usage {
                    prompt_tokens: prompt_token_count,
                    completion_tokens: completion_token_count,
                    total_tokens: prompt_token_count + completion_token_count,
                },
            };

            let settlement = SettlementReport {
                job_id: Uuid::now_v7().to_string(),
                agent_id: agent_id_owned,
                model_name: req_model,
                tokens_in: prompt_token_count,
                tokens_out: completion_token_count,
                duration_ms,
            };

            info!(
                model = %model_name,
                tokens_in = prompt_token_count,
                tokens_out = completion_token_count,
                duration_ms,
                "inference completed (candle)"
            );

            Ok(InferenceResult { response, settlement })
        })
        .await
        .map_err(|e| FerrisError::Inference(format!("inference task panicked: {e}")))?;

        result
    }
}

struct RequestGuardSync(Arc<AtomicU32>);

impl Drop for RequestGuardSync {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
