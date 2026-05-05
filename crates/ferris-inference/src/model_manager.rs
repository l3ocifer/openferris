use std::path::{Path, PathBuf};

#[cfg(feature = "candle-backend")]
use ferris_common::{FerrisError, Result};
#[cfg(feature = "candle-backend")]
use tracing::info;

/// A model recommendation based on available system resources.
pub struct ModelRecommendation {
    pub repo_id: String,
    pub filename: String,
    pub display_name: String,
    pub size_mb: u64,
}

/// Select the best default model based on available system RAM.
pub fn recommend_model() -> ModelRecommendation {
    let sys = sysinfo::System::new_all();
    let ram_mb = sys.total_memory() / (1024 * 1024);

    if ram_mb >= 16_000 {
        ModelRecommendation {
            repo_id: "Qwen/Qwen2.5-3B-Instruct-GGUF".into(),
            filename: "qwen2.5-3b-instruct-q4_k_m.gguf".into(),
            display_name: "Qwen2.5-3B-Q4_K_M".into(),
            size_mb: 2048,
        }
    } else if ram_mb >= 6_000 {
        ModelRecommendation {
            repo_id: "Qwen/Qwen2.5-1.5B-Instruct-GGUF".into(),
            filename: "qwen2.5-1.5b-instruct-q4_k_m.gguf".into(),
            display_name: "Qwen2.5-1.5B-Q4_K_M".into(),
            size_mb: 1024,
        }
    } else {
        ModelRecommendation {
            repo_id: "Qwen/Qwen2.5-0.5B-Instruct-GGUF".into(),
            filename: "qwen2.5-0.5b-instruct-q4_k_m.gguf".into(),
            display_name: "Qwen2.5-0.5B-Q4_K_M".into(),
            size_mb: 400,
        }
    }
}

/// Find the first .gguf file in the models directory, if any.
pub fn find_local_model(models_dir: &Path) -> Option<PathBuf> {
    if !models_dir.exists() {
        return None;
    }
    let mut entries: Vec<_> = std::fs::read_dir(models_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "gguf").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries.first().map(|e| e.path())
}

/// Download a model from HuggingFace Hub to the models directory.
/// Returns the local path to the downloaded file.
#[cfg(feature = "candle-backend")]
pub async fn download_model(models_dir: &Path) -> Result<PathBuf> {
    let rec = recommend_model();

    std::fs::create_dir_all(models_dir)
        .map_err(|e| FerrisError::Inference(format!("create models dir: {e}")))?;

    let dest = models_dir.join(&rec.filename);
    if dest.exists() {
        info!(model = %rec.display_name, "model already downloaded");
        return Ok(dest);
    }

    info!(
        model = %rec.display_name,
        repo = %rec.repo_id,
        size_mb = rec.size_mb,
        "downloading model from HuggingFace Hub"
    );

    let api = hf_hub::api::tokio::Api::new()
        .map_err(|e| FerrisError::Inference(format!("HuggingFace API init: {e}")))?;
    let repo = api.model(rec.repo_id.clone());
    let downloaded_path = repo
        .get(&rec.filename)
        .await
        .map_err(|e| FerrisError::Inference(format!("model download failed: {e}")))?;

    // Copy from HF cache to our models dir for predictable access
    if downloaded_path != dest {
        tokio::fs::copy(&downloaded_path, &dest)
            .await
            .map_err(|e| FerrisError::Inference(format!("copy model to models dir: {e}")))?;
    }

    info!(model = %rec.display_name, path = %dest.display(), "model downloaded");
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommend_model_returns_valid_recommendation() {
        let rec = recommend_model();
        assert!(!rec.repo_id.is_empty());
        assert!(rec.filename.ends_with(".gguf"));
        assert!(rec.size_mb > 0);
    }

    #[test]
    fn find_local_model_returns_none_for_missing_dir() {
        let result = find_local_model(Path::new("/tmp/nonexistent-ferris-test-dir"));
        assert!(result.is_none());
    }

    #[test]
    fn find_local_model_returns_none_for_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = find_local_model(tmp.path());
        assert!(result.is_none());
    }

    #[test]
    fn find_local_model_finds_gguf_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("test-model.gguf"), b"fake").unwrap();
        let result = find_local_model(tmp.path());
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("test-model.gguf"));
    }
}
