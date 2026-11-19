import init, {
  analyze_scenario_json,
  catalog_json,
  validate_scenario_json,
  WasmBattle,
} from './generated/wasm/sim_wasm';
import wasmUrl from './generated/wasm/sim_wasm_bg.wasm?url';
import type {
  BattleResult,
  Catalog,
  ScenarioAnalysis,
  Snapshot,
  UnitInventoryUpdate,
  Validation,
  WorkerCommand,
  WorkerReply,
} from './lib/types';

const STEP_MS = 50;
const MAX_CATCHUP_STEPS = 5;
const COMPACT_SNAPSHOT_THRESHOLD = 32;
const CAUGHT_UP_PUMPS_TO_CLEAR = 10;

let initialized = false;
let battle: WasmBattle | undefined;
let battleId = 0;
let running = false;
let useCompact = false;
let inspectedUnitId: number | null = null;
let inspectedInventoryVersion = 0;
let previousTime = 0;
let accumulator = 0;
let emittedResult = false;
let lastPublishedTick = 0;
let behindRealTime = false;
let caughtUpPumps = 0;
let timer: ReturnType<typeof setInterval> | undefined;

const send = (message: WorkerReply) => self.postMessage(message);

function setBehindRealTime(next: boolean) {
  if (behindRealTime === next) return;
  behindRealTime = next;
  send({ type: 'performance', battleId, behindRealTime: next });
}

function updateLagState(clamped: boolean) {
  if (clamped) {
    caughtUpPumps = 0;
    setBehindRealTime(true);
  } else if (behindRealTime) {
    caughtUpPumps++;
    if (caughtUpPumps >= CAUGHT_UP_PUMPS_TO_CLEAR) {
      caughtUpPumps = 0;
      setBehindRealTime(false);
    }
  }
}

function stop() {
  running = false;
  accumulator = 0;
  caughtUpPumps = 0;
  setBehindRealTime(false);
  if (timer !== undefined) clearInterval(timer);
  timer = undefined;
}

function fullSnapshot(): Snapshot {
  if (!battle) throw new Error('No battle has been started.');
  return JSON.parse(battle.snapshot_json()) as Snapshot;
}

function finishIfResult(result: BattleResult | null) {
  if (result && !emittedResult) {
    emittedResult = true;
    stop();
    send({ type: 'result', battleId, result });
  }
}

function publishCompact() {
  if (!battle) throw new Error('No battle has been started.');
  const tick = battle.tick();
  const elapsed = battle.elapsed();
  const unitCount = battle.unit_count();
  const bytes = battle.compact_units_bytes();
  const resultJson = battle.result_json();
  const result = resultJson ? (JSON.parse(resultJson) as BattleResult) : null;
  let inspectedInventory: UnitInventoryUpdate | null = null;
  if (inspectedUnitId !== null) {
    const invJson = battle.unit_inventory_if_changed_json(
      inspectedUnitId,
      inspectedInventoryVersion,
    );
    if (invJson) {
      inspectedInventory = JSON.parse(invJson) as UnitInventoryUpdate;
      inspectedInventoryVersion = inspectedInventory.version;
    }
  }
  const buffer =
    bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength
      ? (bytes.buffer as ArrayBuffer)
      : bytes.slice().buffer;
  lastPublishedTick = tick;
  const reply: WorkerReply = {
    type: 'compactSnapshot',
    battleId,
    tick,
    elapsed,
    unitCount,
    buffer,
    result,
    inspectedInventory,
  };
  self.postMessage(reply, [buffer]);
  finishIfResult(result);
}

function publish() {
  if (useCompact) {
    publishCompact();
    return;
  }
  const value = fullSnapshot();
  lastPublishedTick = value.tick;
  send({ type: 'snapshot', battleId, snapshot: value });
  finishIfResult(value.result);
}

function pump() {
  if (!running || !battle) return;
  try {
    const now = performance.now();
    const rawAccumulator = accumulator + (now - previousTime);
    const clamped = rawAccumulator > STEP_MS * MAX_CATCHUP_STEPS;
    accumulator = clamped ? STEP_MS * MAX_CATCHUP_STEPS : rawAccumulator;
    previousTime = now;
    const steps = Math.floor(accumulator / STEP_MS);
    if (steps === 0) return;
    battle.advance(steps);
    accumulator -= steps * STEP_MS;
    updateLagState(clamped);
    // Build and serialize snapshots only when publishing (10 Hz) or when the battle finishes.
    const tick = battle.tick();
    if (battle.is_finished() || Math.floor(tick / 2) !== Math.floor(lastPublishedTick / 2)) {
      publish();
    }
  } catch (error) {
    stop();
    send({ type: 'error', battleId, message: String(error) });
  }
}

function run() {
  previousTime = performance.now();
  accumulator = 0;
  caughtUpPumps = 0;
  running = true;
  timer = setInterval(pump, 20);
  send({ type: 'state', battleId, state: 'running' });
}

async function command(message: WorkerCommand) {
  if (message.type === 'initialize') {
    if (!initialized) {
      await init({ module_or_path: wasmUrl });
      initialized = true;
    }
    send({ type: 'ready', catalog: JSON.parse(catalog_json()) as Catalog });
    return;
  }
  if (!initialized) throw new Error('The simulation has not initialized.');
  if (message.type === 'analyze') {
    const analysis = JSON.parse(
      analyze_scenario_json(JSON.stringify(message.scenario)),
    ) as ScenarioAnalysis;
    send({ type: 'analysis', requestId: message.requestId, revision: message.revision, analysis });
    return;
  }
  if (message.type === 'validate') {
    const validation = JSON.parse(
      validate_scenario_json(JSON.stringify(message.scenario), message.forStart),
    ) as Validation;
    send({ type: 'validation', requestId: message.requestId, validation });
    return;
  }
  if (message.type === 'start') {
    if (message.battleId <= battleId) throw new Error('Cannot start a stale battle.');
    const next = new WasmBattle(JSON.stringify(message.scenario));
    stop();
    battle?.free();
    battle = next;
    battleId = message.battleId;
    useCompact = message.compact ?? message.scenario.units.length >= COMPACT_SNAPSHOT_THRESHOLD;
    inspectedUnitId = null;
    inspectedInventoryVersion = 0;
    emittedResult = false;
    publish();
    run();
    return;
  }
  if (message.type === 'reset') {
    if (message.battleId <= battleId) return;
    stop();
    battle?.free();
    battle = undefined;
    battleId = message.battleId;
    useCompact = false;
    inspectedUnitId = null;
    inspectedInventoryVersion = 0;
    emittedResult = false;
    send({ type: 'state', battleId, state: 'setup' });
    return;
  }
  if (message.battleId !== battleId) return;
  if (!battle) throw new Error('No battle has been started.');
  if (message.type === 'inspect') {
    if (inspectedUnitId !== message.unitId) {
      inspectedUnitId = message.unitId;
      inspectedInventoryVersion = 0;
    }
    if (useCompact && inspectedUnitId !== null) {
      const invJson = battle.unit_inventory_if_changed_json(
        inspectedUnitId,
        inspectedInventoryVersion,
      );
      if (invJson) {
        const update = JSON.parse(invJson) as UnitInventoryUpdate;
        inspectedInventoryVersion = update.version;
        send({ type: 'unitInventory', battleId, update });
      }
    }
  } else if (message.type === 'pause') {
    // A terminal result wins over a pause queued at the same time.
    if (emittedResult) return;
    stop();
    publish();
    send({ type: 'state', battleId, state: 'paused' });
  } else if (message.type === 'resume') {
    if (emittedResult) throw new Error('Reset the finished battle before starting again.');
    if (!running) run();
  } else if (message.type === 'stop') {
    if (emittedResult) return;
    stop();
    battle.stop();
    publish();
  }
}

// Initialization is asynchronous; preserve command order across the await.
let queue = Promise.resolve();
self.onmessage = (event: MessageEvent<WorkerCommand>) => {
  const message = event.data;
  queue = queue
    .then(() => command(message))
    .catch((error: unknown) => {
      send({
        type: 'error',
        ...('battleId' in message ? { battleId: message.battleId } : {}),
        ...('requestId' in message ? { requestId: message.requestId } : {}),
        message: String(error),
      });
    });
};
