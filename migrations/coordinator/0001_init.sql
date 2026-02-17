-- OpenFerris coordinator schema (Phase 2)

CREATE TABLE IF NOT EXISTS agents (
    agent_id            TEXT PRIMARY KEY,
    public_key          BLOB NOT NULL,
    created_at          INTEGER NOT NULL,
    last_heartbeat      INTEGER NOT NULL,
    status              TEXT NOT NULL DEFAULT 'active',
    reputation          REAL NOT NULL DEFAULT 50.0,
    tier                TEXT NOT NULL DEFAULT 'new',

    gpu_model           TEXT,
    gpu_vram_mb         INTEGER,
    cpu_cores           INTEGER NOT NULL,
    ram_mb              INTEGER NOT NULL,
    storage_avail_mb    INTEGER NOT NULL,
    bandwidth_mbps      REAL,

    contribute_gpu      INTEGER NOT NULL DEFAULT 0,
    contribute_storage  INTEGER NOT NULL DEFAULT 0,
    contribute_cpu      INTEGER NOT NULL DEFAULT 0,
    max_concurrent_req  INTEGER NOT NULL DEFAULT 4,
    current_requests    INTEGER NOT NULL DEFAULT 0,

    endpoint_url        TEXT,
    nat_type            TEXT,
    region              TEXT
);

CREATE TABLE IF NOT EXISTS models (
    agent_id            TEXT NOT NULL REFERENCES agents(agent_id),
    model_name          TEXT NOT NULL,
    model_family        TEXT,
    parameter_count_b   REAL,
    quantization        TEXT,
    is_hot              INTEGER NOT NULL DEFAULT 0,
    avg_tokens_sec      REAL,
    last_verified       INTEGER,
    PRIMARY KEY (agent_id, model_name)
);

CREATE TABLE IF NOT EXISTS capabilities (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id            TEXT NOT NULL REFERENCES agents(agent_id),
    capability          TEXT NOT NULL,
    description         TEXT,
    price_millicredits  INTEGER,
    avg_rating          REAL NOT NULL DEFAULT 0.0,
    total_jobs          INTEGER NOT NULL DEFAULT 0,
    UNIQUE(agent_id, capability)
);

CREATE TABLE IF NOT EXISTS credits (
    agent_id                TEXT PRIMARY KEY REFERENCES agents(agent_id),
    soft_balance_mc         INTEGER NOT NULL DEFAULT 0,
    hard_balance_mc         INTEGER NOT NULL DEFAULT 0,
    total_earned_soft_mc    INTEGER NOT NULL DEFAULT 0,
    total_earned_hard_mc    INTEGER NOT NULL DEFAULT 0,
    total_spent_mc          INTEGER NOT NULL DEFAULT 0,
    total_cashed_out_mc     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS transactions (
    tx_id               TEXT PRIMARY KEY,
    timestamp           INTEGER NOT NULL,
    from_agent          TEXT REFERENCES agents(agent_id),
    to_agent            TEXT REFERENCES agents(agent_id),
    tx_type             TEXT NOT NULL,
    amount_mc           INTEGER NOT NULL,
    credit_type         TEXT NOT NULL,
    model_name          TEXT,
    tokens_in           INTEGER,
    tokens_out          INTEGER,
    job_id              TEXT,
    platform_fee_mc     INTEGER NOT NULL DEFAULT 0,
    status              TEXT NOT NULL DEFAULT 'completed'
);

CREATE TABLE IF NOT EXISTS escrow (
    escrow_id           TEXT PRIMARY KEY,
    job_id              TEXT NOT NULL,
    buyer_agent         TEXT NOT NULL REFERENCES agents(agent_id),
    seller_agent        TEXT NOT NULL REFERENCES agents(agent_id),
    amount_mc           INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    expires_at          INTEGER NOT NULL,
    status              TEXT NOT NULL DEFAULT 'held'
);

CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_models_name ON models(model_name);
CREATE INDEX IF NOT EXISTS idx_models_hot ON models(is_hot) WHERE is_hot = 1;
CREATE INDEX IF NOT EXISTS idx_transactions_from ON transactions(from_agent);
CREATE INDEX IF NOT EXISTS idx_transactions_to ON transactions(to_agent);
CREATE INDEX IF NOT EXISTS idx_transactions_job ON transactions(job_id);
CREATE INDEX IF NOT EXISTS idx_escrow_buyer ON escrow(buyer_agent);
CREATE INDEX IF NOT EXISTS idx_escrow_status ON escrow(status);
