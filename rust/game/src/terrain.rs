//! The bridge between `bf-worldgen` and the Terrain3D addon.
//!
//! Terrain3D is a different extension, so godot-rust has no typed
//! bindings for it. This module calls Terrain3D by name
//! (`Object::call`, `Object::set`). All of these calls are in this file.

use crate::cave_mesh::CaveWorld;
use crate::view_distance::terrain_lods;
use bf_worldgen::caves::{CaveParams, Caves};
use bf_worldgen::highlands::{generate, HighlandsParams};
use bf_worldgen::obstacles::{place_obstacles, Obstacle, ObstacleParams, KIND_NAMES};
use bf_worldgen::smoke::SmokeParams;
use bf_worldgen::Zone;
use godot::classes::image::Format;
use godot::classes::{ClassDb, Image, ImageTexture, InputEvent, InputEventKey, Node3D, Shader};
use godot::global::Key;
use godot::prelude::*;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

/// The group that holds the `BfTerrain` node. Other nodes find it there.
pub const TERRAIN_GROUP: &str = "bf_terrain";

/// The most mesh rings (LODs) that Terrain3D makes. This is its default.
const MAX_MESH_LODS: i32 = 7;

/// The light terrain shader (S-139). It replaces the default shader of
/// Terrain3D through `shader_override`.
const LIGHT_SHADER: &str = include_str!("terrain_light.gdshader");

/// Terrain3D can hold 32 x 32 regions.
const REGION_MAP_SIZE: i32 = 32;

/// The hole bit of the Terrain3D control map (bit 3).
const HOLE_BIT: u32 = 1 << 2;

/// The side of the grain texture of the ground, in pixels.
const GRAIN_SIZE: i32 = 64;

/// The RGBA bytes of the grain texture of the ground: small light and
/// dark grains. Alpha is the height of the texture.
fn grain_bytes() -> PackedByteArray {
    let mut albedo = PackedByteArray::new();
    albedo.resize((GRAIN_SIZE * GRAIN_SIZE * 4) as usize);
    for (i, px) in albedo.as_mut_slice().chunks_exact_mut(4).enumerate() {
        let grain = bf_worldgen::noise::hash01(i as i32 % GRAIN_SIZE, i as i32 / GRAIN_SIZE, 17);
        let v = (225.0 + grain * 30.0) as u8;
        px.copy_from_slice(&[v, v, v, 128]);
    }
    albedo
}

/// Gives the grain texture of the ground, with mipmaps. The cave walls
/// use it too (S-164), so the rim at a mouth looks like the ground.
pub fn grain_texture() -> Option<Gd<ImageTexture>> {
    let mut image =
        Image::create_from_data(GRAIN_SIZE, GRAIN_SIZE, false, Format::RGBA8, &grain_bytes())?;
    image.generate_mipmaps();
    ImageTexture::create_from_image(&image)
}

/// The water at one place (S-160).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterAt {
    /// The number of the body.
    pub number: u32,
    /// "lake" or "pond".
    pub kind: &'static str,
    /// The height of the water surface in metres.
    pub level: f32,
    /// The level minus the ground height, in metres.
    pub depth: f32,
    /// From 0 (water) to 1 (thick tar).
    pub viscosity: f32,
    /// 1.0 to 1.6.
    pub density: f32,
}

/// A zone that a worker thread made.
struct Generated {
    zone: Zone,
    obstacles: Vec<Obstacle>,
    colours: Vec<[u8; 3]>,
    /// The caves of the zone (S-164), if they are on.
    caves: Option<Arc<CaveWorld>>,
    /// The part of `seconds` for the caves.
    cave_seconds: f32,
    seconds: f32,
}

/// Makes the zone in Rust (scorched highlands) and shows it with
/// Terrain3D. Push F4 to make a new zone with the next seed.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfTerrain {
    /// The world seed. The same seed always makes the same zone.
    #[export]
    #[init(val = 1)]
    seed: i64,

    /// The length of one side of the zone in metres.
    #[export]
    #[init(val = 2048.0)]
    zone_size_m: f32,

    /// The distance between two terrain vertices in metres.
    #[export]
    #[init(val = 1.0)]
    vertex_spacing: f32,

    /// If this is on, the terrain uses the light shader of Broken
    /// Friends (`terrain_light.gdshader`, S-139). If it is off, it uses
    /// the default shader of Terrain3D, which is about 25 ms slower on
    /// Intel UHD Graphics at 1080p.
    #[export]
    #[init(val = true)]
    light_shader: bool,

    /// How much the grain texture changes the ground colour (0 to 1).
    /// Only the light shader uses this value.
    #[export]
    #[init(val = 1.0)]
    grain_strength: f32,

    /// The size of the most detailed mesh ring of Terrain3D, in vertices
    /// (Terrain3D's `mesh_size`). The rings farther out have the same
    /// size with 2, 4, 8 ... times the spacing. A smaller value draws
    /// fewer triangles. But at 32 and 24 the rings reach past the smog
    /// distance, and the extra far ground made the frame slower on Intel
    /// UHD Graphics (S-139). Thus 48 (the default of Terrain3D) stays.
    #[export]
    #[init(val = 48)]
    mesh_size: i32,

    /// If this is on, the terrain casts sun shadows on itself and on the
    /// bear. This is a cost: the shadow map draws the terrain again.
    #[export]
    #[init(val = true)]
    casts_shadows: bool,

    /// The height of the base terrain in metres, before erosion.
    #[export]
    #[init(val = HighlandsParams::default().relief_m)]
    relief_m: f32,

    /// The distance between two mountain ranges in metres.
    #[export]
    #[init(val = HighlandsParams::default().ridge_wavelength_m)]
    ridge_wavelength_m: f32,

    /// Where the valley floor starts (0.2: wide valleys, 1: no valley floor).
    #[export]
    #[init(val = HighlandsParams::default().valley_at)]
    valley_at: f32,

    /// More than 1 makes wide valleys and thin ridges.
    #[export]
    #[init(val = HighlandsParams::default().ridge_sharpness)]
    ridge_sharpness: f32,

    /// How far the domain warp bends the ranges, in metres.
    #[export]
    #[init(val = HighlandsParams::default().warp_amount_m)]
    warp_amount_m: f32,

    /// The size of the largest gullies, as a part of the zone size.
    #[export]
    #[init(val = HighlandsParams::default().erosion.scale)]
    erosion_scale: f32,

    /// The depth of the gullies, as a part of their size.
    #[export]
    #[init(val = HighlandsParams::default().erosion.strength)]
    erosion_strength: f32,

    /// The number of gully layers. Each layer has half the size.
    #[export]
    #[init(val = HighlandsParams::default().erosion.octaves as i32)]
    erosion_octaves: i32,

    /// The height of the fine roughness of the ground in metres.
    #[export]
    #[init(val = HighlandsParams::default().detail_height_m)]
    detail_height_m: f32,

    /// The part of each deep basin that is a flat dry lake bed (0 to 1).
    #[export]
    #[init(val = HighlandsParams::default().lake_floor_part)]
    lake_floor_part: f32,

    /// The rain area that starts a dry riverbed, in square metres.
    #[export]
    #[init(val = HighlandsParams::default().river_min_area_m2)]
    river_min_area_m2: f32,

    /// The width of the largest riverbed in metres.
    #[export]
    #[init(val = HighlandsParams::default().river_max_width_m)]
    river_max_width_m: f32,

    /// Multiplies the number of obstacles (R23). 0 makes a zone with no obstacles.
    #[export]
    #[init(val = 1.0)]
    obstacle_density: f32,

    /// No obstacle is nearer to the start point than this, in metres.
    #[export]
    #[init(val = ObstacleParams::default().clear_start_m)]
    obstacle_clear_start_m: f32,

    /// At this distance from the start point, the obstacles have their full difficulty, in metres.
    #[export]
    #[init(val = ObstacleParams::default().hard_at_m)]
    obstacle_hard_at_m: f32,

    /// The grip of rust-coloured rock for a bear that climbs (0 to 1).
    #[export]
    #[init(val = 1.0)]
    grip_rust_rock: f32,

    /// The grip of ash-coloured rock (0 to 1). It is slippery.
    #[export]
    #[init(val = 0.45)]
    grip_ash_rock: f32,

    /// The grip of steep soil (0 to 1).
    #[export]
    #[init(val = 0.8)]
    grip_soil: f32,

    /// The grip of the sand in a dry riverbed (0 to 1). It does not hold.
    #[export]
    #[init(val = 0.1)]
    grip_sand: f32,

    /// If this is on, the zone has caves (S-164). `--bf-caves=off` on
    /// the command line turns them off for one process.
    #[export]
    #[init(val = true)]
    caves_enabled: bool,

    /// The band of hole cells around each cave mouth, in samples. The
    /// cave mesh follows the ground in the band.
    #[export]
    #[init(val = 2)]
    hole_band_cells: i32,

    /// The ring of cave mesh under the terrain edge at a mouth, in
    /// samples. It hides the cracks.
    #[export]
    #[init(val = 2)]
    rim_ring_cells: i32,

    /// How far Terrain3D draws, in metres (set by `BfViewDistance`).
    /// Terrain3D draws rings of mesh around the camera. This value
    /// sets the number of rings. Zero means the default of the addon.
    draw_distance_m: f32,

    /// The Terrain3D node. It is a child of this node.
    terrain: Option<Gd<Node3D>>,

    /// The light terrain shader, after the first use.
    light_shader_res: Option<Gd<Shader>>,

    /// The current zone.
    zone: Option<Zone>,

    /// The obstacles of the current zone (R23).
    obstacles: Vec<Obstacle>,

    /// The caves of the current zone (S-164).
    cave_world: Option<Arc<CaveWorld>>,

    /// The time to make the current zone on the worker thread, in seconds.
    generation_seconds: f32,

    /// The time to give the current zone to Terrain3D, in milliseconds.
    import_ms: f32,

    /// The worker thread sends the new zone here.
    pending: Option<Receiver<Generated>>,

    /// The worker thread. The node waits for it when the node leaves the
    /// tree. If Godot unloads the library while the thread still runs,
    /// the game crashes on exit.
    worker: Option<JoinHandle<()>>,

    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfTerrain {
    fn ready(&mut self) {
        self.base_mut().add_to_group(TERRAIN_GROUP);
        for arg in godot::classes::Os::singleton()
            .get_cmdline_user_args()
            .as_slice()
        {
            if *arg == "--bf-caves=off" {
                self.caves_enabled = false;
            }
        }
        self.regenerate();
    }

    fn process(&mut self, _delta: f64) {
        let Some(receiver) = &self.pending else {
            return;
        };
        match receiver.try_recv() {
            Ok(generated) => {
                self.pending = None;
                self.join_worker();
                self.show(generated);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                godot_error!("BfTerrain: the generation thread stopped with an error.");
            }
        }
    }

    fn exit_tree(&mut self) {
        // Quit during generation: wait until the thread stops.
        self.pending = None;
        self.join_worker();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(key) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::F4 {
            self.new_zone();
        }
    }
}

#[godot_api]
impl BfTerrain {
    /// The node sends this signal when a new zone is on the screen.
    #[signal]
    fn zone_changed();

    /// Makes the zone again from the current seed and settings. The
    /// work runs on a different thread. The zone shows when it is ready.
    #[func]
    pub fn regenerate(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let seed = self.seed as u32;
        let size = self.sample_count();
        let spacing = self.vertex_spacing.max(0.25);
        let params = self.params();
        let caves_on = self.caves_enabled;
        let band = self.hole_band_cells.clamp(0, 8) as usize;
        let ring = self.rim_ring_cells.clamp(1, 8) as u8;
        let mut obstacle_params = ObstacleParams {
            density: self.obstacle_density.max(0.0),
            clear_start_m: self.obstacle_clear_start_m.max(0.0),
            hard_at_m: self.obstacle_hard_at_m.max(50.0),
            ..ObstacleParams::default()
        };
        let (sender, receiver) = channel();
        self.join_worker();
        let worker = std::thread::spawn(move || {
            let start = Instant::now();
            let mut zone = generate(seed, size, spacing, &params);
            // The caves read the heights before the obstacles dig their
            // pits (S-164). No obstacle stands at a mouth.
            let cave_start = Instant::now();
            let caves = caves_on
                .then(|| Caves::generate(&zone, &SmokeParams::default(), &CaveParams::default()));
            if let Some(caves) = &caves {
                let pad = (band + ring as usize) as f32 * spacing + 3.0;
                obstacle_params.keep_out = caves
                    .mouths()
                    .map(|m| [m.pos[0], m.pos[2], m.exempt_radius_m + pad])
                    .collect();
            }
            let mut cave_seconds = cave_start.elapsed().as_secs_f32();
            // The obstacles come after the terrain. The ash crusts dig
            // their pits into the heights.
            let obstacles = place_obstacles(&mut zone, &obstacle_params);
            let colours = zone.ground_colours();
            let cave_start = Instant::now();
            let caves = caves.map(|c| {
                let ground = zone.heights.clone();
                Arc::new(CaveWorld::new(c, ground, colours.clone(), band, ring))
            });
            cave_seconds += cave_start.elapsed().as_secs_f32();
            let seconds = start.elapsed().as_secs_f32();
            // If the game stopped, nobody receives the zone. That is not an error.
            let _ = sender.send(Generated {
                zone,
                obstacles,
                colours,
                caves,
                cave_seconds,
                seconds,
            });
        });
        self.pending = Some(receiver);
        self.worker = Some(worker);
    }

    /// Waits for the worker thread, if there is one.
    fn join_worker(&mut self) {
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                godot_error!("BfTerrain: the generation thread panicked.");
            }
        }
    }

    /// Makes a new zone with the next seed (dev key: F4).
    #[func]
    pub fn new_zone(&mut self) {
        if self.pending.is_none() {
            self.seed += 1;
            self.regenerate();
        }
    }

    /// Makes the zone of this seed (the dev menu, S-113). Gives false if
    /// a zone is in work now. Then the caller must try again.
    #[func]
    pub fn make_zone(&mut self, seed: i64) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.seed = seed.max(0);
        self.regenerate();
        true
    }

    /// Sets how far the terrain is drawn, in metres. The terrain makes
    /// fewer rings of mesh when the distance is short. This saves
    /// triangles. Give 0 for the default of the addon.
    #[func]
    pub fn set_draw_distance(&mut self, metres: f32) {
        if (metres - self.draw_distance_m).abs() < 1.0 {
            return;
        }
        self.draw_distance_m = metres.max(0.0);
        if let Some(node) = self.terrain.clone() {
            if node.is_instance_valid() {
                self.apply_draw_distance(node);
            }
        }
    }

    /// Gives the shader choice, the grain and the shadow setting to
    /// Terrain3D. Call it again after a change of these fields.
    #[func]
    pub fn apply_look(&mut self) {
        let Some(mut node) = self.terrain.clone().filter(|t| t.is_instance_valid()) else {
            return;
        };
        let mesh_size = self.mesh_size.clamp(8, 64);
        if node.get("mesh_size").try_to::<i32>().ok() != Some(mesh_size) {
            node.set("mesh_size", &mesh_size.to_variant());
            // The number of rings for the draw distance depends on it.
            self.apply_draw_distance(node.clone());
        }
        // GeometryInstance3D.ShadowCastingSetting: 0 is off, 1 is on.
        let cast = i32::from(self.casts_shadows);
        if node.get("cast_shadows").try_to::<i32>().ok() != Some(cast) {
            node.set("cast_shadows", &cast.to_variant());
        }
        let Ok(mut material) = node.get("material").try_to::<Gd<Object>>() else {
            return;
        };
        if self.light_shader {
            let shader = self.light_shader_resource();
            material.set("shader_override", &shader.to_variant());
            material.call(
                "set_shader_param",
                &[
                    "grain_strength".to_variant(),
                    self.grain_strength.clamp(0.0, 1.0).to_variant(),
                ],
            );
        }
        material.set("shader_override_enabled", &self.light_shader.to_variant());
    }

    /// Tells if the light terrain shader is on (for the recorder).
    #[func]
    pub fn uses_light_shader(&self) -> bool {
        self.light_shader
    }

    /// Gives the number of mesh rings that Terrain3D draws now, or 0 if
    /// the terrain is not ready.
    #[func]
    pub fn mesh_lods(&self) -> i32 {
        match &self.terrain {
            Some(node) if node.is_instance_valid() => node.get("mesh_lods").to::<i32>(),
            _ => 0,
        }
    }

    /// Gives the seed of the current zone.
    #[func]
    pub fn current_seed(&self) -> i64 {
        self.seed
    }

    /// Tells if a new zone is in work.
    #[func]
    pub fn is_generating(&self) -> bool {
        self.pending.is_some()
    }

    /// Gives the height of the water surface at a place, or NaN if the
    /// place is dry (S-160).
    #[func]
    pub fn water_level_at(&self, x: f32, z: f32) -> f32 {
        self.water_at(x, z).map_or(f32::NAN, |w| w.level)
    }

    /// Gives the ground height at a world position in metres.
    /// The value comes from the Rust heightmap, not from Terrain3D.
    #[func]
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.zone
            .as_ref()
            .map_or(0.0, |zone| zone.heights.sample_world(x, z))
    }

    /// Gives the floor for a point, with the caves (S-164): the cave
    /// floor below a point in a cave, the air above a point in rock,
    /// and the ground height where no cave is near. The point is through
    /// the floor by `floor_at - y` when that is positive. Gives minus
    /// infinity if a point in the air has no floor within 600 m.
    #[func]
    pub fn floor_at(&self, x: f32, y: f32, z: f32) -> f32 {
        match (&self.cave_world, &self.zone) {
            (Some(w), _) => w.caves.floor_under(&w.ground, [x, y, z], 600.0),
            (None, Some(zone)) => zone.heights.sample_world(x, z),
            (None, None) => 0.0,
        }
    }

    /// Tells if a point is in the air of a cave, under the ground (S-164).
    #[func]
    pub fn in_cave(&self, x: f32, y: f32, z: f32) -> bool {
        self.cave_world
            .as_ref()
            .is_some_and(|w| w.caves.in_cave(&w.ground, [x, y, z]))
    }

    /// Tells if a cave can be within about 6 m of a point, or if the
    /// point is above a hole of a mouth (S-164). If not, the heightmap
    /// alone is the ground there.
    #[func]
    pub fn near_cave(&self, x: f32, y: f32, z: f32) -> bool {
        self.cave_world
            .as_ref()
            .is_some_and(|w| w.caves.near_cave([x, y, z]) || w.rim.at(x, z) <= w.rim.ring())
    }

    /// Gives the distance along a ray to the first rock or ground, with
    /// the caves (S-164). Gives `max_m` if the ray meets nothing. `dir`
    /// must have length 1.
    #[func]
    pub fn cave_ray(&self, from: Vector3, dir: Vector3, max_m: f32) -> f32 {
        let Some(w) = &self.cave_world else {
            return max_m;
        };
        w.caves
            .ray_hit(
                &w.ground,
                [from.x, from.y, from.z],
                [dir.x, dir.y, dir.z],
                max_m,
            )
            .unwrap_or(max_m)
    }

    /// Gives the name of the main ground type at a world position, for
    /// example "rust rock" or "sand".
    #[func]
    pub fn ground_name_at(&self, x: f32, z: f32) -> GString {
        let name = self
            .zone
            .as_ref()
            .map_or("none", |zone| zone.ground_at(x, z).name());
        GString::from(name)
    }

    /// Gives the start point of the zone (x, z): the flattest place
    /// near the centre.
    #[func]
    pub fn start_point(&self) -> Vector2 {
        let (x, z) = self.zone.as_ref().map_or((0.0, 0.0), Zone::start_point);
        Vector2::new(x, z)
    }

    /// Gives the number of obstacles of each type as text, for example
    /// "ash_crust 50, rolling_stones 36".
    #[func]
    pub fn obstacle_text(&self) -> GString {
        let text = KIND_NAMES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let count = self.obstacles.iter().filter(|o| o.kind.index() == i);
                format!("{name} {}", count.count())
            })
            .collect::<Vec<_>>()
            .join(", ");
        GString::from(&text)
    }

    /// Gives the time to make the current zone, in seconds.
    #[func]
    pub fn generation_seconds(&self) -> f32 {
        self.generation_seconds
    }

    /// Gives the time to give the current zone to Terrain3D, in milliseconds.
    #[func]
    pub fn import_ms(&self) -> f32 {
        self.import_ms
    }

    /// Gives the grip of the ground at a world position, from 0 (does
    /// not hold) to 1 (holds fully). The ground type sets the grip.
    #[func]
    pub fn grip_at(&self, x: f32, z: f32) -> f32 {
        self.zone.as_ref().map_or(1.0, |zone| {
            zone.ground_at(x, z).grip(
                self.grip_rust_rock,
                self.grip_ash_rock,
                self.grip_soil,
                self.grip_sand,
            )
        })
    }
}

impl BfTerrain {
    /// Gives the current zone, if there is one.
    pub fn zone(&self) -> Option<&Zone> {
        self.zone.as_ref()
    }

    /// Gives the lake or pond at a place, with the depth of the water
    /// (S-160). `None` if the place is dry.
    pub fn water_at(&self, x: f32, z: f32) -> Option<WaterAt> {
        let zone = self.zone.as_ref()?;
        let here = zone.water.at(&zone.heights, x, z)?;
        let b = here.body;
        Some(WaterAt {
            number: b.number,
            kind: b.kind.name(),
            level: here.level,
            depth: here.depth,
            viscosity: b.viscosity,
            density: b.density,
        })
    }

    /// Gives the push of a waterfall at a place (S-165), if the place is
    /// in the curtain of a fall. `None` if the zone has no falls.
    pub fn fall_push_at(
        &self,
        x: f32,
        y: f32,
        z: f32,
    ) -> Option<bf_worldgen::waterfalls::FallPush> {
        let zone = self.zone.as_ref()?;
        if zone.water.falls.is_empty() {
            return None;
        }
        zone.water.fall_push_at(x, y, z)
    }

    /// Gives the caves of the current zone (S-164).
    pub fn cave_world(&self) -> Option<&Arc<CaveWorld>> {
        self.cave_world.as_ref()
    }

    /// Gives the obstacles of the current zone.
    pub fn obstacles(&self) -> &[Obstacle] {
        &self.obstacles
    }

    /// Gives the light terrain shader. It is made one time.
    fn light_shader_resource(&mut self) -> Gd<Shader> {
        if let Some(shader) = &self.light_shader_res {
            return shader.clone();
        }
        let mut shader = Shader::new_gd();
        shader.set_code(LIGHT_SHADER);
        self.light_shader_res = Some(shader.clone());
        shader
    }

    /// Gives the number of samples on each side of the zone.
    fn sample_count(&self) -> usize {
        let spacing = self.vertex_spacing.max(0.25);
        ((self.zone_size_m / spacing).round() as usize).clamp(64, 8192)
    }

    /// Gives the generator settings from the exported fields.
    fn params(&self) -> HighlandsParams {
        let mut p = HighlandsParams {
            relief_m: self.relief_m,
            ridge_wavelength_m: self.ridge_wavelength_m.max(10.0),
            valley_at: self.valley_at.clamp(0.05, 1.0),
            ridge_sharpness: self.ridge_sharpness.max(0.1),
            warp_amount_m: self.warp_amount_m,
            detail_height_m: self.detail_height_m,
            lake_floor_part: self.lake_floor_part.clamp(0.0, 1.0),
            river_min_area_m2: self.river_min_area_m2.max(100.0),
            river_max_width_m: self.river_max_width_m.max(1.0),
            ..HighlandsParams::default()
        };
        p.erosion.scale = self.erosion_scale.max(0.005);
        p.erosion.strength = self.erosion_strength;
        p.erosion.octaves = self.erosion_octaves.clamp(0, 8) as u32;
        p
    }

    /// Gives the Terrain3D child node. Makes the node if necessary.
    fn terrain_node(&mut self) -> Option<Gd<Node3D>> {
        if let Some(t) = &self.terrain {
            if t.is_instance_valid() {
                return Some(t.clone());
            }
        }
        if !ClassDb::singleton().class_exists("Terrain3D") {
            godot_error!(
                "BfTerrain: the Terrain3D addon is not installed. \
                 Run tools/install-terrain3d (see game/README.md)."
            );
            return None;
        }
        let node = ClassDb::singleton().instantiate("Terrain3D");
        let Ok(mut node) = node.try_to::<Gd<Node3D>>() else {
            godot_error!("BfTerrain: could not make a Terrain3D node.");
            return None;
        };
        node.set_name("Terrain3D");
        // The zone has no saved assets, thus this option only makes a warning.
        node.set("free_editor_textures", &false.to_variant());
        self.base_mut().add_child(&node);

        // Draw nothing outside the zone.
        if let Ok(mut material) = node.get("material").try_to::<Gd<Object>>() {
            material.set("world_background", &0.to_variant());
        }
        Self::add_ground_texture(&node);
        self.terrain = Some(node.clone());
        self.apply_look();
        self.apply_draw_distance(node.clone());
        Some(node)
    }

    /// Gives the draw distance to Terrain3D as a number of mesh rings.
    fn apply_draw_distance(&self, mut node: Gd<Node3D>) {
        if self.draw_distance_m <= 0.0 {
            return;
        }
        let mesh_size = node.get("mesh_size").try_to::<i32>().unwrap_or(48);
        let spacing = node.get("vertex_spacing").try_to::<f32>().unwrap_or(1.0);
        let lods = terrain_lods(self.draw_distance_m, mesh_size, spacing, MAX_MESH_LODS);
        if node.get("mesh_lods").try_to::<i32>().ok() != Some(lods) {
            node.set("mesh_lods", &lods.to_variant());
        }
    }

    /// Gives Terrain3D one plain ground texture. Without a texture,
    /// Terrain3D draws a checker pattern. The colour map gives the colour.
    fn add_ground_texture(terrain: &Gd<Node3D>) {
        const SIZE: i32 = GRAIN_SIZE;
        let albedo = grain_bytes();
        let mut normal = PackedByteArray::new();
        normal.resize((SIZE * SIZE * 4) as usize);
        for px in normal.as_mut_slice().chunks_exact_mut(4) {
            // A flat normal. Alpha is the roughness: the ground is dry.
            px.copy_from_slice(&[128, 128, 255, 240]);
        }
        let texture = |bytes: &PackedByteArray| {
            let mut image = Image::create_from_data(SIZE, SIZE, false, Format::RGBA8, bytes)?;
            image.generate_mipmaps();
            ImageTexture::create_from_image(&image)
        };
        let (Some(albedo), Some(normal)) = (texture(&albedo), texture(&normal)) else {
            godot_error!("BfTerrain: could not make the ground texture.");
            return;
        };
        let asset = ClassDb::singleton().instantiate("Terrain3DTextureAsset");
        let Ok(mut asset) = asset.try_to::<Gd<Object>>() else {
            godot_error!("BfTerrain: could not make a Terrain3DTextureAsset.");
            return;
        };
        asset.set("name", &"Ground".to_variant());
        asset.set("albedo_texture", &albedo.to_variant());
        asset.set("normal_texture", &normal.to_variant());
        asset.set("uv_scale", &0.25.to_variant());
        if let Ok(mut assets) = terrain.get("assets").try_to::<Gd<Object>>() {
            assets.call("set_texture_asset", &[0.to_variant(), asset.to_variant()]);
        }
    }

    /// Gives the new zone to Terrain3D.
    fn show(&mut self, generated: Generated) {
        let Generated {
            zone,
            obstacles,
            colours,
            caves,
            cave_seconds,
            seconds,
        } = generated;
        self.obstacles = obstacles;
        self.cave_world = caves;
        let start = Instant::now();
        let map = &zone.heights;
        let size = map.size as i32;
        let (lo, hi) = map.min_max();
        let Some(mut terrain) = self.terrain_node() else {
            self.zone = Some(zone);
            return;
        };

        // A region is one square tile of Terrain3D data. Use the smallest
        // region size that can hold the zone in 4 x 4 regions or less.
        let mut region_size = 256;
        while region_size < 2048 && size > region_size * 4 {
            region_size *= 2;
        }
        if size > region_size * REGION_MAP_SIZE {
            godot_error!("BfTerrain: a zone of {size} samples is too large for Terrain3D.");
            return;
        }
        terrain.set("vertex_spacing", &map.spacing.to_variant());
        terrain.set("region_size", &region_size.to_variant());

        let Ok(mut data) = terrain.get("data").try_to::<Gd<Object>>() else {
            godot_error!("BfTerrain: Terrain3D has no data object.");
            return;
        };
        // Remove the regions of the zone before this one.
        if let Ok(regions) = data.call("get_regions_active", &[]).try_to::<VarArray>() {
            for region in regions.iter_shared() {
                data.call("remove_region", &[region, false.to_variant()]);
            }
        }

        // The height image: one 32-bit float for each height, in metres.
        let mut bytes = PackedByteArray::new();
        bytes.resize(map.data.len() * 4);
        for (dst, h) in bytes.as_mut_slice().chunks_exact_mut(4).zip(&map.data) {
            dst.copy_from_slice(&h.to_le_bytes());
        }
        let heights = Image::create_from_data(size, size, false, Format::RF, &bytes);

        // The colour image: red, green, blue. Alpha changes the roughness
        // (128 is no change).
        let mut bytes = PackedByteArray::new();
        bytes.resize(colours.len() * 4);
        for (dst, c) in bytes.as_mut_slice().chunks_exact_mut(4).zip(&colours) {
            dst.copy_from_slice(&[c[0], c[1], c[2], 128]);
        }
        let colour_map = Image::create_from_data(size, size, false, Format::RGBA8, &bytes);
        let (Some(heights), Some(colour_map)) = (heights, colour_map) else {
            godot_error!("BfTerrain: could not make the terrain images.");
            return;
        };

        // The control image: the bits of a 32-bit integer in each float.
        // Only the hole bit is set, in the holes of the cave mouths and
        // their band (S-164).
        let control = self.cave_world.as_ref().and_then(|world| {
            let mut bytes = PackedByteArray::new();
            bytes.resize(world.mask.len() * 4);
            for (dst, &m) in bytes.as_mut_slice().chunks_exact_mut(4).zip(&world.mask) {
                let bits: u32 = if m > 0 { HOLE_BIT } else { 0 };
                dst.copy_from_slice(&bits.to_le_bytes());
            }
            Image::create_from_data(size, size, false, Format::RF, &bytes)
        });

        // import_images wants three images: height, control and colour.
        let mut images = VarArray::new();
        images.push(&heights.to_variant());
        match &control {
            Some(c) => images.push(&c.to_variant()),
            None => images.push(&Variant::nil()),
        }
        images.push(&colour_map.to_variant());
        let half = map.world_size() * 0.5;
        let corner = Vector3::new(-half, 0.0, -half);
        data.call(
            "import_images",
            &[
                images.to_variant(),
                corner.to_variant(),
                0.0.to_variant(),
                1.0.to_variant(),
            ],
        );

        let show_ms = start.elapsed().as_secs_f64() * 1000.0;
        godot_print!(
            "BfTerrain: seed {}, {size} x {size} samples, heights {lo:.0} to {hi:.0} m, \
             generation {seconds:.1} s, Terrain3D import {show_ms:.0} ms, water bodies: {}, \
             {}, cave systems: {} ({cave_seconds:.2} s), obstacles: {}",
            zone.seed,
            zone.water.bodies.len(),
            zone.water.falls_report(&self.params().water),
            self.cave_world
                .as_ref()
                .map_or(0, |w| w.caves.systems.len()),
            self.obstacle_text()
        );
        self.zone = Some(zone);
        self.generation_seconds = seconds;
        self.import_ms = show_ms as f32;
        self.signals().zone_changed().emit();
    }
}
