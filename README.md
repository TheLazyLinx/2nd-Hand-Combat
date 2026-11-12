# 2nd Hand Combat

2nd Hand Combat is a browser strategy game about indirect control. Place soldiers, configure their doctrine, and watch them fight autonomously. The current Skirmish slice is a small infantry sandbox; campaign and puzzles remain future work.

Skirmish offers three handcrafted 32 × 20 terrain maps, two factions of up to eight infantry each, and one rifle. Select a soldier to adjust engagement distance, the health threshold for falling back, position preference, and route preference. Hills, depressions, directional cover, vegetation, and surface/slope movement costs affect combat. Start a battle with at least one soldier per faction, pause or resume it, and reset to the starting setup to compare doctrine. Results show the outcome and losses. The map is fully visible to the player; soldiers have their own limited perception. Options for fullscreen and health indicators last only for the current app session. Map Maker creates full scenarios up to 128 × 128 cells, with a browser library and portable JSON backups. See [TERRAIN.md](TERRAIN.md) for terrain rules and [MAP_MAKER.md](MAP_MAKER.md) for authoring, resizing, and storage.

The current browser target is desktop Chromium. Rust owns the simulation and runs in WebAssembly in a dedicated Worker; Svelte and PixiJS provide the interface and battlefield view. See [MVP.md](MVP.md) for the implemented slice and remaining first-playable scope, and [ARCHITECTURE.md](ARCHITECTURE.md) for boundaries.

## Development

Install Node.js 22.12 or newer, stable Rust with the `wasm32-unknown-unknown` target, and `wasm-pack`. The Rust Clippy and rustfmt components are needed for their checks. On Windows, the usual MSVC Rust toolchain requires the Visual Studio C++ build tools. This workspace can instead use a portable GNU toolchain under `.tools`; `scripts/tooling.mjs` picks it up automatically when present. `.tools` is local tooling, not a prerequisite for other environments.

From the repository root:

```sh
npm ci
npx playwright install chromium
npm run dev
```

The Playwright browser installation is needed once per environment for browser tests. The development command builds the WASM adapter before starting Vite; rerun it after changing Rust code. Open the local URL printed by Vite.

Available checks and production commands:

```sh
npm run build       # Build WASM and the production site in dist/
npm run preview     # Serve an existing production build locally
npm run check       # Svelte and TypeScript checks
npm run test:rust   # Native Rust tests
npm run lint:rust   # Clippy
npm run fmt:check   # Rust formatting check
npm run test:e2e    # Build and run Playwright browser tests
```

`npm run check` requires generated WASM bindings, created by `npm run dev` or `npm run build`. Native tests exercise the simulation, and browser tests exercise the real Worker/WASM path.

The terrain and Map Maker implementation was verified with 75 native tests, 31 production-Chromium browser tests, type checks, Clippy, rustfmt, and a production build. Browser checks exercise the real Worker/WASM path, terrain editing, resize/undo, invalid drafts, library operations and storage failures, JSON round trips, scenario loading, and pause/reset. Performance checks cover 16-soldier battles on all three presets and a representative 128 × 128 mixed-terrain map, plus rendering on an extreme-elevation map. Contours, legends, selection, resize previews, and layouts at 900 and 1440 pixels were visually inspected. These short checks establish the tested scenarios, not capacity for arbitrary terrain or future battle sizes; terrain balance still needs playtesting.

Actual background-tab pause still needs a manual check: start a battle, switch to another tab, then return and confirm that the battle is paused until Resume is pressed. The automated browser reports itself visible during that check.

An open-source or possibly Creative Commons release is intended later. No license has been selected or granted. The project is not currently licensed for reuse.
