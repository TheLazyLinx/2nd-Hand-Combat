# ADR 0008: Spatial Grid, Compact Binary Snapshots, and Storage Layout Evaluation

- **Status:** Accepted
- **Target phase:** 6
- **Related:** [ARCHITECTURE.md](../../ARCHITECTURE.md), [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [ADR 0001](0001-performance-targets.md)

## Context

`ARCHITECTURE.md` left open whether scaling toward 100 / 1,000 / 5,000 units warrants migrating `crates/sim-core` from componentized `Vec<Unit>` storage to packed SoA component arrays or a third-party Rust ECS crate. Phase 6 requires profiling 100 / 1,000 / 5,000-unit sweeps with `sim-bench` before making structural storage changes.

### Measured native release benchmarks (128×128 open arena, 40 steps)

Before Phase 6 (pairwise scans + JSON snapshots):

| Scenario | Mean (ms) | Worst (ms) | Percept | Decide | Move | PostPerc | ItemAct | Combat | SnapJSON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `sweep-100-open128` | 2.160 | 54.67 | 0.0379 | 0.0152 | 2.0657 | 0.0368 | 0.0024 | 0.0015 | 0.2563 |
| `sweep-1000-open128` | 27.244 | 499.94 | 3.7850 | 0.4819 | 19.0853 | 3.7836 | 0.0736 | 0.0214 | 3.0494 |
| `sweep-5000-open128` | 315.602 | 2566.55 | 105.3226 | 9.6993 | 90.9408 | 108.9834 | 0.4441 | 0.1249 | 18.7939 |

After Phase 6 (4 m CSR `SpatialGrid` with 32-byte packed `SpatialEntry` records + per-cell 64-bit faction bitmasks + 72-byte binary compact snapshots + dirty inventory versioning):

| Scenario | Mean (ms) | Worst (ms) | Sust | Percept | Decide | Move | PostPerc | ItemAct | Combat | SnapJSON | SnapBin |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `sweep-100-open128` | 2.312 | 60.20 | 0.0003 | 0.0152 | 0.0050 | 2.2695 | 0.0152 | 0.0040 | 0.0020 | 0.2802 | 0.0053 |
| `sweep-1000-open128` | 20.109 | 487.74 | 0.0043 | 0.1447 | 0.0842 | 19.6302 | 0.1565 | 0.0610 | 0.0221 | 3.2202 | 0.1052 |
| `sweep-5000-open128` | 89.944 | 2158.32 | 0.0421 | 1.0689 | 0.5886 | 86.7151 | 0.9866 | 0.3755 | 0.1262 | 17.6342 | 0.5904 |
| `sweep-1000-contact` | 175.949 | 257.06 | 0.0035 | 69.5584 | 45.9379 | 2.3890 | 57.9083 | 0.1173 | 0.0264 | 3.3334 | 0.5238 |

Key observations from the profiling data:

1. **Spatial partitioning eliminates O(N²) perception, ally, and blast scans:** On `sweep-5000-open128`, `Percept` dropped from `105.3 ms` to `1.07 ms` (~98×), `PostPerc` dropped from `109.0 ms` to `0.99 ms` (~110×), and `Decide` dropped from `9.70 ms` to `0.59 ms` (~16×).
2. **Compact binary snapshots eliminate serialization overhead:** Encoding 72-byte little-endian records (`SnapBin`) plus version-gated inspected-unit inventory dropped snapshot generation at 5,000 units from `17.6 ms` (`SnapJSON`) to `0.59 ms` (~30×) and transfers zero-copy over `postMessage(..., [buffer])`.
3. **Linear per-unit `Vec<Unit>` storage is not the bottleneck:** Even at 5,000 living units, `Sust` (`0.04 ms`), `ItemAct` (`0.38 ms`), and `Combat` (`0.13 ms`) total `0.55 ms` combined (< 1.1% of a 50 ms step budget). In dense 1,000-unit contact (`sweep-1000-contact`), `Sust` + `ItemAct` + `Combat` total `0.15 ms`, while > 98% of time sits in raycasting (`Percept`/`PostPerc` `127.5 ms`) and per-threat decision exposure sweeps (`Decide` `45.9 ms`). In sparse 5,000-unit marches, > 95% sits in Tick-1 bulk A\* pathfinding (`Move` `2.16 s` spike).

## Decision

1. **Keep componentized `Vec<Unit>` paired with the packed 32-byte `SpatialEntry` CSR grid.** Do **not** introduce a third-party ECS crate or rewrite all subsystems into parallel SoA arrays at this stage. `SpatialEntry` already serves as a cache-line-packed projection (`index`, `id`, `faction`, `revealed_until`, `position`) for hot spatial loops while keeping unit logic direct, readable, and dependency-free.
2. **Use hybrid snapshot transport across the Worker boundary:** Small battles (`< 32` units, default Skirmish) keep JSON `snapshot` replies built only when publishing (10 Hz or terminal/pause transitions). Larger battles (`>= 32` units or explicit `compact: true`) switch automatically to transferable 72-byte/unit binary `ArrayBuffer` frames (`compactSnapshot`) plus version-gated on-demand `InventorySnapshot` updates for the inspected unit only.
3. **Surface Worker catch-up lag explicitly:** When the Worker's 5-step catch-up cap clamps accumulated wall time, emit `{ type: 'performance', behindRealTime: true }` so the UI displays a visible `Simulation behind real time` indicator until caught up or paused.
4. **Use texture-batched PixiJS sprites and camera viewport culling** in `apps/web/src/lib/battlefield.ts` (one cached texture per faction colour plus dead colour, single batched HP/selection overlay `Graphics`, deferred labels above 64 units, and visibility culling outside camera world bounds).

## Consequences

- **No new dependencies** and zero changes to deterministic simulation outcomes (all 15 doctrine fingerprints remain bit-for-bit identical).
- **Clear path for future division-scale work:** Re-evaluating ECS or SoA only becomes worthwhile if profiling shows linear per-unit passes exceeding a meaningful share of the step budget. Future dense-combat scaling beyond 1,000 units should instead focus on algorithmic reductions: flow fields or shared squad pathfinding to eliminate Tick-1 A\* spikes, bounded threat sets / visibility caching in dense contact, and formation aggregation.

## Open Questions

- Exact threshold for automatic formation aggregation and hierarchical pathfinding if future modes target > 1,000 simultaneous close-contact combatants.
