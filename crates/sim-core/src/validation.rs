use crate::{map, types::*};
use std::collections::HashSet;
fn doctrine_valid(d: &Doctrine) -> bool {
    let g = &d.grenades;
    let m = &d.medical;
    let r = crate::definitions::doctrine_rules();
    fn valid(v: f64, r: &Rule) -> bool {
        v.is_finite()
            && (r.min..=r.max).contains(&v)
            && (((v - r.min) / r.step).round() - (v - r.min) / r.step).abs() < 1e-8
    }
    valid(g.max_distance, &r.grenade_max_distance)
        && valid(g.min_enemies as f64, &r.grenade_min_enemies)
        && valid(g.safety_margin, &r.grenade_safety_margin)
        && g.max_distance > crate::consumables::BLAST_RADIUS + g.safety_margin
        && valid(m.hp_threshold, &r.medical_hp_threshold)
        && valid(m.safe_seconds, &r.medical_safe_seconds)
        && valid(m.cooldown_seconds, &r.medical_cooldown_seconds)
}
fn structural_problem(s: &Scenario) -> Option<&'static str> {
    let r = crate::definitions::editor_rules();
    let m = &s.map;
    if !(r.min_dimension..=r.max_dimension).contains(&m.width)
        || !(r.min_dimension..=r.max_dimension).contains(&m.height)
    {
        return Some("Map dimensions must be between 1 and 128.");
    }
    let area = (m.width * m.height) as usize;
    if m.surfaces.len() != area
        || m.elevations.len() != ((m.width + 1) * (m.height + 1)) as usize
        || !m
            .elevations
            .iter()
            .all(|h| h.is_finite() && *h >= r.min_elevation && *h <= r.max_elevation)
    {
        return Some("Terrain arrays must match map dimensions and contain finite elevations between -100 and 100.");
    }
    if s.units.len() > 256
        || s.factions.len() > 256
        || m.patrol.len() > area
        || m.obstacles.len() > area
    {
        return Some("The draft contains too many factions, units, patrol points, or obstacles.");
    }
    let mut footprint = 0usize;
    for o in &m.obstacles {
        if o.width == 0
            || o.height == 0
            || !o.x.checked_add(o.width).is_some_and(|x| x <= m.width)
            || !o.y.checked_add(o.height).is_some_and(|y| y <= m.height)
            || !o.height_metres.is_finite()
            || o.height_metres <= 0.0
            || o.height_metres > r.max_obstacle_height
        {
            return Some(
                "Obstacles must have positive dimensions and height and fit inside the map.",
            );
        }
        footprint += (o.width * o.height) as usize;
        if footprint > area * 4 {
            return Some("The combined obstacle footprint is too large.");
        }
    }
    if s.units.iter().any(|u| !doctrine_valid(&u.doctrine)) {
        return Some("Inventory doctrine values are outside supported ranges or inconsistent.");
    }
    if !s.time_limit.is_finite()
        || s.units.iter().any(|u| {
            !u.doctrine.engagement_distance.is_finite() || !u.doctrine.fallback_hp.is_finite()
        })
        || m.patrol
            .iter()
            .chain(s.units.iter().map(|u| &u.position))
            .any(|p| !p.x.is_finite() || !p.y.is_finite())
    {
        return Some("Scenario coordinates and numeric values must be finite.");
    }
    None
}
fn grid_blocked(m: &ArenaMap, occupied: &[bool], p: Point) -> bool {
    if !map::inside(m, p) {
        return true;
    }
    let x = p.x.floor() as i32;
    let y = p.y.floor() as i32;
    for cy in (y - 1).max(0)..=(y + 1).min(m.height as i32 - 1) {
        for cx in (x - 1).max(0)..=(x + 1).min(m.width as i32 - 1) {
            if occupied[cy as usize * m.width as usize + cx as usize] {
                let dx = p.x - p.x.clamp(cx as f64, cx as f64 + 1.0);
                let dy = p.y - p.y.clamp(cy as f64, cy as f64 + 1.0);
                if dx * dx + dy * dy < 0.25_f64.powi(2) - 1e-9 {
                    return true;
                }
            }
        }
    }
    false
}
pub fn validate_scenario(s: &Scenario, for_start: bool) -> Validation {
    if let Some(message) = structural_problem(s) {
        return Validation {
            valid: false,
            issues: vec![ValidationIssue {
                message: message.into(),
                unit_id: None,
            }],
        };
    }
    let mut issues = Vec::new();
    let mut add = |message: &str, unit_id| {
        issues.push(ValidationIssue {
            message: message.into(),
            unit_id,
        })
    };
    if s.factions.len() != 2 || s.factions[0] == s.factions[1] {
        add("Exactly two distinct factions are required.", None);
    }
    if !s.time_limit.is_finite() || !(1.0..=3600.0).contains(&s.time_limit) {
        add("Time limit must be between 1 and 3600 seconds.", None);
    }
    let mut occupied = vec![false; (s.map.width * s.map.height) as usize];
    for o in &s.map.obstacles {
        let mut overlap = false;
        for y in o.y..o.y + o.height {
            for x in o.x..o.x + o.width {
                let index = (y * s.map.width + x) as usize;
                overlap |= occupied[index];
                occupied[index] = true;
            }
        }
        if overlap {
            add("Obstacles must not overlap.", None);
        }
    }
    for p in &s.map.patrol {
        if grid_blocked(&s.map, &occupied, *p) {
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
        if grid_blocked(&s.map, &occupied, u.position) {
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

/// Analyze an editable draft without querying malformed terrain.
pub fn analyze_scenario(s: &Scenario) -> ScenarioAnalysis {
    let m = &s.map;
    if let Some(message) = structural_problem(s) {
        return ScenarioAnalysis {
            structural_valid: false,
            can_start: false,
            issues: vec![AnalysisIssue {
                severity: "error".into(),
                message: message.into(),
                unit_id: None,
                position: None,
            }],
            cells: vec![],
            steep_edges: vec![],
        };
    }
    let structural_valid = true;
    let validation = validate_scenario(s, true);
    let mut occupancy = vec![false; (m.width * m.height) as usize];
    for o in &m.obstacles {
        for y in o.y..o.y + o.height {
            for x in o.x..o.x + o.width {
                occupancy[(y * m.width + x) as usize] = true;
            }
        }
    }
    let mut patrol_errors = m
        .patrol
        .iter()
        .copied()
        .filter(|p| grid_blocked(m, &occupancy, *p));
    let mut issues: Vec<AnalysisIssue> = validation
        .issues
        .into_iter()
        .map(|issue| {
            let position = issue
                .unit_id
                .and_then(|id| s.units.iter().find(|u| u.id == id).map(|u| u.position))
                .or_else(|| {
                    if issue.message == "Patrol points must be on open ground." {
                        patrol_errors.next()
                    } else {
                        None
                    }
                });
            AnalysisIssue {
                severity: "error".into(),
                message: issue.message,
                unit_id: issue.unit_id,
                position,
            }
        })
        .collect();
    if m.patrol.is_empty() {
        issues.push(AnalysisIssue {
            severity: "warning".into(),
            message: "The map has no patrol points.".into(),
            unit_id: None,
            position: None,
        });
    }
    let w = m.width as usize;
    let h = m.height as usize;
    let occupied = occupancy;
    let point = |i: usize| Point {
        x: (i % w) as f64 + 0.5,
        y: (i / w) as f64 + 0.5,
    };
    let cells: Vec<_> = (0..w * h)
        .map(|i| {
            let p = point(i);
            TerrainCellInfo {
                surface: map::surface(m, p),
                elevation: map::ground_height(m, p),
                movement_multiplier: map::movement_multiplier(m, p, p),
                traversable: !occupied[i],
            }
        })
        .collect();
    let mut links = vec![Vec::new(); w * h];
    let mut steep_edges = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let a = y * w + x;
            if occupied[a] {
                continue;
            }
            for (dx, dy) in [(1isize, 0isize), (0, 1), (1, 1), (-1, 1)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || nx >= w as isize || ny >= h as isize {
                    continue;
                }
                let b = ny as usize * w + nx as usize;
                if occupied[b]
                    || (dx != 0
                        && dy != 0
                        && (occupied[y * w + nx as usize] || occupied[ny as usize * w + x]))
                {
                    continue;
                }
                if map::ground_traversable(m, point(a), point(b)) {
                    links[a].push(b);
                    links[b].push(a);
                } else {
                    let pa = point(a);
                    let pb = point(b);
                    steep_edges.push(Point {
                        x: (pa.x + pb.x) / 2.0,
                        y: (pa.y + pb.y) / 2.0,
                    });
                }
            }
        }
    }
    if !steep_edges.is_empty() {
        issues.push(AnalysisIssue {
            severity: "warning".into(),
            message: "Steep terrain blocks some transitions.".into(),
            unit_id: None,
            position: steep_edges.first().copied(),
        });
    }
    let mut seen = vec![false; w * h];
    let mut regions = 0;
    for i in 0..w * h {
        if occupied[i] || seen[i] {
            continue;
        }
        regions += 1;
        seen[i] = true;
        let mut stack = vec![i];
        while let Some(a) = stack.pop() {
            for &b in &links[a] {
                if !seen[b] {
                    seen[b] = true;
                    stack.push(b);
                }
            }
        }
    }
    if regions > 1 {
        issues.push(AnalysisIssue {
            severity: "warning".into(),
            message: "Open terrain contains disconnected regions.".into(),
            unit_id: None,
            position: None,
        });
    }
    ScenarioAnalysis {
        structural_valid,
        can_start: validation.valid,
        issues,
        cells,
        steep_edges,
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
    fn structural_gate_rejects_nonfinite_values_and_excessive_inputs() {
        let mut s = scenario();
        s.time_limit = f64::NAN;
        assert!(!analyze_scenario(&s).structural_valid);
        assert!(!validate_scenario(&s, true).valid);
        s.time_limit = 180.0;
        s.map = ArenaMap::flat(2, 2);
        s.map.patrol = vec![Point { x: 0.5, y: 0.5 }; 5];
        assert!(!analyze_scenario(&s).structural_valid);
        s.map.patrol.clear();
        s.map.obstacles = vec![
            Obstacle {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
                height_metres: 1.0
            };
            4
        ];
        assert!(analyze_scenario(&s).structural_valid);
        s.map.obstacles.push(s.map.obstacles[0].clone());
        assert!(!analyze_scenario(&s).structural_valid);
        s.map.obstacles.clear();
        let unit = UnitPlacement {
            id: 1,
            faction: 1,
            definition_id: "infantry".into(),
            position: Point { x: 0.5, y: 0.5 },
            doctrine: crate::catalog().default_doctrine,
        };
        s.units = vec![unit; 257];
        assert!(!analyze_scenario(&s).structural_valid);
        s.units.truncate(1);
        s.units[0].doctrine.fallback_hp = f64::INFINITY;
        assert!(!analyze_scenario(&s).structural_valid);
    }
    #[test]
    fn analysis_reports_each_blocked_patrol_location() {
        let mut s = scenario();
        s.map = ArenaMap::flat(3, 2);
        s.map.obstacles.push(Obstacle {
            x: 1,
            y: 0,
            width: 1,
            height: 2,
            height_metres: 1.0,
        });
        s.map.patrol = vec![Point { x: 1.5, y: 0.5 }, Point { x: 1.5, y: 1.5 }];
        let result = analyze_scenario(&s);
        let positions: Vec<_> = result
            .issues
            .iter()
            .filter(|i| i.message.contains("Patrol points"))
            .map(|i| i.position)
            .collect();
        assert_eq!(
            positions,
            vec![Some(s.map.patrol[0]), Some(s.map.patrol[1])]
        );
    }
    #[test]
    fn full_size_analysis_handles_many_obstacles_and_patrol_points() {
        let mut s = scenario();
        s.map = ArenaMap::flat(128, 128);
        for y in 0..128 {
            for x in 0..128 {
                if (x + y) % 2 == 0 {
                    s.map.obstacles.push(Obstacle {
                        x,
                        y,
                        width: 1,
                        height: 1,
                        height_metres: 1.0,
                    });
                }
                s.map.patrol.push(Point {
                    x: x as f64 + 0.5,
                    y: y as f64 + 0.5,
                });
            }
        }
        let result = analyze_scenario(&s);
        assert!(result.structural_valid);
        assert_eq!(result.cells.len(), 128 * 128);
        assert!(!result.can_start);
    }
    #[test]
    fn analysis_rejects_unsafe_structure_before_terrain_queries() {
        let mut s = scenario();
        s.map.elevations.clear();
        let result = analyze_scenario(&s);
        assert!(!result.structural_valid && !result.can_start);
        assert!(result.cells.is_empty());
    }
    #[test]
    fn analysis_keeps_invalid_layout_editable_and_warns_about_regions() {
        let mut s = scenario();
        s.map = ArenaMap::flat(3, 2);
        s.map.obstacles.push(Obstacle {
            x: 1,
            y: 0,
            width: 1,
            height: 2,
            height_metres: 1.0,
        });
        s.map.patrol.push(Point { x: 1.5, y: 0.5 });
        let result = analyze_scenario(&s);
        assert!(result.structural_valid && !result.can_start);
        assert!(!result.cells[1].traversable);
        assert!(result
            .issues
            .iter()
            .any(|i| i.severity == "error" && i.position == Some(Point { x: 1.5, y: 0.5 })));
        assert!(result
            .issues
            .iter()
            .any(|i| i.severity == "warning" && i.message.contains("disconnected")));
    }
    #[test]
    fn analysis_reports_steep_transitions_and_empty_patrol() {
        let mut s = scenario();
        s.map = ArenaMap::flat(3, 2);
        for y in 0..=2 {
            s.map.elevations[y * 4 + 1] = 2.0;
        }
        let result = analyze_scenario(&s);
        assert!(!result.steep_edges.is_empty());
        assert!(result
            .issues
            .iter()
            .any(|i| i.severity == "warning" && i.message.contains("no patrol")));
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
    #[test]
    fn inventory_doctrine_rejects_invalid_steps_and_unsafe_ranges() {
        let mut s = scenario();
        s.units.push(UnitPlacement {
            id: 1,
            faction: 1,
            definition_id: "infantry".into(),
            position: Point { x: 2.5, y: 2.5 },
            doctrine: crate::catalog().default_doctrine,
        });
        s.units[0].doctrine.grenades.max_distance = 2.1;
        let invalid = validate_scenario(&s, false);
        assert!(!invalid.valid);
        assert!(invalid.issues[0].message.contains("Inventory doctrine"));
        s.units[0].doctrine.grenades.max_distance = 2.0;
        s.units[0].doctrine.grenades.safety_margin = 0.5;
        assert!(!analyze_scenario(&s).structural_valid);
        s.units[0].doctrine.grenades.max_distance = 3.0;
        assert!(analyze_scenario(&s).structural_valid);
    }
}
