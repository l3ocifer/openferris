# Contributing to OpenFerris

Thanks for helping build OpenFerris.

## Quick Start

1. Install stable Rust with `clippy` and `rustfmt`.
2. Fork and clone the repository.
3. Run:
   - `cargo check --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
4. Pick an issue labeled `good first issue` or `help wanted`.

## Development Rules

1. Keep PRs small and focused (one change theme per PR).
2. Add tests for behavior changes.
3. Update docs when user-visible behavior changes.
4. Keep commits readable and descriptive.
5. Follow `docs/spec-v1.md` for canonical decisions.
6. Follow `docs/agent-interoperability.md` to avoid agent-runtime duplication.

## Branch and PR Flow

1. Create a feature branch from `main`.
2. Make changes and run local checks.
3. Open a PR using the PR template.
4. Address review feedback quickly.
5. Squash merge unless maintainers request otherwise.

## Issue Guidelines

- Use issue templates.
- Include reproduction steps for bugs.
- Include expected vs actual behavior.
- Include logs and environment details where possible.

## Good First Contribution Areas

- CLI ergonomics and help text.
- Basic tests for memory/storage/tasks crates.
- Documentation improvements and examples.
- CI quality-of-life improvements.

## Security

Please do not file public issues for security vulnerabilities.
Follow `SECURITY.md` for private reporting.
