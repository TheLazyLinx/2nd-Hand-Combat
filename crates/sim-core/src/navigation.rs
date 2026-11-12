use crate::{map, types::*};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Default)]
pub(crate) struct Route {
    goal: Option<Point>,
    threats: Vec<Point>,
    revealed: bool,
    preference: Option<RoutePreference>,
    path: Vec<Point>,
    next: usize,
}

#[derive(Clone, Copy)]
struct Entry {
    cost: f64,
    index: usize,
}
impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.index == other.index
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.index.cmp(&self.index))
    }
}

pub(crate) struct Search {
    width: usize,
    start: Point,
    costs: Vec<f64>,
    previous: Vec<usize>,
    revealed: bool,
}
impl Search {
    #[cfg(test)]
    pub(crate) fn new(
        map: &ArenaMap,
        start: Point,
        threat: Option<Point>,
        preference: RoutePreference,
    ) -> Self {
        let threats: Vec<_> = threat.into_iter().collect();
        Self::with_threats(map, start, &threats, preference, false)
    }
    pub(crate) fn with_threats(
        map: &ArenaMap,
        start: Point,
        threats: &[Point],
        preference: RoutePreference,
        revealed: bool,
    ) -> Self {
        Self::run(map, start, threats, preference, revealed, None)
    }
    pub(crate) fn for_targets(
        map: &ArenaMap,
        start: Point,
        threats: &[Point],
        preference: RoutePreference,
        revealed: bool,
        targets: &[Point],
    ) -> Self {
        Self::run(map, start, threats, preference, revealed, Some(targets))
    }
    fn run(
        map: &ArenaMap,
        start: Point,
        threats: &[Point],
        preference: RoutePreference,
        revealed: bool,
        targets: Option<&[Point]>,
    ) -> Self {
        let width = map.width as usize;
        let height = map.height as usize;
        let mut remaining = vec![false; width * height];
        let mut count = 0;
        if let Some(targets) = targets {
            for p in targets {
                if p.x < 0.0 || p.y < 0.0 {
                    continue;
                }
                let x = p.x.floor() as usize;
                let y = p.y.floor() as usize;
                if x < width
                    && y < height
                    && !map::blocked(map, center(width, y * width + x))
                    && !remaining[y * width + x]
                {
                    remaining[y * width + x] = true;
                    count += 1;
                }
            }
        }
        let mut costs = vec![f64::INFINITY; width * height];
        let mut previous = vec![usize::MAX; width * height];
        let mut queue = BinaryHeap::new();
        if targets.is_some() && count == 0 {
            return Self {
                width,
                start,
                costs,
                previous,
                revealed,
            };
        }
        // A cell center or edge midpoint is shared by several edges. Cache the
        // threat exposure at each half-grid coordinate for this search.
        let risk_width = width * 2 + 1;
        let mut risk_cache = vec![f64::NAN; risk_width * (height * 2 + 1)];
        // Seed all adjacent cell centers directly reachable from the exact starting position.
        let sx = start.x.floor() as i32;
        let sy = start.y.floor() as i32;
        for y in (sy - 1)..=(sy + 1) {
            for x in (sx - 1)..=(sx + 1) {
                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    continue;
                }
                let index = y as usize * width + x as usize;
                let point = center(width, index);
                if let Some(time) = map::travel_time(map, start, point) {
                    let cost = weighted(map, start, point, time, threats, preference, revealed);
                    costs[index] = cost;
                    previous[index] = index;
                    queue.push(Entry { cost, index });
                }
            }
        }
        while let Some(Entry { cost, index }) = queue.pop() {
            if cost > costs[index] {
                continue;
            }
            if targets.is_some() && remaining[index] {
                remaining[index] = false;
                count -= 1;
                if count == 0 {
                    break;
                }
            }
            let x = (index % width) as i32;
            let y = (index / width) as i32;
            let from = center(width, index);
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
                let to = center(width, next);
                // Swept-radius segment validation also prevents diagonal corner cutting.
                let Some(time) = map::travel_time(map, from, to) else {
                    continue;
                };
                if cost + time + 1e-10 >= costs[next] {
                    continue;
                }
                let edge_cost = if preference == RoutePreference::Protected && !threats.is_empty() {
                    let mut risk = 0.0_f64;
                    for p in [
                        from,
                        Point {
                            x: (from.x + to.x) * 0.5,
                            y: (from.y + to.y) * 0.5,
                        },
                        to,
                    ] {
                        let key = (p.y * 2.0) as usize * risk_width + (p.x * 2.0) as usize;
                        let value = &mut risk_cache[key];
                        if value.is_nan() {
                            *value = exposure(map, p, threats, revealed);
                        }
                        risk = risk.max(*value);
                        if risk >= 1.0 {
                            break;
                        }
                    }
                    time * (1.0 + 2.0 * risk)
                } else {
                    time
                };
                let candidate = cost + edge_cost;
                if candidate + 1e-10 < costs[next] {
                    costs[next] = candidate;
                    previous[next] = index;
                    queue.push(Entry {
                        cost: candidate,
                        index: next,
                    });
                }
            }
        }
        Self {
            width,
            start,
            costs,
            previous,
            revealed,
        }
    }
    pub(crate) fn cost(&self, p: Point) -> Option<f64> {
        let index = self.index(p)?;
        self.costs[index].is_finite().then_some(self.costs[index])
    }
    fn index(&self, p: Point) -> Option<usize> {
        if p.x < 0.0 || p.y < 0.0 {
            return None;
        }
        let x = p.x.floor() as usize;
        let y = p.y.floor() as usize;
        let i = y.checked_mul(self.width)?.checked_add(x)?;
        (x < self.width && i < self.costs.len()).then_some(i)
    }
    #[cfg(test)]
    pub(crate) fn route(
        &self,
        map: &ArenaMap,
        goal: Point,
        threat: Option<Point>,
        preference: RoutePreference,
    ) -> Route {
        let threats: Vec<_> = threat.into_iter().collect();
        self.route_with_threats(map, goal, &threats, preference)
    }
    pub(crate) fn route_with_threats(
        &self,
        map: &ArenaMap,
        goal: Point,
        threats: &[Point],
        preference: RoutePreference,
    ) -> Route {
        let mut path = self.path(map, goal);
        if preference == RoutePreference::Fastest {
            if let (Some(direct), Some(grid)) =
                (map::travel_time(map, self.start, goal), self.cost(goal))
            {
                if direct <= grid + 1e-9 {
                    path = vec![goal];
                }
            }
        }
        Route {
            goal: Some(goal),
            threats: threats.to_vec(),
            revealed: self.revealed,
            preference: Some(preference),
            path,
            next: 0,
        }
    }
    fn path(&self, map: &ArenaMap, goal: Point) -> Vec<Point> {
        let Some(mut index) = self.index(goal) else {
            return vec![];
        };
        if !self.costs[index].is_finite() {
            return vec![];
        }
        let mut path = Vec::new();
        if map::travel_time(map, center(self.width, index), goal).is_none() {
            return path;
        }
        path.push(goal);
        loop {
            path.push(center(self.width, index));
            let prev = self.previous[index];
            if prev == index {
                break;
            }
            if prev == usize::MAX {
                return vec![];
            }
            index = prev;
        }
        path.reverse();
        if path.first().is_some_and(|p| p.distance(self.start) < 1e-9) {
            path.remove(0);
        }
        path
    }
}
fn center(width: usize, index: usize) -> Point {
    Point {
        x: (index % width) as f64 + 0.5,
        y: (index / width) as f64 + 0.5,
    }
}
fn weighted(
    map: &ArenaMap,
    a: Point,
    b: Point,
    time: f64,
    threats: &[Point],
    preference: RoutePreference,
    revealed: bool,
) -> f64 {
    if preference == RoutePreference::Fastest {
        return time;
    }
    let risk = [
        a,
        Point {
            x: (a.x + b.x) * 0.5,
            y: (a.y + b.y) * 0.5,
        },
        b,
    ]
    .into_iter()
    .map(|p| exposure(map, p, threats, revealed))
    .fold(0.0_f64, f64::max);
    time * (1.0 + 2.0 * risk)
}
fn exposure(map: &ArenaMap, p: Point, threats: &[Point], revealed: bool) -> f64 {
    let mut risk = 0.0_f64;
    for &enemy in threats {
        risk = risk.max(map::exposure(map, enemy, p, revealed));
        if risk >= 1.0 {
            break;
        }
    }
    risk
}
impl Route {
    pub(crate) fn prepare(
        &mut self,
        map: &ArenaMap,
        from: Point,
        goal: Point,
        threats: &[Point],
        preference: RoutePreference,
        revealed: bool,
    ) {
        let same_threat = self.threats.len() == threats.len()
            && self
                .threats
                .iter()
                .zip(threats)
                .all(|(a, b)| a.distance(*b) < 2.0);
        if self.goal == Some(goal)
            && same_threat
            && self.preference == Some(preference)
            && self.revealed == revealed
        {
            return;
        }
        let search = Search::for_targets(map, from, threats, preference, revealed, &[goal]);
        *self = search.route_with_threats(map, goal, threats, preference);
    }
    pub(crate) fn advance(
        &mut self,
        map: &ArenaMap,
        mut position: Point,
        mut budget: f64,
    ) -> Point {
        while self.next < self.path.len() && budget > 1e-10 {
            let target = self.path[self.next];
            let (next, used) = map::advance_toward(map, position, target, budget);
            position = next;
            budget -= used;
            if position.distance(target) < 1e-7 {
                self.next += 1;
            } else {
                break;
            }
        }
        position
    }
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn arena() -> ArenaMap {
        ArenaMap {
            width: 7,
            height: 5,
            surfaces: vec![Surface::Open; 35],
            elevations: vec![0.0; 48],
            obstacles: vec![],
            patrol: vec![],
        }
    }
    #[test]
    fn routes_around_wall_and_reaches_goal() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 3,
            y: 0,
            width: 1,
            height: 4,
            height_metres: 2.5,
        });
        let start = Point { x: 1.5, y: 1.5 };
        let goal = Point { x: 5.5, y: 1.5 };
        let mut route = Search::new(&map, start, None, RoutePreference::Fastest).route(
            &map,
            goal,
            None,
            RoutePreference::Fastest,
        );
        let mut p = start;
        for _ in 0..300 {
            let next = route.advance(&map, p, 0.05);
            assert!(map::travel_time(&map, p, next).is_some());
            p = next;
        }
        assert!(p.distance(goal) < 1e-6);
    }
    #[test]
    fn targeted_search_matches_full_weighted_routes_through_detours() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 3,
            y: 0,
            width: 1,
            height: 4,
            height_metres: 2.5,
        });
        map.surfaces[4 * 7 + 3] = Surface::Mud;
        let start = Point { x: 1.5, y: 1.5 };
        let targets = [Point { x: 5.5, y: 1.5 }, Point { x: 5.5, y: 3.5 }];
        let threats = [Point { x: 6.5, y: 0.5 }];
        for preference in [RoutePreference::Fastest, RoutePreference::Protected] {
            let full = Search::with_threats(&map, start, &threats, preference, false);
            let limited = Search::for_targets(&map, start, &threats, preference, false, &targets);
            for target in targets {
                assert_eq!(full.cost(target), limited.cost(target));
                assert_eq!(full.path(&map, target), limited.path(&map, target));
            }
        }
    }
    #[test]
    fn nearby_target_does_not_expand_the_whole_large_map() {
        let map = ArenaMap::flat(128, 128);
        let start = Point { x: 57.5, y: 55.5 };
        let goal = Point { x: 61.5, y: 55.5 };
        let search =
            Search::for_targets(&map, start, &[], RoutePreference::Fastest, false, &[goal]);
        assert!(search.cost(goal).is_some());
        assert!(search.costs.iter().filter(|cost| cost.is_finite()).count() < 200);
    }
    #[test]
    fn targeted_search_explores_all_reachable_cells_for_unreachable_target() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 3,
            y: 0,
            width: 1,
            height: 5,
            height_metres: 2.5,
        });
        let start = Point { x: 1.5, y: 1.5 };
        let targets = [Point { x: 2.5, y: 1.5 }, Point { x: 5.5, y: 1.5 }];
        let full = Search::with_threats(&map, start, &[], RoutePreference::Fastest, false);
        let limited =
            Search::for_targets(&map, start, &[], RoutePreference::Fastest, false, &targets);
        for target in targets {
            assert_eq!(full.cost(target), limited.cost(target));
        }
    }
    #[test]
    fn weighted_search_prefers_faster_surface() {
        let mut map = arena();
        for x in 1..6 {
            map.surfaces[2 * 7 + x] = Surface::Mud;
        }
        let start = Point { x: 0.5, y: 2.5 };
        let goal = Point { x: 6.5, y: 2.5 };
        let search = Search::new(&map, start, None, RoutePreference::Fastest);
        assert!(search.cost(goal).unwrap() < map::travel_time(&map, start, goal).unwrap());
    }
    #[test]
    fn unreachable_goal_has_no_route() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 3,
            y: 0,
            width: 1,
            height: 5,
            height_metres: 2.5,
        });
        let start = Point { x: 1.5, y: 1.5 };
        let goal = Point { x: 5.5, y: 1.5 };
        assert!(Search::new(&map, start, None, RoutePreference::Fastest)
            .cost(goal)
            .is_none());
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    #[test]
    fn small_threat_movement_keeps_route_progress() {
        let map = ArenaMap {
            width: 7,
            height: 5,
            surfaces: vec![Surface::Open; 35],
            elevations: vec![0.0; 48],
            obstacles: vec![Obstacle {
                x: 3,
                y: 0,
                width: 1,
                height: 4,
                height_metres: 2.5,
            }],
            patrol: vec![],
        };
        let start = Point { x: 1.5, y: 1.5 };
        let goal = Point { x: 5.5, y: 1.5 };
        let mut route = Route::default();
        route.prepare(
            &map,
            start,
            goal,
            &[Point { x: 6.5, y: 1.5 }],
            RoutePreference::Protected,
            false,
        );
        let p = route.advance(&map, start, 0.8);
        let remaining = route.path[route.next..].to_vec();
        route.prepare(
            &map,
            p,
            goal,
            &[Point { x: 6.4, y: 1.5 }],
            RoutePreference::Protected,
            false,
        );
        assert_eq!(&route.path[route.next..], remaining.as_slice());
        assert!(route.advance(&map, p, 0.1).distance(p) > 0.0);
    }
}

#[cfg(test)]
mod preference_tests {
    use super::*;
    #[test]
    fn firing_reveal_increases_route_risk_through_vegetation() {
        let mut map = ArenaMap::flat(16, 4);
        map.surfaces.fill(Surface::Woodland);
        let enemy = Point { x: 1.5, y: 1.5 };
        let start = Point { x: 8.5, y: 1.5 };
        let end = Point { x: 9.5, y: 1.5 };
        let time = map::travel_time(&map, start, end).unwrap();
        let concealed = weighted(
            &map,
            start,
            end,
            time,
            &[enemy],
            RoutePreference::Protected,
            false,
        );
        let revealed = weighted(
            &map,
            start,
            end,
            time,
            &[enemy],
            RoutePreference::Protected,
            true,
        );
        assert_eq!(concealed, time);
        assert!(revealed > concealed);
    }
    #[test]
    fn second_known_threat_changes_protected_route() {
        let mut map = ArenaMap::flat(16, 12);
        map.obstacles.push(Obstacle {
            x: 3,
            y: 4,
            width: 10,
            height: 1,
            height_metres: 2.5,
        });
        let start = Point { x: 1.5, y: 3.5 };
        let goal = Point { x: 14.5, y: 3.5 };
        let north = Point { x: 7.5, y: 0.5 };
        let south = Point { x: 7.5, y: 9.5 };
        let mut one =
            Search::with_threats(&map, start, &[north], RoutePreference::Protected, false)
                .route_with_threats(&map, goal, &[north], RoutePreference::Protected);
        let mut both = Search::with_threats(
            &map,
            start,
            &[north, south],
            RoutePreference::Protected,
            false,
        )
        .route_with_threats(&map, goal, &[north, south], RoutePreference::Protected);
        let one_pos = one.advance(&map, start, 1.5);
        let both_pos = both.advance(&map, start, 1.5);
        assert!(
            one_pos.distance(both_pos) > 0.5,
            "one={one_pos:?} both={both_pos:?}"
        );
    }
    #[test]
    fn protected_route_uses_less_exposed_path() {
        let mut map = ArenaMap::flat(16, 12);
        map.obstacles.push(Obstacle {
            x: 3,
            y: 4,
            width: 10,
            height: 1,
            height_metres: 2.5,
        });
        let start = Point { x: 1.5, y: 3.5 };
        let goal = Point { x: 14.5, y: 3.5 };
        let enemy = Point { x: 7.5, y: 0.5 };
        let mut fast = Search::new(&map, start, Some(enemy), RoutePreference::Fastest).route(
            &map,
            goal,
            Some(enemy),
            RoutePreference::Fastest,
        );
        let mut safe = Search::new(&map, start, Some(enemy), RoutePreference::Protected).route(
            &map,
            goal,
            Some(enemy),
            RoutePreference::Protected,
        );
        let measure = |route: &mut Route| {
            let mut p = start;
            let mut exposure = 0.0;
            let mut steps = 0;
            while p.distance(goal) > 0.001 && steps < 400 {
                p = route.advance(&map, p, 0.05);
                if map::detectable(&map, enemy, p, false) {
                    exposure += 0.05 * map::shot_fraction(&map, enemy, p);
                }
                steps += 1;
            }
            assert!(p.distance(goal) < 0.001);
            (steps, exposure)
        };
        let (fast_steps, fast_exposure) = measure(&mut fast);
        let (safe_steps, safe_exposure) = measure(&mut safe);
        assert!(safe_steps > fast_steps);
        assert!(safe_exposure < fast_exposure);
    }
}
