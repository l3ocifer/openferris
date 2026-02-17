# First Contribution Guide

This is the fastest path to your first merged PR.

## 1) Setup

1. Fork the repository.
2. Clone your fork.
3. Run:
   - `cargo check --workspace`
   - `cargo test --workspace`
4. Read:
   - `docs/PRD.md`
   - `docs/spec-v1.md`
   - `docs/DOCS_INDEX.md`
   - `docs/local-development.md`

## 2) Pick an Issue

Start with labels:
- `good first issue`
- `help wanted`
- `docs`

## 3) Make the Change

1. Create a branch from `main`.
2. Keep scope small and focused.
3. Add tests when behavior changes.
4. Run clippy and tests locally.

## 4) Open PR

1. Fill in the PR template completely.
2. Link the issue (`Closes #...`).
3. Request review.

## 5) Collaborate on Review

1. Address requested changes quickly.
2. Keep discussion technical and respectful.
3. Re-run checks after each update.
