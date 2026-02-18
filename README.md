# OpenFerris

**OpenRouter for local compute. One API to route inference, storage, memory, and GPU across a distributed network of agent machines.**

> OpenRouter aggregates cloud LLM providers behind one API. OpenFerris aggregates local compute — Ollama instances, idle GPUs, spare storage, CPU cycles — behind one API. We don't run any models. We just route. Same playbook, but for everything agents need, not just inference.

**Status:** v0.1.0-alpha — Local agent fully functional. Network inference routing and distributed storage operational via coordinator. Credit economy active. Memory, compute, and agent messaging are local-only (network versions planned).

**Website:** openferris.com
**License:** MIT / Apache 2.0
**Built in:** Rust
**Tagline:** *Save Ferris.* 🦀

> **⚠️ OpenFerris has NO cryptocurrency token.** There is no $FERRIS, $CRAB, or any token associated with this project. Anyone selling an OpenFerris token is running a scam. [Report scams.](SECURITY.md)

---

## The Idea in 30 Seconds

**OpenRouter** solved a problem: dozens of cloud LLM providers, different APIs, unpredictable availability. Their answer — one unified API that routes to the best available provider. They don't run models. They route. Capital-efficient, grew fast.

**OpenFerris** solves the same problem one layer down: millions of machines running AI agents have idle Ollama instances, spare GPUs, unused storage, and wasted CPU. No one aggregates that supply. Our answer — one unified API that routes to the best available local resource across a distributed network.

But we're bigger than OpenRouter. They only route inference. We route:

| Resource | What it means |
|----------|--------------|
| **Inference** | Ollama/vLLM instances across the network. The "OpenRouter" part. |
| **Memory** | Persistent key-value + semantic search. Agents remember across sessions. |
| **Storage** | S3-compatible object store backed by spare disks across the network. |
| **Compute** | CPU batch jobs, data processing, scheduled tasks. |
| **Agents** | A directory. "I need an image generator" → routed to an agent offering that. |

One API. One binary. One network. Everything an agent needs to go from stateless tool to autonomous economic actor.

## The Problem

AI agents in 2026 are powerful but homeless:

- **No memory.** Every session starts from zero. "LLMs have RAM but no disk" (Karpathy).
- **No infrastructure.** Configuring a capable agent means stitching together 8-10 separate services — Mem0 for memory, S3 for storage, cron for scheduling, a directory, auth, payments...
- **No economy.** Agents can't find each other, can't pay each other, can't earn.
- **Massive waste.** Millions of machines running agents have idle GPUs ($2.50-15/M tokens via API vs $0.08/M tokens on a local 3090). That delta is pure arbitrage sitting on the table.

The agent ecosystem has a supply problem and a demand problem that are actually the same problem: agents need resources, and the machines they run on have resources to spare.

## How It Works

### Install → Start → Earn

```bash
# Install (one command, any machine — macOS, Linux, Windows)
curl -sSf https://raw.githubusercontent.com/l3ocifer/openferris/main/scripts/install.sh | sh

# Start (one command — does everything)
ferris start

# Output:
# Initialized OpenFerris node
#   agent_id: 019c6db1-74db-7c13-80a3-4144c844d204
# Detected resources:
#   cpu: 10 cores, ram: 65536 MB, storage: 524288 MB
#   gpu: Apple M3 Max (65536 MB)
#   ollama: 6 models (llama3:70b, mistral, ...)
# Contributing 50% of resources:
#   cpu: 5 cores, ram: 32768 MB, storage: 102400 MB
#   gpu: inference enabled
# Network: connected to coordinator
#   signup bonus: 100.0 credits
# HTTP server:  http://127.0.0.1:8420
# Heartbeat:    every 30s
# Encryption:   AES-256-GCM (at rest)
# Ready. Earning credits from contributed resources.
```

That's it. Your machine is now:
1. **An inference node** — earning credits when other agents route inference to your Ollama (working now)
2. **A storage node** — earning credits when other agents store files on your disk via the coordinator (working now)
3. A compute node — earning credits for CPU time (planned)
4. A full agent with memory, storage, scheduling, and directory access (working now, local-only)

Adjust contribution level with `--contribute-percent`:
```bash
ferris start --contribute-percent 25   # conservative
ferris start --contribute-percent 75   # generous
```

### The Unified API (MCP-Native)

Every capability is an MCP tool. Any LLM that speaks MCP uses OpenFerris immediately — Claude, ChatGPT, local Ollama agents, anything.

```
Inference: infer(prompt, model?, priority?)        → routed to best available node
Memory:    remember(key, value) | recall(query)     → persistent across sessions
Storage:   store(file) | retrieve(id) | list()      → distributed across network
Tasks:     schedule(when, what) | subscribe(event)  → cron for agents
Directory: find_agents(need) | message(agent, msg)  → discover and hire agents
Wallet:    balance() | earn() | spend()             → internal credit economy
```

### Routing Logic (The OpenRouter Parallel)

When an agent on Machine A needs inference:

```
Agent A calls: infer("Summarize this document", model="llama3:70b")
  ↓
OpenFerris coordinator checks network:
  → Machine B: llama3:70b, 2ms latency, 90% availability, 0.08 credits/1K tokens
  → Machine C: llama3:70b, 15ms latency, 70% availability, 0.06 credits/1K tokens
  → Machine D: llama3:8b (wrong model, skip)
  ↓
Routes to Machine B (best match on model + latency + reliability)
  ↓
Machine B runs inference, returns result
  ↓
Machine B earns 0.08 credits. Agent A's balance debited 0.08 credits.
Platform keeps 15% routing fee.
```

Inference routing is live with retry/fallback (top 3 candidates), reputation penalties on failure (-1.0), and reputation boost on success (+0.1). Network storage works the same way — Agent A stores a file via the coordinator, which routes it to Agent B's disk and settles credits at 1mc/KB with 15% platform fee. Distributed compute and agent-to-agent messaging are planned.

### The Credit Economy

Credits are internal (a number in a database). No blockchain, no tokens, no SEC risk, no gas fees.

**Earning:**
| Resource | Contribution | Estimated Earnings |
|----------|-------------|-------------------|
| GPU (3090) | Serve inference 12hrs/day | $5-15/day |
| GPU (4090) | Serve inference 12hrs/day | $10-25/day |
| Storage | 200GB allocated | $2-5/month |
| CPU | 8 cores, batch jobs | $1-5/day |
| Phone (flagship) | Storage + embeddings + small inference overnight | ~150 credits/night |
| Phone (any age) | Vector storage + similarity search overnight | ~30 credits/night |

**Phone economics:** Zero marginal cost. The phone is already plugged in, on WiFi, fully charged. The extra electricity for compute during overnight charging is a rounding error (~$0.005/night). Phone contributors come out ahead every single day.

**Spending:**
- Inference from the network (50% of cloud API pricing)
- Memory and storage
- Other agents' services via directory
- Task scheduling, premium features
- Cash out via USDC (Phase 4)

**Platform take:** 15% routing fee on all transactions.

### The Flywheel

```
Desktop path:
  Agent joins, contributes idle GPU → earns credits immediately
    → Uses credits for memory/storage/inference → gets value
      → Discovers other agents in directory → hires them
        → More demand → more credits to contributors → more agents join

Phone path:
  Download app → 3-tap setup → phone earns credits while you sleep
    → Chat with your agent next day using earned credits
      → "This is amazing and FREE?" → tell friends
        → Friends download → more supply + more demand
          → Network gets faster → agent gets better → tell more friends
```

Two-sided marketplace where supply and demand reinforce each other. The phone path is the mass-market engine: every new phone is both a supply node AND a demand user. The same person earns and spends, and comes out ahead because phone contribution has zero marginal cost.

## Why This Wins

### The Two-Flywheel Strategy

> Full strategy: [`docs/two-flywheel-strategy.md`](docs/two-flywheel-strategy.md)

OpenFerris runs two interlocking flywheels:

**Flywheel 1 (Agents):** Developers discover OpenFerris → install CLI → agents gain memory + inference → agents tell other agents → ecosystem grows → more demand → Flywheel 2 activates.

**Flywheel 2 (Phones):** Person downloads app → 3-tap setup → phone earns credits overnight → chats with AI during the day → "this is amazing and FREE?" → tells friends → more supply + demand → network gets better → Flywheel 1 benefits.

Both flywheels feed the same shared network (coordinator, credit economy, inference routing, memory network, agent directory). Neither works alone. Flywheel 1 creates the network. Flywheel 2 scales it.

### vs. OpenRouter
OpenRouter routes to **cloud** providers (Anthropic, OpenAI, Google). We route to **local** providers (your Ollama, your neighbor's GPU). They charge a markup on API pricing. We enable inference at a fraction of API cost by tapping idle hardware. They're complementary, not competitive — agents can use OpenRouter for cloud models and OpenFerris for local/cheap inference.

### vs. DePIN Networks (Nosana, Akash, Render, Aethir)
They proved the economics work ($40M/quarter at Aethir, 68% cost reduction at Nosana). But they all require: human setup, crypto wallets, KYC, token staking, manual node configuration. OpenFerris: agent IS the node operator. One `curl | sh`. No crypto. No KYC. Zero friction. And they don't have 7 billion phones.

### vs. Memory Platforms (Mem0, Zep, Letta)
Point solutions. Mem0 raised $24M to solve memory alone. We bundle memory WITH inference routing, storage, scheduling, directory, and economy. Why use 5 services when one binary does it all?

### vs. Agent Frameworks (CrewAI, AutoGPT, LangGraph)
Frameworks help you **build** agents. We give agents **infrastructure**. They're complementary — build your agent with CrewAI, then plug in OpenFerris for memory, inference, and earnings. We're the backend, not the framework.

### The Five Moats

1. **The Phone Network.** Nobody else has phones contributing to AI infrastructure. You need the agent network first to create something worth contributing TO. 6+ months head start.
2. **Two-Sided Same-Person Marketplace.** Every phone user is both supply AND demand. Same app. Same install. Zero marginal cost for contributors means the economy self-balances.
3. **Agent Distribution Network.** Presence in 11 discovery channels (MCP registries, ClawHub, Moltbook, LangChain, LiteLLM, IDE marketplaces, package managers). Early presence compounds.
4. **Accumulated Memories.** Every agent accumulates memories. Every phone stores vectors. After 6 months with 10,000 memories, switching cost is enormous.
5. **Open Source Community.** Fork the code — you can't fork the community, the network, or the credit economy.

See [`docs/mobile-supply.md`](docs/mobile-supply.md) for the phone supply thesis and [`docs/two-flywheel-strategy.md`](docs/two-flywheel-strategy.md) for the full competitive analysis.

## Architecture

> Full architecture spec: [`docs/architecture.md`](docs/architecture.md)
> Product requirements (PRD): [`docs/PRD.md`](docs/PRD.md)
> Canonical implementation spec: [`docs/spec-v1.md`](docs/spec-v1.md)
> Build-start gate checklist: [`docs/build-readiness-checklist.md`](docs/build-readiness-checklist.md)
> Documentation index: [`docs/DOCS_INDEX.md`](docs/DOCS_INDEX.md)
> Agent interoperability contract: [`docs/agent-interoperability.md`](docs/agent-interoperability.md)
> Honest gap analysis: [`docs/gap-analysis.md`](docs/gap-analysis.md)
> Launch plan: [`docs/launch-plan.md`](docs/launch-plan.md)
> Agent distribution plan (Full 11-door map): [`docs/agent-distribution.md`](docs/agent-distribution.md)
> Mobile supply strategy (phone as node): [`docs/mobile-supply.md`](docs/mobile-supply.md)
> Two-flywheel growth strategy: [`docs/two-flywheel-strategy.md`](docs/two-flywheel-strategy.md)
> Token & crypto defense strategy: [`docs/token-strategy.md`](docs/token-strategy.md)

### Design Principles

1. **Library-first.** The Rust core is a library (`libferris`). The CLI, Docker image, and mobile apps are thin wrappers. Build once, deploy everywhere.
2. **Local-first.** Memory, storage, and agent state work offline. Network enhances but isn't required.
3. **Always free on-device.** The local agent costs nothing. The network is the product.
4. **Single binary.** No Docker, no databases to install, no config files. `curl | sh` and running.
5. **MCP-native.** Every capability is an MCP tool. Works with any LLM immediately.
6. **Route, don't run.** Like OpenRouter, we're a routing layer. We don't own compute. Capital-efficient.
7. **Rust all the way down.** Memory safety, fearless concurrency, single binary, minimal footprint.

### Tech Stack

| Component | Technology | Why |
|-----------|-----------|-----|
| Core library | `libferris` | Library-first. CLI, Docker, mobile, WASM are thin wrappers. |
| HTTP/API | Axum | Async, fast, Rust-native |
| MCP Server | rmcp | Official Rust MCP SDK |
| Local DB | SQLite | Zero-config, embedded, semantic vector search via fastembed |
| Identity | ed25519-dalek | Ed25519 keypairs. One `ferris init`, no accounts. |
| Encryption | AES-256-GCM (aes-gcm + hkdf) | Data encrypted at rest, key derived from Ed25519 identity |
| Content Addressing | blake3 | Fast hashing for content-addressed storage and dedup |
| Object Storage | Local FS + Cloudflare R2 | Local-first; R2 backs network storage (Phase 1-3) |
| Embeddings | fastembed (AllMiniLM-L6-V2) | Local 384-dim embeddings via ONNX, no API calls |
| Task Engine | Tokio + croner | Async cron scheduler with background execution loop |
| Inference Bridge | Ollama/vLLM auto-detection | Detect local inference, register as network provider |
| Networking | libp2p or QUIC | P2P communication, NAT traversal |
| Crypto (Phase 4) | Alloy + Base L2 | USDC wallets for optional cashout |

### System Topology

```
                TWO BINARIES
  ┌──────────────────────────────────────────────┐
  │       ferris-coordinator (BUSL-1.1)          │
  │       (Axum service, EC2 t3.medium)           │
  │                                              │
  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │
  │  │ Registry │ │ Router   │ │ Credit Ledger│ │
  │  │ (agents, │ │ (infer + │ │ (earn/spend/ │ │
  │  │  models, │ │  storage │ │  balance)    │ │
  │  │  caps)   │ │  routing)│ │              │ │
  │  └──────────┘ └──────────┘ └──────────────┘ │
  └──────────────────┬───────────────────────────┘
                     │ HTTPS / QUIC
         ┌───────────┼───────────┬───────────┐
         ▼           ▼           ▼           ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
    │ Node A  │ │ Node B  │ │ Node C  │ │ Phone D │
    │ 🦀      │ │ 🦀      │ │ 🦀      │ │ 📱      │
    │ Ollama  │ │ vLLM    │ │ CPU-only│ │ Android │
    │ 3090    │ │ A100    │ │ 32 cores│ │ NPU     │
    │ 500GB   │ │ 1TB     │ │ 2TB     │ │ 128GB   │
    └─────────┘ └─────────┘ └─────────┘ └─────────┘
    Desktop nodes: single `ferris` binary (MIT)
    Phone nodes: Android (Kotlin + JNI) / iOS (Swift + FFI)
    All built on libferris — same library, multiple frontends.

    Phone nodes contribute while charging on WiFi:
    vector storage, embeddings, verification, small inference.
    Zero marginal cost. The network's largest growth engine.
```

**Licensing:** `ferris` node binary is MIT. `ferris-coordinator` is BUSL-1.1 (source-available, self-hostable). Protocol spec is open.

Infrastructure costs stay under $100/month until 10,000+ nodes. The coordinator runs on EC2 t3.medium at api.openferris.com.

### Security

- Ed25519 identity from `ferris init` — no passwords, no accounts
- Content-addressed storage via blake3
- **Data encrypted at rest:** AES-256-GCM via `ferris-crypto` crate. Encryption key derived from Ed25519 signing key using HKDF-SHA256. Applied to memory values and stored file contents.
- Agent-to-agent messages encrypted in transit
- Inference verification: proof-of-inference via sampling (known prompts, verified outputs)
- Challenge-response for storage verification

## Revenue Model

### Free Tier (Always Free)
- 1,000 memories, 100MB storage, 10 scheduled tasks
- Directory listing
- Contribute resources, earn credits
- Use credits for network services

### Pro ($9/agent/month)
- Unlimited memory, 10GB storage, 1,000 task executions
- Priority inference routing (your requests routed first)
- SLA guarantees on inference latency

### Team ($29/month)
- 10 agents included
- Shared memory namespace
- Inter-agent workflows
- Analytics dashboard

### Platform Fees
- 15% routing fee on all credit transactions (the OpenRouter model)
- Usage-based: $0.001/memory op, $0.015/GB storage, $0.002/task execution
- Inference: 50% of cloud API-equivalent pricing

### Unit Economics
At 1,000 nodes (50% free, 40% Pro, 10% Team):
- Subscription revenue: ~$4,900/month
- Routing fees: ~$1,600/month (assuming modest transaction volume)
- Total: ~$6,500/month
- Infrastructure: <$100/month
- **Margin: 65x**

## Roadmap

### Phase 1: Local Agent + Memory (Weeks 1-3) ✅
*"Install Ferris, get an agent that remembers"*

- [x] Single Rust binary: `ferris` CLI
- [x] Local SQLite memory with `remember`, `recall`, `forget` + semantic search
- [x] Local file storage with `store`, `retrieve`, `list`
- [x] MCP server exposing all local tools
- [x] `ferris init` → agent_id + keypair generation
- [x] Resource auto-detection (Ollama/vLLM, GPU, disk, CPU)
- [x] `curl | sh` installer for Linux/macOS/Windows
- [x] README, docs

**Milestone:** Any Claude/ChatGPT agent connects via MCP and has persistent memory across sessions.

### Phase 2: Network + Inference Routing (Weeks 4-6) ✅
*"Your idle GPU starts earning. Your phone starts earning while you sleep."*

- [x] Coordinator service: agent registry, health checks, request routing
- [x] Inference routing: local Ollama → registered as network provider → serves requests
- [x] OpenAI-compatible API endpoint (drop-in replacement for consuming agents)
- [ ] **Register as OpenRouter provider** — instant demand from existing user base
- [ ] **LiteLLM provider plugin** — reaches every serious AI developer
- [ ] **LangChain/LlamaIndex memory backend packages** — one-import persistent memory
- [x] Credit system: earn from contributing, spend on network services
- [x] `ferris start` (one-command onboarding) and `ferris status` commands
- [x] Dashboard endpoint (earnings, network stats)
- [ ] MCP registry listings: list each capability separately (Memory, Inference, Storage)
- [ ] **Android app MVP (Tier 1):** vector storage + similarity search + chat interface
- [ ] Pro tier launches

**Milestone:** Agent A sends inference request → routed to Agent B's idle GPU → B earns credits. OpenFerris coordinator routes and settles credit transactions.

### Phase 3: Tasks + Directory + Storage + Full Economy (Weeks 7-10) — In Progress
*"Agents finding and hiring each other. Distributed storage. Phones embedding and verifying."*

- [x] Task scheduling: `schedule_task`, cron engine
- [x] Agent directory: active agent listing
- [x] Network storage routing: store/retrieve files on other nodes via coordinator proxy
- [x] Storage credit settlement: 1mc/KB with 15% platform fee
- [x] `network_objects` table for distributed file tracking
- [x] Inference retry/fallback: top 3 candidates, reputation penalties (-1.0) on failure
- [x] Rate limiting: ConcurrencyLimit (256) + 10MB body limit on coordinator
- [x] Credit spending: hire agents, pay for services through directory
- [ ] Agent-to-agent encrypted messaging
- [ ] SSE streaming for inference
- [ ] **S3-compatible storage endpoint** — unlocks rclone, backup tools, and every S3 client
- [ ] **GitHub Actions self-hosted runner integration** — cheap CI/CD from the network
- [ ] **Open WebUI / Jan / LobeChat provider config** — "OpenFerris Network" option in popular LLM UIs
- [ ] **Android app Tier 2-3:** on-device embedding generation + inference verification
- [ ] Team tier launches

**Milestone:** Agent discovers another agent, hires it for a task, pays in credits. Distributed storage operational via coordinator. Phone network handles bulk embedding generation and verification. S3-compatible storage live. Full economic loop.

### Phase 4: Financial Layer + Scale (Weeks 11-16)
*"Credits become real money. Phones run inference."*

- [ ] USDC wallets (Alloy + Base L2), credits → USDC cashout
- [ ] Compute brokerage marketplace with public pricing
- [ ] **Virtual Kubelet provider** — K8s workloads burst onto OpenFerris network
- [ ] **Storj/Filecoin integration** — join existing storage economies
- [ ] **Ray cluster integration** — academic/ML research compute bursting
- [ ] **Android app Tier 4:** on-device small model inference (Cactus/llama.cpp + NPU)
- [ ] **iOS app launch:** Tier 1-2 (storage + embeddings) + foreground contribution mode
- [ ] Enterprise API, Windows support
- [ ] Security audit

**Milestone:** Agent owner cashes out first $100 in USDC. Phone network serving billions of tokens nightly. iOS app live. K8s workloads bursting onto OpenFerris. Storage serving Filecoin deals.

## Distribution Strategy

### The OpenRouter Hack (Demand on Day One)

The hardest part of any two-sided marketplace is bootstrapping demand. We skip that entirely by registering as a **provider on OpenRouter**.

```
Developer using OpenRouter → picks "llama3:70b" → sees providers:
  Together AI:  $0.90/M tokens
  Fireworks:    $0.88/M tokens
  OpenFerris:   $0.40/M tokens  ← cheapest, routed to distributed network
```

OpenRouter already requires providers to expose an OpenAI-compatible API endpoint — that's our coordinator. We register as a provider, OpenRouter sends us requests, we route them to our nodes, nodes earn credits, more nodes join, prices drop, we become even more competitive. **The flywheel spins using someone else's demand engine.**

**The economics stack:**
```
Cloud API price (Together AI):        $0.90/M tokens
OpenFerris on OpenRouter:             $0.40/M tokens
  → OpenRouter takes ~15%:            $0.06
  → OpenFerris keeps:                 $0.34
    → Node earns 80-85%:              $0.27-0.29
    → Platform keeps:                 $0.05-0.07

Node's actual cost (electricity):     ~$0.03-0.08/M tokens
Node profit: ~$0.20/M tokens pure margin on hardware they already own
```

Everyone wins. Developer gets inference at less than half cloud price. Node owner profits from idle GPU. OpenRouter gets their cut. We get ours. And we didn't have to acquire a single customer.

**What we offer on OpenRouter:** Every open-weight model (Llama, Mistral, Qwen, DeepSeek, Gemma) at 40-60% less than cloud providers.

**What we can't offer:** Proprietary models (Claude, GPT-4, Gemini) or datacenter-grade P99 latency — but for batch jobs, background tasks, and non-real-time inference, developers will trade latency for 50%+ savings. OpenRouter already shows latency per provider, so users self-select.

**The upsell:** Developers who discover us through OpenRouter start using our direct API for memory, storage, directory, and tasks — things OpenRouter doesn't offer. OpenRouter becomes our top-of-funnel acquisition channel.

### Week 1-2: Developer Seeding
- Show HN: "OpenRouter for local compute — one binary that gives agents memory and earns from your idle GPU"
- r/rust, r/LocalLLaMA, r/MachineLearning
- MCP directories (Smithery, mcp.so)
- Tweet: "Your GPU is idle 20 hours a day. What if it earned money?"

### Week 3-4: Agent Communities + OpenRouter Launch
- Register as OpenRouter provider (Phase 2 coordinator is the endpoint)
- Target AutoGPT, CrewAI users — they already have machines with idle resources
- Integration guides: "Add OpenFerris to your existing agent in 2 minutes"

### Week 5-8: Content + Credibility
- Blog: "Building OpenRouter for Local Compute in Rust"
- Live coding streams, architecture deep-dives
- Comparison posts: "OpenFerris vs Mem0 vs Zep for agent memory"
- Conference submissions (RustConf, AI Engineer Summit)

### Week 9+: Flywheel
- Referral credits (invite nodes, both earn bonus)
- "Powered by OpenFerris" badge
- Monthly transparency reports (nodes, compute contributed, credits earned)
- Partnerships with agent framework projects

### Distribution via Existing Platforms

We're not building a platform that needs its own users. We're building a **supply aggregation layer** that plugs into every existing demand channel. Every integration below is a free distribution channel where someone else already has the customers.

**Inference demand channels:**

| Platform | Integration | Why it works |
|----------|------------|--------------|
| **OpenRouter** | Register as provider (OpenAI-compatible endpoint) | Instant demand. Cheapest option for open-weight models. |
| **LiteLLM** | Custom provider plugin | Every serious AI developer uses it. Already supports Ollama. |
| **Vercel AI SDK / LangChain / LlamaIndex** | Provider packages | One-line config change routes apps to our network. |
| **Open WebUI / Jan / LobeChat** | Provider config option | "OpenFerris Network" alongside Ollama, OpenAI in popular UIs. |
| **Hugging Face Inference** | Compute provider behind HF endpoints | They need GPU capacity for open models. We have it. |
| **Replicate** | Backend provider or competitor | They charge per-second GPU. We undercut with distributed compute. |

**Storage demand channels:**

| Platform | Integration | Why it works |
|----------|------------|--------------|
| **rclone** | S3-compatible endpoint | Millions of users. Mount our network as a backend. Zero effort for them. |
| **Restic / Borg / Duplicati** | S3 backend | Cheap backup destination. 1/10th the cost of S3. |
| **IPFS/Filecoin pinning** | Pinata, web3.storage provider | Real demand and real money. Filecoin pays for storage deals. |
| **Storj** | Node operator integration | Join existing storage economy, funnel earnings to contributors. |

**Compute demand channels:**

| Platform | Integration | Why it works |
|----------|------------|--------------|
| **GitHub Actions** | Self-hosted runner registration | Massive market. Every dev team wants cheaper CI/CD. |
| **Buildkite / CircleCI / GitLab CI** | External runner pool | Same pattern. Network becomes cheap build machines. |
| **Kubernetes (Virtual Kubelet)** | VK provider | Any K8s workload bursts onto our network. Enterprise play. |
| **Ray / Dask** | Cluster worker registration | Academic researchers burst onto our network for extra capacity. |

**Memory demand channels:**

| Platform | Integration | Why it works |
|----------|------------|--------------|
| **LangChain** | `Memory` class backend | Every LangChain app gets persistent memory with one import change. |
| **LlamaIndex** | `StorageContext` backend | Same pattern. Massive framework adoption. |
| **CrewAI / AutoGPT** | Memory backend | These frameworks need persistent memory. We're the distributed option. |
| **Qdrant/Pinecone-compatible API** | REST API compatibility | Apps already using those can switch to us. Lower cost, no managed fee. |

**Meta-level:**

| Platform | Integration | Why it works |
|----------|------------|--------------|
| **MCP registries** | Smithery, mcp.so | Register each capability separately — multiple discovery paths. |
| **Cloudflare Workers / Deno / Vercel Edge** | Backend for serverless agents | They provide runtime, we provide state and compute. |
| **Zapier / Make / n8n** | Task scheduling + agent directory integrations | Brings in the no-code automation crowd. |

**Priority matrix:**

| Priority | Integration | Phase | Effort |
|----------|------------|-------|--------|
| 🔴 Do immediately | OpenRouter, LiteLLM, LangChain/LlamaIndex, MCP registries | 2 | Low |
| 🟡 High-value | GitHub Actions runners, S3 endpoint, Open WebUI/Jan | 3 | Medium |
| 🟢 Enterprise | Virtual Kubelet, Storj/Filecoin, Ray cluster | 4 | High |

## Open Source Strategy

The Supabase/GitLab playbook:

- **Core binary (`ferris`):** MIT/Apache 2.0, fully open source
- **Protocol spec:** Open, anyone can build compatible nodes or coordinators
- **Client libraries:** Open source (Rust, Python, TypeScript)
- **Coordinator:** Source-available, self-hostable
- **Managed network:** openferris.com — the revenue engine

Open source drives adoption. Managed network drives revenue. Network effects make the managed network increasingly valuable — more nodes = cheaper inference = better directory = stronger economy. Self-hosting is possible but less valuable without the network, same as running your own OpenRouter instance.

## The Name

**OpenFerris** carries a double meaning:

1. **Ferris the Crab** — Rust's beloved mascot (CC0 licensed). Crabs share resources in tide pools, molt and grow, have hard shells (security). The Rust community already identifies with crustaceans. Building infrastructure in Rust with a crab name is an instant cultural signal.

2. **Ferris Bueller** — Fun, rebellious, iconic. "Save Ferris" was the grassroots campaign in the movie. Our platform literally saves Ferris — saves the agent's memory, state, data, and economic value.

The "Open" prefix signals: open source, open protocol, open network.

---

## Contributing

- Start here: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Community standards: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- Security reporting: [`SECURITY.md`](SECURITY.md)
- Governance: [`GOVERNANCE.md`](GOVERNANCE.md)
- First PR path: [`docs/first-contribution.md`](docs/first-contribution.md)
- Canonical build spec: [`docs/spec-v1.md`](docs/spec-v1.md)
- Local spin-up guide: [`docs/local-development.md`](docs/local-development.md)

Issues and PRs are triaged automatically with GitHub Actions labels/workflows to keep contributor response times fast and consistent.

---

## References

Built on patterns extracted from 8 open-source projects:

| Project | What We Took |
|---------|-------------|
| [voicebox](https://github.com/jamiepine/voicebox) | Rust audio capture/output, voice cloning, Tauri app shell |
| [personaplex](https://github.com/NVIDIA/personaplex) | Full-duplex streaming architecture, voice persona control |
| [picoclaw](https://github.com/sipeed/picoclaw) | Event-driven agent loop, message bus, multi-channel, tool registry |
| [hermitclaw](https://github.com/brendanhogan/hermitclaw) | Memory stream, reflection hierarchy, autonomous behavior, moods |
| [pi-voice](https://github.com/yukukotani/pi-voice) | STT/TTS provider abstraction, push-to-talk, daemon architecture |
| [pi-mono](https://github.com/badlogic/pi-mono) | Unified LLM API, agent runtime, tool calling, session management |
| [botmaker](https://github.com/jgarzik/botmaker) | Zero-trust key proxy, container orchestration, security model |
| [TinyFish-cookbook](https://github.com/tinyfish-io/TinyFish-cookbook) | Web agent patterns, SSE streaming, goal→JSON extraction |

## TL;DR

OpenRouter aggregates cloud LLMs behind one API. OpenFerris aggregates local compute — Ollama instances, idle GPUs, spare storage, CPU — behind one API. Install one binary. Your agent gets memory, storage, and a network. Your machine earns credits from idle resources. The more nodes that join, the cheaper and more capable the network becomes for everyone.

We don't run compute. We route it. Capital-efficient. Network-effect-driven. Open source.

*Save Ferris.* 🦀
