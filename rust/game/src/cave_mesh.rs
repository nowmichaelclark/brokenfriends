//! The meshes of the cave chunks (S-164). This module has no Godot code,
//! so it has unit tests. `crate::caves::BfCaves` gives the meshes to
//! Godot.
//!
//! A chunk is a cube of 32 m with 1 m voxels. Its field is the air value
//! of `bf_worldgen::caves` (positive in air, negative in rock) on 34³
//! samples: the chunk and one sample of padding on each side. Surface
//! Nets (`fast-surface-nets`) makes the mesh. The chunks fit together
//! with no gaps and no double faces (a test checks this).
//!
//! The air value also holds the ground surface. Terrain3D draws the
//! ground, so a chunk keeps the ground part of its mesh only at the cave
//! mouths: in the hole cells, in the band around them and in a ring of
//! `ring` cells more. The ring goes under the terrain edge, so no crack
//! shows. The vertices in the ring sink a few centimetres, so the
//! terrain hides them.

use bf_worldgen::caves::Caves;
use bf_worldgen::HeightMap;
use fast_surface_nets::ndshape::{RuntimeShape, Shape as _};
use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};

/// The side of one chunk in metres.
pub const CHUNK_M: f32 = 32.0;
/// The side of one voxel in metres.
pub const VOXEL_M: f32 = 1.0;
/// The samples on each side of a chunk: 32 voxels and the padding.
pub const SAMPLES: u32 = 34;

/// The place of a chunk: its corner is `key * CHUNK_M`.
pub type ChunkKey = [i32; 3];

/// Gives the chunk that holds a point.
pub fn chunk_of(p: [f32; 3]) -> ChunkKey {
    p.map(|v| (v / CHUNK_M).floor() as i32)
}

/// Gives the distance from a point to the box of a chunk, in metres
/// (0 inside the box).
pub fn chunk_distance(key: ChunkKey, p: [f32; 3]) -> f32 {
    let mut d2 = 0.0;
    for a in 0..3 {
        let lo = key[a] as f32 * CHUNK_M;
        let hi = lo + CHUNK_M;
        let d = (lo - p[a]).max(p[a] - hi).max(0.0);
        d2 += d * d;
    }
    d2.sqrt()
}

/// The settings of the cave meshes.
#[derive(Clone, Debug, PartialEq)]
pub struct CaveMeshParams {
    /// A vertex that is less than this below the ground, with a normal
    /// that points up, is a part of the ground surface, in metres.
    pub ground_eps_m: f32,
    /// The vertices of the ring under the terrain edge go down this far,
    /// in metres.
    pub rim_sink_m: f32,
    /// The colour of the cave rock (linear light).
    pub rock_colour: [f32; 3],
    /// The colour changes from the ground colour to the rock colour in
    /// this depth below the ground, in metres.
    pub colour_depth_m: f32,
}

impl Default for CaveMeshParams {
    fn default() -> Self {
        Self {
            ground_eps_m: 0.6,
            rim_sink_m: 0.08,
            rock_colour: [0.16, 0.085, 0.05],
            colour_depth_m: 4.0,
        }
    }
}

/// The distance of each heightmap sample from the holes of the cave
/// mouths, in samples: 0 in a hole or its band, 1 to `ring` in the ring
/// around them, 255 farther away.
#[derive(Clone, Debug, PartialEq)]
pub struct RimMap {
    size: usize,
    spacing: f32,
    ring: u8,
    dist: Vec<u8>,
    /// The samples with a distance of `ring` or less (x, z).
    cells: Vec<(u32, u32)>,
}

/// "Far from a mouth" in a [`RimMap`].
pub const FAR: u8 = 255;

impl RimMap {
    /// Makes the map from the hole mask (`Caves::hole_mask`: 1 or 2 in
    /// the band and the holes) of a heightmap with `size` samples on each
    /// side.
    pub fn new(mask: &[u8], size: usize, spacing: f32, ring: u8) -> Self {
        let mut dist = vec![FAR; size * size];
        let mut cells = Vec::new();
        let r = ring as isize;
        for (i, &m) in mask.iter().enumerate() {
            if m == 0 {
                continue;
            }
            let (x, z) = ((i % size) as isize, (i / size) as isize);
            for dz in -r..=r {
                for dx in -r..=r {
                    let (nx, nz) = (x + dx, z + dz);
                    if nx < 0 || nz < 0 || nx >= size as isize || nz >= size as isize {
                        continue;
                    }
                    let d = dx.unsigned_abs().max(dz.unsigned_abs()) as u8;
                    let j = nz as usize * size + nx as usize;
                    if dist[j] == FAR {
                        cells.push((nx as u32, nz as u32));
                    }
                    dist[j] = dist[j].min(d);
                }
            }
        }
        cells.sort_unstable_by_key(|&(x, z)| (z, x));
        Self {
            size,
            spacing,
            ring,
            dist,
            cells,
        }
    }

    /// Gives the distance at the sample nearest to a world position.
    pub fn at(&self, x: f32, z: f32) -> u8 {
        if self.size == 0 {
            return FAR;
        }
        let half = self.size as f32 * 0.5;
        let ix = (x / self.spacing + half).round();
        let iz = (z / self.spacing + half).round();
        if ix < 0.0 || iz < 0.0 || ix >= self.size as f32 || iz >= self.size as f32 {
            return FAR;
        }
        self.dist[iz as usize * self.size + ix as usize]
    }

    /// The width of the ring under the terrain edge, in samples.
    pub fn ring(&self) -> u8 {
        self.ring
    }

    /// Gives the world position (x, z) of each sample in a hole, its
    /// band or the ring.
    pub fn cells(&self) -> impl Iterator<Item = (f32, f32)> + '_ {
        let half = self.size as f32 * 0.5;
        self.cells.iter().map(move |&(x, z)| {
            (
                (x as f32 - half) * self.spacing,
                (z as f32 - half) * self.spacing,
            )
        })
    }
}

/// The geometry of one chunk, ready for Godot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChunkMesh {
    pub key: ChunkKey,
    /// The world positions.
    pub positions: Vec<[f32; 3]>,
    /// The normals, length 1, pointing into the air.
    pub normals: Vec<[f32; 3]>,
    /// The vertex colours (linear light, alpha 1).
    pub colours: Vec<[f32; 4]>,
    /// Three indices for each triangle, clockwise seen from the air
    /// (the front face in Godot).
    pub indices: Vec<u32>,
    /// The ground part of the mesh is at a mouth (the chunk is a rim
    /// chunk).
    pub has_rim: bool,
}

impl ChunkMesh {
    /// The number of triangles.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// Gives the triangles as a flat list of corners, for a collision
    /// shape (`ConcavePolygonShape3D::set_faces`).
    pub fn faces(&self) -> Vec<[f32; 3]> {
        self.indices
            .iter()
            .map(|&i| self.positions[i as usize])
            .collect()
    }
}

/// A raw Surface Nets mesh in world space, before the ground rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RawMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Clockwise seen from the air.
    pub indices: Vec<u32>,
}

/// Runs Surface Nets on the field of one chunk (34³ samples, x changes
/// fastest). `origin` is the world position of the first sample. Rock
/// (negative) is inside, so the normals point into the air.
pub fn surface(field: &[f32], origin: [f32; 3]) -> RawMesh {
    let mut out = RawMesh::default();
    let (lo, hi) = field
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    // No sign change: no surface (most chunks under a big chamber).
    if !(lo < 0.0 && hi >= 0.0) {
        return out;
    }
    let shape = RuntimeShape::<u32, 3>::new([SAMPLES; 3]);
    debug_assert_eq!(shape.usize(), field.len());
    let mut buffer = SurfaceNetsBuffer::default();
    surface_nets(field, &shape, [0; 3], [SAMPLES - 1; 3], &mut buffer);
    out.positions = buffer
        .positions
        .iter()
        .map(|p| {
            [
                origin[0] + p[0] * VOXEL_M,
                origin[1] + p[1] * VOXEL_M,
                origin[2] + p[2] * VOXEL_M,
            ]
        })
        .collect();
    out.normals = buffer
        .normals
        .iter()
        .map(|n| {
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if l > 1e-9 {
                [n[0] / l, n[1] / l, n[2] / l]
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect();
    // Surface Nets gives counter-clockwise triangles. Godot wants
    // clockwise front faces, so swap two corners of each triangle.
    out.indices.reserve(buffer.indices.len());
    for tri in buffer.indices.chunks_exact(3) {
        out.indices.extend([tri[0], tri[2], tri[1]]);
    }
    out
}

/// Gives the world position of the first sample of a chunk.
pub fn chunk_origin(key: ChunkKey) -> [f32; 3] {
    key.map(|k| k as f32 * CHUNK_M - VOXEL_M)
}

/// Changes an sRGB colour part (0 to 1) to linear light.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The data that a chunk build reads.
pub struct ChunkInput<'a> {
    pub caves: &'a Caves,
    pub ground: &'a HeightMap,
    /// The colour map of the ground (sRGB), one colour for each sample.
    pub colours: &'a [[u8; 3]],
    pub rim: &'a RimMap,
    pub params: &'a CaveMeshParams,
}

/// Makes the mesh of one chunk. It runs on a worker thread.
pub fn build_chunk(input: &ChunkInput, key: ChunkKey) -> ChunkMesh {
    let origin = chunk_origin(key);
    let mut field = Vec::new();
    let n = SAMPLES as usize;
    input
        .caves
        .fill_air(input.ground, origin, VOXEL_M, [n, n, n], &mut field);
    let raw = surface(&field, origin);
    apply_ground_rules(input, key, &raw)
}

/// Keeps the ground part of a raw mesh only near the mouths, sinks the
/// ring and gives the colours.
pub fn apply_ground_rules(input: &ChunkInput, key: ChunkKey, raw: &RawMesh) -> ChunkMesh {
    let p = input.params;
    let g = input.ground;
    let size = g.size;
    let half = size as f32 * 0.5;
    let colour_at = |x: f32, z: f32| -> [f32; 3] {
        if input.colours.len() != size * size {
            return p.rock_colour;
        }
        let ix = ((x / g.spacing + half).round() as isize).clamp(0, size as isize - 1) as usize;
        let iz = ((z / g.spacing + half).round() as isize).clamp(0, size as isize - 1) as usize;
        let c = input.colours[iz * size + ix];
        [
            linear(c[0] as f32 / 255.0),
            linear(c[1] as f32 / 255.0),
            linear(c[2] as f32 / 255.0),
        ]
    };
    let ring = input.rim.ring();
    let mut out = ChunkMesh {
        key,
        ..ChunkMesh::default()
    };
    // The new index of each raw vertex, or `u32::MAX` if it is left out.
    let mut remap = vec![u32::MAX; raw.positions.len()];
    for (i, (&pos, &nrm)) in raw.positions.iter().zip(&raw.normals).enumerate() {
        let h = g.sample_world(pos[0], pos[2]);
        let on_ground = pos[1] > h - p.ground_eps_m && nrm[1] > 0.2;
        let d = input.rim.at(pos[0], pos[2]);
        if on_ground && d > ring {
            continue;
        }
        let mut pos = pos;
        if on_ground {
            out.has_rim = true;
            // The terrain edge is at the first ring: a small sink there
            // keeps the step small; the full sink is under the terrain.
            if d == 1 {
                pos[1] -= p.rim_sink_m * 0.25;
            } else if d >= 2 {
                pos[1] -= p.rim_sink_m;
            }
        }
        let t = ((h - pos[1]) / p.colour_depth_m.max(0.01)).clamp(0.0, 1.0);
        let top = colour_at(pos[0], pos[2]);
        let c = [0, 1, 2].map(|k| top[k] + (p.rock_colour[k] - top[k]) * t);
        remap[i] = out.positions.len() as u32;
        out.positions.push(pos);
        out.normals.push(nrm);
        out.colours.push([c[0], c[1], c[2], 1.0]);
    }
    out.indices.reserve(raw.indices.len());
    for tri in raw.indices.chunks_exact(3) {
        let t = [
            remap[tri[0] as usize],
            remap[tri[1] as usize],
            remap[tri[2] as usize],
        ];
        if t.iter().all(|&i| i != u32::MAX) {
            out.indices.extend(t);
        }
    }
    // Vertices of dropped triangles stay; they cost little and keep the
    // code simple.
    out
}

/// All cave data of one zone that the game needs. The terrain makes it
/// on its worker thread, after the obstacles.
pub struct CaveWorld {
    pub caves: Caves,
    /// A copy of the final ground heights, for the worker threads.
    pub ground: HeightMap,
    /// The colour map of the ground (sRGB).
    pub colours: Vec<[u8; 3]>,
    /// The hole mask (`Caves::hole_mask`): 2 in a hole, 1 in its band.
    pub mask: Vec<u8>,
    pub rim: RimMap,
    /// The chunks that can have a cave wall, sorted.
    pub wall_chunks: Vec<ChunkKey>,
    /// The chunks that hold the ground at a mouth, sorted. The game
    /// always meshes these, so a hole never shows the sky.
    pub rim_chunks: Vec<ChunkKey>,
}

impl CaveWorld {
    /// Makes the data. `ring` is the width of the ring under the terrain
    /// edge, in samples.
    pub fn new(
        caves: Caves,
        ground: HeightMap,
        colours: Vec<[u8; 3]>,
        band_cells: usize,
        ring: u8,
    ) -> Self {
        let mask = caves.hole_mask(&ground, band_cells);
        let rim = RimMap::new(&mask, ground.size, ground.spacing, ring);
        let wall_chunks = caves.wall_blocks(CHUNK_M);
        let mut rim_chunks: Vec<ChunkKey> = Vec::new();
        for (x, z) in rim.cells() {
            let h = ground.sample_world(x, z);
            for dy in [-1.0, 0.0, 1.0] {
                rim_chunks.push(chunk_of([x, h + dy, z]));
            }
        }
        rim_chunks.sort_unstable();
        rim_chunks.dedup();
        Self {
            caves,
            ground,
            colours,
            mask,
            rim,
            wall_chunks,
            rim_chunks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The field of a ball of rock (negative inside) at `c` with radius `r`.
    fn ball_field(origin: [f32; 3], c: [f32; 3], r: f32) -> Vec<f32> {
        let n = SAMPLES as usize;
        let mut f = Vec::with_capacity(n * n * n);
        for iz in 0..n {
            for iy in 0..n {
                for ix in 0..n {
                    let p = [
                        origin[0] + ix as f32 * VOXEL_M,
                        origin[1] + iy as f32 * VOXEL_M,
                        origin[2] + iz as f32 * VOXEL_M,
                    ];
                    let d = ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2))
                        .sqrt();
                    f.push(d - r);
                }
            }
        }
        f
    }

    #[test]
    fn chunks_fit_together_with_no_gaps() {
        // A ball across the corner of 8 chunks: the welded mesh of the 8
        // chunks is closed. Each edge has exactly two triangles.
        let c = [CHUNK_M, CHUNK_M, CHUNK_M];
        let mut weld: HashMap<[i64; 3], usize> = HashMap::new();
        let mut tris: Vec<[usize; 3]> = Vec::new();
        for k in 0..8 {
            let key = [k & 1, (k >> 1) & 1, (k >> 2) & 1];
            let origin = chunk_origin(key);
            let raw = surface(&ball_field(origin, c, 11.3), origin);
            assert!(!raw.indices.is_empty());
            let ids: Vec<usize> = raw
                .positions
                .iter()
                .map(|p| {
                    let q = p.map(|v| (v * 1000.0).round() as i64);
                    let next = weld.len();
                    *weld.entry(q).or_insert(next)
                })
                .collect();
            for t in raw.indices.chunks_exact(3) {
                tris.push([ids[t[0] as usize], ids[t[1] as usize], ids[t[2] as usize]]);
            }
        }
        let mut edges: HashMap<(usize, usize), i32> = HashMap::new();
        for t in &tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                // A directed edge: a closed mesh with one winding has
                // each edge once in each direction.
                *edges.entry((a, b)).or_default() += 1;
            }
        }
        for (&(a, b), &n) in &edges {
            assert_eq!(n, 1, "the edge {a}-{b} is in {n} triangles");
            assert_eq!(edges.get(&(b, a)), Some(&1), "the edge {a}-{b} has no twin");
        }
    }

    #[test]
    fn triangles_face_the_air() {
        // Clockwise seen from outside of the ball of rock: the face normal
        // from the winding (b - a) x (c - a) points into the rock in a
        // right-handed system, the opposite of the vertex normals.
        let origin = chunk_origin([0, 0, 0]);
        let c = [16.0, 16.0, 16.0];
        let raw = surface(&ball_field(origin, c, 9.0), origin);
        let mut agree = 0;
        for t in raw.indices.chunks_exact(3) {
            let [a, b, d] = [t[0], t[1], t[2]].map(|i| raw.positions[i as usize]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let out = [a[0] - c[0], a[1] - c[1], a[2] - c[2]];
            if n[0] * out[0] + n[1] * out[1] + n[2] * out[2] < 0.0 {
                agree += 1;
            }
            // The vertex normal points out of the rock.
            let vn = raw.normals[t[0] as usize];
            assert!(vn[0] * out[0] + vn[1] * out[1] + vn[2] * out[2] > 0.0);
        }
        assert_eq!(agree, raw.indices.len() / 3);
    }

    #[test]
    fn empty_field_gives_no_mesh() {
        let n = (SAMPLES * SAMPLES * SAMPLES) as usize;
        assert!(surface(&vec![1.0; n], [0.0; 3]).indices.is_empty());
        assert!(surface(&vec![-1.0; n], [0.0; 3]).indices.is_empty());
    }

    #[test]
    fn rim_map_rings() {
        let size = 16;
        let mut mask = vec![0u8; size * size];
        mask[8 * size + 8] = 2;
        let rim = RimMap::new(&mask, size, 1.0, 2);
        // Sample (8, 8) is at world (0, 0).
        assert_eq!(rim.at(0.0, 0.0), 0);
        assert_eq!(rim.at(1.0, 0.0), 1);
        assert_eq!(rim.at(-2.0, 2.0), 2);
        assert_eq!(rim.at(3.0, 0.0), FAR);
        assert_eq!(rim.cells().count(), 25);
    }

    #[test]
    fn chunk_keys_and_distances() {
        assert_eq!(chunk_of([0.5, -0.5, 33.0]), [0, -1, 1]);
        assert_eq!(chunk_distance([0, 0, 0], [16.0, 16.0, 16.0]), 0.0);
        assert!((chunk_distance([0, 0, 0], [35.0, 16.0, 16.0]) - 3.0).abs() < 1e-5);
        assert_eq!(chunk_origin([1, 0, -1]), [31.0, -1.0, -33.0]);
    }
}

#[cfg(test)]
mod zone_tests {
    use super::*;
    use bf_worldgen::caves::{CaveParams, Caves, MouthKind};
    use bf_worldgen::highlands::{generate, HighlandsParams};
    use bf_worldgen::smoke::SmokeParams;

    #[test]
    fn mouth_chunks_fill_the_hole_and_drop_the_far_ground() {
        let zone = generate(7, 512, 4.0, &HighlandsParams::default());
        let caves = Caves::generate(&zone, &SmokeParams::default(), &CaveParams::default());
        let g = &zone.heights;
        let mask = caves.hole_mask(g, 2);
        let rim = RimMap::new(&mask, g.size, g.spacing, 2);
        let colours = zone.ground_colours();
        let params = CaveMeshParams::default();
        let input = ChunkInput {
            caves: &caves,
            ground: g,
            colours: &colours,
            rim: &rim,
            params: &params,
        };
        let mouth = caves
            .mouths()
            .find(|m| m.kind == MouthKind::Hillside)
            .unwrap();
        // The chunks around the mouth.
        let c = chunk_of(mouth.pos);
        let mut meshes = Vec::new();
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    meshes.push(build_chunk(&input, [c[0] + dx, c[1] + dy, c[2] + dz]));
                }
            }
        }
        assert!(meshes.iter().any(|m| m.has_rim), "no rim at the mouth");
        let tris: usize = meshes.iter().map(ChunkMesh::triangles).sum();
        assert!(tris > 100, "only {tris} triangles at the mouth");
        // Each kept vertex on the ground is near a mouth.
        for m in &meshes {
            for (p, n) in m.positions.iter().zip(&m.normals) {
                let h = g.sample_world(p[0], p[2]);
                if p[1] > h - params.ground_eps_m + params.rim_sink_m && n[1] > 0.2 {
                    assert!(rim.at(p[0], p[2]) <= rim.ring(), "ground at {p:?}");
                }
            }
        }
        // Each hole sample near the mouth has a mesh vertex near it.
        let half = g.size as f32 * 0.5;
        let mut holes = 0;
        for (i, &v) in mask.iter().enumerate() {
            if v != 2 {
                continue;
            }
            let x = ((i % g.size) as f32 - half) * g.spacing;
            let z = ((i / g.size) as f32 - half) * g.spacing;
            if (x - mouth.pos[0]).abs() > 30.0 || (z - mouth.pos[2]).abs() > 30.0 {
                continue;
            }
            holes += 1;
            let near = meshes
                .iter()
                .flat_map(|m| m.positions.iter())
                .any(|p| (p[0] - x).abs() <= g.spacing && (p[2] - z).abs() <= g.spacing);
            assert!(near, "no mesh at the hole sample ({x}, {z})");
        }
        assert!(holes > 0);
    }
}
