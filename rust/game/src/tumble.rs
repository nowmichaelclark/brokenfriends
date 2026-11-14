//! The tumble of the bear (R11, R12), with no Godot types. Unit tests
//! cover this module.
//!
//! When the bear hits something hard, it tumbles: it becomes a simple
//! ragdoll of Jolt bodies (`ragdoll.rs`). A ragdoll is a set of bodies
//! with joints that the physics engine moves, with no animation. This
//! module has the rules: when a tumble starts, how the ragdoll starts
//! to turn, when it is at rest, which way the bear looks when it gets
//! up, and the shape and mass of each body.

use crate::bear_shape::{Part, PartId};
use glam::{Mat3, Vec3};

/// The tuning values of the tumble. The bear copies them from its
/// exported fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TumbleParams {
    /// A hit at this speed or faster makes the bear tumble, in m/s.
    pub impact_speed: f32,
    /// A bear that slides off a wall at this speed or faster tumbles, in m/s.
    pub let_go_speed: f32,
    /// The ragdoll starts to turn at this rate for each m/s of speed, in rad/s.
    pub spin_per_speed: f32,
    /// The fastest start turn, in rad/s.
    pub max_spin: f32,
    /// The body is at rest below this speed, in m/s.
    pub rest_speed: f32,
    /// The body is at rest below this turn rate, in rad/s.
    pub rest_spin: f32,
    /// The body must be at rest this long before the bear gets up, in s.
    pub rest_time: f32,
    /// The shortest tumble, in seconds.
    pub min_time: f32,
    /// After this time, the bear gets up even if the body still moves.
    pub max_time: f32,
}

impl Default for TumbleParams {
    fn default() -> TumbleParams {
        TumbleParams {
            impact_speed: 7.0,
            let_go_speed: 3.0,
            spin_per_speed: 0.6,
            max_spin: 9.0,
            rest_speed: 0.25,
            rest_spin: 2.0,
            rest_time: 0.4,
            min_time: 0.8,
            max_time: 6.0,
        }
    }
}

/// Gives the speed of a hit: the change of the velocity that a
/// collision made. `before` is the velocity that the bear wanted, and
/// `after` is the velocity after the collision.
pub fn impact(before: Vec3, after: Vec3) -> f32 {
    (before - after).length()
}

/// Gives the start turn of the ragdoll (an angular velocity in rad/s).
/// The body turns over the direction of the motion. A straight fall
/// turns over `facing`, so the bear falls on its face.
pub fn start_spin(velocity: Vec3, facing: Vec3, p: &TumbleParams) -> Vec3 {
    let speed = velocity.length();
    let flat = Vec3::new(velocity.x, 0.0, velocity.z);
    let facing = Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z);
    // A mix of the motion on the ground and the facing: a fast slide
    // turns over the slide, a fall turns over the face.
    let dir = (flat + facing * 0.5).normalize_or(facing);
    let axis = Vec3::Y.cross(dir).normalize_or(Vec3::X);
    axis * (speed * p.spin_per_speed).min(p.max_spin)
}

/// Finds when the ragdoll is at rest.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Settle {
    /// The time since the tumble started.
    pub time: f32,
    /// The time that the body is at rest now.
    rest: f32,
}

impl Settle {
    /// One step. `speed` and `spin` are of the body of the bear. Gives
    /// true when the bear can get up.
    pub fn update(&mut self, dt: f32, speed: f32, spin: f32, p: &TumbleParams) -> bool {
        self.time += dt;
        if speed < p.rest_speed && spin < p.rest_spin {
            self.rest += dt;
        } else {
            self.rest = 0.0;
        }
        (self.time >= p.min_time && self.rest >= p.rest_time) || self.time >= p.max_time
    }
}

/// Gives the direction that the bear looks at when it gets up, as an
/// angle around Y. `body` is the turn of the body of the ragdoll. The
/// bear looks along its belly. If the belly is up or down, it looks
/// toward its head.
pub fn get_up_yaw(body: Mat3) -> f32 {
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
    let forward = flat(body.z_axis);
    let dir = if forward.length() > 0.3 {
        forward
    } else {
        flat(body.y_axis)
    };
    let dir = dir.normalize_or(Vec3::Z);
    dir.x.atan2(dir.z)
}

/// The collision shape of one body of the ragdoll, in the space of the
/// part (relative to its joint).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Collider {
    /// A capsule from `a` to `b` (the centres of its two round ends).
    Capsule {
        a: Vec3,
        b: Vec3,
        radius: f32,
    },
    Sphere {
        centre: Vec3,
        radius: f32,
    },
}

impl Collider {
    /// The volume in cubic metres.
    pub fn volume(&self) -> f32 {
        use std::f32::consts::PI;
        match *self {
            Collider::Capsule { a, b, radius } => {
                PI * radius * radius * (a.distance(b) + 4.0 / 3.0 * radius)
            }
            Collider::Sphere { radius, .. } => 4.0 / 3.0 * PI * radius.powi(3),
        }
    }
}

/// One body of the ragdoll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyDef {
    pub id: PartId,
    pub collider: Collider,
    /// The mass in kilograms.
    pub mass: f32,
}

/// The density of the bodies, in kg per cubic metre. The capsules are a
/// little larger than the parts. A teddy bear of 0.40 m weighs about
/// 0.3 to 0.5 kg.
const DENSITY: f32 = 120.0;

/// Gives the bodies of the ragdoll: the body, the head and the four
/// limbs. The ears stay on the head.
pub fn bodies(parts: &[Part]) -> Vec<BodyDef> {
    parts
        .iter()
        .filter(|p| !matches!(p.id, PartId::LeftEar | PartId::RightEar))
        .filter_map(|part| {
            let collider = collider(part)?;
            let mass = (collider.volume() * DENSITY).max(0.01);
            Some(BodyDef {
                id: part.id,
                collider,
                mass,
            })
        })
        .collect()
}

/// Gives the collision shape of a part from its mesh.
fn collider(part: &Part) -> Option<Collider> {
    let points: Vec<Vec3> = part
        .mesh
        .positions
        .iter()
        .map(|&p| Vec3::from_array(p))
        .collect();
    if points.is_empty() {
        return None;
    }
    let lo = points.iter().fold(Vec3::splat(f32::MAX), |a, &p| a.min(p));
    let hi = points.iter().fold(Vec3::splat(f32::MIN), |a, &p| a.max(p));
    let half = (hi - lo) * 0.5;
    let centre = (hi + lo) * 0.5;
    Some(match part.id {
        PartId::Body => {
            // An upright capsule inside the belly and the chest.
            let radius = half.x.min(half.z) * 0.95;
            let reach = (half.y - radius).max(0.0);
            Collider::Capsule {
                a: centre - Vec3::Y * reach,
                b: centre + Vec3::Y * reach,
                radius,
            }
        }
        PartId::Head | PartId::LeftEar | PartId::RightEar => Collider::Sphere {
            centre,
            radius: (half.x + half.y + half.z) / 3.0 * 0.9,
        },
        _ => {
            // A limb: a capsule from near the joint to the tip, as thick
            // as the middle of the limb.
            let tip = part.tip - part.joint;
            let length = tip.length().max(0.01);
            let dir = tip / length;
            let mean = points
                .iter()
                .map(|&p| {
                    let t = p.dot(dir).clamp(0.0, length);
                    p.distance(dir * t)
                })
                .sum::<f32>()
                / points.len() as f32;
            let radius = (mean * 0.85).clamp(0.008, length * 0.45);
            Collider::Capsule {
                a: dir * radius,
                b: dir * (length - radius * 0.5).max(radius),
                radius,
            }
        }
    })
}

/// The limits of the joint of a part, in radians: the swing (how far
/// the part turns away from its start direction) and the twist (the
/// turn around it). `looseness` is 0 for a new seam and 1 for a limb
/// that dangles.
pub fn joint_limits(id: PartId, looseness: f32) -> (f32, f32) {
    let l = looseness.clamp(0.0, 1.0);
    let (swing, twist) = match id {
        PartId::Head => (35.0_f32, 30.0_f32),
        PartId::LeftArm | PartId::RightArm => (100.0, 40.0),
        _ => (80.0, 30.0),
    };
    let swing = (swing + l * 60.0).min(175.0);
    let twist = (twist + l * 40.0).min(175.0);
    (swing.to_radians(), twist.to_radians())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bear_shape::{build, Recipe};

    #[test]
    fn a_hard_landing_is_an_impact() {
        let p = TumbleParams::default();
        // A landing: the floor stops the fall.
        let landing = impact(Vec3::new(1.0, -9.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
        assert!((landing - 9.0).abs() < 1e-5);
        assert!(landing >= p.impact_speed);
        // A walk into nothing is no impact.
        assert_eq!(impact(Vec3::X, Vec3::X), 0.0);
    }

    #[test]
    fn the_spin_turns_over_the_motion_and_has_a_limit() {
        let p = TumbleParams::default();
        // A slide along +X turns around -Z: the head goes toward +X.
        let s = start_spin(Vec3::new(8.0, -1.0, 0.0), Vec3::X, &p);
        assert!(s.z < -0.9 * s.length(), "{s}");
        // A straight fall turns over the face.
        let s = start_spin(Vec3::new(0.0, -10.0, 0.0), Vec3::Z, &p);
        assert!(s.x > 0.9 * s.length(), "{s}");
        // Turning around +X moves +Y toward +Z: the head goes forward.
        assert!(Vec3::X.cross(Vec3::Y).z > 0.0);
        let fast = start_spin(Vec3::new(0.0, -100.0, 0.0), Vec3::Z, &p);
        assert!((fast.length() - p.max_spin).abs() < 1e-4);
    }

    #[test]
    fn the_bear_gets_up_after_a_rest_or_a_long_time() {
        let p = TumbleParams::default();
        let dt = 1.0 / 60.0;
        let mut s = Settle::default();
        // At rest from the start: it waits for the shortest tumble.
        let mut steps = 0;
        while !s.update(dt, 0.0, 0.0, &p) {
            steps += 1;
        }
        assert!((steps as f32 * dt - p.min_time).abs() < 2.0 * dt, "{steps}");
        // A body that rolls all the time gets up at the longest time.
        let mut s = Settle::default();
        let mut steps = 0;
        while !s.update(dt, 1.0, 5.0, &p) {
            steps += 1;
        }
        assert!((steps as f32 * dt - p.max_time).abs() < 2.0 * dt, "{steps}");
        // A short stop does not count.
        let mut s = Settle::default();
        for _ in 0..60 {
            assert!(!s.update(dt, 1.0, 0.0, &p));
        }
        for _ in 0..10 {
            assert!(!s.update(dt, 0.0, 0.0, &p));
        }
        assert!(!s.update(dt, 1.0, 0.0, &p));
    }

    #[test]
    fn the_bear_gets_up_looking_along_its_belly_or_its_head() {
        // Upright, looking along +X.
        let upright = Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2);
        assert!((get_up_yaw(upright) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        // On its back with the head toward -X: the belly is up.
        let on_back = Mat3::from_cols(Vec3::Z, -Vec3::X, Vec3::Y);
        let yaw = get_up_yaw(on_back);
        assert!((yaw.abs() - std::f32::consts::FRAC_PI_2).abs() < 1e-4 && yaw < 0.0);
    }

    #[test]
    fn the_ragdoll_has_six_bodies_inside_the_bear() {
        let parts = build(&Recipe::default());
        let bodies = bodies(&parts);
        assert_eq!(bodies.len(), 6);
        let total: f32 = bodies.iter().map(|b| b.mass).sum();
        assert!((0.15..=0.8).contains(&total), "mass {total}");
        for b in &bodies {
            match b.collider {
                Collider::Capsule { a, b: end, radius } => {
                    assert!(radius > 0.007 && radius < 0.08, "{:?}", b);
                    assert!(a.distance(end) > 0.005, "{:?}", b);
                }
                Collider::Sphere { radius, .. } => {
                    assert!(radius > 0.03 && radius < 0.09, "{:?}", b);
                }
            }
        }
        // The body is the heaviest part, and a limb is lighter than the head.
        let mass = |id| bodies.iter().find(|b| b.id == id).unwrap().mass;
        assert!(mass(PartId::Body) > mass(PartId::Head));
        assert!(mass(PartId::Head) > mass(PartId::LeftArm));
        // The arm capsule points from the shoulder to the paw.
        let arm = parts.iter().find(|p| p.id == PartId::LeftArm).unwrap();
        let tip = arm.tip - arm.joint;
        let arm_body = bodies.iter().find(|b| b.id == PartId::LeftArm).unwrap();
        let Collider::Capsule { b: end, .. } = arm_body.collider else {
            panic!("the arm is a capsule");
        };
        assert!(end.normalize().dot(tip.normalize()) > 0.99);
    }

    #[test]
    fn a_loose_seam_gives_a_wider_joint() {
        let (s0, t0) = joint_limits(PartId::LeftLeg, 0.0);
        let (s1, t1) = joint_limits(PartId::LeftLeg, 1.0);
        assert!(s1 > s0 && t1 > t0);
        assert!(s1 < std::f32::consts::PI);
    }
}
