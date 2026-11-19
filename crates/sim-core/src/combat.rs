//! Post-movement perception refresh, per-tick item/weapon action execution, and simultaneous damage.

use crate::{
    battle::StepTimer, item_actions::Action, map, perception, terrain::Terrain, types::*,
    unit::Unit,
};

#[cfg(test)]
use crate::terrain::TerrainCache;

#[cfg(test)]
pub(crate) fn execute(units: &mut [Unit], map: &ArenaMap, rng: &mut u64, tick: u32) {
    let cache = TerrainCache::without_neighbors(map);
    let terrain = cache.view(map);
    let pre = vec![Vec::new(); units.len()];
    let moved = vec![true; units.len()];
    let mut post = vec![Vec::new(); units.len()];
    execute_with_context(
        units,
        terrain,
        rng,
        tick,
        &pre,
        &moved,
        &mut post,
        &mut crate::battle::NoopTimer,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_with_context<T: StepTimer>(
    units: &mut [Unit],
    terrain: Terrain<'_>,
    rng: &mut u64,
    tick: u32,
    pre_observations: &[Vec<usize>],
    moved: &[bool],
    observations: &mut [Vec<usize>],
    timer: &mut T,
) {
    perception::refresh_post_movement(units, terrain, tick, pre_observations, moved, observations);
    for (u, seen) in units.iter_mut().zip(observations.iter()) {
        if !seen.is_empty() {
            u.perception.last_contact = tick;
        }
    }
    timer.mark_post_perception();
    let actions = crate::item_actions::select(units, terrain, tick, observations);
    timer.mark_item_actions();
    let mut damage = vec![0.0; units.len()];
    let mut heals = vec![false; units.len()];
    for (i, action) in actions.into_iter().enumerate() {
        match action {
            Some(Action::Medical(item)) => {
                units[i]
                    .inventory
                    .use_item_by_id(item)
                    .expect("selected item remains usable");
                units[i].combat.last_medical = Some(tick);
                heals[i] = true;
            }
            Some(Action::Water(item)) => {
                units[i]
                    .inventory
                    .use_item_by_id(item)
                    .expect("selected item remains usable");
                units[i].sustainment.hydration =
                    (units[i].sustainment.hydration + crate::consumables::NEED_RESTORE).min(100.0);
            }
            Some(Action::Ration(item)) => {
                units[i]
                    .inventory
                    .use_item_by_id(item)
                    .expect("selected item remains usable");
                units[i].sustainment.nutrition =
                    (units[i].sustainment.nutrition + crate::consumables::NEED_RESTORE).min(100.0);
            }
            Some(Action::Grenade { item, target_idx }) => {
                let target = units[target_idx].nav.position;
                units[i]
                    .inventory
                    .use_item_by_id(item)
                    .expect("selected item remains usable");
                units[i].combat.set_attack_cooldown(
                    (crate::consumables::GRENADE_COOLDOWN_SECONDS / crate::definitions::STEP)
                        as u32,
                );
                units[i].combat.revealed_until = tick + crate::definitions::REVEAL_TICKS;
                for (k, v) in units.iter().enumerate() {
                    if v.vitals.hp > 0.0
                        && v.nav.position.distance(target) <= crate::consumables::BLAST_RADIUS
                        && map::visible_t(terrain, target, v.nav.position, 0.1, 1.0)
                    {
                        damage[k] += crate::consumables::GRENADE_DAMAGE;
                    }
                }
            }
            Some(Action::Fire {
                entry_id,
                target_idx,
                exposure,
                moving_multiplier,
                weapon,
            }) => {
                units[i]
                    .inventory
                    .consume_item(weapon.ammo_item)
                    .expect("selected ammunition remains available");
                if units[i].inventory.count_item(weapon.ammo_item) == 0 {
                    units[i].needs_decision = true;
                }
                units[i].combat.shots += 1;
                units[i].combat.set_weapon_cooldown(
                    entry_id,
                    (weapon.cooldown / crate::definitions::STEP) as u32,
                );
                units[i].combat.revealed_until = tick + crate::definitions::REVEAL_TICKS;
                *rng ^= *rng << 13;
                *rng ^= *rng >> 7;
                *rng ^= *rng << 17;
                let roll = (*rng >> 11) as f64 / ((1_u64 << 53) as f64);
                if roll < weapon.hit_chance * exposure * moving_multiplier {
                    damage[target_idx] += weapon.damage;
                }
            }
            None => {}
        }
    }
    for ((u, d), heal) in units.iter_mut().zip(damage).zip(heals) {
        let was_low = u.is_low_hp();
        if d > 0.0 {
            u.perception.last_contact = tick;
        }
        u.vitals.hp = (u.vitals.hp - d).max(0.0);
        if u.vitals.hp > 0.0 && heal {
            u.vitals.hp = (u.vitals.hp + crate::consumables::MEDICAL_HEAL).min(u.vitals.max_hp);
        }
        if was_low != u.is_low_hp() {
            u.needs_decision = true;
        }
        if u.vitals.hp == 0.0 {
            u.combat.behavior = Behavior::Dead;
            u.perception.target = None;
            u.nav.goal = None;
        }
    }
    timer.mark_combat();
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
            loadout: None,
        });
        unit.combat.behavior = Behavior::Engaging;
        unit.perception.target = Some(if id == 1 { 2 } else { 1 });
        unit
    }
    #[test]
    fn firing_reveal_does_not_change_other_shooters_visibility_in_the_same_tick() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 8.5)];
        units[1].combat.revealed_until = 100;
        let mut rng = 2;
        execute(&mut units, &map, &mut rng, 1);
        assert_eq!(units[0].combat.shots, 1);
        assert_eq!(units[1].combat.shots, 0);
        execute(&mut units, &map, &mut rng, 2);
        assert_eq!(units[1].combat.shots, 1);
    }
    #[test]
    fn firing_reveal_expires_on_simulation_tick_and_never_bypasses_solid_cover() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 8.5)];
        units[1].combat.revealed_until = 41;
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
        units[1].nav.goal = Some(Point { x: 9.5, y: 2.5 });
        units[1].combat.behavior = Behavior::Advancing;
        movement::execute(&mut units, &map, 1);
        assert!(units[1].nav.position.x > 8.83);
        assert_eq!(perception::observe(&units, &map, 0, 1), None);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].combat.shots, 0);
        assert_eq!(units[1].vitals.hp, 100.0);
    }
    #[test]
    fn mutually_visible_lethal_shots_are_resolved_simultaneously() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        for u in &mut units {
            u.vitals.hp = crate::definitions::DAMAGE;
        }
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].vitals.hp, 0.0);
        assert_eq!(units[1].vitals.hp, 0.0);
        assert_eq!(units[0].combat.shots, 1);
        assert_eq!(units[1].combat.shots, 1);
    }
    #[test]
    fn grenade_cluster_damages_visible_enemies_and_spends_one() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5), soldier(3, 7.0)];
        units[2].placement.faction = 1;
        units[1].combat.set_attack_cooldown(10);
        units[2].combat.set_attack_cooldown(10);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].inventory.count("frag-grenade"), 1);
        assert_eq!((units[1].vitals.hp, units[2].vitals.hp), (60.0, 60.0));
        assert_eq!(units[0].inventory.count("rifle-ammo"), 60);
    }
    #[test]
    fn dead_soldier_neither_progresses_needs_nor_consumes_supplies() {
        let map = ArenaMap::flat(16, 6);
        let mut dead = soldier(1, 2.5);
        dead.vitals.hp = 0.0;
        dead.sustainment.nutrition = 50.0;
        dead.sustainment.hydration = 50.0;
        let before = dead.inventory.snapshot();
        crate::sustainment::advance(&mut dead);
        execute(std::slice::from_mut(&mut dead), &map, &mut 2, 1);
        assert_eq!(dead.inventory.snapshot(), before);
        assert_eq!(
            (
                dead.sustainment.nutrition,
                dead.sustainment.hydration,
                dead.vitals.hp
            ),
            (50.0, 50.0, 0.0)
        );
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
            u.combat.set_attack_cooldown(10);
        }
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].inventory.count("frag-grenade"), 2);
        assert_eq!(units[0].inventory.count("rifle-ammo"), 59);
    }
    #[test]
    fn needs_action_precedes_a_valid_shot() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        units[0].sustainment.hydration = 50.0;
        units[1].combat.set_attack_cooldown(10);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].sustainment.hydration, 90.0);
        assert_eq!(units[0].inventory.count("water"), 1);
        assert_eq!(units[0].combat.shots, 0);
    }
    #[test]
    fn safe_medical_use_restores_health_and_interrupts_fallback() {
        let map = ArenaMap::flat(32, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 29.5)];
        units[0].vitals.hp = 20.0;
        execute(&mut units, &map, &mut 2, 100);
        assert_eq!(units[0].vitals.hp, 45.0);
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
        units[1].combat.set_attack_cooldown(10);
        let mut rng = 2;
        execute(&mut units, &map, &mut rng, 1);
        assert_eq!(rng, 2);
        assert_eq!(units[0].combat.shots, 0);
        assert_eq!(units[1].vitals.hp, 100.0);
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
        units[1].combat.set_attack_cooldown(10);
        units[2].combat.set_attack_cooldown(10);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[1].vitals.hp, 60.0);
        assert_eq!(units[2].vitals.hp, 100.0);
    }
    #[test]
    fn committed_medical_supply_is_spent_if_simultaneous_shot_kills() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 8.5)];
        units[0].vitals.hp = 20.0;
        units[0].combat.revealed_until = 100;
        units[0].placement.doctrine.medical.safe_seconds = 0.0;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].vitals.hp, 0.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
        assert_eq!(units[1].combat.shots, 1);
    }
    #[test]
    fn low_health_treats_under_fire_but_out_of_contact_waits() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        units[0].vitals.hp = 40.0;
        units[0].placement.doctrine.medical.policy = MedicalPolicy::LowHealth;
        units[1].combat.set_attack_cooldown(10);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[0].vitals.hp, 65.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
        units[0].vitals.hp = 40.0;
        units[0].combat.last_medical = None;
        units[0].placement.doctrine.medical.policy = MedicalPolicy::OutOfContact;
        execute(&mut units, &map, &mut 2, 2);
        assert_eq!(units[0].vitals.hp, 40.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
    }
    #[test]
    fn out_of_contact_does_not_treat_above_threshold_and_obeys_cooldown() {
        let map = ArenaMap::flat(32, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 29.5)];
        units[0].vitals.hp = 51.0;
        execute(&mut units, &map, &mut 2, 100);
        assert_eq!(units[0].inventory.count("medical-supply"), 2);
        units[0].vitals.hp = 40.0;
        execute(&mut units, &map, &mut 2, 101);
        assert_eq!(units[0].vitals.hp, 65.0);
        units[0].vitals.hp = 40.0;
        execute(&mut units, &map, &mut 2, 102);
        assert_eq!(units[0].vitals.hp, 40.0);
        assert_eq!(units[0].inventory.count("medical-supply"), 1);
    }
    #[test]
    fn blast_reaches_unseen_victim_through_vegetation() {
        let mut map = ArenaMap::flat(16, 6);
        map.surfaces.fill(Surface::Woodland);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5), soldier(3, 7.5)];
        units[2].placement.faction = 1;
        units[0].placement.doctrine.grenades.policy = GrenadePolicy::AnySafeTarget;
        units[1].combat.set_attack_cooldown(10);
        units[2].combat.set_attack_cooldown(10);
        assert_eq!(perception::observe_all(&units, &map, 0, 1), vec![1]);
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!(units[1].vitals.hp, 60.0);
        assert_eq!(units[2].vitals.hp, 60.0);
    }
    #[test]
    fn grenade_and_rifle_resolve_simultaneously() {
        let map = ArenaMap::flat(16, 6);
        let mut units = vec![soldier(1, 2.5), soldier(2, 6.5)];
        units[0].vitals.hp = 20.0;
        units[1].vitals.hp = 40.0;
        units[0].placement.doctrine.grenades.policy = GrenadePolicy::AnySafeTarget;
        units[1].placement.doctrine.grenades.policy = GrenadePolicy::Never;
        execute(&mut units, &map, &mut 2, 1);
        assert_eq!((units[0].vitals.hp, units[1].vitals.hp), (0.0, 0.0));
        assert_eq!(units[0].inventory.count("frag-grenade"), 1);
        assert_eq!(units[1].combat.shots, 1);
    }
    #[test]
    fn moving_behaviors_require_fire_while_moving_for_rifle_and_grenades() {
        let map = ArenaMap::flat(16, 6);
        for behavior in [
            Behavior::Advancing,
            Behavior::FallingBack,
            Behavior::Investigating,
        ] {
            // Default off: neither rifle nor grenade attacks while moving.
            let mut off = vec![soldier(1, 2.5), soldier(2, 6.5)];
            off[0].combat.behavior = behavior.clone();
            off[0].placement.doctrine.fire_while_moving = false;
            off[0].placement.doctrine.grenades.policy = GrenadePolicy::AnySafeTarget;
            off[1].combat.set_attack_cooldown(10);
            execute(&mut off, &map, &mut 2, 1);
            assert_eq!(off[0].combat.shots, 0, "{behavior:?} off should not fire");
            assert_eq!(
                off[0].inventory.count("frag-grenade"),
                2,
                "{behavior:?} off should not throw grenade"
            );

            // Enabled: throws grenade when policy matches, or fires rifle when grenades disabled.
            let mut on_grenade = vec![soldier(1, 2.5), soldier(2, 6.5)];
            on_grenade[0].combat.behavior = behavior.clone();
            on_grenade[0].placement.doctrine.fire_while_moving = true;
            on_grenade[0].placement.doctrine.grenades.policy = GrenadePolicy::AnySafeTarget;
            on_grenade[1].combat.set_attack_cooldown(10);
            execute(&mut on_grenade, &map, &mut 2, 1);
            assert_eq!(
                on_grenade[0].inventory.count("frag-grenade"),
                1,
                "{behavior:?} on should allow grenade"
            );

            let mut on_rifle = vec![soldier(1, 2.5), soldier(2, 6.5)];
            on_rifle[0].combat.behavior = behavior.clone();
            on_rifle[0].placement.doctrine.fire_while_moving = true;
            on_rifle[0].placement.doctrine.grenades.policy = GrenadePolicy::Never;
            on_rifle[1].combat.set_attack_cooldown(10);
            execute(&mut on_rifle, &map, &mut 2, 1);
            assert_eq!(
                on_rifle[0].combat.shots, 1,
                "{behavior:?} on should allow rifle fire"
            );
        }
    }
}
