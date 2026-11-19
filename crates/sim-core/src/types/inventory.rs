use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct UnitDefinition {
    pub id: String,
    pub name: String,
    pub max_hp: f64,
    pub speed: f64,
    pub eye_height: f64,
    pub muzzle_height: f64,
    pub target_heights: Vec<f64>,
    pub spotting_range: f64,
    pub moving_fire_multiplier: f64,
    pub max_weight_grams: u32,
    pub max_stowed_volume_ml: u32,
    pub default_loadout: Vec<LoadoutEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct WeaponDefinition {
    pub id: String,
    pub range: f64,
    pub damage: f64,
    pub hit_chance: f64,
    pub ammo_item_id: String,
    pub cooldown: f64,
    pub moving_fire_multiplier: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum ItemCategory {
    Weapon,
    Ammo,
    Grenade,
    Tool,
    Medical,
    Ration,
    Water,
}

impl ItemCategory {
    pub const fn consumable(self) -> bool {
        matches!(
            self,
            Self::Ammo | Self::Grenade | Self::Medical | Self::Ration | Self::Water
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub category: ItemCategory,
    pub mass_grams: u32,
    pub volume_ml: u32,
    pub hands_required: u32,
    pub weapon_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct InventoryEntry {
    pub id: u32,
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct ItemUse {
    pub item_id: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
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
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct Sustainment {
    pub nutrition: f64,
    pub hydration: f64,
    pub needs_factor: f64,
    pub nutrition_shortage: bool,
    pub hydration_shortage: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct LoadoutEntry {
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct NamedLoadout {
    pub name: String,
    pub entries: Vec<LoadoutEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts-export",
    derive(ts_rs::TS),
    ts(tag = "kind", rename_all = "camelCase")
)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UnitLoadout {
    Named { name: String },
    Inline { entries: Vec<LoadoutEntry> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum ItemUseKind {
    Grenade,
    Medical,
    Ration,
    Water,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct ItemUseProperties {
    pub kind: ItemUseKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub blast_radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub damage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub cooldown_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub heal_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub restore_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub use_threshold: Option<f64>,
}

impl ItemUseProperties {
    pub const fn new(kind: ItemUseKind) -> Self {
        Self {
            kind,
            blast_radius: None,
            damage: None,
            cooldown_seconds: None,
            heal_amount: None,
            restore_amount: None,
            use_threshold: None,
        }
    }
}
