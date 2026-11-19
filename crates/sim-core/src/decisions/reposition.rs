//! Tactical repositioning proposal: selects nearby cover/concealment within engagement range.

use super::{grid_window, Decision, DecisionContext};
use crate::{definitions::TUNING, map, navigation::SearchScratch, types::*};

#[derive(Clone, Copy)]
struct PositionCandidate {
    point: Point,
    score: f64,
    cost: f64,
}

impl PositionCandidate {
    fn is_better_than(&self, other: &Self) -> bool {
        let tol = TUNING.score_tolerance;
        self.score < other.score - tol
            || ((self.score - other.score).abs() <= tol && self.cost < other.cost)
    }
}

fn position_score(ctx: &DecisionContext<'_>, p: Point) -> f64 {
    let pref = ctx.input.unit.placement.doctrine.position_preference;
    ctx.threats
        .iter()
        .copied()
        .map(|enemy| match pref {
            PositionPreference::Balanced => map::exposure_t(ctx.terrain, enemy, p, ctx.revealed),
            PositionPreference::Cover => map::shot_fraction_t(ctx.terrain, enemy, p),
            PositionPreference::Concealment => {
                1.0 / (1.0 + map::spotting_cost_t(ctx.terrain, enemy, p))
            }
        })
        .fold(0.0_f64, f64::max)
}

pub(crate) fn propose(ctx: &DecisionContext<'_>, scratch: &mut SearchScratch) -> Option<Decision> {
    let me = ctx.input.unit;
    let enemy = ctx.input.visible_enemy?;
    let engagement_distance = me.placement.doctrine.engagement_distance;
    if me.nav.position.distance(enemy) > engagement_distance {
        return None;
    }

    let tol = TUNING.score_tolerance;
    let current = position_score(ctx, me.nav.position);
    if let Some(goal) = me.nav.goal {
        if me.combat.behavior == Behavior::Advancing
            && me.nav.position.distance(goal) > TUNING.advance_arrive_distance
            && goal.distance(enemy) <= engagement_distance
            && map::exposure_t(ctx.terrain, goal, enemy, ctx.input.visible_enemy_revealed) > 0.0
            && position_score(ctx, goal) <= current + tol
        {
            return Some(Decision::move_to(
                Behavior::Advancing,
                goal,
                None,
                me.nav.patrol_index,
                None,
            ));
        }
    }

    let radius = TUNING.reposition_radius;
    let map = ctx.terrain.map;
    let width = ctx.terrain.cache.width;
    let (x0, x1, y0, y1) = grid_window(map, me.nav.position, radius);

    let mut candidates = Vec::new();
    for y in y0..=y1 {
        let row = y as usize * width;
        for x in x0..=x1 {
            if ctx.terrain.cache.occupied[row + x as usize] {
                continue;
            }
            let p = Point {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
            };
            if me.nav.position.distance(p) > radius
                || p.distance(enemy) > engagement_distance
                || map::exposure_t(ctx.terrain, p, enemy, ctx.input.visible_enemy_revealed) <= 0.0
            {
                continue;
            }
            let score = position_score(ctx, p);
            if score >= current - tol {
                continue;
            }
            candidates.push((p, score));
        }
    }

    if candidates.is_empty() {
        return Some(Decision::hold(
            Behavior::Engaging,
            None,
            me.nav.patrol_index,
        ));
    }

    let targets: Vec<_> = candidates.iter().map(|(p, _)| *p).collect();
    scratch.run_dijkstra(
        ctx.terrain,
        me.nav.position,
        ctx.threats,
        ctx.preference,
        ctx.revealed,
        Some(&targets),
    );

    let mut best: Option<PositionCandidate> = None;
    for (p, score) in candidates {
        let Some(cost) = scratch.cost(p) else {
            continue;
        };
        let candidate = PositionCandidate {
            point: p,
            score,
            cost,
        };
        if best.as_ref().is_none_or(|b| candidate.is_better_than(b)) {
            best = Some(candidate);
        }
    }

    if let Some(chosen) = best {
        let route =
            scratch.route_with_threats(ctx.terrain, chosen.point, ctx.threats, ctx.preference);
        return Some(Decision::move_to(
            Behavior::Advancing,
            chosen.point,
            None,
            me.nav.patrol_index,
            Some(route),
        ));
    }

    Some(Decision::hold(
        Behavior::Engaging,
        None,
        me.nav.patrol_index,
    ))
}
