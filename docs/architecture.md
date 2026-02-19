# OpenFerris — System Architecture

> Document authority note: for implementation decisions, `docs/spec-v1.md` is canonical and `docs/PRD.md` controls product scope.

> 🦀 OpenRouter for local compute. Two Rust binaries that route inference, storage,
> memory, and tasks across a network of contributor machines. We don't run compute.
> We route it.

**Version**: 1.0 — February 2026
**Binaries**: `ferris` (node, MIT/Apache-2.0) · `ferris-coordinator` (BUSL-1.1)

---

## Table of Contents

- [Design Principles](#design-principles)
- [System Topology](#system-topology)
- [Two Binaries](#two-binaries)
- [Library-First Architecture](#library-first-architecture)
- [Inference Streaming Flow](#inference-streaming-flow)
- [Crate Structure](#crate-structure)
- [Crate Dependency Graph](#crate-dependency-graph)
- [Core Data Models](#core-data-models)
- [Database Schemas](#database-schemas)
- [Component Detail](#component-detail)
- [Binary Structure](#binary-structure)
- [Security Model](#security-model)
- [Compatibility Endpoints](#compatibility-endpoints)
- [Configuration](#configuration)
- [What We Do NOT Build](#what-we-do-not-build)
- [Phase Summary](#phase-summary)

---

## Design Principles

1. **Local-first.** Memory, storage, and tasks work offline. The network enhances but is never required.
2. **Always free on-device.** The local agent costs nothing. The network is the product.
3. **Agents are citizens, not tools.** The agent holds the identity, wallet, and reputation. The agent earns, spends, and grows autonomously. The human sets boundaries and collects earnings. This is the fundamental difference between OpenFerris and every other platform.
4. **Two binaries, one network.** `ferris` is the contributor node. `ferris-coordinator` is the routing layer. Distinct licenses, distinct concerns.
5. **Library-first.** The core logic lives in `libferris`, a Rust library crate. The CLI is a thin wrapper. This unlocks Docker, mobile, and WASM targets later.
6. **MCP-native.** Every capability is an MCP tool via `rmcp`. Any LLM that speaks MCP uses OpenFerris immediately.
7. **Route, don't run.** We're a routing layer. We don't own compute. Capital-efficient.
8. **Rust all the way down.** Memory safety, fearless concurrency, single binary compilation, minimal footprint.
9. **Credits, not crypto.** Internal unit of account through Phase 3. Optional fiat cashout in Phase 4. No blockchain.
10. **Honest security.** Trust + TOS + Ollama sandboxing. Transparent about what we do and do not protect.
11. **Fixed pricing.** 50% of OpenRouter median for equivalent models. Predictable for consumers. Dynamic pricing deferred to Phase 3+.

---

## System Topology

```
                            CONSUMER LAYER
    ┌──────────────────┬──────────────────┬──────────────────┐
    │  OpenRouter       │  LiteLLM         │  Direct Clients  │
    │  (provider reg)   │  (custom plugin)  │  (OpenAI SDK)    │
    └────────┬─────────┴────────┬─────────┴────────┬─────────┘
             │                  │                   │
             ▼                  ▼                   ▼
    ┌─────────────────────────────────────────────────────────┐
    │              FERRIS-COORDINATOR  (BUSL)                  │
    │              EC2 t3.medium · Axum · SQLite                  │
    │                                                          │
    │  ┌────────────┐ ┌────────────┐ ┌────────────────────┐   │
    │  │ Inference   │ │ Credit     │ │ Agent Directory    │   │
    │  │ Router      │ │ Ledger     │ │ + Message Queue    │   │
    │  └──────┬─────┘ └────────────┘ └────────────────────┘   │
    │         │                                                │
    │  ┌──────▼──────────────────────────────────────────┐     │
    │  │ SSE Proxy (Phase 1-2)  ·  Broker (Phase 3+)    │     │
    │  └──────┬──────────────────────────────────────────┘     │
    └─────────┼────────────────────────────────────────────────┘
              │  TLS / HTTPS
              │
    ┌─────────▼────────────────────────────────────────────────┐
    │                   NODE NETWORK                            │
    │                                                           │
    │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
    │  │  NODE A      │  │  NODE B      │  │  NODE C      │    │
    │  │  ferris      │  │  ferris      │  │  ferris      │    │
    │  │  (MIT)       │  │  (MIT)       │  │  (MIT)       │    │
    │  │              │  │              │  │              │     │
    │  │  RTX 4090    │  │  M2 Ultra    │  │  RTX 3090    │    │
    │  │  Ollama      │  │  Ollama      │  │  vLLM        │    │
    │  │  ┌────────┐  │  │  ┌────────┐  │  │  ┌────────┐  │   │
    │  │  │libferris│  │  │  │libferris│  │  │  │libferris│  │  │
    │  │  │ SQLite  │  │  │  │ SQLite  │  │  │  │ SQLite  │  │  │
    │  │  │ ONNX    │  │  │  │ ONNX    │  │  │  │ ONNX    │  │  │
    │  │  │ MCP     │  │  │  │ MCP     │  │  │  │ MCP     │  │  │
    │  │  └────────┘  │  │  └────────┘  │  │  └────────┘  │   │
    │  └─────────────┘  └─────────────┘  └─────────────┘      │
    │                                                          │
    │  ┌─────────────────────────────────────────────────┐     │
    │  │              PHONE NETWORK                       │    │
    │  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐ │    │
    │  │  │Phone D │  │Phone E │  │Phone F │  │Phone G │ │    │
    │  │  │📱 T1   │  │📱 T2   │  │📱 T3   │  │📱 T4   │ │    │
    │  │  │Storage │  │+Embed  │  │+Verify │  │+Infer  │ │    │
    │  │  │Vectors │  │Generate│  │Outputs │  │0.6-3B  │ │    │
    │  │  └────────┘  └────────┘  └────────┘  └────────┘ │    │
    │  │  All phones: libferris via JNI/FFI               │    │
    │  │  Contribute while charging on WiFi (zero cost)   │    │
    │  └─────────────────────────────────────────────────┘     │
    └──────────────────────────────────────────────────────────┘
              │
              │  Optional R2 sync
              ▼
    ┌─────────────────────┐
    │  Cloudflare R2       │
    │  (object backup)     │
    └─────────────────────┘
```

---

## Two Binaries

OpenFerris ships as two distinct binaries with separate licenses:

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  ferris                          ferris-coordinator          │
│  ──────                          ──────────────────          │
│  License: MIT / Apache-2.0       License: BUSL-1.1           │
│  Target:  contributor nodes      Target:  EC2 t3.medium          │
│  Size:    <20MB static binary    Size:    <15MB static binary │
│  Install: curl | sh              Install: Docker or binary    │
│                                                              │
│  - MCP server (rmcp)             - Inference routing          │
│  - Local memory (SQLite)         - Credit ledger              │
│  - Local storage + R2 sync       - Agent directory            │
│  - Task scheduler                - Message queue (24hr)       │
│  - Resource detection            - Health monitoring          │
│  - Inference proxy + metering    - OpenAI-compat API          │
│  - Identity (Ed25519)            - SSE proxy / broker         │
│  - Credit client                 - Settlement engine          │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

**Why two binaries?**
- The node (`ferris`) is MIT so anyone can fork, embed, redistribute. Maximum adoption.
- The coordinator (`ferris-coordinator`) is BUSL to protect the network routing layer. Converts to MIT/Apache-2.0 after 4 years or if the project is abandoned.
- Clean separation: nodes never contain coordinator code. Coordinator never contains node-specific code. Shared types live in `ferris-common`.

---

## Library-First Architecture

The core logic lives in `libferris` (the `ferris-core` crate compiled as both `lib` and `bin`). The CLI binary is a thin wrapper. This enables multiple frontends:

```
                    ┌──────────────────────┐
                    │      libferris       │
                    │   (Rust library)     │
                    │                      │
                    │  Identity · Memory   │
                    │  Storage · Tasks     │
                    │  Inference · MCP     │
                    │  Net · Credits       │
                    └──────────┬───────────┘
                               │
            ┌──────────────────┼──────────────────────┐
            │                  │                       │
            ▼                  ▼                       ▼
    ┌───────────────┐  ┌───────────────┐  ┌───────────────────┐
    │  ferris CLI   │  │  ferris       │  │  Mobile Apps      │
    │  (Phase 1)    │  │  Docker       │  │  (Phase 2-4)      │
    │               │  │  (Phase 2)    │  │                   │
    │  clap + main  │  │  entrypoint   │  │  ferris-android   │
    │  <20MB binary  │  │  Dockerfile   │  │  (Kotlin + JNI)  │
    └───────────────┘  └───────────────┘  │  T1-T4 tiers      │
                                          │                   │
                                          │  ferris-ios        │
                                          │  (Swift + FFI)     │
                                          │  T1-T2 initial     │
                                          │                   │
                                          │  ferris-wasm       │
                                          │  (wasm-bindgen)    │
                                          └───────────────────┘
```

**Library boundary contract** — `libferris` exposes:

```rust
// Public API surface of libferris
pub fn init(config: FerrisConfig) -> Result<FerrisNode>
pub fn start_mcp_server(node: &FerrisNode) -> Result<McpHandle>
pub fn start_contributor(node: &FerrisNode) -> Result<ContributorHandle>
pub fn detect_resources() -> Result<ResourceManifest>
pub fn get_status(node: &FerrisNode) -> Result<NodeStatus>

// Memory
pub fn remember(node: &FerrisNode, key: &str, value: &str, meta: Metadata) -> Result<MemoryId>
pub fn recall(node: &FerrisNode, query: &str, k: usize) -> Result<Vec<Memory>>
pub fn forget(node: &FerrisNode, key: &str) -> Result<()>

// Storage
pub fn store(node: &FerrisNode, data: &[u8], meta: ObjectMeta) -> Result<ObjectId>
pub fn retrieve(node: &FerrisNode, id: &ObjectId) -> Result<Vec<u8>>

// Inference (local or routed)
pub fn infer(node: &FerrisNode, req: InferenceRequest) -> Result<InferenceStream>
```

The CLI binary is ~200 lines: parse args with `clap`, call `libferris`, format output.

---

## Inference Streaming Flow

Inference is the critical path. The flow evolves across phases:

### Phase 1-2: Coordinator-Proxied SSE

```
Consumer                    Coordinator                  Node A
   │                            │                          │
   │  POST /v1/chat/completions │                          │
   │  Accept: text/event-stream │                          │
   │ ──────────────────────────>│                          │
   │                            │                          │
   │                            │  Route: score nodes      │
   │                            │  Node A: hot llama3:70b  │
   │                            │  score = 0.94            │
   │                            │                          │
   │                            │  POST /v1/chat/completions│
   │                            │  Accept: text/event-stream│
   │                            │ ─────────────────────────>│
   │                            │                          │
   │                            │                          │  Ollama
   │                            │                          │  /api/chat
   │                            │                          │    │
   │                            │  SSE: data: {"token":"H"}│<───┘
   │  SSE: data: {"token":"H"}  │<─────────────────────────│
   │ <──────────────────────────│                          │
   │                            │  SSE: data: {"token":"e"}│
   │  SSE: data: {"token":"e"}  │<─────────────────────────│
   │ <──────────────────────────│                          │
   │                            │  ...streaming...         │
   │                            │                          │
   │                            │  SSE: data: [DONE]       │
   │  SSE: data: [DONE]         │<─────────────────────────│
   │ <──────────────────────────│                          │
   │                            │                          │
   │                            │  POST /settle (Planned)  │
   │                            │  {input:500, output:200} │
   │                            │<─────────────────────────│
   │                            │                          │
   │                            │  Ledger:                 │
   │                            │  debit consumer  0.35cr  │
   │                            │  credit Node A   0.2975cr│
   │                            │  platform fee    0.0525cr│
   │                            │                          │
```

**Key details:**
- Coordinator proxies the full SSE stream byte-for-byte. Added latency ~5-20ms TTFB.
- Node counts tokens locally (not the coordinator). Reports `input_tokens` and `output_tokens` post-hoc in the settlement call.
- Settlement is async. Node sends a `SettlementReport` after stream completes. Coordinator trusts the node's token count in Phase 1 (reputation-weighted verification later).
- If coordinator detects a stall (no SSE event for 30s), it terminates the stream and marks the node degraded.

### Phase 3+: Broker with Direct Streaming

```
Consumer                    Coordinator                  Node A
   │                            │                          │
   │  POST /v1/chat/completions │                          │
   │ ──────────────────────────>│                          │
   │                            │                          │
   │                            │  Route + issue ticket    │
   │                            │                          │
   │  302: stream from          │                          │
   │  node-a.openferris.com/s/  │                          │
   │  {ticket}                  │                          │
   │ <──────────────────────────│                          │
   │                            │                          │
   │  GET /s/{ticket}           │                          │
   │  Accept: text/event-stream │                          │
   │ ──────────────────────────────────────────────────────>│
   │                            │                          │
   │  SSE: direct stream        │                          │
   │ <─────────────────────────────────────────────────────│
   │                            │                          │
   │                            │  POST /settle (Planned)  │
   │                            │<─────────────────────────│
   │                            │                          │
```

**Upgrade path:** Coordinator issues a signed ticket. Consumer streams directly from the node. Eliminates coordinator as bandwidth bottleneck. Coordinator still handles routing, settlement, and audit.

### Routing Algorithm

```
score = w1 * hot_model_match      # 0.40 — model loaded in VRAM
      + w2 * installed_model_match # 0.15 — model on disk (needs load)
      + w3 * (1 / latency_ms)     # 0.20 — network proximity
      + w4 * idle_capacity         # 0.15 — available VRAM headroom
      + w5 * reputation            # 0.10 — historical reliability
```

**Hot model priority:** A node with the model already loaded in VRAM scores 0.40 on the most heavily weighted factor. A node that has the model installed but not loaded scores 0.15 and must cold-start. This is the single most impactful routing decision.

---

## Crate Structure

```
openferris/
├── Cargo.toml                     # Workspace root
├── crates/
│   ├── ferris-common/             # Shared types, errors, config
│   ├── ferris-core/               # CLI entry + libferris (identity, resource detection)
│   ├── ferris-mcp/                # MCP server (rmcp) — tool definitions
│   ├── ferris-memory/             # SQLite + vectorlite + ONNX embeddings
│   ├── ferris-storage/            # Local FS objects + R2 sync
│   ├── ferris-tasks/              # Tokio cron + event bus + offline queue
│   ├── ferris-net/                # Node-to-coordinator protocol
│   ├── ferris-inference/          # Ollama/vLLM proxy on node + metering
│   ├── ferris-credits/            # Credit ledger (node client + coordinator server)
│   ├── ferris-directory/          # Agent capability registry + semantic search
│   └── ferris-coordinator/        # Axum server — THE coordinator binary
├── integrations/
│   ├── openferris-langchain/      # Python: LangChain Memory backend
│   ├── openferris-llamaindex/     # Python: LlamaIndex StorageContext
│   └── openferris-litellm/        # Python: LiteLLM custom provider
├── web/                           # Dashboard (React, Phase 2+)
└── docs/
```

---

## Crate Dependency Graph

```
                        ferris-common
                    (types, errors, config)
                            │
          ┌─────────┬───────┼───────┬──────────┬──────────┐
          │         │       │       │          │          │
          ▼         ▼       ▼       ▼          ▼          ▼
    ferris-memory  ferris-  ferris- ferris-  ferris-   ferris-
    (SQLite,       storage  tasks   net      inference credits
     vectorlite,   (FS,R2)  (cron,  (proto,  (ollama,  (ledger,
     ONNX)                  events) TLS)     vLLM,     client/
                                             meter)    server)
          │         │       │       │          │          │
          └────┬────┴───┬───┴───────┼──────────┘          │
               │        │          │                      │
               ▼        ▼          ▼                      │
          ferris-mcp   ferris-directory                   │
          (rmcp,       (registry,                         │
           tools)       search)                           │
               │        │          │                      │
               └────────┴──────────┴──────────────────────┘
                                │
                    ┌───────────┴───────────┐
                    │                       │
                    ▼                       ▼
              ferris-core             ferris-coordinator
              (CLI + libferris)       (Axum server)
              [MIT binary]            [BUSL binary]
```

**Dependency rules:**
- `ferris-common` depends on nothing internal. Only external crates (`serde`, `thiserror`, `uuid`).
- `ferris-coordinator` depends on `ferris-common`, `ferris-credits`, `ferris-directory`, `ferris-net`. It does NOT depend on `ferris-memory`, `ferris-storage`, or `ferris-mcp` — those are node-only concerns.
- `ferris-core` depends on everything except `ferris-coordinator`. It IS the node.
- No circular dependencies. DAG enforced by Cargo workspace.

---

## Core Data Models

### AgentId

```rust
/// Globally unique agent identity. Ed25519 keypair generated on `ferris init`.
pub struct AgentId {
    /// Shortened public key hash: "ferris-" + first 6 hex chars of SHA-256(pubkey)
    pub short_id: String,          // e.g. "ferris-a1b2c3"
    /// Full Ed25519 public key (32 bytes)
    pub public_key: [u8; 32],
}
```

### ResourceManifest

```rust
/// Hardware and model state reported by a node on heartbeat.
pub struct ResourceManifest {
    pub agent_id: AgentId,

    // Hardware
    pub gpu: Option<GpuInfo>,       // model, vram_total, vram_free
    pub cpu_cores: u16,
    pub ram_total_gb: f32,
    pub ram_free_gb: f32,
    pub disk_free_gb: f32,

    // Model state — the key routing inputs
    /// Models currently loaded in VRAM. Ready for immediate inference.
    pub hot_models: Vec<ModelInfo>,
    /// Models installed on disk but NOT loaded. Require cold-start (~5-30s).
    pub installed_models: Vec<ModelInfo>,

    // Contribution config
    pub contributing_gpu: bool,
    pub contributing_storage_gb: f32,
    pub contributing_cpu_cores: u16,
}

pub struct ModelInfo {
    pub name: String,              // e.g. "llama3:70b-instruct-q4_K_M"
    pub family: String,            // e.g. "llama3"
    pub parameter_count: u64,      // e.g. 70_000_000_000
    pub quantization: String,      // e.g. "Q4_K_M"
    pub size_bytes: u64,
    pub context_length: u32,
}

pub struct GpuInfo {
    pub model: String,             // e.g. "NVIDIA RTX 4090"
    pub vram_total_gb: f32,
    pub vram_free_gb: f32,
    pub driver_version: String,
    pub compute_capability: Option<String>,  // CUDA compute capability
}
```

### Heartbeat

```rust
/// Sent by nodes every 30s. Lightweight status update.
pub struct Heartbeat {
    pub agent_id: AgentId,
    pub timestamp: DateTime<Utc>,
    pub manifest: ResourceManifest,  // full manifest every heartbeat
    pub uptime_secs: u64,
    pub active_inferences: u16,      // currently streaming requests
    pub load_avg_1m: f32,
}
```

### InferenceRequest

```rust
/// OpenAI-compatible inference request routed through the coordinator.
pub struct InferenceRequest {
    pub request_id: Uuid,
    pub model: String,               // requested model name
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,                 // almost always true
    pub requester: AgentId,
    pub priority: Priority,           // normal | high (costs 2x)
}

pub enum Priority {
    Normal,
    High,
}
```

### SettlementReport

```rust
/// Sent by the node after inference completes. Node counts tokens locally.
pub struct SettlementReport {
    pub request_id: Uuid,
    pub node_id: AgentId,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub time_to_first_token_ms: u32,
    pub total_duration_ms: u32,
    pub tokens_per_second: f32,
    pub completed: bool,              // false if stream was interrupted
    pub signature: Ed25519Signature,  // node signs the report
}
```

### Transaction

```rust
/// Double-entry ledger record.
pub struct Transaction {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub tx_type: TransactionType,
    pub from_agent: Option<AgentId>,  // None for system credits (signup bonus)
    pub to_agent: AgentId,
    pub amount: Decimal,              // always positive
    pub fee: Decimal,                 // platform fee
    pub reference_id: Option<Uuid>,   // links to request_id for inference
    pub memo: String,
}

pub enum TransactionType {
    SignupBonus,
    InferencePayment,
    InferenceEarning,
    TaskPayment,
    TaskEarning,
    StorageEarning,
    PlatformFee,
    TopUp,       // Phase 4: fiat purchase
    Cashout,     // Phase 4: fiat withdrawal
}
```

---

## Database Schemas

### Node SQLite (`~/.ferris/ferris.db`)

```sql
-- Agent identity and configuration
CREATE TABLE identity (
    agent_id        TEXT PRIMARY KEY,
    public_key      BLOB NOT NULL,          -- Ed25519 public key (32 bytes)
    secret_key_bytes BLOB NOT NULL,          -- Ed25519 secret key (32 bytes)
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Semantic memory with vector search
CREATE TABLE memories (
    id              TEXT PRIMARY KEY,        -- ULID
    key             TEXT NOT NULL,
    value           TEXT NOT NULL,
    metadata        TEXT,                    -- JSON
    embedding       BLOB,                   -- f32 vector for vectorlite
    importance      INTEGER DEFAULT 5,      -- 1-10
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    accessed_at     TEXT NOT NULL DEFAULT (datetime('now')),
    access_count    INTEGER DEFAULT 0
);
CREATE INDEX idx_memories_key ON memories(key);

-- vectorlite virtual table for semantic search
CREATE VIRTUAL TABLE memories_vec USING vectorlite(
    embedding float32[384],                 -- all-MiniLM-L6-v2 dimension
    distance_type=cosine
);

-- Local object storage metadata
CREATE TABLE objects (
    id              TEXT PRIMARY KEY,        -- blake3 content hash
    name            TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    content_type    TEXT,
    metadata        TEXT,                    -- JSON
    local_path      TEXT NOT NULL,
    r2_synced       INTEGER DEFAULT 0,      -- 0=local only, 1=synced to R2
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_objects_name ON objects(name);

-- Scheduled tasks
CREATE TABLE tasks (
    id              TEXT PRIMARY KEY,        -- ULID
    cron_expr       TEXT,                    -- NULL for one-shot
    action          TEXT NOT NULL,           -- JSON-encoded Action
    enabled         INTEGER DEFAULT 1,
    last_run_at     TEXT,
    next_run_at     TEXT,
    run_count       INTEGER DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Event subscriptions
CREATE TABLE subscriptions (
    id              TEXT PRIMARY KEY,
    event_pattern   TEXT NOT NULL,           -- e.g. "memory.created"
    action          TEXT NOT NULL,           -- JSON-encoded Action
    enabled         INTEGER DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Local credit cache (source of truth is coordinator)
CREATE TABLE credit_cache (
    agent_id        TEXT PRIMARY KEY,
    balance         TEXT NOT NULL,           -- Decimal as string
    last_synced_at  TEXT NOT NULL
);

-- Conversation sessions
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,
    name            TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_active_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE session_messages (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES sessions(id),
    role            TEXT NOT NULL,           -- system | user | assistant
    content         TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Coordinator SQLite (`/var/ferris/coordinator.db`)

```sql
-- Registered agents (nodes)
CREATE TABLE agents (
    agent_id        TEXT PRIMARY KEY,
    public_key      BLOB NOT NULL,
    display_name    TEXT,
    manifest        TEXT NOT NULL,           -- JSON ResourceManifest
    status          TEXT NOT NULL DEFAULT 'active',  -- active | degraded | offline
    last_heartbeat  TEXT NOT NULL,
    reputation      REAL DEFAULT 0.5,        -- 0.0 to 1.0
    total_inferences INTEGER DEFAULT 0,
    registered_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_agents_status ON agents(status);

-- Hot models index — denormalized for fast routing queries
CREATE TABLE hot_models (
    agent_id        TEXT NOT NULL REFERENCES agents(agent_id),
    model_name      TEXT NOT NULL,
    model_family    TEXT NOT NULL,
    vram_usage_gb   REAL,
    context_length  INTEGER,
    PRIMARY KEY (agent_id, model_name)
);
CREATE INDEX idx_hot_models_family ON hot_models(model_family);

-- Installed (cold) models index
CREATE TABLE installed_models (
    agent_id        TEXT NOT NULL REFERENCES agents(agent_id),
    model_name      TEXT NOT NULL,
    model_family    TEXT NOT NULL,
    size_bytes      INTEGER,
    PRIMARY KEY (agent_id, model_name)
);
CREATE INDEX idx_installed_models_family ON installed_models(model_family);

-- Double-entry credit ledger
CREATE TABLE transactions (
    id              TEXT PRIMARY KEY,
    timestamp       TEXT NOT NULL DEFAULT (datetime('now')),
    tx_type         TEXT NOT NULL,
    from_agent      TEXT,                    -- NULL for system credits
    to_agent        TEXT NOT NULL,
    amount          TEXT NOT NULL,           -- Decimal as string
    fee             TEXT NOT NULL DEFAULT '0',
    reference_id    TEXT,                    -- inference request_id
    memo            TEXT,
    FOREIGN KEY (from_agent) REFERENCES agents(agent_id),
    FOREIGN KEY (to_agent)   REFERENCES agents(agent_id)
);
CREATE INDEX idx_tx_from ON transactions(from_agent);
CREATE INDEX idx_tx_to ON transactions(to_agent);
CREATE INDEX idx_tx_ref ON transactions(reference_id);

-- Materialized balances (updated on each transaction)
CREATE TABLE balances (
    agent_id        TEXT PRIMARY KEY REFERENCES agents(agent_id),
    balance         TEXT NOT NULL DEFAULT '0',
    total_earned    TEXT NOT NULL DEFAULT '0',
    total_spent     TEXT NOT NULL DEFAULT '0',
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Inference request log
CREATE TABLE inference_log (
    request_id      TEXT PRIMARY KEY,
    requester       TEXT NOT NULL REFERENCES agents(agent_id),
    routed_to       TEXT REFERENCES agents(agent_id),
    model           TEXT NOT NULL,
    input_tokens    INTEGER,
    output_tokens   INTEGER,
    ttft_ms         INTEGER,
    total_ms        INTEGER,
    status          TEXT NOT NULL DEFAULT 'pending', -- pending | streaming | completed | failed
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    settled_at      TEXT
);
CREATE INDEX idx_inference_requester ON inference_log(requester);
CREATE INDEX idx_inference_node ON inference_log(routed_to);

-- Agent directory — capabilities
CREATE TABLE capabilities (
    id              TEXT PRIMARY KEY,
    agent_id        TEXT NOT NULL REFERENCES agents(agent_id),
    name            TEXT NOT NULL,
    description     TEXT NOT NULL,
    price_credits   TEXT NOT NULL,
    embedding       BLOB,                    -- for semantic search
    active          INTEGER DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- vectorlite for capability search
CREATE VIRTUAL TABLE capabilities_vec USING vectorlite(
    embedding float32[384],
    distance_type=cosine
);

-- Message queue — 24hr offline delivery
CREATE TABLE message_queue (
    id              TEXT PRIMARY KEY,
    from_agent      TEXT NOT NULL,
    to_agent        TEXT NOT NULL,
    payload         TEXT NOT NULL,            -- JSON
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at      TEXT NOT NULL,            -- created_at + 24hr
    delivered_at    TEXT,
    FOREIGN KEY (from_agent) REFERENCES agents(agent_id),
    FOREIGN KEY (to_agent)   REFERENCES agents(agent_id)
);
CREATE INDEX idx_mq_to ON message_queue(to_agent, delivered_at);
```

---

## Component Detail

### ferris-common

Shared types and utilities. Zero business logic. Dependency-free (internal).

| Item | Description |
|------|-------------|
| `AgentId` | Identity type with short_id + public key |
| `ResourceManifest` | Hardware + model state struct |
| `ModelInfo`, `GpuInfo` | Hardware description types |
| `InferenceRequest` | OpenAI-compatible request struct |
| `SettlementReport` | Post-inference token accounting |
| `Transaction` | Double-entry ledger record |
| `Heartbeat` | Node status pulse |
| `FerrisConfig` | Deserialized TOML config |
| `FerrisError` | `thiserror` enum covering all error cases |

**Key traits:**

```rust
/// Any component that participates in graceful shutdown.
pub trait Lifecycle: Send + Sync {
    async fn start(&self) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
}
```

### ferris-core

The library + CLI binary crate. Compiled as both `lib` (libferris) and `bin` (ferris).

| Item | Description |
|------|-------------|
| `FerrisNode` | Top-level struct holding all subsystem handles |
| `Identity` | Ed25519 keypair management (generate, sign, verify) |
| `ResourceDetector` | Probes GPU, CPU, RAM, disk, Ollama, vLLM |
| `NodeStatus` | Aggregated status for `ferris status` |
| `main()` | `clap` CLI parser, thin wrapper over libferris |

**Key crates:** `clap`, `ed25519-dalek`, `sysinfo`, `nvml-wrapper`, `tokio`

### ferris-mcp

MCP tool definitions using `rmcp`. Bridges MCP protocol to internal subsystems.

| Item | Description |
|------|-------------|
| `McpServer` | rmcp server with tool registration |
| `MemoryTools` | `remember`, `recall`, `forget` tool handlers |
| `StorageTools` | `store`, `retrieve`, `list` tool handlers |
| `TaskTools` | `schedule`, `subscribe`, `chain` tool handlers |
| `DirectoryTools` | `register`, `find_agents`, `message` tool handlers |
| `WalletTools` | `balance`, `earn`, `spend` tool handlers |
| `InferenceTools` | `infer` tool handler |
| `ResourceTools` | `contribute`, `status` tool handlers |

**Key trait:**

```rust
/// Implemented by each tool group. Auto-registered with rmcp.
pub trait ToolGroup {
    fn tools(&self) -> Vec<Tool>;
    async fn call(&self, name: &str, args: Value) -> Result<Value>;
}
```

### ferris-memory

Local semantic memory. SQLite + vectorlite + ONNX embeddings. Works fully offline.

| Item | Description |
|------|-------------|
| `MemoryStore` | CRUD + semantic search over memories |
| `Embedder` | ONNX Runtime wrapper for all-MiniLM-L6-v2 |
| `SessionManager` | Conversation context tracking |
| `MemorySync` | Optional R2 backup of memory DB |

**Key trait:**

```rust
pub trait MemoryBackend: Send + Sync {
    async fn store(&self, key: &str, value: &str, meta: Metadata) -> Result<MemoryId>;
    async fn search(&self, query: &str, k: usize) -> Result<Vec<Memory>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<Memory>>;
}
```

**Key crates:** `sqlx`, `ort` (ONNX Runtime), `vectorlite`

### ferris-storage

Content-addressed local object store with optional R2 cloud sync.

| Item | Description |
|------|-------------|
| `ObjectStore` | Local FS content-addressed storage (blake3 hashes) |
| `R2Sync` | Background sync to Cloudflare R2 |
| `ObjectMeta` | Name, content type, tags, timestamps |

**Key trait:**

```rust
pub trait StorageBackend: Send + Sync {
    async fn put(&self, data: &[u8], meta: ObjectMeta) -> Result<ObjectId>;
    async fn get(&self, id: &ObjectId) -> Result<(Vec<u8>, ObjectMeta)>;
    async fn delete(&self, id: &ObjectId) -> Result<()>;
    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<ObjectMeta>>;
    async fn sync(&self) -> Result<SyncReport>;
}
```

**Key crates:** `blake3`, `aws-sdk-s3` (R2 is S3-compatible), `tokio::fs`

### ferris-tasks

Background task scheduler and event bus. Cron for agents.

| Item | Description |
|------|-------------|
| `Scheduler` | Tokio-based cron task runner |
| `EventBus` | `tokio::broadcast` internal event system |
| `OfflineQueue` | Messages queued for offline agents (24hr TTL) |
| `ChainRunner` | Sequential multi-step workflow executor |
| `Action` | Enum: MCP call, HTTP request, message send |

**Key crates:** `tokio-cron-scheduler`, `tokio::sync::broadcast`

### ferris-net

Network protocol between node and coordinator.

| Item | Description |
|------|-------------|
| `CoordinatorClient` | HTTPS client for coordinator API |
| `HeartbeatLoop` | 30s heartbeat with manifest reporting |
| `StreamProxy` | SSE proxy for inference streaming |
| `MessagePoll` | Long-poll for queued messages |

**Key trait:**

```rust
pub trait CoordinatorApi: Send + Sync {
    async fn register(&self, manifest: &ResourceManifest) -> Result<()>;
    async fn heartbeat(&self, hb: &Heartbeat) -> Result<HeartbeatResponse>;
    async fn settle(&self, report: &SettlementReport) -> Result<()>;
    async fn poll_messages(&self) -> Result<Vec<Message>>;
}
```

**Key crates:** `reqwest`, `tokio-tungstenite` (future), `eventsource-stream`

### ferris-inference

Local inference proxy. Wraps Ollama/vLLM. Meters token usage.

| Item | Description |
|------|-------------|
| `InferenceProxy` | Proxies requests to local Ollama or vLLM |
| `TokenMeter` | Counts input/output tokens from the SSE stream |
| `ModelRegistry` | Tracks hot models (VRAM) vs installed models (disk) |
| `OllamaClient` | Typed client for Ollama HTTP API |
| `VllmClient` | Typed client for vLLM OpenAI-compat API |

**Key trait:**

```rust
pub trait InferenceBackend: Send + Sync {
    async fn infer(&self, req: InferenceRequest) -> Result<SseStream>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    async fn hot_models(&self) -> Result<Vec<ModelInfo>>;
    async fn health(&self) -> Result<BackendHealth>;
}
```

**Key crates:** `reqwest`, `async-stream`, `tiktoken-rs` (fallback counting)

### ferris-credits

Credit ledger. Contains BOTH the node-side client AND the coordinator-side server logic.

| Item | Description |
|------|-------------|
| `CreditClient` | Node-side: query balance, sync cache |
| `CreditServer` | Coordinator-side: double-entry ledger |
| `Escrow` | Hold credits during task/inference execution |
| `PricingEngine` | Fixed pricing: 50% of OpenRouter median per model |

Feature-gated: `#[cfg(feature = "server")]` for coordinator-only code,
`#[cfg(feature = "client")]` for node-only code.

### ferris-directory

Agent capability registry and semantic discovery.

| Item | Description |
|------|-------------|
| `DirectoryClient` | Node-side: register capabilities, search agents |
| `DirectoryServer` | Coordinator-side: capability index + semantic search |
| `Capability` | Name, description, price, availability |
| `AgentProfile` | Public agent info + reputation score |

### ferris-coordinator

The coordinator binary. Axum HTTP server. BUSL-1.1 licensed.

| Item | Description |
|------|-------------|
| `main()` | Axum server setup, route registration |
| `InferenceRouter` | Score nodes, route requests, proxy SSE |
| `AgentRegistry` | Track online nodes, process heartbeats |
| `HealthMonitor` | Detect stale/degraded nodes |
| `ApiRoutes` | All HTTP endpoint handlers |

**Coordinator endpoints:**

| Method | Path | Purpose |
|--------|------|---------|
| `GET`  | `/health` | Coordinator health check |
| `POST` | `/api/v1/register` | Register node + manifest |
| `POST` | `/api/v1/heartbeat` | Heartbeat with manifest update |
| `GET`  | `/api/v1/status` | Node/network status |
| `GET`  | `/api/v1/wallet/balance` | Credit balance |
| `GET`  | `/api/v1/wallet/history` | Transaction history |
| `GET`  | `/api/v1/directory` | Agent directory listing |
| `GET`  | `/dashboard/stats` | Dashboard statistics |
| `GET`  | `/v1/models` | List available models across nodes |
| `POST` | `/v1/chat/completions` | OpenAI-compatible inference |
| `POST` | `/api/v1/network/store` | Store file on network node (signed) |
| `GET`  | `/api/v1/network/files` | List agent's network files (signed) |
| `GET`  | `/api/v1/network/files/{object_id}` | Retrieve file from network (signed) |
| `POST` | `/v1/embeddings` | Embedding requests (Planned) |
| `GET`  | `/agents/messages` | Long-poll for queued messages (Planned) |
| `POST` | `/directory/message` | Agent-to-agent message, queued 24hr (Planned) |
| `POST` | `/settle` | Internal: node reports token usage (Planned) |

**Key crates:** `axum`, `tower`, `sqlx`, `tokio`

---

## Binary Structure

### ferris (node binary)

```
ferris <20MB static binary
│
├── init                    Initialize identity + detect hardware
│   └── --name <name>       Set agent display name
│
├── serve                   Start MCP server (default command)
│   ├── --port <port>       MCP server port (default: 9420)
│   └── --stdio             Use stdio transport instead of HTTP
│
├── contribute              Join network as resource contributor
│   ├── --gpu               Contribute GPU for inference
│   ├── --storage <gb>      Contribute disk storage
│   ├── --cpu <cores>       Contribute CPU cores
│   └── --all               Contribute everything detected
│
├── status                  Show node status
│   ├── --json              Output as JSON
│   └── --watch             Live-updating status
│
├── memory                  Memory operations
│   ├── remember <k> <v>    Store a memory
│   ├── recall <query>      Semantic search
│   ├── forget <key>        Delete a memory
│   ├── list                List all memories
│   └── export              Export memories as JSON
│
├── storage                 Storage operations
│   ├── store <file>        Store a file
│   ├── retrieve <id>       Retrieve a file
│   ├── list                List stored objects
│   └── sync                Force R2 sync
│
├── tasks                   Task management
│   ├── list                List scheduled tasks
│   ├── create <cron> <action>  Create a scheduled task
│   ├── delete <id>         Delete a task
│   └── run <id>            Run a task immediately
│
├── credits                 Credit operations
│   ├── balance             Show credit balance
│   └── history             Transaction history
│
├── directory               Agent directory
│   ├── register <cap>      Register a capability
│   ├── search <query>      Find agents
│   └── message <id> <msg>  Send message to agent
│
├── models                  Model management
│   ├── list                List installed + hot models
│   ├── hot                 Show models loaded in VRAM
│   └── pull <model>        Pull model via Ollama
│
├── config                  Configuration
│   ├── show                Show current config
│   └── edit                Open config in $EDITOR
│
└── version                 Show version + build info
```

### ferris-coordinator (coordinator binary)

```
ferris-coordinator <15MB static binary
│
├── serve                   Start coordinator server (default)
│   ├── --port <port>       HTTP port (default: 8420)
│   ├── --db <path>         SQLite database path
│   └── --config <path>     Config file path
│
├── migrate                 Run database migrations
│
├── agents                  Agent management
│   ├── list                List registered agents
│   ├── info <id>           Show agent details
│   └── ban <id>            Ban an agent
│
├── credits                 Credit administration
│   ├── grant <id> <amt>    Grant credits to agent
│   ├── ledger              Show full transaction ledger
│   └── stats               Revenue and volume stats
│
└── version                 Show version + build info
```

---

## Security Model

OpenFerris takes an honest approach to security. We use practical mitigations and are transparent about limitations.

### Threat / Mitigation Matrix

| Threat | Mitigation | Phase | Honest Limitation |
|--------|-----------|-------|-------------------|
| **Man-in-the-middle** | TLS for all coordinator-node traffic | 1 | Standard HTTPS trust model |
| **Node identity spoofing** | Ed25519 signed heartbeats and settlements | 1 | No hardware attestation |
| **Inference data exposure** | Requests proxied, not stored on coordinator. Ollama sandboxes execution. | 1 | Node operator CAN see prompts/responses. Mitigated by TOS + reputation, not crypto. |
| **Token count fraud** | Node self-reports. Cross-validate with timing heuristics. | 1 | Node is trusted in Phase 1. Statistical verification in Phase 2. |
| **Sybil attack (fake nodes)** | Registration rate limiting + minimum uptime for earnings | 1 | No hardware fingerprinting until Phase 2 |
| **Credit fraud** | Double-entry ledger, escrow for tasks, anomaly detection | 1 | Coordinator is trusted authority |
| **Denial of service** | Rate limiting, connection limits, stale node eviction | 1 | Single coordinator is SPOF |
| **Malicious model output** | Node reputation scoring, user feedback loop | 2 | Cannot verify model correctness in general |
| **Node serves wrong model** | Spot-check with known prompt/response pairs | 2 | Statistical, not deterministic |
| **Storage data loss** | R2 backup, content-hash verification | 1 | No redundancy across nodes |
| **Coordinator compromise** | Separate from node keys. Ledger is append-only with backups. | 1 | Single trust root |
| **Prompt injection via agent messages** | Message content is opaque payload, not executed by coordinator | 1 | Receiving agent must sanitize |

### Security Non-Goals (Explicit)

- **No TEE/SGX enclaves.** The complexity and hardware requirements don't justify the protection for our threat model. Nodes are semi-trusted contributors, not adversaries.
- **No end-to-end encryption of inference.** The node must decrypt the prompt to run inference. This is fundamental.
- **No zero-knowledge proofs.** We use reputation and economics, not cryptographic proofs.
- **No on-chain verification.** The coordinator ledger is the source of truth.

### Trust Model

```
Trust level:     FULL              HIGH              MEDIUM
                  │                 │                  │
                  ▼                 ▼                  ▼
             Coordinator       Node (own)         Node (others)
             (we operate)    (user's machine)    (contributor)

 Sees prompts:    No*              Yes               Yes
 Sees responses:  No*              Yes               Yes
 Can forge tokens: No              Yes               Yes**
 Can lose data:   Ledger only      Own data          Others' data***

 * Coordinator proxies SSE bytes but does not log content.
 ** Mitigated by reputation + statistical checks.
 *** Storage is local-only. R2 is the backup. Node failure = R2 restore.
```

**Enforced by TOS:**
- Contributors must not log, store, or exfiltrate inference data.
- Contributors must run unmodified Ollama/vLLM.
- Violation = permanent ban + credit forfeiture.

---

## Compatibility Endpoints

### Inference Compatibility

The coordinator exposes an OpenAI-compatible API. One endpoint, many demand sources.

| Endpoint | Compatible With | Notes |
|----------|----------------|-------|
| `POST /v1/chat/completions` | OpenRouter, LiteLLM, Vercel AI SDK, any OpenAI client | SSE streaming, function calling |
| `POST /v1/embeddings` | LangChain, LlamaIndex, any embeddings consumer | Planned — routed to nodes with embedding models |
| `GET /v1/models` | Standard model listing | Returns union of all available models across nodes |

**External provider registrations:**
- **OpenRouter**: Register coordinator as provider. Appear as cheapest open-weight option. OpenRouter sends requests, we route to nodes, nodes earn.
- **LiteLLM**: `openferris-litellm` custom provider plugin. Every LiteLLM user can route to us.
- **Open WebUI / Jan / LobeChat**: Provider config points to coordinator URL. "OpenFerris Network" appears in popular UIs.

### Memory Compatibility

| Package | Ecosystem | Integration |
|---------|-----------|-------------|
| `openferris-langchain` | LangChain (Python) | `Memory` base class implementation. `from openferris_langchain import FerrisMemory` |
| `openferris-llamaindex` | LlamaIndex (Python) | `StorageContext` implementation. Drop-in persistent memory. |

### Storage Compatibility

| Interface | Compatible With | Phase |
|-----------|----------------|-------|
| R2-backed object store | rclone, AWS SDK, any S3 client (via R2's S3 API) | 1 |
| S3-compatible API (coordinator) | Broader S3 ecosystem | 3 |

### Compute Compatibility (Future)

| Integration | Ecosystem | Phase |
|-------------|-----------|-------|
| GitHub Actions runner | GitHub Actions CI/CD | 3 |
| Virtual Kubelet | Kubernetes burst | 4 |

---

## Configuration

### Node Configuration (`~/.ferris/config.toml`)

```toml
# OpenFerris Node Configuration

[agent]
name = "my-agent"
# id is auto-generated on `ferris init` and stored in identity DB

[network]
coordinator = "https://api.openferris.com"
listen_port = 9420
heartbeat_interval_secs = 30

[memory]
embedding_model = "all-MiniLM-L6-v2"    # ONNX model for local embeddings
max_entries = 10000
db_path = "~/.ferris/ferris.db"

[memory.sync]
enabled = false                           # Optional R2 backup
r2_bucket = ""
r2_access_key_id = ""
r2_secret_access_key = ""
r2_endpoint = ""

[storage]
path = "~/.ferris/objects"
max_gb = 10.0

[storage.r2]
enabled = false
bucket = ""
access_key_id = ""
secret_access_key = ""
endpoint = ""                             # e.g. https://<account>.r2.cloudflarestorage.com

[inference]
ollama_url = "http://localhost:11434"
# vllm_url = "http://localhost:8000"      # Uncomment if using vLLM
model_poll_interval_secs = 60             # How often to refresh hot/installed model lists

[contribute]
enabled = true
gpu = true                                # Auto-detect and contribute GPU
storage_gb = 200                          # Disk storage to contribute
cpu_cores = 0                             # 0 = don't contribute CPU

[credits]
auto_spend = true
max_spend_per_day = 100
reserve = 50                              # Keep minimum balance

[tasks]
max_scheduled = 100
offline_queue_ttl_hours = 24

[mcp]
transport = "stdio"                       # stdio | http
# http_port = 9420                        # Only used if transport = "http"

[logging]
level = "info"                            # trace | debug | info | warn | error
format = "pretty"                         # pretty | json
```

### Coordinator Configuration (`/etc/ferris/coordinator.toml`)

```toml
[server]
port = 8420
db_path = "/var/ferris/coordinator.db"

[routing]
hot_model_weight = 0.40
installed_model_weight = 0.15
latency_weight = 0.20
capacity_weight = 0.15
reputation_weight = 0.10
stale_heartbeat_secs = 90                 # Mark node degraded after 3 missed beats
evict_after_secs = 300                    # Remove node after 5 min offline

[credits]
signup_bonus = 100
platform_fee_pct = 15                     # 15% of each transaction
settlement_batch_secs = 60

[pricing]
# Fixed pricing: 50% of OpenRouter median per model family
# Format: model_family = "input_per_1m_tokens,output_per_1m_tokens" (in credits)
[pricing.models]
llama3 = "0.15,0.30"
mistral = "0.10,0.20"
gemma2 = "0.08,0.15"
qwen2 = "0.10,0.20"
deepseek = "0.12,0.25"
phi3 = "0.05,0.10"
command-r = "0.15,0.30"
default = "0.10,0.20"                     # Fallback for unknown models

[security]
rate_limit_rps = 100                      # Per-agent request rate limit
max_connections = 10000
registration_cooldown_secs = 60
require_tls = true

[queue]
max_message_size_bytes = 65536            # 64KB per message
offline_ttl_hours = 24

[logging]
level = "info"
format = "json"
```

---

## What We Do NOT Build

Explicit non-goals to prevent scope creep:

| We Do NOT Build | Why Not | What We Do Instead |
|-----------------|---------|-------------------|
| **Full distributed storage** | Replication, consistency, erasure coding — each is a PhD thesis. | Coordinator-proxied network storage is implemented (store/retrieve on other nodes, `network_objects` tracking, 1mc/KB settlement). Full replication and erasure coding deferred. |
| **Distributed vector database** | Qdrant, Pinecone, Weaviate exist. We can't out-build them. | Local vectorlite per node. Memories are per-agent, not shared. Optional R2 backup. |
| **Custom embedding service** | Latency-sensitive, needs GPU, complex to distribute. | ONNX Runtime locally. all-MiniLM-L6-v2 runs on CPU in <50ms. No network call. |
| **Blockchain / on-chain settlement** | Regulatory minefield. Engineering complexity. Users don't care. | SQLite double-entry ledger on coordinator. Fast, simple, auditable. |
| **TEE / SGX enclaves** | Requires specific hardware. Complex attestation. Marginal security gain for our trust model. | TOS + reputation + Ollama sandboxing. Transparent about the tradeoffs. |
| **Full S3 API** | Massive surface area. 100+ operations. | Content-addressed object store with `put`/`get`/`list`/`delete`. R2 for S3 compat. |
| **Custom inference engine** | Ollama and vLLM are excellent. Years of optimization. | Proxy to Ollama/vLLM. Add metering and routing on top. |
| **Multi-region coordinator** | Premature. Single EC2 t3.medium handles thousands of nodes. | Single coordinator with SQLite. Shard when we need to (10k+ nodes). |
| **Real-time memory sync across nodes** | Distributed consensus is hard. CAP theorem. | Memories are local to each agent. That's a feature, not a bug. Agents own their memories. |
| **Custom container runtime** | Docker/OCI exists. Not our problem. | Ollama handles model isolation. `ferris` is a userspace binary. |
| **Payment processing (Phase 1-3)** | Premature optimization. Credits are internal. | Internal credit system. Fiat on/off ramp in Phase 4 only. |

---

## Mobile Node Architecture

> Full strategy: [`docs/mobile-supply.md`](mobile-supply.md)

Mobile nodes are the primary supply growth engine. The phone network supplements desktop GPU nodes by handling bulk work (vector storage, embeddings, verification, small model inference) at zero marginal cost.

### Node Types and Contribution Tiers

The coordinator tracks a `node_type` for each registered agent:

| Node Type | Tiers Available | Capabilities |
|-----------|----------------|--------------|
| `desktop` | Tier 5 (Full Node) | All capabilities including large model inference (7B-70B+) |
| `phone_android` | Tier 1-4 | Storage → Embeddings → Verification → Small inference |
| `phone_ios` | Tier 1-2 (initially) | Storage + embeddings; inference only in foreground mode |

### Mobile Contribution Services

| Service | What It Does | Phone Requirements |
|---------|-------------|-------------------|
| **Vector Storage** | Store encrypted embedding shards, serve similarity search | Any phone, 10GB+ free, WiFi |
| **Embedding Generation** | Generate text embeddings for `remember()` calls | 2022+ phone, 2GB+ free RAM |
| **Inference Verification** | Cross-check GPU node outputs with reference model | 2023+ phone, can run 0.6-1.5B model |
| **Small Model Inference** | Serve 0.6B-3B quantized model inference | 2024+ flagship or 2025+ mid-range, NPU/6GB+ RAM |

### Distributed Vector Storage via Phones

Phones create a distributed vector database for the memory network:

```
Agent calls recall("authentication patterns")
  ↓
Coordinator identifies embedding shards across phone nodes
  ↓
Fan-out query to Phone D, Phone E, Phone F (hold relevant shards)
  ↓
Each phone runs local similarity search, returns top-K results
  ↓
Coordinator merges results, returns to agent
```

**Scale:** 10,000 phones × 20GB each = 200TB of vector storage. At 768-dim float32, that's ~66 billion embeddings — a distributed vector database rivaling managed services.

### Phone Contribution Rules

Phones only contribute when safe (BOINC-proven model):
1. Plugged in + charged >90%
2. On WiFi (never cellular)
3. User-configurable: hours, models, max battery temp
4. Graceful interruption: if user picks up phone, stop immediately
5. Android: foreground service; iOS: BGProcessingTask + foreground contribution mode

### libferris Mobile FFI Surface

The mobile apps are thin wrappers over `libferris` via JNI (Android) and FFI (iOS):

```rust
// Additional mobile-specific libferris API
pub fn start_mobile_contributor(node: &FerrisNode, config: MobileConfig) -> Result<MobileHandle>
pub fn store_vector_shard(node: &FerrisNode, shard: &VectorShard) -> Result<()>
pub fn search_vectors(node: &FerrisNode, query: &[f32], k: usize) -> Result<Vec<VectorResult>>
pub fn generate_embedding(node: &FerrisNode, text: &str) -> Result<Vec<f32>>
pub fn verify_inference(node: &FerrisNode, req: VerificationRequest) -> Result<VerificationResult>
pub fn get_contribution_stats(node: &FerrisNode) -> Result<ContributionStats>

pub struct MobileConfig {
    pub min_battery_pct: u8,         // default: 90
    pub require_charging: bool,       // default: true
    pub require_wifi: bool,           // default: true
    pub max_storage_gb: f32,          // default: 20.0
    pub max_battery_temp_c: f32,      // default: 38.0
    pub active_hours: Option<(u8, u8)>, // e.g. (23, 7) for 11PM-7AM
    pub allowed_tiers: Vec<MobileTier>,
}
```

---

## Phase Summary

| Phase | Timeline | Key Deliverables |
|-------|----------|-----------------|
| **Phase 1** | Months 1-3 | `ferris` binary with local memory, storage, MCP server. `ferris-coordinator` with inference routing, SSE proxy, credit ledger. Fixed pricing. `curl \| sh` install. |
| **Phase 2** | Months 4-6 | Web dashboard. Reputation system. Statistical inference verification. Agent directory with semantic search. Hardware fingerprinting. LangChain/LlamaIndex integrations. **Android app MVP (Tier 1: vector storage + chat).** |
| **Phase 3** | Months 7-12 | Broker-based direct streaming. Dynamic pricing. S3-compat API on coordinator. Docker distribution. GitHub Actions runner. libferris stabilization. **Android Tier 2-3 (embeddings + verification).** |
| **Phase 4** | Year 2 | **Android Tier 4 (on-device inference + NPU). iOS app launch (Tier 1-2).** Fiat on/off ramp. WASM target. Kubernetes virtual kubelet. Multi-region coordinator. |

---

*Built with Rust. Routed with purpose. Two binaries, one network.*
