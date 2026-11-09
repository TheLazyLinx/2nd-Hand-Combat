use crate::{map, perception::Unit, types::*};
pub(crate) fn execute(units: &mut [Unit], map: &ArenaMap, rng: &mut u64) {
    let mut damage = vec![0.0; units.len()];
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
        if units[i].position.distance(units[j].position) > crate::definitions::RANGE
            || !map::clear(map, units[i].position, units[j].position)
        {
            continue;
        }
        units[i].shots += 1;
        units[i].cooldown = (crate::definitions::COOLDOWN / crate::definitions::STEP) as u32;
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        *rng ^= *rng << 17;
        let roll = (*rng >> 11) as f64 / ((1_u64 << 53) as f64);
        if roll < crate::definitions::HIT_CHANCE {
            damage[j] += if map::in_cover(map, units[j].position) {
                crate::definitions::DAMAGE / 2.0
            } else {
                crate::definitions::DAMAGE
            };
        }
    }
    for (u, d) in units.iter_mut().zip(damage) {
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
