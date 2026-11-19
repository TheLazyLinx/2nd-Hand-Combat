//! Engagement approach proposal: closes distance toward a visible enemy beyond engagement range.

use super::{grid_window, Decision, DecisionContext};
use crate::{definitions::TUNING, map, navigation::SearchScratch, types::*};

#[derive(Clone, Copy)]
struct ApproachCandidate {
    point: Point,
    cost: f64,
    offset: f64,
}

impl ApproachCandidate {
    fn is_better_than(&self, other: &Self) -> bool {
        let tol = TUNING.score_tolerance;
        self.cost < other.cost - tol
            || ((self.cost - other.cost).abs() <= tol && self.offset < other.offset)
    }
}

pub(crate) fn propose(ctx: &DecisionContext<'_>, scratch: &mut SearchScratch) -> Option<Decision> {
    let me = ctx.input.unit;
    let enemy = ctx.input.visible_enemy?;
    let engagement_distance = me.placement.doctrine.engagement_distance;
    let distance = me.position.distance(enemy);
    if distance <= engagement_distance {
        return None;
    }

    let ratio = (distance - engagement_distance) / distance;
    let desired = Point {
        x: me.position.x + (enemy.x - me.position.x) * ratio,
        y: me.position.y + (enemy.y - me.position.y) * ratio,
    };
    if map::travel_time_t(ctx.terrain, me.position, desired).is_some() {
        return Some(Decision::move_to(
            Behavior::Advancing,
            desired,
            None,
            me.patrol_index,
            None,
        ));
    }

    // Only cells within engagement_distance of the enemy can be valid firing positions.
    let map = ctx.terrain.map;
    let width = ctx.terrain.cache.width;
    let (x0, x1, y0, y1) = grid_window(map, enemy, engagement_distance);
    let mut firing_cells = Vec::new();
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
            if p.distance(enemy) <= engagement_distance
                && map::exposure_t(ctx.terrain, p, enemy, ctx.input.visible_enemy_revealed) > 0.0
            {
                firing_cells.push(p);
            }
        }
    }

    if !firing_cells.is_empty() {
        let params = ctx.search_params();
        scratch.run_dijkstra_until_best_target(
            ctx.terrain,
            &params,
            &firing_cells,
            TUNING.score_tolerance,
            f64::INFINITY,
        );
        let mut best: Option<ApproachCandidate> = None;
        for p in firing_cells {
            let Some(cost) = scratch.cost(p) else {
                continue;
            };
            let candidate = ApproachCandidate {
                point: p,
                cost,
                offset: p.distance(desired),
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
                me.patrol_index,
                Some(route),
            ));
        }
    }

    Some(Decision::hold(Behavior::Engaging, None, me.patrol_index))
}
