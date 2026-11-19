//! Patrol/search proposal: advances along scenario patrol waypoints in the faction's patrol direction.

use super::{Decision, DecisionContext};
use crate::{definitions::TUNING, types::*};

pub(crate) fn propose(ctx: &DecisionContext<'_>) -> Decision {
    let me = ctx.input.unit;
    let patrol = &ctx.terrain.map.patrol;
    if patrol.is_empty() {
        return Decision::hold(Behavior::Searching, None, me.nav.patrol_index);
    }
    let mut idx = me.nav.patrol_index % patrol.len();
    let step = match ctx.patrol_direction {
        PatrolDirection::Forward => 1,
        PatrolDirection::Reverse => patrol.len() - 1,
    };
    if me.nav.position.distance(patrol[idx]) < TUNING.patrol_arrive_distance {
        idx = (idx + step) % patrol.len();
    }
    Decision::move_to(Behavior::Searching, patrol[idx], None, idx, None)
}
