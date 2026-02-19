# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Network storage routing:** agents can store and retrieve files on other nodes via
  coordinator. Files tracked in `network_objects` table, credits settled at 1mc/KB with
  15% platform fee.
- **Inference retry/fallback:** coordinator tries up to 3 providers on failure, with
  reputation penalties (-1.0) for failing nodes.
- **Rate limiting:** coordinator enforces 256 concurrent request limit and 10MB body
  size limit.
- **Storage credit settlement:** new `settle_storage()` in credit ledger with atomic
  transactions.
- `ferris start` one-command onboarding: auto-initializes, joins the network,
  contributes resources, starts HTTP server, and runs heartbeat — all in one command.
- `contribute_percent` config (default 50%) with `ResourceManifest::contributed()`
  for proportional resource sharing.
- Graceful offline mode: if coordinator is unreachable, node runs local-only
  and retries registration every 60 seconds in the background.
- Cross-platform release workflow (macOS x86_64 + aarch64, Linux x86_64 + aarch64,
  Windows x86_64) with SHA256 checksums.
- Install script (`scripts/install.sh`) for `curl | sh` onboarding.
- `cargo-deny` license and advisory auditing in CI.
- Dependabot for automated dependency updates.
- `CHANGELOG.md` (this file).
- **Semantic memory search:** hybrid text + vector recall using `fastembed`
  (AllMiniLM-L6-V2, 384-dim embeddings). Cosine similarity computed in Rust.
  Graceful fallback to LIKE search if embedding model unavailable.
- **Encryption at rest:** `ferris-crypto` crate with AES-256-GCM. Keys derived
  from Ed25519 identity via HKDF-SHA256. Applied to memory values and stored files.
- **Task execution engine:** background Tokio loop (60s poll) evaluates cron
  expressions via `croner` v3. Actions: `log`, `http`, `webhook`. Run history
  tracked in `task_runs` table.
- **Production coordinator infrastructure:** EC2 t3.medium at api.openferris.com
  with SSM access, hardened security group, encrypted EBS, termination protection,
  and automatic updates.

- **SSE streaming passthrough:** coordinator proxies Server-Sent Events from upstream
  inference providers. Background task extracts usage from final chunk for credit settlement.
- **MCP `infer` tool:** run inference via local Ollama directly from any MCP client.
- **MCP `balance` tool:** query coordinator credit balance from MCP.
- **docker-compose.yml:** local development setup with coordinator + node services.
- **Docker Ubuntu 24.04:** switched Docker base images from Debian bookworm to Ubuntu 24.04
  (glibc 2.39) so ONNX Runtime / fastembed (semantic search) works inside containers.
- **justfile:** common development tasks (build, test, lint, fmt, docker, ci).
- **`POST /v1/embeddings`:** OpenAI-compatible embedding requests routed to nodes, with
  retry/fallback and credit settlement.
- **`POST /api/v1/settle`:** node-reported settlement endpoint. Nodes can report
  token usage after serving inference, with signed auth and credit settlement.
- **Agent-to-agent messaging:** `POST /api/v1/messages/send` queues JSON messages with
  24-hour TTL. `GET /api/v1/messages` polls and delivers queued messages.
- **`message_queue` migration:** new coordinator table for agent messaging.
- **Tests:** 89 tests across all crates. Added tests for MCP server, config, identity,
  settlement, messaging endpoints, inference backend trait, model manager, and candle prompt formatting.
- **Doc comments:** comprehensive `///` documentation on all public types and functions.
- **Embedded inference backend (candle):** pure-Rust inference via `candle-core` and
  `candle-transformers`. Loads quantized GGUF models (Qwen2.5 family). No external daemon
  required — works on desktop and mobile.
- **`InferenceBackend` trait:** unified async trait for all inference backends. `OllamaBackend`
  (renamed from `OllamaProxy`) and `CandleBackend` both implement it. All callers use
  `Arc<dyn InferenceBackend>`.
- **Model auto-download:** RAM-based model selection (0.5B for phones, 1.5B for mid-range,
  3B for desktops). Auto-downloads from HuggingFace Hub to `~/.ferris/models/`.
- **`create_backend()` auto-detection:** `ferris start` probes Ollama first; if unavailable,
  falls back to embedded candle backend with automatic model provisioning.
- **Feature flags:** `ollama`, `candle-backend`, `mobile` Cargo features. `mobile` flag
  excludes Ollama and fastembed for lean mobile builds.

### Changed
- Coordinator URL updated from `api.openferris.dev` to `api.openferris.com`.
- MCP tools expanded from 10 to 12 (`infer`, `balance` added).
- CI uses `--locked` on all cargo commands for deterministic builds.
- Documentation aligned with implementation: corrected API paths, MCP tool list, fee percentages.
- architecture.md routing algorithm corrected to match spec-v1.md (reputation-first scoring).
- economy.md pricing clarified: fixed pricing in Phase 2, dynamic pricing planned for Phase 3+.
- `OllamaProxy` renamed to `OllamaBackend` and refactored to implement `InferenceBackend` trait.
- All inference callers (`server.rs`, `main.rs`, `lib.rs` in ferris-mcp) updated to use
  `Arc<dyn InferenceBackend>` instead of concrete `OllamaProxy`.
- Heartbeat loop uses shared `Arc<dyn InferenceBackend>` instead of re-creating clients.
- `ferris start`, `ferris serve`, `ferris join`, `ferris status` all use `create_backend()`
  for automatic Ollama-vs-candle detection.

### Fixed
- `rand`/`rand_core` dependency conflict with `fastembed` resolved by pinning `rand_core = "0.6"`.
- Windows panic on `dirs::home_dir()` replaced with graceful fallback.
- Production `unwrap()` calls in coordinator response builder, storage path handling, and
  memory mutex replaced with proper error handling.
- All remaining `expect()` calls in production code (coordinator main, OllamaProxy::new,
  CoordinatorClient::new) replaced with `Result`-based error handling.

## [0.1.0] - 2026-02-17

Initial implementation of the OpenFerris platform.

### Added

#### Phase 1 — Local Agent Platform
- `ferris init` — create data directory, config, SQLite DB, Ed25519 identity.
- `ferris serve --transport stdio` — MCP server with 10 tools.
- `ferris serve --transport http` — REST API dev server.
- `ferris status` — agent ID, system resources, Ollama detection, data counts.
- `ferris-memory` — persistent key-value memory with `remember`, `recall`, `forget`.
- `ferris-storage` — content-addressed object storage with blake3 deduplication.
- `ferris-tasks` — scheduled task system with `schedule`, `list`, `cancel`.
- `ferris-mcp` — MCP server exposing all local capabilities via rmcp 0.16.

#### Phase 2 — Coordinator + Network
- `ferris-coordinator` binary — standalone Axum server (BUSL-1.1).
- Agent registry with registration, heartbeat, stale-agent sweeping.
- Canonical routing algorithm (reputation/speed/latency/availability scoring).
- `ferris-credits` — double-entry credit ledger with soft/hard balances,
  atomic SQLite transactions, signup bonus, and availability rewards.
- `ferris-net` — node-to-coordinator client with Ed25519 request signing.
- `ferris-inference` — local Ollama proxy with concurrency limiting.
- `ferris join` — register node with coordinator, auto-detect Ollama models.
- `ferris balance` — query credit balance from coordinator.
- OpenAI-compatible inference endpoints (`/v1/chat/completions`, `/v1/models`).
- Ed25519 signature verification on all protected coordinator endpoints.

#### Phase 3 — Settlement, Reputation, Dashboard
- Credit settlement: 1 millicredit/token with 15% platform fee.
- Reputation adjustments based on inference performance.
- Availability rewards: periodic soft credit awards for active nodes.
- `/api/v1/directory` — active agent listing.
- `/dashboard/stats` — network-wide metrics.

### Security
- Ed25519 identity and request signing from day one.
- Signature verification on coordinator endpoints.
- Atomic credit operations via SQLite transactions (no race conditions).
- All handlers use proper error propagation (zero `.unwrap()` in production).
- Listens on 127.0.0.1 by default (not 0.0.0.0).

[Unreleased]: https://github.com/l3ocifer/openferris/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/l3ocifer/openferris/releases/tag/v0.1.0
