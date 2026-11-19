use serde::{Deserialize, Serialize};

use super::{InventorySnapshot, Point, Surface, Sustainment};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum Behavior {
    Searching,
    Investigating,
    Advancing,
    Engaging,
    FallingBack,
    Defending,
    Dead,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct TerrainStatus {
    pub surface: Surface,
    pub elevation: f64,
    pub movement_multiplier: f64,
    pub revealed: bool,
    pub protection: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
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
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
pub struct FactionResult {
    pub faction: u32,
    pub survivors: u32,
    pub casualties: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts-export",
    derive(ts_rs::TS),
    ts(rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum ResultReason {
    Elimination,
    SimultaneousElimination,
    Timeout,
    Stopped,
}

impl ResultReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Elimination => "elimination",
            Self::SimultaneousElimination => "simultaneous_elimination",
            Self::Timeout => "timeout",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
pub struct BattleResult {
    pub winner: Option<u32>,
    pub reason: ResultReason,
    pub elapsed: f64,
    pub factions: Vec<FactionResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
pub struct Snapshot {
    pub tick: u32,
    pub elapsed: f64,
    pub units: Vec<UnitSnapshot>,
    pub result: Option<BattleResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct UnitInventoryUpdate {
    pub unit_id: u32,
    pub version: u32,
    pub inventory: InventorySnapshot,
}
