use crate::{map, perception::Unit, types::*};
pub(crate) fn execute(units: &mut [Unit], map: &ArenaMap, rng: &mut u64, tick: u32) {
    let mut damage = vec![0.0; units.len()];
    // Reveal changes apply after every shooter has checked the same pre-fire state.
    let mut fired = vec![false; units.len()];
    for i in 0..units.len() {
        if units[i].hp <= 0.0
            || units[i].cooldown > 0
            || !matches!(
                units[i].behavior,
                Behavior::Engaging | Behavior::FallingBack | Behavior::Defending
            )
        {
            continue;
        }
        let Some(id) = units[i].target else {
            continue;
        };
        let Some(j) = units
            .iter()
            .position(|u| u.placement.id == id && u.hp > 0.0)
        else {
            continue;
        };
        if units[i].position.distance(units[j].position) > crate::definitions::RANGE {
            continue;
        }
        let exposure = map::exposure(
            map,
            units[i].position,
            units[j].position,
            tick < units[j].revealed_until,
        );
        if exposure == 0.0 {
            continue;
        }
        units[i].shots += 1;
        fired[i] = true;
        units[i].cooldown = (crate::definitions::COOLDOWN / crate::definitions::STEP) as u32;
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        *rng ^= *rng << 17;
        let roll = (*rng >> 11) as f64 / ((1_u64 << 53) as f64);
        if roll < crate::definitions::HIT_CHANCE * exposure {
            damage[j] += crate::definitions::DAMAGE;
        }
    }
    for ((u, d), fired) in units.iter_mut().zip(damage).zip(fired) {
        if fired {
            u.revealed_until = tick + crate::definitions::REVEAL_TICKS;
        }
        let was_low = u.is_low_hp();
        u.hp = (u.hp - d).max(0.0);
        if !was_low && u.is_low_hp() {
            u.needs_decision = true;
        }
        if u.hp == 0.0 {
            u.behavior = Behavior::Dead;
            u.target = None;
            u.goal = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog, movement, perception};

    fn soldier(id: u32, x: f64) -> Unit {
        let mut unit = Unit::new(UnitPlacement {
            id,
            faction: id - 1,
            definition_id: "infantry".into(),
            position: Point { x, y: 2.5 },
            doctrine: catalog().default_doctrine,
        });
        unit.behavior = Behavior::Engaging;
        unit.target = Some(if id == 1 { 2 } else { 1 });
        unit
    }
    #[test]
    fn firing_reveal_does_not_change_other_shooters_visibility_in_the_same_tick() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 8.5)];
        units[1].revealed_until = 100;
        let mut rng = 2;
        execute(&mut units, &map, &mut rng, 1);
        assert_eq!(units[0].shots, 1);
        assert_eq!(units[1].shots, 0);
        execute(&mut units, &map, &mut rng, 2);
        assert_eq!(units[1].shots, 1);
    }
    #[test]
    fn firing_reveal_expires_on_simulation_tick_and_never_bypasses_solid_cover() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 8.5)];
        units[1].revealed_until = 41;
        assert_eq!(perception::observe(&units, &map, 0, 40), Some(1));
        assert_eq!(perception::observe(&units, &map, 0, 41), None);
        map.obstacles.push(Obstacle {
            x: 5,
            y: 2,
            width: 1,
            height: 1,
            height_metres: 2.4,
        });
        assert_eq!(perception::observe(&units, &map, 0, 40), None);
    }
    #[test]
    fn movement_into_concealment_prevents_a_stale_target_from_being_shot() {
        let mut map = ArenaMap::flat(16, 6);
        for x in 5..10 {
            map.surfaces[2 * 16 + x] = Surface::Woodland;
        }
        let mut units = vec![soldier(1, 2.5), soldier(2, 8.83)];
        assert_eq!(perception::observe(&units, &map, 0, 1), Some(1));
        units[1].goal = Some(Point { x: 9.5, y: 2.5 });
        units[1].behavior = Behavior::Advancing;
        movement::execute(&mut units, &map, 1);
        assert!(units[1].position.x > 8.83);
        assert_eq!(perception::observe(&units, &map, 0, 1), None);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].shots, 0);
        assert_eq!(units[1].hp, 100.0);
    }
    #[test]
    fn mutually_visible_lethal_shots_are_resolved_simultaneously() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        for u in &mut units {
            u.hp = crate::definitions::DAMAGE;
        }
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].hp, 0.0);
        assert_eq!(units[1].hp, 0.0);
        assert_eq!(units[0].shots, 1);
        assert_eq!(units[1].shots, 1);
    }
}
