//! The visible teddy bear: meshes from `bear_shape`, the fur material
//! and the wet value of each part.

use crate::bear_shape::{self, PartId, Recipe, Seam};
use godot::classes::image::Format;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{
    ArrayMesh, INode3D, Image, ImageTexture, Material, MeshInstance3D, Node3D, Shader,
    ShaderMaterial,
};
use godot::prelude::*;

/// The fur shader. The vertex colour tells the look of each place:
/// red is fur, green is the pale felt, blue is the dark thread, alpha
/// is the gloss. Object space triplanar mapping keeps the fur texture
/// on the part while the part moves.
const FUR_SHADER: &str = r#"
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx;

uniform vec3 fur_colour : source_color = vec3(0.60, 0.46, 0.30);
uniform vec3 pale_colour : source_color = vec3(0.83, 0.74, 0.58);
uniform vec3 dark_colour : source_color = vec3(0.06, 0.045, 0.035);
uniform vec3 worn_colour : source_color = vec3(0.70, 0.61, 0.47);
uniform sampler2D fur_texture : hint_default_white, filter_linear_mipmap, repeat_enable;
uniform float tiles_per_metre = 26.0;
uniform float fibre_contrast = 0.75;
uniform float worn = 0.5;
uniform float rim = 0.5;
uniform float fuzz = 0.0015;
uniform float wet : hint_range(0.0, 1.0) = 0.0;

varying vec3 obj_pos;
varying vec3 obj_normal;

float hash(vec3 p) {
    return fract(sin(dot(p, vec3(12.9898, 78.233, 37.719))) * 43758.5453);
}

void vertex() {
    obj_pos = VERTEX;
    obj_normal = NORMAL;
    // A little noise on the fur makes the outline fuzzy. Wet fur is flat.
    float fur = COLOR.r * (1.0 - wet);
    VERTEX += NORMAL * fuzz * fur * hash(floor(VERTEX * 400.0));
}

vec4 triplanar(vec3 p, vec3 w) {
    return texture(fur_texture, p.zy) * w.x
        + texture(fur_texture, p.xz) * w.y
        + texture(fur_texture, p.xy) * w.z;
}

void fragment() {
    float fur = COLOR.r;
    vec3 w = pow(abs(normalize(obj_normal)), vec3(4.0));
    w /= (w.x + w.y + w.z);
    vec4 t = triplanar(obj_pos * tiles_per_metre, w);
    // The worn places come from a larger, second sample, so they do
    // not repeat as often as the fibres.
    float worn_mask = triplanar(obj_pos * tiles_per_metre * 0.23 + 0.37, w).b;
    vec3 base = mix(fur_colour, pale_colour, COLOR.g);
    base = mix(base, dark_colour, COLOR.b);

    // Wet fur clumps and lies flat: less contrast between the fibres.
    float contrast = fibre_contrast * (1.0 - 0.6 * wet);
    float fibres = mix(1.0, (0.5 + 0.9 * t.r) * (0.8 + 0.4 * t.g) * (0.7 + 0.3 * t.a), contrast);
    vec3 colour = base * mix(1.0, fibres, fur);
    // Worn places show the paler backing cloth.
    colour = mix(colour, worn_colour * (0.85 + 0.3 * t.r), worn_mask * worn * fur);
    // The felt has a faint texture too.
    colour *= mix(1.0, 0.9 + 0.2 * t.g, COLOR.g * (1.0 - fur));
    // Wet cloth is darker.
    colour *= mix(1.0, 0.55, wet);

    ALBEDO = colour;
    ROUGHNESS = mix(mix(0.95, 0.2, COLOR.a), 0.45, wet);
    SPECULAR = mix(mix(0.12, 0.7, COLOR.a), 0.4, wet);
    RIM = rim * fur * (1.0 - 0.8 * wet);
    RIM_TINT = 0.6;
}
"#;

/// The size of the fur texture in pixels.
const FUR_TEXTURE_SIZE: usize = 128;

/// The visible teddy bear. Rust makes each part from SDF shapes when
/// the node is ready. No model file is used.
///
/// Each part is a `MeshInstance3D` with the name of the part. Its
/// origin is the joint, so a turn of the node turns the part around
/// the joint. The head and the ears are children of the body.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfBearModel {
    /// The colour of the mohair fur.
    #[export]
    #[init(val = Color::from_rgb(0.60, 0.46, 0.30))]
    fur_colour: Color,

    /// The colour of the felt pads on the paws and the feet.
    #[export]
    #[init(val = Color::from_rgb(0.83, 0.74, 0.58))]
    pad_colour: Color,

    /// The colour of the thread of the nose and the mouth.
    #[export]
    #[init(val = Color::from_rgb(0.06, 0.045, 0.035))]
    thread_colour: Color,

    /// The colour of the backing cloth where the fur is worn away.
    #[export]
    #[init(val = Color::from_rgb(0.70, 0.61, 0.47))]
    worn_colour: Color,

    /// How much of the fur is worn away (0 to 1).
    #[export]
    #[init(val = 0.5)]
    worn_amount: f32,

    /// The contrast between light and dark fibres (0 to 1).
    #[export]
    #[init(val = 0.75)]
    fibre_contrast: f32,

    /// How many times the fur texture repeats in one metre.
    #[export]
    #[init(val = 26.0)]
    fur_tiles_per_metre: f32,

    /// The strength of the soft rim light on the edges of the fur (0 to 1).
    #[export]
    #[init(val = 0.5)]
    rim_strength: f32,

    /// How far the fur moves the outline out, in metres. This makes it fuzzy.
    #[export]
    #[init(val = 0.0015)]
    fuzz_m: f32,

    /// The size of the mesh cells on the fur, in metres. Smaller is
    /// smoother and has more triangles. A change needs a restart.
    #[export]
    #[init(val = 0.011)]
    mesh_cell_m: f32,

    parts: Vec<(PartId, Gd<MeshInstance3D>, Gd<ShaderMaterial>)>,
    seams: Vec<(PartId, Option<Seam>)>,
    /// The wet value of each part, in the order of `PartId::ALL`.
    wet: Vec<f32>,
    triangles: usize,
    refresh_left: f64,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfBearModel {
    fn ready(&mut self) {
        self.build();
    }

    fn process(&mut self, delta: f64) {
        // Read the values again twice each second, so a change in the
        // Remote Inspector shows soon.
        self.refresh_left -= delta;
        if self.refresh_left <= 0.0 {
            self.refresh_left = 0.5;
            self.apply_look();
        }
    }
}

#[godot_api]
impl BfBearModel {
    /// Sets the wet value (0 dry, 1 wet) of one part, for example "LeftArm".
    #[func]
    pub fn set_part_wet(&mut self, part: GString, wet: f32) {
        let Some(id) = PartId::from_name(&part.to_string()) else {
            godot_warn!("BfBearModel: no part with the name {part}");
            return;
        };
        if let Some(i) = PartId::ALL.iter().position(|&p| p == id) {
            self.wet[i] = wet.clamp(0.0, 1.0);
        }
        self.apply_look();
    }

    /// Gives the wet value of one part.
    #[func]
    pub fn part_wet(&self, part: GString) -> f32 {
        PartId::from_name(&part.to_string())
            .and_then(|id| PartId::ALL.iter().position(|&p| p == id))
            .and_then(|i| self.wet.get(i).copied())
            .unwrap_or(0.0)
    }

    /// Gives the number of triangles of all parts.
    #[func]
    pub fn triangle_count(&self) -> i64 {
        self.triangles as i64
    }
}

impl BfBearModel {
    /// Gives the node of a part.
    pub fn part(&self, id: PartId) -> Option<Gd<MeshInstance3D>> {
        self.parts
            .iter()
            .find(|(p, _, _)| *p == id)
            .map(|(_, node, _)| node.clone())
    }

    /// Gives the seam ring of a part, in the space of the part.
    #[allow(dead_code)] // S-114 uses the seams for tearing.
    pub fn seam(&self, id: PartId) -> Option<Seam> {
        self.seams.iter().find(|(p, _)| *p == id).and_then(|s| s.1)
    }

    fn build(&mut self) {
        let start = std::time::Instant::now();
        let recipe = Recipe {
            cell: self.mesh_cell_m.max(0.004),
            ..Recipe::default()
        };
        let parts = bear_shape::build(&recipe);

        let mut shader = Shader::new_gd();
        shader.set_code(FUR_SHADER);
        let texture = fur_texture();

        self.wet = vec![0.0; PartId::ALL.len()];
        self.triangles = 0;
        for part in &parts {
            let mut material = ShaderMaterial::new_gd();
            material.set_shader(&shader);
            if let Some(texture) = &texture {
                material.set_shader_parameter("fur_texture", &texture.to_variant());
            }
            let mut node = MeshInstance3D::new_alloc();
            node.set_name(part.id.name());
            node.set_mesh(&array_mesh(&part.mesh));
            node.set_material_override(&material.clone().upcast::<Material>());
            // The node sits at the joint, relative to the joint of the parent.
            let parent_joint = part
                .id
                .parent()
                .and_then(|p| parts.iter().find(|q| q.id == p))
                .map_or(glam::Vec3::ZERO, |q| q.joint);
            let local = part.joint - parent_joint;
            node.set_position(Vector3::new(local.x, local.y, local.z));
            match part.id.parent().and_then(|p| self.part(p)) {
                Some(mut parent) => parent.add_child(&node),
                None => self.base_mut().add_child(&node),
            }
            self.triangles += part.mesh.triangles();
            self.parts.push((part.id, node, material));
            self.seams.push((part.id, part.seam));
        }
        self.apply_look();
        godot_print!(
            "BfBearModel: {} parts, {} triangles, made in {:.1} ms",
            parts.len(),
            self.triangles,
            start.elapsed().as_secs_f64() * 1000.0
        );
    }

    /// Gives the exported values and the wet values to the materials.
    fn apply_look(&mut self) {
        let colour = |c: Color| Vector3::new(c.r, c.g, c.b).to_variant();
        let values = [
            ("fur_colour", colour(self.fur_colour)),
            ("pale_colour", colour(self.pad_colour)),
            ("dark_colour", colour(self.thread_colour)),
            ("worn_colour", colour(self.worn_colour)),
            ("worn", self.worn_amount.to_variant()),
            ("fibre_contrast", self.fibre_contrast.to_variant()),
            ("tiles_per_metre", self.fur_tiles_per_metre.to_variant()),
            ("rim", self.rim_strength.to_variant()),
            ("fuzz", self.fuzz_m.to_variant()),
        ];
        for (id, _, material) in &mut self.parts {
            for (name, value) in &values {
                material.set_shader_parameter(*name, value);
            }
            let i = PartId::ALL.iter().position(|p| p == id).unwrap_or(0);
            let wet = self.wet.get(i).copied().unwrap_or(0.0);
            material.set_shader_parameter("wet", &wet.to_variant());
        }
    }
}

/// Makes the fur texture with mipmaps.
fn fur_texture() -> Option<Gd<ImageTexture>> {
    let bytes = PackedByteArray::from(bear_shape::fur_texture(FUR_TEXTURE_SIZE, 11));
    let size = FUR_TEXTURE_SIZE as i32;
    let mut image = Image::create_from_data(size, size, false, Format::RGBA8, &bytes)?;
    image.generate_mipmaps();
    ImageTexture::create_from_image(&image)
}

/// Changes a part mesh into a Godot mesh with one surface.
fn array_mesh(mesh: &bear_shape::PartMesh) -> Gd<ArrayMesh> {
    let v3 = |p: &[f32; 3]| Vector3::new(p[0], p[1], p[2]);
    let positions: PackedVector3Array = mesh.positions.iter().map(v3).collect();
    let normals: PackedVector3Array = mesh.normals.iter().map(v3).collect();
    let colours: PackedColorArray = mesh
        .looks
        .iter()
        .map(|l| Color::from_rgba(l[0], l[1], l[2], l[3]))
        .collect();
    let indices: PackedInt32Array = mesh.indices.iter().map(|&i| i as i32).collect();

    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::COLOR.ord() as usize, &colours.to_variant());
    arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
    let mut out = ArrayMesh::new_gd();
    out.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    out
}
