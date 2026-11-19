//! Withdrawal decision proposal: selects bounded cover or rally points for low-HP / out-of-ammo units.

use super::{grid_window, Decision, DecisionContext};
use crate::{definitions::TUNING, map, navigation::SearchScratch, types::*};

#[derive(Clone, Copy)]
struct FallbackCandidate {
    point: Point,
    protection: f64,
    enemy_distance: f64,
    cost: f64,
    is_ally: bool,
}

impl FallbackCandidate {
    fn is_better_than(&self, other: &Self, current_protection: f64) -> bool {
        let tol = TUNING.score_tolerance;
        if self.protection > other.protection + tol {
            return true;
        }
        if (self.protection - other.protection).abs() > tol {
            return false;
        }
        if self.is_ally != other.is_ally {
            return self.is_ally;
        }
        if self.protection > current_protection + tol {
            self.cost < other.cost - tol
        } else {
            self.enemy_distance > other.enemy_distance + tol
        }
    }
}

fn safety(ctx: &DecisionContext<'_>, p: Point) -> (f64, f64) {
    let exposure = ctx
        .threats
        .iter()
        .copied()
        .map(|enemy| map::exposure_t(ctx.terrain, enemy, p, ctx.revealed))
        .fold(0.0_f64, f64::max);
    let nearest = ctx
        .threats
        .iter()
        .map(|enemy| p.distance(*enemy))
        .fold(f64::INFINITY, f64::min);
    (1.0 - exposure, nearest)
}

pub(crate) fn propose(ctx: &DecisionContext<'_>, scratch: &mut SearchScratch) -> Option<Decision> {
    let me = ctx.input.unit;
    if !me.withdrawing() {
        return None;
    }
    let Some(enemy) = ctx.threat else {
        return Some(Decision::hold(
            Behavior::Defending,
            None,
            me.nav.patrol_index,
        ));
    };

    let tol = TUNING.score_tolerance;
    let current = safety(ctx, me.nav.position);
    if let (Some(goal), Some(old_threat)) = (me.nav.fallback_goal, me.nav.fallback_threat) {
        let safe = safety(ctx, goal);
        if old_threat.distance(enemy) < TUNING.threat_moved_distance
            && (safe.0 > current.0 + tol
                || (safe.0 >= current.0 - tol
                    && me.nav.position.distance(goal) > TUNING.fallback_arrive_distance)
                || me.nav.position.distance(goal) <= TUNING.fallback_arrive_distance)
        {
            if me.nav.position.distance(goal) <= TUNING.fallback_arrive_distance {
                return Some(Decision::hold(
                    Behavior::Defending,
                    Some(goal),
                    me.nav.patrol_index,
                ));
            }
            return Some(Decision::move_to(
                Behavior::FallingBack,
                goal,
                Some(goal),
                me.nav.patrol_index,
                None,
            ));
        }
    }

    let radius = TUNING.fallback_radius;
    let map = ctx.terrain.map;
    let width = ctx.terrain.cache.width;
    let (x0, x1, y0, y1) = grid_window(map, me.nav.position, radius);

    let mut targets = Vec::new();
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
            if me.nav.position.distance(p) <= radius {
                targets.push(p);
            }
        }
    }
    for &ally in &ctx.input.allies {
        if me.nav.position.distance(ally) <= radius {
            targets.push(ally);
        }
    }

    scratch.run_dijkstra(
        ctx.terrain,
        me.nav.position,
        ctx.threats,
        ctx.preference,
        ctx.revealed,
        Some(&targets),
    );

    let mut best: Option<FallbackCandidate> = None;
    for p in targets {
        let Some(cost) = scratch.cost(p) else {
            continue;
        };
        let (protection, enemy_distance) = safety(ctx, p);
        if protection <= current.0 + tol
            && !(protection >= current.0 - tol
                && enemy_distance > current.1 + TUNING.fallback_distance_margin)
        {
            continue;
        }
        let candidate = FallbackCandidate {
            point: p,
            protection,
            enemy_distance,
            cost,
            is_ally: ctx.input.allies.contains(&p),
        };
        if best
            .as_ref()
            .is_none_or(|b| candidate.is_better_than(b, current.0))
        {
            best = Some(candidate);
        }
    }

    if let Some(chosen) = best {
        let route =
            scratch.route_with_threats(ctx.terrain, chosen.point, ctx.threats, ctx.preference);
        return Some(Decision::move_to(
            Behavior::FallingBack,
            chosen.point,
            Some(chosen.point),
            me.nav.patrol_index,
            Some(route),
        ));
    }
    Some(Decision::hold(
        Behavior::Defending,
        None,
        me.nav.patrol_index,
    ))
}
