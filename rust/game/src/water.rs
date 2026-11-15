//! `BfWater` shows the lakes, ponds (S-160) and rivers (S-162) of the
//! zone (`plan/demo-2.md`, "Water look").
//!
//! `bf-worldgen` (`water.rs`) makes the bodies with the zone. This node
//! makes their meshes (`water_mesh.rs`) when a new zone shows. All water
//! uses one material: the look of each body is in its vertices. Each
//! tile of the zone is one mesh, thus one draw call.
//!
//! The bear asks `BfTerrain` for the water at its feet. This node only
//! draws.

use crate::atmosphere::ATMOSPHERE_GROUP;
use crate::terrain::{BfTerrain, TERRAIN_GROUP};
use crate::water_mesh::{build_meshes, MeshParams, TileMesh};
use crate::waterfall::BfWaterfalls;
use bf_worldgen::water::WaterKind;
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

/// The group that holds the `BfWater` node.
pub const WATER_GROUP: &str = "bf_water";

/// The water shader. `/*EXTRA_MODES*/` gets the render modes of the
/// exported switches.
const WATER_SHADER: &str = include_str!("water.gdshader");

/// The Water view test uses the largest lake (and a river rapid) nearer
/// to the centre than this, in metres: inside the smoke wall (at least 590 m). The view
/// looks toward the centre of the zone, away from the wall.
const TEST_LAKE_RADIUS_M: f32 = 580.0;

/// The camera of a fall view is this far from the fall, in metres (the
/// worst view of the GPU budget: 15 m from the largest fall).
const TEST_FALL_DISTANCE_M: f32 = 15.0;

/// The size of the ripple texture in pixels.
const RIPPLE_SIZE: usize = 128;

/// Draws the water of the zone: lakes, ponds and rivers.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfWater {
    /// Shows the water. The command line `-- --bf-water=off` turns it
    /// off, to measure its cost in a new process.
    #[export]
    #[init(val = true)]
    show_water: bool,

    /// The size of one quad of a small lake or pond, in metres.
    #[export]
    #[init(val = MeshParams::default().cell_m)]
    cell_m: f32,

    /// The size of one quad of a large lake, in metres.
    #[export]
    #[init(val = MeshParams::default().large_cell_m)]
    large_cell_m: f32,

    /// A lake with more area than this uses `large_cell_m`, in square metres.
    #[export]
    #[init(val = MeshParams::default().large_area_m2)]
    large_area_m2: f32,

    /// The size of one water tile (one mesh, one draw call), in metres.
    #[export]
    #[init(val = MeshParams::default().tile_m)]
    tile_m: f32,

    /// The size of one quad of a river, in metres.
    #[export]
    #[init(val = MeshParams::default().river_cell_m)]
    river_cell_m: f32,

    /// A river quad whose corners differ by more than this in height is
    /// left out (a waterfall; S-165 draws it), in metres.
    #[export]
    #[init(val = MeshParams::default().max_quad_drop_m)]
    max_quad_drop_m: f32,

    /// A river vertex averages the river levels up to this many samples
    /// away, so the surface has no steps between the river cells.
    #[export]
    #[init(val = MeshParams::default().river_smooth_samples as i32)]
    river_smooth_samples: i32,

    /// The number of ripple layers: 2, 1 (the first cut of the GPU
    /// budget) or 0.
    #[export]
    #[init(val = 2)]
    ripple_layers: i32,

    /// The size of one tile of the ripple texture, in metres.
    #[export]
    #[init(val = 5.0)]
    ripple_scale_m: f32,

    /// How fast the ripples move, in tiles each second.
    #[export]
    #[init(val = 0.03)]
    ripple_speed: f32,

    /// How much the ripples bend the light (0: a mirror-flat surface).
    #[export]
    #[init(val = 0.18)]
    ripple_strength: f32,

    /// The water fades out at the shore above this depth, in metres.
    /// 0 turns the fade off (the second cut of the GPU budget).
    #[export]
    #[init(val = 0.15)]
    shore_fade_m: f32,

    /// The opacity that a flat view adds (Fresnel), 0 to 1.
    #[export]
    #[init(val = 0.45)]
    fresnel_alpha: f32,

    /// The least opacity of thin, clear water, 0 to 1.
    #[export]
    #[init(val = 0.1)]
    min_alpha: f32,

    /// The roughness of the surface: low values give a small, bright sun
    /// glint.
    #[export]
    #[init(val = 0.12)]
    roughness: f32,

    /// The strength of the sun glint, 0 to 1.
    #[export]
    #[init(val = 0.5)]
    specular: f32,

    /// The pale colour that a flat view of the water shows (the water
    /// has no reflections).
    #[export]
    #[init(val = Color::from_rgb(0.82, 0.74, 0.60))]
    sky_colour: Color,

    /// How much of `sky_colour` a flat view shows, 0 to 1.
    #[export]
    #[init(val = 0.35)]
    sky_reflect: f32,

    /// A large quad near the shore is cut into four: when a corner is
    /// dry or less deep than this, in metres.
    #[export]
    #[init(val = MeshParams::default().shore_detail_m)]
    shore_detail_m: f32,

    /// The water gets the sun shadows. Only for `shaded` water. Off
    /// saves the shadow look-ups for each water pixel.
    #[export]
    #[init(val = true)]
    receive_shadows: bool,

    /// The water gets the lights of Godot. Off (the default) makes the
    /// shader do a cheap light itself: a Lambert term of the sun, the
    /// flat ambient light and a sun glint, with no shadows (the last cut
    /// of the GPU budget, S-180).
    #[export]
    #[init(val = false)]
    shaded: bool,

    /// Quads deeper than this many clarities are opaque: no blending,
    /// which saves GPU time. 0 turns it off. The water then hides this
    /// share of the ground: 2 gives 86%, 3 gives 95%.
    #[export]
    #[init(val = MeshParams::default().opaque_clarities)]
    opaque_clarities: f32,

    /// The time of one cycle of the river ripples, in seconds. A long
    /// cycle stretches the ripples more.
    #[export]
    #[init(val = 1.5)]
    flow_cycle_s: f32,

    /// Fast water bends the light more: extra ripple strength for each
    /// metre per second of flow.
    #[export]
    #[init(val = 0.25)]
    flow_ripple_gain: f32,

    /// The colour of the foam on rapids.
    #[export]
    #[init(val = Color::from_rgb(0.86, 0.84, 0.78))]
    foam_colour: Color,

    /// How much the foam on rapids shows, 0 to 1.
    #[export]
    #[init(val = 0.85)]
    foam_strength: f32,

    /// Test only (`--bf-water-fog_off=on`): the water has no fog.
    fog_off_test: bool,
    /// The waterfalls (S-165): a child node.
    falls: Option<Gd<BfWaterfalls>>,
    material: Option<Gd<ShaderMaterial>>,
    /// The sun of the atmosphere, for the glint of unshaded water.
    sun: Option<Gd<DirectionalLight3D>>,
    /// The render modes of the material now.
    /// The render modes of the materials now.
    material_modes: (bool, bool),
    /// The material of the opaque deep quads.
    deep_material: Option<Gd<ShaderMaterial>>,
    tiles: Vec<(Gd<MeshInstance3D>, TileMesh)>,
    /// The time to make the meshes of the zone, in milliseconds.
    build_ms: f32,
    triangles: usize,
    bodies: usize,
    rivers: usize,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfWater {
    fn ready(&mut self) {
        self.base_mut().add_to_group(WATER_GROUP);
        let mut falls = BfWaterfalls::new_alloc();
        falls.set_name("Waterfalls");
        self.base_mut().add_child(&falls);
        self.falls = Some(falls);
        for arg in Os::singleton().get_cmdline_user_args().as_slice() {
            let arg = arg.to_string();
            if arg == "--bf-water=off" {
                self.show_water = false;
                godot_print!("BfWater: the water is off (--bf-water=off).");
            } else if let Some(rest) = arg.strip_prefix("--bf-water-") {
                // `--bf-water-<name>=<value>` sets one look value, so
                // that each cut of the GPU budget is one process.
                if let Some((name, value)) = rest.split_once('=') {
                    self.set_by_name(name, value);
                }
            }
        }
        let tree = self.base().get_tree();
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("on_zone_changed");
            terrain.connect("zone_changed", &on_zone);
        }
    }

    fn process(&mut self, _delta: f64) {
        let shown = self.show_water;
        if self.base().is_visible() != shown {
            self.base_mut().set_visible(shown);
        }
        if shown {
            self.apply_look();
        }
    }
}

#[godot_api]
impl BfWater {
    /// Makes the meshes of the new zone.
    #[func]
    fn on_zone_changed(&mut self) {
        self.rebuild();
    }

    /// Makes the meshes again (after a change of the mesh settings).
    #[func]
    pub fn rebuild(&mut self) {
        for (mut node, _) in self.tiles.drain(..) {
            node.queue_free();
        }
        let start = Instant::now();
        let params = self.mesh_params();
        let meshes = {
            let Some(terrain) = self.terrain() else {
                return;
            };
            let terrain = terrain.bind();
            let Some(zone) = terrain.zone() else {
                return;
            };
            self.bodies = zone.water.bodies.len();
            self.rivers = zone
                .water
                .bodies
                .iter()
                .filter(|b| b.kind == WaterKind::River)
                .count();
            build_meshes(&zone.water, &zone.heights, &params)
        };
        let (material, deep_material) = self.materials();
        self.triangles = meshes.iter().map(TileMesh::triangles).sum();
        for tile in meshes {
            let mut node = MeshInstance3D::new_alloc();
            node.set_name(&format!("Water_{}_{}", tile.tile.0, tile.tile.1));
            let mut mesh = make_mesh(&tile);
            set_surface_materials(&mut mesh, &tile, &material, &deep_material);
            node.set_mesh(&mesh);
            node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
            self.base_mut().add_child(&node);
            self.tiles.push((node, tile));
        }
        self.build_ms = start.elapsed().as_secs_f32() * 1000.0;
        godot_print!(
            "BfWater: {} bodies ({} rivers), {} meshes, {} triangles ({} opaque), {:.0} ms",
            self.bodies,
            self.rivers,
            self.tiles.len(),
            self.triangles,
            self.tiles
                .iter()
                .map(|(_, t)| t.deep_indices.len() / 3)
                .sum::<usize>(),
            self.build_ms
        );
    }

    /// Tells if the water shows.
    #[func]
    pub fn is_water_shown(&self) -> bool {
        self.show_water
    }

    /// Shows or hides the water (the dev menu).
    #[func]
    pub fn set_water_shown(&mut self, on: bool) {
        self.show_water = on;
    }

    /// Gives the number of water meshes that the current camera can see
    /// (in the view and nearer than its far plane). Each one is one draw
    /// call.
    #[func]
    pub fn drawn_meshes(&self) -> i32 {
        if !self.show_water || !self.base().is_inside_tree() {
            return 0;
        }
        let Some(camera) = self.base().get_viewport().and_then(|v| v.get_camera_3d()) else {
            return 0;
        };
        let falls = self.falls.as_ref().map_or(0, |f| f.bind().drawn_meshes());
        self.tiles
            .iter()
            .filter(|(_, t)| aabb_in_view(&camera, t.aabb))
            .count() as i32
            + falls
    }

    /// Gives the overlay line: bodies, meshes, triangles and the build time.
    #[func]
    pub fn debug_text(&self) -> GString {
        if !self.show_water {
            return GString::from("off");
        }
        let falls = self
            .falls
            .as_ref()
            .map_or(GString::new(), |f| f.bind().debug_text());
        GString::from(&format!(
            "{} bodies ({} rivers), {} meshes ({} draws in view), {} triangles, built in {:.0} ms; falls: {falls}",
            self.bodies,
            self.rivers,
            self.tiles.len(),
            self.drawn_meshes(),
            self.triangles,
            self.build_ms
        ))
    }

    /// Gives a fixed camera view for the Water view test: the eye and
    /// the target. `name` is "largest_lake" or "river_rapids". An empty
    /// array means that the zone has no such place.
    #[func]
    pub fn test_view(&self, name: GString) -> PackedVector3Array {
        let mut out = PackedVector3Array::new();
        let Some(terrain) = self.terrain() else {
            return out;
        };
        let terrain = terrain.bind();
        let Some(zone) = terrain.zone() else {
            return out;
        };
        if name == "largest_lake" {
            if let Some(lake) = zone.water.largest_lake_within(TEST_LAKE_RADIUS_M) {
                let v = zone.water.view_across(lake, 4.0);
                out.push(Vector3::new(v.eye[0], v.eye[1], v.eye[2]));
                out.push(Vector3::new(v.target[0], v.target[1], v.target[2]));
            }
        } else if name == "river_rapids" {
            if let Some(v) = zone.water.river_view(TEST_LAKE_RADIUS_M, 3.0) {
                out.push(Vector3::new(v.eye[0], v.eye[1], v.eye[2]));
                out.push(Vector3::new(v.target[0], v.target[1], v.target[2]));
            }
        } else if let Some(tallest) = fall_view_kind(&name) {
            if let Some(v) = zone.water.fall_view(
                &zone.heights,
                TEST_LAKE_RADIUS_M,
                tallest,
                TEST_FALL_DISTANCE_M,
            ) {
                out.push(Vector3::new(v.eye[0], v.eye[1], v.eye[2]));
                out.push(Vector3::new(v.target[0], v.target[1], v.target[2]));
            }
        }
        out
    }

    /// Gives a short text about the place of a test view: the body, its
    /// look and its area.
    #[func]
    pub fn test_view_text(&self, name: GString) -> GString {
        let Some(terrain) = self.terrain() else {
            return GString::new();
        };
        let terrain = terrain.bind();
        let Some(zone) = terrain.zone() else {
            return GString::new();
        };
        if name == "largest_lake" {
            if let Some(lake) = zone.water.largest_lake_within(TEST_LAKE_RADIUS_M) {
                return GString::from(&format!(
                    "lake #{} ({}, {:.0} m2, {:.0} m deep)",
                    lake.number,
                    lake.look.name(),
                    lake.area_m2,
                    lake.max_depth_m
                ));
            }
        } else if name == "river_rapids" {
            if let Some(v) = zone.water.river_view(TEST_LAKE_RADIUS_M, 3.0) {
                let river = &zone.water.bodies[v.body as usize];
                let rapid = if v.drop_m > 0.0 {
                    format!("rapid of {:.1} m", v.drop_m)
                } else {
                    "no rapid".to_string()
                };
                return GString::from(&format!(
                    "river #{} ({}, {:.0} m2, {rapid}, flow {:.1} m/s)",
                    river.number,
                    river.look.name(),
                    river.area_m2,
                    river.flow_mps
                ));
            }
        } else if let Some(tallest) = fall_view_kind(&name) {
            if let Some(v) = zone.water.fall_view(
                &zone.heights,
                TEST_LAKE_RADIUS_M,
                tallest,
                TEST_FALL_DISTANCE_M,
            ) {
                let f = &zone.water.falls[v.fall as usize];
                return GString::from(&format!(
                    "fall #{} ({}, {:.0} m high, {:.0} m wide, {})",
                    f.number,
                    f.feature.name(),
                    f.height_m,
                    f.width_m,
                    if f.thick { "thick" } else { "normal water" }
                ));
            }
        }
        GString::new()
    }
}

/// Tells if a test view name is a fall view: `Some(true)` for the
/// tallest fall, `Some(false)` for the widest fall.
fn fall_view_kind(name: &GString) -> Option<bool> {
    match name.to_string().as_str() {
        "tallest_fall" => Some(true),
        "widest_fall" => Some(false),
        _ => None,
    }
}

impl BfWater {
    /// Sets one exported value from a command-line text.
    fn set_by_name(&mut self, name: &str, value: &str) {
        let on = matches!(value, "on" | "true" | "1");
        match name {
            "ripple_layers" => self.ripple_layers = value.parse().unwrap_or(self.ripple_layers),
            "shore_fade_m" => self.shore_fade_m = value.parse().unwrap_or(self.shore_fade_m),
            "receive_shadows" => self.receive_shadows = on,
            "shaded" => self.shaded = on,
            "opaque_clarities" => {
                self.opaque_clarities = value.parse().unwrap_or(self.opaque_clarities)
            }
            "fog_off" => self.fog_off_test = on,
            "tile_m" => self.tile_m = value.parse().unwrap_or(self.tile_m),
            "cell_m" => self.cell_m = value.parse().unwrap_or(self.cell_m),
            "river_cell_m" => self.river_cell_m = value.parse().unwrap_or(self.river_cell_m),
            "max_quad_drop_m" => {
                self.max_quad_drop_m = value.parse().unwrap_or(self.max_quad_drop_m)
            }
            "large_cell_m" => self.large_cell_m = value.parse().unwrap_or(self.large_cell_m),
            "large_area_m2" => self.large_area_m2 = value.parse().unwrap_or(self.large_area_m2),
            "fresnel_alpha" => self.fresnel_alpha = value.parse().unwrap_or(self.fresnel_alpha),
            "specular" => self.specular = value.parse().unwrap_or(self.specular),
            "flow_cycle_s" => self.flow_cycle_s = value.parse().unwrap_or(self.flow_cycle_s),
            "foam_strength" => self.foam_strength = value.parse().unwrap_or(self.foam_strength),
            _ => godot_print!("BfWater: unknown option --bf-water-{name}"),
        }
        godot_print!("BfWater: --bf-water-{name}={value}");
    }

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

    fn mesh_params(&self) -> MeshParams {
        MeshParams {
            cell_m: self.cell_m.max(0.5),
            large_cell_m: self.large_cell_m.max(0.5),
            large_area_m2: self.large_area_m2,
            tile_m: self.tile_m.max(32.0),
            river_cell_m: self.river_cell_m.max(0.5),
            max_quad_drop_m: self.max_quad_drop_m,
            river_smooth_samples: self.river_smooth_samples.clamp(0, 8) as usize,
            shore_detail_m: self.shore_detail_m,
            opaque_clarities: self.opaque_clarities.max(0.0),
            ..MeshParams::default()
        }
    }

    /// Gives the materials: the blended one and the opaque one for deep
    /// quads. Makes them again if a render mode changed.
    fn materials(&mut self) -> (Gd<ShaderMaterial>, Gd<ShaderMaterial>) {
        let modes = (self.receive_shadows, self.shaded);
        if let (Some(m), Some(d), true) = (
            &self.material,
            &self.deep_material,
            self.material_modes == modes,
        ) {
            return (m.clone(), d.clone());
        }
        let mut extra = String::new();
        if !self.receive_shadows {
            extra.push_str(", shadows_disabled");
        }
        if !self.shaded {
            extra.push_str(", unshaded");
        }
        if self.fog_off_test {
            extra.push_str(", fog_disabled");
        }
        let ripples = ripple_texture();
        let make = |opaque: bool| {
            let code = WATER_SHADER
                .replace("/*EXTRA_MODES*/", &extra)
                .replace(
                    "/*BLEND_MODES*/",
                    if opaque {
                        ""
                    } else {
                        "blend_mix, depth_draw_never, "
                    },
                )
                .replace(
                    "/*DEFINES*/",
                    &format!(
                        "{}{}",
                        if opaque { "#define OPAQUE_WATER\n" } else { "" },
                        if self.shaded {
                            ""
                        } else {
                            "#define FAKE_GLINT\n"
                        }
                    ),
                );
            let mut shader = Shader::new_gd();
            shader.set_code(&code);
            let mut material = ShaderMaterial::new_gd();
            material.set_shader(&shader);
            if let Some(ripples) = &ripples {
                material.set_shader_parameter(
                    "ripples",
                    &ripples.clone().upcast::<Texture2D>().to_variant(),
                );
            }
            material.set_shader_parameter(
                "clarity_scale",
                &MeshParams::default().clarity_scale.to_variant(),
            );
            material
        };
        let mut blend = make(false);
        let mut deep = make(true);
        for (node, tile) in &self.tiles {
            if let Some(mut mesh) = node.get_mesh().and_then(|m| m.try_cast::<ArrayMesh>().ok()) {
                set_surface_materials(&mut mesh, tile, &blend, &deep);
            }
        }
        self.material_modes = modes;
        self.set_uniforms(&mut blend);
        self.set_uniforms(&mut deep);
        self.material = Some(blend.clone());
        self.deep_material = Some(deep.clone());
        (blend, deep)
    }

    /// Gives the direction toward the sun and its light colour for the
    /// glint of unshaded water.
    fn sun_values(&mut self) -> (Vector3, Vector3, Vector3) {
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
        match &self.sun {
            Some(sun) if sun.is_instance_valid() => {
                let c = sun.get_color();
                let e = sun.get_param(LightParam::ENERGY);
                (
                    sun.get_global_transform().basis * Vector3::BACK,
                    Vector3::new(c.r, c.g, c.b) * e,
                    self.ambient(),
                )
            }
            _ => (Vector3::UP, Vector3::ONE, Vector3::splat(0.5)),
        }
    }

    /// Gives the flat ambient light of the environment.
    fn ambient(&self) -> Vector3 {
        let env = self
            .base()
            .get_viewport()
            .and_then(|v| v.get_world_3d())
            .and_then(|w| w.get_environment());
        match env {
            Some(env) => {
                let c = env.get_ambient_light_color();
                Vector3::new(c.r, c.g, c.b) * env.get_ambient_light_energy()
            }
            None => Vector3::splat(0.5),
        }
    }

    fn set_uniforms(&mut self, material: &mut Gd<ShaderMaterial>) {
        let (sun_dir, sun_light, ambient) = if self.shaded {
            (Vector3::UP, Vector3::ONE, Vector3::splat(0.5))
        } else {
            self.sun_values()
        };
        let values: [(&str, Variant); 19] = [
            ("sky_colour", self.sky_colour.to_variant()),
            ("sky_reflect", self.sky_reflect.to_variant()),
            ("ripple_layers", self.ripple_layers.clamp(0, 2).to_variant()),
            ("ripple_scale_m", self.ripple_scale_m.max(0.1).to_variant()),
            ("ripple_speed", self.ripple_speed.to_variant()),
            ("ripple_strength", self.ripple_strength.to_variant()),
            ("shore_fade_m", self.shore_fade_m.max(0.0).to_variant()),
            ("fresnel_alpha", self.fresnel_alpha.to_variant()),
            ("min_alpha", self.min_alpha.to_variant()),
            ("roughness", self.roughness.to_variant()),
            ("specular", self.specular.to_variant()),
            (
                "clarity_scale",
                MeshParams::default().clarity_scale.to_variant(),
            ),
            ("sun_dir", sun_dir.to_variant()),
            ("sun_light", sun_light.to_variant()),
            ("ambient_light", ambient.to_variant()),
            ("flow_cycle_s", self.flow_cycle_s.max(0.1).to_variant()),
            ("flow_ripple_gain", self.flow_ripple_gain.to_variant()),
            ("foam_colour", self.foam_colour.to_variant()),
            ("foam_strength", self.foam_strength.to_variant()),
        ];
        for (name, value) in values {
            if material.get_shader_parameter(name) != value {
                material.set_shader_parameter(name, &value);
            }
        }
    }

    /// Gives the exported look values to the material. A change in the
    /// Remote Inspector shows at once.
    fn apply_look(&mut self) {
        if self.tiles.is_empty() {
            return;
        }
        let (mut blend, mut deep) = self.materials();
        self.set_uniforms(&mut blend);
        self.set_uniforms(&mut deep);
    }
}

/// Tells if a box is in the view of a camera and nearer than its far
/// plane.
fn aabb_in_view(camera: &Gd<Camera3D>, aabb: ([f32; 3], [f32; 3])) -> bool {
    let (lo, hi) = aabb;
    let planes = camera.get_frustum();
    for plane in planes.iter_shared() {
        // The corner of the box that is most on the inner side. The
        // normals of the frustum planes point out of the view.
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

/// Makes the Godot mesh of one tile.
fn make_mesh(t: &TileMesh) -> Gd<ArrayMesh> {
    let positions: PackedVector3Array = t
        .positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let normals: PackedVector3Array = t.positions.iter().map(|_| Vector3::UP).collect();
    let colours: PackedColorArray = t
        .colours
        .iter()
        .map(|c| Color::from_rgba(c[0], c[1], c[2], c[3]))
        .collect();
    let uvs: PackedVector2Array = t.uvs.iter().map(|u| Vector2::new(u[0], u[1])).collect();
    let flows: PackedVector2Array = t.flows.iter().map(|f| Vector2::new(f[0], f[1])).collect();
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::COLOR.ord() as usize, &colours.to_variant());
    arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
    arrays.set(ArrayType::TEX_UV2.ord() as usize, &flows.to_variant());
    let mut mesh = ArrayMesh::new_gd();
    // Surface 0 blends. The opaque deep quads are the next surface.
    for list in [&t.indices, &t.deep_indices] {
        if list.is_empty() {
            continue;
        }
        let indices = PackedInt32Array::from(list.as_slice());
        arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
        mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    }
    mesh
}

/// Gives the materials to the surfaces of a tile mesh. A surface exists
/// only if it has triangles.
fn set_surface_materials(
    mesh: &mut Gd<ArrayMesh>,
    t: &TileMesh,
    blend: &Gd<ShaderMaterial>,
    deep: &Gd<ShaderMaterial>,
) {
    let mut surface = 0;
    if !t.indices.is_empty() {
        mesh.surface_set_material(surface, &blend.clone().upcast::<Material>());
        surface += 1;
    }
    if !t.deep_indices.is_empty() {
        mesh.surface_set_material(surface, &deep.clone().upcast::<Material>());
    }
}

/// Makes the ripple texture: a normal map from the repeating noise of
/// `bf-worldgen`. Red and green are the slope in x and z (0.5 is flat).
fn ripple_texture() -> Option<Gd<ImageTexture>> {
    let n = RIPPLE_SIZE;
    let height = bf_worldgen::smoke::tile_noise(n, 0x5EA);
    let h = |x: usize, z: usize| f32::from(height[(z % n) * n + (x % n)]) / 255.0;
    let mut bytes = Vec::with_capacity(n * n * 3);
    for z in 0..n {
        for x in 0..n {
            let dx = h(x + 1, z) - h(x + n - 1, z);
            let dz = h(x, z + 1) - h(x, z + n - 1);
            let k = 6.0;
            bytes.push(((0.5 - dx * k * 0.5).clamp(0.0, 1.0) * 255.0) as u8);
            bytes.push(((0.5 - dz * k * 0.5).clamp(0.0, 1.0) * 255.0) as u8);
            bytes.push(255);
        }
    }
    let bytes = PackedByteArray::from(bytes.as_slice());
    let mut image = Image::create_from_data(n as i32, n as i32, false, Format::RGB8, &bytes)?;
    image.generate_mipmaps();
    ImageTexture::create_from_image(&image)
}
