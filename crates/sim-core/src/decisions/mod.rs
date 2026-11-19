//! Tactical doctrine evaluation coordinator: gathers typed proposals from per-behaviour modules
//! (`fallback`, `engage`, `reposition`, `investigate`, `patrol`) without mutating world state.
//!
//! Invariant: decisions read only immutable unit perception/memory/doctrine, faction patrol
//! direction, and terrain queries; `battle.rs` applies the returned `Decision` intention.

mod engage;
mod fallback;
mod investigate;
mod patrol;
mod reposition;

use crate::{
    navigation::{Route, SearchParams, SearchScratch},
    terrain::Terrain,
    types::*,
    unit::Unit,
};

#[cfg(test)]
use crate::terrain::TerrainCache;

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
    pub route: Option<Route>,
    pub mark_investigated: bool,
}

impl Decision {
    pub(crate) fn hold(
        behavior: Behavior,
        fallback_goal: Option<Point>,
        patrol_index: usize,
    ) -> Self {
        Self {
            behavior,
            goal: None,
            fallback_goal,
            patrol_index,
            route: None,
            mark_investigated: false,
        }
    }

    pub(crate) fn move_to(
        behavior: Behavior,
        goal: Point,
        fallback_goal: Option<Point>,
        patrol_index: usize,
        route: Option<Route>,
    ) -> Self {
        Self {
            behavior,
            goal: Some(goal),
            fallback_goal,
            patrol_index,
            route,
            mark_investigated: false,
        }
    }
}

pub(crate) struct DecisionContext<'a> {
    pub input: &'a DecisionInput<'a>,
    pub terrain: Terrain<'a>,
    pub patrol_direction: PatrolDirection,
    pub threat: Option<Point>,
    pub threats: &'a [Point],
    pub preference: RoutePreference,
    pub revealed: bool,
}

impl<'a> DecisionContext<'a> {
    pub(crate) fn search_params(&self) -> SearchParams<'a> {
        SearchParams {
            start: self.input.unit.nav.position,
            threats: self.threats,
            preference: self.preference,
            revealed: self.revealed,
        }
    }
}

/// Clamped integer cell bounds `[x0..=x1] × [y0..=y1]` covering all cell centers within `radius`.
pub(crate) fn grid_window(map: &ArenaMap, center: Point, radius: f64) -> (u32, u32, u32, u32) {
    let x0 = (center.x - radius).floor().max(0.0) as u32;
    let x1 = (center.x + radius)
        .floor()
        .clamp(0.0, (map.width - 1) as f64) as u32;
    let y0 = (center.y - radius).floor().max(0.0) as u32;
    let y1 = (center.y + radius)
        .floor()
        .clamp(0.0, (map.height - 1) as f64) as u32;
    (x0, x1, y0, y1)
}

#[cfg(test)]
pub(crate) fn decide(
    input: &DecisionInput<'_>,
    map: &ArenaMap,
    patrol_direction: PatrolDirection,
) -> Decision {
    let cache = TerrainCache::new(map);
    let mut scratch = SearchScratch::new(cache.width, cache.height);
    let mut threat_positions = Vec::new();
    decide_with(
        input,
        cache.view(map),
        &mut scratch,
        &mut threat_positions,
        patrol_direction,
    )
}

pub(crate) fn decide_with(
    input: &DecisionInput<'_>,
    terrain: Terrain<'_>,
    scratch: &mut SearchScratch,
    threat_positions: &mut Vec<Point>,
    patrol_direction: PatrolDirection,
) -> Decision {
    let me = input.unit;
    let threat = input
        .visible_enemy
        .or_else(|| me.perception.primary().map(|m| m.position));
    me.perception.fill_threat_positions(threat_positions);
    let fallback_threat = threat;
    let threats = if threat_positions.is_empty() {
        fallback_threat.as_slice()
    } else {
        threat_positions.as_slice()
    };
    let ctx = DecisionContext {
        input,
        terrain,
        patrol_direction,
        threat,
        threats,
        preference: me.placement.doctrine.route_preference,
        revealed: input.tick < me.combat.revealed_until,
    };

    if let Some(decision) = fallback::propose(&ctx, scratch) {
        return decision;
    }
    if let Some(decision) = engage::propose(&ctx, scratch) {
        return decision;
    }
    if let Some(decision) = reposition::propose(&ctx, scratch) {
        return decision;
    }
    let mut mark_investigated = false;
    match investigate::propose(&ctx, scratch) {
        investigate::InvestigateOutcome::Decision(decision) => return decision,
        investigate::InvestigateOutcome::Unreachable => mark_investigated = true,
        investigate::InvestigateOutcome::None => {}
    }
    let mut decision = patrol::propose(&ctx);
    decision.mark_investigated = mark_investigated;
    decision
}

#[cfg(test)]
mod acceptance_tests {
    use super::*;
    use crate::{definitions::catalog, map};
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
            loadout: None,
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
            PatrolDirection::Forward,
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
        let mut pos = u.nav.position;
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
        u.perception.remember_for_test(enemy, 0);
        let d = choose(&u, &map, None);
        let goal = d.goal.unwrap();
        assert_ne!(goal, enemy);
        assert!(map::detectable(&map, goal, enemy, false));
        let mut route = d.route.unwrap();
        let mut pos = u.nav.position;
        for _ in 0..250 {
            pos = route.advance(&map, pos, 0.05);
        }
        assert!(pos.distance(goal) < 0.001);
    }
    #[test]
    fn unreachable_investigation_marks_investigated_and_falls_back_to_patrol() {
        let mut map = arena();
        map.patrol = vec![p(2.5, 2.5)];
        // Full dividing wall across the entire map height isolates the remembered position.
        map.obstacles.push(Obstacle {
            x: 7,
            y: 0,
            width: 1,
            height: 12,
            height_metres: 2.5,
        });
        let mut u = unit(p(5.5, 5.5));
        u.perception.remember_for_test(p(9.5, 5.5), 0);
        let d = choose(&u, &map, None);
        assert!(d.mark_investigated);
        assert_eq!(d.behavior, Behavior::Searching);
        assert_eq!(d.goal, Some(p(2.5, 2.5)));
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
            map::shot_fraction(&map, enemy, goal) < map::shot_fraction(&map, enemy, u.nav.position)
        );
        let mut route = d.route.unwrap();
        u.nav.fallback_goal = Some(goal);
        u.nav.fallback_threat = Some(enemy);
        for _ in 0..200 {
            u.nav.position = route.advance(&map, u.nav.position, 0.05);
            if u.nav.position.distance(goal) > 0.3 {
                assert_eq!(choose(&u, &map, Some(enemy)).goal, Some(goal));
            }
        }
        assert!(u.nav.position.distance(goal) < 0.001);
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
        moving.combat.behavior = Behavior::Advancing;
        moving.nav.goal = Some(cover);
        let mut cover_route = cover_decision.route.clone().unwrap();
        for _ in 0..20 {
            moving.nav.position = cover_route.advance(&map, moving.nav.position, 0.05);
            if moving.nav.position.distance(cover) > 0.2 {
                assert_eq!(choose(&moving, &map, Some(enemy)).goal, Some(cover));
            }
        }
        for (goal, mut route) in [
            (cover, cover_decision.route.unwrap()),
            (concealed, concealed_decision.route.unwrap()),
        ] {
            let mut pos = u.nav.position;
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
    #[test]
    fn patrol_follows_faction_patrol_direction() {
        let mut map = arena();
        map.patrol = vec![p(2.5, 2.5), p(6.5, 2.5), p(10.5, 2.5)];
        let mut u = unit(p(6.5, 2.5));
        u.nav.patrol_index = 1;
        let input = DecisionInput {
            unit: &u,
            visible_enemy: None,
            allies: vec![],
            tick: 1,
            visible_enemy_revealed: false,
        };
        let forward = decide(&input, &map, PatrolDirection::Forward);
        let reverse = decide(&input, &map, PatrolDirection::Reverse);
        assert_eq!(forward.patrol_index, 2);
        assert_eq!(forward.goal, Some(p(10.5, 2.5)));
        assert_eq!(reverse.patrol_index, 0);
        assert_eq!(reverse.goal, Some(p(2.5, 2.5)));
    }
}
