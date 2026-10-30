# Architecture

Status: planning baseline, not an implementation report.

Source: [Game Architecture Options](https://chatgpt.com/c/6ae4ac8c-6904-83eb-a33e-2944fd754973), 30 October 2026. The final Rust/WebAssembly direction supersedes the earlier proposal to start the simulation in TypeScript and migrate later. Supporting tools below reflect the latest recommended baseline; preferences and unresolved choices are identified separately.

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
| Local persistence | IndexedDB and exportable save/scenario files |
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
- Separate static weapon and doctrine definitions from changing unit state.
- Use faction IDs rather than a fixed two-team boolean; the MVP still presents only two teams.
- Use a fixed simulation timestep independent of rendering frequency.
- Use seeded randomness to support repeatable scenarios and doctrine comparisons. Cross-platform determinism still requires verification.
- Keep world coordinates separate from screen pixels.
- Let map generators and future importers produce a common map representation.
- Let every game mode produce a common scenario for the same simulation.
- Exchange bulk commands and compact snapshots across the Rust/JavaScript and Worker boundaries. Avoid per-entity calls each frame.
- Start with ordinary messages and compact buffers. Shared memory and additional simulation threads are later options.

## Proposed organization

Keep one repository with a browser application, a pure Rust simulation crate, a WASM adapter, and a native benchmark runner. Assets and scenarios remain separate from engine code. Exact folder names and public API signatures are not yet fixed.

## Open choices and deferred work

- Lightweight Rust ECS versus explicit packed component stores; no ECS library has been selected.
- Final doctrine evaluation algorithm, rule conflicts, inheritance, and editor detail.
- Exact timestep, snapshot format, update rates, and persistence schema.
- Chunk layout and optional elevation/material fields; the MVP needs only flat terrain, obstacles, and cover.
- Replay determinism, recording format, and seeking.
- Hosting provider, dependency versions, and supported browser/device matrix.

Do not implement campaign services, map importers, multiplayer, LLM integration, terrain deformation, or large-scale simulation machinery merely to preserve the possibility of adding them.

Related: [scope](SCOPE.md), [scale](SCALE.md), [MVP](MVP.md), [dream goals](DREAM_GOALS.md).
