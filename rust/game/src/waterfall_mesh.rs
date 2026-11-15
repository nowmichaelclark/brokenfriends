//! The meshes of the waterfalls (S-165, `plan/demo-2.md`, "Waterfalls").
//! This module has no Godot code, so it has unit tests.
//! `crate::waterfall::BfWaterfalls` gives the meshes to Godot.
//!
//! Each tile of the zone (as in `water_mesh.rs`) gets two surfaces:
//!
//! - The **curtain**: a sheet that follows the path of each fall and
//!   lies above the ground. It draws opaque with an alpha
//!   scissor, so it needs no blending. The vertex data is:
//!   `COLOR.rgb` the deep colour (linear), `COLOR.a` the water mask
//!   (1 where the ground is below the water surface, 0 on a bank above
//!   it), `UV` (the place across the fall in metres from the middle, the
//!   distance along the fall in metres), `UV2` (the speed of the water
//!   in m/s, the half width in metres plus 1000 for a thick fall).
//! - The **effects**: one foam quad on the pool of each fall, the splash
//!   particles and the mist cards. They draw blended. A splash and a mist
//!   card has 4 vertices at one place: the shader makes the card face the
//!   camera and moves it (no particle system). The vertex data is:
//!   `COLOR.rgb` the tint, `COLOR.a` the size in metres, `UV` the corner
//!   (-1 to 1), `UV2` (the kind: 1 foam, 2 mist, 3 splash; a seed from 0
//!   to 1).

use bf_worldgen::water::Water;
use bf_worldgen::waterfalls::{FallFeature, Waterfall};
use bf_worldgen::HeightMap;
use std::collections::HashMap;

/// The kinds of effect cards (`UV2.x` of the effects surface).
pub const KIND_FOAM: f32 = 1.0;
pub const KIND_MIST: f32 = 2.0;
pub const KIND_SPLASH: f32 = 3.0;

/// `UV2.y` of the curtain has this added for a thick fall.
pub const THICK_FLAG: f32 = 1000.0;

/// The settings of the fall meshes.
#[derive(Clone, Debug, PartialEq)]
pub struct FallParams {
    /// The size of one tile (as for the water meshes), in metres.
    pub tile_m: f32,
    /// The distance between two rows of the curtain, in metres.
    pub row_m: f32,
    /// The distance between two columns of the curtain, in metres.
    pub column_m: f32,
    /// The curtain reaches this far upstream of the lip, in metres.
    pub lip_overlap_m: f32,
    /// The curtain lies this far above the ground, in metres.
    pub ground_offset_m: f32,
    /// The most mist cards of one fall.
    pub mist_max: usize,
    /// The most splash particles of one fall.
    pub splash_max: usize,
    /// The least splash particles of a fall that has splash.
    pub splash_min: usize,
    /// The mist cards and the splash particles move this far from their
    /// place, in metres, at the most. The box of a tile has this margin.
    pub effect_reach_m: f32,
}

impl Default for FallParams {
    fn default() -> Self {
        Self {
            tile_m: 1024.0,
            row_m: 2.0,
            column_m: 1.5,
            lip_overlap_m: 1.5,
            ground_offset_m: 0.6,
            mist_max: 8,
            splash_max: 64,
            splash_min: 32,
            effect_reach_m: 24.0,
        }
    }
}

/// The vertex data of one surface.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Surface {
    pub positions: Vec<[f32; 3]>,
    pub colours: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub uvs2: Vec<[f32; 2]>,
    pub indices: Vec<i32>,
}

impl Surface {
    fn quad(&mut self, centre: [f32; 3], rgb: [f32; 3], size: f32, kind: f32, seed: f32) {
        let base = self.positions.len() as i32;
        for (cx, cy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            self.positions.push(centre);
            self.colours.push([rgb[0], rgb[1], rgb[2], size]);
            self.uvs.push([cx, cy]);
            self.uvs2.push([kind, seed]);
        }
        self.indices
            .extend([base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    /// The same as `quad`, but the corners are in the world: a flat
    /// quad on the water.
    fn flat_quad(&mut self, centre: [f32; 3], rgb: [f32; 3], half: f32, kind: f32, seed: f32) {
        let base = self.positions.len() as i32;
        for (cx, cy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            self.positions
                .push([centre[0] + cx * half, centre[1], centre[2] + cy * half]);
            self.colours.push([rgb[0], rgb[1], rgb[2], half]);
            self.uvs.push([cx, cy]);
            self.uvs2.push([kind, seed]);
        }
        self.indices
            .extend([base, base + 2, base + 1, base + 1, base + 2, base + 3]);
    }
}

/// The meshes of the falls of one tile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileFalls {
    pub tile: (i32, i32),
    pub curtain: Surface,
    pub effects: Surface,
    /// The number of falls, splash particles and mist cards.
    pub falls: usize,
    pub splash: usize,
    pub mist: usize,
    /// The box around the tile, with the reach of the effects.
    pub aabb: ([f32; 3], [f32; 3]),
}

impl TileFalls {
    pub fn triangles(&self) -> usize {
        (self.curtain.indices.len() + self.effects.indices.len()) / 3
    }
}

/// Changes an sRGB colour part (0 to 1) to linear light.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// A seeded number from 0 to 1 for the effects of a fall.
fn rand(fall: u32, index: u32, salt: u32) -> f32 {
    let mut h = fall
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(index.wrapping_mul(0x85EB_CA6B))
        .wrapping_add(salt.wrapping_mul(0xC2B2_AE35))
        ^ 0x27D4_EB2F;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// The speed of the water of a fall, in metres each second. A thick
/// fall is slow.
pub fn fall_speed(f: &Waterfall) -> f32 {
    let speed = (4.0 + 0.5 * f.height_m).clamp(4.0, 14.0);
    if f.thick {
        speed * 0.3
    } else {
        speed
    }
}

/// The number of splash particles of a fall.
pub fn splash_count(f: &Waterfall, p: &FallParams) -> usize {
    if f.height_m < 3.0 {
        return 0;
    }
    let n = (8.0 + 1.5 * f.height_m).round() as usize;
    // A tall or wide fall has the full count.
    let n = if f.feature == FallFeature::Plain {
        n
    } else {
        n.max(p.splash_min)
    };
    n.min(p.splash_max)
}

/// The number of mist cards of a fall: 3 to 8. A thick fall and a low
/// fall have none.
pub fn mist_count(f: &Waterfall, p: &FallParams) -> usize {
    if f.thick || f.height_m < 6.0 {
        return 0;
    }
    ((2.0 + f.height_m / 6.0).round() as usize).clamp(3, p.mist_max.max(3))
}

/// Makes the meshes of all falls, one for each tile that has a fall.
pub fn build_falls(water: &Water, heights: &HeightMap, p: &FallParams) -> Vec<TileFalls> {
    let corner = water.size as f32 * 0.5 * water.spacing;
    let mut tiles: HashMap<(i32, i32), TileFalls> = HashMap::new();
    for f in &water.falls {
        let top = f.top();
        let tile = (
            ((top[0] + corner) / p.tile_m).floor() as i32,
            ((top[2] + corner) / p.tile_m).floor() as i32,
        );
        let t = tiles.entry(tile).or_insert_with(|| TileFalls {
            tile,
            ..TileFalls::default()
        });
        add_curtain(t, f, heights, p);
        add_effects(t, f, p);
        t.falls += 1;
    }
    let mut out: Vec<TileFalls> = tiles.into_values().collect();
    out.sort_by_key(|t| t.tile);
    for t in &mut out {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in t.curtain.positions.iter().chain(&t.effects.positions) {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k]);
                hi[k] = hi[k].max(v[k]);
            }
        }
        for k in 0..3 {
            lo[k] -= p.effect_reach_m;
            hi[k] += p.effect_reach_m;
        }
        t.aabb = (lo, hi);
    }
    out
}

/// Adds the curtain of a fall. It goes from `lip_overlap_m` upstream of
/// the lip to a little below the foot. Each vertex lies
/// `ground_offset_m` above the ground and never above the level of the
/// lip, never below the level of the pool, and never higher than the
/// vertex above it. A vertex on ground that is higher than the water
/// there has the mask 0.
fn add_curtain(t: &mut TileFalls, f: &Waterfall, heights: &HeightMap, p: &FallParams) {
    let top = f.top();
    let bottom = f.bottom();
    let colour = {
        let c = f.colour;
        [linear(c[0]), linear(c[1]), linear(c[2])]
    };
    let half = f.width_m * 0.5;
    let columns = ((f.width_m / p.column_m.max(0.25)).ceil() as usize + 1).clamp(3, 32);
    let rows = (((f.length_m + p.lip_overlap_m) / p.row_m.max(0.25)).ceil() as usize + 1).max(2);
    let speed = fall_speed(f);
    let flag = if f.thick { THICK_FLAG } else { 0.0 };
    let base = t.curtain.positions.len() as i32;
    // The lowest y so far of each column: the curtain never goes up.
    let mut floor = vec![top[1]; columns];
    for row in 0..rows {
        let d = -p.lip_overlap_m + (f.length_m + p.lip_overlap_m) * row as f32 / (rows - 1) as f32;
        let (pos, dir) = if d < 0.0 {
            // Upstream of the lip: go back along the first segment.
            let (a, dir) = f.at_distance(0.0);
            ([a[0] + dir[0] * d, a[1], a[2] + dir[1] * d], dir)
        } else {
            f.at_distance(d)
        };
        let side = [-dir[1], dir[0]];
        let level = pos[1].clamp(bottom[1], top[1]);
        for (c, floor_y) in floor.iter_mut().enumerate() {
            let s = if columns > 1 {
                c as f32 / (columns - 1) as f32 * 2.0 - 1.0
            } else {
                0.0
            };
            let (x, z) = (pos[0] + side[0] * s * half, pos[2] + side[1] * s * half);
            // The highest ground within 1 m: the sheet clears small bumps
            // and the coarse triangles of the far terrain.
            let ground = [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
                .iter()
                .map(|&(dx, dz)| heights.sample_world(x + dx, z + dz))
                .fold(f32::MIN, f32::max);
            // The water mask: 1 if the ground is below the surface here.
            let above = ground - level;
            let mask = (1.0 - (above - 0.3) / 2.0).clamp(0.0, 1.0);
            let y = (ground + p.ground_offset_m)
                .clamp(bottom[1], top[1])
                .min(*floor_y);
            *floor_y = y;
            t.curtain.positions.push([x, y, z]);
            t.curtain
                .colours
                .push([colour[0], colour[1], colour[2], mask]);
            t.curtain.uvs.push([s * half, d]);
            t.curtain.uvs2.push([speed, half + flag]);
        }
    }
    for row in 0..rows - 1 {
        for c in 0..columns - 1 {
            let i = base + (row * columns + c) as i32;
            let j = i + columns as i32;
            t.curtain.indices.extend([i, i + 1, j, i + 1, j + 1, j]);
        }
    }
}

/// Adds the foam, the splash and the mist of a fall at its foot.
fn add_effects(t: &mut TileFalls, f: &Waterfall, p: &FallParams) {
    let bottom = f.bottom();
    let dir = f.dir;
    let side = [-dir[1], dir[0]];
    let tint = {
        let c = f.colour;
        [
            linear(c[0]) * 0.5 + 0.5,
            linear(c[1]) * 0.5 + 0.5,
            linear(c[2]) * 0.5 + 0.5,
        ]
    };
    let n = f.number;
    // The foam: a square on the pool, a little over the water.
    let half = (f.width_m * 0.8 + 2.0 + f.height_m * 0.06).clamp(3.0, 28.0);
    let centre = [
        bottom[0] + dir[0] * half * 0.35,
        bottom[1] + 0.04,
        bottom[2] + dir[1] * half * 0.35,
    ];
    t.effects
        .flat_quad(centre, [1.0, 1.0, 1.0], half, KIND_FOAM, rand(n, 0, 1));

    // The splash: small cards at the foot that jump and fall.
    let splash = splash_count(f, p);
    let splash_size = (0.15 + f.height_m * 0.006).clamp(0.15, 0.4);
    for i in 0..splash {
        let across = (rand(n, i as u32, 2) - 0.5) * f.width_m * 0.9;
        let along = 0.3 + rand(n, i as u32, 3) * 1.5;
        let pos = [
            bottom[0] + side[0] * across + dir[0] * along,
            bottom[1] + 0.1,
            bottom[2] + side[1] * across + dir[1] * along,
        ];
        t.effects.quad(
            pos,
            [1.0, 1.0, 1.0],
            splash_size,
            KIND_SPLASH,
            rand(n, i as u32, 4),
        );
    }
    t.splash += splash;

    // The mist: a few large soft cards over the pool.
    let mist = mist_count(f, p);
    let mist_size = (3.0 + f.height_m * 0.18).clamp(3.0, 12.0);
    for i in 0..mist {
        let across = ((i as f32 + 0.5) / mist as f32 - 0.5) * f.width_m * 0.8;
        let along = 1.0 + rand(n, i as u32, 5) * 3.0;
        let pos = [
            bottom[0] + side[0] * across + dir[0] * along,
            bottom[1] + mist_size * 0.3,
            bottom[2] + side[1] * across + dir[1] * along,
        ];
        t.effects.quad(
            pos,
            tint,
            mist_size * (0.8 + 0.4 * rand(n, i as u32, 6)),
            KIND_MIST,
            (i as f32 + rand(n, i as u32, 7) * 0.5) / mist as f32,
        );
    }
    t.mist += mist;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_worldgen::highlands::{generate, HighlandsParams};

    fn zone() -> bf_worldgen::highlands::Zone {
        generate(1, 2048, 1.0, &HighlandsParams::default())
    }

    #[test]
    fn the_meshes_are_valid() {
        let zone = zone();
        let p = FallParams::default();
        let tiles = build_falls(&zone.water, &zone.heights, &p);
        assert!(!tiles.is_empty(), "seed 1 has falls");
        let falls: usize = tiles.iter().map(|t| t.falls).sum();
        assert_eq!(falls, zone.water.falls.len());
        for t in &tiles {
            for s in [&t.curtain, &t.effects] {
                let n = s.positions.len();
                assert_eq!(n, s.colours.len());
                assert_eq!(n, s.uvs.len());
                assert_eq!(n, s.uvs2.len());
                assert_eq!(s.indices.len() % 3, 0);
                assert!(s.indices.iter().all(|&i| (i as usize) < n));
            }
            for v in t.curtain.positions.iter().chain(&t.effects.positions) {
                assert!(v.iter().all(|c| c.is_finite()));
                assert!(
                    (0..3).all(|k| v[k] >= t.aabb.0[k] && v[k] <= t.aabb.1[k]),
                    "a vertex is outside the box"
                );
            }
        }
    }

    #[test]
    fn the_curtain_never_goes_up_the_fall() {
        let zone = zone();
        let f = zone.water.falls.iter().find(|f| f.height_m > 8.0).unwrap();
        let mut t = TileFalls::default();
        let p = FallParams::default();
        add_curtain(&mut t, f, &zone.heights, &p);
        let columns = (f.width_m / p.column_m).ceil() as usize + 1;
        let columns = columns.clamp(3, 32);
        let rows = t.curtain.positions.len() / columns;
        for c in 0..columns {
            for r in 1..rows {
                let (a, b) = (
                    t.curtain.positions[(r - 1) * columns + c][1],
                    t.curtain.positions[r * columns + c][1],
                );
                assert!(b <= a + 1e-4);
            }
        }
    }

    #[test]
    fn the_counts_follow_the_spec() {
        let zone = zone();
        let p = FallParams::default();
        for f in &zone.water.falls {
            let (s, m) = (splash_count(f, &p), mist_count(f, &p));
            assert!(s <= 64 && (m == 0 || (3..=8).contains(&m)));
            if f.thick {
                assert_eq!(m, 0);
            }
            if f.feature != FallFeature::Plain {
                assert!(s >= 32);
            }
        }
    }

    #[test]
    fn the_same_zone_gives_the_same_meshes() {
        let zone = zone();
        let p = FallParams::default();
        assert_eq!(
            build_falls(&zone.water, &zone.heights, &p),
            build_falls(&zone.water, &zone.heights, &p)
        );
    }
}
