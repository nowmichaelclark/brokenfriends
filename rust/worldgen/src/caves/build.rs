//! Makes the seeded cave graph of a zone.
//!
//! The steps for each system:
//! 1. Find a hillside mouth on a slope with a hill behind it.
//! 2. Dig the entrance into the hill until the rock above is thick.
//! 3. Put the hall (a chamber of 100 m or more) at the end of a tunnel
//!    that the bear can walk.
//! 4. Put more chambers in a chain down to the bottom of the system
//!    (250 to 400 m below the mouth).
//! 5. Add side tunnels, a loop and sometimes a sinkhole shaft.
//! 6. Bend the tunnels with noise, then move down each part of a
//!    tunnel that has a roof thinner than `roof_min_m`.
//!
//! A step that finds no good place tries again with other random
//! values. A system that fails is not made.

use super::{CaveEdge, CaveNode, CaveParams, CaveSystem, EdgeKind, Mouth, MouthKind, NodeKind};
use crate::detmath::sin_cos;
use crate::heightmap::HeightMap;
use crate::highlands::Zone;
use crate::noise::{gradient_noise, hash01, hash2};

/// The roof gets this much more rock than `roof_min_m`, for the
/// pits of the obstacles and for rounding.
const ROOF_SAFETY_M: f32 = 1.0;
/// The distance between two candidate places for a hillside mouth.
const SITE_STEP_M: f32 = 8.0;
/// The step of the straight-line roof check of a tunnel.
const LINE_STEP_M: f32 = 8.0;

/// Seeded random numbers: a counter that goes through a hash.
struct Rng {
    seed: u32,
    stream: i32,
    n: i32,
}

impl Rng {
    fn new(seed: u32, stream: i32) -> Self {
        Self { seed, stream, n: 0 }
    }

    /// A number from 0 to 1.
    fn f(&mut self) -> f32 {
        self.n += 1;
        hash01(self.n, self.stream, self.seed)
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }

    /// A whole number from 0 to `n - 1`.
    fn below(&mut self, n: u32) -> u32 {
        self.n += 1;
        hash2(self.n, self.stream, self.seed) % n.max(1)
    }
}

/// The ground heights with the helpers that the generator needs.
struct Land<'a> {
    h: &'a HeightMap,
    half: f32,
}

impl Land<'_> {
    fn at(&self, x: f32, z: f32) -> f32 {
        self.h.sample_world(x, z)
    }

    /// The lowest ground in a disc. It reads each sample near the disc,
    /// so the bilinear ground between the samples is never lower.
    fn min_disc(&self, x: f32, z: f32, r: f32) -> f32 {
        let sp = self.h.spacing;
        let reach = r + sp * 1.5;
        let to_i = |w: f32| (w / sp + self.half / sp).floor() as isize;
        let (x0, x1) = (to_i(x - reach), to_i(x + reach) + 1);
        let (z0, z1) = (to_i(z - reach), to_i(z + reach) + 1);
        let mut lo = f32::INFINITY;
        for iz in z0..=z1 {
            let wz = iz as f32 * sp - self.half;
            for ix in x0..=x1 {
                let wx = ix as f32 * sp - self.half;
                let (dx, dz) = (wx - x, wz - z);
                if dx * dx + dz * dz <= reach * reach {
                    lo = lo.min(self.h.get(ix, iz));
                }
            }
        }
        lo.min(self.at(x, z))
    }

    /// The uphill direction and the slope at a point.
    fn uphill(&self, x: f32, z: f32) -> ([f32; 2], f32) {
        let d = 2.0 * self.h.spacing;
        let gx = (self.at(x + d, z) - self.at(x - d, z)) / (2.0 * d);
        let gz = (self.at(x, z + d) - self.at(x, z - d)) / (2.0 * d);
        let s = (gx * gx + gz * gz).sqrt();
        if s > 1e-6 {
            ([gx / s, gz / s], s)
        } else {
            ([1.0, 0.0], 0.0)
        }
    }
}

fn dist2(a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dz) = (a[0] - b[0], a[1] - b[1]);
    (dx * dx + dz * dz).sqrt()
}

fn rotate(d: [f32; 2], angle: f32) -> [f32; 2] {
    let (s, c) = sin_cos(angle);
    [d[0] * c - d[1] * s, d[0] * s + d[1] * c]
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// A chamber that the generator placed, for the space checks.
#[derive(Clone, Copy)]
struct Placed {
    c: [f32; 3],
    rx: f32,
    hy: f32,
}

/// The shared state of the generator for one zone.
struct Ctx<'a> {
    land: Land<'a>,
    p: &'a CaveParams,
    seed: u32,
    safe: f32,
    start: [f32; 2],
    placed: Vec<Placed>,
    mouths: Vec<[f32; 2]>,
}

/// A tunnel before it gets its bent path.
struct Plan {
    a: usize,
    b: usize,
    ra: f32,
    rb: f32,
    kind: EdgeKind,
    /// The mouth tunnel and the shafts keep their path, and they may
    /// have a thin roof.
    fixed: bool,
    path: Vec<[f32; 3]>,
}

/// A system while the generator makes it.
#[derive(Default)]
struct Draft {
    nodes: Vec<CaveNode>,
    plans: Vec<Plan>,
    mouths: Vec<Mouth>,
    chambers: Vec<usize>,
}

impl Draft {
    fn joint(&mut self, pos: [f32; 3], r: f32) -> usize {
        self.nodes.push(CaveNode {
            pos,
            radius_m: r,
            half_height_m: 0.0,
            floor_y: pos[1],
            kind: NodeKind::Joint,
            mouth_path_m: f32::INFINITY,
        });
        self.nodes.len() - 1
    }

    fn chamber(&mut self, c: [f32; 3], rx: f32, hy: f32, floor_y: f32) -> usize {
        self.nodes.push(CaveNode {
            pos: c,
            radius_m: rx,
            half_height_m: hy,
            floor_y,
            kind: NodeKind::Chamber,
            mouth_path_m: f32::INFINITY,
        });
        self.chambers.push(self.nodes.len() - 1);
        self.nodes.len() - 1
    }

    fn link(&mut self, a: usize, b: usize, ra: f32, rb: f32, kind: EdgeKind) {
        let path = vec![self.nodes[a].pos, self.nodes[b].pos];
        self.plans.push(Plan {
            a,
            b,
            ra,
            rb,
            kind,
            fixed: kind != EdgeKind::Tunnel,
            path,
        });
    }
}

impl Ctx<'_> {
    fn noise_of(&self, r: f32) -> f32 {
        (self.p.tunnel_noise_part * r).min(self.p.noise_max_m)
    }

    /// How far above the centre line of a tunnel its air can reach.
    fn tube_top(&self, r: f32) -> f32 {
        r * (1.0 + self.p.radius_wobble) + self.noise_of(r) + self.p.smooth_m
    }

    /// The highest centre height of a tunnel point with radius `r` and a
    /// roof of `roof_min_m` in a disc of radius `around` + the tunnel.
    fn tube_y_max(&self, x: f32, z: f32, r: f32, around: f32) -> f32 {
        let top = self.tube_top(r);
        self.land.min_disc(x, z, around + top) - self.p.roof_min_m - ROOF_SAFETY_M - top
    }

    /// How far a straight tunnel from `a` to `b` must go down at the
    /// worst place to keep its roof. 0 if the roof is thick enough.
    fn line_lowering(&self, a: [f32; 3], b: [f32; 3], r: f32) -> f32 {
        let length = dist2([a[0], a[2]], [b[0], b[2]]);
        let steps = (length / LINE_STEP_M).ceil().max(1.0) as usize;
        let around = LINE_STEP_M + self.p.bend_m;
        let mut worst: f32 = 0.0;
        for i in 1..steps {
            let q = lerp3(a, b, i as f32 / steps as f32);
            let y_max = self.tube_y_max(q[0], q[2], r, around) - self.p.bend_up_m;
            worst = worst.max(q[1] - y_max);
        }
        worst
    }

    /// True if a disc of radius `r` around (x, z) is in the safe part.
    fn safe_at(&self, x: f32, z: f32, r: f32) -> bool {
        (x * x + z * z).sqrt() + r <= self.safe
    }

    /// True if a chamber here keeps a gap of `gap` m to all other chambers.
    fn chamber_free(&self, c: [f32; 3], rx: f32, hy: f32, gap: f32) -> bool {
        self.placed.iter().all(|o| {
            let side = dist2([c[0], c[2]], [o.c[0], o.c[2]]) >= rx + o.rx + gap;
            let above = (c[1] - o.c[1]).abs() >= hy + o.hy + gap;
            side || above
        })
    }

    /// The chamber sizes: (half width, half height).
    fn chamber_size(&self, rng: &mut Rng, min_width: f32, shrink: f32) -> (f32, f32) {
        let p = self.p;
        let u = rng.f();
        let w_max = (p.chamber_width_max_m * shrink).max(min_width);
        let w = min_width + (w_max - min_width) * u * u;
        let h = rng
            .range(p.chamber_height_min_m, p.chamber_height_max_m)
            .clamp(w * p.chamber_flat_min, w * p.chamber_flat_max)
            .clamp(p.chamber_height_min_m, p.chamber_height_max_m);
        (w * 0.5, h * 0.5)
    }

    /// A tunnel radius, most often a small one. It fits the chamber of
    /// half height `hy`.
    fn tunnel_radius(&self, rng: &mut Rng, hy: f32) -> f32 {
        let p = self.p;
        let u = rng.f();
        let w = p.tunnel_width_min_m + (p.tunnel_width_max_m - p.tunnel_width_min_m) * u * u * u;
        (w * 0.5).min(hy * 0.6).max(p.tunnel_width_min_m * 0.5)
    }

    /// The chamber centre height for a wanted floor height. The centre
    /// moves down if the roof is too thin. Gives (centre y, floor y).
    fn chamber_y(&self, x: f32, z: f32, rx: f32, hy: f32, floor_want: f32) -> (f32, f32) {
        let p = self.p;
        let noise = (p.chamber_noise_part * hy).min(p.noise_max_m);
        let top = hy + noise + p.smooth_m;
        let y_max =
            self.land.min_disc(x, z, rx + noise + p.smooth_m) - p.roof_min_m - ROOF_SAFETY_M - top;
        let cy = (floor_want + p.floor_cut * hy).min(y_max);
        (cy, cy - p.floor_cut * hy)
    }
}

/// A place for a hillside mouth.
struct Site {
    x: f32,
    z: f32,
    uphill: [f32; 2],
    key: u32,
}

fn find_sites(ctx: &Ctx) -> Vec<Site> {
    let p = ctx.p;
    let n = (ctx.safe / SITE_STEP_M).floor() as i32;
    let mut sites = Vec::new();
    for iz in -n..=n {
        for ix in -n..=n {
            let (x, z) = (ix as f32 * SITE_STEP_M, iz as f32 * SITE_STEP_M);
            if !ctx.safe_at(x, z, 60.0) {
                continue;
            }
            let (up, slope) = ctx.land.uphill(x, z);
            if slope < p.hill_slope_min || slope > p.hill_slope_max {
                continue;
            }
            // A real hill behind the mouth, not a small bank.
            let h0 = ctx.land.at(x, z);
            let behind = ctx.land.at(x + up[0] * 40.0, z + up[1] * 40.0);
            if behind - h0 < 12.0 {
                continue;
            }
            sites.push(Site {
                x,
                z,
                uphill: up,
                key: hash2(ix, iz, ctx.seed ^ 0x0517_E5ED),
            });
        }
    }
    sites.sort_by_key(|s| s.key);
    sites
}

/// Makes all systems of a zone. Gives the systems and the number of
/// roof faults.
pub(crate) fn build(zone: &Zone, safe: f32, p: &CaveParams) -> (Vec<CaveSystem>, usize) {
    let (sx, sz) = zone.start_point();
    let mut ctx = Ctx {
        land: Land {
            h: &zone.heights,
            half: zone.heights.world_size() * 0.5,
        },
        p,
        seed: zone.seed ^ 0xCA7E_0000,
        safe,
        start: [sx, sz],
        placed: Vec::new(),
        mouths: Vec::new(),
    };
    let sites = find_sites(&ctx);
    let mut used = vec![false; sites.len()];
    let mut rng = Rng::new(ctx.seed, -1);
    let span = p.systems_max.saturating_sub(p.systems_min) + 1;
    let want = (p.systems_min + rng.below(span)) as usize;
    let mut systems = Vec::new();
    let mut faults = 0;
    for attempt in 0..want * 6 {
        if systems.len() >= want {
            break;
        }
        // The first free site in key order. The first system takes a
        // site near the start point if the zone has one.
        let free = |near: bool, ctx: &Ctx| {
            (0..sites.len()).find(|&i| {
                let s = &sites[i];
                let d = dist2([s.x, s.z], ctx.start);
                !used[i]
                    && d >= p.start_clear_m
                    && (!near || d <= p.near_mouth_m)
                    && ctx
                        .mouths
                        .iter()
                        .all(|m| dist2(*m, [s.x, s.z]) >= p.system_spacing_m)
            })
        };
        let found = if systems.is_empty() {
            free(true, &ctx).or_else(|| free(false, &ctx))
        } else {
            free(false, &ctx)
        };
        let Some(i) = found else { break };
        used[i] = true;
        let index = systems.len();
        let mut srng = Rng::new(ctx.seed, 1000 + attempt as i32);
        let saved = ctx.placed.len();
        match build_system(&mut ctx, &sites[i], index, &mut srng) {
            Some((system, f)) => {
                ctx.mouths.push([sites[i].x, sites[i].z]);
                faults += f;
                systems.push(system);
            }
            None => ctx.placed.truncate(saved),
        }
    }
    (systems, faults)
}

/// Makes one system from a hillside site. Gives `None` if it does not fit.
fn build_system(
    ctx: &mut Ctx,
    site: &Site,
    index: usize,
    rng: &mut Rng,
) -> Option<(CaveSystem, usize)> {
    let p = ctx.p;
    let mut d = Draft::default();
    let k = p.smooth_m;

    // 1 and 2: the mouth and the entrance.
    let r0 = rng.range(p.mouth_width_min_m, p.mouth_width_max_m) * 0.5;
    let ground = ctx.land.at(site.x, site.z);
    let up = site.uphill;
    let y_c = ground + 0.35 * r0;
    let out = [site.x - up[0] * r0, y_c, site.z - up[1] * r0];
    let mouth_node = d.joint(out, r0);
    let mut path = vec![out];
    let mut inner = None;
    for j in 1..=24 {
        let s = j as f32 * 8.0;
        let q = [
            site.x + up[0] * s,
            y_c - p.entry_descent * s,
            site.z + up[1] * s,
        ];
        if !ctx.safe_at(q[0], q[2], r0 + 10.0) {
            return None;
        }
        path.push(q);
        let roof = ctx
            .land
            .min_disc(q[0], q[2], 8.0 + r0 + ctx.noise_of(r0) + k)
            - (q[1] + r0 + ctx.noise_of(r0) + k);
        if j >= 2 && roof >= p.roof_min_m + ROOF_SAFETY_M {
            inner = Some(q);
            break;
        }
    }
    let inner = inner?;
    let entry = d.joint(inner, r0);
    d.plans.push(Plan {
        a: mouth_node,
        b: entry,
        ra: r0,
        rb: r0,
        kind: EdgeKind::Tunnel,
        fixed: true,
        path,
    });
    let exempt = dist2([site.x, site.z], [inner[0], inner[2]]) + ctx.tube_top(r0) + 2.0;
    d.mouths.push(Mouth {
        pos: [site.x, ground, site.z],
        kind: MouthKind::Hillside,
        radius_m: r0,
        node: mouth_node,
        facing: up,
        exempt_radius_m: exempt,
    });
    let top_y = ground;
    let depth = rng.range(p.depth_min_m + 5.0, p.depth_max_m - 10.0);
    let bottom = top_y - depth;

    // 3: the hall.
    let mut hall = None;
    let mut best: Option<(f32, [f32; 3], f32, f32, f32, f32)> = None;
    for attempt in 0..24 {
        let shrink = if attempt < 12 { 1.0 } else { 0.6 };
        let (rx, hy) = ctx.chamber_size(rng, p.hall_width_min_m, shrink);
        let rt = ctx.tunnel_radius(rng, hy).max(r0 * 0.8);
        let dir = rotate(up, rng.range(-1.2, 1.2));
        let length = rng.range(60.0, 180.0);
        let door_xz = [inner[0] + dir[0] * length, inner[2] + dir[1] * length];
        let c_xz = [
            door_xz[0] + dir[0] * 0.7 * rx,
            door_xz[1] + dir[1] * 0.7 * rx,
        ];
        let noise = (p.chamber_noise_part * hy).min(p.noise_max_m);
        if !ctx.safe_at(c_xz[0], c_xz[1], rx + noise + k + 2.0) {
            continue;
        }
        let floor_want = inner[1] - r0 - p.walk_slope * 0.8 * length;
        let (cy, floor_y) = ctx.chamber_y(c_xz[0], c_xz[1], rx, hy, floor_want);
        if floor_y < bottom || !ctx.chamber_free([c_xz[0], cy, c_xz[1]], rx, hy, 15.0) {
            continue;
        }
        let door = [door_xz[0], floor_y + rt, door_xz[1]];
        let lower = ctx.line_lowering(inner, door, rt.max(r0));
        let slope = ((inner[1] - r0) - floor_y + lower) / length;
        if slope < 0.0 {
            continue;
        }
        if slope <= p.walk_slope && lower <= 0.0 {
            hall = Some(([c_xz[0], cy, c_xz[1]], rx, hy, floor_y, rt));
            break;
        }
        if slope <= p.steep_slope && best.is_none_or(|b| slope < b.0) {
            best = Some((slope, [c_xz[0], cy, c_xz[1]], rx, hy, floor_y, rt));
        }
    }
    let (hc, hrx, hhy, hfloor, hrt) = hall.or(best.map(|b| (b.1, b.2, b.3, b.4, b.5)))?;
    let hall_node = d.chamber(hc, hrx, hhy, hfloor);
    ctx.placed.push(Placed {
        c: hc,
        rx: hrx,
        hy: hhy,
    });
    let hall_dir = {
        let v = [hc[0] - inner[0], hc[2] - inner[2]];
        let l = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-3);
        [v[0] / l, v[1] / l]
    };
    let hall_door = d.joint(
        [
            hc[0] - hall_dir[0] * 0.7 * hrx,
            hfloor + hrt,
            hc[2] - hall_dir[1] * 0.7 * hrx,
        ],
        hrt,
    );
    d.link(entry, hall_door, r0, hrt, EdgeKind::Tunnel);
    d.link(hall_door, hall_node, hrt, hrt, EdgeKind::Inside);

    // 4: the chain of chambers down to the bottom.
    let drop = hfloor - bottom;
    let links = ((drop / p.link_drop_m).ceil() as usize).clamp(1, 4);
    let mut prev = hall_node;
    for i in 1..=links {
        let floor_want = hfloor + (bottom - hfloor) * (i as f32 / links as f32);
        let pc = d.nodes[prev];
        let mut made = None;
        for attempt in 0..32 {
            let shrink = 1.0 - (attempt / 8) as f32 * 0.25;
            let (rx, hy) = ctx.chamber_size(rng, p.chamber_width_min_m, shrink);
            let rt = ctx.tunnel_radius(rng, hy.min(pc.half_height_m));
            let dir = rotate([1.0, 0.0], rng.range(0.0, std::f32::consts::TAU));
            let fall = pc.floor_y - floor_want;
            let length = (fall / rng.range(0.35, p.steep_slope)).clamp(40.0, 300.0);
            let door_a_xz = [
                pc.pos[0] + dir[0] * 0.7 * pc.radius_m,
                pc.pos[2] + dir[1] * 0.7 * pc.radius_m,
            ];
            let door_b_xz = [
                door_a_xz[0] + dir[0] * length,
                door_a_xz[1] + dir[1] * length,
            ];
            let c_xz = [
                door_b_xz[0] + dir[0] * 0.7 * rx,
                door_b_xz[1] + dir[1] * 0.7 * rx,
            ];
            let noise = (p.chamber_noise_part * hy).min(p.noise_max_m);
            if !ctx.safe_at(c_xz[0], c_xz[1], rx + noise + k + 2.0) {
                continue;
            }
            let (cy, floor_y) = ctx.chamber_y(c_xz[0], c_xz[1], rx, hy, floor_want);
            if floor_y < bottom - 1.0 || floor_want - floor_y > 40.0 {
                continue;
            }
            if !ctx.chamber_free([c_xz[0], cy, c_xz[1]], rx, hy, 15.0) {
                continue;
            }
            let a = [door_a_xz[0], pc.floor_y + rt, door_a_xz[1]];
            let b = [door_b_xz[0], floor_y + rt, door_b_xz[1]];
            let slope = (a[1] - b[1]) / length;
            if slope > p.steep_slope * 1.15 {
                continue;
            }
            let lower = ctx.line_lowering(a, b, rt);
            if lower > 25.0 || b[1] - lower < bottom {
                continue;
            }
            made = Some(([c_xz[0], cy, c_xz[1]], rx, hy, floor_y, rt, a, b));
            break;
        }
        let (c, rx, hy, floor_y, rt, a, b) = made?;
        let node = d.chamber(c, rx, hy, floor_y);
        ctx.placed.push(Placed { c, rx, hy });
        let da = d.joint(a, rt);
        let db = d.joint(b, rt);
        d.link(prev, da, rt, rt, EdgeKind::Inside);
        d.link(da, db, rt, rt, EdgeKind::Tunnel);
        d.link(db, node, rt, rt, EdgeKind::Inside);
        prev = node;
    }

    // 5: side tunnels, a loop and a sinkhole.
    let branches = rng.below(p.branch_max + 1);
    for _ in 0..branches {
        let ci = d.chambers[rng.below(d.chambers.len() as u32) as usize];
        let c = d.nodes[ci];
        let rt = ctx.tunnel_radius(rng, c.half_height_m) * 0.7;
        let rt = rt.max(p.tunnel_width_min_m * 0.5);
        let dir = rotate([1.0, 0.0], rng.range(0.0, std::f32::consts::TAU));
        let length = rng.range(30.0, 90.0);
        let a = [
            c.pos[0] + dir[0] * 0.7 * c.radius_m,
            c.floor_y + rt,
            c.pos[2] + dir[1] * 0.7 * c.radius_m,
        ];
        let mut b = [
            a[0] + dir[0] * length,
            a[1] + rng.range(-10.0, 6.0),
            a[2] + dir[1] * length,
        ];
        if !ctx.safe_at(b[0], b[2], ctx.tube_top(rt) + 2.0) {
            continue;
        }
        b[1] = b[1].min(ctx.tube_y_max(b[0], b[2], rt, p.segment_m));
        if b[1] < bottom || ctx.line_lowering(a, b, rt) > 15.0 {
            continue;
        }
        let ja = d.joint(a, rt);
        let jb = d.joint(b, rt * 0.8);
        d.link(ci, ja, rt, rt, EdgeKind::Inside);
        d.link(ja, jb, rt, rt * 0.8, EdgeKind::Tunnel);
    }
    if d.chambers.len() >= 3 && rng.f() < p.loop_chance {
        let (c0, c2) = (d.nodes[d.chambers[0]], d.nodes[d.chambers[2]]);
        let v = [c2.pos[0] - c0.pos[0], c2.pos[2] - c0.pos[2]];
        let l = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-3);
        let dir = [v[0] / l, v[1] / l];
        let rt = ctx.tunnel_radius(rng, c0.half_height_m.min(c2.half_height_m));
        let a = [
            c0.pos[0] + dir[0] * 0.7 * c0.radius_m,
            c0.floor_y + rt,
            c0.pos[2] + dir[1] * 0.7 * c0.radius_m,
        ];
        let b = [
            c2.pos[0] - dir[0] * 0.7 * c2.radius_m,
            c2.floor_y + rt,
            c2.pos[2] - dir[1] * 0.7 * c2.radius_m,
        ];
        let length = dist2([a[0], a[2]], [b[0], b[2]]);
        let slope = (a[1] - b[1]).abs() / length.max(1.0);
        if length > 20.0
            && length < 350.0
            && slope <= p.steep_slope
            && ctx.line_lowering(a, b, rt) <= 15.0
        {
            let ja = d.joint(a, rt);
            let jb = d.joint(b, rt);
            d.link(d.chambers[0], ja, rt, rt, EdgeKind::Inside);
            d.link(ja, jb, rt, rt, EdgeKind::Tunnel);
            d.link(jb, d.chambers[2], rt, rt, EdgeKind::Inside);
        }
    }
    if rng.f() < p.sinkhole_chance {
        add_sinkhole(ctx, &mut d, rng, bottom);
    }

    // 6: bend the tunnels and keep their roofs.
    let sys_seed = hash2(index as i32, 77, ctx.seed);
    let mut edges = Vec::new();
    let mut faults = 0;
    for (ei, plan) in d.plans.iter().enumerate() {
        let noise_seed = hash2(ei as i32, 5, sys_seed);
        let (path, radii) = if plan.kind == EdgeKind::Inside {
            (plan.path.clone(), vec![plan.ra, plan.rb])
        } else if plan.fixed {
            let n = plan.path.len();
            let radii = (0..n)
                .map(|j| plan.ra + (plan.rb - plan.ra) * j as f32 / (n - 1).max(1) as f32)
                .collect();
            (plan.path.clone(), radii)
        } else {
            let (mut path, radii) = bend(ctx, plan, noise_seed);
            faults += keep_roof(ctx, &mut path, &radii, bottom);
            (path, radii)
        };
        let length_m = path
            .windows(2)
            .map(|w| {
                let v = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
                (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
            })
            .sum();
        edges.push(CaveEdge {
            a: plan.a,
            b: plan.b,
            radius_start_m: plan.ra,
            radius_end_m: plan.rb,
            noise_seed,
            kind: plan.kind,
            path,
            radii,
            length_m,
        });
    }
    // The distance to the nearest mouth for each node (Dijkstra).
    let mut nodes = d.nodes;
    for m in &d.mouths {
        nodes[m.node].mouth_path_m = 0.0;
    }
    let mut done = vec![false; nodes.len()];
    loop {
        let next = (0..nodes.len())
            .filter(|&i| !done[i] && nodes[i].mouth_path_m.is_finite())
            .min_by(|&a, &b| nodes[a].mouth_path_m.total_cmp(&nodes[b].mouth_path_m));
        let Some(i) = next else { break };
        done[i] = true;
        for e in &edges {
            let other = if e.a == i {
                e.b
            } else if e.b == i {
                e.a
            } else {
                continue;
            };
            let w = match e.kind {
                EdgeKind::Tunnel => e.length_m,
                EdgeKind::Shaft => e.length_m * p.shaft_light_scale,
                EdgeKind::Inside => e.length_m * 0.5,
            };
            let via = nodes[i].mouth_path_m + w;
            if via < nodes[other].mouth_path_m {
                nodes[other].mouth_path_m = via;
            }
        }
    }

    let mut lowest = f32::INFINITY;
    let mut highest = f32::NEG_INFINITY;
    for n in &nodes {
        if n.kind == NodeKind::Chamber {
            lowest = lowest.min(n.floor_y);
            highest = highest.max(n.pos[1] + n.half_height_m);
        }
    }
    for e in &edges {
        if e.kind == EdgeKind::Inside {
            continue;
        }
        for (q, r) in e.path.iter().zip(&e.radii) {
            lowest = lowest.min(q[1] - r);
            highest = highest.max(q[1] + r);
        }
    }
    Some((
        CaveSystem {
            index,
            nodes,
            edges,
            mouths: d.mouths,
            lowest_y: lowest,
            highest_y: highest,
        },
        faults,
    ))
}

/// Adds a sinkhole shaft from the ground down into a chamber.
fn add_sinkhole(ctx: &mut Ctx, d: &mut Draft, rng: &mut Rng, bottom: f32) {
    let p = ctx.p;
    // The chamber with the thinnest roof gets the shaft.
    let mut order: Vec<(f32, usize)> = d
        .chambers
        .iter()
        .map(|&ci| {
            let c = d.nodes[ci];
            let roof = ctx.land.min_disc(c.pos[0], c.pos[2], c.radius_m * 0.5)
                - (c.pos[1] + c.half_height_m);
            (roof, ci)
        })
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    for &(_, ci) in order.iter().take(3) {
        let c = d.nodes[ci];
        for _ in 0..8 {
            let rs = rng.range(p.shaft_width_min_m, p.shaft_width_max_m) * 0.5;
            let rs = rs.min(c.radius_m * 0.3);
            let dir = rotate([1.0, 0.0], rng.range(0.0, std::f32::consts::TAU));
            let off = rng.range(0.0, 0.5) * c.radius_m;
            let (x, z) = (c.pos[0] + dir[0] * off, c.pos[2] + dir[1] * off);
            let h = ctx.land.at(x, z);
            let (_, slope) = ctx.land.uphill(x, z);
            let free = dist2([x, z], ctx.start) >= p.start_clear_m
                && ctx.safe_at(x, z, rs + 10.0)
                && slope < 0.6
                && h - bottom <= p.depth_max_m - 5.0;
            if !free {
                continue;
            }
            let t = off / c.radius_m;
            let ceiling = c.pos[1] + c.half_height_m * (1.0 - t * t).max(0.0).sqrt();
            if h - ceiling > p.shaft_length_max_m || h - ceiling < p.roof_min_m {
                continue;
            }
            let bottom_y = ceiling - (c.half_height_m * 0.3).min(5.0);
            let low = d.joint([x, bottom_y, z], rs);
            let high = d.joint([x, h + rs, z], rs);
            d.link(high, low, rs, rs, EdgeKind::Shaft);
            d.link(low, ci, rs, rs, EdgeKind::Inside);
            d.mouths.push(Mouth {
                pos: [x, h, z],
                kind: MouthKind::Sinkhole,
                radius_m: rs,
                node: high,
                facing: [0.0, 0.0],
                exempt_radius_m: ctx.tube_top(rs) + 2.0,
            });
            return;
        }
    }
}

/// Gives the bent path of a tunnel and the radius at each point.
fn bend(ctx: &Ctx, plan: &Plan, seed: u32) -> (Vec<[f32; 3]>, Vec<f32>) {
    let p = ctx.p;
    let (a, b) = (plan.path[0], plan.path[plan.path.len() - 1]);
    let flat = dist2([a[0], a[2]], [b[0], b[2]]);
    let dy = a[1] - b[1];
    let full = (flat * flat + dy * dy).sqrt();
    let n = ((full / p.segment_m).ceil() as usize).max(1);
    let side = if flat > 1e-3 {
        [-(b[2] - a[2]) / flat, (b[0] - a[0]) / flat]
    } else {
        [1.0, 0.0]
    };
    let r_lo = p.tunnel_width_min_m * 0.5;
    let r_hi = p.tunnel_width_max_m * 0.5;
    let mut path = Vec::with_capacity(n + 1);
    let mut radii = Vec::with_capacity(n + 1);
    for j in 0..=n {
        let t = j as f32 / n as f32;
        let window = (4.0 * t * (1.0 - t)).min(1.0);
        let s = t * full;
        let wave =
            |salt: u32, wavelength: f32| gradient_noise(s / wavelength, 0.5, seed ^ salt).value;
        let mut q = lerp3(a, b, t);
        let lateral = p.bend_m * window * wave(0x1, p.bend_wavelength_m);
        q[0] += side[0] * lateral;
        q[2] += side[1] * lateral;
        q[1] += p.bend_up_m * window * wave(0x2, p.bend_wavelength_m * 0.7);
        // Keep the bent tunnel in the safe part of the zone.
        let r_flat = (q[0] * q[0] + q[2] * q[2]).sqrt();
        let limit = ctx.safe - ctx.tube_top(plan.ra.max(plan.rb)) - 1.0;
        if r_flat > limit && r_flat > 0.0 {
            q[0] *= limit / r_flat;
            q[2] *= limit / r_flat;
        }
        let r = plan.ra + (plan.rb - plan.ra) * t;
        let r = r * (1.0 + p.radius_wobble * window * wave(0x3, 30.0));
        path.push(q);
        radii.push(r.clamp(r_lo, r_hi));
    }
    (path, radii)
}

/// Moves the inner points of a tunnel down where its roof is too thin.
/// Gives the number of parts that still have a thin roof.
fn keep_roof(ctx: &Ctx, path: &mut [[f32; 3]], radii: &[f32], bottom: f32) -> usize {
    let p = ctx.p;
    let n = path.len();
    if n < 2 {
        return 0;
    }
    for _ in 0..8 {
        let mut moved = false;
        for j in 0..n - 1 {
            let (a, b) = (path[j], path[j + 1]);
            let mid = [(a[0] + b[0]) * 0.5, (a[2] + b[2]) * 0.5];
            let half = dist2([a[0], a[2]], [b[0], b[2]]) * 0.5;
            let r = radii[j].max(radii[j + 1]);
            let top = ctx.tube_top(r);
            let y_max =
                ctx.land.min_disc(mid[0], mid[1], half + top) - p.roof_min_m - ROOF_SAFETY_M - top;
            let need = a[1].max(b[1]) - y_max;
            if need <= 0.0 {
                continue;
            }
            // Only the inner points move. The ends belong to nodes.
            for i in [j, j + 1] {
                if i > 0 && i < n - 1 && path[i][1] > y_max {
                    path[i][1] = y_max.max(bottom + radii[i]);
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
    let mut faults = 0;
    for j in 0..n - 1 {
        let (a, b) = (path[j], path[j + 1]);
        let mid = [(a[0] + b[0]) * 0.5, (a[2] + b[2]) * 0.5];
        let half = dist2([a[0], a[2]], [b[0], b[2]]) * 0.5;
        let r = radii[j].max(radii[j + 1]);
        let top = ctx.tube_top(r);
        let y_max = ctx.land.min_disc(mid[0], mid[1], half + top) - p.roof_min_m - top;
        if a[1].max(b[1]) > y_max + 0.01 {
            faults += 1;
        }
    }
    faults
}
