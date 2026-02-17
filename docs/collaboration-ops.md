# Collaboration Ops and Agent Moderation

This file defines how OpenFerris uses automation and agents to scale collaboration.

## Goals

1. Fast response times for contributors.
2. Consistent triage quality.
3. Low maintainer overhead.
4. Healthy discussion spaces.

## Automation in this repo

1. `issue-triage-agent.yml`
   - Adds triage/type labels.
   - Scores issue completeness.
   - Requests missing repro details.
   - Flags potential abuse for moderation.
2. `pr-labeler.yml`
   - Applies area labels from changed paths.
3. `stale.yml`
   - Handles inactivity lifecycle with explicit messaging.
4. `labels-sync.yml`
   - Keeps labels standardized.

## Human + Agent Workflow

1. Agent handles first-pass triage and labeling.
2. Maintainer reviews `ready/triaged` issues in priority order.
3. `needs/info` issues are routed back to reporters.
4. `needs/moderation` issues get immediate maintainer review.

## Recommended Operating Cadence

1. Daily:
   - Triage new `needs/triage` issues.
2. Weekly:
   - Curate `good first issue`.
   - Close resolved `needs/info`.
3. Monthly:
   - Review automation false positives/negatives.
   - Tune labels and triage heuristics.

## External Channel Moderation (Agent Distribution)

Align with `docs/agent-distribution.md`.

1. Daily:
   - Review inbound feedback from registry/community channels.
   - Flag abuse/spam patterns and update response templates.
2. Weekly:
   - Update MCP/ClawHub listing metadata based on real search terms.
   - Review Moltbook engagement quality (helpful vs promotional noise).
3. Monthly:
   - Evaluate channel conversion (discoverability -> install -> activation).
   - Retire low-signal messaging and reinforce high-signal capability descriptions.
