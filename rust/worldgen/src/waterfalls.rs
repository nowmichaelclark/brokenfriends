//! Waterfalls (S-165, `plan/demo-2.md`, "Waterfalls").
//!
//! `rivers.rs` keeps each drop of more than 2.5 m (or steeper than 45
//! degrees) from one river cell to the next as a fall step. This module
//! joins the fall steps that follow each other into one waterfall. It
//! gives each fall a path, a width, a height and a direction for the
//! game to draw (`water_mesh`-like code in `waterfall_mesh.rs`) and to
//! push the bear.
//!
//! Each zone tries to have one tall fall (40 m or more) and one wide
//! fall (30 m or more). The zone does not carve a cliff (assumed, see
//! `game/README.md`). The tall fall is the highest natural fall if it is
//! high enough. The wide fall is the best natural fall (the one with
//! most height and water) that is at least `wide_min_height_m` high. The
//! game draws it wider than its river, and only where the ground is
//! low enough for water. If no fall is good, the zone has none of that
//! kind. `Water::falls_report` says so.
//!
//! The same seed always gives the same falls.

use crate::heightmap::HeightMap;
use crate::rivers::{RiverCell, RiverStep};
use crate::water::{Water, WaterKind, WaterParams};
use std::collections::HashMap;

/// A part of the river bed width that the water of a fall covers.
const WATER_PART_OF_BED: f32 = 0.8;

/// The least width of a fall in metres.
const MIN_WIDTH_M: f32 = 2.0;

/// A body with at least this viscosity falls as a few ropes (assumed).
pub const THICK_VISCOSITY: f32 = 0.5;

/// The kind of a fall: a plain fall of a river, or the feature of the
/// zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallFeature {
    /// A river step that is a fall.
    Plain,
    /// The tall fall of the zone: 40 m or more.
    Tall,
    /// The wide fall of the zone: drawn 30 m wide or more.
    Wide,
}

impl FallFeature {
    pub fn name(self) -> &'static str {
        match self {
            FallFeature::Plain => "plain",
            FallFeature::Tall => "tall",
            FallFeature::Wide => "wide",
        }
    }
}

/// One waterfall: one or more fall steps of a river that follow each
/// other.
#[derive(Clone, Debug, PartialEq)]
pub struct Waterfall {
    /// The number of the fall in `Water::falls`.
    pub number: u32,
    /// The index of the river body in `Water::bodies`.
    pub body: u32,
    /// The points of the fall from the top to the bottom: the world
    /// position (x, y, z) of each river cell, with the water level as y.
    /// At least 2 points.
    pub path: Vec<[f32; 3]>,
    /// The width of the water in metres.
    pub width_m: f32,
    /// The drop of the water surface in metres.
    pub height_m: f32,
    /// The horizontal length of the path in metres.
    pub length_m: f32,
    /// The horizontal direction from the top to the bottom (unit).
    pub dir: [f32; 2],
    /// The flow of the river above the fall, in metres each second.
    pub flow_mps: f32,
    /// A thick body (mud, tar) falls as a few ropes, slowly, with no
    /// mist.
    pub thick: bool,
    /// The deep colour of the water (sRGB, 0 to 1).
    pub colour: [f32; 3],
    pub feature: FallFeature,
}

impl Waterfall {
    /// The top of the fall: the lip.
    pub fn top(&self) -> [f32; 3] {
        self.path[0]
    }

    /// The bottom of the fall: the water level of the pool.
    pub fn bottom(&self) -> [f32; 3] {
        self.path[self.path.len() - 1]
    }

    /// Gives the point of the path at a distance from the top, in
    /// metres along the horizontal path. It gives the position, the
    /// horizontal direction there and the water level.
    pub fn at_distance(&self, d: f32) -> ([f32; 3], [f32; 2]) {
        let mut left = d.max(0.0);
        for w in self.path.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
            let len = (dx * dx + dz * dz).sqrt();
            if len < 1e-4 {
                continue;
            }
            if left <= len {
                let t = left / len;
                return (
                    [a[0] + dx * t, a[1] + (b[1] - a[1]) * t, a[2] + dz * t],
                    [dx / len, dz / len],
                );
            }
            left -= len;
        }
        (self.bottom(), self.dir)
    }
}

/// How hard a fall pushes the bear (a place in the curtain).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallPush {
    /// The unit direction of the push: along the fall and down.
    pub direction: [f32; 3],
    /// A number from 0 to 1: the share of the full push. It is 1 in the
    /// middle of the curtain and falls to 0 at its edge.
    pub strength: f32,
    /// The number of the fall.
    pub fall: u32,
    /// `true` for a thick fall (the push is weaker).
    pub thick: bool,
}

/// A fixed camera view of a fall for the Water view test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallView {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    /// The number of the fall in `Water::falls`.
    pub fall: u32,
}

/// Joins the fall steps into falls and chooses the tall and the wide
/// fall of the zone. `cells` are the river cells; `colour_of` and
/// `thick_of` give the look of a body.
pub(crate) fn make_falls(
    water: &Water,
    cells: &[RiverCell],
    steps: &[RiverStep],
    p: &WaterParams,
) -> Vec<Waterfall> {
    let falls: Vec<&RiverStep> = steps.iter().filter(|s| s.fall).collect();
    // The fall step that starts at a cell.
    let mut start_at: HashMap<u32, &RiverStep> = HashMap::new();
    let mut is_inner = std::collections::HashSet::new();
    for s in &falls {
        start_at.insert(s.from, s);
        is_inner.insert(s.to);
    }
    let mut starts: Vec<&RiverStep> = falls
        .iter()
        .copied()
        .filter(|s| !is_inner.contains(&s.from))
        .collect();
    starts.sort_by_key(|s| s.from);

    let mut out: Vec<Waterfall> = Vec::new();
    for first in starts {
        let mut ids = vec![first.from, first.to];
        let mut at = first.to;
        while let Some(next) = start_at.get(&at) {
            // A fall that joins into a loop cannot happen: the levels
            // never rise. This guard stops a bad cycle.
            if ids.len() > 64 {
                break;
            }
            ids.push(next.to);
            at = next.to;
        }
        let path: Vec<[f32; 3]> = ids
            .iter()
            .map(|&c| water.river_cell_world(&cells[c as usize]))
            .collect();
        let top = path[0];
        let bottom = path[path.len() - 1];
        let height = top[1] - bottom[1];
        let mut length = 0.0;
        for w in path.windows(2) {
            let (dx, dz) = (w[1][0] - w[0][0], w[1][2] - w[0][2]);
            length += (dx * dx + dz * dz).sqrt();
        }
        let (dx, dz) = (bottom[0] - top[0], bottom[2] - top[2]);
        let dl = (dx * dx + dz * dz).sqrt();
        let dir = if dl > 1e-3 {
            [dx / dl, dz / dl]
        } else {
            [1.0, 0.0]
        };
        let bed: f32 =
            ids.iter().map(|&c| cells[c as usize].width_m).sum::<f32>() / ids.len() as f32;
        let body = &water.bodies[first.body as usize];
        out.push(Waterfall {
            number: out.len() as u32,
            body: first.body,
            path,
            width_m: (bed * WATER_PART_OF_BED).max(MIN_WIDTH_M),
            height_m: height,
            length_m: length.max(1.0),
            dir,
            flow_mps: cells[first.from as usize].flow[0].hypot(cells[first.from as usize].flow[1]),
            thick: body.viscosity >= THICK_VISCOSITY,
            colour: body.deep_colour,
            feature: FallFeature::Plain,
        });
    }
    choose_features(&mut out, p);
    out
}

/// Marks the tall fall and the wide fall of the zone.
fn choose_features(falls: &mut [Waterfall], p: &WaterParams) {
    // The tall fall: the highest fall, if it is high enough.
    let tall = falls
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.height_m
                .total_cmp(&b.height_m)
                .then(b.number.cmp(&a.number))
        })
        .map(|(i, _)| i);
    if let Some(i) = tall.filter(|&i| falls[i].height_m >= p.tall_fall_m) {
        falls[i].feature = FallFeature::Tall;
    }
    // The wide fall: the best of the other falls by height and water.
    let wide = falls
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.feature == FallFeature::Plain && !f.thick && f.height_m >= p.wide_min_height_m
        })
        .max_by(|(_, a), (_, b)| {
            (a.height_m * a.width_m)
                .total_cmp(&(b.height_m * b.width_m))
                .then(b.number.cmp(&a.number))
        })
        .map(|(i, _)| i);
    if let Some(i) = wide {
        falls[i].feature = FallFeature::Wide;
        falls[i].width_m = falls[i].width_m.max(p.wide_fall_m);
    }
}

impl Water {
    /// Gives the push of the falls at a place (the bear is in the
    /// curtain), if there is one. `y` is the height of the bear.
    pub fn fall_push_at(&self, x: f32, y: f32, z: f32) -> Option<FallPush> {
        let mut best: Option<FallPush> = None;
        for f in &self.falls {
            let half = f.width_m * 0.5;
            // A quick test with the box of the fall.
            let (t, b) = (f.top(), f.bottom());
            let reach = half + f.length_m;
            if (x - t[0]).abs() > reach || (z - t[2]).abs() > reach {
                continue;
            }
            if y > t[1] + 2.0 || y < b[1] - 1.0 {
                continue;
            }
            // The nearest place on the path.
            let mut near: Option<(f32, f32, f32, [f32; 2])> = None;
            let mut run = 0.0;
            for w in f.path.windows(2) {
                let (a, c) = (w[0], w[1]);
                let (dx, dz) = (c[0] - a[0], c[2] - a[2]);
                let len2 = dx * dx + dz * dz;
                if len2 < 1e-6 {
                    continue;
                }
                let len = len2.sqrt();
                let u = (((x - a[0]) * dx + (z - a[2]) * dz) / len2).clamp(0.0, 1.0);
                let (px, pz) = (a[0] + dx * u, a[2] + dz * u);
                let d = ((x - px).powi(2) + (z - pz).powi(2)).sqrt();
                let level = a[1] + (c[1] - a[1]) * u;
                if near.is_none_or(|n| d < n.0) {
                    near = Some((d, level, run + len * u, [dx / len, dz / len]));
                }
                run += len;
            }
            let Some((d, level, along, dir)) = near else {
                continue;
            };
            // The curtain is near the water level of its place: the bear
            // is in it from a little under to a little over that.
            if d > half || y > level + 2.0 || y < level - 1.5 {
                continue;
            }
            let _ = along;
            let strength = (1.0 - d / half.max(0.1)).clamp(0.0, 1.0).sqrt();
            let steep = (f.height_m / f.length_m).clamp(0.3, 3.0);
            let down = (steep / (1.0 + steep)).clamp(0.2, 0.8);
            let h = (1.0 - down * down).sqrt();
            let push = FallPush {
                direction: [dir[0] * h, -down, dir[1] * h],
                strength,
                fall: f.number,
                thick: f.thick,
            };
            if best.is_none_or(|b| push.strength > b.strength) {
                best = Some(push);
            }
        }
        best
    }

    /// Gives the camera view of a fall for the Water view test, 15 m
    /// from the fall: downstream of its pool, looking at its middle.
    /// `tallest` selects the highest fall, else the widest fall. The fall
    /// must be nearer to the centre of the zone than `radius_m`.
    pub fn fall_view(
        &self,
        heights: &HeightMap,
        radius_m: f32,
        tallest: bool,
        distance_m: f32,
    ) -> Option<FallView> {
        let inside = |f: &&Waterfall| {
            let t = f.top();
            t[0] * t[0] + t[2] * t[2] < radius_m * radius_m
        };
        let f = self
            .falls
            .iter()
            .filter(inside)
            .filter(|f| f.length_m < 80.0)
            .max_by(|a, b| {
                let key = |f: &Waterfall| {
                    if tallest {
                        f.height_m
                    } else {
                        f.width_m
                    }
                };
                key(a)
                    .total_cmp(&key(b))
                    .then(a.height_m.total_cmp(&b.height_m))
                    .then(b.number.cmp(&a.number))
            })?;
        let bottom = f.bottom();
        let top = f.top();
        let mid = [
            (top[0] + bottom[0]) * 0.5,
            (top[1] + bottom[1]) * 0.5,
            (top[2] + bottom[2]) * 0.5,
        ];
        // Try the downstream side first, then the other sides: the eye
        // is above the ground there.
        let mut eye = None;
        for k in 0..8 {
            let turn = k as f32 * std::f32::consts::FRAC_PI_4;
            let (s, c) = turn.sin_cos();
            let (dx, dz) = (f.dir[0] * c - f.dir[1] * s, f.dir[0] * s + f.dir[1] * c);
            let (ex, ez) = (bottom[0] + dx * distance_m, bottom[2] + dz * distance_m);
            let ground = heights.sample_world(ex, ez);
            if ground < bottom[1] + 4.0 {
                eye = Some([ex, ground.max(bottom[1]) + 2.5, ez]);
                break;
            }
        }
        let eye = eye.unwrap_or_else(|| {
            let (ex, ez) = (
                bottom[0] + f.dir[0] * distance_m,
                bottom[2] + f.dir[1] * distance_m,
            );
            [ex, heights.sample_world(ex, ez).max(bottom[1]) + 3.0, ez]
        });
        Some(FallView {
            eye,
            target: mid,
            fall: f.number,
        })
    }

    /// Tells about the falls of the zone: how many, the tall fall and
    /// the wide fall, or why they are missing.
    pub fn falls_report(&self, p: &WaterParams) -> String {
        let tall = self.falls.iter().find(|f| f.feature == FallFeature::Tall);
        let wide = self.falls.iter().find(|f| f.feature == FallFeature::Wide);
        let best = self.falls.iter().map(|f| f.height_m).fold(0.0f32, f32::max);
        let text = |f: Option<&Waterfall>, none: String| match f {
            Some(f) => {
                let t = f.top();
                format!(
                    "{:.0} m high, {:.0} m wide at ({:.0}, {:.0})",
                    f.height_m, f.width_m, t[0], t[2]
                )
            }
            None => none,
        };
        format!(
            "{} waterfalls; tall fall: {}; wide fall: {}",
            self.falls.len(),
            text(
                tall,
                format!(
                    "none (the highest fall is {best:.0} m, the limit is {:.0} m)",
                    p.tall_fall_m
                )
            ),
            text(
                wide,
                format!(
                    "none (no fall is {:.0} m high with normal water)",
                    p.wide_min_height_m
                )
            ),
        )
    }

    /// The falls of rivers, for the test code.
    pub fn river_falls(&self) -> impl Iterator<Item = &Waterfall> {
        self.falls
            .iter()
            .filter(|f| self.bodies[f.body as usize].kind == WaterKind::River)
    }
}

#[cfg(test)]
mod tests {
    use crate::highlands::{generate, HighlandsParams};

    #[test]
    fn falls_follow_the_rules() {
        let p = HighlandsParams::default();
        let zone = generate(7, 1024, 2.0, &p);
        let w = &zone.water;
        for f in &w.falls {
            assert!(f.path.len() >= 2);
            assert!(f.height_m > 0.0, "a fall goes down");
            assert!(f.width_m >= 2.0);
            let n = (f.dir[0] * f.dir[0] + f.dir[1] * f.dir[1]).sqrt();
            assert!((n - 1.0).abs() < 1e-3);
            // The path never goes up.
            for pair in f.path.windows(2) {
                assert!(pair[1][1] <= pair[0][1] + 1e-3);
            }
        }
        let again = generate(7, 1024, 2.0, &p);
        assert_eq!(again.water.falls, w.falls);
    }

    #[test]
    fn a_bear_in_a_curtain_is_pushed_down_the_fall() {
        let p = HighlandsParams::default();
        for seed in [1, 7, 42] {
            let zone = generate(seed, 2048, 1.0, &p);
            let w = &zone.water;
            let Some(f) = w.falls.first() else {
                continue;
            };
            let (pos, _) = f.at_distance(f.length_m * 0.5);
            let push = w
                .fall_push_at(pos[0], pos[1], pos[2])
                .expect("in the curtain");
            assert!(push.direction[1] < 0.0);
            assert!(push.strength > 0.9);
            let n = push.direction.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!((n - 1.0).abs() < 1e-3);
            // Far away there is no push.
            assert!(w.fall_push_at(pos[0] + 500.0, pos[1], pos[2]).is_none());
            return;
        }
    }

    /// Prints the falls of the test seeds (run with `--ignored
    /// --nocapture`).
    #[test]
    #[ignore]
    fn print_falls() {
        for seed in [1, 7, 42] {
            let p = HighlandsParams::default();
            let zone = generate(seed, 2048, 1.0, &p);
            let w = &zone.water;
            let mut heights: Vec<f32> = w.falls.iter().map(|f| f.height_m).collect();
            heights.sort_by(|a, b| b.total_cmp(a));
            for f in w.falls.iter().filter(|f| f.height_m > 8.0) {
                println!(
                    "  fall {} h {:.1} len {:.1} cells {} width {:.1} slope {:.2}",
                    f.number,
                    f.height_m,
                    f.length_m,
                    f.path.len(),
                    f.width_m,
                    f.height_m / f.length_m
                );
            }
            println!(
                "seed {seed}: {} falls, tallest {:?}; {}",
                w.falls.len(),
                &heights[..heights.len().min(8)],
                w.falls_report(&p.water)
            );
        }
    }
}
