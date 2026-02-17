# OpenFerris — Implementation Plan

> Document authority note: execution must follow `docs/PRD.md` (scope) and `docs/spec-v1.md` (technical canon) when details diverge.

16-week phased build plan. Single Rust binary. Ship early, iterate fast.

## Crate Structure

```
openferris/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── ferris-core/         # Config, identity, resource detection, CLI entry
│   ├── ferris-mcp/          # MCP server (rmcp) — all tool definitions
│   ├── ferris-memory/       # SQLite + vectorlite + ONNX embeddings
│   ├── ferris-storage/      # Local FS object store + network contribution
│   ├── ferris-tasks/        # Tokio cron scheduler, event subscriptions
│   ├── ferris-net/          # Network protocol, peer comms, QUIC/libp2p
│   ├── ferris-credits/      # Credit ledger, accounting, metering
│   ├── ferris-inference/    # Inference routing, Ollama/vLLM proxy, metering
│   ├── ferris-directory/    # Agent registry, capability search, messaging
│   └── ferris-coordinator/  # Axum server — routing, health, API
├── web/                     # Dashboard (optional, Phase 2+)
└── docs/
```

## Dependency Map

```
ferris-core (config, identity, resource detection)
     │
     ├──→ ferris-mcp (MCP server, tool definitions)
     │         │
     │         ├──→ ferris-memory (SQLite + vectorlite + ort)
     │         ├──→ ferris-storage (local FS + blake3)
     │         ├──→ ferris-tasks (tokio cron)
     │         ├──→ ferris-credits (ledger, wallet tools)
     │         └──→ ferris-directory (find_agents, message, register)
     │
     ├──→ ferris-net (QUIC/libp2p transport)
     │         │
     │         └──→ ferris-inference (Ollama proxy + routing)
     │
     └──→ ferris-coordinator (Axum server, combines net + credits + directory)
```

---

## Phase 1: Local Agent + Memory (Weeks 1-3)

*"Install Ferris, get a smart agent that remembers"*

### 1.1 `ferris-core`

```rust
pub struct Config {
    pub agent_id: String,
    pub data_dir: PathBuf,       // ~/.ferris/
    pub coordinator_url: String,
    pub resources: ResourceConfig,
}

pub struct ResourceManifest {
    pub gpu: Option<GpuInfo>,
    pub ram_gb: u32,
    pub disk_free_gb: u32,
    pub cpu_cores: u32,
    pub ollama_models: Vec<String>,
}
```

**Tasks:**
- [ ] CLI skeleton (`clap`): `ferris init`, `ferris serve`, `ferris status`
- [ ] Config loading: `~/.ferris/config.toml` with env overrides
- [ ] Identity generation: Ed25519 keypair + agent_id
- [ ] Resource detection: GPU (`nvml-wrapper`, Metal), CPU/RAM/disk (`sysinfo`), Ollama probe
- [ ] XDG-compliant data directories

**Effort:** ~800 LOC, 3-4 days

### 1.2 `ferris-memory`

```rust
pub struct MemoryStore {
    db: SqlitePool,
    embedder: OrtSession,  // all-MiniLM-L6-v2
}

// MCP tools
pub async fn remember(&self, key: &str, value: &str, metadata: Option<Value>) -> Result<()>;
pub async fn recall(&self, query: &str, k: usize) -> Result<Vec<MemoryEntry>>;
pub async fn forget(&self, key: &str) -> Result<()>;
```

**Tasks:**
- [ ] SQLite database with `sqlx` (memories table, FTS5 index)
- [ ] vectorlite extension loading (SQLite vector search)
- [ ] ONNX embedding model via `ort` (all-MiniLM-L6-v2, ~90MB)
- [ ] `remember(key, value)` → embed + store
- [ ] `recall(query, k)` → embed query → vectorlite cosine search → top-k
- [ ] `forget(key)` → delete from DB + index
- [ ] Capacity enforcement (1,000 for free tier)
- [ ] Migration support (sqlx migrations)

**Effort:** ~1,500 LOC, 5-7 days

### 1.3 `ferris-storage`

```rust
pub struct ObjectStore {
    root: PathBuf,       // ~/.ferris/objects/
    max_bytes: u64,
}

pub async fn store(&self, data: &[u8], metadata: Value) -> Result<FileId>;  // blake3 hash
pub async fn retrieve(&self, file_id: &FileId) -> Result<Vec<u8>>;
pub async fn list(&self, prefix: &str) -> Result<Vec<FileEntry>>;
```

**Tasks:**
- [ ] Content-addressed storage (blake3 hashing)
- [ ] `store(file)` → hash, write to `~/.ferris/objects/{hash}`, metadata in SQLite
- [ ] `retrieve(file_id)` → read from disk
- [ ] `list(prefix)` → query metadata
- [ ] Quota enforcement (100MB free, 10GB pro)
- [ ] Garbage collection (unreferenced objects)

**Effort:** ~800 LOC, 3-4 days

### 1.4 `ferris-mcp`

```rust
// MCP server using rmcp
pub struct FerrisMcpServer {
    memory: Arc<MemoryStore>,
    storage: Arc<ObjectStore>,
    tasks: Arc<TaskEngine>,
    // ... other services
}
```

**Tasks:**
- [ ] MCP server setup via `rmcp`
- [ ] Register memory tools: `remember`, `recall`, `forget`
- [ ] Register storage tools: `store`, `retrieve`, `list`
- [ ] Tool schema definitions (JSON Schema for each tool)
- [ ] Error handling and MCP-compliant responses

**Effort:** ~600 LOC, 2-3 days

### 1.5 Phase 1 Integration

**Tasks:**
- [ ] `ferris init` → generate identity + detect resources + create DB
- [ ] `ferris serve` → start MCP server on stdio/SSE
- [ ] `curl | sh` installer script (Linux + macOS)
- [ ] ONNX model download on first run (with progress bar)
- [ ] Integration test: Claude connects via MCP, remembers across sessions
- [ ] README, Show HN draft

**Effort:** ~500 LOC, 2-3 days

### Phase 1 Total: ~4,200 LOC, 15-21 days

---

## Phase 2: Network + Inference Routing (Weeks 4-6)

*"Your idle GPU starts earning"*

### 2.1 `ferris-net`

```rust
pub struct NetworkClient {
    coordinator_url: String,
    node_id: NodeId,
    keypair: Ed25519Keypair,
}

pub async fn register(&self, manifest: ResourceManifest) -> Result<()>;
pub async fn heartbeat(&self, status: NodeStatus) -> Result<()>;
```

**Tasks:**
- [ ] Node registration with coordinator (POST /agents/register)
- [ ] Heartbeat protocol (every 30s: load, uptime, available models)
- [ ] QUIC transport (`quinn`) for low-latency inference streaming
- [ ] Request signing (Ed25519)
- [ ] Connection management (reconnect, backoff)

**Effort:** ~1,500 LOC, 5-7 days

### 2.2 `ferris-inference`

```rust
pub struct InferenceProxy {
    ollama_url: String,
    models: Vec<ModelInfo>,
    meter: InferenceMeter,
    max_concurrent: u32,
}

pub async fn serve_request(&self, req: ChatRequest) -> Result<impl Stream<Item = ChatChunk>>;
```

**Tasks:**
- [ ] Ollama API proxy (forward `/api/chat`, `/api/generate`, `/api/embeddings`)
- [ ] vLLM API proxy (OpenAI-compatible)
- [ ] Token metering (count input/output tokens per request)
- [ ] Concurrent request limiting with backpressure
- [ ] SSE streaming passthrough
- [ ] Model capability reporting to coordinator
- [ ] Health checking (periodic probe of backend)

**Effort:** ~1,800 LOC, 5-7 days

### 2.3 `ferris-credits`

```rust
pub struct CreditLedger {
    db: SqlitePool,  // on coordinator
}

pub struct CreditClient {
    coordinator_url: String,
}

pub async fn balance(&self) -> Result<u64>;
pub async fn earn(&self) -> Result<Vec<EarningsSummary>>;
pub async fn spend(&self, to: NodeId, amount: u64, reason: &str) -> Result<TxId>;
```

**Tasks:**
- [ ] SQLite double-entry ledger on coordinator (`sqlx`)
- [ ] Transaction types: inference, storage, compute, agent-hire, fee
- [ ] Escrow: hold during execution, release on completion
- [ ] Batch settlement (every 60s)
- [ ] Signup bonus (100 credits)
- [ ] Platform fee calculation (10-20%)
- [ ] Balance and history API
- [ ] MCP tools: `balance()`, `earn()`, `spend()`

**Effort:** ~2,000 LOC, 5-7 days

### 2.4 `ferris-coordinator`

```rust
// Axum server
pub struct Coordinator {
    agents: AgentRegistry,
    router: InferenceRouter,
    ledger: CreditLedger,
}
```

**Tasks:**
- [ ] Axum HTTP server
- [ ] Agent registry (register, heartbeat, status)
- [ ] Inference routing: `POST /v1/chat/completions` (OpenAI-compatible)
- [ ] Routing algorithm: score = f(latency, capacity, reputation, model_match)
- [ ] Credit endpoints: balance, history, topup
- [ ] `ferris contribute --gpu --storage --cpu` command
- [ ] `ferris status` — earnings, uptime, resources
- [ ] Basic web dashboard (earnings, status)

**Effort:** ~2,500 LOC, 7-10 days

### 2.5 Phase 2 Integrations (Demand Channels)

The OpenAI-compatible endpoint built in 2.4 is our ticket into multiple existing platforms:

**Tasks:**
- [ ] **OpenRouter**: register as provider — coordinator IS the API endpoint
- [ ] **LiteLLM**: write custom provider plugin (our endpoint is already OpenAI-compatible)
- [ ] **LangChain**: `OpenFerrisMemory` class — wraps `remember`/`recall` MCP tools
- [ ] **LlamaIndex**: `OpenFerrisStorageContext` — wraps memory + storage tools
- [ ] **MCP registries**: submit to Smithery, mcp.so, Awesome MCP Servers (each tool listed separately)
- [ ] Integration docs + quickstart guides for each

**Effort:** ~1,200 LOC (mostly thin wrappers + docs), 3-5 days

### Phase 2 Total: ~9,000 LOC, 25-36 days

---

## Phase 3: Tasks + Directory + Economy (Weeks 7-10)

*"Agents finding and hiring each other"*

### 3.1 `ferris-tasks`

```rust
pub struct TaskEngine {
    scheduler: CronScheduler,
    event_bus: broadcast::Sender<Event>,
}

pub async fn schedule(&self, cron: &str, action: Action) -> Result<TaskId>;
pub async fn subscribe(&self, event: EventFilter, handler: Action) -> Result<SubId>;
pub async fn chain(&self, steps: Vec<Step>) -> Result<ChainId>;
```

**Tasks:**
- [ ] Cron scheduler (`tokio-cron-scheduler`)
- [ ] `schedule(cron, action)` — recurring tasks persisted in SQLite
- [ ] `subscribe(event, handler)` — event-driven triggers
- [ ] `chain(steps)` — sequential multi-step workflows with result passing
- [ ] Task persistence (survive restarts)
- [ ] Capacity enforcement (10 tasks free, 1000/month pro)
- [ ] MCP tools registration

**Effort:** ~1,500 LOC, 5-7 days

### 3.2 `ferris-directory`

```rust
pub struct AgentDirectory {
    db: SqlitePool,       // on coordinator
    embedder: OrtSession, // for semantic matching
}

pub async fn register_capability(&self, service: ServiceDef) -> Result<()>;
pub async fn find_agents(&self, query: &str, filters: Filters) -> Result<Vec<AgentListing>>;
pub async fn message(&self, agent_id: NodeId, payload: Value) -> Result<MessageId>;
```

**Tasks:**
- [ ] Capability registration (name, description, price, availability)
- [ ] Semantic search on capabilities (vectorlite on coordinator)
- [ ] Agent-to-agent messaging (encrypted, via coordinator relay)
- [ ] Reputation display (uptime, jobs completed, average rating)
- [ ] MCP tools: `register_capability`, `find_agents`, `message`

**Effort:** ~1,800 LOC, 5-7 days

### 3.3 Phase 3 Integrations (High-Value Channels)

**Tasks:**
- [ ] **S3-compatible endpoint** (`rusoto`/custom): unlocks rclone, Restic, Duplicati, every S3 client
- [ ] **GitHub Actions runner**: register contributing nodes as self-hosted runners
- [ ] **Open WebUI / Jan / LobeChat**: provider config for "OpenFerris Network"
- [ ] **Vercel AI SDK**: provider package (`@openferris/vercel-ai`)
- [ ] Full economic loop: agent hires → escrow → work → settle
- [ ] Team tier (shared memory namespace, inter-agent workflows)
- [ ] Dashboard: agent directory, task monitoring
- [ ] Documentation and integration guides

**Effort:** ~2,500 LOC, 5-8 days

### Phase 3 Total: ~5,800 LOC, 15-22 days

---

## Phase 4: Financial Layer + Scale (Weeks 11-16)

*"Credits become real money"*

### 4.1 Financial Integration

**Tasks:**
- [ ] USDC wallets via Alloy + Base L2
- [ ] Credits → USDC cashout flow
- [ ] USD → credits topup flow
- [ ] KYC integration for cashout (>$600/year threshold)
- [ ] Enterprise API: bulk credit purchases

**Effort:** ~2,500 LOC, 7-10 days

### 4.2 Enterprise Integrations

**Tasks:**
- [ ] **Virtual Kubelet provider**: K8s workloads burst onto OpenFerris network
- [ ] **Storj/Filecoin integration**: join existing storage economies, earn from storage deals
- [ ] **Ray cluster workers**: register nodes as Ray workers for ML/research compute
- [ ] **Qdrant/Pinecone-compatible REST API**: memory as a vector DB replacement

**Effort:** ~3,000 LOC, 7-10 days

### 4.3 Scale & Polish

**Tasks:**
- [ ] Cross-platform builds: macOS (x86 + ARM), Linux (x86 + ARM), Windows
- [ ] Mobile app (view earnings, manage agents) — React Native or Swift
- [ ] Coordinator HA (multi-instance with shared Postgres)
- [ ] Performance profiling (latency, throughput)
- [ ] Security audit
- [ ] Monitoring + alerting (Prometheus + Grafana)

**Effort:** ~3,000 LOC, 10-14 days

### 4.3 Growth

**Tasks:**
- [ ] Referral program (invite agents, both earn bonus credits)
- [ ] "Powered by OpenFerris" badges
- [ ] Monthly transparency reports
- [ ] Conference talks (RustConf, AI Engineer Summit)
- [ ] Enterprise sales outreach

### Phase 4 Total: ~5,500 LOC, 17-24 days

---

## Total Estimates

| Phase | Crates + Integrations | LOC | Duration |
|-------|----------------------|-----|----------|
| 1. Local Agent + Memory | core, memory, storage, mcp | ~4,200 | 3 weeks |
| 2. Network + Inference + Demand Channels | net, inference, credits, coordinator, OpenRouter/LiteLLM/LangChain integrations | ~9,000 | 3 weeks |
| 3. Tasks + Directory + Platform Integrations | tasks, directory, S3 endpoint, GH Actions, Open WebUI | ~5,800 | 4 weeks |
| 4. Financial + Enterprise Integrations + Scale | financial, Virtual Kubelet, Storj/Filecoin, Ray, platform, growth | ~11,500 | 6 weeks |
| **Total** | **10 crates + integrations** | **~30,500** | **16 weeks** |

Note: LOC estimates are for production Rust code excluding tests. Actual with tests: ~1.5x.

## Key Rust Crates

| Category | Crate | Purpose |
|----------|-------|---------|
| **Async** | `tokio` | Runtime, channels, spawn, timers |
| **HTTP** | `reqwest` | Ollama/vLLM proxy, web fetch |
| **Server** | `axum` | Coordinator REST + WebSocket API |
| **MCP** | `rmcp` | Official Rust MCP SDK |
| **Database** | `sqlx` | SQLite for memory, credits, tasks |
| **Vector Search** | `vectorlite` | SQLite extension for embeddings |
| **ML** | `ort` | ONNX Runtime for local embeddings |
| **Serialization** | `serde`, `serde_json`, `toml` | Config, messages, MCP |
| **Crypto** | `aes-gcm` | Data encryption at rest |
| **Crypto** | `ed25519-dalek` | Agent identity + request signing |
| **Hashing** | `blake3` | Content-addressed storage |
| **Network** | `quinn` | QUIC transport |
| **Network** | `libp2p` | Alternative: P2P with NAT traversal |
| **System** | `sysinfo` | CPU, RAM, disk detection |
| **GPU** | `nvml-wrapper` | NVIDIA GPU detection |
| **CLI** | `clap` | Command-line interface |
| **Scheduling** | `tokio-cron-scheduler` | Background task scheduling |
| **Logging** | `tracing` | Structured logging |
| **Time** | `chrono` | Timestamps |
| **UUID** | `uuid` | Transaction and agent identifiers |
| **Config** | `config` | Layered config with env overrides |
| **Blockchain** | `alloy` | Phase 4: USDC integration on Base L2 |

## What NOT to Build

| Component | Reason | Alternative |
|-----------|--------|-------------|
| Custom vector DB | SQLite + vectorlite is sufficient at our scale | vectorlite extension |
| Custom embeddings model | all-MiniLM-L6-v2 is fast and good enough | ONNX via `ort` |
| Full voice pipeline | Not Phase 1 scope; can add later | Focus on MCP tools first |
| Mobile native app | Web dashboard sufficient initially | Phase 4 if needed |
| Blockchain/token | SEC risk, unnecessary friction | Internal credits, USDC cashout Phase 4 |

## Risk Areas

### Technical
1. **vectorlite maturity**: Relatively new SQLite extension. Fallback: `hnsw_rs` in-process.
2. **rmcp stability**: Official but young Rust MCP SDK. Fallback: implement MCP protocol directly over stdio/SSE.
3. **Inference routing latency**: Coordinator adds hop. Mitigate: direct QUIC connections post-routing.
4. **ONNX model size**: 90MB for all-MiniLM-L6-v2. Acceptable for a 20MB binary + 90MB model download.
5. **Cross-platform builds**: GPU detection varies by OS. Test early on all platforms.

### Economic
6. **Cold start**: Need minimum supply before consumers get value. Mitigate: signup bonus + the memory/storage tools provide standalone value even without network.
7. **Inference quality**: Variable hardware = variable quality. Mitigate: reputation system + proof-of-inference from day one.
8. **Pricing stability**: Dynamic pricing can oscillate. Mitigate: dampen with moving averages + floor/ceiling.
9. **Free-tier abuse**: Rate limit + capacity enforcement from launch.

### Market
10. **MCP adoption**: Bet on MCP becoming the standard. Current trajectory is strong (Claude, Cursor, etc.).
11. **DePIN comparison**: Need to clearly differentiate from crypto DePIN (no tokens, no wallets, agent-native).
12. **Mem0 competition**: They have $24M and head start on memory. Our advantage: bundled services + credit economy.
