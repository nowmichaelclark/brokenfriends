//! Ground scatter (S-108): where the rocks, the pebbles and the dead
//! shrubs stand.
//!
//! The scatter is a pure function of the zone and the settings. Each
//! class has a grid of cells. Each cell holds at most one prop. A hash
//! of the cell number and the zone seed gives the size, the place and
//! the chance of that prop. Thus the same seed always gives the same
//! props, and no state or random source is needed.
//!
//! The grid is global, and a chunk is a block of cells. The game asks
//! for one chunk at a time, near the camera (`chunk_instances`). Two
//! chunks never need each other: a prop stays inside its own cell, so
//! no prop crosses a chunk border, and no two props touch.
//!
//! Where props stand:
//! - **Rocks** like steep rock and the foot of slopes. They come in
//!   clumps. Dry lake beds have none.
//! - **Pebbles** are many and small. They like riverbeds, rocky ground
//!   and the clumps near them.
//! - **Dead shrubs** stand on gentle soil, in patches. They avoid rock,
//!   sand and lake beds.
//!
//! No prop stands in an exclusion circle: the start point (not for
//! pebbles) and the places of obstacles (R23).

use crate::detmath::smoothstep;
use crate::highlands::Zone;
use crate::noise::{gradient_noise, hash01};
use crate::obstacles::{Kind, Obstacle};
use crate::props::{PEBBLE_VARIANTS, ROCK_VARIANTS, SHRUB_VARIANTS};

/// The classes of props.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    Rock,
    Pebble,
    Shrub,
}

impl Class {
    /// All classes.
    pub const ALL: [Class; 3] = [Class::Rock, Class::Pebble, Class::Shrub];

    /// Gives the number of the class, from 0 to 2.
    pub fn index(self) -> usize {
        match self {
            Class::Rock => 0,
            Class::Pebble => 1,
            Class::Shrub => 2,
        }
    }

    /// Gives the name for logs.
    pub fn name(self) -> &'static str {
        ["rock", "pebble", "shrub"][self.index()]
    }

    /// Gives the number of mesh shapes of the class.
    pub fn variants(self) -> u8 {
        match self {
            Class::Rock => ROCK_VARIANTS,
            Class::Pebble => PEBBLE_VARIANTS,
            Class::Shrub => SHRUB_VARIANTS,
        }
    }

    /// A number that makes the random numbers of a class different from
    /// the ones of the other classes.
    fn salt(self) -> u32 {
        [0x1A2B_3C4D, 0x5E6F_7081, 0x92A3_B4C5][self.index()]
    }
}

/// The settings of one class.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassParams {
    /// The side of one cell in metres. One cell holds at most one prop.
    pub cell_m: f32,
    /// The number of cells on each side of a chunk.
    pub cells_per_chunk: i32,
    /// The chance that a cell has a prop, on average ground (0 to 1).
    pub chance: f32,
    /// The smallest size in metres. Rocks and pebbles: the width.
    /// Shrubs: the height.
    pub size_min_m: f32,
    /// The largest size in metres.
    pub size_max_m: f32,
}

/// The settings of the scatter.
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterParams {
    pub rock: ClassParams,
    pub pebble: ClassParams,
    pub shrub: ClassParams,
    /// Multiplies all chances. 0 makes no props.
    pub density: f32,
    /// No rock and no shrub stands nearer to the start point than this,
    /// in metres.
    pub clear_start_m: f32,
}

impl Default for ScatterParams {
    fn default() -> Self {
        Self {
            rock: ClassParams {
                cell_m: 5.0,
                cells_per_chunk: 32,
                chance: 0.35,
                size_min_m: 0.18,
                size_max_m: 3.2,
            },
            pebble: ClassParams {
                cell_m: 0.9,
                cells_per_chunk: 36,
                chance: 0.90,
                size_min_m: 0.025,
                size_max_m: 0.10,
            },
            shrub: ClassParams {
                cell_m: 4.0,
                cells_per_chunk: 16,
                chance: 0.22,
                size_min_m: 0.30,
                size_max_m: 1.10,
            },
            density: 1.0,
            clear_start_m: 6.0,
        }
    }
}

impl ScatterParams {
    /// Gives the settings of one class.
    pub fn class(&self, class: Class) -> &ClassParams {
        match class {
            Class::Rock => &self.rock,
            Class::Pebble => &self.pebble,
            Class::Shrub => &self.shrub,
        }
    }

    /// Gives the side of a chunk of this class in metres.
    pub fn chunk_m(&self, class: Class) -> f32 {
        let p = self.class(class);
        p.cell_m * p.cells_per_chunk as f32
    }
}

/// A circle where no prop stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exclusion {
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    /// If this is true, pebbles keep out of the circle too.
    pub pebbles: bool,
}

/// Gives the exclusion circles of a zone: the start point and the
/// obstacles. The result depends on the zone and the obstacles only.
pub fn exclusions(zone: &Zone, obstacles: &[Obstacle], params: &ScatterParams) -> Vec<Exclusion> {
    let (sx, sz) = zone.start_point();
    let mut out = vec![Exclusion {
        x: sx,
        z: sz,
        radius: params.clear_start_m,
        pebbles: false,
    }];
    for o in obstacles {
        let radius = match &o.kind {
            Kind::AshCrust { radius, .. } | Kind::SinkingSand { radius, .. } => radius + 2.0,
            Kind::VentPush { .. } | Kind::VentLaunch { .. } => 2.5,
            Kind::SteppingStones {
                half_length,
                half_width,
                ..
            } => half_length.max(*half_width) + 2.0,
            Kind::ThornSnag { radius } => radius + 1.5,
            Kind::RockTeeth { half_width, .. } => half_width + 1.5,
            Kind::FallingRocks { spread, radius, .. } => spread.max(*radius) + 1.0,
            // The wind, the falling stones and the warm air need open ground
            // only in a loose way. Props may stand there.
            Kind::Gust { .. } | Kind::RollingStones { .. } | Kind::Updraft { .. } => 0.0,
        };
        if radius > 0.0 {
            out.push(Exclusion {
                x: o.pos[0],
                z: o.pos[2],
                radius,
                pebbles: true,
            });
        }
    }
    out
}

/// One prop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    /// The place of the base of the prop (x, y, z) in metres.
    pub pos: [f32; 3],
    /// The turn around the Y axis in radians.
    pub yaw: f32,
    /// A small lean around the X axis and the Z axis in radians.
    pub tilt: [f32; 2],
    /// The size on each axis. The mesh is 1 m wide (1 m high for a
    /// shrub), so this is the size in metres.
    pub scale: [f32; 3],
    /// The mesh shape (`Class::variants`).
    pub variant: u8,
    /// The colour of the ground rock: 0 is rust and 1 is ash.
    pub tone: f32,
    /// How light the prop is, about 0.85 to 1.15.
    pub shade: f32,
}

/// No prop stands nearer to the border of the zone than this, in metres.
const EDGE_M: f32 = 8.0;

/// The most that the ground factors can multiply the chance.
const MAX_FACTOR: f32 = 2.2;

/// The slope (rise over run) where a class stops. 0.9 is 42 degrees.
fn slope_limit(class: Class) -> f32 {
    match class {
        Class::Rock => 0.9,
        Class::Pebble => 0.8,
        Class::Shrub => 0.55,
    }
}

/// The part of the height of a prop that is below the ground.
fn sink_part(class: Class) -> f32 {
    match class {
        Class::Rock => 0.18,
        Class::Pebble => 0.35,
        Class::Shrub => 0.0,
    }
}

/// The size of a clump of this class in metres.
fn clump_m(class: Class) -> f32 {
    match class {
        Class::Rock => 80.0,
        Class::Pebble => 7.0,
        Class::Shrub => 120.0,
    }
}

/// The random number `k` of a cell, from 0 to 1.
fn rand(gx: i32, gz: i32, seed: u32, k: u32) -> f32 {
    hash01(gx, gz, seed ^ k.wrapping_mul(0x9E37_79B9))
}

/// Gives the slope (rise over run) at a place.
fn slope_at(zone: &Zone, x: f32, z: f32) -> f32 {
    let h = |x, z| zone.heights.sample_world(x, z);
    let dx = (h(x + 1.0, z) - h(x - 1.0, z)) * 0.5;
    let dz = (h(x, z + 1.0) - h(x, z - 1.0)) * 0.5;
    (dx * dx + dz * dz).sqrt()
}

/// Gives the props of one chunk of one class. The chunk (`cx`, `cz`)
/// covers the world from `cx * chunk_m` to `(cx + 1) * chunk_m` on the X
/// axis, and the same on the Z axis. The result is in the order of the
/// cells, so it is the same each time.
pub fn chunk_instances(
    zone: &Zone,
    params: &ScatterParams,
    exclusions: &[Exclusion],
    class: Class,
    cx: i32,
    cz: i32,
) -> Vec<Instance> {
    let cp = params.class(class);
    let chance = cp.chance * params.density.max(0.0);
    let mut out = Vec::new();
    if chance <= 0.0 {
        return out;
    }
    let n = cp.cells_per_chunk;
    let cell = cp.cell_m;
    let chunk = cell * n as f32;
    let (x0, z0) = (cx as f32 * chunk, cz as f32 * chunk);
    let edge = zone.heights.world_size() * 0.5 - EDGE_M;
    if x0 > edge || z0 > edge || x0 + chunk < -edge || z0 + chunk < -edge {
        return out;
    }
    let seed = zone.seed ^ class.salt();
    let local: Vec<&Exclusion> = exclusions
        .iter()
        .filter(|e| {
            (class != Class::Pebble || e.pebbles)
                && e.x + e.radius >= x0 - 1.0
                && e.x - e.radius <= x0 + chunk + 1.0
                && e.z + e.radius >= z0 - 1.0
                && e.z - e.radius <= z0 + chunk + 1.0
        })
        .collect();
    let max_chance = chance * MAX_FACTOR;
    let clump_seed = seed ^ 0xC1C1_C1C1;
    let ground_h = |x: f32, z: f32| zone.heights.sample_world(x, z);

    for j in 0..n {
        for i in 0..n {
            let (gx, gz) = (cx * n + i, cz * n + j);
            // The first test uses no ground data. Most cells stop here.
            let chance_roll = rand(gx, gz, seed, 0);
            if chance_roll >= max_chance {
                continue;
            }

            // The size, from many small to few large (cubic).
            let u = rand(gx, gz, seed, 2);
            let size = cp.size_min_m + (cp.size_max_m - cp.size_min_m) * u * u * u;
            let (sx, sz, sy) = match class {
                Class::Rock => (
                    0.85 + 0.35 * rand(gx, gz, seed, 7),
                    0.85 + 0.35 * rand(gx, gz, seed, 8),
                    0.7 + 0.4 * rand(gx, gz, seed, 9),
                ),
                Class::Pebble => (
                    0.8 + 0.4 * rand(gx, gz, seed, 7),
                    0.8 + 0.4 * rand(gx, gz, seed, 8),
                    0.7 + 0.5 * rand(gx, gz, seed, 9),
                ),
                Class::Shrub => (
                    0.8 + 0.6 * rand(gx, gz, seed, 7),
                    0.8 + 0.6 * rand(gx, gz, seed, 7),
                    1.0,
                ),
            };
            // The half width of the footprint. The prop stays in its cell.
            let width = match class {
                Class::Shrub => size * 0.5 * sx.max(sz) * 1.2,
                _ => size * 0.5 * sx.max(sz),
            };
            let foot = width.min(cell * 0.49);
            let free = (cell - 2.0 * foot).max(0.0);
            let x = (gx as f32) * cell + foot + free * rand(gx, gz, seed, 1);
            let z = (gz as f32) * cell + foot + free * rand(gx, gz, seed, 3);
            if x.abs() > edge || z.abs() > edge {
                continue;
            }
            if local
                .iter()
                .any(|e| (x - e.x).powi(2) + (z - e.z).powi(2) < (e.radius + foot).powi(2))
            {
                continue;
            }

            let slope = slope_at(zone, x, z);
            if slope > slope_limit(class) {
                continue;
            }
            let ground = zone.ground_at(x, z);
            let clump = {
                let v = gradient_noise(x / clump_m(class), z / clump_m(class), clump_seed).value;
                smoothstep(-0.4, 0.7, v)
            };
            let factor = match class {
                Class::Rock => {
                    (0.35 + 0.9 * ground.rock)
                        * (0.25 + 1.5 * clump)
                        * (1.0 - 0.9 * ground.lake)
                        * (1.0 - 0.5 * ground.sand)
                }
                Class::Pebble => {
                    (0.5 + 1.2 * ground.sand)
                        * (0.6 + 0.8 * ground.rock)
                        * (0.2 + 1.6 * clump)
                        * (1.0 - 0.7 * ground.lake)
                }
                Class::Shrub => {
                    if ground.rock >= 0.5
                        || ground.sand >= 0.5
                        || ground.lake >= 0.5
                        || ground.water >= 0.5
                    {
                        continue;
                    }
                    (0.1 + 1.7 * clump) * (1.0 - ground.rock)
                }
            };
            if chance_roll >= chance * factor.min(MAX_FACTOR) {
                continue;
            }

            // A rock sits on the lowest ground under it: the uphill side
            // goes into the slope.
            let mut y = ground_h(x, z);
            if class == Class::Rock {
                for (dx, dz) in [(foot, 0.0), (-foot, 0.0), (0.0, foot), (0.0, -foot)] {
                    y = y.min(ground_h(x + dx, z + dz));
                }
            }
            let height = match class {
                Class::Shrub => size,
                _ => size * 0.8 * sy,
            };
            y -= sink_part(class) * height;

            let lean = match class {
                Class::Rock => 0.12,
                Class::Pebble => 0.3,
                Class::Shrub => 0.08,
            };
            let variant = ((rand(gx, gz, seed, 10) * class.variants() as f32) as u8)
                .min(class.variants() - 1);
            let tone = if class == Class::Pebble && (ground.sand >= 0.5 || ground.lake >= 0.5) {
                1.0
            } else {
                (ground.ash + 0.25 * (rand(gx, gz, seed, 11) - 0.5)).clamp(0.0, 1.0)
            };
            out.push(Instance {
                pos: [x, y, z],
                yaw: std::f32::consts::TAU * rand(gx, gz, seed, 4),
                tilt: [
                    (rand(gx, gz, seed, 5) * 2.0 - 1.0) * lean,
                    (rand(gx, gz, seed, 6) * 2.0 - 1.0) * lean,
                ],
                scale: match class {
                    Class::Shrub => [size * sx, size, size * sz],
                    _ => [size * sx, size * sy, size * sz],
                },
                variant,
                tone,
                shade: 0.85 + 0.3 * rand(gx, gz, seed, 12),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlands::{generate, HighlandsParams};
    use crate::obstacles::{place_obstacles, ObstacleParams};

    fn test_zone(seed: u32) -> Zone {
        generate(seed, 768, 1.0, &HighlandsParams::default())
    }

    /// All instances of one class in the whole zone.
    fn all(zone: &Zone, params: &ScatterParams, excl: &[Exclusion], class: Class) -> Vec<Instance> {
        let chunk = params.chunk_m(class);
        let k = (zone.heights.world_size() * 0.5 / chunk).ceil() as i32 + 1;
        let mut out = Vec::new();
        for cz in -k..=k {
            for cx in -k..=k {
                out.extend(chunk_instances(zone, params, excl, class, cx, cz));
            }
        }
        out
    }

    #[test]
    fn same_seed_same_props_and_a_new_seed_changes_them() {
        let p = ScatterParams::default();
        let a = test_zone(7);
        let b = test_zone(7);
        let c = test_zone(8);
        for class in Class::ALL {
            let ia = all(&a, &p, &[], class);
            assert!(!ia.is_empty(), "{} has no props", class.name());
            assert_eq!(ia, all(&b, &p, &[], class), "{}", class.name());
            assert_ne!(ia, all(&c, &p, &[], class), "{}", class.name());
        }
    }

    #[test]
    fn a_chunk_does_not_depend_on_the_other_chunks() {
        // The same chunk gives the same props, in any order of calls.
        let p = ScatterParams::default();
        let zone = test_zone(3);
        let first = chunk_instances(&zone, &p, &[], Class::Rock, 0, 0);
        let _ = chunk_instances(&zone, &p, &[], Class::Rock, 1, 0);
        let _ = chunk_instances(&zone, &p, &[], Class::Shrub, 0, 0);
        assert_eq!(first, chunk_instances(&zone, &p, &[], Class::Rock, 0, 0));
    }

    #[test]
    fn props_stay_in_their_chunk_and_their_cell() {
        let p = ScatterParams::default();
        let zone = test_zone(5);
        for class in Class::ALL {
            let chunk = p.chunk_m(class);
            let k = (384.0 / chunk).ceil() as i32 + 1;
            for cz in -k..=k {
                for cx in -k..=k {
                    for inst in chunk_instances(&zone, &p, &[], class, cx, cz) {
                        let [x, _, z] = inst.pos;
                        assert!(x >= cx as f32 * chunk && x <= (cx + 1) as f32 * chunk);
                        assert!(z >= cz as f32 * chunk && z <= (cz + 1) as f32 * chunk);
                    }
                }
            }
        }
    }

    #[test]
    fn the_numbers_are_plausible() {
        let p = ScatterParams::default();
        let zone = test_zone(7);
        let area = 768.0 * 768.0;
        let rocks = all(&zone, &p, &[], Class::Rock).len() as f32 / area;
        let pebbles = all(&zone, &p, &[], Class::Pebble).len() as f32 / area;
        let shrubs = all(&zone, &p, &[], Class::Shrub).len() as f32 / area;
        // Per square metre.
        assert!((0.0005..0.02).contains(&rocks), "rocks {rocks}");
        assert!((0.02..0.6).contains(&pebbles), "pebbles {pebbles}");
        assert!((0.0002..0.02).contains(&shrubs), "shrubs {shrubs}");
    }

    #[test]
    fn the_sizes_are_inside_the_limits() {
        let p = ScatterParams::default();
        let zone = test_zone(7);
        for class in Class::ALL {
            let cp = p.class(class);
            for inst in all(&zone, &p, &[], class) {
                let size = match class {
                    Class::Shrub => inst.scale[1],
                    _ => inst.scale[0].max(inst.scale[2]),
                };
                assert!(size >= cp.size_min_m * 0.8, "{}: {size}", class.name());
                assert!(size <= cp.size_max_m * 1.25, "{}: {size}", class.name());
                assert!((inst.variant as usize) < class.variants() as usize);
                assert!((0.0..=1.0).contains(&inst.tone));
            }
        }
    }

    #[test]
    fn no_two_rocks_touch() {
        let p = ScatterParams::default();
        let zone = test_zone(7);
        let rocks = all(&zone, &p, &[], Class::Rock);
        assert!(rocks.len() > 100);
        // Spatial buckets keep the test fast.
        let mut buckets = std::collections::HashMap::<(i32, i32), Vec<usize>>::new();
        for (i, r) in rocks.iter().enumerate() {
            buckets
                .entry(((r.pos[0] / 8.0) as i32, (r.pos[2] / 8.0) as i32))
                .or_default()
                .push(i);
        }
        let radius = |r: &Instance| 0.5 * r.scale[0].max(r.scale[2]);
        for (i, r) in rocks.iter().enumerate() {
            let (bx, bz) = ((r.pos[0] / 8.0) as i32, (r.pos[2] / 8.0) as i32);
            for dz in -1..=1 {
                for dx in -1..=1 {
                    for &j in buckets.get(&(bx + dx, bz + dz)).into_iter().flatten() {
                        if j <= i {
                            continue;
                        }
                        let o = &rocks[j];
                        let d =
                            ((r.pos[0] - o.pos[0]).powi(2) + (r.pos[2] - o.pos[2]).powi(2)).sqrt();
                        assert!(d >= radius(r) + radius(o) - 0.01, "{r:?} touches {o:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn exclusions_keep_props_away() {
        let p = ScatterParams::default();
        let zone = test_zone(7);
        let free = all(&zone, &p, &[], Class::Rock);
        let middle = free[free.len() / 2].pos;
        let circle = Exclusion {
            x: middle[0],
            z: middle[2],
            radius: 40.0,
            pebbles: true,
        };
        for class in Class::ALL {
            for inst in all(&zone, &p, &[circle], class) {
                let d =
                    ((inst.pos[0] - circle.x).powi(2) + (inst.pos[2] - circle.z).powi(2)).sqrt();
                assert!(d >= circle.radius, "{} at {d}", class.name());
            }
        }
        // A circle with `pebbles: false` lets pebbles stand in it.
        let soft = Exclusion {
            pebbles: false,
            ..circle
        };
        let inside = all(&zone, &p, &[soft], Class::Pebble)
            .iter()
            .filter(|i| {
                ((i.pos[0] - soft.x).powi(2) + (i.pos[2] - soft.z).powi(2)).sqrt() < soft.radius
            })
            .count();
        assert!(inside > 0);
    }

    #[test]
    fn the_start_point_and_the_obstacles_are_clear() {
        let p = ScatterParams::default();
        let mut zone = test_zone(7);
        let obstacles = place_obstacles(&mut zone, &ObstacleParams::default());
        let excl = exclusions(&zone, &obstacles, &p);
        assert!(excl.len() > 5, "{}", excl.len());
        let (sx, sz) = zone.start_point();
        for class in [Class::Rock, Class::Shrub] {
            for inst in all(&zone, &p, &excl, class) {
                let d = ((inst.pos[0] - sx).powi(2) + (inst.pos[2] - sz).powi(2)).sqrt();
                assert!(d >= p.clear_start_m, "{} near the start", class.name());
            }
        }
    }

    #[test]
    fn rocks_and_shrubs_follow_the_ground() {
        let p = ScatterParams::default();
        let zone = test_zone(7);
        for inst in all(&zone, &p, &[], Class::Rock) {
            assert!(slope_at(&zone, inst.pos[0], inst.pos[2]) <= slope_limit(Class::Rock) + 1e-4);
        }
        for inst in all(&zone, &p, &[], Class::Shrub) {
            let g = zone.ground_at(inst.pos[0], inst.pos[2]);
            assert!(g.rock < 0.5 && g.sand < 0.5 && g.lake < 0.5);
            // The foot is on the ground.
            let h = zone.heights.sample_world(inst.pos[0], inst.pos[2]);
            assert!((inst.pos[1] - h).abs() < 1e-3);
        }
    }

    #[test]
    fn density_zero_makes_nothing_and_more_density_makes_more() {
        let zone = test_zone(7);
        let none = ScatterParams {
            density: 0.0,
            ..ScatterParams::default()
        };
        let more = ScatterParams {
            density: 2.0,
            ..ScatterParams::default()
        };
        let base = ScatterParams::default();
        for class in Class::ALL {
            assert!(all(&zone, &none, &[], class).is_empty());
            let a = all(&zone, &base, &[], class).len();
            let b = all(&zone, &more, &[], class).len();
            assert!(b > a, "{}: {a} then {b}", class.name());
        }
    }

    #[test]
    fn a_chunk_is_fast() {
        // A rough guard: 3 chunks of pebbles must not take a second, even
        // in a slow test build.
        let p = ScatterParams::default();
        let zone = test_zone(7);
        let start = std::time::Instant::now();
        for cx in 0..3 {
            let _ = chunk_instances(&zone, &p, &[], Class::Pebble, cx, 0);
        }
        assert!(start.elapsed().as_secs_f32() < 1.0);
    }
}
