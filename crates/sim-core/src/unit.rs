//! Runtime unit state decomposed into cohesive components (`Vitals`, `NavigationState`,
//! `PerceptionMemory`, `CombatState`, `SustainmentState`, `Inventory`) plus per-slot `WeaponState`.
//!
//! Invariant: unit definitions and starting loadouts are resolved once at construction into
//! static `&'static UnitSpec`, compact `WeaponId` slots, and typed `Inventory` entries.

use crate::{
    definitions::{unit_spec, UnitSpec, WeaponId},
    inventory::Inventory,
    navigation::Route,
    types::*,
};

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

#[derive(Clone, Copy, Debug)]
pub(crate) struct Vitals {
    pub hp: f64,
    pub max_hp: f64,
}

#[derive(Clone)]
pub(crate) struct NavigationState {
    pub position: Point,
    pub goal: Option<Point>,
    pub patrol_index: usize,
    pub fallback_goal: Option<Point>,
    pub fallback_threat: Option<Point>,
    pub route: Route,
    pub movement_multiplier: f64,
}

#[derive(Clone, Default)]
pub(crate) struct PerceptionMemory {
    pub target: Option<u32>,
    pub memory: Option<Memory>,
    pub known_threats: Vec<Point>,
    pub threat_memories: Vec<ThreatMemory>,
    pub last_contact: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WeaponState {
    pub entry_id: u32,
    pub weapon: WeaponId,
    pub cooldown: u32,
}

#[derive(Clone)]
pub(crate) struct CombatState {
    pub behavior: Behavior,
    pub shots: u32,
    pub weapons: Vec<WeaponState>,
    pub action_cooldown: u32,
    pub revealed_until: u32,
    pub last_medical: Option<u32>,
}

impl CombatState {
    pub(crate) fn from_inventory(inventory: &Inventory) -> Self {
        let weapons = inventory
            .weapon_entries()
            .into_iter()
            .map(|(entry_id, weapon)| WeaponState {
                entry_id,
                weapon,
                cooldown: 0,
            })
            .collect();
        Self {
            behavior: Behavior::Searching,
            shots: 0,
            weapons,
            action_cooldown: 0,
            revealed_until: 0,
            last_medical: None,
        }
    }

    #[inline]
    pub(crate) fn attack_ready(&self) -> bool {
        self.action_cooldown == 0
    }

    pub(crate) fn weapon_ready(&self, entry_id: u32) -> bool {
        self.action_cooldown == 0
            && self
                .weapons
                .iter()
                .find(|w| w.entry_id == entry_id)
                .is_none_or(|w| w.cooldown == 0)
    }

    pub(crate) fn set_attack_cooldown(&mut self, ticks: u32) {
        self.action_cooldown = self.action_cooldown.max(ticks);
    }

    pub(crate) fn set_weapon_cooldown(&mut self, entry_id: u32, ticks: u32) {
        self.action_cooldown = self.action_cooldown.max(ticks);
        if let Some(state) = self.weapons.iter_mut().find(|w| w.entry_id == entry_id) {
            state.cooldown = state.cooldown.max(ticks);
        }
    }

    pub(crate) fn tick_cooldowns(&mut self) {
        if self.action_cooldown > 0 {
            self.action_cooldown -= 1;
        }
        for state in &mut self.weapons {
            if state.cooldown > 0 {
                state.cooldown -= 1;
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SustainmentState {
    pub nutrition: f64,
    pub hydration: f64,
}

#[derive(Clone)]
pub(crate) struct Unit {
    pub placement: UnitPlacement,
    pub def: &'static UnitSpec,
    pub vitals: Vitals,
    pub nav: NavigationState,
    pub perception: PerceptionMemory,
    pub combat: CombatState,
    pub sustainment: SustainmentState,
    pub inventory: Inventory,
    pub needs_decision: bool,
}

impl Unit {
    #[inline]
    pub(crate) fn is_low_hp(&self) -> bool {
        self.vitals.hp <= self.vitals.max_hp * self.placement.doctrine.fallback_hp / 100.0
    }

    #[inline]
    pub(crate) fn needs_factor(&self) -> f64 {
        crate::sustainment::factor(self.sustainment.nutrition, self.sustainment.hydration)
    }

    /// True when the unit's currently equipped weapon has zero ammunition remaining.
    pub(crate) fn is_out_of_ammo(&self) -> bool {
        self.inventory
            .held_weapon_spec()
            .is_some_and(|weapon| self.inventory.count_item(weapon.ammo_item) == 0)
    }

    /// True when the unit's currently equipped weapon still has ammunition available.
    pub(crate) fn has_weapon_ammo(&self) -> bool {
        self.inventory
            .held_weapon_spec()
            .is_some_and(|weapon| self.inventory.count_item(weapon.ammo_item) > 0)
    }

    pub(crate) fn withdrawing(&self) -> bool {
        self.is_low_hp()
            || (self.is_out_of_ammo()
                && self.placement.doctrine.out_of_ammo_behavior == OutOfAmmoBehavior::Withdraw)
    }

    pub(crate) fn from_placement(
        placement: UnitPlacement,
        loadouts: &[NamedLoadout],
    ) -> Result<Self, String> {
        let def = unit_spec(&placement.definition_id)
            .ok_or_else(|| format!("Unknown unit definition `{}`.", placement.definition_id))?;
        let inventory = match &placement.loadout {
            None => Inventory::from_static_loadout(
                def.default_loadout,
                def.max_weight_grams,
                def.max_stowed_volume_ml,
            )
            .map_err(|e| format!("Invalid default loadout: {e:?}"))?,
            Some(UnitLoadout::Named { name }) => {
                let named = loadouts
                    .iter()
                    .find(|l| l.name == *name)
                    .ok_or_else(|| format!("Unknown scenario loadout `{name}`."))?;
                Inventory::from_loadout(
                    &named.entries,
                    def.max_weight_grams,
                    def.max_stowed_volume_ml,
                )
                .map_err(|e| format!("Invalid scenario loadout `{name}`: {e:?}"))?
            }
            Some(UnitLoadout::Inline { entries }) => {
                Inventory::from_loadout(entries, def.max_weight_grams, def.max_stowed_volume_ml)
                    .map_err(|e| format!("Invalid inline loadout: {e:?}"))?
            }
        };
        let combat = CombatState::from_inventory(&inventory);
        Ok(Self {
            vitals: Vitals {
                hp: def.max_hp,
                max_hp: def.max_hp,
            },
            nav: NavigationState {
                position: placement.position,
                goal: None,
                patrol_index: 0,
                fallback_goal: None,
                fallback_threat: None,
                route: Route::default(),
                movement_multiplier: 0.0,
            },
            perception: PerceptionMemory::default(),
            combat,
            sustainment: SustainmentState {
                nutrition: 100.0,
                hydration: 100.0,
            },
            inventory,
            def,
            placement,
            needs_decision: true,
        })
    }

    #[cfg(test)]
    pub(crate) fn new(placement: UnitPlacement) -> Self {
        Self::from_placement(placement, &[]).expect("valid test unit placement")
    }
}
