# 0003 — Fire on the move as a doctrine option

- **Status:** Accepted
- **Date:** 2026-11-19
- **Phases:** 3 (reserved mechanical field), 4 (doctrine toggle and behavior)
- **Related:** [IMPROVEMENT_PLAN.md](../../IMPROVEMENT_PLAN.md), [MVP.md](../../MVP.md), [INVENTORY.md](../../INVENTORY.md)

## Context

In the current implementation, rifle firing eligibility in `item_actions.rs` relies on a hardcoded `Behavior` match: `FallingBack` units can fire while moving, `Advancing` and `Investigating` units cannot, and grenades ignore movement state entirely. Players should be able to choose doctrinally whether soldiers shoot on the move, cleanly separated from any future mechanical accuracy penalty.

## Decision

1. **Doctrinal toggle for the test phase, default off.** Add `Doctrine.fire_while_moving: bool` defaulting to `false`. Legacy schema v1 and v2 documents migrate to `false`.
2. **Doctrine-driven firing eligibility.** Replace the hardcoded `Behavior` match in `item_actions.rs` with doctrine rules:
   - Stationary behaviors may fire normally.
   - Moving behaviors (`Advancing`, `FallingBack`, `Investigating`) may fire only when `fire_while_moving` is enabled.
   - Align grenade throwing with the same movement/firing rules and document the behavior in [INVENTORY.md](../../INVENTORY.md).
3. **Reserve a mechanical accuracy field for later.** Add a `moving_fire_multiplier` field on weapon/unit definitions in Phase 3, fixed to `1.0` (no accuracy penalty) during the test phase so future mechanical tuning does not require another structural refactor.

## Consequences

- **Intentional behavior change in Phase 4:** Falling-back soldiers stop firing by default unless `fire_while_moving` is enabled. [MVP.md](../../MVP.md), fallback-firing tests (such as `fallback_can_fire_at_a_visible_enemy`), and outcome fingerprints must be updated with human approval.
- Toggling `fire_while_moving` on vs. off produces a clear, testable doctrine difference on the same scenario and seed.
- Adding mechanical moving-fire penalties later becomes a data tuning change rather than an architectural rewrite.
