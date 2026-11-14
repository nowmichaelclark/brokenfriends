//! A free camera for debug work.

use crate::dev_menu;
use crate::terrain::TERRAIN_GROUP;
use godot::classes::input::MouseMode;
use godot::classes::{
    Camera3D, ICamera3D, Input, InputEvent, InputEventKey, InputEventMouseMotion,
};
use godot::global::{JoyAxis, JoyButton, Key, MouseButton};
use godot::prelude::*;

/// The group that holds the fly camera. The dev menu finds it here.
pub const FLY_CAMERA_GROUP: &str = "bf_fly_camera";

/// A camera that flies. Push F2 to change between this camera and the
/// camera of the game. Hold the right mouse button to look around.
/// Move with W, A, S, D. Go down with Q and up with E. Hold Shift to go
/// faster. On a gamepad, the dev menu changes the camera. The left stick
/// moves, the right stick looks, the triggers go down and up, and the
/// right shoulder button goes faster.
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

    /// The turn speed with the right stick, in degrees per second.
    #[export]
    #[init(val = 90.0)]
    stick_speed_degrees: f32,

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
        self.base_mut().add_to_group(FLY_CAMERA_GROUP);
        self.base_mut()
            .add_to_group(crate::view_distance::VIEW_CAMERA_GROUP);
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
        if !self.base().is_current() || dev_menu::is_open() {
            return;
        }
        let input = Input::singleton();
        self.look_with_stick(delta as f32);
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
        let pad = |a: JoyAxis| {
            let v = input.get_joy_axis(0, a);
            if v.abs() > 0.2 {
                v
            } else {
                0.0
            }
        };
        let mut local = Vector3::new(
            axis(Key::A, Key::D) + pad(JoyAxis::LEFT_X),
            axis(Key::Q, Key::E) + pad(JoyAxis::TRIGGER_RIGHT) - pad(JoyAxis::TRIGGER_LEFT),
            axis(Key::W, Key::S) + pad(JoyAxis::LEFT_Y),
        );
        if local.length() > 1.0 {
            local = local.normalized();
        }
        if local == Vector3::ZERO {
            return;
        }
        let mut speed = self.speed;
        if input.is_physical_key_pressed(Key::SHIFT)
            || input.is_joy_button_pressed(0, JoyButton::RIGHT_SHOULDER)
        {
            speed *= self.fast_factor;
        }
        let basis = self.base().get_global_transform().basis;
        let step = basis * local * speed * delta as f32;
        let position = self.base().get_global_position() + step;
        self.base_mut().set_global_position(position);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::F2 {
                self.toggle();
            }
            return;
        }
        if !self.base().is_current() || dev_menu::is_open() {
            return;
        }
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

impl BfFlyCamera {
    /// Turns the camera with the right stick.
    fn look_with_stick(&mut self, delta: f32) {
        let input = Input::singleton();
        let stick = Vector2::new(
            input.get_joy_axis(0, JoyAxis::RIGHT_X),
            input.get_joy_axis(0, JoyAxis::RIGHT_Y),
        );
        if stick.length() <= 0.2 {
            return;
        }
        let k = self.stick_speed_degrees.to_radians() * delta;
        self.yaw -= stick.x * k;
        self.pitch = (self.pitch - stick.y * k).clamp(-1.55, 1.55);
        let rotation = Vector3::new(self.pitch, self.yaw, 0.0);
        self.base_mut().set_rotation(rotation);
    }
}

#[godot_api]
impl BfFlyCamera {
    /// Tells if this camera is the active camera.
    #[func]
    pub fn is_active(&self) -> bool {
        self.base().is_current()
    }

    /// Changes between this camera and the camera of the game (F2).
    #[func]
    fn toggle(&mut self) {
        if self.base().is_current() {
            // Godot then uses the camera that was active before.
            self.base_mut().clear_current();
            Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
            return;
        }
        // Start at the position of the game camera.
        if let Some(viewport) = self.base().get_viewport() {
            if let Some(camera) = viewport.get_camera_3d() {
                let transform = camera.get_global_transform();
                self.base_mut().set_global_transform(transform);
                let rotation = self.base().get_rotation();
                self.pitch = rotation.x;
                self.yaw = rotation.y;
            }
        }
        self.base_mut().make_current();
    }

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
