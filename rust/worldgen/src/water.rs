//! Still water: lakes and ponds from the basin records (S-160,
//! `plan/demo-2.md`, "Water bodies"). The rivers (S-162) are in
//! `rivers.rs`; their bodies are in the same list, after the still water.
//!
//! The drainage step (`drainage.rs`) gives a record for each closed
//! basin. A deep basin becomes a lake with a seeded level. Some shallow
//! basins keep their hollow and become ponds. Each body gets a seeded
//! look: a kind, a deep colour, a clarity, a viscosity, a density and a
//! temperature. The same seed always gives the same bodies.
//!
//! The level of a body is checked on the full grid. A small window
//! around the basin is filled (priority flood), so the water never
//! leaks through a gap that the coarse drainage grid did not see.

use crate::drainage::{fill_basins, Basin};
use crate::heightmap::HeightMap;
use crate::noise::hash01;
pub use crate::rivers::{RiverCell, RiverStep};
use std::collections::VecDeque;

/// The settings of the water bodies. Lengths are in metres.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterParams {
    /// The lowest seeded fill part of a lake (0 to 1). The level is
    /// `floor + part * (spill - floor)`.
    pub lake_part_min: f32,
    /// The chance that a lake is full (part 1), so it spills at its
    /// spill cell. The other lakes get a part from `lake_part_min` to 1.
    pub full_lake_chance: f32,
    /// The part of the shallow basins that keep their hollow and become
    /// ponds (0 to 1).
    pub pond_part: f32,
    /// A shallow basin must be this deep to become a pond.
    pub pond_min_depth_m: f32,
    /// A shallow basin must have this area to become a pond, in square
    /// metres.
    pub pond_min_area_m2: f32,
    /// A pond is this much lower than its spill height, at least.
    pub pond_drop_min_m: f32,
    /// A pond is this much lower than its spill height, at most.
    pub pond_drop_max_m: f32,
    /// A body with less water than this at its deepest place is left out.
    pub min_depth_m: f32,
    /// A body with less area than this is left out, in square metres.
    pub min_area_m2: f32,
    /// The ground is damp up to this height above the water level.
    pub damp_height_m: f32,
    /// A riverbed has water if its wet area is more than this, in square
    /// metres (S-162). The wet area does not count the rain that a closed
    /// lake keeps.
    pub wet_min_area_m2: f32,
    /// The lowest seeded part of the bed depth that a river fills (0 to 1).
    pub river_part_min: f32,
    /// The highest seeded part of the bed depth that a river fills.
    pub river_part_max: f32,
    /// A river is at least this deep at the centre of its bed, in metres.
    pub river_min_depth_m: f32,
    /// A steep river is deeper: this part of the drop of the bed to the
    /// next cell is added to the least depth. Thus the ground does not
    /// show through the water of a rapid.
    pub steep_depth_part: f32,
    /// A river stays this much below the lowest bank next to it, in
    /// metres. Thus the water never stands above the ground beside it.
    pub bank_margin_m: f32,
    /// Near its mouth, a river gets this much deeper for each metre away
    /// from the water that it flows into. Thus its surface comes down to
    /// the lake.
    pub mouth_slope: f32,
    /// A drop of more than this from one river cell to the next is a
    /// rapid, in metres.
    pub rapid_drop_m: f32,
    /// A drop of more than this is a waterfall, in metres (S-165 draws it).
    pub fall_drop_m: f32,
    /// A drop steeper than this (rise divided by run, 1 is 45 degrees) is
    /// a waterfall.
    pub fall_slope: f32,
    /// The tall fall of a zone is at least this high, in metres (S-165).
    pub tall_fall_m: f32,
    /// The wide fall of a zone is drawn at least this wide, in metres.
    pub wide_fall_m: f32,
    /// The wide fall is the best fall that is at least this high, in
    /// metres.
    pub wide_min_height_m: f32,
    /// The slowest river flow, in metres each second.
    pub flow_min_mps: f32,
    /// The flow gets this much faster for each unit of the square root
    /// of the slope of the water surface.
    pub flow_slope_k: f32,
    /// The fastest river flow, in metres each second.
    pub flow_max_mps: f32,
    /// A river with fewer coarse cells than this is left out.
    pub river_min_cells: u32,
}

impl Default for WaterParams {
    fn default() -> Self {
        Self {
            lake_part_min: 0.2,
            full_lake_chance: 0.25,
            pond_part: 0.3,
            pond_min_depth_m: 0.6,
            pond_min_area_m2: 60.0,
            pond_drop_min_m: 0.1,
            pond_drop_max_m: 0.5,
            min_depth_m: 0.3,
            min_area_m2: 25.0,
            damp_height_m: 0.6,
            wet_min_area_m2: 50_000.0,
            river_part_min: 0.4,
            river_part_max: 0.8,
            river_min_depth_m: 0.25,
            steep_depth_part: 0.5,
            bank_margin_m: 0.1,
            mouth_slope: 0.05,
            rapid_drop_m: 0.5,
            fall_drop_m: 2.5,
            fall_slope: 1.0,
            tall_fall_m: 40.0,
            wide_fall_m: 30.0,
            wide_min_height_m: 8.0,
            flow_min_mps: 0.3,
            flow_slope_k: 6.0,
            flow_max_mps: 3.5,
            river_min_cells: 6,
        }
    }
}

/// The kind of a water body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterKind {
    /// Water in a deep basin.
    Lake,
    /// Water in a shallow hollow.
    Pond,
    /// One river system: a tree of riverbeds with water (S-162).
    River,
}

impl WaterKind {
    pub fn name(self) -> &'static str {
        match self {
            WaterKind::Lake => "lake",
            WaterKind::Pond => "pond",
            WaterKind::River => "river",
        }
    }
}

/// The look of a water body (`demo-2.md`, "Look kinds", assumed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    ClearSpring,
    SiltyBrown,
    MilkyAlkaline,
    RustOrange,
    GreenAlgae,
    GreyMud,
    BlackTar,
}

/// All looks, in the order of the weight tables.
pub const LOOKS: [Look; 7] = [
    Look::ClearSpring,
    Look::SiltyBrown,
    Look::MilkyAlkaline,
    Look::RustOrange,
    Look::GreenAlgae,
    Look::GreyMud,
    Look::BlackTar,
];

impl Look {
    pub fn name(self) -> &'static str {
        match self {
            Look::ClearSpring => "clear_spring",
            Look::SiltyBrown => "silty_brown",
            Look::MilkyAlkaline => "milky_alkaline",
            Look::RustOrange => "rust_orange",
            Look::GreenAlgae => "green_algae",
            Look::GreyMud => "grey_mud",
            Look::BlackTar => "black_tar",
        }
    }

    /// The viscosity of the look, from 0 (water) to 1.
    pub fn viscosity(self) -> f32 {
        match self {
            Look::ClearSpring => 0.0,
            Look::SiltyBrown => 0.1,
            Look::MilkyAlkaline => 0.15,
            Look::RustOrange => 0.2,
            Look::GreenAlgae => 0.3,
            Look::GreyMud => 0.6,
            Look::BlackTar => 0.85,
        }
    }

    /// The deep colour (sRGB, 0 to 1).
    fn colour(self) -> [f32; 3] {
        match self {
            Look::ClearSpring => [0.10, 0.36, 0.40],
            Look::SiltyBrown => [0.36, 0.27, 0.15],
            Look::MilkyAlkaline => [0.66, 0.74, 0.68],
            Look::RustOrange => [0.62, 0.30, 0.12],
            Look::GreenAlgae => [0.26, 0.38, 0.12],
            Look::GreyMud => [0.40, 0.37, 0.33],
            Look::BlackTar => [0.06, 0.05, 0.045],
        }
    }

    /// The range of the clarity: the depth in metres at which the water
    /// hides about 63% of the ground below it.
    pub(crate) fn clarity(self) -> (f32, f32) {
        match self {
            Look::ClearSpring => (4.0, 8.0),
            Look::SiltyBrown => (0.4, 1.0),
            Look::MilkyAlkaline => (0.3, 0.8),
            Look::RustOrange => (0.5, 1.2),
            Look::GreenAlgae => (0.6, 1.5),
            Look::GreyMud => (0.2, 0.3),
            Look::BlackTar => (0.2, 0.2),
        }
    }

    /// The range of the density (1 is water).
    pub(crate) fn density(self) -> (f32, f32) {
        match self {
            Look::GreyMud => (1.3, 1.6),
            Look::BlackTar => (1.15, 1.35),
            Look::MilkyAlkaline => (1.05, 1.15),
            _ => (1.0, 1.05),
        }
    }
}

/// One lake, pond or river system.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterBody {
    /// The number of the body. The lakes and ponds are numbered in the
    /// order of their spill cells. The rivers come after them, in the
    /// order of their mouths.
    pub number: u32,
    pub kind: WaterKind,
    /// The height of the water surface in metres. A river has a level
    /// for each cell (`Water::level_at_sample`); this is the level at
    /// its mouth, the lowest one.
    pub level: f32,
    /// The spill height of the basin in metres. For a river: the highest
    /// level (at its sources).
    pub spill: f32,
    /// The sample where water leaves the basin (x, z). For a river: its
    /// mouth.
    pub spill_sample: (u32, u32),
    /// The deepest sample (x, z).
    pub deepest_sample: (u32, u32),
    /// The first sample of the box around the wet samples (x, z).
    pub min: (u32, u32),
    /// The last sample of the box around the wet samples (x, z).
    pub max: (u32, u32),
    /// The number of wet samples.
    pub wet_samples: u32,
    /// The area of the water in square metres.
    pub area_m2: f32,
    /// The largest depth of the water in metres.
    pub max_depth_m: f32,
    /// The part of the basin that is filled (1: the lake spills). For a
    /// river: the seeded part of the bed depth.
    pub fill_part: f32,
    pub look: Look,
    /// The colour of deep water (sRGB, 0 to 1).
    pub deep_colour: [f32; 3],
    /// The depth in metres at which the water hides about 63% of the
    /// ground below it (0.2 to 8 m).
    pub clarity_m: f32,
    /// From 0 (water) to 1 (thick tar).
    pub viscosity: f32,
    /// 1.0 to 1.6. A dense body holds the bear higher.
    pub density: f32,
    /// From 0 (cold) to 1 (hot). Above 0.7 the water steams (later).
    pub temperature: f32,
    /// The mean flow speed in metres each second (0 for still water).
    pub flow_mps: f32,
}

/// All water of a zone: the lakes, the ponds and the rivers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Water {
    /// The number of samples on each side of the zone.
    pub size: usize,
    /// The distance between two samples in metres.
    pub spacing: f32,
    /// The bodies, in the order of their numbers.
    pub bodies: Vec<WaterBody>,
    /// For each sample: 0 if dry, else the index of the body plus 1.
    /// It is empty if the zone has no water.
    pub owner: Vec<u16>,
    /// The ground is damp up to this height above the water level.
    pub damp_height_m: f32,
    /// The coarse cells of the rivers that have water (S-162), in the
    /// order of their cell indexes.
    pub river_cells: Vec<RiverCell>,
    /// The wet samples of the rivers (`z * size + x`), from low to high.
    pub river_samples: Vec<u32>,
    /// For each item of `river_samples`: its index in `river_cells`.
    pub river_sample_cell: Vec<u32>,
    /// The rapids and the waterfalls of the rivers.
    pub river_steps: Vec<RiverStep>,
    /// The waterfalls: the fall steps of the rivers, joined (S-165).
    pub falls: Vec<crate::waterfalls::Waterfall>,
}

/// The water at one place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterHere<'a> {
    pub body: &'a WaterBody,
    /// The height of the water surface here, in metres.
    pub level: f32,
    /// The water level minus the ground height, in metres.
    pub depth: f32,
    /// The flow of the water in metres each second (x, z). Still water
    /// has none.
    pub flow: [f32; 2],
}

/// A seeded number from 0 to 1 for a body. `salt` selects which one.
pub(crate) fn rand(key: u32, seed: u32, salt: u32) -> f32 {
    hash01(
        key as i32,
        salt as i32,
        seed ^ 0x3A7E_2B00 ^ salt.wrapping_mul(0x9E37_79B9),
    )
}

/// Selects the shallow basins that keep their hollow and become ponds.
/// `river_cells` tells if a cell has a riverbed: a pond is not on a
/// river. Gives the index of each selected basin in `basins`.
pub fn choose_ponds(
    basins: &[Basin],
    cell_area_m2: f32,
    seed: u32,
    params: &WaterParams,
    river_cells: impl Fn(u32) -> bool,
) -> Vec<usize> {
    basins
        .iter()
        .enumerate()
        .filter(|(_, b)| {
            !b.deep
                && b.pass == 0
                && b.depth >= params.pond_min_depth_m
                && b.cells.len() as f32 * cell_area_m2 >= params.pond_min_area_m2
                && !b.cells.iter().any(|&c| river_cells(c))
                && rand(b.spill_cell, seed, 1) < params.pond_part
        })
        .map(|(i, _)| i)
        .collect()
}

/// Gives the seeded fill part of the lake in a deep basin (0 to 1). A
/// lake with part 1 is full: it spills at its spill cell.
pub fn lake_part(basin: &Basin, seed: u32, params: &WaterParams) -> f32 {
    let key = basin.spill_cell;
    if rand(key, seed, 2) < params.full_lake_chance {
        1.0
    } else {
        let lo = params.lake_part_min.clamp(0.0, 1.0);
        lo + (1.0 - lo) * rand(key, seed, 3)
    }
}

/// A basin that can hold water, with the grid of the drainage step.
pub struct Candidate<'a> {
    pub basin: &'a Basin,
    pub kind: WaterKind,
}

/// One body before the bodies get their numbers.
struct Found {
    kind: WaterKind,
    level: f32,
    spill: f32,
    spill_sample: (u32, u32),
    deepest: (u32, u32),
    fill_part: f32,
    wet: Vec<u32>,
    max_depth: f32,
}

/// Makes the water bodies of a zone.
///
/// `heights` is the full grid. The basins are on the coarse grid of the
/// drainage step: each coarse cell is `step` samples.
pub fn make_water(
    heights: &HeightMap,
    candidates: &[Candidate],
    coarse_size: usize,
    step: usize,
    seed: u32,
    params: &WaterParams,
) -> Water {
    let size = heights.size;
    let spacing = heights.spacing;
    let to_sample = |c: u32| {
        let (cx, cz) = (c as usize % coarse_size, c as usize / coarse_size);
        ((cx * step).min(size - 1), (cz * step).min(size - 1))
    };

    let mut found: Vec<Found> = Vec::new();
    for cand in candidates {
        let b = cand.basin;
        let key = b.spill_cell;
        let (target, part) = match cand.kind {
            WaterKind::Lake => {
                let part = lake_part(b, seed, params);
                (b.floor + part * (b.spill - b.floor), part)
            }
            WaterKind::Pond => {
                let drop = params.pond_drop_min_m
                    + (params.pond_drop_max_m - params.pond_drop_min_m) * rand(key, seed, 4);
                // A shallow pond keeps at least half of its depth.
                let drop = drop.min(b.depth * 0.5);
                (b.spill - drop, 1.0 - drop / b.depth.max(0.01))
            }
            // Rivers do not come from basins (`rivers.rs`).
            WaterKind::River => continue,
        };

        // The window: the box of the basin cells and a margin.
        let margin = 3 * step + 2;
        let (mut x0, mut z0, mut x1, mut z1) = (usize::MAX, usize::MAX, 0, 0);
        for &c in &b.cells {
            let (x, z) = to_sample(c);
            x0 = x0.min(x);
            z0 = z0.min(z);
            x1 = x1.max(x);
            z1 = z1.max(z);
        }
        let x0 = x0.saturating_sub(margin);
        let z0 = z0.saturating_sub(margin);
        let x1 = (x1 + margin).min(size - 1);
        let z1 = (z1 + margin).min(size - 1);
        let (w, h) = (x1 - x0 + 1, z1 - z0 + 1);
        if w < 3 || h < 3 {
            continue;
        }

        // The deepest sample near the floor cell of the basin.
        let (fx, fz) = to_sample(b.floor_cell);
        let mut deepest = (fx, fz);
        for z in fz.saturating_sub(step)..=(fz + step).min(size - 1) {
            for x in fx.saturating_sub(step)..=(fx + step).min(size - 1) {
                let here = heights.data[z * size + x];
                let best = heights.data[deepest.1 * size + deepest.0];
                if here < best {
                    deepest = (x, z);
                }
            }
        }

        // Fill the window on the full grid. Its border is the outlet.
        let n = w.max(h);
        let mut window = vec![f32::MAX; n * n];
        for z in 0..h {
            for x in 0..w {
                window[z * n + x] = heights.data[(z0 + z) * size + x0 + x];
            }
        }
        // `fill_basins` needs a square grid. The extra cells are very
        // high, so they are a wall; the border of the real window is
        // the outlet.
        let mut outlets = vec![false; n * n];
        for z in 0..h {
            for x in 0..w {
                if x == 0 || z == 0 || x == w - 1 || z == h - 1 {
                    outlets[z * n + x] = true;
                }
            }
        }
        for v in window.iter_mut() {
            if *v == f32::MAX {
                *v = 1.0e6;
            }
        }
        let filled = fill_basins(&window, n, 0.0, Some(&outlets));
        let local = (deepest.1 - z0) * n + (deepest.0 - x0);
        let full_spill = filled[local];
        let level = target.min(full_spill - 0.02);
        let ground = heights.data[deepest.1 * size + deepest.0];
        if level - ground < params.min_depth_m {
            continue;
        }

        // The wet samples: all samples below the level that join the
        // deepest sample.
        let mut seen = vec![false; w * h];
        let mut wet = Vec::new();
        let mut queue = VecDeque::new();
        seen[(deepest.1 - z0) * w + (deepest.0 - x0)] = true;
        queue.push_back(deepest);
        while let Some((x, z)) = queue.pop_front() {
            wet.push((z * size + x) as u32);
            let around = [
                (x.wrapping_sub(1), z),
                (x + 1, z),
                (x, z.wrapping_sub(1)),
                (x, z + 1),
            ];
            for (nx, nz) in around {
                if nx < x0 || nz < z0 || nx > x1 || nz > z1 {
                    continue;
                }
                let li = (nz - z0) * w + (nx - x0);
                if seen[li] || heights.data[nz * size + nx] >= level {
                    continue;
                }
                seen[li] = true;
                queue.push_back((nx, nz));
            }
        }
        let area = wet.len() as f32 * spacing * spacing;
        if area < params.min_area_m2 {
            continue;
        }
        wet.sort_unstable();
        found.push(Found {
            kind: cand.kind,
            level,
            spill: b.spill,
            spill_sample: {
                let (x, z) = to_sample(b.spill_cell);
                (x as u32, z as u32)
            },
            deepest: (deepest.0 as u32, deepest.1 as u32),
            fill_part: part,
            wet,
            max_depth: level - ground,
        });
    }

    // A body under the water of a higher body is left out. The higher
    // bodies claim their samples first.
    let mut order: Vec<usize> = (0..found.len()).collect();
    order.sort_by(|&a, &b| {
        found[b]
            .level
            .total_cmp(&found[a].level)
            .then(found[a].spill_sample.cmp(&found[b].spill_sample))
            .then(found[a].wet[0].cmp(&found[b].wet[0]))
    });
    let mut claimed = vec![false; size * size];
    let mut keep = vec![false; found.len()];
    for i in order {
        if found[i].wet.iter().any(|&s| claimed[s as usize]) {
            continue;
        }
        for &s in &found[i].wet {
            claimed[s as usize] = true;
        }
        keep[i] = true;
    }
    let mut kept: Vec<Found> = found
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(f, _)| f)
        .collect();
    let spill_index = |f: &Found| f.spill_sample.1 as usize * size + f.spill_sample.0 as usize;
    kept.sort_by(|a, b| {
        spill_index(a)
            .cmp(&spill_index(b))
            .then(a.wet[0].cmp(&b.wet[0]))
    });
    kept.truncate(u16::MAX as usize - 1);

    let mut owner = if kept.is_empty() {
        Vec::new()
    } else {
        vec![0u16; size * size]
    };
    let bodies = kept
        .into_iter()
        .enumerate()
        .map(|(number, f)| {
            let id = number as u16 + 1;
            let (mut min, mut max) = ((u32::MAX, u32::MAX), (0, 0));
            for &s in &f.wet {
                owner[s as usize] = id;
                let (x, z) = (s % size as u32, s / size as u32);
                min = (min.0.min(x), min.1.min(z));
                max = (max.0.max(x), max.1.max(z));
            }
            let key = spill_index(&f) as u32;
            let look = pick_look(&f, spacing, key, seed);
            let v = look_values(look, key, seed);
            WaterBody {
                number: number as u32,
                kind: f.kind,
                level: f.level,
                spill: f.spill,
                spill_sample: f.spill_sample,
                deepest_sample: f.deepest,
                min,
                max,
                wet_samples: f.wet.len() as u32,
                area_m2: f.wet.len() as f32 * spacing * spacing,
                max_depth_m: f.max_depth,
                fill_part: f.fill_part,
                look,
                deep_colour: v.deep_colour,
                clarity_m: v.clarity_m,
                viscosity: look.viscosity(),
                density: v.density,
                temperature: v.temperature,
                flow_mps: 0.0,
            }
        })
        .collect();
    Water {
        size,
        spacing,
        bodies,
        owner,
        damp_height_m: params.damp_height_m,
        ..Water::default()
    }
}

/// The seeded values of a look for one body.
pub(crate) struct LookValues {
    pub deep_colour: [f32; 3],
    pub clarity_m: f32,
    pub density: f32,
    pub temperature: f32,
}

/// Gives the seeded colour, clarity, density and temperature of a body.
/// `key` is the sample of the spill cell (or the mouth of a river).
pub(crate) fn look_values(look: Look, key: u32, seed: u32) -> LookValues {
    let colour = look.colour();
    let vary = |salt: u32| 0.92 + 0.16 * rand(key, seed, salt);
    let (c0, c1) = look.clarity();
    let (d0, d1) = look.density();
    let t = rand(key, seed, 14);
    LookValues {
        deep_colour: [
            (colour[0] * vary(10)).min(1.0),
            (colour[1] * vary(11)).min(1.0),
            (colour[2] * vary(12)).min(1.0),
        ],
        clarity_m: c0 + (c1 - c0) * rand(key, seed, 13),
        density: d0 + (d1 - d0) * rand(key, seed, 15),
        // Most water is cool. A few bodies are warm.
        temperature: t * t,
    }
}

/// Selects the look of a body from weights. The weights change with
/// the kind and the size of the body (`demo-2.md`, "Look kinds").
fn pick_look(f: &Found, spacing: f32, key: u32, seed: u32) -> Look {
    let area = f.wet.len() as f32 * spacing * spacing;
    // The order is the order of `LOOKS`: clear, silty, milky, rust,
    // algae, mud, tar. Most water looks natural; a few bodies are
    // strange (assumed, question 1 in `demo-2.md`).
    let weights: [f32; 7] = match f.kind {
        // A very large lake is never mud or tar: it is the main water
        // of its part of the zone.
        WaterKind::Lake if area >= 20_000.0 => [2.0, 3.0, 2.5, 1.5, 0.6, 0.0, 0.0],
        WaterKind::Lake if f.fill_part < 1.0 => [1.0, 2.0, 3.0, 1.5, 1.0, 0.5, 0.4],
        WaterKind::Lake => [2.0, 3.0, 1.0, 1.5, 1.0, 0.3, 0.1],
        WaterKind::Pond if f.max_depth < 1.0 => [0.8, 1.0, 0.5, 0.5, 2.0, 3.0, 0.3],
        WaterKind::Pond if area < 400.0 => [3.0, 1.0, 0.5, 0.5, 2.5, 1.5, 0.2],
        WaterKind::Pond => [2.0, 1.5, 1.0, 0.8, 3.0, 1.0, 0.2],
        WaterKind::River => crate::rivers::RIVER_LOOK_WEIGHTS,
    };
    look_from_weights(&weights, key, seed)
}

/// Selects a seeded look from weights in the order of `LOOKS`.
pub(crate) fn look_from_weights(weights: &[f32; 7], key: u32, seed: u32) -> Look {
    let total: f32 = weights.iter().sum();
    let mut r = rand(key, seed, 9) * total;
    for (look, &w) in LOOKS.iter().zip(weights) {
        if r < w {
            return *look;
        }
        r -= w;
    }
    LOOKS[0]
}

impl Water {
    /// Gives the body at a sample, if the sample is wet.
    pub fn body_at_sample(&self, x: usize, z: usize) -> Option<&WaterBody> {
        if self.owner.is_empty() || x >= self.size || z >= self.size {
            return None;
        }
        match self.owner[z * self.size + x] {
            0 => None,
            id => self.bodies.get(id as usize - 1),
        }
    }

    /// Gives the body at a world position (the nearest sample).
    pub fn body_at(&self, wx: f32, wz: f32) -> Option<&WaterBody> {
        if self.owner.is_empty() {
            return None;
        }
        let half = self.size as f32 * 0.5;
        let index = |w: f32| (w / self.spacing + half).round();
        let (x, z) = (index(wx), index(wz));
        if x < 0.0 || z < 0.0 {
            return None;
        }
        self.body_at_sample(x as usize, z as usize)
    }

    /// Gives the water at a world position, with the depth from the
    /// ground of `heights`. A place at the shore with no water gives
    /// `None`. In a river, the level and the flow are those of the
    /// nearest sample.
    pub fn at(&self, heights: &HeightMap, wx: f32, wz: f32) -> Option<WaterHere<'_>> {
        let (x, z) = self.nearest_sample(wx, wz)?;
        let body = self.body_at_sample(x, z)?;
        let (level, flow) = match body.kind {
            WaterKind::River => {
                let cell = self.river_cell_at_sample(x, z)?;
                (cell.level, cell.flow)
            }
            _ => (body.level, [0.0, 0.0]),
        };
        let depth = level - heights.sample_world(wx, wz);
        (depth > 0.0).then_some(WaterHere {
            body,
            level,
            depth,
            flow,
        })
    }

    /// Gives the nearest sample (x, z) of a world position, if it is in
    /// the zone.
    fn nearest_sample(&self, wx: f32, wz: f32) -> Option<(usize, usize)> {
        let half = self.size as f32 * 0.5;
        let index = |w: f32| (w / self.spacing + half).round();
        let (x, z) = (index(wx), index(wz));
        (x >= 0.0 && z >= 0.0 && (x as usize) < self.size && (z as usize) < self.size)
            .then_some((x as usize, z as usize))
    }

    /// Gives the river cell of a wet river sample.
    pub fn river_cell_at_sample(&self, x: usize, z: usize) -> Option<&RiverCell> {
        let s = (z * self.size + x) as u32;
        let i = self.river_samples.binary_search(&s).ok()?;
        self.river_cells.get(self.river_sample_cell[i] as usize)
    }

    /// Gives the water level at a wet sample: the level of the lake or
    /// pond, or the level of the river cell there.
    pub fn level_at_sample(&self, x: usize, z: usize) -> Option<f32> {
        let body = self.body_at_sample(x, z)?;
        match body.kind {
            WaterKind::River => self.river_cell_at_sample(x, z).map(|c| c.level),
            _ => Some(body.level),
        }
    }

    /// Gives the world position (x, z) of a sample.
    pub fn world(&self, x: u32, z: u32) -> (f32, f32) {
        let half = self.size as f32 * 0.5;
        (
            (x as f32 - half) * self.spacing,
            (z as f32 - half) * self.spacing,
        )
    }

    /// Gives the lake with the largest area, if the zone has a lake.
    pub fn largest_lake(&self) -> Option<&WaterBody> {
        self.largest_lake_within(f32::MAX)
    }

    /// Gives the lake with the largest area whose deepest place is
    /// nearer to the centre of the zone than `radius_m`.
    pub fn largest_lake_within(&self, radius_m: f32) -> Option<&WaterBody> {
        self.bodies
            .iter()
            .filter(|b| b.kind == WaterKind::Lake)
            .filter(|b| {
                let (x, z) = self.world(b.deepest_sample.0, b.deepest_sample.1);
                x * x + z * z < radius_m * radius_m
            })
            .max_by(|a, b| {
                a.wet_samples
                    .cmp(&b.wet_samples)
                    .then(b.number.cmp(&a.number))
            })
    }

    /// Gives a hash of all bodies. Use it to compare two zones.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |bits: u32| {
            for b in bits.to_le_bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        for b in &self.bodies {
            eat(b.number);
            eat(b.kind as u32);
            eat(b.level.to_bits());
            eat(b.wet_samples);
            eat(b.look as u32);
            eat(b.clarity_m.to_bits());
            eat(b.density.to_bits());
        }
        for &o in &self.owner {
            eat(u32::from(o));
        }
        for c in &self.river_cells {
            eat(c.body);
            eat(c.level.to_bits());
            eat(c.foam.to_bits());
            eat(c.flow[0].to_bits());
        }
        eat(self.river_steps.len() as u32);
        for f in &self.falls {
            eat(f.body);
            eat(f.height_m.to_bits());
            eat(f.width_m.to_bits());
        }
        h
    }

    /// Makes the ground damp along each shore: a darker colour just
    /// above the water level. `colours` has one colour for each sample.
    pub fn damp_shores(&self, heights: &HeightMap, colours: &mut [[u8; 3]]) {
        let damp_height_m = self.damp_height_m;
        if self.owner.is_empty() || damp_height_m <= 0.0 {
            return;
        }
        let size = self.size;
        // The damp band is at most a few samples wide.
        let reach = (damp_height_m.max(1.0) * 4.0 / self.spacing).ceil() as usize;
        // A river spans much of the zone: it has its own, faster pass.
        self.damp_river_shores(heights, colours, reach);
        for b in self.bodies.iter().filter(|b| b.kind != WaterKind::River) {
            let x0 = (b.min.0 as usize).saturating_sub(reach);
            let z0 = (b.min.1 as usize).saturating_sub(reach);
            let x1 = (b.max.0 as usize + reach).min(size - 1);
            let z1 = (b.max.1 as usize + reach).min(size - 1);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    let i = z * size + x;
                    if self.owner[i] != 0 {
                        continue;
                    }
                    let above = heights.data[i] - b.level;
                    if !(0.0..damp_height_m).contains(&above) {
                        continue;
                    }
                    // Only near the water of this body.
                    let id = b.number as usize + 1;
                    let near = (1..=reach as isize).any(|d| {
                        [
                            (1, 0),
                            (-1, 0),
                            (0, 1),
                            (0, -1),
                            (1, 1),
                            (-1, -1),
                            (1, -1),
                            (-1, 1),
                        ]
                        .iter()
                        .any(|&(dx, dz)| {
                            let (xx, zz) = (x as isize + dx * d, z as isize + dz * d);
                            xx >= 0
                                && zz >= 0
                                && (xx as usize) < size
                                && (zz as usize) < size
                                && self.owner[zz as usize * size + xx as usize] as usize == id
                        })
                    });
                    if !near {
                        continue;
                    }
                    let t = above / damp_height_m;
                    let dark = 0.62 + 0.38 * t * t;
                    for c in colours[i].iter_mut() {
                        *c = (*c as f32 * dark).round() as u8;
                    }
                }
            }
        }
    }
}

/// A fixed camera view of a lake for the Water view test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LakeView {
    /// The place of the camera in world space.
    pub eye: [f32; 3],
    /// The point that the camera looks at.
    pub target: [f32; 3],
    /// The length of water in front of the camera, in metres.
    pub fetch_m: f32,
}

impl Water {
    /// Gives a view across a body: the camera is over the water near
    /// the shore, `eye_height_m` above the level, and looks across the
    /// longest stretch of water. On a large lake, the water fills about
    /// half of the screen. A view that looks toward the centre of the
    /// zone (away from the smoke wall) gets up to 50% more score.
    pub fn view_across(&self, body: &WaterBody, eye_height_m: f32) -> LakeView {
        let id = body.number as usize + 1;
        let wet = |x: f32, z: f32| {
            let (xi, zi) = (x.round(), z.round());
            xi >= 0.0
                && zi >= 0.0
                && (xi as usize) < self.size
                && (zi as usize) < self.size
                && self.owner[zi as usize * self.size + xi as usize] as usize == id
        };
        let (cx, cz) = (body.deepest_sample.0 as f32, body.deepest_sample.1 as f32);
        // The distance along a direction until the water ends, in samples.
        let run = |x: f32, z: f32, dx: f32, dz: f32| {
            let mut d = 0.0;
            while wet(x + dx * (d + 1.0), z + dz * (d + 1.0)) && d < 4096.0 {
                d += 1.0;
            }
            d
        };
        // 16 directions, from a fixed table (no `sin` and `cos`).
        const H: f32 = std::f32::consts::FRAC_1_SQRT_2;
        const DIRS: [(f32, f32); 16] = [
            (1.0, 0.0),
            (0.9239, 0.3827),
            (H, H),
            (0.3827, 0.9239),
            (0.0, 1.0),
            (-0.3827, 0.9239),
            (-H, H),
            (-0.9239, 0.3827),
            (-1.0, 0.0),
            (-0.9239, -0.3827),
            (-H, -H),
            (-0.3827, -0.9239),
            (0.0, -1.0),
            (0.3827, -0.9239),
            (H, -H),
            (0.9239, -0.3827),
        ];
        let half = self.size as f32 * 0.5;
        let mut best = (-1.0f32, 0.0, (cx, cz), (1.0, 0.0));
        for (dx, dz) in DIRS {
            // Go from the deepest place to the shore, then back a little.
            let out = run(cx, cz, dx, dz);
            let back = (3.0 / self.spacing).min(out);
            let (ex, ez) = (cx + dx * (out - back), cz + dz * (out - back));
            let fetch = run(ex, ez, -dx, -dz);
            // The camera looks along (-dx, -dz). The centre of the zone
            // is at the sample (half, half).
            let (tx, tz) = (half - ex, half - ez);
            let length = (tx * tx + tz * tz).sqrt().max(1.0);
            let toward = (-dx * tx - dz * tz) / length;
            let score = fetch * (1.25 + 0.25 * toward);
            if score > best.0 {
                best = (score, fetch, (ex, ez), (-dx, -dz));
            }
        }
        let (_, fetch, (ex, ez), (dx, dz)) = best;
        let world = |s: f32| (s - half) * self.spacing;
        let look = (fetch * self.spacing).clamp(5.0, 60.0);
        LakeView {
            eye: [world(ex), body.level + eye_height_m, world(ez)],
            target: [world(ex) + dx * look, body.level, world(ez) + dz * look],
            fetch_m: fetch * self.spacing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drainage::partial_fill_basins;

    /// A bowl with a rim at 50 m and a deep part in the centre.
    fn bowl(size: usize) -> HeightMap {
        HeightMap::from_fn(size, 1.0, |x, z| {
            let d = ((x as f32 - 64.0).powi(2) + (z as f32 - 64.0).powi(2)).sqrt();
            if d < 40.0 {
                20.0 + d * 0.75
            } else {
                90.0 - d
            }
        })
    }

    /// The lakes of a map. The drainage runs on each `step`-th sample.
    fn lakes_step(map: &HeightMap, step: usize, seed: u32, params: &WaterParams) -> Water {
        let cs = map.size.div_ceil(step);
        let coarse: Vec<f32> = (0..cs * cs)
            .map(|i| map.get(((i % cs) * step) as isize, ((i / cs) * step) as isize))
            .collect();
        let r = partial_fill_basins(&coarse, cs, 1e-3, 6.0, 0.2);
        let cands: Vec<Candidate> = r
            .basins
            .iter()
            .filter(|b| b.deep)
            .map(|basin| Candidate {
                basin,
                kind: WaterKind::Lake,
            })
            .collect();
        make_water(map, &cands, cs, step, seed, params)
    }

    fn lakes_of(map: &HeightMap, seed: u32, params: &WaterParams) -> Water {
        lakes_step(map, 1, seed, params)
    }

    #[test]
    fn a_bowl_gets_a_lake_below_its_rim() {
        let map = bowl(129);
        let water = lakes_of(&map, 7, &WaterParams::default());
        assert_eq!(water.bodies.len(), 1);
        let lake = &water.bodies[0];
        assert_eq!(lake.kind, WaterKind::Lake);
        // The level is between the floor and the rim.
        assert!(
            lake.level > 20.0 && lake.level < 50.0,
            "level {}",
            lake.level
        );
        // Each wet sample is below the level, and the box holds it.
        for (i, &o) in water.owner.iter().enumerate() {
            if o != 0 {
                assert!(map.data[i] < lake.level);
            }
        }
        let wet = water.owner.iter().filter(|&&o| o != 0).count() as u32;
        assert_eq!(wet, lake.wet_samples);
        // The centre is wet, the rim is dry.
        assert!(water.body_at(0.0, 0.0).is_some());
        assert!(water.body_at(-62.0, 0.0).is_none());
        let here = water.at(&map, 0.5, 0.5).unwrap();
        assert!((here.depth - (lake.level - map.sample_world(0.5, 0.5))).abs() < 1e-4);
    }

    #[test]
    fn a_full_lake_does_not_leak_through_a_gap() {
        // A thin channel at 40 m through the rim, on an odd row. The
        // drainage grid (each second sample) does not see it.
        let base = bowl(129);
        let map = HeightMap::from_fn(129, 1.0, |x, z| {
            let h = base.data[z * 129 + x];
            if z == 65 && x > 70 {
                h.min(40.0)
            } else {
                h
            }
        });
        let params = WaterParams {
            full_lake_chance: 1.0,
            ..WaterParams::default()
        };
        let water = lakes_step(&map, 2, 3, &params);
        let lake = &water.bodies[0];
        assert!(lake.spill > 45.0, "coarse spill {}", lake.spill);
        assert!(lake.level < 40.0, "level {}", lake.level);
        // No wet sample is outside the rim.
        for (i, &o) in water.owner.iter().enumerate() {
            let (x, z) = ((i % 129) as f32 - 64.0, (i / 129) as f32 - 64.0);
            if o != 0 {
                assert!((x * x + z * z).sqrt() < 41.0);
            }
        }
    }

    #[test]
    fn same_seed_same_water() {
        let map = bowl(129);
        let p = WaterParams::default();
        let a = lakes_of(&map, 11, &p);
        let b = lakes_of(&map, 11, &p);
        assert_eq!(a.checksum(), b.checksum());
        // Other seeds give other levels or looks.
        let differs = (12..30).any(|s| lakes_of(&map, s, &p).checksum() != a.checksum());
        assert!(differs);
    }

    #[test]
    fn looks_are_in_range() {
        let map = bowl(129);
        for seed in 0..40 {
            let w = lakes_of(&map, seed, &WaterParams::default());
            for b in &w.bodies {
                assert!((0.2..=8.0).contains(&b.clarity_m), "{}", b.clarity_m);
                assert!((1.0..=1.6).contains(&b.density));
                assert!((0.0..=1.0).contains(&b.temperature));
                assert!((0.0..=1.0).contains(&b.viscosity));
                assert!(b.fill_part >= 0.2 - 1e-4 && b.fill_part <= 1.0);
            }
        }
    }

    #[test]
    fn the_view_looks_across_the_water() {
        let map = bowl(129);
        let water = lakes_of(&map, 7, &WaterParams::default());
        let lake = &water.bodies[0];
        let v = water.view_across(lake, 4.0);
        assert!((v.eye[1] - lake.level - 4.0).abs() < 1e-4);
        assert!(water.body_at(v.eye[0], v.eye[2]).is_some());
        assert!(v.fetch_m > 10.0, "fetch {}", v.fetch_m);
    }

    #[test]
    fn shores_get_damp() {
        let map = bowl(129);
        let water = lakes_of(&map, 7, &WaterParams::default());
        let mut colours = vec![[200u8; 3]; 129 * 129];
        water.damp_shores(&map, &mut colours);
        let dark = colours.iter().filter(|c| c[0] < 200).count();
        assert!(dark > 0);
        // The wet samples and the far ground keep their colour.
        assert_eq!(colours[0], [200; 3]);
        let lake = &water.bodies[0];
        let (x, z) = lake.deepest_sample;
        assert_eq!(colours[z as usize * 129 + x as usize], [200; 3]);
    }
}

#[cfg(test)]
mod zone_tests {
    use crate::highlands::{generate, HighlandsParams};
    use crate::water::WaterKind;

    #[test]
    fn the_test_seeds_have_lakes_and_the_rules_hold() {
        for seed in [1, 7, 42] {
            let zone = generate(seed, 2048, 1.0, &HighlandsParams::default());
            let w = &zone.water;
            assert!(
                w.largest_lake_within(580.0).is_some(),
                "seed {seed} has no lake inside the smoke wall"
            );
            // The water level is above the ground in each wet sample.
            for (i, &o) in w.owner.iter().enumerate() {
                if o != 0 {
                    let level = w.level_at_sample(i % w.size, i / w.size).unwrap();
                    assert!(zone.heights.data[i] < level, "seed {seed}");
                }
            }
            // Each test seed has a wet river inside the smoke wall
            // (`demo-2.md`, "Done when").
            assert!(
                w.river_view(580.0, 3.0).is_some(),
                "seed {seed} has no wet river inside the smoke wall"
            );
            // The bear does not start in the water.
            let (sx, sz) = zone.start_point();
            assert!(w.body_at(sx, sz).is_none(), "seed {seed}");
            for b in &w.bodies {
                assert!(b.max_depth_m >= 0.3 && b.area_m2 >= 25.0);
                assert!(b.level <= b.spill + 1e-3 || b.kind == WaterKind::Pond);
            }
        }
        // The same seed gives the same water.
        let p = HighlandsParams::default();
        let a = generate(7, 1024, 2.0, &p);
        let b = generate(7, 1024, 2.0, &p);
        assert_eq!(a.water.checksum(), b.water.checksum());
        assert!(!a.water.bodies.is_empty());
    }

    /// Prints the water of the test seeds (run with `--ignored --nocapture`).
    #[test]
    #[ignore]
    fn print_water_of_the_test_seeds() {
        for seed in [1, 7, 42, 3, 5, 9] {
            let t = std::time::Instant::now();
            let zone = generate(seed, 2048, 1.0, &HighlandsParams::default());
            let secs = t.elapsed().as_secs_f32();
            let w = &zone.water;
            let lakes: Vec<_> = w
                .bodies
                .iter()
                .filter(|b| b.kind == WaterKind::Lake)
                .collect();
            let rivers = w
                .bodies
                .iter()
                .filter(|b| b.kind == WaterKind::River)
                .count();
            let ponds = w.bodies.len() - lakes.len() - rivers;
            let area: f32 = w.bodies.iter().map(|b| b.area_m2).sum();
            let river_area: f32 = w
                .bodies
                .iter()
                .filter(|b| b.kind == WaterKind::River)
                .map(|b| b.area_m2)
                .sum();
            let rapids = w.river_steps.iter().filter(|s| !s.fall).count();
            let falls = w.river_steps.len() - rapids;
            println!(
                "seed {seed}: {secs:.2} s, {} lakes, {ponds} ponds, {rivers} rivers \
                 ({} cells, {rapids} rapids, {falls} falls, {river_area:.0} m2), {:.0} m2 of water",
                lakes.len(),
                w.river_cells.len(),
                area
            );
            for b in &w.bodies {
                let (x, z) = w.world(b.deepest_sample.0, b.deepest_sample.1);
                println!(
                    "  #{} {} {} at ({x:.0}, {z:.0}): {:.0} m2, deep {:.1} m, part {:.2}, \
                     clarity {:.1}, level {:.1} to {:.1}, flow {:.1} m/s",
                    b.number,
                    b.kind.name(),
                    b.look.name(),
                    b.area_m2,
                    b.max_depth_m,
                    b.fill_part,
                    b.clarity_m,
                    b.level,
                    b.spill,
                    b.flow_mps
                );
            }
            if let Some(v) = w.river_view(580.0, 3.0) {
                println!(
                    "  river view: river #{}, rapid {:.2} m, eye {:?}, target {:?}",
                    v.body, v.drop_m, v.eye, v.target
                );
            }
            let (sx, sz) = zone.start_point();
            println!("  start ({sx}, {sz})");
        }
    }
}
