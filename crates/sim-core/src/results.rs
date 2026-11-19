//! Victory and battle termination evaluation across arbitrary faction counts.
//!
//! Invariant: a battle ends when zero factions survive (`simultaneous_elimination`), when no two
//! surviving factions remain hostile to each other (`elimination`), or when `time_limit` is met.

use crate::{types::*, unit::Unit};

pub(crate) fn evaluate(scenario: &Scenario, units: &[Unit], tick: u32) -> Option<BattleResult> {
    let survivors: Vec<usize> = scenario
        .factions
        .iter()
        .map(|f| {
            units
                .iter()
                .filter(|u| u.placement.faction == f.id && u.vitals.hp > 0.0)
                .count()
        })
        .collect();
    let surviving_factions: Vec<u32> = scenario
        .factions
        .iter()
        .zip(survivors.iter())
        .filter_map(|(f, &n)| (n > 0).then_some(f.id))
        .collect();
    let hostile_pair_alive = surviving_factions.iter().enumerate().any(|(i, &a)| {
        surviving_factions[i + 1..]
            .iter()
            .any(|&b| scenario.is_hostile(a, b))
    });
    let elapsed = tick as f64 * crate::definitions::STEP;
    let reason = if surviving_factions.is_empty() {
        Some("simultaneous_elimination")
    } else if !hostile_pair_alive {
        Some("elimination")
    } else if scenario.time_limit.is_some_and(|limit| elapsed >= limit) {
        Some("timeout")
    } else {
        None
    };
    let reason = reason?;
    let factions = scenario
        .factions
        .iter()
        .zip(survivors.iter())
        .map(|(f, &n)| {
            let total = units.iter().filter(|u| u.placement.faction == f.id).count() as u32;
            FactionResult {
                faction: f.id,
                survivors: n as u32,
                casualties: total - n as u32,
            }
        })
        .collect();
    let winner = if surviving_factions.len() == 1 {
        Some(surviving_factions[0])
    } else {
        None
    };
    Some(BattleResult {
        winner,
        reason: reason.into(),
        elapsed,
        factions,
    })
}
