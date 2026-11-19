//! Standard doctrine scenario suite, multi-seed reporting, and outcome fingerprints.

use crate::metrics::{fnv1a_update, run_battle_metrics, RunMetrics};
use serde::{Deserialize, Serialize};
use sim_core::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const DEFAULT_SEEDS: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
const MAX_TICKS: u32 = 1200; // 60 s simulated limit per standard scenario

#[derive(Clone, Debug)]
pub struct StandardScenario {
    pub name: &'static str,
    pub group: &'static str,
    pub description: &'static str,
    pub build: fn(u32) -> Scenario,
}

#[derive(Clone, Debug)]
pub struct SuiteReport {
    pub name: String,
    pub group: String,
    pub seeds: Vec<u32>,
    pub runs: Vec<RunMetrics>,
    pub win_rate_f0: f64,
    pub win_rate_f1: f64,
    pub draw_rate: f64,
    pub mean_elapsed: f64,
    pub mean_shots_f0: f64,
    pub mean_damage_taken_f0: f64,
    pub mean_cover_share_f0: f64,
    pub mean_protection_f0: f64,
    pub mean_veg_share_f0: f64,
    pub mean_exposed_advancing_f0: f64,
    pub mean_behavior_f0: BTreeMap<String, f64>,
    pub mean_item_uses_f0: BTreeMap<String, f64>,
    pub suite_fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FingerprintFile {
    pub seeds: Vec<u32>,
    pub scenarios: BTreeMap<String, String>,
}

fn base_doctrine() -> Doctrine {
    let mut d = catalog().default_doctrine;
    d.fallback_hp = 0.0;
    d.grenades.policy = GrenadePolicy::Never;
    d.medical.policy = MedicalPolicy::Never;
    d
}

fn place(id: u32, faction: u32, x: f64, y: f64, doctrine: Doctrine) -> UnitPlacement {
    UnitPlacement {
        id,
        faction,
        definition_id: "infantry".into(),
        position: Point { x, y },
        doctrine,
        loadout: None,
    }
}

fn engagement_scenario(seed: u32, engagement_distance: f64) -> Scenario {
    let mut d0 = base_doctrine();
    d0.engagement_distance = engagement_distance;
    let d1 = base_doctrine();
    Scenario {
        map: ArenaMap::flat(20, 8),
        seed,
        factions: default_factions(),
        loadouts: vec![],
        units: vec![place(1, 0, 2.5, 4.5, d0), place(2, 1, 14.5, 4.5, d1)],
        time_limit: Some(60.0),
    }
}

fn engagement_short(seed: u32) -> Scenario {
    engagement_scenario(seed, 3.0)
}

fn engagement_long(seed: u32) -> Scenario {
    engagement_scenario(seed, 12.0)
}

fn position_scenario(seed: u32, pref: PositionPreference) -> Scenario {
    // Open starting ground between low cover to the north and woodland concealment to the south.
    let mut map = ArenaMap::flat(16, 10);
    for y in 6..9 {
        for x in 3..8 {
            map.surfaces[y * 16 + x] = Surface::Woodland;
        }
    }
    for (x, y) in [(7, 3), (7, 4)] {
        map.obstacles.push(Obstacle {
            x,
            y,
            width: 1,
            height: 1,
            height_metres: 0.9,
        });
    }
    let mut d0 = base_doctrine();
    d0.engagement_distance = 10.0;
    d0.position_preference = pref;
    let mut d1 = base_doctrine();
    d1.engagement_distance = 10.0;
    Scenario {
        map,
        seed,
        factions: default_factions(),
        loadouts: vec![],
        units: vec![
            place(1, 0, 5.5, 5.5, d0.clone()),
            place(2, 0, 6.5, 5.5, d0),
            place(3, 1, 12.5, 4.5, d1.clone()),
            place(4, 1, 12.5, 6.5, d1),
        ],
        time_limit: Some(60.0),
    }
}

fn position_cover(seed: u32) -> Scenario {
    position_scenario(seed, PositionPreference::Cover)
}
fn position_balanced(seed: u32) -> Scenario {
    position_scenario(seed, PositionPreference::Balanced)
}
fn position_concealment(seed: u32) -> Scenario {
    position_scenario(seed, PositionPreference::Concealment)
}

fn route_scenario(seed: u32, pref: RoutePreference) -> Scenario {
    // Direct open lane along y=5 vs shielded northern flank behind a chest-high parapet wall
    // (1.45m blocks muzzle/body fire at <=1.4m while eye height 1.6m preserves visual contact).
    let mut map = ArenaMap::flat(18, 10);
    map.obstacles.push(Obstacle {
        x: 10,
        y: 2,
        width: 1,
        height: 3,
        height_metres: 1.45,
    });
    let mut d0 = base_doctrine();
    d0.engagement_distance = 3.0;
    d0.route_preference = pref;
    let mut d1 = base_doctrine();
    d1.engagement_distance = 12.0;
    Scenario {
        map,
        seed,
        factions: default_factions(),
        loadouts: vec![],
        units: vec![
            place(1, 0, 2.5, 5.5, d0.clone()),
            place(2, 0, 3.5, 5.5, d0),
            place(3, 1, 13.5, 5.5, d1.clone()),
            place(4, 1, 13.5, 6.5, d1),
        ],
        time_limit: Some(60.0),
    }
}

fn route_protected(seed: u32) -> Scenario {
    route_scenario(seed, RoutePreference::Protected)
}
fn route_fastest(seed: u32) -> Scenario {
    route_scenario(seed, RoutePreference::Fastest)
}

fn grenade_scenario(seed: u32, policy: GrenadePolicy) -> Scenario {
    let mut d0 = base_doctrine();
    d0.engagement_distance = 6.0;
    d0.grenades.policy = policy;
    d0.grenades.max_distance = 6.0;
    d0.grenades.safety_margin = 0.25;
    let mut d1 = base_doctrine();
    d1.engagement_distance = 8.0;
    Scenario {
        map: ArenaMap::flat(16, 10),
        seed,
        factions: default_factions(),
        loadouts: vec![],
        units: vec![
            place(1, 0, 4.5, 4.5, d0.clone()),
            place(2, 0, 4.5, 5.5, d0),
            place(3, 1, 9.5, 4.5, d1.clone()),
            place(4, 1, 9.5, 5.5, d1.clone()),
            place(5, 1, 10.2, 5.0, d1),
        ],
        time_limit: Some(60.0),
    }
}

fn grenades_any_safe(seed: u32) -> Scenario {
    grenade_scenario(seed, GrenadePolicy::AnySafeTarget)
}
fn grenades_never(seed: u32) -> Scenario {
    grenade_scenario(seed, GrenadePolicy::Never)
}

fn medical_scenario(seed: u32, policy: MedicalPolicy) -> Scenario {
    let mut map = ArenaMap::flat(16, 10);
    for y in [3, 6] {
        map.obstacles.push(Obstacle {
            x: 6,
            y,
            width: 1,
            height: 1,
            height_metres: 0.9,
        });
        map.obstacles.push(Obstacle {
            x: 9,
            y,
            width: 1,
            height: 1,
            height_metres: 0.9,
        });
    }
    let mut d0 = base_doctrine();
    d0.engagement_distance = 9.0;
    d0.medical.policy = policy;
    d0.medical.hp_threshold = 80.0;
    d0.medical.cooldown_seconds = 3.0;
    let mut d1 = base_doctrine();
    d1.engagement_distance = 9.0;
    Scenario {
        map,
        seed,
        factions: default_factions(),
        loadouts: vec![],
        units: vec![
            place(1, 0, 4.5, 3.5, d0.clone()),
            place(2, 0, 4.5, 6.5, d0),
            place(3, 1, 11.5, 3.5, d1.clone()),
            place(4, 1, 11.5, 6.5, d1),
        ],
        time_limit: Some(60.0),
    }
}

fn medical_low_health(seed: u32) -> Scenario {
    medical_scenario(seed, MedicalPolicy::LowHealth)
}
fn medical_never(seed: u32) -> Scenario {
    medical_scenario(seed, MedicalPolicy::Never)
}

fn fallback_scenario(seed: u32, fallback_hp: f64) -> Scenario {
    let mut map = ArenaMap::flat(18, 10);
    for (x, y) in [(4, 3), (4, 6)] {
        map.obstacles.push(Obstacle {
            x,
            y,
            width: 1,
            height: 1,
            height_metres: 2.4,
        });
    }
    let mut d0 = base_doctrine();
    d0.engagement_distance = 8.0;
    d0.fallback_hp = fallback_hp;
    let mut d1 = base_doctrine();
    d1.engagement_distance = 8.0;
    Scenario {
        map,
        seed,
        factions: default_factions(),
        loadouts: vec![],
        units: vec![
            place(1, 0, 7.5, 3.5, d0.clone()),
            place(2, 0, 7.5, 6.5, d0),
            place(3, 1, 13.5, 3.5, d1.clone()),
            place(4, 1, 13.5, 6.5, d1),
        ],
        time_limit: Some(60.0),
    }
}

fn fallback_high(seed: u32) -> Scenario {
    fallback_scenario(seed, 80.0)
}
fn fallback_none(seed: u32) -> Scenario {
    fallback_scenario(seed, 0.0)
}

pub fn standard_scenarios() -> Vec<StandardScenario> {
    vec![
        StandardScenario {
            name: "engagement_short",
            group: "engagement",
            description: "1v1 open arena, F0 engagement_distance=3",
            build: engagement_short,
        },
        StandardScenario {
            name: "engagement_long",
            group: "engagement",
            description: "1v1 open arena, F0 engagement_distance=12",
            build: engagement_long,
        },
        StandardScenario {
            name: "position_cover",
            group: "position",
            description: "2v2 cover + woodland, F0 Cover preference",
            build: position_cover,
        },
        StandardScenario {
            name: "position_balanced",
            group: "position",
            description: "2v2 cover + woodland, F0 Balanced preference",
            build: position_balanced,
        },
        StandardScenario {
            name: "position_concealment",
            group: "position",
            description: "2v2 cover + woodland, F0 Concealment preference",
            build: position_concealment,
        },
        StandardScenario {
            name: "route_protected",
            group: "route",
            description: "2v2 open lane vs parapet flank, F0 Protected route",
            build: route_protected,
        },
        StandardScenario {
            name: "route_fastest",
            group: "route",
            description: "2v2 open lane vs parapet flank, F0 Fastest route",
            build: route_fastest,
        },
        StandardScenario {
            name: "grenades_any_safe",
            group: "grenades",
            description: "2v3 cluster within 6m, F0 AnySafeTarget grenades",
            build: grenades_any_safe,
        },
        StandardScenario {
            name: "grenades_never",
            group: "grenades",
            description: "2v3 cluster within 6m, F0 Never grenades",
            build: grenades_never,
        },
        StandardScenario {
            name: "medical_low_health",
            group: "medical",
            description: "2v2 cover duel, F0 LowHealth medical (80% HP)",
            build: medical_low_health,
        },
        StandardScenario {
            name: "medical_never",
            group: "medical",
            description: "2v2 cover duel, F0 Never medical",
            build: medical_never,
        },
        StandardScenario {
            name: "fallback_high",
            group: "fallback",
            description: "2v2 near walls, F0 fallback_hp=80%",
            build: fallback_high,
        },
        StandardScenario {
            name: "fallback_none",
            group: "fallback",
            description: "2v2 near walls, F0 fallback_hp=0%",
            build: fallback_none,
        },
    ]
}

pub fn run_scenario(spec: &StandardScenario, seeds: &[u32]) -> SuiteReport {
    let runs: Vec<RunMetrics> = seeds
        .iter()
        .map(|&seed| run_battle_metrics((spec.build)(seed), MAX_TICKS))
        .collect();
    let n = runs.len().max(1) as f64;
    let wins_f0 = runs.iter().filter(|r| r.winner == Some(0)).count() as f64;
    let wins_f1 = runs.iter().filter(|r| r.winner == Some(1)).count() as f64;
    let draws = runs.iter().filter(|r| r.winner.is_none()).count() as f64;

    let mut behavior_sums: BTreeMap<String, f64> = BTreeMap::new();
    let mut item_sums: BTreeMap<String, f64> = BTreeMap::new();
    let mut suite_hash: u64 = 0xcbf2_9ce4_8422_2325;

    for r in &runs {
        suite_hash = fnv1a_update(suite_hash, &r.seed.to_le_bytes());
        suite_hash = fnv1a_update(suite_hash, &r.fingerprint.to_le_bytes());
        let f0 = r.faction(0);
        for key in [
            "searching",
            "investigating",
            "advancing",
            "engaging",
            "falling_back",
            "defending",
        ] {
            *behavior_sums.entry(key.into()).or_default() += f0.behavior_share(key);
        }
        for (item, &count) in &f0.item_uses {
            *item_sums.entry(item.clone()).or_default() += count as f64;
        }
    }

    for v in behavior_sums.values_mut() {
        *v /= n;
    }
    for v in item_sums.values_mut() {
        *v /= n;
    }

    SuiteReport {
        name: spec.name.into(),
        group: spec.group.into(),
        seeds: seeds.to_vec(),
        win_rate_f0: wins_f0 / n,
        win_rate_f1: wins_f1 / n,
        draw_rate: draws / n,
        mean_elapsed: runs.iter().map(|r| r.elapsed).sum::<f64>() / n,
        mean_shots_f0: runs
            .iter()
            .map(|r| r.faction(0).shots_fired as f64)
            .sum::<f64>()
            / n,
        mean_damage_taken_f0: runs.iter().map(|r| r.faction(0).damage_taken).sum::<f64>() / n,
        mean_cover_share_f0: runs.iter().map(|r| r.faction(0).cover_share()).sum::<f64>() / n,
        mean_protection_f0: runs
            .iter()
            .map(|r| r.faction(0).mean_protection())
            .sum::<f64>()
            / n,
        mean_veg_share_f0: runs
            .iter()
            .map(|r| r.faction(0).vegetation_share())
            .sum::<f64>()
            / n,
        mean_exposed_advancing_f0: runs
            .iter()
            .map(|r| r.faction(0).exposed_advancing_ticks as f64)
            .sum::<f64>()
            / n,
        mean_behavior_f0: behavior_sums,
        mean_item_uses_f0: item_sums,
        suite_fingerprint: format!("{suite_hash:016x}"),
        runs,
    }
}

pub fn run_standard_suite(seeds: &[u32]) -> Vec<SuiteReport> {
    standard_scenarios()
        .iter()
        .map(|spec| run_scenario(spec, seeds))
        .collect()
}

pub fn fingerprints_from_reports(reports: &[SuiteReport], seeds: &[u32]) -> FingerprintFile {
    let mut scenarios = BTreeMap::new();
    for report in reports {
        scenarios.insert(report.name.clone(), report.suite_fingerprint.clone());
    }
    FingerprintFile {
        seeds: seeds.to_vec(),
        scenarios,
    }
}

pub fn load_fingerprints(path: &Path) -> Result<FingerprintFile, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))
}

pub fn save_fingerprints(path: &Path, file: &FingerprintFile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(file).map_err(|e| e.to_string())? + "\n";
    fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
}

pub fn format_markdown_report(reports: &[SuiteReport]) -> String {
    let mut out = String::new();
    out.push_str("| Scenario | F0 Win% | Draw% | Time (s) | Shots | Dmg Taken | Cover% | Veg% | Adv/Eng/FB% | Items (G/M) | Fingerprint |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- | --- |\n");
    for r in reports {
        let adv = r.mean_behavior_f0.get("advancing").copied().unwrap_or(0.0) * 100.0;
        let eng = r.mean_behavior_f0.get("engaging").copied().unwrap_or(0.0) * 100.0;
        let fb = (r
            .mean_behavior_f0
            .get("falling_back")
            .copied()
            .unwrap_or(0.0)
            + r.mean_behavior_f0.get("defending").copied().unwrap_or(0.0))
            * 100.0;
        let gren = r
            .mean_item_uses_f0
            .get("frag-grenade")
            .copied()
            .unwrap_or(0.0);
        let med = r
            .mean_item_uses_f0
            .get("medical-supply")
            .copied()
            .unwrap_or(0.0);
        out.push_str(&format!(
            "| {} | {:5.1}% | {:5.1}% | {:5.1} | {:5.1} | {:5.1} | {:5.1}% | {:5.1}% | {:2.0}/{:2.0}/{:2.0} | {:.1}/{:.1} | `{}` |\n",
            r.name,
            r.win_rate_f0 * 100.0,
            r.draw_rate * 100.0,
            r.mean_elapsed,
            r.mean_shots_f0,
            r.mean_damage_taken_f0,
            r.mean_cover_share_f0 * 100.0,
            r.mean_veg_share_f0 * 100.0,
            adv,
            eng,
            fb,
            gren,
            med,
            r.suite_fingerprint
        ));
    }
    out
}
