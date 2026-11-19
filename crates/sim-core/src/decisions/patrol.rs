//! Patrol/search proposal: advances along scenario patrol waypoints when no threat is active.

use super::{Decision, DecisionContext};
use crate::{definitions::TUNING, types::*};

pub(crate) fn propose(ctx: &DecisionContext<'_>) -> Decision {
    let me = ctx.input.unit;
    let patrol = &ctx.terrain.map.patrol;
    if patrol.is_empty() {
        return Decision::hold(Behavior::Searching, None, me.patrol_index);
    }
    let mut idx = me.patrol_index % patrol.len();
    let direction = if ctx.faction_index == 0 {
        1
    } else {
        patrol.len() - 1
    };
    if me.position.distance(patrol[idx]) < TUNING.patrol_arrive_distance {
        idx = (idx + direction) % patrol.len();
    }
    Decision::move_to(Behavior::Searching, patrol[idx], None, idx, None)
}
