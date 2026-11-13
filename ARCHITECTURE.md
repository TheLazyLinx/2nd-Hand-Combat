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

- `crates/sim-core` is browser-independent Rust. It owns scenario validation, observations, doctrine decisions, movement, combat, seeded randomness, and battle results. Decisions produce intentions that movement and combat execute.
- `crates/sim-wasm` adapts that core for the browser. Catalog, scenario validation, battle creation, and snapshots cross the WASM boundary as bulk JSON strings; stepping advances the battle in batches.
- `apps/web` contains the Svelte shell, PixiJS WebGL battlefield, one Worker, and the typed Worker command and reply contract. The Worker owns the WASM battle and sends snapshots to the UI. The renderer interpolates presentation only; it does not resolve gameplay.
- Simulation steps are fixed at 20 Hz. Sight refreshes each step; doctrine decisions normally run at 2 Hz and can be requested earlier by relevant state changes. The Worker publishes snapshots at approximately 10 Hz and caps catch-up at five steps after a stall. Rendering runs independently of these rates.
- Native Rust tests and Playwright tests for the real browser Worker/WASM path are present. The current browser target is desktop Chromium. The game has no backend; the browser owns the local scenario library.

The current implementation uses three handcrafted map presets and explicit Rust structures. Maps contain row-major surface cells, vertex elevations, and rectangular solid obstacles with heights relative to the ground. Each cell is triangulated along its northwest-to-southeast diagonal. Shared Rust queries resolve physical sight, directional shot exposure, vegetation spotting, swept-radius traversal, and travel time; the renderer only displays their results. See [TERRAIN.md](TERRAIN.md) for the rules and initial tuning.

Navigation uses deterministic eight-neighbour weighted searches and cached per-unit routes. A decision reuses one search when comparing candidate positions and skips it when no candidate improves the current position. Protected searches memoize repeated exposure samples and prune edges using travel-time lower bounds. Protected routing considers observed threats and per-enemy last sightings, without reading unseen enemy positions. Movement consumes a fixed-step time budget across surface and triangle boundaries. Load and needs scale that budget uniformly without changing route topology; see [INVENTORY.md](INVENTORY.md). Combat rechecks visibility after movement; simultaneous fire and damage use the same pre-fire reveal state.

The catalog sends static maps, surface definitions, and supported editor limits during initialization. Bulk Rust scenario analysis supplies readouts for the actual edited or selected map, located issues, and start readiness. Structurally safe drafts may retain layout errors; malformed structures never reach terrain queries. Editor revisions guard asynchronous analysis results. Snapshots include live unit terrain status and protection relative to the current target. Static map data is not repeated in snapshots. Pixi caches static terrain at bounded display resolution, invalidating it when maps, overlays, or scale change. Dense slope warnings are grouped at display resolution, and contour density adapts to large elevation ranges. Searches for known destinations stop once those destinations are settled, preserving full-map detours and weighted route costs. A native benchmark runner remains future work.

Map Maker edits portable scenario documents separately from live simulation state. TypeScript owns authoring and undo history; Rust owns map and scenario validity. IndexedDB stores local document records, while JSON exports carry versioned complete scenarios without browser-local record IDs. Skirmish starts and resets detached scenario copies. Screen transitions dispose the active Worker and renderer; returning from an editor test restores the retained editor session. See [MAP_MAKER.md](MAP_MAKER.md).

## Open choices and deferred work

- Whether future scale warrants a Rust ECS or packed component stores; no ECS library has been selected.
- Final doctrine evaluation algorithm, rule conflicts, inheritance, and editor detail.
- Future timestep, snapshot representation, update rates, and persistence schema beyond the current slice.
- Chunk layout for maps larger than the current small surface/elevation grids.
- Replay determinism, recording format, and seeking.
- Hosting provider, dependency versions, and supported browser/device matrix.

Do not implement campaign services, map importers, multiplayer, LLM integration, terrain deformation, or large-scale simulation machinery merely to preserve the possibility of adding them.

Soldier inventory uses fixed starting equipment, bounded weight and volume, and one action per soldier per tick. The catalog and snapshots carry inventory information in bulk. Scenario schema version 2 records the added doctrine; version 1 is normalized in memory from Rust catalog defaults without automatic writes or an IndexedDB schema change. See [INVENTORY.md](INVENTORY.md).

Related: [scope](SCOPE.md), [scale](SCALE.md), [MVP](MVP.md), [dream goals](DREAM_GOALS.md).
