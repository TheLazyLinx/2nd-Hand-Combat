//! Uniform CSR spatial grid for fast radius queries over living units.
//!
//! Invariant: `rebuild` stores only living units (`hp > 0.0`) packed into contiguous 32-byte
//! `SpatialEntry` records, with zero per-tick heap allocations after warm-up and conservative
//! cell padding so radius queries never miss boundary units compared to a full linear scan.

use crate::{types::Point, unit::Unit};

pub(crate) const CELL_SIZE: f64 = 4.0;
const INV_CELL_SIZE: f64 = 1.0 / CELL_SIZE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpatialEntry {
    index: u32,
    pub id: u32,
    pub faction: u32,
    pub revealed_until: u32,
    pub position: Point,
}

impl SpatialEntry {
    #[inline]
    pub(crate) fn index(self) -> usize {
        self.index as usize
    }
}

pub(crate) struct SpatialGrid {
    cols: usize,
    rows: usize,
    counts: Vec<u32>,
    cell_factions: Vec<u64>,
    offsets: Vec<u32>,
    cursors: Vec<u32>,
    unit_cells: Vec<u32>,
    entries: Vec<SpatialEntry>,
}

#[inline]
const fn faction_bit(faction: u32) -> u64 {
    if faction < 63 {
        1u64 << faction
    } else {
        1u64 << 63
    }
}

impl SpatialGrid {
    pub(crate) fn new(map_width: u32, map_height: u32) -> Self {
        let cols = ((map_width as f64) * INV_CELL_SIZE).ceil().max(1.0) as usize;
        let rows = ((map_height as f64) * INV_CELL_SIZE).ceil().max(1.0) as usize;
        let cell_count = cols * rows;
        Self {
            cols,
            rows,
            counts: vec![0; cell_count],
            cell_factions: vec![0; cell_count],
            offsets: vec![0; cell_count + 1],
            cursors: vec![0; cell_count],
            unit_cells: Vec::new(),
            entries: Vec::new(),
        }
    }

    #[inline]
    fn cell_index(&self, p: Point) -> usize {
        let cx = ((p.x * INV_CELL_SIZE).floor() as i32).clamp(0, self.cols as i32 - 1) as usize;
        let cy = ((p.y * INV_CELL_SIZE).floor() as i32).clamp(0, self.rows as i32 - 1) as usize;
        cy * self.cols + cx
    }

    /// Rebuilds the flat CSR grid for all living units (`hp > 0.0`).
    /// Preserves ascending unit-index order inside every cell.
    pub(crate) fn rebuild(&mut self, units: &[Unit]) {
        self.counts.fill(0);
        self.cell_factions.fill(0);
        self.unit_cells.clear();
        self.unit_cells.reserve(units.len());
        let mut living = 0usize;
        for u in units {
            if u.vitals.hp <= 0.0 {
                self.unit_cells.push(u32::MAX);
                continue;
            }
            let cell = self.cell_index(u.nav.position);
            self.counts[cell] += 1;
            self.cell_factions[cell] |= faction_bit(u.placement.faction);
            self.unit_cells.push(cell as u32);
            living += 1;
        }
        let mut sum = 0u32;
        for (i, &count) in self.counts.iter().enumerate() {
            self.offsets[i] = sum;
            self.cursors[i] = sum;
            sum += count;
        }
        self.offsets[self.counts.len()] = sum;
        self.entries.resize(
            living,
            SpatialEntry {
                index: 0,
                id: 0,
                faction: 0,
                revealed_until: 0,
                position: Point { x: 0.0, y: 0.0 },
            },
        );
        for (i, (u, &cell)) in units.iter().zip(self.unit_cells.iter()).enumerate() {
            if cell == u32::MAX {
                continue;
            }
            let cell = cell as usize;
            let dst = self.cursors[cell] as usize;
            self.cursors[cell] += 1;
            self.entries[dst] = SpatialEntry {
                index: i as u32,
                id: u.placement.id,
                faction: u.placement.faction,
                revealed_until: u.combat.revealed_until,
                position: u.nav.position,
            };
        }
    }

    #[inline]
    fn cell_bounds(&self, center: Point, radius: f64) -> Option<(usize, usize, usize, usize)> {
        let r = radius + 1e-6;
        let min_cx = ((center.x - r) * INV_CELL_SIZE).floor() as i32;
        let max_cx = ((center.x + r) * INV_CELL_SIZE).floor() as i32;
        let min_cy = ((center.y - r) * INV_CELL_SIZE).floor() as i32;
        let max_cy = ((center.y + r) * INV_CELL_SIZE).floor() as i32;
        if max_cx < 0 || min_cx >= self.cols as i32 || max_cy < 0 || min_cy >= self.rows as i32 {
            return None;
        }
        let x0 = min_cx.max(0) as usize;
        let x1 = max_cx.min(self.cols as i32 - 1) as usize;
        let y0 = min_cy.max(0) as usize;
        let y1 = max_cy.min(self.rows as i32 - 1) as usize;
        Some((x0, x1, y0, y1))
    }

    /// Visits every living unit in cells overlapping the circle `(center, radius)`.
    /// Callers must still check exact Euclidean distance if exact circle membership is required.
    #[inline]
    pub(crate) fn query_radius(&self, center: Point, radius: f64, mut f: impl FnMut(SpatialEntry)) {
        let Some((x0, x1, y0, y1)) = self.cell_bounds(center, radius) else {
            return;
        };
        for cy in y0..=y1 {
            let row = cy * self.cols;
            for cx in x0..=x1 {
                let cell = row + cx;
                let start = self.offsets[cell] as usize;
                let end = self.offsets[cell + 1] as usize;
                for &entry in &self.entries[start..end] {
                    f(entry);
                }
            }
        }
    }

    /// Visits every living unit in cells overlapping `(center, radius)` that may contain
    /// units hostile to `my_faction`, skipping purely friendly/empty cells in O(1).
    #[inline]
    pub(crate) fn query_hostile_radius(
        &self,
        center: Point,
        radius: f64,
        my_faction: u32,
        mut f: impl FnMut(SpatialEntry),
    ) {
        let Some((x0, x1, y0, y1)) = self.cell_bounds(center, radius) else {
            return;
        };
        let exclude = if my_faction < 63 {
            1u64 << my_faction
        } else {
            0
        };
        for cy in y0..=y1 {
            let row = cy * self.cols;
            for cx in x0..=x1 {
                let cell = row + cx;
                if (self.cell_factions[cell] & !exclude) == 0 {
                    continue;
                }
                let start = self.offsets[cell] as usize;
                let end = self.offsets[cell + 1] as usize;
                for &entry in &self.entries[start..end] {
                    f(entry);
                }
            }
        }
    }

    /// Visits every living unit in cells overlapping `(center, radius)` that may contain
    /// allies of `my_faction`, skipping cells with no matching faction bit.
    #[inline]
    pub(crate) fn query_ally_radius(
        &self,
        center: Point,
        radius: f64,
        my_faction: u32,
        mut f: impl FnMut(SpatialEntry),
    ) {
        let Some((x0, x1, y0, y1)) = self.cell_bounds(center, radius) else {
            return;
        };
        let bit = faction_bit(my_faction);
        for cy in y0..=y1 {
            let row = cy * self.cols;
            for cx in x0..=x1 {
                let cell = row + cx;
                if (self.cell_factions[cell] & bit) == 0 {
                    continue;
                }
                let start = self.offsets[cell] as usize;
                let end = self.offsets[cell + 1] as usize;
                for &entry in &self.entries[start..end] {
                    f(entry);
                }
            }
        }
    }

    /// Returns `true` if any living unit in cells overlapping `(center, radius)` matching
    /// `my_faction`'s bit satisfies `f`.
    #[inline]
    pub(crate) fn any_ally_in_radius(
        &self,
        center: Point,
        radius: f64,
        my_faction: u32,
        mut f: impl FnMut(SpatialEntry) -> bool,
    ) -> bool {
        let Some((x0, x1, y0, y1)) = self.cell_bounds(center, radius) else {
            return false;
        };
        let bit = faction_bit(my_faction);
        for cy in y0..=y1 {
            let row = cy * self.cols;
            for cx in x0..=x1 {
                let cell = row + cx;
                if (self.cell_factions[cell] & bit) == 0 {
                    continue;
                }
                let start = self.offsets[cell] as usize;
                let end = self.offsets[cell + 1] as usize;
                for &entry in &self.entries[start..end] {
                    if f(entry) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{definitions::catalog, types::UnitPlacement};

    fn unit(id: u32, faction: u32, x: f64, y: f64, alive: bool) -> Unit {
        let mut u = Unit::new(UnitPlacement {
            id,
            faction,
            definition_id: "infantry".into(),
            position: Point { x, y },
            doctrine: catalog().default_doctrine,
            loadout: None,
        });
        if !alive {
            u.vitals.hp = 0.0;
        }
        u
    }

    #[test]
    fn spatial_entry_fits_compact_32_bytes() {
        assert_eq!(std::mem::size_of::<SpatialEntry>(), 32);
    }

    #[test]
    fn spatial_grid_excludes_dead_units_and_hits_exact_boundaries() {
        let units = vec![
            unit(1, 0, 4.0, 4.0, true),   // exact cell edge
            unit(2, 1, 18.0, 4.0, true),  // exact distance 14.0 from unit 1
            unit(3, 1, 18.01, 4.0, true), // just outside 14.0
            unit(4, 1, 6.0, 4.0, false),  // dead unit inside range
        ];
        let mut grid = SpatialGrid::new(32, 20);
        grid.rebuild(&units);
        let mut found = Vec::new();
        grid.query_radius(units[0].nav.position, 14.0, |e| {
            if units[0].nav.position.distance(e.position) <= 14.0 {
                found.push(e.id);
            }
        });
        found.sort();
        assert_eq!(found, vec![1, 2]);
        assert!(!grid.any_ally_in_radius(units[0].nav.position, 14.0, 1, |e| e.id == 4));
    }

    #[test]
    fn spatial_grid_matches_linear_scan_over_500_units_and_rebuilds_cleanly() {
        let mut rng = 0x5EED_CAFE_u64;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng >> 11) as f64 / ((1_u64 << 53) as f64)
        };
        let mut units = Vec::with_capacity(500);
        for i in 0..500u32 {
            units.push(unit(
                i + 1,
                i % 3,
                0.5 + next() * 127.0,
                0.5 + next() * 127.0,
                !i.is_multiple_of(7),
            ));
        }
        let mut grid = SpatialGrid::new(128, 128);
        grid.rebuild(&units);

        for radius in [2.0, 4.0, 8.0, 14.0] {
            for probe in &units[..25] {
                let center = probe.nav.position;
                let mut got = Vec::new();
                grid.query_radius(center, radius, |e| {
                    if center.distance(e.position) <= radius {
                        got.push(e.index());
                    }
                });
                got.sort();
                let expected: Vec<usize> = units
                    .iter()
                    .enumerate()
                    .filter(|(_, u)| u.vitals.hp > 0.0 && center.distance(u.nav.position) <= radius)
                    .map(|(idx, _)| idx)
                    .collect();
                assert_eq!(got, expected, "mismatch at radius {radius}");
            }
        }

        // Move units and kill some, then rebuild and re-verify.
        for (i, u) in units.iter_mut().enumerate() {
            u.nav.position.x = (u.nav.position.x + 17.3) % 127.0 + 0.5;
            if i % 5 == 0 {
                u.vitals.hp = 0.0;
            }
        }
        grid.rebuild(&units);
        let center = Point { x: 64.0, y: 64.0 };
        let mut got = Vec::new();
        grid.query_radius(center, 14.0, |e| {
            if center.distance(e.position) <= 14.0 {
                got.push(e.index());
            }
        });
        got.sort();
        let expected: Vec<usize> = units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.vitals.hp > 0.0 && center.distance(u.nav.position) <= 14.0)
            .map(|(idx, _)| idx)
            .collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn faction_bitmask_handles_high_faction_ids_beyond_63() {
        let units = vec![
            unit(1, 0, 5.0, 5.0, true),
            unit(2, 63, 7.0, 5.0, true),
            unit(3, 100, 9.0, 5.0, true),
        ];
        let mut grid = SpatialGrid::new(32, 20);
        grid.rebuild(&units);

        let mut seen_by_0 = Vec::new();
        grid.query_hostile_radius(units[0].nav.position, 10.0, 0, |e| {
            if crate::types::is_hostile(0, e.faction) {
                seen_by_0.push(e.id);
            }
        });
        seen_by_0.sort();
        assert_eq!(seen_by_0, vec![2, 3]);

        let mut seen_by_63 = Vec::new();
        grid.query_hostile_radius(units[1].nav.position, 10.0, 63, |e| {
            if crate::types::is_hostile(63, e.faction) {
                seen_by_63.push(e.id);
            }
        });
        seen_by_63.sort();
        assert_eq!(seen_by_63, vec![1, 3]);
    }
}
