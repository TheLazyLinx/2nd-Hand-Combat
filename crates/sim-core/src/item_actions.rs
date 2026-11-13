use crate::{map, types::*, unit::Unit};
#[derive(Clone)]
pub(crate) enum Action {
    Medical,
    Water,
    Ration,
    Grenade(usize),
    Rifle(usize, f64, WeaponDefinition),
}
pub(crate) fn select(
    units: &[Unit],
    map: &ArenaMap,
    tick: u32,
    observations: &[Vec<usize>],
) -> Vec<Option<Action>> {
    units
        .iter()
        .enumerate()
        .map(|(i, u)| {
            if u.hp <= 0.0 {
                return None;
            }
            let med = &u.placement.doctrine.medical;
            let safe = observations[i].is_empty()
                && tick.saturating_sub(u.last_contact) as f64 * crate::definitions::STEP
                    >= med.safe_seconds;
            let medical_due = u.last_medical.is_none_or(|last| {
                tick.saturating_sub(last) as f64 * crate::definitions::STEP >= med.cooldown_seconds
            });
            if u.hp <= med.hp_threshold
                && u.hp < 100.0
                && u.inventory.can_use("medical-supply")
                && medical_due
                && match med.policy {
                    MedicalPolicy::Never => false,
                    MedicalPolicy::OutOfContact => safe,
                    MedicalPolicy::LowHealth => true,
                }
            {
                return Some(Action::Medical);
            }
            if u.hydration <= crate::consumables::NEED_USE_THRESHOLD && u.inventory.can_use("water")
            {
                return Some(Action::Water);
            }
            if u.nutrition <= crate::consumables::NEED_USE_THRESHOLD
                && u.inventory.can_use("ration")
            {
                return Some(Action::Ration);
            }
            if u.cooldown > 0 {
                return None;
            }
            let grenade = &u.placement.doctrine.grenades;
            if grenade.policy != GrenadePolicy::Never
                && u.inventory.can_use("frag-grenade")
                && (grenade.policy != GrenadePolicy::EmptyRifleOnly
                    || u.inventory.count("rifle-ammo") == 0)
            {
                for &j in &observations[i] {
                    let target = units[j].position;
                    if u.position.distance(target) > grenade.max_distance
                        || !map::clear(map, u.position, target)
                        || units.iter().any(|ally| {
                            ally.hp > 0.0
                                && ally.placement.faction == u.placement.faction
                                && ally.position.distance(target)
                                    <= crate::consumables::BLAST_RADIUS + grenade.safety_margin
                        })
                    {
                        continue;
                    }
                    let count = observations[i]
                        .iter()
                        .filter(|&&k| {
                            units[k].position.distance(target) <= crate::consumables::BLAST_RADIUS
                                && map::visible(map, target, units[k].position, 0.1, 1.0)
                        })
                        .count();
                    if grenade.policy != GrenadePolicy::Clusters
                        || count >= grenade.min_enemies as usize
                    {
                        return Some(Action::Grenade(j));
                    }
                }
            }
            let weapon = u.inventory.held_weapon()?;
            if !matches!(
                u.behavior,
                Behavior::Engaging | Behavior::FallingBack | Behavior::Defending
            ) || u.inventory.count(&weapon.ammo_item_id) == 0
            {
                return None;
            }
            let j = observations[i].first().copied()?;
            if u.position.distance(units[j].position) > weapon.range {
                return None;
            }
            let exposure = map::exposure(
                map,
                u.position,
                units[j].position,
                tick < units[j].revealed_until,
            );
            (exposure > 0.0).then_some(Action::Rifle(j, exposure, weapon))
        })
        .collect()
}
