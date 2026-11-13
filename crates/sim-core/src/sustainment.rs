use crate::{definitions::STEP, unit::Unit};
pub(crate) fn advance(unit: &mut Unit) {
    if unit.hp > 0.0 {
        unit.nutrition = (unit.nutrition - 0.5 * STEP).max(0.0);
        unit.hydration = (unit.hydration - STEP).max(0.0);
    }
}
pub(crate) fn factor(nutrition: f64, hydration: f64) -> f64 {
    1.0 - 0.15 * ((25.0 - nutrition).max(0.0) / 25.0) - 0.25 * ((25.0 - hydration).max(0.0) / 25.0)
}
pub(crate) fn shortage(value: f64) -> bool {
    value < 25.0
}
