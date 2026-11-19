# Architecture

Status: committed direction and current Skirmish implementation, with future choices identified below.

Source: Game Architecture Options planning discussion, 30 October 2026. The final Rust/WebAssembly direction supersedes the earlier proposal to start the simulation in TypeScript and migrate later. Supporting tools below reflect the latest recommended baseline; preferences and unresolved choices are identified separately.

## Committed direction

- The game runs in a browser from a normal link, including a link on a CV.
- Visitors do not install software, run commands, or launch a downloaded executable.
- Use Rust compiled to WebAssembly for gameplay simulation from the beginning.
- Use TypeScript for presentation, browser integration, and player input.
- Keep simulation, decision-making, rendering, and interface responsibilities separate.
- Build a small first playable game while preserving practical extension points.

## Technical baseline

| Responsibility | Baseline |
| --- | --- |
| Simulation | Rust |
| Browser execution | WebAssembly |
| Rust/browser bindings | wasm-bindgen and wasm-pack |
| Browser application | TypeScript |
| 2D rendering | PixiJS, initially WebGL |
| Menus and doctrine editor | Svelte |
| Builds | Vite and Cargo |
| Simulation execution | One dedicated Web Worker, initially single-threaded |
| Local persistence | IndexedDB scenario library and versioned JSON import/export |
| Verification | Native Rust simulation tests and browser integration tests |

Static hosting is the intended deployment model. Cloudflare Pages is the preferred option discussed; GitHub Pages or another static host remains possible. The initial combat simulation runs on the visitor's device and does not require a simulation server or account system. First-load size and initialization time matter because opening the link is the entry experience.

## Ownership and boundaries

| Module | Responsibility |
| --- | --- |
| Game shell | Splash screen, settings, mode selection, setup, and results |
| Scenario layer | Map, seed, factions, starting units, equipment, doctrines, restrictions, and victory conditions |
| Doctrine and decisions | Interpret observations and configured rules; produce intentions |
| Simulation core | Own authoritative world state; execute movement, navigation, combat, damage, and terrain interactions |
| Browser adapter | Load the Rust engine and exchange commands, snapshots, events, and results |
| Renderer | Display simulation snapshots, terrain, sprites, effects, and camera views |
| Persistence | Store and load versioned scenarios, doctrine definitions, and later progress data |

The renderer does not determine hits, casualties, movement outcomes, or victory. The simulation owns those outcomes.

Keep the Rust simulation core independent of browser APIs, WebAssembly bindings, PixiJS, and Svelte. A separate WASM adapter exposes it to the browser. The same core must be usable in native headless tests and benchmarks.

Keep perception, doctrine evaluation, and intentions separate from the movement and weapon systems that execute intentions. Future manual or external controllers can use that boundary without replacing combat mechanics.

## Data and execution

- Use entity IDs and composable, data-oriented state rather than a deep soldier/vehicle class hierarchy.
- Separate static equipment and doctrine definitions from changing unit state. Inventory and needs belong to units, separate from perception; decisions choose actions and execution applies them.
- Use faction IDs rather than a fixed two-team boolean; the MVP still presents only two teams.
- Use a fixed simulation timestep independent of rendering frequency.
- Use seeded randomness to support repeatable scenarios and doctrine comparisons. Cross-platform determinism still requires verification.
- Keep world coordinates separate from screen pixels.
- Let map generators and future importers produce a common map representation.
- Let every game mode produce a common scenario for the same simulation.
- Exchange bulk commands and compact snapshots across the Rust/JavaScript and Worker boundaries. Avoid per-entity calls each frame.
- Start with ordinary messages and compact buffers. Shared memory and additional simulation threads are later options.

## Current implementation

- `crates/sim-core` is browser-independent Rust (enforced by `crates/sim-core/tests/boundaries.rs`). It owns scenario validation and analysis, terrain queries, uniform spatial-grid queries (`spatial.rs`), perception, doctrine decisions, inventory and sustainment, movement, combat, seeded randomness, compact binary snapshot encoding (`compact_snapshot.rs`), and battle results. Decisions and item-action selection produce intentions that movement and combat execute.
- `crates/sim-wasm` adapts `sim-core` for the browser. Catalog, scenario validation/analysis, battle creation, and small-battle snapshots cross the WASM boundary as bulk JSON; stepping advances the battle in batches, and larger battles (`>= 32` units) encode high-frequency unit state as fixed-stride binary `Uint8Array` buffers (`72` bytes/unit) transferred zero-copy from the Worker alongside version-gated inspected-unit inventory updates.
- `apps/web` contains the Svelte shell, PixiJS WebGL battlefield (with cached per-faction sprite textures, batched HP overlays, and camera viewport culling), one dedicated Worker, and the typed Worker protocol (`apps/web/src/lib/types.ts`). Wire types are generated from `crates/sim-core` via `ts-rs` into `apps/web/src/generated/sim-types.ts` and checked for parity in CI. ESLint boundary rules (`eslint.config.js`, guarded by `tests/boundaries.test.mjs`) keep WASM inside `worker.ts`, Worker construction inside `BattleClient`, and rendering snapshot-only inside `battlefield.ts`.
- Simulation steps are fixed at 20 Hz, with decisions running on a lower cadence plus event-driven interrupts. The Worker builds snapshots only when publishing (~10 Hz, pause, stop, or terminal result), applies a 5-step catch-up cap, and emits a `Simulation behind real time` status indicator if wall time is clamped, while rendering interpolates presentation independently on the main thread.
- Verification combines native Rust tests, `cargo-deny` dependency policy checks, generated-contract sync checks, boundary and privacy checks, a native benchmark and multi-seed doctrine suite (`crates/sim-bench`), and Playwright browser tests over the real Worker/WASM path (currently targeting desktop Chromium). GitHub Actions runs `.github/workflows/verify.yml` alongside `.github/workflows/privacy.yml`. See [docs/decisions/](docs/decisions/README.md) for recorded decisions.

Maps consist of row-major surface cells, vertex elevations triangulated along each cell's northwest-to-southeast diagonal, and rectangular solid obstacles with heights relative to ground. Three handcrafted presets ship in the catalog. Shared Rust terrain queries own physical sight, directional shot exposure, vegetation spotting, swept-radius traversal, and segment travel time; the renderer only presents terrain and live status. See [TERRAIN.md](TERRAIN.md) for rules and tuning.

Navigation plans deterministic weighted routes from terrain travel times and each unit's observed and remembered threats, without reading unseen enemy positions. Movement spends the fixed-step time budget scaled uniformly by load and needs without changing route topology. Combat resolves post-movement visibility, per-tick item/weapon actions, and simultaneous fire and damage against a shared pre-fire reveal state.

Soldier inventory uses bounded weight and stowed volume, starting equipment from the Rust catalog, and at most one action per soldier per tick. Catalog initialization and snapshots carry inventory state in bulk. Scenario schema version 2 records inventory doctrine; version 1 documents normalize in memory from Rust catalog defaults without automatic rewrites or an IndexedDB schema change. See [INVENTORY.md](INVENTORY.md).

Initialization delivers static maps, surface definitions, item/weapon definitions, and editor limits in one catalog payload. Bulk Rust scenario analysis supplies cell readouts, located issues, and start readiness; structurally safe drafts may hold editable layout errors, while malformed structures are rejected before terrain queries. Revision tokens guard asynchronous editor analysis replies. Snapshots carry live unit state and terrain status without repeating static map geometry, and the PixiJS renderer caches static terrain presentation separately from snapshots.

Map Maker edits portable scenario documents independently from live simulation state. TypeScript owns authoring, undo/redo history, IndexedDB storage, and JSON import/export; Rust owns map and scenario validity. Skirmish runs detached scenario copies, screen transitions dispose the active Worker and renderer, and returning from an editor test restores the retained editor session. See [MAP_MAKER.md](MAP_MAKER.md).

## Open choices and deferred work

- Whether future division-scale dense combat (> 1,000 close-contact units) warrants hierarchical pathfinding, visibility caching, or formation aggregation; Phase 6 profiling confirmed componentized `Vec<Unit>` plus packed 32-byte `SpatialEntry` grids are sufficient without a third-party ECS crate (see [ADR 0008](docs/decisions/0008-scaling-spatial-grid-and-storage-layout.md)).
- Final doctrine evaluation algorithm, rule conflicts, inheritance, and editor detail.
- Future timestep, snapshot representation, update rates, and persistence schema beyond the current slice.
- Chunk layout for maps larger than the current small surface/elevation grids.
- Replay determinism, recording format, and seeking.
- Hosting provider, dependency versions, and supported browser/device matrix.

Do not implement campaign services, map importers, multiplayer, LLM integration, terrain deformation, or large-scale simulation machinery merely to preserve the possibility of adding them.

Related: [scope](SCOPE.md), [scale](SCALE.md), [MVP](MVP.md), [dream goals](DREAM_GOALS.md), [improvement plan](IMPROVEMENT_PLAN.md), [ADRs](docs/decisions/README.md).
