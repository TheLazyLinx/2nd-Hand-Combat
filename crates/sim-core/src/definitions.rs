use crate::{map, types::*};
pub(crate) const STEP: f64 = 0.05;
pub(crate) const MAX_HP: f64 = 100.0;
pub(crate) const SPEED: f64 = 2.0;
pub(crate) const RANGE: f64 = 12.0;
pub(crate) const DAMAGE: f64 = 20.0;
pub(crate) const HIT_CHANCE: f64 = 0.7;
pub(crate) const COOLDOWN: f64 = 1.0;
pub(crate) const REVEAL_TICKS: u32 = 40;
pub(crate) const LOW_OBSTACLE_HEIGHT: f64 = 0.9;
pub(crate) const FULL_OBSTACLE_HEIGHT: f64 = 2.4;
pub fn surface_definition(id: Surface) -> &'static SurfaceDefinition {
    static DEFS: std::sync::OnceLock<Vec<SurfaceDefinition>> = std::sync::OnceLock::new();
    let defs = DEFS.get_or_init(|| {
        [
            Surface::Open,
            Surface::Road,
            Surface::Mud,
            Surface::Brush,
            Surface::Woodland,
        ]
        .into_iter()
        .map(|id| {
            let (name, movement_multiplier, spotting_multiplier, vegetation_height, color) =
                match id {
                    Surface::Open => ("Open", 1.0, 1.0, 0.0, 0x91a873),
                    Surface::Road => ("Road", 1.15, 1.0, 0.0, 0xc0ac83),
                    Surface::Mud => ("Mud", 0.6, 1.0, 0.0, 0x796b51),
                    Surface::Brush => ("Brush", 0.8, 2.0, 1.8, 0x71894e),
                    Surface::Woodland => ("Woodland", 0.7, 3.0, 3.0, 0x3e6845),
                };
            SurfaceDefinition {
                id,
                name: name.into(),
                movement_multiplier,
                spotting_multiplier,
                vegetation_height,
                color,
            }
        })
        .collect()
    });
    &defs[match id {
        Surface::Open => 0,
        Surface::Road => 1,
        Surface::Mud => 2,
        Surface::Brush => 3,
        Surface::Woodland => 4,
    }]
}
fn base() -> ArenaMap {
    ArenaMap {
        width: 32,
        height: 20,
        surfaces: vec![Surface::Open; 640],
        elevations: vec![0.0; 693],
        obstacles: vec![],
        patrol: vec![
            Point { x: 5.5, y: 5.5 },
            Point { x: 12.5, y: 5.5 },
            Point { x: 19.5, y: 14.5 },
            Point { x: 26.5, y: 14.5 },
        ],
    }
}
fn paint(m: &mut ArenaMap, x0: usize, y0: usize, x1: usize, y1: usize, s: Surface) {
    for y in y0..y1 {
        for x in x0..x1 {
            m.surfaces[y * 32 + x] = s;
        }
    }
}
fn obstacle(m: &mut ArenaMap, x: u32, y: u32, height_metres: f64) {
    m.obstacles.push(Obstacle {
        x,
        y,
        width: 1,
        height: 1,
        height_metres,
    });
}
fn presets() -> Vec<MapPreset> {
    let mut training = base();
    paint(&mut training, 0, 9, 32, 11, Surface::Road);
    for y in (4..16).filter(|y| *y != 9 && *y != 10) {
        obstacle(&mut training, 15, y, FULL_OBSTACLE_HEIGHT);
    }
    for (x, y) in [(10, 6), (10, 13), (21, 6), (21, 13)] {
        obstacle(&mut training, x, y, LOW_OBSTACLE_HEIGHT);
    }
    let mut ridge = base();
    for y in 0..=20 {
        for x in 0..=32 {
            let ridge_height = (1.0 - ((x as f64 - 16.0).abs() / 8.0)).max(0.0) * 2.4;
            let hollow = (1.0
                - (((x as f64 - 7.0).powi(2) + (y as f64 - 13.0).powi(2)).sqrt() / 5.0))
                .max(0.0)
                * 2.2;
            ridge.elevations[y * 33 + x] = ridge_height - hollow;
        }
    }
    paint(&mut ridge, 0, 9, 32, 11, Surface::Road);
    paint(&mut ridge, 18, 3, 25, 7, Surface::Brush);
    for (x, y) in [(12, 6), (20, 14)] {
        obstacle(&mut ridge, x, y, LOW_OBSTACLE_HEIGHT);
    }
    let mut woodland = base();
    paint(&mut woodland, 5, 2, 13, 9, Surface::Woodland);
    paint(&mut woodland, 19, 11, 27, 18, Surface::Woodland);
    paint(&mut woodland, 7, 12, 14, 17, Surface::Brush);
    paint(&mut woodland, 18, 3, 25, 8, Surface::Brush);
    paint(&mut woodland, 0, 9, 32, 11, Surface::Road);
    paint(&mut woodland, 14, 0, 18, 9, Surface::Mud);
    paint(&mut woodland, 14, 11, 18, 20, Surface::Mud);
    for (x, y) in [(11, 6), (21, 13)] {
        obstacle(&mut woodland, x, y, LOW_OBSTACLE_HEIGHT);
    }
    vec![
        MapPreset {
            id: "training".into(),
            name: "Training Ground".into(),
            description: "A road crosses a central wall with low obstacles.".into(),
            map: training,
        },
        MapPreset {
            id: "ridge".into(),
            name: "Ridge and Hollow".into(),
            description: "A gradual ridge and a depression change sight and travel.".into(),
            map: ridge,
        },
        MapPreset {
            id: "woodland".into(),
            name: "Woodland Crossing".into(),
            description: "Woodland and brush flank a road and muddy crossing.".into(),
            map: woodland,
        },
    ]
}
pub fn editor_rules() -> EditorRules {
    EditorRules {
        min_dimension: 1,
        max_dimension: 128,
        min_elevation: -100.0,
        max_elevation: 100.0,
        max_obstacle_height: 100.0,
    }
}
pub(crate) fn rifle() -> WeaponDefinition {
    WeaponDefinition {
        id: "rifle".into(),
        range: RANGE,
        damage: DAMAGE,
        hit_chance: HIT_CHANCE,
        ammo_item_id: "rifle-ammo".into(),
        cooldown: COOLDOWN,
    }
}
pub(crate) fn weapon(id: &str) -> Option<WeaponDefinition> {
    (id == "rifle").then(rifle)
}
fn rule(min: f64, max: f64, step: f64) -> Rule {
    Rule { min, max, step }
}
pub(crate) fn doctrine_rules() -> DoctrineRules {
    DoctrineRules {
        grenade_max_distance: rule(2.0, 6.0, 0.5),
        grenade_min_enemies: rule(2.0, 8.0, 1.0),
        grenade_safety_margin: rule(0.0, 2.0, 0.25),
        medical_hp_threshold: rule(1.0, 99.0, 1.0),
        medical_safe_seconds: rule(0.0, 30.0, 1.0),
        medical_cooldown_seconds: rule(1.0, 60.0, 1.0),
    }
}
pub fn catalog() -> Catalog {
    let maps = presets();
    let terrain_cells = maps
        .iter()
        .map(|preset| {
            let m = &preset.map;
            (0..m.height)
                .flat_map(|y| {
                    (0..m.width).map(move |x| {
                        let p = Point {
                            x: x as f64 + 0.5,
                            y: y as f64 + 0.5,
                        };
                        TerrainCellInfo {
                            surface: map::surface(m, p),
                            elevation: map::ground_height(m, p),
                            movement_multiplier: map::movement_multiplier(m, p, p),
                            traversable: !map::blocked(m, p),
                        }
                    })
                })
                .collect()
        })
        .collect();
    Catalog {
        map: maps[0].map.clone(),
        editor_rules: editor_rules(),
        maps,
        surfaces: [
            Surface::Open,
            Surface::Road,
            Surface::Mud,
            Surface::Brush,
            Surface::Woodland,
        ]
        .into_iter()
        .map(|s| surface_definition(s).clone())
        .collect(),
        low_obstacle_height: LOW_OBSTACLE_HEIGHT,
        full_obstacle_height: FULL_OBSTACLE_HEIGHT,
        terrain_cells,
        unit: UnitDefinition {
            id: "infantry".into(),
            name: "Infantry".into(),
            max_hp: MAX_HP,
            speed: SPEED,
            max_weight_grams: 20000,
            max_stowed_volume_ml: 15000,
        },
        weapon: rifle(),
        default_doctrine: Doctrine {
            engagement_distance: 8.0,
            fallback_hp: 30.0,
            position_preference: PositionPreference::Balanced,
            route_preference: RoutePreference::Fastest,
            grenades: GrenadeDoctrine::default(),
            medical: MedicalDoctrine::default(),
            out_of_ammo_behavior: OutOfAmmoBehavior::default(),
        },
        items: crate::inventory::items(),
        item_use_properties: crate::consumables::catalog(),
        weapons: vec![rifle()],
        default_loadout: crate::inventory::loadout(),
        default_inventory: crate::inventory::Inventory::new().snapshot(),
        doctrine_rules: doctrine_rules(),
        max_per_faction: 8,
        default_time_limit: 180.0,
        default_seed: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_connect_opposite_sides() {
        for preset in catalog().maps {
            for point in &preset.map.patrol {
                assert!(
                    !map::blocked(&preset.map, *point),
                    "{} has a blocked patrol point",
                    preset.name
                );
            }
            let search = crate::navigation::Search::new(
                &preset.map,
                Point { x: 2.5, y: 10.5 },
                None,
                RoutePreference::Fastest,
            );
            assert!(
                search.cost(Point { x: 29.5, y: 10.5 }).is_some(),
                "{}",
                preset.name
            );
        }
    }
}
