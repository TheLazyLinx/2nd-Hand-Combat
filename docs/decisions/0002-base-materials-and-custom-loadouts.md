# 0002 — Game-wide base materials and custom loadouts

- **Status:** Accepted
- **Date:** 2026-11-19
- **Phase:** 3
- **Related:** [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [INVENTORY.md](../../INVENTORY.md), [MAP_MAKER.md](../../MAP_MAKER.md)

## Context

Inventory v1 gives every soldier the same hardcoded starting loadout. As the game expands to multiple weapons, consumables, and tactical roles, scenario authors need a way to equip units differently without allowing untrusted portable JSON scenarios to invent arbitrary item stats, bypass capacity rules, or break game-wide balance updates.

## Decision

1. **Base materials are game-wide.** Unit types (`UnitDefinition`), weapons (`WeaponDefinition`), and items/consumables (`ItemDefinition`, typed `ItemUseProperties`) live exclusively in the shared Rust game catalog. Scenarios cannot define custom base items, weapons, or unit stat blocks.
2. **Separate definitions from starting gear.** Remove starting quantities from base item specifications (`ItemSpec`) and model starting equipment as a `Loadout` (catalog item IDs and quantities) attached to each `UnitDefinition`.
3. **Allow custom per-scenario and per-unit loadouts.**
   - A scenario may declare named loadouts (for example, `"Marksman"`) composed of catalog item IDs and quantities.
   - Each `UnitPlacement` retains its `definition_id` and may select a named scenario loadout or specify an inline loadout. Omitting both falls back to the unit type's `default_loadout`.
4. **Validate loadouts in Rust.** Scenario validation verifies that every custom loadout references known catalog items, fits within the unit type's weight and stowed-volume capacities, and satisfies slot/compatibility constraints.

## Consequences

- Portable scenario files stay compact, safe to validate, and automatically benefit from game-wide weapon and item tuning.
- Data structures support diverse unit roles and multiple weapons without requiring a loadout editor UI before one is requested ([AGENTS.md](../../AGENTS.md) rule 9).
- Introduces a scenario `schemaVersion` bump (v2 → v3 in Phase 3); v1 and v2 documents migrate in memory to default loadouts without modifying stored IndexedDB records automatically.
