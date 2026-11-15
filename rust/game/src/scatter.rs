//! `BfScatter`: rocks, pebbles and dead shrubs on the ground (S-108).
//!
//! The placement is in `bf_worldgen::scatter`. It is a pure function of
//! the zone: the same seed makes the same props. This node is the thin
//! part that talks to Godot:
//!
//! - **Chunks.** The ground is cut into square chunks. For each class,
//!   the node makes the chunks near the camera and frees the far ones.
//!   Each chunk is one `MultiMeshInstance3D` for each mesh shape. A
//!   MultiMesh draws many copies of one mesh in one draw call. The
//!   chunks are made on demand, a few each frame (`build_budget_ms`),
//!   so the zone does not need 1.8 million pebbles in memory.
//! - **Range.** Each chunk is in the group `bf_view_ranged`. Its
//!   visibility range ends at the view distance, or at the range of its
//!   class if that is shorter (pebbles: 50 m).
//! - **Collision.** Rocks above `collision_min_size_m` get a collision
//!   body only near the bear. A pool of bodies moves to the nearest
//!   rocks. A body has the shape of the mesh (a convex hull) and the
//!   grip of rock (`bf_grip`). Pebbles and shrubs have no collision.
//!   The bear walks through shrubs [assumed].
//!
//! Cost: one draw call for each visible chunk and shape. The overlay
//! shows the numbers (`debug_text`).

use crate::bear::{BEAR_GROUP, GRIP_META, SOFT_META};
use crate::terrain::{BfTerrain, TERRAIN_GROUP};
use crate::view_distance::{
    ranged_end, VIEW_DISTANCE_GROUP, VIEW_MARGIN_META, VIEW_MAX_META, VIEW_RANGED_GROUP,
};
use bf_worldgen::props::{pebble_mesh, rock_mesh, shrub_mesh, MeshData};
use bf_worldgen::scatter::{
    chunk_instances, exclusions, Class, Exclusion, Instance, ScatterParams,
};
use glam::{EulerRot, Mat3, Vec3};
use godot::classes::base_material_3d::Flags;
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::multi_mesh::TransformFormat;
use godot::classes::{
    ArrayMesh, CollisionShape3D, ConvexPolygonShape3D, DisplayServer, INode3D, Material, MultiMesh,
    MultiMeshInstance3D, Node3D, StandardMaterial3D, StaticBody3D,
};
use godot::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// The group that holds the `BfScatter` node.
pub const SCATTER_GROUP: &str = "bf_scatter";

/// The grip of a rock that the bear grabs (1 is full grip).
const ROCK_GRIP: f32 = 0.9;

/// A chunk adds this much to its box, so the big rocks at its border
/// are inside it, in metres.
const AABB_MARGIN_M: f32 = 4.0;

/// Chunks stay a little longer than their range, so a chunk at the
/// edge does not go and come all the time. In chunk sizes.
const KEEP_EXTRA: f32 = 0.35;

/// The time between two looks for new chunks, in seconds.
const REFRESH_SECONDS: f64 = 0.1;

/// The time between two moves of the collision bodies, in seconds.
const BODY_SECONDS: f64 = 0.2;

fn to_godot(v: Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

/// Gives the transform of a prop: turn and lean, then size.
fn instance_matrix(inst: &Instance) -> Mat3 {
    Mat3::from_euler(EulerRot::YXZ, inst.yaw, inst.tilt[0], inst.tilt[1])
        * Mat3::from_diagonal(Vec3::from_array(inst.scale))
}

/// Gives the colour of a prop (red, green, blue). It is the ground
/// colour of rust or ash rock, made light or dark by the shade.
fn instance_colour(class: Class, inst: &Instance) -> [f32; 3] {
    let mix = |a: [f32; 3], b: [f32; 3]| {
        [
            a[0] + (b[0] - a[0]) * inst.tone,
            a[1] + (b[1] - a[1]) * inst.tone,
            a[2] + (b[2] - a[2]) * inst.tone,
        ]
    };
    let base = match class {
        // The same bands as the ground rock (`Zone::ground_colours`).
        Class::Rock => mix([0.50, 0.31, 0.20], [0.50, 0.42, 0.36]),
        // Pebbles are paler than the rock, like the dust on them.
        Class::Pebble => mix([0.62, 0.50, 0.38], [0.66, 0.60, 0.52]),
        // Dead wood: grey-brown.
        Class::Shrub => [0.33, 0.27, 0.21],
    };
    [
        base[0] * inst.shade,
        base[1] * inst.shade,
        base[2] * inst.shade,
    ]
}

/// Writes one prop in the buffer of a MultiMesh: 12 numbers for the
/// transform (3 rows of 4, the origin is the last column) and 4 for the
/// colour. `centre` is the place of the node of the chunk.
fn pack_instance(class: Class, inst: &Instance, centre: Vec3, out: &mut Vec<f32>) {
    let m = instance_matrix(inst);
    let o = Vec3::from_array(inst.pos) - centre;
    out.extend_from_slice(&[
        m.x_axis.x, m.y_axis.x, m.z_axis.x, o.x, //
        m.x_axis.y, m.y_axis.y, m.z_axis.y, o.y, //
        m.x_axis.z, m.y_axis.z, m.z_axis.z, o.z,
    ]);
    let c = instance_colour(class, inst);
    out.extend_from_slice(&[c[0], c[1], c[2], 1.0]);
}

/// Gives the distance from a point to a chunk (a square on the ground).
fn chunk_distance(x: f32, z: f32, cx: i32, cz: i32, chunk_m: f32) -> f32 {
    let (x0, z0) = (cx as f32 * chunk_m, cz as f32 * chunk_m);
    let dx = (x0 - x).max(x - (x0 + chunk_m)).max(0.0);
    let dz = (z0 - z).max(z - (z0 + chunk_m)).max(0.0);
    (dx * dx + dz * dz).sqrt()
}

/// Gives the chunks that are nearer than `range_m` to the point, with
/// their distance. The nearest chunk is first.
fn wanted_chunks(x: f32, z: f32, range_m: f32, chunk_m: f32) -> Vec<(i32, i32, f32)> {
    let mut out = Vec::new();
    if range_m <= 0.0 || chunk_m <= 0.0 {
        return out;
    }
    let lo = |v: f32| ((v - range_m) / chunk_m).floor() as i32;
    let hi = |v: f32| ((v + range_m) / chunk_m).floor() as i32;
    for cz in lo(z)..=hi(z) {
        for cx in lo(x)..=hi(x) {
            let d = chunk_distance(x, z, cx, cz, chunk_m);
            if d <= range_m {
                out.push((cx, cz, d));
            }
        }
    }
    out.sort_by(|a, b| a.2.total_cmp(&b.2).then((a.0, a.1).cmp(&(b.0, b.1))));
    out
}

/// The key of a rock: its chunk and its number in the chunk.
type RockKey = (i32, i32, u32);

/// Chooses the rocks that get a body: the nearest ones, `count` at most.
/// Each candidate is (distance, key).
fn nearest_rocks(mut candidates: Vec<(f32, RockKey)>, count: usize) -> Vec<RockKey> {
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    candidates.truncate(count);
    candidates.into_iter().map(|(_, key)| key).collect()
}

/// The nodes and the data of one chunk.
struct Chunk {
    nodes: Vec<Gd<MultiMeshInstance3D>>,
    /// The props of the chunk. Only rocks keep them, for the bodies.
    rocks: Vec<Instance>,
    count: usize,
}

/// One collision body of the pool.
struct BodySlot {
    body: Gd<StaticBody3D>,
    shape: Gd<CollisionShape3D>,
    key: Option<RockKey>,
}

/// Rocks, pebbles and dead shrubs. Dev menu: the scatter can go off.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfScatter {
    /// Turns the scatter on or off. Off frees all props.
    #[export]
    #[init(val = true)]
    enabled: bool,

    /// Shows the rocks.
    #[export]
    #[init(val = true)]
    show_rocks: bool,

    /// Shows the pebbles.
    #[export]
    #[init(val = true)]
    show_pebbles: bool,

    /// Shows the dead shrubs.
    #[export]
    #[init(val = true)]
    show_shrubs: bool,

    /// Multiplies the number of rocks. 2 makes twice as many.
    #[export]
    #[init(val = 1.0)]
    rock_density: f32,

    /// Multiplies the number of pebbles.
    #[export]
    #[init(val = 1.0)]
    pebble_density: f32,

    /// Multiplies the number of dead shrubs.
    #[export]
    #[init(val = 1.0)]
    shrub_density: f32,

    /// Multiplies the size of the rocks. The default sizes run from
    /// 0.18 m to 3.2 m wide.
    #[export]
    #[init(val = 1.0)]
    rock_size_scale: f32,

    /// Multiplies the size of the pebbles (2.5 to 10 cm wide).
    #[export]
    #[init(val = 1.0)]
    pebble_size_scale: f32,

    /// Multiplies the size of the shrubs (0.3 m to 1.1 m high).
    #[export]
    #[init(val = 1.0)]
    shrub_size_scale: f32,

    /// How far rocks are drawn, in metres. The view distance (the smog)
    /// can make it shorter.
    #[export]
    #[init(val = 320.0)]
    rock_range_m: f32,

    /// How far pebbles are drawn, in metres. Pebbles are very small, so
    /// this is short.
    #[export]
    #[init(val = 50.0)]
    pebble_range_m: f32,

    /// How far shrubs are drawn, in metres.
    #[export]
    #[init(val = 110.0)]
    shrub_range_m: f32,

    /// If this is on, rocks cast shadows (the shadow range of the sun
    /// still limits it).
    #[export]
    #[init(val = true)]
    rock_shadows: bool,

    /// Rocks nearer to the bear than this have a collision body, in
    /// metres.
    #[export]
    #[init(val = 16.0)]
    collision_range_m: f32,

    /// Rocks narrower than this have no collision body, in metres. The
    /// bear walks over small stones.
    #[export]
    #[init(val = 0.25)]
    collision_min_size_m: f32,

    /// The most collision bodies at one time.
    #[export]
    #[init(val = 80)]
    collision_pool: i32,

    /// The time that the node may use for new chunks in one frame, in
    /// milliseconds. At least one chunk is made each frame.
    #[export]
    #[init(val = 1.5)]
    build_budget_ms: f32,

    terrain: Option<Gd<BfTerrain>>,
    /// The settings that the chunks were made with.
    built_params: Option<ScatterParams>,
    exclusions: Vec<Exclusion>,
    /// A zone is on the screen.
    zone_ready: bool,
    chunks: HashMap<(usize, i32, i32), Chunk>,
    /// Chunks to make: (class, cx, cz, distance). The nearest is last.
    pending: Vec<(usize, i32, i32, f32)>,
    /// The meshes of each class, one for each shape.
    meshes: Vec<Vec<Gd<ArrayMesh>>>,
    /// The corners of the collision hull of each rock shape.
    rock_hulls: Vec<Vec<Vec3>>,
    material: Option<Gd<StandardMaterial3D>>,
    pool: Vec<BodySlot>,
    assigned: HashMap<RockKey, usize>,
    since_refresh: f64,
    since_bodies: f64,
    /// The time of the last chunk, and the longest one, in ms.
    last_build_ms: f32,
    max_build_ms: f32,
    /// Tells if the buffer layout was checked.
    layout_checked: bool,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfScatter {
    fn ready(&mut self) {
        self.base_mut().add_to_group(SCATTER_GROUP);
        self.make_meshes();
        let tree = self.base().get_tree();
        if let Some(node) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let mut node = node;
            let on_zone = self.base().callable("on_zone_changed");
            node.connect("zone_changed", &on_zone);
            self.terrain = node.try_cast::<BfTerrain>().ok();
        }
    }

    fn process(&mut self, delta: f64) {
        if !self.enabled || !self.zone_ready {
            if !self.chunks.is_empty() {
                self.clear_all();
            }
            return;
        }
        let params = self.params();
        if self.built_params.as_ref() != Some(&params) {
            // A setting changed (the Inspector or the dev menu).
            self.clear_all();
            self.built_params = Some(params);
        }
        let Some((x, z)) = self.camera_xz() else {
            return;
        };
        self.since_refresh += delta;
        if self.since_refresh >= REFRESH_SECONDS {
            self.since_refresh = 0.0;
            self.refresh(x, z);
        }
        self.build_pending();
    }

    fn physics_process(&mut self, delta: f64) {
        self.since_bodies += delta;
        if self.since_bodies < BODY_SECONDS {
            return;
        }
        self.since_bodies = 0.0;
        self.update_bodies();
    }
}

#[godot_api]
impl BfScatter {
    /// Called when the terrain shows a new zone: all props are new.
    #[func]
    fn on_zone_changed(&mut self) {
        self.clear_all();
        self.zone_ready = false;
        let Some(terrain) = self.terrain.clone() else {
            return;
        };
        let params = self.params();
        {
            let t = terrain.bind();
            let Some(zone) = t.zone() else {
                return;
            };
            self.exclusions = exclusions(zone, t.obstacles(), &params);
            // No props in the holes of the cave mouths (S-164).
            if let Some(world) = t.cave_world() {
                let pad = (world.rim.ring() as f32 + 4.0) * zone.heights.spacing;
                self.exclusions
                    .extend(world.caves.mouths().map(|m| Exclusion {
                        x: m.pos[0],
                        z: m.pos[2],
                        radius: m.exempt_radius_m + pad,
                        pebbles: true,
                    }));
            }
        }
        self.built_params = Some(params);
        self.zone_ready = true;
    }

    /// Turns the scatter on or off (the dev menu).
    #[func]
    pub fn set_scatter_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    /// Tells if the scatter is on.
    #[func]
    pub fn is_scatter_enabled(&self) -> bool {
        self.enabled
    }

    /// Gives the numbers of the scatter as one line for the overlay.
    #[func]
    pub fn debug_text(&self) -> GString {
        if !self.enabled {
            return GString::from("off");
        }
        let mut parts = Vec::new();
        for class in Class::ALL {
            let (chunks, props) = self
                .chunks
                .iter()
                .filter(|((c, _, _), _)| *c == class.index())
                .fold((0, 0), |(a, b), (_, ch)| (a + 1, b + ch.count));
            parts.push(format!("{} {}/{}", class.name(), chunks, props));
        }
        let bodies = self.assigned.len();
        GString::from(&format!(
            "{} (chunks/props), bodies {bodies}, queue {}, chunk {:.2} ms (max {:.2})",
            parts.join(", "),
            self.pending.len(),
            self.last_build_ms,
            self.max_build_ms,
        ))
    }
}

impl BfScatter {
    /// Gives the settings of the placement from the exported values.
    fn params(&self) -> ScatterParams {
        let mut p = ScatterParams::default();
        let set = |c: &mut bf_worldgen::scatter::ClassParams, on: bool, density: f32, size: f32| {
            c.chance *= if on { density.max(0.0) } else { 0.0 };
            let size = size.max(0.05);
            c.size_min_m *= size;
            c.size_max_m *= size;
        };
        set(
            &mut p.rock,
            self.show_rocks,
            self.rock_density,
            self.rock_size_scale,
        );
        set(
            &mut p.pebble,
            self.show_pebbles,
            self.pebble_density,
            self.pebble_size_scale,
        );
        set(
            &mut p.shrub,
            self.show_shrubs,
            self.shrub_density,
            self.shrub_size_scale,
        );
        p
    }

    /// Gives the range of a class in metres: its own range, or the view
    /// distance if that is shorter.
    fn range_m(&self, class: Class) -> f32 {
        let own = match class {
            Class::Rock => self.rock_range_m,
            Class::Pebble => self.pebble_range_m,
            Class::Shrub => self.shrub_range_m,
        };
        own.min(self.view_distance()).max(0.0)
    }

    fn view_distance(&self) -> f32 {
        if !self.base().is_inside_tree() {
            return 767.0;
        }
        self.base()
            .get_tree()
            .get_first_node_in_group(VIEW_DISTANCE_GROUP)
            .map(|mut n| n.call("view_distance_m", &[]).to::<f32>())
            .unwrap_or(767.0)
    }

    fn camera_xz(&self) -> Option<(f32, f32)> {
        let camera = self.base().get_viewport()?.get_camera_3d()?;
        let p = camera.get_global_position();
        Some((p.x, p.z))
    }

    /// Makes the meshes and the material. They stay for all zones.
    fn make_meshes(&mut self) {
        let mut material = StandardMaterial3D::new_gd();
        material.set_roughness(1.0);
        material.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, true);
        let shapes: [Vec<MeshData>; 3] = [
            (0..Class::Rock.variants()).map(rock_mesh).collect(),
            (0..Class::Pebble.variants()).map(pebble_mesh).collect(),
            (0..Class::Shrub.variants()).map(shrub_mesh).collect(),
        ];
        self.rock_hulls = shapes[0]
            .iter()
            .map(|m| {
                m.unique_points()
                    .iter()
                    .map(|p| Vec3::from_array(*p))
                    .collect()
            })
            .collect();
        self.meshes = shapes
            .iter()
            .map(|list| {
                list.iter()
                    .map(|data| make_mesh(data, &material))
                    .collect::<Vec<_>>()
            })
            .collect();
        self.material = Some(material);
    }

    /// Looks for chunks to make and chunks to free.
    fn refresh(&mut self, x: f32, z: f32) {
        let Some(params) = self.built_params.clone() else {
            return;
        };
        let mut pending = Vec::new();
        for class in Class::ALL {
            let range = self.range_m(class);
            let chunk_m = params.chunk_m(class);
            let on = params.class(class).chance > 0.0;
            let wanted = if on {
                wanted_chunks(x, z, range, chunk_m)
            } else {
                Vec::new()
            };
            for (cx, cz, d) in wanted {
                if !self.chunks.contains_key(&(class.index(), cx, cz)) {
                    pending.push((class.index(), cx, cz, d));
                }
            }
            // Free the chunks that are too far.
            let keep = range + chunk_m * KEEP_EXTRA;
            let far: Vec<(usize, i32, i32)> = self
                .chunks
                .keys()
                .filter(|(c, cx, cz)| {
                    *c == class.index() && (!on || chunk_distance(x, z, *cx, *cz, chunk_m) > keep)
                })
                .copied()
                .collect();
            for key in far {
                if let Some(chunk) = self.chunks.remove(&key) {
                    for mut node in chunk.nodes {
                        node.queue_free();
                    }
                }
            }
        }
        // The nearest chunk goes last, so `pop` gives it.
        pending.sort_by(|a, b| b.3.total_cmp(&a.3));
        self.pending = pending;
    }

    /// Makes chunks from the queue until the budget is used.
    fn build_pending(&mut self) {
        let start = Instant::now();
        let budget = self.build_budget_ms.max(0.1) / 1000.0;
        while let Some((class_index, cx, cz, _)) = self.pending.pop() {
            let t0 = Instant::now();
            self.build_chunk(Class::ALL[class_index], cx, cz);
            self.last_build_ms = t0.elapsed().as_secs_f32() * 1000.0;
            self.max_build_ms = self.max_build_ms.max(self.last_build_ms);
            if start.elapsed().as_secs_f32() >= budget {
                break;
            }
        }
    }

    /// Makes the props of one chunk and their nodes.
    fn build_chunk(&mut self, class: Class, cx: i32, cz: i32) {
        let (Some(params), Some(terrain)) = (self.built_params.clone(), self.terrain.clone())
        else {
            return;
        };
        let instances = {
            let t = terrain.bind();
            let Some(zone) = t.zone() else {
                return;
            };
            chunk_instances(zone, &params, &self.exclusions, class, cx, cz)
        };
        let chunk_m = params.chunk_m(class);
        let centre = Vec3::new(
            (cx as f32 + 0.5) * chunk_m,
            0.0,
            (cz as f32 + 0.5) * chunk_m,
        );
        let key = (class.index(), cx, cz);
        let count = instances.len();
        let mut nodes = Vec::new();

        for variant in 0..class.variants() {
            let mut buffer: Vec<f32> = Vec::new();
            let (mut y_lo, mut y_hi) = (f32::INFINITY, f32::NEG_INFINITY);
            let mut n = 0;
            for inst in instances.iter().filter(|i| i.variant == variant) {
                pack_instance(class, inst, centre, &mut buffer);
                y_lo = y_lo.min(inst.pos[1]);
                y_hi = y_hi.max(inst.pos[1] + inst.scale[1] * 1.3 + 0.3);
                n += 1;
            }
            if n == 0 {
                continue;
            }
            let Some(mesh) = self
                .meshes
                .get(class.index())
                .and_then(|l| l.get(variant as usize))
            else {
                continue;
            };
            let mut multimesh = MultiMesh::new_gd();
            multimesh.set_transform_format(TransformFormat::TRANSFORM_3D);
            multimesh.set_use_colors(true);
            multimesh.set_mesh(&mesh.clone().upcast::<godot::classes::Mesh>());
            multimesh.set_instance_count(n);
            multimesh.set_buffer(&PackedFloat32Array::from(buffer.as_slice()));
            // The headless renderer does not keep the buffer, so it
            // cannot check the layout.
            if !self.layout_checked && !is_headless() {
                self.layout_checked = true;
                // The first prop must come back as we wrote it.
                let back = multimesh.get_instance_transform(0).origin;
                let first = instances
                    .iter()
                    .find(|i| i.variant == variant)
                    .map(|i| Vec3::from_array(i.pos) - centre);
                if let Some(first) = first {
                    if (back - to_godot(first)).length() > 0.01 {
                        godot_error!(
                            "BfScatter: the MultiMesh buffer layout is wrong: {back} and {first}."
                        );
                    }
                }
            }
            let half = chunk_m * 0.5 + AABB_MARGIN_M;
            let mut node = MultiMeshInstance3D::new_alloc();
            node.set_name(&format!("{}_{cx}_{cz}_{variant}", class.name()));
            node.set_multimesh(&multimesh);
            node.set_position(to_godot(centre));
            node.set_custom_aabb(Aabb::new(
                Vector3::new(-half, y_lo - 2.0, -half),
                Vector3::new(half * 2.0, (y_hi - y_lo) + 4.0, half * 2.0),
            ));
            let shadows = class == Class::Rock && self.rock_shadows;
            node.set_cast_shadows_setting(if shadows {
                ShadowCastingSetting::ON
            } else {
                ShadowCastingSetting::OFF
            });
            // The chunk is one node. Godot measures its distance from the
            // middle [unverified], so the end of the range gets half the
            // diagonal of the chunk.
            let max_m = self.range_m(class);
            let margin = chunk_m * 0.71;
            node.add_to_group(VIEW_RANGED_GROUP);
            node.set_meta(VIEW_MAX_META, &max_m.to_variant());
            node.set_meta(VIEW_MARGIN_META, &margin.to_variant());
            node.set_visibility_range_end(ranged_end(self.view_distance(), max_m, margin));
            self.base_mut().add_child(&node);
            nodes.push(node);
        }

        let rocks = if class == Class::Rock {
            instances
        } else {
            Vec::new()
        };
        self.chunks.insert(
            key,
            Chunk {
                nodes,
                rocks,
                count,
            },
        );
    }

    /// Frees all chunks and all bodies.
    fn clear_all(&mut self) {
        for (_, chunk) in self.chunks.drain() {
            for mut node in chunk.nodes {
                node.queue_free();
            }
        }
        self.pending.clear();
        self.assigned.clear();
        for slot in &mut self.pool {
            slot.key = None;
            slot.shape.set_deferred("disabled", &true.to_variant());
        }
        self.built_params = None;
    }

    /// Moves the collision bodies to the rocks that are nearest to the
    /// bear.
    fn update_bodies(&mut self) {
        let bear = self
            .base()
            .get_tree()
            .get_first_node_in_group(BEAR_GROUP)
            .and_then(|n| n.try_cast::<Node3D>().ok());
        let Some(bear) = bear else {
            return;
        };
        let p = bear.get_global_position();
        let wanted: Vec<RockKey> = if self.enabled && self.zone_ready && self.collision_pool > 0 {
            let range = self.collision_range_m.max(0.0);
            let min_size = self.collision_min_size_m;
            let chunk_m = self
                .built_params
                .as_ref()
                .map(|b| b.chunk_m(Class::Rock))
                .unwrap_or(160.0);
            let mut candidates = Vec::new();
            for ((class, cx, cz), chunk) in &self.chunks {
                if *class != Class::Rock.index()
                    || chunk.rocks.is_empty()
                    || chunk_distance(p.x, p.z, *cx, *cz, chunk_m) > range + AABB_MARGIN_M
                {
                    continue;
                }
                for (i, rock) in chunk.rocks.iter().enumerate() {
                    if rock.scale[0].max(rock.scale[2]) < min_size {
                        continue;
                    }
                    let d = (Vec3::from_array(rock.pos) - Vec3::new(p.x, p.y, p.z)).length();
                    if d <= range {
                        candidates.push((d, (*cx, *cz, i as u32)));
                    }
                }
            }
            nearest_rocks(candidates, self.collision_pool as usize)
        } else {
            Vec::new()
        };
        let wanted_set: HashSet<RockKey> = wanted.iter().copied().collect();

        // Free the bodies of rocks that are not wanted now.
        let gone: Vec<RockKey> = self
            .assigned
            .keys()
            .filter(|k| !wanted_set.contains(k))
            .copied()
            .collect();
        for key in gone {
            if let Some(slot) = self.assigned.remove(&key) {
                self.pool[slot].key = None;
                self.pool[slot]
                    .shape
                    .set_deferred("disabled", &true.to_variant());
            }
        }
        // Give a free body to each new rock.
        for key in wanted {
            if self.assigned.contains_key(&key) {
                continue;
            }
            let Some(rock) = self
                .chunks
                .get(&(Class::Rock.index(), key.0, key.1))
                .and_then(|c| c.rocks.get(key.2 as usize))
                .copied()
            else {
                continue;
            };
            let slot = match self.pool.iter().position(|s| s.key.is_none()) {
                Some(i) => i,
                None => {
                    if self.pool.len() >= self.collision_pool.max(0) as usize {
                        continue;
                    }
                    self.add_slot()
                }
            };
            self.place_body(slot, key, &rock);
        }
    }

    fn add_slot(&mut self) -> usize {
        let mut body = StaticBody3D::new_alloc();
        body.set_name(&format!("RockBody{}", self.pool.len()));
        body.set_meta(GRIP_META, &ROCK_GRIP.to_variant());
        body.set_meta(SOFT_META, &0.0f32.to_variant());
        let mut shape = CollisionShape3D::new_alloc();
        shape.set_disabled(true);
        body.add_child(&shape);
        self.base_mut().add_child(&body);
        self.pool.push(BodySlot {
            body,
            shape,
            key: None,
        });
        self.pool.len() - 1
    }

    /// Puts a body at a rock.
    fn place_body(&mut self, slot: usize, key: RockKey, rock: &Instance) {
        let m = instance_matrix(rock);
        let points: PackedVector3Array = self
            .rock_hulls
            .get(rock.variant as usize)
            .map(|hull| hull.iter().map(|p| to_godot(m * *p)).collect())
            .unwrap_or_default();
        let mut hull = ConvexPolygonShape3D::new_gd();
        hull.set_points(&points);
        let s = &mut self.pool[slot];
        s.body
            .set_global_position(Vector3::new(rock.pos[0], rock.pos[1], rock.pos[2]));
        s.shape.set_shape(&hull);
        s.shape.set_deferred("disabled", &false.to_variant());
        s.key = Some(key);
        self.assigned.insert(key, slot);
    }
}

/// Tells if the game runs without a window (the dummy renderer).
fn is_headless() -> bool {
    DisplayServer::singleton().get_name() == "headless"
}

/// Makes a mesh from the data of a shape.
fn make_mesh(data: &MeshData, material: &Gd<StandardMaterial3D>) -> Gd<ArrayMesh> {
    let positions: PackedVector3Array = data
        .positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let normals: PackedVector3Array = data
        .normals
        .iter()
        .map(|n| Vector3::new(n[0], n[1], n[2]))
        .collect();
    let colours: PackedColorArray = data
        .colours
        .iter()
        .map(|c| Color::from_rgba(c[0], c[1], c[2], c[3]))
        .collect();
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::COLOR.ord() as usize, &colours.to_variant());
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh.surface_set_material(0, &material.clone().upcast::<Material>());
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst() -> Instance {
        Instance {
            pos: [110.0, 5.0, -40.0],
            yaw: 0.0,
            tilt: [0.0, 0.0],
            scale: [2.0, 1.0, 3.0],
            variant: 0,
            tone: 0.0,
            shade: 1.0,
        }
    }

    #[test]
    fn a_prop_is_packed_as_rows_with_the_origin_last() {
        let mut out = Vec::new();
        pack_instance(Class::Rock, &inst(), Vec3::new(100.0, 0.0, -50.0), &mut out);
        assert_eq!(out.len(), 16);
        // No turn: the basis is the scale. The origin is relative to the chunk.
        assert_eq!(&out[0..4], &[2.0, 0.0, 0.0, 10.0]);
        assert_eq!(&out[4..8], &[0.0, 1.0, 0.0, 5.0]);
        assert_eq!(&out[8..12], &[0.0, 0.0, 3.0, 10.0]);
        // The colour: rust rock at full shade.
        assert_eq!(&out[12..16], &[0.50, 0.31, 0.20, 1.0]);
    }

    #[test]
    fn a_turned_prop_keeps_its_size() {
        let turned = Instance {
            yaw: 1.0,
            tilt: [0.1, -0.1],
            ..inst()
        };
        let m = instance_matrix(&turned);
        assert!((m.x_axis.length() - 2.0).abs() < 1e-4);
        assert!((m.y_axis.length() - 1.0).abs() < 1e-4);
        assert!((m.z_axis.length() - 3.0).abs() < 1e-4);
    }

    #[test]
    fn the_colour_follows_the_tone_and_the_shade() {
        let ash = Instance {
            tone: 1.0,
            shade: 0.5,
            ..inst()
        };
        let c = instance_colour(Class::Rock, &ash);
        assert!((c[0] - 0.25).abs() < 1e-5 && (c[1] - 0.21).abs() < 1e-5);
    }

    #[test]
    fn chunk_distance_is_zero_inside() {
        assert_eq!(chunk_distance(10.0, 10.0, 0, 0, 32.0), 0.0);
        assert_eq!(chunk_distance(-5.0, 10.0, 0, 0, 32.0), 5.0);
        let d = chunk_distance(-3.0, -4.0, 0, 0, 32.0);
        assert!((d - 5.0).abs() < 1e-5);
    }

    #[test]
    fn wanted_chunks_are_near_and_sorted() {
        let list = wanted_chunks(10.0, 10.0, 50.0, 32.0);
        assert!(!list.is_empty());
        assert_eq!((list[0].0, list[0].1), (0, 0));
        assert!(list.windows(2).all(|w| w[0].2 <= w[1].2));
        assert!(list.iter().all(|c| c.2 <= 50.0));
        // The count follows the area: a circle of 50 m with chunks of 32 m.
        assert!((9..=25).contains(&list.len()), "{}", list.len());
        assert!(wanted_chunks(0.0, 0.0, 0.0, 32.0).is_empty());
    }

    #[test]
    fn the_nearest_rocks_get_the_bodies() {
        let picks = nearest_rocks(
            vec![
                (5.0, (0, 0, 1)),
                (1.0, (0, 0, 2)),
                (3.0, (1, 0, 0)),
                (9.0, (0, 0, 3)),
            ],
            2,
        );
        assert_eq!(picks, vec![(0, 0, 2), (1, 0, 0)]);
        assert!(nearest_rocks(vec![(1.0, (0, 0, 0))], 0).is_empty());
    }
}
