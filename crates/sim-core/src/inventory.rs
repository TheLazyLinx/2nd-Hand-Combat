//! Per-unit inventory storage, weight/volume capacity checks, hand slots, and atomic item use.
//!
//! Invariant: hot-path operations resolve items through compact `ItemId` indices rather than
//! string lookups, and failed multi-step transitions roll back without mutating inventory state.

use crate::{
    definitions::{WeaponId, WeaponSpec, WEAPON_RIFLE},
    types::*,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ItemId(pub(crate) usize);

pub(crate) const ITEM_RIFLE: ItemId = ItemId(0);
pub(crate) const ITEM_RIFLE_AMMO: ItemId = ItemId(1);
pub(crate) const ITEM_FRAG_GRENADE: ItemId = ItemId(2);
pub(crate) const ITEM_ENTRENCHING_TOOL: ItemId = ItemId(3);
pub(crate) const ITEM_MEDICAL_SUPPLY: ItemId = ItemId(4);
pub(crate) const ITEM_RATION: ItemId = ItemId(5);
pub(crate) const ITEM_WATER: ItemId = ItemId(6);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ItemCategory {
    Weapon,
    Ammo,
    Grenade,
    Tool,
    Medical,
    Ration,
    Water,
}

impl ItemCategory {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Weapon => "weapon",
            Self::Ammo => "ammo",
            Self::Grenade => "grenade",
            Self::Tool => "tool",
            Self::Medical => "medical",
            Self::Ration => "ration",
            Self::Water => "water",
        }
    }

    pub(crate) const fn consumable(self) -> bool {
        matches!(
            self,
            Self::Ammo | Self::Grenade | Self::Medical | Self::Ration | Self::Water
        )
    }
}

pub(crate) struct ItemSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: ItemCategory,
    pub mass: u32,
    pub volume: u32,
    pub hands: u32,
    pub weapon: Option<WeaponId>,
}

const ITEMS: [ItemSpec; 7] = [
    ItemSpec {
        id: "rifle",
        name: "Rifle",
        category: ItemCategory::Weapon,
        mass: 4000,
        volume: 6000,
        hands: 2,
        weapon: Some(WEAPON_RIFLE),
    },
    ItemSpec {
        id: "rifle-ammo",
        name: "Rifle rounds",
        category: ItemCategory::Ammo,
        mass: 12,
        volume: 15,
        hands: 0,
        weapon: None,
    },
    ItemSpec {
        id: "frag-grenade",
        name: "Frag grenade",
        category: ItemCategory::Grenade,
        mass: 400,
        volume: 400,
        hands: 1,
        weapon: None,
    },
    ItemSpec {
        id: "entrenching-tool",
        name: "Entrenching tool",
        category: ItemCategory::Tool,
        mass: 1000,
        volume: 1000,
        hands: 2,
        weapon: None,
    },
    ItemSpec {
        id: "medical-supply",
        name: "Medical supply",
        category: ItemCategory::Medical,
        mass: 500,
        volume: 600,
        hands: 2,
        weapon: None,
    },
    ItemSpec {
        id: "ration",
        name: "Ration",
        category: ItemCategory::Ration,
        mass: 500,
        volume: 800,
        hands: 1,
        weapon: None,
    },
    ItemSpec {
        id: "water",
        name: "Water",
        category: ItemCategory::Water,
        mass: 500,
        volume: 500,
        hands: 1,
        weapon: None,
    },
];

#[inline]
pub(crate) fn item_spec(id: ItemId) -> &'static ItemSpec {
    &ITEMS[id.0]
}

pub(crate) fn item_id_by_str(id: &str) -> Result<ItemId, InventoryError> {
    ITEMS
        .iter()
        .position(|item| item.id == id)
        .map(ItemId)
        .ok_or(InventoryError::UnknownItem)
}

pub(crate) fn items() -> Vec<ItemDefinition> {
    ITEMS
        .iter()
        .map(|d| ItemDefinition {
            id: d.id.into(),
            name: d.name.into(),
            category: d.category.as_str().into(),
            mass_grams: d.mass,
            volume_ml: d.volume,
            hands_required: d.hands,
            weapon_id: d
                .weapon
                .map(|w| crate::definitions::weapon_spec(w).id.to_owned()),
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InventoryError {
    UnknownItem,
    NotConsumable,
    Unavailable,
    Capacity,
    InvalidHands,
    InvalidEntry,
    DuplicateItem,
    InvalidQuantity,
    Overflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    id: u32,
    item: ItemId,
    quantity: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UseRecord {
    item: ItemId,
    count: u32,
}

#[derive(Clone)]
pub(crate) struct Inventory {
    entries: Vec<Entry>,
    left: Option<u32>,
    right: Option<u32>,
    uses: Vec<UseRecord>,
    max_weight: u32,
    max_volume: u32,
    next_id: Option<u32>,
    weight_grams: u32,
    stowed_volume_ml: u32,
    load_factor: f64,
}

#[inline]
fn compute_load_factor(weight: u32, volume: u32, max_weight: u32, max_volume: u32) -> f64 {
    let weight_fraction = if max_weight == 0 {
        0.0
    } else {
        weight as f64 / max_weight as f64
    };
    let volume_fraction = if max_volume == 0 {
        0.0
    } else {
        volume as f64 / max_volume as f64
    };
    1.0 - 0.25 * weight_fraction - 0.15 * volume_fraction
}

impl Inventory {
    pub(crate) fn from_static_loadout(
        loadout: &[(ItemId, u32)],
        max_weight: u32,
        max_volume: u32,
    ) -> Result<Self, InventoryError> {
        let mut entries = Vec::with_capacity(loadout.len());
        for (i, &(item, quantity)) in loadout.iter().enumerate() {
            if quantity == 0 {
                return Err(InventoryError::InvalidQuantity);
            }
            if entries.iter().any(|existing: &Entry| existing.item == item) {
                return Err(InventoryError::DuplicateItem);
            }
            entries.push(Entry {
                id: i as u32 + 1,
                item,
                quantity,
            });
        }
        Self::build_with_default_equip(entries, max_weight, max_volume)
    }

    pub(crate) fn from_loadout(
        loadout: &[LoadoutEntry],
        max_weight: u32,
        max_volume: u32,
    ) -> Result<Self, InventoryError> {
        let mut entries = Vec::with_capacity(loadout.len());
        for (i, entry) in loadout.iter().enumerate() {
            let item = item_id_by_str(&entry.item_id)?;
            if entry.quantity == 0 {
                return Err(InventoryError::InvalidQuantity);
            }
            if entries.iter().any(|existing: &Entry| existing.item == item) {
                return Err(InventoryError::DuplicateItem);
            }
            entries.push(Entry {
                id: i as u32 + 1,
                item,
                quantity: entry.quantity,
            });
        }
        Self::build_with_default_equip(entries, max_weight, max_volume)
    }

    fn build_with_default_equip(
        mut entries: Vec<Entry>,
        max_weight: u32,
        max_volume: u32,
    ) -> Result<Self, InventoryError> {
        let mut left = None;
        let mut right = None;
        if let Some(index) = entries.iter().position(|e| {
            let s = item_spec(e.item);
            s.weapon.is_some() && s.hands > 0
        }) {
            let hands = item_spec(entries[index].item).hands;
            if hands > 2 {
                return Err(InventoryError::InvalidHands);
            }
            let equipped_id = if entries[index].quantity == 1 {
                entries[index].id
            } else {
                let next_id = entries
                    .iter()
                    .map(|e| e.id)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or(InventoryError::Overflow)?;
                entries[index].quantity -= 1;
                let item = entries[index].item;
                entries.push(Entry {
                    id: next_id,
                    item,
                    quantity: 1,
                });
                next_id
            };
            if hands == 2 {
                left = Some(equipped_id);
                right = Some(equipped_id);
            } else {
                right = Some(equipped_id);
            }
        }
        Self::from_entries(entries, left, right, max_weight, max_volume)
    }

    pub(crate) fn default_infantry() -> Self {
        let spec = crate::definitions::default_unit_spec();
        Self::from_static_loadout(
            spec.default_loadout,
            spec.max_weight_grams,
            spec.max_stowed_volume_ml,
        )
        .expect("default infantry inventory must be valid")
    }

    fn from_entries(
        entries: Vec<Entry>,
        left: Option<u32>,
        right: Option<u32>,
        max_weight: u32,
        max_volume: u32,
    ) -> Result<Self, InventoryError> {
        let next_id = entries
            .iter()
            .map(|entry| entry.id)
            .max()
            .unwrap_or(0)
            .checked_add(1);
        let mut inventory = Self {
            entries,
            left,
            right,
            uses: Vec::new(),
            max_weight,
            max_volume,
            next_id,
            weight_grams: 0,
            stowed_volume_ml: 0,
            load_factor: 1.0,
        };
        inventory.validate()?;
        Ok(inventory)
    }

    fn totals(&self) -> Result<(u32, u32), InventoryError> {
        let (mut weight, mut volume) = (0u32, 0u32);
        for (index, entry) in self.entries.iter().enumerate() {
            let d = item_spec(entry.item);
            if self.entries[..index]
                .iter()
                .any(|other| other.id == entry.id)
            {
                return Err(InventoryError::InvalidEntry);
            }
            let held = self.left == Some(entry.id) || self.right == Some(entry.id);
            if held
                && (entry.quantity != 1
                    || d.hands == 0
                    || (d.hands == 2
                        && (self.left != Some(entry.id) || self.right != Some(entry.id)))
                    || (d.hands == 1
                        && self.left == Some(entry.id)
                        && self.right == Some(entry.id)))
            {
                return Err(InventoryError::InvalidHands);
            }
            weight = weight
                .checked_add(
                    d.mass
                        .checked_mul(entry.quantity)
                        .ok_or(InventoryError::Overflow)?,
                )
                .ok_or(InventoryError::Overflow)?;
            volume = volume
                .checked_add(
                    d.volume
                        .checked_mul(if held { 0 } else { entry.quantity })
                        .ok_or(InventoryError::Overflow)?,
                )
                .ok_or(InventoryError::Overflow)?;
        }
        for id in [self.left, self.right].into_iter().flatten() {
            if !self.entries.iter().any(|entry| entry.id == id) {
                return Err(InventoryError::InvalidHands);
            }
        }
        Ok((weight, volume))
    }

    fn validate(&mut self) -> Result<(), InventoryError> {
        let (weight, volume) = self.totals()?;
        if weight > self.max_weight || volume > self.max_volume {
            return Err(InventoryError::Capacity);
        }
        self.weight_grams = weight;
        self.stowed_volume_ml = volume;
        self.load_factor = compute_load_factor(weight, volume, self.max_weight, self.max_volume);
        Ok(())
    }

    pub(crate) fn count_item(&self, item: ItemId) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.item == item)
            .map(|e| e.quantity)
            .sum()
    }

    pub(crate) fn weapon_entries(&self) -> Vec<(u32, WeaponId)> {
        self.entries
            .iter()
            .filter_map(|e| {
                let weapon = item_spec(e.item).weapon?;
                (e.quantity > 0).then_some((e.id, weapon))
            })
            .collect()
    }

    pub(crate) fn held_weapon_entry(&self) -> Option<(u32, WeaponId)> {
        [self.left, self.right]
            .into_iter()
            .flatten()
            .find_map(|id| {
                let entry = self.entries.iter().find(|entry| entry.id == id)?;
                let weapon = item_spec(entry.item).weapon?;
                Some((entry.id, weapon))
            })
    }

    pub(crate) fn held_weapon_spec(&self) -> Option<&'static WeaponSpec> {
        self.held_weapon_entry()
            .map(|(_, weapon_id)| crate::definitions::weapon_spec(weapon_id))
    }

    pub(crate) fn find_usable_category(&self, category: ItemCategory) -> Option<ItemId> {
        self.entries.iter().find_map(|entry| {
            (item_spec(entry.item).category == category && self.can_use_item(entry.item))
                .then_some(entry.item)
        })
    }

    #[inline]
    pub(crate) fn load_factor(&self) -> f64 {
        self.load_factor
    }

    fn stow_all(&mut self) -> Result<(), InventoryError> {
        let mut candidate = self.clone();
        candidate.left = None;
        candidate.right = None;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    fn equip(&mut self, id: u32, hand: bool) -> Result<u32, InventoryError> {
        let mut candidate = self.clone();
        let equipped = candidate.equip_inner(id, hand)?;
        *self = candidate;
        Ok(equipped)
    }

    fn equip_inner(&mut self, id: u32, hand: bool) -> Result<u32, InventoryError> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(InventoryError::InvalidEntry)?;
        let hands = item_spec(self.entries[index].item).hands;
        if hands == 0 || self.entries[index].quantity == 0 {
            return Err(InventoryError::Unavailable);
        }
        if (hands == 2 && (self.left.is_some() || self.right.is_some()))
            || (hand && self.left.is_some())
            || (!hand && self.right.is_some())
        {
            return Err(InventoryError::InvalidHands);
        }
        let equipped = if self.entries[index].quantity == 1 {
            id
        } else {
            let next_id = self.next_id.ok_or(InventoryError::Overflow)?;
            self.next_id = next_id.checked_add(1);
            self.entries[index].quantity -= 1;
            let item = self.entries[index].item;
            self.entries.push(Entry {
                id: next_id,
                item,
                quantity: 1,
            });
            next_id
        };
        if hands == 2 {
            self.left = Some(equipped);
            self.right = Some(equipped);
        } else if hand {
            self.left = Some(equipped);
        } else {
            self.right = Some(equipped);
        }
        self.validate()?;
        Ok(equipped)
    }

    fn record_use(&mut self, item: ItemId) -> Result<(), InventoryError> {
        if let Some(used) = self.uses.iter_mut().find(|used| used.item == item) {
            used.count = used.count.checked_add(1).ok_or(InventoryError::Overflow)?;
        } else {
            self.uses.push(UseRecord { item, count: 1 });
        }
        Ok(())
    }

    pub(crate) fn consume_item(&mut self, item_id: ItemId) -> Result<(), InventoryError> {
        let item = item_spec(item_id);
        if !item.category.consumable() {
            return Err(InventoryError::NotConsumable);
        }
        let idx = self
            .entries
            .iter()
            .position(|entry| {
                entry.item == item_id
                    && entry.quantity > 0
                    && self.left != Some(entry.id)
                    && self.right != Some(entry.id)
            })
            .ok_or(InventoryError::Unavailable)?;
        if let Some(used) = self.uses.iter().find(|used| used.item == item_id) {
            used.count.checked_add(1).ok_or(InventoryError::Overflow)?;
        }
        self.entries[idx].quantity -= 1;
        self.weight_grams -= item.mass;
        self.stowed_volume_ml -= item.volume;
        self.load_factor = compute_load_factor(
            self.weight_grams,
            self.stowed_volume_ml,
            self.max_weight,
            self.max_volume,
        );
        self.record_use(item_id)?;
        Ok(())
    }

    pub(crate) fn can_use_item(&self, item_id: ItemId) -> bool {
        let item = item_spec(item_id);
        if !item.category.consumable() {
            return false;
        }
        let entry = if item.hands == 0 {
            self.entries.iter().find(|entry| {
                entry.item == item_id
                    && entry.quantity > 0
                    && self.left != Some(entry.id)
                    && self.right != Some(entry.id)
            })
        } else {
            self.entries
                .iter()
                .find(|entry| entry.item == item_id && entry.quantity > 0)
        };
        let Some(entry) = entry else { return false };
        if item.hands != 0 {
            let mut stowed = self.stowed_volume_ml;
            for (index, held_id) in [self.left, self.right]
                .into_iter()
                .enumerate()
                .filter_map(|(index, id)| id.map(|id| (index, id)))
            {
                if index == 1 && self.left == Some(held_id) {
                    continue;
                }
                let Some(held) = self.entries.iter().find(|entry| entry.id == held_id) else {
                    return false;
                };
                let Some(next) = stowed.checked_add(item_spec(held.item).volume) else {
                    return false;
                };
                stowed = next;
            }
            if stowed > self.max_volume {
                return false;
            }
            if self.left == Some(entry.id) || self.right == Some(entry.id) {
                return false;
            }
            if entry.quantity > 1 && self.next_id.is_none() {
                return false;
            }
        }
        if let Some(used) = self.uses.iter().find(|used| used.item == item_id) {
            if used.count.checked_add(1).is_none() {
                return false;
            }
        }
        true
    }

    pub(crate) fn use_item_by_id(&mut self, item_id: ItemId) -> Result<(), InventoryError> {
        let item = item_spec(item_id);
        if !item.category.consumable() {
            return Err(InventoryError::NotConsumable);
        }
        if item.hands == 0 {
            return self.consume_item(item_id);
        }
        let mut candidate = self.clone();
        let original_left = candidate.left;
        let original_right = candidate.right;
        candidate.stow_all()?;
        let entry_id = candidate
            .entries
            .iter()
            .find(|entry| entry.item == item_id && entry.quantity > 0)
            .ok_or(InventoryError::Unavailable)?
            .id;
        let held = candidate.equip(entry_id, true)?;
        let entry = candidate
            .entries
            .iter_mut()
            .find(|entry| entry.id == held)
            .ok_or(InventoryError::InvalidEntry)?;
        entry.quantity = 0;
        // A split entry is temporary; its original stack retains its identity.
        if held != entry_id {
            candidate.entries.retain(|entry| entry.id != held);
        }
        candidate.left = None;
        candidate.right = None;
        candidate.validate()?;
        candidate.record_use(item_id)?;
        if let Some(id) = original_left {
            candidate.equip(id, true)?;
        }
        if let Some(id) = original_right {
            if original_left != Some(id) {
                candidate.equip(id, false)?;
            }
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> InventorySnapshot {
        InventorySnapshot {
            entries: self
                .entries
                .iter()
                .map(|e| InventoryEntry {
                    id: e.id,
                    item_id: item_spec(e.item).id.into(),
                    quantity: e.quantity,
                })
                .collect(),
            left_hand: self.left,
            right_hand: self.right,
            weight_grams: self.weight_grams,
            stowed_volume_ml: self.stowed_volume_ml,
            max_weight_grams: self.max_weight,
            max_stowed_volume_ml: self.max_volume,
            load_factor: self.load_factor,
            uses: self
                .uses
                .iter()
                .map(|u| ItemUse {
                    item_id: item_spec(u.item).id.into(),
                    count: u.count,
                })
                .collect(),
        }
    }

    #[cfg(test)]
    pub(crate) fn count(&self, id: &str) -> u32 {
        item_id_by_str(id)
            .map(|item| self.count_item(item))
            .unwrap_or(0)
    }

    #[cfg(test)]
    pub(crate) fn can_use(&self, id: &str) -> bool {
        item_id_by_str(id)
            .map(|item| self.can_use_item(item))
            .unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) fn consume(&mut self, id: &str) -> Result<(), InventoryError> {
        let item = item_id_by_str(id)?;
        self.consume_item(item)
    }

    #[cfg(test)]
    pub(crate) fn use_item(&mut self, id: &str) -> Result<(), InventoryError> {
        let item = item_id_by_str(id)?;
        self.use_item_by_id(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32, item: &str, quantity: u32) -> Entry {
        Entry {
            id,
            item: item_id_by_str(item).expect("known test item"),
            quantity,
        }
    }

    fn from_parts(
        entries: Vec<Entry>,
        left: Option<u32>,
        right: Option<u32>,
        max_weight: u32,
        max_volume: u32,
    ) -> Result<Inventory, InventoryError> {
        Inventory::from_entries(entries, left, right, max_weight, max_volume)
    }

    #[test]
    fn default_quick_use_restores_rifle_and_consumes_one() {
        let mut inv = Inventory::default_infantry();
        assert_eq!(
            (inv.snapshot().weight_grams, inv.snapshot().stowed_volume_ml),
            (9520, 6500)
        );
        inv.use_item("water").unwrap();
        let after = inv.snapshot();
        assert_eq!((after.weight_grams, after.stowed_volume_ml), (9020, 6000));
        assert_eq!((after.left_hand, after.right_hand), (Some(1), Some(1)));
        assert_eq!(inv.count("water"), 1);
        assert_eq!(after.uses[0].count, 1);
    }

    #[test]
    fn construction_checks_capacity_boundaries_and_overflow() {
        assert!(from_parts(vec![entry(1, "water", 2)], None, None, 1000, 1000).is_ok());
        assert!(matches!(
            from_parts(vec![entry(1, "water", 2)], None, None, 999, 1000),
            Err(InventoryError::Capacity)
        ));
        assert!(matches!(
            from_parts(vec![entry(1, "water", 2)], None, None, 1000, 999),
            Err(InventoryError::Capacity)
        ));
        assert!(matches!(
            from_parts(
                vec![entry(1, "water", u32::MAX)],
                None,
                None,
                u32::MAX,
                u32::MAX
            ),
            Err(InventoryError::Overflow)
        ));
    }

    #[test]
    fn held_items_require_valid_hand_references_and_single_instances() {
        assert!(from_parts(vec![entry(1, "rifle", 1)], Some(1), Some(1), 4000, 0).is_ok());
        assert!(matches!(
            from_parts(vec![entry(1, "rifle", 1)], Some(1), None, 4000, 6000),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            from_parts(vec![entry(1, "water", 2)], Some(1), None, 1000, 1000),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            from_parts(vec![entry(1, "water", 1)], Some(1), Some(1), 500, 500),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            from_parts(vec![], Some(7), None, 0, 0),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            from_parts(vec![entry(1, "water", 0)], Some(1), None, 500, 500),
            Err(InventoryError::InvalidHands)
        ));
    }

    #[test]
    fn equip_splits_stack_with_stable_original_id() {
        let mut inv = from_parts(vec![entry(5, "water", 2)], None, None, 1000, 1000).unwrap();
        let held = inv.equip(5, true).unwrap();
        assert_ne!(held, 5);
        assert_eq!(
            inv.entries,
            vec![entry(5, "water", 1), entry(held, "water", 1)]
        );
        assert_eq!(inv.snapshot().stowed_volume_ml, 500);
    }

    #[test]
    fn intermediate_stow_failure_rolls_back_before_supply_is_removed() {
        let mut inv = from_parts(
            vec![
                entry(1, "rifle", 1),
                entry(2, "water", 2),
                entry(3, "ration", 10),
                entry(4, "frag-grenade", 1),
            ],
            Some(1),
            Some(1),
            20000,
            15000,
        )
        .unwrap();
        assert_eq!(inv.snapshot().stowed_volume_ml, 9400);
        let before = inv.snapshot();
        assert_eq!(inv.use_item("water"), Err(InventoryError::Capacity));
        assert_eq!(inv.snapshot(), before);
    }

    #[test]
    fn unavailable_and_unknown_items_leave_state_unchanged() {
        let mut inv = Inventory::default_infantry();
        let before = inv.snapshot();
        assert_eq!(inv.use_item("unknown"), Err(InventoryError::UnknownItem));
        assert_eq!(inv.consume("unknown"), Err(InventoryError::UnknownItem));
        for _ in 0..60 {
            inv.consume("rifle-ammo").unwrap();
        }
        let empty = inv.snapshot();
        assert_eq!(inv.consume("rifle-ammo"), Err(InventoryError::Unavailable));
        assert_eq!(inv.snapshot(), empty);
        assert_ne!(before, empty);
    }

    #[test]
    fn reusable_tools_and_weapons_cannot_be_consumed() {
        let mut inventory = Inventory::default_infantry();
        let before = inventory.snapshot();
        for id in ["entrenching-tool", "rifle"] {
            assert!(!inventory.can_use(id));
            assert_eq!(inventory.use_item(id), Err(InventoryError::NotConsumable));
            assert_eq!(inventory.consume(id), Err(InventoryError::NotConsumable));
        }
        assert_eq!(inventory.snapshot(), before);
    }

    #[test]
    fn eligibility_matches_transactions_across_inventory_states() {
        let mut scenarios = vec![
            Inventory::default_infantry(),
            from_parts(
                vec![
                    entry(1, "rifle", 1),
                    entry(2, "water", 2),
                    entry(3, "ration", 10),
                    entry(4, "frag-grenade", 1),
                ],
                Some(1),
                Some(1),
                20000,
                15000,
            )
            .unwrap(),
            from_parts(vec![entry(1, "water", 1)], Some(1), None, 500, 500).unwrap(),
        ];
        for inv in &mut scenarios {
            for id in [
                "unknown",
                "water",
                "ration",
                "frag-grenade",
                "rifle-ammo",
                "rifle",
            ] {
                assert_eq!(inv.can_use(id), inv.clone().use_item(id).is_ok(), "{id}");
            }
        }
    }

    #[test]
    fn repeated_uses_do_not_accumulate_split_entries() {
        let mut inv = from_parts(vec![entry(1, "water", 100)], None, None, 50000, 50000).unwrap();
        for _ in 0..99 {
            inv.use_item("water").unwrap();
            assert_eq!(inv.entries.len(), 1);
        }
        assert_eq!(inv.entries[0], entry(1, "water", 1));
        inv.use_item("water").unwrap();
        assert_eq!(inv.entries.len(), 1);
        assert_eq!((inv.left, inv.right), (None, None));
    }

    #[test]
    fn weight_and_volume_independently_reduce_load_factor() {
        let held = from_parts(vec![entry(1, "water", 1)], Some(1), None, 1000, 1000).unwrap();
        let stowed = from_parts(vec![entry(1, "water", 1)], None, None, 1000, 1000).unwrap();
        let lighter_limit =
            from_parts(vec![entry(1, "water", 1)], Some(1), None, 2000, 1000).unwrap();
        assert!(held.load_factor() < lighter_limit.load_factor());
        assert!(stowed.load_factor() < held.load_factor());
    }

    #[test]
    fn from_loadout_equips_primary_weapon_and_rejects_bad_loadouts() {
        let custom = vec![
            LoadoutEntry {
                item_id: "rifle".into(),
                quantity: 1,
            },
            LoadoutEntry {
                item_id: "rifle-ammo".into(),
                quantity: 10,
            },
        ];
        let inv = Inventory::from_loadout(&custom, 20_000, 15_000).unwrap();
        assert_eq!(inv.held_weapon_spec().map(|w| w.id), Some("rifle"));
        assert_eq!(inv.count("rifle-ammo"), 10);

        let unarmed = vec![LoadoutEntry {
            item_id: "medical-supply".into(),
            quantity: 2,
        }];
        let medic = Inventory::from_loadout(&unarmed, 20_000, 15_000).unwrap();
        assert!(medic.held_weapon_spec().is_none());

        assert_eq!(
            Inventory::from_loadout(
                &[LoadoutEntry {
                    item_id: "unknown".into(),
                    quantity: 1,
                }],
                20_000,
                15_000,
            )
            .err(),
            Some(InventoryError::UnknownItem)
        );
        assert_eq!(
            Inventory::from_loadout(
                &[LoadoutEntry {
                    item_id: "water".into(),
                    quantity: 0,
                }],
                20_000,
                15_000,
            )
            .err(),
            Some(InventoryError::InvalidQuantity)
        );
        assert_eq!(
            Inventory::from_loadout(
                &[
                    LoadoutEntry {
                        item_id: "water".into(),
                        quantity: 1,
                    },
                    LoadoutEntry {
                        item_id: "water".into(),
                        quantity: 1,
                    },
                ],
                20_000,
                15_000,
            )
            .err(),
            Some(InventoryError::DuplicateItem)
        );
    }
}
