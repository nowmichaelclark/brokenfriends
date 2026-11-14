//! A limb that dangles on a thread (R12), with no Godot types. Unit
//! tests cover this module.
//!
//! The thread and the limb are a chain of Verlet points (Jakobsen,
//! "Advanced Character Physics", GDC 2001). A Verlet point keeps its
//! last place instead of a velocity. Fixed distances join the points.
//! The first point is the socket on the body. The thread points come
//! next, then the two ends of the limb: the joint end and the tip.

use glam::Vec3;

/// The number of points: the socket, three thread points and the two
/// ends of the limb.
pub const POINTS: usize = 6;
/// The index of the joint end of the limb.
pub const LIMB_JOINT: usize = POINTS - 2;
/// The index of the tip of the limb.
pub const LIMB_TIP: usize = POINTS - 1;

/// The tuning values of a dangling limb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreadParams {
    /// The length of the thread from the socket to the limb, in metres.
    pub length: f32,
    /// The gravity in m/s each second.
    pub gravity: f32,
    /// The part of the speed that each point keeps in one step at 60 Hz.
    pub keep: f32,
    /// No point goes below the floor plus this distance, in metres.
    pub radius: f32,
}

impl Default for ThreadParams {
    fn default() -> ThreadParams {
        ThreadParams {
            length: 0.035,
            gravity: 14.0,
            keep: 0.97,
            radius: 0.02,
        }
    }
}

/// A limb on a thread.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thread {
    pub points: [Vec3; POINTS],
    last: [Vec3; POINTS],
    /// The rest distance between point `i` and point `i + 1`.
    rest: [f32; POINTS - 1],
}

impl Thread {
    /// Makes a straight thread from the socket along `dir`. The limb is
    /// `limb_length` long.
    pub fn new(socket: Vec3, dir: Vec3, limb_length: f32, p: &ThreadParams) -> Thread {
        let dir = dir.normalize_or(-Vec3::Y);
        let segment = p.length.max(0.001) / (POINTS - 2) as f32;
        let mut rest = [segment; POINTS - 1];
        rest[POINTS - 2] = limb_length.max(0.01);
        let mut points = [socket; POINTS];
        for i in 1..POINTS {
            points[i] = points[i - 1] + dir * rest[i - 1];
        }
        Thread {
            points,
            last: points,
            rest,
        }
    }

    /// One step. `socket` is the new place of the socket. `floor_y` is
    /// the height of the ground below the bear.
    pub fn step(&mut self, dt: f32, socket: Vec3, floor_y: f32, p: &ThreadParams) {
        let dt = dt.clamp(0.0, 0.05);
        // The damping is the same at each step rate.
        let keep = p.keep.clamp(0.0, 1.0).powf(dt * 60.0);
        let gravity = Vec3::new(0.0, -p.gravity, 0.0) * dt * dt;
        for i in 1..POINTS {
            let now = self.points[i];
            self.points[i] += (now - self.last[i]) * keep + gravity;
            self.last[i] = now;
        }
        self.points[0] = socket;
        self.last[0] = socket;
        for _ in 0..10 {
            for i in 0..POINTS - 1 {
                let (a, b) = (self.points[i], self.points[i + 1]);
                let d = b - a;
                let len = d.length();
                if len < 1e-6 {
                    continue;
                }
                let fix = d * ((len - self.rest[i]) / len);
                if i == 0 {
                    // The socket does not move.
                    self.points[1] -= fix;
                } else {
                    self.points[i] += fix * 0.5;
                    self.points[i + 1] -= fix * 0.5;
                }
            }
            for i in 1..POINTS {
                let low = floor_y + p.radius;
                if self.points[i].y < low {
                    self.points[i].y = low;
                    // Friction: the point loses its motion on the ground.
                    let last = self.last[i];
                    self.last[i] = Vec3::new(
                        last.x + (self.points[i].x - last.x) * 0.5,
                        last.y,
                        last.z + (self.points[i].z - last.z) * 0.5,
                    );
                }
            }
        }
    }

    /// The joint end of the limb and its direction to the tip.
    pub fn limb(&self) -> (Vec3, Vec3) {
        let a = self.points[LIMB_JOINT];
        let dir = (self.points[LIMB_TIP] - a).normalize_or(-Vec3::Y);
        (a, dir)
    }

    /// The velocity of the tip of the limb, in m/s.
    pub fn tip_velocity(&self, dt: f32) -> Vec3 {
        (self.points[LIMB_TIP] - self.last[LIMB_TIP]) / dt.max(1e-4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_limb_on_a_thread_swings_down_and_keeps_its_length() {
        let p = ThreadParams::default();
        let socket = Vec3::new(0.0, 1.0, 0.0);
        // It starts to the side, straight out.
        let mut t = Thread::new(socket, Vec3::X, 0.15, &p);
        for _ in 0..240 {
            t.step(1.0 / 60.0, socket, -10.0, &p);
        }
        let (a, dir) = t.limb();
        assert!(dir.y < -0.95, "the limb hangs down: {dir}");
        assert!(a.y < socket.y);
        let length = t.points[LIMB_TIP].distance(a);
        assert!((length - 0.15).abs() < 0.005, "{length}");
        let thread: f32 = (0..LIMB_JOINT)
            .map(|i| t.points[i].distance(t.points[i + 1]))
            .sum();
        assert!((thread - p.length).abs() < 0.005, "{thread}");
    }

    #[test]
    fn the_limb_stays_above_the_floor_and_follows_the_socket() {
        let p = ThreadParams::default();
        let mut socket = Vec3::new(0.0, 0.1, 0.0);
        let mut t = Thread::new(socket, -Vec3::Y, 0.12, &p);
        for _ in 0..300 {
            socket.x += 0.01;
            t.step(1.0 / 60.0, socket, 0.0, &p);
            for q in &t.points[1..] {
                assert!(q.y >= p.radius - 1e-4, "{q}");
            }
        }
        // The limb drags behind the socket.
        assert!(t.points[LIMB_TIP].x < socket.x);
        assert!(socket.x - t.points[LIMB_TIP].x < 0.25);
        assert!(t.tip_velocity(1.0 / 60.0).x > 0.0);
    }
}
