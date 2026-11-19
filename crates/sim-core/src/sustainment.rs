use crate::{
    definitions::{STEP, TUNING},
    unit::Unit,
};
pub(crate) fn advance(unit: &mut Unit) {
    if unit.hp > 0.0 {
        unit.nutrition = (unit.nutrition - TUNING.nutrition_drain_per_second * STEP).max(0.0);
        unit.hydration = (unit.hydration - TUNING.hydration_drain_per_second * STEP).max(0.0);
    }
}
pub(crate) fn factor(nutrition: f64, hydration: f64) -> f64 {
    let threshold = TUNING.shortage_threshold;
    1.0 - TUNING.nutrition_shortage_penalty * ((threshold - nutrition).max(0.0) / threshold)
        - TUNING.hydration_shortage_penalty * ((threshold - hydration).max(0.0) / threshold)
}
pub(crate) fn shortage(value: f64) -> bool {
    value < TUNING.shortage_threshold
}
