//! The visible teddy bear: meshes from `bear_shape`, the fur material,
//! the wet value of each part, and the pose of the parts (`anim`).

use crate::anim::{AnimInput, AnimParams, BearAnim, Ground, Pose, Rig, LIMBS};
use crate::bear_shape::{self, PartId, Recipe, Seam};
use crate::lost_limb::PartLook;
use crate::ragdoll::RagdollPart;
use crate::tumble::{self, BodyDef};
use crate::verlet::{Thread, ThreadParams, LIMB_JOINT};
use godot::classes::base_material_3d::{Flags, ShadingMode};
use godot::classes::image::Format;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{
    ArrayMesh, INode3D, Image, ImageTexture, ImmediateMesh, Material, MeshInstance3D, Node3D,
    Shader, ShaderMaterial, StandardMaterial3D,
};
use godot::prelude::*;

/// The fur shader. The vertex colour tells the look of each place:
/// red is fur, green is the pale felt, blue is the dark thread, alpha
/// is the gloss. Object space triplanar mapping keeps the fur texture
/// on the part while the part moves.
const FUR_SHADER: &str = r#"
shader_type spatial;
render_mode diffuse_lambert_wrap, specular_schlick_ggx;

uniform vec3 fur_colour : source_color = vec3(0.45, 0.32, 0.19);
uniform vec3 pale_colour : source_color = vec3(0.66, 0.57, 0.42);
uniform vec3 dark_colour : source_color = vec3(0.06, 0.045, 0.035);
uniform vec3 worn_colour : source_color = vec3(0.54, 0.44, 0.32);
uniform sampler2D fur_texture : hint_default_white, filter_linear_mipmap, repeat_enable;
uniform float tiles_per_metre = 26.0;
uniform float fibre_contrast = 0.75;
uniform float worn = 0.35;
uniform float rim = 0.2;
uniform float fuzz = 0.0015;
uniform float wet : hint_range(0.0, 1.0) = 0.0;
// A soft place of the part (the hump) that the animation moves.
uniform vec3 wobble_centre = vec3(0.0);
uniform float wobble_radius = 0.0;
uniform vec3 wobble_offset = vec3(0.0);

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
    if (wobble_radius > 0.0) {
        float near = 1.0 - smoothstep(0.0, wobble_radius, distance(obj_pos, wobble_centre));
        VERTEX += wobble_offset * near;
    }
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
    #[init(val = Color::from_rgb(0.45, 0.32, 0.19))]
    fur_colour: Color,

    /// The colour of the felt pads on the paws and the feet.
    #[export]
    #[init(val = Color::from_rgb(0.66, 0.57, 0.42))]
    pad_colour: Color,

    /// The colour of the thread of the nose and the mouth.
    #[export]
    #[init(val = Color::from_rgb(0.06, 0.045, 0.035))]
    thread_colour: Color,

    /// The colour of the backing cloth where the fur is worn away.
    #[export]
    #[init(val = Color::from_rgb(0.54, 0.44, 0.32))]
    worn_colour: Color,

    /// How much of the fur is worn away (0 to 1).
    #[export]
    #[init(val = 0.35)]
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
    #[init(val = 0.2)]
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

    /// Turns the animation on. Without it, the parts keep their last pose.
    #[export]
    #[init(val = true)]
    animate: bool,

    /// The length of one full stride (two steps) of a slow bear, in metres.
    #[export]
    #[init(val = 0.13)]
    stride_base_m: f32,

    /// The stride gets this much longer for each m/s of speed, in metres.
    #[export]
    #[init(val = 0.165)]
    stride_per_speed: f32,

    /// How far the body moves while one foot stands, in metres. Less is more of a run.
    #[export]
    #[init(val = 0.11)]
    stance_length_m: f32,

    /// The height of a step in metres.
    #[export]
    #[init(val = 0.03)]
    step_height_m: f32,

    /// The time of a step of a bear that stands, in seconds.
    #[export]
    #[init(val = 0.14)]
    idle_step_time: f32,

    /// A bear that stands moves a foot that is this far from its hip, in metres.
    #[export]
    #[init(val = 0.025)]
    idle_step_distance_m: f32,

    /// The distance from the middle of the sole to the toes, in metres.
    #[export]
    #[init(val = 0.035)]
    foot_half_length_m: f32,

    /// How far the body moves up and down with the steps, in metres.
    #[export]
    #[init(val = 0.008)]
    bob_height_m: f32,

    /// How far the body rolls toward the foot that stands, in degrees.
    #[export]
    #[init(val = 4.0)]
    waddle_degrees: f32,

    /// How far the hips turn with the legs, in degrees.
    #[export]
    #[init(val = 3.0)]
    twist_degrees: f32,

    /// The forward lean for each m/s of speed, in degrees.
    #[export]
    #[init(val = 2.0)]
    speed_lean_degrees: f32,

    /// The tilt for each m/s² of acceleration, in degrees.
    #[export]
    #[init(val = 0.35)]
    accel_tilt_degrees: f32,

    /// The forward lean of a bear that stands, in degrees.
    #[export]
    #[init(val = 1.7)]
    idle_slump_degrees: f32,

    /// How far a bear that stands sways, in degrees.
    #[export]
    #[init(val = 0.7)]
    idle_sway_degrees: f32,

    /// The part of the slope that the body leans with (0 to 1).
    #[export]
    #[init(val = 0.4)]
    slope_lean: f32,

    /// The frequency of the body tilt spring in Hz. Higher is stiffer.
    #[export]
    #[init(val = 2.2)]
    tilt_frequency: f32,

    /// The damping of the body tilt spring. Below 1, the body swings.
    #[export]
    #[init(val = 0.45)]
    tilt_damping: f32,

    /// How much the body squashes for each m/s of landing speed.
    #[export]
    #[init(val = 0.9)]
    land_squash: f32,

    /// How much the body stretches at the start of a jump.
    #[export]
    #[init(val = 1.5)]
    jump_stretch: f32,

    /// The frequency of the squash spring in Hz.
    #[export]
    #[init(val = 3.5)]
    squash_frequency: f32,

    /// The damping of the squash spring. Below 1, the body bounces.
    #[export]
    #[init(val = 0.35)]
    squash_damping: f32,

    /// How deep a foot sinks into loose sand, in metres.
    #[export]
    #[init(val = 0.015)]
    sink_depth_m: f32,

    /// The part of the body movement that a foot with no grip slides (0 to 1).
    #[export]
    #[init(val = 0.6)]
    slip_amount: f32,

    /// With this ground grip or more, a foot does not slip (0 to 1).
    #[export]
    #[init(val = 0.7)]
    slip_firm_grip: f32,

    /// How far the arms swing at the walk speed, in degrees.
    #[export]
    #[init(val = 26.0)]
    arm_swing_degrees: f32,

    /// The frequency of the paw springs in Hz. Lower is floppier.
    #[export]
    #[init(val = 3.5)]
    arm_frequency: f32,

    /// The damping of the paw springs.
    #[export]
    #[init(val = 0.5)]
    arm_damping: f32,

    /// How far a paw stays behind for each m/s² of acceleration, in metres.
    #[export]
    #[init(val = 0.0015)]
    arm_inertia: f32,

    /// The part of the body tilt that the head removes, to stay level (0 to 1).
    #[export]
    #[init(val = 0.6)]
    head_level: f32,

    /// How far the head turns into a turn, in radians for each rad/s.
    #[export]
    #[init(val = 0.04)]
    head_turn: f32,

    /// The frequency of the head spring in Hz.
    #[export]
    #[init(val = 3.0)]
    head_frequency: f32,

    /// The damping of the head spring.
    #[export]
    #[init(val = 0.5)]
    head_damping: f32,

    /// How far an ear bends for each m/s² of acceleration, in degrees.
    #[export]
    #[init(val = 0.7)]
    ear_flop_degrees: f32,

    /// The frequency of the ear springs in Hz.
    #[export]
    #[init(val = 5.5)]
    ear_frequency: f32,

    /// The damping of the ear springs. Lower is more wobble.
    #[export]
    #[init(val = 0.25)]
    ear_damping: f32,

    /// How far the hump moves for each m/s² of acceleration, in millimetres.
    #[export]
    #[init(val = 0.4)]
    hump_wobble_mm: f32,

    /// The frequency of the hump spring in Hz.
    #[export]
    #[init(val = 4.5)]
    hump_frequency: f32,

    /// The damping of the hump spring.
    #[export]
    #[init(val = 0.3)]
    hump_damping: f32,

    /// How high the body goes in a hop on one leg, in metres.
    #[export]
    #[init(val = 0.035)]
    hop_height_m: f32,

    /// The part of a hop in which the foot stands (0.2 to 0.8).
    #[export]
    #[init(val = 0.45)]
    hop_duty: f32,

    /// How far the body leans over the leg in a hop, in degrees.
    #[export]
    #[init(val = 6.0)]
    hop_lean_degrees: f32,

    /// How far the arms go out for balance in a hop, in degrees.
    #[export]
    #[init(val = 34.0)]
    hop_arms_out_degrees: f32,

    /// How far a bear without legs tips forward onto its belly, in degrees.
    #[export]
    #[init(val = 77.0)]
    lie_angle_degrees: f32,

    /// The height of the hips above the ground when the bear lies, in metres.
    #[export]
    #[init(val = 0.065)]
    lie_height_m: f32,

    /// The part of the belly tilt that the head removes, to look ahead (0 to 1).
    #[export]
    #[init(val = 0.75)]
    lie_head: f32,

    /// In a crawl, a paw lands at this part of the arm length from the shoulder.
    #[export]
    #[init(val = 0.9)]
    crawl_reach: f32,

    /// How far the body rolls along its length in a crawl or a drag, in degrees.
    #[export]
    #[init(val = 9.0)]
    drag_roll_degrees: f32,

    /// How far the body turns from side to side in a wriggle, in degrees.
    #[export]
    #[init(val = 14.0)]
    wriggle_yaw_degrees: f32,

    /// How far the body rolls along its length in a wriggle, in degrees.
    #[export]
    #[init(val = 11.0)]
    wriggle_roll_degrees: f32,

    /// On a wall, a hand reaches up this part of the arm length (0 to 1).
    #[export]
    #[init(val = 0.7)]
    hand_reach: f32,

    /// On a wall, a hand or a foot steps when it is this far from its place, in metres.
    #[export]
    #[init(val = 0.055)]
    hold_step_m: f32,

    /// The time of a step of a hand on a wall, in seconds.
    #[export]
    #[init(val = 0.16)]
    hand_step_time: f32,

    /// The time of a step of a foot on a wall, in seconds.
    #[export]
    #[init(val = 0.14)]
    foot_step_time: f32,

    /// The distance between the rays of the fan that finds a hold, in metres.
    #[export]
    #[init(val = 0.035)]
    fan_spread_m: f32,

    /// How far a hand or a foot comes off the wall in a step, in metres.
    #[export]
    #[init(val = 0.03)]
    hold_lift_m: f32,

    /// The part of the hand movement that the body follows on a wall (0 to 1).
    #[export]
    #[init(val = 0.4)]
    hang_follow: f32,

    /// How far the body hangs below the hands, and more while a hand steps, in metres.
    #[export]
    #[init(val = 0.01)]
    hang_sag_m: f32,

    /// The roll of the body on a wall for each metre between the hand heights, in radians.
    #[export]
    #[init(val = 3.0)]
    hang_roll: f32,

    /// How far a torn limb droops toward the ground, in degrees (at a seam that is almost gone).
    #[export]
    #[init(val = 40.0)]
    droop_degrees: f32,

    /// The length of the thread of a dangling limb, in metres.
    #[export]
    #[init(val = 0.035)]
    thread_length_m: f32,

    /// The part of its speed that a dangling limb keeps in each step (0 to 1).
    #[export]
    #[init(val = 0.97)]
    thread_keep: f32,

    /// The time from the pose of the ragdoll back to the animation when the bear gets up, in seconds.
    #[export]
    #[init(val = 0.35)]
    get_up_time: f32,

    parts: Vec<(PartId, Gd<MeshInstance3D>, Gd<ShaderMaterial>)>,
    /// The bodies of the ragdoll, from the meshes (`tumble.rs`).
    bodies: Vec<BodyDef>,
    /// The look of each limb, in the order of `LIMBS`.
    limb_looks: [LimbLook; 4],
    /// The thread of each dangling limb.
    threads: [Option<Thread>; 4],
    /// The lines of the threads. Rust makes it when a limb dangles.
    thread_mesh: Option<Gd<ImmediateMesh>>,
    /// The move from the ragdoll back to the animation.
    blend: Option<Blend>,
    /// The fixed points that the animation needs.
    rig: Option<Rig>,
    anim: BearAnim,
    /// The last hump offset that the body material got.
    hump: Vector3,
    /// The smoothed time of one animation step, in microseconds.
    anim_us: f32,
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

    /// Gives the time of one animation step, in microseconds (smoothed).
    #[func]
    pub fn anim_microseconds(&self) -> f32 {
        self.anim_us
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

    /// Poses the parts for one physics step. The bear calls this.
    pub fn animate(&mut self, input: &AnimInput, ground: &mut dyn Ground) {
        let Some(rig) = self.rig else {
            return;
        };
        if !self.animate {
            return;
        }
        let start = std::time::Instant::now();
        let params = self.anim_params();
        let pose = self.anim.update(&rig, &params, input, ground);
        self.apply_pose(&pose);
        let us = start.elapsed().as_secs_f32() * 1e6;
        self.anim_us += (us - self.anim_us) * 0.05;
    }

    /// Gives the pose to the nodes of the parts. While the bear gets up,
    /// the pose is a mix of the ragdoll pose and the animation.
    fn apply_pose(&mut self, pose: &Pose) {
        let blend = self.blend;
        let mix = |i: usize, to: glam::Affine3A| -> Transform3D {
            match blend.and_then(|b| b.from[i].map(|from| (from, b.weight()))) {
                Some((from, k)) => to_transform(&mix_affine(&from, &to, k)),
                None => to_transform(&to),
            }
        };
        for (n, (id, node, material)) in self.parts.iter_mut().enumerate() {
            let i = PartId::ALL.iter().position(|p| p == id).unwrap_or(n);
            match *id {
                PartId::Body => {
                    node.set_transform(mix(i, pose.body));
                    let hump = to_vector(pose.hump);
                    if hump.distance_to(self.hump) > 1e-5 {
                        self.hump = hump;
                        material.set_shader_parameter("wobble_offset", &hump.to_variant());
                    }
                }
                PartId::Head => {
                    let at = to_glam(node.get_position());
                    let t = mix(i, glam::Affine3A::from_rotation_translation(pose.head, at));
                    node.set_basis(t.basis);
                }
                PartId::LeftEar => node.set_basis(to_basis(pose.ears[0])),
                PartId::RightEar => node.set_basis(to_basis(pose.ears[1])),
                limb => {
                    if let Some(l) = LIMBS.iter().position(|&x| x == limb) {
                        node.set_transform(mix(i, pose.limbs[l]));
                    }
                }
            }
        }
    }

    /// Starts the animation again from the rest pose (after a tumble).
    pub fn reset_anim(&mut self) {
        self.anim.reset();
    }

    /// Sets the look of each limb: on the body (with its looseness),
    /// on a thread, or gone.
    pub fn set_limb_looks(&mut self, looks: [LimbLook; 4]) {
        self.limb_looks = looks;
        for (i, look) in looks.iter().enumerate() {
            if *look != LimbLook::Dangling {
                self.threads[i] = None;
            }
            if let Some(mut node) = self.part(LIMBS[i]) {
                node.set_visible(*look != LimbLook::Gone);
            }
        }
    }

    /// The work after the pose: torn limbs droop, dangling limbs hang on
    /// their threads, and the move back from the ragdoll goes on.
    /// `floor_y` is the height of the ground below the bear.
    pub fn after_pose(&mut self, dt: f32, floor_y: f32) {
        if let Some(blend) = &mut self.blend {
            blend.left -= dt;
            if blend.left <= 0.0 {
                self.blend = None;
            }
        }
        let (Some(rig), Some(body)) = (self.rig, self.part(PartId::Body)) else {
            return;
        };
        let body_global = body.get_global_transform();
        let params = self.thread_params();
        let mut any_thread = false;
        for (i, &id) in LIMBS.iter().enumerate() {
            let Some(mut node) = self.part(id) else {
                continue;
            };
            let tip = to_vector(rig.limb_tip[i]);
            match self.limb_looks[i] {
                LimbLook::On { loose } if loose > 0.01 && self.animate => {
                    // A torn limb droops toward the ground.
                    let local = node.get_transform();
                    let dir = (local.basis * tip).normalized_or_zero();
                    let down = (body_global.basis.inverse() * Vector3::DOWN).normalized_or_zero();
                    let axis = dir.cross(down);
                    if axis.length() > 1e-3 {
                        let angle =
                            (self.droop_degrees.to_radians() * loose).min(dir.angle_to(down));
                        let turn = Basis::from_quaternion(Quaternion::from_axis_angle(
                            axis.normalized(),
                            angle,
                        ));
                        node.set_basis(turn * local.basis);
                    }
                }
                LimbLook::Dangling => {
                    any_thread = true;
                    let socket = to_glam(body_global * to_vector(rig.limb_joint[i]));
                    let thread = self.threads[i].get_or_insert_with(|| {
                        let global = node.get_global_transform();
                        let dir = to_glam(global.basis * tip);
                        Thread::new(socket, dir, rig.limb_tip[i].length(), &params)
                    });
                    thread.step(dt, socket, floor_y, &params);
                    let (at, dir) = thread.limb();
                    let turn =
                        glam::Quat::from_rotation_arc(rig.limb_tip[i].normalize_or_zero(), dir);
                    let global = Transform3D::new(to_basis(turn), to_vector(at));
                    node.set_transform(body_global.affine_inverse() * global);
                }
                _ => {}
            }
        }
        self.draw_threads(any_thread);
    }

    /// The tip velocity of a dangling limb, in m/s.
    pub fn thread_velocity(&self, limb: usize, dt: f32) -> Option<Vector3> {
        let thread = self.threads.get(limb)?.as_ref()?;
        Some(to_vector(thread.tip_velocity(dt)))
    }

    fn thread_params(&self) -> ThreadParams {
        ThreadParams {
            length: self.thread_length_m.clamp(0.005, 0.2),
            keep: self.thread_keep.clamp(0.5, 1.0),
            ..ThreadParams::default()
        }
    }

    /// Draws each thread as a line from the socket to the limb.
    fn draw_threads(&mut self, any: bool) {
        if !any && self.thread_mesh.is_none() {
            return;
        }
        let mut mesh = match self.thread_mesh.clone() {
            Some(mesh) => mesh,
            None => {
                let mesh = ImmediateMesh::new_gd();
                let mut node = MeshInstance3D::new_alloc();
                node.set_name("Threads");
                node.set_as_top_level(true);
                node.set_mesh(&mesh);
                let mut material = StandardMaterial3D::new_gd();
                material.set_shading_mode(ShadingMode::UNSHADED);
                material.set_flag(Flags::DISABLE_FOG, false);
                let c = self.thread_colour;
                material.set_albedo(Color::from_rgb(c.r * 2.0, c.g * 2.0, c.b * 2.0));
                node.set_material_override(&material.upcast::<Material>());
                self.base_mut().add_child(&node);
                node.set_global_transform(Transform3D::IDENTITY);
                self.thread_mesh = Some(mesh.clone());
                mesh
            }
        };
        mesh.clear_surfaces();
        for thread in self.threads.iter().flatten() {
            mesh.surface_begin(godot::classes::mesh::PrimitiveType::LINE_STRIP);
            for p in &thread.points[..=LIMB_JOINT] {
                mesh.surface_add_vertex(to_vector(*p));
            }
            mesh.surface_end();
        }
    }

    /// Starts the move from the ragdoll back to the animation. The
    /// transforms are of the meshes in the world. Call it after the bear
    /// and this model are at their new place.
    pub fn blend_from_world(&mut self, world: &[(PartId, Transform3D)]) {
        let find = |id: PartId| world.iter().find(|w| w.0 == id).map(|w| w.1);
        let Some(body) = find(PartId::Body) else {
            self.blend = None;
            return;
        };
        let model = self.base().get_global_transform();
        let mut from = [None; 8];
        for (i, id) in PartId::ALL.iter().enumerate() {
            let local = match *id {
                PartId::Body => Some(model.affine_inverse() * body),
                PartId::LeftEar | PartId::RightEar => None,
                other => find(other).map(|t| body.affine_inverse() * t),
            };
            from[i] = local.map(|t| to_affine(&t));
        }
        let time = self.get_up_time.max(0.01);
        self.blend = Some(Blend {
            from,
            left: time,
            total: time,
        });
        self.threads = [None; 4];
    }

    /// The parts for a ragdoll: the body, the head (with the ears) and
    /// each limb that is on the body. `looseness` is of each seam.
    pub fn ragdoll_parts(&self, looseness: [f32; 4]) -> Vec<RagdollPart> {
        let mut out = Vec::new();
        let body_global = self.part(PartId::Body).map(|b| b.get_global_transform());
        for def in &self.bodies {
            let limb = LIMBS.iter().position(|&l| l == def.id);
            if let Some(i) = limb {
                if self.limb_looks[i] == LimbLook::Gone {
                    continue;
                }
            }
            let Some(look) = self.part_look(def.id) else {
                continue;
            };
            let Some(node) = self.part(def.id) else {
                continue;
            };
            let mut looks = vec![look];
            if def.id == PartId::Head {
                for ear in [PartId::LeftEar, PartId::RightEar] {
                    if let (Some(mut l), Some(e)) = (self.part_look(ear), self.part(ear)) {
                        l.local = e.get_transform();
                        looks.push(l);
                    }
                }
            }
            let dangling_socket = match (limb, self.rig, body_global) {
                (Some(i), Some(rig), Some(b)) if self.limb_looks[i] == LimbLook::Dangling => {
                    Some(b * to_vector(rig.limb_joint[i]))
                }
                _ => None,
            };
            out.push(RagdollPart {
                id: def.id,
                def: *def,
                global: node.get_global_transform(),
                looks,
                looseness: limb.map_or(0.0, |i| looseness[i]),
                dangling_socket,
            });
        }
        out
    }

    /// The body and the look of one limb, and its transform in the
    /// world, for a lost limb.
    pub fn limb_part(&self, limb: usize) -> Option<(BodyDef, PartLook, Transform3D)> {
        let id = *LIMBS.get(limb)?;
        let def = *self.bodies.iter().find(|b| b.id == id)?;
        let mut look = self.part_look(id)?;
        let global = self.part(id)?.get_global_transform();
        let basis = global.basis.orthonormalized();
        look.local = Transform3D::new(basis.inverse() * global.basis, Vector3::ZERO);
        Some((def, look, Transform3D::new(basis, global.origin)))
    }

    fn part_look(&self, id: PartId) -> Option<PartLook> {
        let (_, node, material) = self.parts.iter().find(|p| p.0 == id)?;
        Some(PartLook {
            mesh: node.get_mesh()?,
            material: material.clone().upcast::<Material>(),
            local: Transform3D::IDENTITY,
        })
    }

    /// Gives the tuning values of the animation from the exported fields.
    fn anim_params(&self) -> AnimParams {
        AnimParams {
            stride_base: self.stride_base_m.max(0.02),
            stride_per_speed: self.stride_per_speed.max(0.0),
            stance_length: self.stance_length_m.max(0.01),
            step_height: self.step_height_m.max(0.0),
            idle_step_time: self.idle_step_time.max(0.02),
            idle_step_distance: self.idle_step_distance_m.max(0.005),
            foot_half_length: self.foot_half_length_m.max(0.0),
            bob_height: self.bob_height_m,
            waddle: self.waddle_degrees.to_radians(),
            twist: self.twist_degrees.to_radians(),
            speed_lean: self.speed_lean_degrees.to_radians(),
            accel_tilt: self.accel_tilt_degrees.to_radians(),
            idle_slump: self.idle_slump_degrees.to_radians(),
            idle_sway: self.idle_sway_degrees.to_radians(),
            slope_lean: self.slope_lean.clamp(0.0, 1.0),
            tilt_frequency: self.tilt_frequency,
            tilt_damping: self.tilt_damping,
            land_squash: self.land_squash.max(0.0),
            jump_stretch: self.jump_stretch.max(0.0),
            squash_frequency: self.squash_frequency,
            squash_damping: self.squash_damping,
            sink_depth: self.sink_depth_m.max(0.0),
            slip_amount: self.slip_amount.clamp(0.0, 1.0),
            firm_grip: self.slip_firm_grip.clamp(0.01, 1.0),
            arm_swing: self.arm_swing_degrees.to_radians(),
            arm_frequency: self.arm_frequency,
            arm_damping: self.arm_damping,
            arm_inertia: self.arm_inertia.max(0.0),
            head_level: self.head_level.clamp(0.0, 1.0),
            head_turn: self.head_turn,
            head_frequency: self.head_frequency,
            head_damping: self.head_damping,
            ear_flop: self.ear_flop_degrees.to_radians(),
            ear_frequency: self.ear_frequency,
            ear_damping: self.ear_damping,
            hump_wobble: self.hump_wobble_mm * 0.001,
            hump_frequency: self.hump_frequency,
            hump_damping: self.hump_damping,
            hop_height: self.hop_height_m.max(0.0),
            hop_duty: self.hop_duty.clamp(0.2, 0.8),
            hop_lean: self.hop_lean_degrees.to_radians(),
            hop_arms_out: self.hop_arms_out_degrees.to_radians(),
            lie_angle: self.lie_angle_degrees.to_radians(),
            lie_height: self.lie_height_m.max(0.0),
            lie_head: self.lie_head.clamp(0.0, 1.0),
            crawl_reach: self.crawl_reach.clamp(0.3, 1.2),
            drag_roll: self.drag_roll_degrees.to_radians(),
            wriggle_yaw: self.wriggle_yaw_degrees.to_radians(),
            wriggle_roll: self.wriggle_roll_degrees.to_radians(),
            hand_reach: self.hand_reach.clamp(0.0, 1.0),
            hold_step: self.hold_step_m.max(0.01),
            hand_step_time: self.hand_step_time.max(0.02),
            foot_step_time: self.foot_step_time.max(0.02),
            fan_spread: self.fan_spread_m.max(0.005),
            hold_lift: self.hold_lift_m.max(0.0),
            hang_follow: self.hang_follow.clamp(0.0, 1.0),
            hang_sag: self.hang_sag_m.max(0.0),
            hang_roll: self.hang_roll,
        }
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
        self.rig = Rig::from_points(parts.iter().map(|p| (p.id, p.joint, p.tip)));
        self.bodies = tumble::bodies(&parts);
        if let (Some(rig), Some((_, _, body))) = (
            self.rig,
            self.parts.iter_mut().find(|p| p.0 == PartId::Body),
        ) {
            let centre = to_vector(rig.hump_centre);
            body.set_shader_parameter("wobble_centre", &centre.to_variant());
            body.set_shader_parameter("wobble_radius", &rig.hump_radius.to_variant());
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

fn to_vector(v: glam::Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

fn to_glam(v: Vector3) -> glam::Vec3 {
    glam::Vec3::new(v.x, v.y, v.z)
}

fn to_affine(t: &Transform3D) -> glam::Affine3A {
    let b = t.basis;
    glam::Affine3A::from_cols(
        to_glam(b.col_a()).into(),
        to_glam(b.col_b()).into(),
        to_glam(b.col_c()).into(),
        to_glam(t.origin).into(),
    )
}

/// Mixes two transforms: `k` 0 gives `a`, 1 gives `b`.
fn mix_affine(a: &glam::Affine3A, b: &glam::Affine3A, k: f32) -> glam::Affine3A {
    let (sa, ra, ta) = a.to_scale_rotation_translation();
    let (sb, rb, tb) = b.to_scale_rotation_translation();
    glam::Affine3A::from_scale_rotation_translation(
        sa.lerp(sb, k),
        ra.normalize().slerp(rb.normalize(), k),
        ta.lerp(tb, k),
    )
}

/// The look of a limb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LimbLook {
    /// On the body. `loose` is 0 for a new seam and near 1 for a torn one.
    On { loose: f32 },
    /// On a thread.
    Dangling,
    /// Not on the bear.
    Gone,
}

impl Default for LimbLook {
    fn default() -> LimbLook {
        LimbLook::On { loose: 0.0 }
    }
}

/// The move from the pose of the ragdoll back to the animation.
#[derive(Clone, Copy, Debug)]
struct Blend {
    /// The ragdoll pose of each part, in the space of its parent, in the
    /// order of `PartId::ALL`.
    from: [Option<glam::Affine3A>; 8],
    left: f32,
    total: f32,
}

impl Blend {
    /// The part of the animation in the mix (0 to 1), with an ease.
    fn weight(&self) -> f32 {
        let t = (1.0 - self.left / self.total.max(1e-4)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

fn to_basis(q: glam::Quat) -> Basis {
    Basis::from_quaternion(Quaternion::new(q.x, q.y, q.z, q.w))
}

fn to_transform(a: &glam::Affine3A) -> Transform3D {
    let m = a.matrix3;
    Transform3D::new(
        Basis::from_cols(
            to_vector(m.x_axis.into()),
            to_vector(m.y_axis.into()),
            to_vector(m.z_axis.into()),
        ),
        to_vector(a.translation.into()),
    )
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
