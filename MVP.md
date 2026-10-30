# MVP

Status: first playable scope. Exact counts and controls remain adjustable.

Source: [Game Architecture Options](https://chatgpt.com/c/6ae4ac8c-6904-83eb-a33e-2944fd754973), 30 October 2026.

## Purpose

Demonstrate that configuring a few behavior rules changes the way autonomous soldiers fight.

The MVP is a small combat sandbox accessible from a browser link. It is not the full campaign, puzzle collection, or large-scale warfare simulation.

## Committed minimum

- A handful of soldiers on two opposing teams.
- A handful of weapons with different characteristics.
- A few configurable behavior rules.
- Autonomous combat that makes those rules observable.
- A design that permits later customization without requiring those future systems now.
- Browser access without installing or manually launching software.
- The Rust/WebAssembly simulation and TypeScript presentation split described in [ARCHITECTURE.md](ARCHITECTURE.md).

## Proposed playable baseline

The discussion refined that minimum into the following first implementation:

1. Open the game and enter Sandbox from a simple Play/Settings shell.
2. Load a small flat battlefield with obstacles, cover, and team starting positions.
3. Choose doctrine presets and adjust a few parameters.
4. Press Play and watch units move, select targets, fire, and take casualties.
5. End the battle and show the winner and losses.
6. Restart the same starting scenario with changed doctrine to compare behavior.

Two teams of approximately 4–8 soldiers each and two or three weapons were suggested. Final counts, weapon selection, map dimensions, and victory conditions are not fixed.

A simple procedural arena is the proposed initial map source. A full map editor or importer is unnecessary.

Doctrine remains fixed during the battle in this baseline. Live orders and direct control are deferred.

## Doctrine controls

Start with presets and a small set of meaningful parameters. Examples discussed include preferred engagement distance, cover priority, and withdrawal conditions.

Those examples do not fix the final controls or numeric ranges. Behavioral triggers are part of the intended design, but a general conditional-rule editor is not required for the first playable version.

A small priority-based decision system is the proposed starting implementation.

## Success test

Run the same starting battle with different doctrine settings while keeping troops, weapons, map, and seed comparable.

An aggressive configuration and a cautious configuration should produce visibly different, understandable behavior. This is the core acceptance test; a larger unit count does not substitute for it.

Seeded scenarios should support restarting and investigation. Exact deterministic replay across platforms is not assumed.

## Initial deliverables

- A headless Rust battle simulation with basic movement, target selection, weapons, damage, and doctrine evaluation.
- A browser build with Worker/WASM integration and a 2D battlefield view.
- A minimal setup interface, doctrine controls, battle start, and results.
- A repeatable scenario suitable for comparing behavior settings.
- Native simulation checks and browser integration verification.

This list is a planning baseline, not a claim that any component exists.

## Outside the MVP

- Campaign currency, persistent rosters, progression, and unlocks.
- Puzzle mode and authored puzzle content.
- Vehicles, aircraft, artillery, nuclear effects, and complex logistics.
- Formation hierarchies and division-scale battles.
- Elevation, terrain deformation, and persistent terrain changes.
- Custom-map tools, geographical services, and image conversion.
- LLM integration, online services, and multiplayer.
- Live doctrine editing, RTS orders, and direct unit control.
- A full behavior-tree editor, scripting language, or modding framework.
- Advanced replay recording and timeline seeking.

Local saves and exportable scenarios are part of the architecture direction; a campaign save system is not a prerequisite for proving the MVP.

Related: [scope](SCOPE.md), [scale](SCALE.md), [dream goals](DREAM_GOALS.md).
