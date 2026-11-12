use crate::{
    definitions::{surface_definition, SPEED},
    types::*,
};

pub(crate) const UNIT_RADIUS: f64 = 0.25;
const EPS: f64 = 1e-9;
fn point(a: Point, b: Point, t: f64) -> Point {
    Point {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
    }
}
pub(crate) fn inside(m: &ArenaMap, p: Point) -> bool {
    p.x.is_finite()
        && p.y.is_finite()
        && p.x >= UNIT_RADIUS
        && p.y >= UNIT_RADIUS
        && p.x <= m.width as f64 - UNIT_RADIUS
        && p.y <= m.height as f64 - UNIT_RADIUS
}
pub(crate) fn blocked(m: &ArenaMap, p: Point) -> bool {
    !inside(m, p)
        || m.obstacles.iter().any(|o| {
            let x = p.x.clamp(o.x as f64, (o.x.saturating_add(o.width)) as f64);
            let y = p.y.clamp(o.y as f64, (o.y.saturating_add(o.height)) as f64);
            (p.x - x).powi(2) + (p.y - y).powi(2) < UNIT_RADIUS.powi(2) - EPS
        })
}
pub(crate) fn surface(m: &ArenaMap, p: Point) -> Surface {
    let x = (p.x.floor() as usize).min(m.width as usize - 1);
    let y = (p.y.floor() as usize).min(m.height as usize - 1);
    m.surfaces[y * m.width as usize + x]
}
pub(crate) fn ground_height(m: &ArenaMap, p: Point) -> f64 {
    let x = (p.x.floor() as usize).min(m.width as usize - 1);
    let y = (p.y.floor() as usize).min(m.height as usize - 1);
    let u = p.x - x as f64;
    let v = p.y - y as f64;
    let w = m.width as usize + 1;
    let a = m.elevations[y * w + x];
    let b = m.elevations[y * w + x + 1];
    let c = m.elevations[(y + 1) * w + x];
    let d = m.elevations[(y + 1) * w + x + 1];
    if u >= v {
        a + (b - a) * u + (d - b) * v
    } else {
        a + (d - c) * u + (c - a) * v
    }
}
// Split at every grid line and diagonal: ground and surface are linear/constant in each interval.
fn breaks(m: &ArenaMap, a: Point, b: Point) -> Vec<f64> {
    let mut ts = vec![0.0, 1.0];
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    for x in
        (a.x.min(b.x).floor().max(0.0) as u32)..=(a.x.max(b.x).ceil().min(m.width as f64) as u32)
    {
        if dx.abs() > EPS {
            let t = (x as f64 - a.x) / dx;
            if t > EPS && t < 1.0 - EPS {
                ts.push(t);
            }
        }
    }
    for y in
        (a.y.min(b.y).floor().max(0.0) as u32)..=(a.y.max(b.y).ceil().min(m.height as f64) as u32)
    {
        if dy.abs() > EPS {
            let t = (y as f64 - a.y) / dy;
            if t > EPS && t < 1.0 - EPS {
                ts.push(t);
            }
        }
    }
    if (dx - dy).abs() > EPS {
        for k in
            ((a.x - a.y).min(b.x - b.y).floor() as i32)..=((a.x - a.y).max(b.x - b.y).ceil() as i32)
        {
            let t = (k as f64 - (a.x - a.y)) / (dx - dy);
            if t > EPS && t < 1.0 - EPS {
                ts.push(t);
            }
        }
    }
    ts.sort_by(|a, b| a.total_cmp(b));
    ts.dedup_by(|a, b| (*a - *b).abs() < EPS);
    ts
}
fn rect_interval(a: Point, b: Point, x0: f64, y0: f64, x1: f64, y1: f64) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for (v, d, min, max) in [(a.x, b.x - a.x, x0, x1), (a.y, b.y - a.y, y0, y1)] {
        if d.abs() < EPS {
            if v < min || v > max {
                return None;
            }
        } else {
            let t = (min - v) / d;
            let u = (max - v) / d;
            lo = lo.max(t.min(u));
            hi = hi.min(t.max(u));
            if lo > hi {
                return None;
            }
        }
    }
    Some((lo, hi))
}
fn visible(m: &ArenaMap, a: Point, b: Point, ah: f64, bh: f64) -> bool {
    let za = ground_height(m, a) + ah;
    let zb = ground_height(m, b) + bh;
    let cuts = breaks(m, a, b);
    for t in &cuts {
        let z = za + (zb - za) * t;
        if ground_height(m, point(a, b, *t)) > z + EPS {
            return false;
        }
    }
    for o in &m.obstacles {
        if let Some((lo, hi)) = rect_interval(
            a,
            b,
            o.x as f64,
            o.y as f64,
            (o.x.saturating_add(o.width)) as f64,
            (o.y.saturating_add(o.height)) as f64,
        ) {
            for t in cuts
                .iter()
                .copied()
                .filter(|t| *t > lo && *t < hi)
                .chain([lo, hi])
            {
                if ground_height(m, point(a, b, t)) + o.height_metres >= za + (zb - za) * t - EPS {
                    return false;
                }
            }
        }
    }
    true
}
pub(crate) fn clear(m: &ArenaMap, a: Point, b: Point) -> bool {
    visible(m, a, b, 1.6, 1.6)
}
pub(crate) fn shot_fraction(m: &ArenaMap, a: Point, b: Point) -> f64 {
    [0.4, 0.9, 1.4]
        .iter()
        .filter(|h| visible(m, a, b, 1.4, **h))
        .count() as f64
        / 3.0
}
pub(crate) fn spotting_cost(m: &ArenaMap, a: Point, b: Point) -> f64 {
    spotting_cost_with_cuts(m, a, b, &breaks(m, a, b))
}
fn spotting_cost_with_cuts(m: &ArenaMap, a: Point, b: Point, cuts: &[f64]) -> f64 {
    let len = a.distance(b);
    let za = ground_height(m, a) + 1.6;
    let zb = ground_height(m, b) + 1.6;
    let mut cost = 0.0;
    for pair in cuts.windows(2) {
        let mid = (pair[0] + pair[1]) / 2.0;
        let def = surface_definition(surface(m, point(a, b, mid)));
        let mut fraction = 0.0;
        if def.vegetation_height > 0.0 {
            let delta = |t: f64| {
                ground_height(m, point(a, b, t)) + def.vegetation_height - (za + (zb - za) * t)
            };
            let d0 = delta(pair[0]);
            let d1 = delta(pair[1]);
            fraction = if d0 >= 0.0 && d1 >= 0.0 {
                1.0
            } else if d0 < 0.0 && d1 < 0.0 {
                0.0
            } else {
                let crossing = d0 / (d0 - d1);
                if d0 >= 0.0 {
                    crossing
                } else {
                    1.0 - crossing
                }
            };
        }
        cost += len * (pair[1] - pair[0]) * (1.0 + (def.spotting_multiplier - 1.0) * fraction);
    }
    cost
}
pub(crate) fn detectable(m: &ArenaMap, a: Point, b: Point, revealed: bool) -> bool {
    let d = a.distance(b);
    d <= 14.0 && clear(m, a, b) && (revealed || d <= 2.0 || spotting_cost(m, a, b) <= 14.0 + EPS)
}
/// Detection multiplied by target exposure, sharing ground intersections for all four rays.
/// Navigation uses this hot query repeatedly; its result matches detectable * shot_fraction.
pub(crate) fn exposure(m: &ArenaMap, a: Point, b: Point, revealed: bool) -> f64 {
    let distance = a.distance(b);
    if distance > 14.0 {
        return 0.0;
    }
    let cuts = breaks(m, a, b);
    let ground_a = ground_height(m, a);
    let ground_b = ground_height(m, b);
    let eye_a = ground_a + 1.6;
    let eye_b = ground_b + 1.6;
    let muzzle = ground_a + 1.4;
    let targets = [ground_b + 0.4, ground_b + 0.9, ground_b + 1.4];
    let mut unblocked = [true; 3];
    let mut test = |t: f64, top: f64, solid: bool| {
        let blocks = |height: f64| {
            if solid {
                top >= height - EPS
            } else {
                top > height + EPS
            }
        };
        if blocks(eye_a + (eye_b - eye_a) * t) {
            return false;
        }
        for (open, target) in unblocked.iter_mut().zip(targets) {
            if *open && blocks(muzzle + (target - muzzle) * t) {
                *open = false;
            }
        }
        true
    };
    for t in &cuts {
        if !test(*t, ground_height(m, point(a, b, *t)), false) {
            return 0.0;
        }
    }
    for o in &m.obstacles {
        if let Some((lo, hi)) = rect_interval(
            a,
            b,
            o.x as f64,
            o.y as f64,
            o.x.saturating_add(o.width) as f64,
            o.y.saturating_add(o.height) as f64,
        ) {
            for t in cuts
                .iter()
                .copied()
                .filter(|t| *t > lo && *t < hi)
                .chain([lo, hi])
            {
                if !test(t, ground_height(m, point(a, b, t)) + o.height_metres, true) {
                    return 0.0;
                }
            }
        }
    }
    if !revealed && distance > 2.0 && spotting_cost_with_cuts(m, a, b, &cuts) > 14.0 + EPS {
        return 0.0;
    }
    unblocked.iter().filter(|open| **open).count() as f64 / 3.0
}
fn multiplier(m: &ArenaMap, a: Point, b: Point) -> f64 {
    let d = a.distance(b);
    if d < EPS {
        return surface_definition(surface(m, a)).movement_multiplier;
    }
    let grade = (ground_height(m, b) - ground_height(m, a)) / d;
    if grade.abs() > 0.6 + EPS {
        return 0.0;
    }
    let slope = if grade >= 0.0 {
        1.0 / (1.0 + 2.0 * grade)
    } else if grade >= -0.2 {
        1.0 + 0.15 * (-grade) / 0.2
    } else {
        1.15 - 0.6 * (-grade - 0.2) / 0.4
    };
    (slope * surface_definition(surface(m, point(a, b, 0.5))).movement_multiplier).min(1.3)
}
pub(crate) fn movement_multiplier(m: &ArenaMap, a: Point, b: Point) -> f64 {
    multiplier(m, a, b)
}
fn segment_point_distance(a: Point, b: Point, p: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let l = dx * dx + dy * dy;
    let t = if l < EPS {
        0.0
    } else {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / l
    };
    point(a, b, t.clamp(0.0, 1.0)).distance(p)
}
fn sweep_clear(m: &ArenaMap, a: Point, b: Point) -> bool {
    if blocked(m, a) || blocked(m, b) {
        return false;
    }
    for o in &m.obstacles {
        let (x, y, x1, y1) = (
            o.x as f64,
            o.y as f64,
            (o.x.saturating_add(o.width)) as f64,
            (o.y.saturating_add(o.height)) as f64,
        );
        if rect_interval(a, b, x - UNIT_RADIUS + EPS, y, x1 + UNIT_RADIUS - EPS, y1).is_some()
            || rect_interval(a, b, x, y - UNIT_RADIUS + EPS, x1, y1 + UNIT_RADIUS - EPS).is_some()
            || [
                Point { x, y },
                Point { x: x1, y },
                Point { x, y: y1 },
                Point { x: x1, y: y1 },
            ]
            .iter()
            .any(|p| segment_point_distance(a, b, *p) < UNIT_RADIUS - EPS)
        {
            return false;
        }
    }
    true
}
pub(crate) fn ground_traversable(m: &ArenaMap, a: Point, b: Point) -> bool {
    breaks(m, a, b)
        .windows(2)
        .all(|pair| multiplier(m, point(a, b, pair[0]), point(a, b, pair[1])) > 0.0)
}
pub(crate) fn travel_time(m: &ArenaMap, a: Point, b: Point) -> Option<f64> {
    if !sweep_clear(m, a, b) {
        return None;
    }
    let mut time = 0.0;
    for pair in breaks(m, a, b).windows(2) {
        let p = point(a, b, pair[0]);
        let q = point(a, b, pair[1]);
        let speed = multiplier(m, p, q);
        if speed <= 0.0 {
            return None;
        }
        time += p.distance(q) / (speed * SPEED);
    }
    Some(time)
}
pub(crate) fn advance_toward(m: &ArenaMap, a: Point, b: Point, budget: f64) -> (Point, f64) {
    if budget <= 0.0 {
        return (a, 0.0);
    }
    if travel_time(m, a, b).is_none() {
        return (a, 0.0);
    }
    let mut spent = 0.0;
    for pair in breaks(m, a, b).windows(2) {
        let p = point(a, b, pair[0]);
        let q = point(a, b, pair[1]);
        let time = p.distance(q) / (multiplier(m, p, q) * SPEED);
        if spent + time > budget {
            return (point(p, q, (budget - spent) / time), budget);
        }
        spent += time;
    }
    (b, spent)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    #[test]
    fn shared_exposure_preserves_detection_and_directional_protection() {
        let positions = [
            p(1.5, 1.5),
            p(9.5, 6.5),
            p(14.5, 8.5),
            p(16.5, 11.5),
            p(22.5, 13.5),
            p(28.5, 17.5),
            p(3.5, 15.5),
            p(7.5, 13.5),
        ];
        for preset in crate::catalog().maps {
            for a in positions {
                for b in positions {
                    for revealed in [false, true] {
                        let expected = if detectable(&preset.map, a, b, revealed) {
                            shot_fraction(&preset.map, a, b)
                        } else {
                            0.0
                        };
                        assert_eq!(
                            exposure(&preset.map, a, b, revealed),
                            expected,
                            "{}: {a:?} -> {b:?}",
                            preset.name
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn triangulated_ground_is_continuous_across_diagonal() {
        let mut m = ArenaMap::flat(2, 2);
        m.elevations[4] = 2.0;
        assert!((ground_height(&m, p(0.5, 0.5)) - 1.0).abs() < 1e-8);
        assert!(
            (ground_height(&m, p(0.5, 0.5 - 1e-8)) - ground_height(&m, p(0.5, 0.5 + 1e-8))).abs()
                < 1e-6
        );
    }
    #[test]
    fn low_obstacle_blocks_only_lower_shot_samples() {
        let mut m = ArenaMap::flat(8, 4);
        m.obstacles.push(Obstacle {
            x: 4,
            y: 1,
            width: 1,
            height: 1,
            height_metres: 1.0,
        });
        assert!(clear(&m, p(2.0, 1.5), p(6.0, 1.5)));
        assert!(shot_fraction(&m, p(2.0, 1.5), p(6.0, 1.5)) < 1.0);
        assert!(shot_fraction(&m, p(2.0, 1.5), p(6.0, 1.5)) > 0.0);
    }
    #[test]
    fn ridge_blocks_physical_sight() {
        let mut m = ArenaMap::flat(4, 2);
        for y in 0..=2 {
            m.elevations[y * 5 + 2] = 3.0;
        }
        assert!(!clear(&m, p(0.5, 1.0), p(3.5, 1.0)));
    }
    #[test]
    fn foliage_cost_and_reveal_preserve_physical_sight() {
        let mut m = ArenaMap::flat(16, 3);
        m.surfaces.fill(Surface::Woodland);
        assert!(!detectable(&m, p(1.0, 1.5), p(8.0, 1.5), false));
        assert!(detectable(&m, p(1.0, 1.5), p(8.0, 1.5), true));
        assert!(detectable(&m, p(1.0, 1.5), p(2.5, 1.5), false));
        m.obstacles.push(Obstacle {
            x: 4,
            y: 1,
            width: 1,
            height: 1,
            height_metres: 3.0,
        });
        assert!(!detectable(&m, p(1.0, 1.5), p(8.0, 1.5), true));
    }
    #[test]
    fn swept_radius_and_segment_speed_are_applied() {
        let mut m = ArenaMap::flat(8, 4);
        m.obstacles.push(Obstacle {
            x: 3,
            y: 1,
            width: 1,
            height: 1,
            height_metres: 1.0,
        });
        assert!(travel_time(&m, p(1.0, 0.8), p(6.0, 0.8)).is_none());
        assert!(travel_time(&m, p(1.0, 0.75), p(6.0, 0.75)).is_some());
        m.surfaces.fill(Surface::Road);
        let t = travel_time(&m, p(1.0, 3.0), p(6.0, 3.0)).unwrap();
        assert!((t - 5.0 / (2.0 * 1.15)).abs() < 1e-8);
        let (end, spent) = advance_toward(&m, p(1.0, 3.0), p(6.0, 3.0), 1.0);
        assert!((end.x - 3.3).abs() < 1e-8 && (spent - 1.0).abs() < 1e-8);
    }
    #[test]
    fn steep_slope_cannot_be_crossed() {
        let mut m = ArenaMap::flat(3, 2);
        for y in 0..=2 {
            m.elevations[y * 4 + 1] = 1.0;
        }
        assert!(travel_time(&m, p(0.25, 0.5), p(1.0, 0.5)).is_none());
    }
    #[test]
    fn exact_detection_limit_across_grid_corners() {
        let m = ArenaMap::flat(20, 20);
        let a = p(1.0, 1.0);
        let b = p(1.0 + 14.0 / 2.0_f64.sqrt(), 1.0 + 14.0 / 2.0_f64.sqrt());
        assert!(detectable(&m, a, b, false));
        assert!(!detectable(&m, a, p(b.x + 0.01, b.y + 0.01), false));
    }
    #[test]
    fn depressed_target_is_hidden_by_ground() {
        let mut m = ArenaMap::flat(8, 4);
        for y in 0..=4 {
            for x in 6..=8 {
                m.elevations[y * 9 + x] = -4.0;
            }
        }
        assert!(!clear(&m, p(1.0, 1.5), p(6.5, 1.5)));
    }
    #[test]
    fn elevated_observer_sees_over_full_obstacle() {
        let mut m = ArenaMap::flat(8, 4);
        for y in 0..=4 {
            for x in 0..=2 {
                m.elevations[y * 9 + x] = 4.0;
            }
        }
        m.obstacles.push(Obstacle {
            x: 4,
            y: 1,
            width: 1,
            height: 1,
            height_metres: 2.4,
        });
        assert!(clear(&m, p(1.0, 1.5), p(7.0, 1.5)));
        assert!(!clear(&m, p(3.5, 1.5), p(7.0, 1.5)));
    }
    #[test]
    fn partial_cover_is_directional_and_full_cover_blocks_both_directions() {
        let mut m = ArenaMap::flat(8, 5);
        m.obstacles.push(Obstacle {
            x: 4,
            y: 1,
            width: 1,
            height: 1,
            height_metres: 0.9,
        });
        let a = p(2.0, 1.5);
        let b = p(6.0, 1.5);
        let partial = shot_fraction(&m, a, b);
        assert!(partial > 0.0 && partial < 1.0);
        assert_eq!(shot_fraction(&m, p(5.5, 3.5), b), 1.0);
        m.obstacles[0].height_metres = 2.4;
        assert_eq!(shot_fraction(&m, a, b), 0.0);
        assert_eq!(shot_fraction(&m, b, a), 0.0);
    }
    #[test]
    fn grazing_obstacle_corner_blocks_sight_in_both_directions() {
        let mut m = ArenaMap::flat(8, 5);
        m.obstacles.push(Obstacle {
            x: 3,
            y: 2,
            width: 1,
            height: 1,
            height_metres: 2.4,
        });
        let a = p(2.0, 3.0);
        let b = p(4.0, 1.0);
        assert!(!clear(&m, a, b));
        assert!(!clear(&m, b, a));
        assert!(clear(&m, p(2.0, 2.99), p(4.0, 0.99)));
    }
    #[test]
    fn canopy_cost_integrates_crossings_within_triangles() {
        let mut m = ArenaMap::flat(6, 3);
        m.surfaces.fill(Surface::Brush);
        for y in 0..=3 {
            for (x, h) in [(1, 0.0), (2, -0.2), (3, -0.4), (4, -0.2), (5, 0.0)] {
                m.elevations[y * 7 + x] = h;
            }
        }
        assert!((spotting_cost(&m, p(1.0, 1.5), p(5.0, 1.5)) - 6.0).abs() < 1e-8);
    }
    #[test]
    fn slope_multipliers_and_cap_follow_direction() {
        let mut m = ArenaMap::flat(4, 2);
        for y in 0..=2 {
            for x in 0..=4 {
                m.elevations[y * 5 + x] = 0.2 * x as f64;
            }
        }
        assert!((movement_multiplier(&m, p(1.0, 0.5), p(2.0, 0.5)) - 1.0 / 1.4).abs() < 1e-8);
        assert!((movement_multiplier(&m, p(2.0, 0.5), p(1.0, 0.5)) - 1.15).abs() < 1e-8);
        m.surfaces.fill(Surface::Road);
        assert!((movement_multiplier(&m, p(2.0, 0.5), p(1.0, 0.5)) - 1.3).abs() < 1e-8);
        for y in 0..=2 {
            for x in 0..=4 {
                m.elevations[y * 5 + x] = 0.6 * x as f64;
            }
        }
        m.surfaces.fill(Surface::Open);
        assert!((movement_multiplier(&m, p(2.0, 0.5), p(1.0, 0.5)) - 0.55).abs() < 1e-8);
        assert!(travel_time(&m, p(1.0, 0.5), p(2.0, 0.5)).is_some());
        for y in 0..=2 {
            for x in 0..=4 {
                m.elevations[y * 5 + x] = 0.61 * x as f64;
            }
        }
        assert!(travel_time(&m, p(1.0, 0.5), p(2.0, 0.5)).is_none());
        assert!(travel_time(&m, p(1.5, 0.5), p(1.5, 1.5)).is_some());
    }
    #[test]
    fn movement_integrates_road_mud_boundary_without_overshoot() {
        let mut m = ArenaMap::flat(4, 2);
        for y in 0..2 {
            m.surfaces[y * 4 + 1] = Surface::Road;
            m.surfaces[y * 4 + 2] = Surface::Mud;
        }
        let a = p(1.5, 0.5);
        let b = p(2.5, 0.5);
        let expected = 0.5 / (2.0 * 1.15) + 0.5 / (2.0 * 0.6);
        assert!((travel_time(&m, a, b).unwrap() - expected).abs() < 1e-8);
        let (q, used) = advance_toward(&m, a, b, 0.5 / (2.0 * 1.15));
        assert!((q.x - 2.0).abs() < 1e-8 && (used - 0.5 / (2.0 * 1.15)).abs() < 1e-8);
        let (q, used) = advance_toward(&m, a, b, 10.0);
        assert_eq!(q, b);
        assert!((used - expected).abs() < 1e-8);
    }
}
