# Scale

Status: small initial scope, long-term scaling direction, and proposed measurements. No capacity has been demonstrated.

Source: Game Architecture Options planning discussion, 30 October 2026.

## Initial scale

The committed MVP scope is a handful of soldiers on each side, a few weapons, and a small battlefield.

The conversation proposed two teams of approximately 4–8 soldiers each and two or three weapons as a concrete starting point. These are planning examples, not fixed limits or measured capacity.

The first scale requirement is that the browser remains usable while a small autonomous battle runs.

## Long-term ambition

Grow from small infantry engagements toward division-scale warfare.

This is a direction for future development, not a promise that the first implementation will simulate a division at full individual fidelity. The conversation does not establish a final entity count, map area, hardware requirement, or fidelity standard.

Future scale may require different levels of processing for individuals, groups, and larger formations. Aggregation and simulation level of detail remain design work.

## Architectural provisions

- Keep simulation independent of rendering and interface updates.
- Use compact, data-oriented entity storage and spatial queries.
- Separate entity identity from how often its AI is processed.
- Keep movement/combat updates separate from more expensive perception and decisions.
- Send only the state needed for display to the renderer.
- Support native headless runs of the same Rust simulation.
- Keep rendering detail separate from simulation fidelity. Camera zoom should not determine the battle outcome.

Rust/WebAssembly is the chosen foundation, not evidence that any particular scale has been achieved.

## Proposed performance starting points

| Operation | Suggested starting rate |
| --- | --- |
| Rendering | Up to 60 frames per second |
| Movement and combat | 30 updates per second |
| Target acquisition and local decisions | 5–10 updates per second |
| Strategic and formation decisions | Deferred |

These numbers came from the architecture discussion and require profiling. They are neither release guarantees nor measured results.

## Proposed measurements

Use headless benchmarks at 100, 1,000, and 5,000 simple entities to expose scaling behavior. These are test sizes, not claims of playable battle capacity.

Measure movement, perception, decisions, and combat separately. Check the browser integration as well as native simulation performance, including the cost of snapshot exchange and rendering.

Evaluate on a desktop and an actual mobile device before making broad performance claims. No supported-device baseline or acceptance threshold has been agreed.

## Deferred techniques and open questions

Spatial partitioning, less frequent decisions, aggregation, and simulation level of detail may be needed as battles grow. More specialized sprite batching, shared memory, or additional threads should follow measurements and concrete requirements.

Open questions include how aggregate units preserve identity and outcomes, how fidelity changes affect doctrine, and what scale remains playable on the intended devices.

There is no agreed multiplayer concurrency target, server capacity target, or backend scaling plan.

Related: [architecture](ARCHITECTURE.md), [MVP](MVP.md), [dream goals](DREAM_GOALS.md).
