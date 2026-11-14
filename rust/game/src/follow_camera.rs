//! The third-person camera.

use crate::bear::BEAR_GROUP;
use crate::motion::clear_distance;
use crate::terrain::TERRAIN_GROUP;
use godot::classes::input::MouseMode;
use godot::classes::{
    Camera3D, ICamera3D, Input, InputEvent, InputEventKey, InputEventMouseMotion,
};
use godot::global::{JoyAxis, Key, MouseButton};
use godot::prelude::*;

/// The group that holds the follow camera. The bear finds it there.
pub const FOLLOW_CAMERA_GROUP: &str = "bf_follow_camera";

/// A camera that turns around the bear. The mouse or the right stick
/// turns it. It does not go into the terrain. Push Esc to release the
/// mouse. Click the window to hold the mouse again.
#[derive(GodotClass)]
#[class(base = Camera3D, init)]
pub struct BfFollowCamera {
    /// The distance from the bear in metres.
    #[export]
    #[init(val = 1.8)]
    distance: f32,

    /// The height of the point that the camera looks at, above the feet
    /// of the bear, in metres.
    #[export]
    #[init(val = 0.38)]
    look_height: f32,

    /// The least height of the camera above the ground in metres.
    #[export]
    #[init(val = 0.15)]
    ground_clearance: f32,

    /// The turn for each pixel of mouse movement, in degrees.
    #[export]
    #[init(val = 0.12)]
    mouse_sensitivity: f32,

    /// The turn speed with the right stick, in degrees per second.
    #[export]
    #[init(val = 150.0)]
    stick_speed_degrees: f32,

    /// The lowest angle of the camera in degrees (looks up at the bear).
    #[export]
    #[init(val = -35.0)]
    min_pitch_degrees: f32,

    /// The highest angle of the camera in degrees (looks down at the bear).
    #[export]
    #[init(val = 75.0)]
    max_pitch_degrees: f32,

    /// How fast the camera goes back to its full distance, in metres
    /// per second. It comes nearer without delay.
    #[export]
    #[init(val = 4.0)]
    zoom_out_speed: f32,

    /// The angle around the bear in radians.
    yaw: f32,
    /// The angle above the bear in radians.
    #[init(val = 0.35)]
    pitch: f32,
    current_distance: f32,
    base: Base<Camera3D>,
}

#[godot_api]
impl ICamera3D for BfFollowCamera {
    fn ready(&mut self) {
        self.base_mut().add_to_group(FOLLOW_CAMERA_GROUP);
        self.base_mut()
            .add_to_group(crate::view_distance::VIEW_CAMERA_GROUP);
        // The camera sets its own position in the world.
        self.base_mut().set_as_top_level(true);
        self.current_distance = self.distance;
        Input::singleton().set_mouse_mode(MouseMode::CAPTURED);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if !self.base().is_current() || crate::dev_menu::is_open() {
            return;
        }
        let mut input = Input::singleton();
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if key.is_pressed() && key.get_keycode() == Key::ESCAPE {
                input.set_mouse_mode(MouseMode::VISIBLE);
            }
            return;
        }
        if input.get_mouse_mode() != MouseMode::CAPTURED {
            if input.is_mouse_button_pressed(MouseButton::LEFT) {
                input.set_mouse_mode(MouseMode::CAPTURED);
            }
            return;
        }
        if let Ok(motion) = event.try_cast::<InputEventMouseMotion>() {
            let relative = motion.get_relative();
            let k = self.mouse_sensitivity.to_radians();
            self.turn(-relative.x * k, relative.y * k);
        }
    }

    fn process(&mut self, delta: f64) {
        let delta = delta as f32;
        let tree = self.base().get_tree();
        let Some(bear) = tree.get_first_node_in_group(BEAR_GROUP) else {
            return;
        };
        let Ok(bear) = bear.try_cast::<Node3D>() else {
            return;
        };

        let input = Input::singleton();
        let stick = Vector2::new(
            input.get_joy_axis(0, JoyAxis::RIGHT_X),
            input.get_joy_axis(0, JoyAxis::RIGHT_Y),
        );
        if stick.length() > 0.2 && self.base().is_current() && !crate::dev_menu::is_open() {
            let k = self.stick_speed_degrees.to_radians() * delta;
            self.turn(-stick.x * k, stick.y * k);
        }

        let target = bear.get_global_position() + Vector3::new(0.0, self.look_height, 0.0);
        // The direction from the target to the camera.
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let dir = Vector3::new(sin_yaw * cos_pitch, sin_pitch, cos_yaw * cos_pitch);

        // Come nearer if the ground is in the way.
        let mut wanted = self.distance;
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let height = |x: f32, z: f32| {
                terrain
                    .call("height_at", &[x.to_variant(), z.to_variant()])
                    .to::<f32>()
            };
            // `clear_distance` wants `Fn`: keep the node in a cell.
            let height = std::cell::RefCell::new(height);
            wanted = clear_distance(
                [target.x, target.y, target.z],
                [dir.x, dir.y, dir.z],
                self.distance,
                self.ground_clearance,
                |x, z| (height.borrow_mut())(x, z),
            );
        }
        self.current_distance = if wanted < self.current_distance {
            wanted
        } else {
            (self.current_distance + self.zoom_out_speed * delta).min(wanted)
        };

        let position = target + dir * self.current_distance.max(0.05);
        self.base_mut().set_global_position(position);
        self.base_mut().look_at(target);
    }
}

impl BfFollowCamera {
    fn turn(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        let lo = self.min_pitch_degrees.to_radians();
        let hi = self.max_pitch_degrees.to_radians().max(lo);
        self.pitch = (self.pitch + pitch).clamp(lo, hi.min(1.55));
    }
}
