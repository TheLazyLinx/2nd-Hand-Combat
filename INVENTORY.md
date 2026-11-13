# Soldier inventory v1

These rules describe the approved infantry inventory slice. Values are gameplay tuning, not standards for real equipment. Rust owns inventory, needs, decisions, action execution, and outcomes. All soldiers start with the same fixed loadout; there is no loadout editor.

| Item | Starting amount | Weight each | Volume each | Hands to use |
| --- | ---: | ---: | ---: | ---: |
| Rifle | 1 | 4 kg | 6 L | 2 |
| Rifle round | 60 | 0.012 kg | 0.015 L | Consumed by rifle |
| Grenade | 2 | 0.4 kg | 0.4 L | 1 |
| Entrenching tool | 1 | 1 kg | 1 L | 2 |
| Medical kit | 2 | 0.5 kg | 0.6 L | 2 |
| Ration | 2 | 0.5 kg | 0.8 L | 1 |
| Water | 2 | 0.5 kg | 0.5 L | 1 |

Capacity is 20 kg and 15 L. The starting load weighs 9.52 kg and occupies 6.5 L: the rifle occupies both hands as one item and contributes weight but no carried volume. All held items contribute weight. The entrenching tool has no action in v1.

Using an item atomically stows held equipment, uses the item, and restores equipment when capacity permits. The transition must respect capacity throughout and cannot leave a partial action. At most one action executes per soldier per fixed tick. Actions have no duration and do not interrupt movement. The priority is medical kit, water, ration, grenade, then rifle. Normal tactical positioning continues; grenade throws are opportunistic.

## Load and needs

Food and water start at 100. Food drains by 0.5 per second and water by 1 per second. At or below 50, a soldier automatically consumes the corresponding item when available, restoring 40 up to 100. Needs do not cause health damage.

The movement load multiplier is `1 - 0.25 * weight / 20 - 0.15 * volume / 15`. The needs multiplier is `1 - 0.15 * food_deficit - 0.25 * water_deficit`, where each deficit is `clamp((25 - meter) / 25, 0, 1)`. These multiply the fixed-step movement budget uniformly; they do not change route topology. The live movement readout represents the resulting movement effect, including terrain, load, and needs.

## Grenades and rifle

Grenade doctrine is per soldier: Never, Clusters (default), Any safe target, or Empty rifle only. Throw range is 2-6 units, default 6; cluster threshold is 2-8 visible enemies, default 2; friendly safety margin is 0-2 units, default 0.25. Range must exceed blast radius plus safety margin. A throw targets the nearest qualifying personally visible enemy, breaking ties by ID. Clusters count visible enemies reachable by the blast. A clear straight throw from eye height 1.6 is required; there is no trajectory arc.

A grenade detonates immediately with radius 1.5 and deals 40 damage. Terrain and solid obstacles block blast paths from 0.1 above the blast source to each victim's torso at height 1; vegetation does not block blast. All nearby victims can be hit, including unseen units. Any friendly unit, including the thrower, within radius plus safety margin prevents the throw regardless of intervening walls. Grenades and rifle share an attack cooldown: a grenade sets it to three seconds and a rifle shot to one second.

When rifle ammunition is empty, the soldier withdraws by default; Continue is the alternative doctrine. Stalemates resolve through the battle time limit.

## Medical doctrine

Medical policies are Never, Out of contact (default), and Low HP. The health threshold is 1-99, default 50; the safe interval is 0-30 seconds, default 5; cooldown is 1-60 seconds, default 10. Both active policies require health at or below the configured threshold. Out of contact also requires no visible enemies and the configured interval since contact or damage. Low HP ignores contact and the safe interval. A kit heals 25, capped at 100. Lethal damage wins over healing in the same tick; a committed kit is still spent.

## Persistence and boundaries

Scenario schema version 2 explicitly stores the new doctrine. Version 1 documents normalize in memory using defaults from the Rust catalog; loading does not automatically rewrite records or bump the IndexedDB schema. Save, rename, duplicate, and export produce version 2. Old documents do not persist equipment and receive the current default loadout. Reset restores the full starting battle state.

Inventory state and needs belong to units, separate from perception. Decisions select intentions; execution applies inventory, movement, and combat effects. Catalog and snapshots cross the single Worker/WASM boundary in bulk. Calibers, magazines, timed actions, custom loadouts, resupply, loot, and tool actions are deferred.
