use crate::types::*;
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
    pub inventory: crate::inventory::Inventory,
    pub nutrition: f64,
    pub hydration: f64,
    pub last_contact: u32,
    pub last_medical: Option<u32>,
}
impl Unit {
    pub(crate) fn is_low_hp(&self) -> bool {
        self.hp <= crate::definitions::MAX_HP * self.placement.doctrine.fallback_hp / 100.0
    }

    pub(crate) fn needs_factor(&self) -> f64 {
        crate::sustainment::factor(self.nutrition, self.hydration)
    }
    pub(crate) fn withdrawing(&self) -> bool {
        self.is_low_hp()
            || (self.inventory.count("rifle-ammo") == 0
                && self.placement.doctrine.out_of_ammo_behavior == OutOfAmmoBehavior::Withdraw)
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
            movement_multiplier: 0.0,
            known_threats: Vec::new(),
            threat_memories: Vec::new(),
            inventory: crate::inventory::Inventory::new(),
            nutrition: 100.0,
            hydration: 100.0,
            last_contact: 0,
            last_medical: None,
        }
    }
}
