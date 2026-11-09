use crate::{navigation, perception::Unit, types::*};
pub(crate) fn execute(units: &mut [Unit], map: &ArenaMap) {
    for u in units {
        if u.hp <= 0.0 {
            continue;
        }
        if let Some(goal) = u.goal {
            u.position = navigation::move_toward(map, u.position, goal);
            if u.behavior == Behavior::Investigating && u.position.distance(goal) < 0.2 {
                if let Some(memory) = &mut u.memory {
                    memory.investigated = true;
                }
                u.goal = None;
            }
        }
        if u.cooldown > 0 {
            u.cooldown -= 1;
        }
    }
}
