use crate::types::*;

pub(crate) fn inside(map: &ArenaMap, p: Point) -> bool {
    p.x.is_finite()
        && p.y.is_finite()
        && p.x >= 0.25
        && p.y >= 0.25
        && p.x <= map.width as f64 - 0.25
        && p.y <= map.height as f64 - 0.25
}
pub(crate) fn blocked(map: &ArenaMap, p: Point) -> bool {
    !inside(map, p)
        || map
            .walls
            .iter()
            .filter(|w| w.x.is_finite() && w.y.is_finite())
            .any(|w| {
                let x = p.x.clamp(w.x, w.x + 1.0);
                let y = p.y.clamp(w.y, w.y + 1.0);
                (p.x - x).powi(2) + (p.y - y).powi(2) < 0.25_f64.powi(2)
            })
}
pub(crate) fn in_cover(map: &ArenaMap, p: Point) -> bool {
    map.cover
        .iter()
        .any(|c| p.x >= c.x && p.x < c.x + 1.0 && p.y >= c.y && p.y < c.y + 1.0)
}
pub(crate) fn clear(map: &ArenaMap, a: Point, b: Point) -> bool {
    // Intersect the segment with each wall's axis-aligned rectangle.
    !map.walls.iter().any(|w| {
        let mut lo: f64 = 0.0;
        let mut hi: f64 = 1.0;
        for (start, delta, min, max) in [
            (a.x, b.x - a.x, w.x, w.x + 1.0),
            (a.y, b.y - a.y, w.y, w.y + 1.0),
        ] {
            if delta.abs() < 1e-12 {
                if start >= min && start <= max {
                    continue;
                } else {
                    return false;
                }
            }
            let t1 = (min - start) / delta;
            let t2 = (max - start) / delta;
            lo = lo.max(t1.min(t2));
            hi = hi.min(t1.max(t2));
            if lo > hi {
                return false;
            }
        }
        true
    })
}
