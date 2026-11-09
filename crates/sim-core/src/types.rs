use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub(crate) fn distance(self, other: Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Doctrine {
    pub engagement_distance: f64,
    pub fallback_hp: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitPlacement {
    pub id: u32,
    pub faction: u32,
    pub definition_id: String,
    pub position: Point,
    pub doctrine: Doctrine,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArenaMap {
    pub width: u32,
    pub height: u32,
    pub walls: Vec<Point>,
    pub cover: Vec<Point>,
    pub patrol: Vec<Point>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub map: ArenaMap,
    pub seed: u32,
    pub factions: Vec<u32>,
    pub units: Vec<UnitPlacement>,
    pub time_limit: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Behavior {
    Searching,
    Investigating,
    Advancing,
    Engaging,
    #[serde(rename = "Falling back")]
    FallingBack,
    Defending,
    Dead,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnitSnapshot {
    pub id: u32,
    pub faction: u32,
    pub position: Point,
    pub hp: f64,
    pub behavior: Behavior,
    pub target_id: Option<u32>,
    pub shots_fired: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FactionResult {
    pub faction: u32,
    pub survivors: u32,
    pub casualties: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BattleResult {
    pub winner: Option<u32>,
    pub reason: String,
    pub elapsed: f64,
    pub factions: Vec<FactionResult>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub tick: u32,
    pub elapsed: f64,
    pub units: Vec<UnitSnapshot>,
    pub result: Option<BattleResult>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub message: String,
    pub unit_id: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Validation {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitDefinition {
    pub id: String,
    pub name: String,
    pub max_hp: f64,
    pub speed: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDefinition {
    pub id: String,
    pub range: f64,
    pub damage: f64,
    pub hit_chance: f64,
    pub cooldown: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub map: ArenaMap,
    pub unit: UnitDefinition,
    pub weapon: WeaponDefinition,
    pub default_doctrine: Doctrine,
    pub max_per_faction: u32,
    pub default_time_limit: f64,
    pub default_seed: u32,
}
