use crate::{perception::Unit, types::*};
pub(crate) fn execute(units: &mut [Unit], map: &ArenaMap, tick: u32) {
    for u in units {
        if u.hp <= 0.0 {
            continue;
        }
        let start = u.position;
        if let Some(goal) = u.goal {
            let fallback = u.memory.as_ref().map(|m| m.position);
            let threats = if u.known_threats.is_empty() {
                fallback.as_slice()
            } else {
                u.known_threats.as_slice()
            };
            u.route.prepare(
                map,
                u.position,
                goal,
                threats,
                u.placement.doctrine.route_preference,
                tick < u.revealed_until,
            );
            u.position = u.route.advance(map, u.position, crate::definitions::STEP);
            if u.behavior == Behavior::Investigating && u.position.distance(goal) < 0.2 {
                if let Some(memory) = &mut u.memory {
                    memory.investigated = true;
                }
                u.goal = None;
                u.route.clear();
            }
        } else {
            u.route.clear();
        }
        u.movement_multiplier = if u.position.distance(start) > 1e-9 {
            u.position.distance(start) / (crate::definitions::SPEED * crate::definitions::STEP)
        } else {
            0.0
        };
        if u.cooldown > 0 {
            u.cooldown -= 1;
        }
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
        });
        u.goal = Some(Point { x: 2.53, y: 2.5 });
        execute(std::slice::from_mut(&mut u), &map, 1);
        assert!((u.position.x - 2.53).abs() < 1e-8);
        assert!((u.movement_multiplier - 0.3).abs() < 1e-8);
    }
}
