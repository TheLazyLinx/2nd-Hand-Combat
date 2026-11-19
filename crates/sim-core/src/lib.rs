mod battle;
mod combat;
mod compact_snapshot;
mod consumables;
mod decisions;
mod definitions;
mod inventory;
mod item_actions;
mod map;
mod movement;
mod navigation;
#[cfg(test)]
mod oracles;
mod perception;
mod results;
mod spatial;
mod sustainment;
mod terrain;
mod types;
mod unit;
mod validation;

pub use battle::{Battle, StepTimings};
pub use compact_snapshot::{
    COMPACT_FLAG_HAS_PROTECTION, COMPACT_FLAG_HYDRATION_SHORTAGE, COMPACT_FLAG_NUTRITION_SHORTAGE,
    COMPACT_FLAG_REVEALED, COMPACT_UNIT_BYTES,
};
pub use definitions::{catalog, default_factions};
pub use types::*;
pub use validation::{
    analyze_scenario, analyze_scenario_with_rules, analyze_skirmish_scenario, validate_scenario,
    validate_scenario_with_rules, validate_skirmish_scenario, ModeRules, ENGINE_MAX_FACTIONS,
    ENGINE_MAX_UNITS, SKIRMISH_MAX_PER_FACTION,
};
