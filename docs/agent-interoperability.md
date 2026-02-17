# Agent Interoperability Contract

Status: Canonical for agent adoption strategy

## 1) Purpose

OpenFerris must be easy to adopt for:
1. Agents built from scratch.
2. Existing agent runtimes.
3. Ironclaw-style personal agents.

Ironclaw is a reference profile for hardening patterns (security, memory ergonomics, Rust runtime discipline), not a required dependency.

## 2) Design Rule: No Runtime Lock-In

OpenFerris must never require an agent to be Ironclaw-based.

Required integration surfaces:
1. MCP tools for agent-native integration.
2. OpenAI-compatible HTTP endpoints for inference compatibility.
3. Stable REST endpoints for platform capabilities.

## 3) Responsibility Split

### Agent Runtime Responsibility (any runtime, including Ironclaw)
1. Planning/execution loop.
2. Prompt strategy/persona behavior.
3. Local tool orchestration policy.
4. Session UX and interaction style.

### OpenFerris Responsibility
1. Network routing and coordinator logic.
2. Resource contribution onboarding.
3. Credit/reputation/settlement systems.
4. Platform APIs and interoperability contracts.
5. Install/bootstrap developer experience.

## 4) Reuse-First Rules

Before adding runtime code in OpenFerris:
1. Check whether an existing runtime (Ironclaw or third-party) already solves it.
2. Prefer adapter layers and protocol contracts over runtime reimplementation.
3. New code must justify why adapter integration is insufficient.

## 5) Required Adoption Paths

### Path A: MCP Agent
1. Agent connects to OpenFerris MCP endpoint.
2. Agent uses memory/storage/tasks tools directly.
3. Network features are opt-in.

### Path B: OpenAI-Compatible Agent
1. Agent points inference base URL at OpenFerris coordinator.
2. Uses `/v1/*` with no runtime rewrite.
3. Adds OpenFerris-specific features incrementally.

### Path C: Embedded SDK/Library (future)
1. Runtime links OpenFerris client crates.
2. Uses typed API contracts.
3. Enables tighter integration where desired.

## 6) DX Targets

1. New agent integration path documented in <= 15 minutes.
2. First successful inference/memory request in <= 10 minutes.
3. No mandatory runtime migration required for baseline adoption.

## 7) Acceptance Criteria

1. Documentation includes at least one quickstart for MCP agents.
2. Documentation includes at least one quickstart for OpenAI-compatible agents.
3. PR review policy rejects runtime-duplication changes without clear justification.
