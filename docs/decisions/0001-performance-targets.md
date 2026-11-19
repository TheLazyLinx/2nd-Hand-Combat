# 0001 — Performance targets

- **Status:** Accepted
- **Date:** 2026-11-19
- **Phases:** 0C (measurement), 1 & 2 (hot-path and decision optimization), 6 (many-unit scaling)
- **Related:** [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [SCALE.md](../../SCALE.md), [ARCHITECTURE.md](../../ARCHITECTURE.md)

## Context

The Rust simulation runs at a fixed 20 Hz step (50 ms budget) in a dedicated Web Worker. Slow steps do not block the browser main thread, but exceeding 50 ms triggers catch-up capping and visibly slows the battle.

Baseline profiling on `dd945eb` (native release, 8 v 8, Cover doctrine) showed 32 × 20 presets spiking up to 50 ms on Protected routing and 128 × 128 maps with 1,500 obstacles taking over 31 s for a single worst step. Concrete targets are needed to guide optimization work and prevent premature complexity.

## Decision

Adopt the following acceptance targets for all built-in presets at 8 v 8 and for 128 × 128 maps with 1,500 obstacles at 8 v 8:

### Browser acceptance targets (desktop Chromium, mid-range laptop, WASM in Worker)

| Measure | Target | Rationale |
| --- | --- | --- |
| Mean step | ≤ 5 ms | 10% of the 50 ms step budget; headroom for fast-forward and slower machines |
| p99 step | ≤ 20 ms | Decision spikes stay comfortably inside one simulation step |
| Worst step | ≤ 40 ms | Avoids missed ticks and catch-up capping |
| Any step | Never > 250 ms | Hard ceiling above which players notice a stall |
| Map Maker analysis (128 × 128) | ≤ 100 ms | Map editing remains responsive |
| Main-thread rendering | 60 fps | Independent of simulation rate |

### Native release benchmark proxy (`sim-bench`)

Because native release builds are roughly 2× faster than WASM in the browser, use half of the browser step budgets as the local benchmark proxy:

- Mean step: **≤ 2.5 ms**
- p99 step: **≤ 10 ms**
- Worst step: **≤ 20 ms**

### Larger unit counts

Targets for 100, 1,000, and 5,000 units ([SCALE.md](../../SCALE.md)) will be established after Phase 6 measurements once unit caps are decoupled from Skirmish mode limits.

## Consequences

- Every performance change to simulation, terrain, navigation, perception, or inventory is evaluated against recorded `sim-bench` baselines and these budgets.
- Heavy scaling machinery (hierarchical pathfinding, flow fields, ECS libraries, shared memory, or extra simulation Workers) remains deferred unless measurements show simpler optimizations cannot hit these targets.

## Open questions

- Which concrete hardware specification serves as the reference "mid-range laptop" for browser acceptance measurements.
- Supported mobile device targets, pending a supported-device matrix decision.
