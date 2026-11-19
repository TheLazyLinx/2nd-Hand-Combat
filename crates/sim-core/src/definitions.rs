//! Game-wide catalog definitions: surfaces, map presets, unit specs, weapon specs,
//! default factions, doctrine rules, and centralized simulation tuning constants.
//!
//! Invariant: base materials (units, weapons, items, surfaces) are defined game-wide here
//! and referenced by compact static IDs in simulation loops; scenarios only compose loadouts.

use crate::{
    inventory::{
        item_spec, ItemId, ITEM_ENTRENCHING_TOOL, ITEM_FRAG_GRENADE, ITEM_MEDICAL_SUPPLY,
        ITEM_RATION, ITEM_RIFLE, ITEM_RIFLE_AMMO, ITEM_WATER,
    },
    map,
    types::*,
};

pub(crate) const STEP: f64 = 0.05;
pub(crate) const REVEAL_TICKS: u32 = 40;
pub(crate) const LOW_OBSTACLE_HEIGHT: f64 = 0.9;
pub(crate) const FULL_OBSTACLE_HEIGHT: f64 = 2.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct WeaponId(pub(crate) usize);

pub(crate) const WEAPON_RIFLE: WeaponId = WeaponId(0);

pub(crate) struct WeaponSpec {
    pub id: &'static str,
    pub range: f64,
    pub damage: f64,
    pub hit_chance: f64,
    pub ammo_item: ItemId,
    pub cooldown: f64,
    pub moving_fire_multiplier: f64,
}

const WEAPONS: [WeaponSpec; 1] = [WeaponSpec {
    id: "rifle",
    range: 12.0,
    damage: 20.0,
    hit_chance: 0.7,
    ammo_item: ITEM_RIFLE_AMMO,
    cooldown: 1.0,
    moving_fire_multiplier: 1.0,
}];

#[inline]
pub(crate) fn weapon_spec(id: WeaponId) -> &'static WeaponSpec {
    &WEAPONS[id.0]
}

pub(crate) struct UnitSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub max_hp: f64,
    pub speed: f64,
    pub eye_height: f64,
    pub muzzle_height: f64,
    pub target_heights: [f64; 3],
    pub spotting_range: f64,
    pub moving_fire_multiplier: f64,
    pub max_weight_grams: u32,
    pub max_stowed_volume_ml: u32,
    pub default_loadout: &'static [(ItemId, u32)],
}

const INFANTRY_DEFAULT_LOADOUT: &[(ItemId, u32)] = &[
    (ITEM_RIFLE, 1),
    (ITEM_RIFLE_AMMO, 60),
    (ITEM_FRAG_GRENADE, 2),
    (ITEM_ENTRENCHING_TOOL, 1),
    (ITEM_MEDICAL_SUPPLY, 2),
    (ITEM_RATION, 2),
    (ITEM_WATER, 2),
];

pub(crate) const INFANTRY_SPEC: UnitSpec = UnitSpec {
    id: "infantry",
    name: "Infantry",
    max_hp: 100.0,
    speed: 2.0,
    eye_height: 1.6,
    muzzle_height: 1.4,
    target_heights: [0.4, 0.9, 1.4],
    spotting_range: 14.0,
    moving_fire_multiplier: 1.0,
    max_weight_grams: 20_000,
    max_stowed_volume_ml: 15_000,
    default_loadout: INFANTRY_DEFAULT_LOADOUT,
};

const UNIT_SPECS: [&UnitSpec; 1] = [&INFANTRY_SPEC];

pub(crate) fn unit_spec(id: &str) -> Option<&'static UnitSpec> {
    UNIT_SPECS.iter().copied().find(|spec| spec.id == id)
}

#[inline]
pub(crate) fn default_unit_spec() -> &'static UnitSpec {
    &INFANTRY_SPEC
}

// Re-export infantry defaults for geometry/tests that reference baseline unit metrics.
pub(crate) const SPEED: f64 = INFANTRY_SPEC.speed;
#[cfg(test)]
pub(crate) const DAMAGE: f64 = WEAPONS[0].damage;

/// Centralized simulation tuning constants for decision cadence, tactical radii, sustainment,
/// and consumables. Keeps gameplay parameters in one place rather than scattered literals.
pub(crate) struct Tuning {
    pub decision_interval_ticks: u32,
    pub memory_expiry_ticks: u32,
    pub threat_moved_distance: f64,
    pub score_tolerance: f64,
    pub advance_arrive_distance: f64,
    pub fallback_arrive_distance: f64,
    pub fallback_distance_margin: f64,
    pub patrol_arrive_distance: f64,
    pub reposition_radius: f64,
    pub fallback_radius: f64,
    pub investigate_max_radius: f64,
    pub investigate_max_cost: f64,
    pub nutrition_drain_per_second: f64,
    pub hydration_drain_per_second: f64,
    pub shortage_threshold: f64,
    pub nutrition_shortage_penalty: f64,
    pub hydration_shortage_penalty: f64,
    pub blast_radius: f64,
    pub grenade_damage: f64,
    pub grenade_cooldown_seconds: f64,
    pub medical_heal: f64,
    pub need_use_threshold: f64,
    pub need_restore: f64,
}

pub(crate) const TUNING: Tuning = Tuning {
    decision_interval_ticks: 10,
    memory_expiry_ticks: 200,
    threat_moved_distance: 2.0,
    score_tolerance: 0.01,
    advance_arrive_distance: 0.2,
    fallback_arrive_distance: 0.3,
    fallback_distance_margin: 0.25,
    patrol_arrive_distance: 0.4,
    reposition_radius: 3.0,
    fallback_radius: 5.0,
    investigate_max_radius: 24.0,
    investigate_max_cost: 40.0,
    nutrition_drain_per_second: 0.5,
    hydration_drain_per_second: 1.0,
    shortage_threshold: 25.0,
    nutrition_shortage_penalty: 0.15,
    hydration_shortage_penalty: 0.25,
    blast_radius: 1.5,
    grenade_damage: 40.0,
    grenade_cooldown_seconds: 3.0,
    medical_heal: 25.0,
    need_use_threshold: 50.0,
    need_restore: 40.0,
};

pub fn default_factions() -> Vec<FactionDefinition> {
    vec![
        FactionDefinition {
            id: 0,
            name: "North".into(),
            color: 0x315f62,
            patrol_direction: PatrolDirection::Forward,
        },
        FactionDefinition {
            id: 1,
            name: "South".into(),
            color: 0xa24f32,
            patrol_direction: PatrolDirection::Reverse,
        },
    ]
}

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

fn weapon_definitions() -> Vec<WeaponDefinition> {
    WEAPONS
        .iter()
        .map(|w| WeaponDefinition {
            id: w.id.into(),
            range: w.range,
            damage: w.damage,
            hit_chance: w.hit_chance,
            ammo_item_id: item_spec(w.ammo_item).id.into(),
            cooldown: w.cooldown,
            moving_fire_multiplier: w.moving_fire_multiplier,
        })
        .collect()
}

fn unit_definitions() -> Vec<UnitDefinition> {
    UNIT_SPECS
        .iter()
        .map(|u| UnitDefinition {
            id: u.id.into(),
            name: u.name.into(),
            max_hp: u.max_hp,
            speed: u.speed,
            eye_height: u.eye_height,
            muzzle_height: u.muzzle_height,
            target_heights: u.target_heights.to_vec(),
            spotting_range: u.spotting_range,
            moving_fire_multiplier: u.moving_fire_multiplier,
            max_weight_grams: u.max_weight_grams,
            max_stowed_volume_ml: u.max_stowed_volume_ml,
            default_loadout: u
                .default_loadout
                .iter()
                .map(|&(item, quantity)| LoadoutEntry {
                    item_id: item_spec(item).id.into(),
                    quantity,
                })
                .collect(),
        })
        .collect()
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
            let cache = crate::terrain::TerrainCache::without_neighbors(m);
            let terrain = cache.view(m);
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
                            traversable: !map::blocked_t(terrain, p),
                        }
                    })
                })
                .collect()
        })
        .collect();
    Catalog {
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
        units: unit_definitions(),
        weapons: weapon_definitions(),
        default_factions: default_factions(),
        default_doctrine: Doctrine {
            engagement_distance: 8.0,
            fallback_hp: 30.0,
            position_preference: PositionPreference::Balanced,
            route_preference: RoutePreference::Fastest,
            grenades: GrenadeDoctrine::default(),
            medical: MedicalDoctrine::default(),
            out_of_ammo_behavior: OutOfAmmoBehavior::default(),
            fire_while_moving: false,
        },
        items: crate::inventory::items(),
        item_use_properties: crate::consumables::catalog(),
        default_inventory: crate::inventory::Inventory::default_infantry().snapshot(),
        doctrine_rules: doctrine_rules(),
        max_per_faction: crate::validation::SKIRMISH_MAX_PER_FACTION,
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
