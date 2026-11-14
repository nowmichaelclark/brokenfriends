//! The teddy bear: the player.

use crate::anim::{AnimInput, AnimMode, Ground, LIMBS};
use crate::bear_model::{BfBearModel, LimbLook};
use crate::climb::{
    find_grab, slope_angle, stamina_drain, wall_axes, wall_input, Climb, ClimbParams, Hit, Step,
    Surface,
};
use crate::follow_camera::FOLLOW_CAMERA_GROUP;
use crate::limbs::{profile, GaitProfile, GaitTuning, LimbSet};
use crate::lost_limb::{self, BfLostLimb, LostLimbSpec, BEAR_PARTS_LAYER, LOST_LIMB_GROUP};
use crate::motion::{angle_difference, approach, jump_speed, JumpTimers};
use crate::ragdoll::Ragdoll;
use crate::tear::{Change, Seams, Stage, TearParams};
use crate::terrain::TERRAIN_GROUP;
use crate::tumble::{get_up_yaw, impact, start_spin, Settle, TumbleParams};
use glam::{Vec2, Vec3};
use godot::classes::base_material_3d::{BillboardMode, CullMode};
use godot::classes::cpu_particles_3d::Parameter;
use godot::classes::{
    CapsuleShape3D, CharacterBody3D, CollisionShape3D, CpuParticles3D, ICharacterBody3D, Input,
    InputEvent, InputEventKey, Material, PhysicsBody3D, PhysicsDirectSpaceState3D,
    PhysicsRayQueryParameters3D, ProjectSettings, QuadMesh, StandardMaterial3D,
};
use godot::global::{JoyAxis, JoyButton, Key};
use godot::prelude::*;

/// The radius of the collision capsule in metres.
const RADIUS: f32 = 0.11;
/// The distance from the feet to the centre of the bear in metres.
const HALF_HEIGHT: f32 = 0.20;
/// The collider metadata that gives the grip of an object (0 to 1).
/// Without it, the terrain gives the grip.
pub const GRIP_META: &str = "bf_grip";
/// The collider metadata that gives the softness of an object (0 to 1).
/// Without it, the terrain gives the softness: only sand is soft.
pub const SOFT_META: &str = "bf_soft";

/// What the bear does now.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Mode {
    /// The bear walks, jumps or falls. The physics engine moves it.
    #[default]
    Walk,
    /// The bear is on a wall. Ray casts move it.
    Climb(Climb),
    /// The bear moves from a wall to a place to stand, in a short time.
    Mantle {
        from: Vector3,
        to: Vector3,
        time: f32,
    },
    /// The bear tumbles: it is a ragdoll, and Jolt moves it (R11).
    Tumble,
}

fn to_glam(v: Vector3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn to_godot(v: Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

/// The physics world as a climbing surface. Each `cast` is one ray cast.
struct RaySurface {
    space: Gd<PhysicsDirectSpaceState3D>,
    query: Gd<PhysicsRayQueryParameters3D>,
    terrain: Option<Gd<Node>>,
}

impl Surface for RaySurface {
    fn cast(&mut self, from: Vec3, to: Vec3) -> Option<Hit> {
        self.query.set_from(to_godot(from));
        self.query.set_to(to_godot(to));
        let result = self.space.intersect_ray(&self.query);
        let point = result.get("position")?.try_to::<Vector3>().ok()?;
        let normal = result.get("normal")?.try_to::<Vector3>().ok()?;
        let id = result.get("collider_id")?.try_to::<i64>().unwrap_or(0);
        Some(Hit {
            point: to_glam(point),
            normal: to_glam(normal).normalize_or(Vec3::Y),
            id: id as u64,
        })
    }

    fn grip(&mut self, hit: &Hit) -> f32 {
        let collider = (hit.id != 0)
            .then(|| Gd::<Object>::try_from_instance_id(InstanceId::from_i64(hit.id as i64)).ok())
            .flatten();
        if let Some(collider) = collider {
            if collider.has_meta(GRIP_META) {
                let grip = collider.get_meta(GRIP_META).try_to::<f32>();
                return grip.unwrap_or(1.0).clamp(0.0, 1.0);
            }
        }
        let Some(terrain) = &mut self.terrain else {
            return 1.0;
        };
        let at = [hit.point.x.to_variant(), hit.point.z.to_variant()];
        terrain.call("grip_at", &at).try_to::<f32>().unwrap_or(1.0)
    }
}

impl Ground for RaySurface {
    fn softness(&mut self, hit: &Hit) -> f32 {
        let collider = (hit.id != 0)
            .then(|| Gd::<Object>::try_from_instance_id(InstanceId::from_i64(hit.id as i64)).ok())
            .flatten();
        if let Some(collider) = collider {
            if collider.has_meta(SOFT_META) {
                let soft = collider.get_meta(SOFT_META).try_to::<f32>();
                return soft.unwrap_or(0.0).clamp(0.0, 1.0);
            }
        }
        let Some(terrain) = &mut self.terrain else {
            return 0.0;
        };
        // Only the loose sand of the riverbeds is soft.
        let at = [hit.point.x.to_variant(), hit.point.z.to_variant()];
        let name = terrain.call("ground_name_at", &at).to_string();
        if name == "sand" {
            1.0
        } else {
            0.0
        }
    }
}

/// What the player pushes now.
#[derive(Clone, Copy, Default)]
struct Controls {
    /// The move direction on the ground (x, z), 1 long at most.
    wish: Vector2,
    jog: bool,
    jump: bool,
    /// The let-go button (R21).
    let_go: bool,
}

/// The group that holds the bear. Other nodes find it there.
pub const BEAR_GROUP: &str = "bf_bear";

/// The teddy bear. It is 0.40 m tall. It walks, jogs, jumps and climbs.
///
/// Keys: W, A, S, D or the arrow keys to move, Shift to jog, Space to
/// jump, C to let go of a wall. Gamepad: left stick to move, A to jump,
/// the right shoulder button to jog, B to let go.
///
/// A jog uses stamina (R20). A tired bear walks.
///
/// The bear starts to climb when it pushes into a surface that is too
/// steep to walk. On the wall, "into the wall" is up and "away from
/// the wall" is down. Jump makes a hop up the wall, and it uses
/// stamina (R20). Jump with the stick away from the wall jumps off.
/// The let-go button drops the bear (R21).
///
/// The bear can lose limbs (R12). The limbs that it has set its gait,
/// its speed, its jump and its climb limits (`limbs.rs`). Debug keys:
/// 1 to 4 remove or give back the left arm, the right arm, the left leg
/// and the right leg. 0 gives back all limbs.
#[derive(GodotClass)]
#[class(base = CharacterBody3D, init)]
pub struct BfBear {
    /// The walk speed in metres per second.
    #[export]
    #[init(val = 1.4)]
    walk_speed: f32,

    /// The jog speed in metres per second (hold Shift).
    #[export]
    #[init(val = 3.2)]
    jog_speed: f32,

    /// How fast the bear gets its speed on the ground, in m/s each second.
    #[export]
    #[init(val = 18.0)]
    acceleration: f32,

    /// How fast the bear stops on the ground, in m/s each second.
    #[export]
    #[init(val = 24.0)]
    deceleration: f32,

    /// The part of the control that the bear has in the air (0 to 1).
    #[export]
    #[init(val = 0.45)]
    air_control: f32,

    /// The height of a jump in metres.
    #[export]
    #[init(val = 0.55)]
    jump_height: f32,

    /// The gravity on the bear in m/s each second.
    #[export]
    #[init(val = 14.0)]
    fall_acceleration: f32,

    /// The gravity is multiplied by this value while the bear falls.
    #[export]
    #[init(val = 1.4)]
    fall_gravity_factor: f32,

    /// The highest fall speed in metres per second.
    #[export]
    #[init(val = 25.0)]
    max_fall_speed: f32,

    /// The time after an edge in which the bear can jump, in seconds.
    #[export]
    #[init(val = 0.12)]
    coyote_time: f32,

    /// The time before a landing in which a jump press counts, in seconds.
    #[export]
    #[init(val = 0.14)]
    jump_buffer_time: f32,

    /// The steepest slope that the bear can walk, in degrees.
    #[export]
    #[init(val = 50.0)]
    max_slope_degrees: f32,

    /// How fast the bear turns to its move direction, in degrees per second.
    #[export]
    #[init(val = 720.0)]
    turn_speed_degrees: f32,

    /// The stick must move more than this value (0 to 1).
    #[export]
    #[init(val = 0.2)]
    stick_dead_zone: f32,

    /// The climb speed in metres per second.
    #[export]
    #[init(val = 0.7)]
    climb_speed: f32,

    /// The climb stops on a slope this much flatter than `max_slope_degrees`.
    #[export]
    #[init(val = 5.0)]
    climb_stop_margin_degrees: f32,

    /// The steepest surface that the bear can hold, in degrees (90 is a wall).
    #[export]
    #[init(val = 110.0)]
    climb_max_degrees: f32,

    /// How far the paws reach past the body to find a wall, in metres.
    #[export]
    #[init(val = 0.14)]
    climb_reach: f32,

    /// How long the bear on the ground must push into a wall to start a climb, in seconds.
    #[export]
    #[init(val = 0.15)]
    grab_delay: f32,

    /// A bear that moves faster than this speed cannot grab a wall, in m/s.
    #[export]
    #[init(val = 6.0)]
    max_grab_speed: f32,

    /// The height of the jump off a wall in metres.
    #[export]
    #[init(val = 0.3)]
    climb_jump_height: f32,

    /// The speed away from the wall when the bear jumps off, in m/s.
    #[export]
    #[init(val = 1.8)]
    climb_jump_push: f32,

    /// The height of a hop up the wall in metres (jump, with the stick not away from the wall).
    #[export]
    #[init(val = 0.35)]
    climb_hop_height: f32,

    /// The time of a hop up the wall in seconds.
    #[export]
    #[init(val = 0.25)]
    climb_hop_time: f32,

    /// The stamina that one hop uses (0 to 1). A climb of the same height uses about 0.017.
    #[export]
    #[init(val = 0.04)]
    climb_hop_stamina: f32,

    /// On a wall, a jump with the stick this far away from the wall jumps off (0 to 1).
    #[export]
    #[init(val = 0.5)]
    jump_off_stick: f32,

    /// How long the stamina lasts while the bear jogs on the ground, in seconds.
    #[export]
    #[init(val = 20.0)]
    jog_stamina_seconds: f32,

    /// The time after a jump off or a slide off in which the bear cannot grab, in seconds.
    #[export]
    #[init(val = 0.4)]
    regrab_time: f32,

    /// How long the stamina lasts on a vertical wall, in seconds.
    #[export]
    #[init(val = 30.0)]
    stamina_seconds: f32,

    /// The part of the stamina use on the least steep climb (0 to 1).
    #[export]
    #[init(val = 0.3)]
    gentle_drain_factor: f32,

    /// The time on the ground that makes the stamina full again, in seconds.
    #[export]
    #[init(val = 3.0)]
    stamina_recover_seconds: f32,

    /// After the stamina is gone, the bear can climb again at this part of it (0 to 1).
    #[export]
    #[init(val = 0.3)]
    regrab_stamina_part: f32,

    /// With this grip or more, the bear does not slide on a wall (0 to 1).
    #[export]
    #[init(val = 0.7)]
    firm_grip: f32,

    /// With this grip or less, the bear slides off completely (0 to 1).
    #[export]
    #[init(val = 0.25)]
    loose_grip: f32,

    /// The fastest slow slide on a slippery wall, in m/s.
    #[export]
    #[init(val = 0.35)]
    slow_slide_speed: f32,

    /// If the bear slides faster than this speed, it lets go, in m/s.
    #[export]
    #[init(val = 1.5)]
    let_go_slide_speed: f32,

    /// The time to climb from a wall onto a ledge, in seconds.
    #[export]
    #[init(val = 0.2)]
    mantle_time: f32,

    /// How fast the body turns to the wall direction, in 1/s.
    #[export]
    #[init(val = 12.0)]
    climb_turn_rate: f32,

    /// The speed of a bear with two legs and fewer than two arms, as a part of the normal speed.
    #[export]
    #[init(val = 0.95)]
    armless_walk_speed_scale: f32,

    /// The speed of a hop on one leg with two arms, as a part of the normal speed.
    #[export]
    #[init(val = 0.6)]
    hop_speed_scale: f32,

    /// The hop gets slower by this part for each missing arm (0 to 1).
    #[export]
    #[init(val = 0.15)]
    hop_arm_slowdown: f32,

    /// The jump height on one leg, as a part of the normal jump.
    #[export]
    #[init(val = 0.6)]
    hop_jump_scale: f32,

    /// The speed of an arm-crawl (no legs), as a part of the normal speed.
    #[export]
    #[init(val = 0.3)]
    crawl_speed_scale: f32,

    /// The speed of a one-arm drag (no legs, one arm), as a part of the normal speed.
    #[export]
    #[init(val = 0.18)]
    drag_speed_scale: f32,

    /// The speed of a wriggle (no limbs), as a part of the normal speed.
    #[export]
    #[init(val = 0.1)]
    wriggle_speed_scale: f32,

    /// The steepest surface that a bear with one arm can hold, in degrees.
    #[export]
    #[init(val = 68.0)]
    one_arm_climb_degrees: f32,

    /// The climb stamina use of a bear with one arm is multiplied by this value.
    #[export]
    #[init(val = 1.6)]
    one_arm_stamina_factor: f32,

    /// The climb stamina use of a bear with one leg is multiplied by this value.
    #[export]
    #[init(val = 1.15)]
    one_leg_stamina_factor: f32,

    /// The climb stamina use of a bear with no legs is multiplied by this value.
    #[export]
    #[init(val = 1.3)]
    no_legs_stamina_factor: f32,

    /// A hit at this speed or faster makes the bear tumble, in m/s (a landing, a wall in the air).
    #[export]
    #[init(val = TumbleParams::default().impact_speed)]
    tumble_impact_speed: f32,

    /// A bear that slides off a wall at this speed or faster tumbles, in m/s.
    #[export]
    #[init(val = TumbleParams::default().let_go_speed)]
    tumble_let_go_speed: f32,

    /// At the start of a tumble, the body turns this fast for each m/s of speed, in rad/s.
    #[export]
    #[init(val = TumbleParams::default().spin_per_speed)]
    tumble_spin_per_speed: f32,

    /// The fastest turn at the start of a tumble, in rad/s.
    #[export]
    #[init(val = TumbleParams::default().max_spin)]
    tumble_max_spin: f32,

    /// The ragdoll is at rest below this speed, in m/s.
    #[export]
    #[init(val = TumbleParams::default().rest_speed)]
    tumble_rest_speed: f32,

    /// The ragdoll is at rest below this turn rate, in rad/s.
    #[export]
    #[init(val = TumbleParams::default().rest_spin)]
    tumble_rest_spin: f32,

    /// The ragdoll must be at rest this long before the bear gets up, in seconds.
    #[export]
    #[init(val = TumbleParams::default().rest_time)]
    tumble_rest_seconds: f32,

    /// The shortest tumble in seconds.
    #[export]
    #[init(val = TumbleParams::default().min_time)]
    tumble_min_seconds: f32,

    /// After this time, the bear gets up even if the ragdoll still moves, in seconds.
    #[export]
    #[init(val = TumbleParams::default().max_time)]
    tumble_max_seconds: f32,

    /// How strongly the wind carries a tumbling bear, in 1/s.
    #[export]
    #[init(val = 2.0)]
    tumble_wind_drag: f32,

    /// A hit on a limb slower than this does no damage to its seam, in m/s.
    #[export]
    #[init(val = TearParams::default().damage_speed)]
    seam_damage_speed: f32,

    /// The seam health (0 to 1) that each m/s above `seam_damage_speed` takes away.
    #[export]
    #[init(val = TearParams::default().damage_per_speed)]
    seam_damage_per_speed: f32,

    /// The seam health that the thread of a dangling limb loses for each metre the bear moves.
    #[export]
    #[init(val = TearParams::default().thread_wear_per_m)]
    thread_wear_per_m: f32,

    /// The bear sews a lost limb on again when it is this near to it, in metres.
    #[export]
    #[init(val = 0.25)]
    sew_distance_m: f32,

    /// The bear must stand still on a lost limb this long to sew it on, in seconds.
    #[export]
    #[init(val = 1.0)]
    sew_seconds: f32,

    /// The most lost limbs in the world. The oldest go first.
    #[export]
    #[init(val = 12)]
    max_lost_limbs: i32,

    /// The limbs that work: on the body, and not dangling. The seams set it.
    limbs: LimbSet,
    /// The seam of each limb (`tear.rs`).
    seams: Seams,
    /// The bodies of a tumble.
    ragdoll: Option<Ragdoll>,
    /// Finds when the ragdoll is at rest.
    settle: Settle,
    /// The place of the bear in the last step, for the wear of threads.
    last_feet: Vector3,
    /// Counts the steps between two looks for lost limbs to sew.
    sew_wait: u32,
    /// How long the bear stands still on a lost limb, in seconds.
    sew_hold: f32,
    /// The puff of stuffing from a torn seam.
    stuffing: Option<Gd<CpuParticles3D>>,
    /// The collision capsule. It is off while the bear tumbles.
    shape: Option<Gd<CollisionShape3D>>,

    /// The time until wet fur is dry again, in seconds.
    #[export]
    #[init(val = 40.0)]
    dry_seconds: f32,

    /// The wind from the obstacles: a velocity that carries the bear.
    /// `BfObstacles` sets it before each physics step.
    wind: Vector3,
    /// The air lifts a bear in the air: the speed up that it goes to
    /// (m/s) and how fast it changes (m/s each second). `BfObstacles`
    /// sets it before each physics step.
    lift: Vector2,
    /// Sand slows a bear on the ground: from 0 (no slow down) to 1.
    /// `BfObstacles` sets it before each physics step.
    ground_slow: f32,
    /// An obstacle threw the bear. In the air, with no input, the bear
    /// keeps this velocity on the ground plane (x, z).
    thrown: Option<Vector2>,
    /// How wet the fur is, from 0 to 1. It dries with time.
    wet: f32,
    /// The wet value that the model has now.
    wet_shown: f32,

    timers: JumpTimers,
    jump_was_down: bool,
    let_go_was_down: bool,
    mode: Mode,
    /// The stamina, from 0 (empty) to 1 (full).
    #[init(val = 1.0)]
    stamina: f32,
    /// The stamina was gone. The bear cannot climb until some comes back.
    exhausted: bool,
    /// The time that is left until the bear can grab a wall again.
    regrab_left: f32,
    /// How long the bear on the ground pushes into a wall.
    push_time: f32,
    /// The direction that the body looks at, as an angle around Y.
    yaw: f32,
    /// The move direction of the last physics step (x, z).
    last_wish: Vector2,
    /// The time in the air since the bear left the ground, in seconds.
    air_time: f32,
    /// The bear slides on a wall now.
    was_sliding: bool,
    /// Events for the playtest recorder, since its last `take_events`.
    events: Vec<String>,
    /// A second copy of the events for the autoplay bot (S-133), or
    /// `None` while no bot listens.
    tap: Option<Vec<String>>,
    /// One ray query for all ray casts.
    ray_query: Option<Gd<PhysicsRayQueryParameters3D>>,
    /// The visible bear. It turns; the collision shape does not.
    body: Option<Gd<Node3D>>,
    model: Option<Gd<BfBearModel>>,
    base: Base<CharacterBody3D>,
}

#[godot_api]
impl ICharacterBody3D for BfBear {
    fn ready(&mut self) {
        self.base_mut().add_to_group(BEAR_GROUP);
        self.build_body();

        // When a new zone shows, go to its start point.
        let tree = self.base().get_tree();
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("go_to_start");
            terrain.connect("zone_changed", &on_zone);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let delta = delta as f32;
        let Controls {
            wish,
            jog,
            jump: jump_down,
            let_go: let_go_down,
        } = self.read_input();
        let jump_pressed = jump_down && !self.jump_was_down;
        self.jump_was_down = jump_down;
        let let_go_pressed = let_go_down && !self.let_go_was_down;
        self.let_go_was_down = let_go_down;
        self.last_wish = wish;
        self.regrab_left = (self.regrab_left - delta).max(0.0);

        match self.mode {
            Mode::Walk => self.walk(delta, wish, jog, jump_pressed),
            Mode::Climb(climb) => self.climb(delta, climb, wish, jump_pressed, let_go_pressed),
            Mode::Mantle { from, to, time } => self.mantle(delta, from, to, time),
            Mode::Tumble => self.tumble(delta),
        }
        if self.mode != Mode::Tumble {
            self.pose_body(delta);
            self.after_move(delta);
        }
        self.dry(delta);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(key) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if !key.is_pressed() || key.is_echo() {
            return;
        }
        // Debug keys: 1 to 4 remove or give back one limb, 0 gives back all.
        let limb = match key.get_keycode() {
            Key::KEY_1 => Some(0),
            Key::KEY_2 => Some(1),
            Key::KEY_3 => Some(2),
            Key::KEY_4 => Some(3),
            Key::KEY_0 => {
                self.restore_limbs();
                None
            }
            // 5: a small throw, 6: a hard throw that can tear limbs.
            Key::KEY_5 => {
                self.tumble_now(false);
                None
            }
            Key::KEY_6 => {
                self.tumble_now(true);
                None
            }
            _ => None,
        };
        if let Some(i) = limb {
            let on = self.seams.stage(i) != Stage::Detached;
            self.set_seam(i, if on { 0.0 } else { 1.0 });
        }
    }
}

#[godot_api]
impl BfBear {
    /// Puts the bear on the start point of the zone: the flattest place
    /// near the centre.
    #[func]
    pub fn go_to_start(&mut self) {
        let tree = self.base().get_tree();
        let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) else {
            return;
        };
        // The terrain knows the start point. The obstacles stay away
        // from it.
        let start = terrain.call("start_point", &[]).try_to::<Vector2>();
        let (x, z) = start.map_or((0.0, 0.0), |p| (p.x, p.y));
        let height = terrain
            .call("height_at", &[x.to_variant(), z.to_variant()])
            .to::<f32>();
        let position = Vector3::new(x, height + 0.3, z);
        self.end_tumble();
        self.base_mut().set_global_position(position);
        self.base_mut().set_velocity(Vector3::ZERO);
        self.mode = Mode::Walk;
        // A new start is a new bear: all limbs and new seams. The lost
        // limbs go.
        self.remove_lost_limbs(None);
        for i in 0..4 {
            if let Some(change) = self.seams.mend(i) {
                self.seam_changed(change);
            }
        }
        self.last_feet = position;
        self.stamina = 1.0;
        self.exhausted = false;
        self.wind = Vector3::ZERO;
        self.lift = Vector2::ZERO;
        self.ground_slow = 0.0;
        self.thrown = None;
        self.event("restart");
    }

    /// Removes or gives back a limb: "LeftArm", "RightArm", "LeftLeg"
    /// or "RightLeg". S-114 and the dev menu (S-113) call this.
    #[func]
    pub fn set_limb(&mut self, part: GString, present: bool) {
        let Some(i) = limb_index(&part) else {
            return;
        };
        // A limb that goes falls into the world.
        self.set_seam(i, if present { 1.0 } else { 0.0 });
    }

    /// Gives back all limbs. The lost limbs in the world go.
    #[func]
    pub fn restore_limbs(&mut self) {
        for i in 0..4 {
            self.set_seam(i, 1.0);
        }
    }

    /// Takes health away from the seam of a limb (0 to 1). At 0, the limb
    /// falls off. Obstacles that tear limbs (S-129) call this.
    #[func]
    pub fn damage_seam(&mut self, part: GString, amount: f32) {
        let Some(i) = limb_index(&part) else {
            return;
        };
        if let Some(change) = self.seams.damage(i, amount) {
            self.seam_changed(change);
        }
    }

    /// Gives the health of the seam of a limb, from 0 (off) to 1 (new).
    #[func]
    pub fn seam_health(&self, part: GString) -> f32 {
        limb_index(&part).map_or(0.0, |i| self.seams.health(i))
    }

    /// Tells if the bear tumbles now (it is a ragdoll).
    #[func]
    pub fn is_tumbling(&self) -> bool {
        self.mode == Mode::Tumble
    }

    /// A dev tool: throws the bear, so it tumbles. A hard throw goes down
    /// into the ground fast and can tear limbs.
    #[func]
    pub fn tumble_now(&mut self, hard: bool) {
        if self.mode == Mode::Tumble {
            return;
        }
        let forward = Vector3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let (velocity, cause) = if hard {
            (forward * 4.0 + Vector3::new(0.0, -13.0, 0.0), "dev_hard")
        } else {
            (forward * 2.5 + Vector3::new(0.0, 4.5, 0.0), "dev")
        };
        // A hard throw starts a little above the ground.
        if hard {
            let up = self.base().get_global_position() + Vector3::new(0.0, 0.25, 0.0);
            self.base_mut().set_global_position(up);
        }
        self.start_tumble(velocity, cause);
    }

    /// Tells if the player holds the move stick (not in a tumble).
    #[func]
    pub fn stick_held(&self) -> bool {
        self.last_wish.length() > 0.01 && self.mode != Mode::Tumble
    }

    /// Takes health away from a seam, for an obstacle (S-129). `limb` is
    /// 0 to 3, in the order of the limbs. If the bear does not have
    /// that limb, the next limb that it has gets the damage. Gives the
    /// limb that got it, or -1.
    #[func]
    pub fn tear_limb(&mut self, limb: i32, amount: f32) -> i32 {
        for k in 0..4 {
            let i = (limb.max(0) as usize + k) % 4;
            if self.limbs.has(i) {
                if let Some(change) = self.seams.damage(i, amount) {
                    self.seam_changed(change);
                }
                return i as i32;
            }
        }
        -1
    }

    /// Tells if the bear has a limb, for example "LeftLeg".
    #[func]
    pub fn has_limb(&self, part: GString) -> bool {
        let name = part.to_string();
        LIMBS
            .iter()
            .position(|l| l.name() == name)
            .is_some_and(|i| self.limbs.has(i))
    }

    /// Gives the name of the gait now, for example "walk" or "hop".
    #[func]
    pub fn gait_name(&self) -> GString {
        GString::from(self.gait_profile().gait.name())
    }

    /// Kills the bear and starts it again at the start point (R16).
    /// `cause` is a short name for the playtest recorder, for example
    /// "smoke:time". There is no other penalty.
    #[func]
    pub fn die(&mut self, cause: GString) {
        self.event(&format!("death:{cause}"));
        self.go_to_start();
    }

    /// Keeps an event for the playtest recorder. Other nodes use it,
    /// for example the smoke edge ("smoke_enter").
    #[func]
    pub fn report_event(&mut self, name: GString) {
        self.event(&name.to_string());
    }

    /// Sets the wind from the obstacles for the next physics step. The
    /// wind is a velocity that carries the bear, in m/s.
    #[func]
    pub fn set_wind(&mut self, wind: Vector3) {
        self.wind = wind;
    }

    /// Sets the lift of the air for the next physics step: the speed up
    /// (m/s) that a bear in the air goes to, and how fast it changes
    /// (m/s each second). Use 0 and 0 for no lift.
    #[func]
    pub fn set_lift(&mut self, speed: f32, accel: f32) {
        self.lift = Vector2::new(speed, accel);
    }

    /// Sets the slow down of the ground for the next physics step: 0 is
    /// none, 0.5 is half speed and a lower jump.
    #[func]
    pub fn set_ground_slow(&mut self, slow: f32) {
        self.ground_slow = slow.clamp(0.0, 0.95);
    }

    /// An obstacle hits or throws the bear: the bear gets this velocity.
    /// A bear on a wall lets go. In the air, the bear keeps the speed
    /// until the player gives a direction or the bear lands.
    #[func]
    pub fn knock(&mut self, velocity: Vector3) {
        if let (Mode::Tumble, Some(ragdoll)) = (self.mode, &mut self.ragdoll) {
            let now = ragdoll
                .torso()
                .map_or(Vector3::ZERO, |t| t.get_linear_velocity());
            ragdoll.push(velocity - now);
            return;
        }
        match self.mode {
            Mode::Climb(_) => {
                self.event("let_go:knock");
                self.let_go(velocity);
            }
            Mode::Mantle { .. } => {
                self.mode = Mode::Walk;
                self.base_mut().set_velocity(velocity);
            }
            Mode::Walk => self.base_mut().set_velocity(velocity),
            Mode::Tumble => {}
        }
        self.timers = JumpTimers::default();
        self.thrown = Some(Vector2::new(velocity.x, velocity.z));
        self.regrab_left = self.regrab_time;
    }

    /// Makes the fur wet (1 is fully wet). The fur dries with time.
    #[func]
    pub fn make_wet(&mut self, wet: f32) {
        self.wet = self.wet.max(wet.clamp(0.0, 1.0));
    }

    /// Puts the bear at a place on the ground (a dev tool).
    #[func]
    pub fn go_to(&mut self, position: Vector3) {
        self.end_tumble();
        self.base_mut().set_global_position(position);
        self.base_mut().set_velocity(Vector3::ZERO);
        self.mode = Mode::Walk;
        self.thrown = None;
        self.event("dev:go_to");
    }

    /// Gives the gravity on the bear (m/s each second) and the factor
    /// for a fall. The launch vents use them.
    pub fn gravity(&self) -> (f32, f32) {
        (self.fall_acceleration, self.fall_gravity_factor)
    }

    /// Gives the stamina, from 0 (empty) to 1 (full).
    #[func]
    pub fn stamina_part(&self) -> f32 {
        self.stamina
    }

    /// Tells if the stamina was gone and the bear cannot climb yet.
    #[func]
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// Tells if the bear is on a wall now.
    #[func]
    pub fn is_climbing(&self) -> bool {
        matches!(self.mode, Mode::Climb(_))
    }

    /// Gives one line of text for the debug overlay.
    #[func]
    pub fn debug_text(&self) -> GString {
        let stamina = self.stamina * 100.0;
        let mode = match self.mode {
            Mode::Walk if self.exhausted => format!("walk (tired)   stamina {stamina:.0}%"),
            Mode::Walk => format!("walk   stamina {stamina:.0}%"),
            Mode::Mantle { .. } => format!("onto a ledge   stamina {stamina:.0}%"),
            Mode::Tumble => format!("tumble {:.1} s   stamina {stamina:.0}%", self.settle.time),
            Mode::Climb(c) => format!(
                "climb   slope {:.0} deg   grip {:.2}   slide {:.2} m/s   stamina {stamina:.0}%",
                slope_angle(c.normal).to_degrees(),
                c.grip,
                c.slide_speed
            ),
        };
        let seams: Vec<String> = (0..4)
            .map(|i| format!("{:.0}", self.seams.health(i) * 100.0))
            .collect();
        let text = format!(
            "{mode}   gait {}   limbs {}   seams {}%",
            self.gait_profile().gait.name(),
            self.limbs.label(),
            seams.join("/")
        );
        GString::from(&text)
    }

    /// Gives the state of the bear for the playtest recorder.
    #[func]
    pub fn recorder_state(&self) -> VarDictionary {
        let velocity = self.base().get_velocity();
        let on_floor = self.base().is_on_floor();
        let speed = velocity.length();
        let (state, slope, grip) = match self.mode {
            Mode::Climb(c) => (
                if c.slide_speed > 0.01 {
                    "slide"
                } else {
                    "climb"
                },
                slope_angle(c.normal).to_degrees(),
                c.grip,
            ),
            Mode::Mantle { .. } => ("mantle", 0.0, -1.0),
            Mode::Tumble => ("tumble", 0.0, -1.0),
            Mode::Walk => {
                let state = if !on_floor {
                    if velocity.y > 0.0 {
                        "jump"
                    } else {
                        "fall"
                    }
                } else if speed > (self.walk_speed + self.jog_speed) * 0.5 {
                    "jog"
                } else if speed > 0.05 {
                    "walk"
                } else {
                    "idle"
                };
                let slope = if on_floor {
                    self.base()
                        .get_floor_normal()
                        .angle_to(Vector3::UP)
                        .to_degrees()
                } else {
                    0.0
                };
                (state, slope, -1.0)
            }
        };
        let mut d = VarDictionary::new();
        d.set("pos", self.base().get_global_position());
        d.set("velocity", velocity);
        d.set("speed", speed);
        d.set("state", state);
        d.set("slope_deg", slope);
        // -1: the recorder asks the terrain for the grip below the bear.
        d.set("grip", grip);
        d.set("stamina", self.stamina);
        d.set("exhausted", self.exhausted);
        d.set("limbs", self.limbs.count() as i64);
        let seam_min = (0..4).map(|i| self.seams.health(i)).fold(1.0, f32::min);
        d.set("seam_min", seam_min);
        d.set("gait", self.gait_profile().gait.name());
        let anim_us = self
            .model
            .as_ref()
            .map_or(0.0, |m| m.bind().anim_microseconds());
        d.set("anim_us", anim_us);
        // In a tumble, the stick does nothing: the bear is not stuck.
        let held = self.last_wish.length() > 0.01 && self.mode != Mode::Tumble;
        d.set("stick_held", held);
        d
    }

    /// Starts or stops a second copy of the events for the autoplay bot
    /// (S-133). The recorder keeps its own copy.
    #[func]
    pub fn set_event_tap(&mut self, on: bool) {
        self.tap = on.then(Vec::new);
    }

    /// Gives the events of the second copy since the last call.
    pub fn take_tap_events(&mut self) -> Vec<String> {
        self.tap.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// Gives the events since the last call and forgets them. Each event
    /// is a short name, for example "jump", "grab" or "let_go:jump_off".
    #[func]
    pub fn take_events(&mut self) -> PackedStringArray {
        let events = std::mem::take(&mut self.events);
        events.iter().map(GString::from).collect()
    }
}

impl BfBear {
    /// One step on the ground or in the air.
    fn walk(&mut self, delta: f32, wish: Vector2, jog: bool, jump_pressed: bool) {
        let max_slope = self.max_slope_degrees.to_radians();
        self.base_mut().set_floor_max_angle(max_slope);
        let on_floor = self.base().is_on_floor();

        let mut velocity = self.base().get_velocity();
        // A throw ends when the bear is on the ground and does not go up.
        if on_floor && velocity.y <= 0.01 {
            self.thrown = None;
        }

        // A jog uses stamina. A tired bear walks.
        let jogging = jog && wish != Vector2::ZERO && !self.exhausted && self.stamina > 0.0;
        if jogging && on_floor {
            self.stamina -= delta / self.jog_stamina_seconds.max(0.1);
            if self.stamina <= 0.0 {
                self.stamina = 0.0;
                self.exhausted = true;
                self.event("tired:jog");
            }
        }

        // Speed on the ground plane. Fewer limbs are slower.
        let gait = self.gait_profile();
        let slow = if on_floor {
            1.0 - self.ground_slow
        } else {
            1.0
        };
        let speed = gait.speed_scale
            * slow
            * if jogging {
                self.jog_speed
            } else {
                self.walk_speed
            };
        let mut target = (wish.x * speed, wish.y * speed);
        // The wind carries the bear: it is a part of the speed that the
        // bear goes to. An obstacle that threw the bear gives the speed
        // in the air while the player gives no direction.
        target.0 += self.wind.x;
        target.1 += self.wind.z;
        if let (false, true, Some(thrown)) = (on_floor, wish == Vector2::ZERO, self.thrown) {
            target = (thrown.x, thrown.y);
        }
        let rate = if wish == Vector2::ZERO {
            self.deceleration
        } else {
            self.acceleration
        };
        let control = if on_floor { 1.0 } else { self.air_control };
        // Just after the bear leaves a wall, it keeps its speed in the
        // air if the player gives no direction.
        let keeps_speed = !on_floor && wish == Vector2::ZERO && self.regrab_left > 0.0;
        if !keeps_speed {
            let (vx, vz) = approach((velocity.x, velocity.z), target, rate * control * delta);
            velocity.x = vx;
            velocity.z = vz;
        }

        // Gravity and the jump.
        if on_floor {
            velocity.y = velocity.y.max(0.0);
        } else {
            let factor = if velocity.y < 0.0 {
                self.fall_gravity_factor
            } else {
                1.0
            };
            velocity.y =
                (velocity.y - self.fall_acceleration * factor * delta).max(-self.max_fall_speed);
            // Warm air lifts the bear, up to the speed that the air gives.
            if self.lift.y > 0.0 && self.lift.x > velocity.y {
                velocity.y = (velocity.y + self.lift.y * delta).min(self.lift.x);
            }
        }
        // A bear without legs cannot jump.
        if self.timers.update(
            delta,
            on_floor,
            jump_pressed && gait.can_jump(),
            self.coyote_time,
            self.jump_buffer_time,
        ) {
            let height = self.jump_height * gait.jump_scale * (1.0 - self.ground_slow * 0.8);
            velocity.y = jump_speed(height, self.fall_acceleration);
            self.event("jump");
        }

        self.base_mut().set_velocity(velocity);
        self.base_mut().move_and_slide();
        // A hard hit (a landing, a wall in the air) makes the bear tumble.
        let after = self.base().get_velocity();
        let hit = impact(to_glam(velocity), to_glam(after));
        if hit >= self.tumble_params().impact_speed {
            let what = if self.base().is_on_floor() {
                "land"
            } else {
                "wall"
            };
            self.start_tumble(velocity, what);
            return;
        }
        self.stay_above_ground();

        // Turn the body to the move direction.
        if wish != Vector2::ZERO {
            let wanted = wish.x.atan2(wish.y);
            let step = self.turn_speed_degrees.to_radians() * delta;
            self.yaw += angle_difference(self.yaw, wanted).clamp(-step, step);
        }

        // A landing after a long time in the air is an event.
        if self.base().is_on_floor() {
            if self.air_time > 0.5 {
                let fall = -velocity.y.min(0.0);
                self.event(&format!("land:{:.1}s:{fall:.1}m/s", self.air_time));
            }
            self.air_time = 0.0;
        } else {
            self.air_time += delta;
        }

        // The stamina comes back on the ground, but not during a jog.
        if self.base().is_on_floor() && !jogging {
            let gain = delta / self.stamina_recover_seconds.max(0.01);
            self.stamina = (self.stamina + gain).min(1.0);
            if self.stamina >= self.regrab_stamina_part.clamp(0.0, 1.0) {
                self.exhausted = false;
            }
        }
        self.try_grab(delta, wish);
    }

    /// Starts a climb if the bear pushes into a surface that is too
    /// steep to walk. This uses 2 ray casts at most.
    fn try_grab(&mut self, delta: f32, wish: Vector2) {
        let velocity = self.base().get_velocity();
        let can_grab = wish != Vector2::ZERO
            && self.gait_profile().climb.can_climb
            && !self.exhausted
            && self.regrab_left <= 0.0
            && velocity.length() <= self.max_grab_speed;
        if !can_grab {
            self.push_time = 0.0;
            return;
        }
        let Some(mut surface) = self.surface() else {
            return;
        };
        let p = self.climb_params();
        let centre = to_glam(self.base().get_global_position()) + Vec3::Y * HALF_HEIGHT;
        let push = Vec3::new(wish.x, 0.0, wish.y).normalize_or_zero();
        let Some(hit) = find_grab(&mut surface, centre, push, &p) else {
            self.push_time = 0.0;
            return;
        };
        // In the air, the bear grabs at once.
        self.push_time += delta;
        if self.base().is_on_floor() && self.push_time < self.grab_delay {
            return;
        }
        self.push_time = 0.0;
        let climb = Climb::start(&mut surface, hit, to_glam(velocity), &p);
        self.base_mut().set_velocity(Vector3::ZERO);
        self.set_centre(climb.centre);
        self.mode = Mode::Climb(climb);
        self.was_sliding = false;
        self.event("grab");
    }

    /// One step on a wall.
    fn climb(
        &mut self,
        delta: f32,
        mut climb: Climb,
        wish: Vector2,
        jump_pressed: bool,
        let_go_pressed: bool,
    ) {
        let p = self.climb_params();
        let gait = self.gait_profile();
        // A bear without arms cannot hold a wall.
        if !gait.climb.can_climb {
            self.event("let_go:no_arms");
            self.let_go(to_godot(climb.slide_velocity()));
            return;
        }
        let Some(mut surface) = self.surface() else {
            self.event("let_go:no_physics");
            self.let_go(Vector3::ZERO);
            return;
        };
        // The let-go button (R21): the bear drops with a small push off the wall.
        if let_go_pressed {
            self.event("let_go:button");
            self.let_go(to_godot(climb.normal * 0.3 + climb.slide_velocity()));
            return;
        }
        let input = wall_input(Vec2::new(wish.x, wish.y), climb.normal);
        if jump_pressed {
            if input.y < -self.jump_off_stick.clamp(0.0, 1.0) {
                // The stick points away from the wall: jump off.
                let up = jump_speed(self.climb_jump_height, self.fall_acceleration);
                let velocity = climb.normal * self.climb_jump_push + Vec3::Y * up;
                self.event("let_go:jump_off");
                self.let_go(to_godot(velocity));
                return;
            }
            // Otherwise: a hop up the wall (R20). It costs stamina.
            if !climb.is_hopping() {
                climb.hop(self.climb_hop_height, self.climb_hop_time);
                self.stamina -= self.climb_hop_stamina.max(0.0) * gait.climb.stamina_factor;
                self.event("hop");
            }
        }
        let sliding_now = climb.slide_speed > 0.01;
        if sliding_now != self.was_sliding {
            self.was_sliding = sliding_now;
            self.event(if sliding_now { "slide" } else { "slide_stop" });
        }

        let moving = input.length() > 0.05;
        let sliding = climb.slide_speed > 0.01;
        let factor = gait.climb.stamina_factor;
        self.stamina -= stamina_drain(climb.angle, moving, sliding, &p) * factor * delta;
        if self.stamina <= 0.0 {
            self.stamina = 0.0;
            self.exhausted = true;
            self.event("let_go:no_stamina");
            self.let_go(to_godot(climb.slide_velocity()));
            return;
        }

        match climb.step(&mut surface, input, delta, &p) {
            Step::Hold => {
                self.set_centre(climb.centre);
                self.mode = Mode::Climb(climb);
            }
            Step::Ground { feet } => {
                // The upright bear looks at the wall that it came from.
                self.yaw = (-climb.normal.x).atan2(-climb.normal.z);
                self.event("mantle");
                self.mode = Mode::Mantle {
                    from: self.base().get_global_position(),
                    to: to_godot(feet) + Vector3::new(0.0, 0.02, 0.0),
                    time: 0.0,
                };
            }
            Step::LetGo { velocity } => {
                self.event("let_go:slide_or_edge");
                self.let_go(to_godot(velocity));
                // A fast slide off a wall makes the bear tumble.
                if velocity.length() >= self.tumble_params().let_go_speed {
                    self.start_tumble(to_godot(velocity), "slide");
                }
            }
        }
    }

    /// One step of the move from a wall to a place to stand.
    fn mantle(&mut self, delta: f32, from: Vector3, to: Vector3, time: f32) {
        let time = time + delta;
        let t = (time / self.mantle_time.max(0.01)).min(1.0);
        let ease = t * t * (3.0 - 2.0 * t);
        // A small arc lifts the feet above the edge.
        let lift = (t * std::f32::consts::PI).sin() * 0.04;
        let position = from.lerp(to, ease) + Vector3::new(0.0, lift, 0.0);
        self.base_mut().set_global_position(position);
        self.base_mut().set_velocity(Vector3::ZERO);
        if t >= 1.0 {
            self.timers = JumpTimers::default();
            self.mode = Mode::Walk;
        } else {
            self.mode = Mode::Mantle { from, to, time };
        }
    }

    /// Stops the climb. The bear falls with this velocity.
    fn let_go(&mut self, velocity: Vector3) {
        if let Mode::Climb(climb) = self.mode {
            self.yaw = (-climb.normal.x).atan2(-climb.normal.z);
        }
        self.mode = Mode::Walk;
        self.timers = JumpTimers::default();
        self.regrab_left = self.regrab_time;
        self.push_time = 0.0;
        self.base_mut().set_velocity(velocity);
    }

    /// Gives the tuning values of the tumble from the exported fields.
    fn tumble_params(&self) -> TumbleParams {
        TumbleParams {
            impact_speed: self.tumble_impact_speed.max(1.0),
            let_go_speed: self.tumble_let_go_speed.max(0.5),
            spin_per_speed: self.tumble_spin_per_speed.max(0.0),
            max_spin: self.tumble_max_spin.max(0.0),
            rest_speed: self.tumble_rest_speed.max(0.01),
            rest_spin: self.tumble_rest_spin.max(0.01),
            rest_time: self.tumble_rest_seconds.max(0.0),
            min_time: self.tumble_min_seconds.max(0.1),
            max_time: self.tumble_max_seconds.max(0.5),
        }
    }

    /// Gives the tuning values of the seams from the exported fields.
    fn tear_params(&self) -> TearParams {
        TearParams {
            damage_speed: self.seam_damage_speed.max(0.0),
            damage_per_speed: self.seam_damage_per_speed.max(0.0),
            thread_wear_per_m: self.thread_wear_per_m.max(0.0),
        }
    }

    /// The gravity on the ragdoll in m/s each second: the fall gravity
    /// of the bear.
    fn ragdoll_gravity(&self) -> f32 {
        self.fall_acceleration * self.fall_gravity_factor.max(0.1)
    }

    /// Starts a tumble. The ragdoll starts with this velocity. `cause`
    /// is a short name for the recorder, for example "land".
    fn start_tumble(&mut self, velocity: Vector3, cause: &str) {
        if self.ragdoll.is_some() {
            return;
        }
        let Some(mut model) = self.model.clone() else {
            return;
        };
        let looseness = [0, 1, 2, 3].map(|i| self.seams.looseness(i));
        let parts = model.bind().ragdoll_parts(looseness);
        if parts.is_empty() {
            return;
        }
        let facing = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let spin = start_spin(to_glam(velocity), facing, &self.tumble_params());
        let world_gravity = ProjectSettings::singleton()
            .get_setting("physics/3d/default_gravity")
            .try_to::<f32>()
            .unwrap_or(9.8)
            .max(0.1);
        let scale = self.ragdoll_gravity() / world_gravity;
        let mut owner = self.base().clone().upcast::<PhysicsBody3D>();
        self.ragdoll = Some(Ragdoll::build(
            &mut owner,
            parts,
            velocity,
            to_godot(spin),
            scale,
        ));
        model.set_visible(false);
        self.set_capsule(false);
        self.mode = Mode::Tumble;
        self.settle = Settle::default();
        self.timers = JumpTimers::default();
        self.push_time = 0.0;
        self.event(&format!("tumble:{cause}:{:.1}m/s", velocity.length()));
    }

    /// One step of a tumble: the hits tear the seams, the bear follows
    /// the ragdoll, and it gets up when the ragdoll is at rest.
    fn tumble(&mut self, delta: f32) {
        let gravity = Vector3::new(0.0, -self.ragdoll_gravity(), 0.0);
        let wind = self.wind;
        let drag = self.tumble_wind_drag.max(0.0);
        let Some(ragdoll) = self.ragdoll.as_mut() else {
            self.mode = Mode::Walk;
            return;
        };
        let hits = ragdoll.step(delta, gravity);
        ragdoll.wind(wind, drag);
        let Some(torso) = ragdoll.torso() else {
            self.get_up();
            return;
        };
        let tear = self.tear_params();
        for (limb, speed) in hits {
            let (_, change) = self.seams.hit(limb, speed, &tear);
            if let Some(change) = change {
                self.seam_changed(change);
            }
        }
        // The bear node follows the body, so the camera and the other
        // nodes see where the bear is.
        // The node stays above the ground, so the tests that look for a
        // fall through the ground do not see a ragdoll that lies flat.
        let at = torso.get_global_position();
        let velocity = torso.get_linear_velocity();
        let ground = self.ground_height(at);
        let mut feet = at - Vector3::new(0.0, HALF_HEIGHT, 0.0);
        if let Some(ground) = ground {
            feet.y = feet.y.max(ground);
        }
        self.base_mut().set_global_position(feet);
        self.base_mut().set_velocity(velocity);
        let spin = torso.get_angular_velocity().length();
        let p = self.tumble_params();
        // The body fell through the ground: the same event as for the
        // bear on its feet, so the tests count it.
        let below = ground.filter(|g| at.y < g - 1.0);
        if let Some(g) = below {
            self.event(&format!("back_on_ground:{:.2}m", g - at.y));
        }
        let below = below.is_some();
        if self.settle.update(delta, velocity.length(), spin, &p) || below {
            self.get_up();
        }
    }

    /// Ends a tumble: the bear stands up where the ragdoll lies, and the
    /// pose moves back from the ragdoll to the animation.
    fn get_up(&mut self) {
        let Some(ragdoll) = self.ragdoll.take() else {
            return;
        };
        let world = ragdoll.transforms();
        let torso = ragdoll.torso().map(|t| t.get_global_transform());
        let time = self.settle.time;
        ragdoll.free();
        self.set_capsule(true);
        self.mode = Mode::Walk;
        self.timers = JumpTimers::default();
        self.regrab_left = self.regrab_time;
        self.base_mut().set_velocity(Vector3::ZERO);
        let Some(torso) = torso else {
            self.show_model(None);
            return;
        };
        let b = torso.basis;
        let basis =
            glam::Mat3::from_cols(to_glam(b.col_a()), to_glam(b.col_b()), to_glam(b.col_c()));
        self.yaw = get_up_yaw(basis);
        // The feet go on the ground below the body.
        let at = torso.origin;
        let mut feet = Vector3::new(at.x, at.y - HALF_HEIGHT, at.z);
        if let Some(mut surface) = self.surface() {
            let from = to_glam(at) + Vec3::Y * 0.3;
            if let Some(hit) = surface.cast(from, from - Vec3::Y * 1.5) {
                feet = to_godot(hit.point);
            }
        }
        if let Some(ground) = self.ground_height(at) {
            feet.y = feet.y.max(ground);
        }
        self.base_mut()
            .set_global_position(feet + Vector3::new(0.0, 0.02, 0.0));
        self.last_feet = feet;
        self.show_model(Some(&world));
        self.event(&format!("get_up:{time:.1}s"));
    }

    /// Removes the ragdoll at once, with no move back (a restart).
    fn end_tumble(&mut self) {
        if let Some(ragdoll) = self.ragdoll.take() {
            ragdoll.free();
            self.set_capsule(true);
            self.mode = Mode::Walk;
            self.show_model(None);
        }
    }

    /// Shows the model again after a tumble, turned to `yaw`. With the
    /// ragdoll pose, the model moves from it to the animation.
    fn show_model(&mut self, world: Option<&[(crate::bear_shape::PartId, Transform3D)]>) {
        let Some(mut model) = self.model.clone() else {
            return;
        };
        let basis = Basis::from_quaternion(Quaternion::from_axis_angle(Vector3::UP, self.yaw));
        let pivot = Vector3::new(0.0, HALF_HEIGHT, 0.0);
        model.set_basis(basis);
        model.set_position(pivot - basis * pivot);
        model.set_visible(true);
        let mut m = model.bind_mut();
        m.reset_anim();
        if let Some(world) = world {
            m.blend_from_world(world);
        }
    }

    /// Turns the collision capsule on or off.
    fn set_capsule(&mut self, on: bool) {
        if let Some(shape) = &mut self.shape {
            shape.set_deferred("disabled", &(!on).to_variant());
        }
    }

    /// The height of the terrain below a point, if there is a terrain.
    fn ground_height(&self, at: Vector3) -> Option<f32> {
        let mut terrain = self
            .base()
            .get_tree()
            .get_first_node_in_group(TERRAIN_GROUP)?;
        terrain
            .call("height_at", &[at.x.to_variant(), at.z.to_variant()])
            .try_to::<f32>()
            .ok()
    }

    /// Sets the health of a seam (a dev key, the dev menu, a restart).
    fn set_seam(&mut self, limb: usize, health: f32) {
        if let Some(change) = self.seams.set(limb, health) {
            self.seam_changed(change);
        }
    }

    /// A seam changed its stage: events, stuffing, the look of the
    /// limb, the gait, and a lost limb in the world.
    fn seam_changed(&mut self, change: Change) {
        let name = Seams::limb_name(change.limb);
        self.event(&format!("seam:{name}:{}", change.to.name()));
        let looseness = self.seams.looseness(change.limb);
        if let Some(ragdoll) = &mut self.ragdoll {
            ragdoll.loosen(change.limb, looseness);
        }
        if change.to > change.from {
            let amount = match change.to {
                Stage::Frayed => 6,
                Stage::Torn => 14,
                _ => 24,
            };
            self.puff(change.limb, amount);
        }
        if change.to == Stage::Detached {
            self.drop_limb(change.limb);
            self.event(&format!("limb_lost:{name}"));
        }
        if change.from == Stage::Detached {
            // The limb is on again: it is not in the world any more.
            self.remove_lost_limbs(Some(change.limb));
        }
        self.update_limb_looks();
        self.change_limbs(self.seams.working());
    }

    /// Gives the model the look of each limb from its seam.
    fn update_limb_looks(&mut self) {
        let looks = [0, 1, 2, 3].map(|i| match self.seams.stage(i) {
            Stage::Detached => LimbLook::Gone,
            Stage::Dangling => LimbLook::Dangling,
            _ => LimbLook::On {
                loose: self.seams.looseness(i),
            },
        });
        if let Some(mut model) = self.model.clone() {
            model.bind_mut().set_limb_looks(looks);
        }
    }

    /// Makes a limb that came off into a lost limb in the world.
    fn drop_limb(&mut self, limb: usize) {
        let Some(model) = self.model.clone() else {
            return;
        };
        let Some((def, look, transform)) = model.bind().limb_part(limb) else {
            return;
        };
        let from_ragdoll = self.ragdoll.as_mut().and_then(|r| r.detach(limb));
        let (transform, linear, angular) = from_ragdoll.unwrap_or_else(|| {
            let v = model
                .bind()
                .thread_velocity(limb, 1.0 / 60.0)
                .unwrap_or(self.base().get_velocity());
            (transform, v, Vector3::ZERO)
        });
        let Some(mut parent) = self.base().get_parent() else {
            return;
        };
        let owner = self.base().clone().upcast::<PhysicsBody3D>();
        let spec = LostLimbSpec {
            limb,
            id: LIMBS[limb],
            def,
            looks: vec![look],
            transform,
            linear,
            angular,
            gravity_scale: 1.0,
        };
        let mut lost = lost_limb::spawn(
            &mut parent,
            &owner,
            spec,
            self.max_lost_limbs.max(1) as usize,
        );
        let world_gravity = ProjectSettings::singleton()
            .get_setting("physics/3d/default_gravity")
            .try_to::<f32>()
            .unwrap_or(9.8)
            .max(0.1);
        lost.set_gravity_scale(self.ragdoll_gravity() / world_gravity);
    }

    /// Removes the lost limbs of one limb (or of all limbs) from the world.
    fn remove_lost_limbs(&mut self, limb: Option<usize>) {
        let tree = self.base().get_tree();
        for node in tree.get_nodes_in_group(LOST_LIMB_GROUP).iter_shared() {
            let Ok(mut lost) = node.try_cast::<BfLostLimb>() else {
                continue;
            };
            if limb.is_none_or(|l| lost.bind().limb() == l) {
                lost.remove_from_group(LOST_LIMB_GROUP);
                lost.queue_free();
            }
        }
    }

    /// After a step on the ground or on a wall: the dangling threads
    /// wear with the distance, and the bear sews on a lost limb that it
    /// stands on.
    fn after_move(&mut self, delta: f32) {
        let feet = self.base().get_global_position();
        // A jump to a new place (a restart, a dev tool) is not a move.
        let moved = feet.distance_to(self.last_feet);
        self.last_feet = feet;
        if moved < 1.0 {
            let tear = self.tear_params();
            for change in self.seams.wear(moved, &tear) {
                self.seam_changed(change);
            }
        }
        let lost = (0..4).any(|i| self.seams.stage(i) == Stage::Detached);
        let still =
            self.mode == Mode::Walk && self.last_wish == Vector2::ZERO && self.base().is_on_floor();
        if !(lost && still) {
            self.sew_hold = 0.0;
            return;
        }
        self.sew_wait = self.sew_wait.saturating_sub(1);
        if self.sew_wait == 0 {
            const EVERY: u32 = 6;
            self.sew_wait = EVERY;
            self.try_sew(delta * EVERY as f32);
        }
    }

    /// Sews on a lost limb of the bear that is near and at rest, after
    /// the bear stood still on it for `sew_seconds`. `dt` is the time
    /// since the last call.
    fn try_sew(&mut self, dt: f32) {
        let centre = self.base().get_global_position() + Vector3::new(0.0, 0.1, 0.0);
        let reach = self.sew_distance_m.max(0.0);
        let tree = self.base().get_tree();
        let found = tree
            .get_nodes_in_group(LOST_LIMB_GROUP)
            .iter_shared()
            .filter_map(|n| n.try_cast::<BfLostLimb>().ok())
            .find(|l| {
                let limb = l.bind().limb();
                self.seams.stage(limb) == Stage::Detached
                    && l.bind().is_frozen()
                    && l.get_global_position().distance_to(centre) <= reach
            });
        let Some(lost) = found else {
            self.sew_hold = 0.0;
            return;
        };
        self.sew_hold += dt;
        if self.sew_hold >= self.sew_seconds.max(0.0) {
            self.sew_hold = 0.0;
            let limb = lost.bind().limb();
            self.event(&format!("limb_sewn:{}", Seams::limb_name(limb)));
            if let Some(change) = self.seams.mend(limb) {
                self.seam_changed(change);
            }
            self.puff(limb, 10);
        }
    }

    /// A puff of stuffing at the seam of a limb.
    fn puff(&mut self, limb: usize, amount: i32) {
        let at = match (&self.ragdoll, self.model.as_ref()) {
            (Some(ragdoll), _) => ragdoll.limb_position(limb),
            (None, Some(model)) => model
                .bind()
                .part(LIMBS[limb])
                .map(|n| n.get_global_position()),
            _ => None,
        };
        let (Some(at), Some(stuffing)) = (at, &mut self.stuffing) else {
            return;
        };
        stuffing.set_global_position(at);
        stuffing.set_amount(amount.max(1));
        stuffing.restart();
    }

    /// Makes the particles of the stuffing: small pale flecks that fall.
    fn make_stuffing(&mut self) -> Gd<CpuParticles3D> {
        let mut p = CpuParticles3D::new_alloc();
        p.set_name("Stuffing");
        p.set_as_top_level(true);
        p.set_one_shot(true);
        p.set_emitting(false);
        p.set_amount(12);
        p.set_lifetime(1.2);
        p.set_explosiveness_ratio(0.9);
        p.set_use_local_coordinates(false);
        p.set_direction(Vector3::UP);
        p.set_spread(70.0);
        p.set_gravity(Vector3::new(0.0, -2.5, 0.0));
        p.set_param_min(Parameter::INITIAL_LINEAR_VELOCITY, 0.25);
        p.set_param_max(Parameter::INITIAL_LINEAR_VELOCITY, 0.8);
        p.set_param_min(Parameter::DAMPING, 1.5);
        p.set_param_max(Parameter::DAMPING, 3.0);
        p.set_param_min(Parameter::SCALE, 0.6);
        p.set_param_max(Parameter::SCALE, 1.3);
        let mut quad = QuadMesh::new_gd();
        quad.set_size(Vector2::new(0.012, 0.012));
        let mut material = StandardMaterial3D::new_gd();
        material.set_billboard_mode(BillboardMode::PARTICLES);
        material.set_albedo(Color::from_rgb(0.86, 0.83, 0.76));
        material.set_cull_mode(CullMode::DISABLED);
        quad.set_material(&material.upcast::<Material>());
        p.set_mesh(&quad);
        self.base_mut().add_child(&p);
        p
    }

    /// Dries the fur a little and gives the wet value to the model.
    fn dry(&mut self, delta: f32) {
        self.wet = (self.wet - delta / self.dry_seconds.max(0.1)).max(0.0);
        if (self.wet - self.wet_shown).abs() < 0.02 && (self.wet > 0.0 || self.wet_shown == 0.0) {
            return;
        }
        self.wet_shown = self.wet;
        if let Some(model) = &mut self.model {
            let mut model = model.bind_mut();
            for part in crate::bear_shape::PartId::ALL {
                model.set_part_wet(GString::from(part.name()), self.wet);
            }
        }
    }

    /// Keeps an event for the playtest recorder. At most 64 events wait;
    /// without a recorder, the oldest go.
    fn event(&mut self, name: &str) {
        if self.events.len() >= 64 {
            self.events.remove(0);
        }
        self.events.push(name.to_string());
        if let Some(tap) = &mut self.tap {
            if tap.len() < 256 {
                tap.push(name.to_string());
            }
        }
    }

    /// Puts the centre of the bear at a position.
    fn set_centre(&mut self, centre: Vec3) {
        let feet = to_godot(centre) - Vector3::new(0.0, HALF_HEIGHT, 0.0);
        self.base_mut().set_global_position(feet);
    }

    /// Gives the physics world as a climbing surface.
    fn surface(&mut self) -> Option<RaySurface> {
        let space = self.base().get_world_3d()?.get_direct_space_state()?;
        if self.ray_query.is_none() {
            let mut query = PhysicsRayQueryParameters3D::new_gd();
            let mut exclude = Array::<Rid>::new();
            exclude.push(self.base().get_rid());
            query.set_exclude(&exclude);
            // A ray that starts below the ground must not hit it.
            query.set_hit_back_faces(false);
            // The rays do not see the parts of the bear (the ragdoll and
            // the lost limbs).
            query.set_collision_mask(!BEAR_PARTS_LAYER);
            self.ray_query = Some(query);
        }
        Some(RaySurface {
            space,
            query: self.ray_query.clone()?,
            terrain: self
                .base()
                .get_tree()
                .get_first_node_in_group(TERRAIN_GROUP),
        })
    }

    /// Gives what the limbs of the bear can do (the gait table).
    fn gait_profile(&self) -> GaitProfile {
        let tuning = GaitTuning {
            armless_walk_speed: self.armless_walk_speed_scale.clamp(0.05, 1.0),
            hop_speed: self.hop_speed_scale.clamp(0.05, 1.0),
            hop_arm_slowdown: self.hop_arm_slowdown.clamp(0.0, 0.45),
            hop_jump: self.hop_jump_scale.clamp(0.0, 1.0),
            crawl_speed: self.crawl_speed_scale.clamp(0.02, 1.0),
            drag_speed: self.drag_speed_scale.clamp(0.02, 1.0),
            wriggle_speed: self.wriggle_speed_scale.clamp(0.02, 1.0),
            one_arm_max_degrees: self.one_arm_climb_degrees,
            one_arm_stamina: self.one_arm_stamina_factor.max(0.1),
            one_leg_stamina: self.one_leg_stamina_factor.max(0.1),
            no_legs_stamina: self.no_legs_stamina_factor.max(0.1),
        };
        profile(self.limbs, &tuning)
    }

    /// Changes the limbs that work. The recorder gets an event for each
    /// change.
    fn change_limbs(&mut self, limbs: LimbSet) {
        if limbs == self.limbs {
            return;
        }
        for (i, limb) in LIMBS.iter().enumerate() {
            if limbs.has(i) != self.limbs.has(i) {
                let what = if limbs.has(i) { "limb_on" } else { "limb_off" };
                self.event(&format!("{what}:{}", limb.name()));
            }
        }
        self.limbs = limbs;
        self.event(&format!("gait:{}", self.gait_profile().gait.name()));
    }

    /// Gives the tuning values of the climb from the exported fields.
    fn climb_params(&self) -> ClimbParams {
        let start = self.max_slope_degrees;
        // With one arm, the bear holds only less steep surfaces.
        let max = match self.gait_profile().climb.max_degrees {
            Some(limit) => self.climb_max_degrees.min(limit),
            None => self.climb_max_degrees,
        };
        ClimbParams {
            start_angle: start.to_radians(),
            stop_angle: (start - self.climb_stop_margin_degrees.max(0.0)).to_radians(),
            max_angle: max.max(start).to_radians(),
            radius: RADIUS,
            half_height: HALF_HEIGHT,
            reach: self.climb_reach.max(0.02),
            climb_speed: self.climb_speed,
            gravity: self.fall_acceleration,
            firm_grip: self.firm_grip,
            loose_grip: self.loose_grip.min(self.firm_grip),
            slow_slide_speed: self.slow_slide_speed,
            let_go_slide_speed: self.let_go_slide_speed,
            drain_per_second: 1.0 / self.stamina_seconds.max(0.1),
            gentle_drain_factor: self.gentle_drain_factor.clamp(0.0, 1.0),
            ..ClimbParams::default()
        }
    }

    /// Turns the model and poses its parts. On the ground, the model
    /// stands and looks along `yaw`. On a wall, the belly is on the wall
    /// and the head is up. The animation (`anim.rs`) poses the parts.
    fn pose_body(&mut self, delta: f32) {
        let Some(mut body) = self.body.clone() else {
            return;
        };
        let (target, rate, mode) = match self.mode {
            Mode::Climb(climb) => {
                let (_, up) = wall_axes(climb.normal);
                let (up, forward) = (to_godot(up), to_godot(-climb.normal));
                let basis = Basis::from_cols(up.cross(forward), up, forward);
                (
                    basis.get_quaternion(),
                    self.climb_turn_rate,
                    AnimMode::Climb {
                        travel: climb.travel,
                    },
                )
            }
            Mode::Mantle { .. } => (
                Quaternion::from_axis_angle(Vector3::UP, self.yaw),
                self.climb_turn_rate * 2.5,
                AnimMode::Mantle,
            ),
            Mode::Walk | Mode::Tumble => (
                Quaternion::from_axis_angle(Vector3::UP, self.yaw),
                self.climb_turn_rate * 2.5,
                AnimMode::Ground,
            ),
        };
        let now = body.get_basis().get_quaternion();
        let k = 1.0 - (-rate.max(0.0) * delta).exp();
        let turn = now.slerp(target, k).normalized();
        let basis = Basis::from_quaternion(turn);
        // The body turns around the centre of the bear, not around the feet.
        let pivot = Vector3::new(0.0, HALF_HEIGHT, 0.0);
        let place = pivot - basis * pivot;
        body.set_basis(basis);
        body.set_position(place);

        let Some(mut model) = self.model.clone() else {
            return;
        };
        let Some(mut ground) = self.surface() else {
            return;
        };
        let on_floor = self.mode == Mode::Walk && self.base().is_on_floor();
        let floor_normal = if on_floor {
            to_glam(self.base().get_floor_normal())
        } else {
            Vec3::Y
        };
        let input = AnimInput {
            dt: delta,
            model: glam::Affine3A::from_rotation_translation(
                glam::Quat::from_xyzw(turn.x, turn.y, turn.z, turn.w),
                to_glam(self.base().get_global_position() + place),
            ),
            on_floor,
            floor_normal,
            mode,
            limbs: self.limbs,
            profile: self.gait_profile(),
        };
        model.bind_mut().animate(&input, &mut ground);
        let feet_y = self.base().get_global_position().y;
        let floor_y = if on_floor { feet_y } else { feet_y - 1.0 };
        model.bind_mut().after_pose(delta, floor_y);
    }

    /// Reads the keys and the gamepad.
    fn read_input(&self) -> Controls {
        let tree = self.base().get_tree();
        let Some(camera) = tree.get_first_node_in_group(FOLLOW_CAMERA_GROUP) else {
            return Controls::default();
        };
        let Ok(camera) = camera.try_cast::<godot::classes::Camera3D>() else {
            return Controls::default();
        };
        // While a debug camera or the dev menu is active, the bear does not move.
        if !camera.is_current() || crate::dev_menu::is_open() {
            return Controls::default();
        }

        let input = Input::singleton();
        let key = |k: Key| input.is_physical_key_pressed(k);
        let axis = |negative: bool, positive: bool| (positive as i32 - negative as i32) as f32;
        let mut stick = Vector2::new(
            axis(
                key(Key::A) || key(Key::LEFT),
                key(Key::D) || key(Key::RIGHT),
            ),
            axis(key(Key::W) || key(Key::UP), key(Key::S) || key(Key::DOWN)),
        );
        let pad = Vector2::new(
            input.get_joy_axis(0, JoyAxis::LEFT_X),
            input.get_joy_axis(0, JoyAxis::LEFT_Y),
        );
        if pad.length() > self.stick_dead_zone {
            stick += pad;
        }
        if stick.length() > 1.0 {
            stick = stick.normalized();
        }

        // "Up" on the stick is "away from the camera".
        let yaw = camera.get_global_rotation().y;
        let (sin, cos) = yaw.sin_cos();
        let wish = Vector2::new(
            stick.x * cos + stick.y * sin,
            -stick.x * sin + stick.y * cos,
        );

        let jog = key(Key::SHIFT) || input.is_joy_button_pressed(0, JoyButton::RIGHT_SHOULDER);
        let jump = key(Key::SPACE) || input.is_joy_button_pressed(0, JoyButton::A);
        let let_go = key(Key::C) || input.is_joy_button_pressed(0, JoyButton::B);
        Controls {
            wish,
            jog,
            jump,
            let_go,
        }
    }

    /// If the bear is below the ground, puts it back on the ground.
    /// This can occur before Terrain3D makes the collision of an area.
    fn stay_above_ground(&mut self) {
        let tree = self.base().get_tree();
        let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) else {
            return;
        };
        let mut position = self.base().get_global_position();
        let ground = terrain
            .call(
                "height_at",
                &[position.x.to_variant(), position.z.to_variant()],
            )
            .to::<f32>();
        if position.y < ground - 0.25 {
            self.event(&format!("back_on_ground:{:.2}m", ground - position.y));
            position.y = ground + 0.05;
            self.base_mut().set_global_position(position);
            let mut velocity = self.base().get_velocity();
            velocity.y = 0.0;
            self.base_mut().set_velocity(velocity);
        }
    }

    /// Makes the collision shape and the visible bear. The origin of
    /// the bear is between its feet.
    fn build_body(&mut self) {
        // The collision shape: a capsule, 0.40 m tall.
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(RADIUS);
        capsule.set_height(HALF_HEIGHT * 2.0);
        let mut shape = CollisionShape3D::new_alloc();
        shape.set_shape(&capsule);
        shape.set_position(Vector3::new(0.0, HALF_HEIGHT, 0.0));
        self.base_mut().add_child(&shape);
        self.shape = Some(shape);
        self.stuffing = Some(self.make_stuffing());

        // The model makes its parts when it enters the tree.
        let mut model = BfBearModel::new_alloc();
        model.set_name("Model");
        self.base_mut().add_child(&model);
        self.body = Some(model.clone().upcast());
        self.model = Some(model);
    }
}

/// Finds a limb by its name, for example "LeftArm".
fn limb_index(name: &GString) -> Option<usize> {
    let name = name.to_string();
    let i = LIMBS.iter().position(|l| l.name() == name);
    if i.is_none() {
        godot_warn!("BfBear: no limb with the name {name}");
    }
    i
}
