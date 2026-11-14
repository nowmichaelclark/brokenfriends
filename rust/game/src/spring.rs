//! A second-order spring with no Godot types. Unit tests cover this
//! module.
//!
//! The spring makes a value follow a target in a soft way. Three values
//! set its feel:
//! - `f`: the frequency in Hz. A higher value follows faster.
//! - `zeta`: the damping. Below 1, the value swings past the target and
//!   back. At 1 or more, it does not swing.
//! - `r`: the first response. At 0, the value starts slowly. Above 1,
//!   it goes past the target at once. Below 0, it first moves back.
//!
//! The maths is the system `y + k1 y' + k2 y'' = x + k3 x'`, as in the
//! talk "Giving Personality to Procedural Animations using Math" by
//! t3ssel8r. The code is our own.

use std::f32::consts::PI;
use std::ops::{Add, Mul, Sub};

/// A value that a spring can move: `f32`, `Vec2` or `Vec3`.
pub trait SpringValue:
    Copy + Add<Output = Self> + Sub<Output = Self> + Mul<f32, Output = Self>
{
}

impl<T> SpringValue for T where T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T> {}

/// A second-order spring. Call `update` one time in each step.
#[derive(Clone, Copy, Debug)]
pub struct SecondOrder<T> {
    k1: f32,
    k2: f32,
    k3: f32,
    /// The target of the last step.
    last_target: T,
    /// The value now.
    value: T,
    /// The speed of the value, for each second.
    speed: T,
}

impl<T: SpringValue> SecondOrder<T> {
    /// Makes a spring that rests at `start`.
    pub fn new(f: f32, zeta: f32, r: f32, start: T) -> Self {
        let mut spring = SecondOrder {
            k1: 0.0,
            k2: 1.0,
            k3: 0.0,
            last_target: start,
            value: start,
            speed: start * 0.0,
        };
        spring.tune(f, zeta, r);
        spring
    }

    /// Changes the feel of the spring. The value and the speed stay.
    pub fn tune(&mut self, f: f32, zeta: f32, r: f32) {
        let f = f.max(0.01);
        self.k1 = zeta.max(0.0) / (PI * f);
        self.k2 = 1.0 / (2.0 * PI * f).powi(2);
        self.k3 = r * zeta.max(0.0) / (2.0 * PI * f);
    }

    /// Moves the value one step of `dt` seconds toward `target`. Gives
    /// the new value.
    pub fn update(&mut self, dt: f32, target: T) -> T {
        if dt <= 0.0 {
            return self.value;
        }
        let target_speed = (target - self.last_target) * (1.0 / dt);
        self.last_target = target;
        // A long step with a stiff spring is not stable. A larger k2
        // makes the spring slower for this step, and keeps it stable.
        let k2 = self
            .k2
            .max(dt * dt / 2.0 + dt * self.k1 / 2.0)
            .max(dt * self.k1);
        self.value = self.value + self.speed * dt;
        let force = target + target_speed * self.k3 - self.value - self.speed * self.k1;
        self.speed = self.speed + force * (dt / k2);
        self.value
    }

    /// Gives the value now.
    #[allow(dead_code)] // For the tests and for S-122.
    pub fn value(&self) -> T {
        self.value
    }

    /// Puts the spring at rest at `value`.
    pub fn reset(&mut self, value: T) {
        self.last_target = value;
        self.value = value;
        self.speed = value * 0.0;
    }

    /// Adds to the speed of the value. Use this for a hit, for example
    /// a landing.
    pub fn kick(&mut self, speed: T) {
        self.speed = self.speed + speed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    const DT: f32 = 1.0 / 60.0;

    /// Runs a spring from 0 toward 1. Gives the largest value and the
    /// last value.
    fn step_response(f: f32, zeta: f32, r: f32, seconds: f32) -> (f32, f32) {
        let mut spring = SecondOrder::new(f, zeta, r, 0.0_f32);
        let mut largest = 0.0_f32;
        for _ in 0..(seconds / DT) as usize {
            largest = largest.max(spring.update(DT, 1.0));
        }
        (largest, spring.value())
    }

    #[test]
    fn the_value_comes_to_the_target() {
        for (f, zeta, r) in [(1.0, 1.0, 0.0), (3.0, 0.4, 0.0), (5.0, 0.25, 2.0)] {
            let (_, last) = step_response(f, zeta, r, 6.0);
            assert!((last - 1.0).abs() < 1e-3, "f {f} zeta {zeta}: {last}");
        }
    }

    #[test]
    fn full_damping_does_not_swing_past_the_target() {
        let (largest, _) = step_response(2.0, 1.0, 0.0, 4.0);
        assert!(largest <= 1.0 + 1e-3, "largest {largest}");
    }

    #[test]
    fn low_damping_swings_past_the_target() {
        let (largest, _) = step_response(2.0, 0.3, 0.0, 4.0);
        assert!(largest > 1.2, "largest {largest}");
    }

    #[test]
    fn a_higher_frequency_follows_faster() {
        let after = |f: f32| {
            let mut spring = SecondOrder::new(f, 1.0, 0.0, 0.0_f32);
            for _ in 0..6 {
                spring.update(DT, 1.0);
            }
            spring.value()
        };
        assert!(after(6.0) > after(2.0) * 2.0);
    }

    #[test]
    fn a_negative_response_moves_back_first() {
        let mut spring = SecondOrder::new(2.0, 1.0, -2.0, 0.0_f32);
        spring.update(DT, 0.0);
        spring.update(DT, 1.0);
        assert!(spring.update(DT, 1.0) < 0.0);
    }

    #[test]
    fn a_long_step_stays_stable() {
        let mut spring = SecondOrder::new(12.0, 0.2, 0.0, 0.0_f32);
        for _ in 0..200 {
            let value = spring.update(0.25, 1.0);
            assert!(value.is_finite() && value.abs() < 10.0, "value {value}");
        }
        assert!((spring.value() - 1.0).abs() < 0.05);
    }

    #[test]
    fn a_kick_moves_the_value_and_it_comes_back() {
        let mut spring = SecondOrder::new(3.5, 0.4, 0.0, 0.0_f32);
        spring.kick(4.0);
        let mut largest = 0.0_f32;
        for _ in 0..240 {
            largest = largest.max(spring.update(DT, 0.0));
        }
        assert!(largest > 0.05, "largest {largest}");
        assert!(spring.value().abs() < 1e-3);
    }

    #[test]
    fn a_vector_follows_its_target() {
        let target = Vec3::new(1.0, -2.0, 0.5);
        let mut spring = SecondOrder::new(3.0, 0.6, 0.0, Vec3::ZERO);
        for _ in 0..300 {
            spring.update(DT, target);
        }
        assert!((spring.value() - target).length() < 1e-3);
        spring.reset(Vec3::ZERO);
        assert_eq!(spring.update(DT, Vec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn a_step_of_zero_seconds_changes_nothing() {
        let mut spring = SecondOrder::new(3.0, 0.6, 0.0, 0.5_f32);
        assert_eq!(spring.update(0.0, 9.0), 0.5);
    }
}
