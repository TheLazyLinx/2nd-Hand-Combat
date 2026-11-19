/** Faction presentation helpers, unit definition lookup, and setup inventory preview. */

import type {
  Behavior,
  Catalog,
  FactionDefinition,
  InventoryEntry,
  InventorySnapshot,
  LoadoutEntry,
  Scenario,
  UnitDefinition,
  UnitPlacement,
} from './types';

const BEHAVIOR_LABELS: Record<Behavior, string> = {
  searching: 'Searching',
  investigating: 'Investigating',
  advancing: 'Advancing',
  engaging: 'Engaging',
  fallingBack: 'Falling back',
  defending: 'Defending',
  dead: 'Dead',
};

export function behaviorLabel(behavior: Behavior): string {
  return BEHAVIOR_LABELS[behavior];
}

export const FACTION_PALETTE: readonly number[] = [
  0x315f62, 0xa24f32, 0x5d6e37, 0x7a5482, 0xb5892b, 0x3f6493, 0x8c4a64, 0x4b7a6b,
];

export function factionColorHex(color: number): string {
  return `#${(color & 0xffffff).toString(16).padStart(6, '0')}`;
}

export function parseColorHex(hex: string): number {
  const cleaned = hex.trim().replace(/^#/, '');
  const parsed = Number.parseInt(cleaned, 16);
  return Number.isFinite(parsed) ? parsed & 0xffffff : FACTION_PALETTE[0];
}

export function factionById(
  factions: readonly FactionDefinition[],
  id: number,
): FactionDefinition | undefined {
  return factions.find((f) => f.id === id);
}

export function factionName(factions: readonly FactionDefinition[], id: number): string {
  return factionById(factions, id)?.name ?? `Faction ${id}`;
}

export function factionColor(factions: readonly FactionDefinition[], id: number): number {
  return factionById(factions, id)?.color ?? FACTION_PALETTE[0];
}

export function defaultLegacyFaction(
  id: number,
  index: number,
  defaults?: readonly FactionDefinition[],
): FactionDefinition {
  const template = defaults?.[index];
  if (template) {
    return {
      id,
      name: template.name,
      color: template.color,
      patrolDirection: template.patrolDirection,
    };
  }
  return {
    id,
    name: `Faction ${index + 1}`,
    color: FACTION_PALETTE[index % FACTION_PALETTE.length],
    patrolDirection: index % 2 === 0 ? 'forward' : 'reverse',
  };
}

export function nextFactionDefinition(existing: readonly FactionDefinition[]): FactionDefinition {
  const usedIds = new Set(existing.map((f) => f.id));
  let id = 0;
  while (usedIds.has(id)) id++;
  const usedNames = new Set(existing.map((f) => f.name.toLowerCase()));
  let num = existing.length + 1;
  let name = `Faction ${num}`;
  while (usedNames.has(name.toLowerCase())) {
    num++;
    name = `Faction ${num}`;
  }
  const usedColors = new Set(existing.map((f) => f.color));
  const color =
    FACTION_PALETTE.find((c) => !usedColors.has(c)) ??
    FACTION_PALETTE[existing.length % FACTION_PALETTE.length];
  return {
    id,
    name,
    color,
    patrolDirection: existing.length % 2 === 0 ? 'forward' : 'reverse',
  };
}

export function unitDefinition(catalog: Catalog, definitionId: string): UnitDefinition {
  return catalog.units.find((u) => u.id === definitionId) ?? catalog.units[0];
}

/** Builds a setup-time inventory preview matching Rust's loadout resolution for custom loadouts. */
export function previewUnitInventory(
  catalog: Catalog,
  scenario: Scenario,
  unit: UnitPlacement,
): InventorySnapshot {
  if (!unit.loadout && unit.definitionId === catalog.units[0]?.id) {
    return catalog.defaultInventory;
  }
  const def = unitDefinition(catalog, unit.definitionId);
  let loadout: LoadoutEntry[] = def.defaultLoadout;
  if (unit.loadout?.kind === 'inline') {
    loadout = unit.loadout.entries;
  } else if (unit.loadout?.kind === 'named') {
    const loadoutName = unit.loadout.name;
    const named = scenario.loadouts.find((l) => l.name === loadoutName);
    if (named) loadout = named.entries;
  }
  const entries: InventoryEntry[] = loadout.map((e, i) => ({
    id: i + 1,
    itemId: e.itemId,
    quantity: e.quantity,
  }));
  let leftHand: number | null = null;
  let rightHand: number | null = null;
  const itemMap = new Map(catalog.items.map((item) => [item.id, item]));
  const weaponIdx = entries.findIndex((e) => {
    const item = itemMap.get(e.itemId);
    return !!item?.weaponId && item.handsRequired > 0;
  });
  if (weaponIdx >= 0) {
    const entry = entries[weaponIdx];
    const item = itemMap.get(entry.itemId)!;
    let equippedId = entry.id;
    if (entry.quantity > 1) {
      equippedId = entries.length + 1;
      entry.quantity -= 1;
      entries.push({ id: equippedId, itemId: entry.itemId, quantity: 1 });
    }
    if (item.handsRequired === 2) {
      leftHand = equippedId;
      rightHand = equippedId;
    } else {
      rightHand = equippedId;
    }
  }
  let weightGrams = 0;
  let stowedVolumeMl = 0;
  for (const entry of entries) {
    const item = itemMap.get(entry.itemId);
    if (!item) continue;
    const held = leftHand === entry.id || rightHand === entry.id;
    weightGrams += item.massGrams * entry.quantity;
    if (!held) stowedVolumeMl += item.volumeMl * entry.quantity;
  }
  const weightFraction = def.maxWeightGrams > 0 ? weightGrams / def.maxWeightGrams : 0;
  const volumeFraction = def.maxStowedVolumeMl > 0 ? stowedVolumeMl / def.maxStowedVolumeMl : 0;
  return {
    entries,
    leftHand,
    rightHand,
    weightGrams,
    stowedVolumeMl,
    maxWeightGrams: def.maxWeightGrams,
    maxStowedVolumeMl: def.maxStowedVolumeMl,
    loadFactor: 1 - 0.25 * weightFraction - 0.15 * volumeFraction,
    uses: [],
  };
}
