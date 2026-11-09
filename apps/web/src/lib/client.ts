import type { Scenario, Validation, WorkerCommand, WorkerReply } from './types';

interface PendingValidation {
  resolve: (value: Validation) => void;
  reject: (reason: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

export class BattleClient {
  private worker: Worker;
  private battleId = 0;
  private requestId = 0;
  private disposed = false;
  private pending = new Map<number, PendingValidation>();

  constructor(private readonly onMessage: (message: WorkerReply) => void) {
    this.worker = new Worker(new URL('../worker.ts', import.meta.url), { type: 'module' });
    this.worker.onmessage = (event: MessageEvent<WorkerReply>) => {
      if (this.disposed) return;
      const message = event.data;
      if ('battleId' in message && message.battleId !== undefined && message.battleId !== this.battleId) return;
      if (message.type === 'validation') {
        const pending = this.take(message.requestId);
        pending?.resolve(message.validation);
      } else if (message.type === 'error' && message.requestId !== undefined) {
        this.take(message.requestId)?.reject(new Error(message.message));
      } else {
        this.onMessage(message);
      }
    };
    this.worker.onerror = event => {
      event.preventDefault();
      this.fail(event.message || 'The simulation Worker failed to load.');
    };
    this.worker.onmessageerror = () => this.fail('The simulation sent an unreadable message.');
  }

  private take(id: number) {
    const pending = this.pending.get(id);
    if (pending) clearTimeout(pending.timer);
    this.pending.delete(id);
    return pending;
  }

  private fail(message: string) {
    if (this.disposed) return;
    for (const id of this.pending.keys()) this.take(id)?.reject(new Error(message));
    this.onMessage({ type: 'error', message });
    this.dispose();
  }

  private send(message: WorkerCommand) {
    if (this.disposed) throw new Error('This simulation session has been closed.');
    this.worker.postMessage(message);
  }

  initialize() { this.send({ type: 'initialize' }); }

  validate(scenario: Scenario, forStart: boolean): Promise<Validation> {
    if (this.disposed) return Promise.reject(new Error('This simulation session has been closed.'));
    const requestId = ++this.requestId;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.take(requestId)?.reject(new Error('The simulation did not respond to validation. Reopen Skirmish to retry.'));
      }, 10_000);
      this.pending.set(requestId, { resolve, reject, timer });
      this.send({ type: 'validate', requestId, scenario, forStart });
    });
  }

  start(scenario: Scenario) {
    this.send({ type: 'start', battleId: ++this.battleId, scenario });
    return this.battleId;
  }

  pause() { this.send({ type: 'pause', battleId: this.battleId }); }
  resume() { this.send({ type: 'resume', battleId: this.battleId }); }

  reset() {
    this.send({ type: 'reset', battleId: ++this.battleId });
    return this.battleId;
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    for (const id of this.pending.keys()) this.take(id)?.reject(new Error('The simulation session was closed.'));
    this.worker.terminate();
  }
}
