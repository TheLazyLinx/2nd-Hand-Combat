# 0004 — Battle time limit, unlimited mode, and End battle

- **Status:** Accepted
- **Date:** 2026-11-19
- **Phases:** 3 (nullable `timeLimit` in schema v3), 4 (defaults, UI controls, and End battle action)
- **Related:** [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [MVP.md](../../MVP.md), [MAP_MAKER.md](../../MAP_MAKER.md)

## Context

The initial 180-second (3-minute) default time limit is often too short for cover-heavy doctrines or large 128 × 128 maps, and Skirmish setup provides no control to edit it. Players also need an option to run battles with no time limit without being forced into a silent reset if a battle stalls (for example, when both sides run out of ammunition under `ContinueDoctrine`).

## Decision

1. **Raise the default time limit to 600 seconds (10 minutes).** Update catalog and Skirmish defaults from 180 s to 600 s.
2. **Support an optional "No time limit" mode.** Change `Scenario.time_limit` to `Option<f64>` in Rust (`number | null` in TypeScript), where `None` / `null` disables the battle timeout. Finite numeric limits remain validated within `1..=3600` seconds.
3. **Expose time-limit controls in Skirmish setup.** Provide a minutes input and a "No time limit" toggle in Skirmish (alongside Map Maker's scenario properties), and render unlimited battles cleanly in the battle clock and after-action report.
4. **Add an End battle action.** Allow ending a live or paused battle immediately, producing a draw `BattleResult` with `reason: "stopped"` that reports elapsed time, survivors, and casualties.

## Consequences

- Cautious doctrines and large custom maps can play out to resolution without hitting an artificial 3-minute cutoff.
- Unlimited battles always have a clean exit path that preserves after-action results instead of discarding them via Reset.
- Nullable `timeLimit` lands with scenario `schemaVersion` 3; existing v1 and v2 documents retain their numeric `timeLimit` on in-memory migration.
