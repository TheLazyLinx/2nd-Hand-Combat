//! Frozen test-only reference oracles for terrain geometry and weighted grid search.
//!
//! Phase 1+ optimizations in `map.rs` and `navigation.rs` (terrain caches, grid ray traversal,
//! precomputed travel-time tables, allocation-free breaks, A* search) must match these
//! unoptimized reference implementations on randomized queries and boundary cases.

use crate::{
    definitions::{catalog, surface_definition, SPEED},
    map, navigation,
    types::*,
};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

const UNIT_RADIUS: f64 = 0.25;
const EPS: f64 = 1e-9;

fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
    }
}

fn oracle_inside(m: &ArenaMap, p: Point) -> bool {
    p.x.is_finite()
        && p.y.is_finite()
        && p.x >= UNIT_RADIUS
        && p.y >= UNIT_RADIUS
        && p.x <= m.width as f64 - UNIT_RADIUS
        && p.y <= m.height as f64 - UNIT_RADIUS
}

fn oracle_blocked(m: &ArenaMap, p: Point) -> bool {
    !oracle_inside(m, p)
        || m.obstacles.iter().any(|o| {
            let x = p.x.clamp(o.x as f64, (o.x.saturating_add(o.width)) as f64);
            let y = p.y.clamp(o.y as f64, (o.y.saturating_add(o.height)) as f64);
            (p.x - x).powi(2) + (p.y - y).powi(2) < UNIT_RADIUS.powi(2) - EPS
        })
}

fn oracle_surface(m: &ArenaMap, p: Point) -> Surface {
    let x = (p.x.floor() as usize).min(m.width as usize - 1);
    let y = (p.y.floor() as usize).min(m.height as usize - 1);
    m.surfaces[y * m.width as usize + x]
}

fn oracle_ground_height(m: &ArenaMap, p: Point) -> f64 {
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

fn oracle_breaks(m: &ArenaMap, a: Point, b: Point) -> Vec<f64> {
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

fn oracle_rect_interval(
    a: Point,
    b: Point,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Option<(f64, f64)> {
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

fn oracle_visible(m: &ArenaMap, a: Point, b: Point, ah: f64, bh: f64) -> bool {
    let za = oracle_ground_height(m, a) + ah;
    let zb = oracle_ground_height(m, b) + bh;
    let cuts = oracle_breaks(m, a, b);
    for t in &cuts {
        let z = za + (zb - za) * t;
        if oracle_ground_height(m, lerp(a, b, *t)) > z + EPS {
            return false;
        }
    }
    for o in &m.obstacles {
        if let Some((lo, hi)) = oracle_rect_interval(
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
                if oracle_ground_height(m, lerp(a, b, t)) + o.height_metres
                    >= za + (zb - za) * t - EPS
                {
                    return false;
                }
            }
        }
    }
    true
}

fn oracle_spotting_cost(m: &ArenaMap, a: Point, b: Point) -> f64 {
    let cuts = oracle_breaks(m, a, b);
    let len = a.distance(b);
    let za = oracle_ground_height(m, a) + 1.6;
    let zb = oracle_ground_height(m, b) + 1.6;
    let mut cost = 0.0;
    for pair in cuts.windows(2) {
        let mid = (pair[0] + pair[1]) / 2.0;
        let def = surface_definition(oracle_surface(m, lerp(a, b, mid)));
        let mut fraction = 0.0;
        if def.vegetation_height > 0.0 {
            let delta = |t: f64| {
                oracle_ground_height(m, lerp(a, b, t)) + def.vegetation_height
                    - (za + (zb - za) * t)
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

fn oracle_exposure(m: &ArenaMap, a: Point, b: Point, revealed: bool) -> f64 {
    let d = a.distance(b);
    let detectable = d <= 14.0
        && oracle_visible(m, a, b, 1.6, 1.6)
        && (revealed || d <= 2.0 || oracle_spotting_cost(m, a, b) <= 14.0 + EPS);
    if !detectable {
        return 0.0;
    }
    [0.4, 0.9, 1.4]
        .iter()
        .filter(|h| oracle_visible(m, a, b, 1.4, **h))
        .count() as f64
        / 3.0
}

fn oracle_multiplier(m: &ArenaMap, a: Point, b: Point) -> f64 {
    let d = a.distance(b);
    if d < EPS {
        return surface_definition(oracle_surface(m, a)).movement_multiplier;
    }
    let grade = (oracle_ground_height(m, b) - oracle_ground_height(m, a)) / d;
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
    (slope * surface_definition(oracle_surface(m, lerp(a, b, 0.5))).movement_multiplier).min(1.3)
}

fn oracle_segment_point_distance(a: Point, b: Point, p: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let l = dx * dx + dy * dy;
    let t = if l < EPS {
        0.0
    } else {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / l
    };
    lerp(a, b, t.clamp(0.0, 1.0)).distance(p)
}

fn oracle_sweep_clear(m: &ArenaMap, a: Point, b: Point) -> bool {
    if oracle_blocked(m, a) || oracle_blocked(m, b) {
        return false;
    }
    for o in &m.obstacles {
        let (x, y, x1, y1) = (
            o.x as f64,
            o.y as f64,
            o.x.saturating_add(o.width) as f64,
            o.y.saturating_add(o.height) as f64,
        );
        if oracle_rect_interval(a, b, x - UNIT_RADIUS + EPS, y, x1 + UNIT_RADIUS - EPS, y1)
            .is_some()
            || oracle_rect_interval(a, b, x, y - UNIT_RADIUS + EPS, x1, y1 + UNIT_RADIUS - EPS)
                .is_some()
            || [
                Point { x, y },
                Point { x: x1, y },
                Point { x, y: y1 },
                Point { x: x1, y: y1 },
            ]
            .iter()
            .any(|p| oracle_segment_point_distance(a, b, *p) < UNIT_RADIUS - EPS)
        {
            return false;
        }
    }
    true
}

fn oracle_travel_time(m: &ArenaMap, a: Point, b: Point) -> Option<f64> {
    if !oracle_sweep_clear(m, a, b) {
        return None;
    }
    let mut time = 0.0;
    for pair in oracle_breaks(m, a, b).windows(2) {
        let p = lerp(a, b, pair[0]);
        let q = lerp(a, b, pair[1]);
        let speed = oracle_multiplier(m, p, q);
        if speed <= 0.0 {
            return None;
        }
        time += p.distance(q) / (speed * SPEED);
    }
    Some(time)
}

#[derive(Clone, Copy)]
struct OracleEntry {
    cost: f64,
    index: usize,
}
impl PartialEq for OracleEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.index == other.index
    }
}
impl Eq for OracleEntry {}
impl PartialOrd for OracleEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OracleEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.index.cmp(&self.index))
    }
}

fn oracle_edge_cost(
    m: &ArenaMap,
    a: Point,
    b: Point,
    time: f64,
    threats: &[Point],
    preference: RoutePreference,
    revealed: bool,
) -> f64 {
    if preference == RoutePreference::Fastest || threats.is_empty() {
        return time;
    }
    let mid = Point {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
    };
    let risk = [a, mid, b]
        .into_iter()
        .map(|p| {
            threats
                .iter()
                .map(|&enemy| oracle_exposure(m, enemy, p, revealed))
                .fold(0.0_f64, f64::max)
        })
        .fold(0.0_f64, f64::max);
    time * (1.0 + 2.0 * risk)
}

fn oracle_search_costs(
    m: &ArenaMap,
    start: Point,
    threats: &[Point],
    preference: RoutePreference,
    revealed: bool,
) -> Vec<f64> {
    let width = m.width as usize;
    let height = m.height as usize;
    let center = |idx: usize| Point {
        x: (idx % width) as f64 + 0.5,
        y: (idx / width) as f64 + 0.5,
    };
    let mut costs = vec![f64::INFINITY; width * height];
    let mut queue = BinaryHeap::new();
    let sx = start.x.floor() as i32;
    let sy = start.y.floor() as i32;
    for y in (sy - 1)..=(sy + 1) {
        for x in (sx - 1)..=(sx + 1) {
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let index = y as usize * width + x as usize;
            let point = center(index);
            if let Some(time) = oracle_travel_time(m, start, point) {
                let cost = oracle_edge_cost(m, start, point, time, threats, preference, revealed);
                costs[index] = cost;
                queue.push(OracleEntry { cost, index });
            }
        }
    }
    while let Some(OracleEntry { cost, index }) = queue.pop() {
        if cost > costs[index] {
            continue;
        }
        let x = (index % width) as i32;
        let y = (index / width) as i32;
        let from = center(index);
        for (dx, dy) in [
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (-1, -1),
            (1, -1),
        ] {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let next = ny as usize * width + nx as usize;
            let to = center(next);
            let Some(time) = oracle_travel_time(m, from, to) else {
                continue;
            };
            let edge = oracle_edge_cost(m, from, to, time, threats, preference, revealed);
            let candidate = cost + edge;
            if candidate + 1e-10 < costs[next] {
                costs[next] = candidate;
                queue.push(OracleEntry {
                    cost: candidate,
                    index: next,
                });
            }
        }
    }
    costs
}

struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
    }
    fn point(&mut self, w: u32, h: u32) -> Point {
        Point {
            x: 0.3 + self.next_f64() * (w as f64 - 0.6),
            y: 0.3 + self.next_f64() * (h as f64 - 0.6),
        }
    }
}

fn generated_test_map(seed: u64, width: u32, height: u32) -> ArenaMap {
    let mut rng = Rng(seed);
    let mut map = ArenaMap::flat(width, height);
    let surfaces = [
        Surface::Open,
        Surface::Road,
        Surface::Mud,
        Surface::Brush,
        Surface::Woodland,
    ];
    for cell in &mut map.surfaces {
        *cell = surfaces[(rng.next_u64() as usize) % surfaces.len()];
    }
    for y in 0..=height as usize {
        for x in 0..=width as usize {
            let hill = ((x as f64 * 0.45).sin() + (y as f64 * 0.5).cos()) * 0.7;
            let bump = (rng.next_f64() - 0.5) * 0.35;
            map.elevations[y * (width as usize + 1) + x] = hill + bump;
        }
    }
    // Add a steep cliff patch to exercise impassable slope edges.
    if width >= 10 && height >= 8 {
        for y in 2..=5 {
            map.elevations[y * (width as usize + 1) + 5] += 1.4;
        }
    }
    // Add non-overlapping 1x1 and 2x1 obstacles of mixed heights.
    let mut occupied = vec![false; (width * height) as usize];
    for _ in 0..((width * height) / 10) {
        let ow = if rng.next_u64().is_multiple_of(4) {
            2
        } else {
            1
        };
        let oh = 1;
        let ox = 1 + (rng.next_u64() as u32 % (width - ow - 1));
        let oy = 1 + (rng.next_u64() as u32 % (height - oh - 1));
        let free =
            (oy..oy + oh).all(|cy| (ox..ox + ow).all(|cx| !occupied[(cy * width + cx) as usize]));
        if free {
            for cy in oy..oy + oh {
                for cx in ox..ox + ow {
                    occupied[(cy * width + cx) as usize] = true;
                }
            }
            map.obstacles.push(Obstacle {
                x: ox,
                y: oy,
                width: ow,
                height: oh,
                height_metres: if rng.next_u64().is_multiple_of(2) {
                    0.9
                } else {
                    2.4
                },
            });
        }
    }
    map
}

#[test]
fn geometry_queries_match_reference_oracles_across_presets_and_generated_maps() {
    let mut maps: Vec<(String, ArenaMap)> = catalog()
        .maps
        .into_iter()
        .map(|preset| (preset.name, preset.map))
        .collect();
    maps.push(("generated-14x10".into(), generated_test_map(101, 14, 10)));
    maps.push(("generated-18x12".into(), generated_test_map(202, 18, 12)));

    let mut rng = Rng(0x5eed_cafe_1234);
    for (name, m) in &maps {
        for _ in 0..80 {
            let a = rng.point(m.width, m.height);
            let b = rng.point(m.width, m.height);
            assert_eq!(map::blocked(m, a), oracle_blocked(m, a), "{name} blocked");
            for (ah, bh) in [(1.6, 1.6), (1.4, 0.9), (0.1, 1.0)] {
                assert_eq!(
                    map::visible(m, a, b, ah, bh),
                    oracle_visible(m, a, b, ah, bh),
                    "{name} visible({a:?}, {b:?}, {ah}, {bh})"
                );
            }
            for revealed in [false, true] {
                let actual = map::exposure(m, a, b, revealed);
                let expected = oracle_exposure(m, a, b, revealed);
                assert!(
                    (actual - expected).abs() < 1e-9,
                    "{name} exposure({a:?}, {b:?}, {revealed}): actual={actual} expected={expected}"
                );
            }
            let actual_tt = map::travel_time(m, a, b);
            let expected_tt = oracle_travel_time(m, a, b);
            match (actual_tt, expected_tt) {
                (Some(t1), Some(t2)) => assert!(
                    (t1 - t2).abs() < 1e-9,
                    "{name} travel_time({a:?}, {b:?}): actual={t1} expected={t2}"
                ),
                (None, None) => {}
                _ => panic!(
                    "{name} travel_time({a:?}, {b:?}): actual={actual_tt:?} expected={expected_tt:?}"
                ),
            }
        }
    }
}

#[test]
fn search_matches_reference_oracle_for_fastest_protected_and_targeted_queries() {
    let mut maps: Vec<(String, ArenaMap)> = catalog()
        .maps
        .into_iter()
        .map(|preset| (preset.name, preset.map))
        .collect();
    maps.push(("generated-14x10".into(), generated_test_map(303, 14, 10)));

    for (name, m) in &maps {
        let start = Point { x: 2.5, y: 2.5 };
        let threats = [
            Point {
                x: m.width as f64 - 3.5,
                y: 2.5,
            },
            Point {
                x: (m.width / 2) as f64 + 0.5,
                y: (m.height / 2) as f64 + 0.5,
            },
        ];
        for (pref, active_threats, revealed) in [
            (RoutePreference::Fastest, &[][..], false),
            (RoutePreference::Protected, &threats[..1], false),
            (RoutePreference::Protected, &threats[..], true),
        ] {
            let expected = oracle_search_costs(m, start, active_threats, pref, revealed);
            let actual = navigation::Search::with_threats(m, start, active_threats, pref, revealed);
            for y in 0..m.height {
                for x in 0..m.width {
                    let p = Point {
                        x: x as f64 + 0.5,
                        y: y as f64 + 0.5,
                    };
                    let idx = (y * m.width + x) as usize;
                    let exp = expected[idx];
                    match (actual.cost(p), exp.is_finite()) {
                        (Some(got), true) => assert!(
                            (got - exp).abs() < 1e-7,
                            "{name} {pref:?} cell ({x},{y}): actual={got} expected={exp}"
                        ),
                        (None, false) => {}
                        (got, finite) => panic!(
                            "{name} {pref:?} cell ({x},{y}): actual={got:?} expected_finite={finite}"
                        ),
                    }
                }
            }
            // Verify targeted search agrees with full oracle on selected destinations.
            let targets = [
                Point {
                    x: m.width as f64 - 2.5,
                    y: m.height as f64 - 2.5,
                },
                Point {
                    x: (m.width / 2) as f64 + 0.5,
                    y: 2.5,
                },
            ];
            let targeted =
                navigation::Search::for_targets(m, start, active_threats, pref, revealed, &targets);
            for target in targets {
                let idx = (target.y.floor() as u32 * m.width + target.x.floor() as u32) as usize;
                let exp = expected[idx];
                match (targeted.cost(target), exp.is_finite()) {
                    (Some(got), true) => assert!(
                        (got - exp).abs() < 1e-7,
                        "{name} targeted {pref:?} {target:?}: actual={got} expected={exp}"
                    ),
                    (None, false) => {}
                    (got, finite) => panic!(
                        "{name} targeted {pref:?} {target:?}: actual={got:?} expected_finite={finite}"
                    ),
                }
            }
        }
    }
}

#[test]
fn astar_single_goal_matches_dijkstra_oracle_costs_across_presets_and_generated_maps() {
    let mut maps: Vec<(String, ArenaMap)> = catalog()
        .maps
        .into_iter()
        .map(|preset| (preset.name, preset.map))
        .collect();
    maps.push(("generated-14x10".into(), generated_test_map(404, 14, 10)));

    for (name, m) in &maps {
        let cache = crate::terrain::TerrainCache::new(m);
        let terrain = cache.view(m);
        let mut scratch = navigation::SearchScratch::new(cache.width, cache.height);
        let start = Point { x: 2.5, y: 2.5 };
        let threats = [
            Point {
                x: m.width as f64 - 3.5,
                y: 2.5,
            },
            Point {
                x: (m.width / 2) as f64 + 0.5,
                y: (m.height / 2) as f64 + 0.5,
            },
        ];
        let goals = [
            Point {
                x: m.width as f64 - 2.5,
                y: m.height as f64 - 2.5,
            },
            Point {
                x: (m.width / 2) as f64 + 0.5,
                y: 2.5,
            },
            Point {
                x: 2.5,
                y: m.height as f64 - 2.5,
            },
        ];
        for (pref, active_threats, revealed) in [
            (RoutePreference::Fastest, &[][..], false),
            (RoutePreference::Protected, &threats[..1], false),
            (RoutePreference::Protected, &threats[..], true),
        ] {
            let expected = oracle_search_costs(m, start, active_threats, pref, revealed);
            for goal in goals {
                scratch.run_astar(terrain, start, goal, active_threats, pref, revealed);
                let idx = (goal.y.floor() as u32 * m.width + goal.x.floor() as u32) as usize;
                let exp = expected[idx];
                match (scratch.cost(goal), exp.is_finite()) {
                    (Some(got), true) => assert!(
                        (got - exp).abs() < 1e-7,
                        "{name} astar {pref:?} {goal:?}: actual={got} expected={exp}"
                    ),
                    (None, false) => {}
                    (got, finite) => panic!(
                        "{name} astar {pref:?} {goal:?}: actual={got:?} expected_finite={finite}"
                    ),
                }
            }
        }
    }
}

#[test]
fn single_pass_advance_toward_matches_whole_segment_validity_and_travel_time() {
    let mut m = ArenaMap::flat(6, 2);
    // Steep cliff in column 3 (x in 3..4) while columns 0..3 are flat open ground.
    for y in 0..=2 {
        for x in 4..=6 {
            m.elevations[y * 7 + x] = 2.0;
        }
    }
    let cache = crate::terrain::TerrainCache::without_neighbors(&m);
    let terrain = cache.view(&m);
    let a = Point { x: 0.5, y: 1.0 };
    let b = Point { x: 4.5, y: 1.0 };
    assert!(oracle_travel_time(&m, a, b).is_none());
    // Even with a tiny budget that would finish inside the flat first cell, whole-segment steepness rejects movement.
    assert_eq!(map::advance_toward_t(terrain, a, b, 0.05), (a, 0.0));

    let mid = Point { x: 2.5, y: 1.0 };
    let expected = oracle_travel_time(&m, a, mid).unwrap();
    let (end, spent) = map::advance_toward_t(terrain, a, mid, expected + 1.0);
    assert_eq!(end, mid);
    assert!((spent - expected).abs() < 1e-9);
}
