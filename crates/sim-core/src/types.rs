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
    pub position_preference: PositionPreference,
    pub route_preference: RoutePreference,
    #[serde(default)]
    pub grenades: GrenadeDoctrine,
    #[serde(default)]
    pub medical: MedicalDoctrine,
    #[serde(default)]
    pub out_of_ammo_behavior: OutOfAmmoBehavior,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GrenadePolicy {
    Never,
    Clusters,
    AnySafeTarget,
    EmptyRifleOnly,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrenadeDoctrine {
    pub policy: GrenadePolicy,
    pub max_distance: f64,
    pub min_enemies: u32,
    pub safety_margin: f64,
}
impl Default for GrenadeDoctrine {
    fn default() -> Self {
        Self {
            policy: GrenadePolicy::Clusters,
            max_distance: 6.0,
            min_enemies: 2,
            safety_margin: 0.25,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MedicalPolicy {
    Never,
    OutOfContact,
    LowHealth,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MedicalDoctrine {
    pub policy: MedicalPolicy,
    pub hp_threshold: f64,
    pub safe_seconds: f64,
    pub cooldown_seconds: f64,
}
impl Default for MedicalDoctrine {
    fn default() -> Self {
        Self {
            policy: MedicalPolicy::OutOfContact,
            hp_threshold: 50.0,
            safe_seconds: 5.0,
            cooldown_seconds: 10.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutOfAmmoBehavior {
    #[default]
    Withdraw,
    ContinueDoctrine,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PositionPreference {
    Balanced,
    Cover,
    Concealment,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RoutePreference {
    Fastest,
    Protected,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Surface {
    Open,
    Road,
    Mud,
    Brush,
    Woodland,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obstacle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub height_metres: f64,
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
    pub surfaces: Vec<Surface>,
    pub elevations: Vec<f64>,
    pub obstacles: Vec<Obstacle>,
    pub patrol: Vec<Point>,
}
impl ArenaMap {
    /// Row-major cells and vertices; each cell's diagonal runs NW to SE.
    pub fn flat(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            surfaces: vec![Surface::Open; width as usize * height as usize],
            elevations: vec![0.0; (width as usize + 1) * (height as usize + 1)],
            obstacles: Vec::new(),
            patrol: Vec::new(),
        }
    }
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
    pub terrain: TerrainStatus,
    pub inventory: InventorySnapshot,
    pub sustainment: Sustainment,
    pub mobility_factor: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerrainStatus {
    pub surface: Surface,
    pub elevation: f64,
    pub movement_multiplier: f64,
    pub revealed: bool,
    pub protection: Option<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceDefinition {
    pub id: Surface,
    pub name: String,
    pub movement_multiplier: f64,
    pub spotting_multiplier: f64,
    pub vegetation_height: f64,
    pub color: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub map: ArenaMap,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainCellInfo {
    pub surface: Surface,
    pub elevation: f64,
    pub movement_multiplier: f64,
    pub traversable: bool,
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
    pub max_weight_grams: u32,
    pub max_stowed_volume_ml: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDefinition {
    pub id: String,
    pub range: f64,
    pub damage: f64,
    pub hit_chance: f64,
    pub ammo_item_id: String,
    pub cooldown: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorRules {
    pub min_dimension: u32,
    pub max_dimension: u32,
    pub min_elevation: f64,
    pub max_elevation: f64,
    pub max_obstacle_height: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisIssue {
    pub severity: String,
    pub message: String,
    pub unit_id: Option<u32>,
    pub position: Option<Point>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioAnalysis {
    pub structural_valid: bool,
    pub can_start: bool,
    pub issues: Vec<AnalysisIssue>,
    pub cells: Vec<TerrainCellInfo>,
    pub steep_edges: Vec<Point>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub item_use_properties: std::collections::BTreeMap<String, ItemUseProperties>,
    pub weapons: Vec<WeaponDefinition>,
    pub default_loadout: Vec<LoadoutEntry>,
    pub default_inventory: InventorySnapshot,
    pub doctrine_rules: DoctrineRules,
    pub max_per_faction: u32,
    pub default_time_limit: f64,
    pub default_seed: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub mass_grams: u32,
    pub volume_ml: u32,
    pub hands_required: u32,
    pub weapon_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    pub id: u32,
    pub item_id: String,
    pub quantity: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemUse {
    pub item_id: String,
    pub count: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InventorySnapshot {
    pub entries: Vec<InventoryEntry>,
    pub left_hand: Option<u32>,
    pub right_hand: Option<u32>,
    pub weight_grams: u32,
    pub stowed_volume_ml: u32,
    pub max_weight_grams: u32,
    pub max_stowed_volume_ml: u32,
    pub load_factor: f64,
    pub uses: Vec<ItemUse>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sustainment {
    pub nutrition: f64,
    pub hydration: f64,
    pub needs_factor: f64,
    pub nutrition_shortage: bool,
    pub hydration_shortage: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadoutEntry {
    pub item_id: String,
    pub quantity: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctrineRules {
    pub grenade_max_distance: Rule,
    pub grenade_min_enemies: Rule,
    pub grenade_safety_margin: Rule,
    pub medical_hp_threshold: Rule,
    pub medical_safe_seconds: Rule,
    pub medical_cooldown_seconds: Rule,
}

#[cfg(test)]
mod doctrine_serde_tests {
    use super::*;
    #[test]
    fn absent_new_fields_receive_defaults_but_partial_groups_are_rejected() {
        let legacy = r#"{"engagementDistance":8,"fallbackHp":30,"positionPreference":"balanced","routePreference":"fastest"}"#;
        let d: Doctrine = serde_json::from_str(legacy).unwrap();
        assert_eq!(d.grenades.policy, GrenadePolicy::Clusters);
        assert_eq!(d.medical.policy, MedicalPolicy::OutOfContact);
        assert_eq!(d.out_of_ammo_behavior, OutOfAmmoBehavior::Withdraw);
        let partial = r#"{"engagementDistance":8,"fallbackHp":30,"positionPreference":"balanced","routePreference":"fastest","medical":{"policy":"never"}}"#;
        assert!(serde_json::from_str::<Doctrine>(partial).is_err());
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemUseProperties {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blast_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heal_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_threshold: Option<f64>,
}
