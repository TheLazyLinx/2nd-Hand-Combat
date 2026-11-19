use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{
    ArenaMap, Doctrine, DoctrineRules, EditorRules, InventorySnapshot, ItemDefinition,
    ItemUseProperties, MapPreset, NamedLoadout, Point, SurfaceDefinition, TerrainCellInfo,
    UnitDefinition, UnitLoadout, WeaponDefinition,
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum PatrolDirection {
    #[default]
    Forward,
    Reverse,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct FactionDefinition {
    pub id: u32,
    pub name: String,
    pub color: u32,
    #[serde(default)]
    pub patrol_direction: PatrolDirection,
}

/// Central hostility query between two faction IDs.
/// Hook for a future diplomacy/relations table; currently all distinct factions are mutually hostile.
#[inline]
pub fn is_hostile(faction_a: u32, faction_b: u32) -> bool {
    faction_a != faction_b
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct UnitPlacement {
    pub id: u32,
    pub faction: u32,
    pub definition_id: String,
    pub position: Point,
    pub doctrine: Doctrine,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub loadout: Option<UnitLoadout>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub map: ArenaMap,
    pub seed: u32,
    pub factions: Vec<FactionDefinition>,
    pub units: Vec<UnitPlacement>,
    #[serde(default)]
    pub loadouts: Vec<NamedLoadout>,
    pub time_limit: Option<f64>,
}

impl Scenario {
    #[inline]
    pub fn is_hostile(&self, faction_a: u32, faction_b: u32) -> bool {
        is_hostile(faction_a, faction_b)
    }

    pub fn faction(&self, id: u32) -> Option<&FactionDefinition> {
        self.factions.iter().find(|f| f.id == id)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum IssueCode {
    InvalidMapDimensions,
    InvalidTerrainArrays,
    ExcessiveScenarioElements,
    ExcessiveLoadoutEntries,
    InvalidObstacle,
    ExcessiveObstacleFootprint,
    NonFiniteValue,
    TooFewFactions,
    DuplicateFactionId,
    InvalidFactionName,
    InvalidFactionColor,
    InvalidTimeLimit,
    InvalidLoadoutName,
    DuplicateLoadoutName,
    InvalidLoadout,
    OverlappingObstacle,
    BlockedPatrolPoint,
    DuplicateUnitId,
    UnknownUnitFaction,
    UnknownUnitDefinition,
    BlockedUnitPlacement,
    OverlappingUnitPlacement,
    InvalidDoctrine,
    InvalidInventoryDoctrine,
    UnknownUnitLoadout,
    FactionUnitLimitExceeded,
    EmptyFaction,
    NoPatrolPoints,
    SteepTransitions,
    DisconnectedRegions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub code: IssueCode,
    pub message: String,
    pub unit_id: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
pub struct Validation {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct AnalysisIssue {
    pub code: IssueCode,
    pub severity: IssueSeverity,
    pub message: String,
    pub unit_id: Option<u32>,
    pub position: Option<Point>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct ScenarioAnalysis {
    pub structural_valid: bool,
    pub can_start: bool,
    pub issues: Vec<AnalysisIssue>,
    pub cells: Vec<TerrainCellInfo>,
    pub steep_edges: Vec<Point>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub editor_rules: EditorRules,
    pub maps: Vec<MapPreset>,
    pub surfaces: Vec<SurfaceDefinition>,
    pub low_obstacle_height: f64,
    pub full_obstacle_height: f64,
    pub units: Vec<UnitDefinition>,
    pub weapons: Vec<WeaponDefinition>,
    pub default_factions: Vec<FactionDefinition>,
    pub default_doctrine: Doctrine,
    pub items: Vec<ItemDefinition>,
    pub item_use_properties: BTreeMap<String, ItemUseProperties>,
    pub default_inventory: InventorySnapshot,
    pub doctrine_rules: DoctrineRules,
    pub max_per_faction: u32,
    pub default_time_limit: f64,
    pub default_seed: u32,
}
