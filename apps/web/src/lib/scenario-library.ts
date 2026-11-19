import type { Doctrine, ScenarioDocument } from './types';

export interface SavedScenario {
  id: string;
  updatedAt: string;
  document: ScenarioDocument;
}

export const MAX_DOCUMENT_BYTES = 8 * 1024 * 1024;
const DATABASE = '2nd-hand-combat';
const STORE = 'scenarios';

function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value))
    throw new Error(`${label} must be an object.`);
  return value as Record<string, unknown>;
}
function list(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${label} must be an array.`);
  return value;
}
function text(
  value: unknown,
  label: string,
  maximum: number,
  required = false,
): asserts value is string {
  if (typeof value !== 'string' || value.length > maximum || (required && !value.trim()))
    throw new Error(
      `${label} must ${required ? 'be non-empty and ' : ''}contain at most ${maximum} characters.`,
    );
}

/** Upgrade only absent legacy fields. Rust validates field values, enums, and numeric limits. */
function migrateInventoryDoctrine(
  doctrine: Record<string, unknown>,
  legacy: boolean,
  defaults?: Doctrine,
) {
  function introduced(key: 'grenades' | 'medical' | 'outOfAmmoBehavior'): unknown {
    if (doctrine[key] !== undefined || !legacy) return doctrine[key];
    if (!defaults) throw new Error('The simulation catalog is required to open legacy scenarios.');
    return structuredClone(defaults[key]);
  }
  const grenades = record(introduced('grenades'), 'Grenade doctrine');
  const medical = record(introduced('medical'), 'Medical doctrine');
  const outOfAmmoBehavior = introduced('outOfAmmoBehavior');
  if (typeof outOfAmmoBehavior !== 'string') throw new Error('Unknown empty-rifle behavior.');
  return { ...doctrine, grenades, medical, outOfAmmoBehavior };
}

/** Checks portable envelope and upgrades v1 in memory; Rust owns scenario field and gameplay validation. */
export function parseScenarioDocument(json: string, defaultDoctrine?: Doctrine): ScenarioDocument {
  if (new TextEncoder().encode(json).byteLength > MAX_DOCUMENT_BYTES)
    throw new Error('Scenario files must be no larger than 8 MiB.');
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    throw new Error('This file is not valid scenario JSON.');
  }
  const doc = record(parsed, 'Scenario document');
  if (doc.format !== '2nd-hand-combat-scenario')
    throw new Error('This file is not a 2nd Hand Combat scenario.');
  if (doc.schemaVersion !== 1 && doc.schemaVersion !== 2)
    throw new Error('This scenario version is not supported.');
  text(doc.name, 'Scenario name', 100, true);
  const description = doc.description === undefined ? '' : doc.description;
  text(description, 'Description', 2000);
  const scenario = record(doc.scenario, 'Scenario');
  const units = list(scenario.units, 'Units').map((value) => {
    const unit = record(value, 'Soldier');
    const doctrine = record(unit.doctrine, 'Doctrine');
    return {
      ...unit,
      doctrine: migrateInventoryDoctrine(doctrine, doc.schemaVersion === 1, defaultDoctrine),
    };
  });
  // Parsing makes a detached plain-data document. Unknown metadata has no executable behavior.
  return {
    format: '2nd-hand-combat-scenario',
    schemaVersion: 2,
    name: doc.name,
    description,
    scenario: { ...scenario, units } as unknown as ScenarioDocument['scenario'],
  };
}

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (!globalThis.indexedDB) {
      reject(new Error('Browser scenario storage is unavailable. Export a JSON backup instead.'));
      return;
    }
    let request: IDBOpenDBRequest;
    let blocked = false;
    try {
      request = indexedDB.open(DATABASE, 1);
    } catch (error) {
      reject(error);
      return;
    }
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(STORE))
        request.result.createObjectStore(STORE, { keyPath: 'id' });
    };
    request.onerror = () =>
      reject(
        new Error(
          `Cannot open the scenario library: ${request.error?.message ?? 'storage failed'}`,
        ),
      );
    request.onblocked = () => {
      blocked = true;
      reject(
        new Error('The scenario library is blocked by another tab. Close that tab and retry.'),
      );
    };
    request.onsuccess = () => {
      if (blocked) {
        request.result.close();
        return;
      }
      request.result.onversionchange = () => request.result.close();
      resolve(request.result);
    };
  });
}

async function transaction<T>(
  mode: IDBTransactionMode,
  operation: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    let tx: IDBTransaction;
    try {
      tx = db.transaction(STORE, mode);
    } catch (error) {
      db.close();
      reject(error);
      return;
    }
    let result: T;
    let request: IDBRequest<T>;
    try {
      request = operation(tx.objectStore(STORE));
    } catch (error) {
      tx.abort();
      db.close();
      reject(error);
      return;
    }
    request.onsuccess = () => {
      result = request.result;
    };
    tx.oncomplete = () => {
      db.close();
      resolve(result);
    };
    const fail = () => {
      db.close();
      reject(
        new Error(
          `Scenario storage failed: ${tx.error?.message ?? request.error?.message ?? 'the transaction was aborted'}. Your draft is still open; export a JSON backup.`,
        ),
      );
    };
    tx.onerror = fail;
    tx.onabort = fail;
  });
}

export async function listScenarios(defaultDoctrine: Doctrine): Promise<SavedScenario[]> {
  const entries = await transaction<unknown[]>('readonly', (store) => store.getAll());
  return entries
    .map((value) => {
      const entry = record(value, 'Saved scenario');
      text(entry.id, 'Saved scenario ID', 100, true);
      text(entry.updatedAt, 'Modification time', 100, true);
      const document = parseScenarioDocument(JSON.stringify(entry.document), defaultDoctrine);
      return { id: entry.id, updatedAt: entry.updatedAt, document };
    })
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt) || a.id.localeCompare(b.id));
}

/** Caller must obtain structural approval from Rust before saving this editable document. */
export async function saveScenario(
  document: ScenarioDocument,
  id: string = crypto.randomUUID(),
): Promise<SavedScenario> {
  const safe = parseScenarioDocument(JSON.stringify(document));
  const entry: SavedScenario = { id, updatedAt: new Date().toISOString(), document: safe };
  await transaction('readwrite', (store) => store.put(entry));
  return structuredClone(entry);
}

export async function deleteScenario(id: string): Promise<void> {
  await transaction('readwrite', (store) => store.delete(id));
}

export function downloadScenario(document: ScenarioDocument): void {
  const safe = parseScenarioDocument(JSON.stringify(document));
  const url = URL.createObjectURL(new Blob([JSON.stringify(safe)], { type: 'application/json' }));
  const anchor = window.document.createElement('a');
  anchor.href = url;
  anchor.download = `${safe.name.replace(/[^a-zA-Z0-9_-]+/g, '-').slice(0, 80) || 'scenario'}.json`;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
