# AGENTS.md — `crates/sim-core`

Area guide for Rust simulation work. Follow root [AGENTS.md](../../AGENTS.md) (including human-approval checkpoints) and [ARCHITECTURE.md](../../ARCHITECTURE.md) first.

## Module ownership

| Path | Responsibility |
| --- | --- |
| `src/types/` | Public wire data structures (`mod.rs`, `map.rs`, `doctrine.rs`, `inventory.rs`, `scenario.rs`, `snapshot.rs`), typed wire enums (`Behavior`, `ResultReason`, `IssueSeverity`, `IssueCode`, `ItemCategory`, `ItemUseKind`), `serde` + `ts-rs` export (`export_typescript_types`), and JSON-vs-TS contract parity tests (`ts_export_tests`). |
| `src/battle.rs` | `Battle` orchestrator: validates scenarios on `Battle::new`, owns unit list, tick counter, seeded RNG, fixed-step `step()` pipeline (sustainment → perception → decisions → movement → combat/item actions → results), `stop()`, and `snapshot()` assembly. |
| `src/unit.rs` | Componentized runtime `Unit` state (`Vitals`, `NavigationState`, `PerceptionMemory` with canonical `threats` + `primary_id`, `CombatState` with per-slot `WeaponState`, `SustainmentState`, `Inventory`) and low-HP / equipped-weapon withdrawal queries. |
| `src/map.rs` | Static terrain queries: map bounds and clearance (`inside`, `blocked_t`), surface lookup, NW-to-SE triangulated `ground_height`, unified ray evaluation (`visible_t`, `clear_t`, `shot_fraction_t`, `detectable_t`, `exposure_t`), swept-radius clearance (`sweep_clear_t`), segment `travel_time_t`, and single-pass `advance_toward_t`. |
| `src/terrain.rs` | Static per-battle `TerrainCache` (pre-rasterized cell occupancy/obstacle heights, `CellSurface`, flat-map fast path, and precomputed 8-neighbour travel-time table) and `Terrain<'_>` view. |
| `src/navigation.rs` | Deterministic 8-neighbor weighted pathfinding (`SearchScratch` with generation-stamped buffers, single-goal A*, multi-target Dijkstra) and cached per-unit `Route` planning and advancement (`Fastest` and `Protected`). |
| `src/perception.rs` | Per-unit hostile spotting (`observe_all_with_grid`, `refresh_post_movement`) backed by `SpatialGrid` and `is_hostile`, sorted by distance then unit ID, respecting spotting range, vegetation, and firing reveal. |
| `src/spatial.rs` | Uniform 4 m CSR `SpatialGrid` over living units (`hp > 0.0`) with packed 32-byte `SpatialEntry` records and per-cell 64-bit faction bitmasks for perception, grenade blast/ally safety, and fallback ally queries. |
| `src/compact_snapshot.rs` | Fixed-stride 72-byte little-endian binary snapshot encoder (`write_units`) for high-frequency unit state and `binary_search_by_key` target lookups. |
| `src/decisions/` | Modular tactical doctrine evaluation (`mod.rs`, `fallback.rs`, `engage.rs`, `reposition.rs`, `investigate.rs`, `patrol.rs`): bounded candidate scans, faction `PatrolDirection`, and typed proposals producing `Decision` intentions without mutating world state. |
| `src/item_actions.rs` | Per-tick candidate-proposal action selection (`ActionProposal` + `ActionPriority`) using typed `ItemId` / `ItemCategory` / `WeaponId` lookups: chooses at most one action per soldier per tick (medical → water → ration → grenade → weapon fire) according to doctrine and capacity. |
| `src/inventory.rs` | Game-wide item specifications (`ITEMS`, no starting quantities), typed `ItemId`, `Inventory::from_loadout`, hand/stow capacity accounting (`Inventory`), atomic `use_item_by_id`/`consume_item`, weight/volume load factor, and inventory snapshot generation. |
| `src/consumables.rs` | Consumable constants (blast radius, grenade damage/cooldown, medical heal, need threshold/restore) and `ItemUseProperties` catalog entries. |
| `src/combat.rs` | Post-movement perception refresh, per-slot weapon/action cooldown ticking, `item_actions::select`, item uses, grenade blasts, RNG weapon hit rolls, firing reveal, and simultaneous damage and `max_hp`-clamped healing. |
| `src/movement.rs` | Executes movement goals along cached `Route`s using the load/needs-scaled fixed step budget and computes live `movement_multiplier`. |
| `src/sustainment.rs` | Per-tick nutrition/hydration drain, shortage thresholds, and needs mobility factor. |
| `src/validation.rs` | Structural safety gate (`structural_problem`, `ENGINE_MAX_*`), mode-decoupled `validate_scenario` / `validate_skirmish_scenario` (`ModeRules`), custom loadout and doctrine validation with typed `IssueCode`/`IssueSeverity`, and `analyze_scenario` (cell readouts, steep edges, located errors/warnings). |
| `src/results.rs` | Hostility-based terminal battle evaluation across arbitrary factions (`ResultReason::{Elimination, SimultaneousElimination, Timeout, Stopped}` with nullable `time_limit`). |
| `src/definitions.rs` | Game-wide unit (`UnitSpec` + default loadouts), weapon (`WeaponSpec`), surface, and default faction registries, `TUNING`, map presets, `editor_rules`, `doctrine_rules`, and `catalog()`. |
| `src/bin/export_types.rs` | Thin binary invoked by `npm run gen:types` to print `sim_core::export_typescript_types()`. |
| `src/oracles.rs` | `#[cfg(test)]` frozen geometry and pathfinding reference oracles guarding hot-path optimizations. |
| `tests/boundaries.rs` | Enforces that `sim-core` never depends on browser or WASM crates. |
| `tests/behavior.rs` | End-to-end native simulation, terrain, doctrine, inventory, and reproducibility tests. |

`crates/sim-wasm` is a thin JSON/WASM adapter over `sim-core` (`catalog`, `validate_scenario`, `analyze_scenario`, `Battle`). Never put gameplay rules or validation logic inside `crates/sim-wasm`. `crates/sim-bench` is a native-only workspace crate providing release benchmarks (`bench`), multi-seed doctrine reports (`report`), and baseline outcome fingerprints (`crates/sim-bench/fingerprints.json`).

## Key invariants

- **Browser independence:** `sim-core` must never reference or transitively depend on `wasm-bindgen`, `web-sys`, `js-sys`, `gloo`, or `wasm-streams` (enforced by `tests/boundaries.rs`).
- **Fixed 20 Hz simulation step:** `STEP = 0.05` s. Never depend on wall-clock timers, system randomness, or frame rate.
- **Seeded RNG and ID-sorted determinism:** `Battle::new` sorts units by `placement.id`. Observations, threat memories, and path ties break deterministically by ID/index. Randomness uses only the explicit battle RNG state seeded from `scenario.seed`. Identical scenario + seed must produce identical snapshots in one environment.
- **Decisions separate from execution:** `decisions.rs` and `item_actions.rs` compute intentions/actions from immutable state; `movement.rs` and `combat.rs` execute them.
- **No unseen enemy leaks:** Perception, memory, protected routing, and tactical decisions must never read unseen enemy positions.
- **Pre-fire simultaneous reveal and damage:** In `combat.rs`, all living units observe and select actions against the same pre-fire reveal state. Firing reveal, RNG rolls, and damage apply simultaneously; lethal damage takes priority over same-tick healing (a committed medical kit is still consumed).
- **`ts-rs` contract parity:** Every public wire struct/enum in `src/types/` must carry matching `#[serde(...)]` and `#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ...)]` attributes. Any new wire root must be visited in `export_typescript_types()`, covered in `ts_export_tests`, and regenerated with `npm run gen:types`.
- **Code hygiene:** Target files under ~400 lines where practical, with a brief header comment describing ownership and invariants. Handle expected failure states explicitly instead of panicking.

## Checklists and recipes

### Adding an item or weapon

1. Confirm base materials stay game-wide in Rust ([ADR 0002](../../docs/decisions/0002-base-materials-and-custom-loadouts.md)); check human-approval rules if adding dependencies or changing scope docs.
2. Add the item specification to `ITEMS` in `src/inventory.rs` (ID, name, `ItemCategory`, `mass`, `volume`, `hands`, optional `weapon`).
3. Wire definitions:
   - **Weapon:** Add its `WeaponDefinition` and lookup in `src/definitions.rs`, and include it in `catalog().weapons`.
   - **Consumable / usable item:** Add tuning constants and its `ItemUseProperties` entry in `src/consumables.rs`.
4. Wire behavior:
   - Add eligibility and priority handling in `src/item_actions.rs` (respecting one action per tick and atomic capacity checks via `Inventory::can_use`).
   - Add execution effects in `src/combat.rs` (or the appropriate execution module), keeping reveal and simultaneous damage semantics intact.
5. If wire types in `src/types/inventory.rs` changed, update `ts_export_tests` in `src/types/mod.rs` and run `npm run gen:types`.
6. Add native tests covering capacity/weight/volume accounting, action selection, execution, and determinism; update [INVENTORY.md](../../INVENTORY.md) if rules changed; run `npm run verify:fast`.

### Adding a doctrine option

1. Pause for human approval if changing the meaning of an existing doctrine or updating outcome fingerprints ([ADR 0007](../../docs/decisions/0007-agent-workflow-and-human-checkpoints.md)).
2. Add the field/enum to `src/types/doctrine.rs` with matching `serde` and `ts-rs` attributes (`rename_all = "camelCase"`), `Default` (plus `#[serde(default)]` where backward-compatible), and any numeric `Rule` in `DoctrineRules`.
3. Populate defaults and numeric bounds in `src/definitions.rs` (`catalog().default_doctrine`, `doctrine_rules()`).
4. Add range and consistency checks in `src/validation.rs`.
5. Consume the option in `src/decisions.rs` (positioning/movement intentions) and/or `src/item_actions.rs` (per-tick combat/item actions).
6. Update `ts_export_tests` in `src/types/mod.rs` and run `npm run gen:types`. If portable scenarios need a `schemaVersion` bump, get human approval and follow [apps/web/AGENTS.md](../../apps/web/AGENTS.md) for UI and migration wiring.
7. Add controlled same-scenario, same-seed tests proving the option produces an observable, understandable difference in unit behavior ([AGENTS.md](../../AGENTS.md) rule 10), then run `npm run verify:fast`.

### Adding a terrain surface

1. Add the enum variant to `Surface` in `src/types/map.rs` (inherits `camelCase` for both `serde` and `ts-rs`).
2. Add its `SurfaceDefinition` (name, `movement_multiplier`, `spotting_multiplier`, `vegetation_height`, display `color`) in `surface_definition()` and include it in `catalog().surfaces` in `src/definitions.rs`.
3. Verify `src/map.rs` queries (`movement_multiplier`, `detectable`, `exposure`) work through `surface_definition()` without hardcoded surface lists.
4. Add the new variant to `assert_enum_variants_match_ts` for `Surface` in `src/types/mod.rs`, then run `npm run gen:types`.
5. Check frontend display and Map Maker palette handling (most metadata comes from `catalog.surfaces`).
6. Add Rust tests in `src/map.rs` / `tests/behavior.rs` covering movement speed and spotting/concealment effects; update [TERRAIN.md](../../TERRAIN.md); run `npm run verify:fast`.

### Adding a scenario field

1. Decide whether portable scenario compatibility requires bumping `schemaVersion` — **bumping `schemaVersion` requires human approval** ([ADR 0007](../../docs/decisions/0007-agent-workflow-and-human-checkpoints.md)).
2. Add the field to `Scenario`, `UnitPlacement`, or `ArenaMap` in `src/types/scenario.rs` (or `src/types/map.rs`) with matching `serde` and `ts-rs` attributes.
3. Update `Catalog` defaults and presets in `src/definitions.rs`, and add validation/analysis checks in `src/validation.rs` (keep safety bounds in `structural_problem`; keep editable draft/start checks in `validate_scenario` and `analyze_scenario`).
4. Wire simulation behavior in `src/battle.rs`, `src/results.rs`, or the owning subsystem.
5. Update `ts_export_tests` in `src/types/mod.rs` and run `npm run gen:types` (verify with `npm run check:types-sync`).
6. Wire UI controls, `blankDocument`, and legacy in-memory migration in `apps/web` (see [apps/web/AGENTS.md](../../apps/web/AGENTS.md)).
7. Add Rust validation/simulation tests and browser round-trip tests; update [MAP_MAKER.md](../../MAP_MAKER.md) if the portable JSON envelope changed; run `npm run verify:fast`.

## Useful commands

```sh
npm run test:rust        # Native Rust unit, behavior, oracle, doctrine suite, boundary, and TS-parity tests
npm run lint:rust        # Clippy (--all-targets --all-features -D warnings)
npm run fmt:check        # rustfmt check
npm run gen:types        # Regenerate apps/web/src/generated/sim-types.ts
npm run check:types-sync # Verify generated TS contract matches sim-core
npm run deny:rust        # Licence, advisory, ban, and source policy check (deny.toml)
npm run bench:rust       # Release benchmark sweep with stage timing breakdown
npm run bench:rust:quick # Quick release benchmark smoke run
npm run report:doctrine  # Headless multi-seed doctrine report (13 scenarios x 10 seeds)
npm run verify:fast      # Fast full-stack verification (run on every change)
npm run verify           # Full pre-merge verification
```
