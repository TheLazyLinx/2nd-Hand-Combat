use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{
    ArenaMap, Doctrine, DoctrineRules, EditorRules, InventorySnapshot, ItemDefinition,
    ItemUseProperties, LoadoutEntry, MapPreset, Point, SurfaceDefinition, TerrainCellInfo,
    UnitDefinition, WeaponDefinition,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct UnitPlacement {
    pub id: u32,
    pub faction: u32,
    pub definition_id: String,
    pub position: Point,
    pub doctrine: Doctrine,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub map: ArenaMap,
    pub seed: u32,
    pub factions: Vec<u32>,
    pub units: Vec<UnitPlacement>,
    pub time_limit: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
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
    pub severity: String,
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
    pub map: ArenaMap,
    pub editor_rules: EditorRules,
    pub maps: Vec<MapPreset>,
    pub surfaces: Vec<SurfaceDefinition>,
    pub low_obstacle_height: f64,
    pub full_obstacle_height: f64,
    pub terrain_cells: Vec<Vec<TerrainCellInfo>>,
    pub unit: UnitDefinition,
    pub weapon: WeaponDefinition,
    pub default_doctrine: Doctrine,
    pub items: Vec<ItemDefinition>,
    pub item_use_properties: BTreeMap<String, ItemUseProperties>,
    pub weapons: Vec<WeaponDefinition>,
    pub default_loadout: Vec<LoadoutEntry>,
    pub default_inventory: InventorySnapshot,
    pub doctrine_rules: DoctrineRules,
    pub max_per_faction: u32,
    pub default_time_limit: f64,
    pub default_seed: u32,
}
