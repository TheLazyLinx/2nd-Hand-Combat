import type { ArenaMap, Catalog, Point, Scenario, ScenarioDocument, Surface } from './types';

export type EditorTool = 'surface' | 'raise' | 'lower' | 'set' | 'smooth' | 'obstacle' | 'select' | 'patrol' | 'soldier';
export interface EditorSession {
  document: ScenarioDocument;
  savedId: string | null;
  savedSnapshot: string | null;
  undo: ScenarioDocument[];
  redo: ScenarioDocument[];
  tool: EditorTool;
  selection: { kind: 'unit' | 'obstacle' | 'patrol'; index: number } | null;
  camera: { x: number; y: number; zoom: number } | null;
  settings?: { surface: Surface; amount: number; setValue: number; obstacleHeight: number; faction: number; brushRadius: number; terrainOverlay: boolean };
}
export const copy = <T>(value: T): T => structuredClone(value);
export const fingerprint = (value: ScenarioDocument) => JSON.stringify(value);
export function createSession(document: ScenarioDocument, savedId: string | null = null): EditorSession {
  return { document: copy(document), savedId, savedSnapshot: savedId ? fingerprint(document) : null, undo: [], redo: [], tool: 'surface', selection: null, camera: null };
}
export function blankDocument(catalog: Catalog, width = 32, height = 20): ScenarioDocument {
  return { format: '2nd-hand-combat-scenario', schemaVersion: 2, name: 'Untitled scenario', description: '', scenario: { map: {width, height, surfaces: Array(width * height).fill('open') as Surface[], elevations: Array((width + 1) * (height + 1)).fill(0) as number[], obstacles: [], patrol: []}, seed: catalog.defaultSeed, factions: [0, 1], units: [], timeLimit: catalog.defaultTimeLimit } };
}
export function resizeMap(map: ArenaMap, width: number, height: number): ArenaMap {
  const surfaces: Surface[] = [], elevations: number[] = [];
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) surfaces.push(x < map.width && y < map.height ? map.surfaces[y * map.width + x] : 'open');
  for (let y = 0; y <= height; y++) for (let x = 0; x <= width; x++) elevations.push(map.elevations[Math.min(y, map.height) * (map.width + 1) + Math.min(x, map.width)]);
  return {width, height, surfaces, elevations, obstacles: map.obstacles.filter(o => o.x < width && o.y < height).map(o => ({...o, width: Math.min(o.width, width - o.x), height: Math.min(o.height, height - o.y)})), patrol: map.patrol.filter(p => p.x >= 0 && p.y >= 0 && p.x < width && p.y < height)};
}
export function resizeScenario(scenario: Scenario, width: number, height: number): Scenario {
  return {...scenario, map: resizeMap(scenario.map, width, height), units: scenario.units.filter(u => u.position.x >= 0 && u.position.y >= 0 && u.position.x < width && u.position.y < height)};
}
export function paint(map: ArenaMap, tool: EditorTool, point: Point, surface: Surface, amount: number, setValue: number, minElevation: number, maxElevation: number) {
  const x = Math.max(0, Math.min(map.width - 1, Math.floor(point.x))), y = Math.max(0, Math.min(map.height - 1, Math.floor(point.y)));
  if (tool === 'surface') { map.surfaces[y * map.width + x] = surface; return; }
  const vx = Math.max(0, Math.min(map.width, Math.round(point.x))), vy = Math.max(0, Math.min(map.height, Math.round(point.y))), index = vy * (map.width + 1) + vx;
  let value = map.elevations[index];
  if (tool === 'raise') value += amount;
  if (tool === 'lower') value -= amount;
  if (tool === 'set') value = setValue;
  if (tool === 'smooth') {
    const neighbors: number[] = [];
    for (const [dx, dy] of [[-1,0],[1,0],[0,-1],[0,1]]) { const nx = vx + dx, ny = vy + dy; if (nx >= 0 && nx <= map.width && ny >= 0 && ny <= map.height) neighbors.push(map.elevations[ny * (map.width + 1) + nx]); }
    if (neighbors.length) value = neighbors.reduce((a, b) => a + b, 0) / neighbors.length;
  }
  map.elevations[index] = Math.max(minElevation, Math.min(maxElevation, value));
}
