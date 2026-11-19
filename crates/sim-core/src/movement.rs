//! Executes per-tick unit movement along cached routes and records which units changed position.

use crate::{
    navigation::{RouteRequest, SearchScratch},
    terrain::Terrain,
    types::*,
    unit::Unit,
};

#[cfg(test)]
use crate::terrain::TerrainCache;

#[cfg(test)]
pub(crate) fn execute(units: &mut [Unit], map: &ArenaMap, tick: u32) {
    let cache = TerrainCache::new(map);
    let mut scratch = SearchScratch::new(cache.width, cache.height);
    let mut moved = vec![false; units.len()];
    execute_with(units, cache.view(map), &mut scratch, tick, &mut moved);
}

pub(crate) fn execute_with(
    units: &mut [Unit],
    terrain: Terrain<'_>,
    scratch: &mut SearchScratch,
    tick: u32,
    moved: &mut [bool],
) {
    for (u, has_moved) in units.iter_mut().zip(moved.iter_mut()) {
        *has_moved = false;
        u.nav.movement_multiplier = 0.0;
        if u.vitals.hp <= 0.0 {
            continue;
        }
        let start = u.nav.position;
        if let Some(goal) = u.nav.goal {
            let fallback = u.perception.memory.as_ref().map(|m| m.position);
            let threats = if u.perception.known_threats.is_empty() {
                fallback.as_slice()
            } else {
                u.perception.known_threats.as_slice()
            };
            u.nav.route.prepare_with(
                terrain,
                scratch,
                RouteRequest {
                    from: u.nav.position,
                    goal,
                    threats,
                    preference: u.placement.doctrine.route_preference,
                    revealed: tick < u.combat.revealed_until,
                },
            );
            u.nav.position = u.nav.route.advance_t(
                terrain,
                u.nav.position,
                crate::definitions::STEP * u.inventory.load_factor() * u.needs_factor(),
            );
            if u.combat.behavior == Behavior::Investigating
                && u.nav.position.distance(goal)
                    < crate::definitions::TUNING.advance_arrive_distance
            {
                if let Some(memory) = &mut u.perception.memory {
                    memory.investigated = true;
                }
                u.nav.goal = None;
                u.nav.route.clear();
            }
        } else {
            u.nav.route.clear();
        }
        let dist = u.nav.position.distance(start);
        *has_moved = u.nav.position != start;
        u.nav.movement_multiplier = if dist > 1e-9 {
            dist / (u.def.speed * crate::definitions::STEP)
        } else {
            0.0
        };
        u.combat.tick_cooldowns();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movement_readout_tracks_distance_actually_travelled_in_tick() {
        let map = ArenaMap::flat(5, 5);
        let start = Point { x: 2.5, y: 2.5 };
        let mut u = Unit::new(UnitPlacement {
            id: 1,
            faction: 0,
            definition_id: "infantry".into(),
            position: start,
            doctrine: crate::definitions::catalog().default_doctrine,
            loadout: None,
        });
        u.nav.goal = Some(Point { x: 2.53, y: 2.5 });
        execute(std::slice::from_mut(&mut u), &map, 1);
        assert!((u.nav.position.x - 2.53).abs() < 1e-8);
        assert!((u.nav.movement_multiplier - 0.3).abs() < 1e-8);
    }
    #[test]
    fn terrain_load_and_shortage_factors_compose_in_actual_movement() {
        let mut map = ArenaMap::flat(8, 5);
        map.surfaces.fill(Surface::Mud);
        let mut unit = Unit::new(UnitPlacement {
            id: 1,
            faction: 0,
            definition_id: "infantry".into(),
            position: Point { x: 1.5, y: 2.5 },
            doctrine: crate::definitions::catalog().default_doctrine,
            loadout: None,
        });
        unit.nav.goal = Some(Point { x: 6.5, y: 2.5 });
        let start = unit.nav.position;
        execute(std::slice::from_mut(&mut unit), &map, 1);
        let expected = unit.def.speed * crate::definitions::STEP * 0.6 * 0.816;
        assert!((unit.nav.position.distance(start) - expected).abs() < 1e-8);

        let mut deprived = unit.clone();
        deprived.sustainment.nutrition = 0.0;
        deprived.sustainment.hydration = 0.0;
        let start = unit.nav.position;
        execute(std::slice::from_mut(&mut unit), &map, 2);
        execute(std::slice::from_mut(&mut deprived), &map, 2);
        assert!(
            (deprived.nav.position.distance(start) - unit.nav.position.distance(start) * 0.6).abs()
                < 1e-8
        );

        let mut lighter = unit.clone();
        lighter.inventory.use_item("water").unwrap();
        let start = unit.nav.position;
        execute(std::slice::from_mut(&mut unit), &map, 3);
        execute(std::slice::from_mut(&mut lighter), &map, 3);
        assert!(lighter.nav.position.distance(start) > unit.nav.position.distance(start));
    }
}
