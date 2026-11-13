use crate::{map, types::*, unit::Unit};

pub(crate) fn observe_all(units: &[Unit], map: &ArenaMap, index: usize, tick: u32) -> Vec<usize> {
    let me = &units[index];
    let mut seen: Vec<_> = units
        .iter()
        .enumerate()
        .filter(|(_, u)| {
            u.hp > 0.0
                && u.placement.faction != me.placement.faction
                && map::detectable(map, me.position, u.position, tick < u.revealed_until)
        })
        .map(|(i, _)| i)
        .collect();
    seen.sort_by(|a, b| {
        me.position
            .distance(units[*a].position)
            .total_cmp(&me.position.distance(units[*b].position))
            .then(units[*a].placement.id.cmp(&units[*b].placement.id))
    });
    seen
}
#[cfg(test)]
pub(crate) fn observe(units: &[Unit], map: &ArenaMap, index: usize, tick: u32) -> Option<usize> {
    observe_all(units, map, index, tick).first().copied()
}
