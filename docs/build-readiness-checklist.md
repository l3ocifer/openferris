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
| `ferris-memory` | Done | 8 unit tests |
| `ferris-storage` | Done | 6 unit tests |
| `ferris-tasks` | Done | 5 unit tests |
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

### Features

- Agent registry with registration, heartbeat, stale-agent sweep
- Canonical routing algorithm with reputation/speed/latency/load scoring
- Double-entry credit ledger (soft/hard balances)
- Signup bonus (100 credits), availability rewards (10mc/min)
- Inference settlement with 15% platform fee (atomic transactions)
- Escrow hold/release/refund lifecycle
- Ed25519 signature verification on protected endpoints
- OpenAI-compatible inference routing (coordinator → node → Ollama)

## 9) Quality Gates — ALL PASSING

- [x] `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings
- [x] `cargo test --workspace` — 51 tests, all passing
- [x] No `.unwrap()` calls in production handler code
- [x] Atomic credit operations (SQLite transactions)
- [x] Ed25519 signature verification on coordinator endpoints
- [x] No unused dependencies
- [x] No dead code or unused types
- [x] Configurable via TOML + env overrides (all sections)
- [x] E2E verified: node → coordinator → inference routing → settlement
