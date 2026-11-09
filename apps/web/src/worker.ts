import init, { catalog_json, validate_scenario_json, WasmBattle } from './generated/sim_wasm';
import wasmUrl from './generated/sim_wasm_bg.wasm?url';
import type { Catalog, Snapshot, Validation, WorkerCommand, WorkerReply } from './lib/types';

const STEP_MS = 50;
let initialized = false;
let battle: WasmBattle | undefined;
let battleId = 0;
let running = false;
let previousTime = 0;
let accumulator = 0;
let emittedResult = false;
let lastPublishedTick = 0;
let timer: ReturnType<typeof setInterval> | undefined;
const send = (message: WorkerReply) => self.postMessage(message);

function stop() {
  running = false;
  accumulator = 0;
  if (timer !== undefined) clearInterval(timer);
  timer = undefined;
}

function snapshot(): Snapshot {
  if (!battle) throw new Error('No battle has been started.');
  return JSON.parse(battle.snapshot_json()) as Snapshot;
}

function publish(value: Snapshot) {
  lastPublishedTick = value.tick;
  send({ type: 'snapshot', battleId, snapshot: value });
  if (value.result && !emittedResult) {
    emittedResult = true;
    stop();
    send({ type: 'result', battleId, result: value.result });
  }
}

function pump() {
  if (!running || !battle) return;
  try {
    const now = performance.now();
    accumulator = Math.min(accumulator + now - previousTime, STEP_MS * 5);
    previousTime = now;
    const steps = Math.floor(accumulator / STEP_MS);
    if (steps === 0) return;
    battle.advance(steps);
    accumulator -= steps * STEP_MS;
    const value = snapshot();
    if (value.result || Math.floor(value.tick / 2) !== Math.floor(lastPublishedTick / 2)) publish(value);
  } catch (error) {
    stop();
    send({ type: 'error', battleId, message: String(error) });
  }
}

function run() {
  previousTime = performance.now();
  accumulator = 0;
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
  if (message.type === 'validate') {
    const validation = JSON.parse(validate_scenario_json(JSON.stringify(message.scenario), message.forStart)) as Validation;
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
    emittedResult = false;
    publish(snapshot());
    run();
    return;
  }
  if (message.type === 'reset') {
    if (message.battleId <= battleId) return;
    stop();
    battle?.free();
    battle = undefined;
    battleId = message.battleId;
    emittedResult = false;
    send({ type: 'state', battleId, state: 'setup' });
    return;
  }
  if (message.battleId !== battleId) return;
  if (!battle) throw new Error('No battle has been started.');
  if (message.type === 'pause') {
    // A terminal result wins over a pause queued at the same time.
    if (emittedResult) return;
    stop();
    publish(snapshot());
    send({ type: 'state', battleId, state: 'paused' });
  } else if (message.type === 'resume') {
    if (emittedResult) throw new Error('Reset the finished battle before starting again.');
    if (!running) run();
  }
}

// Initialization is asynchronous; preserve command order across the await.
let queue = Promise.resolve();
self.onmessage = (event: MessageEvent<WorkerCommand>) => {
  const message = event.data;
  queue = queue.then(() => command(message)).catch((error: unknown) => {
    send({
      type: 'error',
      ...('battleId' in message ? { battleId: message.battleId } : {}),
      ...('requestId' in message ? { requestId: message.requestId } : {}),
      message: String(error),
    });
  });
};
