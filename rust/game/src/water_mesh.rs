//! The meshes of the water (S-160, rivers S-162). This module has no
//! Godot code, so it has unit tests. `crate::water::BfWater` gives the
//! meshes to Godot.
//!
//! A river is a grid of 1 m quads over its wet samples. Each vertex has
//! the level, the flow vector and the foam of the river cell at that
//! place, so the surface slopes down the river.
//!
//! Each body is a flat grid of quads at its level. A quad is in the mesh
//! if one of its samples is wet. The quads at the shore reach a little
//! over the dry ground; the terrain hides that part, and the shader
//! fades the water where the depth goes to zero. The depth (level minus
//! ground) and the look of the body are in the vertex data, so the
//! shader needs no depth texture.
//!
//! The quads go into square tiles of the zone. Each tile is one mesh
//! and one draw call, so the water never needs more draw calls than
//! there are tiles (4 for a zone of 2 km with tiles of 1024 m; each tile has a second draw call for its opaque quads).

use bf_worldgen::water::{Water, WaterBody, WaterKind};
use bf_worldgen::HeightMap;
use std::collections::HashMap;

/// The settings of the water meshes.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshParams {
    /// The size of one quad of a small body, in metres.
    pub cell_m: f32,
    /// The size of one quad of a large body, in metres.
    pub large_cell_m: f32,
    /// A body with more area than this uses `large_cell_m`, in square
    /// metres.
    pub large_area_m2: f32,
    /// A large quad near the shore is cut into four quads: when a corner
    /// is dry or less deep than this, in metres.
    pub shore_detail_m: f32,
    /// The size of one tile (one mesh, one draw call), in metres.
    pub tile_m: f32,
    /// The size of one quad of a river, in metres. Rivers are shallow, so
    /// their quads follow the 1 m detail of the ground.
    pub river_cell_m: f32,
    /// A river quad whose corners differ by more than this in height is
    /// left out, in metres: it is a waterfall (S-165 draws it). 1 m on a
    /// quad of 1 m is 45 degrees, the slope of a fall in `WaterParams`.
    pub max_quad_drop_m: f32,
    /// A river vertex averages the wet samples up to this many samples
    /// away (2: a square of 5 x 5 samples).
    pub river_smooth_samples: usize,
    /// The clarity in the vertex alpha is clarity / this value.
    pub clarity_scale: f32,
    /// A quad is opaque (no blending, one cut of the GPU budget) when
    /// all its corners are deeper than this many clarities. At 2, the
    /// water hides 86% of the ground. 0 turns the opaque quads off.
    pub opaque_clarities: f32,
}

impl Default for MeshParams {
    fn default() -> Self {
        Self {
            cell_m: 2.0,
            large_cell_m: 4.0,
            large_area_m2: 20_000.0,
            shore_detail_m: 1.5,
            tile_m: 1024.0,
            river_cell_m: 1.0,
            max_quad_drop_m: 1.0,
            river_smooth_samples: 2,
            clarity_scale: 8.0,
            opaque_clarities: 2.0,
        }
    }
}

/// The vertex data of one tile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileMesh {
    /// The tile (x, z), from the corner of the zone.
    pub tile: (i32, i32),
    pub positions: Vec<[f32; 3]>,
    /// The linear deep colour and the clarity / `clarity_scale`.
    pub colours: Vec<[f32; 4]>,
    /// The depth at the vertex (level minus ground) and the foam.
    pub uvs: Vec<[f32; 2]>,
    /// The flow vector in m/s (x, z). Still water has none.
    pub flows: Vec<[f32; 2]>,
    pub indices: Vec<i32>,
    /// The triangles of the opaque deep quads. They use the same
    /// vertices as `indices` and draw without blending.
    pub deep_indices: Vec<i32>,
    /// The box around the vertices: the lowest and the highest corner.
    pub aabb: ([f32; 3], [f32; 3]),
}

impl TileMesh {
    /// The number of triangles.
    pub fn triangles(&self) -> usize {
        (self.indices.len() + self.deep_indices.len()) / 3
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

/// Makes the meshes of all bodies, one for each tile that has water.
pub fn build_meshes(water: &Water, heights: &HeightMap, p: &MeshParams) -> Vec<TileMesh> {
    let mut tiles: HashMap<(i32, i32), TileMesh> = HashMap::new();
    for (index, body) in water.bodies.iter().enumerate() {
        if body.kind == WaterKind::River {
            add_river(&mut tiles, water, heights, p, index);
        } else {
            add_body(&mut tiles, water, heights, p, index, body);
        }
    }
    let mut out: Vec<TileMesh> = tiles.into_values().collect();
    out.sort_by_key(|t| t.tile);
    for t in &mut out {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in &t.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k]);
                hi[k] = hi[k].max(v[k]);
            }
        }
        t.aabb = (lo, hi);
    }
    out
}

fn add_body(
    tiles: &mut HashMap<(i32, i32), TileMesh>,
    water: &Water,
    heights: &HeightMap,
    p: &MeshParams,
    index: usize,
    body: &WaterBody,
) {
    let size = water.size;
    let spacing = water.spacing;
    let cell = if body.area_m2 >= p.large_area_m2 {
        p.large_cell_m
    } else {
        p.cell_m
    };
    let k = ((cell / spacing).round() as usize).max(1);
    let id = index as u16 + 1;
    let wet = |x: usize, z: usize| x < size && z < size && water.owner[z * size + x] == id;

    // The lattice of the quads, aligned to multiples of `k` samples.
    let (x0, z0) = (
        (body.min.0 as usize).saturating_sub(1) / k * k,
        (body.min.1 as usize).saturating_sub(1) / k * k,
    );
    let (x1, z1) = (
        (body.max.0 as usize + 1).min(size - 1),
        (body.max.1 as usize + 1).min(size - 1),
    );
    let colour = [
        linear(body.deep_colour[0]),
        linear(body.deep_colour[1]),
        linear(body.deep_colour[2]),
        (body.clarity_m / p.clarity_scale.max(0.01)).clamp(0.0, 1.0),
    ];
    let half = size as f32 * 0.5;
    let world = |s: usize| (s as f32 - half) * spacing;
    // One vertex for each lattice corner and tile.
    let mut vertex_of: HashMap<((i32, i32), usize, usize), i32> = HashMap::new();

    // Adds one quad if it has a wet sample. Gives true if it did.
    let mut quad = |x: usize, z: usize, step: usize| {
        let (xe, ze) = ((x + step).min(size - 1), (z + step).min(size - 1));
        if xe == x || ze == z || !(z..=ze).any(|zz| (x..=xe).any(|xx| wet(xx, zz))) {
            return;
        }
        let tile = tile_of(water, p, world((x + xe) / 2), world((z + ze) / 2));
        let mesh = tiles.entry(tile).or_insert_with(|| TileMesh {
            tile,
            ..TileMesh::default()
        });
        let mut corner = |cx: usize, cz: usize| -> i32 {
            *vertex_of.entry((tile, cx, cz)).or_insert_with(|| {
                let ground = heights.get(cx as isize, cz as isize);
                mesh.positions.push([world(cx), body.level, world(cz)]);
                mesh.colours.push(colour);
                mesh.uvs.push([body.level - ground, 0.0]);
                mesh.flows.push([0.0, 0.0]);
                mesh.positions.len() as i32 - 1
            })
        };
        let a = corner(x, z);
        let b = corner(xe, z);
        let c = corner(x, ze);
        let d = corner(xe, ze);
        // Clockwise when seen from above: Godot draws clockwise
        // triangles as front faces.
        let deep_from = body.clarity_m * p.opaque_clarities;
        let deep = p.opaque_clarities > 0.0
            && [a, b, c, d]
                .iter()
                .all(|&i| mesh.uvs[i as usize][0] > deep_from);
        let list = if deep {
            &mut mesh.deep_indices
        } else {
            &mut mesh.indices
        };
        list.extend([a, b, c, b, d, c]);
    };
    let depth = |x: usize, z: usize| body.level - heights.get(x as isize, z as isize);

    let mut z = z0;
    while z <= z1 {
        let mut x = x0;
        while x <= x1 {
            let (xe, ze) = ((x + k).min(size - 1), (z + k).min(size - 1));
            // A large quad at the shore becomes four smaller quads, so
            // the fade at the shore follows the ground better. The small
            // corners are on the edges of the large quads: the surface
            // is flat, so this leaves no gaps.
            let shore = k >= 2
                && [(x, z), (xe, z), (x, ze), (xe, ze)]
                    .iter()
                    .any(|&(cx, cz)| depth(cx, cz) < p.shore_detail_m);
            if shore {
                let h = k / 2;
                for (dx, dz) in [(0, 0), (h, 0), (0, h), (h, h)] {
                    quad(x + dx, z + dz, h);
                }
            } else {
                quad(x, z, k);
            }
            x += k;
        }
        z += k;
    }
}

/// Gives the tile (x, z) of a world position.
fn tile_of(water: &Water, p: &MeshParams, wx: f32, wz: f32) -> (i32, i32) {
    let corner = water.size as f32 * 0.5 * water.spacing;
    (
        ((wx + corner) / p.tile_m).floor() as i32,
        ((wz + corner) / p.tile_m).floor() as i32,
    )
}

/// The values of a river vertex: the level, the flow and the foam.
#[derive(Clone, Copy)]
struct RiverVertex {
    level: f32,
    flow: [f32; 2],
    foam: f32,
}

/// Adds the quads of one river. A quad is in the mesh if one of its
/// samples is wet. Each vertex averages the level, the flow and the foam
/// of the wet samples near it (`river_smooth_samples`), so the surface
/// has no steps from one river cell to the next. A quad that drops more
/// than `max_quad_drop_m` is a waterfall and is left out.
fn add_river(
    tiles: &mut HashMap<(i32, i32), TileMesh>,
    water: &Water,
    heights: &HeightMap,
    p: &MeshParams,
    index: usize,
) {
    let size = water.size;
    let spacing = water.spacing;
    let body = &water.bodies[index];
    let k = ((p.river_cell_m / spacing).round() as usize).max(1);
    let r = p.river_smooth_samples;
    let colour = [
        linear(body.deep_colour[0]),
        linear(body.deep_colour[1]),
        linear(body.deep_colour[2]),
        (body.clarity_m / p.clarity_scale.max(0.01)).clamp(0.0, 1.0),
    ];
    let half = size as f32 * 0.5;
    let world = |s: usize| (s as f32 - half) * spacing;

    // A dense grid of the river cells in the box of the river, with a
    // margin: fast look-ups for the averages.
    let margin = k + r + 1;
    let gx0 = (body.min.0 as usize).saturating_sub(margin);
    let gz0 = (body.min.1 as usize).saturating_sub(margin);
    let gx1 = (body.max.0 as usize + margin).min(size - 1);
    let gz1 = (body.max.1 as usize + margin).min(size - 1);
    let gw = gx1 - gx0 + 1;
    let mut grid = vec![u32::MAX; gw * (gz1 - gz0 + 1)];
    let mut quads: Vec<(usize, usize)> = Vec::new();
    for (&s, &c) in water.river_samples.iter().zip(&water.river_sample_cell) {
        if water.river_cells[c as usize].body as usize != index {
            continue;
        }
        let (x, z) = (s as usize % size, s as usize / size);
        grid[(z - gz0) * gw + (x - gx0)] = c;
        // The quads that have this sample as a corner.
        let (qx, qz) = (x / k * k, z / k * k);
        for (ax, az) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (ox, oz) = (qx as isize - ax * k as isize, qz as isize - az * k as isize);
            if (ax == 1 && x % k != 0) || (az == 1 && z % k != 0) || ox < 0 || oz < 0 {
                continue;
            }
            quads.push((ox as usize, oz as usize));
        }
    }
    quads.sort_unstable_by_key(|&(x, z)| (z, x));
    quads.dedup();
    let cell_at = |x: usize, z: usize| -> Option<&bf_worldgen::water::RiverCell> {
        if x < gx0 || z < gz0 || x > gx1 || z > gz1 {
            return None;
        }
        match grid[(z - gz0) * gw + (x - gx0)] {
            u32::MAX => None,
            c => water.river_cells.get(c as usize),
        }
    };
    // The average of the wet samples near a vertex.
    let vertex = |cx: usize, cz: usize| -> Option<RiverVertex> {
        let mut sum = RiverVertex {
            level: 0.0,
            flow: [0.0; 2],
            foam: 0.0,
        };
        let mut n = 0.0f32;
        for z in cz.saturating_sub(r)..=(cz + r).min(size - 1) {
            for x in cx.saturating_sub(r)..=(cx + r).min(size - 1) {
                if let Some(c) = cell_at(x, z) {
                    sum.level += c.level;
                    sum.flow[0] += c.flow[0];
                    sum.flow[1] += c.flow[1];
                    sum.foam = sum.foam.max(c.foam);
                    n += 1.0;
                }
            }
        }
        (n > 0.0).then(|| RiverVertex {
            level: sum.level / n,
            flow: [sum.flow[0] / n, sum.flow[1] / n],
            foam: sum.foam,
        })
    };

    let mut vertex_of: HashMap<((i32, i32), usize, usize), i32> = HashMap::new();
    for (x, z) in quads {
        let (xe, ze) = ((x + k).min(size - 1), (z + k).min(size - 1));
        if xe == x || ze == z {
            continue;
        }
        let corners = [(x, z), (xe, z), (x, ze), (xe, ze)];
        let values: Vec<RiverVertex> = corners
            .iter()
            .filter_map(|&(cx, cz)| vertex(cx, cz))
            .collect();
        if values.len() < 4 {
            continue;
        }
        // A steep quad is a waterfall: leave it out.
        let lo = values.iter().map(|v| v.level).fold(f32::MAX, f32::min);
        let hi = values.iter().map(|v| v.level).fold(f32::MIN, f32::max);
        if hi - lo > p.max_quad_drop_m {
            continue;
        }
        let tile = tile_of(water, p, world((x + xe) / 2), world((z + ze) / 2));
        let mesh = tiles.entry(tile).or_insert_with(|| TileMesh {
            tile,
            ..TileMesh::default()
        });
        let mut ids = [0i32; 4];
        for (j, (&(cx, cz), v)) in corners.iter().zip(&values).enumerate() {
            ids[j] = *vertex_of.entry((tile, cx, cz)).or_insert_with(|| {
                let ground = heights.get(cx as isize, cz as isize);
                mesh.positions.push([world(cx), v.level, world(cz)]);
                mesh.colours.push(colour);
                mesh.uvs.push([v.level - ground, v.foam]);
                mesh.flows.push(v.flow);
                mesh.positions.len() as i32 - 1
            });
        }
        let [a, b, c, d] = ids;
        let deep_from = body.clarity_m * p.opaque_clarities;
        let deep =
            p.opaque_clarities > 0.0 && ids.iter().all(|&i| mesh.uvs[i as usize][0] > deep_from);
        let list = if deep {
            &mut mesh.deep_indices
        } else {
            &mut mesh.indices
        };
        list.extend([a, b, c, b, d, c]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_worldgen::water::{Look, WaterKind};

    /// A flat zone of 64 samples at 0 m with a round pond of radius 10 m
    /// at level 1 m.
    fn pond() -> (Water, HeightMap) {
        let size = 64;
        let heights = HeightMap::from_fn(size, 1.0, |x, z| {
            let d = ((x as f32 - 32.0).powi(2) + (z as f32 - 32.0).powi(2)).sqrt();
            if d < 10.0 {
                0.0
            } else {
                2.0
            }
        });
        let mut owner = vec![0u16; size * size];
        let (mut min, mut max) = ((u32::MAX, u32::MAX), (0, 0));
        let mut n = 0;
        for z in 0..size {
            for x in 0..size {
                if heights.data[z * size + x] < 1.0 {
                    owner[z * size + x] = 1;
                    min = (min.0.min(x as u32), min.1.min(z as u32));
                    max = (max.0.max(x as u32), max.1.max(z as u32));
                    n += 1;
                }
            }
        }
        let body = WaterBody {
            number: 0,
            kind: WaterKind::Pond,
            level: 1.0,
            spill: 2.0,
            spill_sample: (0, 0),
            deepest_sample: (32, 32),
            min,
            max,
            wet_samples: n,
            area_m2: n as f32,
            max_depth_m: 1.0,
            fill_part: 0.5,
            look: Look::ClearSpring,
            deep_colour: [0.1, 0.4, 0.4],
            clarity_m: 4.0,
            viscosity: 0.0,
            density: 1.0,
            temperature: 0.1,
            flow_mps: 0.0,
        };
        let water = Water {
            size,
            spacing: 1.0,
            bodies: vec![body],
            owner,
            damp_height_m: 0.6,
            ..Water::default()
        };
        (water, heights)
    }

    #[test]
    fn a_pond_is_one_flat_mesh_that_covers_its_water() {
        let (water, heights) = pond();
        let meshes = build_meshes(&water, &heights, &MeshParams::default());
        assert_eq!(meshes.len(), 1);
        let m = &meshes[0];
        assert!(m.positions.iter().all(|p| p[1] == 1.0));
        assert_eq!(m.positions.len(), m.colours.len());
        assert_eq!(m.positions.len(), m.uvs.len());
        assert_eq!(m.positions.len(), m.flows.len());
        assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
        // A circle of 10 m has about 314 m2.
        let area: f32 = m
            .indices
            .chunks(3)
            .map(|t| {
                let [a, b, c] = [0, 1, 2].map(|i| m.positions[t[i] as usize]);
                ((b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])).abs() * 0.5
            })
            .sum();
        assert!(area > 314.0 && area < 600.0, "area {area}");
        // The centre has the full depth, the shore vertices are dry.
        assert!(m.uvs.iter().any(|uv| (uv[0] - 1.0).abs() < 1e-6));
        assert!(m.uvs.iter().any(|uv| uv[0] < 0.0));
        // The clarity is in the alpha.
        assert!((m.colours[0][3] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn deep_quads_go_to_the_opaque_list() {
        let (water, heights) = pond();
        let off = MeshParams {
            opaque_clarities: 0.0,
            ..MeshParams::default()
        };
        let none = &build_meshes(&water, &heights, &off)[0];
        assert!(none.deep_indices.is_empty());
        // The pond is 1 m deep and the clarity is 4 m: 0.2 clarities
        // are already opaque, so the centre is deep.
        let p = MeshParams {
            opaque_clarities: 0.2,
            ..MeshParams::default()
        };
        let m = &build_meshes(&water, &heights, &p)[0];
        assert!(!m.deep_indices.is_empty());
        assert!(!m.indices.is_empty());
        assert_eq!(m.triangles(), none.triangles());
        assert!(m.deep_indices.len().is_multiple_of(6));
    }

    #[test]
    fn triangles_face_up() {
        let (water, heights) = pond();
        let m = &build_meshes(&water, &heights, &MeshParams::default())[0];
        for t in m.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| m.positions[t[i] as usize]);
            let u = [b[0] - a[0], b[2] - a[2]];
            let v = [c[0] - a[0], c[2] - a[2]];
            // Godot draws a face that is clockwise on the screen as the
            // front. Seen from above, x goes to the right and z goes down
            // the screen. Thus a clockwise triangle has a positive cross
            // product in (x, z).
            let cross = u[0] * v[1] - u[1] * v[0];
            assert!(cross > 0.0, "cross {cross}");
        }
    }

    #[test]
    fn small_tiles_split_the_mesh() {
        let (water, heights) = pond();
        let p = MeshParams {
            tile_m: 16.0,
            ..MeshParams::default()
        };
        let meshes = build_meshes(&water, &heights, &p);
        // The pond is at the corner of four tiles of 16 m.
        assert!(meshes.len() >= 4, "tiles {}", meshes.len());
        let all: usize = meshes.iter().map(TileMesh::triangles).sum();
        let one = build_meshes(&water, &heights, &MeshParams::default())[0].triangles();
        assert_eq!(all, one);
    }

    #[test]
    fn a_river_mesh_slopes_and_flows() {
        use bf_worldgen::highlands::{generate, HighlandsParams};
        let zone = generate(7, 1024, 2.0, &HighlandsParams::default());
        let w = &zone.water;
        let p = MeshParams {
            river_cell_m: 2.0,
            ..MeshParams::default()
        };
        let meshes = build_meshes(w, &zone.heights, &p);
        let flowing = meshes
            .iter()
            .flat_map(|m| &m.flows)
            .filter(|f| f[0] != 0.0 || f[1] != 0.0)
            .count();
        assert!(flowing > 100, "flowing vertices {flowing}");
        // Each river vertex is within the levels of the river cells.
        let lo = w
            .river_cells
            .iter()
            .map(|c| c.level)
            .fold(f32::MAX, f32::min);
        let hi = w
            .river_cells
            .iter()
            .map(|c| c.level)
            .fold(f32::MIN, f32::max);
        for m in &meshes {
            for (v, f) in m.positions.iter().zip(&m.flows) {
                if f[0] != 0.0 || f[1] != 0.0 {
                    assert!(v[1] >= lo && v[1] <= hi);
                }
            }
            assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
            assert!(m.uvs.iter().all(|uv| (0.0..=1.0).contains(&uv[1])));
        }
    }

    #[test]
    fn large_quads_are_cut_at_the_shore() {
        let (water, heights) = pond();
        let coarse = MeshParams {
            cell_m: 4.0,
            shore_detail_m: 0.0,
            ..MeshParams::default()
        };
        let fine = MeshParams {
            cell_m: 4.0,
            shore_detail_m: 1.5,
            ..MeshParams::default()
        };
        let a = &build_meshes(&water, &heights, &coarse)[0];
        let b = &build_meshes(&water, &heights, &fine)[0];
        // The pond is 1 m deep: all quads are at the shore and are cut.
        assert!(b.triangles() > a.triangles());
        let area = |m: &TileMesh| m.triangles();
        // Cut quads cover less dry ground: less than 4 times the count.
        assert!(area(b) < area(a) * 4);
        assert!(b.indices.iter().all(|&i| (i as usize) < b.positions.len()));
    }
}
