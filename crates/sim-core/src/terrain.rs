//! Static per-battle terrain cache for O(1) cell lookups and precomputed 8-neighbour movement.
//!
//! Built once per `Battle::new` (and via `without_neighbors` for scenario analysis). Invariant:
//! every query accelerated by `TerrainCache` must match the uncached reference oracles in
//! `oracles.rs` for obstacle occlusion, corner grazing, swept-radius clearance, and travel times.

use crate::{
    definitions::{surface_definition, SPEED},
    map,
    types::*,
};

/// 8-connected neighbor directions matching `navigation::Search` expansion order.
pub(crate) const DIRS: [(i32, i32); 8] = [
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
    (1, -1),
];

/// Maximum possible unit travel speed on any surface/slope (`1.3` speed multiplier cap * `SPEED`).
pub(crate) const MAX_SPEED: f64 = 1.3 * SPEED;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CellSurface {
    pub move_mult: f64,
    pub spot_mult: f64,
    pub veg_height: f64,
}

#[derive(Clone, Copy)]
pub(crate) struct Terrain<'a> {
    pub map: &'a ArenaMap,
    pub cache: &'a TerrainCache,
}

#[derive(Clone)]
pub(crate) struct TerrainCache {
    pub width: usize,
    pub height: usize,
    pub occupied: Vec<bool>,
    pub obstacle_height: Vec<f64>,
    pub cell_surfaces: Vec<CellSurface>,
    pub neighbor_time: Vec<[f64; 8]>,
    pub dir_offsets: [isize; 8],
    pub has_obstacles: bool,
    pub is_flat: bool,
    pub flat_elevation: f64,
    #[cfg(test)]
    source_obstacles: Vec<(u32, u32, u32, u32, f64)>,
    #[cfg(test)]
    source_surfaces: Vec<Surface>,
    #[cfg(test)]
    source_elevations: Vec<f64>,
}

impl TerrainCache {
    pub(crate) fn new(map: &ArenaMap) -> Self {
        let mut cache = Self::without_neighbors(map);
        cache.build_neighbor_times(map);
        cache
    }

    pub(crate) fn without_neighbors(map: &ArenaMap) -> Self {
        let width = map.width as usize;
        let height = map.height as usize;
        let len = width * height;
        let mut occupied = vec![false; len];
        let mut obstacle_height = vec![0.0_f64; len];
        let mut has_obstacles = false;

        for o in &map.obstacles {
            let x_end = o.x.saturating_add(o.width).min(map.width) as usize;
            let y_end = o.y.saturating_add(o.height).min(map.height) as usize;
            let x_start = (o.x as usize).min(width);
            let y_start = (o.y as usize).min(height);
            for y in y_start..y_end {
                let row = y * width;
                for x in x_start..x_end {
                    let idx = row + x;
                    occupied[idx] = true;
                    if o.height_metres > obstacle_height[idx] {
                        obstacle_height[idx] = o.height_metres;
                    }
                    has_obstacles = true;
                }
            }
        }

        let cell_surfaces: Vec<CellSurface> = map
            .surfaces
            .iter()
            .map(|&s| {
                let def = surface_definition(s);
                CellSurface {
                    move_mult: def.movement_multiplier,
                    spot_mult: def.spotting_multiplier,
                    veg_height: def.vegetation_height,
                }
            })
            .collect();

        let flat_elevation = map.elevations.first().copied().unwrap_or(0.0);
        let is_flat = map.elevations.iter().all(|&e| e == flat_elevation);

        let w = width as isize;
        let dir_offsets = [1, w, -1, -w, w + 1, w - 1, -w - 1, -w + 1];

        Self {
            width,
            height,
            occupied,
            obstacle_height,
            cell_surfaces,
            neighbor_time: Vec::new(),
            dir_offsets,
            has_obstacles,
            is_flat,
            flat_elevation,
            #[cfg(test)]
            source_obstacles: map
                .obstacles
                .iter()
                .map(|o| (o.x, o.y, o.width, o.height, o.height_metres))
                .collect(),
            #[cfg(test)]
            source_surfaces: map.surfaces.clone(),
            #[cfg(test)]
            source_elevations: map.elevations.clone(),
        }
    }

    fn build_neighbor_times(&mut self, map: &ArenaMap) {
        let width = self.width;
        let height = self.height;
        let len = width * height;
        let mut neighbor_time = vec![[f64::INFINITY; 8]; len];
        let terrain = Terrain { map, cache: self };

        for y in 0..height {
            for x in 0..width {
                let from_idx = y * width + x;
                if self.occupied[from_idx] {
                    continue;
                }
                let from = Point {
                    x: x as f64 + 0.5,
                    y: y as f64 + 0.5,
                };
                for (dir, &(dx, dy)) in DIRS.iter().enumerate() {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let nxu = nx as usize;
                    let nyu = ny as usize;
                    let to_idx = nyu * width + nxu;
                    if self.occupied[to_idx] {
                        continue;
                    }
                    // Diagonal sweeps pass through the shared corner, requiring both adjacent cells open.
                    if dx != 0
                        && dy != 0
                        && (self.occupied[y * width + nxu] || self.occupied[nyu * width + x])
                    {
                        continue;
                    }
                    let to = Point {
                        x: nxu as f64 + 0.5,
                        y: nyu as f64 + 0.5,
                    };
                    if let Some(time) = map::segment_travel_time(terrain, from, to) {
                        neighbor_time[from_idx][dir] = time;
                    }
                }
            }
        }
        self.neighbor_time = neighbor_time;
    }

    #[inline]
    pub(crate) fn view<'a>(&'a self, map: &'a ArenaMap) -> Terrain<'a> {
        Terrain { map, cache: self }
    }

    #[inline]
    pub(crate) fn cell_surface(&self, p: Point) -> CellSurface {
        let x = (p.x.floor() as usize).min(self.width - 1);
        let y = (p.y.floor() as usize).min(self.height - 1);
        self.cell_surfaces[y * self.width + x]
    }

    #[inline]
    pub(crate) fn ground_height(&self, map: &ArenaMap, p: Point) -> f64 {
        if self.is_flat {
            return self.flat_elevation;
        }
        map::ground_height(map, p)
    }

    #[inline]
    pub(crate) fn neighbor_times(&self, index: usize) -> &[f64; 8] {
        &self.neighbor_time[index]
    }

    #[cfg(test)]
    pub(crate) fn sync_if_mutated(&mut self, map: &ArenaMap) {
        let same_obstacles = self.source_obstacles.len() == map.obstacles.len()
            && self
                .source_obstacles
                .iter()
                .zip(&map.obstacles)
                .all(|(&(x, y, w, h, hm), o)| {
                    x == o.x && y == o.y && w == o.width && h == o.height && hm == o.height_metres
                });
        if self.width != map.width as usize
            || self.height != map.height as usize
            || !same_obstacles
            || self.source_surfaces != map.surfaces
            || self.source_elevations != map.elevations
        {
            *self = Self::new(map);
        }
    }
}
