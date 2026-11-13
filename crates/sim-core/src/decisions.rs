use crate::{map, navigation, types::*, unit::Unit};

pub(crate) struct DecisionInput<'a> {
    pub unit: &'a Unit,
    pub visible_enemy: Option<Point>,
    pub allies: Vec<Point>,
    pub tick: u32,
    pub visible_enemy_revealed: bool,
}
pub(crate) struct Decision {
    pub behavior: Behavior,
    pub goal: Option<Point>,
    pub fallback_goal: Option<Point>,
    pub patrol_index: usize,
    pub route: Option<navigation::Route>,
}

pub(crate) fn decide(input: &DecisionInput<'_>, map: &ArenaMap, faction_index: usize) -> Decision {
    let me = input.unit;
    let threat = input
        .visible_enemy
        .or_else(|| me.memory.as_ref().map(|m| m.position));
    let fallback_threat = threat;
    let threats = if me.known_threats.is_empty() {
        fallback_threat.as_slice()
    } else {
        me.known_threats.as_slice()
    };
    let preference = me.placement.doctrine.route_preference;
    let revealed = input.tick < me.revealed_until;
    let decision = |behavior, goal, fallback_goal, patrol_index, route| Decision {
        behavior,
        goal,
        fallback_goal,
        patrol_index,
        route,
    };
    if me.withdrawing() {
        let Some(enemy) = threat else {
            return decision(Behavior::Defending, None, None, me.patrol_index, None);
        };
        let safety = |p: Point| {
            let exposure = threats
                .iter()
                .copied()
                .map(|enemy| map::exposure(map, enemy, p, revealed))
                .fold(0.0_f64, f64::max);
            let nearest = threats
                .iter()
                .map(|enemy| p.distance(*enemy))
                .fold(f64::INFINITY, f64::min);
            (1.0 - exposure, nearest)
        };
        let current = safety(me.position);
        if let (Some(goal), Some(old_threat)) = (me.fallback_goal, me.fallback_threat) {
            let safe = safety(goal);
            if old_threat.distance(enemy) < 2.0
                && (safe.0 > current.0 + 0.01
                    || (safe.0 >= current.0 - 0.01 && me.position.distance(goal) > 0.3)
                    || me.position.distance(goal) <= 0.3)
            {
                if me.position.distance(goal) <= 0.3 {
                    return decision(Behavior::Defending, None, Some(goal), me.patrol_index, None);
                }
                return decision(
                    Behavior::FallingBack,
                    Some(goal),
                    Some(goal),
                    me.patrol_index,
                    None,
                );
            }
        }
        let targets: Vec<_> = (0..map.height)
            .flat_map(|y| {
                (0..map.width).map(move |x| Point {
                    x: x as f64 + 0.5,
                    y: y as f64 + 0.5,
                })
            })
            .chain(input.allies.iter().copied())
            .filter(|p| me.position.distance(*p) <= 5.0)
            .collect();
        let search = navigation::Search::for_targets(
            map,
            me.position,
            threats,
            preference,
            revealed,
            &targets,
        );
        let mut best: Option<(Point, f64, f64, f64, bool)> = None;
        let cells = (0..map.height).flat_map(|y| {
            (0..map.width).map(move |x| Point {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
            })
        });
        for p in cells.chain(input.allies.iter().copied()) {
            if me.position.distance(p) > 5.0 {
                continue;
            }
            let Some(cost) = search.cost(p) else {
                continue;
            };
            let (protection, distance) = safety(p);
            if protection <= current.0 + 0.01
                && !(protection >= current.0 - 0.01 && distance > current.1 + 0.25)
            {
                continue;
            }
            let ally = input.allies.contains(&p);
            let candidate = (p, protection, distance, cost, ally);
            if best.as_ref().is_none_or(|b| {
                protection > b.1 + 0.01
                    || ((protection - b.1).abs() <= 0.01
                        && ((ally && !b.4)
                            || (ally == b.4
                                && if protection > current.0 + 0.01 {
                                    cost < b.3 - 0.01
                                } else {
                                    distance > b.2 + 0.01
                                })))
            }) {
                best = Some(candidate);
            }
        }
        if let Some((goal, _, _, _, _)) = best {
            let route = search.route_with_threats(map, goal, threats, preference);
            return decision(
                Behavior::FallingBack,
                Some(goal),
                Some(goal),
                me.patrol_index,
                Some(route),
            );
        }
        return decision(Behavior::Defending, None, None, me.patrol_index, None);
    }
    if let Some(enemy) = input.visible_enemy {
        let distance = me.position.distance(enemy);
        if distance > me.placement.doctrine.engagement_distance {
            let ratio = (distance - me.placement.doctrine.engagement_distance) / distance;
            let desired = Point {
                x: me.position.x + (enemy.x - me.position.x) * ratio,
                y: me.position.y + (enemy.y - me.position.y) * ratio,
            };
            if map::travel_time(map, me.position, desired).is_some() {
                return decision(
                    Behavior::Advancing,
                    Some(desired),
                    None,
                    me.patrol_index,
                    None,
                );
            }
            let search =
                navigation::Search::with_threats(map, me.position, threats, preference, revealed);
            let mut best: Option<(Point, f64, f64)> = None;
            for y in 0..map.height {
                for x in 0..map.width {
                    let p = Point {
                        x: x as f64 + 0.5,
                        y: y as f64 + 0.5,
                    };
                    let Some(cost) = search.cost(p) else {
                        continue;
                    };
                    if p.distance(enemy) > me.placement.doctrine.engagement_distance
                        || !map::detectable(map, p, enemy, input.visible_enemy_revealed)
                        || map::shot_fraction(map, p, enemy) <= 0.0
                    {
                        continue;
                    }
                    let offset = p.distance(desired);
                    if best.as_ref().is_none_or(|b| {
                        cost < b.1 - 0.01 || ((cost - b.1).abs() <= 0.01 && offset < b.2)
                    }) {
                        best = Some((p, cost, offset));
                    }
                }
            }
            if let Some((goal, _, _)) = best {
                return decision(
                    Behavior::Advancing,
                    Some(goal),
                    None,
                    me.patrol_index,
                    Some(search.route_with_threats(map, goal, threats, preference)),
                );
            }
            return decision(Behavior::Engaging, None, None, me.patrol_index, None);
        }
        let score = |p: Point| {
            threats
                .iter()
                .copied()
                .map(|enemy| match me.placement.doctrine.position_preference {
                    PositionPreference::Balanced => {
                        if map::detectable(map, enemy, p, revealed) {
                            map::shot_fraction(map, enemy, p)
                        } else {
                            0.0
                        }
                    }
                    PositionPreference::Cover => map::shot_fraction(map, enemy, p),
                    PositionPreference::Concealment => {
                        1.0 / (1.0 + map::spotting_cost(map, enemy, p))
                    }
                })
                .fold(0.0_f64, f64::max)
        };
        let current = score(me.position);
        if let Some(goal) = me.goal {
            if me.behavior == Behavior::Advancing
                && me.position.distance(goal) > 0.2
                && goal.distance(enemy) <= me.placement.doctrine.engagement_distance
                && map::detectable(map, goal, enemy, input.visible_enemy_revealed)
                && map::shot_fraction(map, goal, enemy) > 0.0
                && score(goal) <= current + 0.01
            {
                return decision(Behavior::Advancing, Some(goal), None, me.patrol_index, None);
            }
        }
        let mut candidates = Vec::new();
        for y in 0..map.height {
            for x in 0..map.width {
                let p = Point {
                    x: x as f64 + 0.5,
                    y: y as f64 + 0.5,
                };
                if me.position.distance(p) > 3.0
                    || p.distance(enemy) > me.placement.doctrine.engagement_distance
                    || !map::detectable(map, p, enemy, input.visible_enemy_revealed)
                    || map::shot_fraction(map, p, enemy) <= 0.0
                {
                    continue;
                }
                let value = score(p);
                if value >= current - 0.01 {
                    continue;
                }
                candidates.push((p, value));
            }
        }
        if candidates.is_empty() {
            return decision(Behavior::Engaging, None, None, me.patrol_index, None);
        }
        let targets: Vec<_> = candidates.iter().map(|(p, _)| *p).collect();
        let search = navigation::Search::for_targets(
            map,
            me.position,
            threats,
            preference,
            revealed,
            &targets,
        );
        let mut best: Option<(Point, f64, f64)> = None;
        for (p, value) in candidates {
            let Some(cost) = search.cost(p) else {
                continue;
            };
            if best
                .as_ref()
                .is_none_or(|b| value < b.1 - 0.01 || ((value - b.1).abs() <= 0.01 && cost < b.2))
            {
                best = Some((p, value, cost));
            }
        }
        if let Some((goal, _, _)) = best {
            return decision(
                Behavior::Advancing,
                Some(goal),
                None,
                me.patrol_index,
                Some(search.route_with_threats(map, goal, threats, preference)),
            );
        }
        return decision(Behavior::Engaging, None, None, me.patrol_index, None);
    }
    if let Some(memory) = &me.memory {
        if !memory.investigated {
            if map::clear(map, me.position, memory.position) {
                return decision(
                    Behavior::Investigating,
                    Some(memory.position),
                    None,
                    me.patrol_index,
                    None,
                );
            }
            let search =
                navigation::Search::with_threats(map, me.position, threats, preference, revealed);
            let mut best: Option<(Point, f64)> = None;
            for y in 0..map.height {
                for x in 0..map.width {
                    let p = Point {
                        x: x as f64 + 0.5,
                        y: y as f64 + 0.5,
                    };
                    let Some(cost) = search.cost(p) else {
                        continue;
                    };
                    if !map::detectable(map, p, memory.position, false)
                        || map::shot_fraction(map, p, memory.position) <= 0.0
                    {
                        continue;
                    }
                    if best.as_ref().is_none_or(|b| cost < b.1) {
                        best = Some((p, cost));
                    }
                }
            }
            if let Some((goal, _)) = best {
                return decision(
                    Behavior::Investigating,
                    Some(goal),
                    None,
                    me.patrol_index,
                    Some(search.route_with_threats(map, goal, threats, preference)),
                );
            }
            return decision(
                Behavior::Investigating,
                Some(memory.position),
                None,
                me.patrol_index,
                None,
            );
        }
    }
    if map.patrol.is_empty() {
        return decision(Behavior::Searching, None, None, me.patrol_index, None);
    }
    let mut idx = me.patrol_index % map.patrol.len();
    let direction = if faction_index == 0 {
        1
    } else {
        map.patrol.len() - 1
    };
    if me.position.distance(map.patrol[idx]) < 0.4 {
        idx = (idx + direction) % map.patrol.len();
    }
    decision(Behavior::Searching, Some(map.patrol[idx]), None, idx, None)
}

#[cfg(test)]
mod acceptance_tests {
    use super::*;
    use crate::{definitions::catalog, unit::Memory};
    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    fn arena() -> ArenaMap {
        ArenaMap::flat(16, 12)
    }
    fn unit(at: Point) -> Unit {
        Unit::new(UnitPlacement {
            id: 1,
            faction: 0,
            definition_id: "infantry".into(),
            position: at,
            doctrine: catalog().default_doctrine,
        })
    }
    fn choose(unit: &Unit, map: &ArenaMap, enemy: Option<Point>) -> Decision {
        decide(
            &DecisionInput {
                unit,
                visible_enemy: enemy,
                allies: vec![],
                tick: 1,
                visible_enemy_revealed: false,
            },
            map,
            0,
        )
    }
    #[test]
    fn blocked_straight_approach_selects_a_reachable_firing_position() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 6,
            y: 5,
            width: 1,
            height: 1,
            height_metres: 0.9,
        });
        let mut u = unit(p(2.5, 5.5));
        u.placement.doctrine.engagement_distance = 4.0;
        let enemy = p(10.5, 5.5);
        let d = choose(&u, &map, Some(enemy));
        let goal = d.goal.unwrap();
        assert!(!map::blocked(&map, goal));
        assert!(goal.distance(enemy) <= 4.0);
        let mut route = d.route.unwrap();
        let mut pos = u.position;
        for _ in 0..200 {
            pos = route.advance(&map, pos, 0.05);
        }
        assert!(pos.distance(goal) < 0.001);
        assert!(map::shot_fraction(&map, pos, enemy) > 0.0);
    }
    #[test]
    fn investigation_moves_to_an_opening_after_sight_loss() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 7,
            y: 2,
            width: 1,
            height: 8,
            height_metres: 2.5,
        });
        let mut u = unit(p(5.5, 5.5));
        let enemy = p(9.5, 5.5);
        u.memory = Some(Memory {
            position: enemy,
            tick: 0,
            investigated: false,
        });
        let d = choose(&u, &map, None);
        let goal = d.goal.unwrap();
        assert_ne!(goal, enemy);
        assert!(map::detectable(&map, goal, enemy, false));
        let mut route = d.route.unwrap();
        let mut pos = u.position;
        for _ in 0..250 {
            pos = route.advance(&map, pos, 0.05);
        }
        assert!(pos.distance(goal) < 0.001);
    }
    #[test]
    fn fallback_reaches_solid_protection_and_retains_goal() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 5,
            y: 3,
            width: 1,
            height: 1,
            height_metres: 2.5,
        });
        let enemy = p(12.5, 5.5);
        let mut u = unit(p(8.5, 5.5));
        u.placement.doctrine.fallback_hp = 100.0;
        let d = choose(&u, &map, Some(enemy));
        let goal = d.goal.unwrap();
        assert!(
            map::shot_fraction(&map, enemy, goal) < map::shot_fraction(&map, enemy, u.position)
        );
        let mut route = d.route.unwrap();
        u.fallback_goal = Some(goal);
        u.fallback_threat = Some(enemy);
        for _ in 0..200 {
            u.position = route.advance(&map, u.position, 0.05);
            if u.position.distance(goal) > 0.3 {
                assert_eq!(choose(&u, &map, Some(enemy)).goal, Some(goal));
            }
        }
        assert!(u.position.distance(goal) < 0.001);
    }
    #[test]
    fn hidden_enemy_is_not_used_without_memory() {
        let map = arena();
        let mut u = unit(p(2.5, 2.5));
        u.placement.doctrine.fallback_hp = 100.0;
        let d = choose(&u, &map, None);
        assert_eq!(d.behavior, Behavior::Defending);
        assert!(d.goal.is_none());
    }
    #[test]
    fn position_preferences_choose_distinct_reachable_positions() {
        let mut map = arena();
        map.obstacles.push(Obstacle {
            x: 9,
            y: 4,
            width: 1,
            height: 1,
            height_metres: 0.9,
        });
        for y in 5..8 {
            for x in 5..9 {
                map.surfaces[y * 16 + x] = Surface::Woodland;
            }
        }
        let enemy = p(12.5, 5.5);
        let mut u = unit(p(8.5, 6.5));
        u.placement.doctrine.position_preference = PositionPreference::Cover;
        let cover_decision = choose(&u, &map, Some(enemy));
        let cover = cover_decision.goal.unwrap();
        u.placement.doctrine.position_preference = PositionPreference::Concealment;
        let concealed_decision = choose(&u, &map, Some(enemy));
        let concealed = concealed_decision.goal.unwrap();
        assert_ne!(cover, concealed);
        assert!(
            map::shot_fraction(&map, enemy, cover) < map::shot_fraction(&map, enemy, concealed)
        );
        assert!(
            map::spotting_cost(&map, enemy, concealed) > map::spotting_cost(&map, enemy, cover)
        );
        let mut moving = u.clone();
        moving.placement.doctrine.position_preference = PositionPreference::Cover;
        moving.behavior = Behavior::Advancing;
        moving.goal = Some(cover);
        let mut cover_route = cover_decision.route.clone().unwrap();
        for _ in 0..20 {
            moving.position = cover_route.advance(&map, moving.position, 0.05);
            if moving.position.distance(cover) > 0.2 {
                assert_eq!(choose(&moving, &map, Some(enemy)).goal, Some(cover));
            }
        }
        for (goal, mut route) in [
            (cover, cover_decision.route.unwrap()),
            (concealed, concealed_decision.route.unwrap()),
        ] {
            let mut pos = u.position;
            for _ in 0..100 {
                pos = route.advance(&map, pos, 0.05);
            }
            assert!(pos.distance(goal) < 0.001);
        }
    }
    #[test]
    fn empty_ammunition_withdrawal_depends_on_doctrine() {
        let map = arena();
        let mut u = unit(p(5.5, 5.5));
        for _ in 0..60 {
            u.inventory.consume("rifle-ammo").unwrap();
        }
        let retreat = choose(&u, &map, Some(p(10.5, 5.5)));
        assert_eq!(retreat.behavior, Behavior::FallingBack);
        u.placement.doctrine.out_of_ammo_behavior = OutOfAmmoBehavior::ContinueDoctrine;
        let continued = choose(&u, &map, Some(p(10.5, 5.5)));
        assert_eq!(continued.behavior, Behavior::Engaging);
    }
}
