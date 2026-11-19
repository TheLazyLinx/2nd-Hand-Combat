//! CLI entry point for `sim-bench` (`bench` and `report` subcommands).

use sim_bench::{
    bench::{format_bench_table, run_benchmarks, BenchConfig},
    doctrine_suite::{
        fingerprints_from_reports, format_markdown_report, run_standard_suite, save_fingerprints,
        DEFAULT_SEEDS,
    },
};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map(String::as_str).unwrap_or("bench");

    match mode {
        "bench" => {
            let quick = args.iter().any(|a| a == "--quick");
            let show_sweep_plan = args.iter().any(|a| a == "--sweep");
            let max_secs_per_scenario = args
                .windows(2)
                .find(|w| w[0] == "--max-secs")
                .and_then(|w| w[1].parse::<u64>().ok())
                .unwrap_or(if quick { 15 } else { 40 });
            let config = BenchConfig {
                quick,
                max_secs_per_scenario,
                show_sweep_plan,
            };
            let results = run_benchmarks(&config);
            print!("{}", format_bench_table(&results, show_sweep_plan));
        }
        "report" => {
            let seed_count = args
                .windows(2)
                .find(|w| w[0] == "--seeds")
                .and_then(|w| w[1].parse::<u32>().ok());
            let seeds: Vec<u32> = match seed_count {
                Some(n) if n > 0 => (1..=n).collect(),
                _ => DEFAULT_SEEDS.to_vec(),
            };
            let update_fp = args.iter().any(|a| a == "--update-fingerprints");
            let reports = run_standard_suite(&seeds);
            print!("{}", format_markdown_report(&reports));
            if update_fp {
                let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fingerprints.json");
                let file = fingerprints_from_reports(&reports, &seeds);
                save_fingerprints(&path, &file).expect("write fingerprints.json");
                eprintln!("Updated {}", path.display());
            }
        }
        other => {
            eprintln!(
                "Unknown subcommand `{other}`. Usage:\n  sim-bench bench [--quick] [--max-secs N] [--sweep]\n  sim-bench report [--seeds N] [--update-fingerprints]"
            );
            std::process::exit(2);
        }
    }
}
