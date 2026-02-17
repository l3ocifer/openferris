# OpenFerris Build Readiness Checklist

This document converts current docs into a build-start gate.

Status legend:
- [x] Locked
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

## 2) Spec Conflicts To Resolve Before First Commit

These are the current blockers for "ready to build".

1. [x] Platform fee policy:
   - Docs currently use both fixed `15%` and variable `10-20%`.
   - Decision needed: one canonical rule for Phase 2.
2. [x] Credit bootstrap policy:
   - Docs include both "availability rewards" and "100 signup bonus".
   - Decision needed: keep one, or keep both with exact ordering/rules.
3. [x] Routing formula:
   - Competing versions exist:
     - hot/installed/latency/capacity/reputation weighting
     - reputation/speed/latency/load plus hot bonus
   - Decision needed: one canonical algorithm for implementation.
4. [x] Reputation scale:
   - Some docs use `0..100`, others `0.0..1.0`.
   - Decision needed: canonical numeric range and storage type.
5. [x] Storage positioning:
   - "Distributed node-backed storage" vs "R2-backed storage in early phases".
   - Decision needed: canonical statement for Phase 1-3.
6. [x] Transport:
   - "HTTPS/SSE proxy first" vs "QUIC/libp2p in early network phase".
   - Decision needed: Phase 2 default transport and Phase 3 upgrade path.
7. [x] API namespace:
   - mixed endpoint families (`/agents/*`, `/api/v1/*`, `/v1/*`).
   - Decision needed: one path convention for node, consumer, and dashboard APIs.
8. [x] Timeline mismatch:
   - some docs use week-based phases, one uses month-based phases.
   - Decision needed: single roadmap timeline format.

## 3) Canonical Data Model Freeze (Needed Before DB Migrations)

1. [x] Agent identity format:
   - UUID v7 vs derived key hash style.
2. [x] Coordinator schema baseline:
   - choose one canonical set of tables/columns for:
     - `agents`
     - `models` (or split `hot_models` + `installed_models`)
     - `capabilities`
     - `credits/balances`
     - `transactions`
     - `escrow`
3. [x] Numeric types:
   - define exact types for money/credits (`REAL` vs fixed-point integer).
4. [x] Time format:
   - define canonical timestamp format (`INTEGER unix` vs `TEXT datetime`).

## 4) Build-Ready Engineering Baseline

1. [x] Create workspace scaffold:
   - root `Cargo.toml` workspace
   - `crates/` directory with Phase 1 crates only:
     - `ferris-common`
     - `ferris-core`
     - `ferris-memory`
     - `ferris-storage`
     - `ferris-tasks`
     - `ferris-mcp`
2. [x] Toolchain and quality gates:
   - rust-toolchain pinned
   - `clippy` and `rustfmt` config
   - CI for `check`, `clippy`, `test`
3. [x] Migration strategy:
   - `sqlx` migrations directory
   - initial node DB migration
4. [x] Config strategy:
   - canonical `config.toml`
   - env override naming convention
5. [x] Error strategy:
   - shared `thiserror` enums in `ferris-common`
   - no `anyhow` in shared APIs

## 5) Security/Trust Baseline For First Release

1. [x] Identity:
   - Ed25519 key generation and persistent storage format.
2. [x] Request signing:
   - canonical signed payload format for future coordinator calls.
3. [x] Local secrets:
   - define at-rest handling for private key.
4. [x] Explicit trust statement:
   - add one short "what we do and do not protect" section in README.

## 6) Mobile Supply Readiness (Non-Blocking, Phase 2)

1. [x] Mobile supply strategy documented (`docs/mobile-supply.md`).
2. [x] Contribution tiers defined (T1-T5) with credit rates.
3. [x] Coordinator schema extended for mobile nodes (`node_type`, `mobile_tier`, `device_model`).
4. [x] Mobile-specific API endpoints defined in `docs/spec-v1.md`.
5. [~] Android app architecture: Kotlin + JNI wrapper over `libferris`.
6. [~] `libferris` mobile FFI surface designed (`MobileConfig`, contribution lifecycle).
7. [ ] Android NDK cross-compilation target validated.
8. [ ] iOS FFI compilation target validated.
9. [x] Contribution policy defined (charging/WiFi/thermal safeguards).
10. [x] Platform constraints documented (Android vs iOS limitations).

## 7) Definition of "Ready To Start Coding"

Coding should start once all of these are true:

1. [x] Section 2 conflicts are resolved and documented in one source of truth.
2. [x] Section 3 data model decisions are frozen.
3. [x] Phase 1 scope is locked and accepted.
4. [x] Workspace scaffold and CI are in place.
5. [x] First milestone is narrowed to:
   - `ferris init`
   - `remember/recall/forget`
   - local MCP serve command

Source of truth for all locked decisions: `docs/spec-v1.md`.

## 8) Phase 1 Implementation Status

All Phase 1 deliverables are complete and verified.

### Crates Implemented

| Crate | Status | Tests |
|-------|--------|-------|
| `ferris-common` | Done | types, errors, config structs |
| `ferris-core` | Done | CLI (`init`, `serve`, `status`), config, identity, resources, HTTP server |
| `ferris-memory` | Done | 8 unit tests (CRUD, upsert, capacity, metadata, search) |
| `ferris-storage` | Done | 6 unit tests (store/retrieve, dedup, capacity, blake3, listing) |
| `ferris-tasks` | Done | 5 unit tests (schedule, cancel, capacity, listing) |
| `ferris-mcp` | Done | 10 MCP tools wired to real backends via rmcp 0.16 |

### Integration Tests

| Test Suite | Tests | Coverage |
|------------|-------|----------|
| HTTP API (`tests/http_api.rs`) | 7 | health, status, memory CRUD, storage CRUD, tasks CRUD, 404 handling |

### Quality Gates

- [x] `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings
- [x] `cargo test --workspace` — 26/26 passing
- [x] E2E smoke test: `ferris init` → `ferris serve --transport http` → curl all endpoints → verified

### CLI Commands

- `ferris init` — creates data dir, config.toml, SQLite DB, Ed25519 identity
- `ferris serve --transport stdio` — MCP server for Claude/Cursor
- `ferris serve --transport http` — REST dev server on configurable host:port
- `ferris status` — agent ID, resources, data counts

### HTTP API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/health` | Health check |
| GET | `/api/v1/status` | Memory/object/task counts |
| POST | `/api/v1/memory/remember` | Store key-value memory |
| POST | `/api/v1/memory/recall` | Search memories |
| DELETE | `/api/v1/memory/{key}` | Delete memory |
| POST | `/api/v1/storage/store` | Store file (base64) |
| GET | `/api/v1/storage` | List files |
| GET | `/api/v1/storage/{file_id}` | Retrieve file |
| POST | `/api/v1/tasks` | Schedule task |
| GET | `/api/v1/tasks` | List tasks |
| DELETE | `/api/v1/tasks/{task_id}` | Cancel task |

### MCP Tools (stdio)

`whoami`, `remember`, `recall`, `forget`, `store`, `retrieve`, `list_files`, `schedule_task`, `list_tasks`, `cancel_task`
