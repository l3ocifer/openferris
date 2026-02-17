use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Errors ──────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum FerrisError {
    #[error("config error: {0}")]
    Config(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("identity error: {0}")]
    Identity(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("capacity exceeded: {0}")]
    CapacityExceeded(String),
}

pub type Result<T> = std::result::Result<T, FerrisError>;

// ── Config ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerrisConfig {
    pub agent: AgentConfig,
    pub mcp: McpConfig,
    pub memory: MemoryConfig,
    pub storage: StorageConfig,
    pub tasks: TasksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub transport: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub max_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
    pub max_scheduled: u32,
}

impl Default for FerrisConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig {
                name: "ferris-agent".into(),
                data_dir: "~/.ferris".into(),
            },
            mcp: McpConfig {
                transport: "stdio".into(),
                port: 9420,
            },
            memory: MemoryConfig { max_entries: 1000 },
            storage: StorageConfig { max_mb: 100 },
            tasks: TasksConfig { max_scheduled: 10 },
        }
    }
}

// ── Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManifest {
    pub cpu_cores: u16,
    pub ram_mb: u64,
    pub storage_avail_mb: u64,
    pub gpu: Option<GpuInfo>,
    pub ollama_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_mb: u64,
}

// ── Helpers ─────────────────────────────────────────────────────────────

pub fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs() as i64
}
