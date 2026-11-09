# 2nd Hand Combat

2nd Hand Combat is a browser strategy game about indirect control. Place soldiers, configure their doctrine, and watch them fight autonomously. The current Skirmish slice is a small infantry sandbox; campaign and puzzles remain future work.

Skirmish uses one fixed 32 × 20 map with walls and cover, two factions of up to eight infantry each, and one rifle. Select a soldier to adjust engagement distance and the health threshold for falling back. Start a battle with at least one soldier per faction, pause or resume it, and reset to the starting setup to compare doctrine. Results show the outcome and losses. The map is fully visible to the player; soldiers have their own limited perception. Options for fullscreen and health indicators last only for the current app session. There are no saves yet.

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

The initial sprint was verified with 29 native tests, eight production-browser tests, type checks, Clippy, rustfmt, and a production build. Splash and setup layouts were also visually inspected. Actual background-tab pause still needs a manual check: start a battle, switch to another tab, then return and confirm that the battle is paused until Resume is pressed. The automated browser kept reporting itself visible, and the desktop verification tool could not safely identify the browser URL.

An open-source or possibly Creative Commons release is intended later. No license has been selected or granted. The project is not currently licensed for reuse.
