# Scope

Status: agreed product direction with a deliberately smaller first release.

Source: [Game Architecture Options](https://chatgpt.com/c/6ae4ac8c-6904-83eb-a33e-2944fd754973), 30 October 2026. This document separates the game's identity from features that remain outside the MVP.

## Core identity

2nd Hand Combat is a browser-based strategy and combat simulation game with 2D, sprite-based visuals viewed from above or a similar perspective.

The player primarily configures how units behave, then watches autonomous combat. Doctrine means the rules and parameters that guide that behavior: when to fire, when to move or withdraw, and how to use cover.

The intended presentation resembles a straightforward Newgrounds-style browser game: open a link, reach a splash screen with Play and Settings, and enter the game.

The central gameplay test is whether changing doctrine produces visible, understandable changes in unit behavior.

## Product direction

- Provide usable doctrine presets and simple adjustments instead of requiring players to program behavior from scratch.
- Support configuration by unit or unit type as the product develops.
- Allow parameters and behavioral triggers to express decisions such as holding position, holding fire, seeking cover, or withdrawing.
- Keep weapons and behavior definitions driven by data.
- Keep decisions separate from movement and combat execution so future control methods can be added.
- Allow customization later without implementing a general customization framework in the MVP.

The exact sliders, trigger conditions, thresholds, and configuration hierarchy are not settled.

## Intended modes

| Mode | Intended role | Release status |
| --- | --- | --- |
| Sandbox | Configure a battle and experiment with doctrine | First playable mode |
| Puzzles | Solve constrained combat scenarios through behavior configuration | Future |
| Campaign | Earn currency and spend it on soldiers and weapons across battles | Future |

All modes should use the same combat simulation. Restrictions, scenario construction, and progression distinguish them.

Random maps are part of the intended direction. The initial generator can be small and simple. Custom maps, real-world map services, and image-derived maps are separate future possibilities.

## Player control

Autonomous behavior is the core mechanic. The first playable version uses doctrine configured before the battle.

Live orders or direct unit control remain possibilities. Preserve a boundary through which another controller could supply intentions, but do not implement those controls for the MVP. Their eventual availability and restrictions by mode are unresolved.

## Scope boundary

The MVP is a handful of soldiers, a handful of weapons, simple terrain, and a few behavior rules. See [MVP.md](MVP.md) for the first playable baseline.

Campaign progression, puzzle content, advanced doctrine editors, command hierarchies, vehicles, artillery, complex logistics, terrain deformation, nuclear effects, and LLM controllers are outside that baseline.

Large battles are a long-term ambition, not a requirement to build every underlying system before demonstrating the core mechanic.

## Unresolved product questions

- Exact initial weapons, doctrine controls, and victory rules.
- How much individual configuration is necessary in the first release.
- Whether later modes permit live orders or direct control.
- Final campaign economy, puzzle structure, and content progression.
- Supported devices and the minimum acceptable browser experience.

The existing README records an intended later open-source or possibly Creative Commons release. No license has been selected or granted; these planning documents do not change that status.

Related: [architecture](ARCHITECTURE.md), [scale](SCALE.md), [dream goals](DREAM_GOALS.md).
