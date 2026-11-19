// Generated from crates/sim-core by `npm run gen:types`. Do not edit by hand.

export type AnalysisIssue = {
  code: IssueCode;
  severity: IssueSeverity;
  message: string;
  unitId: number | null;
  position: Point | null;
};
export type ArenaMap = {
  width: number;
  height: number;
  surfaces: Array<Surface>;
  elevations: Array<number>;
  obstacles: Array<Obstacle>;
  patrol: Array<Point>;
};
export type BattleResult = {
  winner: number | null;
  reason: ResultReason;
  elapsed: number;
  factions: Array<FactionResult>;
};
export type Behavior =
  'searching' | 'investigating' | 'advancing' | 'engaging' | 'fallingBack' | 'defending' | 'dead';
export type Catalog = {
  editorRules: EditorRules;
  maps: Array<MapPreset>;
  surfaces: Array<SurfaceDefinition>;
  lowObstacleHeight: number;
  fullObstacleHeight: number;
  units: Array<UnitDefinition>;
  weapons: Array<WeaponDefinition>;
  defaultFactions: Array<FactionDefinition>;
  defaultDoctrine: Doctrine;
  items: Array<ItemDefinition>;
  itemUseProperties: { [key in string]: ItemUseProperties };
  defaultInventory: InventorySnapshot;
  doctrineRules: DoctrineRules;
  maxPerFaction: number;
  defaultTimeLimit: number;
  defaultSeed: number;
};
export type Doctrine = {
  engagementDistance: number;
  fallbackHp: number;
  positionPreference: PositionPreference;
  routePreference: RoutePreference;
  grenades: GrenadeDoctrine;
  medical: MedicalDoctrine;
  outOfAmmoBehavior: OutOfAmmoBehavior;
  fireWhileMoving: boolean;
};
export type DoctrineRules = {
  grenadeMaxDistance: Rule;
  grenadeMinEnemies: Rule;
  grenadeSafetyMargin: Rule;
  medicalHpThreshold: Rule;
  medicalSafeSeconds: Rule;
  medicalCooldownSeconds: Rule;
};
export type EditorRules = {
  minDimension: number;
  maxDimension: number;
  minElevation: number;
  maxElevation: number;
  maxObstacleHeight: number;
};
export type FactionDefinition = {
  id: number;
  name: string;
  color: number;
  patrolDirection: PatrolDirection;
};
export type FactionResult = { faction: number; survivors: number; casualties: number };
export type GrenadeDoctrine = {
  policy: GrenadePolicy;
  maxDistance: number;
  minEnemies: number;
  safetyMargin: number;
};
export type GrenadePolicy = 'never' | 'clusters' | 'anySafeTarget' | 'emptyRifleOnly';
export type InventoryEntry = { id: number; itemId: string; quantity: number };
export type InventorySnapshot = {
  entries: Array<InventoryEntry>;
  leftHand: number | null;
  rightHand: number | null;
  weightGrams: number;
  stowedVolumeMl: number;
  maxWeightGrams: number;
  maxStowedVolumeMl: number;
  loadFactor: number;
  uses: Array<ItemUse>;
};
export type IssueCode =
  | 'invalidMapDimensions'
  | 'invalidTerrainArrays'
  | 'excessiveScenarioElements'
  | 'excessiveLoadoutEntries'
  | 'invalidObstacle'
  | 'excessiveObstacleFootprint'
  | 'nonFiniteValue'
  | 'tooFewFactions'
  | 'duplicateFactionId'
  | 'invalidFactionName'
  | 'invalidFactionColor'
  | 'invalidTimeLimit'
  | 'invalidLoadoutName'
  | 'duplicateLoadoutName'
  | 'invalidLoadout'
  | 'overlappingObstacle'
  | 'blockedPatrolPoint'
  | 'duplicateUnitId'
  | 'unknownUnitFaction'
  | 'unknownUnitDefinition'
  | 'blockedUnitPlacement'
  | 'overlappingUnitPlacement'
  | 'invalidDoctrine'
  | 'invalidInventoryDoctrine'
  | 'unknownUnitLoadout'
  | 'factionUnitLimitExceeded'
  | 'emptyFaction'
  | 'noPatrolPoints'
  | 'steepTransitions'
  | 'disconnectedRegions';
export type IssueSeverity = 'error' | 'warning';
export type ItemCategory = 'weapon' | 'ammo' | 'grenade' | 'tool' | 'medical' | 'ration' | 'water';
export type ItemDefinition = {
  id: string;
  name: string;
  category: ItemCategory;
  massGrams: number;
  volumeMl: number;
  handsRequired: number;
  weaponId: string | null;
};
export type ItemUse = { itemId: string; count: number };
export type ItemUseKind = 'grenade' | 'medical' | 'ration' | 'water';
export type ItemUseProperties = {
  kind: ItemUseKind;
  blastRadius?: number;
  damage?: number;
  cooldownSeconds?: number;
  healAmount?: number;
  restoreAmount?: number;
  useThreshold?: number;
};
export type LoadoutEntry = { itemId: string; quantity: number };
export type MapPreset = { id: string; name: string; description: string; map: ArenaMap };
export type MedicalDoctrine = {
  policy: MedicalPolicy;
  hpThreshold: number;
  safeSeconds: number;
  cooldownSeconds: number;
};
export type MedicalPolicy = 'never' | 'outOfContact' | 'lowHealth';
export type NamedLoadout = { name: string; entries: Array<LoadoutEntry> };
export type Obstacle = {
  x: number;
  y: number;
  width: number;
  height: number;
  heightMetres: number;
};
export type OutOfAmmoBehavior = 'withdraw' | 'continueDoctrine';
export type PatrolDirection = 'forward' | 'reverse';
export type Point = { x: number; y: number };
export type PositionPreference = 'balanced' | 'cover' | 'concealment';
export type ResultReason = 'elimination' | 'simultaneous_elimination' | 'timeout' | 'stopped';
export type RoutePreference = 'fastest' | 'protected';
export type Rule = { min: number; max: number; step: number };
export type Scenario = {
  map: ArenaMap;
  seed: number;
  factions: Array<FactionDefinition>;
  units: Array<UnitPlacement>;
  loadouts: Array<NamedLoadout>;
  timeLimit: number | null;
};
export type ScenarioAnalysis = {
  structuralValid: boolean;
  canStart: boolean;
  issues: Array<AnalysisIssue>;
  cells: Array<TerrainCellInfo>;
  steepEdges: Array<Point>;
};
export type Snapshot = {
  tick: number;
  elapsed: number;
  units: Array<UnitSnapshot>;
  result: BattleResult | null;
};
export type Surface = 'open' | 'road' | 'mud' | 'brush' | 'woodland';
export type SurfaceDefinition = {
  id: Surface;
  name: string;
  movementMultiplier: number;
  spottingMultiplier: number;
  vegetationHeight: number;
  color: number;
};
export type Sustainment = {
  nutrition: number;
  hydration: number;
  needsFactor: number;
  nutritionShortage: boolean;
  hydrationShortage: boolean;
};
export type TerrainCellInfo = {
  surface: Surface;
  elevation: number;
  movementMultiplier: number;
  traversable: boolean;
};
export type TerrainStatus = {
  surface: Surface;
  elevation: number;
  movementMultiplier: number;
  revealed: boolean;
  protection: number | null;
};
export type UnitDefinition = {
  id: string;
  name: string;
  maxHp: number;
  speed: number;
  eyeHeight: number;
  muzzleHeight: number;
  targetHeights: Array<number>;
  spottingRange: number;
  movingFireMultiplier: number;
  maxWeightGrams: number;
  maxStowedVolumeMl: number;
  defaultLoadout: Array<LoadoutEntry>;
};
export type UnitLoadout =
  { kind: 'named'; name: string } | { kind: 'inline'; entries: Array<LoadoutEntry> };
export type UnitPlacement = {
  id: number;
  faction: number;
  definitionId: string;
  position: Point;
  doctrine: Doctrine;
  loadout?: UnitLoadout;
};
export type UnitSnapshot = {
  id: number;
  faction: number;
  position: Point;
  hp: number;
  behavior: Behavior;
  targetId: number | null;
  shotsFired: number;
  terrain: TerrainStatus;
  inventory: InventorySnapshot;
  sustainment: Sustainment;
  mobilityFactor: number;
};
export type Validation = { valid: boolean; issues: Array<ValidationIssue> };
export type ValidationIssue = { code: IssueCode; message: string; unitId: number | null };
export type WeaponDefinition = {
  id: string;
  range: number;
  damage: number;
  hitChance: number;
  ammoItemId: string;
  cooldown: number;
  movingFireMultiplier: number;
};
