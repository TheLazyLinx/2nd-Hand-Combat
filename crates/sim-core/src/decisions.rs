use crate::{map, navigation, perception::Unit, types::*};

pub(crate) struct DecisionInput<'a> {
    pub unit: &'a Unit,
    pub visible_enemy: Option<Point>,
    pub allies: Vec<Point>,
}

pub(crate) fn decide(
    input: &DecisionInput<'_>,
    map: &ArenaMap,
    faction_index: usize,
) -> (Behavior, Option<Point>, Option<Point>, usize) {
    let me = input.unit;
    let threat = input
        .visible_enemy
        .or_else(|| me.memory.as_ref().map(|m| m.position));
    if me.is_low_hp() {
        let Some(enemy) = threat else {
            return (Behavior::Defending, None, None, me.patrol_index);
        };
        let score = |p: Point| p.distance(enemy) + if map::in_cover(map, p) { 3.0 } else { 0.0 };
        if let (Some(goal), Some(old_threat)) = (me.fallback_goal, me.fallback_threat) {
            if old_threat.distance(enemy) < 2.0 && navigation::reachable(map, me.position, goal) {
                if me.position.distance(goal) > 0.3 && score(goal) > score(me.position) + 0.05 {
                    return (
                        Behavior::FallingBack,
                        Some(goal),
                        Some(goal),
                        me.patrol_index,
                    );
                }
                if me.position.distance(goal) <= 0.3 {
                    return (Behavior::Defending, None, Some(goal), me.patrol_index);
                }
            }
        }
        let safe = |p: &Point| {
            score(*p) > score(me.position) + 0.25 && navigation::reachable(map, me.position, *p)
        };
        let best = |points: Vec<Point>| {
            points
                .into_iter()
                .filter(safe)
                .max_by(|a, b| score(*a).total_cmp(&score(*b)))
        };
        let covers = map
            .cover
            .iter()
            .map(|p| Point {
                x: p.x + 0.5,
                y: p.y + 0.5,
            })
            .collect();
        let allies = input.allies.clone();
        let dx = me.position.x - enemy.x;
        let dy = me.position.y - enemy.y;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let mut retreats = vec![Point {
            x: (me.position.x + dx / len * 4.0).clamp(0.5, map.width as f64 - 0.5),
            y: (me.position.y + dy / len * 4.0).clamp(0.5, map.height as f64 - 0.5),
        }];
        for (x, y) in [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (0.707, 0.707),
            (0.707, -0.707),
            (-0.707, 0.707),
            (-0.707, -0.707),
        ] {
            for distance in [1.0, 2.0, 4.0] {
                retreats.push(Point {
                    x: (me.position.x + x * distance).clamp(0.5, map.width as f64 - 0.5),
                    y: (me.position.y + y * distance).clamp(0.5, map.height as f64 - 0.5),
                });
            }
        }
        if let Some(goal) = best(covers)
            .or_else(|| best(allies))
            .or_else(|| best(retreats))
        {
            return (
                Behavior::FallingBack,
                Some(goal),
                Some(goal),
                me.patrol_index,
            );
        }
        return (Behavior::Defending, None, None, me.patrol_index);
    }
    if let Some(enemy) = input.visible_enemy {
        let distance = me.position.distance(enemy);
        if distance > me.placement.doctrine.engagement_distance {
            let ratio = (distance - me.placement.doctrine.engagement_distance) / distance;
            return (
                Behavior::Advancing,
                Some(Point {
                    x: me.position.x + (enemy.x - me.position.x) * ratio,
                    y: me.position.y + (enemy.y - me.position.y) * ratio,
                }),
                None,
                me.patrol_index,
            );
        }
        if !map::in_cover(map, me.position) {
            let cover = map
                .cover
                .iter()
                .map(|p| Point {
                    x: p.x + 0.5,
                    y: p.y + 0.5,
                })
                .filter(|p| {
                    me.position.distance(*p) <= 3.0
                        && p.distance(enemy) <= me.placement.doctrine.engagement_distance
                        && map::clear(map, *p, enemy)
                        && navigation::reachable(map, me.position, *p)
                })
                .min_by(|a, b| {
                    me.position
                        .distance(*a)
                        .total_cmp(&me.position.distance(*b))
                });
            if let Some(goal) = cover {
                return (Behavior::Advancing, Some(goal), None, me.patrol_index);
            }
        }
        return (Behavior::Engaging, None, None, me.patrol_index);
    }
    if let Some(memory) = &me.memory {
        if !memory.investigated {
            return (
                Behavior::Investigating,
                Some(memory.position),
                None,
                me.patrol_index,
            );
        }
    }
    if map.patrol.is_empty() {
        return (Behavior::Searching, None, None, me.patrol_index);
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
    (Behavior::Searching, Some(map.patrol[idx]), None, idx)
}
