//! `BfWaterfalls` draws the waterfalls of the zone (S-165,
//! `plan/demo-2.md`, "Waterfalls").
//!
//! `bf-worldgen` (`waterfalls.rs`) finds the falls with the rivers.
//! `waterfall_mesh.rs` makes two surfaces for each tile of the zone: the
//! curtain (opaque, alpha scissor) and the effects (foam, splash and
//! mist, blended). `BfWater` makes this node as its child, so
//! `--bf-water=off` hides the falls too. The command line
//! `-- --bf-waterfalls=off` hides only the falls, to measure their cost
//! in a new process.
//!
//! The push on the bear is in `BfTerrain::fall_push_at` and `bear.rs`.

use crate::atmosphere::ATMOSPHERE_GROUP;
use crate::terrain::{BfTerrain, TERRAIN_GROUP};
use crate::waterfall_mesh::{build_falls, FallParams, Surface, TileFalls};
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::image::Format;
use godot::classes::light_3d::Param as LightParam;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{
    ArrayMesh, Camera3D, DirectionalLight3D, INode3D, Image, ImageTexture, Material,
    MeshInstance3D, Os, Shader, ShaderMaterial, Texture2D,
};
use godot::prelude::*;
use std::time::Instant;

/// The group that holds the `BfWaterfalls` node.
pub const WATERFALLS_GROUP: &str = "bf_waterfalls";

const FALL_SHADER: &str = include_str!("waterfall.gdshader");

/// The size of the noise texture in pixels.
const NOISE_SIZE: usize = 64;

/// Draws the waterfalls of the zone.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfWaterfalls {
    /// Shows the waterfalls. The command line `-- --bf-waterfalls=off`
    /// turns them off.
    #[export]
    #[init(val = true)]
    show_falls: bool,

    /// Shows the mist cards. Off is the first cut of the GPU budget.
    #[export]
    #[init(val = true)]
    mist: bool,

    /// Shows the splash particles. Off is the second cut of the GPU
    /// budget.
    #[export]
    #[init(val = true)]
    splash: bool,

    /// The mist fades out beyond this camera distance, in metres.
    #[export]
    #[init(val = 250.0)]
    mist_far_m: f32,

    /// The most opacity of a mist card, 0 to 1.
    #[export]
    #[init(val = 0.3)]
    mist_alpha: f32,

    /// The splash fades out beyond this camera distance, in metres.
    #[export]
    #[init(val = 160.0)]
    splash_far_m: f32,

    /// How much of the foam on the pool shows, 0 to 1.
    #[export]
    #[init(val = 0.85)]
    foam_strength: f32,

    /// The colour of the foam, the streaks and the mist.
    #[export]
    #[init(val = Color::from_rgb(0.92, 0.90, 0.85))]
    foam_colour: Color,

    /// The width of a streak in the curtain, in metres.
    #[export]
    #[init(val = 1.6)]
    streak_width_m: f32,

    /// The length of a streak in the curtain, in metres.
    #[export]
    #[init(val = 7.0)]
    streak_length_m: f32,

    /// The curtain lies this far above the ground, in metres.
    #[export]
    #[init(val = FallParams::default().ground_offset_m)]
    ground_offset_m: f32,

    /// The most mist cards of one fall.
    #[export]
    #[init(val = FallParams::default().mist_max as i32)]
    mist_max: i32,

    /// The most splash particles of one fall.
    #[export]
    #[init(val = FallParams::default().splash_max as i32)]
    splash_max: i32,

    curtain_material: Option<Gd<ShaderMaterial>>,
    effects_material: Option<Gd<ShaderMaterial>>,
    sun: Option<Gd<DirectionalLight3D>>,
    tiles: Vec<(Gd<MeshInstance3D>, TileFalls)>,
    falls: usize,
    build_ms: f32,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfWaterfalls {
    fn ready(&mut self) {
        self.base_mut().add_to_group(WATERFALLS_GROUP);
        for arg in Os::singleton().get_cmdline_user_args().as_slice() {
            match arg.to_string().as_str() {
                "--bf-waterfalls=off" => {
                    self.show_falls = false;
                    godot_print!("BfWaterfalls: the falls are off (--bf-waterfalls=off).");
                }
                "--bf-waterfalls-mist=off" => {
                    self.mist = false;
                    godot_print!("BfWaterfalls: no mist (--bf-waterfalls-mist=off).");
                }
                "--bf-waterfalls-splash=off" => {
                    self.splash = false;
                    godot_print!("BfWaterfalls: no splash (--bf-waterfalls-splash=off).");
                }
                _ => {}
            }
        }
        let tree = self.base().get_tree();
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("on_zone_changed");
            terrain.connect("zone_changed", &on_zone);
        }
    }

    fn process(&mut self, _delta: f64) {
        let shown = self.show_falls;
        if self.base().is_visible() != shown {
            self.base_mut().set_visible(shown);
        }
        if shown && !self.tiles.is_empty() {
            self.apply_look();
        }
    }
}

#[godot_api]
impl BfWaterfalls {
    /// Makes the meshes of the new zone.
    #[func]
    fn on_zone_changed(&mut self) {
        self.rebuild();
    }

    /// Makes the meshes again.
    #[func]
    pub fn rebuild(&mut self) {
        for (mut node, _) in self.tiles.drain(..) {
            node.queue_free();
        }
        let start = Instant::now();
        let params = FallParams {
            ground_offset_m: self.ground_offset_m.max(0.0),
            mist_max: self.mist_max.clamp(3, 16) as usize,
            splash_max: self.splash_max.clamp(0, 256) as usize,
            ..FallParams::default()
        };
        let meshes = {
            let Some(terrain) = self.terrain() else {
                return;
            };
            let terrain = terrain.bind();
            let Some(zone) = terrain.zone() else {
                return;
            };
            build_falls(&zone.water, &zone.heights, &params)
        };
        let (curtain, effects) = self.materials();
        self.falls = meshes.iter().map(|t| t.falls).sum();
        for tile in meshes {
            let mut node = MeshInstance3D::new_alloc();
            node.set_name(&format!("Falls_{}_{}", tile.tile.0, tile.tile.1));
            let mut mesh = ArrayMesh::new_gd();
            add_surface(&mut mesh, &tile.curtain, &curtain);
            add_surface(&mut mesh, &tile.effects, &effects);
            node.set_mesh(&mesh);
            node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
            let (lo, hi) = tile.aabb;
            node.set_custom_aabb(Aabb::new(
                Vector3::new(lo[0], lo[1], lo[2]),
                Vector3::new(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]),
            ));
            self.base_mut().add_child(&node);
            self.tiles.push((node, tile));
        }
        self.build_ms = start.elapsed().as_secs_f32() * 1000.0;
        godot_print!(
            "BfWaterfalls: {} falls, {} meshes, {} triangles, {} splash, {} mist, {:.0} ms",
            self.falls,
            self.tiles.len(),
            self.tiles.iter().map(|(_, t)| t.triangles()).sum::<usize>(),
            self.tiles.iter().map(|(_, t)| t.splash).sum::<usize>(),
            self.tiles.iter().map(|(_, t)| t.mist).sum::<usize>(),
            self.build_ms
        );
    }

    /// Tells if the falls show.
    #[func]
    pub fn is_falls_shown(&self) -> bool {
        self.show_falls
    }

    /// Gives the number of draw calls of the falls that the current
    /// camera can see: 2 for each tile mesh in the view.
    #[func]
    pub fn drawn_meshes(&self) -> i32 {
        if !self.show_falls || !self.base().is_inside_tree() {
            return 0;
        }
        let Some(camera) = self.base().get_viewport().and_then(|v| v.get_camera_3d()) else {
            return 0;
        };
        self.tiles
            .iter()
            .filter(|(_, t)| aabb_in_view(&camera, t.aabb))
            .map(|(_, t)| {
                i32::from(!t.curtain.indices.is_empty()) + i32::from(!t.effects.indices.is_empty())
            })
            .sum()
    }

    /// Gives the overlay line: falls, meshes, triangles and the build time.
    #[func]
    pub fn debug_text(&self) -> GString {
        if !self.show_falls {
            return GString::from("off");
        }
        GString::from(&format!(
            "{} falls, {} meshes ({} draws in view), {} triangles, built in {:.0} ms",
            self.falls,
            self.tiles.len(),
            self.drawn_meshes(),
            self.tiles.iter().map(|(_, t)| t.triangles()).sum::<usize>(),
            self.build_ms
        ))
    }
}

impl BfWaterfalls {
    fn terrain(&self) -> Option<Gd<BfTerrain>> {
        if !self.base().is_inside_tree() {
            return None;
        }
        self.base()
            .get_tree()
            .get_first_node_in_group(TERRAIN_GROUP)?
            .try_cast::<BfTerrain>()
            .ok()
    }

    /// Gives the two materials: the curtain and the effects. They are
    /// made one time.
    fn materials(&mut self) -> (Gd<ShaderMaterial>, Gd<ShaderMaterial>) {
        if let (Some(c), Some(e)) = (&self.curtain_material, &self.effects_material) {
            return (c.clone(), e.clone());
        }
        let noise = noise_texture();
        let make = |define: &str| {
            let mut shader = Shader::new_gd();
            shader.set_code(&FALL_SHADER.replace("/*DEFINES*/", define));
            let mut material = ShaderMaterial::new_gd();
            material.set_shader(&shader);
            if let Some(noise) = &noise {
                material.set_shader_parameter(
                    "noise",
                    &noise.clone().upcast::<Texture2D>().to_variant(),
                );
            }
            material
        };
        let curtain = make("#define CURTAIN\n");
        let effects = make("#define EFFECTS\n");
        self.curtain_material = Some(curtain.clone());
        self.effects_material = Some(effects.clone());
        (curtain, effects)
    }

    /// Gives the light colour of the water: the ambient light and a
    /// part of the sun.
    fn light_colour(&mut self) -> Vector3 {
        if self.sun.is_none() && self.base().is_inside_tree() {
            self.sun = self
                .base()
                .get_tree()
                .get_first_node_in_group(ATMOSPHERE_GROUP)
                .and_then(|a| {
                    a.get_children()
                        .iter_shared()
                        .find_map(|c| c.try_cast::<DirectionalLight3D>().ok())
                });
        }
        let env = self
            .base()
            .get_viewport()
            .and_then(|v| v.get_world_3d())
            .and_then(|w| w.get_environment());
        let ambient = match env {
            Some(env) => {
                let c = env.get_ambient_light_color();
                Vector3::new(c.r, c.g, c.b) * env.get_ambient_light_energy()
            }
            None => Vector3::splat(0.5),
        };
        match &self.sun {
            Some(sun) if sun.is_instance_valid() => {
                let c = sun.get_color();
                let e = sun.get_param(LightParam::ENERGY);
                // A part of the sun: the curtain faces many ways.
                ambient + Vector3::new(c.r, c.g, c.b) * e * 0.4
            }
            _ => ambient + Vector3::splat(0.4),
        }
    }

    /// Gives the exported look values to the materials. A change in the
    /// Remote Inspector shows at once.
    fn apply_look(&mut self) {
        let (mut curtain, mut effects) = self.materials();
        let light = self.light_colour();
        let values: [(&str, Variant); 9] = [
            ("light", light.to_variant()),
            ("foam_colour", self.foam_colour.to_variant()),
            ("foam_strength", self.foam_strength.to_variant()),
            ("mist_far_m", self.mist_far_m.max(1.0).to_variant()),
            ("mist_alpha", self.mist_alpha.to_variant()),
            ("splash_far_m", self.splash_far_m.max(1.0).to_variant()),
            ("mist_on", f32::from(u8::from(self.mist)).to_variant()),
            ("splash_on", f32::from(u8::from(self.splash)).to_variant()),
            (
                "streak_m",
                Vector2::new(self.streak_width_m.max(0.1), self.streak_length_m.max(0.1))
                    .to_variant(),
            ),
        ];
        for (name, value) in values {
            for material in [&mut curtain, &mut effects] {
                // A material ignores a value that its shader does not have.
                if material.get_shader_parameter(name) != value {
                    material.set_shader_parameter(name, &value);
                }
            }
        }
    }
}

/// Tells if a box is in the view of a camera and nearer than its far
/// plane.
fn aabb_in_view(camera: &Gd<Camera3D>, aabb: ([f32; 3], [f32; 3])) -> bool {
    let (lo, hi) = aabb;
    let planes = camera.get_frustum();
    for plane in planes.iter_shared() {
        let n = plane.normal;
        let p = Vector3::new(
            if n.x < 0.0 { hi[0] } else { lo[0] },
            if n.y < 0.0 { hi[1] } else { lo[1] },
            if n.z < 0.0 { hi[2] } else { lo[2] },
        );
        if n.dot(p) - plane.d > 0.0 {
            return false;
        }
    }
    true
}

/// Adds one surface to a mesh, if it has triangles.
fn add_surface(mesh: &mut Gd<ArrayMesh>, s: &Surface, material: &Gd<ShaderMaterial>) {
    if s.indices.is_empty() {
        return;
    }
    let positions: PackedVector3Array = s
        .positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let normals: PackedVector3Array = s.positions.iter().map(|_| Vector3::UP).collect();
    let colours: PackedColorArray = s
        .colours
        .iter()
        .map(|c| Color::from_rgba(c[0], c[1], c[2], c[3]))
        .collect();
    let uvs: PackedVector2Array = s.uvs.iter().map(|u| Vector2::new(u[0], u[1])).collect();
    let uvs2: PackedVector2Array = s.uvs2.iter().map(|u| Vector2::new(u[0], u[1])).collect();
    let indices = PackedInt32Array::from(s.indices.as_slice());
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::COLOR.ord() as usize, &colours.to_variant());
    arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
    arrays.set(ArrayType::TEX_UV2.ord() as usize, &uvs2.to_variant());
    arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
    let surface = mesh.get_surface_count();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh.surface_set_material(surface, &material.clone().upcast::<Material>());
}

/// Makes the noise texture: the repeating noise of `bf-worldgen` in one
/// channel (R8).
fn noise_texture() -> Option<Gd<ImageTexture>> {
    let n = NOISE_SIZE;
    let height = bf_worldgen::smoke::tile_noise(n, 0xFA11);
    let bytes = PackedByteArray::from(height.as_slice());
    let mut image = Image::create_from_data(n as i32, n as i32, false, Format::R8, &bytes)?;
    image.generate_mipmaps();
    ImageTexture::create_from_image(&image)
}
