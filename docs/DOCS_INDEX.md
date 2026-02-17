# OpenFerris Documentation Index

This index defines which docs are authoritative for development and which are supporting references.

## Authoritative Docs (Implementation)

1. `docs/PRD.md`  
   Product requirements, priorities, acceptance criteria, and phase scope.
2. `docs/spec-v1.md`  
   Canonical technical decisions, data model, routing, API namespace.
3. `docs/build-readiness-checklist.md`  
   Build gate and kickoff criteria.
4. `docs/first-contribution.md`  
   Contributor onboarding path.
5. `docs/collaboration-ops.md`  
   Maintainer triage/moderation operations.
6. `docs/local-development.md`  
   Local runtime spin-up and API testing quickstart.
7. `docs/agent-interoperability.md`  
   Agent adoption contract (Ironclaw-informed, runtime-agnostic).
8. `docs/gap-analysis.md`  
   Honest risk register and stage-gate constraints.
9. `docs/launch-plan.md`  
   8-week execution sequencing and exit criteria.
10. `docs/agent-distribution.md`
   Agent/human-channel distribution execution (11-door map: registries, frameworks, IDEs, package managers, provider listings, messaging funnel).
11. `docs/mobile-supply.md`
   Phone contribution strategy and tiered mobile-node roadmap.
12. `docs/two-flywheel-strategy.md`
   Two-flywheel growth model (Agent + Phone), competitive moats, revenue model, execution timeline, narratives.

## Supporting Reference Docs

1. `docs/architecture.md`  
   Deep architecture context and expanded design rationale.
2. `docs/rust-port-plan.md`  
   Implementation sequencing reference; execution details should align to PRD/spec.
3. `docs/economy.md`  
   Economic model and pricing narrative.
4. `docs/component-matrix.md`  
   Mapping of reference projects to OpenFerris components.

## Reading Order for New Collaborators

1. `README.md`
2. `docs/PRD.md`
3. `docs/spec-v1.md`
4. `docs/first-contribution.md`
5. `CONTRIBUTING.md`
6. `docs/local-development.md`
7. `docs/agent-interoperability.md`
8. `docs/gap-analysis.md`
9. `docs/launch-plan.md`
10. `docs/agent-distribution.md`
11. `docs/mobile-supply.md`
12. `docs/two-flywheel-strategy.md`

## Documentation Rules

1. If `docs/PRD.md` and another file conflict on product scope, `docs/PRD.md` wins.
2. If `docs/spec-v1.md` and another file conflict on technical details, `docs/spec-v1.md` wins.
3. Significant feature/API/schema changes must update:
   - `docs/PRD.md`
   - `docs/spec-v1.md`
   - relevant supporting docs
