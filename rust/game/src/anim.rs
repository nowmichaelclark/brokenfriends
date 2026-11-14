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
//! - **Gaits.** The limbs that the bear has set the gait (`limbs.rs`).
//!   With one leg, the bear hops. Without legs, it lies on its belly and
//!   pulls with its arms, drags itself with one arm, or wriggles.
//! - **Holds on a wall.** Each hand and foot holds one place on the
//!   wall. When the body moves too far from it, the limb steps to a new
//!   place that a fan of rays finds. The hands step in turn, and the
//!   body hangs from them.
//!
//! Model space: the origin is between the feet, Y is up, the bear looks
//! along +Z and its left side is +X.

// The limbs and the feet are small arrays with fixed places. A loop
// over the index is the clear way to read them.
#![allow(clippy::needless_range_loop)]

use crate::bear_shape::PartId;
use crate::climb::{Hit, Surface};
use crate::limbs::{Gait, GaitProfile, LimbSet};
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
    /// How high the body goes in a hop on one leg.
    pub hop_height: f32,
    /// The part of a hop in which the foot stands.
    pub hop_duty: f32,
    /// How far the body leans over the leg in a hop.
    pub hop_lean: f32,
    /// How far the arms go out for balance in a hop.
    pub hop_arms_out: f32,
    /// How far a bear without legs tips forward onto its belly.
    pub lie_angle: f32,
    /// The height of the hips above the ground when the bear lies.
    pub lie_height: f32,
    /// The part of the belly tilt that the head removes, to look ahead.
    pub lie_head: f32,
    /// In a crawl, a paw lands at this part of the arm length from the
    /// shoulder, in front of it.
    pub crawl_reach: f32,
    /// How far the body rolls along its length in a crawl or a drag.
    pub drag_roll: f32,
    /// How far the body turns from side to side in a wriggle.
    pub wriggle_yaw: f32,
    /// How far the body rolls along its length in a wriggle.
    pub wriggle_roll: f32,
    /// On a wall, a hand reaches up this part of the arm length.
    pub hand_reach: f32,
    /// On a wall, a hand or a foot steps when it is this far from its place.
    pub hold_step: f32,
    /// The time of a step of a hand on a wall, in seconds.
    pub hand_step_time: f32,
    /// The time of a step of a foot on a wall, in seconds.
    pub foot_step_time: f32,
    /// The distance between the rays of the fan of a hand.
    pub fan_spread: f32,
    /// How far a hand or a foot comes off the wall in a step.
    pub hold_lift: f32,
    /// The part of the hand movement that the body follows on a wall.
    pub hang_follow: f32,
    /// How far the body hangs below the hands, and more for each hand that
    /// is in a step.
    pub hang_sag: f32,
    /// The roll of the body on a wall for each metre between the heights
    /// of the hands.
    pub hang_roll: f32,
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
            hop_height: 0.035,
            hop_duty: 0.45,
            hop_lean: 6f32.to_radians(),
            hop_arms_out: 34f32.to_radians(),
            lie_angle: 77f32.to_radians(),
            lie_height: 0.065,
            lie_head: 0.75,
            crawl_reach: 0.9,
            drag_roll: 9f32.to_radians(),
            wriggle_yaw: 14f32.to_radians(),
            wriggle_roll: 11f32.to_radians(),
            hand_reach: 0.7,
            hold_step: 0.055,
            hand_step_time: 0.16,
            foot_step_time: 0.14,
            fan_spread: 0.035,
            hold_lift: 0.03,
            hang_follow: 0.4,
            hang_sag: 0.01,
            hang_roll: 3.0,
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
    /// The limbs that the bear has.
    pub limbs: LimbSet,
    /// What the limbs can do (the cell of the gait table).
    pub profile: GaitProfile,
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

/// What a hand or a foot on a wall does now.
#[derive(Clone, Copy, Debug, PartialEq)]
enum HoldState {
    /// No ray found a place to hold. The limb reaches to the wall, and
    /// tries again after a short time.
    Free { retry: f32 },
    /// The limb holds one place in the world.
    Held,
    /// The limb moves to a new place `to` in the world. `from` is the
    /// start, relative to the home of the limb. `u` goes from 0 to 1.
    Reach { from: Vec3, to: Vec3, u: f32 },
}

/// A hand or a foot on a wall.
#[derive(Clone, Copy, Debug)]
struct Hold {
    /// The place of the paw or the sole in the world.
    pos: Vec3,
    state: HoldState,
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
    /// The places where the bear stands: the feet, or the paws in a
    /// crawl. [`contact_limb`] gives the limb of each.
    feet: [Foot; 2],
    /// The gait of the last step.
    gait: Gait,
    /// 1 when the bear lies on its belly, 0 when it stands.
    lie: f32,
    /// The hands and the feet on a wall, in the order of [`LIMBS`].
    holds: [Hold; 4],
    /// 1 when the limbs follow their holds on a wall.
    climb_weight: f32,
    /// The bear was on a wall in the last step.
    was_climbing: bool,
    /// The number of ray casts of the holds, for the tests.
    hold_casts: usize,
    /// How far the body hangs from the hands, in model space.
    hang: SecondOrder<Vec3>,
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
            gait: Gait::Walk,
            lie: 0.0,
            holds: [Hold {
                pos: Vec3::ZERO,
                state: HoldState::Free { retry: 0.0 },
            }; 4],
            climb_weight: 0.0,
            was_climbing: false,
            hold_casts: 0,
            hang: SecondOrder::new(3.0, 0.6, 0.0, Vec3::ZERO),
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

/// The distance from the centre of the bear to the wall in model space,
/// while it climbs (the hold distance of `climb.rs`).
const WALL_Z: f32 = 0.12;

/// After a fan of rays finds no hold, a limb tries again after this time.
const HOLD_RETRY_S: f32 = 0.15;

/// Gives the limb (an index of [`LIMBS`]) that stands on the ground at
/// contact `c` (0 or 1) in a gait. `None`: the gait has no contact
/// there.
pub fn contact_limb(gait: Gait, c: usize) -> Option<usize> {
    match gait {
        Gait::Walk => Some(2 + c),
        Gait::Hop { leg } => (c == leg).then_some(2 + leg),
        Gait::ArmCrawl => Some(c),
        Gait::OneArmDrag { arm } => (c == arm).then_some(arm),
        Gait::Wriggle => None,
    }
    .filter(|_| c < 2)
}

impl Rig {
    /// Gives where a point of the body (relative to the body joint) is
    /// when the bear lies on its belly, in model space.
    fn lie_point(&self, p: &AnimParams, local: Vec3) -> Vec3 {
        let pivot = self.body_joint + self.hip_pivot;
        let low = Vec3::new(pivot.x, p.lie_height, pivot.z);
        low + Quat::from_rotation_x(p.lie_angle) * (local - self.hip_pivot)
    }

    /// Gives the place on the ground of contact `c` in a gait, in model
    /// space. In a crawl, the paw lands in front of the shoulder.
    fn contact_home(&self, p: &AnimParams, gait: Gait, c: usize) -> Option<Vec3> {
        let limb = contact_limb(gait, c)?;
        if limb >= 2 {
            let mut home = self.foot_home(c);
            if let Gait::Hop { .. } = gait {
                // One leg stands below the middle of the body.
                home.x *= 0.3;
            }
            return Some(home);
        }
        let shoulder = self.lie_point(p, self.limb_joint[limb]);
        let reach = p.crawl_reach * self.limb_tip[limb].length();
        let ahead = (reach * reach - shoulder.y * shoulder.y).max(0.0).sqrt();
        Some(Vec3::new(
            shoulder.x + SIDE[limb] * 0.02,
            0.0,
            shoulder.z + ahead,
        ))
    }

    /// Gives the place on the wall of a limb (an index of [`LIMBS`]), in
    /// model space: the hands above the shoulders, the feet below the
    /// hips.
    fn wall_home(&self, p: &AnimParams, limb: usize) -> Vec3 {
        let joint = self.body_joint + self.limb_joint[limb];
        let length = self.limb_tip[limb].length();
        let offset = if limb < 2 {
            Vec3::new(SIDE[limb] * 0.025, length * p.hand_reach, 0.0)
        } else {
            Vec3::new(SIDE[limb] * 0.015, -length * 0.8, 0.0)
        };
        Vec3::new(joint.x + offset.x, joint.y + offset.y, WALL_Z)
    }
}

/// The height of the body in a hop at a place `phase` of the hop, from
/// -0.25 (the foot pushes) to 1 (the top of the hop).
fn hop_lift(phase: f32, duty: f32) -> f32 {
    if phase >= duty {
        (PI * (phase - duty) / (1.0 - duty).max(0.05)).sin()
    } else {
        -0.25 * (PI * phase / duty.max(0.05)).sin()
    }
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
        let gait = input.profile.gait;
        if !self.started || origin.distance(self.last_origin) > TELEPORT_M {
            self.restart(rig, p, gait, origin, turn, heading);
        }
        self.time = (self.time + dt) % 4096.0;
        self.tune(p);
        let new_gait = gait != self.gait;
        self.gait = gait;

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
        let homes = [0, 1].map(|c| rig.contact_home(p, gait, c));
        if grounded && (!self.grounded || new_gait) {
            // A landing: the body squashes, and the feet find the ground.
            // A new limb set also needs new places to stand.
            if self.started && !self.grounded {
                self.squash
                    .kick((self.fall_speed * p.land_squash).clamp(0.0, 6.0));
            }
            for (c, home) in homes.iter().enumerate() {
                if let Some(home) = home {
                    self.plant(c, origin + turn * *home, origin.y, p, ground);
                }
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
        let stride = stride_length(speed, self.slip, p) * input.profile.stride_scale.max(0.1);
        self.duty = match gait {
            Gait::Hop { .. } => p.hop_duty.clamp(0.2, 0.8),
            Gait::OneArmDrag { .. } => 0.6,
            _ => duty_factor(stride, p),
        };
        let phase_step = if grounded {
            flat(step).length() / stride
        } else {
            0.0
        };
        self.phase = (self.phase + phase_step).fract();
        // A slow gait is "moving" at a lower speed.
        let slow = input.profile.speed_scale.clamp(0.05, 1.0);
        let moving_target = if grounded {
            (speed / (0.5 * slow)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.moving += (moving_target - self.moving) * blend(10.0, dt);

        if grounded {
            self.step_feet(
                p, homes, origin, turn, step, speed, slow, stride, phase_step, dt, ground,
            );
        }
        let leg_target = if grounded { 1.0 } else { 0.0 };
        self.leg_weight += (leg_target - self.leg_weight) * blend(18.0, dt);
        let arm_target = if on_ground_mode { 1.0 } else { 0.0 };
        self.arm_weight += (arm_target - self.arm_weight) * blend(12.0, dt);
        let lie_target = if gait.lies() && on_ground_mode {
            1.0
        } else {
            0.0
        };
        self.lie += (lie_target - self.lie) * blend(5.0, dt);

        // The hands and the feet on a wall.
        let climbing = matches!(input.mode, AnimMode::Climb { .. });
        let (climb_targets, hang_target, hang_roll) = if climbing {
            self.climb_holds(rig, p, input.limbs, origin, turn, velocity, dt, ground)
        } else {
            self.was_climbing = false;
            ([None; 4], Vec3::ZERO, 0.0)
        };
        let climb_target = if climbing { 1.0 } else { 0.0 };
        self.climb_weight += (climb_target - self.climb_weight) * blend(14.0, dt);
        let hang = self.hang.update(dt, hang_target).clamp_length_max(0.06);

        // The body.
        let gait_weight = self.moving * self.leg_weight;
        let walk_weight = if gait == Gait::Walk { gait_weight } else { 0.0 };
        let stand = (1.0 - self.moving) * self.leg_weight;
        let running = ((0.5 - self.duty) / 0.25).clamp(0.0, 1.0);
        let mid_stance = self.phase - 0.5 * self.duty;
        let speed_part = (speed / 2.0).min(1.0);
        let bob = match gait {
            Gait::Hop { leg } => {
                let p_leg = (self.phase + 0.5 * leg as f32).fract();
                gait_weight * p.hop_height * hop_lift(p_leg, self.duty)
            }
            // A walk is highest in the middle of a stance. A run is
            // lowest there. A bear that lies bobs less.
            _ => {
                gait_weight
                    * p.bob_height
                    * (0.5 + 0.5 * speed_part)
                    * (1.0 - 2.0 * running)
                    * (1.0 - 0.5 * self.lie)
                    * (2.0 * TAU * mid_stance).cos()
            }
        };

        let mut lowest_foot = 0.0_f32;
        let mut soft = 0.0;
        let mut contacts = 0.0;
        for (c, foot) in self.feet.iter().enumerate() {
            if homes[c].is_none() {
                continue;
            }
            if foot.state == FootState::Planted {
                lowest_foot = lowest_foot.min(foot.pos.y - origin.y);
            }
            soft += foot.soft;
            contacts += 1.0;
        }
        let soft = if contacts > 0.0 { soft / contacts } else { 0.0 };
        let height_target = if grounded {
            0.6 * lowest_foot.max(-0.06) - soft * p.sink_depth
        } else {
            0.0
        };
        let height = self.height.update(dt, height_target)
            + bob
            + self.lie * (p.lie_height - rig.hip_height());
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
            // On a wall, the shoulders follow the line of the hands.
            AnimMode::Climb { .. } => Vec2::new(0.0, hang_roll),
            AnimMode::Mantle => Vec2::ZERO,
        };
        let tilt = self
            .tilt
            .update(dt, tilt_target.clamp_length_max(0.6))
            .clamp_length_max(0.8);
        let pitch = tilt.x
            + stand * p.idle_sway * 0.6 * (TAU * 0.17 * self.time + 1.0).sin()
            + self.lie * p.lie_angle;
        let hop_lean = match gait {
            // The body leans over the leg that it stands on.
            Gait::Hop { leg } => -SIDE[2 + leg] * p.hop_lean * self.leg_weight,
            _ => 0.0,
        };
        let roll = tilt.y - walk_weight * p.waddle * (TAU * mid_stance).cos()
            + stand * p.idle_sway * (TAU * 0.23 * self.time).sin()
            + hop_lean;
        let wave = TAU * self.phase;
        let twist = match gait {
            Gait::Walk => -walk_weight * p.twist * wave.cos(),
            Gait::Wriggle => gait_weight * p.wriggle_yaw * wave.sin(),
            _ => 0.0,
        };
        // A roll along the length of a body that lies.
        let spin = self.lie
            * gait_weight
            * match gait {
                Gait::Wriggle => p.wriggle_roll * (wave + 1.0).sin(),
                Gait::OneArmDrag { arm } => SIDE[arm] * p.drag_roll * (0.5 - 0.5 * wave.cos()),
                Gait::ArmCrawl => 0.5 * p.drag_roll * wave.sin(),
                _ => 0.0,
            };

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
            * Quat::from_rotation_z(roll)
            * Quat::from_rotation_y(spin))
        .normalize();
        // A squash makes the body shorter and wider. The hips come down
        // by the same part, so the feet stay on the ground.
        let scale = Vec3::new(1.0 + 0.5 * squash, 1.0 - squash, 1.0 + 0.5 * squash);
        let body_place = rig.body_joint + Vec3::Y * (height - squash * rig.hip_height()) + hang;
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
            let contact = (0..2).find(|&c| homes[c].is_some() && contact_limb(gait, c) == Some(i));
            let (weight, target, limits) = if let Some(target) = climb_targets[i] {
                (self.climb_weight, target, (0.8, 1.25))
            } else if let Some(c) = contact {
                let foot = &self.feet[c];
                let mut target = to_model * (foot.pos - origin);
                if i >= 2 {
                    // A foot that is not below the hip stands on its toes
                    // or its heel. This lifts the middle of the sole.
                    let away = ((target.z - joint.z).abs() / rest.length().max(1e-3)).min(1.0);
                    target.y += p.foot_half_length * away;
                    (self.leg_weight, target, (0.72, 1.12))
                } else {
                    (self.leg_weight, target, (0.8, 1.2))
                }
            } else if i < 2 {
                let target = self.paw_target(
                    i,
                    rig,
                    p,
                    joint,
                    normal,
                    accel_m,
                    velocity,
                    gait_weight,
                    speed_part,
                    legs_pass,
                    dt,
                );
                (self.arm_weight, target, (0.9, 1.1))
            } else {
                // A leg with no place to stand keeps its free pose.
                (0.0, joint + rest, (1.0, 1.0))
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
        // down while the bear falls. A bear that lies lifts its head to
        // look ahead.
        let head_target = Vec3::new(
            (-p.head_level * tilt.x
                - 0.04 * velocity_m.y * (1.0 - self.leg_weight)
                - self.lie * p.lie_angle * p.lie_head)
                .clamp(-0.5 - self.lie, 0.5),
            (self.turn_rate * p.head_turn).clamp(-0.5, 0.5),
            -p.head_level * roll,
        );
        let h = self.head.update(dt, head_target).clamp_length_max(1.5);
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
    fn restart(
        &mut self,
        rig: &Rig,
        p: &AnimParams,
        gait: Gait,
        origin: Vec3,
        turn: Quat,
        heading: f32,
    ) {
        let phase = self.phase;
        *self = BearAnim::default();
        self.phase = phase;
        self.gait = gait;
        self.lie = if gait.lies() { 1.0 } else { 0.0 };
        self.last_origin = origin;
        self.last_heading = heading;
        for c in 0..2 {
            let home = rig.contact_home(p, gait, c).unwrap_or(rig.foot_home(c));
            self.feet[c].pos = origin + turn * home;
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

    /// Moves the contacts of a bear on the ground: a planted foot (or
    /// paw) stays, and a foot in a swing moves to its next place.
    /// `homes` gives the place of each contact in model space; `None`
    /// is no contact. `slow` is the speed scale of the gait.
    #[allow(clippy::too_many_arguments)]
    fn step_feet(
        &mut self,
        p: &AnimParams,
        homes: [Option<Vec3>; 2],
        origin: Vec3,
        turn: Quat,
        step: Vec3,
        speed: f32,
        slow: f32,
        stride: f32,
        phase_step: f32,
        dt: f32,
        ground: &mut dyn Ground,
    ) {
        let walks = speed > 0.15 * slow;
        let dir = if walks {
            flat(step).normalize_or_zero()
        } else {
            Vec3::ZERO
        };
        // In the middle of a stance, the foot is below the hip. So a
        // step lands half a stance in front of the hip.
        let stance = self.duty * stride;
        let ahead = dir * (0.5 * stance * self.moving);
        // A hop lifts the foot with the body.
        let hop = match self.gait {
            Gait::Hop { .. } => p.hop_height * self.moving,
            _ => 0.0,
        };
        let mut slip_sum = 0.0;
        let mut contacts = 0.0;
        for i in 0..2 {
            let Some(home_m) = homes[i] else {
                continue;
            };
            let other_planted =
                homes[1 - i].is_none() || self.feet[1 - i].state == FootState::Planted;
            let home = origin + turn * home_m;
            let target = home + ahead;
            let p_leg = (self.phase + 0.5 * i as f32).fract();
            let foot = &mut self.feet[i];
            let slip = (1.0 - foot.grip / p.firm_grip.max(0.01)).clamp(0.0, 1.0)
                * p.slip_amount.clamp(0.0, 1.0);
            slip_sum += slip / p.slip_amount.clamp(0.01, 1.0);
            contacts += 1.0;
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
                        let lift = p.step_height * (0.5 + 0.5 * self.moving)
                            + p.sink_depth * foot.soft
                            + hop;
                        foot.pos =
                            (home + from).lerp(point, smooth(u)) + Vec3::Y * lift * (PI * u).sin();
                        foot.state = FootState::Swing { from, u };
                    }
                }
            }
        }
        let slip = if contacts > 0.0 {
            slip_sum / contacts
        } else {
            0.0
        };
        self.slip += (slip - self.slip) * blend(6.0, dt);
    }

    /// Moves the hands and the feet on a wall. Each limb holds a place
    /// in the world. When the body moves too far from it, the limb steps
    /// to a new place that a fan of rays finds. The two hands step in
    /// turn, and so do the two feet.
    ///
    /// Gives the target of each limb in model space (`None` for a lost
    /// limb), the place of the body below the hands, and the roll of the
    /// body.
    #[allow(clippy::too_many_arguments)]
    fn climb_holds(
        &mut self,
        rig: &Rig,
        p: &AnimParams,
        limbs: LimbSet,
        origin: Vec3,
        turn: Quat,
        velocity: Vec3,
        dt: f32,
        ground: &mut dyn Ground,
    ) -> ([Option<Vec3>; 4], Vec3, f32) {
        let to_model = turn.inverse();
        let start = !self.was_climbing;
        self.was_climbing = true;
        let velocity_m = to_model * velocity;
        let along = Vec2::new(velocity_m.x, velocity_m.y);
        let dir = along.normalize_or_zero();
        let out = turn * -Vec3::Z;
        let mut targets = [None; 4];
        for i in 0..4 {
            if !limbs.has(i) {
                continue;
            }
            let home = rig.wall_home(p, i);
            let home_world = origin + turn * home;
            let step_time = if i < 2 {
                p.hand_step_time
            } else {
                p.foot_step_time
            }
            .max(0.02);
            // The two hands step in turn, and so do the two feet.
            let partner = i ^ 1;
            let partner_moves =
                limbs.has(partner) && matches!(self.holds[partner].state, HoldState::Reach { .. });
            let mut hold = self.holds[i];
            hold.state = match hold.state {
                _ if start => match self.find_hold(ground, p, origin, turn, home, i) {
                    Some(at) => {
                        hold.pos = at;
                        HoldState::Held
                    }
                    None => HoldState::Free {
                        retry: HOLD_RETRY_S,
                    },
                },
                HoldState::Free { retry } if retry > dt => HoldState::Free { retry: retry - dt },
                HoldState::Free { .. } => match self.find_hold(ground, p, origin, turn, home, i) {
                    Some(at) => HoldState::Reach {
                        from: hold.pos - home_world,
                        to: at,
                        u: 0.0,
                    },
                    None => HoldState::Free {
                        retry: HOLD_RETRY_S,
                    },
                },
                HoldState::Held => {
                    let at = to_model * (hold.pos - origin);
                    let offset = Vec2::new(at.x - home.x, at.y - home.y);
                    if offset.length() > p.hold_step && !partner_moves {
                        // Reach ahead in the climb direction, so the hold is
                        // near the home when the step ends.
                        let lead = along * step_time + dir * (0.5 * p.hold_step);
                        let want = home + lead.extend(0.0);
                        match self.find_hold(ground, p, origin, turn, want, i) {
                            Some(to) => HoldState::Reach {
                                from: hold.pos - home_world,
                                to,
                                u: 0.0,
                            },
                            None => HoldState::Free {
                                retry: HOLD_RETRY_S,
                            },
                        }
                    } else {
                        HoldState::Held
                    }
                }
                HoldState::Reach { from, to, u } => {
                    let u = u + dt / step_time;
                    if u >= 1.0 {
                        hold.pos = to;
                        HoldState::Held
                    } else {
                        HoldState::Reach { from, to, u }
                    }
                }
            };
            match hold.state {
                HoldState::Free { .. } => hold.pos = home_world,
                HoldState::Reach { from, to, u } => {
                    hold.pos = (home_world + from).lerp(to, smooth(u))
                        + out * p.hold_lift * (PI * u).sin();
                }
                HoldState::Held => {}
            }
            self.holds[i] = hold;
            targets[i] = Some(to_model * (hold.pos - origin));
        }

        // The body hangs from the hands: it follows them a little, it
        // hangs lower while a hand steps, and the shoulders follow the
        // line of the hands.
        let mut sum = Vec2::ZERO;
        let mut held = 0.0;
        let mut moving = 0.0;
        let mut height = [None; 2];
        for arm in 0..2 {
            let Some(target) = targets[arm] else {
                continue;
            };
            let home = rig.wall_home(p, arm);
            height[arm] = Some(target.y - home.y);
            match self.holds[arm].state {
                HoldState::Reach { .. } => moving += 1.0,
                _ => {
                    sum += Vec2::new(target.x - home.x, target.y - home.y);
                    held += 1.0;
                }
            }
        }
        let follow = if held > 0.0 {
            sum / held * p.hang_follow
        } else {
            Vec2::ZERO
        };
        let sag = p.hang_sag * (1.0 + moving + 2.0 - (held + moving));
        let hang = Vec3::new(follow.x, follow.y - sag, 0.0);
        let roll = match height {
            [Some(left), Some(right)] => p.hang_roll * (left - right),
            // With one hand, the body hangs to the side of the hand.
            [Some(_), None] => 0.1,
            [None, Some(_)] => -0.1,
            [None, None] => 0.0,
        };
        (targets, hang, roll.clamp(-0.35, 0.35))
    }

    /// Looks for a place to hold near `want` (model space) with a fan of
    /// rays into the wall: 4 rays for a hand, 2 for a foot. Each hit gets
    /// a score from its distance to `want`, how much it faces the bear,
    /// and its grip. Gives the best place in the world.
    fn find_hold(
        &mut self,
        ground: &mut dyn Ground,
        p: &AnimParams,
        origin: Vec3,
        turn: Quat,
        want: Vec3,
        limb: usize,
    ) -> Option<Vec3> {
        let s = p.fan_spread.max(0.005);
        let hand = [
            Vec2::ZERO,
            Vec2::new(s, 0.0),
            Vec2::new(-s, 0.0),
            Vec2::new(0.0, s),
        ];
        let foot = [Vec2::ZERO, Vec2::new(0.0, -s)];
        let fan: &[Vec2] = if limb < 2 { &hand } else { &foot };
        let out = turn * -Vec3::Z;
        let to_model = turn.inverse();
        let mut best: Option<(f32, Vec3)> = None;
        for offset in fan {
            let at = Vec2::new(want.x + offset.x, want.y + offset.y);
            let from = origin + turn * Vec3::new(at.x, at.y, -0.02);
            let to = origin + turn * Vec3::new(at.x, at.y, WALL_Z + 0.23);
            self.hold_casts += 1;
            let Some(hit) = ground.cast(from, to) else {
                continue;
            };
            let facing = hit.normal.dot(out);
            if facing < 0.2 {
                continue;
            }
            let point = to_model * (hit.point - origin);
            let distance = Vec2::new(point.x - want.x, point.y - want.y).length()
                + (point.z - WALL_Z).abs() * 0.5;
            let score = facing + ground.grip(&hit) - 0.5 * distance / s;
            if best.is_none_or(|b| score > b.0) {
                best = Some((score, hit.point + hit.normal * 0.012));
            }
        }
        best.map(|b| b.1)
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
        if let Gait::Hop { .. } = self.gait {
            // On one leg, the arms go out for balance.
            out += p.hop_arms_out * self.leg_weight;
            up += 0.3 * p.hop_arms_out * self.leg_weight;
        }
        let hang = Quat::from_rotation_z(side * out) * Quat::from_rotation_x(-up) * rest;
        let inertia = (-accel_m * p.arm_inertia).clamp_length_max(0.5 * rest.length());
        let soft = self.paws[arm].update(dt, shoulder + hang + inertia);
        // The left arm goes forward with the right leg. The spring is
        // too slow for the steps, so the swing comes after the spring.
        // Only a walk swings the arms.
        let gait = if self.gait == Gait::Walk { gait } else { 0.0 };
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

    /// Gives the place of the hold of a limb on a wall, and tells if the
    /// limb holds it now (not in a step and not free).
    #[cfg(test)]
    fn hold(&self, limb: usize) -> (Vec3, bool) {
        let h = &self.holds[limb.min(3)];
        (h.pos, h.state == HoldState::Held)
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
    use crate::limbs::{profile, GaitTuning};

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
        limbs: LimbSet,
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
                limbs: LimbSet::ALL,
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
                limbs: self.limbs,
                profile: profile(self.limbs, &GaitTuning::default()),
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

        /// The speed of the gait of the limbs, at a part of the walk speed.
        fn gait_speed(&self, walk: f32) -> f32 {
            walk * profile(self.limbs, &GaitTuning::default()).speed_scale
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
                limbs: LimbSet::ALL,
                profile: profile(LimbSet::ALL, &GaitTuning::default()),
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

    fn without(lost: &[usize]) -> LimbSet {
        lost.iter().fold(LimbSet::ALL, |s, &i| s.with(i, false))
    }

    #[test]
    fn a_bear_on_one_leg_hops_and_bounces() {
        let mut w = Walker::new(Plane::flat());
        w.limbs = without(&[3]);
        let speed = w.gait_speed(1.4);
        w.run(speed, 1.0);
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        let mut steps = 0;
        let mut last = w.anim.foot(0).1;
        for _ in 0..180 {
            w.step(speed, true);
            let y = w.pose.body.translation.y;
            low = low.min(y);
            high = high.max(y);
            let planted = w.anim.foot(0).1;
            if planted && !last {
                steps += 1;
            }
            last = planted;
            // The one sole stays near the middle of the body.
            let sole = w.model().inverse().transform_point3(w.tip(2));
            assert!(sole.x.abs() < 0.05, "sole {sole}");
        }
        assert!(steps >= 4, "{steps} hops in 3 s");
        assert!(high - low > 0.6 * w.p.hop_height, "bounce {}", high - low);
        // The body leans over the left leg: its top goes to +X.
        let up = w.pose.body.matrix3 * Vec3::Y;
        assert!(up.x > 0.02, "up {up}");
    }

    #[test]
    fn a_bear_without_legs_lies_and_pulls_with_its_arms() {
        let mut w = Walker::new(Plane::flat());
        w.limbs = without(&[2, 3]);
        let speed = w.gait_speed(1.4);
        w.run(speed, 2.0);
        let mut steps = [0; 2];
        let mut last = [w.anim.foot(0), w.anim.foot(1)];
        for _ in 0..240 {
            w.step(speed, true);
            // The body lies on its belly, low on the ground.
            let up = w.pose.body.matrix3 * Vec3::Y;
            assert!(up.z > 0.85, "the body points ahead: {up}");
            let hips = w.pose.body.transform_point3(w.rig.hip_pivot);
            assert!(hips.y < 0.1, "hips at {hips}");
            for arm in 0..2 {
                let now = w.anim.foot(arm);
                if now.1 {
                    // A planted paw stays, and the arm reaches it.
                    if last[arm].1 {
                        assert!(now.0.distance(last[arm].0) < 1e-5);
                    }
                    let tip = w.tip(arm);
                    assert!(tip.distance(now.0) < 0.03, "paw {tip}, contact {}", now.0);
                } else if last[arm].1 {
                    steps[arm] += 1;
                }
                last[arm] = now;
            }
        }
        assert!(steps.iter().all(|&n| n >= 2), "steps {steps:?}");
    }

    #[test]
    fn a_wriggle_turns_the_body_from_side_to_side_and_uses_no_rays() {
        let mut w = Walker::new(Plane::flat());
        w.limbs = without(&[0, 1, 2, 3]);
        let speed = w.gait_speed(1.4);
        w.run(speed, 2.0);
        let casts = w.ground.casts;
        let (mut left, mut right) = (0.0_f32, 0.0_f32);
        for _ in 0..240 {
            w.step(speed, true);
            let up = w.pose.body.matrix3 * Vec3::Y;
            left = left.max(up.x);
            right = right.min(up.x);
        }
        assert!(left > 0.05 && right < -0.05, "wriggle {right} to {left}");
        assert_eq!(w.ground.casts, casts);
    }

    #[test]
    fn each_limb_set_gives_a_safe_pose() {
        for bits in 0..16_usize {
            let limbs = (0..4).fold(LimbSet::ALL, |s, i| s.with(i, bits & (1 << i) != 0));
            let mut w = Walker::new(Plane {
                rise: 0.3,
                grip: 0.5,
                soft: 0.5,
                casts: 0,
            });
            w.limbs = limbs;
            let speed = w.gait_speed(3.2);
            w.run(speed, 1.0);
            w.run(0.0, 0.5);
            // A fall, and a change to all limbs in the middle of a walk.
            w.y += 0.5;
            for _ in 0..20 {
                w.step(speed, false);
            }
            w.limbs = LimbSet::ALL;
            w.run(1.4, 0.5);
        }
    }

    /// A vertical wall at z = `at`. It faces -Z. Above `top`, it has no
    /// surface.
    struct Wall {
        at: f32,
        top: f32,
        casts: usize,
    }

    impl Surface for Wall {
        fn cast(&mut self, from: Vec3, to: Vec3) -> Option<Hit> {
            self.casts += 1;
            if from.z > self.at || to.z < self.at {
                return None;
            }
            let t = (self.at - from.z) / (to.z - from.z).max(1e-6);
            let point = from.lerp(to, t);
            (point.y <= self.top).then_some(Hit {
                point,
                normal: -Vec3::Z,
                id: 0,
            })
        }

        fn grip(&mut self, _hit: &Hit) -> f32 {
            1.0
        }
    }

    impl Ground for Wall {
        fn softness(&mut self, _hit: &Hit) -> f32 {
            0.0
        }
    }

    /// A bear that climbs the wall: the model looks at the wall.
    struct Climber {
        rig: Rig,
        p: AnimParams,
        anim: BearAnim,
        wall: Wall,
        limbs: LimbSet,
        y: f32,
        travel: f32,
        pose: Pose,
    }

    impl Climber {
        fn new(limbs: LimbSet) -> Climber {
            let mut c = Climber {
                rig: rig(),
                p: AnimParams::default(),
                anim: BearAnim::default(),
                wall: Wall {
                    at: 1.0,
                    top: 100.0,
                    casts: 0,
                },
                limbs,
                y: 0.0,
                travel: 0.0,
                pose: Pose {
                    body: Affine3A::IDENTITY,
                    head: Quat::IDENTITY,
                    ears: [Quat::IDENTITY; 2],
                    limbs: [Affine3A::IDENTITY; 4],
                    hump: Vec3::ZERO,
                },
            };
            c.step(0.0);
            c
        }

        fn model(&self) -> Affine3A {
            Affine3A::from_translation(Vec3::new(0.0, self.y, self.wall.at - WALL_Z))
        }

        /// One step up the wall at a speed.
        fn step(&mut self, speed: f32) {
            self.y += speed * DT;
            self.travel += speed.abs() * DT;
            let input = AnimInput {
                dt: DT,
                model: self.model(),
                on_floor: false,
                floor_normal: Vec3::Y,
                mode: AnimMode::Climb {
                    travel: self.travel,
                },
                limbs: self.limbs,
                profile: profile(self.limbs, &GaitTuning::default()),
            };
            self.pose = self.anim.update(&self.rig, &self.p, &input, &mut self.wall);
            assert!(self.pose.body.is_finite());
            assert!(self.pose.limbs.iter().all(|l| l.is_finite()));
        }

        fn tip(&self, limb: usize) -> Vec3 {
            (self.model() * self.pose.body * self.pose.limbs[limb])
                .transform_point3(self.rig.limb_tip[limb])
        }
    }

    #[test]
    fn the_hands_hold_the_wall_and_step_in_turn() {
        let mut c = Climber::new(LimbSet::ALL);
        for _ in 0..30 {
            c.step(0.0);
        }
        let mut last = [0, 1, 2, 3].map(|i| c.anim.hold(i));
        let mut steps = [0; 4];
        for _ in 0..180 {
            c.step(0.7);
            let now = [0, 1, 2, 3].map(|i| c.anim.hold(i));
            // The two hands never move at the same time.
            assert!(now[0].1 || now[1].1, "both hands are off the wall");
            for i in 0..4 {
                if now[i].1 {
                    // A hold is on the wall and stays at its place.
                    assert!((now[i].0.z - (c.wall.at - 0.012)).abs() < 1e-4);
                    if last[i].1 {
                        assert!(now[i].0.distance(last[i].0) < 1e-5);
                    }
                    if !last[i].1 {
                        steps[i] += 1;
                    }
                }
                last[i] = now[i];
            }
            // The paws reach their holds while the bear climbs.
            for hand in 0..2 {
                if now[hand].1 {
                    let tip = c.tip(hand);
                    assert!(tip.distance(now[hand].0) < 0.03, "paw {tip}");
                }
            }
        }
        // 2.1 m in 3 s: each limb steps many times.
        assert!(steps.iter().all(|&n| n >= 8), "steps {steps:?}");
    }

    #[test]
    fn a_bear_that_holds_still_on_a_wall_uses_no_rays() {
        let mut c = Climber::new(LimbSet::ALL);
        for _ in 0..60 {
            c.step(0.7);
        }
        for _ in 0..30 {
            c.step(0.0);
        }
        let casts = c.wall.casts;
        for _ in 0..60 {
            c.step(0.0);
        }
        assert_eq!(c.wall.casts, casts);
        // Each step of a hand uses its fan of 4 rays, and a foot 2 rays.
        let mut c = Climber::new(LimbSet::ALL);
        assert_eq!(c.wall.casts, 4 + 4 + 2 + 2);
        c.step(0.0);
        assert_eq!(c.wall.casts, 12);
    }

    #[test]
    fn with_one_hand_the_body_hangs_to_its_side() {
        let mut c = Climber::new(without(&[0]));
        for _ in 0..90 {
            c.step(0.0);
        }
        // The right side (-X) is higher: the X axis of the body goes down.
        let x = c.pose.body.matrix3 * Vec3::X;
        assert!(x.y < -0.03, "x axis {x}");
        let full = {
            let mut c = Climber::new(LimbSet::ALL);
            for _ in 0..90 {
                c.step(0.0);
            }
            c.pose.body.translation.y
        };
        assert!(c.pose.body.translation.y < full, "one hand hangs lower");
    }

    #[test]
    fn a_hand_above_the_top_of_the_wall_reaches_and_tries_again() {
        let mut c = Climber::new(LimbSet::ALL);
        c.wall.top = 0.5;
        for _ in 0..120 {
            c.step(0.5);
        }
        // The hands are above the top now. They cannot hold, and they try
        // again only after a short time.
        assert!(!c.anim.hold(0).1 && !c.anim.hold(1).1);
        let casts = c.wall.casts;
        c.step(0.0);
        assert!(c.wall.casts - casts <= 8);
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
