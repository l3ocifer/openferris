use ferris_common::{FerrisError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

// ── Routing Types ───────────────────────────────────────────────────────

/// A scored provider candidate for routing an inference request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCandidate {
    pub agent_id: String,
    pub endpoint_url: String,
    pub model_name: String,
    pub score: f64,
    pub reputation: f64,
    pub avg_tokens_sec: f64,
    pub current_load: f64,
    pub is_hot: bool,
    pub region: Option<String>,
}

/// Available model across the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkModel {
    pub id: String,
    pub object: String,
    pub owned_by: String,
    pub providers: u32,
}

// ── Inference Router ────────────────────────────────────────────────────

/// Routes inference requests to the best available provider node.
pub struct InferenceRouter {
    pool: SqlitePool,
}

impl InferenceRouter {
    /// Create a new inference router backed by the given database pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Score and rank all candidates for a model.
    pub async fn score_candidates(
        &self,
        model_name: &str,
        requester_region: Option<&str>,
    ) -> Result<Vec<RouteCandidate>> {
        let rows = sqlx::query(
            "SELECT a.agent_id, a.endpoint_url, a.reputation,
                    a.current_requests, a.max_concurrent_req, a.region,
                    m.model_name, m.is_hot, m.avg_tokens_sec
             FROM agents a
             JOIN models m ON a.agent_id = m.agent_id
             WHERE m.model_name = ? AND a.status = 'active'
               AND a.endpoint_url IS NOT NULL
               AND a.current_requests < a.max_concurrent_req
             ORDER BY a.reputation DESC",
        )
        .bind(model_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        let mut candidates: Vec<RouteCandidate> = rows
            .iter()
            .map(|row| {
                let reputation: f64 = row.get("reputation");
                let avg_tokens_sec: Option<f64> = row.get("avg_tokens_sec");
                let current_req: i64 = row.get("current_requests");
                let max_req: i64 = row.get("max_concurrent_req");
                let is_hot: i32 = row.get("is_hot");
                let region: Option<String> = row.get("region");

                let score = compute_score(
                    reputation,
                    avg_tokens_sec.unwrap_or(0.0),
                    &region,
                    requester_region,
                    current_req as f64 / max_req.max(1) as f64,
                    is_hot != 0,
                );

                RouteCandidate {
                    agent_id: row.get("agent_id"),
                    endpoint_url: row.get::<String, _>("endpoint_url"),
                    model_name: row.get("model_name"),
                    score,
                    reputation,
                    avg_tokens_sec: avg_tokens_sec.unwrap_or(0.0),
                    current_load: current_req as f64 / max_req.max(1) as f64,
                    is_hot: is_hot != 0,
                    region,
                }
            })
            .collect();

        candidates
            .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(candidates)
    }

    /// List all unique models available across the network.
    pub async fn list_models(&self) -> Result<Vec<NetworkModel>> {
        let rows = sqlx::query(
            "SELECT m.model_name, COUNT(DISTINCT m.agent_id) as providers
             FROM models m
             JOIN agents a ON m.agent_id = a.agent_id
             WHERE a.status = 'active'
             GROUP BY m.model_name
             ORDER BY providers DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerrisError::Database(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| {
                let name: String = row.get("model_name");
                let providers: i64 = row.get::<i64, _>("providers");
                NetworkModel {
                    id: name.clone(),
                    object: "model".into(),
                    owned_by: "openferris-network".into(),
                    providers: providers as u32,
                }
            })
            .collect())
    }
}

// ── Scoring Algorithm (spec-v1.md Section 5) ────────────────────────────

/// Canonical routing score.
///
/// ```text
/// score = reputation_norm * 0.40
///       + speed_norm      * 0.25
///       + latency_norm    * 0.20
///       + availability    * 0.15
///       + hot_bonus
/// ```
fn compute_score(
    reputation: f64,
    tokens_per_sec: f64,
    agent_region: &Option<String>,
    requester_region: Option<&str>,
    current_load: f64,
    is_hot: bool,
) -> f64 {
    let reputation_norm = (reputation / 100.0).clamp(0.0, 1.0);
    let speed_norm = (tokens_per_sec / 100.0).clamp(0.0, 1.0);

    let latency_norm = match (agent_region.as_deref(), requester_region) {
        (Some(a), Some(r)) if a == r => 1.0,
        (Some(_), Some(_)) => 0.5,
        _ => 0.7, // unknown region — neutral
    };

    let availability = (1.0 - current_load).max(0.0);
    let hot_bonus = if is_hot { 0.10 } else { 0.0 };

    reputation_norm * 0.40
        + speed_norm * 0.25
        + latency_norm * 0.20
        + availability * 0.15
        + hot_bonus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_perfect_agent() {
        let score =
            compute_score(100.0, 100.0, &Some("us-east".into()), Some("us-east"), 0.0, true);
        // 0.40 + 0.25 + 0.20 + 0.15 + 0.10 = 1.10
        assert!((score - 1.10).abs() < 0.001);
    }

    #[test]
    fn score_new_agent() {
        let score = compute_score(50.0, 20.0, &None, None, 0.5, false);
        // 0.20 + 0.05 + 0.14 + 0.075 + 0.0 = 0.465
        let expected = 0.50 * 0.40 + 0.20 * 0.25 + 0.70 * 0.20 + 0.50 * 0.15;
        assert!((score - expected).abs() < 0.001);
    }

    #[test]
    fn hot_bonus_applied() {
        let cold = compute_score(50.0, 50.0, &None, None, 0.0, false);
        let hot = compute_score(50.0, 50.0, &None, None, 0.0, true);
        assert!((hot - cold - 0.10).abs() < 0.001);
    }

    #[test]
    fn same_region_beats_different_region() {
        let same = compute_score(50.0, 50.0, &Some("us".into()), Some("us"), 0.0, false);
        let diff = compute_score(50.0, 50.0, &Some("eu".into()), Some("us"), 0.0, false);
        assert!(same > diff);
    }
}
