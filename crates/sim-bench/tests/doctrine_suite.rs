use sim_bench::doctrine_suite::{
    fingerprints_from_reports, load_fingerprints, run_scenario, run_standard_suite,
    standard_scenarios, DEFAULT_SEEDS,
};
use std::path::Path;

fn scenario(name: &str) -> sim_bench::doctrine_suite::StandardScenario {
    standard_scenarios()
        .into_iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("unknown standard scenario `{name}`"))
}

#[test]
fn engagement_distance_changes_first_firing_position_across_seeds() {
    let short = run_scenario(&scenario("engagement_short"), DEFAULT_SEEDS);
    let long = run_scenario(&scenario("engagement_long"), DEFAULT_SEEDS);
    let closer_count = short
        .runs
        .iter()
        .zip(&long.runs)
        .filter(|(s, l)| {
            match (
                s.faction(0).mean_first_shot_x(),
                l.faction(0).mean_first_shot_x(),
            ) {
                (Some(sx), Some(lx)) => sx > lx + 2.0,
                _ => false,
            }
        })
        .count();
    assert!(
        closer_count * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected engagement_short to fire closer in >= 80% of seeds, got {closer_count}/{}",
        DEFAULT_SEEDS.len()
    );
}

#[test]
fn position_preference_separates_cover_and_concealment_across_seeds() {
    let cover = run_scenario(&scenario("position_cover"), DEFAULT_SEEDS);
    let balanced = run_scenario(&scenario("position_balanced"), DEFAULT_SEEDS);
    let concealment = run_scenario(&scenario("position_concealment"), DEFAULT_SEEDS);

    let cover_more_protected_than_concealment = cover
        .runs
        .iter()
        .zip(&concealment.runs)
        .filter(|(c, con)| c.faction(0).mean_protection() > con.faction(0).mean_protection())
        .count();
    let concealment_more_veg_than_cover = concealment
        .runs
        .iter()
        .zip(&cover.runs)
        .filter(|(con, c)| con.faction(0).vegetation_share() > c.faction(0).vegetation_share())
        .count();
    assert!(
        cover_more_protected_than_concealment * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected Cover to exceed Concealment protection in >= 80% of seeds, got {cover_more_protected_than_concealment}/{} (cover mean {:.3} vs concealment {:.3}, balanced {:.3})",
        DEFAULT_SEEDS.len(),
        cover.mean_protection_f0,
        concealment.mean_protection_f0,
        balanced.mean_protection_f0,
    );
    assert!(
        concealment_more_veg_than_cover * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected Concealment to exceed Cover vegetation share in >= 80% of seeds, got {concealment_more_veg_than_cover}/{} (concealment {:.1}% vs cover {:.1}%)",
        DEFAULT_SEEDS.len(),
        concealment.mean_veg_share_f0 * 100.0,
        cover.mean_veg_share_f0 * 100.0,
    );
}

#[test]
fn protected_route_reduces_exposed_advancing_ticks_across_seeds() {
    let protected = run_scenario(&scenario("route_protected"), DEFAULT_SEEDS);
    let fastest = run_scenario(&scenario("route_fastest"), DEFAULT_SEEDS);
    let less_exposed = protected
        .runs
        .iter()
        .zip(&fastest.runs)
        .filter(|(p, f)| {
            p.faction(0).exposed_advancing_ticks < f.faction(0).exposed_advancing_ticks
        })
        .count();
    assert!(
        less_exposed * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected Protected to have fewer exposed advancing ticks than Fastest in >= 80% of seeds, got {less_exposed}/{} (protected mean {:.1} vs fastest {:.1})",
        DEFAULT_SEEDS.len(),
        protected.mean_exposed_advancing_f0,
        fastest.mean_exposed_advancing_f0,
    );
}

#[test]
fn grenade_policy_controls_grenade_usage_across_seeds() {
    let active = run_scenario(&scenario("grenades_any_safe"), DEFAULT_SEEDS);
    let never = run_scenario(&scenario("grenades_never"), DEFAULT_SEEDS);
    assert!(
        never
            .runs
            .iter()
            .all(|r| r.faction(0).item_use_count("frag-grenade") == 0),
        "grenades_never must never throw grenades"
    );
    let threw = active
        .runs
        .iter()
        .filter(|r| r.faction(0).item_use_count("frag-grenade") > 0)
        .count();
    assert!(
        threw * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected grenades_any_safe to throw grenades in >= 80% of seeds, got {threw}/{}",
        DEFAULT_SEEDS.len()
    );
}

#[test]
fn medical_policy_controls_medical_usage_across_seeds() {
    let active = run_scenario(&scenario("medical_low_health"), DEFAULT_SEEDS);
    let never = run_scenario(&scenario("medical_never"), DEFAULT_SEEDS);
    assert!(
        never
            .runs
            .iter()
            .all(|r| r.faction(0).item_use_count("medical-supply") == 0),
        "medical_never must never use medical supplies"
    );
    let healed = active
        .runs
        .iter()
        .filter(|r| r.faction(0).item_use_count("medical-supply") > 0)
        .count();
    assert!(
        healed * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected medical_low_health to use medical supplies in >= 80% of seeds, got {healed}/{}",
        DEFAULT_SEEDS.len()
    );
}

#[test]
fn fallback_threshold_increases_fallback_and_defending_share_across_seeds() {
    let high = run_scenario(&scenario("fallback_high"), DEFAULT_SEEDS);
    let none = run_scenario(&scenario("fallback_none"), DEFAULT_SEEDS);
    let more_retreat = high
        .runs
        .iter()
        .zip(&none.runs)
        .filter(|(h, n)| {
            let h_share = h.faction(0).behavior_share("falling_back")
                + h.faction(0).behavior_share("defending");
            let n_share = n.faction(0).behavior_share("falling_back")
                + n.faction(0).behavior_share("defending");
            h_share > n_share + 0.05
        })
        .count();
    assert!(
        more_retreat * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected fallback_high to spend more time falling back/defending in >= 80% of seeds, got {more_retreat}/{}",
        DEFAULT_SEEDS.len()
    );
}

#[test]
fn fire_while_moving_increases_shots_while_advancing_across_seeds() {
    let on = run_scenario(&scenario("fire_on_move_on"), DEFAULT_SEEDS);
    let off = run_scenario(&scenario("fire_on_move_off"), DEFAULT_SEEDS);
    let more_shots = on
        .runs
        .iter()
        .zip(&off.runs)
        .filter(|(a, b)| a.faction(0).shots_fired > b.faction(0).shots_fired)
        .count();
    assert!(
        more_shots * 10 >= DEFAULT_SEEDS.len() * 8,
        "expected fire_on_move_on to fire more shots than fire_on_move_off in >= 80% of seeds, got {more_shots}/{} (mean {:.1} vs {:.1})",
        DEFAULT_SEEDS.len(),
        on.mean_shots_f0,
        off.mean_shots_f0
    );
}

#[test]
fn standard_doctrine_outcome_fingerprints_match_baseline() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fingerprints.json");
    let expected = load_fingerprints(&path).unwrap_or_else(|e| {
        panic!("Missing or unreadable baseline fingerprints ({e}). Run `npm run report:doctrine -- --update-fingerprints`.")
    });
    let reports = run_standard_suite(&expected.seeds);
    let actual = fingerprints_from_reports(&reports, &expected.seeds);
    if actual != expected {
        let mut diffs = Vec::new();
        for (name, exp_hash) in &expected.scenarios {
            let got = actual
                .scenarios
                .get(name)
                .map(String::as_str)
                .unwrap_or("<missing>");
            if got != exp_hash {
                diffs.push(format!("  {name}: expected {exp_hash}, got {got}"));
            }
        }
        for name in actual.scenarios.keys() {
            if !expected.scenarios.contains_key(name) {
                diffs.push(format!("  {name}: new scenario not in fingerprints.json"));
            }
        }
        panic!(
            "Standard doctrine outcome fingerprints changed!\n{}\n\
             If this behaviour change is intentional, run `npm run report:doctrine -- --update-fingerprints`, \
             inspect the before/after report, and document the change in your handoff (requires human approval).",
            diffs.join("\n")
        );
    }
}
