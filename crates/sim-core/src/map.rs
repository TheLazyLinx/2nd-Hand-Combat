//! Terrain geometry queries: bounds, grid-cached obstacle checks, unified ray visibility/exposure,
//! vegetation spotting cost, swept-radius clearance, travel time, and single-pass advancement.
//!
//! Invariant: all `*_t` queries backed by `TerrainCache` match the reference oracles in `oracles.rs`
//! (solid obstacles block at `>=`, ground blocks at `>`, corner grazing blocks both directions).

use crate::{
    definitions::{surface_definition, SPEED},
    terrain::Terrain,
    types::*,
};

#[cfg(test)]
use crate::terrain::TerrainCache;

pub(crate) const UNIT_RADIUS: f64 = 0.25;
const EPS: f64 = 1e-9;

/// Stack-backed sorted break points along a segment; avoids heap allocation for normal ray lengths.
struct Cuts {
    inline: [f64; 96],
    len: usize,
    overflow: Vec<f64>,
}

impl Cuts {
    #[inline]
    fn new() -> Self {
        let mut inline = [0.0; 96];
        inline[0] = 0.0;
        inline[1] = 1.0;
        Self {
            inline,
            len: 2,
            overflow: Vec::new(),
        }
    }

    #[inline]
    fn push(&mut self, value: f64) {
        if self.overflow.is_empty() {
            if self.len < self.inline.len() {
                self.inline[self.len] = value;
                self.len += 1;
            } else {
                self.overflow.reserve(self.inline.len() * 2);
                self.overflow.extend_from_slice(&self.inline[..self.len]);
                self.overflow.push(value);
            }
        } else {
            self.overflow.push(value);
        }
    }

    fn sort_and_dedup(&mut self) {
        if self.overflow.is_empty() {
            let slice = &mut self.inline[..self.len];
            slice.sort_by(|a, b| a.total_cmp(b));
            if self.len > 1 {
                let mut write = 1usize;
                for read in 1..self.len {
                    if (slice[read] - slice[write - 1]).abs() >= EPS {
                        slice[write] = slice[read];
                        write += 1;
                    }
                }
                self.len = write;
            }
        } else {
            self.overflow.sort_by(|a, b| a.total_cmp(b));
            self.overflow.dedup_by(|a, b| (*a - *b).abs() < EPS);
        }
    }

    #[inline]
    fn as_slice(&self) -> &[f64] {
        if self.overflow.is_empty() {
            &self.inline[..self.len]
        } else {
            self.overflow.as_slice()
        }
    }
}

#[inline]
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

pub(crate) fn blocked_t(t: Terrain<'_>, p: Point) -> bool {
    if !inside(t.map, p) {
        return true;
    }
    if !t.cache.has_obstacles {
        return false;
    }
    let w = t.cache.width as i32;
    let h = t.cache.height as i32;
    let x0 = ((p.x - UNIT_RADIUS - EPS).floor() as i32).max(0);
    let x1 = ((p.x + UNIT_RADIUS + EPS).floor() as i32).min(w - 1);
    let y0 = ((p.y - UNIT_RADIUS - EPS).floor() as i32).max(0);
    let y1 = ((p.y + UNIT_RADIUS + EPS).floor() as i32).min(h - 1);
    let limit = UNIT_RADIUS * UNIT_RADIUS - EPS;
    for cy in y0..=y1 {
        let row = cy as usize * t.cache.width;
        let fy = cy as f64;
        let dy = p.y - p.y.clamp(fy, fy + 1.0);
        let dy2 = dy * dy;
        for cx in x0..=x1 {
            if t.cache.occupied[row + cx as usize] {
                let fx = cx as f64;
                let dx = p.x - p.x.clamp(fx, fx + 1.0);
                if dx * dx + dy2 < limit {
                    return true;
                }
            }
        }
    }
    false
}

#[inline]
pub(crate) fn surface(m: &ArenaMap, p: Point) -> Surface {
    let x = (p.x.floor() as usize).min(m.width as usize - 1);
    let y = (p.y.floor() as usize).min(m.height as usize - 1);
    m.surfaces[y * m.width as usize + x]
}

#[inline]
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
fn breaks(m: &ArenaMap, a: Point, b: Point) -> Cuts {
    let mut ts = Cuts::new();
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx.abs() > EPS {
        let inv_dx = 1.0 / dx;
        let x0 = a.x.min(b.x).floor().max(0.0) as u32;
        let x1 = a.x.max(b.x).ceil().min(m.width as f64) as u32;
        for x in x0..=x1 {
            let t = (x as f64 - a.x) * inv_dx;
            if t > EPS && t < 1.0 - EPS {
                ts.push(t);
            }
        }
    }
    if dy.abs() > EPS {
        let inv_dy = 1.0 / dy;
        let y0 = a.y.min(b.y).floor().max(0.0) as u32;
        let y1 = a.y.max(b.y).ceil().min(m.height as f64) as u32;
        for y in y0..=y1 {
            let t = (y as f64 - a.y) * inv_dy;
            if t > EPS && t < 1.0 - EPS {
                ts.push(t);
            }
        }
    }
    if (dx - dy).abs() > EPS {
        let inv_d = 1.0 / (dx - dy);
        let d0 = (a.x - a.y).min(b.x - b.y).floor() as i32;
        let d1 = (a.x - a.y).max(b.x - b.y).ceil() as i32;
        for k in d0..=d1 {
            let t = (k as f64 - (a.x - a.y)) * inv_d;
            if t > EPS && t < 1.0 - EPS {
                ts.push(t);
            }
        }
    }
    ts.sort_and_dedup();
    ts
}

#[inline]
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

/// Shared ground + grid-obstacle ray evaluator for N simultaneous rays sharing horizontal path `a -> b`.
/// When `require_first` is true, returns early as soon as ray 0 (eye sight) is blocked.
fn eval_rays_t<const N: usize>(
    t: Terrain<'_>,
    a: Point,
    b: Point,
    cuts: &[f64],
    ray_a: [f64; N],
    ray_b: [f64; N],
    require_first: bool,
) -> [bool; N] {
    let cache = t.cache;
    let m = t.map;
    let w = cache.width as i32;
    let h = cache.height as i32;
    let mut unblocked = [true; N];
    let mut open_count = N;

    let mut test = |u: f64, top: f64, solid: bool| -> bool {
        for idx in 0..N {
            if !unblocked[idx] {
                continue;
            }
            let z = ray_a[idx] + (ray_b[idx] - ray_a[idx]) * u;
            let hit = if solid { top >= z - EPS } else { top > z + EPS };
            if hit {
                unblocked[idx] = false;
                open_count -= 1;
                if (require_first && idx == 0) || open_count == 0 {
                    return false;
                }
            }
        }
        true
    };

    if !cache.is_flat {
        for &u in cuts {
            if !test(u, ground_height(m, point(a, b, u)), false) {
                return unblocked;
            }
        }
    } else if !test(0.0, cache.flat_elevation, false) || !test(1.0, cache.flat_elevation, false) {
        return unblocked;
    }

    if !cache.has_obstacles {
        return unblocked;
    }

    let pad_x = EPS * (b.x - a.x).abs().max(1.0) * 4.0;
    let pad_y = EPS * (b.y - a.y).abs().max(1.0) * 4.0;

    for &u in cuts {
        let p = point(a, b, u);
        let x0 = ((p.x - pad_x).floor() as i32).max(0);
        let x1 = ((p.x + pad_x).floor() as i32).min(w - 1);
        let y0 = ((p.y - pad_y).floor() as i32).max(0);
        let y1 = ((p.y + pad_y).floor() as i32).min(h - 1);
        for cy in y0..=y1 {
            let row = cy as usize * cache.width;
            for cx in x0..=x1 {
                let cell = row + cx as usize;
                if !cache.occupied[cell] {
                    continue;
                }
                let fx = cx as f64;
                let fy = cy as f64;
                let Some((lo, hi)) = rect_interval(a, b, fx, fy, fx + 1.0, fy + 1.0) else {
                    continue;
                };
                let oh = cache.obstacle_height[cell];
                if u > lo && u < hi && !test(u, cache.ground_height(m, p) + oh, true) {
                    return unblocked;
                }
                if (u - lo).abs() <= EPS * 4.0
                    && !test(lo, cache.ground_height(m, point(a, b, lo)) + oh, true)
                {
                    return unblocked;
                }
                if (u - hi).abs() <= EPS * 4.0
                    && (hi - lo).abs() > 0.0
                    && !test(hi, cache.ground_height(m, point(a, b, hi)) + oh, true)
                {
                    return unblocked;
                }
            }
        }
    }
    unblocked
}

pub(crate) fn visible_t(t: Terrain<'_>, a: Point, b: Point, ah: f64, bh: f64) -> bool {
    let ga = t.cache.ground_height(t.map, a);
    let gb = t.cache.ground_height(t.map, b);
    let cuts = breaks(t.map, a, b);
    eval_rays_t(t, a, b, cuts.as_slice(), [ga + ah], [gb + bh], true)[0]
}

pub(crate) fn clear_t(t: Terrain<'_>, a: Point, b: Point) -> bool {
    visible_t(t, a, b, 1.6, 1.6)
}

pub(crate) fn shot_fraction_t(t: Terrain<'_>, a: Point, b: Point) -> f64 {
    let ga = t.cache.ground_height(t.map, a);
    let gb = t.cache.ground_height(t.map, b);
    let cuts = breaks(t.map, a, b);
    let open = eval_rays_t(
        t,
        a,
        b,
        cuts.as_slice(),
        [ga + 1.4; 3],
        [gb + 0.4, gb + 0.9, gb + 1.4],
        false,
    );
    open.into_iter().filter(|ok| *ok).count() as f64 / 3.0
}

pub(crate) fn spotting_cost_t(t: Terrain<'_>, a: Point, b: Point) -> f64 {
    let cuts = breaks(t.map, a, b);
    spotting_cost_with_cuts_t(t, a, b, cuts.as_slice())
}

fn spotting_cost_with_cuts_t(t: Terrain<'_>, a: Point, b: Point, cuts: &[f64]) -> f64 {
    let len = a.distance(b);
    let ga = t.cache.ground_height(t.map, a);
    let gb = t.cache.ground_height(t.map, b);
    let za = ga + 1.6;
    let zb = gb + 1.6;
    let is_flat = t.cache.is_flat;
    let mut cost = 0.0;
    for pair in cuts.windows(2) {
        let mid = (pair[0] + pair[1]) * 0.5;
        let def = t.cache.cell_surface(point(a, b, mid));
        let mut fraction = 0.0;
        if def.veg_height > 0.0 {
            if is_flat {
                fraction = if def.veg_height >= 1.6 { 1.0 } else { 0.0 };
            } else {
                let delta = |u: f64| {
                    ground_height(t.map, point(a, b, u)) + def.veg_height - (za + (zb - za) * u)
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
        }
        cost += len * (pair[1] - pair[0]) * (1.0 + (def.spot_mult - 1.0) * fraction);
    }
    cost
}

pub(crate) fn detectable_t(t: Terrain<'_>, a: Point, b: Point, revealed: bool) -> bool {
    let distance = a.distance(b);
    if distance > 14.0 {
        return false;
    }
    let ga = t.cache.ground_height(t.map, a);
    let gb = t.cache.ground_height(t.map, b);
    let cuts = breaks(t.map, a, b);
    if !eval_rays_t(t, a, b, cuts.as_slice(), [ga + 1.6], [gb + 1.6], true)[0] {
        return false;
    }
    revealed || distance <= 2.0 || spotting_cost_with_cuts_t(t, a, b, cuts.as_slice()) <= 14.0 + EPS
}

/// Detection multiplied by target exposure, sharing break points and grid traversal for all four rays.
/// Matches `if detectable_t(t, a, b, revealed) { shot_fraction_t(t, a, b) } else { 0.0 }`.
pub(crate) fn exposure_t(t: Terrain<'_>, a: Point, b: Point, revealed: bool) -> f64 {
    let distance = a.distance(b);
    if distance > 14.0 {
        return 0.0;
    }
    let ga = t.cache.ground_height(t.map, a);
    let gb = t.cache.ground_height(t.map, b);
    let cuts = breaks(t.map, a, b);
    let open = eval_rays_t(
        t,
        a,
        b,
        cuts.as_slice(),
        [ga + 1.6, ga + 1.4, ga + 1.4, ga + 1.4],
        [gb + 1.6, gb + 0.4, gb + 0.9, gb + 1.4],
        true,
    );
    if !open[0] {
        return 0.0;
    }
    if !revealed
        && distance > 2.0
        && spotting_cost_with_cuts_t(t, a, b, cuts.as_slice()) > 14.0 + EPS
    {
        return 0.0;
    }
    open[1..].iter().filter(|ok| **ok).count() as f64 / 3.0
}

#[inline]
fn slope_factor(grade: f64) -> f64 {
    if grade.abs() > 0.6 + EPS {
        return 0.0;
    }
    if grade >= 0.0 {
        1.0 / (1.0 + 2.0 * grade)
    } else if grade >= -0.2 {
        1.0 + 0.15 * (-grade) / 0.2
    } else {
        1.15 - 0.6 * (-grade - 0.2) / 0.4
    }
}

#[inline]
fn multiplier_t(t: Terrain<'_>, a: Point, b: Point) -> f64 {
    let d = a.distance(b);
    if d < EPS {
        return t.cache.cell_surface(a).move_mult;
    }
    let move_mult = t.cache.cell_surface(point(a, b, 0.5)).move_mult;
    if t.cache.is_flat {
        return move_mult.min(1.3);
    }
    let grade = (ground_height(t.map, b) - ground_height(t.map, a)) / d;
    let slope = slope_factor(grade);
    if slope == 0.0 {
        return 0.0;
    }
    (slope * move_mult).min(1.3)
}

fn multiplier(m: &ArenaMap, a: Point, b: Point) -> f64 {
    let d = a.distance(b);
    if d < EPS {
        return surface_definition(surface(m, a)).movement_multiplier;
    }
    let grade = (ground_height(m, b) - ground_height(m, a)) / d;
    let slope = slope_factor(grade);
    if slope == 0.0 {
        return 0.0;
    }
    (slope * surface_definition(surface(m, point(a, b, 0.5))).movement_multiplier).min(1.3)
}

pub(crate) fn movement_multiplier(m: &ArenaMap, a: Point, b: Point) -> f64 {
    multiplier(m, a, b)
}

#[inline]
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

#[inline]
fn cell_blocks_sweep(a: Point, b: Point, x: f64, y: f64) -> bool {
    let x1 = x + 1.0;
    let y1 = y + 1.0;
    rect_interval(a, b, x - UNIT_RADIUS + EPS, y, x1 + UNIT_RADIUS - EPS, y1).is_some()
        || rect_interval(a, b, x, y - UNIT_RADIUS + EPS, x1, y1 + UNIT_RADIUS - EPS).is_some()
        || [
            Point { x, y },
            Point { x: x1, y },
            Point { x, y: y1 },
            Point { x: x1, y: y1 },
        ]
        .iter()
        .any(|p| segment_point_distance(a, b, *p) < UNIT_RADIUS - EPS)
}

fn sweep_clear_t(t: Terrain<'_>, a: Point, b: Point) -> bool {
    if blocked_t(t, a) || blocked_t(t, b) {
        return false;
    }
    if !t.cache.has_obstacles {
        return true;
    }
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let w = t.cache.width as i32;
    let h = t.cache.height as i32;
    let pad = UNIT_RADIUS + EPS;

    if dx.abs() >= dy.abs() {
        if dx.abs() < EPS {
            return true;
        }
        let inv_dx = 1.0 / dx;
        let cx_min = ((a.x.min(b.x) - pad).floor() as i32).max(0);
        let cx_max = ((a.x.max(b.x) + pad).floor() as i32).min(w - 1);
        for cx in cx_min..=cx_max {
            let fx = cx as f64;
            let t0 = ((fx - pad - a.x) * inv_dx).clamp(0.0, 1.0);
            let t1 = ((fx + 1.0 + pad - a.x) * inv_dx).clamp(0.0, 1.0);
            let y0 = a.y + dy * t0;
            let y1 = a.y + dy * t1;
            let cy_min = ((y0.min(y1) - pad).floor() as i32).max(0);
            let cy_max = ((y0.max(y1) + pad).floor() as i32).min(h - 1);
            for cy in cy_min..=cy_max {
                if t.cache.occupied[cy as usize * t.cache.width + cx as usize]
                    && cell_blocks_sweep(a, b, fx, cy as f64)
                {
                    return false;
                }
            }
        }
    } else {
        let inv_dy = 1.0 / dy;
        let cy_min = ((a.y.min(b.y) - pad).floor() as i32).max(0);
        let cy_max = ((a.y.max(b.y) + pad).floor() as i32).min(h - 1);
        for cy in cy_min..=cy_max {
            let fy = cy as f64;
            let t0 = ((fy - pad - a.y) * inv_dy).clamp(0.0, 1.0);
            let t1 = ((fy + 1.0 + pad - a.y) * inv_dy).clamp(0.0, 1.0);
            let x0 = a.x + dx * t0;
            let x1 = a.x + dx * t1;
            let cx_min = ((x0.min(x1) - pad).floor() as i32).max(0);
            let cx_max = ((x0.max(x1) + pad).floor() as i32).min(w - 1);
            let row = cy as usize * t.cache.width;
            for cx in cx_min..=cx_max {
                if t.cache.occupied[row + cx as usize] && cell_blocks_sweep(a, b, cx as f64, fy) {
                    return false;
                }
            }
        }
    }
    true
}

pub(crate) fn ground_traversable(m: &ArenaMap, a: Point, b: Point) -> bool {
    let cuts = breaks(m, a, b);
    cuts.as_slice()
        .windows(2)
        .all(|pair| multiplier(m, point(a, b, pair[0]), point(a, b, pair[1])) > 0.0)
}

/// Travel time along an already sweep-cleared segment `a -> b` (used to precompute neighbor edges).
pub(crate) fn segment_travel_time(t: Terrain<'_>, a: Point, b: Point) -> Option<f64> {
    let cuts = breaks(t.map, a, b);
    let mut time = 0.0;
    for pair in cuts.as_slice().windows(2) {
        let p = point(a, b, pair[0]);
        let q = point(a, b, pair[1]);
        let speed = multiplier_t(t, p, q);
        if speed <= 0.0 {
            return None;
        }
        time += p.distance(q) / (speed * SPEED);
    }
    Some(time)
}

pub(crate) fn travel_time_t(t: Terrain<'_>, a: Point, b: Point) -> Option<f64> {
    if !sweep_clear_t(t, a, b) {
        return None;
    }
    segment_travel_time(t, a, b)
}

/// Single-pass advancement along `a -> b` with time budget `budget`, preserving whole-segment validity.
pub(crate) fn advance_toward_t(t: Terrain<'_>, a: Point, b: Point, budget: f64) -> (Point, f64) {
    if budget <= 0.0 || !sweep_clear_t(t, a, b) {
        return (a, 0.0);
    }
    let cuts = breaks(t.map, a, b);
    let mut spent = 0.0;
    let mut stopped: Option<(Point, f64)> = None;
    for pair in cuts.as_slice().windows(2) {
        let p = point(a, b, pair[0]);
        let q = point(a, b, pair[1]);
        let speed = multiplier_t(t, p, q);
        if speed <= 0.0 {
            return (a, 0.0);
        }
        if stopped.is_none() {
            let time = p.distance(q) / (speed * SPEED);
            if spent + time > budget {
                stopped = Some((point(p, q, (budget - spent) / time), budget));
            } else {
                spent += time;
            }
        }
    }
    stopped.unwrap_or((b, spent))
}

// Convenience wrappers for standalone tests that operate on a raw `&ArenaMap`.
#[cfg(test)]
pub(crate) fn blocked(m: &ArenaMap, p: Point) -> bool {
    let cache = TerrainCache::without_neighbors(m);
    blocked_t(cache.view(m), p)
}
#[cfg(test)]
pub(crate) fn visible(m: &ArenaMap, a: Point, b: Point, ah: f64, bh: f64) -> bool {
    let cache = TerrainCache::without_neighbors(m);
    visible_t(cache.view(m), a, b, ah, bh)
}
#[cfg(test)]
pub(crate) fn clear(m: &ArenaMap, a: Point, b: Point) -> bool {
    let cache = TerrainCache::without_neighbors(m);
    clear_t(cache.view(m), a, b)
}
#[cfg(test)]
pub(crate) fn shot_fraction(m: &ArenaMap, a: Point, b: Point) -> f64 {
    let cache = TerrainCache::without_neighbors(m);
    shot_fraction_t(cache.view(m), a, b)
}
#[cfg(test)]
pub(crate) fn spotting_cost(m: &ArenaMap, a: Point, b: Point) -> f64 {
    let cache = TerrainCache::without_neighbors(m);
    spotting_cost_t(cache.view(m), a, b)
}
#[cfg(test)]
pub(crate) fn detectable(m: &ArenaMap, a: Point, b: Point, revealed: bool) -> bool {
    let cache = TerrainCache::without_neighbors(m);
    detectable_t(cache.view(m), a, b, revealed)
}
#[cfg(test)]
pub(crate) fn exposure(m: &ArenaMap, a: Point, b: Point, revealed: bool) -> f64 {
    let cache = TerrainCache::without_neighbors(m);
    exposure_t(cache.view(m), a, b, revealed)
}
#[cfg(test)]
pub(crate) fn travel_time(m: &ArenaMap, a: Point, b: Point) -> Option<f64> {
    let cache = TerrainCache::without_neighbors(m);
    travel_time_t(cache.view(m), a, b)
}
#[cfg(test)]
pub(crate) fn advance_toward(m: &ArenaMap, a: Point, b: Point, budget: f64) -> (Point, f64) {
    let cache = TerrainCache::without_neighbors(m);
    advance_toward_t(cache.view(m), a, b, budget)
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
