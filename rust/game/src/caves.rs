//! `BfCaves`: the walkable caves (S-164, `plan/demo-2.md`, "Walkable
//! caves").
//!
//! The terrain makes the cave layer and a `CaveWorld` for each zone
//! (`crate::cave_mesh`). This node is the thin part that talks to Godot:
//!
//! - **Chunks.** The zone has a list of the 32 m chunks that can have a
//!   cave wall, and of the rim chunks at the mouths. Worker threads
//!   (`rayon`) make the mesh of a chunk when the camera comes within
//!   `mesh_radius_m`. The rim chunks always have a mesh, so a hole in
//!   the ground never shows the sky.
//! - **Collision.** A chunk within `collision_radius_m` of the bear gets
//!   a `StaticBody3D` with a `ConcavePolygonShape3D`. A chunk gets its
//!   collision before its mesh: the jobs and the uploads for the bear
//!   come first.
//! - **Budget.** The main thread makes at most `meshes_per_frame` meshes
//!   and `collisions_per_frame` collision shapes in each frame.
//! - **Outside.** If the camera is outside and far from all mouths, the
//!   chunks under the ground are hidden (not the rim chunks).
//! - **Events** for the recorder: `cave_enter`, `cave_leave` and
//!   `cave_no_collision` (the bear is near a cave chunk with no
//!   collision).

use crate::atmosphere::{BfAtmosphere, ATMOSPHERE_GROUP};
use crate::bear::{BEAR_GROUP, GRIP_META, SOFT_META};
use crate::cave_mesh::{
    build_chunk, chunk_distance, CaveMeshParams, CaveWorld, ChunkInput, ChunkKey, ChunkMesh,
};
use crate::terrain::{BfTerrain, TERRAIN_GROUP};
use bf_worldgen::caves::{Tour, TOUR_REACH_M};
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::rendering_server::GlobalShaderParameterType;
use godot::classes::{
    ArrayMesh, Camera3D, CollisionShape3D, ConcavePolygonShape3D, INode3D, MeshInstance3D, Node3D,
    Os, RenderingServer, Shader, ShaderMaterial, StaticBody3D, Time,
};
use godot::prelude::*;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

/// The group that holds the `BfCaves` node.
pub const CAVES_GROUP: &str = "bf_caves";

/// The shader of the cave walls.
const CAVE_SHADER: &str = include_str!("cave.gdshader");

/// The name of the global shader uniform with the sky visibility at the
/// bear (0 to 1). The fur shader reads it (`global uniform float`).
pub const CAVE_SKY_GLOBAL: &str = "bf_cave_sky";

/// Makes the global shader uniform `bf_cave_sky` (1: open sky) if it does
/// not exist. Call it before a shader that reads it is compiled.
pub fn register_cave_sky_global() {
    static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    // Godot prints an error if a name is added twice, and a lookup of the
    // name is for the editor only. So the game keeps its own flag.
    if DONE.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    RenderingServer::singleton().global_shader_parameter_add(
        &StringName::from(CAVE_SKY_GLOBAL),
        GlobalShaderParameterType::FLOAT,
        &1.0f32.to_variant(),
    );
}

/// A chunk stays this much longer than its radius, so a chunk at the
/// edge does not go and come all the time, in metres.
const KEEP_EXTRA_M: f32 = 16.0;

/// The mesh radius is never smaller than this, in metres.
const MIN_MESH_RADIUS_M: f32 = 16.0;

/// A finished job of a worker thread.
struct Built {
    generation: u64,
    mesh: ChunkMesh,
    build_ms: f32,
}

/// What the node has for one chunk.
#[derive(Default)]
struct Chunk {
    /// A rim chunk always has a mesh.
    rim: bool,
    /// The geometry, when a worker made it.
    data: Option<Arc<ChunkMesh>>,
    /// A worker makes the geometry now.
    job: bool,
    mesh: Option<Gd<MeshInstance3D>>,
    body: Option<Gd<StaticBody3D>>,
}

impl Chunk {
    /// The chunk has geometry, and it has no triangles.
    fn is_empty(&self) -> bool {
        self.data.as_ref().is_some_and(|d| d.indices.is_empty())
    }
    fn has_mesh(&self) -> bool {
        self.mesh.is_some() || self.is_empty()
    }
    fn has_body(&self) -> bool {
        self.body.is_some() || self.is_empty()
    }
}

/// The numbers for the overlay, the recorder and the Cave tour.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaveStats {
    /// Chunks with a mesh node.
    pub meshes: usize,
    /// Chunks with a mesh node that shows now.
    pub shown: usize,
    /// Chunks with a collision body.
    pub bodies: usize,
    /// Chunks that wait for a worker or for an upload.
    pub queue: usize,
    /// Jobs on the worker threads now.
    pub jobs: usize,
    /// The CPU time of the cave work in the last frame (uploads,
    /// frees and the chunk list), in milliseconds.
    pub upload_ms: f32,
    /// The longest such frame since the zone showed, in milliseconds.
    pub max_upload_ms: f32,
    /// The longest chunk build on a worker, in milliseconds.
    pub max_build_ms: f32,
    /// The triangles of the meshes that show.
    pub triangles: usize,
}

/// The walkable caves: chunk meshes near the camera and collision near
/// the bear.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfCaves {
    /// If this is off, the node shows no caves and makes no collision.
    /// The holes in the ground stay (the terrain makes them).
    #[export]
    #[init(val = true)]
    enabled: bool,

    /// Chunks within this distance of the camera get a mesh, in metres.
    /// The first cut of the GPU budget is 64.
    #[export]
    #[init(val = 96.0)]
    mesh_radius_m: f32,

    /// Chunks within this distance of the bear get collision, in metres.
    #[export]
    #[init(val = 20.0)]
    collision_radius_m: f32,

    /// The most meshes that the main thread makes in one frame.
    #[export]
    #[init(val = 2)]
    meshes_per_frame: i32,

    /// The most collision shapes that the main thread makes in one frame.
    #[export]
    #[init(val = 1)]
    collisions_per_frame: i32,

    /// The most chunk jobs on the worker threads at one time.
    #[export]
    #[init(val = 6)]
    max_jobs: i32,

    /// From outside, the camera sees the chunks under the ground only
    /// this near to a mouth, in metres. Farther away they are hidden.
    #[export]
    #[init(val = 70.0)]
    outside_view_m: f32,

    /// The grip of the cave walls for a bear that climbs (0 to 1).
    #[export]
    #[init(val = 0.85)]
    wall_grip: f32,

    /// The cave colour is multiplied by this value (the shader).
    #[export]
    #[init(val = 1.0)]
    brightness: f32,

    /// How much the grain texture of the ground changes the cave colour
    /// (0 to 1). The terrain uses 1.
    #[export]
    #[init(val = 1.0)]
    grain_strength: f32,

    /// The vertices of the ring under the terrain edge go down this far,
    /// in metres. This hides the cracks at the mouths.
    #[export]
    #[init(val = CaveMeshParams::default().rim_sink_m)]
    rim_sink_m: f32,

    /// A vertex less than this below the ground, with a normal that
    /// points up, is a part of the ground surface, in metres.
    #[export]
    #[init(val = CaveMeshParams::default().ground_eps_m)]
    ground_eps_m: f32,

    /// The colour of the cave rock (sRGB).
    #[export]
    #[init(val = Color::from_rgb(0.44, 0.32, 0.25))]
    rock_colour: Color,

    /// The colour changes from the ground colour to the rock colour in
    /// this depth below the ground, in metres.
    #[export]
    #[init(val = CaveMeshParams::default().colour_depth_m)]
    colour_depth_m: f32,

    /// The `cave_no_collision` event: the bear is this near to a cave
    /// chunk that has no collision, in metres.
    #[export]
    #[init(val = 4.0)]
    no_collision_m: f32,

    /// The light of a cave with no sky, 0 (black) to 1. It is the AO
    /// floor of the cave walls and of the bear.
    #[export]
    #[init(val = 0.35)]
    dark_level: f32,

    /// The colour of the cave fog with no sky (sRGB).
    #[export]
    #[init(val = Color::from_rgb(0.025, 0.028, 0.035))]
    cave_fog_colour: Color,

    /// The density of the cave fog with no sky, for each metre.
    #[export]
    #[init(val = 0.012)]
    cave_fog_density: f32,

    /// The fog colour of the cave walls near the sky. It matches the
    /// smog outside (sRGB).
    #[export]
    #[init(val = Color::from_rgb(0.74, 0.60, 0.42))]
    smog_fog_colour: Color,

    /// The fog density of the cave walls near the sky, for each metre.
    /// It matches the smog outside.
    #[export]
    #[init(val = 0.006)]
    smog_fog_density: f32,

    /// The size of the cells that can hold a glowing mineral, in metres.
    #[export]
    #[init(val = CaveMeshParams::default().mineral_cell_m)]
    mineral_cell_m: f32,

    /// The share of the cells that hold a glowing mineral (0 to 1).
    #[export]
    #[init(val = CaveMeshParams::default().mineral_chance)]
    mineral_chance: f32,

    /// The radius of the glow of one mineral, in metres.
    #[export]
    #[init(val = CaveMeshParams::default().mineral_radius_m)]
    mineral_radius_m: f32,

    /// Minerals grow only where the sky visibility is below this value.
    #[export]
    #[init(val = CaveMeshParams::default().mineral_sky_max)]
    mineral_sky_max: f32,

    /// The light of a mineral. The colour is cyan or amber (sRGB).
    #[export]
    #[init(val = 2.0)]
    mineral_strength: f32,

    /// The first colour of the minerals (sRGB).
    #[export]
    #[init(val = Color::from_rgb(0.15, 0.85, 0.75))]
    mineral_colour_a: Color,

    /// The second colour of the minerals (sRGB).
    #[export]
    #[init(val = Color::from_rgb(0.95, 0.55, 0.15))]
    mineral_colour_b: Color,

    /// The deep-cave cut (S-166). If this is on, the camera is in a cave
    /// with a sky visibility below `cut_sky` and no mouth is in view: the
    /// terrain hides and the sun makes no shadows. `--bf-cave-cut=off`
    /// turns it off for one process.
    #[export]
    #[init(val = true)]
    deep_cut: bool,

    /// The cut starts when the sky visibility at the camera is below
    /// this value.
    #[export]
    #[init(val = 0.02)]
    cut_sky: f32,

    /// A mouth that is nearer than this and in view stops the cut, in
    /// metres.
    #[export]
    #[init(val = 90.0)]
    cut_mouth_view_m: f32,

    /// The sky visibility at the bear that the fur shader gets (smooth).
    sky_at_bear: f32,
    /// The deep-cave cut is on now.
    cut_on: bool,
    world: Option<Arc<CaveWorld>>,
    params: Arc<CaveMeshParams>,
    generation: u64,
    chunks: HashMap<ChunkKey, Chunk>,
    sender: Option<Sender<Built>>,
    receiver: Option<Receiver<Built>>,
    /// Jobs of all zones that did not finish yet.
    jobs: usize,
    material: Option<Gd<ShaderMaterial>>,
    terrain: Option<Gd<BfTerrain>>,
    /// The chunks under the ground show now (not only the rim chunks).
    inside_shown: bool,
    /// The bear was in a cave in the last frame.
    bear_in_cave: bool,
    /// The time of the last `cave_no_collision` event, in seconds.
    last_no_collision_s: f64,
    events: Vec<String>,
    stats: CaveStats,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfCaves {
    fn ready(&mut self) {
        self.base_mut().add_to_group(CAVES_GROUP);
        // `--bf-caves=off` on the command line hides the caves (a test
        // in a new process).
        for arg in Os::singleton().get_cmdline_user_args().as_slice() {
            if *arg == "--bf-caves=off" {
                self.enabled = false;
            } else if *arg == "--bf-cave-cut=off" {
                self.deep_cut = false;
            }
        }
        register_cave_sky_global();
        self.sky_at_bear = 1.0;
        let (sender, receiver) = channel();
        self.sender = Some(sender);
        self.receiver = Some(receiver);
        let tree = self.base().get_tree();
        if let Some(mut node) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("on_zone_changed");
            node.connect("zone_changed", &on_zone);
            self.terrain = node.try_cast::<BfTerrain>().ok();
        }
    }

    fn process(&mut self, _delta: f64) {
        self.receive();
        if (!self.enabled || self.world.is_none()) && (self.cut_on || self.sky_at_bear < 1.0) {
            self.reset_light();
        }
        if !self.enabled || self.world.is_none() {
            let any = self
                .chunks
                .values()
                .any(|c| c.mesh.is_some() || c.body.is_some());
            if any {
                self.free_nodes();
            }
            return;
        }
        let Some(camera) = self.base().get_viewport().and_then(|v| v.get_camera_3d()) else {
            return;
        };
        let cam = camera.get_global_position();
        // The middle of the bear.
        let bear = self
            .base()
            .get_tree()
            .get_first_node_in_group(BEAR_GROUP)
            .and_then(|b| b.try_cast::<Node3D>().ok())
            .map(|b| b.get_global_position() + Vector3::new(0.0, 0.2, 0.0))
            .unwrap_or(cam);
        let start = Instant::now();
        self.update(cam, bear);
        self.update_light(&camera, cam, bear, _delta as f32);
        self.stats.upload_ms = start.elapsed().as_secs_f32() * 1000.0;
        self.stats.max_upload_ms = self.stats.max_upload_ms.max(self.stats.upload_ms);
        self.watch_bear(bear);
    }

    fn exit_tree(&mut self) {
        // The jobs hold only `Arc`s and a sender. They end by themselves.
        self.receiver = None;
        self.sender = None;
    }
}

#[godot_api]
impl BfCaves {
    /// Called when the terrain shows a new zone: all chunks are new.
    #[func]
    fn on_zone_changed(&mut self) {
        self.free_nodes();
        self.chunks.clear();
        self.generation += 1;
        self.stats = CaveStats::default();
        self.inside_shown = true;
        self.bear_in_cave = false;
        self.reset_light();
        self.world = self
            .terrain
            .as_ref()
            .and_then(|t| t.bind().cave_world().cloned());
        let Some(world) = self.world.clone() else {
            return;
        };
        self.params = Arc::new(self.mesh_params());
        for &key in &world.wall_chunks {
            self.chunks.entry(key).or_default();
        }
        for &key in &world.rim_chunks {
            self.chunks.entry(key).or_default().rim = true;
        }
        let mouths: Vec<String> = world
            .caves
            .mouths()
            .map(|m| {
                format!(
                    "{:?} ({:.0}, {:.0}, {:.0}) {:.0} m wide",
                    m.kind,
                    m.pos[0],
                    m.pos[1],
                    m.pos[2],
                    m.radius_m * 2.0
                )
            })
            .collect();
        godot_print!(
            "BfCaves: {} systems, {} wall chunks, {} rim chunks, mouths: {}",
            world.caves.systems.len(),
            world.wall_chunks.len(),
            world.rim_chunks.len(),
            mouths.join(", ")
        );
    }

    /// Turns the caves on or off (the dev menu).
    #[func]
    pub fn set_caves_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    /// Tells if the deep-cave cut is allowed (`--bf-cave-cut=off`).
    #[func]
    pub fn is_deep_cut_enabled(&self) -> bool {
        self.deep_cut
    }

    /// Tells if the caves are on.
    #[func]
    pub fn is_caves_enabled(&self) -> bool {
        self.enabled
    }

    /// Gives the events since the last call (the recorder reads them).
    #[func]
    pub fn take_events(&mut self) -> PackedStringArray {
        let list: Vec<GString> = self.events.drain(..).map(|e| GString::from(&e)).collect();
        PackedStringArray::from(list.as_slice())
    }

    /// Gives the numbers of the caves as one line for the overlay.
    #[func]
    pub fn debug_text(&self) -> GString {
        if !self.enabled {
            return GString::from("off");
        }
        let Some(world) = &self.world else {
            return GString::from("no caves in this zone");
        };
        let s = self.stats;
        GString::from(&format!(
            "{} systems, chunks {} (mesh {}, shown {}, collision {}), queue {}, jobs {}, \
             {}k triangles, frame {:.2} ms (max {:.2}), build max {:.1} ms",
            world.caves.systems.len(),
            self.chunks.len(),
            s.meshes,
            s.shown,
            s.bodies,
            s.queue,
            s.jobs,
            s.triangles / 1000,
            s.upload_ms,
            s.max_upload_ms,
            s.max_build_ms
        ))
    }

    /// Gives the points of the Cave tour path, or an empty array if the
    /// zone has no cave. `tour_parts` gives the part of each point (0
    /// outside, 1 tunnel, 2 chamber, 3 back).
    #[func]
    pub fn tour_points(&self) -> PackedVector3Array {
        self.tour()
            .map(|t| {
                t.points
                    .iter()
                    .map(|p| Vector3::new(p[0], p[1], p[2]))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gives the part of each point of `tour_points`.
    #[func]
    pub fn tour_parts(&self) -> PackedInt32Array {
        self.tour()
            .map(|t| t.parts.iter().map(|&p| p as i32).collect())
            .unwrap_or_default()
    }

    /// Gives the cave data at a place for the recorder: an empty
    /// dictionary if the place is not in a cave.
    #[func]
    pub fn recorder_state(&self, at: Vector3) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(world) = &self.world else {
            return d;
        };
        let p = [at.x, at.y + 0.2, at.z];
        if !world.caves.in_cave(&world.ground, p) {
            return d;
        }
        let s = self.stats;
        d.set("sky", world.caves.sky_visibility(&world.ground, p));
        d.set("meshes", s.meshes as i64);
        d.set("bodies", s.bodies as i64);
        d.set("queue", s.queue as i64);
        d.set("upload_ms", s.upload_ms);
        d
    }
}

impl BfCaves {
    /// Gives the numbers now.
    pub fn stats(&self) -> CaveStats {
        self.stats
    }

    /// Gives the camera path of the Cave tour test, or `None` if the
    /// zone has no cave.
    pub fn tour(&self) -> Option<Tour> {
        let world = self.world.as_ref()?;
        world.caves.tour(&world.ground, TOUR_REACH_M)
    }

    fn mesh_params(&self) -> CaveMeshParams {
        let c = self.rock_colour;
        let lin = |v: f32| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        CaveMeshParams {
            ground_eps_m: self.ground_eps_m.max(0.05),
            rim_sink_m: self.rim_sink_m.max(0.0),
            rock_colour: [lin(c.r), lin(c.g), lin(c.b)],
            colour_depth_m: self.colour_depth_m.max(0.1),
            mineral_cell_m: self.mineral_cell_m.max(0.5),
            mineral_chance: self.mineral_chance.clamp(0.0, 1.0),
            mineral_radius_m: self.mineral_radius_m.max(0.1),
            mineral_sky_max: self.mineral_sky_max.clamp(0.01, 1.0),
            ..CaveMeshParams::default()
        }
    }

    /// Takes the finished jobs from the worker threads.
    fn receive(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        while let Ok(built) = receiver.try_recv() {
            self.jobs = self.jobs.saturating_sub(1);
            if built.generation != self.generation {
                continue;
            }
            self.stats.max_build_ms = self.stats.max_build_ms.max(built.build_ms);
            if let Some(chunk) = self.chunks.get_mut(&built.mesh.key) {
                chunk.job = false;
                chunk.data = Some(Arc::new(built.mesh));
            }
        }
    }

    /// The work of one frame: jobs, uploads, frees and visibility.
    fn update(&mut self, cam: Vector3, bear: Vector3) {
        let Some(world) = self.world.clone() else {
            return;
        };
        let cam_a = [cam.x, cam.y, cam.z];
        let bear_a = [bear.x, bear.y, bear.z];
        let mesh_r = self.mesh_radius_m.max(MIN_MESH_RADIUS_M);
        let body_r = self.collision_radius_m.max(2.0);

        // From outside, far from all mouths, the chunks under the
        // ground do not show.
        let view = self.outside_view_m.max(0.0);
        let near_mouth = world.caves.mouths().any(|m| {
            let (dx, dz) = (m.pos[0] - cam.x, m.pos[2] - cam.z);
            dx * dx + dz * dz <= view * view
        });
        let inside = near_mouth || world.caves.near_cave(cam_a);
        if inside != self.inside_shown {
            self.inside_shown = inside;
            for chunk in self.chunks.values_mut() {
                if let (false, Some(mesh)) = (chunk.rim, &mut chunk.mesh) {
                    mesh.set_visible(inside);
                }
            }
        }

        let mut want_jobs: Vec<(f32, ChunkKey)> = Vec::new();
        let mut want_bodies: Vec<(f32, ChunkKey)> = Vec::new();
        let mut want_meshes: Vec<(f32, ChunkKey)> = Vec::new();
        let mut to_free: Vec<(ChunkKey, bool, bool, bool)> = Vec::new();
        let mut stats = CaveStats {
            max_upload_ms: self.stats.max_upload_ms,
            max_build_ms: self.stats.max_build_ms,
            ..CaveStats::default()
        };
        for (&key, chunk) in &self.chunks {
            let d_cam = chunk_distance(key, cam_a);
            let d_bear = chunk_distance(key, bear_a);
            let want_mesh = chunk.rim || d_cam <= mesh_r;
            let want_body = d_bear <= body_r;
            let keep_mesh = chunk.rim || d_cam <= mesh_r + KEEP_EXTRA_M;
            let keep_body = d_bear <= body_r + KEEP_EXTRA_M * 0.5;
            let free_mesh = !keep_mesh && chunk.mesh.is_some();
            let free_body = !keep_body && chunk.body.is_some();
            let free_data = !keep_mesh && !keep_body && chunk.data.is_some();
            if free_mesh || free_body || free_data {
                to_free.push((key, free_mesh, free_body, free_data));
            }
            let needs = (want_mesh && !chunk.has_mesh()) || (want_body && !chunk.has_body());
            if needs {
                stats.queue += 1;
            }
            if chunk.data.is_none() {
                if needs && !chunk.job {
                    // The bear first, then the camera, then far rims.
                    let p = if want_body {
                        d_bear
                    } else if d_cam <= mesh_r {
                        1000.0 + d_cam
                    } else {
                        5000.0 + d_cam
                    };
                    want_jobs.push((p, key));
                }
            } else {
                if want_body && !chunk.has_body() {
                    want_bodies.push((d_bear, key));
                }
                if want_mesh && !chunk.has_mesh() {
                    let p = if d_cam <= mesh_r {
                        d_cam
                    } else {
                        5000.0 + d_cam
                    };
                    want_meshes.push((p, key));
                }
            }
            if chunk.mesh.is_some() && !free_mesh {
                stats.meshes += 1;
                if chunk.rim || self.inside_shown {
                    stats.shown += 1;
                    stats.triangles += chunk.data.as_ref().map_or(0, |d| d.triangles());
                }
            }
            if chunk.body.is_some() && !free_body {
                stats.bodies += 1;
            }
        }
        for (key, mesh, body, data) in to_free {
            if let Some(chunk) = self.chunks.get_mut(&key) {
                if mesh {
                    if let Some(mut m) = chunk.mesh.take() {
                        m.queue_free();
                    }
                }
                if body {
                    if let Some(mut b) = chunk.body.take() {
                        b.queue_free();
                    }
                }
                if data {
                    chunk.data = None;
                }
            }
        }

        // Jobs for the worker threads.
        let sort = |list: &mut Vec<(f32, ChunkKey)>| {
            list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        };
        sort(&mut want_jobs);
        let free_slots = (self.max_jobs.max(1) as usize).saturating_sub(self.jobs);
        for &(_, key) in want_jobs.iter().take(free_slots) {
            self.spawn_job(&world, key);
        }

        // Uploads: collision first.
        sort(&mut want_bodies);
        let bodies = self.collisions_per_frame.max(0) as usize;
        for &(_, key) in want_bodies.iter().take(bodies) {
            if self.make_body(key) {
                stats.bodies += 1;
            }
        }
        sort(&mut want_meshes);
        let meshes = self.meshes_per_frame.max(0) as usize;
        for &(_, key) in want_meshes.iter().take(meshes) {
            if self.make_mesh(key) {
                stats.meshes += 1;
            }
        }
        stats.jobs = self.jobs;
        self.stats = stats;
    }

    fn spawn_job(&mut self, world: &Arc<CaveWorld>, key: ChunkKey) {
        let Some(sender) = self.sender.clone() else {
            return;
        };
        let Some(chunk) = self.chunks.get_mut(&key) else {
            return;
        };
        chunk.job = true;
        self.jobs += 1;
        let world = world.clone();
        let params = self.params.clone();
        let generation = self.generation;
        rayon::spawn(move || {
            let start = Instant::now();
            let input = ChunkInput {
                caves: &world.caves,
                ground: &world.ground,
                colours: &world.colours,
                rim: &world.rim,
                params: &params,
            };
            let mesh = build_chunk(&input, key);
            let build_ms = start.elapsed().as_secs_f32() * 1000.0;
            // If the game stopped, nobody receives it. That is not an error.
            let _ = sender.send(Built {
                generation,
                mesh,
                build_ms,
            });
        });
    }

    fn material(&mut self) -> Gd<ShaderMaterial> {
        if let Some(m) = &self.material {
            return m.clone();
        }
        let mut shader = Shader::new_gd();
        shader.set_code(CAVE_SHADER);
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);
        material.set_shader_parameter("brightness", &self.brightness.to_variant());
        material.set_shader_parameter("dark_level", &self.dark_level.clamp(0.0, 1.0).to_variant());
        let rgb = |c: Color| Vector3::new(c.r, c.g, c.b).to_variant();
        material.set_shader_parameter("cave_fog_colour", &rgb(self.cave_fog_colour));
        material.set_shader_parameter(
            "cave_fog_density",
            &self.cave_fog_density.max(0.0).to_variant(),
        );
        material.set_shader_parameter("smog_fog_colour", &rgb(self.smog_fog_colour));
        material.set_shader_parameter(
            "smog_fog_density",
            &self.smog_fog_density.max(0.0).to_variant(),
        );
        material.set_shader_parameter("mineral_colour_a", &rgb(self.mineral_colour_a));
        material.set_shader_parameter("mineral_colour_b", &rgb(self.mineral_colour_b));
        material.set_shader_parameter(
            "mineral_strength",
            &self.mineral_strength.max(0.0).to_variant(),
        );
        material.set_shader_parameter(
            "grain_strength",
            &self.grain_strength.clamp(0.0, 1.0).to_variant(),
        );
        if let Some(grain) = crate::terrain::grain_texture() {
            material.set_shader_parameter("grain", &grain.to_variant());
        }
        self.material = Some(material.clone());
        material
    }

    /// Makes the mesh node of a chunk. Gives true if it made one.
    fn make_mesh(&mut self, key: ChunkKey) -> bool {
        let Some(chunk) = self.chunks.get(&key) else {
            return false;
        };
        let (Some(data), rim) = (chunk.data.clone(), chunk.rim) else {
            return false;
        };
        if data.indices.is_empty() {
            return false;
        }
        let material = self.material();
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
        let indices: PackedInt32Array = data.indices.iter().map(|&i| i as i32).collect();
        let mut arrays = VarArray::new();
        arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
        arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
        arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
        arrays.set(ArrayType::COLOR.ord() as usize, &colours.to_variant());
        let uvs: PackedVector2Array = data.uvs.iter().map(|u| Vector2::new(u[0], u[1])).collect();
        arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
        arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
        let mut mesh = ArrayMesh::new_gd();
        mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
        mesh.surface_set_material(0, &material);
        let mut node = MeshInstance3D::new_alloc();
        node.set_name(&format!("Cave_{}_{}_{}", key[0], key[1], key[2]));
        node.set_mesh(&mesh);
        node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
        node.set_visible(rim || self.inside_shown);
        self.base_mut().add_child(&node);
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.mesh = Some(node);
        }
        true
    }

    /// Makes the collision body of a chunk. Gives true if it made one.
    fn make_body(&mut self, key: ChunkKey) -> bool {
        let Some(data) = self.chunks.get(&key).and_then(|c| c.data.clone()) else {
            return false;
        };
        if data.indices.is_empty() {
            return false;
        }
        let faces: PackedVector3Array = data
            .faces()
            .iter()
            .map(|p| Vector3::new(p[0], p[1], p[2]))
            .collect();
        let mut shape = ConcavePolygonShape3D::new_gd();
        shape.set_faces(&faces);
        let mut collision = CollisionShape3D::new_alloc();
        collision.set_shape(&shape);
        let mut body = StaticBody3D::new_alloc();
        body.set_name(&format!("CaveBody_{}_{}_{}", key[0], key[1], key[2]));
        body.set_meta(GRIP_META, &self.wall_grip.clamp(0.0, 1.0).to_variant());
        body.set_meta(SOFT_META, &0.0f32.to_variant());
        body.add_child(&collision);
        self.base_mut().add_child(&body);
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.body = Some(body);
        }
        true
    }

    fn free_nodes(&mut self) {
        for chunk in self.chunks.values_mut() {
            if let Some(mut m) = chunk.mesh.take() {
                m.queue_free();
            }
            if let Some(mut b) = chunk.body.take() {
                b.queue_free();
            }
        }
    }

    /// Gives the light back to the open-air state: the sky uniform is 1,
    /// the terrain shows and the sun makes shadows.
    fn reset_light(&mut self) {
        self.sky_at_bear = 1.0;
        RenderingServer::singleton()
            .global_shader_parameter_set(&StringName::from(CAVE_SKY_GLOBAL), &1.0f32.to_variant());
        if self.cut_on {
            self.set_cut(false);
        }
    }

    /// Turns the deep-cave cut on or off: the terrain node and the sun
    /// shadows.
    fn set_cut(&mut self, on: bool) {
        self.cut_on = on;
        if let Some(terrain) = &self.terrain {
            if let Some(node) = terrain
                .clone()
                .upcast::<Node>()
                .get_node_or_null("Terrain3D")
            {
                if let Ok(mut node) = node.try_cast::<Node3D>() {
                    node.set_visible(!on);
                }
            }
        }
        let tree = self.base().get_tree();
        if let Some(node) = tree.get_first_node_in_group(ATMOSPHERE_GROUP) {
            if let Ok(mut atmosphere) = node.try_cast::<BfAtmosphere>() {
                atmosphere.bind_mut().set_shadows_cut(on);
            }
        }
    }

    /// The cave light of one frame: the sky uniform of the bear and the
    /// deep-cave cut.
    fn update_light(&mut self, camera: &Gd<Camera3D>, cam: Vector3, bear: Vector3, delta: f32) {
        let Some(world) = self.world.clone() else {
            return;
        };
        // The bear. Far from a cave, the sky is open and the work is none.
        let b = [bear.x, bear.y, bear.z];
        let target = if world.caves.near_cave(b) {
            world.caves.sky_visibility(&world.ground, b)
        } else {
            1.0
        };
        // A smooth change, so the fur does not jump at a mouth.
        let k = 1.0 - (-delta * 6.0).exp();
        self.sky_at_bear += (target - self.sky_at_bear) * k;
        RenderingServer::singleton().global_shader_parameter_set(
            &StringName::from(CAVE_SKY_GLOBAL),
            &self.sky_at_bear.to_variant(),
        );
        // The cut. Stop at once if a mouth is in view; start only in the dark.
        let c = [cam.x, cam.y, cam.z];
        let dark = self.deep_cut
            && world.caves.in_cave(&world.ground, c)
            && world.caves.sky_visibility(&world.ground, c)
                < self.cut_sky * if self.cut_on { 2.0 } else { 1.0 };
        let want = dark && !self.mouth_in_view(&world, camera, c);
        if want != self.cut_on {
            self.set_cut(want);
            self.events
                .push(if want { "cave_cut_on" } else { "cave_cut_off" }.to_string());
        }
    }

    /// Tells if a mouth is in the view of the camera and open to it.
    fn mouth_in_view(&self, world: &CaveWorld, camera: &Gd<Camera3D>, c: [f32; 3]) -> bool {
        let r = self.cut_mouth_view_m.max(0.0);
        world.caves.mouths().any(|m| {
            let p = [m.pos[0], m.pos[1] + m.radius_m * 0.5, m.pos[2]];
            let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if len > r || !camera.is_position_in_frustum(Vector3::new(p[0], p[1], p[2])) {
                return false;
            }
            let dir = [
                d[0] / len.max(1e-3),
                d[1] / len.max(1e-3),
                d[2] / len.max(1e-3),
            ];
            // No rock between the camera and the mouth.
            world
                .caves
                .ray_hit(&world.ground, c, dir, (len - 1.0).max(0.0))
                .is_none()
        })
    }

    /// The events of the bear: in a cave or not, and a cave chunk near
    /// the bear that has no collision.
    fn watch_bear(&mut self, bear: Vector3) {
        let Some(world) = self.world.clone() else {
            return;
        };
        let p = [bear.x, bear.y, bear.z];
        let in_cave = world.caves.in_cave(&world.ground, p);
        if in_cave != self.bear_in_cave {
            self.bear_in_cave = in_cave;
            self.events.push(if in_cave {
                let sky = world.caves.sky_visibility(&world.ground, p);
                format!("cave_enter:sky={sky:.2}")
            } else {
                "cave_leave".to_string()
            });
        }
        let near = world.caves.near_cave(p) || world.rim.at(p[0], p[2]) <= world.rim.ring();
        if !near {
            return;
        }
        let now = Time::singleton().get_ticks_msec() as f64 / 1000.0;
        if now - self.last_no_collision_s < 2.0 {
            return;
        }
        let r = self.no_collision_m.max(0.0);
        let missing = self
            .chunks
            .iter()
            .filter(|(k, c)| !c.has_body() && chunk_distance(**k, p) <= r)
            .count();
        if missing > 0 {
            self.last_no_collision_s = now;
            self.events.push(format!("cave_no_collision:{missing}"));
        }
    }
}
