//! Investigation proposal: moves to observe a remembered last-seen threat position within a
//! bounded search cost/radius, and marks unreachable memories investigated so units never stall.

use super::{Decision, DecisionContext};
use crate::{definitions::TUNING, map, navigation::SearchScratch, types::*};

pub(crate) enum InvestigateOutcome {
    Decision(Decision),
    Unreachable,
    None,
}

pub(crate) fn propose(
    ctx: &DecisionContext<'_>,
    scratch: &mut SearchScratch,
) -> InvestigateOutcome {
    let me = ctx.input.unit;
    let Some(memory) = me.perception.primary() else {
        return InvestigateOutcome::None;
    };
    if memory.investigated {
        return InvestigateOutcome::None;
    }

    if map::clear_t(ctx.terrain, me.nav.position, memory.position) {
        if map::travel_time_t(ctx.terrain, me.nav.position, memory.position).is_some() {
            return InvestigateOutcome::Decision(Decision::move_to(
                Behavior::Investigating,
                memory.position,
                None,
                me.nav.patrol_index,
                None,
            ));
        }
        scratch.run_astar(
            ctx.terrain,
            me.nav.position,
            memory.position,
            ctx.threats,
            ctx.preference,
            ctx.revealed,
        );
        if scratch.cost(memory.position).is_some() {
            let route = scratch.route_with_threats(
                ctx.terrain,
                memory.position,
                ctx.threats,
                ctx.preference,
            );
            return InvestigateOutcome::Decision(Decision::move_to(
                Behavior::Investigating,
                memory.position,
                None,
                me.nav.patrol_index,
                Some(route),
            ));
        }
        return InvestigateOutcome::Unreachable;
    }

    let params = ctx.search_params();
    let max_radius = TUNING.investigate_max_radius;
    let spotting_range = me.def.spotting_range;
    let target_pos = memory.position;
    let start_pos = me.nav.position;

    if let Some(goal) = scratch.run_dijkstra_first_matching(
        ctx.terrain,
        &params,
        TUNING.investigate_max_cost,
        |p| {
            p.distance(target_pos) <= spotting_range
                && p.distance(start_pos) <= max_radius
                && map::exposure_t(ctx.terrain, p, target_pos, false) > 0.0
        },
    ) {
        let route = scratch.route_with_threats(ctx.terrain, goal, ctx.threats, ctx.preference);
        return InvestigateOutcome::Decision(Decision::move_to(
            Behavior::Investigating,
            goal,
            None,
            me.nav.patrol_index,
            Some(route),
        ));
    }

    if scratch.cost(target_pos).is_some() {
        let route =
            scratch.route_with_threats(ctx.terrain, target_pos, ctx.threats, ctx.preference);
        return InvestigateOutcome::Decision(Decision::move_to(
            Behavior::Investigating,
            target_pos,
            None,
            me.nav.patrol_index,
            Some(route),
        ));
    }

    InvestigateOutcome::Unreachable
}
