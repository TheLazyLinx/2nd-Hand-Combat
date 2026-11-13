use crate::{item_actions::Action, map, perception, types::*, unit::Unit};
pub(crate) fn execute(units: &mut [Unit], map: &ArenaMap, rng: &mut u64, tick: u32) {
    let observations: Vec<_> = (0..units.len())
        .map(|i| {
            if units[i].hp > 0.0 {
                perception::observe_all(units, map, i, tick)
            } else {
                vec![]
            }
        })
        .collect();
    for (u, seen) in units.iter_mut().zip(&observations) {
        if !seen.is_empty() {
            u.last_contact = tick;
        }
    }
    let actions = crate::item_actions::select(units, map, tick, &observations);
    let mut damage = vec![0.0; units.len()];
    let mut heals = vec![false; units.len()];
    for (i, action) in actions.into_iter().enumerate() {
        match action {
            Some(Action::Medical) => {
                units[i]
                    .inventory
                    .use_item("medical-supply")
                    .expect("selected item remains usable");
                units[i].last_medical = Some(tick);
                heals[i] = true;
            }
            Some(Action::Water) => {
                units[i]
                    .inventory
                    .use_item("water")
                    .expect("selected item remains usable");
                units[i].hydration =
                    (units[i].hydration + crate::consumables::NEED_RESTORE).min(100.0);
            }
            Some(Action::Ration) => {
                units[i]
                    .inventory
                    .use_item("ration")
                    .expect("selected item remains usable");
                units[i].nutrition =
                    (units[i].nutrition + crate::consumables::NEED_RESTORE).min(100.0);
            }
            Some(Action::Grenade(j)) => {
                let target = units[j].position;
                units[i]
                    .inventory
                    .use_item("frag-grenade")
                    .expect("selected item remains usable");
                units[i].cooldown = (crate::consumables::GRENADE_COOLDOWN_SECONDS
                    / crate::definitions::STEP) as u32;
                units[i].revealed_until = tick + crate::definitions::REVEAL_TICKS;
                for (k, v) in units.iter().enumerate() {
                    if v.hp > 0.0
                        && v.position.distance(target) <= crate::consumables::BLAST_RADIUS
                        && map::visible(map, target, v.position, 0.1, 1.0)
                    {
                        damage[k] += crate::consumables::GRENADE_DAMAGE;
                    }
                }
            }
            Some(Action::Rifle(j, exposure, weapon)) => {
                units[i]
                    .inventory
                    .consume(&weapon.ammo_item_id)
                    .expect("selected ammunition remains available");
                if units[i].inventory.count(&weapon.ammo_item_id) == 0 {
                    units[i].needs_decision = true;
                }
                units[i].shots += 1;
                units[i].cooldown = (weapon.cooldown / crate::definitions::STEP) as u32;
                units[i].revealed_until = tick + crate::definitions::REVEAL_TICKS;
                *rng ^= *rng << 13;
                *rng ^= *rng >> 7;
                *rng ^= *rng << 17;
                let roll = (*rng >> 11) as f64 / ((1_u64 << 53) as f64);
                if roll < weapon.hit_chance * exposure {
                    damage[j] += weapon.damage;
                }
            }
            None => {}
        }
    }
    for ((u, d), heal) in units.iter_mut().zip(damage).zip(heals) {
        let was_low = u.is_low_hp();
        if d > 0.0 {
            u.last_contact = tick;
        }
        u.hp = (u.hp - d).max(0.0);
        if u.hp > 0.0 && heal {
            u.hp = (u.hp + crate::consumables::MEDICAL_HEAL).min(100.0);
        }
        if was_low != u.is_low_hp() {
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
    #[test]
    fn grenade_cluster_damages_visible_enemies_and_spends_one() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5), soldier(3, 7.0)];
        units[2].placement.faction = 1;
        units[1].cooldown = 10;
        units[2].cooldown = 10;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].inventory.count("frag-grenade"), 1);
        assert_eq!((units[1].hp, units[2].hp), (60.0, 60.0));
        assert_eq!(units[0].inventory.count("rifle-ammo"), 60);
    }
    #[test]
    fn dead_soldier_neither_progresses_needs_nor_consumes_supplies() {
        let map = ArenaMap::flat(16, 6);
        let mut dead = soldier(1, 2.5);
        dead.hp = 0.0;
        dead.nutrition = 50.0;
        dead.hydration = 50.0;
        let before = dead.inventory.snapshot();
        crate::sustainment::advance(&mut dead);
        execute(std::slice::from_mut(&mut dead), &map, &mut 2, 1);
        assert_eq!(dead.inventory.snapshot(), before);
        assert_eq!((dead.nutrition, dead.hydration, dead.hp), (50.0, 50.0, 0.0));
    }
    #[test]
    fn grenade_safety_checks_all_allies_even_when_cluster_exists() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![
            soldier(1, 2.5),
            soldier(2, 6.5),
            soldier(3, 7.0),
            soldier(4, 7.5),
        ];
        units[2].placement.faction = 1;
        units[3].placement.faction = 0;
        for u in &mut units[1..] {
            u.cooldown = 10;
        }
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].inventory.count("frag-grenade"), 2);
        assert_eq!(units[0].inventory.count("rifle-ammo"), 59);
    }
    #[test]
    fn needs_action_precedes_a_valid_shot() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        units[0].hydration = 50.0;
        units[1].cooldown = 10;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].hydration, 90.0);
        assert_eq!(units[0].inventory.count("water"), 1);
        assert_eq!(units[0].shots, 0);
    }
    #[test]
    fn safe_medical_use_restores_health_and_interrupts_fallback() {
        let map = ArenaMap::flat(32, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 29.5)];
        units[0].hp = 20.0;
        execute(&mut units, &map, &mut 2, 100);
        assert_eq!(units[0].hp, 45.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
        assert!(units[0].needs_decision);
    }
    #[test]
    fn empty_ammo_never_advances_rng_or_fires() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        for _ in 0..60 {
            units[0].inventory.consume("rifle-ammo").unwrap();
        }
        units[0].placement.doctrine.grenades.policy = GrenadePolicy::Never;
        units[1].cooldown = 10;
        let mut rng = 2;
        execute(&mut units, &map, &mut rng, 1);
        assert_eq!(rng, 2);
        assert_eq!(units[0].shots, 0);
        assert_eq!(units[1].hp, 100.0);
    }
    #[test]
    fn solid_cover_blocks_blast_to_hidden_victim() {
        let mut map = ArenaMap::flat(16, 6);
        map.obstacles.push(Obstacle {
            x: 7,
            y: 2,
            width: 1,
            height: 1,
            height_metres: 2.4,
        });
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.9), soldier(3, 8.1)];
        units[2].placement.faction = 1;
        units[0].placement.doctrine.grenades.policy = GrenadePolicy::AnySafeTarget;
        units[1].cooldown = 10;
        units[2].cooldown = 10;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[1].hp, 60.0);
        assert_eq!(units[2].hp, 100.0);
    }
    #[test]
    fn committed_medical_supply_is_spent_if_simultaneous_shot_kills() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 8.5)];
        units[0].hp = 20.0;
        units[0].revealed_until = 100;
        units[0].placement.doctrine.medical.safe_seconds = 0.0;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].hp, 0.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
        assert_eq!(units[1].shots, 1);
    }
    #[test]
    fn low_health_treats_under_fire_but_out_of_contact_waits() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        units[0].hp = 40.0;
        units[0].placement.doctrine.medical.policy = MedicalPolicy::LowHealth;
        units[1].cooldown = 10;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].hp, 65.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
        units[0].hp = 40.0;
        units[0].last_medical = None;
        units[0].placement.doctrine.medical.policy = MedicalPolicy::OutOfContact;
        execute(&mut units, &map, &mut 2, 2);
        assert_eq!(units[0].hp, 40.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
    }
    #[test]
    fn out_of_contact_does_not_treat_above_threshold_and_obeys_cooldown() {
        let map = ArenaMap::flat(32, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 29.5)];
        units[0].hp = 51.0;
        execute(&mut units, &map, &mut 2, 100);
        assert_eq!(units[0].inventory.count("medical-supply"), 2);
        units[0].hp = 40.0;
        execute(&mut units, &map, &mut 2, 101);
        assert_eq!(units[0].hp, 65.0);
        units[0].hp = 40.0;
        execute(&mut units, &map, &mut 2, 102);
        assert_eq!(units[0].hp, 40.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
    }
    #[test]
    fn blast_reaches_unseen_victim_through_vegetation() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5), soldier(3, 7.5)];
        units[2].placement.faction = 1;
        units[0].placement.doctrine.grenades.policy = GrenadePolicy::AnySafeTarget;
        units[1].cooldown = 10;
        units[2].cooldown = 10;
        assert_eq!(perception::observe_all(&units, &map, 0, 1), vec![1]);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[1].hp, 60.0);
        assert_eq!(units[2].hp, 60.0);
    }
    #[test]
    fn grenade_and_rifle_resolve_simultaneously() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        units[0].hp = 20.0;
        units[1].hp = 40.0;
        units[0].placement.doctrine.grenades.policy = GrenadePolicy::AnySafeTarget;
        units[1].placement.doctrine.grenades.policy = GrenadePolicy::Never;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!((units[0].hp, units[1].hp), (0.0, 0.0));
        assert_eq!(units[0].inventory.count("frag-grenade"), 1);
        assert_eq!(units[1].shots, 1);
    }
}
