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
    let dimensions_valid =
        s.map.width > 0 && s.map.height > 0 && s.map.width <= 128 && s.map.height <= 128;
    let terrain_valid = dimensions_valid
        && s.map.surfaces.len() == s.map.width as usize * s.map.height as usize
        && s.map.elevations.len() == (s.map.width as usize + 1) * (s.map.height as usize + 1)
        && s.map
            .elevations
            .iter()
            .all(|h| h.is_finite() && h.abs() <= 100.0);
    if !terrain_valid {
        add("Terrain arrays must match map dimensions and contain finite elevations between -100 and 100.", None);
    }
    for (i, o) in s.map.obstacles.iter().enumerate() {
        let valid = o.width > 0
            && o.height > 0
            && o.x.checked_add(o.width).is_some_and(|x| x <= s.map.width)
            && o.y.checked_add(o.height).is_some_and(|y| y <= s.map.height)
            && o.height_metres.is_finite()
            && o.height_metres > 0.0
            && o.height_metres <= 100.0;
        if !valid {
            add(
                "Obstacles must have positive dimensions and height and fit inside the map.",
                None,
            );
        }
        if valid
            && s.map.obstacles[..i].iter().any(|p| {
                p.x < o.x + o.width
                    && o.x < p.x.saturating_add(p.width)
                    && p.y < o.y + o.height
                    && o.y < p.y.saturating_add(p.height)
            })
        {
            add("Obstacles must not overlap.", None);
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

#[cfg(test)]
mod tests {
    use super::*;
    fn scenario() -> Scenario {
        Scenario {
            map: crate::catalog().map,
            seed: 1,
            factions: vec![1, 2],
            units: vec![],
            time_limit: 180.0,
        }
    }
    #[test]
    fn rejects_malformed_terrain_arrays_without_accessing_them() {
        let mut s = scenario();
        s.map.elevations.clear();
        s.map.surfaces.clear();
        assert!(!validate_scenario(&s, false).valid);
    }
    #[test]
    fn rejects_overlapping_and_out_of_bounds_obstacles() {
        let mut s = scenario();
        s.map.obstacles.push(s.map.obstacles[0].clone());
        assert!(!validate_scenario(&s, false).valid);
        s.map.obstacles.clear();
        s.map.obstacles.push(Obstacle {
            x: u32::MAX,
            y: 0,
            width: 2,
            height: 1,
            height_metres: 1.0,
        });
        assert!(!validate_scenario(&s, false).valid);
    }
}
