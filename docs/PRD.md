# OpenFerris Product Requirements Document (PRD)

Version: 1.0
Date: 2026-02-17
Status: Approved for implementation kickoff

## 1) Product Summary

OpenFerris is a Rust-first platform where agents have persistent identity, memory, storage, and scheduling locally, then optionally join a routed network economy for inference and services.

Growth follows a two-flywheel model (see `docs/two-flywheel-strategy.md`): **Flywheel 1 (Agents)** creates the network via developer distribution. **Flywheel 2 (Phones)** scales it via a consumer app where phones earn credits overnight and users chat with AI during the day. Both flywheels feed the same shared network.

This PRD defines what must be built, in what order, and with what acceptance criteria.

Ironclaw is a personal/reference agent runtime that informs OpenFerris defaults and security posture. OpenFerris is not Ironclaw-specific; it must support easy adoption by existing and newly created agents across ecosystems.

## 2) Problem Statement

Agents today are powerful but fragmented:
1. No persistent memory by default.
2. No unified local infrastructure (memory + storage + tasks).
3. No built-in economic model for agent-to-agent or resource sharing.
4. High cloud inference costs despite widespread idle local hardware.

## 3) Product Goals

1. [HIGH] Deliver a local-first agent runtime that works without network dependencies.
2. [HIGH] Provide a clean Phase 2 path to coordinator-based inference routing and credits.
3. [HIGH] Minimize operator friction (`curl | sh`, single binary UX).
4. [HIGH] Ensure easy adoption by non-Ironclaw agents via stable MCP/OpenAI-compatible interfaces.
5. [MEDIUM] Make open-source collaboration fast, safe, and scalable via automation.
6. [MEDIUM] Establish architecture constraints that avoid early over-engineering.
7. [HIGH] Architect for mobile supply as the primary growth engine (phone contribution tiers) without blocking Phase 1/2 delivery.

## 4) Non-Goals (Phase 1)

1. [HIGH] No distributed multi-node storage replication.
2. [HIGH] No QUIC/libp2p production path in Phase 1.
3. [HIGH] No USDC cashout or on-chain settlement.
4. [MEDIUM] No full S3 API surface.
5. [MEDIUM] No enterprise orchestration integrations (K8s/Ray/Virtual Kubelet).
6. [MEDIUM] No full reimplementation of a bespoke agent runtime if equivalent behavior already exists in Ironclaw or third-party agents.

## 5) Personas

1. **Agent Builder (primary)**
   Wants local memory, storage, and scheduling with minimal setup.
2. **Desktop Node Contributor (Phase 2+)**
   Wants to contribute GPU/compute/storage and earn credits from idle hardware.
3. **Phone Contributor (Phase 2+)**
   Non-technical user who downloads the app. Wants free AI by contributing phone resources overnight while charging. Zero marginal cost. The mass-market growth engine.
4. **Existing Agent Integrator**
   Wants to connect an existing agent quickly without rewriting core runtime logic.
5. **Maintainer/Collaborator**
   Wants clear docs, predictable workflows, and low triage overhead.

## 6) User Stories

1. [HIGH] As an Agent Builder, I can run `ferris init` and get identity + local state in minutes.
2. [HIGH] As an Agent Builder, I can use MCP tools `remember/recall/forget` for persistent memory.
3. [HIGH] As an Agent Builder, I can store and retrieve local files via MCP tools.
4. [HIGH] As an Agent Builder, I can schedule and manage tasks locally.
5. [MEDIUM] As a Desktop Contributor, I can view contribution readiness and local status.
6. [HIGH] As a Phone Contributor, I can download the app, complete setup in 3 taps, and earn credits overnight while my phone charges on WiFi.
7. [HIGH] As a Phone Contributor, I can chat with my AI agent during the day using credits earned overnight from phone contributions.
8. [MEDIUM] As a Phone Contributor, I can see live contribution stats (credits earned, vectors stored, embeddings generated, verifications completed).
9. [HIGH] As an Existing Agent Integrator, I can onboard my agent using OpenFerris APIs/tools without replacing my runtime.
10. [MEDIUM] As a Maintainer, I can rely on issue/PR triage automation and policy docs to scale contributions.

## 7) Functional Requirements

### 7.1 Phase 1 (must ship first)

1. [HIGH] CLI commands:
   - `ferris init`
   - `ferris serve`
   - `ferris status`
2. [HIGH] MCP local tools:
   - Identity: `whoami`
   - Memory: `remember`, `recall`, `forget`
   - Storage: `store`, `retrieve`, `list_files`
   - Tasks: `schedule_task`, `list_tasks`, `cancel_task`
3. [HIGH] Local persistence:
   - node-local SQLite schema and migrations.
4. [HIGH] Config:
   - default TOML config and documented env override convention.
5. [HIGH] Quality gates:
   - workspace compiles and tests pass in CI.
6. [HIGH] Agent interoperability baseline:
   - document adapter contract for "bring-your-own-agent" integration.
   - define Ironclaw as reference profile, not mandatory runtime dependency.
   - enforce no-redundancy boundary between runtime behavior and OpenFerris platform behavior.
7. [HIGH] Agent distribution baseline:
   - execute launch-critical channel presence from `docs/agent-distribution.md` (Doors 1-6 minimum).
   - publish machine-readable discovery docs (`llms.txt` path and briefing template).

### 7.2 Phase 2 (network baseline)

1. [HIGH] `ferris-coordinator` with Axum + SQLite.
2. [HIGH] Node registration + heartbeat endpoints under `/api/v1/*`.
3. [HIGH] OpenAI-compatible inference endpoint under `/v1/chat/completions`.
4. [HIGH] Credit ledger with soft/hard balances and fixed 15% platform fee.
5. [MEDIUM] Initial reputation and routing implementation per `docs/spec-v1.md`.

### 7.2b Phase 2 — Mobile (Android MVP, non-blocking)

1. [HIGH] Android app: Tier 1 (vector storage + similarity search) + chat interface.
2. [HIGH] Coordinator API extensions for mobile node registration (node_type, mobile_tier, device_model).
3. [MEDIUM] Mobile contribution policy enforcement (charging/WiFi/thermal safeguards).
4. [MEDIUM] Credit tracking and balance display in mobile app.

### 7.3 Phase 3+ (deferred)

1. [MEDIUM] Direct stream broker mode.
2. [MEDIUM] Directory hiring + escrow lifecycle.
3. [LOW] Broader ecosystem integrations.
4. [HIGH] Android Tier 2-3: on-device embedding generation + inference verification.
5. [HIGH] Android Tier 4: small model inference (0.6B-3B via Cactus/llama.cpp + NPU).
6. [MEDIUM] iOS app: Tier 1-2 (storage + embeddings) + foreground contribution mode.
7. [LOW] Federated learning pilot (Phase 4+).

## 8) Inputs, Outputs, and Constraints

### Inputs

1. CLI commands and config.
2. MCP tool invocations from agent clients.
3. (Phase 2+) signed API requests from nodes/consumers.

### Outputs

1. Deterministic local state changes in SQLite/filesystem.
2. MCP responses with predictable schemas.
3. (Phase 2+) routed inference responses and ledger records.

### Constraints

1. [HIGH] Canonical spec precedence: `docs/spec-v1.md`.
2. [HIGH] Local-first reliability: core tools must function offline.
3. [HIGH] Simplicity-first architecture: avoid premature distributed complexity.
4. [MEDIUM] Rust stable toolchain compatibility.

## 9) Non-Functional Requirements

1. [HIGH] Build Reliability:
   - CI green on `check`, `fmt`, `clippy -D warnings`, `test`.
2. [HIGH] Security Baseline:
   - documented reporting process and CoC enforcement.
3. [HIGH] Developer Experience:
   - contributors can run project checks in <= 10 minutes on a typical laptop.
4. [MEDIUM] Observability:
   - structured logging in core CLI/runtime paths.
5. [MEDIUM] Documentation Quality:
   - authoritative docs are clearly marked; conflicting docs are downgraded to reference.

## 10) Acceptance Criteria

### Phase 1 Exit Criteria

1. [HIGH] `cargo check --workspace` passes on `main`.
2. [HIGH] `cargo clippy --workspace --all-targets -- -D warnings` passes.
3. [HIGH] `cargo test --workspace` passes.
4. [HIGH] CLI can initialize and report status without coordinator.
5. [HIGH] MCP server exposes all Phase 1 local tools.
6. [HIGH] Migration `migrations/node/0001_init.sql` is applied in local setup flow.
7. [MEDIUM] First-contribution path docs are complete and accurate.

### Collaboration/Moderation Exit Criteria

1. [HIGH] Issue and PR templates are live.
2. [HIGH] Automated triage labels and stale handling are active.
3. [HIGH] CoC, Security, Contributing, Governance docs are published.
4. [MEDIUM] Maintainers can triage incoming issues in < 24h using automation outputs.

### Interoperability Exit Criteria

1. [HIGH] `docs/agent-interoperability.md` is published and linked in `README.md`.
2. [HIGH] At least one MCP-based integration path is documented and testable.
3. [HIGH] At least one OpenAI-compatible adoption path is documented and testable.
4. [MEDIUM] Contributor review guidance includes no-redundancy runtime boundary checks.

### Distribution Exit Criteria

1. [HIGH] MCP registry presence is live with production metadata.
2. [HIGH] ClawHub primary skill is published and install-tested.
3. [HIGH] `llms.txt` and briefing templates are published and linked.
4. [MEDIUM] Moltbook participation cadence is operational.
5. [HIGH] At least one framework integration path (LangChain/CrewAI/AutoGen family) is published.
6. [HIGH] LiteLLM provider integration path is documented and testable.
7. [MEDIUM] Package-manager install path includes Homebrew readiness.
8. [HIGH] Mobile supply strategy (`docs/mobile-supply.md`) is published with tier definitions, credit rates, and platform constraints.
9. [HIGH] Android app architecture aligned to coordinator APIs and `libferris` FFI surface.

## 11) Milestones

1. **M1 (Week 1):** Phase 1 scaffold + green CI baseline.
2. **M2 (Week 2):** Memory/storage/tasks local implementations behind MCP.
3. **M3 (Week 3):** Phase 1 stabilization, docs hardening, onboarding validation. Target: 500 installs, 50 DAU.
4. **M4 (Week 4-6):** Coordinator + network + Telegram bot. Target: 2,000 installs, 200 DAU, Telegram bot with 1,000 users.
5. **M5 (Week 6-10):** Android app launch (Flywheel 2 activation). Target: 10,000 app downloads, 2,000 phone nodes contributing nightly.
6. **M6 (Week 10-14):** Pro tier + monetization. Target: $1,000 MRR, 50,000 app downloads.
7. **M7 (Week 14-20):** iOS + phone inference. Target: 100,000+ app users, $5,000+ MRR.

## 12) Risks and Mitigations

1. [HIGH] Risk: scope bleed into Phase 2 while Phase 1 is incomplete.
   Mitigation: enforce PR review against this PRD + `spec-v1`.
2. [MEDIUM] Risk: contributor confusion from legacy docs.
   Mitigation: publish docs index with authority levels.
3. [MEDIUM] Risk: moderation false positives in automation.
   Mitigation: keep human-in-the-loop for lock/ban decisions.
4. [HIGH] Risk: demand does not materialize at expected pace.
   Mitigation: run explicit market validation and inference-channel execution before deeper feature expansion.
5. [HIGH] Risk: onboarding friction causes early churn.
   Mitigation: enforce first-5-minute UX and "magic moment" validation as a release gate.
6. [HIGH] Risk: runtime duplication slows delivery.
   Mitigation: enforce interoperability contract and adapter-first implementation policy.
7. [MEDIUM] Risk: iOS platform restrictions prevent meaningful phone contribution.
   Mitigation: Android-first strategy; iOS starts with low-complexity tiers (storage + embeddings).
8. [MEDIUM] Risk: phone contribution economics not meaningful enough for user retention.
   Mitigation: start with Tier 1 (zero-cost storage) which provides immediate value; monitor earn-vs-spend ratio.

## 13) Dependencies

1. Rust stable + workspace toolchain.
2. GitHub Actions for CI and moderation/triage automation.
3. Local SQLite runtime for persistence.
4. Market validation inputs from target user communities.
5. Distribution execution plan from `docs/agent-distribution.md`.
6. Mobile supply strategy from `docs/mobile-supply.md`.
7. Android NDK toolchain for `libferris` JNI compilation (Phase 2).
8. iOS toolchain for `libferris` FFI compilation (Phase 3+).

## 14) Source of Truth and Change Control

1. Canonical implementation decisions: `docs/spec-v1.md`
2. Product requirements and scope: `docs/PRD.md` (this file)
3. Build gate checklist: `docs/build-readiness-checklist.md`
4. Gap/risk posture: `docs/gap-analysis.md`
5. Delivery sequencing: `docs/launch-plan.md`
6. Agent-channel GTM execution: `docs/agent-distribution.md`
7. Two-flywheel growth model: `docs/two-flywheel-strategy.md`
8. Mobile supply strategy: `docs/mobile-supply.md`

Any PR that changes architecture, schema, API namespace, or phase scope must update this PRD and `spec-v1` in the same change set.

## 15) Stage Gates

### Gate 0: Validation
1. [HIGH] Evidence of user demand from target communities.
2. [HIGH] First-5-minute onboarding path defined and tested.

### Gate 1: Local MVP
1. [HIGH] Local value loop works in < 10 minutes for external testers.
2. [HIGH] Local runtime + docs quality gates pass.

### Gate 2: Network Beta
1. [HIGH] End-to-end routed inference is stable.
2. [HIGH] Coordinator backup/restore path is tested.
3. [HIGH] Baseline legal docs are published before paid rollout.

### Gate 2b: Flywheel 2 Activation (Phone App)
1. [HIGH] Telegram bot validates consumer demand (>500 users, meaningful engagement).
2. [HIGH] Coordinator API supports mobile node registration.
3. [HIGH] Android app Tier 1-2 contribution validated (phone contributors earn more than they spend).
4. [MEDIUM] In-app referral system functional.

## 16) Success Metrics and Kill Signals

### Success Metrics
1. [HIGH] Install and activation growth after launch (CLI + app store).
2. [HIGH] Active node retention and inference throughput (desktop + phone).
3. [MEDIUM] Time-to-first-value and onboarding completion rate.
4. [MEDIUM] Paid conversion once monetization launches (target: 1% of app users).
5. [HIGH] Phone earn-vs-spend ratio (must stay net positive for average user).
6. [HIGH] Nightly phone contribution rate (retention night-over-night).
7. [MEDIUM] Telegram bot → app download conversion rate.

### Kill Signals (pause and reassess)
1. [HIGH] Persistently low install/adoption after launch effort (Flywheel 1).
2. [HIGH] Very low contributor retention.
3. [HIGH] No viable demand channel traction.
4. [HIGH] Severe unresolved trust/security incidents.
5. [HIGH] Phone app <1,000 downloads after 2 weeks on Play Store (Flywheel 2).
6. [HIGH] Phone earn-vs-spend ratio persistently negative.
7. [MEDIUM] App store rejection without viable workaround.
