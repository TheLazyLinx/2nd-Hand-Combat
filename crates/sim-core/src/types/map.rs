use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub(crate) fn distance(self, other: Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub enum Surface {
    Open,
    Road,
    Mud,
    Brush,
    Woodland,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct Obstacle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub height_metres: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
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
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct MapPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub map: ArenaMap,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct TerrainCellInfo {
    pub surface: Surface,
    pub elevation: f64,
    pub movement_multiplier: f64,
    pub traversable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(rename_all = "camelCase"))]
#[serde(rename_all = "camelCase")]
pub struct EditorRules {
    pub min_dimension: u32,
    pub max_dimension: u32,
    pub min_elevation: f64,
    pub max_elevation: f64,
    pub max_obstacle_height: f64,
}
