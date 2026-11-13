//! Movement maths with no Godot types. Unit tests cover this module.

/// The timers that make a jump easy to time.
///
/// Coyote time lets the bear jump a short time after it leaves an edge.
/// The jump buffer keeps a jump press that comes a short time before
/// the landing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct JumpTimers {
    /// The time that is left to jump after the bear left the floor.
    coyote_left: f32,
    /// The time that is left for a jump press to start a jump.
    buffer_left: f32,
}

impl JumpTimers {
    /// Updates the timers for one physics step. Returns `true` if the
    /// bear must start a jump now.
    pub fn update(
        &mut self,
        delta: f32,
        on_floor: bool,
        jump_pressed: bool,
        coyote_time: f32,
        buffer_time: f32,
    ) -> bool {
        self.coyote_left = if on_floor {
            coyote_time
        } else {
            (self.coyote_left - delta).max(0.0)
        };
        self.buffer_left = if jump_pressed {
            buffer_time
        } else {
            (self.buffer_left - delta).max(0.0)
        };
        // A press on the floor always jumps, also with timers of 0.
        let can_jump = on_floor || self.coyote_left > 0.0;
        let wants_jump = jump_pressed || self.buffer_left > 0.0;
        if can_jump && wants_jump {
            self.coyote_left = 0.0;
            self.buffer_left = 0.0;
            return true;
        }
        false
    }
}

/// Gives the start speed of a jump that goes `height` metres up.
pub fn jump_speed(height: f32, gravity: f32) -> f32 {
    (2.0 * gravity.max(0.0) * height.max(0.0)).sqrt()
}

/// Moves a 2D velocity toward a target velocity. The change is
/// `max_change` at most.
pub fn approach(current: (f32, f32), target: (f32, f32), max_change: f32) -> (f32, f32) {
    let (dx, dy) = (target.0 - current.0, target.1 - current.1);
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= max_change || dist == 0.0 {
        return target;
    }
    let k = max_change / dist;
    (current.0 + dx * k, current.1 + dy * k)
}

/// Gives the shortest way from angle `from` to angle `to`, in radians.
pub fn angle_difference(from: f32, to: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let d = (to - from).rem_euclid(tau);
    if d > std::f32::consts::PI {
        d - tau
    } else {
        d
    }
}

/// Gives the distance from the target to the camera, such that the
/// camera and the line of sight stay above the ground.
///
/// `target` is the point that the camera looks at. `dir` is the unit
/// direction from the target to the camera. `wanted` is the distance
/// without ground. `clearance` is the least height above the ground.
/// `height` gives the ground height at a position (x, z).
pub fn clear_distance(
    target: [f32; 3],
    dir: [f32; 3],
    wanted: f32,
    clearance: f32,
    height: impl Fn(f32, f32) -> f32,
) -> f32 {
    const STEPS: usize = 24;
    let mut clear = 0.0;
    for i in 1..=STEPS {
        let t = wanted * i as f32 / STEPS as f32;
        // Near the target, the line can be near the ground: the bear
        // stands on it. The full clearance applies at the camera.
        let need = clearance * (t / wanted).min(1.0);
        let p = [
            target[0] + dir[0] * t,
            target[1] + dir[1] * t,
            target[2] + dir[2] * t,
        ];
        if p[1] < height(p[0], p[2]) + need {
            break;
        }
        clear = t;
    }
    clear
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn jump_on_the_floor() {
        let mut t = JumpTimers::default();
        assert!(!t.update(DT, true, false, 0.1, 0.1));
        assert!(t.update(DT, true, true, 0.1, 0.1));
    }

    #[test]
    fn coyote_time_lets_the_bear_jump_after_an_edge() {
        let mut t = JumpTimers::default();
        t.update(DT, true, false, 0.1, 0.1);
        // 3 frames in the air (0.05 s): the jump is permitted.
        for _ in 0..3 {
            assert!(!t.update(DT, false, false, 0.1, 0.1));
        }
        assert!(t.update(DT, false, true, 0.1, 0.1));
        // One jump only.
        assert!(!t.update(DT, false, true, 0.1, 0.1));
    }

    #[test]
    fn no_jump_after_the_coyote_time() {
        let mut t = JumpTimers::default();
        t.update(DT, true, false, 0.1, 0.1);
        for _ in 0..10 {
            t.update(DT, false, false, 0.1, 0.1);
        }
        assert!(!t.update(DT, false, true, 0.1, 0.0));
    }

    #[test]
    fn the_buffer_keeps_a_press_before_the_landing() {
        let mut t = JumpTimers::default();
        for _ in 0..30 {
            t.update(DT, false, false, 0.1, 0.12);
        }
        assert!(!t.update(DT, false, true, 0.1, 0.12));
        for _ in 0..4 {
            assert!(!t.update(DT, false, false, 0.1, 0.12));
        }
        assert!(
            t.update(DT, true, false, 0.1, 0.12),
            "the jump starts on the landing"
        );
    }

    #[test]
    fn jump_speed_reaches_the_height() {
        let v = jump_speed(0.5, 12.0);
        assert!((v * v / (2.0 * 12.0) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn approach_does_not_go_past_the_target() {
        assert_eq!(approach((0.0, 0.0), (3.0, 4.0), 10.0), (3.0, 4.0));
        let (x, y) = approach((0.0, 0.0), (3.0, 4.0), 1.0);
        assert!((x - 0.6).abs() < 1e-6 && (y - 0.8).abs() < 1e-6);
    }

    #[test]
    fn angle_difference_uses_the_short_way() {
        assert!((angle_difference(0.1, -0.1) + 0.2).abs() < 1e-6);
        assert!((angle_difference(3.0, -3.0) - 0.283_185).abs() < 1e-4);
    }

    #[test]
    fn the_camera_stops_before_a_hill() {
        // Flat ground, and a wall of 10 m that starts 2 m behind the bear.
        let ground = |x: f32, _z: f32| if x > 2.0 { 10.0 } else { 0.0 };
        let d = clear_distance([0.0, 0.4, 0.0], [1.0, 0.0, 0.0], 4.0, 0.2, ground);
        assert!(d <= 2.0 && d > 1.5, "distance {d}");
        // With no wall, the camera gets the full distance.
        let d = clear_distance([0.0, 0.4, 0.0], [-1.0, 0.0, 0.0], 4.0, 0.2, ground);
        assert_eq!(d, 4.0);
    }
}
