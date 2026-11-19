# AGENTS.md — `apps/web`

Area guide for Svelte, TypeScript, Worker, and PixiJS frontend work. Follow root [AGENTS.md](../../AGENTS.md) (including human-approval checkpoints) and [ARCHITECTURE.md](../../ARCHITECTURE.md) first.

## Module ownership

| Path | Responsibility |
| --- | --- |
| `src/worker.ts` | Dedicated simulation Web Worker. Loads generated WASM (`src/generated/wasm/sim_wasm`), owns `WasmBattle`, runs the fixed-step catch-up pump (50 ms step, 5-step cap, snapshot-on-publish at ~10 Hz, behind-real-time status), switches battles with `>= 32` units to transferable binary `compactSnapshot` frames + inspected-unit inventory updates, and handles `WorkerCommand` / `WorkerReply`. |
| `src/lib/client.ts` | `BattleClient`: sole owner of `new Worker(...)`, request/response timeouts (`validate`, `analyze`), battle ID filtering, unit inspection requests, and Worker disposal. |
| `src/lib/compact-snapshot.ts` | Zero-copy `DataView` decoder (`CompactSnapshotView`) for 72-byte/unit transferable binary frames and on-demand inspected-unit `UnitSnapshot` reconstruction. |
| `src/lib/types.ts` | Re-exports generated simulation wire types from `src/generated/sim-types.ts` and defines browser-only contracts (`ScenarioDocument`, `WorkerCommand`, `WorkerReply`). Never hand-copy Rust types here. |
| `src/generated/sim-types.ts` | Auto-generated from `crates/sim-core` via `npm run gen:types`. Never edit by hand. |
| `src/generated/wasm/` | Auto-generated WASM bindings from `npm run build` / `npm run dev`. Never edit by hand. |
| `src/lib/battlefield.ts` | `Battlefield`: PixiJS WebGL presentation layer (terrain cache, contours, editor gizmos, camera, dynamic faction colours, texture-batched unit sprites, batched HP overlays, viewport culling, interpolated `Snapshot` and `CompactSnapshotView` frames). Display only — never resolves gameplay. |
| `src/lib/factions.ts` | Faction display helpers (colour hex conversion, lookup, palette generation), unit definition lookup, and setup-screen inventory preview. |
| `src/lib/scenario-library.ts` | IndexedDB scenario library (`2nd-hand-combat` / `scenarios`), 8 MiB JSON import/export, portable envelope validation, and in-memory legacy schema migration (v1/v2 → v3). Leaves gameplay/field validation to Rust. |
| `src/lib/editor.ts` | Pure Map Maker helpers (`EditorSession`, `blankDocument`, terrain `paint`, `resizeScenario`, undo/redo snapshots). |
| `src/App.svelte` | Top-level app shell: home splash, options, screen transitions, and retained Map Maker session state. |
| `src/components/Skirmish.svelte` | Skirmish setup, preset/library loading, live battle controls, and unit/terrain inspection. |
| `src/components/MapMaker.svelte` | Scenario authoring workspace, tools, resize confirmation, library dialogs, and Test-in-Skirmish bridge. |
| `src/components/DoctrineEditor.svelte` | Per-soldier doctrine editor controls (bounds sourced from `Catalog.doctrineRules`). |
| `src/components/FactionPanel.svelte` | Per-faction roster and add-soldier trigger. |
| `src/components/InventoryDisplay.svelte` | Starting and live soldier inventory, hands, weight/volume, and sustainment readout. |
| `src/components/Results.svelte` | After-action battle results summary. |
| `src/components/UnitPicker.svelte` | Modal unit-type picker for deployment. |
| `src/main.ts`, `src/style.css` | Application mount point and shared styles. |

## Enforced ESLint boundaries

Module boundaries are enforced by [eslint.config.js](../../eslint.config.js) (`npm run lint:web`) and covered by regression tests in [tests/boundaries.test.mjs](../../tests/boundaries.test.mjs) (`npm run test:boundaries`):

- **WASM isolation:** Only `apps/web/src/worker.ts` may import `generated/wasm/sim_wasm*`.
- **Worker isolation:** No module outside `worker.ts` may import `worker.ts` directly; UI code talks to the Worker only through `BattleClient`.
- **Single Worker factory:** Only `apps/web/src/lib/client.ts` (`BattleClient`) may invoke `new Worker(...)`.
- **Worker purity:** `apps/web/src/worker.ts` cannot import `pixi.js`, `svelte`, `components/*`, `lib/battlefield*`, or `lib/client*`.
- **Renderer purity:** `apps/web/src/lib/battlefield.ts` is snapshot-only presentation; it cannot import `svelte`, `lib/client`, `components/*`, `worker.ts`, or generated WASM.
- **No duplicated simulation rules:** Rendering never resolves hits, movement, sight, or victory, and TypeScript never duplicates Rust gameplay validation.

## Formatting, linting, typechecking, and Svelte 5 rules

- **Prettier:** Run `npm run fmt:web` after editing web/script/test files (`npm run fmt:web:check` in verification). Uses `prettier-plugin-svelte`, `printWidth: 100`, single quotes, trailing commas.
- **ESLint:** `npm run lint:web` runs with `--max-warnings=0`. Fix root causes instead of disabling rules.
- **Type checks:** `npm run check` runs `svelte-check` on `apps/web` and `tsc -p apps/web/tsconfig.worker.json`. Note: `npm run check` requires generated WASM bindings produced by `npm run build` or `npm run dev`.
- **Contract sync:** `npm run check:types-sync` fails if `src/generated/sim-types.ts` differs from `crates/sim-core`. Run `npm run gen:types` after any Rust wire-type edit.
- **Svelte 5 runes ([ADR 0005](../../docs/decisions/0005-svelte-5-runes-migration.md)):** Author all new components with Svelte 5 runes (`$props`, `$state`, `$derived`, `$effect`), never `export let` or `$:`. Existing components migrate one at a time in Phase 7 with browser tests green after each. Keep files focused (under ~400 lines where practical) with a short header comment stating ownership.
- **Browser tests (`tests/browser/*.spec.ts`, Playwright desktop Chromium, real Worker/WASM path):**
  - `@smoke` (`npm run test:browser:smoke`, part of `npm run verify:fast`): fast end-to-end checks across Skirmish, doctrine, inventory, Map Maker, and persistence.
  - Non-`@stress` (`npm run test:e2e:fast` / `npm run test:browser:fast`): full functional browser suite excluding long performance stress runs.
  - `@stress` (`npm run test:e2e` / `npm run test:browser` / `npm run verify`): long-running 16-soldier preset and 128 × 128 performance/rendering specs. Tag heavy multi-minute performance checks with `@stress` so `verify:fast` stays fast.

## Recipe: wiring a new scenario or doctrine field into the UI and `scenario-library.ts`

1. **Land the Rust contract first** (see [crates/sim-core/AGENTS.md](../../crates/sim-core/AGENTS.md)): add the field in `crates/sim-core/src/types/`, defaults/bounds in `definitions.rs`, and validation in `validation.rs`.
2. **Regenerate TypeScript types:** Run `npm run gen:types` and confirm `src/generated/sim-types.ts` includes the new field (re-exported by `src/lib/types.ts`). Never hand-edit `generated/sim-types.ts`.
3. **Update persistence and migration (`src/lib/scenario-library.ts`):**
   - If the field requires bumping `schemaVersion`, confirm human approval first ([ADR 0007](../../docs/decisions/0007-agent-workflow-and-human-checkpoints.md)), then update `ScenarioDocument['schemaVersion']` in `src/lib/types.ts` and `parseScenarioDocument` in `src/lib/scenario-library.ts`.
   - Normalize older schema versions **in memory** using `Catalog` defaults without rewriting stored IndexedDB records automatically.
   - Keep TypeScript checks limited to the portable envelope and version migration; let Rust (`validate` / `analyze`) check field values, enums, and gameplay ranges.
4. **Update default scenario builders:**
   - Update `blankDocument` (and any preset copy helpers) in `src/lib/editor.ts`.
   - Update initial Skirmish setup construction in `src/components/Skirmish.svelte` so new setups populate the field from `Catalog` defaults.
5. **Wire UI controls and readouts:**
   - **Doctrine fields:** Add controls in `src/components/DoctrineEditor.svelte`, sourcing min/max/step from `catalog.doctrineRules` rather than duplicating Rust constants, and propagate updates via `onChange(...)`.
   - **Scenario fields:** Add setup/property inputs in `src/components/Skirmish.svelte` and/or `src/components/MapMaker.svelte`. In Map Maker, push an undoable snapshot and trigger revision-guarded `client.analyze(...)`.
   - **Live readouts / rendering:** Display snapshot values in `Skirmish.svelte`, `InventoryDisplay.svelte`, `Results.svelte`, or pass them to `Battlefield` without computing simulation rules on the main thread.
6. **Add real-browser tests:** Add Playwright tests in `tests/browser/` verifying UI editing, IndexedDB/JSON round-trip (including legacy schema migration when relevant), and observable Worker/WASM behavior.
7. **Format and verify:** Run `npm run fmt:web`, then `npm run verify:fast` (and `npm run verify` before merging).

## Useful commands

```sh
npm run dev              # Build dev WASM and start Vite dev server
npm run build            # Build release WASM and production bundle in dist/
npm run fmt:web          # Format web, scripts, and tests with Prettier
npm run fmt:web:check    # Check formatting
npm run lint:web         # ESLint with boundary rules (--max-warnings=0)
npm run test:boundaries  # Regression tests for ESLint boundary rules
npm run check            # svelte-check + Worker TypeScript check (needs built WASM)
npm run check:types-sync # Verify generated TS types match crates/sim-core
npm run test:e2e:fast    # Build + non-@stress Playwright browser tests
npm run verify:fast      # Fast full-stack verification (run on every change)
npm run verify           # Full pre-merge verification (including @stress tests)
```
