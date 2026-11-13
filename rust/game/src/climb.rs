//! Climbing maths with no Godot types. Unit tests cover this module.
//!
//! The bear climbs on a [`Surface`]. A surface answers ray casts. In
//! the game, the physics engine answers them. In the tests, a height
//! function answers them. The bear is a point (its centre) that stays
//! at a fixed distance from the surface. No physics body moves while
//! the bear climbs.

use glam::{Vec2, Vec3};

/// The place where a ray hits a surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The hit point.
    pub point: Vec3,
    /// The unit normal of the surface. It points away from the surface.
    pub normal: Vec3,
    /// A number for the object that the ray hit. The surface uses it to
    /// find the grip.
    pub id: u64,
}

/// A thing that the bear can climb.
pub trait Surface {
    /// Casts a ray from `from` to `to`. Gives the first hit. A ray that
    /// starts in the ground gives no hit.
    fn cast(&mut self, from: Vec3, to: Vec3) -> Option<Hit>;

    /// Gives the grip at a hit: 1 holds fully, 0 does not hold.
    fn grip(&mut self, hit: &Hit) -> f32;
}

/// The tuning values of the climb. The bear copies them from its
/// exported fields. Angles are in radians, lengths in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimbParams {
    /// A surface steeper than this angle starts a climb.
    pub start_angle: f32,
    /// A surface flatter than this angle stops the climb.
    pub stop_angle: f32,
    /// The bear cannot hold a surface steeper than this angle (an
    /// overhang is more than 90 degrees).
    pub max_angle: f32,
    /// The radius of the body.
    pub radius: f32,
    /// The distance from the centre of the bear to its feet.
    pub half_height: f32,
    /// How far the paws reach past the body.
    pub reach: f32,
    /// The climb speed in metres per second.
    pub climb_speed: f32,
    /// The distance between the centre ray and the side rays.
    pub probe_spread: f32,
    /// How fast the bear turns to a new surface direction, in 1/s.
    pub normal_smoothing: f32,
    /// The gravity in m/s each second.
    pub gravity: f32,
    /// With this grip or more, the bear does not slide.
    pub firm_grip: f32,
    /// With this grip or less, the bear slides off completely.
    pub loose_grip: f32,
    /// The speed of the slow slide just above `loose_grip`, in m/s.
    pub slow_slide_speed: f32,
    /// How fast a slide stops on a surface with more grip, in m/s each
    /// second.
    pub slide_brake: f32,
    /// If the slide is faster than this speed, the bear lets go.
    pub let_go_slide_speed: f32,
    /// The stamina that a climb on a vertical wall uses each second.
    pub drain_per_second: f32,
    /// The part of the drain on the least steep surface (0 to 1).
    pub gentle_drain_factor: f32,
}

impl Default for ClimbParams {
    fn default() -> Self {
        Self {
            start_angle: 50f32.to_radians(),
            stop_angle: 45f32.to_radians(),
            max_angle: 110f32.to_radians(),
            radius: 0.11,
            half_height: 0.20,
            reach: 0.14,
            climb_speed: 0.7,
            probe_spread: 0.07,
            normal_smoothing: 14.0,
            gravity: 14.0,
            firm_grip: 0.7,
            loose_grip: 0.25,
            slow_slide_speed: 0.35,
            slide_brake: 6.0,
            let_go_slide_speed: 1.5,
            drain_per_second: 1.0 / 30.0,
            gentle_drain_factor: 0.3,
        }
    }
}

impl ClimbParams {
    /// The distance from the centre of the bear to the surface.
    fn hold(&self) -> f32 {
        self.radius + 0.01
    }
}

/// Gives the angle between a surface and flat ground, in radians.
/// Flat ground is 0. A vertical wall is a quarter turn.
pub fn slope_angle(normal: Vec3) -> f32 {
    normal.y.clamp(-1.0, 1.0).acos()
}

/// Gives the directions "right" and "up" on a wall, for a bear that
/// looks at the wall.
pub fn wall_axes(normal: Vec3) -> (Vec3, Vec3) {
    let up = (Vec3::Y - normal * normal.y).normalize_or_zero();
    // On flat ground, "up the wall" has no meaning. Use a fixed axis.
    let up = if up == Vec3::ZERO { Vec3::Z } else { up };
    (up.cross(normal), up)
}

/// Changes a move direction on the ground into a move direction on the
/// wall. `wish` is (x, z) in the world. Into the wall is up. Away from
/// the wall is down. The result is (right, up).
pub fn wall_input(wish: Vec2, normal: Vec3) -> Vec2 {
    let (right, _) = wall_axes(normal);
    let out = Vec3::new(normal.x, 0.0, normal.z).normalize_or_zero();
    let wish = Vec3::new(wish.x, 0.0, wish.y);
    Vec2::new(wish.dot(right), -wish.dot(out)).clamp_length_max(1.0)
}

/// Looks for a surface that the bear can start to climb. `wish` is the
/// unit move direction on the ground. The bear must push into the
/// surface, and the surface must be too steep to walk.
pub fn find_grab(
    surface: &mut impl Surface,
    centre: Vec3,
    wish: Vec3,
    p: &ClimbParams,
) -> Option<Hit> {
    let length = p.radius + p.reach;
    // The second ray points down a little. It finds a slope that the
    // bear stands on.
    let low = (wish - Vec3::Y * 0.7).normalize_or_zero();
    for dir in [wish, low] {
        let Some(hit) = surface.cast(centre, centre + dir * length) else {
            continue;
        };
        let angle = slope_angle(hit.normal);
        let out = Vec3::new(hit.normal.x, 0.0, hit.normal.z).normalize_or_zero();
        if angle > p.start_angle && angle <= p.max_angle && -wish.dot(out) > 0.35 {
            return Some(hit);
        }
    }
    None
}

/// Gives the new slide speed after one step. `along` is the gravity
/// along the wall.
///
/// - Firm grip: the slide stops.
/// - Between loose and firm grip: a slow slide with a top speed.
/// - Loose grip: the slide gets faster and faster (it keeps momentum).
pub fn slide_step(speed: f32, grip: f32, along: f32, dt: f32, p: &ClimbParams) -> f32 {
    let brake = p.slide_brake * dt;
    if grip >= p.firm_grip {
        return (speed - brake).max(0.0);
    }
    let faster = speed + along * (1.0 - grip) * dt;
    if grip <= p.loose_grip {
        return faster;
    }
    let range = (p.firm_grip - p.loose_grip).max(1e-3);
    let top = p.slow_slide_speed * (p.firm_grip - grip) / range;
    if speed > top {
        (speed - brake).max(top)
    } else {
        faster.min(top)
    }
}

/// Gives the stamina that the climb uses each second. A bear that
/// holds still on firm ground uses none. A steep surface uses more.
pub fn stamina_drain(angle: f32, moving: bool, sliding: bool, p: &ClimbParams) -> f32 {
    if !moving && !sliding {
        return 0.0;
    }
    let quarter = std::f32::consts::FRAC_PI_2;
    let range = (quarter - p.start_angle).max(1e-3);
    let steep = ((angle - p.start_angle) / range).clamp(0.0, 1.0);
    p.drain_per_second * (p.gentle_drain_factor + (1.0 - p.gentle_drain_factor) * steep)
}

/// The result of one climb step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// The bear is still on the wall.
    Hold,
    /// The bear came to ground that it can walk on. `feet` is the
    /// place to stand.
    Ground { feet: Vec3 },
    /// The bear lost the wall. It falls with this velocity.
    LetGo { velocity: Vec3 },
}

/// A bear on a wall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climb {
    /// The centre of the bear.
    pub centre: Vec3,
    /// The direction away from the wall. It changes smoothly.
    pub normal: Vec3,
    /// The angle of the surface below the bear, in radians.
    pub angle: f32,
    /// The grip of the surface below the bear.
    pub grip: f32,
    /// The speed of the slide down the wall in m/s.
    pub slide_speed: f32,
    /// The distance that the bear moved on the wall. The animation
    /// uses it.
    pub travel: f32,
}

impl Climb {
    /// Starts a climb at a hit. `velocity` is the velocity of the bear
    /// before the grab. On a surface without firm grip, the bear keeps
    /// its speed down the wall.
    pub fn start(surface: &mut impl Surface, hit: Hit, velocity: Vec3, p: &ClimbParams) -> Self {
        let grip = surface.grip(&hit);
        let (_, up) = wall_axes(hit.normal);
        let down_speed = (-velocity.dot(up)).max(0.0);
        Self {
            centre: hit.point + hit.normal * p.hold(),
            normal: hit.normal,
            angle: slope_angle(hit.normal),
            grip,
            slide_speed: if grip >= p.firm_grip { 0.0 } else { down_speed },
            travel: 0.0,
        }
    }

    /// Gives the velocity of the slide, down the wall.
    pub fn slide_velocity(&self) -> Vec3 {
        let (_, up) = wall_axes(self.normal);
        -up * self.slide_speed
    }

    /// Moves the bear for one step. `input` is (right, up) on the wall,
    /// with a length of 1 at most. The step uses 5 or 6 ray casts.
    pub fn step(
        &mut self,
        surface: &mut impl Surface,
        input: Vec2,
        dt: f32,
        p: &ClimbParams,
    ) -> Step {
        let (right, up) = wall_axes(self.normal);
        let hold = p.hold();

        // The slide.
        let along = p.gravity * slope_angle(self.normal).sin();
        self.slide_speed = slide_step(self.slide_speed, self.grip, along, dt, p);
        if self.slide_speed > p.let_go_slide_speed {
            return Step::LetGo {
                velocity: self.slide_velocity() + self.normal * 0.3,
            };
        }

        let velocity = (right * input.x + up * input.y) * p.climb_speed - up * self.slide_speed;
        let moved = velocity * dt;
        let dist = moved.length();
        let dir = if dist > 1e-6 { moved / dist } else { -up };

        // Ray 1: is something in the way? The body is long from head to
        // feet, thus the ray is longer up and down than to the side.
        if dist > 1e-6 {
            let body = p.radius + (p.half_height - p.radius) * dir.dot(up).abs();
            if let Some(hit) = surface.cast(self.centre, self.centre + dir * (dist + body)) {
                if slope_angle(hit.normal) < p.stop_angle {
                    return Step::Ground { feet: hit.point };
                }
                // A second wall in an inner corner: go onto it.
                if (hit.point - self.centre).length() <= dist + hold {
                    self.put_on(surface, hit, hit.point + hit.normal * hold, dt, p);
                    return Step::Hold;
                }
            }
        }

        // Ray 2: find the surface again below the new place.
        let target = self.centre + moved;
        let depth = self.normal * (hold + p.reach);
        let mut below = surface.cast(target, target - depth);
        if below.is_none() {
            // The bear went past an outer corner, for example the edge
            // of a ledge. Look back from behind the corner.
            let from = target - self.normal * (hold + 0.5 * p.reach);
            below = surface.cast(from, from - dir * (dist + p.half_height));
        }
        let Some(below) = below else {
            return Step::LetGo { velocity };
        };
        let angle = slope_angle(below.normal);
        if angle < p.stop_angle {
            return Step::Ground { feet: below.point };
        }
        if angle > p.max_angle {
            return Step::LetGo { velocity };
        }

        // Stay on the ray, at the hold distance from the surface. A
        // step along the hit normal moves a bear that holds still.
        let facing = self.normal.dot(below.normal);
        let centre = if facing > 0.5 {
            below.point + self.normal * (hold / facing)
        } else {
            below.point + below.normal * hold
        };

        // Rays 3 to 5: the mean direction of the surface near the bear
        // makes the turn at a corner smooth.
        let mut sum = below.normal;
        for offset in [up, right, -right] {
            let from = target + offset * p.probe_spread;
            if let Some(hit) = surface.cast(from, from - depth) {
                sum += hit.normal;
            }
        }
        let mut near = below;
        near.normal = sum.normalize_or(below.normal);
        self.put_on(surface, near, centre, dt, p);
        self.angle = angle;
        Step::Hold
    }

    /// Puts the bear at `centre` on the surface of `hit`.
    fn put_on(
        &mut self,
        surface: &mut impl Surface,
        hit: Hit,
        centre: Vec3,
        dt: f32,
        p: &ClimbParams,
    ) {
        let k = 1.0 - (-p.normal_smoothing * dt).exp();
        self.normal = self.normal.lerp(hit.normal, k).normalize_or(hit.normal);
        self.angle = slope_angle(hit.normal);
        self.grip = surface.grip(&hit);
        self.travel += (centre - self.centre).length();
        self.centre = centre;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// A test surface: the ground height is a function of x. The grip
    /// is the same at all places.
    struct Ground<F: Fn(f32) -> f32> {
        height: F,
        grip: f32,
        casts: usize,
    }

    impl<F: Fn(f32) -> f32> Ground<F> {
        fn new(height: F, grip: f32) -> Self {
            Self {
                height,
                grip,
                casts: 0,
            }
        }
    }

    impl<F: Fn(f32) -> f32> Surface for Ground<F> {
        fn cast(&mut self, from: Vec3, to: Vec3) -> Option<Hit> {
            self.casts += 1;
            let above = |p: Vec3| p.y - (self.height)(p.x);
            if above(from) < 0.0 {
                return None;
            }
            let steps = ((to - from).length() / 0.002).ceil().max(1.0) as usize;
            let mut a = 0.0;
            for i in 1..=steps {
                let b = i as f32 / steps as f32;
                if above(from.lerp(to, b)) < 0.0 {
                    let (mut lo, mut hi) = (a, b);
                    for _ in 0..30 {
                        let mid = (lo + hi) * 0.5;
                        if above(from.lerp(to, mid)) < 0.0 {
                            hi = mid;
                        } else {
                            lo = mid;
                        }
                    }
                    let point = from.lerp(to, lo);
                    let e = 1e-3;
                    let rise =
                        ((self.height)(point.x + e) - (self.height)(point.x - e)) / (2.0 * e);
                    return Some(Hit {
                        point,
                        normal: Vec3::new(-rise, 1.0, 0.0).normalize(),
                        id: 0,
                    });
                }
                a = b;
            }
            None
        }

        fn grip(&mut self, _hit: &Hit) -> f32 {
            self.grip
        }
    }

    /// Flat ground, then a ramp that starts at x = 0, then a flat top at
    /// `top` metres.
    fn ramp(degrees: f32, top: f32) -> impl Fn(f32) -> f32 {
        let rise = degrees.to_radians().tan();
        move |x: f32| (x * rise).clamp(0.0, top)
    }

    /// Puts a bear on the ramp, `height` metres up.
    fn on_ramp<F: Fn(f32) -> f32>(ground: &mut Ground<F>, height: f32) -> Climb {
        let p = ClimbParams::default();
        let from = Vec3::new(-1.0, height, 0.0);
        let hit = ground.cast(from, from + Vec3::X * 40.0).expect("the ramp");
        Climb::start(ground, hit, Vec3::ZERO, &p)
    }

    #[test]
    fn the_bear_grabs_a_steep_surface_that_it_pushes_into() {
        let p = ClimbParams::default();
        let mut steep = Ground::new(ramp(70.0, 3.0), 1.0);
        let centre = Vec3::new(-0.12, 0.2, 0.0);
        assert!(find_grab(&mut steep, centre, Vec3::X, &p).is_some());
        assert!(find_grab(&mut steep, centre, -Vec3::X, &p).is_none());
        assert!(find_grab(&mut steep, centre, Vec3::Z, &p).is_none());
        // Too far away.
        let far = Vec3::new(-0.6, 0.2, 0.0);
        assert!(find_grab(&mut steep, far, Vec3::X, &p).is_none());
        // A slope that the bear can walk does not start a climb.
        let mut gentle = Ground::new(ramp(40.0, 3.0), 1.0);
        assert!(find_grab(&mut gentle, centre, Vec3::X, &p).is_none());
    }

    #[test]
    fn the_bear_grabs_a_slope_that_it_stands_on() {
        let p = ClimbParams::default();
        let mut ground = Ground::new(ramp(55.0, 30.0), 1.0);
        // The capsule touches the slope 2 m up. The centre is 0.2 m
        // above the feet.
        let x = 2.0 / 55f32.to_radians().tan();
        let centre = Vec3::new(x - 0.19, 2.0 + 0.03, 0.0);
        assert!(find_grab(&mut ground, centre, Vec3::X, &p).is_some());
    }

    #[test]
    fn the_bear_climbs_a_ramp_and_stands_on_the_top() {
        let p = ClimbParams::default();
        for degrees in [55.0, 70.0, 88.0] {
            let mut ground = Ground::new(ramp(degrees, 3.0), 1.0);
            let mut climb = on_ramp(&mut ground, 0.5);
            let mut end = None;
            for _ in 0..1200 {
                match climb.step(&mut ground, Vec2::Y, DT, &p) {
                    Step::Hold => {}
                    other => {
                        end = Some(other);
                        break;
                    }
                }
            }
            let Some(Step::Ground { feet }) = end else {
                panic!("{degrees} degrees: the climb ended with {end:?}");
            };
            assert!((feet.y - 3.0).abs() < 0.02, "{degrees}: feet at {feet}");
            let edge = 3.0 / degrees.to_radians().tan();
            assert!(feet.x > edge - 0.05, "{degrees}: feet at {feet}");
        }
    }

    #[test]
    fn the_bear_climbs_down_to_the_ground() {
        let p = ClimbParams::default();
        let mut ground = Ground::new(ramp(75.0, 3.0), 1.0);
        let mut climb = on_ramp(&mut ground, 1.0);
        let mut end = None;
        for _ in 0..600 {
            let step = climb.step(&mut ground, -Vec2::Y, DT, &p);
            if step != Step::Hold {
                end = Some(step);
                break;
            }
        }
        let Some(Step::Ground { feet }) = end else {
            panic!("the climb ended with {end:?}");
        };
        assert!(feet.y.abs() < 0.02, "feet at {feet}");
    }

    #[test]
    fn a_bear_that_holds_still_does_not_move() {
        let p = ClimbParams::default();
        // The ramp gets steeper at x = 0.5: the bear holds on the corner.
        let corner = |x: f32| {
            if x < 0.5 {
                x.max(0.0) * 1.5
            } else {
                0.75 + (x - 0.5) * 6.0
            }
        };
        let mut ground = Ground::new(corner, 1.0);
        let mut climb = on_ramp(&mut ground, 0.8);
        let start = climb.centre;
        for _ in 0..600 {
            assert_eq!(climb.step(&mut ground, Vec2::ZERO, DT, &p), Step::Hold);
        }
        assert!((climb.centre - start).length() < 0.005, "{}", climb.centre);
        assert_eq!(climb.slide_speed, 0.0);
    }

    #[test]
    fn a_move_to_the_side_keeps_the_height() {
        let p = ClimbParams::default();
        let mut ground = Ground::new(ramp(80.0, 3.0), 1.0);
        let mut climb = on_ramp(&mut ground, 1.0);
        let start = climb.centre;
        for _ in 0..120 {
            assert_eq!(climb.step(&mut ground, Vec2::X, DT, &p), Step::Hold);
        }
        assert!((climb.centre.y - start.y).abs() < 0.005);
        let moved = (climb.centre.z - start.z).abs();
        assert!((moved - 2.0 * p.climb_speed).abs() < 0.02, "moved {moved}");
        assert!((climb.travel - moved).abs() < 0.02);
    }

    #[test]
    fn a_step_uses_few_rays() {
        let p = ClimbParams::default();
        let mut ground = Ground::new(ramp(70.0, 30.0), 1.0);
        let mut climb = on_ramp(&mut ground, 1.0);
        ground.casts = 0;
        climb.step(&mut ground, Vec2::Y, DT, &p);
        assert!(ground.casts <= 6, "{} rays", ground.casts);
    }

    #[test]
    fn medium_grip_makes_a_slow_slide() {
        let p = ClimbParams::default();
        let mut ground = Ground::new(ramp(70.0, 30.0), 0.45);
        let mut climb = on_ramp(&mut ground, 20.0);
        let start = climb.centre.y;
        for _ in 0..300 {
            assert_eq!(climb.step(&mut ground, Vec2::ZERO, DT, &p), Step::Hold);
        }
        let top = p.slow_slide_speed * (0.7 - 0.45) / (0.7 - 0.25);
        assert!((climb.slide_speed - top).abs() < 1e-4);
        assert!(climb.centre.y < start - 0.5, "the bear slides down");
        // The bear can still climb up, but slowly.
        let low = climb.centre.y;
        for _ in 0..60 {
            climb.step(&mut ground, Vec2::Y, DT, &p);
        }
        let gain = climb.centre.y - low;
        assert!(gain > 0.2 && gain < 0.6, "gain {gain}");
    }

    #[test]
    fn low_grip_makes_the_bear_slide_off_with_its_speed() {
        let p = ClimbParams::default();
        let mut ground = Ground::new(ramp(70.0, 30.0), 0.1);
        let mut climb = on_ramp(&mut ground, 20.0);
        let mut end = None;
        for _ in 0..120 {
            let step = climb.step(&mut ground, Vec2::Y, DT, &p);
            if step != Step::Hold {
                end = Some(step);
                break;
            }
        }
        let Some(Step::LetGo { velocity }) = end else {
            panic!("the climb ended with {end:?}");
        };
        assert!(velocity.y < -1.0, "velocity {velocity}");
        assert!(velocity.length() > p.let_go_slide_speed);
    }

    #[test]
    fn a_grab_on_a_slippery_surface_keeps_the_fall_speed() {
        let p = ClimbParams::default();
        let fall = Vec3::new(0.0, -3.0, 0.0);
        let mut slippery = Ground::new(ramp(70.0, 30.0), 0.4);
        let from = Vec3::new(-1.0, 5.0, 0.0);
        let hit = slippery.cast(from, from + Vec3::X * 5.0).unwrap();
        let climb = Climb::start(&mut slippery, hit, fall, &p);
        assert!(climb.slide_speed > 2.5);
        let mut firm = Ground::new(ramp(70.0, 30.0), 1.0);
        let climb = Climb::start(&mut firm, hit, fall, &p);
        assert_eq!(climb.slide_speed, 0.0);
    }

    #[test]
    fn slide_step_has_three_ranges() {
        let p = ClimbParams::default();
        // Firm grip stops a slide.
        assert!(slide_step(1.0, 0.9, 10.0, 0.1, &p) < 1.0);
        assert_eq!(slide_step(0.0, 0.9, 10.0, 0.1, &p), 0.0);
        // Loose grip has no top speed.
        let mut speed = 0.0;
        for _ in 0..100 {
            speed = slide_step(speed, 0.1, 10.0, 0.1, &p);
        }
        assert!(speed > 50.0);
        // Between them, less grip makes a faster slide.
        let top = |grip: f32| {
            let mut speed = 0.0;
            for _ in 0..100 {
                speed = slide_step(speed, grip, 10.0, 0.1, &p);
            }
            speed
        };
        assert!(top(0.3) > top(0.5) && top(0.5) > top(0.65));
        assert!(top(0.3) <= p.slow_slide_speed);
    }

    #[test]
    fn stamina_drains_only_when_the_bear_works() {
        let p = ClimbParams::default();
        let wall = 90f32.to_radians();
        let slope = 52f32.to_radians();
        assert_eq!(stamina_drain(wall, false, false, &p), 0.0);
        assert!((stamina_drain(wall, true, false, &p) - p.drain_per_second).abs() < 1e-6);
        assert!(stamina_drain(wall, false, true, &p) > 0.0);
        assert!(stamina_drain(slope, true, false, &p) < 0.5 * p.drain_per_second);
    }

    #[test]
    fn wall_input_turns_the_stick_to_the_wall() {
        // A wall that looks at +Z. The bear looks at -Z.
        let n = Vec3::new(0.0, 0.2, 1.0).normalize();
        let into = wall_input(Vec2::new(0.0, -1.0), n);
        assert!((into - Vec2::Y).length() < 1e-5, "{into}");
        let away = wall_input(Vec2::new(0.0, 1.0), n);
        assert!((away + Vec2::Y).length() < 1e-5, "{away}");
        let side = wall_input(Vec2::new(1.0, 0.0), n);
        assert!((side - Vec2::X).length() < 1e-5, "{side}");
    }
}
