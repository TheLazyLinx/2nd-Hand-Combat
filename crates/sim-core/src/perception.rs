use crate::{map, types::*};

#[derive(Clone)]
pub(crate) struct Memory {
    pub position: Point,
    pub tick: u32,
    pub investigated: bool,
}
#[derive(Clone)]
pub(crate) struct ThreatMemory {
    pub id: u32,
    pub position: Point,
    pub tick: u32,
}
#[derive(Clone)]
pub(crate) struct Unit {
    pub placement: UnitPlacement,
    pub position: Point,
    pub hp: f64,
    pub behavior: Behavior,
    pub target: Option<u32>,
    pub shots: u32,
    pub cooldown: u32,
    pub memory: Option<Memory>,
    pub goal: Option<Point>,
    pub patrol_index: usize,
    pub fallback_goal: Option<Point>,
    pub fallback_threat: Option<Point>,
    pub needs_decision: bool,
    pub route: crate::navigation::Route,
    pub revealed_until: u32,
    pub movement_multiplier: f64,
    pub known_threats: Vec<Point>,
    pub threat_memories: Vec<ThreatMemory>,
}
impl Unit {
    pub(crate) fn is_low_hp(&self) -> bool {
        self.hp <= crate::definitions::MAX_HP * self.placement.doctrine.fallback_hp / 100.0
    }

    pub fn new(p: UnitPlacement) -> Self {
        Self {
            position: p.position,
            placement: p,
            hp: crate::definitions::MAX_HP,
            behavior: Behavior::Searching,
            target: None,
            shots: 0,
            cooldown: 0,
            memory: None,
            goal: None,
            patrol_index: 0,
            fallback_goal: None,
            fallback_threat: None,
            needs_decision: true,
            route: crate::navigation::Route::default(),
            revealed_until: 0,
            movement_multiplier: 1.0,
            known_threats: Vec::new(),
            threat_memories: Vec::new(),
        }
    }
}

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
