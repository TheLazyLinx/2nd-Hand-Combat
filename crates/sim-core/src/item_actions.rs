//! Per-tick soldier action selection via typed candidate proposals filtered by doctrine.
//!
//! Each item/weapon category proposes a typed `ActionProposal` with an explicit `ActionPriority`.
//! `select` picks the highest-priority eligible proposal (at most one action per soldier per tick)
//! without mutating unit or world state, using typed `ItemId` / `WeaponSpec` lookups.

use crate::{
    consumables,
    definitions::{weapon_spec, WeaponSpec, STEP},
    inventory::ItemId,
    map,
    terrain::Terrain,
    types::*,
    unit::Unit,
};

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Medical(ItemId),
    Water(ItemId),
    Ration(ItemId),
    Grenade {
        item: ItemId,
        target_idx: usize,
    },
    Fire {
        entry_id: u32,
        target_idx: usize,
        exposure: f64,
        moving_multiplier: f64,
        weapon: &'static WeaponSpec,
    },
}

#[inline]
fn is_moving_behavior(behavior: &Behavior) -> bool {
    matches!(
        behavior,
        Behavior::Advancing | Behavior::FallingBack | Behavior::Investigating
    )
}

#[inline]
fn can_attack(unit: &Unit) -> bool {
    match unit.combat.behavior {
        Behavior::Engaging | Behavior::Defending => true,
        Behavior::Advancing | Behavior::FallingBack | Behavior::Investigating => {
            unit.placement.doctrine.fire_while_moving
        }
        Behavior::Searching | Behavior::Dead => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ActionPriority {
    Fire = 1,
    Grenade = 2,
    Ration = 3,
    Water = 4,
    Medical = 5,
}

#[derive(Clone, Copy)]
struct ActionProposal {
    priority: ActionPriority,
    action: Action,
}

struct ActionContext<'a> {
    unit: &'a Unit,
    units: &'a [Unit],
    terrain: Terrain<'a>,
    tick: u32,
    observed: &'a [usize],
}

fn propose_medical(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    let med = &u.placement.doctrine.medical;
    let hp_cutoff = u.vitals.max_hp * med.hp_threshold / 100.0;
    if u.vitals.hp > hp_cutoff || u.vitals.hp >= u.vitals.max_hp {
        return None;
    }
    let item = u.inventory.find_usable_category(ItemCategory::Medical)?;
    let medical_due = u
        .combat
        .last_medical
        .is_none_or(|last| ctx.tick.saturating_sub(last) as f64 * STEP >= med.cooldown_seconds);
    if !medical_due {
        return None;
    }
    let allowed = match med.policy {
        MedicalPolicy::Never => false,
        MedicalPolicy::OutOfContact => {
            ctx.observed.is_empty()
                && ctx.tick.saturating_sub(u.perception.last_contact) as f64 * STEP
                    >= med.safe_seconds
        }
        MedicalPolicy::LowHealth => true,
    };
    allowed.then_some(ActionProposal {
        priority: ActionPriority::Medical,
        action: Action::Medical(item),
    })
}

fn propose_water(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    if u.sustainment.hydration > consumables::NEED_USE_THRESHOLD {
        return None;
    }
    let item = u.inventory.find_usable_category(ItemCategory::Water)?;
    Some(ActionProposal {
        priority: ActionPriority::Water,
        action: Action::Water(item),
    })
}

fn propose_ration(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    if u.sustainment.nutrition > consumables::NEED_USE_THRESHOLD {
        return None;
    }
    let item = u.inventory.find_usable_category(ItemCategory::Ration)?;
    Some(ActionProposal {
        priority: ActionPriority::Ration,
        action: Action::Ration(item),
    })
}

fn propose_grenade(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    if !can_attack(u) || !u.combat.attack_ready() {
        return None;
    }
    let grenade = &u.placement.doctrine.grenades;
    if grenade.policy == GrenadePolicy::Never
        || (grenade.policy == GrenadePolicy::EmptyRifleOnly && u.has_weapon_ammo())
    {
        return None;
    }
    let item = u.inventory.find_usable_category(ItemCategory::Grenade)?;
    let safe_radius = consumables::BLAST_RADIUS + grenade.safety_margin;
    for &j in ctx.observed {
        let target = ctx.units[j].nav.position;
        if u.nav.position.distance(target) > grenade.max_distance
            || !map::clear_t(ctx.terrain, u.nav.position, target)
            || ctx.units.iter().any(|ally| {
                ally.vitals.hp > 0.0
                    && !is_hostile(ally.placement.faction, u.placement.faction)
                    && ally.nav.position.distance(target) <= safe_radius
            })
        {
            continue;
        }
        let affected = ctx
            .observed
            .iter()
            .filter(|&&k| {
                ctx.units[k].nav.position.distance(target) <= consumables::BLAST_RADIUS
                    && map::visible_t(ctx.terrain, target, ctx.units[k].nav.position, 0.1, 1.0)
            })
            .count();
        if grenade.policy != GrenadePolicy::Clusters || affected >= grenade.min_enemies as usize {
            return Some(ActionProposal {
                priority: ActionPriority::Grenade,
                action: Action::Grenade {
                    item,
                    target_idx: j,
                },
            });
        }
    }
    None
}

fn propose_fire(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    if !can_attack(u) {
        return None;
    }
    let (entry_id, weapon_id) = u.inventory.held_weapon_entry()?;
    if !u.combat.weapon_ready(entry_id) {
        return None;
    }
    let weapon = weapon_spec(weapon_id);
    if !u.inventory.can_use_item(weapon.ammo_item) {
        return None;
    }
    let j = ctx.observed.first().copied()?;
    if u.nav.position.distance(ctx.units[j].nav.position) > weapon.range {
        return None;
    }
    let exposure = map::exposure_t(
        ctx.terrain,
        u.nav.position,
        ctx.units[j].nav.position,
        ctx.tick < ctx.units[j].combat.revealed_until,
    );
    let moving_multiplier = if is_moving_behavior(&u.combat.behavior) {
        weapon.moving_fire_multiplier * u.def.moving_fire_multiplier
    } else {
        1.0
    };
    (exposure > 0.0).then_some(ActionProposal {
        priority: ActionPriority::Fire,
        action: Action::Fire {
            entry_id,
            target_idx: j,
            exposure,
            moving_multiplier,
            weapon,
        },
    })
}

type Proposer = fn(&ActionContext<'_>) -> Option<ActionProposal>;

const PROPOSERS: [Proposer; 5] = [
    propose_medical,
    propose_water,
    propose_ration,
    propose_grenade,
    propose_fire,
];

fn select_for_unit(ctx: &ActionContext<'_>) -> Option<Action> {
    if ctx.unit.vitals.hp <= 0.0 {
        return None;
    }
    // Proposers are ordered from highest to lowest priority; select the highest-priority proposal.
    let mut best: Option<ActionProposal> = None;
    for propose in PROPOSERS {
        if let Some(proposal) = propose(ctx) {
            if best.is_none_or(|current| proposal.priority > current.priority) {
                best = Some(proposal);
                break;
            }
        }
    }
    best.map(|p| p.action)
}

pub(crate) fn select(
    units: &[Unit],
    terrain: Terrain<'_>,
    tick: u32,
    observations: &[Vec<usize>],
) -> Vec<Option<Action>> {
    units
        .iter()
        .enumerate()
        .map(|(i, unit)| {
            let ctx = ActionContext {
                unit,
                units,
                terrain,
                tick,
                observed: &observations[i],
            };
            select_for_unit(&ctx)
        })
        .collect()
}
