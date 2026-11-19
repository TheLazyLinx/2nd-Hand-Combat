//! Fixed-stride little-endian binary snapshot encoder for high-frequency unit state.
//!
//! Layout (`COMPACT_UNIT_BYTES = 72` bytes per unit, naturally aligned):
//! - `0..8`:   `x: f64`
//! - `8..16`:  `y: f64`
//! - `16..24`: `hp: f64`
//! - `24..28`: `id: u32`
//! - `28..32`: `faction: u32`
//! - `32..36`: `shots_fired: u32`
//! - `36..40`: `target_id: i32` (`-1` when `None`)
//! - `40..44`: `elevation: f32`
//! - `44..48`: `movement_multiplier: f32`
//! - `48..52`: `protection: f32` (`0.0` when `None`; see `COMPACT_FLAG_HAS_PROTECTION`)
//! - `52..56`: `mobility_factor: f32`
//! - `56..60`: `nutrition: f32`
//! - `60..64`: `hydration: f32`
//! - `64..68`: `needs_factor: f32`
//! - `68`:     `behavior: u8` (`0..=6`)
//! - `69`:     `surface: u8` (`0..=4`)
//! - `70`:     `flags: u8` (bit 0 revealed, bit 1 has_protection, bit 2 nutrition_shortage, bit 3 hydration_shortage)
//! - `71`:     `reserved: u8` (`0`)

use crate::{
    map,
    terrain::{Terrain, TerrainCache},
    types::*,
    unit::Unit,
};

pub const COMPACT_UNIT_BYTES: usize = 72;

pub const COMPACT_FLAG_REVEALED: u8 = 1 << 0;
pub const COMPACT_FLAG_HAS_PROTECTION: u8 = 1 << 1;
pub const COMPACT_FLAG_NUTRITION_SHORTAGE: u8 = 1 << 2;
pub const COMPACT_FLAG_HYDRATION_SHORTAGE: u8 = 1 << 3;

#[inline]
pub const fn encode_behavior(behavior: &Behavior) -> u8 {
    match behavior {
        Behavior::Searching => 0,
        Behavior::Investigating => 1,
        Behavior::Advancing => 2,
        Behavior::Engaging => 3,
        Behavior::FallingBack => 4,
        Behavior::Defending => 5,
        Behavior::Dead => 6,
    }
}

#[inline]
pub const fn encode_surface(surface: Surface) -> u8 {
    match surface {
        Surface::Open => 0,
        Surface::Road => 1,
        Surface::Mud => 2,
        Surface::Brush => 3,
        Surface::Woodland => 4,
    }
}

#[inline]
pub(crate) fn find_living_unit_by_id(units: &[Unit], id: u32) -> Option<&Unit> {
    if let Ok(idx) = units.binary_search_by_key(&id, |u| u.placement.id) {
        let u = &units[idx];
        return (u.vitals.hp > 0.0).then_some(u);
    }
    units
        .iter()
        .find(|other| other.placement.id == id && other.vitals.hp > 0.0)
}

pub(crate) fn write_units(
    units: &[Unit],
    map_data: &ArenaMap,
    terrain_cache: &TerrainCache,
    terrain: Terrain<'_>,
    tick: u32,
    out: &mut Vec<u8>,
) {
    out.clear();
    out.reserve(units.len() * COMPACT_UNIT_BYTES);
    for u in units {
        let needs_factor = u.needs_factor();
        let mobility_factor = u.inventory.load_factor() * needs_factor;
        let surface = map::surface(map_data, u.nav.position);
        let elevation = terrain_cache.ground_height(map_data, u.nav.position);
        let revealed = tick < u.combat.revealed_until;
        let protection = u
            .perception
            .target
            .and_then(|id| find_living_unit_by_id(units, id))
            .map(|other| 1.0 - map::shot_fraction_t(terrain, other.nav.position, u.nav.position));
        let mut flags = 0u8;
        if revealed {
            flags |= COMPACT_FLAG_REVEALED;
        }
        if protection.is_some() {
            flags |= COMPACT_FLAG_HAS_PROTECTION;
        }
        if crate::sustainment::shortage(u.sustainment.nutrition) {
            flags |= COMPACT_FLAG_NUTRITION_SHORTAGE;
        }
        if crate::sustainment::shortage(u.sustainment.hydration) {
            flags |= COMPACT_FLAG_HYDRATION_SHORTAGE;
        }
        let target_id = u.perception.target.map_or(-1i32, |id| id as i32);

        out.extend_from_slice(&u.nav.position.x.to_le_bytes());
        out.extend_from_slice(&u.nav.position.y.to_le_bytes());
        out.extend_from_slice(&u.vitals.hp.to_le_bytes());
        out.extend_from_slice(&u.placement.id.to_le_bytes());
        out.extend_from_slice(&u.placement.faction.to_le_bytes());
        out.extend_from_slice(&u.combat.shots.to_le_bytes());
        out.extend_from_slice(&target_id.to_le_bytes());
        out.extend_from_slice(&(elevation as f32).to_le_bytes());
        out.extend_from_slice(&(u.nav.movement_multiplier as f32).to_le_bytes());
        out.extend_from_slice(&(protection.unwrap_or(0.0) as f32).to_le_bytes());
        out.extend_from_slice(&(mobility_factor as f32).to_le_bytes());
        out.extend_from_slice(&(u.sustainment.nutrition as f32).to_le_bytes());
        out.extend_from_slice(&(u.sustainment.hydration as f32).to_le_bytes());
        out.extend_from_slice(&(needs_factor as f32).to_le_bytes());
        out.push(encode_behavior(&u.combat.behavior));
        out.push(encode_surface(surface));
        out.push(flags);
        out.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{definitions::default_factions, Battle};

    fn read_f64(buf: &[u8], off: usize) -> f64 {
        f64::from_le_bytes(buf[off..off + 8].try_into().unwrap())
    }
    fn read_f32(buf: &[u8], off: usize) -> f32 {
        f32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
    }
    fn read_u32(buf: &[u8], off: usize) -> u32 {
        u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
    }
    fn read_i32(buf: &[u8], off: usize) -> i32 {
        i32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
    }

    #[test]
    fn compact_binary_encoding_matches_full_snapshot_and_tracks_inventory_changes() {
        let mut map = ArenaMap::flat(16, 8);
        map.surfaces[2 * 16 + 3] = Surface::Mud;
        map.surfaces[2 * 16 + 7] = Surface::Woodland;
        let doctrine = crate::catalog().default_doctrine;
        let scenario = Scenario {
            map,
            seed: 7,
            factions: default_factions(),
            loadouts: vec![],
            time_limit: Some(60.0),
            units: vec![
                UnitPlacement {
                    id: 1,
                    faction: 0,
                    definition_id: "infantry".into(),
                    position: Point { x: 3.5, y: 2.5 },
                    doctrine: doctrine.clone(),
                    loadout: None,
                },
                UnitPlacement {
                    id: 2,
                    faction: 1,
                    definition_id: "infantry".into(),
                    position: Point { x: 7.5, y: 2.5 },
                    doctrine,
                    loadout: None,
                },
            ],
        };
        let mut battle = Battle::new(scenario).unwrap();
        let initial_inv = battle.unit_inventory_if_changed(1, 0).expect("initial inv");
        assert_eq!(initial_inv.0, 1);
        assert!(battle.unit_inventory_if_changed(1, initial_inv.0).is_none());

        for _ in 0..10 {
            battle.step();
        }
        let full = battle.snapshot();
        let mut bytes = Vec::new();
        battle.write_compact_units(&mut bytes);
        assert_eq!(bytes.len(), full.units.len() * COMPACT_UNIT_BYTES);

        for (i, u) in full.units.iter().enumerate() {
            let rec = &bytes[i * COMPACT_UNIT_BYTES..(i + 1) * COMPACT_UNIT_BYTES];
            assert_eq!(read_f64(rec, 0), u.position.x);
            assert_eq!(read_f64(rec, 8), u.position.y);
            assert_eq!(read_f64(rec, 16), u.hp);
            assert_eq!(read_u32(rec, 24), u.id);
            assert_eq!(read_u32(rec, 28), u.faction);
            assert_eq!(read_u32(rec, 32), u.shots_fired);
            assert_eq!(read_i32(rec, 36), u.target_id.map_or(-1, |t| t as i32));
            assert!((read_f32(rec, 40) as f64 - u.terrain.elevation).abs() < 1e-5);
            assert!((read_f32(rec, 44) as f64 - u.terrain.movement_multiplier).abs() < 1e-5);
            let flags = rec[70];
            assert_eq!(flags & COMPACT_FLAG_REVEALED != 0, u.terrain.revealed);
            assert_eq!(
                flags & COMPACT_FLAG_HAS_PROTECTION != 0,
                u.terrain.protection.is_some()
            );
            if let Some(prot) = u.terrain.protection {
                assert!((read_f32(rec, 48) as f64 - prot).abs() < 1e-5);
            }
            assert!((read_f32(rec, 52) as f64 - u.mobility_factor).abs() < 1e-5);
            assert!((read_f32(rec, 56) as f64 - u.sustainment.nutrition).abs() < 1e-4);
            assert!((read_f32(rec, 60) as f64 - u.sustainment.hydration).abs() < 1e-4);
            assert!((read_f32(rec, 64) as f64 - u.sustainment.needs_factor).abs() < 1e-5);
            assert_eq!(rec[68], encode_behavior(&u.behavior));
            assert_eq!(rec[69], encode_surface(u.terrain.surface));
            assert_eq!(
                flags & COMPACT_FLAG_NUTRITION_SHORTAGE != 0,
                u.sustainment.nutrition_shortage
            );
            assert_eq!(
                flags & COMPACT_FLAG_HYDRATION_SHORTAGE != 0,
                u.sustainment.hydration_shortage
            );
        }

        // After firing shots, unit 1's inventory version must advance and match snapshot.
        let updated = battle
            .unit_inventory_if_changed(1, initial_inv.0)
            .expect("inventory changed after combat");
        assert!(updated.0 > initial_inv.0);
        assert_eq!(updated.1, full.units[0].inventory);
        assert!(battle.unit_inventory_if_changed(1, updated.0).is_none());
    }
}
