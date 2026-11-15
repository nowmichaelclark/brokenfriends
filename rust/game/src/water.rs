//! `BfWater` shows the lakes and ponds of the zone (S-160,
//! `plan/demo-2.md`, "Water look").
//!
//! `bf-worldgen` (`water.rs`) makes the bodies with the zone. This node
//! makes their meshes (`water_mesh.rs`) when a new zone shows. All water
//! uses one material: the look of each body is in its vertices. Each
//! tile of the zone is one mesh, thus one draw call.
//!
//! The bear asks `BfTerrain` for the water at its feet. This node only
//! draws.

use crate::terrain::{BfTerrain, TERRAIN_GROUP};
use crate::water_mesh::{build_meshes, MeshParams, TileMesh};
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::image::Format;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{
    ArrayMesh, Camera3D, INode3D, Image, ImageTexture, Material, MeshInstance3D, Os, Shader,
    ShaderMaterial, Texture2D,
};
use godot::prelude::*;
use std::time::Instant;

/// The group that holds the `BfWater` node.
pub const WATER_GROUP: &str = "bf_water";

/// The water shader. `/*EXTRA_MODES*/` gets the render modes of the
/// exported switches.
const WATER_SHADER: &str = include_str!("water.gdshader");

/// The Water view test uses the largest lake nearer to the centre than
/// this, in metres: inside the smoke wall (at least 590 m). The view
/// looks toward the centre of the zone, away from the wall.
const TEST_LAKE_RADIUS_M: f32 = 580.0;

/// The size of the ripple texture in pixels.
const RIPPLE_SIZE: usize = 128;

/// Draws the still water of the zone.
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

    /// The water gets the sun shadows. Off saves the shadow look-ups
    /// for each water pixel.
    #[export]
    #[init(val = true)]
    receive_shadows: bool,

    /// The water gets light. Off makes unshaded water (the last cut of
    /// the GPU budget).
    #[export]
    #[init(val = true)]
    shaded: bool,

    material: Option<Gd<ShaderMaterial>>,
    /// The render modes of the material now.
    material_modes: (bool, bool),
    tiles: Vec<(Gd<MeshInstance3D>, TileMesh)>,
    /// The time to make the meshes of the zone, in milliseconds.
    build_ms: f32,
    triangles: usize,
    bodies: usize,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfWater {
    fn ready(&mut self) {
        self.base_mut().add_to_group(WATER_GROUP);
        for arg in Os::singleton().get_cmdline_user_args().as_slice() {
            if *arg == "--bf-water=off" {
                self.show_water = false;
                godot_print!("BfWater: the water is off (--bf-water=off).");
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
            build_meshes(&zone.water, &zone.heights, &params)
        };
        let material = self.material();
        self.triangles = meshes.iter().map(TileMesh::triangles).sum();
        for tile in meshes {
            let mut node = MeshInstance3D::new_alloc();
            node.set_name(&format!("Water_{}_{}", tile.tile.0, tile.tile.1));
            node.set_mesh(&make_mesh(&tile));
            node.set_material_override(&material.clone().upcast::<Material>());
            node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
            self.base_mut().add_child(&node);
            self.tiles.push((node, tile));
        }
        self.build_ms = start.elapsed().as_secs_f32() * 1000.0;
        godot_print!(
            "BfWater: {} bodies, {} meshes, {} triangles, {:.0} ms",
            self.bodies,
            self.tiles.len(),
            self.triangles,
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
        self.tiles
            .iter()
            .filter(|(_, t)| aabb_in_view(&camera, t.aabb))
            .count() as i32
    }

    /// Gives the overlay line: bodies, meshes, triangles and the build time.
    #[func]
    pub fn debug_text(&self) -> GString {
        if !self.show_water {
            return GString::from("off");
        }
        GString::from(&format!(
            "{} bodies, {} meshes ({} in view), {} triangles, built in {:.0} ms",
            self.bodies,
            self.tiles.len(),
            self.drawn_meshes(),
            self.triangles,
            self.build_ms
        ))
    }

    /// Gives a fixed camera view for the Water view test: the eye and
    /// the target. `name` is "largest_lake". An empty array means that
    /// the zone has no such place.
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
        }
        GString::new()
    }
}

impl BfWater {
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
            shore_detail_m: self.shore_detail_m,
            ..MeshParams::default()
        }
    }

    /// Gives the material. Makes it again if a render mode changed.
    fn material(&mut self) -> Gd<ShaderMaterial> {
        let modes = (self.receive_shadows, self.shaded);
        if let (Some(m), true) = (&self.material, self.material_modes == modes) {
            return m.clone();
        }
        let mut extra = String::new();
        if !self.receive_shadows {
            extra.push_str(", shadows_disabled");
        }
        if !self.shaded {
            extra.push_str(", unshaded");
        }
        let mut shader = Shader::new_gd();
        shader.set_code(&WATER_SHADER.replace("/*EXTRA_MODES*/", &extra));
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);
        if let Some(ripples) = ripple_texture() {
            material.set_shader_parameter("ripples", &ripples.upcast::<Texture2D>().to_variant());
        }
        material.set_shader_parameter(
            "clarity_scale",
            &MeshParams::default().clarity_scale.to_variant(),
        );
        for (node, _) in &mut self.tiles {
            node.set_material_override(&material.clone().upcast::<Material>());
        }
        self.material = Some(material.clone());
        self.material_modes = modes;
        self.set_uniforms(&mut material);
        material
    }

    fn set_uniforms(&self, material: &mut Gd<ShaderMaterial>) {
        let values: [(&str, Variant); 12] = [
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
        let mut material = self.material();
        self.set_uniforms(&mut material);
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
    let indices = PackedInt32Array::from(t.indices.as_slice());
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::COLOR.ord() as usize, &colours.to_variant());
    arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
    arrays.set(ArrayType::TEX_UV2.ord() as usize, &flows.to_variant());
    arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh
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
