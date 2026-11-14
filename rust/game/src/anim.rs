//! The procedural animation of the bear (R18), with no Godot types.
//! Unit tests cover this module.
//!
//! The bear has no skeleton and no animation clips. In each physics
//! step, [`BearAnim::update`] reads where the bear is and gives a
//! [`Pose`]: one transform for each part. The movement code stays in
//! control. The animation only follows it.
//!
//! The pose is a sum of layers, as in the talk "Animation Bootcamp: An
//! Indie Approach" (David Rosen, GDC 2014). The code is our own.
//! - **The stride wheel.** The distance that the bear moves turns a
//!   wheel. The place on the wheel (the phase) tells each leg when to
//!   stand and when to swing. A fast bear makes longer strides, and its
//!   feet are on the ground for a shorter part of the stride.
//! - **Planted feet.** A foot that stands stays at one place in the
//!   world. A ray finds the ground for each step. A foot slips on low
//!   grip and sinks into soft ground.
//! - **Aim IK.** A teddy bear has no knees or elbows. Each limb turns at
//!   its joint toward its target and stretches a little. IK (inverse
//!   kinematics) finds the turn of a limb from the place of its end.
//! - **The body.** It bounces with the steps, tilts with acceleration,
//!   leans with the slope and squashes on a landing.
//! - **Springs.** The head, the ears, the hump and the paws follow
//!   their targets through springs ([`SecondOrder`]), so the bear is
//!   soft and floppy.
//!
//! Model space: the origin is between the feet, Y is up, the bear looks
//! along +Z and its left side is +X.

// The limbs and the feet are small arrays with fixed places. A loop
// over the index is the clear way to read them.
#![allow(clippy::needless_range_loop)]

use crate::bear_shape::PartId;
use crate::climb::{Hit, Surface};
use crate::spring::SecondOrder;
use glam::{Affine3A, Mat3, Quat, Vec2, Vec3};
use std::f32::consts::{PI, TAU};

/// The arms and the legs, in the order of the limb arrays.
pub const LIMBS: [PartId; 4] = [
    PartId::LeftArm,
    PartId::RightArm,
    PartId::LeftLeg,
    PartId::RightLeg,
];

/// The side of each limb: +1 is left (+X), -1 is right.
const SIDE: [f32; 4] = [1.0, -1.0, 1.0, -1.0];

/// If the bear moves more than this distance in one step, it is a jump
/// to a new place (for example a restart). The animation starts again.
const TELEPORT_M: f32 = 1.0;

/// The bear is "on the ground" for this time after it leaves the floor.
/// A bump in the ground must not start the air pose.
const AIR_DELAY_S: f32 = 0.08;

/// The ground below the feet: a [`Surface`] that also knows how soft
/// it is.
pub trait Ground: Surface {
    /// Gives the softness at a hit: 0 is firm, 1 is loose sand.
    fn softness(&mut self, hit: &Hit) -> f32;
}

/// The fixed points of the bear that the animation needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rig {
    /// The joint of the body in model space. The body node sits there.
    pub body_joint: Vec3,
    /// The point between the hips, relative to the body joint. The body
    /// tilts around it.
    pub hip_pivot: Vec3,
    /// The joint of each limb, relative to the body joint.
    pub limb_joint: [Vec3; 4],
    /// The end of each limb (paw or sole), relative to its joint.
    pub limb_tip: [Vec3; 4],
    /// The centre of the hump, relative to the body joint.
    pub hump_centre: Vec3,
    /// The radius of the part of the body that moves with the hump.
    pub hump_radius: f32,
}

impl Rig {
    /// Makes a rig from the joint and the tip of each part, in model
    /// space. The tip of the body is the centre of its hump.
    pub fn from_points(points: impl IntoIterator<Item = (PartId, Vec3, Vec3)>) -> Option<Rig> {
        let points: Vec<_> = points.into_iter().collect();
        let find = |id: PartId| points.iter().find(|p| p.0 == id).map(|p| (p.1, p.2));
        let (body_joint, hump) = find(PartId::Body)?;
        let mut limb_joint = [Vec3::ZERO; 4];
        let mut limb_tip = [Vec3::ZERO; 4];
        for (i, id) in LIMBS.into_iter().enumerate() {
            let (joint, tip) = find(id)?;
            limb_joint[i] = joint - body_joint;
            limb_tip[i] = tip - joint;
        }
        let hip_y = 0.5 * (limb_joint[2].y + limb_joint[3].y);
        Some(Rig {
            body_joint,
            hip_pivot: Vec3::new(0.0, hip_y, 0.0),
            limb_joint,
            limb_tip,
            hump_centre: hump - body_joint,
            hump_radius: 0.34 * body_joint.y,
        })
    }

    /// Gives the place below a foot (0 left, 1 right) when the bear
    /// stands, in model space.
    fn foot_home(&self, foot: usize) -> Vec3 {
        let home = self.body_joint + self.limb_joint[2 + foot] + self.limb_tip[2 + foot];
        Vec3::new(home.x, 0.0, home.z)
    }

    /// Gives the height of the hips above the ground when the bear stands.
    fn hip_height(&self) -> f32 {
        self.body_joint.y + self.hip_pivot.y
    }
}

/// The tuning values of the animation. The model copies them from its
/// exported fields. Lengths are in metres, angles in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimParams {
    /// The length of one full stride (two steps) of a bear that stands.
    pub stride_base: f32,
    /// The stride gets this much longer for each m/s of speed.
    pub stride_per_speed: f32,
    /// How far the body moves while one foot stands.
    pub stance_length: f32,
    /// The height of a step.
    pub step_height: f32,
    /// The time of a step when the bear stands and moves a foot back
    /// below its hip, in seconds.
    pub idle_step_time: f32,
    /// A foot of a bear that stands steps when it is this far from the
    /// place below its hip.
    pub idle_step_distance: f32,
    /// The distance from the middle of the sole to the toes. A foot that
    /// is not below the hip stands on its toes or its heel.
    pub foot_half_length: f32,
    /// How far the body moves up and down with the steps.
    pub bob_height: f32,
    /// How far the body rolls toward the foot that stands.
    pub waddle: f32,
    /// How far the hips turn with the legs.
    pub twist: f32,
    /// The forward lean for each m/s of speed.
    pub speed_lean: f32,
    /// The tilt for each m/s² of acceleration.
    pub accel_tilt: f32,
    /// The forward lean of a bear that stands.
    pub idle_slump: f32,
    /// How far a bear that stands sways.
    pub idle_sway: f32,
    /// The part of the slope that the body leans with (0 to 1).
    pub slope_lean: f32,
    /// The frequency of the tilt spring, in Hz.
    pub tilt_frequency: f32,
    /// The damping of the tilt spring.
    pub tilt_damping: f32,
    /// The squash speed for each m/s of landing speed.
    pub land_squash: f32,
    /// The stretch speed at the start of a jump.
    pub jump_stretch: f32,
    /// The frequency of the squash spring, in Hz.
    pub squash_frequency: f32,
    /// The damping of the squash spring.
    pub squash_damping: f32,
    /// How deep a foot sinks into loose sand.
    pub sink_depth: f32,
    /// The part of the body movement that a foot with no grip slides.
    pub slip_amount: f32,
    /// With this grip or more, a foot does not slip.
    pub firm_grip: f32,
    /// How far the arms swing at the walk speed.
    pub arm_swing: f32,
    /// The frequency of the paw springs, in Hz.
    pub arm_frequency: f32,
    /// The damping of the paw springs.
    pub arm_damping: f32,
    /// How far a paw stays behind for each m/s² of acceleration.
    pub arm_inertia: f32,
    /// The part of the body tilt that the head removes, to stay level.
    pub head_level: f32,
    /// How far the head turns into a turn, for each rad/s.
    pub head_turn: f32,
    /// The frequency of the head spring, in Hz.
    pub head_frequency: f32,
    /// The damping of the head spring.
    pub head_damping: f32,
    /// How far an ear bends for each m/s² of acceleration.
    pub ear_flop: f32,
    /// The frequency of the ear springs, in Hz.
    pub ear_frequency: f32,
    /// The damping of the ear springs.
    pub ear_damping: f32,
    /// How far the hump moves for each m/s² of acceleration.
    pub hump_wobble: f32,
    /// The frequency of the hump spring, in Hz.
    pub hump_frequency: f32,
    /// The damping of the hump spring.
    pub hump_damping: f32,
}

impl Default for AnimParams {
    fn default() -> AnimParams {
        AnimParams {
            stride_base: 0.13,
            stride_per_speed: 0.165,
            stance_length: 0.11,
            step_height: 0.03,
            idle_step_time: 0.14,
            idle_step_distance: 0.025,
            foot_half_length: 0.035,
            bob_height: 0.008,
            waddle: 0.07,
            twist: 0.05,
            speed_lean: 0.035,
            accel_tilt: 0.006,
            idle_slump: 0.03,
            idle_sway: 0.012,
            slope_lean: 0.4,
            tilt_frequency: 2.2,
            tilt_damping: 0.45,
            land_squash: 0.9,
            jump_stretch: 1.5,
            squash_frequency: 3.5,
            squash_damping: 0.35,
            sink_depth: 0.015,
            slip_amount: 0.6,
            firm_grip: 0.7,
            arm_swing: 0.45,
            arm_frequency: 3.5,
            arm_damping: 0.5,
            arm_inertia: 0.0015,
            head_level: 0.6,
            head_turn: 0.04,
            head_frequency: 3.0,
            head_damping: 0.5,
            ear_flop: 0.012,
            ear_frequency: 5.5,
            ear_damping: 0.25,
            hump_wobble: 0.0004,
            hump_frequency: 4.5,
            hump_damping: 0.3,
        }
    }
}

/// What the movement code does with the bear now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimMode {
    /// The bear walks, jumps or falls.
    Ground,
    /// The bear is on a wall. `travel` is the distance that it climbed.
    Climb { travel: f32 },
    /// The bear moves from a wall onto a ledge.
    Mantle,
}

/// What the animation reads in each step.
#[derive(Clone, Copy, Debug)]
pub struct AnimInput {
    /// The time of the step in seconds.
    pub dt: f32,
    /// The place and the turn of the model in the world (no scale).
    pub model: Affine3A,
    /// The bear stands on a floor.
    pub on_floor: bool,
    /// The normal of that floor, in the world.
    pub floor_normal: Vec3,
    pub mode: AnimMode,
}

/// The transforms of the parts for one step.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    /// The body, relative to the model.
    pub body: Affine3A,
    /// The turn of the head at the neck.
    pub head: Quat,
    /// The turn of the left and the right ear at its base.
    pub ears: [Quat; 2],
    /// Each limb, relative to the body, in the order of [`LIMBS`].
    pub limbs: [Affine3A; 4],
    /// How far the hump moves, in the space of the body.
    pub hump: Vec3,
}

/// What a foot does now.
#[derive(Clone, Copy, Debug, PartialEq)]
enum FootState {
    /// The foot stands at one place in the world.
    Planted,
    /// The foot moves to its next place. `from` is the start of the
    /// step, relative to the place below the hip. It moves with the
    /// bear, so the leg does not stay far behind a fast bear. `u` goes
    /// from 0 to 1.
    Swing { from: Vec3, u: f32 },
}

#[derive(Clone, Copy, Debug)]
struct Foot {
    /// The place of the sole in the world.
    pos: Vec3,
    state: FootState,
    /// The grip of the ground below the foot (0 to 1).
    grip: f32,
    /// The softness of the ground below the foot (0 to 1).
    soft: f32,
}

/// The animation state of one bear.
#[derive(Clone, Debug)]
pub struct BearAnim {
    started: bool,
    time: f32,
    last_origin: Vec3,
    velocity: Vec3,
    /// The smoothed acceleration in the world.
    acceleration: Vec3,
    last_heading: f32,
    turn_rate: f32,
    /// The place on the stride wheel, from 0 to 1.
    phase: f32,
    /// The part of the stride in which a foot stands.
    duty: f32,
    /// 1 when the bear moves on the ground, 0 when it stands.
    moving: f32,
    feet: [Foot; 2],
    /// The time since the bear left the floor.
    air_time: f32,
    grounded: bool,
    /// The largest fall speed since the bear left the floor.
    fall_speed: f32,
    /// The part of the leg pose that comes from the planted feet.
    leg_weight: f32,
    /// The part of the arm pose that comes from the paw targets.
    arm_weight: f32,
    /// The free turn of each limb, relative to the body. A limb with no
    /// target uses it (in the air, on a wall).
    free: [Quat; 4],
    /// The smoothed slip of the feet (0 to 1).
    slip: f32,
    paws: [SecondOrder<Vec3>; 2],
    tilt: SecondOrder<Vec2>,
    height: SecondOrder<f32>,
    squash: SecondOrder<f32>,
    normal: SecondOrder<Vec3>,
    head: SecondOrder<Vec3>,
    ears: [SecondOrder<Vec2>; 2],
    hump: SecondOrder<Vec3>,
}

impl Default for BearAnim {
    fn default() -> BearAnim {
        let p = AnimParams::default();
        let foot = Foot {
            pos: Vec3::ZERO,
            state: FootState::Planted,
            grip: 1.0,
            soft: 0.0,
        };
        BearAnim {
            started: false,
            time: 0.0,
            last_origin: Vec3::ZERO,
            velocity: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            last_heading: 0.0,
            turn_rate: 0.0,
            phase: 0.0,
            duty: 0.6,
            moving: 0.0,
            feet: [foot; 2],
            air_time: 0.0,
            grounded: false,
            fall_speed: 0.0,
            leg_weight: 0.0,
            arm_weight: 1.0,
            free: [Quat::IDENTITY; 4],
            slip: 0.0,
            paws: [SecondOrder::new(p.arm_frequency, p.arm_damping, 0.0, Vec3::ZERO); 2],
            tilt: SecondOrder::new(p.tilt_frequency, p.tilt_damping, 0.0, Vec2::ZERO),
            height: SecondOrder::new(4.0, 0.8, 0.0, 0.0),
            squash: SecondOrder::new(p.squash_frequency, p.squash_damping, 0.0, 0.0),
            normal: SecondOrder::new(2.5, 0.9, 0.0, Vec3::Y),
            head: SecondOrder::new(p.head_frequency, p.head_damping, 0.0, Vec3::ZERO),
            ears: [SecondOrder::new(p.ear_frequency, p.ear_damping, 0.0, Vec2::ZERO); 2],
            hump: SecondOrder::new(p.hump_frequency, p.hump_damping, 0.0, Vec3::ZERO),
        }
    }
}

/// Gives the length of one full stride at a speed.
pub fn stride_length(speed: f32, slip: f32, p: &AnimParams) -> f32 {
    let length = p.stride_base + p.stride_per_speed * speed.max(0.0);
    // On slippery ground, the bear makes shorter, faster steps.
    (length * (1.0 - 0.4 * slip.clamp(0.0, 1.0))).max(0.02)
}

/// Gives the part of the stride in which a foot stands. Above 0.5 is a
/// walk: one foot is always on the ground. Below 0.5 is a run.
pub fn duty_factor(stride: f32, p: &AnimParams) -> f32 {
    (p.stance_length / stride.max(0.02)).clamp(0.2, 0.65)
}

/// Turns a limb at its joint toward a target. Gives the turn and the
/// stretch (1 is the rest length). `rest` is the rest direction from the
/// joint to the end of the limb, with its length.
pub fn aim_limb(rest: Vec3, to_target: Vec3, min_stretch: f32, max_stretch: f32) -> (Quat, f32) {
    let length = rest.length();
    let distance = to_target.length();
    if length < 1e-6 || distance < 1e-6 {
        return (Quat::IDENTITY, 1.0);
    }
    let turn = Quat::from_rotation_arc(rest / length, to_target / distance);
    (turn, (distance / length).clamp(min_stretch, max_stretch))
}

/// A scale of `stretch` along the unit direction `dir` only.
fn stretch_along(dir: Vec3, stretch: f32) -> Mat3 {
    Mat3::IDENTITY + Mat3::from_cols(dir * dir.x, dir * dir.y, dir * dir.z) * (stretch - 1.0)
}

fn smooth(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// The part of the way to a target for one step of exponential smoothing.
fn blend(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

impl BearAnim {
    /// Makes the pose for one physics step.
    pub fn update(
        &mut self,
        rig: &Rig,
        p: &AnimParams,
        input: &AnimInput,
        ground: &mut dyn Ground,
    ) -> Pose {
        let dt = input.dt.clamp(1e-4, 0.1);
        let origin = Vec3::from(input.model.translation);
        let turn = Quat::from_mat3a(&input.model.matrix3).normalize();
        let to_model = turn.inverse();
        let heading = {
            let forward = turn * Vec3::Z;
            forward.x.atan2(forward.z)
        };
        if !self.started || origin.distance(self.last_origin) > TELEPORT_M {
            self.restart(rig, origin, turn, heading);
        }
        self.time = (self.time + dt) % 4096.0;
        self.tune(p);

        // The speed and the acceleration come from the movement of the
        // model. This works in all modes, also on a wall.
        let step = origin - self.last_origin;
        let velocity = step / dt;
        let acceleration = ((velocity - self.velocity) / dt).clamp_length_max(60.0);
        self.acceleration = self.acceleration.lerp(acceleration, blend(25.0, dt));
        self.velocity = velocity;
        self.last_origin = origin;
        let accel_m = to_model * self.acceleration;
        let velocity_m = to_model * velocity;
        let speed = flat(velocity).length();

        let on_ground_mode = input.mode == AnimMode::Ground;
        let turn_now = crate::motion::angle_difference(self.last_heading, heading) / dt;
        self.last_heading = heading;
        let turn_target = if on_ground_mode { turn_now } else { 0.0 };
        self.turn_rate += (turn_target - self.turn_rate) * blend(15.0, dt);

        // On the floor or in the air.
        if input.on_floor {
            self.air_time = 0.0;
        } else {
            self.air_time += dt;
            self.fall_speed = self.fall_speed.max(-velocity.y);
        }
        let grounded = on_ground_mode && self.air_time < AIR_DELAY_S;
        if grounded && !self.grounded {
            // A landing: the body squashes, and the feet find the ground.
            if self.started {
                self.squash
                    .kick((self.fall_speed * p.land_squash).clamp(0.0, 6.0));
            }
            for foot in 0..2 {
                let home = origin + turn * rig.foot_home(foot);
                self.plant(foot, home, origin.y, p, ground);
            }
        }
        if !grounded && self.grounded && on_ground_mode && velocity.y > 1.0 {
            // The start of a jump: the body stretches.
            self.squash.kick(-p.jump_stretch);
        }
        if grounded {
            self.fall_speed = 0.0;
        }
        self.grounded = grounded;
        self.started = true;

        // The stride wheel turns with the distance on the ground.
        let stride = stride_length(speed, self.slip, p);
        self.duty = duty_factor(stride, p);
        let phase_step = if grounded {
            flat(step).length() / stride
        } else {
            0.0
        };
        self.phase = (self.phase + phase_step).fract();
        let moving_target = if grounded {
            (speed / 0.5).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.moving += (moving_target - self.moving) * blend(10.0, dt);

        if grounded {
            self.step_feet(rig, p, origin, turn, step, speed, phase_step, dt, ground);
        }
        let leg_target = if grounded { 1.0 } else { 0.0 };
        self.leg_weight += (leg_target - self.leg_weight) * blend(18.0, dt);
        let arm_target = if on_ground_mode { 1.0 } else { 0.0 };
        self.arm_weight += (arm_target - self.arm_weight) * blend(12.0, dt);

        // The body.
        let gait = self.moving * self.leg_weight;
        let stand = (1.0 - self.moving) * self.leg_weight;
        let running = ((0.5 - self.duty) / 0.25).clamp(0.0, 1.0);
        let mid_stance = self.phase - 0.5 * self.duty;
        let speed_part = (speed / 2.0).min(1.0);
        // A walk is highest in the middle of a stance. A run is lowest there.
        let bob = gait
            * p.bob_height
            * (0.5 + 0.5 * speed_part)
            * (1.0 - 2.0 * running)
            * (2.0 * TAU * mid_stance).cos();

        let mut lowest_foot = 0.0_f32;
        let mut soft = 0.0;
        for foot in &self.feet {
            if foot.state == FootState::Planted {
                lowest_foot = lowest_foot.min(foot.pos.y - origin.y);
            }
            soft += 0.5 * foot.soft;
        }
        let height_target = if grounded {
            0.6 * lowest_foot.max(-0.06) - soft * p.sink_depth
        } else {
            0.0
        };
        let height = self.height.update(dt, height_target) + bob;
        let squash = self.squash.update(dt, 0.0).clamp(-0.2, 0.35);

        let tilt_target = match input.mode {
            AnimMode::Ground if grounded => Vec2::new(
                p.idle_slump * (1.0 - self.moving)
                    + p.speed_lean * velocity_m.z
                    + p.accel_tilt * accel_m.z,
                -p.accel_tilt * accel_m.x,
            ),
            // In the air, the bear leans back on the way up and forward
            // on the way down.
            AnimMode::Ground => Vec2::new((-velocity.y * 0.04).clamp(-0.2, 0.3), 0.0),
            _ => Vec2::ZERO,
        };
        let tilt = self
            .tilt
            .update(dt, tilt_target.clamp_length_max(0.6))
            .clamp_length_max(0.8);
        let pitch = tilt.x + stand * p.idle_sway * 0.6 * (TAU * 0.17 * self.time + 1.0).sin();
        let roll = tilt.y - gait * p.waddle * (TAU * mid_stance).cos()
            + stand * p.idle_sway * (TAU * 0.23 * self.time).sin();
        let twist = -gait * p.twist * (TAU * self.phase).cos();

        let normal_target = if grounded {
            (to_model * input.floor_normal).normalize_or(Vec3::Y)
        } else {
            Vec3::Y
        };
        let normal = self.normal.update(dt, normal_target).normalize_or(Vec3::Y);
        let lean_to = Vec3::Y
            .lerp(normal, p.slope_lean.clamp(0.0, 1.0))
            .normalize_or(Vec3::Y);
        let lean = Quat::from_rotation_arc(Vec3::Y, lean_to);
        let body_turn = (lean
            * Quat::from_rotation_y(twist)
            * Quat::from_rotation_x(pitch)
            * Quat::from_rotation_z(roll))
        .normalize();
        // A squash makes the body shorter and wider. The hips come down
        // by the same part, so the feet stay on the ground.
        let scale = Vec3::new(1.0 + 0.5 * squash, 1.0 - squash, 1.0 + 0.5 * squash);
        let body_place = rig.body_joint + Vec3::Y * (height - squash * rig.hip_height());
        let body = Affine3A::from_translation(body_place + rig.hip_pivot)
            * Affine3A::from_mat3(Mat3::from_quat(body_turn) * Mat3::from_diagonal(scale))
            * Affine3A::from_translation(-rig.hip_pivot);
        let body_inverse = body.inverse();

        // The limbs.
        self.free_limbs(input.mode, velocity, dt);
        let mut limbs = [Affine3A::IDENTITY; 4];
        let mut legs_pass = [0.0; 2];
        for foot in 0..2 {
            let p_leg = (self.phase + 0.5 * foot as f32).fract();
            // 1 when the leg is in front, -1 when it is behind. The leg
            // goes back while it stands and forward while it swings.
            legs_pass[foot] = if p_leg < self.duty {
                (PI * p_leg / self.duty).cos()
            } else {
                -(PI * (p_leg - self.duty) / (1.0 - self.duty)).cos()
            };
        }
        for i in 0..4 {
            let joint = body.transform_point3(rig.limb_joint[i]);
            let rest = rig.limb_tip[i];
            let rest_dir = rest.normalize_or(-Vec3::Y);
            let (weight, target, limits) = if i < 2 {
                let target = self.paw_target(
                    i, rig, p, joint, normal, accel_m, velocity, gait, speed_part, legs_pass, dt,
                );
                (self.arm_weight, target, (0.9, 1.1))
            } else {
                let foot = &self.feet[i - 2];
                let mut target = to_model * (foot.pos - origin);
                // A foot that is not below the hip stands on its toes or
                // its heel. This lifts the middle of the sole.
                let away = ((target.z - joint.z).abs() / rest.length().max(1e-3)).min(1.0);
                target.y += p.foot_half_length * away;
                (self.leg_weight, target, (0.72, 1.12))
            };
            let (aim, stretch) = aim_limb(rest, target - joint, limits.0, limits.1);
            let aim_local = (body_turn.inverse() * aim).normalize();
            if weight > 0.999 {
                self.free[i] = aim_local;
            }
            let local = self.free[i].slerp(aim_local, weight).normalize();
            let stretch = 1.0 + (stretch - 1.0) * weight;
            let in_model = Affine3A::from_mat3_translation(
                Mat3::from_quat(body_turn * local) * stretch_along(rest_dir, stretch),
                joint,
            );
            limbs[i] = body_inverse * in_model;
        }

        // The head stays a little level, looks into a turn, and looks
        // down while the bear falls.
        let head_target = Vec3::new(
            (-p.head_level * tilt.x - 0.04 * velocity_m.y * (1.0 - self.leg_weight))
                .clamp(-0.5, 0.5),
            (self.turn_rate * p.head_turn).clamp(-0.5, 0.5),
            -p.head_level * roll,
        );
        let h = self.head.update(dt, head_target).clamp_length_max(1.0);
        let head =
            Quat::from_rotation_y(h.y) * Quat::from_rotation_x(h.x) * Quat::from_rotation_z(h.z);

        // The ears and the hump stay behind when the bear accelerates.
        let mut ears = [Quat::IDENTITY; 2];
        for (i, ear) in ears.iter_mut().enumerate() {
            let side = SIDE[i];
            let target = Vec2::new(
                -p.ear_flop * accel_m.z,
                p.ear_flop * (accel_m.x - side * accel_m.y),
            );
            let e = self.ears[i].update(dt, target).clamp_length_max(0.7);
            *ear = Quat::from_rotation_x(e.x) * Quat::from_rotation_z(e.y);
        }
        let hump = self
            .hump
            .update(dt, -accel_m * p.hump_wobble)
            .clamp_length_max(0.3 * rig.hump_radius);

        Pose {
            body,
            head,
            ears,
            limbs,
            hump,
        }
    }

    /// Starts the animation again at a new place.
    fn restart(&mut self, rig: &Rig, origin: Vec3, turn: Quat, heading: f32) {
        let phase = self.phase;
        *self = BearAnim::default();
        self.phase = phase;
        self.last_origin = origin;
        self.last_heading = heading;
        for foot in 0..2 {
            self.feet[foot].pos = origin + turn * rig.foot_home(foot);
        }
        for arm in 0..2 {
            let paw = rig.body_joint + rig.limb_joint[arm] + rig.limb_tip[arm];
            self.paws[arm].reset(paw);
        }
    }

    /// Gives the exported spring values to the springs.
    fn tune(&mut self, p: &AnimParams) {
        self.tilt.tune(p.tilt_frequency, p.tilt_damping, 0.0);
        self.squash.tune(p.squash_frequency, p.squash_damping, 0.0);
        self.head.tune(p.head_frequency, p.head_damping, 0.0);
        self.hump.tune(p.hump_frequency, p.hump_damping, 0.0);
        for (i, paw) in self.paws.iter_mut().enumerate() {
            paw.tune(p.arm_frequency, p.arm_damping, 0.0);
            // The ears are not the same, so they do not move as one.
            self.ears[i].tune(
                p.ear_frequency * (1.0 + 0.11 * i as f32),
                p.ear_damping,
                0.0,
            );
        }
    }

    /// Puts a foot on the ground below `at`. This uses one ray cast.
    fn plant(&mut self, foot: usize, at: Vec3, floor_y: f32, p: &AnimParams, g: &mut dyn Ground) {
        let (point, grip, soft) = probe(g, at, floor_y, true);
        self.feet[foot] = Foot {
            pos: point - Vec3::Y * p.sink_depth * soft,
            state: FootState::Planted,
            grip,
            soft,
        };
    }

    /// Moves the feet of a bear on the ground: a planted foot stays, and
    /// a foot in a swing moves to its next place.
    #[allow(clippy::too_many_arguments)]
    fn step_feet(
        &mut self,
        rig: &Rig,
        p: &AnimParams,
        origin: Vec3,
        turn: Quat,
        step: Vec3,
        speed: f32,
        phase_step: f32,
        dt: f32,
        ground: &mut dyn Ground,
    ) {
        let walks = speed > 0.15;
        let dir = if walks {
            flat(step).normalize_or_zero()
        } else {
            Vec3::ZERO
        };
        // In the middle of a stance, the foot is below the hip. So a
        // step lands half a stance in front of the hip.
        let stance = self.duty * stride_length(speed, self.slip, p);
        let ahead = dir * (0.5 * stance * self.moving);
        let mut slip_sum = 0.0;
        for i in 0..2 {
            let other_planted = self.feet[1 - i].state == FootState::Planted;
            let home = origin + turn * rig.foot_home(i);
            let target = home + ahead;
            let p_leg = (self.phase + 0.5 * i as f32).fract();
            let foot = &mut self.feet[i];
            let slip = (1.0 - foot.grip / p.firm_grip.max(0.01)).clamp(0.0, 1.0)
                * p.slip_amount.clamp(0.0, 1.0);
            slip_sum += 0.5 * slip / p.slip_amount.clamp(0.01, 1.0);
            match foot.state {
                FootState::Planted => {
                    // On low grip, the foot slides with the body.
                    foot.pos += flat(step) * slip;
                    let offset = flat(foot.pos - home);
                    let behind = offset.dot(dir) < 0.15 * stance;
                    // In a walk, one foot is always on the ground.
                    let free = other_planted || self.duty < 0.5;
                    let by_wheel = walks && p_leg >= self.duty && behind && free;
                    let too_far = offset.length() > stance + 0.03;
                    let by_idle = !walks && other_planted && offset.length() > p.idle_step_distance;
                    if by_wheel || too_far || by_idle {
                        // A step that starts late does not start at 0. Thus
                        // it ends at the correct place on the wheel.
                        let late = (p_leg - self.duty) / (1.0 - self.duty).max(0.1);
                        let u = if by_wheel { late.clamp(0.0, 0.5) } else { 0.0 };
                        foot.state = FootState::Swing {
                            from: foot.pos - home,
                            u,
                        };
                    }
                }
                FootState::Swing { from, u } => {
                    // The wheel moves a step. A step also has a longest time,
                    // so a foot does not hang in the air when the bear stops.
                    let by_time = dt
                        / if walks {
                            0.22
                        } else {
                            p.idle_step_time.max(0.02)
                        };
                    let by_wheel = phase_step / (1.0 - self.duty).max(0.1);
                    let u = u + by_time.max(by_wheel);
                    let done = u >= 1.0;
                    let (point, grip, soft) = probe(ground, target, origin.y, done);
                    if done {
                        *foot = Foot {
                            pos: point - Vec3::Y * p.sink_depth * soft,
                            state: FootState::Planted,
                            grip,
                            soft,
                        };
                    } else {
                        // Soft ground needs higher steps.
                        let lift =
                            p.step_height * (0.5 + 0.5 * self.moving) + p.sink_depth * foot.soft;
                        foot.pos =
                            (home + from).lerp(point, smooth(u)) + Vec3::Y * lift * (PI * u).sin();
                        foot.state = FootState::Swing { from, u };
                    }
                }
            }
        }
        self.slip += (slip_sum - self.slip) * blend(6.0, dt);
    }

    /// Moves the free turn of each limb toward the pose of the mode.
    fn free_limbs(&mut self, mode: AnimMode, velocity: Vec3, dt: f32) {
        let k = blend(12.0, dt);
        // -1 on the way up, +1 in a fast fall.
        let fall = (-velocity.y / 3.0).clamp(-1.0, 1.0);
        let kick = (self.air_time / 0.6).min(1.0);
        for i in 0..4 {
            let target = match mode {
                AnimMode::Climb { travel } => {
                    // The arms reach up. The limbs move with the distance.
                    let phase = travel * TAU / 0.24 + i as f32 * 2.2;
                    let reach = if i < 2 { -2.6 } else { -0.2 };
                    Quat::from_rotation_x(reach + 0.3 * phase.sin())
                }
                AnimMode::Mantle => Quat::IDENTITY,
                AnimMode::Ground => {
                    // In the air, the legs stay behind on the way up and
                    // come forward and kick on the way down.
                    let swing = 0.3 * (self.time * 13.0 + i as f32 * PI).sin() * kick;
                    Quat::from_rotation_z(SIDE[i] * 0.12)
                        * Quat::from_rotation_x(0.1 - 0.4 * fall + swing * fall.max(0.0))
                }
            };
            self.free[i] = self.free[i].slerp(target, k).normalize();
        }
    }

    /// Gives the place that a paw goes to, in model space. The arm hangs
    /// from the shoulder and swings against the leg of its side. A
    /// spring makes the paw soft.
    #[allow(clippy::too_many_arguments)]
    fn paw_target(
        &mut self,
        arm: usize,
        rig: &Rig,
        p: &AnimParams,
        shoulder: Vec3,
        normal: Vec3,
        accel_m: Vec3,
        velocity: Vec3,
        gait: f32,
        speed_part: f32,
        legs_pass: [f32; 2],
        dt: f32,
    ) -> Vec3 {
        let rest = rig.limb_tip[arm];
        if self.arm_weight < 0.02 {
            // The arm has a free pose now (on a wall). Keep the spring
            // at the paw, so the arm does not jump when the pose ends.
            let paw = shoulder + self.free[arm] * rest;
            self.paws[arm].reset(paw);
            return paw;
        }
        let side = SIDE[arm];
        // In the air, the arms go up and out. In a long fall, they wave.
        let mut up = 0.0;
        let mut out = 0.0;
        if self.leg_weight < 0.5 {
            let fall = (-velocity.y / 3.0).clamp(-1.0, 1.0);
            let wave = 0.35 * (self.time * 15.0 + arm as f32 * PI).sin();
            let air = 1.0 - self.leg_weight;
            up = air * (0.9 + 0.9 * fall.max(0.0) + wave * (self.air_time / 0.6).min(1.0));
            out = air * 0.5;
        }
        let hang = Quat::from_rotation_z(side * out) * Quat::from_rotation_x(-up) * rest;
        let inertia = (-accel_m * p.arm_inertia).clamp_length_max(0.5 * rest.length());
        let soft = self.paws[arm].update(dt, shoulder + hang + inertia);
        // The left arm goes forward with the right leg. The spring is
        // too slow for the steps, so the swing comes after the spring.
        let swing = -gait * p.arm_swing * (0.6 + 0.6 * speed_part) * legs_pass[1 - arm];
        let swing =
            Quat::from_rotation_z(side * 0.1 * gait * speed_part) * Quat::from_rotation_x(swing);
        let mut paw = shoulder + swing * (soft - shoulder);
        // The paw stays above the ground.
        let above = paw.dot(normal);
        if self.grounded && above < 0.02 {
            paw += normal * (0.02 - above);
        }
        paw
    }

    /// Gives the place on the stride wheel, from 0 to 1.
    #[allow(dead_code)] // For the tests and for S-122.
    pub fn phase(&self) -> f32 {
        self.phase
    }

    /// Gives the place of the sole of a foot (0 left, 1 right) in the
    /// world, and tells if the foot stands.
    #[allow(dead_code)] // For the tests and for S-122.
    pub fn foot(&self, foot: usize) -> (Vec3, bool) {
        let f = &self.feet[foot.min(1)];
        (f.pos, f.state == FootState::Planted)
    }

    /// Tells if the legs follow the planted feet now.
    #[allow(dead_code)] // For the tests and for S-122.
    pub fn is_grounded(&self) -> bool {
        self.grounded
    }
}

/// Finds the ground below `at` with one ray. Gives the point, the grip
/// and the softness. Without a hit, the point is at the height
/// `floor_y`. The grip and the softness are read only if `detail` is
/// set.
fn probe(ground: &mut dyn Ground, at: Vec3, floor_y: f32, detail: bool) -> (Vec3, f32, f32) {
    let from = Vec3::new(at.x, floor_y + 0.2, at.z);
    let to = Vec3::new(at.x, floor_y - 0.3, at.z);
    match ground.cast(from, to) {
        Some(hit) if detail => (hit.point, ground.grip(&hit), ground.softness(&hit)),
        Some(hit) => (hit.point, 1.0, 0.0),
        None => (Vec3::new(at.x, floor_y, at.z), 1.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bear_shape::{define, Recipe};

    const DT: f32 = 1.0 / 60.0;

    /// A ground plane that rises along +Z, with one grip and softness.
    struct Plane {
        rise: f32,
        grip: f32,
        soft: f32,
        casts: usize,
    }

    impl Plane {
        fn flat() -> Plane {
            Plane {
                rise: 0.0,
                grip: 1.0,
                soft: 0.0,
                casts: 0,
            }
        }

        fn height(&self, z: f32) -> f32 {
            self.rise * z
        }

        fn normal(&self) -> Vec3 {
            Vec3::new(0.0, 1.0, -self.rise).normalize()
        }
    }

    impl Surface for Plane {
        fn cast(&mut self, from: Vec3, to: Vec3) -> Option<Hit> {
            self.casts += 1;
            let y = self.height(from.z);
            (from.y >= y && to.y <= y).then(|| Hit {
                point: Vec3::new(from.x, y, from.z),
                normal: self.normal(),
                id: 0,
            })
        }

        fn grip(&mut self, _hit: &Hit) -> f32 {
            self.grip
        }
    }

    impl Ground for Plane {
        fn softness(&mut self, _hit: &Hit) -> f32 {
            self.soft
        }
    }

    fn rig() -> Rig {
        Rig::from_points(
            define(&Recipe::default())
                .iter()
                .map(|d| (d.id, d.joint, d.tip)),
        )
        .expect("the bear has a body and four limbs")
    }

    /// A bear that the test moves along +Z on a plane.
    struct Walker {
        rig: Rig,
        p: AnimParams,
        anim: BearAnim,
        ground: Plane,
        z: f32,
        y: f32,
        pose: Pose,
    }

    impl Walker {
        fn new(ground: Plane) -> Walker {
            let mut w = Walker {
                rig: rig(),
                p: AnimParams::default(),
                anim: BearAnim::default(),
                ground,
                z: 0.0,
                y: 0.0,
                pose: Pose {
                    body: Affine3A::IDENTITY,
                    head: Quat::IDENTITY,
                    ears: [Quat::IDENTITY; 2],
                    limbs: [Affine3A::IDENTITY; 4],
                    hump: Vec3::ZERO,
                },
            };
            w.step(0.0, true);
            w
        }

        fn model(&self) -> Affine3A {
            Affine3A::from_translation(Vec3::new(0.0, self.y, self.z))
        }

        /// One step at a speed along +Z. On the floor, the bear follows
        /// the plane.
        fn step(&mut self, speed: f32, on_floor: bool) {
            self.z += speed * DT;
            if on_floor {
                self.y = self.ground.height(self.z);
            }
            let input = AnimInput {
                dt: DT,
                model: self.model(),
                on_floor,
                floor_normal: self.ground.normal(),
                mode: AnimMode::Ground,
            };
            self.pose = self
                .anim
                .update(&self.rig, &self.p, &input, &mut self.ground);
            self.check();
        }

        fn run(&mut self, speed: f32, seconds: f32) {
            for _ in 0..(seconds / DT) as usize {
                self.step(speed, true);
            }
        }

        /// Gives the end of a limb in the world, from the pose.
        fn tip(&self, limb: usize) -> Vec3 {
            (self.model() * self.pose.body * self.pose.limbs[limb])
                .transform_point3(self.rig.limb_tip[limb])
        }

        fn check(&self) {
            let pose = &self.pose;
            assert!(pose.body.is_finite() && pose.head.is_finite() && pose.hump.is_finite());
            assert!(pose.limbs.iter().all(|l| l.is_finite()));
            assert!(pose.ears.iter().all(|e| e.is_finite()));
        }
    }

    #[test]
    fn the_rig_has_the_size_of_the_bear() {
        let rig = rig();
        assert!((rig.hip_height() - 0.144).abs() < 0.01);
        assert!(rig.limb_joint[0].x > 0.0 && rig.limb_joint[1].x < 0.0);
        // The soles are on the ground, and the paws are below the hips.
        for foot in 0..2 {
            let sole = rig.body_joint + rig.limb_joint[2 + foot] + rig.limb_tip[2 + foot];
            assert!(sole.y.abs() < 0.005, "sole {sole}");
            let paw = rig.body_joint + rig.limb_joint[foot] + rig.limb_tip[foot];
            assert!(paw.y < rig.hip_height() && paw.y > 0.03, "paw {paw}");
        }
    }

    #[test]
    fn a_bear_that_stands_has_its_rest_pose() {
        let mut w = Walker::new(Plane::flat());
        w.run(0.0, 3.0);
        for foot in 0..2 {
            let home = w.rig.foot_home(foot);
            assert!(w.tip(2 + foot).distance(home) < 0.004, "foot {foot}");
            let rest = w.rig.body_joint + w.rig.limb_joint[foot] + w.rig.limb_tip[foot];
            assert!(w.tip(foot).distance(rest) < 0.02, "paw {foot}");
        }
        let lift = w.pose.body.translation.y - w.rig.body_joint.y;
        assert!(lift.abs() < 0.004, "lift {lift}");
        // A bear that stands uses no ray casts.
        let casts = w.ground.casts;
        w.run(0.0, 1.0);
        assert_eq!(w.ground.casts, casts);
    }

    #[test]
    fn slow_is_a_walk_and_fast_is_a_run() {
        let p = AnimParams::default();
        let slow = duty_factor(stride_length(0.4, 0.0, &p), &p);
        let walk = duty_factor(stride_length(1.4, 0.0, &p), &p);
        let jog = duty_factor(stride_length(3.2, 0.0, &p), &p);
        assert!(slow > 0.5, "slow {slow}");
        assert!(walk < slow && jog < walk, "walk {walk}, jog {jog}");
        assert!(stride_length(3.2, 0.0, &p) > stride_length(1.4, 0.0, &p));
        // Slippery ground makes the strides shorter.
        assert!(stride_length(1.4, 1.0, &p) < stride_length(1.4, 0.0, &p));
    }

    #[test]
    fn planted_feet_stay_at_their_place_and_the_legs_take_turns() {
        for speed in [0.5, 1.4, 3.2] {
            let mut w = Walker::new(Plane::flat());
            let mut last = [w.anim.foot(0), w.anim.foot(1)];
            let mut steps = [0; 2];
            let mut both_in_the_air = 0;
            let frames = (4.0 / DT) as usize;
            for _ in 0..frames {
                w.step(speed, true);
                let now = [w.anim.foot(0), w.anim.foot(1)];
                for foot in 0..2 {
                    if now[foot].1 && last[foot].1 {
                        let moved = now[foot].0.distance(last[foot].0);
                        assert!(moved < 1e-5, "a planted foot moved {moved} m");
                    }
                    if now[foot].1 && !last[foot].1 {
                        steps[foot] += 1;
                    }
                }
                if !now[0].1 && !now[1].1 {
                    both_in_the_air += 1;
                }
                last = now;
            }
            let stride = stride_length(speed, 0.0, &w.p);
            let expected = (speed * 4.0 / stride) as i32;
            for count in steps {
                assert!(
                    (count - expected).abs() <= 2,
                    "speed {speed}: {count} steps, expected {expected}"
                );
            }
            if speed < 0.6 {
                assert_eq!(both_in_the_air, 0, "a walk has a foot on the ground");
            } else {
                assert!(both_in_the_air > frames / 10, "a run has a flight part");
            }
        }
    }

    #[test]
    fn the_legs_reach_their_feet() {
        let mut w = Walker::new(Plane::flat());
        w.run(1.4, 1.0);
        for _ in 0..120 {
            w.step(1.4, true);
            for foot in 0..2 {
                let (pos, planted) = w.anim.foot(foot);
                if planted {
                    // The sole is at the foot, or a little above it
                    // when the foot stands on its toes.
                    let tip = w.tip(2 + foot);
                    assert!(flat(tip - pos).length() < 0.012, "tip {tip}, foot {pos}");
                    assert!((tip.y - pos.y).abs() < 0.03, "tip {tip}, foot {pos}");
                }
            }
        }
    }

    #[test]
    fn the_feet_follow_a_slope_and_the_body_leans() {
        let mut w = Walker::new(Plane {
            rise: 0.5,
            ..Plane::flat()
        });
        w.run(1.0, 3.0);
        for _ in 0..60 {
            w.step(1.0, true);
            for foot in 0..2 {
                let (pos, planted) = w.anim.foot(foot);
                if planted {
                    assert!((pos.y - w.ground.height(pos.z)).abs() < 1e-4);
                }
            }
        }
        // The top of the body leans away from the hill, with the normal.
        let up = w.pose.body.matrix3 * Vec3::Y;
        let flat_walk = {
            let mut f = Walker::new(Plane::flat());
            f.run(1.0, 4.0);
            f.pose.body.matrix3 * Vec3::Y
        };
        assert!(up.z < flat_walk.z - 0.05, "up {up}, flat {flat_walk}");
    }

    #[test]
    fn the_bear_sinks_into_sand() {
        let mut w = Walker::new(Plane {
            soft: 1.0,
            ..Plane::flat()
        });
        w.run(1.0, 2.0);
        w.run(0.0, 2.0);
        for foot in 0..2 {
            let (pos, planted) = w.anim.foot(foot);
            assert!(planted);
            assert!((pos.y + w.p.sink_depth).abs() < 1e-4, "foot at {}", pos.y);
        }
        let lift = w.pose.body.translation.y - w.rig.body_joint.y;
        assert!(lift < -0.5 * w.p.sink_depth, "lift {lift}");
    }

    #[test]
    fn the_feet_slip_on_low_grip() {
        let mut w = Walker::new(Plane {
            grip: 0.1,
            ..Plane::flat()
        });
        w.run(1.4, 1.0);
        let mut slid = 0.0;
        let mut last = [w.anim.foot(0), w.anim.foot(1)];
        for _ in 0..120 {
            w.step(1.4, true);
            for foot in 0..2 {
                let now = w.anim.foot(foot);
                if now.1 && last[foot].1 {
                    slid += now.0.z - last[foot].0.z;
                }
                last[foot] = now;
            }
        }
        assert!(slid > 0.1, "the planted feet slid {slid} m");
    }

    #[test]
    fn the_feet_go_back_below_the_hips_after_a_stop() {
        let mut w = Walker::new(Plane::flat());
        w.run(3.2, 2.0);
        w.run(0.0, 1.5);
        for foot in 0..2 {
            let (pos, planted) = w.anim.foot(foot);
            let home = w.model().transform_point3(w.rig.foot_home(foot));
            assert!(planted, "foot {foot} is in the air");
            let limit = w.p.idle_step_distance + 1e-3;
            assert!(pos.distance(home) <= limit, "foot {foot} at {pos}");
        }
    }

    #[test]
    fn a_landing_squashes_the_body_and_it_comes_back() {
        let mut w = Walker::new(Plane::flat());
        w.run(0.0, 0.5);
        // A fall of 0.5 s from 1 m.
        w.y = 1.0;
        let mut speed_y = 0.0;
        while w.y > 0.0 {
            speed_y -= 14.0 * DT;
            w.y = (w.y + speed_y * DT).max(0.0);
            w.step(0.0, false);
        }
        assert!(!w.anim.is_grounded());
        let mut shortest = 1.0_f32;
        for _ in 0..30 {
            w.step(0.0, true);
            shortest = shortest.min((w.pose.body.matrix3 * Vec3::Y).length());
        }
        assert!(
            shortest < 0.92,
            "the body is {shortest} tall at the landing"
        );
        w.run(0.0, 3.0);
        let tall = (w.pose.body.matrix3 * Vec3::Y).length();
        assert!((tall - 1.0).abs() < 0.01, "the body is {tall} tall at rest");
    }

    #[test]
    fn a_start_tilts_the_body_and_moves_the_ears_and_the_hump() {
        let mut w = Walker::new(Plane::flat());
        w.run(0.0, 1.0);
        let mut most_pitch = 0.0_f32;
        let mut most_hump = 0.0_f32;
        let mut most_ear = 0.0_f32;
        for _ in 0..40 {
            w.step(3.2, true);
            let up = w.pose.body.matrix3 * Vec3::Y;
            most_pitch = most_pitch.max(up.z);
            // The hump stays behind (-Z).
            most_hump = most_hump.max(-w.pose.hump.z);
            most_ear = most_ear.max(w.pose.ears[0].angle_between(Quat::IDENTITY));
        }
        assert!(most_pitch > 0.08, "pitch {most_pitch}");
        assert!(most_hump > 0.002, "hump {most_hump}");
        assert!(most_ear > 0.05, "ear {most_ear}");
    }

    #[test]
    fn the_arms_swing_against_the_legs() {
        let mut w = Walker::new(Plane::flat());
        w.run(1.4, 2.0);
        let mut sum = 0.0;
        let mut swing = 0.0_f32;
        for _ in 0..240 {
            w.step(1.4, true);
            let to_model = w.model().inverse();
            let left_paw = to_model.transform_point3(w.tip(0)).z;
            let left_foot = to_model.transform_point3(w.tip(2)).z;
            let right_foot = to_model.transform_point3(w.tip(3)).z;
            // The left paw is in front when the right foot is in front.
            sum += (left_paw - 0.03) * (right_foot - left_foot);
            swing = swing.max((left_paw - 0.03).abs());
        }
        assert!(swing > 0.02, "the paw swings {swing} m");
        assert!(sum > 0.0, "the left arm swings with the left leg ({sum})");
    }

    #[test]
    fn a_climb_gives_the_arms_the_wall_pose_and_a_restart_is_safe() {
        let mut w = Walker::new(Plane::flat());
        w.run(1.4, 1.0);
        for i in 0..90 {
            let input = AnimInput {
                dt: DT,
                model: w.model(),
                on_floor: false,
                floor_normal: Vec3::Y,
                mode: AnimMode::Climb {
                    travel: i as f32 * 0.01,
                },
            };
            w.pose = w.anim.update(&w.rig, &w.p, &input, &mut w.ground);
            w.check();
        }
        // The paws are above the shoulders.
        let shoulder = w.rig.body_joint + w.rig.limb_joint[0];
        assert!(w.tip(0).y > shoulder.y + 0.05, "paw at {}", w.tip(0));
        // A jump to a far place.
        w.z += 500.0;
        w.run(1.4, 1.0);
        let (pos, _) = w.anim.foot(0);
        assert!((pos.z - w.z).abs() < 0.3);
    }

    #[test]
    fn aim_limb_turns_and_stretches() {
        let rest = Vec3::new(0.0, -0.1, 0.0);
        let (turn, stretch) = aim_limb(rest, Vec3::new(0.0, 0.0, 0.105), 0.9, 1.1);
        assert!((turn * rest).normalize().distance(Vec3::Z) < 1e-5);
        assert!((stretch - 1.05).abs() < 1e-5);
        let (_, stretch) = aim_limb(rest, Vec3::new(0.0, -0.5, 0.0), 0.9, 1.1);
        assert_eq!(stretch, 1.1);
        let m = stretch_along(Vec3::Y, 1.1) * Vec3::new(1.0, 1.0, 1.0);
        assert!(m.distance(Vec3::new(1.0, 1.1, 1.0)) < 1e-6);
    }
}
