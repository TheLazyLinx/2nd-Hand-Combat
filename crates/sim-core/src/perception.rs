use crate::{map, types::*};

#[derive(Clone)]
pub(crate) struct Memory {
    pub position: Point,
    pub tick: u32,
    pub investigated: bool,
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
        }
    }
}

pub(crate) fn observe(units: &[Unit], map: &ArenaMap, index: usize) -> Option<usize> {
    let me = &units[index];
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| {
            u.hp > 0.0
                && u.placement.faction != me.placement.faction
                && me.position.distance(u.position) <= 14.0
                && map::clear(map, me.position, u.position)
        })
        .min_by(|(_, a), (_, b)| {
            me.position
                .distance(a.position)
                .total_cmp(&me.position.distance(b.position))
                .then(a.placement.id.cmp(&b.placement.id))
        })
        .map(|(i, _)| i)
}
