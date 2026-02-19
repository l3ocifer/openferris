use ferris_common::{
    platform_fee, unix_timestamp, FerrisError, Result, WalletBalance, SIGNUP_BONUS_MC,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

// ── Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_id: String,
    pub timestamp: i64,
    pub from_agent: Option<String>,
    pub to_agent: Option<String>,
    pub tx_type: String,
    pub amount_mc: i64,
    pub credit_type: String,
    pub model_name: Option<String>,
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
    pub job_id: Option<String>,
    pub platform_fee_mc: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowEntry {
    pub escrow_id: String,
    pub job_id: String,
    pub buyer_agent: String,
    pub seller_agent: String,
    pub amount_mc: i64,
    pub created_at: i64,
    pub expires_at: i64,
    pub status: String,
}

// ── Credit Ledger ───────────────────────────────────────────────────────

pub struct CreditLedger {
    pool: SqlitePool,
}

impl CreditLedger {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Initialize credit record for a new agent with signup bonus.
    pub async fn create_account(&self, agent_id: &str) -> Result<WalletBalance> {
        let now = unix_timestamp();

        sqlx::query(
            "INSERT INTO credits (agent_id, soft_balance_mc, total_earned_soft_mc)
             VALUES (?, ?, ?)",
        )
        .bind(agent_id)
        .bind(SIGNUP_BONUS_MC)
        .bind(SIGNUP_BONUS_MC)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        // Record the signup bonus as a transaction
        let tx_id = Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO transactions (tx_id, timestamp, to_agent, tx_type, amount_mc, credit_type, status)
             VALUES (?, ?, ?, 'signup_bonus', ?, 'soft', 'completed')",
        )
        .bind(&tx_id)
        .bind(now)
        .bind(agent_id)
        .bind(SIGNUP_BONUS_MC)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        self.get_balance(agent_id).await
    }

    /// Get the wallet balance for an agent.
    pub async fn get_balance(&self, agent_id: &str) -> Result<WalletBalance> {
        let row = sqlx::query(
            "SELECT agent_id, soft_balance_mc, hard_balance_mc,
                    total_earned_soft_mc, total_earned_hard_mc, total_spent_mc
             FROM credits WHERE agent_id = ?",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .ok_or_else(|| FerrisError::NotFound(format!("credits for agent: {agent_id}")))?;

        Ok(WalletBalance {
            agent_id: row.get("agent_id"),
            soft_balance_mc: row.get("soft_balance_mc"),
            hard_balance_mc: row.get("hard_balance_mc"),
            total_earned_soft_mc: row.get("total_earned_soft_mc"),
            total_earned_hard_mc: row.get("total_earned_hard_mc"),
            total_spent_mc: row.get("total_spent_mc"),
        })
    }

    /// Award availability credits (soft) for uptime.
    pub async fn award_availability(&self, agent_id: &str, amount_mc: i64) -> Result<Transaction> {
        let now = unix_timestamp();
        let tx_id = Uuid::now_v7().to_string();

        sqlx::query(
            "UPDATE credits SET soft_balance_mc = soft_balance_mc + ?,
                                total_earned_soft_mc = total_earned_soft_mc + ?
             WHERE agent_id = ?",
        )
        .bind(amount_mc)
        .bind(amount_mc)
        .bind(agent_id)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT INTO transactions (tx_id, timestamp, to_agent, tx_type, amount_mc, credit_type, status)
             VALUES (?, ?, ?, 'availability', ?, 'soft', 'completed')",
        )
        .bind(&tx_id)
        .bind(now)
        .bind(agent_id)
        .bind(amount_mc)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(Transaction {
            tx_id,
            timestamp: now,
            from_agent: None,
            to_agent: Some(agent_id.into()),
            tx_type: "availability".into(),
            amount_mc,
            credit_type: "soft".into(),
            model_name: None,
            tokens_in: None,
            tokens_out: None,
            job_id: None,
            platform_fee_mc: 0,
            status: "completed".into(),
        })
    }

    /// Settle an inference job: debit consumer, credit provider (minus platform fee).
    ///
    /// All balance checks and mutations are wrapped in a single SQLite
    /// transaction (`BEGIN IMMEDIATE`) so the read-then-write is atomic.
    #[allow(clippy::too_many_arguments)]
    pub async fn settle_inference(
        &self,
        consumer_id: &str,
        provider_id: &str,
        amount_mc: i64,
        model_name: &str,
        tokens_in: u32,
        tokens_out: u32,
        job_id: &str,
    ) -> Result<Transaction> {
        let now = unix_timestamp();
        let tx_id = Uuid::now_v7().to_string();
        let fee = platform_fee(amount_mc);
        let provider_payout = amount_mc - fee;

        let mut txn = self.pool.begin().await.map_err(|e| FerrisError::Database(e.to_string()))?;

        // Check consumer has enough balance (soft or hard)
        let row =
            sqlx::query("SELECT soft_balance_mc, hard_balance_mc FROM credits WHERE agent_id = ?")
                .bind(consumer_id)
                .fetch_optional(&mut *txn)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?
                .ok_or_else(|| {
                    FerrisError::NotFound(format!("credits for agent: {consumer_id}"))
                })?;

        let soft_balance: i64 = Row::get(&row, "soft_balance_mc");
        let hard_balance: i64 = Row::get(&row, "hard_balance_mc");
        let total_available = soft_balance + hard_balance;
        if total_available < amount_mc {
            return Err(FerrisError::InsufficientCredits(format!(
                "need {amount_mc} mc, have {total_available} mc"
            )));
        }

        // Debit from soft first, then hard
        let soft_debit = amount_mc.min(soft_balance);
        let hard_debit = amount_mc - soft_debit;

        sqlx::query(
            "UPDATE credits SET soft_balance_mc = soft_balance_mc - ?,
                                hard_balance_mc = hard_balance_mc - ?,
                                total_spent_mc = total_spent_mc + ?
             WHERE agent_id = ?",
        )
        .bind(soft_debit)
        .bind(hard_debit)
        .bind(amount_mc)
        .bind(consumer_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        // Credit provider (hard credits — earned)
        sqlx::query(
            "UPDATE credits SET hard_balance_mc = hard_balance_mc + ?,
                                total_earned_hard_mc = total_earned_hard_mc + ?
             WHERE agent_id = ?",
        )
        .bind(provider_payout)
        .bind(provider_payout)
        .bind(provider_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        // Record transaction
        sqlx::query(
            "INSERT INTO transactions
             (tx_id, timestamp, from_agent, to_agent, tx_type, amount_mc, credit_type,
              model_name, tokens_in, tokens_out, job_id, platform_fee_mc, status)
             VALUES (?, ?, ?, ?, 'inference', ?, 'hard', ?, ?, ?, ?, ?, 'completed')",
        )
        .bind(&tx_id)
        .bind(now)
        .bind(consumer_id)
        .bind(provider_id)
        .bind(amount_mc)
        .bind(model_name)
        .bind(tokens_in as i64)
        .bind(tokens_out as i64)
        .bind(job_id)
        .bind(fee)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        txn.commit().await.map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(Transaction {
            tx_id,
            timestamp: now,
            from_agent: Some(consumer_id.into()),
            to_agent: Some(provider_id.into()),
            tx_type: "inference".into(),
            amount_mc,
            credit_type: "hard".into(),
            model_name: Some(model_name.into()),
            tokens_in: Some(tokens_in as i64),
            tokens_out: Some(tokens_out as i64),
            job_id: Some(job_id.into()),
            platform_fee_mc: fee,
            status: "completed".into(),
        })
    }

    /// Settle a storage operation: debit owner, credit storage provider (minus platform fee).
    ///
    /// Pricing: 1 millicredit per KB stored.
    pub async fn settle_storage(
        &self,
        owner_id: &str,
        storage_provider_id: &str,
        amount_mc: i64,
        object_id: &str,
        size_bytes: i64,
    ) -> Result<Transaction> {
        let now = unix_timestamp();
        let tx_id = Uuid::now_v7().to_string();
        let fee = platform_fee(amount_mc);
        let provider_payout = amount_mc - fee;

        let mut txn = self.pool.begin().await.map_err(|e| FerrisError::Database(e.to_string()))?;

        let row =
            sqlx::query("SELECT soft_balance_mc, hard_balance_mc FROM credits WHERE agent_id = ?")
                .bind(owner_id)
                .fetch_optional(&mut *txn)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?
                .ok_or_else(|| FerrisError::NotFound(format!("credits for agent: {owner_id}")))?;

        let soft_balance: i64 = Row::get(&row, "soft_balance_mc");
        let hard_balance: i64 = Row::get(&row, "hard_balance_mc");
        let total_available = soft_balance + hard_balance;
        if total_available < amount_mc {
            return Err(FerrisError::InsufficientCredits(format!(
                "need {amount_mc} mc, have {total_available} mc"
            )));
        }

        let soft_debit = amount_mc.min(soft_balance);
        let hard_debit = amount_mc - soft_debit;

        sqlx::query(
            "UPDATE credits SET soft_balance_mc = soft_balance_mc - ?,
                                hard_balance_mc = hard_balance_mc - ?,
                                total_spent_mc = total_spent_mc + ?
             WHERE agent_id = ?",
        )
        .bind(soft_debit)
        .bind(hard_debit)
        .bind(amount_mc)
        .bind(owner_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        sqlx::query(
            "UPDATE credits SET hard_balance_mc = hard_balance_mc + ?,
                                total_earned_hard_mc = total_earned_hard_mc + ?
             WHERE agent_id = ?",
        )
        .bind(provider_payout)
        .bind(provider_payout)
        .bind(storage_provider_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT INTO transactions
             (tx_id, timestamp, from_agent, to_agent, tx_type, amount_mc, credit_type,
              job_id, platform_fee_mc, status)
             VALUES (?, ?, ?, ?, 'storage', ?, 'hard', ?, ?, 'completed')",
        )
        .bind(&tx_id)
        .bind(now)
        .bind(owner_id)
        .bind(storage_provider_id)
        .bind(amount_mc)
        .bind(object_id)
        .bind(fee)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        txn.commit().await.map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(Transaction {
            tx_id,
            timestamp: now,
            from_agent: Some(owner_id.into()),
            to_agent: Some(storage_provider_id.into()),
            tx_type: "storage".into(),
            amount_mc,
            credit_type: "hard".into(),
            model_name: None,
            tokens_in: None,
            tokens_out: Some(size_bytes),
            job_id: Some(object_id.into()),
            platform_fee_mc: fee,
            status: "completed".into(),
        })
    }

    /// Get recent transaction history for an agent.
    pub async fn get_history(&self, agent_id: &str, limit: usize) -> Result<Vec<Transaction>> {
        let rows = sqlx::query(
            "SELECT tx_id, timestamp, from_agent, to_agent, tx_type, amount_mc,
                    credit_type, model_name, tokens_in, tokens_out, job_id,
                    platform_fee_mc, status
             FROM transactions
             WHERE from_agent = ?1 OR to_agent = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )
        .bind(agent_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(rows.iter().map(row_to_transaction).collect())
    }

    /// Place credits in escrow for a job.
    ///
    /// The balance check and debit are wrapped in a single SQLite transaction
    /// so the read-then-write is atomic.
    pub async fn hold_escrow(
        &self,
        job_id: &str,
        buyer_id: &str,
        seller_id: &str,
        amount_mc: i64,
        ttl_secs: i64,
    ) -> Result<EscrowEntry> {
        let now = unix_timestamp();
        let escrow_id = Uuid::now_v7().to_string();

        let mut txn = self.pool.begin().await.map_err(|e| FerrisError::Database(e.to_string()))?;

        // Check balance
        let row =
            sqlx::query("SELECT soft_balance_mc, hard_balance_mc FROM credits WHERE agent_id = ?")
                .bind(buyer_id)
                .fetch_optional(&mut *txn)
                .await
                .map_err(|e| FerrisError::Database(e.to_string()))?
                .ok_or_else(|| FerrisError::NotFound(format!("credits for agent: {buyer_id}")))?;

        let soft_balance: i64 = Row::get(&row, "soft_balance_mc");
        let hard_balance: i64 = Row::get(&row, "hard_balance_mc");
        let total_available = soft_balance + hard_balance;
        if total_available < amount_mc {
            return Err(FerrisError::InsufficientCredits(format!(
                "escrow requires {amount_mc} mc, have {total_available} mc"
            )));
        }

        // Debit from soft first, then hard
        let soft_debit = amount_mc.min(soft_balance);
        let hard_debit = amount_mc - soft_debit;

        sqlx::query(
            "UPDATE credits SET soft_balance_mc = soft_balance_mc - ?,
                                hard_balance_mc = hard_balance_mc - ?
             WHERE agent_id = ?",
        )
        .bind(soft_debit)
        .bind(hard_debit)
        .bind(buyer_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        sqlx::query(
            "INSERT INTO escrow (escrow_id, job_id, buyer_agent, seller_agent, amount_mc, created_at, expires_at, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, 'held')",
        )
        .bind(&escrow_id)
        .bind(job_id)
        .bind(buyer_id)
        .bind(seller_id)
        .bind(amount_mc)
        .bind(now)
        .bind(now + ttl_secs)
        .execute(&mut *txn)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        txn.commit().await.map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(EscrowEntry {
            escrow_id,
            job_id: job_id.into(),
            buyer_agent: buyer_id.into(),
            seller_agent: seller_id.into(),
            amount_mc,
            created_at: now,
            expires_at: now + ttl_secs,
            status: "held".into(),
        })
    }

    /// Release escrow to seller (minus platform fee).
    pub async fn release_escrow(&self, escrow_id: &str) -> Result<()> {
        let row = sqlx::query(
            "SELECT escrow_id, job_id, buyer_agent, seller_agent, amount_mc, status
             FROM escrow WHERE escrow_id = ? AND status = 'held'",
        )
        .bind(escrow_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .ok_or_else(|| FerrisError::NotFound(format!("held escrow: {escrow_id}")))?;

        let amount_mc: i64 = row.get("amount_mc");
        let seller_id: String = row.get("seller_agent");
        let fee = platform_fee(amount_mc);
        let payout = amount_mc - fee;

        // Credit seller
        sqlx::query(
            "UPDATE credits SET hard_balance_mc = hard_balance_mc + ?,
                                total_earned_hard_mc = total_earned_hard_mc + ?
             WHERE agent_id = ?",
        )
        .bind(payout)
        .bind(payout)
        .bind(&seller_id)
        .execute(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        // Mark released
        sqlx::query("UPDATE escrow SET status = 'released' WHERE escrow_id = ?")
            .bind(escrow_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(())
    }

    /// Refund escrow back to buyer.
    pub async fn refund_escrow(&self, escrow_id: &str) -> Result<()> {
        let row = sqlx::query(
            "SELECT buyer_agent, amount_mc FROM escrow WHERE escrow_id = ? AND status = 'held'",
        )
        .bind(escrow_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?
        .ok_or_else(|| FerrisError::NotFound(format!("held escrow: {escrow_id}")))?;

        let buyer_id: String = row.get("buyer_agent");
        let amount_mc: i64 = row.get("amount_mc");

        // Refund to buyer (hard balance since we don't track original type)
        sqlx::query("UPDATE credits SET hard_balance_mc = hard_balance_mc + ? WHERE agent_id = ?")
            .bind(amount_mc)
            .bind(&buyer_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        sqlx::query("UPDATE escrow SET status = 'refunded' WHERE escrow_id = ?")
            .bind(escrow_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(())
    }
}

fn row_to_transaction(row: &sqlx::sqlite::SqliteRow) -> Transaction {
    Transaction {
        tx_id: row.get("tx_id"),
        timestamp: row.get("timestamp"),
        from_agent: row.get("from_agent"),
        to_agent: row.get("to_agent"),
        tx_type: row.get("tx_type"),
        amount_mc: row.get("amount_mc"),
        credit_type: row.get("credit_type"),
        model_name: row.get("model_name"),
        tokens_in: row.get("tokens_in"),
        tokens_out: row.get("tokens_out"),
        job_id: row.get("job_id"),
        platform_fee_mc: row.get("platform_fee_mc"),
        status: row.get("status"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn run_schema(pool: &SqlitePool) {
        let ddl = [
            "CREATE TABLE IF NOT EXISTS agents (
                agent_id TEXT PRIMARY KEY, public_key BLOB NOT NULL,
                created_at INTEGER NOT NULL, last_heartbeat INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'active', reputation REAL NOT NULL DEFAULT 50.0,
                tier TEXT NOT NULL DEFAULT 'new', gpu_model TEXT, gpu_vram_mb INTEGER,
                cpu_cores INTEGER NOT NULL, ram_mb INTEGER NOT NULL,
                storage_avail_mb INTEGER NOT NULL, bandwidth_mbps REAL,
                contribute_gpu INTEGER NOT NULL DEFAULT 0, contribute_storage INTEGER NOT NULL DEFAULT 0,
                contribute_cpu INTEGER NOT NULL DEFAULT 0, max_concurrent_req INTEGER NOT NULL DEFAULT 4,
                current_requests INTEGER NOT NULL DEFAULT 0, endpoint_url TEXT, nat_type TEXT, region TEXT
            )",
            "CREATE TABLE IF NOT EXISTS credits (
                agent_id TEXT PRIMARY KEY REFERENCES agents(agent_id),
                soft_balance_mc INTEGER NOT NULL DEFAULT 0, hard_balance_mc INTEGER NOT NULL DEFAULT 0,
                total_earned_soft_mc INTEGER NOT NULL DEFAULT 0, total_earned_hard_mc INTEGER NOT NULL DEFAULT 0,
                total_spent_mc INTEGER NOT NULL DEFAULT 0, total_cashed_out_mc INTEGER NOT NULL DEFAULT 0
            )",
            "CREATE TABLE IF NOT EXISTS transactions (
                tx_id TEXT PRIMARY KEY, timestamp INTEGER NOT NULL,
                from_agent TEXT, to_agent TEXT, tx_type TEXT NOT NULL,
                amount_mc INTEGER NOT NULL, credit_type TEXT NOT NULL,
                model_name TEXT, tokens_in INTEGER, tokens_out INTEGER,
                job_id TEXT, platform_fee_mc INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'completed'
            )",
            "CREATE TABLE IF NOT EXISTS escrow (
                escrow_id TEXT PRIMARY KEY, job_id TEXT NOT NULL,
                buyer_agent TEXT NOT NULL, seller_agent TEXT NOT NULL,
                amount_mc INTEGER NOT NULL, created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'held'
            )",
        ];
        for sql in ddl {
            sqlx::query(sql).execute(pool).await.unwrap();
        }
    }

    async fn setup() -> (CreditLedger, SqlitePool) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(":memory:").create_if_missing(true))
            .await
            .unwrap();
        run_schema(&pool).await;

        // Insert test agents
        let now = unix_timestamp();
        for id in &["agent-a", "agent-b"] {
            sqlx::query(
                "INSERT INTO agents (agent_id, public_key, created_at, last_heartbeat, cpu_cores, ram_mb, storage_avail_mb)
                 VALUES (?, X'00', ?, ?, 4, 8192, 50000)",
            )
            .bind(id)
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
        }

        let ledger = CreditLedger::new(pool.clone());
        (ledger, pool)
    }

    #[tokio::test]
    async fn create_account_gives_signup_bonus() {
        let (ledger, _pool) = setup().await;
        let balance = ledger.create_account("agent-a").await.unwrap();
        assert_eq!(balance.soft_balance_mc, SIGNUP_BONUS_MC);
        assert_eq!(balance.hard_balance_mc, 0);
        assert_eq!(balance.total_earned_soft_mc, SIGNUP_BONUS_MC);
    }

    #[tokio::test]
    async fn availability_awards_soft_credits() {
        let (ledger, _pool) = setup().await;
        ledger.create_account("agent-a").await.unwrap();
        ledger.award_availability("agent-a", 1000).await.unwrap();

        let balance = ledger.get_balance("agent-a").await.unwrap();
        assert_eq!(balance.soft_balance_mc, SIGNUP_BONUS_MC + 1000);
    }

    #[tokio::test]
    async fn settle_inference_debits_consumer_credits_provider() {
        let (ledger, _pool) = setup().await;
        ledger.create_account("agent-a").await.unwrap();
        ledger.create_account("agent-b").await.unwrap();

        let amount = 10_000; // 10 credits
        let fee = platform_fee(amount);
        let tx = ledger
            .settle_inference("agent-a", "agent-b", amount, "llama3", 100, 200, "job-1")
            .await
            .unwrap();

        assert_eq!(tx.platform_fee_mc, fee);
        assert_eq!(tx.tx_type, "inference");

        let a_bal = ledger.get_balance("agent-a").await.unwrap();
        assert_eq!(a_bal.soft_balance_mc, SIGNUP_BONUS_MC - amount);
        assert_eq!(a_bal.total_spent_mc, amount);

        let b_bal = ledger.get_balance("agent-b").await.unwrap();
        assert_eq!(b_bal.hard_balance_mc, amount - fee);
        assert_eq!(b_bal.total_earned_hard_mc, amount - fee);
    }

    #[tokio::test]
    async fn insufficient_credits_rejected() {
        let (ledger, _pool) = setup().await;
        ledger.create_account("agent-a").await.unwrap();

        let err = ledger
            .settle_inference(
                "agent-a",
                "agent-b",
                SIGNUP_BONUS_MC + 1,
                "llama3",
                100,
                200,
                "job-x",
            )
            .await
            .unwrap_err();
        assert!(matches!(err, FerrisError::InsufficientCredits(_)));
    }

    #[tokio::test]
    async fn escrow_hold_and_release() {
        let (ledger, _pool) = setup().await;
        ledger.create_account("agent-a").await.unwrap();
        ledger.create_account("agent-b").await.unwrap();

        let escrow = ledger.hold_escrow("job-1", "agent-a", "agent-b", 5000, 300).await.unwrap();
        assert_eq!(escrow.status, "held");

        let a_bal = ledger.get_balance("agent-a").await.unwrap();
        assert_eq!(a_bal.soft_balance_mc, SIGNUP_BONUS_MC - 5000);

        ledger.release_escrow(&escrow.escrow_id).await.unwrap();
        let b_bal = ledger.get_balance("agent-b").await.unwrap();
        let expected_payout = 5000 - platform_fee(5000);
        assert_eq!(b_bal.hard_balance_mc, expected_payout);
    }

    #[tokio::test]
    async fn escrow_refund_returns_to_buyer() {
        let (ledger, _pool) = setup().await;
        ledger.create_account("agent-a").await.unwrap();
        ledger.create_account("agent-b").await.unwrap();

        let escrow = ledger.hold_escrow("job-2", "agent-a", "agent-b", 3000, 300).await.unwrap();

        ledger.refund_escrow(&escrow.escrow_id).await.unwrap();
        let a_bal = ledger.get_balance("agent-a").await.unwrap();
        // Refund goes to hard balance
        assert_eq!(a_bal.soft_balance_mc + a_bal.hard_balance_mc, SIGNUP_BONUS_MC);
    }

    #[tokio::test]
    async fn transaction_history() {
        let (ledger, _pool) = setup().await;
        ledger.create_account("agent-a").await.unwrap();
        ledger.award_availability("agent-a", 500).await.unwrap();

        let history = ledger.get_history("agent-a", 10).await.unwrap();
        assert_eq!(history.len(), 2); // signup + availability
    }
}
