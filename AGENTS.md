# AGENTS.md — openferris

Contract for AI coding agents (Cursor, Claude Code, Codex CLI, etc.) working in this repo. Humans should follow it too.

> Symlink: `CLAUDE.md` → `AGENTS.md`. Both clients read the same source of truth.

## Identity

- **Repo**: `l3ocifer/openferris`
- **Default branch**: `main`
- **Stack**: rust
- **Deploys on merge**: false
- **Tier**: warm  <!-- hot = production-critical, warm = active dev, cold = archived/dormant -->

## Before you start

1. `git status` clean. Pull `main`. Branch from it.
2. Read scoped `AGENTS.md` files in any subtree you touch.
3. If a `docs/status/STATUS.md` exists, read it for in-flight work.
4. Run the install + smoke commands below to confirm the toolchain is healthy before editing.

## Commands

```bash
cargo fetch      # install deps
-          # run locally
cargo test         # run tests
cargo clippy --all-targets -- -D warnings         # lint + format check
cargo build --release        # build / typecheck
```

If a command above is `-`, that surface does not exist in this repo.

## Map

- `config/`
- `crates/`
- `docs/`
- `migrations/`
- `references/`
- `scripts/`

## Conventions

- **Commits**: Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `ci:`). Subject ≤ 72 chars. Body explains *why*, not *what*.
- **PRs**: Small and scoped. Fill the PR template. Include a test plan and rollback note. Link the issue if one exists.
- **Branches**: `feat/<slug>`, `fix/<slug>`, `chore/<slug>`. No work on `main` directly.
- **Code style**: Match surrounding code. Do not bulk-reformat unrelated files.
- **Tests**: Add or update tests for any behavior change. Do not delete tests to make CI pass.
- **Comments**: Explain non-obvious intent only. Do not narrate the code.
- **Generated files**: Do not hand-edit. Regenerate via the documented command.

## DO NOT touch without an explicit request

- (none beyond the universal rules below)
- `.github/workflows/**` unless the task is CI-related
- Lockfiles (`package-lock.json`, `pnpm-lock.yaml`, `Cargo.lock`, `poetry.lock`, `go.sum`) unless the task is dependency-related
- Anything matching `*.pem`, `*.key`, `*.env*`, `*credentials*`, `*secrets*`

If you must touch one of the above, say so explicitly in the PR description and tag a CODEOWNER.

## Safety rails

- **No force-push** to `main`. Branch protection enforces this.
- **No history rewrites** on shared branches.
- **No new top-level dependencies** without justification in the PR body.
- **Secrets never in code**. Use the configured secret store. The `secret-scan` workflow blocks PRs that leak.
- If you are uncertain, **stop and ask** in the PR description rather than guessing.

## Escalation

Open a draft PR with a `needs:owner-review` label and `@`-mention a CODEOWNER when:

- The change crosses a sensitive path listed above.
- The change alters public API, schema, deploy config, or auth.
- The blast radius is unclear or you hit ambiguity that could be solved two reasonable ways.
- A required check is failing for a reason you cannot explain.

## What "done" looks like

- All checks green on the PR (lint, typecheck, tests, build, secret-scan).
- PR description filled out, including test plan.
- New behavior covered by tests or smoke instructions.
- Docs updated if user-visible behavior or public API changed.
- No unrelated diffs.

## Per-subtree guidance

Drop additional `AGENTS.md` files in subdirectories with rules that only apply there. The deepest matching `AGENTS.md` wins for its subtree; root rules still apply unless explicitly overridden.

## Links

- (add per-repo runbook + production URL links here)
