# 0007 — Agent-maintained workflow and human-approval checkpoints

- **Status:** Accepted
- **Date:** 2026-11-19
- **Phase:** 0D
- **Related:** [AGENTS.md](../../AGENTS.md), [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [ADR 0006](0006-automated-verification-and-ci.md)

## Context

Autonomous agents can implement features and refactors efficiently within clear technical guardrails, but high-leverage decisions — introducing dependencies, breaking saved-scenario compatibility, shifting gameplay meaning, weakening test assertions, altering product scope, or adding assets with legal implications — require explicit human approval before proceeding.

## Decision

1. **Mandatory human-approval checkpoints.** Agents must pause and obtain explicit human approval before performing any of the following six actions:
   1. Adding dependencies (Rust crates or npm packages).
   2. Bumping scenario `schemaVersion`.
   3. Changing doctrine meaning or updating outcome fingerprints.
   4. Loosening test tolerances or deleting tests.
   5. Editing scope documents (`SCOPE.md`, `MVP.md`, `SCALE.md`, `DREAM_GOALS.md`).
   6. Adding art or audio assets (which must also be recorded in an asset provenance and licence register with source and usage rights before inclusion).
2. **Transparent task handoffs.** Every handoff must summarize changes and verification run, explicitly flag any edits to existing tests, and include before/after doctrine report summaries whenever simulation behavior changes.
3. **Repository-resident guidance and code hygiene.**
   - Record lasting decisions as numbered ADRs under `docs/decisions/`.
   - Maintain concise area guides (`crates/sim-core/AGENTS.md`, `apps/web/AGENTS.md`) with module ownership, invariants, and step-by-step recipes.
   - Keep source files under ~400 lines where practical, with a brief module-header comment stating ownership and key invariants.

## Consequences

- Routine implementation, bug fixes, and internal refactoring proceed smoothly without blocking on minor details, while irreversible or high-risk changes always pass through human review.
- Code review focuses on product intent and behavioral impact rather than hunting for hidden test weakenings or undocumented gameplay shifts.
