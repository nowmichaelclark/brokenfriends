//! Obstacles (R23): where they are, and what they are.
//!
//! An obstacle is data: a type, a place, a size, a rhythm and a
//! difficulty. This module finds good places on a finished zone and
//! selects the obstacles with the seed. The same seed always gives the
//! same obstacles. The game (`bf-game`) makes them move.
//!
//! The four natural obstacles of this module:
//! - **Ash crust:** thin ground over a pit, on flat ground.
//! - **Rolling stones:** stones that roll down a gully.
//! - **Gust:** wind along a narrow pass.
//! - **Vent:** a hole that blows at intervals. A push vent blows to the
//!   side. A launch vent at the foot of a steep face throws the bear up
//!   to the ledge above.

use crate::detmath::{sin_cos, smoothstep};
use crate::drainage::flow;
use crate::highlands::Zone;
use crate::noise::{gradient_noise, hash01, hash2};

/// The original ground above the pit of an ash crust: a square grid of
/// heights from before the pit was dug.
#[derive(Clone, Debug, PartialEq)]
pub struct Lid {
    /// The world position (x, z) of the first height.
    pub corner: [f32; 2],
    /// The number of heights on each side.
    pub n: usize,
    /// The distance between two heights in metres.
    pub spacing: f32,
    /// The heights in metres, in rows, `n * n` values.
    pub heights: Vec<f32>,
}

impl Lid {
    /// Gives the height of the lid at a world position (bilinear filter).
    pub fn height(&self, wx: f32, wz: f32) -> f32 {
        let max = (self.n - 1) as f32;
        let fx = ((wx - self.corner[0]) / self.spacing).clamp(0.0, max);
        let fz = ((wz - self.corner[1]) / self.spacing).clamp(0.0, max);
        let (x0, z0) = ((fx as usize).min(self.n - 2), (fz as usize).min(self.n - 2));
        let (tx, tz) = (fx - x0 as f32, fz - z0 as f32);
        let at = |x: usize, z: usize| self.heights[z * self.n + x];
        let top = at(x0, z0) + (at(x0 + 1, z0) - at(x0, z0)) * tx;
        let bottom = at(x0, z0 + 1) + (at(x0 + 1, z0 + 1) - at(x0, z0 + 1)) * tx;
        top + (bottom - top) * tz
    }
}

/// The type of an obstacle, with the sizes of that type.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// Thin ground over a pit. `radius` is the radius of the pit and
    /// `depth` is its depth, in metres.
    AshCrust { radius: f32, depth: f32, lid: Lid },
    /// Stones that roll down a gully from the place of the obstacle.
    /// `radius` is the radius of a stone. `count` stones come in each
    /// wave. `run_m` is the length of the path.
    RollingStones { radius: f32, count: u32, run_m: f32 },
    /// Wind along a pass. The area is a box around the place:
    /// `half_length` along the wind and `half_width` across it.
    Gust { half_length: f32, half_width: f32 },
    /// A vent that blows to the side, `range` metres far.
    VentPush { range: f32 },
    /// A vent at the foot of a steep face. The ledge is `rise` metres
    /// above the vent and `reach` metres away along the direction.
    VentLaunch { rise: f32, reach: f32 },
}

/// The names of all obstacle types, in the order of `Kind::index`.
pub const KIND_NAMES: [&str; 5] = [
    "ash_crust",
    "rolling_stones",
    "gust",
    "vent_push",
    "vent_launch",
];

impl Kind {
    /// Gives the number of the type, from 0 to 4.
    pub fn index(&self) -> usize {
        match self {
            Kind::AshCrust { .. } => 0,
            Kind::RollingStones { .. } => 1,
            Kind::Gust { .. } => 2,
            Kind::VentPush { .. } => 3,
            Kind::VentLaunch { .. } => 4,
        }
    }

    /// Gives the name of the type for logs, for example "ash_crust".
    pub fn name(&self) -> &'static str {
        KIND_NAMES[self.index()]
    }
}

/// One obstacle.
#[derive(Clone, Debug, PartialEq)]
pub struct Obstacle {
    /// The number of the obstacle in its zone.
    pub id: u32,
    pub kind: Kind,
    /// The place on the ground (x, y, z) in metres. For an ash crust,
    /// `y` is the height of the lid, not of the pit.
    pub pos: [f32; 3],
    /// A unit direction on the ground plane (x, z). Stones: downhill.
    /// Gust: the wind. Push vent: the jet. Launch vent: to the ledge.
    pub dir: [f32; 2],
    /// The rhythm: the time of one cycle in seconds.
    pub period: f32,
    /// The start of the cycle in seconds, from 0 to `period`.
    pub phase: f32,
    /// From 0 (easy) to 1 (hard). It grows with the distance from the
    /// start point.
    pub difficulty: f32,
}

/// The settings of the obstacle placement.
#[derive(Clone, Debug, PartialEq)]
pub struct ObstacleParams {
    /// Multiplies the number of obstacles. 0 makes none.
    pub density: f32,
    /// The largest number of each type, in the order of `KIND_NAMES`,
    /// at density 1.
    pub max_count: [usize; 5],
    /// The smallest distance between two obstacles of the same type, in
    /// metres, in the order of `KIND_NAMES`.
    pub same_gap_m: [f32; 5],
    /// The smallest distance between two obstacles of any type, in metres.
    pub any_gap_m: f32,
    /// No obstacle is nearer to the start point than this, in metres.
    pub clear_start_m: f32,
    /// At this distance from the start point, the density is full.
    pub full_density_m: f32,
    /// The part of the density just outside `clear_start_m` (0 to 1).
    pub near_density: f32,
    /// The first obstacle of each type is the nearest place that is
    /// between `clear_start_m` and this distance. It is easy.
    pub first_within_m: f32,
    /// At this distance from the start point, the difficulty is full.
    pub hard_at_m: f32,
    /// No obstacle is nearer to the zone border than this, in metres.
    pub border_m: f32,
}

impl Default for ObstacleParams {
    fn default() -> Self {
        Self {
            density: 1.0,
            max_count: [50, 36, 28, 70, 40],
            same_gap_m: [45.0, 60.0, 70.0, 9.0, 45.0],
            any_gap_m: 14.0,
            clear_start_m: 18.0,
            full_density_m: 350.0,
            near_density: 0.35,
            first_within_m: 260.0,
            hard_at_m: 800.0,
            border_m: 40.0,
        }
    }
}

/// A possible place for an obstacle.
#[derive(Clone, Debug)]
struct Site {
    /// A key that is different for each place. It makes the random order.
    key: (i32, i32),
    x: f32,
    z: f32,
    dir: [f32; 2],
    /// Two sizes. Their meaning depends on the type.
    a: f32,
    b: f32,
}

/// Reads the finished zone for the place search.
struct Land<'a> {
    zone: &'a Zone,
    half: f32,
    spacing: f32,
}

impl Land<'_> {
    fn h(&self, x: f32, z: f32) -> f32 {
        self.zone.heights.sample_world(x, z)
    }

    fn index(&self, x: f32, z: f32) -> usize {
        let size = self.zone.heights.size;
        let i = |w: f32| ((w / self.spacing + self.half).round().max(0.0) as usize).min(size - 1);
        i(z) * size + i(x)
    }

    fn sand(&self, x: f32, z: f32) -> bool {
        self.zone.riverbed[self.index(x, z)] > 60
    }

    fn lake(&self, x: f32, z: f32) -> bool {
        self.zone.lakebed[self.index(x, z)] > 60
    }

    /// Tells if the ground around a place is flat: the height changes
    /// less than `limit` metres at `radius` metres in 8 directions.
    fn flat(&self, x: f32, z: f32, radius: f32, limit: f32) -> bool {
        let h0 = self.h(x, z);
        DIRS8.iter().all(|d| {
            (self.h(x + d[0] * radius, z + d[1] * radius) - h0).abs() <= limit
                && (self.h(x + d[0] * radius * 0.5, z + d[1] * radius * 0.5) - h0).abs() <= limit
        })
    }
}

const S: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// 8 unit directions on the ground plane.
const DIRS8: [[f32; 2]; 8] = [
    [1.0, 0.0],
    [S, S],
    [0.0, 1.0],
    [-S, S],
    [-1.0, 0.0],
    [-S, -S],
    [0.0, -1.0],
    [S, -S],
];

/// A random number from 0 to 1 for an obstacle. `salt` selects which one.
fn rand(key: (i32, i32), seed: u32, salt: u32) -> f32 {
    hash01(key.0, key.1, seed ^ salt.wrapping_mul(0x9E37_79B9))
}

impl Zone {
    /// Gives the start point of the zone (x, z): the flattest place on a
    /// grid of 20 m within 200 m of the centre. Of two flat places, the
    /// nearer one wins.
    pub fn start_point(&self) -> (f32, f32) {
        let h = |x: f32, z: f32| self.heights.sample_world(x, z);
        let mut best = (f32::MAX, 0.0, 0.0);
        for iz in -10..=10 {
            for ix in -10..=10 {
                let (x, z) = (ix as f32 * 20.0, iz as f32 * 20.0);
                let h0 = h(x, z);
                let rise = (h(x + 2.0, z) - h0).abs() + (h(x, z + 2.0) - h0).abs();
                let score = rise + (x.abs() + z.abs()) * 0.0005;
                if score < best.0 {
                    best = (score, x, z);
                }
            }
        }
        (best.1, best.2)
    }
}

/// Finds the places for all obstacle types. The result has one list for
/// each type, in the order of `KIND_NAMES`.
fn find_sites(zone: &Zone, params: &ObstacleParams) -> [Vec<Site>; 5] {
    let size = zone.heights.size;
    let spacing = zone.heights.spacing;
    let half = size as f32 * 0.5;
    let land = Land {
        zone,
        half,
        spacing,
    };
    let seed = zone.seed;

    // A coarse grid with cells of about 4 m.
    let step = ((4.0 / spacing).round() as usize).max(1);
    let cs = size / step;
    let cell = step as f32 * spacing;
    let world = |c: usize| ((c * step) as f32 - half) * spacing;
    let coarse: Vec<f32> = (0..cs * cs)
        .map(|i| zone.heights.data[(i / cs) * step * size + (i % cs) * step])
        .collect();
    let ch = |x: isize, z: isize| {
        let max = cs as isize - 1;
        coarse[z.clamp(0, max) as usize * cs + x.clamp(0, max) as usize]
    };
    let margin = ((params.border_m / cell).ceil() as usize).max(4);
    let mut sites: [Vec<Site>; 5] = Default::default();
    if cs < margin * 2 + 8 {
        return sites;
    }
    let water = flow(&coarse, cs);

    for cz in margin..cs - margin {
        for cx in margin..cs - margin {
            let key = (cx as i32, cz as i32);
            let (x, z) = (world(cx), world(cz));
            let (xi, zi) = (cx as isize, cz as isize);
            let h0 = coarse[cz * cs + cx];
            let on_sand = land.sand(x, z);

            // Ash crust: flat ground that is not riverbed sand.
            if !on_sand && land.flat(x, z, 4.4, 0.32) {
                sites[0].push(Site {
                    key,
                    x,
                    z,
                    dir: [1.0, 0.0],
                    a: 2.2 + 1.2 * rand(key, seed, 11),
                    b: 0.0,
                });
            }

            // Rolling stones: a gully with enough slope for about 50 m.
            let area = water.area[cz * cs + cx] * cell * cell;
            if (600.0..20_000.0).contains(&area) && !on_sand {
                if let Some(site) = stone_site(&land, &water.receiver, &coarse, cs, cx, cz, &world)
                {
                    sites[1].push(Site { key, ..site });
                }
            }

            // Gust: a pass. The ground is higher on two opposite sides
            // and about level along the other axis.
            let reach = (12.0 / cell).round().max(1.0) as isize;
            let mut best: Option<(f32, [f32; 2])> = None;
            for d in &DIRS8[..4] {
                let (dx, dz) = (d[0].round() as isize, d[1].round() as isize);
                let (px, pz) = (-dz, dx);
                let left = ch(xi + px * reach, zi + pz * reach) - h0;
                let right = ch(xi - px * reach, zi - pz * reach) - h0;
                let ahead = (ch(xi + dx * reach, zi + dz * reach) - h0).abs();
                let behind = (ch(xi - dx * reach, zi - dz * reach) - h0).abs();
                let walls = left.min(right);
                let along = ahead.max(behind);
                if walls >= 4.0 && along <= 3.5 && best.is_none_or(|b| walls - along > b.0) {
                    best = Some((walls - along, *d));
                }
            }
            if let Some((_, d)) = best {
                // The wind blows one way or the other way.
                let sign = if rand(key, seed, 21) < 0.5 { 1.0 } else { -1.0 };
                let dir = [d[0] * sign, d[1] * sign];
                let across = [-dir[1], dir[0]];
                let mut width = 0.0;
                for side in [1.0f32, -1.0] {
                    let mut w = 1.0;
                    while w < 9.0
                        && land.h(x + across[0] * side * w, z + across[1] * side * w) < h0 + 1.2
                    {
                        w += 1.0;
                    }
                    width += w * 0.5;
                }
                sites[2].push(Site {
                    key,
                    x,
                    z,
                    dir,
                    a: 7.0 + 5.0 * rand(key, seed, 22),
                    b: width.clamp(3.0, 9.0),
                });
            }

            // Vents are on hot rock. A slow noise field tells where the
            // rock is hot.
            let hot = gradient_noise(x / 160.0, z / 160.0, seed ^ 0x5EED_0A57).value;
            if hot > 0.18 && !on_sand {
                let jx = x + (rand(key, seed, 31) - 0.5) * (cell - 1.0);
                let jz = z + (rand(key, seed, 32) - 0.5) * (cell - 1.0);
                if land.flat(jx, jz, 1.5, 0.55) && !land.sand(jx, jz) {
                    let (sin, cos) = sin_cos(rand(key, seed, 33) * std::f32::consts::TAU);
                    sites[3].push(Site {
                        key,
                        x: jx,
                        z: jz,
                        dir: [cos, sin],
                        a: 1.3 + 0.6 * rand(key, seed, 34),
                        b: 0.0,
                    });
                }
            }

            // Launch vent: flat ground at the foot of a steep face with
            // a ledge on top.
            if !land.lake(x, z) && land.flat(x, z, 1.5, 0.55) {
                let first = (hash2(key.0, key.1, seed ^ 0x1A0C) % 8) as usize;
                for k in 0..8 {
                    let d = DIRS8[(first + k) % 8];
                    if let Some(site) = launch_site(&land, x, z, d) {
                        sites[4].push(Site { key, ..site });
                        break;
                    }
                }
            }
        }
    }
    sites
}

/// Looks for a gully that starts at a coarse cell. Follows the water for
/// 12 cells and checks the slope and the shape.
fn stone_site(
    land: &Land,
    receiver: &[u32],
    coarse: &[f32],
    cs: usize,
    cx: usize,
    cz: usize,
    world: &impl Fn(usize) -> f32,
) -> Option<Site> {
    const STEPS: usize = 12;
    let start = cz * cs + cx;
    let mut at = start;
    let mut length = 0.0;
    let cell = world(1) - world(0);
    let mut middle = start;
    for i in 0..STEPS {
        let next = receiver[at] as usize;
        if next == at {
            return None; // a sink or the border
        }
        let (ax, az) = ((at % cs) as f32, (at / cs) as f32);
        let (nx, nz) = ((next % cs) as f32, (next / cs) as f32);
        length += ((nx - ax).powi(2) + (nz - az).powi(2)).sqrt() * cell;
        at = next;
        if land.sand(world(at % cs), world(at / cs)) {
            return None;
        }
        if i == STEPS / 2 {
            middle = at;
        }
    }
    let slope = (coarse[start] - coarse[at]) / length;
    if !(0.27..=0.80).contains(&slope) {
        return None;
    }
    let (sx, sz) = (world(cx), world(cz));
    let (ex, ez) = (world(at % cs), world(at / cs));
    let (dx, dz) = (ex - sx, ez - sz);
    let straight = (dx * dx + dz * dz).sqrt();
    if straight < length * 0.6 {
        return None; // the path turns back
    }
    let dir = [dx / straight, dz / straight];
    // The first metres must go down, or the stones do not start.
    if land.h(sx, sz) - land.h(sx + dir[0] * 4.0, sz + dir[1] * 4.0) < 0.8 {
        return None;
    }
    // The middle of the path is a channel: both sides are higher.
    let (mx, mz) = (world(middle % cs), world(middle / cs));
    let mh = land.h(mx, mz);
    let side = |s: f32| land.h(mx - dir[1] * s, mz + dir[0] * s) - mh;
    if side(8.0) < 0.8 || side(-8.0) < 0.8 {
        return None;
    }
    Some(Site {
        key: (0, 0),
        x: sx,
        z: sz,
        dir,
        a: length,
        b: 0.0,
    })
}

/// Looks for a steep face with a ledge on top, from a flat place in one
/// direction. The vent goes to the foot of the face.
fn launch_site(land: &Land, x: f32, z: f32, d: [f32; 2]) -> Option<Site> {
    let h0 = land.h(x, z);
    let rel = |s: f32| land.h(x + d[0] * s, z + d[1] * s) - h0;
    // The foot: the last place before the ground goes up.
    let mut foot = 0.0;
    while foot < 4.5 && rel(foot + 0.5) <= 0.25 {
        foot += 0.5;
    }
    if foot >= 4.5 {
        return None;
    }
    let base = rel(foot);
    let mut u = 1.0;
    while u <= 8.0 {
        let rise = rel(foot + u) - base;
        if rise > 9.0 {
            return None;
        }
        // The top is a place to stand: less than about 35 degrees.
        let after = |more: f32| (rel(foot + u + more) - base - rise).abs();
        let top_flat = after(0.75) <= 0.5 && after(1.5) <= 1.0;
        if rise >= 1.8 && top_flat {
            // The face must be a climb: about 48 degrees or more.
            if rise / u < 1.1 {
                return None;
            }
            let (vx, vz) = (x + d[0] * (foot - 0.2), z + d[1] * (foot - 0.2));
            if land.sand(vx, vz) {
                return None;
            }
            return Some(Site {
                key: (0, 0),
                x: vx,
                z: vz,
                dir: d,
                a: rise,
                b: u + 0.7,
            });
        }
        u += 0.5;
    }
    None
}

/// Finds the places, selects the obstacles and digs the pits of the ash
/// crusts into the heights of the zone.
///
/// Call this one time, after `generate`. The same zone and the same
/// settings always give the same obstacles.
pub fn place_obstacles(zone: &mut Zone, params: &ObstacleParams) -> Vec<Obstacle> {
    if params.density <= 0.0 {
        return Vec::new();
    }
    let seed = zone.seed;
    let (start_x, start_z) = zone.start_point();
    let from_start = |s: &Site| ((s.x - start_x).powi(2) + (s.z - start_z).powi(2)).sqrt();
    let mut sites = find_sites(zone, params);

    // (type, place, easy)
    let mut chosen: Vec<(usize, Site, bool)> = Vec::new();
    let fits = |chosen: &[(usize, Site, bool)], kind: usize, s: &Site| {
        chosen.iter().all(|(k, c, _)| {
            let gap = if *k == kind {
                params.same_gap_m[kind].max(params.any_gap_m)
            } else {
                params.any_gap_m
            };
            (c.x - s.x).powi(2) + (c.z - s.z).powi(2) >= gap * gap
        })
    };

    // The first obstacle of each type: the nearest place to the start.
    for (kind, list) in sites.iter().enumerate() {
        let mut nearest: Option<(f32, &Site)> = None;
        for s in list {
            let d = from_start(s);
            if d >= params.clear_start_m.max(25.0)
                && d <= params.first_within_m
                && nearest.is_none_or(|n| d < n.0)
                && fits(&chosen, kind, s)
            {
                nearest = Some((d, s));
            }
        }
        if let Some((_, s)) = nearest {
            chosen.push((kind, s.clone(), true));
        }
    }

    // The other obstacles, in a random order that the seed gives. Fewer
    // near the start, more further away.
    for (kind, list) in sites.iter_mut().enumerate() {
        let salt = 0xB0B0_0000 + kind as u32;
        list.sort_by_key(|s| (hash2(s.key.0, s.key.1, seed ^ salt), s.key));
        let max = (params.max_count[kind] as f32 * params.density).round() as usize;
        let mut count = chosen.iter().filter(|c| c.0 == kind).count();
        for s in list.iter() {
            if count >= max {
                break;
            }
            let d = from_start(s);
            if d < params.clear_start_m {
                continue;
            }
            let ramp = smoothstep(params.clear_start_m, params.full_density_m, d);
            let keep = params.near_density + (1.0 - params.near_density) * ramp;
            if rand(s.key, seed, 41 + kind as u32) >= keep {
                continue;
            }
            if fits(&chosen, kind, s) {
                chosen.push((kind, s.clone(), false));
                count += 1;
            }
        }
    }

    // Make the obstacles. Dig the pits last: the search read the zone
    // without pits.
    chosen.sort_by_key(|(kind, s, _)| (*kind, s.key.1, s.key.0));
    let mut out = Vec::with_capacity(chosen.len());
    for (id, (kind, s, easy)) in chosen.into_iter().enumerate() {
        let r = |salt: u32| rand(s.key, seed, salt + 100 * (kind as u32 + 1));
        let far = smoothstep(40.0, params.hard_at_m, from_start(&s));
        let difficulty = if easy {
            0.0
        } else {
            (far * 0.85 + (r(1) - 0.5) * 0.3).clamp(0.0, 1.0)
        };
        let (kind, period, pos) = match kind {
            0 => {
                let (lid, pos) = dig_pit(zone, &s, 0.9 + 0.7 * difficulty);
                let kind = Kind::AshCrust {
                    radius: s.a,
                    depth: 0.9 + 0.7 * difficulty,
                    lid,
                };
                (kind, 1.0, pos)
            }
            1 => (
                Kind::RollingStones {
                    radius: 0.14 + 0.16 * r(2),
                    count: 1 + (difficulty * 2.99) as u32,
                    run_m: s.a,
                },
                12.0 - 4.0 * difficulty + 2.0 * r(3),
                [s.x, zone.heights.sample_world(s.x, s.z), s.z],
            ),
            2 => (
                Kind::Gust {
                    half_length: s.a,
                    half_width: s.b,
                },
                7.0 + 2.5 * r(3),
                [s.x, zone.heights.sample_world(s.x, s.z), s.z],
            ),
            3 => (
                Kind::VentPush { range: s.a },
                5.5 - 1.5 * difficulty + 1.5 * r(3),
                [s.x, zone.heights.sample_world(s.x, s.z), s.z],
            ),
            _ => (
                Kind::VentLaunch {
                    rise: s.a,
                    reach: s.b,
                },
                4.0 + 1.5 * r(3),
                [s.x, zone.heights.sample_world(s.x, s.z), s.z],
            ),
        };
        out.push(Obstacle {
            id: id as u32,
            kind,
            pos,
            dir: s.dir,
            period,
            phase: r(4) * period,
            difficulty,
        });
    }
    out
}

/// Digs the pit of an ash crust into the zone. Gives the lid (the
/// heights from before) and the place of the crust. The place is on a
/// sample of the heightmap.
fn dig_pit(zone: &mut Zone, s: &Site, depth: f32) -> (Lid, [f32; 3]) {
    let map = &mut zone.heights;
    let (size, spacing) = (map.size, map.spacing);
    let half = size as f32 * 0.5;
    let radius = s.a;
    let sample = |w: f32| (w / spacing + half).round() as isize;
    let (cx, cz) = (sample(s.x), sample(s.z));
    let reach = (radius / spacing).ceil() as isize + 2;
    let n = (reach * 2 + 1) as usize;
    let mut heights = Vec::with_capacity(n * n);
    for dz in -reach..=reach {
        for dx in -reach..=reach {
            heights.push(map.get(cx + dx, cz + dz));
        }
    }
    let centre_height = map.get(cx, cz);
    for dz in -reach..=reach {
        for dx in -reach..=reach {
            let (x, z) = (cx + dx, cz + dz);
            if x < 0 || z < 0 || x >= size as isize || z >= size as isize {
                continue;
            }
            let dist = ((dx * dx + dz * dz) as f32).sqrt() * spacing;
            // Full depth in the middle. The rim keeps its height.
            let part = 1.0 - smoothstep(radius - 1.6, radius - 0.2, dist);
            map.data[z as usize * size + x as usize] -= depth * part;
        }
    }
    let world = |i: isize| (i as f32 - half) * spacing;
    let lid = Lid {
        corner: [world(cx - reach), world(cz - reach)],
        n,
        spacing,
        heights,
    };
    (lid, [world(cx), centre_height, world(cz)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlands::{generate, HighlandsParams};
    use crate::HeightMap;

    /// Makes a zone from a height function of the world position.
    fn zone_from(size: usize, f: impl Fn(f32, f32) -> f32 + Sync) -> Zone {
        let half = size as f32 * 0.5;
        let heights = HeightMap::from_fn(size, 1.0, |x, z| f(x as f32 - half, z as f32 - half));
        Zone {
            seed: 3,
            heights,
            riverbed: vec![0; size * size],
            lakebed: vec![0; size * size],
        }
    }

    fn count(list: &[Obstacle], name: &str) -> usize {
        list.iter().filter(|o| o.kind.name() == name).count()
    }

    #[test]
    fn flat_ground_gets_only_crusts_and_push_vents() {
        let mut zone = zone_from(512, |_, _| 10.0);
        let list = place_obstacles(&mut zone, &ObstacleParams::default());
        assert!(count(&list, "ash_crust") > 5);
        assert_eq!(count(&list, "rolling_stones"), 0);
        assert_eq!(count(&list, "gust"), 0);
        assert_eq!(count(&list, "vent_launch"), 0);
    }

    #[test]
    fn a_crust_has_a_pit_below_its_lid() {
        let mut zone = zone_from(512, |x, _| 10.0 + x * 0.01);
        let before = zone.heights.clone();
        let list = place_obstacles(&mut zone, &ObstacleParams::default());
        let crust = list
            .iter()
            .find(|o| o.kind.name() == "ash_crust")
            .expect("a crust");
        let Kind::AshCrust { radius, depth, lid } = &crust.kind else {
            unreachable!()
        };
        let [x, y, z] = crust.pos;
        // The lid has the old ground. The pit is below it.
        assert!((lid.height(x, z) - y).abs() < 1e-4);
        assert!((before.sample_world(x, z) - y).abs() < 1e-4);
        assert!((zone.heights.sample_world(x, z) - (y - depth)).abs() < 1e-3);
        // The rim did not change.
        let rim = x + radius + 0.5;
        assert_eq!(
            zone.heights.sample_world(rim, z),
            before.sample_world(rim, z)
        );
        assert!((lid.height(rim, z) - before.sample_world(rim, z)).abs() < 1e-4);
    }

    #[test]
    fn a_valley_between_walls_gets_gusts_along_it() {
        // A valley along z: flat floor 10 m wide, walls that go up.
        let mut zone = zone_from(512, |x, _| ((x.abs() - 5.0).max(0.0) * 0.8).min(40.0));
        let list = place_obstacles(&mut zone, &ObstacleParams::default());
        let gusts: Vec<_> = list.iter().filter(|o| o.kind.name() == "gust").collect();
        assert!(!gusts.is_empty());
        for g in gusts {
            assert!(g.pos[0].abs() <= 9.0, "the gust is in the valley: {g:?}");
            assert!(g.dir[1].abs() > 0.99, "the wind blows along the valley");
        }
    }

    #[test]
    fn a_gully_on_a_slope_gets_rolling_stones() {
        // A slope of 25 degrees down to +z with a channel along x = 0.
        let mut zone = zone_from(512, |x, z| 200.0 - z * 0.47 + (x.abs().min(12.0)) * 0.5);
        let list = place_obstacles(&mut zone, &ObstacleParams::default());
        let stones: Vec<_> = list
            .iter()
            .filter(|o| o.kind.name() == "rolling_stones")
            .collect();
        assert!(!stones.is_empty());
        for s in stones {
            assert!(s.pos[0].abs() < 6.0, "the stones start in the channel");
            assert!(s.dir[1] > 0.9, "the stones roll downhill");
        }
    }

    #[test]
    fn a_cliff_with_a_ledge_gets_a_launch_vent_at_its_foot() {
        // Flat ground, then a face of 4 m in 2 m at x = 20, then flat.
        let mut zone = zone_from(512, |x, _| ((x - 20.0) * 2.0).clamp(0.0, 4.0));
        let list = place_obstacles(&mut zone, &ObstacleParams::default());
        let vents: Vec<_> = list
            .iter()
            .filter(|o| o.kind.name() == "vent_launch")
            .collect();
        assert!(!vents.is_empty());
        for v in vents {
            let Kind::VentLaunch { rise, reach } = v.kind else {
                unreachable!()
            };
            assert!(v.dir[0] > 0.7, "the ledge is to +x");
            assert!(
                v.pos[0] < 20.5 && v.pos[0] > 16.0,
                "the vent is at the foot"
            );
            assert!((rise - 4.0).abs() < 0.5, "rise {rise}");
            let top = v.pos[0] + v.dir[0] * reach;
            assert!(top > 22.0, "the reach ends on the ledge");
        }
    }

    #[test]
    fn same_seed_same_obstacles_and_the_rules_hold() {
        let hp = HighlandsParams::default();
        let p = ObstacleParams::default();
        let mut a = generate(7, 768, 1.0, &hp);
        let mut b = generate(7, 768, 1.0, &hp);
        let la = place_obstacles(&mut a, &p);
        let lb = place_obstacles(&mut b, &p);
        assert_eq!(la, lb);
        assert_eq!(a.heights.checksum(), b.heights.checksum());
        assert!(!la.is_empty());

        let clean = generate(7, 768, 1.0, &hp);
        let (sx, sz) = clean.start_point();
        let half = 768.0 * 0.5;
        for (i, o) in la.iter().enumerate() {
            assert_eq!(o.id as usize, i);
            let [x, _, z] = o.pos;
            assert!(x.abs() <= half - p.border_m && z.abs() <= half - p.border_m);
            let d = ((x - sx).powi(2) + (z - sz).powi(2)).sqrt();
            assert!(d >= p.clear_start_m - 2.0, "too near to the start: {o:?}");
            assert!((0.0..=1.0).contains(&o.difficulty));
            assert!(o.phase >= 0.0 && o.phase <= o.period);
            let len = (o.dir[0].powi(2) + o.dir[1].powi(2)).sqrt();
            assert!((len - 1.0).abs() < 1e-3);
            for other in &la[i + 1..] {
                let gap = ((x - other.pos[0]).powi(2) + (z - other.pos[2]).powi(2)).sqrt();
                assert!(gap >= p.any_gap_m - 3.0, "{o:?} and {other:?}");
            }
        }
    }

    #[test]
    fn density_zero_makes_no_obstacles() {
        let mut zone = zone_from(256, |_, _| 0.0);
        let before = zone.heights.checksum();
        let p = ObstacleParams {
            density: 0.0,
            ..ObstacleParams::default()
        };
        assert!(place_obstacles(&mut zone, &p).is_empty());
        assert_eq!(zone.heights.checksum(), before);
    }
}
