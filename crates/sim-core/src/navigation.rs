use crate::{map, types::*};
use std::collections::VecDeque;

fn traversable(map: &ArenaMap, from: Point, to: Point) -> bool {
    let steps = (from.distance(to) / 0.1).ceil() as usize;
    (0..=steps).all(|i| {
        let t = i as f64 / steps.max(1) as f64;
        !map::blocked(
            map,
            Point {
                x: from.x + (to.x - from.x) * t,
                y: from.y + (to.y - from.y) * t,
            },
        )
    })
}

pub(crate) fn waypoint(map: &ArenaMap, from: Point, to: Point) -> Point {
    if traversable(map, from, to) {
        return to;
    }
    let w = map.width as usize;
    let h = map.height as usize;
    let cell = |p: Point| {
        (
            (p.x.floor() as usize).min(w - 1),
            (p.y.floor() as usize).min(h - 1),
        )
    };
    let (sx, sy) = cell(from);
    let (tx, ty) = cell(to);
    let start = sy * w + sx;
    let target = ty * w + tx;
    let mut prev = vec![usize::MAX; w * h];
    let mut queue = VecDeque::new();
    prev[start] = start;
    queue.push_back(start);
    while let Some(cur) = queue.pop_front() {
        if cur == target {
            break;
        }
        let x = cur % w;
        let y = cur / w;
        for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let next = ny as usize * w + nx as usize;
            let p = Point {
                x: nx as f64 + 0.5,
                y: ny as f64 + 0.5,
            };
            if prev[next] == usize::MAX && !map::blocked(map, p) {
                prev[next] = cur;
                queue.push_back(next);
            }
        }
    }
    let target = if prev[target] != usize::MAX {
        target
    } else {
        (0..prev.len())
            .filter(|i| prev[*i] != usize::MAX)
            .min_by(|a, b| {
                let point = |i: usize| Point {
                    x: (i % w) as f64 + 0.5,
                    y: (i / w) as f64 + 0.5,
                };
                point(*a).distance(to).total_cmp(&point(*b).distance(to))
            })
            .unwrap_or(start)
    };
    if target == start {
        return from;
    }
    let mut next = target;
    while prev[next] != start && next != start {
        next = prev[next];
    }
    let waypoint = Point {
        x: (next % w) as f64 + 0.5,
        y: (next / w) as f64 + 0.5,
    };
    if traversable(map, from, waypoint) {
        waypoint
    } else {
        Point {
            x: sx as f64 + 0.5,
            y: sy as f64 + 0.5,
        }
    }
}
pub(crate) fn move_toward(map: &ArenaMap, from: Point, to: Point) -> Point {
    let next = waypoint(map, from, to);
    let dist = from.distance(next);
    if dist < 1e-9 {
        return from;
    }
    let scale = (crate::definitions::SPEED * crate::definitions::STEP / dist).min(1.0);
    let p = Point {
        x: from.x + (next.x - from.x) * scale,
        y: from.y + (next.y - from.y) * scale,
    };
    if map::blocked(map, p) {
        from
    } else {
        p
    }
}

// A separate reachability check prevents choosing goals in isolated regions.
pub(crate) fn reachable(map: &ArenaMap, from: Point, to: Point) -> bool {
    if map::blocked(map, to) {
        return false;
    }
    if traversable(map, from, to) {
        return true;
    }
    let w = map.width as usize;
    let h = map.height as usize;
    let index =
        |p: Point| (p.y.floor() as usize).min(h - 1) * w + (p.x.floor() as usize).min(w - 1);
    let start = index(from);
    let target = index(to);
    let mut visited = vec![false; w * h];
    let mut queue = VecDeque::from([start]);
    visited[start] = true;
    while let Some(cur) = queue.pop_front() {
        if cur == target {
            return true;
        }
        let x = cur % w;
        let y = cur / w;
        for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let next = ny as usize * w + nx as usize;
            let p = Point {
                x: nx as f64 + 0.5,
                y: ny as f64 + 0.5,
            };
            if !visited[next] && !map::blocked(map, p) {
                visited[next] = true;
                queue.push_back(next);
            }
        }
    }
    false
}
