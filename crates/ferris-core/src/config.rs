use std::path::{Path, PathBuf};

use ferris_common::{FerrisConfig, FerrisError, Result};

/// Determine the data directory from CLI arg → env var → default.
pub fn resolve_data_dir(cli_override: Option<&str>) -> PathBuf {
    if let Some(dir) = cli_override {
        expand_tilde(dir)
    } else if let Ok(dir) = std::env::var("FERRIS_DATA_DIR") {
        expand_tilde(&dir)
    } else {
        dirs::home_dir()
            .expect("could not determine home directory")
            .join(".ferris")
    }
}

/// Load config from `{data_dir}/config.toml`, falling back to defaults.
pub fn load_config(data_dir: &Path) -> Result<FerrisConfig> {
    let config_path = data_dir.join("config.toml");

    let mut config = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| FerrisError::Config(format!("read config: {e}")))?;
        toml::from_str::<FerrisConfig>(&content)
            .map_err(|e| FerrisError::Config(format!("parse config: {e}")))?
    } else {
        FerrisConfig::default()
    };

    config.agent.data_dir = data_dir.display().to_string();
    apply_env_overrides(&mut config);
    Ok(config)
}

/// Write default config to `{data_dir}/config.toml` if it doesn't exist.
pub fn save_default_config(data_dir: &Path, agent_name: &str) -> Result<()> {
    let config_path = data_dir.join("config.toml");
    if config_path.exists() {
        return Ok(());
    }
    let mut config = FerrisConfig::default();
    config.agent.name = agent_name.to_string();
    let content = toml::to_string_pretty(&config)
        .map_err(|e| FerrisError::Config(format!("serialize config: {e}")))?;
    std::fs::write(&config_path, content)?;
    Ok(())
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .expect("could not determine home directory")
            .join(rest)
    } else if path == "~" {
        dirs::home_dir().expect("could not determine home directory")
    } else {
        PathBuf::from(path)
    }
}

fn apply_env_overrides(config: &mut FerrisConfig) {
    if let Ok(v) = std::env::var("FERRIS_AGENT_NAME") {
        config.agent.name = v;
    }
    if let Ok(v) = std::env::var("FERRIS_MCP_TRANSPORT") {
        config.mcp.transport = v;
    }
    if let Ok(v) = std::env::var("FERRIS_MCP_PORT") {
        if let Ok(p) = v.parse() {
            config.mcp.port = p;
        }
    }
    if let Ok(v) = std::env::var("FERRIS_MEMORY_MAX_ENTRIES") {
        if let Ok(n) = v.parse() {
            config.memory.max_entries = n;
        }
    }
    if let Ok(v) = std::env::var("FERRIS_STORAGE_MAX_MB") {
        if let Ok(n) = v.parse() {
            config.storage.max_mb = n;
        }
    }
    if let Ok(v) = std::env::var("FERRIS_TASKS_MAX_SCHEDULED") {
        if let Ok(n) = v.parse() {
            config.tasks.max_scheduled = n;
        }
    }
    if let Ok(v) = std::env::var("FERRIS_COORDINATOR_URL") {
        config.network.coordinator_url = v;
    }
    if let Ok(v) = std::env::var("FERRIS_HEARTBEAT_INTERVAL") {
        if let Ok(n) = v.parse() {
            config.network.heartbeat_interval_secs = n;
        }
    }
    if let Ok(v) = std::env::var("FERRIS_CONTRIBUTE_PERCENT") {
        if let Ok(n) = v.parse::<u32>() {
            config.network.contribute_percent = n.min(100);
        }
    }
    if let Ok(v) = std::env::var("FERRIS_OLLAMA_URL") {
        config.inference.ollama_url = v;
    }
    if let Ok(v) = std::env::var("FERRIS_MAX_CONCURRENT_REQUESTS") {
        if let Ok(n) = v.parse() {
            config.inference.max_concurrent_requests = n;
        }
    }
}
