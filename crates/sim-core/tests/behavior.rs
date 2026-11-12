use sim_core::*;
fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}
fn obstacle(x: u32, y: u32, height_metres: f64) -> Obstacle {
    Obstacle {
        x,
        y,
        width: 1,
        height: 1,
        height_metres,
    }
}
fn scenario(a: Point, b: Point) -> Scenario {
    let map = ArenaMap::flat(32, 20);
    Scenario {
        map,
        seed: 1,
        factions: vec![0, 1],
        units: vec![
            UnitPlacement {
                id: 1,
                faction: 0,
                definition_id: "infantry".into(),
                position: a,
                doctrine: Doctrine {
                    engagement_distance: 8.0,
                    fallback_hp: 0.0,
                    position_preference: PositionPreference::Balanced,
                    route_preference: RoutePreference::Fastest,
                },
            },
            UnitPlacement {
                id: 2,
                faction: 1,
                definition_id: "infantry".into(),
                position: b,
                doctrine: Doctrine {
                    engagement_distance: 8.0,
                    fallback_hp: 0.0,
                    position_preference: PositionPreference::Balanced,
                    route_preference: RoutePreference::Fastest,
                },
            },
        ],
        time_limit: 180.0,
    }
}
fn run(b: &mut Battle, n: usize) {
    for _ in 0..n {
        b.step();
    }
}
#[test]
fn placement_validation_and_empty_setup() {
    let mut s = scenario(p(2.5, 2.5), p(2.5, 2.5));
    assert!(!validate_scenario(&s, true).valid);
    s.units.clear();
    assert!(validate_scenario(&s, false).valid);
    assert!(!validate_scenario(&s, true).valid);
    s.units.push(UnitPlacement {
        id: 3,
        faction: 0,
        definition_id: "infantry".into(),
        position: p(5.5, 5.5),
        doctrine: catalog().default_doctrine,
    });
    s.map.obstacles.push(obstacle(5, 5, 2.4));
    assert!(!validate_scenario(&s, false).valid);
}
#[test]
fn sight_is_blocked_and_not_shared() {
    let mut s = scenario(p(3.5, 5.5), p(7.5, 5.5));
    s.map.obstacles.push(obstacle(5, 5, 2.4));
    s.units.push(UnitPlacement {
        id: 3,
        faction: 0,
        definition_id: "infantry".into(),
        position: p(6.5, 7.5),
        doctrine: catalog().default_doctrine,
    });
    let mut b = Battle::new(s).unwrap();
    b.step();
    let snap = b.snapshot();
    assert_eq!(snap.units[0].target_id, None);
    assert_eq!(snap.units[2].target_id, Some(2));
}
#[test]
fn doctrine_changes_firing_position() {
    let s = scenario(p(3.5, 10.5), p(15.5, 10.5));
    let mut long = s.clone();
    long.units[0].doctrine.engagement_distance = 12.0;
    let mut short = s;
    short.units[0].doctrine.engagement_distance = 3.0;
    let mut a = Battle::new(long).unwrap();
    let mut b = Battle::new(short).unwrap();
    run(&mut a, 60);
    run(&mut b, 60);
    assert!(b.snapshot().units[0].position.x > a.snapshot().units[0].position.x + 1.0);
    assert!(a.snapshot().units[0].shots_fired > 0);
}
#[test]
fn same_seed_repeats_and_timeout_is_reported() {
    let mut s = scenario(p(1.5, 1.5), p(30.5, 18.5));
    s.time_limit = 1.0;
    let mut a = Battle::new(s.clone()).unwrap();
    let mut b = Battle::new(s).unwrap();
    run(&mut a, 25);
    run(&mut b, 25);
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.snapshot().result.unwrap().reason, "timeout");
}
#[test]
fn cover_changes_hit_probability_without_reducing_hit_damage() {
    let mut s = scenario(p(3.5, 3.5), p(7.5, 3.5));
    s.map.obstacles.push(obstacle(6, 3, 0.9));
    let mut b = Battle::new(s).unwrap();
    let mut previous = b.snapshot();
    let mut hit = false;
    for _ in 0..400 {
        b.step();
        let next = b.snapshot();
        for (before, after) in previous.units.iter().zip(&next.units) {
            let damage = before.hp - after.hp;
            if damage > 0.0 {
                assert_eq!(damage, catalog().weapon.damage);
                hit = true;
            }
        }
        previous = next;
        if previous.result.is_some() {
            break;
        }
    }
    assert!(hit);
}
#[test]
fn low_hp_doctrine_causes_fallback() {
    let mut s = scenario(p(8.5, 10.5), p(12.5, 10.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    let mut b = Battle::new(s).unwrap();
    run(&mut b, 10);
    assert_eq!(b.snapshot().units[0].behavior, Behavior::FallingBack);
    assert!(b.snapshot().units[0].position.x < 8.5);
}
#[test]
fn patrol_routes_around_wall_without_crossing_it() {
    let mut s = scenario(p(3.5, 4.5), p(28.5, 18.5));
    s.map.obstacles = (1..7).map(|y| obstacle(5, y, 2.4)).collect();
    s.map.patrol = vec![p(8.5, 4.5)];
    let mut b = Battle::new(s).unwrap();
    for _ in 0..180 {
        b.step();
        let q = b.snapshot().units[0].position;
        for y in 1..7 {
            let cx = q.x.clamp(5.0, 6.0);
            let cy = q.y.clamp(y as f64, y as f64 + 1.0);
            assert!((q.x - cx).powi(2) + (q.y - cy).powi(2) >= 0.25_f64.powi(2) - 1e-9);
        }
    }
    assert!(
        b.snapshot().units[0].position.x > 6.25,
        "{:?}",
        b.snapshot().units[0].position
    );
}
#[test]
fn fallback_uses_safer_cover_before_an_ally() {
    let mut s = scenario(p(8.5, 10.5), p(12.5, 10.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    s.map.obstacles.push(obstacle(5, 8, 2.4));
    s.units.push(UnitPlacement {
        id: 3,
        faction: 0,
        definition_id: "infantry".into(),
        position: p(5.5, 12.5),
        doctrine: catalog().default_doctrine,
    });
    let mut b = Battle::new(s).unwrap();
    // A solid obstacle requires routing around its footprint before reaching protection.
    run(&mut b, 35);
    let q = b.snapshot().units[0].position;
    assert!(q.x < 8.5 && q.y < 10.5, "fallback position: {q:?}");
}
#[test]
fn no_known_threat_means_low_hp_unit_defends() {
    let mut s = scenario(p(2.5, 2.5), p(29.5, 17.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    s.map.patrol = vec![p(10.5, 2.5)];
    let mut b = Battle::new(s).unwrap();
    run(&mut b, 20);
    let u = &b.snapshot().units[0];
    assert_eq!(u.behavior, Behavior::Defending);
    assert_eq!(u.position, p(2.5, 2.5));
}
#[test]
fn malformed_terrain_is_rejected_without_panicking() {
    let mut s = scenario(p(2.5, 2.5), p(5.5, 5.5));
    s.map.elevations[0] = f64::NAN;
    assert!(!validate_scenario(&s, true).valid);
}

#[test]
fn combat_reports_elimination_and_casualties() {
    let mut b = Battle::new(scenario(p(3.5, 3.5), p(7.5, 3.5))).unwrap();
    run(&mut b, 1000);
    let result = b.snapshot().result.unwrap();
    assert_eq!(result.reason, "elimination");
    assert!(result.winner.is_some());
    assert_eq!(result.factions.iter().map(|f| f.casualties).sum::<u32>(), 1);
}
#[test]
fn fallback_can_fire_at_a_visible_enemy() {
    let mut s = scenario(p(8.5, 10.5), p(12.5, 10.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    let mut b = Battle::new(s).unwrap();
    b.step();
    assert_eq!(b.snapshot().units[0].behavior, Behavior::FallingBack);
    assert_eq!(b.snapshot().units[0].shots_fired, 1);
}
#[test]
fn fallback_moves_toward_safer_ally_when_no_cover() {
    let mut s = scenario(p(8.5, 10.5), p(12.5, 10.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    s.units.push(UnitPlacement {
        id: 3,
        faction: 0,
        definition_id: "infantry".into(),
        position: p(5.5, 12.5),
        doctrine: catalog().default_doctrine,
    });
    let mut b = Battle::new(s).unwrap();
    run(&mut b, 5);
    let q = b.snapshot().units[0].position;
    assert!(q.x < 8.5 && q.y > 10.5);
}
#[test]
fn fallback_holds_when_no_safer_position_exists() {
    let mut s = scenario(p(0.5, 0.5), p(4.5, 4.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    let mut b = Battle::new(s).unwrap();
    b.step();
    assert_eq!(b.snapshot().units[0].behavior, Behavior::Defending);
    assert_eq!(b.snapshot().units[0].position, p(0.5, 0.5));
    assert_eq!(b.snapshot().units[0].shots_fired, 1);
}
#[test]
fn engagement_moves_to_nearby_cover() {
    let mut s = scenario(p(8.5, 10.5), p(13.5, 10.5));
    s.units[0].doctrine.position_preference = PositionPreference::Cover;
    s.map.obstacles.push(obstacle(9, 8, 0.9));
    let mut b = Battle::new(s).unwrap();
    run(&mut b, 10);
    assert!(b.snapshot().units[0].position.y < 10.5);
}
#[test]
fn fallback_skips_unreachable_cover_for_safer_ally() {
    let mut s = scenario(p(8.5, 10.5), p(12.5, 10.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    s.map.obstacles = vec![
        obstacle(1, 2, 2.4),
        obstacle(3, 2, 2.4),
        obstacle(2, 1, 2.4),
        obstacle(2, 3, 2.4),
    ];
    s.units.push(UnitPlacement {
        id: 3,
        faction: 0,
        definition_id: "infantry".into(),
        position: p(5.5, 12.5),
        doctrine: catalog().default_doctrine,
    });
    let mut b = Battle::new(s).unwrap();
    run(&mut b, 5);
    let q = b.snapshot().units[0].position;
    assert!(q.x < 8.5 && q.y > 10.5);
}

#[test]
fn fallback_finds_an_alternative_when_direct_retreat_is_blocked() {
    let mut s = scenario(p(8.5, 10.5), p(12.5, 10.5));
    s.units[0].doctrine.fallback_hp = 100.0;
    s.map.obstacles.push(obstacle(4, 10, 2.4));
    let mut b = Battle::new(s).unwrap();
    run(&mut b, 5);
    let unit = &b.snapshot().units[0];
    assert_eq!(unit.behavior, Behavior::FallingBack);
    assert!((unit.position.x - 8.5).abs() + (unit.position.y - 10.5).abs() > 0.1);
}

#[test]
fn seeded_combat_reproduces_every_snapshot_through_result() {
    let s = scenario(p(3.5, 3.5), p(7.5, 3.5));
    let mut a = Battle::new(s.clone()).unwrap();
    let mut b = Battle::new(s).unwrap();
    let mut shots = false;
    let mut hits = false;
    let mut finished = false;
    for _ in 0..1000 {
        a.step();
        b.step();
        let actual = a.snapshot();
        assert_eq!(actual, b.snapshot());
        shots |= actual.units.iter().any(|u| u.shots_fired > 0);
        hits |= actual.units.iter().any(|u| u.hp < catalog().unit.max_hp);
        if actual.result.is_some() {
            finished = true;
            break;
        }
    }
    assert!(shots && hits && finished);
}
#[test]
fn rejects_nonfinite_position() {
    let mut s = scenario(p(2.5, 2.5), p(8.5, 2.5));
    s.units[0].position.x = f64::NAN;
    assert!(!validate_scenario(&s, true).valid);
    assert!(Battle::new(s).is_err());
}
#[test]
fn rejects_nonfinite_doctrine() {
    let mut s = scenario(p(2.5, 2.5), p(8.5, 2.5));
    s.units[0].doctrine.engagement_distance = f64::NAN;
    assert!(!validate_scenario(&s, true).valid);
    s.units[0].doctrine.engagement_distance = 8.0;
    s.units[0].doctrine.fallback_hp = f64::NAN;
    assert!(!validate_scenario(&s, true).valid);
}
#[test]
fn rejects_unknown_unit_definition() {
    let mut s = scenario(p(2.5, 2.5), p(8.5, 2.5));
    s.units[0].definition_id = "unknown".into();
    assert!(!validate_scenario(&s, true).valid);
    assert!(Battle::new(s).is_err());
}
#[test]
fn rejects_duplicate_unit_ids() {
    let mut s = scenario(p(2.5, 2.5), p(8.5, 2.5));
    s.units[1].id = s.units[0].id;
    assert!(!validate_scenario(&s, true).valid);
    assert!(Battle::new(s).is_err());
}
#[test]
fn rejects_more_than_eight_units_per_faction() {
    let mut s = scenario(p(2.5, 2.5), p(25.5, 2.5));
    for id in 3..=10 {
        let mut u = s.units[0].clone();
        u.id = id;
        u.position = p(id as f64, 5.5);
        s.units.push(u);
    }
    assert!(!validate_scenario(&s, true).valid);
    assert!(Battle::new(s).is_err());
}
#[test]
fn sight_includes_fourteen_units_but_not_beyond() {
    let mut inside = Battle::new(scenario(p(2.5, 5.5), p(16.5, 5.5))).unwrap();
    let mut outside = Battle::new(scenario(p(2.5, 5.5), p(16.501, 5.5))).unwrap();
    inside.step();
    outside.step();
    assert_eq!(inside.snapshot().units[0].target_id, Some(2));
    assert_eq!(outside.snapshot().units[0].target_id, None);
}

#[test]
fn every_terrain_preset_repeats_snapshots_and_restores_the_same_start() {
    for preset in catalog().maps {
        let mut s = scenario(p(9.5, 9.5), p(22.5, 9.5));
        s.map = preset.map;
        s.time_limit = 3.0;
        s.units[0].doctrine.route_preference = RoutePreference::Protected;
        let mut a = Battle::new(s.clone()).unwrap();
        let initial = a.snapshot();
        let mut b = Battle::new(s.clone()).unwrap();
        let mut moved = false;
        for _ in 0..65 {
            a.step();
            b.step();
            assert_eq!(a.snapshot(), b.snapshot(), "{}", preset.name);
            moved |= a.snapshot().units[0].position != initial.units[0].position;
        }
        assert!(moved, "{} did not move", preset.name);
        assert_eq!(a.snapshot().result.unwrap().reason, "timeout");
        assert_eq!(Battle::new(s).unwrap().snapshot(), initial);
    }
}
