//! The bridge between `bf-worldgen` and the Terrain3D addon.
//!
//! Terrain3D is a different extension, so godot-rust has no typed
//! bindings for it. This module calls Terrain3D by name
//! (`Object::call`, `Object::set`). All of these calls are in this file.

use bf_worldgen::HeightMap;
use godot::classes::image::Format;
use godot::classes::{ClassDb, Image, Node3D};
use godot::prelude::*;

/// The group that holds the `BfTerrain` node. Other nodes find it there.
pub const TERRAIN_GROUP: &str = "bf_terrain";

/// Terrain3D can hold 32 x 32 regions.
const REGION_MAP_SIZE: i32 = 32;

/// Makes the zone heightmap in Rust and shows it with Terrain3D.
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

    /// The height range of the test hills in metres.
    #[export]
    #[init(val = 120.0)]
    test_relief_m: f32,

    /// The Terrain3D node. It is a child of this node.
    terrain: Option<Gd<Node3D>>,

    /// The heights of the current zone.
    heights: Option<HeightMap>,

    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfTerrain {
    fn ready(&mut self) {
        self.base_mut().add_to_group(TERRAIN_GROUP);
        self.regenerate();
    }
}

#[godot_api]
impl BfTerrain {
    /// Makes the zone again from the current seed and shows it.
    #[func]
    pub fn regenerate(&mut self) {
        let size = self.sample_count();
        let seed = self.seed as u32;
        let start = std::time::Instant::now();
        let map = bf_worldgen::test_hills(seed, size, self.vertex_spacing, self.test_relief_m);
        let gen_ms = start.elapsed().as_secs_f64() * 1000.0;
        let (lo, hi) = map.min_max();

        let start = std::time::Instant::now();
        let shown = self.show(&map);
        let show_ms = start.elapsed().as_secs_f64() * 1000.0;
        if shown {
            godot_print!(
                "BfTerrain: seed {seed}, {size} x {size} samples, heights {lo:.1} to {hi:.1} m, \
                 generation {gen_ms:.0} ms, Terrain3D import {show_ms:.0} ms"
            );
        }
        self.heights = Some(map);
    }

    /// Gives the seed of the current zone.
    #[func]
    pub fn current_seed(&self) -> i64 {
        self.seed
    }

    /// Gives the ground height at a world position in metres.
    /// The value comes from the Rust heightmap, not from Terrain3D.
    #[func]
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.heights.as_ref().map_or(0.0, |m| m.sample_world(x, z))
    }
}

impl BfTerrain {
    /// Gives the number of samples on each side of the zone.
    fn sample_count(&self) -> usize {
        let spacing = self.vertex_spacing.max(0.25);
        ((self.zone_size_m / spacing).round() as usize).clamp(64, 8192)
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
        self.terrain = Some(node.clone());
        Some(node)
    }

    /// Gives the heightmap to Terrain3D. Returns `false` if it fails.
    fn show(&mut self, map: &HeightMap) -> bool {
        let Some(mut terrain) = self.terrain_node() else {
            return false;
        };
        let size = map.size as i32;

        // A region is one square tile of Terrain3D data. Use the smallest
        // region size that can hold the zone.
        let mut region_size = 256;
        while region_size < 2048
            && (size > region_size * REGION_MAP_SIZE || size >= region_size * 4)
        {
            region_size *= 2;
        }
        if size > region_size * REGION_MAP_SIZE {
            godot_error!("BfTerrain: a zone of {size} samples is too large for Terrain3D.");
            return false;
        }
        terrain.set("vertex_spacing", &map.spacing.to_variant());
        terrain.set("region_size", &region_size.to_variant());

        // Draw nothing outside the zone. The terrain has no ground textures
        // yet, thus Terrain3D draws its checker pattern.
        let material = terrain.get("material");
        if let Ok(mut material) = material.try_to::<Gd<Object>>() {
            material.set("world_background", &0.to_variant());
        }

        // Remove the regions of the zone before this one.
        let data = terrain.get("data");
        let Ok(mut data) = data.try_to::<Gd<Object>>() else {
            godot_error!("BfTerrain: Terrain3D has no data object.");
            return false;
        };
        let old_regions = data.call("get_regions_active", &[]);
        if let Ok(regions) = old_regions.try_to::<VarArray>() {
            for region in regions.iter_shared() {
                data.call("remove_region", &[region, false.to_variant()]);
            }
        }

        // One 32-bit float for each height, in metres.
        let mut bytes = PackedByteArray::new();
        bytes.resize(map.data.len() * 4);
        for (dst, h) in bytes.as_mut_slice().chunks_exact_mut(4).zip(&map.data) {
            dst.copy_from_slice(&h.to_le_bytes());
        }
        let Some(image) = Image::create_from_data(size, size, false, Format::RF, &bytes) else {
            godot_error!("BfTerrain: could not make the height image.");
            return false;
        };

        // import_images wants three images: height, control and colour.
        let mut images = VarArray::new();
        images.push(&image.to_variant());
        images.push(&Variant::nil());
        images.push(&Variant::nil());
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
        true
    }
}
