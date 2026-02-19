# OpenFerris Build Readiness Checklist

This document converts current docs into a build-start gate.

Status legend:
- [x] Locked / Complete
- [ ] Needs decision
- [~] Drafted, needs implementation detail

## 1) MVP Scope Lock (Phase 1)

Goal: start coding with minimal ambiguity and no cross-phase leakage.

1. [x] Lock Phase 1 deliverables to local node only:
   - `ferris init`
   - local memory (`remember`, `recall`, `forget`)
   - local storage (`store`, `retrieve`, `list`)
   - local tasks (`schedule_task`, `list_tasks`, `cancel_task`)
   - MCP server for local tools
2. [x] Explicitly defer all coordinator/network/economy features to Phase 2.
3. [x] Freeze "non-goals" for Phase 1:
   - no QUIC/libp2p
   - no OpenAI-compatible network endpoint
   - no external provider integrations
   - no cashout/topup

## 2) Spec Conflicts Resolved

1. [x] Platform fee: fixed 15%.
2. [x] Credit bootstrap: 100-credit signup bonus + ongoing availability rewards.
3. [x] Routing formula: `reputation(0.40) + speed(0.25) + latency(0.20) + availability(0.15) + hot_bonus(0.10)`.
4. [x] Reputation scale: `0.0..100.0` (stored as `REAL`), default 50.0.
5. [x] Storage: local object store (Phase 1-3); optional R2 sync deferred.
6. [x] Transport: HTTPS + SSE proxy via coordinator (Phase 2 default).
7. [x] API namespace: `/v1/*` (OpenAI-compat), `/api/v1/*` (control), `/dashboard/*` (metrics).
8. [x] Timeline: week-based roadmap.

## 3) Data Model Frozen

1. [x] Agent identity: UUID v7 + Ed25519 keypair.
2. [x] Coordinator schema: `agents`, `models`, `capabilities`, `credits`, `transactions`, `escrow`.
3. [x] Numeric types: millicredits as `INTEGER` (1 credit = 1000 mc).
4. [x] Timestamps: `INTEGER` unix epoch seconds.

## 4) Build-Ready Engineering Baseline

1. [x] Workspace scaffold with 10 crates.
2. [x] Toolchain pinned, clippy/rustfmt config, CI workflows.
3. [x] Migration strategy: `sqlx` for both node and coordinator.
4. [x] Config strategy: TOML + env overrides for all sections.
5. [x] Error strategy: shared `FerrisError` enum in `ferris-common`.

## 5) Security/Trust Baseline

1. [x] Ed25519 identity generation + persistent storage.
2. [x] Request signing: node signs all coordinator requests.
3. [x] Signature verification: coordinator verifies Ed25519 signatures on protected endpoints.
4. [x] Local secrets: `secret_key_bytes` stored as plaintext in local SQLite (machine-local trust).

## 6) Mobile Supply Readiness (Non-Blocking)

1. [x] Mobile supply strategy documented.
2. [x] Contribution tiers defined (T1-T5).
3. [x] Coordinator schema extensible for mobile nodes.
4. [x] Mobile-specific API endpoints specified.
5. [~] Android app architecture: Kotlin + JNI over `libferris`.
6. [~] `libferris` FFI surface designed.
7. [ ] Android NDK cross-compilation validated.
8. [ ] iOS FFI compilation validated.
9. [x] Contribution policy defined.
10. [x] Platform constraints documented.

## 7) Phase 1 Implementation Status — COMPLETE

### Crates

| Crate | Status | Tests |
|-------|--------|-------|
| `ferris-common` | Done | shared types, errors, config, protocol types |
| `ferris-core` | Done | CLI, config, identity, resources, HTTP server |
| `ferris-crypto` | Done | AES-256-GCM encryption, HKDF key derivation, 6 unit tests |
| `ferris-memory` | Done | 11 unit tests (semantic search, cosine similarity, encryption) |
| `ferris-storage` | Done | 6 unit tests (encryption at rest) |
| `ferris-tasks` | Done | 10 unit tests (cron validation, task execution, run history) |
| `ferris-mcp` | Done | 10 MCP tools via rmcp 0.16 |

### CLI Commands

- `ferris init` — create data dir, config, DB, Ed25519 identity
- `ferris serve --transport stdio` — MCP server
- `ferris serve --transport http` — REST dev server
- `ferris status` — agent ID, resources, Ollama models, data counts

### Node HTTP API

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Health check |
| GET | `/api/v1/status` | Memory/object/task counts |
| POST | `/api/v1/memory/remember` | Store key-value |
| POST | `/api/v1/memory/recall` | Search memories |
| DELETE | `/api/v1/memory/{key}` | Delete memory |
| POST | `/api/v1/storage/store` | Store file (base64) |
| GET | `/api/v1/storage` | List files |
| GET | `/api/v1/storage/{file_id}` | Retrieve file |
| POST | `/api/v1/tasks` | Schedule task |
| GET | `/api/v1/tasks` | List tasks |
| DELETE | `/api/v1/tasks/{task_id}` | Cancel task |
| POST | `/v1/chat/completions` | Inference via Ollama |
| GET | `/v1/models` | List local models |

### MCP Tools

`whoami`, `remember`, `recall`, `forget`, `store`, `retrieve`, `list_files`, `schedule_task`, `list_tasks`, `cancel_task`

## 8) Phase 2 Implementation Status — COMPLETE

### New Crates

| Crate | License | Status | Tests |
|-------|---------|--------|-------|
| `ferris-coordinator` | BUSL-1.1 | Done | 4 routing + 10 integration |
| `ferris-credits` | MIT/Apache | Done | 7 unit tests |
| `ferris-net` | MIT/Apache | Done | 1 signature test |
| `ferris-inference` | MIT/Apache | Done | 3 unit tests |

### New CLI Commands

- `ferris join` — register with coordinator, auto-detect Ollama models
- `ferris balance` — query credit balance

### Coordinator Binary

- `ferris-coordinator` — standalone Axum server on port 8421

### Coordinator API

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Health check |
| POST | `/api/v1/register` | Agent registration |
| POST | `/api/v1/heartbeat` | Heartbeat (signed) |
| GET | `/api/v1/status` | Active agents, models |
| GET | `/api/v1/wallet/balance` | Credit balance (signed) |
| GET | `/api/v1/wallet/history` | Transaction history (signed) |
| GET | `/api/v1/directory` | Active agent directory |
| GET | `/dashboard/stats` | Network-wide metrics |
| GET | `/v1/models` | All network models |
| POST | `/v1/chat/completions` | Routed inference (signed) |
| POST | `/api/v1/network/store` | Store file on network node (signed) |
| GET | `/api/v1/network/files` | List agent's network files (signed) |
| GET | `/api/v1/network/files/{id}` | Retrieve file from network (signed) |

### Features

- Agent registry with registration, heartbeat, stale-agent sweep
- Canonical routing algorithm with reputation/speed/latency/load scoring
- Inference retry/fallback (top 3 candidates, reputation penalties)
- Network storage routing (store/retrieve via coordinator proxy)
- Double-entry credit ledger (soft/hard balances)
- Signup bonus (100 credits), availability rewards (10mc/min)
- Inference settlement with 15% platform fee (atomic transactions)
- Storage settlement at 1mc/KB with 15% platform fee
- Escrow hold/release/refund lifecycle
- Rate limiting (ConcurrencyLimit 256, 10MB body limit)
- Ed25519 signature verification on protected endpoints
- OpenAI-compatible inference routing (coordinator → node → Ollama)

## 9) One-Command Onboarding — COMPLETE

### `ferris start` (combines init + join + serve + heartbeat)

- [x] Auto-initializes node (data dir, config, DB, identity) if not already done.
- [x] Detects system resources (CPU, RAM, GPU, storage) and Ollama models.
- [x] `contribute_percent` config (default 50%) — reports only the contributed portion
  of resources to the coordinator.
- [x] Attempts coordinator registration; on failure, runs local-only with 60s background retry.
- [x] Starts HTTP server and heartbeat loop in parallel.
- [x] CLI flag: `--contribute-percent`, env var: `FERRIS_CONTRIBUTE_PERCENT`.

### Cross-Platform Release

- [x] GitHub Actions release workflow (`.github/workflows/release.yml`):
  macOS x86_64 + aarch64, Linux x86_64 + aarch64, Windows x86_64.
- [x] SHA256 checksums in release artifacts.
- [x] Install script (`scripts/install.sh`): detects OS/arch, downloads binary from GitHub Releases.

### CI Hardening

- [x] `cargo audit` in CI (advisory database check).
- [x] `cargo deny` in CI (license + advisory + ban checks).
- [x] DCO sign-off check on pull requests.
- [x] Dependabot for Cargo + GitHub Actions dependency updates.
- [x] `Cargo.lock` committed (binary workspace).

## 10) Feature Implementation Status

### Semantic Memory Search — COMPLETE

- [x] `fastembed` (AllMiniLM-L6-V2, 384-dim) for embedding generation
- [x] Embeddings stored as BLOB in SQLite `memories.embedding` column
- [x] Cosine similarity computed in Rust (no external extensions needed)
- [x] Hybrid search: vector similarity + text LIKE, combined scoring
- [x] Graceful fallback to text search if embedder unavailable
- [x] Migration: `0002_vector_search.sql`

### Encryption at Rest — COMPLETE

- [x] `ferris-crypto` crate: AES-256-GCM encrypt/decrypt
- [x] Key derivation: HKDF-SHA256 from Ed25519 secret key
- [x] Memory values encrypted before storage, decrypted on read
- [x] File contents encrypted before writing to disk
- [x] Embeddings generated from plaintext BEFORE encryption
- [x] Feature-gated: `encryption` feature on `ferris-memory` and `ferris-storage`

### Task Execution Engine — COMPLETE

- [x] `croner` v3 for cron expression parsing and validation
- [x] Background Tokio task polls every 60s for due tasks
- [x] Task actions: `log`, `http`, `webhook`
- [x] Run history recorded in `task_runs` table
- [x] `last_run_at` tracking prevents duplicate execution
- [x] Migration: `0003_task_execution.sql`

## 11) Quality Gates — ALL PASSING

- [x] `cargo clippy --workspace --all-targets` — zero warnings
- [x] `cargo test --workspace` — 66 tests, all passing
- [x] No `.unwrap()` calls in production handler code
- [x] Atomic credit operations (SQLite transactions)
- [x] Ed25519 signature verification on coordinator endpoints
- [x] No unused dependencies
- [x] No dead code or unused types
- [x] Configurable via TOML + env overrides (all sections)
- [x] E2E verified: node → coordinator → inference routing → settlement
- [x] No TODO/FIXME/HACK comments in codebase
- [x] SECURITY.md with 24hr acknowledgment SLA
- [x] CHANGELOG.md following Keep a Changelog format
- [x] `deny.toml` for license compliance auditing

## 13) Distributed Network MVP — COMPLETE

- [x] Inference routing with retry/fallback (top 3 candidates)
- [x] Reputation penalties on failure (-1.0), boost on success (+0.1)
- [x] Network storage routing (store/retrieve via coordinator proxy)
- [x] Storage credit settlement (1mc/KB, 15% platform fee)
- [x] Rate limiting (ConcurrencyLimit 256, 10MB body limit)
- [x] `network_objects` table for distributed file tracking

## 14) Final Hardening — COMPLETE

- [x] SSE streaming passthrough for inference responses (with background credit settlement)
- [x] MCP `infer` tool (local Ollama inference via MCP)
- [x] MCP `balance` tool (query coordinator credit balance via MCP)
- [x] 12 MCP tools total: whoami, remember, recall, forget, store, retrieve, list_files, schedule_task, list_tasks, cancel_task, infer, balance
- [x] `rand`/`rand_core` dependency conflict resolved (explicit `rand_core = "0.6"`)
- [x] CI uses `--locked` for deterministic builds
- [x] All production `unwrap()` calls replaced with proper error handling
- [x] Windows-safe `dirs::home_dir()` fallbacks (no panics)
- [x] `cargo fmt`, `cargo clippy -D warnings`, `cargo test` all green
- [x] `cargo audit` and `cargo deny` passing in CI
- [x] Documentation aligned with actual implementation (API paths, MCP tools, fee %)
- [x] docker-compose.yml for local node + coordinator testing

## 12) Production Infrastructure — COMPLETE

### Coordinator Hosting
- [x] EC2 t3.medium (2 vCPU, 4GB RAM, 20GB gp3 encrypted EBS) in us-east-1
- [x] Elastic IP: 54.196.139.79
- [x] DNS: `api.openferris.com` → A record in Route53 (PIE account)
- [x] Domain: `openferris.com` registered via Amazon Registrar (auto-renew, WHOIS privacy)

### Security Hardening
- [x] AWS SSM access (no SSH port exposed)
- [x] Security group: ports 80, 443, 8421 only (no port 22)
- [x] Password authentication disabled in sshd
- [x] Root login disabled
- [x] Termination protection enabled
- [x] EBS encrypted at rest
- [x] Automatic security updates via dnf-automatic
- [x] Detailed CloudWatch monitoring enabled

### Scaling Path
- Phase 1 (→10K agents): Single EC2 + SQLite (~$32/mo)
- Phase 2 (10K→100K): ALB + EC2 + RDS PostgreSQL (~$60-100/mo)
- Phase 3 (100K→1M): ALB + Auto Scaling Group + RDS (~$200-500/mo)
- Phase 4 (1M+): ECS Fargate + Aurora Serverless (scales with demand)
