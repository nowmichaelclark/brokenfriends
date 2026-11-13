//! The teddy bear: the player.

use crate::follow_camera::FOLLOW_CAMERA_GROUP;
use crate::motion::{angle_difference, approach, jump_speed, JumpTimers};
use crate::terrain::TERRAIN_GROUP;
use godot::classes::{
    CapsuleMesh, CapsuleShape3D, CharacterBody3D, CollisionShape3D, ICharacterBody3D, Input, Mesh,
    MeshInstance3D, SphereMesh, StandardMaterial3D,
};
use godot::global::{JoyAxis, JoyButton, Key};
use godot::prelude::*;

/// The group that holds the bear. Other nodes find it there.
pub const BEAR_GROUP: &str = "bf_bear";

/// The teddy bear. It is 0.40 m tall. It walks, jogs and jumps.
///
/// Keys: W, A, S, D or the arrow keys to move, Shift to jog, Space to
/// jump. Gamepad: left stick to move, the bottom button to jump, the
/// right shoulder button to jog.
#[derive(GodotClass)]
#[class(base = CharacterBody3D, init)]
pub struct BfBear {
    /// The walk speed in metres per second.
    #[export]
    #[init(val = 1.4)]
    walk_speed: f32,

    /// The jog speed in metres per second (hold Shift).
    #[export]
    #[init(val = 3.2)]
    jog_speed: f32,

    /// How fast the bear gets its speed on the ground, in m/s each second.
    #[export]
    #[init(val = 18.0)]
    acceleration: f32,

    /// How fast the bear stops on the ground, in m/s each second.
    #[export]
    #[init(val = 24.0)]
    deceleration: f32,

    /// The part of the control that the bear has in the air (0 to 1).
    #[export]
    #[init(val = 0.45)]
    air_control: f32,

    /// The height of a jump in metres.
    #[export]
    #[init(val = 0.55)]
    jump_height: f32,

    /// The gravity on the bear in m/s each second.
    #[export]
    #[init(val = 14.0)]
    fall_acceleration: f32,

    /// The gravity is multiplied by this value while the bear falls.
    #[export]
    #[init(val = 1.4)]
    fall_gravity_factor: f32,

    /// The highest fall speed in metres per second.
    #[export]
    #[init(val = 25.0)]
    max_fall_speed: f32,

    /// The time after an edge in which the bear can jump, in seconds.
    #[export]
    #[init(val = 0.12)]
    coyote_time: f32,

    /// The time before a landing in which a jump press counts, in seconds.
    #[export]
    #[init(val = 0.14)]
    jump_buffer_time: f32,

    /// The steepest slope that the bear can walk, in degrees.
    #[export]
    #[init(val = 50.0)]
    max_slope_degrees: f32,

    /// How fast the bear turns to its move direction, in degrees per second.
    #[export]
    #[init(val = 720.0)]
    turn_speed_degrees: f32,

    /// The stick must move more than this value (0 to 1).
    #[export]
    #[init(val = 0.2)]
    stick_dead_zone: f32,

    timers: JumpTimers,
    jump_was_down: bool,
    /// The parts of the body. They turn; the collision shape does not.
    body: Option<Gd<Node3D>>,
    base: Base<CharacterBody3D>,
}

#[godot_api]
impl ICharacterBody3D for BfBear {
    fn ready(&mut self) {
        self.base_mut().add_to_group(BEAR_GROUP);
        self.build_body();

        // When a new zone shows, go to its start point.
        let tree = self.base().get_tree();
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("go_to_start");
            terrain.connect("zone_changed", &on_zone);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let delta = delta as f32;
        let max_slope = self.max_slope_degrees.to_radians();
        self.base_mut().set_floor_max_angle(max_slope);
        let on_floor = self.base().is_on_floor();
        let (wish, jog, jump_down) = self.read_input();

        let mut velocity = self.base().get_velocity();

        // Speed on the ground plane.
        let speed = if jog { self.jog_speed } else { self.walk_speed };
        let target = (wish.x * speed, wish.y * speed);
        let rate = if wish == Vector2::ZERO {
            self.deceleration
        } else {
            self.acceleration
        };
        let control = if on_floor { 1.0 } else { self.air_control };
        let (vx, vz) = approach((velocity.x, velocity.z), target, rate * control * delta);
        velocity.x = vx;
        velocity.z = vz;

        // Gravity and the jump.
        if on_floor {
            velocity.y = velocity.y.max(0.0);
        } else {
            let factor = if velocity.y < 0.0 {
                self.fall_gravity_factor
            } else {
                1.0
            };
            velocity.y =
                (velocity.y - self.fall_acceleration * factor * delta).max(-self.max_fall_speed);
        }
        let jump_pressed = jump_down && !self.jump_was_down;
        self.jump_was_down = jump_down;
        if self.timers.update(
            delta,
            on_floor,
            jump_pressed,
            self.coyote_time,
            self.jump_buffer_time,
        ) {
            velocity.y = jump_speed(self.jump_height, self.fall_acceleration);
        }

        self.base_mut().set_velocity(velocity);
        self.base_mut().move_and_slide();
        self.stay_above_ground();

        // Turn the body to the move direction.
        if wish != Vector2::ZERO {
            if let Some(body) = &mut self.body {
                let wanted = wish.x.atan2(wish.y);
                let mut rotation = body.get_rotation();
                let step = self.turn_speed_degrees.to_radians() * delta;
                rotation.y += angle_difference(rotation.y, wanted).clamp(-step, step);
                body.set_rotation(rotation);
            }
        }
    }
}

#[godot_api]
impl BfBear {
    /// Puts the bear on the start point of the zone: the flattest place
    /// near the centre.
    #[func]
    pub fn go_to_start(&mut self) {
        let tree = self.base().get_tree();
        let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) else {
            return;
        };
        let mut height = |x: f32, z: f32| {
            terrain
                .call("height_at", &[x.to_variant(), z.to_variant()])
                .to::<f32>()
        };
        let mut best = (f32::MAX, 0.0, 0.0);
        for iz in -10..=10 {
            for ix in -10..=10 {
                let (x, z) = (ix as f32 * 20.0, iz as f32 * 20.0);
                let h = height(x, z);
                let rise = (height(x + 2.0, z) - h).abs() + (height(x, z + 2.0) - h).abs();
                // A flat place wins. Of two flat places, the nearer one wins.
                let score = rise + (x.abs() + z.abs()) * 0.0005;
                if score < best.0 {
                    best = (score, x, z);
                }
            }
        }
        let (_, x, z) = best;
        let position = Vector3::new(x, height(x, z) + 0.3, z);
        self.base_mut().set_global_position(position);
        self.base_mut().set_velocity(Vector3::ZERO);
    }
}

impl BfBear {
    /// Reads the keys and the gamepad. Gives the move direction on the
    /// ground (x, z) with a length of 1 at most, the jog state and the
    /// jump state.
    fn read_input(&self) -> (Vector2, bool, bool) {
        let tree = self.base().get_tree();
        let Some(camera) = tree.get_first_node_in_group(FOLLOW_CAMERA_GROUP) else {
            return (Vector2::ZERO, false, false);
        };
        let Ok(camera) = camera.try_cast::<godot::classes::Camera3D>() else {
            return (Vector2::ZERO, false, false);
        };
        // While a debug camera is active, the bear does not move.
        if !camera.is_current() {
            return (Vector2::ZERO, false, false);
        }

        let input = Input::singleton();
        let key = |k: Key| input.is_physical_key_pressed(k);
        let axis = |negative: bool, positive: bool| (positive as i32 - negative as i32) as f32;
        let mut stick = Vector2::new(
            axis(
                key(Key::A) || key(Key::LEFT),
                key(Key::D) || key(Key::RIGHT),
            ),
            axis(key(Key::W) || key(Key::UP), key(Key::S) || key(Key::DOWN)),
        );
        let pad = Vector2::new(
            input.get_joy_axis(0, JoyAxis::LEFT_X),
            input.get_joy_axis(0, JoyAxis::LEFT_Y),
        );
        if pad.length() > self.stick_dead_zone {
            stick += pad;
        }
        if stick.length() > 1.0 {
            stick = stick.normalized();
        }

        // "Up" on the stick is "away from the camera".
        let yaw = camera.get_global_rotation().y;
        let (sin, cos) = yaw.sin_cos();
        let wish = Vector2::new(
            stick.x * cos + stick.y * sin,
            -stick.x * sin + stick.y * cos,
        );

        let jog = key(Key::SHIFT) || input.is_joy_button_pressed(0, JoyButton::RIGHT_SHOULDER);
        let jump = key(Key::SPACE) || input.is_joy_button_pressed(0, JoyButton::A);
        (wish, jog, jump)
    }

    /// If the bear is below the ground, puts it back on the ground.
    /// This can occur before Terrain3D makes the collision of an area.
    fn stay_above_ground(&mut self) {
        let tree = self.base().get_tree();
        let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) else {
            return;
        };
        let mut position = self.base().get_global_position();
        let ground = terrain
            .call(
                "height_at",
                &[position.x.to_variant(), position.z.to_variant()],
            )
            .to::<f32>();
        if position.y < ground - 0.25 {
            position.y = ground + 0.05;
            self.base_mut().set_global_position(position);
            let mut velocity = self.base().get_velocity();
            velocity.y = 0.0;
            self.base_mut().set_velocity(velocity);
        }
    }

    /// Makes the collision shape and the body from simple shapes. The
    /// origin of the bear is between its feet.
    fn build_body(&mut self) {
        // The collision shape: a capsule, 0.40 m tall.
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(0.11);
        capsule.set_height(0.40);
        let mut shape = CollisionShape3D::new_alloc();
        shape.set_shape(&capsule);
        shape.set_position(Vector3::new(0.0, 0.20, 0.0));
        self.base_mut().add_child(&shape);

        let fur = material(Color::from_rgb(0.55, 0.36, 0.20));
        let pale = material(Color::from_rgb(0.78, 0.63, 0.44));
        let dark = material(Color::from_rgb(0.08, 0.06, 0.05));

        let mut body = Node3D::new_alloc();
        body.set_name("Body");
        // The bear looks along +Z of the body node.
        let parts: [(Gd<Mesh>, &Gd<StandardMaterial3D>, Vector3); 14] = [
            (ball(0.105, 0.19), &fur, Vector3::new(0.0, 0.165, 0.0)), // torso
            (ball(0.07, 0.10), &pale, Vector3::new(0.0, 0.16, 0.065)), // belly
            (ball(0.085, 0.15), &fur, Vector3::new(0.0, 0.315, 0.0)), // head
            (ball(0.04, 0.06), &pale, Vector3::new(0.0, 0.295, 0.075)), // muzzle
            (ball(0.014, 0.022), &dark, Vector3::new(0.0, 0.305, 0.105)), // nose
            (
                ball(0.011, 0.022),
                &dark,
                Vector3::new(-0.035, 0.335, 0.072),
            ), // left eye
            (ball(0.011, 0.022), &dark, Vector3::new(0.035, 0.335, 0.072)), // right eye
            (ball(0.032, 0.05), &fur, Vector3::new(-0.065, 0.385, 0.0)), // left ear
            (ball(0.032, 0.05), &fur, Vector3::new(0.065, 0.385, 0.0)), // right ear
            (limb(0.035, 0.15), &fur, Vector3::new(-0.115, 0.19, 0.01)), // left arm
            (limb(0.035, 0.15), &fur, Vector3::new(0.115, 0.19, 0.01)), // right arm
            (limb(0.042, 0.13), &fur, Vector3::new(-0.05, 0.065, 0.0)), // left leg
            (limb(0.042, 0.13), &fur, Vector3::new(0.05, 0.065, 0.0)), // right leg
            (ball(0.03, 0.05), &fur, Vector3::new(0.0, 0.11, -0.10)), // tail
        ];
        for (mesh, material, position) in parts {
            let mut part = MeshInstance3D::new_alloc();
            part.set_mesh(&mesh);
            part.set_material_override(material);
            part.set_position(position);
            body.add_child(&part);
        }
        self.base_mut().add_child(&body);
        self.body = Some(body);
    }
}

fn material(colour: Color) -> Gd<StandardMaterial3D> {
    let mut m = StandardMaterial3D::new_gd();
    m.set_albedo(colour);
    m.set_roughness(1.0);
    m.set_specular_mode(godot::classes::base_material_3d::SpecularMode::DISABLED);
    m
}

/// A ball or an egg shape with a radius and a full height.
fn ball(radius: f32, height: f32) -> Gd<Mesh> {
    let mut m = SphereMesh::new_gd();
    m.set_radius(radius);
    m.set_height(height);
    m.set_radial_segments(16);
    m.set_rings(8);
    m.upcast()
}

/// A capsule for an arm or a leg.
fn limb(radius: f32, height: f32) -> Gd<Mesh> {
    let mut m = CapsuleMesh::new_gd();
    m.set_radius(radius);
    m.set_height(height);
    m.set_radial_segments(12);
    m.set_rings(4);
    m.upcast()
}
