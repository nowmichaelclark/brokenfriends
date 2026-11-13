//! A free camera for debug work.

use crate::terrain::TERRAIN_GROUP;
use godot::classes::input::MouseMode;
use godot::classes::{Camera3D, ICamera3D, Input, InputEvent, InputEventMouseMotion};
use godot::global::{Key, MouseButton};
use godot::prelude::*;

/// A camera that flies. Hold the right mouse button to look around.
/// Move with W, A, S, D. Go down with Q and up with E. Hold Shift to go
/// faster.
#[derive(GodotClass)]
#[class(base = Camera3D, init)]
pub struct BfFlyCamera {
    /// The speed in metres per second.
    #[export]
    #[init(val = 20.0)]
    speed: f32,

    /// The speed is multiplied by this value while Shift is down.
    #[export]
    #[init(val = 8.0)]
    fast_factor: f32,

    /// The turn for each pixel of mouse movement, in degrees.
    #[export]
    #[init(val = 0.15)]
    mouse_sensitivity: f32,

    /// The least height above the ground after a new zone shows, in metres.
    #[export]
    #[init(val = 30.0)]
    start_height_m: f32,

    yaw: f32,
    pitch: f32,
    base: Base<Camera3D>,
}

#[godot_api]
impl ICamera3D for BfFlyCamera {
    fn ready(&mut self) {
        let rotation = self.base().get_rotation();
        self.pitch = rotation.x;
        self.yaw = rotation.y;

        // When a new zone shows, go above the ground.
        let tree = self.base().get_tree();
        if let Some(terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("on_zone_changed");
            let mut terrain = terrain;
            terrain.connect("zone_changed", &on_zone);
        }
    }

    fn process(&mut self, delta: f64) {
        let input = Input::singleton();
        let looking = input.is_mouse_button_pressed(MouseButton::RIGHT);
        let mode = if looking {
            MouseMode::CAPTURED
        } else {
            MouseMode::VISIBLE
        };
        if input.get_mouse_mode() != mode {
            Input::singleton().set_mouse_mode(mode);
        }

        let axis = |negative: Key, positive: Key| {
            let n = input.is_physical_key_pressed(negative) as i32;
            let p = input.is_physical_key_pressed(positive) as i32;
            (p - n) as f32
        };
        let local = Vector3::new(
            axis(Key::A, Key::D),
            axis(Key::Q, Key::E),
            axis(Key::W, Key::S),
        );
        if local == Vector3::ZERO {
            return;
        }
        let mut speed = self.speed;
        if input.is_physical_key_pressed(Key::SHIFT) {
            speed *= self.fast_factor;
        }
        let basis = self.base().get_global_transform().basis;
        let step = basis * local.normalized() * speed * delta as f32;
        let position = self.base().get_global_position() + step;
        self.base_mut().set_global_position(position);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(motion) = event.try_cast::<InputEventMouseMotion>() else {
            return;
        };
        if Input::singleton().get_mouse_mode() != MouseMode::CAPTURED {
            return;
        }
        let relative = motion.get_relative();
        let k = self.mouse_sensitivity.to_radians();
        self.yaw -= relative.x * k;
        self.pitch = (self.pitch - relative.y * k).clamp(-1.55, 1.55);
        let rotation = Vector3::new(self.pitch, self.yaw, 0.0);
        self.base_mut().set_rotation(rotation);
    }
}

#[godot_api]
impl BfFlyCamera {
    /// Moves the camera up if the new ground is above it.
    #[func]
    fn on_zone_changed(&mut self) {
        let tree = self.base().get_tree();
        let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) else {
            return;
        };
        let mut position = self.base().get_global_position();
        let ground = terrain.call(
            "height_at",
            &[position.x.to_variant(), position.z.to_variant()],
        );
        let least = ground.try_to::<f32>().unwrap_or(0.0) + self.start_height_m;
        if position.y < least {
            position.y = least;
            self.base_mut().set_global_position(position);
        }
    }
}
