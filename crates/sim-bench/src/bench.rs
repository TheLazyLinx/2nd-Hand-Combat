//! Native simulation benchmarks with mean/p99/worst step latency and per-stage breakdown.

use sim_core::*;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct BenchConfig {
    pub quick: bool,
    pub max_secs_per_scenario: u64,
    pub show_sweep_plan: bool,
}

#[derive(Clone, Debug)]
pub struct BenchResult {
    pub name: String,
    pub steps: usize,
    pub mean_ms: f64,
    pub p99_ms: f64,
    pub worst_ms: f64,
    pub over_50ms: usize,
    pub result_reason: String,
    pub sustainment_ms: f64,
    pub perception_ms: f64,
    pub decisions_ms: f64,
    pub movement_ms: f64,
    pub post_perception_ms: f64,
    pub item_actions_ms: f64,
    pub combat_ms: f64,
    pub snapshot_json_ms: f64,
}

fn spawn_sides(per_side: u32, map: &ArenaMap, route_pref: RoutePreference) -> Vec<UnitPlacement> {
    let mut out = Vec::new();
    let mut id = 1;
    let w = map.width as f64;
    let h = map.height as f64;
    let mut doctrine = catalog().default_doctrine;
    doctrine.position_preference = PositionPreference::Cover;
    doctrine.route_preference = route_pref;

    for faction in 0..2u32 {
        let mut placed = 0;
        let mut y = 1.5;
        let step_y = ((h - 2.0) / per_side as f64).max(1.0);
        while placed < per_side && y < h - 0.5 {
            let x = if faction == 0 { 2.5 } else { w - 2.5 };
            let blocked = map.obstacles.iter().any(|o| {
                x >= o.x as f64 - 0.5
                    && x <= (o.x + o.width) as f64 + 0.5
                    && y >= o.y as f64 - 0.5
                    && y <= (o.y + o.height) as f64 + 0.5
            });
            if !blocked {
                out.push(UnitPlacement {
                    id,
                    faction,
                    definition_id: "infantry".into(),
                    position: Point { x, y },
                    doctrine: doctrine.clone(),
                });
                id += 1;
                placed += 1;
            }
            y += step_y;
        }
    }
    out
}

fn large_map(obstacles_count: usize) -> ArenaMap {
    let mut m = ArenaMap::flat(128, 128);
    let mut rng = 12345u64;
    let mut placed = HashSet::new();
    while placed.len() < obstacles_count {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let x = 6 + (rng % 116) as u32;
        let y = 1 + ((rng >> 20) % 126) as u32;
        if placed.insert((x, y)) {
            m.obstacles.push(Obstacle {
                x,
                y,
                width: 1,
                height: 1,
                height_metres: if rng.is_multiple_of(3) { 2.4 } else { 0.9 },
            });
        }
    }
    m.patrol = vec![Point { x: 40.5, y: 64.5 }, Point { x: 88.5, y: 64.5 }];
    m.obstacles
        .retain(|o| !((o.x == 40 || o.x == 88) && o.y == 64));
    m
}

fn run_one(
    name: &str,
    map: ArenaMap,
    route_pref: RoutePreference,
    max_steps: usize,
    max_secs: u64,
) -> BenchResult {
    let scenario = Scenario {
        units: spawn_sides(8, &map, route_pref),
        map,
        seed: 1,
        factions: vec![0, 1],
        time_limit: 180.0,
    };
    let mut battle = Battle::new(scenario).expect("valid benchmark scenario");
    let mut step_ms = Vec::with_capacity(max_steps.min(3600));
    let mut sustainment_ns = 0u64;
    let mut perception_ns = 0u64;
    let mut decisions_ns = 0u64;
    let mut movement_ns = 0u64;
    let mut post_perception_ns = 0u64;
    let mut item_actions_ns = 0u64;
    let mut combat_ns = 0u64;
    let mut snapshot_json_ns = 0u64;
    let mut snapshot_samples = 0usize;

    let wall_start = Instant::now();
    while !battle.is_finished()
        && step_ms.len() < max_steps
        && wall_start.elapsed().as_secs() < max_secs
    {
        let timings = battle.step_profiled();
        sustainment_ns += timings.sustainment_ns;
        perception_ns += timings.perception_ns;
        decisions_ns += timings.decisions_ns;
        movement_ns += timings.movement_ns;
        post_perception_ns += timings.post_perception_ns;
        item_actions_ns += timings.item_actions_ns;
        combat_ns += timings.combat_ns;
        step_ms.push(timings.total_ns() as f64 / 1e6);

        // Mirror the Worker's 10 Hz snapshot publication rate (every second 20 Hz step).
        if step_ms.len() % 2 == 0 {
            let t0 = Instant::now();
            let snap = battle.snapshot();
            let json = serde_json::to_string(&snap).expect("serialize snapshot");
            std::hint::black_box(json.len());
            snapshot_json_ns += t0.elapsed().as_nanos() as u64;
            snapshot_samples += 1;
        }
    }

    let steps = step_ms.len().max(1);
    let mean_ms = step_ms.iter().sum::<f64>() / steps as f64;
    let over_50ms = step_ms.iter().filter(|&&t| t > 50.0).count();
    step_ms.sort_by(|a, b| a.total_cmp(b));
    let p99_idx = ((step_ms.len().saturating_sub(1)) as f64 * 0.99).ceil() as usize;
    let p99_ms = step_ms
        .get(p99_idx)
        .copied()
        .or_else(|| step_ms.last().copied())
        .unwrap_or(0.0);
    let worst_ms = step_ms.last().copied().unwrap_or(0.0);

    let reason = battle
        .snapshot()
        .result
        .map(|r| r.reason)
        .unwrap_or_else(|| "unfinished".into());
    let to_mean_ms = |ns: u64, count: usize| ns as f64 / 1e6 / count.max(1) as f64;

    BenchResult {
        name: name.into(),
        steps: step_ms.len(),
        mean_ms,
        p99_ms,
        worst_ms,
        over_50ms,
        result_reason: reason,
        sustainment_ms: to_mean_ms(sustainment_ns, steps),
        perception_ms: to_mean_ms(perception_ns, steps),
        decisions_ms: to_mean_ms(decisions_ns, steps),
        movement_ms: to_mean_ms(movement_ns, steps),
        post_perception_ms: to_mean_ms(post_perception_ns, steps),
        item_actions_ms: to_mean_ms(item_actions_ns, steps),
        combat_ms: to_mean_ms(combat_ns, steps),
        snapshot_json_ms: to_mean_ms(snapshot_json_ns, snapshot_samples),
    }
}

pub fn run_benchmarks(config: &BenchConfig) -> Vec<BenchResult> {
    let max_steps = if config.quick { 300 } else { 3600 };
    let mut results = Vec::new();

    for preset in catalog().maps {
        for pref in [RoutePreference::Fastest, RoutePreference::Protected] {
            let label = format!("{}-{pref:?}", preset.id).to_lowercase();
            results.push(run_one(
                &label,
                preset.map.clone(),
                pref,
                max_steps,
                config.max_secs_per_scenario,
            ));
        }
    }

    let open128 = large_map(0);
    for pref in [RoutePreference::Fastest, RoutePreference::Protected] {
        let label = format!("128x128-open-{pref:?}").to_lowercase();
        results.push(run_one(
            &label,
            open128.clone(),
            pref,
            if config.quick { 80 } else { max_steps },
            config.max_secs_per_scenario,
        ));
    }

    if !config.quick {
        let dense128 = large_map(1500);
        for pref in [RoutePreference::Fastest, RoutePreference::Protected] {
            let label = format!("128x128-obs1500-{pref:?}").to_lowercase();
            results.push(run_one(
                &label,
                dense128.clone(),
                pref,
                max_steps,
                config.max_secs_per_scenario,
            ));
        }
    }

    results
}

pub fn format_bench_table(results: &[BenchResult], show_sweep_note: bool) -> String {
    let mut out = String::new();
    out.push_str("| Scenario | Steps | Mean (ms) | p99 (ms) | Worst (ms) | >50ms | Result |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | --- |\n");
    for r in results {
        out.push_str(&format!(
            "| {} | {} | {:.3} | {:.2} | {:.2} | {} | {} |\n",
            r.name, r.steps, r.mean_ms, r.p99_ms, r.worst_ms, r.over_50ms, r.result_reason
        ));
    }
    out.push_str(
        "\n### Mean stage breakdown (ms / step; snapshot JSON is per published snapshot)\n\n",
    );
    out.push_str(
        "| Scenario | Sust | Percept | Decide | Move | PostPerc | ItemAct | Combat | SnapJSON |\n",
    );
    out.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for r in results {
        out.push_str(&format!(
            "| {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} |\n",
            r.name,
            r.sustainment_ms,
            r.perception_ms,
            r.decisions_ms,
            r.movement_ms,
            r.post_perception_ms,
            r.item_actions_ms,
            r.combat_ms,
            r.snapshot_json_ms
        ));
    }
    if show_sweep_note {
        out.push_str(&format!(
            "\nNote: 100 / 1,000 / 5,000 entity sweeps (SCALE.md) are currently gated by core validation `max_per_faction = {}` and will be enabled in Phase 3.\n",
            catalog().max_per_faction
        ));
    }
    out
}
