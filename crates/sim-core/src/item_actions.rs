//! Per-tick soldier action selection via typed candidate proposals filtered by doctrine.
//!
//! Each item/weapon category proposes a typed `ActionProposal` with an explicit `ActionPriority`.
//! `select` picks the highest-priority eligible proposal (at most one action per soldier per tick)
//! without mutating unit or world state.

use crate::{
    consumables,
    definitions::{WeaponSpec, STEP},
    map,
    terrain::Terrain,
    types::*,
    unit::Unit,
};

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Medical,
    Water,
    Ration,
    Grenade(usize),
    Rifle(usize, f64, &'static WeaponSpec),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ActionPriority {
    Rifle = 1,
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
    if u.hp > med.hp_threshold || u.hp >= 100.0 || !u.inventory.can_use("medical-supply") {
        return None;
    }
    let medical_due = u
        .last_medical
        .is_none_or(|last| ctx.tick.saturating_sub(last) as f64 * STEP >= med.cooldown_seconds);
    if !medical_due {
        return None;
    }
    let allowed = match med.policy {
        MedicalPolicy::Never => false,
        MedicalPolicy::OutOfContact => {
            ctx.observed.is_empty()
                && ctx.tick.saturating_sub(u.last_contact) as f64 * STEP >= med.safe_seconds
        }
        MedicalPolicy::LowHealth => true,
    };
    allowed.then_some(ActionProposal {
        priority: ActionPriority::Medical,
        action: Action::Medical,
    })
}

fn propose_water(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    (u.hydration <= consumables::NEED_USE_THRESHOLD && u.inventory.can_use("water")).then_some(
        ActionProposal {
            priority: ActionPriority::Water,
            action: Action::Water,
        },
    )
}

fn propose_ration(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    (u.nutrition <= consumables::NEED_USE_THRESHOLD && u.inventory.can_use("ration")).then_some(
        ActionProposal {
            priority: ActionPriority::Ration,
            action: Action::Ration,
        },
    )
}

fn propose_grenade(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    if u.cooldown > 0 {
        return None;
    }
    let grenade = &u.placement.doctrine.grenades;
    if grenade.policy == GrenadePolicy::Never
        || !u.inventory.can_use("frag-grenade")
        || (grenade.policy == GrenadePolicy::EmptyRifleOnly && u.inventory.count("rifle-ammo") > 0)
    {
        return None;
    }
    let safe_radius = consumables::BLAST_RADIUS + grenade.safety_margin;
    for &j in ctx.observed {
        let target = ctx.units[j].position;
        if u.position.distance(target) > grenade.max_distance
            || !map::clear_t(ctx.terrain, u.position, target)
            || ctx.units.iter().any(|ally| {
                ally.hp > 0.0
                    && ally.placement.faction == u.placement.faction
                    && ally.position.distance(target) <= safe_radius
            })
        {
            continue;
        }
        let affected = ctx
            .observed
            .iter()
            .filter(|&&k| {
                ctx.units[k].position.distance(target) <= consumables::BLAST_RADIUS
                    && map::visible_t(ctx.terrain, target, ctx.units[k].position, 0.1, 1.0)
            })
            .count();
        if grenade.policy != GrenadePolicy::Clusters || affected >= grenade.min_enemies as usize {
            return Some(ActionProposal {
                priority: ActionPriority::Grenade,
                action: Action::Grenade(j),
            });
        }
    }
    None
}

fn propose_rifle(ctx: &ActionContext<'_>) -> Option<ActionProposal> {
    let u = ctx.unit;
    if u.cooldown > 0 {
        return None;
    }
    let weapon = u.inventory.held_weapon_spec()?;
    if !matches!(
        u.behavior,
        Behavior::Engaging | Behavior::FallingBack | Behavior::Defending
    ) || u.inventory.count(weapon.ammo_item_id) == 0
    {
        return None;
    }
    let j = ctx.observed.first().copied()?;
    if u.position.distance(ctx.units[j].position) > weapon.range {
        return None;
    }
    let exposure = map::exposure_t(
        ctx.terrain,
        u.position,
        ctx.units[j].position,
        ctx.tick < ctx.units[j].revealed_until,
    );
    (exposure > 0.0).then_some(ActionProposal {
        priority: ActionPriority::Rifle,
        action: Action::Rifle(j, exposure, weapon),
    })
}

type Proposer = fn(&ActionContext<'_>) -> Option<ActionProposal>;

const PROPOSERS: [Proposer; 5] = [
    propose_medical,
    propose_water,
    propose_ration,
    propose_grenade,
    propose_rifle,
];

fn select_for_unit(ctx: &ActionContext<'_>) -> Option<Action> {
    if ctx.unit.hp <= 0.0 {
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
