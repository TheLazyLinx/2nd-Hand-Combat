use crate::{map, types::*};
use std::collections::HashSet;
pub fn validate_scenario(s: &Scenario, for_start: bool) -> Validation {
    let mut issues = Vec::new();
    let mut add = |message: &str, unit_id| {
        issues.push(ValidationIssue {
            message: message.into(),
            unit_id,
        })
    };
    if s.map.width == 0 || s.map.height == 0 || s.map.width > 128 || s.map.height > 128 {
        add("Map dimensions must be between 1 and 128.", None);
    }
    if s.factions.len() != 2 || s.factions[0] == s.factions[1] {
        add("Exactly two distinct factions are required.", None);
    }
    if !s.time_limit.is_finite() || !(1.0..=3600.0).contains(&s.time_limit) {
        add("Time limit must be between 1 and 3600 seconds.", None);
    }
    for p in s.map.walls.iter().chain(s.map.cover.iter()) {
        if !p.x.is_finite()
            || !p.y.is_finite()
            || p.x.fract() != 0.0
            || p.y.fract() != 0.0
            || p.x < 0.0
            || p.y < 0.0
            || p.x >= s.map.width as f64
            || p.y >= s.map.height as f64
        {
            add(
                "Terrain cells must be integer positions inside the map.",
                None,
            );
        }
    }
    for p in &s.map.patrol {
        if map::blocked(&s.map, *p) {
            add("Patrol points must be on open ground.", None);
        }
    }
    let mut ids = HashSet::new();
    for (i, u) in s.units.iter().enumerate() {
        if !ids.insert(u.id) {
            add("Unit IDs must be unique.", Some(u.id));
        }
        if !s.factions.contains(&u.faction) {
            add("Unit faction is not in the scenario.", Some(u.id));
        }
        if u.definition_id != "infantry" {
            add("Unknown unit definition.", Some(u.id));
        }
        if map::blocked(&s.map, u.position) {
            add(
                "Unit must be placed on open ground inside the map.",
                Some(u.id),
            );
        }
        if s.units[..i]
            .iter()
            .any(|other| other.position.distance(u.position) < 0.5)
        {
            add("Unit placements overlap.", Some(u.id));
        }
        if !u.doctrine.engagement_distance.is_finite()
            || !(2.0..=12.0).contains(&u.doctrine.engagement_distance)
            || !u.doctrine.fallback_hp.is_finite()
            || !(0.0..=100.0).contains(&u.doctrine.fallback_hp)
        {
            add(
                "Doctrine values are outside the supported range.",
                Some(u.id),
            );
        }
    }
    for faction in &s.factions {
        let count = s.units.iter().filter(|u| u.faction == *faction).count();
        if count > 8 {
            add("A faction can have at most eight units.", None);
        }
        if for_start && count == 0 {
            add("Each faction needs at least one unit to start.", None);
        }
    }
    Validation {
        valid: issues.is_empty(),
        issues,
    }
}
