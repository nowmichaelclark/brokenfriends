//! The SDF of the caves, the spatial grid, the bound check and the
//! queries.
//!
//! Each shape has a "base" distance without the wall bumps. The wall
//! bumps add one shared noise value `n` (-1 to 1) times the bump height
//! of the shape. A shape counts at a point only if its base distance is
//! less than its reach. This rule depends only on the point. Thus a
//! point query, a chunk fill and a brute-force loop over all shapes give
//! the same bits, and two chunks agree at their shared samples.

use super::noise3::fbm3;
use super::{CaveParams, CaveSystem, Caves, ChamberRef, EdgeKind, NodeKind};
use crate::detmath::exp;
use crate::heightmap::HeightMap;

/// The cave value is never larger than this, in metres. Values far
/// inside the rock do not matter for meshes or queries.
pub(crate) const FAR_M: f32 = 6.0;
/// A safe upper limit for the slope of the air value along a ray. The
/// wall bumps make the field change faster than 1 m for each metre.
const RAY_LIPSCHITZ: f32 = 2.5;
/// The bumps of a chamber floor, as a part of the bumps of its walls.
const FLOOR_BUMPS: f32 = 0.3;
/// The depth below the ground where the hole test looks for cave air.
const HOLE_PROBE_M: f32 = 0.25;

#[derive(Clone, Debug)]
pub(crate) enum Form {
    /// A flattened ball (an ellipsoid with axes `rx`, `hy`, `rx`) with a
    /// flat floor at `floor_y`. `ratio` is `rx / hy` (1 or more).
    Chamber {
        c: [f32; 3],
        rx: f32,
        hy: f32,
        ratio: f32,
        floor_y: f32,
    },
    /// A capsule from `a` to `b` with radius `ra` at `a` and `rb` at `b`.
    Tube {
        a: [f32; 3],
        ab: [f32; 3],
        inv_len2: f32,
        ra: f32,
        rb: f32,
    },
}

/// The part of the graph that a shape comes from.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Owner {
    Node(u32),
    Edge,
}

#[derive(Clone, Debug)]
pub(crate) struct Shape {
    pub form: Form,
    /// The height of the wall bumps.
    pub noise_m: f32,
    /// The shape counts at a point only if its base distance is less.
    pub reach_m: f32,
    /// A tube: the Lipschitz constant of its base distance.
    pub lip: f32,
    /// Outside this box the base distance is surely `reach_m` or more.
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    pub system: u16,
    pub owner: Owner,
    /// The distance to the nearest mouth at both ends (sky visibility).
    pub path_a: f32,
    pub path_b: f32,
}

#[inline]
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn len(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

/// Polynomial smooth minimum. It is never more than `k` below the
/// smaller value, also after many joins in a row.
#[inline]
pub(crate) fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

impl Shape {
    /// The two easy bounds of a chamber: `s` (Lipschitz 1, never larger
    /// than the true distance outside) and `u` (`s` times `ratio`).
    #[inline]
    fn chamber_su(q: [f32; 3], rx: f32, ratio: f32) -> (f32, f32) {
        let u = len([q[0], q[1] * ratio, q[2]]) - rx;
        (u / ratio, u)
    }

    /// The distance without the wall bumps. Negative inside the shape.
    #[inline]
    pub fn base(&self, p: [f32; 3]) -> f32 {
        let (wall, floor) = self.parts(p);
        wall.max(floor)
    }

    /// The distance with the wall bumps of the noise value `n`. A
    /// chamber floor gets smaller bumps than its walls, so that the bear
    /// can walk on it. The result is never more than `noise_m` from
    /// the base distance.
    #[inline]
    pub fn value(&self, p: [f32; 3], n: f32) -> f32 {
        let (wall, floor) = self.parts(p);
        let a = self.noise_m * n;
        (wall - a).max(floor - a * FLOOR_BUMPS)
    }

    /// The distance to the wall (or the tube) and to the floor plane.
    /// A tube has no floor plane.
    #[inline]
    fn parts(&self, p: [f32; 3]) -> (f32, f32) {
        match self.form {
            Form::Chamber {
                c,
                rx,
                hy,
                ratio,
                floor_y,
            } => {
                let q = sub(p, c);
                let (s, u) = Self::chamber_su(q, rx, ratio);
                // Inigo Quilez's ellipsoid estimate. It is near the true
                // distance; the clamp keeps it between the two bounds.
                let k0 = len([q[0] / rx, q[1] / hy, q[2] / rx]);
                let k1 = len([q[0] / (rx * rx), q[1] / (hy * hy), q[2] / (rx * rx)]);
                let est = if k1 > 1e-12 {
                    k0 * (k0 - 1.0) / k1
                } else {
                    -hy
                };
                let (lo, hi) = if s < u { (s, u) } else { (u, s) };
                (est.clamp(lo, hi), floor_y - p[1])
            }
            Form::Tube {
                a,
                ab,
                inv_len2,
                ra,
                rb,
            } => {
                let ap = sub(p, a);
                let t = (dot(ap, ab) * inv_len2).clamp(0.0, 1.0);
                let d = [ap[0] - ab[0] * t, ap[1] - ab[1] * t, ap[2] - ab[2] * t];
                (len(d) - (ra + (rb - ra) * t), f32::NEG_INFINITY)
            }
        }
    }

    /// The lowest and the highest base distance in a ball with centre
    /// `c` and radius `r`.
    pub fn base_bounds(&self, c: [f32; 3], r: f32) -> (f32, f32) {
        match self.form {
            Form::Chamber {
                c: centre,
                rx,
                ratio,
                floor_y,
                ..
            } => {
                let (s, u) = Self::chamber_su(sub(c, centre), rx, ratio);
                let lo = (s - r).min(u - ratio * r);
                let hi = (s + r).max(u + ratio * r);
                let plane = floor_y - c[1];
                (lo.max(plane - r), hi.max(plane + r))
            }
            Form::Tube { .. } => {
                let b = self.base(c);
                (b - self.lip * r, b + self.lip * r)
            }
        }
    }

    #[inline]
    fn outside_box(&self, p: [f32; 3]) -> bool {
        p[0] < self.lo[0]
            || p[0] > self.hi[0]
            || p[1] < self.lo[1]
            || p[1] > self.hi[1]
            || p[2] < self.lo[2]
            || p[2] > self.hi[2]
    }

    /// The distance to the nearest mouth at the point of the shape that
    /// is nearest to `p`.
    fn path_at(&self, p: [f32; 3]) -> f32 {
        match self.form {
            Form::Chamber { .. } => self.path_a,
            Form::Tube {
                a, ab, inv_len2, ..
            } => {
                let t = (dot(sub(p, a), ab) * inv_len2).clamp(0.0, 1.0);
                self.path_a + (self.path_b - self.path_a) * t
            }
        }
    }
}

/// Makes the SDF shapes from the cave graph.
pub(crate) fn make_shapes(systems: &[CaveSystem], params: &CaveParams) -> Vec<Shape> {
    let k = params.smooth_m;
    let mut shapes = Vec::new();
    for sys in systems {
        for (ni, n) in sys.nodes.iter().enumerate() {
            if n.kind != NodeKind::Chamber {
                continue;
            }
            let (rx, hy) = (n.radius_m, n.half_height_m);
            let ratio = rx / hy;
            let noise = (params.chamber_noise_part * hy).min(params.noise_max_m);
            let reach = FAR_M + 2.0 * k + noise;
            let c = n.pos;
            let side = rx + reach * ratio;
            shapes.push(Shape {
                form: Form::Chamber {
                    c,
                    rx,
                    hy,
                    ratio,
                    floor_y: n.floor_y,
                },
                noise_m: noise,
                reach_m: reach,
                lip: 1.0,
                lo: [c[0] - side, n.floor_y.max(c[1] - hy) - reach, c[2] - side],
                hi: [c[0] + side, c[1] + hy + reach, c[2] + side],
                system: sys.index as u16,
                owner: Owner::Node(ni as u32),
                path_a: n.mouth_path_m,
                path_b: n.mouth_path_m,
            });
        }
        for e in &sys.edges {
            if e.kind == EdgeKind::Inside {
                continue;
            }
            let scale = if e.kind == EdgeKind::Shaft {
                params.shaft_light_scale
            } else {
                1.0
            };
            let (da, db) = (sys.nodes[e.a].mouth_path_m, sys.nodes[e.b].mouth_path_m);
            let mut along = 0.0;
            for j in 0..e.path.len().saturating_sub(1) {
                let (a, b) = (e.path[j], e.path[j + 1]);
                let (ra, rb) = (e.radii[j], e.radii[j + 1]);
                let ab = sub(b, a);
                let l2 = dot(ab, ab).max(1e-6);
                let seg = l2.sqrt();
                let noise = (params.tunnel_noise_part * ra.max(rb)).min(params.noise_max_m);
                let reach = FAR_M + 2.0 * k + noise;
                let grow = ra.max(rb) + reach;
                let path = |s: f32| (da + s * scale).min(db + (e.length_m - s) * scale);
                shapes.push(Shape {
                    form: Form::Tube {
                        a,
                        ab,
                        inv_len2: 1.0 / l2,
                        ra,
                        rb,
                    },
                    noise_m: noise,
                    reach_m: reach,
                    lip: 1.0 + (rb - ra).abs() / seg,
                    lo: [
                        a[0].min(b[0]) - grow,
                        a[1].min(b[1]) - grow,
                        a[2].min(b[2]) - grow,
                    ],
                    hi: [
                        a[0].max(b[0]) + grow,
                        a[1].max(b[1]) + grow,
                        a[2].max(b[2]) + grow,
                    ],
                    system: sys.index as u16,
                    owner: Owner::Edge,
                    path_a: path(along),
                    path_b: path(along + seg),
                });
                along += seg;
            }
        }
    }
    shapes
}

/// A grid of square columns over the zone. Each column lists the shapes
/// whose box touches it, in the order of the shape list.
#[derive(Clone, Debug)]
pub(crate) struct Grid {
    half: f32,
    cell: f32,
    n: usize,
    starts: Vec<u32>,
    items: Vec<u32>,
}

impl Grid {
    pub fn new(shapes: &[Shape], half: f32, cell: f32) -> Self {
        let cell = cell.max(1.0);
        let n = ((2.0 * half / cell).ceil() as usize).max(1);
        let mut lists: Vec<Vec<u32>> = vec![Vec::new(); n * n];
        let mut grid = Self {
            half,
            cell,
            n,
            starts: Vec::new(),
            items: Vec::new(),
        };
        for (i, s) in shapes.iter().enumerate() {
            let (x0, z0) = grid.column(s.lo[0], s.lo[2]);
            let (x1, z1) = grid.column(s.hi[0], s.hi[2]);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    lists[z * n + x].push(i as u32);
                }
            }
        }
        grid.starts.reserve(n * n + 1);
        for list in &lists {
            grid.starts.push(grid.items.len() as u32);
            grid.items.extend_from_slice(list);
        }
        grid.starts.push(grid.items.len() as u32);
        grid
    }

    /// The column of a point. Points outside the zone use the nearest
    /// column at the border.
    #[inline]
    fn column(&self, x: f32, z: f32) -> (usize, usize) {
        let max = self.n as isize - 1;
        let cx = (((x + self.half) / self.cell).floor() as isize).clamp(0, max) as usize;
        let cz = (((z + self.half) / self.cell).floor() as isize).clamp(0, max) as usize;
        (cx, cz)
    }

    #[inline]
    fn at(&self, x: f32, z: f32) -> &[u32] {
        let (cx, cz) = self.column(x, z);
        let i = cz * self.n + cx;
        &self.items[self.starts[i] as usize..self.starts[i + 1] as usize]
    }

    /// All shapes in the columns that touch a rectangle, sorted.
    fn in_rect(&self, x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<u32> {
        let (cx0, cz0) = self.column(x0, z0);
        let (cx1, cz1) = self.column(x1, z1);
        let mut ids = Vec::new();
        for z in cz0..=cz1 {
            for x in cx0..=cx1 {
                let i = z * self.n + x;
                ids.extend_from_slice(
                    &self.items[self.starts[i] as usize..self.starts[i + 1] as usize],
                );
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

/// How a block of the zone meets the caves (the bound check).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    /// No cave air in the block. A cave chunk here needs no mesh.
    NoCave,
    /// Only cave air in the block. A cave chunk here needs no mesh.
    CaveAir,
    /// The block can have a cave wall. Sample it and mesh it.
    Mixed,
}

impl Caves {
    #[inline]
    fn wall_noise(&self, p: [f32; 3]) -> f32 {
        let f = 1.0 / self.params.wall_noise_wavelength_m;
        fbm3(p[0] * f, p[1] * f, p[2] * f, self.noise_seed, 4)
    }

    /// The cave value from a sorted list of shapes that holds all shapes
    /// that can count at `p`.
    fn cave_from(&self, p: [f32; 3], ids: &[u32]) -> f32 {
        let k = self.params.smooth_m;
        let mut acc: Option<f32> = None;
        let mut noise: Option<f32> = None;
        for &i in ids {
            let s = &self.shapes[i as usize];
            if s.outside_box(p) {
                continue;
            }
            let b = s.base(p);
            if b >= s.reach_m {
                continue;
            }
            let n = *noise.get_or_insert_with(|| self.wall_noise(p));
            let d = s.value(p, n);
            acc = Some(match acc {
                None => d,
                Some(a) => smin(a, d, k),
            });
        }
        acc.map_or(FAR_M, |a| a.min(FAR_M))
    }

    /// Gives the cave value at a point: negative in cave air, positive in
    /// rock. The value is about the distance to the nearest cave wall in
    /// metres, and never more than 6 m. It does not know the ground.
    pub fn cave_value(&self, p: [f32; 3]) -> f32 {
        self.cave_from(p, self.grid.at(p[0], p[2]))
    }

    /// The same as [`Caves::cave_value`], with a loop over all shapes.
    /// It is slow. The tests use it to check the grid.
    pub fn cave_value_slow(&self, p: [f32; 3]) -> f32 {
        let all: Vec<u32> = (0..self.shapes.len() as u32).collect();
        self.cave_from(p, &all)
    }

    /// Gives the air value at a point: positive in air (open air or cave
    /// air), negative in rock. This is the field that a cave chunk meshes.
    pub fn air_value(&self, ground: &HeightMap, p: [f32; 3]) -> f32 {
        let above = p[1] - ground.sample_world(p[0], p[2]);
        above.max(-self.cave_value(p))
    }

    /// Gives true if the point is in cave air under the ground. A point
    /// in the open air above a mouth is not in a cave.
    pub fn in_cave(&self, ground: &HeightMap, p: [f32; 3]) -> bool {
        p[1] < ground.sample_world(p[0], p[2]) && self.cave_value(p) < 0.0
    }

    /// Fills `out` with the cave value on a grid of points. The point
    /// (`ix`, `iy`, `iz`) is `origin + i * step`, and its index is
    /// `ix + dims[0] * (iy + dims[1] * iz)` (x changes fastest). The
    /// values are the same bits as [`Caves::cave_value`].
    pub fn fill_cave(&self, origin: [f32; 3], step: f32, dims: [usize; 3], out: &mut Vec<f32>) {
        let far = [
            origin[0] + (dims[0].max(1) - 1) as f32 * step,
            origin[2] + (dims[2].max(1) - 1) as f32 * step,
        ];
        let ids = self.grid.in_rect(origin[0], origin[2], far[0], far[1]);
        out.clear();
        out.reserve(dims[0] * dims[1] * dims[2]);
        for iz in 0..dims[2] {
            let z = origin[2] + iz as f32 * step;
            for iy in 0..dims[1] {
                let y = origin[1] + iy as f32 * step;
                for ix in 0..dims[0] {
                    let x = origin[0] + ix as f32 * step;
                    out.push(self.cave_from([x, y, z], &ids));
                }
            }
        }
    }

    /// Fills `out` with the air value on a grid of points, in the order
    /// of [`Caves::fill_cave`]. Give the result to Surface Nets.
    pub fn fill_air(
        &self,
        ground: &HeightMap,
        origin: [f32; 3],
        step: f32,
        dims: [usize; 3],
        out: &mut Vec<f32>,
    ) {
        self.fill_cave(origin, step, dims, out);
        let mut heights = Vec::with_capacity(dims[0] * dims[2]);
        for iz in 0..dims[2] {
            for ix in 0..dims[0] {
                heights.push(
                    ground.sample_world(origin[0] + ix as f32 * step, origin[2] + iz as f32 * step),
                );
            }
        }
        for iz in 0..dims[2] {
            for iy in 0..dims[1] {
                let y = origin[1] + iy as f32 * step;
                for ix in 0..dims[0] {
                    let i = ix + dims[0] * (iy + dims[1] * iz);
                    out[i] = (y - heights[iz * dims[0] + ix]).max(-out[i]);
                }
            }
        }
    }

    /// The bound check: tells how the cube with corner `min` and side
    /// `size` meets the caves, without sampling it.
    pub fn classify_block(&self, min: [f32; 3], size: f32) -> Block {
        let h = size * 0.5;
        let c = [min[0] + h, min[1] + h, min[2] + h];
        let r = h * 3f32.sqrt();
        let ids = self
            .grid
            .in_rect(min[0], min[2], min[0] + size, min[2] + size);
        let mut lower = FAR_M;
        for i in ids {
            let s = &self.shapes[i as usize];
            let apart = (0..3).any(|a| min[a] > s.hi[a] || min[a] + size < s.lo[a]);
            if apart {
                continue;
            }
            let (lo, hi) = s.base_bounds(c, r);
            if lo >= s.reach_m {
                continue;
            }
            if hi + s.noise_m < 0.0 {
                return Block::CaveAir;
            }
            lower = lower.min(lo - s.noise_m);
        }
        if lower - self.params.smooth_m > 0.0 {
            Block::NoCave
        } else {
            Block::Mixed
        }
    }

    /// Moves along a ray until it meets rock. `dir` must have length 1.
    /// Gives the distance, or `None` if the ray meets no rock within
    /// `max_m`. A start point in rock gives `Some(0.0)`.
    pub fn ray_hit(
        &self,
        ground: &HeightMap,
        from: [f32; 3],
        dir: [f32; 3],
        max_m: f32,
    ) -> Option<f32> {
        let at = |t: f32| {
            [
                from[0] + dir[0] * t,
                from[1] + dir[1] * t,
                from[2] + dir[2] * t,
            ]
        };
        let mut t = 0.0;
        let mut before = 0.0;
        for _ in 0..4096 {
            let v = self.air_value(ground, at(t));
            if v <= 0.0 {
                if t == 0.0 {
                    return Some(0.0);
                }
                // The wall is between `before` and `t`.
                let (mut a, mut b) = (before, t);
                for _ in 0..12 {
                    let m = (a + b) * 0.5;
                    if self.air_value(ground, at(m)) <= 0.0 {
                        b = m;
                    } else {
                        a = m;
                    }
                }
                return Some(b);
            }
            if t >= max_m {
                return None;
            }
            before = t;
            t = (t + (v / RAY_LIPSCHITZ).max(0.05)).min(max_m);
        }
        None
    }

    /// Gives the height (y) of the floor below a point: the first rock
    /// straight down, in a cave or on the ground. Gives `None` if the
    /// point is in rock or if no floor is within `max_drop_m`.
    pub fn floor_below(&self, ground: &HeightMap, p: [f32; 3], max_drop_m: f32) -> Option<f32> {
        if self.air_value(ground, p) <= 0.0 {
            return None;
        }
        self.ray_hit(ground, p, [0.0, -1.0, 0.0], max_drop_m)
            .map(|t| p[1] - t)
    }

    /// Gives the chamber that holds a point in its air, or `None`.
    pub fn chamber_at(&self, p: [f32; 3]) -> Option<ChamberRef> {
        let mut best: Option<(f32, ChamberRef)> = None;
        let mut noise: Option<f32> = None;
        for &i in self.grid.at(p[0], p[2]) {
            let s = &self.shapes[i as usize];
            let Owner::Node(node) = s.owner else { continue };
            if s.outside_box(p) {
                continue;
            }
            let b = s.base(p);
            if b >= s.noise_m {
                continue;
            }
            let n = *noise.get_or_insert_with(|| self.wall_noise(p));
            let d = s.value(p, n);
            if d < 0.0 && best.is_none_or(|(bd, _)| d < bd) {
                let r = ChamberRef {
                    system: s.system as usize,
                    node: node as usize,
                };
                best = Some((d, r));
            }
        }
        best.map(|(_, r)| r)
    }

    /// Gives how much sky a point sees, from 0 (none) to 1 (open sky).
    /// The value falls with the distance along the tunnels to the
    /// nearest mouth (`sky_falloff_m`). Rock gives 0. Open air above the
    /// ground gives 1.
    pub fn sky_visibility(&self, ground: &HeightMap, p: [f32; 3]) -> f32 {
        if p[1] >= ground.sample_world(p[0], p[2]) {
            return 1.0;
        }
        let ids = self.grid.at(p[0], p[2]);
        if self.cave_from(p, ids) >= 0.0 {
            return 0.0;
        }
        let n = self.wall_noise(p);
        let mut path = f32::INFINITY;
        for &i in ids {
            let s = &self.shapes[i as usize];
            if s.outside_box(p) {
                continue;
            }
            let b = s.base(p);
            if b >= s.reach_m {
                continue;
            }
            let d = s.value(p, n);
            path = path.min(s.path_at(p) + d.max(0.0));
        }
        if path.is_finite() {
            exp(-path / self.params.sky_falloff_m.max(0.1)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Gives the hole mask for the Terrain3D control map, one byte for
    /// each heightmap sample: 2 where cave air comes up to the ground (a
    /// mouth), 1 in a band of `band_cells` samples around it, 0 at all
    /// other samples. Give the hole bit to the samples with 1 and 2.
    pub fn hole_mask(&self, ground: &HeightMap, band_cells: usize) -> Vec<u8> {
        let size = ground.size;
        let half = size as f32 * 0.5;
        let spacing = ground.spacing;
        let mut mask = vec![0u8; size * size];
        let index = |w: f32| ((w / spacing + half).round() as isize).clamp(0, size as isize - 1);
        for m in self.mouths() {
            let reach = m.exempt_radius_m + band_cells as f32 * spacing;
            let (x0, x1) = (index(m.pos[0] - reach), index(m.pos[0] + reach));
            let (z0, z1) = (index(m.pos[2] - reach), index(m.pos[2] + reach));
            for z in z0..=z1 {
                for x in x0..=x1 {
                    let wx = (x as f32 - half) * spacing;
                    let wz = (z as f32 - half) * spacing;
                    let h = ground.get(x, z);
                    if self.cave_value([wx, h - HOLE_PROBE_M, wz]) < 0.0 {
                        mask[z as usize * size + x as usize] = 2;
                    }
                }
            }
        }
        if band_cells > 0 {
            let holes = mask.clone();
            let b = band_cells as isize;
            for z in 0..size as isize {
                for x in 0..size as isize {
                    if holes[z as usize * size + x as usize] != 2 {
                        continue;
                    }
                    for dz in -b..=b {
                        for dx in -b..=b {
                            let (nx, nz) = (x + dx, z + dz);
                            if nx < 0 || nz < 0 || nx >= size as isize || nz >= size as isize {
                                continue;
                            }
                            let i = nz as usize * size + nx as usize;
                            mask[i] = mask[i].max(1);
                        }
                    }
                }
            }
        }
        mask
    }
}
