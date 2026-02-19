use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ferris_common::FerrisError;
use ferris_inference::InferenceBackend;
use ferris_memory::MemoryStore;
use ferris_net::CoordinatorClient;
use ferris_storage::ObjectStore;
use ferris_tasks::TaskScheduler;
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;

// ── Tool parameter types ────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
struct RememberParams {
    /// A unique key for this memory
    key: String,
    /// The value to remember
    value: String,
}

#[derive(Deserialize, JsonSchema)]
struct RecallParams {
    /// Search query to match against memory keys and values
    query: String,
    /// Maximum number of results (default 10)
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct ForgetParams {
    /// The key of the memory to delete
    key: String,
}

#[derive(Deserialize, JsonSchema)]
struct StoreParams {
    /// Filename for the stored object
    name: String,
    /// File contents as a base64-encoded string
    data_base64: String,
}

#[derive(Deserialize, JsonSchema)]
struct RetrieveParams {
    /// The file ID returned by the store tool
    file_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct ListFilesParams {
    /// Optional name prefix filter
    prefix: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct ScheduleTaskParams {
    /// Cron schedule expression (e.g. "0 * * * *")
    schedule: String,
    /// Action description to execute
    action: String,
}

#[derive(Deserialize, JsonSchema)]
struct CancelTaskParams {
    /// The task ID to cancel
    task_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct InferParams {
    /// The prompt or message to send
    prompt: String,
    /// Model name (e.g. "llama3:8b"). Uses first available if omitted.
    model: Option<String>,
    /// Sampling temperature (0.0-2.0)
    temperature: Option<f64>,
    /// Maximum tokens to generate
    max_tokens: Option<u32>,
}

// ── MCP Server ──────────────────────────────────────────────────────────

/// MCP server exposing memory, storage, tasks, and inference tools over stdio.
#[derive(Clone)]
pub struct FerrisMcpServer {
    agent_id: String,
    memory: Arc<MemoryStore>,
    storage: Arc<ObjectStore>,
    tasks: Arc<TaskScheduler>,
    inference: Arc<dyn InferenceBackend>,
    coordinator: Option<Arc<CoordinatorClient>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl FerrisMcpServer {
    pub fn new(
        agent_id: String,
        memory: Arc<MemoryStore>,
        storage: Arc<ObjectStore>,
        tasks: Arc<TaskScheduler>,
        inference: Arc<dyn InferenceBackend>,
        coordinator: Option<Arc<CoordinatorClient>>,
    ) -> Self {
        Self {
            agent_id,
            memory,
            storage,
            tasks,
            inference,
            coordinator,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Returns the agent's identity (agent_id)")]
    async fn whoami(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(format!("agent_id: {}", self.agent_id))]))
    }

    #[tool(description = "Store a key-value memory. Updates value if the key already exists.")]
    async fn remember(
        &self,
        Parameters(p): Parameters<RememberParams>,
    ) -> Result<CallToolResult, McpError> {
        self.memory
            .remember(&p.key, &p.value, None)
            .await
            .map(|entry| {
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&entry).unwrap_or_default(),
                )])
            })
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(description = "Search stored memories by keyword match on key or value")]
    async fn recall(
        &self,
        Parameters(p): Parameters<RecallParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = p.limit.unwrap_or(10);
        self.memory
            .recall(&p.query, limit)
            .await
            .map(|entries| {
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&entries).unwrap_or_default(),
                )])
            })
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(description = "Delete a stored memory by its key")]
    async fn forget(
        &self,
        Parameters(p): Parameters<ForgetParams>,
    ) -> Result<CallToolResult, McpError> {
        self.memory
            .forget(&p.key)
            .await
            .map(|()| CallToolResult::success(vec![Content::text("forgotten")]))
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(
        description = "Store a file. Provide data as a base64-encoded string. Returns the file ID and content hash."
    )]
    async fn store(
        &self,
        Parameters(p): Parameters<StoreParams>,
    ) -> Result<CallToolResult, McpError> {
        let bytes = STANDARD
            .decode(&p.data_base64)
            .map_err(|e| mcp_invalid(format!("invalid base64: {e}")))?;

        self.storage
            .store(&p.name, &bytes)
            .await
            .map(|info| {
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&info).unwrap_or_default(),
                )])
            })
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(description = "Retrieve a stored file by ID. Returns metadata and base64-encoded data.")]
    async fn retrieve(
        &self,
        Parameters(p): Parameters<RetrieveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.storage
            .retrieve(&p.file_id)
            .await
            .map(|(info, data)| {
                let resp = serde_json::json!({
                    "file_id": info.file_id,
                    "name": info.name,
                    "size_bytes": info.size_bytes,
                    "data_base64": STANDARD.encode(&data),
                });
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&resp).unwrap_or_default(),
                )])
            })
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(description = "List stored files, optionally filtered by name prefix")]
    async fn list_files(
        &self,
        Parameters(p): Parameters<ListFilesParams>,
    ) -> Result<CallToolResult, McpError> {
        self.storage
            .list_files(p.prefix.as_deref())
            .await
            .map(|files| {
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&files).unwrap_or_default(),
                )])
            })
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(description = "Schedule a task with a cron expression")]
    async fn schedule_task(
        &self,
        Parameters(p): Parameters<ScheduleTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.tasks
            .schedule_task(&p.schedule, &p.action)
            .await
            .map(|task| {
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task).unwrap_or_default(),
                )])
            })
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(description = "List all scheduled tasks")]
    async fn list_tasks(&self) -> Result<CallToolResult, McpError> {
        self.tasks
            .list_tasks()
            .await
            .map(|tasks| {
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&tasks).unwrap_or_default(),
                )])
            })
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(description = "Cancel a scheduled task by its ID")]
    async fn cancel_task(
        &self,
        Parameters(p): Parameters<CancelTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.tasks
            .cancel_task(&p.task_id)
            .await
            .map(|()| CallToolResult::success(vec![Content::text("task cancelled")]))
            .map_err(|e| mcp_internal(e.to_string()))
    }

    #[tool(
        description = "Run inference via the local Ollama instance. Returns the model's response text."
    )]
    async fn infer(
        &self,
        Parameters(p): Parameters<InferParams>,
    ) -> Result<CallToolResult, McpError> {
        let model = p.model.unwrap_or_else(|| "llama3:8b".to_string());
        let mut req = ferris_inference::ChatCompletionRequest {
            model: model.clone(),
            messages: vec![ferris_inference::ChatMessage {
                role: "user".to_string(),
                content: p.prompt,
            }],
            stream: false,
            temperature: p.temperature,
            max_tokens: p.max_tokens,
        };
        req.stream = false;

        match self.inference.chat_completion(&self.agent_id, &req).await {
            Ok(result) => {
                let text = result
                    .response
                    .choices
                    .first()
                    .map(|c| c.message.content.as_str())
                    .unwrap_or("");
                let resp = serde_json::json!({
                    "model": result.response.model,
                    "content": text,
                    "tokens_in": result.response.usage.prompt_tokens,
                    "tokens_out": result.response.usage.completion_tokens,
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&resp).unwrap_or_default(),
                )]))
            }
            Err(e) => Err(mcp_internal(format!("inference failed: {e}"))),
        }
    }

    #[tool(description = "Query credit balance from the coordinator network")]
    async fn balance(&self) -> Result<CallToolResult, McpError> {
        let client = self
            .coordinator
            .as_ref()
            .ok_or_else(|| mcp_internal("not connected to coordinator".to_string()))?;

        match client.get_balance().await {
            Ok(bal) => {
                let soft = bal.soft_balance_mc as f64 / 1000.0;
                let hard = bal.hard_balance_mc as f64 / 1000.0;
                let resp = serde_json::json!({
                    "soft_credits": soft,
                    "hard_credits": hard,
                    "total_credits": soft + hard,
                });
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&resp).unwrap_or_default(),
                )]))
            }
            Err(e) => Err(mcp_internal(format!("balance query failed: {e}"))),
        }
    }
}

#[tool_handler]
impl ServerHandler for FerrisMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "OpenFerris — local-first agent platform with persistent memory, \
                 content-addressed storage, and task scheduling."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// ── Error helpers ───────────────────────────────────────────────────────

fn mcp_internal(msg: String) -> McpError {
    McpError::internal_error(msg, None)
}

fn mcp_invalid(msg: String) -> McpError {
    McpError::invalid_params(msg, None)
}

// ── Public API ──────────────────────────────────────────────────────────

/// Start the MCP server over stdio (for Claude Desktop, Cursor, etc.).
pub async fn serve_stdio(
    agent_id: String,
    memory: Arc<MemoryStore>,
    storage: Arc<ObjectStore>,
    tasks: Arc<TaskScheduler>,
    inference: Arc<dyn InferenceBackend>,
    coordinator: Option<Arc<CoordinatorClient>>,
) -> ferris_common::Result<()> {
    let server = FerrisMcpServer::new(agent_id, memory, storage, tasks, inference, coordinator);
    let service = server
        .serve(stdio())
        .await
        .map_err(|e| FerrisError::Config(format!("MCP server start failed: {e}")))?;
    service.waiting().await.map_err(|e| FerrisError::Config(format!("MCP server error: {e}")))?;
    Ok(())
}
