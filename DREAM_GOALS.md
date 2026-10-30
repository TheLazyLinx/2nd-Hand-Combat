# Dream Goals

Status: future ambitions and possibilities. Nothing in this document is an MVP requirement, implementation claim, or delivery promise.

Source: [Game Architecture Options](https://chatgpt.com/c/6ae4ac8c-6904-83eb-a33e-2944fd754973), 30 October 2026. Some ambitions are only mentioned briefly in that conversation; their mechanics remain unspecified.

## Intended expansion

### Campaign

Earn currency through play and spend it on soldiers and weapons. Build progression around the same battles used by Sandbox.

The economy, roster persistence, unlock rules, campaign structure, and consequences of losses are not settled.

### Puzzles

Provide constrained scenarios in which doctrine is the main means of solving a combat problem.

Unit limits, equipment restrictions, scoring, and manual-control rules remain to be designed.

### Richer doctrine

Expand presets and parameters with behavioral triggers, conditional rules, and configuration by unit or unit type.

Group overrides, faction defaults, inheritance, command hierarchies, and formation decisions were discussed as extensions. The exact hierarchy, conflict resolution, and editor are unresolved.

### Larger battles

Progress from small infantry encounters toward division-scale warfare, potentially using aggregation and simulation level of detail.

Entity counts, map sizes, fidelity, device requirements, and acceptable performance remain undefined. See [SCALE.md](SCALE.md).

## Possible extensions

| Possibility | Direction discussed | Unresolved work |
| --- | --- | --- |
| Custom maps | Create or load manually designed battlefields | Editing tools, format, and validation |
| Real-world maps | Convert geographical map or elevation data into game terrain | Providers, conversion, and useful simulation detail |
| Maps from images | Derive terrain from map images or aerial photographs | Recognition and interpretation; not just OCR |
| Manual orders and direct control | Allow another controller to issue intentions to units | Input model, mode restrictions, and effect on doctrine gameplay |
| Persistent and deformable terrain | Represent terrain changes and later elevation/material effects | Persistence rules, navigation/visibility updates, and simulation cost |
| Vehicles, aircraft, and artillery | Extend beyond the first infantry weapons | Entity composition, movement, damage, and crew mechanics |
| Different historical eras | Broaden the setting and equipment | No eras or content plan specified |
| Nuclear warfare | Add large spatial and persistent effects | No effects model or gameplay specification agreed |
| LLM integration | Experiment with doctrine design or high-level commanders | Interfaces, permitted decisions, service integration, and evaluation |
| Automated doctrine experiments | Run headless battles to compare behaviors | Metrics, experiment setup, and reproducibility |
| Replays | Record and inspect battles beyond simple restarts | Determinism, external actions, snapshots, and seeking |

Optional backend services and multiplayer were mentioned as future possibilities. Neither has an agreed feature set or architecture.

## Boundary for current work

Preserve clear interfaces and data definitions where they serve the small game now. Do not implement these extensions solely to demonstrate that they might be possible.

Future work should retain the core identity: the player influences combat through behavior configuration. Whether direct control becomes an additional mode remains open.

No priority order or delivery dates were agreed for these goals.

Related: [architecture](ARCHITECTURE.md), [scope](SCOPE.md), [MVP](MVP.md).
