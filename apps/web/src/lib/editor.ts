/**
 * Pure scenario-document editing utilities for MapMaker.
 * Owns session types, cloning/fingerprinting, map resizing, brush painting,
 * hit-testing, and bounds-checked entity mutations with zero Svelte/DOM/Worker imports.
 */
import type { ArenaMap, Catalog, Point, Scenario, ScenarioDocument, Surface } from './types';

export type EditorTool =
  'surface' | 'raise' | 'lower' | 'set' | 'smooth' | 'obstacle' | 'select' | 'patrol' | 'soldier';

export type EditorSelection = { kind: 'unit' | 'obstacle' | 'patrol'; index: number };

export interface EditorSettings {
  surface: Surface;
  amount: number;
  setValue: number;
  obstacleHeight: number;
  faction: number;
  brushRadius: number;
  terrainOverlay: boolean;
}

export interface EditorSession {
  document: ScenarioDocument;
  savedId: string | null;
  savedSnapshot: string | null;
  undo: ScenarioDocument[];
  redo: ScenarioDocument[];
  tool: EditorTool;
  selection: EditorSelection | null;
  camera: { x: number; y: number; zoom: number } | null;
  settings?: EditorSettings;
}

export const BRUSH_TOOLS: readonly EditorTool[] = ['surface', 'raise', 'lower', 'set', 'smooth'];

export function isBrushTool(tool: EditorTool): boolean {
  return BRUSH_TOOLS.includes(tool);
}
export const copy = <T>(value: T): T => structuredClone(value);
export const fingerprint = (value: ScenarioDocument) => JSON.stringify(value);
export function createSession(
  document: ScenarioDocument,
  savedId: string | null = null,
): EditorSession {
  return {
    document: copy(document),
    savedId,
    savedSnapshot: savedId ? fingerprint(document) : null,
    undo: [],
    redo: [],
    tool: 'surface',
    selection: null,
    camera: null,
  };
}
export function blankDocument(catalog: Catalog, width = 32, height = 20): ScenarioDocument {
  return {
    format: '2nd-hand-combat-scenario',
    schemaVersion: 3,
    name: 'Untitled scenario',
    description: '',
    scenario: {
      map: {
        width,
        height,
        surfaces: Array(width * height).fill('open') as Surface[],
        elevations: Array((width + 1) * (height + 1)).fill(0) as number[],
        obstacles: [],
        patrol: [],
      },
      seed: catalog.defaultSeed,
      factions: copy(catalog.defaultFactions),
      units: [],
      loadouts: [],
      timeLimit: catalog.defaultTimeLimit,
    },
  };
}
export function resizeMap(map: ArenaMap, width: number, height: number): ArenaMap {
  const surfaces: Surface[] = [],
    elevations: number[] = [];
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++)
      surfaces.push(x < map.width && y < map.height ? map.surfaces[y * map.width + x] : 'open');
  for (let y = 0; y <= height; y++)
    for (let x = 0; x <= width; x++)
      elevations.push(
        map.elevations[Math.min(y, map.height) * (map.width + 1) + Math.min(x, map.width)],
      );
  return {
    width,
    height,
    surfaces,
    elevations,
    obstacles: map.obstacles
      .filter((o) => o.x < width && o.y < height)
      .map((o) => ({
        ...o,
        width: Math.min(o.width, width - o.x),
        height: Math.min(o.height, height - o.y),
      })),
    patrol: map.patrol.filter((p) => p.x >= 0 && p.y >= 0 && p.x < width && p.y < height),
  };
}
export function resizeScenario(scenario: Scenario, width: number, height: number): Scenario {
  return {
    ...scenario,
    map: resizeMap(scenario.map, width, height),
    units: scenario.units.filter(
      (u) =>
        u.position.x >= 0 && u.position.y >= 0 && u.position.x < width && u.position.y < height,
    ),
  };
}
export function paint(
  map: ArenaMap,
  tool: EditorTool,
  point: Point,
  surface: Surface,
  amount: number,
  setValue: number,
  minElevation: number,
  maxElevation: number,
) {
  const x = Math.max(0, Math.min(map.width - 1, Math.floor(point.x))),
    y = Math.max(0, Math.min(map.height - 1, Math.floor(point.y)));
  if (tool === 'surface') {
    map.surfaces[y * map.width + x] = surface;
    return;
  }
  const vx = Math.max(0, Math.min(map.width, Math.round(point.x))),
    vy = Math.max(0, Math.min(map.height, Math.round(point.y))),
    index = vy * (map.width + 1) + vx;
  let value = map.elevations[index];
  if (tool === 'raise') value += amount;
  if (tool === 'lower') value -= amount;
  if (tool === 'set') value = setValue;
  if (tool === 'smooth') {
    const neighbors: number[] = [];
    for (const [dx, dy] of [
      [-1, 0],
      [1, 0],
      [0, -1],
      [0, 1],
    ]) {
      const nx = vx + dx,
        ny = vy + dy;
      if (nx >= 0 && nx <= map.width && ny >= 0 && ny <= map.height)
        neighbors.push(map.elevations[ny * (map.width + 1) + nx]);
    }
    if (neighbors.length) value = neighbors.reduce((a, b) => a + b, 0) / neighbors.length;
  }
  map.elevations[index] = Math.max(minElevation, Math.min(maxElevation, value));
}

export function applyBrushStroke(
  map: ArenaMap,
  tool: EditorTool,
  point: Point,
  settings: Pick<EditorSettings, 'surface' | 'amount' | 'setValue' | 'brushRadius'>,
  catalog: Catalog,
  visited: Set<string>,
): boolean {
  const { surface, amount, setValue, brushRadius } = settings;
  if (
    !Number.isInteger(brushRadius) ||
    brushRadius < 0 ||
    brushRadius > 5 ||
    !Number.isFinite(amount) ||
    amount <= 0 ||
    !Number.isFinite(setValue)
  ) {
    return false;
  }
  for (let dy = -brushRadius; dy <= brushRadius; dy++) {
    for (let dx = -brushRadius; dx <= brushRadius; dx++) {
      const p = { x: point.x + dx, y: point.y + dy };
      if (p.x < 0 || p.y < 0 || p.x >= map.width || p.y >= map.height) continue;
      const key =
        tool === 'surface'
          ? `${Math.floor(p.x)},${Math.floor(p.y)}`
          : `${Math.round(p.x)},${Math.round(p.y)}`;
      if (visited.has(key)) continue;
      visited.add(key);
      paint(
        map,
        tool,
        p,
        surface,
        amount,
        setValue,
        catalog.editorRules.minElevation,
        catalog.editorRules.maxElevation,
      );
    }
  }
  return true;
}

export function pickSelection(scenario: Scenario, point: Point): EditorSelection | null {
  const unit = scenario.units.find(
    (u) => Math.hypot(u.position.x - point.x, u.position.y - point.y) < 0.7,
  );
  if (unit) return { kind: 'unit', index: unit.id };
  const patrol = scenario.map.patrol.findIndex(
    (p) => Math.hypot(p.x - point.x, p.y - point.y) < 0.5,
  );
  if (patrol >= 0) return { kind: 'patrol', index: patrol };
  const obstacle = scenario.map.obstacles.findIndex(
    (o) => point.x >= o.x && point.x < o.x + o.width && point.y >= o.y && point.y < o.y + o.height,
  );
  return obstacle >= 0 ? { kind: 'obstacle', index: obstacle } : null;
}

export function placeSoldier(
  scenario: Scenario,
  faction: number,
  point: Point,
  catalog: Catalog,
): number | null {
  if (
    !scenario.factions.some((f) => f.id === faction) ||
    scenario.units.filter((u) => u.faction === faction).length >= catalog.maxPerFaction
  ) {
    return null;
  }
  const ids = new Set(scenario.units.map((u) => u.id));
  let id = 1;
  while (ids.has(id)) id++;
  scenario.units.push({
    id,
    faction,
    definitionId: catalog.units[0].id,
    position: { x: Math.floor(point.x) + 0.5, y: Math.floor(point.y) + 0.5 },
    doctrine: copy(catalog.defaultDoctrine),
  });
  return id;
}

export function placeObstacle(
  scenario: Scenario,
  start: Point,
  end: Point,
  heightMetres: number,
): number {
  const x1 = Math.floor(Math.min(start.x, end.x));
  const y1 = Math.floor(Math.min(start.y, end.y));
  const x2 = Math.floor(Math.max(start.x, end.x));
  const y2 = Math.floor(Math.max(start.y, end.y));
  scenario.map.obstacles.push({
    x: x1,
    y: y1,
    width: x2 - x1 + 1,
    height: y2 - y1 + 1,
    heightMetres,
  });
  return scenario.map.obstacles.length - 1;
}

export function dragSelection(scenario: Scenario, selection: EditorSelection, point: Point): void {
  const snapped = { x: Math.floor(point.x) + 0.5, y: Math.floor(point.y) + 0.5 };
  if (selection.kind === 'unit') {
    const unit = scenario.units.find((u) => u.id === selection.index);
    if (unit) unit.position = snapped;
  } else if (selection.kind === 'patrol') {
    scenario.map.patrol[selection.index] = snapped;
  } else {
    const obstacle = scenario.map.obstacles[selection.index];
    if (obstacle) {
      obstacle.x = Math.max(0, Math.min(scenario.map.width - obstacle.width, Math.floor(point.x)));
      obstacle.y = Math.max(
        0,
        Math.min(scenario.map.height - obstacle.height, Math.floor(point.y)),
      );
    }
  }
}

export function moveSelectedAxis(
  scenario: Scenario,
  selection: EditorSelection,
  axis: 'x' | 'y',
  value: number,
): boolean {
  if (!Number.isFinite(value)) return false;
  const limit = axis === 'x' ? scenario.map.width : scenario.map.height;
  if (value < 0 || value >= limit) return false;
  if (selection.kind === 'unit') {
    const unit = scenario.units.find((u) => u.id === selection.index);
    if (!unit) return false;
    unit.position[axis] = value;
    return true;
  }
  if (selection.kind === 'patrol') {
    const point = scenario.map.patrol[selection.index];
    if (!point) return false;
    point[axis] = value;
    return true;
  }
  const obstacle = scenario.map.obstacles[selection.index];
  if (!obstacle) return false;
  const size = axis === 'x' ? obstacle.width : obstacle.height;
  if (!Number.isInteger(value) || value + size > limit) return false;
  obstacle[axis] = value;
  return true;
}

export function resizeObstacleAxis(
  scenario: Scenario,
  index: number,
  axis: 'width' | 'height',
  value: number,
): boolean {
  const obstacle = scenario.map.obstacles[index];
  if (!obstacle || !Number.isInteger(value) || value < 1) return false;
  const limit =
    axis === 'width' ? scenario.map.width - obstacle.x : scenario.map.height - obstacle.y;
  if (value > limit) return false;
  obstacle[axis] = value;
  return true;
}
