# Map Maker

Map Maker authors complete Skirmish scenarios in the browser. It uses the same map, soldier, doctrine, seed, and time-limit definitions as Skirmish. Combat rules remain two factions, up to eight infantry per faction, and the existing rifle.

## Authoring and testing

Start with a blank map, copy a built-in map, or open a document from the browser library. Paint surfaces, sculpt vertex elevations, place rectangular obstacles, order patrol points, and place soldiers with individual doctrine. A blank map has open ground, zero elevation, and no patrol points or soldiers. Soldiers without patrol points react to visible enemies but do not patrol while searching.

Use Fit Map, zoom, and pan to navigate maps from 1 to 128 cells in either dimension. Coordinates refer to world cells; rendering and camera transforms never change the scenario. Editing gestures are undoable. Selecting an object exposes its properties; numeric coordinates provide a keyboard-accessible alternative to canvas placement.

Drafts may contain editable layout errors, such as a soldier standing inside a newly painted obstacle. Rust analysis locates errors and reports warnings for cliffs, disconnected regions, and missing patrol points. Structurally malformed data is rejected before terrain queries or rendering. Save and Export allow structurally safe drafts; Test battle requires a valid starting scenario.

Test battle opens a detached copy in Skirmish setup. Start runs the normal simulation. Return to MapMaker restores the editor document, history, tool, selection, and camera. Changes made in Skirmish do not change the editor document. Battle reset restores the exact starting scenario rather than reloading a potentially changed library record.

## Resizing

Resizing is anchored at the northwest corner. Retained objects keep their world coordinates. Overlapping surface cells and vertices retain their values; new surface cells are open ground and new vertices extend the nearest old boundary elevation.

Before shrinking, review the cropped area and the soldiers, patrol points, and obstacles affected. Apply resize removes out-of-bounds soldiers and patrol points, clips crossing obstacles, and removes obstacles entirely outside the map. Cancel leaves the document unchanged. The entire resize is one undo action, including deleted objects and terrain. Seed, time limit, unit IDs, and retained doctrine remain unchanged. Rust revalidates the resulting layout and boundary clearance.

## Library and portable files

The library uses IndexedDB and belongs to this site origin and browser profile. It has no backend or account synchronization. Save updates the current record only after the transaction commits. Save As and Duplicate create independent records. Leaving an unsaved draft offers Save, Discard, and Cancel. Export is also available when browser storage fails.

Portable JSON documents have this envelope:

```json
{
  "format": "2nd-hand-combat-scenario",
  "schemaVersion": 2,
  "name": "My scenario",
  "description": "Optional description",
  "scenario": {
    "map": {
      "width": 2, "height": 2,
      "surfaces": ["open", "open", "open", "open"],
      "elevations": [0, 0, 0, 0, 0, 0, 0, 0, 0],
      "obstacles": [], "patrol": []
    },
    "seed": 1,
    "factions": [0, 1],
    "units": [],
    "timeLimit": 180
  }
}
```

This minimal draft has no soldiers and cannot start a battle. The map contains row-major surface cells and vertex elevations, obstacles, and ordered patrol points. Vertex rows have `width + 1` entries. Units contain stable IDs, faction, definition, position, and doctrine, including inventory-related rules. See [TERRAIN.md](TERRAIN.md) for terrain semantics. Export includes unfinished but structurally safe scenarios. Imports create new local records, never silently overwrite an existing one, and pass runtime format checks and Rust structural analysis before use. Unsupported versions and malformed files produce visible errors. Files are limited to 8 MiB. If storing an import fails, it remains open as an unsaved draft for export or testing.

Version 1 documents are normalized in memory using current Rust catalog doctrine defaults. Loading does not rewrite them or change the IndexedDB schema; save, rename, duplicate, and export produce version 2. Equipment is not persisted in old documents; soldiers receive the current fixed default loadout. See [INVENTORY.md](INVENTORY.md).

## Implementation boundaries

TypeScript owns editable documents, input gestures, undo/redo, storage, and navigation. Rust owns supported limits, terrain queries, placement rules, connectivity analysis, and readiness to start a battle. Analysis uses bulk Worker messages with request IDs and editor revisions; stale replies cannot replace newer edits. Each active screen owns one engine Worker and disposes it on departure.

Skirmish lists built-in maps and saved scenarios. Built-ins start an empty default setup; saved scenarios load the complete authored setup. Loading a different setup is available only before battle and replaces the current map, soldiers, doctrine, seed, and time limit.

Maximum dimensions describe supported authoring, not a capacity guarantee for arbitrary terrain. Performance checks exercise representative 128 × 128 maps with sixteen soldiers and protected routing. Additional simulation Workers, shared memory, procedural generation, new victory rules, and remote sharing are outside this feature.

The browser checks require at least 16 simulation ticks per second against the normal 20 Hz rate, frame-gap p95 below 100 ms, and no sustained gaps of 750 ms or more. Full-map searches for nearby destinations and dense slope-warning overdraw were profiled and corrected before these checks passed. Targeted searches preserve weighted shortest paths and unrestricted detours; display aggregation changes only warning presentation.
