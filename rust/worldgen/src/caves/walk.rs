//! The queries that the game needs to walk in the caves (S-164).
//!
//! - [`Caves::wall_blocks`] lists the blocks that can have a cave wall.
//!   The game meshes only these blocks (and the rim blocks at mouths).
//! - [`Caves::floor_under`] gives the floor for a point: the cave floor
//!   in a cave, the ground outside. The bear's put-back and the "through
//!   the ground" test use it.
//! - [`Caves::tour`] gives the fixed camera path of the Cave tour test.

/// The Cave tour goes at most this far along the tunnels to its chamber.
pub const TOUR_REACH_M: f32 = 400.0;

use super::field::{Block, FAR_M};
use super::{Caves, ChamberRef, EdgeKind, MouthKind, NodeKind};
use crate::detmath::sin_cos;
use crate::heightmap::HeightMap;
use std::collections::BTreeSet;

/// A safe upper limit for the slope of the air value (see `field.rs`).
const LIPSCHITZ: f32 = 2.5;

/// The names of the parts of the Cave tour, in order.
pub const TOUR_PARTS: [&str; 4] = ["outside", "tunnel", "chamber", "back"];

/// The fixed camera path of the Cave tour test.
#[derive(Clone, Debug, PartialEq)]
pub struct Tour {
    /// The points of the path, in order.
    pub points: Vec<[f32; 3]>,
    /// The part (an index in [`TOUR_PARTS`]) of the segment that starts
    /// at each point. The last value is the part of the last point.
    pub parts: Vec<usize>,
    /// The distance along the path at each point, in metres.
    pub along: Vec<f32>,
    /// The mouth that the tour goes into (a point on the ground).
    pub mouth: [f32; 3],
    /// The chamber that the tour goes around.
    pub chamber: ChamberRef,
    /// The width of that chamber, in metres.
    pub chamber_width_m: f32,
}

/// One place on the tour path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TourPoint {
    pub pos: [f32; 3],
    /// The direction of the path, length 1.
    pub dir: [f32; 3],
    /// The part (an index in [`TOUR_PARTS`]).
    pub part: usize,
}

impl Tour {
    /// The length of the path in metres.
    pub fn length_m(&self) -> f32 {
        self.along.last().copied().unwrap_or(0.0)
    }

    /// Gives the place at a distance along the path. After the end, the
    /// last point stays.
    pub fn at(&self, dist_m: f32) -> TourPoint {
        let n = self.points.len();
        if n == 0 {
            return TourPoint {
                pos: [0.0; 3],
                dir: [0.0, 0.0, 1.0],
                part: 0,
            };
        }
        if n == 1 {
            return TourPoint {
                pos: self.points[0],
                dir: [0.0, 0.0, 1.0],
                part: self.parts[0],
            };
        }
        let d = dist_m.clamp(0.0, self.length_m());
        // The segment i goes from point i to point i + 1.
        let i = match self.along.binary_search_by(|a| a.total_cmp(&d)) {
            Ok(i) => i.min(n - 2),
            Err(i) => i.saturating_sub(1).min(n - 2),
        };
        let (a, b) = (self.points[i], self.points[i + 1]);
        let seg = (self.along[i + 1] - self.along[i]).max(1e-6);
        let f = ((d - self.along[i]) / seg).clamp(0.0, 1.0);
        let dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
            .sqrt()
            .max(1e-6);
        TourPoint {
            pos: [a[0] + dir[0] * f, a[1] + dir[1] * f, a[2] + dir[2] * f],
            dir: [dir[0] / l, dir[1] / l, dir[2] / l],
            part: self.parts[i],
        }
    }

    fn push(&mut self, p: [f32; 3], part: usize) {
        if let Some(&q) = self.points.last() {
            let d = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
            if d < 0.05 {
                return;
            }
            if let Some(last) = self.parts.last_mut() {
                *last = part;
            }
            let before = self.along.last().copied().unwrap_or(0.0);
            self.along.push(before + d);
        } else {
            self.along.push(0.0);
        }
        self.points.push(p);
        self.parts.push(part);
    }
}

impl Caves {
    /// Tells if a cave can be near the point (within about 6 m). If not,
    /// the heightmap alone is the ground at the point.
    pub fn near_cave(&self, p: [f32; 3]) -> bool {
        self.cave_value(p) < FAR_M
    }

    /// Lists the blocks of side `size` that can have a cave wall (the
    /// bound check gives [`Block::Mixed`]). The block `[i, j, k]` has
    /// its corner at `[i, j, k] * size`. The list is sorted.
    pub fn wall_blocks(&self, size: f32) -> Vec<[i32; 3]> {
        let size = size.max(1.0);
        let mut set = BTreeSet::new();
        for s in &self.shapes {
            let lo = s.lo.map(|v| (v / size).floor() as i32);
            let hi = s.hi.map(|v| (v / size).floor() as i32);
            for k in lo[2]..=hi[2] {
                for j in lo[1]..=hi[1] {
                    for i in lo[0]..=hi[0] {
                        set.insert([i, j, k]);
                    }
                }
            }
        }
        set.into_iter()
            .filter(|b| {
                let min = [b[0] as f32 * size, b[1] as f32 * size, b[2] as f32 * size];
                self.classify_block(min, size) == Block::Mixed
            })
            .collect()
    }

    /// Goes up from a point in rock until it meets air (open air or cave
    /// air). Gives the height (y) of that surface, or `None` if no air is
    /// within `max_up_m`.
    pub fn air_above(&self, ground: &HeightMap, p: [f32; 3], max_up_m: f32) -> Option<f32> {
        let at = |t: f32| [p[0], p[1] + t, p[2]];
        let mut t = 0.0;
        let mut before = 0.0;
        for _ in 0..4096 {
            let v = self.air_value(ground, at(t));
            if v > 0.0 {
                let (mut a, mut b) = (before, t);
                for _ in 0..12 {
                    let m = (a + b) * 0.5;
                    if self.air_value(ground, at(m)) > 0.0 {
                        b = m;
                    } else {
                        a = m;
                    }
                }
                return Some(p[1] + b);
            }
            if t >= max_up_m {
                return None;
            }
            before = t;
            t = (t + (-v / LIPSCHITZ).max(0.05)).min(max_up_m);
        }
        None
    }

    /// Gives the floor for a point: the height that the point stands on
    /// or must stand on.
    /// - No cave near the point: the ground height (the heightmap).
    /// - The point is in air: the first rock straight down (a cave floor
    ///   or the ground), or minus infinity if there is none within
    ///   `max_drop_m`.
    /// - The point is in rock: the first air above it (within 40 m).
    ///   If there is none, the ground height.
    ///
    /// Thus a point is "through the floor" by `floor_under - y` when
    /// that value is positive.
    pub fn floor_under(&self, ground: &HeightMap, p: [f32; 3], max_drop_m: f32) -> f32 {
        let h = ground.sample_world(p[0], p[2]);
        if !self.near_cave(p) {
            return h;
        }
        if self.air_value(ground, p) > 0.0 {
            return self
                .floor_below(ground, p, max_drop_m)
                .unwrap_or(f32::NEG_INFINITY);
        }
        self.air_above(ground, p, 40.0).unwrap_or(h)
    }

    /// Gives the camera path of the Cave tour test: from 50 m outside
    /// the widest hillside mouth to the mouth, down the tunnels to the
    /// widest chamber of that system within `max_path_m` along the
    /// tunnels, once around the chamber, and the same way back. Gives
    /// `None` if the zone has no cave.
    pub fn tour(&self, ground: &HeightMap, max_path_m: f32) -> Option<Tour> {
        // The widest hillside mouth. The first one wins a tie.
        let mut best: Option<(usize, usize)> = None;
        for (si, sys) in self.systems.iter().enumerate() {
            for (mi, m) in sys.mouths.iter().enumerate() {
                if m.kind != MouthKind::Hillside {
                    continue;
                }
                let wider =
                    best.is_none_or(|(bs, bm)| m.radius_m > self.systems[bs].mouths[bm].radius_m);
                if wider {
                    best = Some((si, mi));
                }
            }
        }
        let (si, mi) = best?;
        let sys = &self.systems[si];
        let mouth = &sys.mouths[mi];

        // The shortest way along the tunnels from the mouth to each node
        // (Dijkstra). Shafts are left out: the camera does not fly up
        // and down a shaft.
        let n = sys.nodes.len();
        let mut dist = vec![f32::INFINITY; n];
        let mut from: Vec<Option<usize>> = vec![None; n];
        let mut done = vec![false; n];
        dist[mouth.node] = 0.0;
        loop {
            let next = (0..n)
                .filter(|&i| !done[i] && dist[i].is_finite())
                .min_by(|&a, &b| dist[a].total_cmp(&dist[b]).then(a.cmp(&b)));
            let Some(i) = next else { break };
            done[i] = true;
            for (ei, e) in sys.edges.iter().enumerate() {
                if e.kind == EdgeKind::Shaft || (e.a != i && e.b != i) {
                    continue;
                }
                let other = if e.a == i { e.b } else { e.a };
                let via = dist[i] + e.length_m.max(0.1);
                if via < dist[other] {
                    dist[other] = via;
                    from[other] = Some(ei);
                }
            }
        }
        // The widest chamber that the tunnels reach within
        // `max_path_m`, or else the nearest chamber.
        let widest = sys
            .chambers()
            .filter(|&c| dist[c] <= max_path_m)
            .max_by(|&a, &b| {
                sys.nodes[a]
                    .radius_m
                    .total_cmp(&sys.nodes[b].radius_m)
                    .then(b.cmp(&a))
            });
        let nearest = || {
            sys.chambers()
                .filter(|&c| dist[c].is_finite())
                .min_by(|&a, &b| dist[a].total_cmp(&dist[b]).then(a.cmp(&b)))
        };
        let chamber = widest.or_else(nearest)?;

        // The nodes and edges from the mouth to the chamber.
        let mut chain = Vec::new();
        let mut at = chamber;
        while let Some(ei) = from[at] {
            chain.push(ei);
            let e = &sys.edges[ei];
            at = if e.a == at { e.b } else { e.a };
        }
        chain.reverse();

        let mut tour = Tour {
            points: Vec::new(),
            parts: Vec::new(),
            along: Vec::new(),
            mouth: mouth.pos,
            chamber: ChamberRef {
                system: si,
                node: chamber,
            },
            chamber_width_m: sys.nodes[chamber].radius_m * 2.0,
        };

        // Part 0: from outside to the mouth. The camera is 12 m above
        // the ground at the start and 1.5 m above the mouth at the end.
        let f = mouth.facing;
        let out = [mouth.pos[0] - f[0] * 50.0, mouth.pos[2] - f[1] * 50.0];
        let out_y = ground.sample_world(out[0], out[1]).max(mouth.pos[1]) + 12.0;
        tour.push([out[0], out_y, out[1]], 0);
        tour.push([mouth.pos[0], mouth.pos[1] + 1.5, mouth.pos[2]], 0);

        // Part 1: down the tunnels, on the middle line of each tunnel.
        let mut node = mouth.node;
        let mut way: Vec<[f32; 3]> = Vec::new();
        for &ei in &chain {
            let e = &sys.edges[ei];
            let forward = e.a == node;
            let mut path: Vec<[f32; 3]> = if e.kind == EdgeKind::Inside {
                vec![sys.nodes[e.a].pos, sys.nodes[e.b].pos]
            } else {
                e.path.clone()
            };
            if !forward {
                path.reverse();
            }
            way.extend(path);
            node = if forward { e.b } else { e.a };
        }
        // Points that are in rock (a bend of the noise) are left out.
        let way: Vec<[f32; 3]> = way
            .into_iter()
            .filter(|&p| self.air_value(ground, p) > 0.3)
            .collect();
        for &p in &way {
            tour.push(p, 1);
        }

        // Part 2: once around the chamber, at half of its radius, a
        // little above its floor.
        let c = sys.nodes[chamber];
        debug_assert_eq!(c.kind, NodeKind::Chamber);
        let room = c.pos[1] + c.half_height_m - c.floor_y;
        let y = c.floor_y + (room * 0.3).clamp(2.0, 15.0);
        let start = tour.points.last().copied().unwrap_or(c.pos);
        // The ring starts on the side of the tunnel.
        let (ux, uz) = {
            let (dx, dz) = (start[0] - c.pos[0], start[2] - c.pos[2]);
            let l = (dx * dx + dz * dz).sqrt();
            if l > 1e-3 {
                (dx / l, dz / l)
            } else {
                (1.0, 0.0)
            }
        };
        let steps = 32;
        let mut ring = Vec::with_capacity(steps + 1);
        for k in 0..=steps {
            let (s, co) = sin_cos(std::f32::consts::TAU * k as f32 / steps as f32);
            let (dx, dz) = (co * ux - s * uz, co * uz + s * ux);
            let mut r = c.radius_m * 0.5;
            let mut p = [c.pos[0] + dx * r, y, c.pos[2] + dz * r];
            // Come nearer to the middle if a wall bump is in the way.
            for _ in 0..6 {
                if self.air_value(ground, p) > 1.0 {
                    break;
                }
                r *= 0.75;
                p = [c.pos[0] + dx * r, y, c.pos[2] + dz * r];
            }
            ring.push(p);
        }
        for &p in &ring {
            tour.push(p, 2);
        }

        // Part 3: the same way back to the start.
        let back: Vec<[f32; 3]> = tour.points.iter().rev().copied().collect();
        let first_back = tour.points.len() - ring.len();
        for &p in back.iter().skip(ring.len()).take(first_back) {
            tour.push(p, 3);
        }
        Some(tour)
    }
}
