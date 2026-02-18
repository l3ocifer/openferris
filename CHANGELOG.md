# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
