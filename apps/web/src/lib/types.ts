export interface Point { x: number; y: number }
export interface Doctrine { engagementDistance: number; fallbackHp: number }
export interface UnitPlacement { id: number; faction: number; definitionId: string; position: Point; doctrine: Doctrine }
export interface ArenaMap { width: number; height: number; walls: Point[]; cover: Point[]; patrol: Point[] }
export interface Scenario { map: ArenaMap; seed: number; factions: number[]; units: UnitPlacement[]; timeLimit: number }
export type Behavior = 'Searching' | 'Investigating' | 'Advancing' | 'Engaging' | 'Falling back' | 'Defending' | 'Dead';
export interface UnitSnapshot { id: number; faction: number; position: Point; hp: number; behavior: Behavior; targetId: number | null; shotsFired: number }
export interface FactionResult { faction: number; survivors: number; casualties: number }
export interface BattleResult { winner: number | null; reason: 'elimination' | 'simultaneous_elimination' | 'timeout'; elapsed: number; factions: FactionResult[] }
export interface Snapshot { tick: number; elapsed: number; units: UnitSnapshot[]; result: BattleResult | null }
export interface ValidationIssue { message: string; unitId: number | null }
export interface Validation { valid: boolean; issues: ValidationIssue[] }
export interface Catalog { map: ArenaMap; unit: { id: string; name: string; maxHp: number; speed: number }; weapon: { id: string; range: number; damage: number; hitChance: number; cooldown: number }; defaultDoctrine: Doctrine; maxPerFaction: number; defaultTimeLimit: number; defaultSeed: number }
export type WorkerCommand =
  | { type: 'initialize' }
  | { type: 'validate'; requestId: number; scenario: Scenario; forStart: boolean }
  | { type: 'start'; battleId: number; scenario: Scenario }
  | { type: 'pause' | 'resume' | 'reset'; battleId: number };
export type WorkerReply =
  | { type: 'ready'; catalog: Catalog }
  | { type: 'validation'; requestId: number; validation: Validation }
  | { type: 'snapshot'; battleId: number; snapshot: Snapshot }
  | { type: 'state'; battleId: number; state: 'running' | 'paused' | 'setup' }
  | { type: 'result'; battleId: number; result: BattleResult }
  | { type: 'error'; battleId?: number; requestId?: number; message: string };
