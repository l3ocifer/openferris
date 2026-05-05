use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Errors ──────────────────────────────────────────────────────────────

/// Unified error type for all OpenFerris subsystems.
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

    #[error("network error: {0}")]
    Network(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("insufficient credits: {0}")]
    InsufficientCredits(String),

    #[error("inference error: {0}")]
    Inference(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}

/// Convenience alias for `Result<T, FerrisError>`.
pub type Result<T> = std::result::Result<T, FerrisError>;

// ── Config ──────────────────────────────────────────────────────────────

/// Top-level configuration for a Ferris agent node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerrisConfig {
    pub agent: AgentConfig,
    pub mcp: McpConfig,
    pub memory: MemoryConfig,
    pub storage: StorageConfig,
    pub tasks: TasksConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub inference: InferenceConfig,
    #[serde(default)]
    pub server: ServerConfig,
}

/// HTTP server settings (auth, CORS).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerConfig {
    /// If set, all `/v1/*` requests must include `Authorization: Bearer <api_key>`.
    /// Generated automatically by `ferris cursor` if missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Allow all origins on `/v1/*` (default: true). Useful for browser-based
    /// OpenAI clients hitting a tunneled local node.
    #[serde(default = "default_cors_permissive")]
    pub cors_permissive: bool,
}

fn default_cors_permissive() -> bool {
    true
}

/// Agent identity and data directory settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub data_dir: String,
}

/// MCP transport and port settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub transport: String,
    pub port: u16,
}

/// Memory subsystem capacity limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_entries: u32,
}

/// Object storage capacity limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub max_mb: u64,
}

/// Task scheduler capacity limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
    pub max_scheduled: u32,
}

/// Coordinator network and resource contribution settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub coordinator_url: String,
    pub heartbeat_interval_secs: u64,
    pub contribute_gpu: bool,
    pub contribute_storage: bool,
    pub contribute_cpu: bool,
    /// Percentage of resources to contribute (0-100). Default 50.
    #[serde(default = "default_contribute_percent")]
    pub contribute_percent: u32,
}

fn default_contribute_percent() -> u32 {
    50
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            coordinator_url: "https://api.openferris.com".into(),
            heartbeat_interval_secs: 30,
            contribute_gpu: true,
            contribute_storage: true,
            contribute_cpu: true,
            contribute_percent: 50,
        }
    }
}

/// Local Ollama inference endpoint settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub ollama_url: String,
    pub max_concurrent_requests: u32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self { ollama_url: "http://localhost:11434".into(), max_concurrent_requests: 4 }
    }
}

impl Default for FerrisConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig { name: "ferris-agent".into(), data_dir: "~/.ferris".into() },
            mcp: McpConfig { transport: "stdio".into(), port: 9420 },
            memory: MemoryConfig { max_entries: 1000 },
            storage: StorageConfig { max_mb: 100 },
            tasks: TasksConfig { max_scheduled: 10 },
            network: NetworkConfig::default(),
            inference: InferenceConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

// ── Types ───────────────────────────────────────────────────────────────

/// Hardware resources available on an agent node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManifest {
    pub cpu_cores: u16,
    pub ram_mb: u64,
    pub storage_avail_mb: u64,
    pub gpu: Option<GpuInfo>,
}

/// GPU device name and VRAM capacity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vram_mb: u64,
}

impl ResourceManifest {
    /// Return a manifest reflecting only the contributed portion of resources.
    pub fn contributed(&self, percent: u32) -> Self {
        let pct = percent.min(100) as u64;
        Self {
            cpu_cores: ((self.cpu_cores as u64 * pct) / 100).max(1) as u16,
            ram_mb: (self.ram_mb * pct) / 100,
            storage_avail_mb: (self.storage_avail_mb * pct) / 100,
            gpu: self.gpu.clone(),
        }
    }
}

// ── Protocol types (node ↔ coordinator) ─────────────────────────────────

/// Registration request from node to coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub agent_id: String,
    pub public_key: Vec<u8>,
    pub resources: ResourceManifest,
    pub models: Vec<ModelInfo>,
    pub contribute_gpu: bool,
    pub contribute_storage: bool,
    pub contribute_cpu: bool,
    pub max_concurrent_requests: u32,
    pub endpoint_url: Option<String>,
    pub region: Option<String>,
}

/// Coordinator response to a registration request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub accepted: bool,
    pub signup_bonus_mc: i64,
    pub message: String,
}

/// Heartbeat from node to coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub agent_id: String,
    pub resources: ResourceManifest,
    pub models: Vec<ModelInfo>,
    pub current_requests: u32,
}

/// Coordinator response to a heartbeat, including queued messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub status: String,
    pub queued_messages: Vec<serde_json::Value>,
}

/// Model info reported by a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub model_name: String,
    pub model_family: Option<String>,
    pub parameter_count_b: Option<f64>,
    pub quantization: Option<String>,
    pub is_hot: bool,
    pub avg_tokens_sec: Option<f64>,
}

/// Wallet balance response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    pub agent_id: String,
    pub soft_balance_mc: i64,
    pub hard_balance_mc: i64,
    pub total_earned_soft_mc: i64,
    pub total_earned_hard_mc: i64,
    pub total_spent_mc: i64,
}

/// Settlement report from node after serving inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementReport {
    pub job_id: String,
    pub agent_id: String,
    pub model_name: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub duration_ms: u64,
}

/// Embedding request (OpenAI-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: EmbeddingInput,
}

/// Embedding input — single string or batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Batch(Vec<String>),
}

/// Settlement request from a node reporting completed inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRequest {
    pub job_id: String,
    pub consumer_agent: String,
    pub model_name: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub duration_ms: u64,
}

/// Agent-to-agent message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub message_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub payload: serde_json::Value,
    pub created_at: i64,
    pub expires_at: i64,
    pub delivered_at: Option<i64>,
}

/// Request to send an agent-to-agent message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub to_agent: String,
    pub payload: serde_json::Value,
}

// ── Constants ───────────────────────────────────────────────────────────

/// Platform fee taken from each settlement, as a percentage.
pub const PLATFORM_FEE_PERCENT: u32 = 15;
/// Signup bonus awarded to new agents (in millicredits).
pub const SIGNUP_BONUS_MC: i64 = 100_000; // 100 credits in millicredits
/// Seconds without a heartbeat before marking an agent degraded.
pub const HEARTBEAT_TIMEOUT_SECS: i64 = 90;
/// Seconds without a heartbeat before evicting an agent.
pub const EVICTION_TIMEOUT_SECS: i64 = 300;
/// Initial reputation score for newly registered agents.
pub const DEFAULT_REPUTATION: f64 = 50.0;

// ── Helpers ─────────────────────────────────────────────────────────────

/// Current time as seconds since the Unix epoch.
pub fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs() as i64
}

/// Calculate platform fee for a transaction amount (in millicredits).
pub fn platform_fee(amount_mc: i64) -> i64 {
    (amount_mc * PLATFORM_FEE_PERCENT as i64) / 100
}
