# MVP

Status: first-playable scope with an initial Skirmish slice implemented. The slice does not yet fulfill every item in the broader MVP minimum.

## Purpose

Demonstrate that changing doctrine produces visible, understandable differences in autonomous combat. The first playable game is a small browser sandbox, not a campaign, puzzle collection, or large-scale war simulator.

## Implemented Skirmish slice

- Place up to eight infantry on each of two factions on a fixed 32 × 20 map with walls and cover. Each soldier uses the same rifle.
- Configure engagement distance from 2 to 12 and fallback health from 0 to 100 per soldier. Defaults are 8 and 30%. Doctrine is fixed during battle.
- Start with at least one soldier on each side; watch autonomous movement, targeting, fire, damage, and casualties. Pause, resume, or reset to the starting setup. Results report the winner or draw, reason, elapsed time, survivors, and casualties. The default time limit is 180 seconds.
- Each soldier sees enemies within 14 world units with clear line of sight and remembers a last sighting for approximately 10 seconds. Soldiers do not share sightings. The player sees the whole map.
- Low-health soldiers seek a safer reachable position when they know of a threat; they defend when no threat or safer position is available. Cover reduces incoming damage. Soldiers may still fire while falling back.
- A fixed seed supports repeatable comparisons in one environment. Reset restores the starting scenario including its seed; cross-platform deterministic replay is not established.

Fullscreen and health-indicator preferences last for the app session. Campaign and puzzles are unavailable; no saves or scenario exports are implemented. The current target is desktop Chromium.

## Broader first-playable scope

The committed minimum includes a handful of soldiers on two teams, a handful of weapons with different characteristics, a few configurable behavior rules, and autonomous combat that makes those rules observable. It must be reachable from a browser link without installation. The current slice provides one weapon and two doctrine controls; further weapon variety and usable doctrine presets remain part of the broader goal, not completed features. A simple map generator was proposed but the current map is fixed.

The central success test compares the same starting scenario and seed with different doctrine, and checks for an observable, understandable change in behavior. Native Rust simulation tests and browser integration tests exist; their presence alone does not establish acceptance of the full MVP.

## Outside the MVP

Campaign currency and progression, puzzles, vehicles, artillery, complex logistics, large-scale command hierarchies, terrain deformation, map importers, multiplayer, LLM controllers, live orders, general doctrine scripting, and advanced replay tools are deferred. Local saves and exportable scenarios remain architectural directions, but are not required to prove the core mechanic.

Related: [architecture](ARCHITECTURE.md), [scope](SCOPE.md), [scale](SCALE.md), [dream goals](DREAM_GOALS.md).
