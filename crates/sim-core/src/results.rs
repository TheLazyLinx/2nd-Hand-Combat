use crate::{perception::Unit, types::*};
pub(crate) fn evaluate(scenario: &Scenario, units: &[Unit], tick: u32) -> Option<BattleResult> {
    let survivors: Vec<_> = scenario
        .factions
        .iter()
        .map(|f| {
            units
                .iter()
                .filter(|u| u.placement.faction == *f && u.hp > 0.0)
                .count()
        })
        .collect();
    let alive = survivors.iter().filter(|n| **n > 0).count();
    let reason = if alive == 0 {
        Some("simultaneous_elimination")
    } else if alive == 1 {
        Some("elimination")
    } else if tick as f64 * crate::definitions::STEP >= scenario.time_limit {
        Some("timeout")
    } else {
        None
    };
    if let Some(reason) = reason {
        let factions = scenario
            .factions
            .iter()
            .zip(survivors.iter())
            .map(|(f, n)| FactionResult {
                faction: *f,
                survivors: *n as u32,
                casualties: units.iter().filter(|u| u.placement.faction == *f).count() as u32
                    - *n as u32,
            })
            .collect();
        let winner = if alive == 1 {
            scenario
                .factions
                .iter()
                .zip(survivors.iter())
                .find(|(_, n)| **n > 0)
                .map(|(f, _)| *f)
        } else {
            None
        };
        return Some(BattleResult {
            winner,
            reason: reason.into(),
            elapsed: tick as f64 * crate::definitions::STEP,
            factions,
        });
    }
    None
}
