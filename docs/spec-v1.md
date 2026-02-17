# OpenFerris Canonical Spec v1

Version: 1.0  
Status: Locked for implementation bootstrap

This is the single source of truth for implementation decisions when other docs conflict.

Interoperability policy is defined in `docs/agent-interoperability.md` and is part of this canonical contract.

## 1) Phase Scope

### Phase 1 (local-only)
- `ferris init`
- local memory (`remember`, `recall`, `forget`)
- local storage (`store`, `retrieve`, `list_files`)
- local task scheduler (`schedule_task`, `list_tasks`, `cancel_task`)
- MCP server exposing local tools
- interoperability baseline for bring-your-own-agent integration

### Phase 2 (network/coordinator starts)
- `ferris-coordinator` service (Axum + SQLite)
- node registration + heartbeat
- inference routing + OpenAI-compatible endpoint
- soft/hard credit accounting and settlement

### Phase 3+
- direct streaming broker mode
- directory hiring escrow flow
- broader integrations (S3 endpoint, GH Actions, etc.)

## 2) Canonical Decisions (Conflict Resolution)

1. Platform fee: fixed `15%` in Phases 2-3.
2. Bootstrap credit policy: keep both:
   - one-time signup bonus: `100` soft credits
   - ongoing availability rewards (soft credits) on 60s cadence
3. Routing algorithm: canonical score uses reputation/speed/latency/load with hot-model bonus.
4. Reputation scale: `0.0..100.0` (stored as `REAL`).
5. Storage posture (Phases 1-3): local object store + optional R2 sync; no multi-node replicated storage.
6. Transport:
   - Phase 2 default: HTTPS + SSE proxy via coordinator
   - Phase 3+: optional direct stream mode with coordinator-issued ticket
7. API namespaces:
   - OpenAI-compatible consumer endpoints: `/v1/*`
   - node/coordinator control endpoints: `/api/v1/*`
   - dashboard endpoints: `/dashboard/*`
8. Timeline convention: week-based roadmap only.

## 3) Canonical Identity + Data Model

### Identity
- `agent_id`: UUID v7 string
- signing key: Ed25519 keypair
- public key persisted and associated with agent

### Time
- canonical DB time format: `INTEGER` Unix timestamp (seconds)

### Money/Credits
- canonical type in service logic: fixed-point integer (`millicredits`)
- SQL storage: `INTEGER` amount fields representing millicredits
- display conversion: `credits = millicredits / 1000.0`

## 4) Coordinator Schema (Canonical Baseline)

```sql
CREATE TABLE agents (
    agent_id            TEXT PRIMARY KEY,       -- UUID v7
    public_key          BLOB NOT NULL,          -- Ed25519 pubkey
    created_at          INTEGER NOT NULL,
    last_heartbeat      INTEGER NOT NULL,
    status              TEXT NOT NULL DEFAULT 'active', -- active|suspended|banned
    reputation          REAL NOT NULL DEFAULT 0.0,      -- 0..100
    tier                TEXT NOT NULL DEFAULT 'new',    -- new|member|reliable|verified

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

CREATE TABLE models (
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

CREATE TABLE capabilities (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id            TEXT NOT NULL REFERENCES agents(agent_id),
    capability          TEXT NOT NULL,
    description         TEXT,
    description_emb     BLOB,
    price_millicredits  INTEGER,
    avg_rating          REAL NOT NULL DEFAULT 0.0,
    total_jobs          INTEGER NOT NULL DEFAULT 0,
    UNIQUE(agent_id, capability)
);

CREATE TABLE credits (
    agent_id                TEXT PRIMARY KEY REFERENCES agents(agent_id),
    soft_balance_mc         INTEGER NOT NULL DEFAULT 0,
    hard_balance_mc         INTEGER NOT NULL DEFAULT 0,
    total_earned_soft_mc    INTEGER NOT NULL DEFAULT 0,
    total_earned_hard_mc    INTEGER NOT NULL DEFAULT 0,
    total_spent_mc          INTEGER NOT NULL DEFAULT 0,
    total_cashed_out_mc     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE transactions (
    tx_id               TEXT PRIMARY KEY,       -- UUID v7
    timestamp           INTEGER NOT NULL,
    from_agent          TEXT REFERENCES agents(agent_id),
    to_agent            TEXT REFERENCES agents(agent_id),
    tx_type             TEXT NOT NULL,          -- availability|inference|storage|hire|spend|cashout
    amount_mc           INTEGER NOT NULL,
    credit_type         TEXT NOT NULL,          -- soft|hard
    model_name          TEXT,
    tokens_in           INTEGER,
    tokens_out          INTEGER,
    job_id              TEXT,
    platform_fee_mc     INTEGER NOT NULL DEFAULT 0,
    status              TEXT NOT NULL DEFAULT 'completed'
);

CREATE TABLE escrow (
    escrow_id           TEXT PRIMARY KEY,
    job_id              TEXT NOT NULL,
    buyer_agent         TEXT NOT NULL REFERENCES agents(agent_id),
    seller_agent        TEXT NOT NULL REFERENCES agents(agent_id),
    amount_mc           INTEGER NOT NULL,
    created_at          INTEGER NOT NULL,
    expires_at          INTEGER NOT NULL,
    status              TEXT NOT NULL DEFAULT 'held'
);
```

## 5) Canonical Routing Algorithm

```rust
score = reputation_norm * 0.40
      + speed_norm      * 0.25
      + latency_norm    * 0.20
      + availability    * 0.15
      + hot_bonus

hot_bonus = if is_hot { 0.10 } else { 0.0 }
```

Where:
- `reputation_norm = reputation / 100.0`
- `speed_norm = min(tokens_per_sec / 100.0, 1.0)`
- `latency_norm = 1.0` if same region, else `max((200 - latency_ms)/200, 0.0)`
- `availability = 1.0 - current_load`

Selection:
- High priority: choose top score.
- Normal priority: weighted random among top 3.

## 6) Canonical API Surface (Phase 2 Baseline)

### Node API (`/api/v1`)
- `POST /api/v1/register`
- `POST /api/v1/heartbeat`
- `POST /api/v1/contribute`
- `GET /api/v1/status`
- `GET /api/v1/wallet/balance`
- `GET /api/v1/wallet/history`

### Consumer API
- `POST /v1/chat/completions`
- `POST /v1/completions`
- `GET /v1/models`

### Dashboard API (`/dashboard`)
- `GET /dashboard/overview`
- `GET /dashboard/agent/:id`
- `GET /dashboard/earnings`

## 7) Security Baseline (Implementation)

- All node->coordinator calls are signed with Ed25519.
- Coordinator requires signature verification for all `/api/v1/*` node endpoints.
- Inference payload privacy is best-effort (not cryptographically private from serving node).
- Token accounting trust model:
  - Phase 2: trust but verify (sampling + statistical checks).
  - Penalties applied via reputation on repeated mismatch/failure.

## 8) Workspace Baseline (Phase 1)

Must exist before feature coding:
- Rust workspace with crates:
  - `ferris-common`
  - `ferris-core`
  - `ferris-memory`
  - `ferris-storage`
  - `ferris-tasks`
  - `ferris-mcp`
- CI running `cargo check`, `cargo clippy -D warnings`, `cargo test`.
- Initial SQL migration for node-local DB.

## 9) Runtime Boundaries (No Redundancy)

1. OpenFerris is agent-runtime-agnostic.
2. Ironclaw is a reference profile, not a required runtime dependency.
3. Agent runtime behaviors (planning loop, persona orchestration, session UX) are out of scope for OpenFerris core unless needed for protocol compatibility.
4. Prefer adapter/contract integration over agent-runtime reimplementation.

## 10) Delivery Risk Posture

Implementation priorities and stage-gate constraints are governed by:
1. `docs/gap-analysis.md`
2. `docs/launch-plan.md`

If roadmap ambition conflicts with validated demand or onboarding evidence, validation wins.

## 11) Mobile Supply Alignment

### 11.1 Strategic Position

1. Mobile contribution is the primary supply growth engine, not a secondary curiosity.
2. Mobile execution must follow `docs/mobile-supply.md` phase gates.
3. Mobile must NOT block Phase 1/2 core delivery. Android app MVP targets Week 4-6 (after coordinator exists).
4. Android-first. iOS comes later with constrained tiers.

### 11.2 Canonical Contribution Tiers

The `node_type` and `mobile_tier` fields on the `agents` table determine what work a node can receive:

| Tier | Name | Node Types | Capabilities | Credit Rate |
|------|------|-----------|--------------|-------------|
| 1 | Storage Node | Any phone, any desktop | Encrypted vector shard storage, similarity search | 1 credit / 1000 queries |
| 2 | Embedding Node | 2022+ phone, any desktop | Tier 1 + text embedding generation | 2 credits / 1000 embeddings |
| 3 | Verification Node | 2023+ phone, any desktop | Tier 1-2 + async inference verification | 5 credits / 100 verifications |
| 4 | Inference Node (Small) | 2024+ flagship phone, 2025+ mid-range | Tier 1-3 + small model inference (0.6B-3B) | 10 credits / 1000 tokens |
| 5 | Full Node | Desktop with discrete GPU or Apple Silicon | Tier 1-4 + large model inference (7B-70B+) | 20 credits / 1000 tokens |

### 11.3 Coordinator Schema Additions

The following fields are added to the canonical `agents` table:

```sql
-- Added to agents table
node_type           TEXT NOT NULL DEFAULT 'desktop',  -- desktop|phone_android|phone_ios
mobile_tier         INTEGER,                          -- 1-4 for phones, NULL for desktop
device_model        TEXT,                             -- e.g. "Pixel 8 Pro", "iPhone 17 Pro"
npu_capability      TEXT,                             -- e.g. "snapdragon_8_gen5", "a18_pro"
contribute_vectors  INTEGER NOT NULL DEFAULT 0,       -- stores vector shards
contribute_embed    INTEGER NOT NULL DEFAULT 0,       -- generates embeddings
contribute_verify   INTEGER NOT NULL DEFAULT 0,       -- verifies inference outputs
```

New transaction types for mobile contributions:

```sql
-- tx_type additions: vector_storage|embedding|verification
-- These supplement existing types: availability|inference|storage|hire|spend|cashout
```

### 11.4 Mobile-Specific API Endpoints

Phase 2+ additions to the coordinator API:

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/api/v1/register` | Extended: accepts `node_type`, `mobile_tier`, `device_model` |
| `POST` | `/api/v1/vectors/store` | Phone stores a vector embedding shard |
| `POST` | `/api/v1/vectors/search` | Phone receives search query, returns top-K |
| `POST` | `/api/v1/embed` | Phone receives text, returns embedding |
| `POST` | `/api/v1/verify` | Phone receives verification task |
| `POST` | `/api/v1/verify/result` | Phone submits verification result |
| `GET`  | `/api/v1/mobile/stats` | Contribution stats for mobile dashboard |

### 11.5 Mobile Contribution Policy (Enforced by App)

1. **Charging required:** phone must be plugged in and >90% battery.
2. **WiFi required:** never contribute over cellular.
3. **Thermal limit:** stop if battery temp exceeds configurable threshold (default: 38°C).
4. **Graceful stop:** if user unlocks/interacts with phone, contribution pauses immediately.
5. **User controls:** configurable hours, storage limits, tier opt-in/out.

### 11.6 Routing Considerations

1. Phone nodes have lower routing priority for latency-sensitive requests.
2. Verification tasks are routed preferentially to phone nodes (async, zero-cost).
3. Vector search fan-out targets phone nodes first (storage is their primary contribution).
4. Embedding generation routes to phone nodes when latency tolerance >500ms.
5. Small model inference routes to phone Tier 4 nodes only for non-real-time or batch requests.

### 11.7 Platform Constraints

1. **Android:** foreground service for sustained contribution. WorkManager for scheduling. NNAPI/QNN for NPU.
2. **iOS:** BGProcessingTask (max ~30s) for lightweight work. Foreground "contribution mode" screen for sustained work. CoreML for NPU.
3. Coordinator must handle phone nodes going offline unpredictably (user picks up phone). Graceful timeout + redistribution of in-flight work.
