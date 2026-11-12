export interface Point { x: number; y: number }
export interface Doctrine { engagementDistance: number; fallbackHp: number; positionPreference: 'balanced' | 'cover' | 'concealment'; routePreference: 'fastest' | 'protected' }
export interface UnitPlacement { id: number; faction: number; definitionId: string; position: Point; doctrine: Doctrine }
export type Surface = 'open' | 'road' | 'mud' | 'brush' | 'woodland';
export interface ArenaMap { width: number; height: number; surfaces: Surface[]; elevations: number[]; obstacles: { x: number; y: number; width: number; height: number; heightMetres: number }[]; patrol: Point[] }
export interface Scenario { map: ArenaMap; seed: number; factions: number[]; units: UnitPlacement[]; timeLimit: number }
export interface ScenarioDocument { format: '2nd-hand-combat-scenario'; schemaVersion: 1; name: string; description: string; scenario: Scenario }
export interface TerrainCellInfo { surface: Surface; elevation: number; movementMultiplier: number; traversable: boolean }
export interface AnalysisIssue { severity: 'error' | 'warning'; message: string; unitId: number | null; position: Point | null }
export interface ScenarioAnalysis { structuralValid: boolean; canStart: boolean; issues: AnalysisIssue[]; cells: TerrainCellInfo[]; steepEdges: Point[] }
export type Behavior = 'Searching' | 'Investigating' | 'Advancing' | 'Engaging' | 'Falling back' | 'Defending' | 'Dead';
export interface UnitSnapshot { id: number; faction: number; position: Point; hp: number; behavior: Behavior; targetId: number | null; shotsFired: number; terrain: { surface: Surface; elevation: number; movementMultiplier: number; revealed: boolean; protection: number | null } }
export interface FactionResult { faction: number; survivors: number; casualties: number }
export interface BattleResult { winner: number | null; reason: 'elimination' | 'simultaneous_elimination' | 'timeout'; elapsed: number; factions: FactionResult[] }
export interface Snapshot { tick: number; elapsed: number; units: UnitSnapshot[]; result: BattleResult | null }
export interface ValidationIssue { message: string; unitId: number | null }
export interface Validation { valid: boolean; issues: ValidationIssue[] }
export interface Catalog { editorRules: { minDimension: number; maxDimension: number; minElevation: number; maxElevation: number; maxObstacleHeight: number }; map: ArenaMap; maps: { id: string; name: string; description: string; map: ArenaMap }[]; surfaces: { id: Surface; name: string; movementMultiplier: number; spottingMultiplier: number; vegetationHeight: number; color: number }[]; lowObstacleHeight: number; fullObstacleHeight: number; terrainCells: { surface: Surface; elevation: number; movementMultiplier: number; traversable: boolean }[][]; unit: { id: string; name: string; maxHp: number; speed: number }; weapon: { id: string; range: number; damage: number; hitChance: number; cooldown: number }; defaultDoctrine: Doctrine; maxPerFaction: number; defaultTimeLimit: number; defaultSeed: number }
export type WorkerCommand =
  | { type: 'initialize' }
  | { type: 'analyze'; requestId: number; revision: number; scenario: Scenario }
  | { type: 'validate'; requestId: number; scenario: Scenario; forStart: boolean }
  | { type: 'start'; battleId: number; scenario: Scenario }
  | { type: 'pause' | 'resume' | 'reset'; battleId: number };
export type WorkerReply =
  | { type: 'ready'; catalog: Catalog }
  | { type: 'analysis'; requestId: number; revision: number; analysis: ScenarioAnalysis }
  | { type: 'validation'; requestId: number; validation: Validation }
  | { type: 'snapshot'; battleId: number; snapshot: Snapshot }
  | { type: 'state'; battleId: number; state: 'running' | 'paused' | 'setup' }
  | { type: 'result'; battleId: number; result: BattleResult }
  | { type: 'error'; battleId?: number; requestId?: number; message: string };
