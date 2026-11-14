//! The bridge between `bf-worldgen` and the Terrain3D addon.
//!
//! Terrain3D is a different extension, so godot-rust has no typed
//! bindings for it. This module calls Terrain3D by name
//! (`Object::call`, `Object::set`). All of these calls are in this file.

use bf_worldgen::highlands::{generate, HighlandsParams};
use bf_worldgen::Zone;
use godot::classes::image::Format;
use godot::classes::{ClassDb, Image, ImageTexture, InputEvent, InputEventKey, Node3D};
use godot::global::Key;
use godot::prelude::*;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread::JoinHandle;
use std::time::Instant;

/// The group that holds the `BfTerrain` node. Other nodes find it there.
pub const TERRAIN_GROUP: &str = "bf_terrain";

/// Terrain3D can hold 32 x 32 regions.
const REGION_MAP_SIZE: i32 = 32;

/// A zone that a worker thread made.
struct Generated {
    zone: Zone,
    colours: Vec<[u8; 3]>,
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

    /// The Terrain3D node. It is a child of this node.
    terrain: Option<Gd<Node3D>>,

    /// The current zone.
    zone: Option<Zone>,

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
        let (sender, receiver) = channel();
        self.join_worker();
        let worker = std::thread::spawn(move || {
            let start = Instant::now();
            let zone = generate(seed, size, spacing, &params);
            let colours = zone.ground_colours();
            let seconds = start.elapsed().as_secs_f32();
            // If the game stopped, nobody receives the zone. That is not an error.
            let _ = sender.send(Generated {
                zone,
                colours,
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

    /// Gives the ground height at a world position in metres.
    /// The value comes from the Rust heightmap, not from Terrain3D.
    #[func]
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.zone
            .as_ref()
            .map_or(0.0, |zone| zone.heights.sample_world(x, z))
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
        Some(node)
    }

    /// Gives Terrain3D one plain ground texture. Without a texture,
    /// Terrain3D draws a checker pattern. The colour map gives the colour.
    fn add_ground_texture(terrain: &Gd<Node3D>) {
        const SIZE: i32 = 64;
        let mut albedo = PackedByteArray::new();
        let mut normal = PackedByteArray::new();
        albedo.resize((SIZE * SIZE * 4) as usize);
        normal.resize((SIZE * SIZE * 4) as usize);
        for (i, px) in albedo.as_mut_slice().chunks_exact_mut(4).enumerate() {
            // Small light and dark grains. Alpha is the height of the texture.
            let grain = bf_worldgen::noise::hash01(i as i32 % SIZE, i as i32 / SIZE, 17);
            let v = (225.0 + grain * 30.0) as u8;
            px.copy_from_slice(&[v, v, v, 128]);
        }
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
            colours,
            seconds,
        } = generated;
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

        // import_images wants three images: height, control and colour.
        let mut images = VarArray::new();
        images.push(&heights.to_variant());
        images.push(&Variant::nil());
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
             generation {seconds:.1} s, Terrain3D import {show_ms:.0} ms",
            zone.seed
        );
        self.zone = Some(zone);
        self.generation_seconds = seconds;
        self.import_ms = show_ms as f32;
        self.signals().zone_changed().emit();
    }
}
