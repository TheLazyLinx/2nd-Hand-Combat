# AGENTS.md — 2nd Hand Combat

PROJECT: A 2D, sprite-based browser strategy game. Players configure unit doctrine (engagement range, cover use, withdrawal, etc.) and watch units fight autonomously. The first playable version is a small Sandbox battle, not a full campaign or large-scale war simulator. It must run from a link with no installation.

PRIORITIES: Correct behavior, readable code, clear module boundaries, smooth browser performance, and room to extend the game without building future features now.

PROJECT DOCUMENTS:
- `ARCHITECTURE.md` — technical decisions and boundaries.
- `MVP.md` — what to build first and what to exclude.
- `SCOPE.md` — gameplay direction and modes.
- `SCALE.md` — future performance goals, not proven capacity.
- `DREAM_GOALS.md` — ideas, not tasks.
- `README.md` — overview and licensing status.

Read `ARCHITECTURE.md` and `MVP.md` before implementing gameplay. Check the repository before assuming files, directories, commands, or tests exist. Where a document marks a choice as proposed or undecided, do not treat it as settled.

## CRITICAL RULES

1. **Rust owns the simulation.** Compile it to WebAssembly for the browser; keep the simulation core runnable natively for tests and benchmarks.
2. **Keep the Rust core independent of the browser.** WebAssembly bindings belong in a separate adapter. UI, rendering, and Worker code must not leak into simulation logic.
3. **Rendering does not change game state.** Movement, targeting, hits, damage, casualties, and victory are resolved by the simulation, never by PixiJS, Svelte, or rendering frame rate.
4. **Separate decisions from execution.** Perception and doctrine choose intentions; movement and combat systems carry them out. Do not bury doctrine logic in the renderer or UI.
5. **Use fixed simulation steps and seeded randomness.** Rendering frequency must not affect outcomes. Repeatability within one test environment matters; cross-platform determinism is not yet guaranteed.
6. **Keep world and screen coordinates separate.** Use entity IDs and data-driven definitions for units, weapons, and doctrine. Avoid unnecessary inheritance and one-off hardcoding.
7. **Use one simulation Worker initially.** Pass commands and snapshots in batches. Do not make per-unit WASM calls every frame or add threads, shared memory, or a backend without a measured need.
8. **The MVP is infantry Sandbox combat.** A few soldiers, weapons, obstacles, doctrine controls, autonomous fighting, and results. See `MVP.md` for details.
9. **Do not implement future features by default.** No campaign economy, puzzles, vehicles, multiplayer, map importers, LLM controllers, advanced doctrine editors, or division-scale systems unless explicitly requested.
10. **The core gameplay test is doctrine.** With the same scenario and seed, changing doctrine must produce an observable, understandable difference in unit behavior.
11. **Keep code modular.** Do not put unrelated logic in entry points, UI components, render loops, or large catch-all files. Reuse existing code before creating new abstractions.
12. **Fix root causes.** Do not hide bugs with empty catches, ignored errors, misleading defaults, or disabled checks. Handle expected failures explicitly and surface unexpected ones.
13. **Optimize where it matters.** Avoid wasteful work in simulation loops and unnecessary allocations or boundary crossings. Profile before adding complex scaling systems; `SCALE.md` is not a performance guarantee.
14. **Inspect before editing.** Follow existing patterns and make the smallest coherent change. Do not overwrite unrelated work, refactor unrelated modules, or add dependencies without a reason.
15. **Do not invent tooling.** Use commands and scripts supported by the actual repository. Do not add throwaway scripts, generated files, or scaffolding without a task-specific need.
16. **Keep the game safe to distribute.** Do not hardcode secrets or add assets with unclear usage rights. No license has been selected; do not add or declare one without approval.
17. **Do not make repository-level changes unasked.** No commits, pushes, pull requests, publishing, or changes to repository settings unless the task calls for them.
18. **Ask only when blocked by a material decision.** For a larger change, state a brief plan. For routine details, use existing conventions and proceed.

## TEST RULES

1. Test **observable behavior**, not merely object construction, private fields, or dependency wiring.
2. For simulation work, prefer native Rust tests. Verify decisions, movement, targeting, damage, battle results, and relevant failure cases.
3. Use controlled scenarios and seeds to test doctrine differences and reproducibility. Do not claim deterministic replay across platforms without evidence.
4. For browser/adapter work, verify the real Worker/WASM path where practical. Mock-only tests do not establish that integration works.
5. Keep tests focused: one behavior per test, clear inputs, and meaningful assertions. Cover failure paths for invalid inputs or broken interfaces.
6. Do not accept tests whose only check is "does not throw" when an observable result can be asserted.
7. Run the relevant available tests and build checks after changes. Do not claim a test passed if it was not run; do not invent commands for tests that do not exist.

## TASK HANDOFF

At the end of a task, report briefly:
- Changed: what and where.
- Tested: commands run and results, or why checks could not run.
- Remaining: known issues or decisions that still need an answer.

Keep the repository understandable without relying on chat history or an agent's private notes.
