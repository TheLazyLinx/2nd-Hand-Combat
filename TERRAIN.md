# Terrain rules

Terrain is static during a battle and resolved by the Rust core. One horizontal world unit represents one metre for these calculations. The following values are initial gameplay tuning, not claims of physical realism.

## Ground and surfaces

Maps contain width × height surface cells and (width + 1) × (height + 1) elevations in row-major order. Elevations can be negative. Each cell's northwest-to-southeast diagonal defines two linear ground triangles, shared by movement and sight queries. Obstacles occupy rectangular cell footprints and extend a specified height above the ground throughout that footprint. Both low and full-height obstacles block movement.

| Surface | Movement multiplier | Spotting-distance multiplier | Vegetation height |
| --- | ---: | ---: | ---: |
| Open | 1.00 | 1 | 0 m |
| Road | 1.15 | 1 | 0 m |
| Mud | 0.60 | 1 | 0 m |
| Brush | 0.80 | 2 | 1.8 m |
| Woodland | 0.70 | 3 | 3 m |

All gameplay values live in Rust. The browser receives surface definitions and cell readouts through the catalog. Elevation shading and interpolated contour lines are presentation only. Cell inspection shows surface movement cost; a moving soldier's status includes its current movement effect.

## Sight, cover, and concealment

Eyes are 1.6 m above local ground. Terrain and solid obstacles block physical sight. Sight/shot segments split at grid and triangle boundaries; the core compares linear ray and ground heights exactly within each interval. Ground above the ray blocks it; touching an obstacle's top or side counts as blocked, including corner grazing. A small numerical tolerance keeps boundary handling stable.

Shots start at 1.4 m and test target heights of 0.4, 0.9, and 1.4 m. Hit probability is the rifle's normal probability multiplied by the fraction of unblocked rays. Successful hits retain normal weapon damage. Low obstacles are initially 0.9 m and full-height obstacles 2.4 m. Protection depends on direction and relative elevation; an elevated shooter may see over an obstacle. Full occlusion blocks shooting both ways. There is no peeking or stance system.

Spotting has a 14-unit maximum distance and budget. Distance inside vegetation counts at the surface's spotting multiplier only for the portion of the ray below the canopy. Within two units, solid geometry permitting, enemies are detectable. Firing removes the shooter's vegetation concealment for 40 simulation ticks (two seconds), without bypassing solid cover or the maximum sight range. No spotting random rolls are used.

Soldiers independently observe enemies and retain each enemy's last-known position for about ten seconds. Routing considers all those known threats; investigation follows the latest selected target's sighting. Unseen enemies never update remembered positions, and sightings are not shared. Acquiring or losing a target interrupts the usual decision interval. Combat rechecks visibility after movement. All shooters use the same pre-fire reveal state; firing reveal and simultaneous damage apply after those checks.

## Travel and doctrine

Base speed is 2 m/s. Grade is signed elevation change divided by horizontal distance within each traversed triangle. Uphill speed is multiplied by `1 / (1 + 2 × grade)`. Descent through 20% grade increases speed linearly to 1.15. From 20% to 60% descent the multiplier decreases linearly to 0.55. Grades above 60% are impassable in either direction. Surface and slope multipliers multiply, with an overall cap of 1.30.

Movement spends the fixed 0.05-second step across each encountered surface and triangle boundary. Routing uses these same travel times, eight neighbours, swept 0.25 m soldier clearance, deterministic ties, and no obstacle corner cutting. A direct route is used only when valid and no slower than the computed fastest route. Routes are cached until the destination, route preference, or known threats materially change.

- **Fastest** minimizes travel time, including terrain effects.
- **Protected** multiplies edge time by `1 + 2 × exposure`, using maximum exposure along the edge and across known threats. Exposure combines physical shot exposure and detectability. Hidden enemies supply no information.
- **Balanced** positioning seeks lower combined incoming exposure; **Cover** prioritizes physical shot protection; **Concealment** prioritizes greater spotting difficulty. Candidate firing positions must be reachable, nearby, within engagement distance, and permit detecting and shooting the target.

Engagement candidates are within three units. Withdrawal candidates are within five units and may completely break sight/fire; protected positions take priority, followed by safer allies and increased threat distance. Chosen goals are retained while still useful, avoiding oscillation as protection starts taking effect. A soldier whose last sighting is hidden by full cover can investigate a reachable opening to reacquire it.

## Presets and limits

Training Ground supplies a road, low obstacles, and a central wall with openings. Ridge and Hollow supplies a gradual ridge and a 2.2 m depression. Woodland Crossing supplies vegetation, mud, an exposed road, and defensive obstacles. All are 32 × 20 and allow up to eight infantry per faction.

Loading another setup replaces its map and placements. Battle reset restores its starting map, units, doctrine, and seed. The player sees the whole map; terrain status and directional protection describe the simulation's current state. [Map Maker](MAP_MAKER.md) authors maps and complete scenarios up to 128 × 128, including browser saving and JSON transfer. Procedural generation, terrain destruction, stamina, stance, and ballistic trajectory simulation remain outside this feature. Cross-platform deterministic replay remains unproven.
