use crate::types::*;
pub(crate) const STEP: f64 = 0.05;
pub(crate) const MAX_HP: f64 = 100.0;
pub(crate) const SPEED: f64 = 2.0;
pub(crate) const RANGE: f64 = 12.0;
pub(crate) const DAMAGE: f64 = 20.0;
pub(crate) const HIT_CHANCE: f64 = 0.7;
pub(crate) const COOLDOWN: f64 = 1.0;

pub fn catalog() -> Catalog {
    let walls = (4..16)
        .filter(|y| *y != 9 && *y != 10)
        .map(|y| Point {
            x: 15.0,
            y: y as f64,
        })
        .collect();
    let cover = vec![
        Point { x: 10.0, y: 6.0 },
        Point { x: 10.0, y: 13.0 },
        Point { x: 21.0, y: 6.0 },
        Point { x: 21.0, y: 13.0 },
    ];
    Catalog {
        map: ArenaMap {
            width: 32,
            height: 20,
            walls,
            cover,
            patrol: vec![
                Point { x: 5.5, y: 5.5 },
                Point { x: 12.5, y: 5.5 },
                Point { x: 19.5, y: 14.5 },
                Point { x: 26.5, y: 14.5 },
            ],
        },
        unit: UnitDefinition {
            id: "infantry".into(),
            name: "Infantry".into(),
            max_hp: MAX_HP,
            speed: SPEED,
        },
        weapon: WeaponDefinition {
            id: "rifle".into(),
            range: RANGE,
            damage: DAMAGE,
            hit_chance: HIT_CHANCE,
            cooldown: COOLDOWN,
        },
        default_doctrine: Doctrine {
            engagement_distance: 8.0,
            fallback_hp: 30.0,
        },
        max_per_faction: 8,
        default_time_limit: 180.0,
        default_seed: 1,
    }
}
