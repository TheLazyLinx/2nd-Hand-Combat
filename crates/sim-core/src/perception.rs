//! Per-unit enemy spotting and incremental post-movement observation refresh.
//!
//! Invariant: `refresh_post_movement` produces the exact same sorted index lists as calling
//! `observe_all_into` from scratch for every living unit.

use crate::{map, terrain::Terrain, types::is_hostile, unit::Unit};

#[cfg(test)]
use crate::{terrain::TerrainCache, types::ArenaMap};

#[inline]
fn sort_seen(units: &[Unit], observer_pos: crate::types::Point, seen: &mut [usize]) {
    seen.sort_by(|a, b| {
        observer_pos
            .distance(units[*a].nav.position)
            .total_cmp(&observer_pos.distance(units[*b].nav.position))
            .then(units[*a].placement.id.cmp(&units[*b].placement.id))
    });
}

pub(crate) fn observe_all_into(
    units: &[Unit],
    terrain: Terrain<'_>,
    index: usize,
    tick: u32,
    out: &mut Vec<usize>,
) {
    out.clear();
    let me = &units[index];
    if me.vitals.hp <= 0.0 {
        return;
    }
    for (i, u) in units.iter().enumerate() {
        if u.vitals.hp > 0.0
            && is_hostile(me.placement.faction, u.placement.faction)
            && map::detectable_t(
                terrain,
                me.nav.position,
                u.nav.position,
                tick < u.combat.revealed_until,
            )
        {
            out.push(i);
        }
    }
    sort_seen(units, me.nav.position, out);
}

/// Refreshes observations after movement by reusing pre-movement results for stationary pairs.
pub(crate) fn refresh_post_movement(
    units: &[Unit],
    terrain: Terrain<'_>,
    tick: u32,
    pre: &[Vec<usize>],
    moved: &[bool],
    post: &mut [Vec<usize>],
) {
    if !moved.iter().any(|&m| m) {
        for (dst, src) in post.iter_mut().zip(pre) {
            dst.clear();
            dst.extend_from_slice(src);
        }
        return;
    }
    for (i, u) in units.iter().enumerate() {
        if u.vitals.hp <= 0.0 {
            post[i].clear();
            continue;
        }
        if moved[i] {
            observe_all_into(units, terrain, i, tick, &mut post[i]);
            continue;
        }
        let dst = &mut post[i];
        dst.clear();
        for &j in &pre[i] {
            if !moved[j] {
                dst.push(j);
            }
        }
        let mut added_moved = false;
        for (j, other) in units.iter().enumerate() {
            if moved[j]
                && other.vitals.hp > 0.0
                && is_hostile(u.placement.faction, other.placement.faction)
                && map::detectable_t(
                    terrain,
                    u.nav.position,
                    other.nav.position,
                    tick < other.combat.revealed_until,
                )
            {
                dst.push(j);
                added_moved = true;
            }
        }
        if added_moved {
            sort_seen(units, u.nav.position, dst);
        }
    }
}

#[cfg(test)]
pub(crate) fn observe_all(units: &[Unit], map: &ArenaMap, index: usize, tick: u32) -> Vec<usize> {
    let cache = TerrainCache::without_neighbors(map);
    let mut out = Vec::new();
    observe_all_into(units, cache.view(map), index, tick, &mut out);
    out
}

#[cfg(test)]
pub(crate) fn observe(units: &[Unit], map: &ArenaMap, index: usize, tick: u32) -> Option<usize> {
    observe_all(units, map, index, tick).first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{definitions::catalog, types::*};

    fn make_unit(id: u32, faction: u32, x: f64, y: f64) -> Unit {
        Unit::new(UnitPlacement {
            id,
            faction,
            definition_id: "infantry".into(),
            position: Point { x, y },
            doctrine: catalog().default_doctrine,
            loadout: None,
        })
    }

    #[test]
    fn incremental_post_movement_refresh_matches_full_observation() {
        let mut map = ArenaMap::flat(18, 10);
        for x in 6..12 {
            for y in 2..6 {
                map.surfaces[y * 18 + x] = Surface::Woodland;
            }
        }
        map.obstacles.push(Obstacle {
            x: 9,
            y: 6,
            width: 1,
            height: 2,
            height_metres: 2.4,
        });
        let cache = TerrainCache::without_neighbors(&map);
        let terrain = cache.view(&map);

        let mut units = vec![
            make_unit(1, 0, 3.5, 4.5),  // stationary observer
            make_unit(2, 0, 4.5, 7.5),  // moving observer
            make_unit(3, 1, 7.5, 4.5),  // stationary enemy (seen by 1 & 2)
            make_unit(4, 1, 11.5, 4.5), // moves out of woodland into sight
            make_unit(5, 1, 11.5, 7.5), // moves behind wall out of sight of 2
        ];
        let tick = 5;
        let mut pre = vec![Vec::new(); units.len()];
        for (i, slot) in pre.iter_mut().enumerate() {
            observe_all_into(&units, terrain, i, tick, slot);
        }

        // Move units 1 (index 1), 3 (index 3), and 4 (index 4); keep indices 0 and 2 stationary.
        units[1].nav.position = Point { x: 5.5, y: 7.5 };
        units[3].nav.position = Point { x: 6.5, y: 4.5 };
        units[4].nav.position = Point { x: 12.5, y: 7.5 };
        let moved = [false, true, false, true, true];

        let mut post = vec![Vec::new(); units.len()];
        refresh_post_movement(&units, terrain, tick, &pre, &moved, &mut post);

        for (i, got) in post.iter().enumerate() {
            let mut expected = Vec::new();
            observe_all_into(&units, terrain, i, tick, &mut expected);
            assert_eq!(got, &expected, "mismatch for unit index {i}");
        }
    }
}
