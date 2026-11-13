use crate::types::*;

struct ItemSpec {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    mass: u32,
    volume: u32,
    hands: u32,
    weapon: Option<&'static str>,
    initial: u32,
}
impl ItemSpec {
    fn consumable(&self) -> bool {
        matches!(
            self.category,
            "ammo" | "grenade" | "medical" | "ration" | "water"
        )
    }
}
const DATA: [ItemSpec; 7] = [
    ItemSpec {
        id: "rifle",
        name: "Rifle",
        category: "weapon",
        mass: 4000,
        volume: 6000,
        hands: 2,
        weapon: Some("rifle"),
        initial: 1,
    },
    ItemSpec {
        id: "rifle-ammo",
        name: "Rifle rounds",
        category: "ammo",
        mass: 12,
        volume: 15,
        hands: 0,
        weapon: None,
        initial: 60,
    },
    ItemSpec {
        id: "frag-grenade",
        name: "Frag grenade",
        category: "grenade",
        mass: 400,
        volume: 400,
        hands: 1,
        weapon: None,
        initial: 2,
    },
    ItemSpec {
        id: "entrenching-tool",
        name: "Entrenching tool",
        category: "tool",
        mass: 1000,
        volume: 1000,
        hands: 2,
        weapon: None,
        initial: 1,
    },
    ItemSpec {
        id: "medical-supply",
        name: "Medical supply",
        category: "medical",
        mass: 500,
        volume: 600,
        hands: 2,
        weapon: None,
        initial: 2,
    },
    ItemSpec {
        id: "ration",
        name: "Ration",
        category: "ration",
        mass: 500,
        volume: 800,
        hands: 1,
        weapon: None,
        initial: 2,
    },
    ItemSpec {
        id: "water",
        name: "Water",
        category: "water",
        mass: 500,
        volume: 500,
        hands: 1,
        weapon: None,
        initial: 2,
    },
];
fn spec(id: &str) -> Result<&'static ItemSpec, InventoryError> {
    DATA.iter()
        .find(|item| item.id == id)
        .ok_or(InventoryError::UnknownItem)
}
pub(crate) fn items() -> Vec<ItemDefinition> {
    DATA.iter()
        .map(|d| ItemDefinition {
            id: d.id.into(),
            name: d.name.into(),
            category: d.category.into(),
            mass_grams: d.mass,
            volume_ml: d.volume,
            hands_required: d.hands,
            weapon_id: d.weapon.map(str::to_owned),
        })
        .collect()
}
pub(crate) fn loadout() -> Vec<LoadoutEntry> {
    DATA.iter()
        .map(|d| LoadoutEntry {
            item_id: d.id.into(),
            quantity: d.initial,
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
    Overflow,
}

#[derive(Clone)]
pub(crate) struct Inventory {
    entries: Vec<InventoryEntry>,
    left: Option<u32>,
    right: Option<u32>,
    uses: Vec<ItemUse>,
    max_weight: u32,
    max_volume: u32,
    next_id: Option<u32>,
}
impl Inventory {
    pub(crate) fn new() -> Self {
        let entries = DATA
            .iter()
            .enumerate()
            .map(|(i, d)| InventoryEntry {
                id: i as u32 + 1,
                item_id: d.id.into(),
                quantity: d.initial,
            })
            .collect();
        Self::from_parts(entries, Some(1), Some(1), 20_000, 15_000)
            .expect("default inventory must be valid")
    }
    fn from_parts(
        entries: Vec<InventoryEntry>,
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
        let inventory = Self {
            entries,
            left,
            right,
            uses: Vec::new(),
            max_weight,
            max_volume,
            next_id,
        };
        inventory.validate()?;
        Ok(inventory)
    }
    fn totals(&self) -> Result<(u32, u32), InventoryError> {
        let (mut weight, mut volume) = (0u32, 0u32);
        for (index, entry) in self.entries.iter().enumerate() {
            let d = spec(&entry.item_id)?;
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
    fn validate(&self) -> Result<(), InventoryError> {
        let (weight, volume) = self.totals()?;
        if weight > self.max_weight || volume > self.max_volume {
            return Err(InventoryError::Capacity);
        }
        Ok(())
    }
    pub(crate) fn count(&self, id: &str) -> u32 {
        self.entries
            .iter()
            .filter(|e| e.item_id == id)
            .map(|e| e.quantity)
            .sum()
    }
    pub(crate) fn held_weapon(&self) -> Option<WeaponDefinition> {
        [self.left, self.right]
            .into_iter()
            .flatten()
            .find_map(|id| {
                let entry = self.entries.iter().find(|entry| entry.id == id)?;
                let weapon_id = spec(&entry.item_id).ok()?.weapon?;
                crate::definitions::weapon(weapon_id)
            })
    }
    pub(crate) fn load_factor(&self) -> f64 {
        let (weight, volume) = self.totals().expect("valid inventory");
        let weight_fraction = if self.max_weight == 0 {
            0.0
        } else {
            weight as f64 / self.max_weight as f64
        };
        let volume_fraction = if self.max_volume == 0 {
            0.0
        } else {
            volume as f64 / self.max_volume as f64
        };
        1.0 - 0.25 * weight_fraction - 0.15 * volume_fraction
    }
    // Operations act on a candidate so failed intermediate steps never alter the inventory.
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
        let hands = spec(&self.entries[index].item_id)?.hands;
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
            self.entries.push(InventoryEntry {
                id: next_id,
                item_id: self.entries[index].item_id.clone(),
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
    fn record_use(&mut self, id: &str) -> Result<(), InventoryError> {
        if let Some(used) = self.uses.iter_mut().find(|used| used.item_id == id) {
            used.count = used.count.checked_add(1).ok_or(InventoryError::Overflow)?;
        } else {
            self.uses.push(ItemUse {
                item_id: id.into(),
                count: 1,
            });
        }
        Ok(())
    }
    pub(crate) fn consume(&mut self, id: &str) -> Result<(), InventoryError> {
        if !spec(id)?.consumable() {
            return Err(InventoryError::NotConsumable);
        }
        let mut candidate = self.clone();
        let entry = candidate
            .entries
            .iter_mut()
            .find(|entry| {
                entry.item_id == id
                    && entry.quantity > 0
                    && candidate.left != Some(entry.id)
                    && candidate.right != Some(entry.id)
            })
            .ok_or(InventoryError::Unavailable)?;
        entry.quantity -= 1;
        candidate.record_use(id)?;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub(crate) fn can_use(&self, id: &str) -> bool {
        let Ok(item) = spec(id) else { return false };
        if !item.consumable() {
            return false;
        }
        let (_, volume) = self.totals().expect("valid inventory");
        let entry = if item.hands == 0 {
            self.entries.iter().find(|entry| {
                entry.item_id == id
                    && entry.quantity > 0
                    && self.left != Some(entry.id)
                    && self.right != Some(entry.id)
            })
        } else {
            self.entries
                .iter()
                .find(|entry| entry.item_id == id && entry.quantity > 0)
        };
        let Some(entry) = entry else { return false };
        if item.hands != 0 {
            let mut stowed = volume;
            for (index, held_id) in [self.left, self.right]
                .into_iter()
                .enumerate()
                .filter_map(|(index, id)| id.map(|id| (index, id)))
            {
                if index == 1 && self.left == Some(held_id) {
                    continue;
                }
                let held = self
                    .entries
                    .iter()
                    .find(|entry| entry.id == held_id)
                    .expect("valid hand reference");
                stowed = stowed
                    .checked_add(spec(&held.item_id).expect("valid item").volume)
                    .expect("inventory volume overflow");
            }
            if stowed > self.max_volume {
                return false;
            }
            if self.left == Some(entry.id) || self.right == Some(entry.id) {
                return false;
            }
            if entry.quantity > 1 {
                self.next_id.expect("inventory entry id overflow");
            }
        }
        if let Some(used) = self.uses.iter().find(|used| used.item_id == id) {
            used.count
                .checked_add(1)
                .expect("inventory use counter overflow");
        }
        true
    }
    pub(crate) fn use_item(&mut self, id: &str) -> Result<(), InventoryError> {
        let item = spec(id)?;
        if !item.consumable() {
            return Err(InventoryError::NotConsumable);
        }
        let hands = item.hands;
        if hands == 0 {
            return self.consume(id);
        }
        let mut candidate = self.clone();
        let original_left = candidate.left;
        let original_right = candidate.right;
        candidate.stow_all()?;
        let entry_id = candidate
            .entries
            .iter()
            .find(|entry| entry.item_id == id && entry.quantity > 0)
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
        candidate.record_use(id)?;
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
        let (weight, volume) = self.totals().expect("valid inventory");
        InventorySnapshot {
            entries: self.entries.clone(),
            left_hand: self.left,
            right_hand: self.right,
            weight_grams: weight,
            stowed_volume_ml: volume,
            max_weight_grams: self.max_weight,
            max_stowed_volume_ml: self.max_volume,
            load_factor: self.load_factor(),
            uses: self.uses.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: u32, item: &str, quantity: u32) -> InventoryEntry {
        InventoryEntry {
            id,
            item_id: item.into(),
            quantity,
        }
    }
    #[test]
    fn default_quick_use_restores_rifle_and_consumes_one() {
        let mut inv = Inventory::new();
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
        assert!(Inventory::from_parts(vec![entry(1, "water", 2)], None, None, 1000, 1000).is_ok());
        assert!(matches!(
            Inventory::from_parts(vec![entry(1, "water", 2)], None, None, 999, 1000),
            Err(InventoryError::Capacity)
        ));
        assert!(matches!(
            Inventory::from_parts(vec![entry(1, "water", 2)], None, None, 1000, 999),
            Err(InventoryError::Capacity)
        ));
        assert!(matches!(
            Inventory::from_parts(
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
        assert!(
            Inventory::from_parts(vec![entry(1, "rifle", 1)], Some(1), Some(1), 4000, 0).is_ok()
        );
        assert!(matches!(
            Inventory::from_parts(vec![entry(1, "rifle", 1)], Some(1), None, 4000, 6000),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            Inventory::from_parts(vec![entry(1, "water", 2)], Some(1), None, 1000, 1000),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            Inventory::from_parts(vec![entry(1, "water", 1)], Some(1), Some(1), 500, 500),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            Inventory::from_parts(vec![], Some(7), None, 0, 0),
            Err(InventoryError::InvalidHands)
        ));
        assert!(matches!(
            Inventory::from_parts(vec![entry(1, "water", 0)], Some(1), None, 500, 500),
            Err(InventoryError::InvalidHands)
        ));
    }
    #[test]
    fn equip_splits_stack_with_stable_original_id() {
        let mut inv =
            Inventory::from_parts(vec![entry(5, "water", 2)], None, None, 1000, 1000).unwrap();
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
        let mut inv = Inventory::from_parts(
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
        let mut inv = Inventory::new();
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
        let mut inventory = Inventory::new();
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
            Inventory::new(),
            Inventory::from_parts(
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
            Inventory::from_parts(vec![entry(1, "water", 1)], Some(1), None, 500, 500).unwrap(),
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
        let mut inv =
            Inventory::from_parts(vec![entry(1, "water", 100)], None, None, 50000, 50000).unwrap();
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
        let held =
            Inventory::from_parts(vec![entry(1, "water", 1)], Some(1), None, 1000, 1000).unwrap();
        let stowed =
            Inventory::from_parts(vec![entry(1, "water", 1)], None, None, 1000, 1000).unwrap();
        let lighter_limit =
            Inventory::from_parts(vec![entry(1, "water", 1)], Some(1), None, 2000, 1000).unwrap();
        assert!(held.load_factor() < lighter_limit.load_factor());
        assert!(stowed.load_factor() < held.load_factor());
    }
}
