# 0006 — Automated verification, generated contracts, and CI

- **Status:** Accepted
- **Date:** 2026-11-19
- **Phases:** 0A, 0B, 0C (implemented)
- **Related:** [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [ARCHITECTURE.md](../../ARCHITECTURE.md), [AGENTS.md](../../AGENTS.md)

## Context

This codebase is primarily built and maintained by agents with human oversight. Architectural boundaries, cross-language wire types, dependency licences, and formatting conventions degrade rapidly when enforced only by prose documentation.

## Decision

Enforce structural and quality invariants automatically through approved dev tooling, local tiered verification scripts, and GitHub Actions CI:

1. **Generated Rust ↔ TypeScript contract (`ts-rs`).** Derive wire types in `crates/sim-core` behind an optional `ts-export` feature (`default-features = false`, zero runtime or WASM impact) and emit `apps/web/src/generated/sim-types.ts` via `npm run gen:types`. Guard against drift with `npm run check:types-sync` and a Rust `serde_json` vs. TypeScript key/variant parity test. `apps/web/src/lib/types.ts` re-exports generated types and holds only browser/Worker protocol and document wrappers.
2. **Rust dependency policy (`cargo-deny`).** Enforce allowed licences (`MIT`, `Apache-2.0`, `Unicode-3.0`), advisories, bans, and crates.io-only sources via `deny.toml` and `npm run deny:rust`, while leaving workspace crates unlicensed until a project licence is selected ([AGENTS.md](../../AGENTS.md) rule 16).
3. **Formatting, linting, and architectural boundaries.**
   - Use Prettier (`prettier-plugin-svelte`) and ESLint (`eslint.config.js`, `--max-warnings=0`) across `apps/web`, `scripts`, and `tests`.
   - Enforce frontend module boundaries in ESLint with regression tests (`tests/boundaries.test.mjs`): only `worker.ts` imports generated WASM; only `BattleClient` instantiates `Worker`; `battlefield.ts` stays snapshot-only; `worker.ts` stays free of PixiJS/Svelte/UI.
   - Enforce `sim-core` browser independence in `crates/sim-core/tests/boundaries.rs`.
4. **Tiered verification commands.**
   - `npm run verify:fast` (run on every change): Rust fmt/clippy/tests, TS contract sync, web fmt/lint, boundary tests, privacy checks/tests, WASM + Vite build, `svelte-check`/`tsc`, and `@smoke` Playwright tests.
   - `npm run verify` (run before merging and in CI): everything in `verify:fast`, plus `deny:rust`, `bench:rust`, and the full Playwright suite (including `@stress` tests).
5. **Continuous integration.** Run `npm run verify` in `.github/workflows/verify.yml` using SHA-pinned GitHub Actions and SHA-256-verified tool binaries (`wasm-pack`, `cargo-deny`), keeping the existing `.github/workflows/privacy.yml` workflow intact.

## Consequences

- Wire contract mismatches, boundary leaks, formatting churn, and disallowed dependencies fail automatically before human review.
- The approved dev dependencies (`ts-rs`, Prettier, ESLint, `prettier-plugin-svelte`, `cargo-deny`) are codified; any future dependency additions still require human approval ([ADR 0007](0007-agent-workflow-and-human-checkpoints.md)).
- Both local contributors and CI run the exact same verification entry point (`npm run verify`).
