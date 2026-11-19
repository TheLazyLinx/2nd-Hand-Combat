/**
 * Zero-copy binary snapshot decoder matching `crates/sim-core/src/compact_snapshot.rs`.
 *
 * Keeps high-frequency unit state in its transferred `ArrayBuffer` (`72` bytes/unit) and
 * reconstructs full `UnitSnapshot` objects only on demand for the currently inspected soldier.
 */

import type {
  BattleResult,
  Behavior,
  InventorySnapshot,
  Snapshot,
  Surface,
  UnitSnapshot,
} from './types';

export const COMPACT_UNIT_BYTES = 72;

const FLAG_REVEALED = 1 << 0;
const FLAG_HAS_PROTECTION = 1 << 1;
const FLAG_NUTRITION_SHORTAGE = 1 << 2;
const FLAG_HYDRATION_SHORTAGE = 1 << 3;

const BEHAVIORS: readonly Behavior[] = [
  'searching',
  'investigating',
  'advancing',
  'engaging',
  'fallingBack',
  'defending',
  'dead',
];

const SURFACES: readonly Surface[] = ['open', 'road', 'mud', 'brush', 'woodland'];

export class CompactSnapshotView {
  readonly kind = 'compact' as const;
  private readonly view: DataView;
  private readonly idToIndex = new Map<number, number>();

  constructor(
    readonly tick: number,
    readonly elapsed: number,
    readonly unitCount: number,
    readonly buffer: ArrayBuffer,
    readonly result: BattleResult | null,
  ) {
    if (buffer.byteLength < unitCount * COMPACT_UNIT_BYTES) {
      throw new Error('Compact snapshot buffer is smaller than unitCount * COMPACT_UNIT_BYTES.');
    }
    this.view = new DataView(buffer);
    for (let i = 0; i < unitCount; i++) {
      this.idToIndex.set(this.view.getUint32(i * COMPACT_UNIT_BYTES + 24, true), i);
    }
  }

  x(index: number): number {
    return this.view.getFloat64(index * COMPACT_UNIT_BYTES, true);
  }
  y(index: number): number {
    return this.view.getFloat64(index * COMPACT_UNIT_BYTES + 8, true);
  }
  hp(index: number): number {
    return this.view.getFloat64(index * COMPACT_UNIT_BYTES + 16, true);
  }
  id(index: number): number {
    return this.view.getUint32(index * COMPACT_UNIT_BYTES + 24, true);
  }
  faction(index: number): number {
    return this.view.getUint32(index * COMPACT_UNIT_BYTES + 28, true);
  }
  shotsFired(index: number): number {
    return this.view.getUint32(index * COMPACT_UNIT_BYTES + 32, true);
  }
  targetId(index: number): number | null {
    const raw = this.view.getInt32(index * COMPACT_UNIT_BYTES + 36, true);
    return raw < 0 ? null : raw;
  }
  behavior(index: number): Behavior {
    return BEHAVIORS[this.view.getUint8(index * COMPACT_UNIT_BYTES + 68)] ?? 'dead';
  }
  indexOf(id: number): number | undefined {
    return this.idToIndex.get(id);
  }

  findNearest(x: number, y: number, maxDistance: number): number | null {
    let bestId: number | null = null;
    let bestDist = maxDistance;
    for (let i = 0; i < this.unitCount; i++) {
      const d = Math.hypot(this.x(i) - x, this.y(i) - y);
      if (d < bestDist) {
        bestDist = d;
        bestId = this.id(i);
      }
    }
    return bestId;
  }

  unitSummary(id: number): { hp: number; behavior: Behavior } | null {
    const index = this.idToIndex.get(id);
    if (index === undefined) return null;
    return { hp: this.hp(index), behavior: this.behavior(index) };
  }

  unitSnapshot(id: number, inventory: InventorySnapshot): UnitSnapshot | null {
    const index = this.idToIndex.get(id);
    if (index === undefined) return null;
    const base = index * COMPACT_UNIT_BYTES;
    const flags = this.view.getUint8(base + 70);
    return {
      id: this.view.getUint32(base + 24, true),
      faction: this.view.getUint32(base + 28, true),
      position: {
        x: this.view.getFloat64(base, true),
        y: this.view.getFloat64(base + 8, true),
      },
      hp: this.view.getFloat64(base + 16, true),
      behavior: BEHAVIORS[this.view.getUint8(base + 68)] ?? 'dead',
      targetId: this.targetId(index),
      shotsFired: this.view.getUint32(base + 32, true),
      terrain: {
        surface: SURFACES[this.view.getUint8(base + 69)] ?? 'open',
        elevation: this.view.getFloat32(base + 40, true),
        movementMultiplier: this.view.getFloat32(base + 44, true),
        revealed: (flags & FLAG_REVEALED) !== 0,
        protection:
          (flags & FLAG_HAS_PROTECTION) !== 0 ? this.view.getFloat32(base + 48, true) : null,
      },
      inventory,
      sustainment: {
        nutrition: this.view.getFloat32(base + 56, true),
        hydration: this.view.getFloat32(base + 60, true),
        needsFactor: this.view.getFloat32(base + 64, true),
        nutritionShortage: (flags & FLAG_NUTRITION_SHORTAGE) !== 0,
        hydrationShortage: (flags & FLAG_HYDRATION_SHORTAGE) !== 0,
      },
      mobilityFactor: this.view.getFloat32(base + 52, true),
    };
  }
}

export type LiveFrame = Snapshot | CompactSnapshotView;

export function isCompactFrame(frame: LiveFrame | null): frame is CompactSnapshotView {
  return frame !== null && 'kind' in frame && frame.kind === 'compact';
}

export function frameUnitSummary(
  frame: LiveFrame | null,
  id: number,
): { hp: number; behavior: Behavior } | null {
  if (!frame) return null;
  if (isCompactFrame(frame)) return frame.unitSummary(id);
  const unit = frame.units.find((u) => u.id === id);
  return unit ? { hp: unit.hp, behavior: unit.behavior } : null;
}
