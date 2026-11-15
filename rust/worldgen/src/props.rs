//! The shapes of the ground props: rocks, pebbles and dead shrubs (S-108).
//!
//! Each function makes one mesh as plain data. The mesh has flat
//! shading: each triangle has its own three vertices and one normal. The
//! origin of each mesh is the middle of its base, and Y is up. The
//! shapes do not depend on a zone seed. They are the same in all zones.
//!
//! Winding: the game engine (Godot) draws the front of a triangle when
//! its vertices run clockwise as seen from the front. The meshes follow
//! that rule.
//!
//! Sizes: a rock or a pebble is 1 m wide. A shrub is 1 m high. The
//! placement (`scatter.rs`) scales them to the real size.

use crate::detmath::sin_cos;
use crate::noise::hash01;

/// The number of rock shapes.
pub const ROCK_VARIANTS: u8 = 2;
/// The number of pebble shapes.
pub const PEBBLE_VARIANTS: u8 = 1;
/// The number of dead shrub shapes.
pub const SHRUB_VARIANTS: u8 = 2;

/// A triangle list with flat shading. Three vertices make one triangle.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshData {
    /// The vertex positions in metres.
    pub positions: Vec<[f32; 3]>,
    /// The normal of each vertex (the same for the 3 vertices of a triangle).
    pub normals: Vec<[f32; 3]>,
    /// The colour of each vertex: red, green, blue, alpha. It makes the
    /// shape darker near the ground.
    pub colours: Vec<[f32; 4]>,
}

impl MeshData {
    /// Gives the number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Gives the smallest and the largest corner of the box that holds
    /// the mesh.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }

    /// Gives the points of the mesh without the copies. The game uses
    /// them as the corners of a convex collision shape.
    pub fn unique_points(&self) -> Vec<[f32; 3]> {
        let mut out: Vec<[f32; 3]> = Vec::new();
        for p in &self.positions {
            if !out.iter().any(|q| q == p) {
                out.push(*p);
            }
        }
        out
    }

    /// Adds a triangle. The corners run counter-clockwise as seen from
    /// outside (the front). The method stores them in the clockwise
    /// order of the engine and gives the triangle one normal.
    fn push_triangle(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], colour: [f32; 4]) {
        let n = normalize(cross(sub(b, a), sub(c, a)));
        for p in [a, c, b] {
            self.positions.push(p);
            self.normals.push(n);
            self.colours.push(colour);
        }
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

fn normalize(a: [f32; 3]) -> [f32; 3] {
    let l = length(a);
    if l < 1.0e-9 {
        [0.0, 1.0, 0.0]
    } else {
        scale(a, 1.0 / l)
    }
}

/// A random number generator that gives the same numbers on all
/// computers. It uses the hash of `noise.rs`.
struct Rng {
    seed: u32,
    counter: i32,
}

impl Rng {
    fn new(seed: u32) -> Self {
        Self { seed, counter: 0 }
    }

    /// Gives a number from 0 to 1.
    fn next(&mut self) -> f32 {
        self.counter += 1;
        hash01(self.counter, 77, self.seed)
    }

    /// Gives a number from -1 to 1.
    fn signed(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}

/// Makes a sphere of radius 1 from an icosahedron (20 triangles). Each
/// step of `subdivide` cuts each triangle into 4. The triangles run
/// counter-clockwise as seen from outside.
fn icosphere(subdivide: u32) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    let t = (1.0 + 5.0f32.sqrt()) * 0.5;
    let mut verts: Vec<[f32; 3]> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .iter()
    .map(|v| normalize(*v))
    .collect();
    let mut faces: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..subdivide {
        let mut cache: Vec<((usize, usize), usize)> = Vec::new();
        let mut middle = |verts: &mut Vec<[f32; 3]>, a: usize, b: usize| -> usize {
            let key = (a.min(b), a.max(b));
            if let Some((_, i)) = cache.iter().find(|(k, _)| *k == key) {
                return *i;
            }
            let m = normalize(scale(add(verts[a], verts[b]), 0.5));
            verts.push(m);
            cache.push((key, verts.len() - 1));
            verts.len() - 1
        };
        let mut next = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let ab = middle(&mut verts, a, b);
            let bc = middle(&mut verts, b, c);
            let ca = middle(&mut verts, c, a);
            next.extend_from_slice(&[[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next;
    }
    (verts, faces)
}

/// Makes a rough stone from an icosphere. `bump` is the size of the
/// random bumps (a part of the radius). `squash` scales the three axes.
/// If `cut_top` is set, the stone has a flat top at that height (as a
/// part of the full height). The stone is 1 m wide, and the lowest
/// point is at Y = 0.
fn stone(subdivide: u32, bump: f32, squash: [f32; 3], cut_top: Option<f32>, seed: u32) -> MeshData {
    let (verts, faces) = icosphere(subdivide);
    let mut points: Vec<[f32; 3]> = verts
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let r = 0.5 * (1.0 + bump * (hash01(i as i32, 5, seed) * 2.0 - 1.0));
            [
                v[0] * r * squash[0],
                v[1] * r * squash[1],
                v[2] * r * squash[2],
            ]
        })
        .collect();
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for p in &points {
        low = low.min(p[1]);
        high = high.max(p[1]);
    }
    if let Some(part) = cut_top {
        let limit = low + (high - low) * part;
        for p in &mut points {
            p[1] = p[1].min(limit);
        }
        high = limit;
    }
    for p in &mut points {
        p[1] -= low;
    }
    let height = (high - low).max(1.0e-4);

    let mut mesh = MeshData::default();
    for (f, [a, b, c]) in faces.iter().enumerate() {
        let (pa, pb, pc) = (points[*a], points[*b], points[*c]);
        // Dark near the ground, a little different for each face.
        let mid = (pa[1] + pb[1] + pc[1]) / 3.0 / height;
        let grain = 0.9 + 0.2 * hash01(f as i32, 9, seed);
        let k = (0.62 + 0.38 * mid.clamp(0.0, 1.0)) * grain;
        mesh.push_triangle(pa, pb, pc, [k, k, k, 1.0]);
    }
    mesh
}

/// Makes a rock. Variant 0 is a round boulder. Variant 1 is a flat
/// slab with a flat top. Both are 1 m wide.
pub fn rock_mesh(variant: u8) -> MeshData {
    match variant % ROCK_VARIANTS {
        0 => stone(1, 0.30, [1.0, 0.85, 1.0], None, 0x0B01_D3A1),
        _ => stone(1, 0.35, [1.0, 0.65, 0.85], Some(0.7), 0x51AB_0002),
    }
}

/// Makes a pebble: a 20-triangle stone, 1 m wide. The game scales it to
/// a few centimetres.
pub fn pebble_mesh(variant: u8) -> MeshData {
    let _ = variant;
    stone(0, 0.30, [1.0, 0.6, 0.8], None, 0x0EB8_1E01)
}

/// A point of the shrub skeleton.
struct Node {
    pos: [f32; 3],
    parent: Option<usize>,
}

/// Makes a dead shrub: a skeleton of thin dry branches, about 1 m high
/// (the low shrub is a little lower).
///
/// The skeleton grows by space colonization (Runions and others, 2007):
/// a cloud of points (the attractors) fills the crown. Each branch tip
/// grows one step toward the attractors that are nearest to it. An
/// attractor that a branch reaches is removed. Each branch is a thin
/// prism with three sides. Variant 0 is tall and narrow. Variant 1 is
/// low and wide.
pub fn shrub_mesh(variant: u8) -> MeshData {
    let wide = variant % SHRUB_VARIANTS == 1;
    let mut rng = Rng::new(0x5111_B000 + u32::from(variant % SHRUB_VARIANTS));
    // The crown: an ellipsoid with its centre `cy` above the ground.
    let (rx, ry, cy) = if wide {
        (0.6, 0.34, 0.5)
    } else {
        (0.34, 0.42, 0.62)
    };
    let step = if wide { 0.13 } else { 0.12 };
    let influence = 0.55;
    let kill = step * 0.8;

    let mut attractors: Vec<[f32; 3]> = Vec::new();
    while attractors.len() < 34 {
        let (u, v, w) = (rng.signed(), rng.signed(), rng.signed());
        if u * u + v * v + w * w <= 1.0 {
            attractors.push([u * rx, cy + v * ry, w * rx]);
        }
    }

    // The trunk is a short stem. Then the branches grow.
    let mut nodes = vec![
        Node {
            pos: [0.0, 0.0, 0.0],
            parent: None,
        },
        Node {
            pos: [0.0, 0.1, 0.0],
            parent: Some(0),
        },
    ];
    const MAX_NODES: usize = 44;
    for _ in 0..40 {
        if attractors.is_empty() || nodes.len() >= MAX_NODES {
            break;
        }
        let mut pull = vec![([0.0f32; 3], 0u32); nodes.len()];
        for a in &attractors {
            let mut best: Option<(usize, f32)> = None;
            for (i, n) in nodes.iter().enumerate() {
                let d = length(sub(*a, n.pos));
                if d < influence && best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((i, d));
                }
            }
            if let Some((i, _)) = best {
                pull[i].0 = add(pull[i].0, normalize(sub(*a, nodes[i].pos)));
                pull[i].1 += 1;
            }
        }
        let mut grew = false;
        for (i, (sum, count)) in pull.iter().enumerate() {
            if *count == 0 || nodes.len() >= MAX_NODES {
                continue;
            }
            // A small random bend makes the branches less straight.
            let bend = [
                rng.signed() * 0.25,
                rng.signed() * 0.15,
                rng.signed() * 0.25,
            ];
            let d = normalize(add(normalize(*sum), bend));
            let pos = add(nodes[i].pos, scale(d, step));
            nodes.push(Node {
                pos,
                parent: Some(i),
            });
            grew = true;
        }
        attractors.retain(|a| nodes.iter().all(|n| length(sub(*a, n.pos)) > kill));
        if !grew {
            break;
        }
    }

    // The radius of each branch: the pipe model. A branch is as thick
    // as the branches above it together (the sum of the squares).
    let leaf = 0.009;
    let mut area = vec![0.0f32; nodes.len()];
    let mut radius = vec![leaf; nodes.len()];
    for i in (0..nodes.len()).rev() {
        radius[i] = area[i].sqrt().max(leaf).min(0.045);
        if let Some(p) = nodes[i].parent {
            area[p] += radius[i] * radius[i];
        }
    }

    // Three corners around the axis: cos and sin of 0, 120 and 240 degrees.
    let ring: [(f32, f32); 3] = {
        let (s1, c1) = sin_cos(std::f32::consts::TAU / 3.0);
        let (s2, c2) = sin_cos(std::f32::consts::TAU * 2.0 / 3.0);
        [(1.0, 0.0), (c1, s1), (c2, s2)]
    };
    let mut mesh = MeshData::default();
    for (i, node) in nodes.iter().enumerate() {
        let Some(p) = node.parent else {
            continue;
        };
        let (from, to) = (nodes[p].pos, node.pos);
        let axis = normalize(sub(to, from));
        let helper = if axis[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let u = normalize(cross(axis, helper));
        let v = cross(axis, u);
        let corner = |centre: [f32; 3], r: f32, k: usize| {
            add(
                centre,
                add(scale(u, ring[k].0 * r), scale(v, ring[k].1 * r)),
            )
        };
        let (r_from, r_to) = (radius[p].max(radius[i]), radius[i]);
        // Dark at the root, a little lighter at the tips.
        let shade = 0.55 + 0.45 * to[1].clamp(0.0, 1.0);
        let colour = [shade, shade, shade, 1.0];
        for k in 0..3 {
            let k1 = (k + 1) % 3;
            let (b0, b1) = (corner(from, r_from, k), corner(from, r_from, k1));
            let (t0, t1) = (corner(to, r_to, k), corner(to, r_to, k1));
            mesh.push_triangle(b0, b1, t1, colour);
            mesh.push_triangle(b0, t1, t0, colour);
        }
    }
    // The lowest point is at Y = 0 already (the root). The branches
    // grow up only.
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_winding(mesh: &MeshData) {
        for t in 0..mesh.triangle_count() {
            let (a, b, c) = (
                mesh.positions[t * 3],
                mesh.positions[t * 3 + 1],
                mesh.positions[t * 3 + 2],
            );
            let n = mesh.normals[t * 3];
            // The engine front is clockwise: the geometric normal of the
            // stored order points away from the stored normal.
            assert!(
                dot(cross(sub(b, a), sub(c, a)), n) < 0.0,
                "triangle {t} has the wrong winding"
            );
            assert!((length(n) - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn stones_face_outward() {
        for v in 0..ROCK_VARIANTS {
            let mesh = rock_mesh(v);
            check_winding(&mesh);
            // The normal of a triangle points away from the middle of
            // the stone.
            let (lo, hi) = mesh.bounds();
            let middle = [0.0, (lo[1] + hi[1]) * 0.5, 0.0];
            for t in 0..mesh.triangle_count() {
                let c = scale(
                    add(
                        add(mesh.positions[t * 3], mesh.positions[t * 3 + 1]),
                        mesh.positions[t * 3 + 2],
                    ),
                    1.0 / 3.0,
                );
                assert!(dot(mesh.normals[t * 3], sub(c, middle)) > 0.0, "{v} {t}");
            }
        }
        check_winding(&pebble_mesh(0));
    }

    #[test]
    fn stones_have_the_right_size() {
        for v in 0..ROCK_VARIANTS {
            let (lo, hi) = rock_mesh(v).bounds();
            assert!(lo[1].abs() < 1e-5, "the base is at Y = 0");
            let width = (hi[0] - lo[0]).max(hi[2] - lo[2]);
            assert!((0.7..=1.4).contains(&width), "variant {v}: {width}");
            assert!(hi[1] > 0.25 && hi[1] < 1.2, "variant {v}: {}", hi[1]);
        }
        let (lo, hi) = pebble_mesh(0).bounds();
        assert!(lo[1].abs() < 1e-5);
        assert!(hi[0] - lo[0] > 0.6 && hi[0] - lo[0] < 1.4);
    }

    #[test]
    fn triangle_counts_are_small() {
        assert_eq!(rock_mesh(0).triangle_count(), 80);
        assert_eq!(rock_mesh(1).triangle_count(), 80);
        assert_eq!(pebble_mesh(0).triangle_count(), 20);
        for v in 0..SHRUB_VARIANTS {
            let n = shrub_mesh(v).triangle_count();
            assert!((60..=300).contains(&n), "shrub {v} has {n} triangles");
        }
    }

    #[test]
    fn shrubs_are_one_metre_high_and_face_outward() {
        for v in 0..SHRUB_VARIANTS {
            let mesh = shrub_mesh(v);
            check_winding(&mesh);
            let (lo, hi) = mesh.bounds();
            assert!(lo[1] > -0.06, "variant {v}: low {}", lo[1]);
            assert!((0.55..=1.2).contains(&hi[1]), "variant {v}: high {}", hi[1]);
            assert!(hi[0] - lo[0] < 1.6 && hi[2] - lo[2] < 1.6);
        }
    }

    #[test]
    fn the_shapes_are_the_same_each_time() {
        assert_eq!(rock_mesh(0), rock_mesh(0));
        assert_eq!(shrub_mesh(1), shrub_mesh(1));
        assert_ne!(shrub_mesh(0), shrub_mesh(1));
        assert_ne!(rock_mesh(0), rock_mesh(1));
    }

    #[test]
    fn unique_points_drop_the_copies() {
        // An icosphere with one subdivision has 42 corners.
        assert_eq!(rock_mesh(0).unique_points().len(), 42);
    }
}
