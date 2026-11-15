//! The cave layer (demo 2, `plan/demo-2.md`, "The cave layer").
//!
//! The caves are a separate layer under the heightmap. Each zone has
//! two to five cave systems. Each system is a seeded graph:
//! - a **node** is a chamber (a flattened ball with a flat floor) or a
//!   joint (a point where tunnels meet);
//! - an **edge** is a tunnel (a chain of capsules with a changing
//!   radius, bent by noise) or a shaft (a vertical tunnel).
//!
//! Smooth union joins the shapes. A signed distance field (SDF) gives
//! the distance to the nearest cave wall. The layer does not store the
//! SDF. It computes the SDF from the graph when a chunk or a query needs
//! it. A spatial grid lists the shapes near each column of the zone, so
//! a query reads only a few shapes. A block with no shape near it has no
//! cave (the bound check).
//!
//! **Signs.** [`Caves::cave_value`] is negative in cave air and positive
//! in rock. [`Caves::air_value`] joins the caves and the ground: it is
//! positive in air (open air or cave air) and negative in rock. Surface
//! Nets (`fast-surface-nets`) takes negative values as "inside", so the
//! rock is inside and the normals point into the air.
//!
//! The same zone and the same settings always give the same caves, with
//! the same bits on all computers. The code uses no `sin`, `cos`,
//! `exp`, `powf` or `powi` from the standard library (`detmath` has
//! the ones that it needs).

mod build;
mod field;
mod noise3;
#[cfg(test)]
mod tests;
mod walk;

pub use field::Block;
pub use walk::{Tour, TourPoint, TOUR_PARTS, TOUR_REACH_M};

use crate::highlands::Zone;
use crate::smoke::SmokeParams;

/// The settings of the cave generator. Lengths are in metres. A slope
/// is the rise for each metre on the ground plane (1.0 is 45 degrees).
#[derive(Clone, Debug, PartialEq)]
pub struct CaveParams {
    /// The fewest cave systems in a zone.
    pub systems_min: u32,
    /// The most cave systems in a zone.
    pub systems_max: u32,
    /// All caves stay this far inside the nearest place of the smoke front.
    pub edge_margin_m: f32,
    /// The smallest distance between the hillside mouths of two systems.
    pub system_spacing_m: f32,
    /// No mouth is nearer than this to the start point of the zone.
    pub start_clear_m: f32,
    /// The first system has its hillside mouth within this distance of
    /// the start point, if the zone has a place for it.
    pub near_mouth_m: f32,
    /// A system goes down at least this far below its highest mouth.
    pub depth_min_m: f32,
    /// A system goes down at most this far below its highest mouth.
    pub depth_max_m: f32,
    /// The largest height between the floors of two chambers in a chain.
    pub link_drop_m: f32,
    /// The first chamber of each system (the hall) is at least this wide.
    pub hall_width_min_m: f32,
    /// The smallest width of a chamber.
    pub chamber_width_min_m: f32,
    /// The largest width of a chamber.
    pub chamber_width_max_m: f32,
    /// The smallest height of a chamber.
    pub chamber_height_min_m: f32,
    /// The largest height of a chamber.
    pub chamber_height_max_m: f32,
    /// The height of a chamber is at least this part of its width.
    pub chamber_flat_min: f32,
    /// The height of a chamber is at most this part of its width.
    pub chamber_flat_max: f32,
    /// The flat floor cuts the ball at this part of its half height
    /// below the centre (0 is the middle, 1 is the bottom).
    pub floor_cut: f32,
    /// The smallest width of a tunnel.
    pub tunnel_width_min_m: f32,
    /// The largest width of a tunnel.
    pub tunnel_width_max_m: f32,
    /// The smallest width of a hillside mouth.
    pub mouth_width_min_m: f32,
    /// The largest width of a hillside mouth.
    pub mouth_width_max_m: f32,
    /// The smallest width of a sinkhole shaft.
    pub shaft_width_min_m: f32,
    /// The largest width of a sinkhole shaft.
    pub shaft_width_max_m: f32,
    /// The longest sinkhole shaft, from the ground to the chamber roof.
    pub shaft_length_max_m: f32,
    /// The chance (0 to 1) that a system has a sinkhole.
    pub sinkhole_chance: f32,
    /// The most dead-end side tunnels in a system.
    pub branch_max: u32,
    /// The chance (0 to 1) of an extra tunnel that makes a loop.
    pub loop_chance: f32,
    /// A hillside mouth is on a slope of at least this value.
    pub hill_slope_min: f32,
    /// A hillside mouth is on a slope of at most this value.
    pub hill_slope_max: f32,
    /// The slope of the first tunnel from a hillside mouth into the hill.
    pub entry_descent: f32,
    /// The tunnel from the entrance to the hall is not steeper than this.
    pub walk_slope: f32,
    /// The steepest tunnel between two chambers.
    pub steep_slope: f32,
    /// The thinnest rock between a cave and the ground above it, except
    /// at mouths.
    pub roof_min_m: f32,
    /// The length of one capsule of a tunnel.
    pub segment_m: f32,
    /// How far noise bends a tunnel to the side.
    pub bend_m: f32,
    /// How far noise bends a tunnel up or down.
    pub bend_up_m: f32,
    /// The length of one bend of a tunnel.
    pub bend_wavelength_m: f32,
    /// How much the radius of a tunnel changes along it (0 to 1).
    pub radius_wobble: f32,
    /// The size of the bumps in the cave walls.
    pub wall_noise_wavelength_m: f32,
    /// The height of the bumps in a chamber, as a part of its half height.
    pub chamber_noise_part: f32,
    /// The height of the bumps in a tunnel, as a part of its radius.
    pub tunnel_noise_part: f32,
    /// The bumps are never higher than this.
    pub noise_max_m: f32,
    /// The size of the smooth join between two shapes (the smooth-union
    /// radius). The join adds at most this much air.
    pub smooth_m: f32,
    /// The side of one cell of the spatial grid.
    pub grid_cell_m: f32,
    /// Sky visibility falls to 37% at this distance along the tunnels
    /// from a mouth.
    pub sky_falloff_m: f32,
    /// The light goes this much farther down a vertical shaft than
    /// along a tunnel (a part: 0.35 counts each metre of shaft as 0.35 m).
    pub shaft_light_scale: f32,
}

impl Default for CaveParams {
    fn default() -> Self {
        Self {
            systems_min: 2,
            systems_max: 5,
            edge_margin_m: 60.0,
            system_spacing_m: 220.0,
            start_clear_m: 80.0,
            near_mouth_m: 450.0,
            depth_min_m: 250.0,
            depth_max_m: 400.0,
            link_drop_m: 110.0,
            hall_width_min_m: 100.0,
            chamber_width_min_m: 40.0,
            chamber_width_max_m: 250.0,
            chamber_height_min_m: 20.0,
            chamber_height_max_m: 120.0,
            chamber_flat_min: 0.25,
            chamber_flat_max: 0.6,
            floor_cut: 0.6,
            tunnel_width_min_m: 3.0,
            tunnel_width_max_m: 25.0,
            mouth_width_min_m: 4.0,
            mouth_width_max_m: 10.0,
            shaft_width_min_m: 6.0,
            shaft_width_max_m: 16.0,
            shaft_length_max_m: 150.0,
            sinkhole_chance: 0.5,
            branch_max: 2,
            loop_chance: 0.35,
            hill_slope_min: 0.35,
            hill_slope_max: 1.2,
            entry_descent: 0.15,
            walk_slope: 0.36,
            steep_slope: 0.9,
            roof_min_m: 8.0,
            segment_m: 10.0,
            bend_m: 5.0,
            bend_up_m: 1.5,
            bend_wavelength_m: 60.0,
            radius_wobble: 0.2,
            wall_noise_wavelength_m: 40.0,
            chamber_noise_part: 0.2,
            tunnel_noise_part: 0.2,
            noise_max_m: 8.0,
            smooth_m: 3.0,
            grid_cell_m: 16.0,
            sky_falloff_m: 24.0,
            shaft_light_scale: 0.35,
        }
    }
}

/// The kind of a node of the cave graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A chamber: a flattened ball with a flat floor.
    Chamber,
    /// A point where tunnels meet, or the end of a tunnel.
    Joint,
}

/// One node of a cave system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaveNode {
    /// The centre (x, y, z) in metres.
    pub pos: [f32; 3],
    /// A chamber: half of its width. A joint: the radius of its tunnel.
    pub radius_m: f32,
    /// A chamber: half of its height (of the full ball). A joint: 0.
    pub half_height_m: f32,
    /// A chamber: the height (y) of its flat floor. A joint: its centre.
    pub floor_y: f32,
    /// The kind of the node.
    pub kind: NodeKind,
    /// The distance along the tunnels to the nearest mouth, for the sky
    /// visibility. A metre of shaft counts as `shaft_light_scale` m.
    pub mouth_path_m: f32,
}

/// The kind of an edge of the cave graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// A tunnel: capsules along a bent path.
    Tunnel,
    /// A vertical tunnel from a sinkhole mouth down to a chamber.
    Shaft,
    /// A link from a chamber to a joint inside it. It makes no shape.
    Inside,
}

/// One edge of a cave system.
#[derive(Clone, Debug, PartialEq)]
pub struct CaveEdge {
    /// The index of the first node in [`CaveSystem::nodes`].
    pub a: usize,
    /// The index of the second node.
    pub b: usize,
    /// The radius at node `a`.
    pub radius_start_m: f32,
    /// The radius at node `b`.
    pub radius_end_m: f32,
    /// The seed of the noise of this edge.
    pub noise_seed: u32,
    /// The kind of the edge.
    pub kind: EdgeKind,
    /// The bent path from node `a` to node `b`, with both ends.
    pub path: Vec<[f32; 3]>,
    /// The radius at each point of `path`.
    pub radii: Vec<f32>,
    /// The length of `path` in metres.
    pub length_m: f32,
}

/// The kind of a cave mouth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouthKind {
    /// A tunnel comes out of a slope sideways.
    Hillside,
    /// A shaft opens upward from a chamber.
    Sinkhole,
    /// A floor that breaks and drops the bear into a cave (R23). The
    /// generator does not make these yet (S-172).
    CrumblingFloor,
}

/// One place where a cave opens to the surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Mouth {
    /// The point on the ground at the middle of the mouth.
    pub pos: [f32; 3],
    /// The kind of the mouth.
    pub kind: MouthKind,
    /// The radius of the tunnel or shaft at the mouth.
    pub radius_m: f32,
    /// The index of the node at the outer end of the mouth tunnel.
    pub node: usize,
    /// The direction into the cave on the ground plane (x, z), a unit
    /// vector. A sinkhole has (0, 0).
    pub facing: [f32; 2],
    /// The rock above the cave can be thinner than `roof_min_m` in
    /// this horizontal distance from `pos`.
    pub exempt_radius_m: f32,
}

/// One cave system: a graph of chambers and tunnels.
#[derive(Clone, Debug, PartialEq)]
pub struct CaveSystem {
    /// The number of the system in the zone (0, 1, 2 ...).
    pub index: usize,
    /// The chambers and joints.
    pub nodes: Vec<CaveNode>,
    /// The tunnels, shafts and inside links.
    pub edges: Vec<CaveEdge>,
    /// The mouths. The first mouth is a hillside mouth.
    pub mouths: Vec<Mouth>,
    /// The lowest point of cave air (without the wall bumps).
    pub lowest_y: f32,
    /// The highest point of cave air (without the wall bumps). A mouth
    /// tunnel can come out above the ground.
    pub highest_y: f32,
}

impl CaveSystem {
    /// Gives the height of the highest mouth (on the ground).
    pub fn highest_mouth_y(&self) -> f32 {
        self.mouths
            .iter()
            .map(|m| m.pos[1])
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Gives the indices of the chamber nodes.
    pub fn chambers(&self) -> impl Iterator<Item = usize> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == NodeKind::Chamber)
            .map(|(i, _)| i)
    }
}

/// A chamber of the zone: the system number and the node number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChamberRef {
    /// The index in [`Caves::systems`].
    pub system: usize,
    /// The index in [`CaveSystem::nodes`].
    pub node: usize,
}

/// The cave layer of one zone.
#[derive(Clone, Debug)]
pub struct Caves {
    /// The seed of the zone.
    pub seed: u32,
    /// The settings that made the caves.
    pub params: CaveParams,
    /// All caves stay inside this radius from the centre of the zone.
    pub safe_radius_m: f32,
    /// The cave systems.
    pub systems: Vec<CaveSystem>,
    /// The number of tunnel parts that could not get a roof of
    /// `roof_min_m`. It is 0 in a good zone.
    pub roof_faults: usize,
    shapes: Vec<field::Shape>,
    grid: field::Grid,
    noise_seed: u32,
}

/// Gives the radius inside which the smoke front never comes, minus
/// `margin_m`. `half_size_m` is half the side of the zone.
pub fn safe_radius(half_size_m: f32, smoke: &SmokeParams, margin_m: f32) -> f32 {
    let nearest_front = ((smoke.front_fraction - smoke.swing_fraction) * half_size_m)
        .max(smoke.min_front_m)
        .min((half_size_m - smoke.border_margin_m).max(smoke.min_front_m));
    (nearest_front - margin_m).max(0.0)
}

impl Caves {
    /// Makes the caves of a zone. Make them right after
    /// `highlands::generate` and before `place_obstacles`: the caves read
    /// the heights, and the ash-crust pits change them. The roof has 1 m
    /// more rock than `roof_min_m` for the pits (0.55 m). `smoke` gives
    /// the safe part of the zone.
    pub fn generate(zone: &Zone, smoke: &SmokeParams, params: &CaveParams) -> Self {
        let half = zone.heights.world_size() * 0.5;
        let safe = safe_radius(half, smoke, params.edge_margin_m);
        let (systems, roof_faults) = build::build(zone, safe, params);
        let noise_seed = zone.seed ^ 0xCA7E_5EED;
        let shapes = field::make_shapes(&systems, params);
        let grid = field::Grid::new(&shapes, half, params.grid_cell_m);
        Self {
            seed: zone.seed,
            params: params.clone(),
            safe_radius_m: safe,
            systems,
            roof_faults,
            shapes,
            grid,
            noise_seed,
        }
    }

    /// Gives all mouths of all systems.
    pub fn mouths(&self) -> impl Iterator<Item = &Mouth> {
        self.systems.iter().flat_map(|s| s.mouths.iter())
    }

    /// Gives the number of SDF shapes (chambers and capsules).
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }

    /// Gives a hash of the cave graph. Use it to compare two zones.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut put = |v: u32| {
            for b in v.to_le_bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        for s in &self.systems {
            put(s.index as u32);
            for n in &s.nodes {
                for v in n.pos {
                    put(v.to_bits());
                }
                put(n.radius_m.to_bits());
                put(n.half_height_m.to_bits());
                put(n.floor_y.to_bits());
                put(n.kind as u32);
                put(n.mouth_path_m.to_bits());
            }
            for e in &s.edges {
                put(e.a as u32);
                put(e.b as u32);
                put(e.noise_seed);
                put(e.kind as u32);
                for (p, r) in e.path.iter().zip(&e.radii) {
                    for v in p {
                        put(v.to_bits());
                    }
                    put(r.to_bits());
                }
            }
            for m in &s.mouths {
                for v in m.pos {
                    put(v.to_bits());
                }
                put(m.kind as u32);
                put(m.radius_m.to_bits());
            }
        }
        h
    }
}
