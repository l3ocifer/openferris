use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use ferris_common::{
    HeartbeatRequest, ModelInfo, RegisterRequest, ResourceManifest, SIGNUP_BONUS_MC,
};
use ferris_coordinator::registry::AgentRegistry;
use ferris_coordinator::router::InferenceRouter;
use ferris_coordinator::routes::{build_coordinator_app, AppState};
use ferris_coordinator::storage_router::StorageRouter;
use ferris_credits::CreditLedger;
use http_body_util::BodyExt;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn setup() -> (axum::Router, SqlitePool) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .create_if_missing(true),
        )
        .await
        .unwrap();

    run_schema(&pool).await;

    let ledger = CreditLedger::new(pool.clone());
    let registry = Arc::new(AgentRegistry::new(pool.clone(), ledger));
    let router = Arc::new(InferenceRouter::new(pool.clone()));
    let storage_router = Arc::new(StorageRouter::new(pool.clone()));

    let state = AppState { registry, router, storage_router };
    let app = build_coordinator_app(state);
    (app, pool)
}

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
        "CREATE TABLE IF NOT EXISTS models (
            agent_id TEXT NOT NULL REFERENCES agents(agent_id),
            model_name TEXT NOT NULL, model_family TEXT, parameter_count_b REAL,
            quantization TEXT, is_hot INTEGER NOT NULL DEFAULT 0, avg_tokens_sec REAL,
            last_verified INTEGER, PRIMARY KEY (agent_id, model_name)
        )",
        "CREATE TABLE IF NOT EXISTS capabilities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id TEXT NOT NULL REFERENCES agents(agent_id),
            capability TEXT NOT NULL, description TEXT, price_millicredits INTEGER,
            avg_rating REAL NOT NULL DEFAULT 0.0, total_jobs INTEGER NOT NULL DEFAULT 0,
            UNIQUE(agent_id, capability)
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
        "CREATE TABLE IF NOT EXISTS network_objects (
            object_id TEXT PRIMARY KEY,
            owner_agent TEXT NOT NULL REFERENCES agents(agent_id),
            storage_agent TEXT NOT NULL REFERENCES agents(agent_id),
            name TEXT NOT NULL, size_bytes INTEGER NOT NULL,
            content_hash TEXT NOT NULL, created_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'active'
        )",
    ];
    for sql in ddl {
        sqlx::query(sql).execute(pool).await.unwrap();
    }
}

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn signed_json_request(
    method: Method,
    uri: &str,
    body: serde_json::Value,
    agent_id: &str,
    signing_key: &SigningKey,
) -> Request<Body> {
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let signature = signing_key.sign(&body_bytes);
    let sig_b64 = STANDARD.encode(signature.to_bytes());
    Request::builder()
        .method(method)
        .uri(uri)
        .header("Content-Type", "application/json")
        .header("X-Agent-Id", agent_id)
        .header("X-Signature", sig_b64)
        .body(Body::from(body_bytes))
        .unwrap()
}

async fn json_body(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn test_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[1u8; 32])
}

fn test_register_request(agent_id: &str) -> RegisterRequest {
    let key = test_signing_key();
    let public_key = key.verifying_key().to_bytes().to_vec();
    RegisterRequest {
        agent_id: agent_id.into(),
        public_key,
        resources: ResourceManifest {
            cpu_cores: 8,
            ram_mb: 16384,
            storage_avail_mb: 100000,
            gpu: None,
        },
        models: vec![ModelInfo {
            model_name: "llama3:8b".into(),
            model_family: Some("llama".into()),
            parameter_count_b: Some(8.0),
            quantization: Some("Q4_K_M".into()),
            is_hot: true,
            avg_tokens_sec: Some(45.0),
        }],
        contribute_gpu: true,
        contribute_storage: true,
        contribute_cpu: true,
        max_concurrent_requests: 4,
        endpoint_url: Some("http://localhost:11434".into()),
        region: Some("us-east".into()),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn health_endpoint() {
    let (app, _pool) = setup().await;
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn register_new_agent() {
    let (app, _pool) = setup().await;
    let req = test_register_request("agent-test-1");
    let resp = app
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&req).unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["accepted"], true);
    assert_eq!(body["signup_bonus_mc"], SIGNUP_BONUS_MC);
}

#[tokio::test]
async fn register_idempotent() {
    let (app, _pool) = setup().await;
    let req = test_register_request("agent-idem");

    let resp1 = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&req).unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(json_body(resp1).await["signup_bonus_mc"], SIGNUP_BONUS_MC);

    let resp2 = app
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&req).unwrap(),
        ))
        .await
        .unwrap();
    // Second registration gives 0 bonus
    assert_eq!(json_body(resp2).await["signup_bonus_mc"], 0);
}

#[tokio::test]
async fn heartbeat_updates_status() {
    let (app, _pool) = setup().await;
    let signing_key = test_signing_key();
    let reg = test_register_request("agent-hb");

    app.clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&reg).unwrap(),
        ))
        .await
        .unwrap();

    let hb = HeartbeatRequest {
        agent_id: "agent-hb".into(),
        resources: reg.resources.clone(),
        models: reg.models.clone(),
        current_requests: 1,
    };

    let resp = app
        .oneshot(signed_json_request(
            Method::POST,
            "/api/v1/heartbeat",
            serde_json::to_value(&hb).unwrap(),
            "agent-hb",
            &signing_key,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn heartbeat_unregistered_returns_unauthorized() {
    let (app, _pool) = setup().await;
    let signing_key = test_signing_key();
    let hb = HeartbeatRequest {
        agent_id: "nonexistent".into(),
        resources: ResourceManifest {
            cpu_cores: 4,
            ram_mb: 8192,
            storage_avail_mb: 50000,
            gpu: None,
        },
        models: vec![],
        current_requests: 0,
    };

    let resp = app
        .oneshot(signed_json_request(
            Method::POST,
            "/api/v1/heartbeat",
            serde_json::to_value(&hb).unwrap(),
            "nonexistent",
            &signing_key,
        ))
        .await
        .unwrap();
    // Agent doesn't exist, so signature verification returns 401
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wallet_balance_after_registration() {
    let (app, _pool) = setup().await;
    let reg = test_register_request("agent-wal");

    app.clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&reg).unwrap(),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::get("/api/v1/wallet/balance?agent_id=agent-wal")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["soft_balance_mc"], SIGNUP_BONUS_MC);
}

#[tokio::test]
async fn list_models_after_registration() {
    let (app, _pool) = setup().await;
    let reg = test_register_request("agent-mod");

    app.clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&reg).unwrap(),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(Request::get("/v1/models").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let models = body["data"].as_array().unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["id"], "llama3:8b");
}

#[tokio::test]
async fn coordinator_status_counts() {
    let (app, _pool) = setup().await;
    let reg = test_register_request("agent-st");

    app.clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&reg).unwrap(),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::get("/api/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = json_body(resp).await;
    assert_eq!(body["active_agents"], 1);
    assert_eq!(body["available_models"], 1);
}

#[tokio::test]
async fn directory_lists_active_agents() {
    let (app, _pool) = setup().await;
    let reg = test_register_request("agent-dir");

    app.clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&reg).unwrap(),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::get("/api/v1/directory")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let entries = body.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["agent_id"], "agent-dir");
    assert_eq!(entries[0]["status"], "active");
    assert!(entries[0]["reputation"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn dashboard_stats_endpoint() {
    let (app, _pool) = setup().await;
    let reg = test_register_request("agent-dash");

    app.clone()
        .oneshot(json_request(
            Method::POST,
            "/api/v1/register",
            serde_json::to_value(&reg).unwrap(),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::get("/dashboard/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["active_agents"], 1);
    // Registration creates a signup_bonus transaction
    assert!(body["total_transactions"].as_i64().unwrap() >= 1);
}
