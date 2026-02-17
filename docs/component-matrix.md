# Component Matrix — Reference Repos → OpenFerris

> Document authority note: this is a reference mapping doc, not a source of implementation truth.

Cross-reference of components across the 8 reference repos and how they map to OpenFerris's MCP-native, Rust-first architecture.

## Legend

- **Lang**: Primary language of the component
- **Ferris Use**: How this maps to OpenFerris (`port` = rewrite in Rust, `adapt` = use patterns, `wrap` = FFI/subprocess, `skip` = not needed)

## MCP & Agent Infrastructure

| Component | Repo | File/Module | Lang | Ferris Use | Ferris Crate | Notes |
|-----------|------|-------------|------|-----------|-------------|-------|
| Event-Driven Agent Loop | picoclaw | `pkg/agent/loop.go` | Go | **adapt** | `ferris-core` | Pattern for MCP tool dispatch |
| Message Bus (Pub/Sub) | picoclaw | `pkg/bus/bus.go` | Go | **port** | `ferris-tasks` | `tokio::broadcast` for event subscriptions |
| Tool Registry | picoclaw | `pkg/tools/registry.go` | Go | **adapt** | `ferris-mcp` | MCP tool definitions via `rmcp` |
| Context Builder | picoclaw | `pkg/agent/` | Go | **adapt** | `ferris-mcp` | Token-aware context for tool responses |
| Session Manager | picoclaw | `pkg/session/` | Go | **adapt** | `ferris-memory` | Conversation context in SQLite |
| Workspace Sandbox | picoclaw | `pkg/tools/` | Go | **adapt** | `ferris-storage` | Path validation for object store |
| Unified LLM API | pi-mono | `packages/ai/` | TS | **adapt** | `ferris-inference` | OpenAI-compatible routing |
| Provider Registry | pi-mono | `packages/ai/src/api-registry.ts` | TS | **adapt** | `ferris-inference` | Ollama/vLLM detection + proxy |
| Agent Runtime Core | pi-mono | `packages/agent/` | TS | **adapt** | `ferris-coordinator` | Agent lifecycle management |
| Streaming Response | pi-mono | `packages/ai/` | TS | **port** | `ferris-inference` | SSE streaming passthrough |
| Config (JSON + env) | picoclaw | `pkg/config/config.go` | Go | **port** | `ferris-core` | TOML + env overrides |

## Memory System

| Component | Repo | File/Module | Lang | Ferris Use | Ferris Crate | Notes |
|-----------|------|-------------|------|-----------|-------------|-------|
| Memory Stream | hermitclaw | `memory.py` | Python | **adapt** | `ferris-memory` | Pattern → SQLite + vectorlite |
| Three-Factor Retrieval | hermitclaw | `memory.py` (retrieve) | Python | **adapt** | `ferris-memory` | Semantic search via vectorlite |
| Embedding Storage | hermitclaw | `memory.py` | Python | **port** | `ferris-memory` | `ort` embeddings + vectorlite index |
| Cosine Similarity | hermitclaw | `memory.py` | Python | **port** | `ferris-memory` | vectorlite handles this natively |
| Importance Scoring | hermitclaw | `memory.py` | Python | **adapt** | `ferris-memory` | Future: layered importance for recall ranking |
| Reflection System | hermitclaw | `memory.py` + `brain.py` | Python | **skip** | — | Phase 1 focuses on simple remember/recall |
| Session History | picoclaw | `pkg/session/` | Go | **adapt** | `ferris-memory` | Conversation windows in SQLite |
| Identity Persistence | hermitclaw | `identity.json` | JSON | **port** | `ferris-core` | Ed25519 keypair + agent_id |

## Storage & Security

| Component | Repo | File/Module | Lang | Ferris Use | Ferris Crate | Notes |
|-----------|------|-------------|------|-----------|-------------|-------|
| Zero-Trust Key Proxy | botmaker | `proxy/` | TS | **adapt** | `ferris-coordinator` | Pattern for inference request auth |
| AES-256-GCM Encryption | botmaker | `proxy/src/crypto/` | TS | **port** | `ferris-storage` | `aes-gcm` crate for data at rest |
| Token-Based Auth | botmaker | `proxy/src/routes/proxy.ts` | TS | **adapt** | `ferris-net` | Ed25519 signed requests |
| Container Lifecycle | botmaker | `src/services/DockerService.ts` | TS | **skip** | — | Single binary, no containers |
| SQLite Database | botmaker | `src/db/` | TS | **port** | `ferris-memory` | `sqlx` for all SQLite |
| Cron/Scheduled Tasks | picoclaw | `pkg/cron/` | Go | **port** | `ferris-tasks` | `tokio-cron-scheduler` |
| Heartbeat System | picoclaw | `pkg/heartbeat/` | Go | **port** | `ferris-net` | Node health reporting |
| SSE Stream Parser | TinyFish | Various `lib/` | TS | **port** | `ferris-inference` | `reqwest` SSE for inference streaming |
| Web Agent Goals | TinyFish | Goal templates | TS | **adapt** | `ferris-tasks` | Task chaining pattern |

## Voice & Audio (Future Phase)

| Component | Repo | File/Module | Lang | Ferris Use | Ferris Crate | Notes |
|-----------|------|-------------|------|-----------|-------------|-------|
| System Audio Capture (macOS) | voicebox | `tauri/src-tauri/src/audio_capture/macos.rs` | Rust | **future** | — | Already Rust, ScreenCaptureKit |
| System Audio Capture (Win) | voicebox | `tauri/src-tauri/src/audio_capture/windows.rs` | Rust | **future** | — | Already Rust, WASAPI |
| System Audio Capture (Linux) | voicebox | `tauri/src-tauri/src/audio_capture/linux.rs` | Rust | **future** | — | Already Rust, ALSA/PulseAudio |
| Multi-Device Audio Output | voicebox | `tauri/src-tauri/src/audio_output.rs` | Rust | **future** | — | Already Rust, cpal-based |
| Full-Duplex WebSocket | personaplex | `moshi/server.py` | Python | **future** | — | Voice channel for agents |
| 3-Loop Streaming Pipeline | personaplex | `moshi/server.py` | Python | **future** | — | recv/process/send pattern |
| Opus Codec | personaplex | `sphn` | Python/C | **future** | — | `audiopus` Rust bindings |
| Whisper STT (Local) | pi-voice | `src/services/stt.ts` | TS | **future** | — | `whisper-rs` |
| Multi-Provider TTS | pi-voice | `src/services/tts.ts` | TS | **future** | — | Provider trait pattern |
| Push-to-Talk Hook | pi-voice | `src/services/fn-hook.ts` | TS | **future** | — | `rdev` Rust crate |

## Network & Economy (New — Phase 2+)

| Component | Inspiration | Ferris Crate | Implementation | Notes |
|-----------|-------------|-------------|---------------|-------|
| Resource Detection | PicoClaw ultra-lightweight | `ferris-core` | `sysinfo` + `nvml-wrapper` | GPU, RAM, disk, CPU auto-detect |
| Ollama/vLLM Detection | New | `ferris-inference` | HTTP probe localhost | Auto-detect local inference |
| OpenAI-Compatible API | OpenRouter pattern | `ferris-coordinator` | Axum endpoint | Drop-in replacement |
| Inference Routing | Inspired by Nosana/Aethir | `ferris-coordinator` | Weighted scoring | latency + capacity + reputation |
| Inference Metering | New | `ferris-inference` | Token counting | Input + output per request |
| Credit Ledger | BotMaker DB patterns | `ferris-credits` | SQLite double-entry | Every tx: debit + credit |
| Escrow | New | `ferris-credits` | Hold during job | Release on completion |
| Dynamic Pricing | New | `ferris-credits` | Supply/demand multiplier | Floor/ceiling constraints |
| Signup Bonus | New | `ferris-credits` | 100 credits | Zero-barrier onboarding |
| Batch Settlement | New | `ferris-credits` | Aggregate every 60s | Reduce write amplification |
| Content-Addressed Storage | Storj/IPFS pattern | `ferris-storage` | `blake3` hashing | Dedup + integrity |
| Storage Quotas | New | `ferris-storage` | Per-node configurable | Contributor controls allocation |
| Agent Directory | New | `ferris-directory` | Coordinator registry | Semantic capability matching |
| Agent Messaging | New | `ferris-directory` | Encrypted relay | Via coordinator |
| Reputation System | New | `ferris-net` | Uptime + quality scoring | Trust builds over time |
| Sybil Resistance | New | `ferris-net` | Hardware fingerprint | One machine = one node |
| QUIC Transport | New | `ferris-net` | `quinn` crate | Encrypted, low-latency |
| USDC Integration | New (Phase 4) | `ferris-credits` | `alloy` + Base L2 | Optional cashout |

## Frontend/UI

| Component | Repo | File/Module | Lang | Ferris Use | Notes |
|-----------|------|-------------|------|-----------|-------|
| Desktop App Shell | voicebox | `tauri/` | Rust | **future** | Tauri 2.0, potential Phase 5 |
| React Frontend | voicebox | `app/` | TS/React | **adapt** | Dashboard patterns |
| Bot Dashboard | botmaker | `dashboard/` | TS/React | **adapt** | Earnings/status UI |
| Web UI Components | pi-mono | `packages/web-ui/` | TS | **skip** | Dashboard is simpler |
| Pixel Art Room | hermitclaw | `frontend/src/GameWorld.tsx` | TS | **skip** | Not relevant |
| TUI Library | pi-mono | `packages/tui/` | TS | **skip** | CLI via clap is sufficient |

## Component Priority by Phase

| Phase | Key Components | Primary Reference Repos |
|-------|---------------|------------------------|
| **1: Local + Memory** | MCP server, SQLite memory, object storage, config | PicoClaw, HermitClaw, BotMaker |
| **2: Network + Inference** | Inference routing, credit ledger, coordinator, QUIC | pi-mono, BotMaker, TinyFish |
| **3: Tasks + Directory** | Cron engine, agent directory, messaging, workflows | PicoClaw (cron, heartbeat), TinyFish (goals) |
| **4: Financial + Scale** | USDC integration, cross-platform, mobile, enterprise | BotMaker (proxy patterns) |
| **Future: Voice** | Full-duplex audio, STT/TTS, Opus streaming | Voicebox, PersonaPlex, pi-voice |
