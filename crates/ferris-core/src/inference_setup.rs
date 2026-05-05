use std::path::Path;
use std::sync::Arc;

use ferris_common::FerrisConfig;
use ferris_inference::{InferenceBackend, OllamaBackend};

/// Build the configured inference backend for a node.
///
/// Tries `create_backend` first (Ollama → embedded Candle if compiled in).
/// Falls back to `OllamaBackend` (which may be unhealthy but is constructable
/// without a network call) so the rest of the node keeps running even when
/// no inference is currently available.
pub async fn build_inference_backend(
    config: &FerrisConfig,
    data_dir: &Path,
) -> ferris_common::Result<Arc<dyn InferenceBackend>> {
    let models_dir = data_dir.join("models");
    match ferris_inference::create_backend(
        &config.inference.ollama_url,
        config.inference.max_concurrent_requests,
        &models_dir,
    )
    .await
    {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::warn!(error = %e, "inference backend init failed, falling back to Ollama proxy");
            Ok(Arc::new(OllamaBackend::new(
                &config.inference.ollama_url,
                config.inference.max_concurrent_requests,
            )?))
        }
    }
}
