# 0005 — Full migration to Svelte 5 runes

- **Status:** Accepted
- **Date:** 2026-11-19
- **Phase:** 7
- **Related:** [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [apps/web/AGENTS.md](../../apps/web/AGENTS.md)

## Context

The frontend already runs on Svelte 5 and uses Svelte 5 DOM event syntax (`onclick`, `oninput`, `onchange`), yet existing components still rely on legacy Svelte 4 declarations (`export let`, `$:`). Maintaining mixed reactivity paradigms across the same UI codebase invites subtle reactivity bugs and inconsistent patterns when agents edit components.

## Decision

1. **Migrate every component in `apps/web/src` to Svelte 5 runes** (`$props`, `$state`, `$derived`, `$effect`), removing all `export let` and `$:` syntax across `App`, `Skirmish`, `MapMaker`, `DoctrineEditor`, `FactionPanel`, `InventoryDisplay`, `Results`, and `UnitPicker`.
2. **Migrate incrementally.** Convert one component at a time, keeping `npm run check`, `npm run lint:web`, and Playwright browser tests green after each change.
3. **Runes-only for new components.** Any newly authored Svelte component must use Svelte 5 runes from the start.

## Consequences

- One consistent reactivity model and stricter TypeScript prop/state inference across the entire web shell.
- Splitting large components (`Skirmish.svelte`, `MapMaker.svelte`) in Phase 7 becomes cleaner because shared props and derived state are explicit.
- Requires dedicated incremental verification so behavior and accessibility attributes remain unchanged during migration.
