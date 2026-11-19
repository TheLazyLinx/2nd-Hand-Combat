// Simulation wire types are generated from crates/sim-core into ../generated/sim-types.ts.
// Keep only browser/Worker protocol and portable document wrappers here.
import type {
  BattleResult,
  Catalog,
  Scenario,
  ScenarioAnalysis,
  Snapshot,
  UnitInventoryUpdate,
  Validation,
} from '../generated/sim-types';

export type * from '../generated/sim-types';

export interface ScenarioDocument {
  format: '2nd-hand-combat-scenario';
  schemaVersion: 3;
  name: string;
  description: string;
  scenario: Scenario;
}

export type WorkerCommand =
  | { type: 'initialize' }
  | { type: 'analyze'; requestId: number; revision: number; scenario: Scenario }
  | { type: 'validate'; requestId: number; scenario: Scenario; forStart: boolean }
  | { type: 'start'; battleId: number; scenario: Scenario; compact?: boolean }
  | { type: 'inspect'; battleId: number; unitId: number | null }
  | { type: 'pause' | 'resume' | 'stop' | 'reset'; battleId: number };

export type WorkerReply =
  | { type: 'ready'; catalog: Catalog }
  | { type: 'analysis'; requestId: number; revision: number; analysis: ScenarioAnalysis }
  | { type: 'validation'; requestId: number; validation: Validation }
  | { type: 'snapshot'; battleId: number; snapshot: Snapshot }
  | {
      type: 'compactSnapshot';
      battleId: number;
      tick: number;
      elapsed: number;
      unitCount: number;
      buffer: ArrayBuffer;
      result: BattleResult | null;
      inspectedInventory: UnitInventoryUpdate | null;
    }
  | { type: 'unitInventory'; battleId: number; update: UnitInventoryUpdate }
  | { type: 'performance'; battleId: number; behindRealTime: boolean }
  | { type: 'state'; battleId: number; state: 'running' | 'paused' | 'setup' }
  | { type: 'result'; battleId: number; result: BattleResult }
  | { type: 'error'; battleId?: number; requestId?: number; message: string };
