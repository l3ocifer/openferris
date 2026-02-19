use ferris_common::{
    unix_timestamp, FerrisError, HeartbeatRequest, HeartbeatResponse, ModelInfo, RegisterRequest,
    RegisterResponse, Result, DEFAULT_REPUTATION, EVICTION_TIMEOUT_SECS, HEARTBEAT_TIMEOUT_SECS,
    SIGNUP_BONUS_MC,
};
use ferris_credits::CreditLedger;
use sqlx::SqlitePool;
use tracing::{info, warn};

// ── Agent Registry ──────────────────────────────────────────────────────

pub struct AgentRegistry {
    pool: SqlitePool,
    ledger: CreditLedger,
}

impl AgentRegistry {
    pub fn new(pool: SqlitePool, ledger: CreditLedger) -> Self {
        Self { pool, ledger }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn ledger(&self) -> &CreditLedger {
        &self.ledger
    }

    /// Register a new agent node.
    pub async fn register(&self, req: &RegisterRequest) -> Result<RegisterResponse> {
        let now = unix_timestamp();

        // Check if already registered
        let existing: Option<String> =
            sqlx::query_scalar("SELECT agent_id FROM agents WHERE agent_id = ?")
                .bind(&req.agent_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;

        if existing.is_some() {
            return Ok(RegisterResponse {
                accepted: true,
                signup_bonus_mc: 0,
                message: "already registered".into(),
            });
        }

        let gpu_model = req.resources.gpu.as_ref().map(|g| g.name.clone());
        let gpu_vram = req.resources.gpu.as_ref().map(|g| g.vram_mb as i64);

        sqlx::query(
            "INSERT INTO agents (agent_id, public_key, created_at, last_heartbeat,
             reputation, cpu_cores, ram_mb, storage_avail_mb,
             gpu_model, gpu_vram_mb, contribute_gpu, contribute_storage,
             contribute_cpu, max_concurrent_req, endpoint_url, region)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&req.agent_id)
        .bind(&req.public_key)
        .bind(now)
        .bind(now)
        .bind(DEFAULT_REPUTATION)
        .bind(req.resources.cpu_cores as i64)
        .bind(req.resources.ram_mb as i64)
        .bind(req.resources.storage_avail_mb as i64)
        .bind(&gpu_model)
        .bind(gpu_vram)
        .bind(req.contribute_gpu as i32)
        .bind(req.contribute_storage as i32)
        .bind(req.contribute_cpu as i32)
        .bind(req.max_concurrent_requests as i64)
        .bind(&req.endpoint_url)
        .bind(&req.region)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        // Insert models
        for model in &req.models {
            self.upsert_model(&req.agent_id, model).await?;
        }

        // Create credit account with signup bonus
        self.ledger.create_account(&req.agent_id).await?;

        info!(agent_id = %req.agent_id, "new agent registered");

        Ok(RegisterResponse {
            accepted: true,
            signup_bonus_mc: SIGNUP_BONUS_MC,
            message: "registered successfully".into(),
        })
    }

    /// Process a heartbeat from an agent node.
    pub async fn heartbeat(&self, req: &HeartbeatRequest) -> Result<HeartbeatResponse> {
        let now = unix_timestamp();

        let result = sqlx::query(
            "UPDATE agents SET last_heartbeat = ?, cpu_cores = ?, ram_mb = ?,
             storage_avail_mb = ?, current_requests = ?,
             gpu_model = ?, gpu_vram_mb = ?,
             status = CASE WHEN status = 'suspended' THEN status ELSE 'active' END
             WHERE agent_id = ?",
        )
        .bind(now)
        .bind(req.resources.cpu_cores as i64)
        .bind(req.resources.ram_mb as i64)
        .bind(req.resources.storage_avail_mb as i64)
        .bind(req.current_requests as i64)
        .bind(req.resources.gpu.as_ref().map(|g| g.name.clone()))
        .bind(req.resources.gpu.as_ref().map(|g| g.vram_mb as i64))
        .bind(&req.agent_id)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(FerrisError::NotFound(format!("agent not registered: {}", req.agent_id)));
        }

        // Update models
        for model in &req.models {
            self.upsert_model(&req.agent_id, model).await?;
        }

        Ok(HeartbeatResponse { status: "ok".into(), queued_messages: vec![] })
    }

    /// Run the health monitor: mark degraded and evict stale agents.
    pub async fn sweep_stale_agents(&self) -> Result<(u32, u32)> {
        let now = unix_timestamp();
        let degraded_cutoff = now - HEARTBEAT_TIMEOUT_SECS;
        let evict_cutoff = now - EVICTION_TIMEOUT_SECS;

        let degraded = sqlx::query(
            "UPDATE agents SET status = 'degraded'
             WHERE status = 'active' AND last_heartbeat < ?",
        )
        .bind(degraded_cutoff)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .rows_affected() as u32;

        let evicted = sqlx::query(
            "UPDATE agents SET status = 'evicted'
             WHERE status IN ('active', 'degraded') AND last_heartbeat < ?",
        )
        .bind(evict_cutoff)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .rows_affected() as u32;

        if degraded > 0 || evicted > 0 {
            warn!(degraded, evicted, "stale agent sweep");
        }

        Ok((degraded, evicted))
    }

    /// Get count of active agents.
    pub async fn active_agent_count(&self) -> Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM agents WHERE status = 'active'")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))
    }

    /// Adjust an agent's reputation (positive for success, negative for failure).
    pub async fn adjust_reputation(&self, agent_id: &str, delta: f64) -> Result<f64> {
        let new_rep: f64 = sqlx::query_scalar(
            "UPDATE agents SET reputation = MIN(MAX(reputation + ?, 0.0), 100.0)
             WHERE agent_id = ?
             RETURNING reputation",
        )
        .bind(delta)
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .ok_or_else(|| FerrisError::NotFound(format!("agent: {agent_id}")))?;

        if delta < 0.0 {
            warn!(agent_id, delta, new_rep, "reputation penalty");
        }

        Ok(new_rep)
    }

    /// Award availability credits to all active agents (called periodically).
    pub async fn award_availability_batch(&self, amount_per_agent_mc: i64) -> Result<u32> {
        let active_agents: Vec<String> =
            sqlx::query_scalar("SELECT agent_id FROM agents WHERE status = 'active'")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?;

        let mut awarded = 0u32;
        for agent_id in &active_agents {
            if self.ledger.award_availability(agent_id, amount_per_agent_mc).await.is_ok() {
                awarded += 1;
            }
        }

        if awarded > 0 {
            info!(awarded, amount_per_agent_mc, "availability rewards distributed");
        }

        Ok(awarded)
    }

    async fn upsert_model(&self, agent_id: &str, model: &ModelInfo) -> Result<()> {
        sqlx::query(
            "INSERT INTO models (agent_id, model_name, model_family, parameter_count_b,
             quantization, is_hot, avg_tokens_sec)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(agent_id, model_name) DO UPDATE SET
             model_family = excluded.model_family,
             parameter_count_b = excluded.parameter_count_b,
             quantization = excluded.quantization,
             is_hot = excluded.is_hot,
             avg_tokens_sec = excluded.avg_tokens_sec",
        )
        .bind(agent_id)
        .bind(&model.model_name)
        .bind(&model.model_family)
        .bind(model.parameter_count_b)
        .bind(&model.quantization)
        .bind(model.is_hot as i32)
        .bind(model.avg_tokens_sec)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;
        Ok(())
    }
}
