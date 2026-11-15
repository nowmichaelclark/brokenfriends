//! Rivers with water (S-162, `plan/demo-2.md`, "Water bodies").
//!
//! The drainage step cuts a riverbed where much rain comes together. It
//! uses a second, wet flow: a full lake passes its water on at its spill
//! cell, and a closed lake keeps its water. This module fills each bed
//! whose wet area is more than `wet_min_area_m2` with water.
//!
//! - A river system is a tree of wet riverbed cells. It ends at its
//!   mouth: a lake, the border of the zone, or the end of the wet bed.
//! - The surface of a cell is its bed height plus a seeded part (0.4 to
//!   0.8) of its bed depth. Near a mouth, the surface goes down to the
//!   water below it. The surface is never higher than the bank beside it.
//!   It never rises downstream.
//! - A drop of more than 0.5 m from one cell to the next is a rapid. A
//!   drop of more than 2.5 m, or steeper than 45 degrees, is a waterfall
//!   (S-165 draws the falls).
//! - Each cell has a flow vector and a foam value for the water shader.
//!
//! The same seed always gives the same rivers.

use crate::heightmap::HeightMap;
use crate::water::{
    look_from_weights, look_values, rand, Water, WaterBody, WaterKind, WaterParams,
};

/// The weights of the looks of a river, in the order of
/// `crate::water::LOOKS`: clear, silty, milky, rust, algae, mud, tar.
/// Most rivers are silty brown (assumed, `demo-2.md`, "Look kinds").
pub(crate) const RIVER_LOOK_WEIGHTS: [f32; 7] = [1.5, 3.0, 0.8, 1.5, 0.5, 0.3, 0.0];

/// One coarse cell of a river with water.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverCell {
    /// The index of the river body in `Water::bodies`.
    pub body: u32,
    /// The sample at the centre of the cell (x, z).
    pub sample: (u32, u32),
    /// The height of the water surface in metres.
    pub level: f32,
    /// The height of the riverbed in metres.
    pub bed: f32,
    /// The flow of the water in metres each second (x, z).
    pub flow: [f32; 2],
    /// The foam from 0 to 1: rapids, falls and fast water.
    pub foam: f32,
    /// The index of the next cell downstream in `Water::river_cells`, or
    /// `u32::MAX` at the mouth.
    pub next: u32,
    /// The width of the riverbed at the cell, in metres.
    pub width_m: f32,
}

/// A rapid or a waterfall: a large drop from one river cell to the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverStep {
    /// The index of the river body in `Water::bodies`.
    pub body: u32,
    /// The upper cell (an index in `Water::river_cells`).
    pub from: u32,
    /// The lower cell.
    pub to: u32,
    /// The drop of the water surface in metres.
    pub drop_m: f32,
    /// `true` for a waterfall, `false` for a rapid.
    pub fall: bool,
}

/// The data of the drainage step that the rivers need. All lists are on
/// the coarse grid of the drainage step, except `stamp_cell`.
pub struct RiverInput<'a> {
    /// The number of coarse cells on each side.
    pub coarse_size: usize,
    /// Each coarse cell is this many samples.
    pub step: usize,
    /// For each cell, the cell that its water goes to (the wet flow).
    pub receiver: &'a [u32],
    /// All cells, from the highest to the lowest (the wet flow).
    pub order: &'a [u32],
    /// For each cell, the number of cells that drain through it (the wet
    /// flow).
    pub area_cells: &'a [f32],
    /// The width of the riverbed of each cell in metres (0: no bed).
    pub width: &'a [f32],
    /// The height of the riverbed of each cell.
    pub bed: &'a [f32],
    /// The depth of a riverbed, as a part of its width.
    pub depth_ratio: f32,
    /// For each sample of the full grid: the coarse cell whose riverbed
    /// shapes the ground there, or `u32::MAX`.
    pub stamp_cell: &'a [u32],
}

const H: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// 16 directions, from a fixed table (no `sin` and `cos`).
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

/// Adds the rivers to the water of a zone. Call it after the lakes and
/// ponds are made (`crate::water::make_water`), on the final heights.
pub fn add_rivers(
    water: &mut Water,
    heights: &HeightMap,
    inp: &RiverInput,
    seed: u32,
    p: &WaterParams,
) {
    let cs = inp.coarse_size;
    let n = cs * cs;
    let size = heights.size;
    let spacing = heights.spacing;
    let cell_m = spacing * inp.step as f32;
    let cell_area = cell_m * cell_m;
    if water.owner.is_empty() {
        water.owner = vec![0; size * size];
    }
    let sample_of = |c: usize| {
        (
            ((c % cs) * inp.step).min(size - 1),
            ((c / cs) * inp.step).min(size - 1),
        )
    };
    let sample_index = |c: usize| {
        let (x, z) = sample_of(c);
        z * size + x
    };
    let still = |c: usize| water.owner[sample_index(c)];
    let dist = |a: usize, b: usize| {
        if a % cs != b % cs && a / cs != b / cs {
            cell_m * std::f32::consts::SQRT_2
        } else {
            cell_m
        }
    };
    let receiver = |i: usize| inp.receiver[i] as usize;

    // The wet cells: a riverbed with enough wet area, not under a lake.
    let wet: Vec<bool> = (0..n)
        .map(|c| {
            inp.width[c] > 0.0
                && inp.area_cells[c] * cell_area >= p.wet_min_area_m2
                && still(c) == 0
        })
        .collect();

    // The systems. Go from the lowest cell to the highest cell: the
    // receiver of a cell has its system first.
    let mut sys = vec![u32::MAX; n];
    let mut mouths: Vec<usize> = Vec::new();
    for &i in inp.order.iter().rev() {
        let i = i as usize;
        if !wet[i] {
            continue;
        }
        let r = receiver(i);
        if r != i && wet[r] {
            sys[i] = sys[r];
        } else {
            sys[i] = mouths.len() as u32;
            mouths.push(i);
        }
    }
    if mouths.is_empty() {
        return;
    }
    let part: Vec<f32> = mouths
        .iter()
        .map(|&m| {
            p.river_part_min
                + (p.river_part_max - p.river_part_min) * rand(sample_index(m) as u32, seed, 20)
        })
        .collect();

    // The least depth of each cell: more on a steep bed.
    let least: Vec<f32> = (0..n)
        .map(|i| {
            let r = receiver(i);
            let drop = if r != i {
                (inp.bed[i] - inp.bed[r]).max(0.0)
            } else {
                0.0
            };
            p.river_min_depth_m + p.steep_depth_part * drop
        })
        .collect();

    // The levels. Near the mouth, the river gets less deep, so its surface
    // comes down to the water that it flows into.
    let mut level = vec![f32::NAN; n];
    let mut to_mouth = vec![f32::INFINITY; n];
    for &i in inp.order.iter().rev() {
        let i = i as usize;
        if !wet[i] {
            continue;
        }
        let r = receiver(i);
        let d = dist(i, r);
        to_mouth[i] = if r != i && wet[r] {
            to_mouth[r] + d
        } else if r != i && still(r) != 0 {
            d
        } else {
            f32::INFINITY
        };
        let full = part[sys[i] as usize] * inp.width[i] * inp.depth_ratio;
        let depth = full
            .min(p.river_min_depth_m + p.mouth_slope * to_mouth[i])
            .max(least[i]);
        level[i] = inp.bed[i] + depth;
    }

    // The water is never higher than the bank beside it: the lowest ground
    // just outside the riverbeds, to the left and the right of the flow.
    // The bank does not push the water below the bed: then the cascade of
    // the next pass would dry the river downstream.
    for i in (0..n).filter(|&i| wet[i]) {
        let r = receiver(i);
        if r == i {
            continue;
        }
        let (sx, sz) = sample_of(i);
        let (fx, fz) = (
            (r % cs) as f32 - (i % cs) as f32,
            (r / cs) as f32 - (i / cs) as f32,
        );
        let flen = (fx * fx + fz * fz).sqrt();
        let w = inp.width[i];
        let ring = (w * 0.5 + w * 0.9 + 3.0 * spacing) / spacing;
        let mut bank = f32::INFINITY;
        for (dx, dz) in DIRS {
            // Only the sides: the ground up and down the river is not a bank.
            if (dx * fx + dz * fz).abs() > 0.4 * flen {
                continue;
            }
            let x = (sx as f32 + dx * ring).round();
            let z = (sz as f32 + dz * ring).round();
            if x < 0.0 || z < 0.0 || x as usize >= size || z as usize >= size {
                continue;
            }
            let s = z as usize * size + x as usize;
            if inp.stamp_cell[s] == u32::MAX {
                bank = bank.min(heights.data[s]);
            }
        }
        if bank.is_finite() {
            level[i] = level[i]
                .min(bank - p.bank_margin_m)
                .max(inp.bed[i] + least[i]);
        }
    }

    // A river that leaves a full lake starts at the lake level.
    for j in 0..n {
        let r = receiver(j);
        if r != j && wet[r] && !wet[j] && still(j) != 0 {
            let lake = water.bodies[still(j) as usize - 1].level;
            level[r] = level[r].min(lake);
        }
    }

    // The surface never rises downstream. Go from the highest cell to the
    // lowest cell: each cell has all its upstream levels first.
    for &i in inp.order {
        let i = i as usize;
        let r = receiver(i);
        if wet[i] && r != i && wet[r] {
            level[r] = level[r].min(level[i]);
        }
    }

    // The flow: the direction to the receiver, and a speed from the slope
    // of the surface. Each cell averages its velocity with the cells
    // before and after it, so the flow turns smoothly.
    let mut velocity = vec![[0.0f32; 2]; n];
    for i in (0..n).filter(|&i| wet[i]) {
        let r = receiver(i);
        if r == i {
            continue;
        }
        let d = dist(i, r);
        let dx = (r % cs) as f32 - (i % cs) as f32;
        let dz = (r / cs) as f32 - (i / cs) as f32;
        let len = (dx * dx + dz * dz).sqrt().max(1e-6);
        let slope = if wet[r] {
            ((level[i] - level[r]) / d).max(0.0)
        } else {
            ((inp.bed[i] - inp.bed[r]) / d).max(0.0)
        };
        let speed =
            (p.flow_min_mps + p.flow_slope_k * slope.sqrt()).clamp(p.flow_min_mps, p.flow_max_mps);
        velocity[i] = [dx / len * speed, dz / len * speed];
    }
    let mut smooth = velocity.clone();
    let mut weight = vec![1.0f32; n];
    for i in (0..n).filter(|&i| wet[i]) {
        let r = receiver(i);
        if r != i && wet[r] {
            for k in 0..2 {
                smooth[i][k] += velocity[r][k];
                smooth[r][k] += velocity[i][k];
            }
            weight[i] += 1.0;
            weight[r] += 1.0;
        }
    }
    for i in (0..n).filter(|&i| wet[i]) {
        smooth[i] = [smooth[i][0] / weight[i], smooth[i][1] / weight[i]];
    }

    // Rapids and falls. Both cells of a large drop get foam: a rapid of
    // 0.5 m gets 0.2, one of 2.5 m gets 0.7, a fall gets 1. Fast water
    // gets a little foam.
    let mut foam = vec![0.0f32; n];
    let mut steps: Vec<(usize, usize, f32, bool)> = Vec::new();
    for i in (0..n).filter(|&i| wet[i]) {
        let speed = (smooth[i][0] * smooth[i][0] + smooth[i][1] * smooth[i][1]).sqrt();
        foam[i] = foam[i].max(((speed - 2.5) * 0.2).clamp(0.0, 0.3));
        let r = receiver(i);
        if r == i || !wet[r] {
            continue;
        }
        let drop = level[i] - level[r];
        if drop > p.rapid_drop_m {
            let fall = drop > p.fall_drop_m || drop / dist(i, r) > p.fall_slope;
            let f = if fall {
                1.0
            } else {
                (0.2 + (drop - p.rapid_drop_m) * 0.25).clamp(0.2, 0.7)
            };
            foam[i] = foam[i].max(f);
            foam[r] = foam[r].max(f);
            steps.push((i, r, drop, fall));
        }
    }

    // The wet samples: the samples of a wet riverbed below its level.
    let systems = mouths.len();
    let mut count = vec![0u32; systems];
    let mut cells_of = vec![0u32; systems];
    let mut deepest = vec![(0.0f32, 0usize); systems];
    for i in (0..n).filter(|&i| wet[i]) {
        cells_of[sys[i] as usize] += 1;
    }
    let mut samples: Vec<(u32, u32)> = Vec::new();
    for (s, &c) in inp.stamp_cell.iter().enumerate() {
        if c == u32::MAX {
            continue;
        }
        let c = c as usize;
        if !wet[c] || water.owner[s] != 0 {
            continue;
        }
        let depth = level[c] - heights.data[s];
        if depth <= 0.0 {
            continue;
        }
        let k = sys[c] as usize;
        count[k] += 1;
        if depth > deepest[k].0 {
            deepest[k] = (depth, s);
        }
        samples.push((s as u32, c as u32));
    }

    // Keep the systems that are large enough. They get their numbers in
    // the order of their mouths.
    let mut kept: Vec<usize> = (0..systems)
        .filter(|&k| {
            cells_of[k] >= p.river_min_cells
                && count[k] as f32 * spacing * spacing >= p.min_area_m2
                && deepest[k].0 >= p.min_depth_m
        })
        .collect();
    kept.sort_by_key(|&k| sample_index(mouths[k]));
    let room = (u16::MAX as usize - 1).saturating_sub(water.bodies.len());
    kept.truncate(room);
    let first = water.bodies.len();
    let mut body_of = vec![u32::MAX; systems];
    for (b, &k) in kept.iter().enumerate() {
        body_of[k] = (first + b) as u32;
    }

    // The river cells, in the order of their cell indexes.
    let mut compact = vec![u32::MAX; n];
    let mut cells: Vec<RiverCell> = Vec::new();
    for i in (0..n).filter(|&i| wet[i] && body_of[sys[i] as usize] != u32::MAX) {
        compact[i] = cells.len() as u32;
        let (x, z) = sample_of(i);
        cells.push(RiverCell {
            body: body_of[sys[i] as usize],
            sample: (x as u32, z as u32),
            level: level[i],
            bed: inp.bed[i],
            flow: smooth[i],
            foam: foam[i],
            next: u32::MAX,
            width_m: inp.width[i],
        });
    }
    for i in (0..n).filter(|&i| compact[i] != u32::MAX) {
        let r = receiver(i);
        if r != i && compact[r] != u32::MAX {
            cells[compact[i] as usize].next = compact[r];
        }
    }

    // The bodies.
    for (b, &k) in kept.iter().enumerate() {
        let number = (first + b) as u32;
        let mouth = mouths[k];
        let key = sample_index(mouth) as u32;
        let look = look_from_weights(&RIVER_LOOK_WEIGHTS, key, seed);
        let v = look_values(look, key, seed);
        let (mx, mz) = sample_of(mouth);
        let (ds, dz) = (deepest[k].1 % size, deepest[k].1 / size);
        let mine = cells.iter().filter(|c| c.body == number);
        let (mut top, mut speed, mut m) = (f32::MIN, 0.0f32, 0u32);
        for c in mine {
            top = top.max(c.level);
            speed += (c.flow[0] * c.flow[0] + c.flow[1] * c.flow[1]).sqrt();
            m += 1;
        }
        water.bodies.push(WaterBody {
            number,
            kind: WaterKind::River,
            level: level[mouth],
            spill: top,
            spill_sample: (mx as u32, mz as u32),
            deepest_sample: (ds as u32, dz as u32),
            min: (u32::MAX, u32::MAX),
            max: (0, 0),
            wet_samples: count[k],
            area_m2: count[k] as f32 * spacing * spacing,
            max_depth_m: deepest[k].0,
            fill_part: part[k],
            look,
            deep_colour: v.deep_colour,
            clarity_m: v.clarity_m,
            viscosity: look.viscosity(),
            density: v.density,
            temperature: v.temperature,
            flow_mps: speed / m.max(1) as f32,
        });
    }

    // The samples: the owner, the box of each body and the cell lists.
    samples.retain(|&(_, c)| compact[c as usize] != u32::MAX);
    samples.sort_unstable();
    for &(s, c) in &samples {
        let body = cells[compact[c as usize] as usize].body as usize;
        water.owner[s as usize] = body as u16 + 1;
        let b = &mut water.bodies[body];
        let (x, z) = (s % size as u32, s / size as u32);
        b.min = (b.min.0.min(x), b.min.1.min(z));
        b.max = (b.max.0.max(x), b.max.1.max(z));
    }
    water.river_samples = samples.iter().map(|&(s, _)| s).collect();
    water.river_sample_cell = samples.iter().map(|&(_, c)| compact[c as usize]).collect();
    water.river_steps = steps
        .into_iter()
        .filter(|&(i, r, _, _)| compact[i] != u32::MAX && compact[r] != u32::MAX)
        .map(|(i, r, drop_m, fall)| RiverStep {
            body: cells[compact[i] as usize].body,
            from: compact[i],
            to: compact[r],
            drop_m,
            fall,
        })
        .collect();
    water.river_cells = cells;
    water.falls = crate::waterfalls::make_falls(water, &water.river_cells, &water.river_steps, p);
}

/// The Water view test prefers a rapid with a drop up to this, in metres.
const MODERATE_RAPID_M: f32 = 1.2;

/// A fixed camera view of a river for the Water view test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiverView {
    /// The place of the camera in world space.
    pub eye: [f32; 3],
    /// The point that the camera looks at.
    pub target: [f32; 3],
    /// The index of the river body in `Water::bodies`.
    pub body: u32,
    /// The drop of the rapid in the view in metres (0: no rapid).
    pub drop_m: f32,
}

impl Water {
    /// Gives the world position of a river cell, at its level.
    pub fn river_cell_world(&self, cell: &RiverCell) -> [f32; 3] {
        let (x, z) = self.world(cell.sample.0, cell.sample.1);
        [x, cell.level, z]
    }

    /// Gives a view of a river for the Water view test: the camera is
    /// `eye_height_m` above the water, about 14 m below a rapid nearer to
    /// the centre than `radius_m`, and looks up the river at the rapid.
    /// The rapid is on the largest river that has one; a rapid of 1.2 m
    /// or less comes first. If no rapid is near, the view looks up the largest
    /// river at its deepest cell.
    pub fn river_view(&self, radius_m: f32, eye_height_m: f32) -> Option<RiverView> {
        let near = |c: &RiverCell| {
            let [x, _, z] = self.river_cell_world(c);
            x * x + z * z < radius_m * radius_m
        };
        // A moderate rapid shows a river; a large one is nearly a fall.
        let rapids = |max_drop: f32| {
            self.river_steps.iter().filter(move |s| {
                !s.fall && s.drop_m <= max_drop && near(&self.river_cells[s.from as usize])
            })
        };
        let pool = if rapids(MODERATE_RAPID_M).next().is_some() {
            MODERATE_RAPID_M
        } else {
            f32::MAX
        };
        let rapid = rapids(pool).max_by(|a, b| {
            let area = |s: &RiverStep| self.bodies[s.body as usize].area_m2;
            area(a)
                .total_cmp(&area(b))
                .then(a.drop_m.total_cmp(&b.drop_m))
                .then(b.from.cmp(&a.from))
        });
        let (top, drop_m) = match rapid {
            Some(s) => (s.from, s.drop_m),
            None => {
                let body = self
                    .bodies
                    .iter()
                    .filter(|b| b.kind == WaterKind::River)
                    .filter(|b| {
                        self.river_cells
                            .iter()
                            .any(|c| c.body == b.number && near(c))
                    })
                    .max_by(|a, b| a.area_m2.total_cmp(&b.area_m2))?;
                let i = self
                    .river_cells
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.body == body.number && near(c) && c.next != u32::MAX)
                    .max_by(|(_, a), (_, b)| (a.level - a.bed).total_cmp(&(b.level - b.bed)))?
                    .0;
                (i as u32, 0.0)
            }
        };
        let target = self.river_cell_world(&self.river_cells[top as usize]);
        // Go downstream about 14 m for the camera.
        let mut at = top;
        let mut walked = 0.0;
        while walked < 14.0 {
            let next = self.river_cells[at as usize].next;
            if next == u32::MAX {
                break;
            }
            let (a, b) = (
                self.river_cell_world(&self.river_cells[at as usize]),
                self.river_cell_world(&self.river_cells[next as usize]),
            );
            let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
            walked += (dx * dx + dz * dz).sqrt();
            at = next;
        }
        let mut eye = self.river_cell_world(&self.river_cells[at as usize]);
        if walked < 4.0 {
            // The mouth is near: look from the other side, along the flow.
            let f = self.river_cells[top as usize].flow;
            let len = (f[0] * f[0] + f[1] * f[1]).sqrt().max(1e-3);
            eye = [
                target[0] + f[0] / len * 12.0,
                target[1],
                target[2] + f[1] / len * 12.0,
            ];
        }
        // The camera is above the water at both ends of the view.
        eye[1] = eye[1].max(target[1]) + eye_height_m;
        Some(RiverView {
            eye,
            target,
            body: self.river_cells[top as usize].body,
            drop_m,
        })
    }

    /// Makes the ground damp along the river banks. It darkens the dry
    /// samples near the edge of the water that are less than
    /// `damp_height_m` above the level of the water next to them.
    pub(crate) fn damp_river_shores(
        &self,
        heights: &HeightMap,
        colours: &mut [[u8; 3]],
        reach: usize,
    ) {
        let damp = self.damp_height_m;
        if self.river_samples.is_empty() || damp <= 0.0 {
            return;
        }
        let size = self.size;
        let r = reach as isize;
        // For each damp sample: the darkest factor. The order of the
        // samples does not change the result.
        let mut dark: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        for (&s, &c) in self.river_samples.iter().zip(&self.river_sample_cell) {
            let s = s as usize;
            let (x, z) = ((s % size) as isize, (s / size) as isize);
            let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| {
                let (xx, zz) = (x + dx, z + dz);
                xx >= 0
                    && zz >= 0
                    && (xx as usize) < size
                    && (zz as usize) < size
                    && self.owner[zz as usize * size + xx as usize] == 0
            });
            if !edge {
                continue;
            }
            let level = self.river_cells[c as usize].level;
            for dz in -r..=r {
                for dx in -r..=r {
                    let (xx, zz) = (x + dx, z + dz);
                    if xx < 0 || zz < 0 || xx as usize >= size || zz as usize >= size {
                        continue;
                    }
                    let i = zz as usize * size + xx as usize;
                    if self.owner[i] != 0 {
                        continue;
                    }
                    let above = heights.data[i] - level;
                    if !(0.0..damp).contains(&above) {
                        continue;
                    }
                    let t = above / damp;
                    let f = 0.62 + 0.38 * t * t;
                    let e = dark.entry(i).or_insert(1.0);
                    *e = e.min(f);
                }
            }
        }
        for (i, f) in dark {
            for c in colours[i].iter_mut() {
                *c = (*c as f32 * f).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::highlands::{generate, HighlandsParams};
    use crate::water::WaterKind;

    #[test]
    fn rivers_follow_the_rules() {
        let p = HighlandsParams::default();
        let zone = generate(7, 1024, 2.0, &p);
        let w = &zone.water;
        assert!(!w.river_cells.is_empty(), "seed 7 has no wet river");
        for c in &w.river_cells {
            assert_eq!(w.bodies[c.body as usize].kind, WaterKind::River);
            // The surface never rises downstream.
            if c.next != u32::MAX {
                let n = &w.river_cells[c.next as usize];
                assert!(n.level <= c.level, "{} rises to {}", c.level, n.level);
                assert_eq!(n.body, c.body);
            }
            assert!((0.0..=1.0).contains(&c.foam));
            let speed = (c.flow[0] * c.flow[0] + c.flow[1] * c.flow[1]).sqrt();
            assert!(speed <= p.water.flow_max_mps + 1e-3);
        }
        // Each wet river sample is below its level and has its body.
        for (&s, &c) in w.river_samples.iter().zip(&w.river_sample_cell) {
            let cell = &w.river_cells[c as usize];
            assert!(zone.heights.data[s as usize] < cell.level);
            assert_eq!(w.owner[s as usize] as u32, cell.body + 1);
        }
        for s in &w.river_steps {
            assert!(s.drop_m > p.water.rapid_drop_m);
        }
        // The same seed gives the same rivers.
        let again = generate(7, 1024, 2.0, &p);
        assert_eq!(again.water.checksum(), w.checksum());
    }

    #[test]
    fn the_river_level_is_found_at_a_wet_sample() {
        let zone = generate(7, 1024, 2.0, &HighlandsParams::default());
        let w = &zone.water;
        let s = w.river_samples[w.river_samples.len() / 2] as usize;
        let (x, z) = (s % w.size, s / w.size);
        let cell = w.river_cell_at_sample(x, z).unwrap();
        assert_eq!(w.level_at_sample(x, z), Some(cell.level));
        let (wx, wz) = w.world(x as u32, z as u32);
        let here = w.at(&zone.heights, wx, wz).unwrap();
        assert_eq!(here.body.kind, WaterKind::River);
        assert_eq!(here.level, cell.level);
        assert_eq!(here.flow, cell.flow);
        let v = w.river_view(4000.0, 3.0).unwrap();
        assert!(v.eye[1] > v.target[1]);
    }
}

#[cfg(test)]
mod sweep {
    use crate::highlands::{generate, HighlandsParams};

    /// Prints the river cells for some wet areas (run with `--ignored
    /// --nocapture`).
    #[test]
    #[ignore]
    fn print_rivers_for_wet_areas() {
        for seed in [1, 7, 42] {
            for area in [30_000.0, 50_000.0, 150_000.0] {
                let mut p = HighlandsParams::default();
                p.water.wet_min_area_m2 = area;
                let zone = generate(seed, 2048, 1.0, &p);
                let w = &zone.water;
                let near = w
                    .river_cells
                    .iter()
                    .filter(|c| {
                        let [x, _, z] = w.river_cell_world(c);
                        x * x + z * z < 580.0 * 580.0
                    })
                    .count();
                let rapids = w.river_steps.iter().filter(|s| !s.fall).count();
                let falls = w.river_steps.len() - rapids;
                let bodies = w
                    .bodies
                    .iter()
                    .filter(|b| b.kind == crate::water::WaterKind::River)
                    .count();
                // How many steps have the same drop in the bed?
                let bed_steps = w
                    .river_steps
                    .iter()
                    .filter(|s| {
                        let (a, b) = (
                            &w.river_cells[s.from as usize],
                            &w.river_cells[s.to as usize],
                        );
                        a.bed - b.bed > 0.5 * s.drop_m
                    })
                    .count();
                let mut depths: Vec<f32> = w.river_cells.iter().map(|c| c.level - c.bed).collect();
                depths.sort_by(f32::total_cmp);
                let q = |f: f32| {
                    depths
                        .get((depths.len() as f32 * f) as usize)
                        .copied()
                        .unwrap_or(0.0)
                };
                println!(
                    "  steps with a bed drop: {bed_steps}; depth over bed q10 {:.2} q50 {:.2} q90 {:.2}",
                    q(0.1),
                    q(0.5),
                    q(0.9)
                );
                println!(
                    "seed {seed} wet {area}: {bodies} rivers, {} cells ({near} near), {} samples, {rapids} rapids, {falls} falls",
                    w.river_cells.len(),
                    w.river_samples.len()
                );
            }
        }
    }
}
