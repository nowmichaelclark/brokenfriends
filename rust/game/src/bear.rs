//! The teddy bear: the player.

use crate::anim::{AnimInput, AnimMode, Ground, LIMBS};
use crate::bear_model::BfBearModel;
use crate::climb::{
    find_grab, slope_angle, stamina_drain, wall_axes, wall_input, Climb, ClimbParams, Hit, Step,
    Surface,
};
use crate::follow_camera::FOLLOW_CAMERA_GROUP;
use crate::limbs::{profile, GaitProfile, GaitTuning, LimbSet};
use crate::motion::{angle_difference, approach, jump_speed, JumpTimers};
use crate::terrain::TERRAIN_GROUP;
use glam::{Vec2, Vec3};
use godot::classes::{
    CapsuleShape3D, CharacterBody3D, CollisionShape3D, ICharacterBody3D, Input, InputEvent,
    InputEventKey, PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D,
};
use godot::global::{JoyAxis, JoyButton, Key};
use godot::prelude::*;

/// The radius of the collision capsule in metres.
const RADIUS: f32 = 0.11;
/// The distance from the feet to the centre of the bear in metres.
const HALF_HEIGHT: f32 = 0.20;
/// The collider metadata that gives the grip of an object (0 to 1).
/// Without it, the terrain gives the grip.
pub const GRIP_META: &str = "bf_grip";
/// The collider metadata that gives the softness of an object (0 to 1).
/// Without it, the terrain gives the softness: only sand is soft.
pub const SOFT_META: &str = "bf_soft";

/// What the bear does now.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Mode {
    /// The bear walks, jumps or falls. The physics engine moves it.
    #[default]
    Walk,
    /// The bear is on a wall. Ray casts move it.
    Climb(Climb),
    /// The bear moves from a wall to a place to stand, in a short time.
    Mantle {
        from: Vector3,
        to: Vector3,
        time: f32,
    },
}

fn to_glam(v: Vector3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn to_godot(v: Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

/// The physics world as a climbing surface. Each `cast` is one ray cast.
struct RaySurface {
    space: Gd<PhysicsDirectSpaceState3D>,
    query: Gd<PhysicsRayQueryParameters3D>,
    terrain: Option<Gd<Node>>,
}

impl Surface for RaySurface {
    fn cast(&mut self, from: Vec3, to: Vec3) -> Option<Hit> {
        self.query.set_from(to_godot(from));
        self.query.set_to(to_godot(to));
        let result = self.space.intersect_ray(&self.query);
        let point = result.get("position")?.try_to::<Vector3>().ok()?;
        let normal = result.get("normal")?.try_to::<Vector3>().ok()?;
        let id = result.get("collider_id")?.try_to::<i64>().unwrap_or(0);
        Some(Hit {
            point: to_glam(point),
            normal: to_glam(normal).normalize_or(Vec3::Y),
            id: id as u64,
        })
    }

    fn grip(&mut self, hit: &Hit) -> f32 {
        let collider = (hit.id != 0)
            .then(|| Gd::<Object>::try_from_instance_id(InstanceId::from_i64(hit.id as i64)).ok())
            .flatten();
        if let Some(collider) = collider {
            if collider.has_meta(GRIP_META) {
                let grip = collider.get_meta(GRIP_META).try_to::<f32>();
                return grip.unwrap_or(1.0).clamp(0.0, 1.0);
            }
        }
        let Some(terrain) = &mut self.terrain else {
            return 1.0;
        };
        let at = [hit.point.x.to_variant(), hit.point.z.to_variant()];
        terrain.call("grip_at", &at).try_to::<f32>().unwrap_or(1.0)
    }
}

impl Ground for RaySurface {
    fn softness(&mut self, hit: &Hit) -> f32 {
        let collider = (hit.id != 0)
            .then(|| Gd::<Object>::try_from_instance_id(InstanceId::from_i64(hit.id as i64)).ok())
            .flatten();
        if let Some(collider) = collider {
            if collider.has_meta(SOFT_META) {
                let soft = collider.get_meta(SOFT_META).try_to::<f32>();
                return soft.unwrap_or(0.0).clamp(0.0, 1.0);
            }
        }
        let Some(terrain) = &mut self.terrain else {
            return 0.0;
        };
        // Only the loose sand of the riverbeds is soft.
        let at = [hit.point.x.to_variant(), hit.point.z.to_variant()];
        let name = terrain.call("ground_name_at", &at).to_string();
        if name == "sand" {
            1.0
        } else {
            0.0
        }
    }
}

/// What the player pushes now.
#[derive(Clone, Copy, Default)]
struct Controls {
    /// The move direction on the ground (x, z), 1 long at most.
    wish: Vector2,
    jog: bool,
    jump: bool,
    /// The let-go button (R21).
    let_go: bool,
}

/// The group that holds the bear. Other nodes find it there.
pub const BEAR_GROUP: &str = "bf_bear";

/// The teddy bear. It is 0.40 m tall. It walks, jogs, jumps and climbs.
///
/// Keys: W, A, S, D or the arrow keys to move, Shift to jog, Space to
/// jump, C to let go of a wall. Gamepad: left stick to move, A to jump,
/// the right shoulder button to jog, B to let go.
///
/// A jog uses stamina (R20). A tired bear walks.
///
/// The bear starts to climb when it pushes into a surface that is too
/// steep to walk. On the wall, "into the wall" is up and "away from
/// the wall" is down. Jump makes a hop up the wall, and it uses
/// stamina (R20). Jump with the stick away from the wall jumps off.
/// The let-go button drops the bear (R21).
///
/// The bear can lose limbs (R12). The limbs that it has set its gait,
/// its speed, its jump and its climb limits (`limbs.rs`). Debug keys:
/// 1 to 4 remove or give back the left arm, the right arm, the left leg
/// and the right leg. 0 gives back all limbs.
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

    /// The climb speed in metres per second.
    #[export]
    #[init(val = 0.7)]
    climb_speed: f32,

    /// The climb stops on a slope this much flatter than `max_slope_degrees`.
    #[export]
    #[init(val = 5.0)]
    climb_stop_margin_degrees: f32,

    /// The steepest surface that the bear can hold, in degrees (90 is a wall).
    #[export]
    #[init(val = 110.0)]
    climb_max_degrees: f32,

    /// How far the paws reach past the body to find a wall, in metres.
    #[export]
    #[init(val = 0.14)]
    climb_reach: f32,

    /// How long the bear on the ground must push into a wall to start a climb, in seconds.
    #[export]
    #[init(val = 0.15)]
    grab_delay: f32,

    /// A bear that moves faster than this speed cannot grab a wall, in m/s.
    #[export]
    #[init(val = 6.0)]
    max_grab_speed: f32,

    /// The height of the jump off a wall in metres.
    #[export]
    #[init(val = 0.3)]
    climb_jump_height: f32,

    /// The speed away from the wall when the bear jumps off, in m/s.
    #[export]
    #[init(val = 1.8)]
    climb_jump_push: f32,

    /// The height of a hop up the wall in metres (jump, with the stick not away from the wall).
    #[export]
    #[init(val = 0.35)]
    climb_hop_height: f32,

    /// The time of a hop up the wall in seconds.
    #[export]
    #[init(val = 0.25)]
    climb_hop_time: f32,

    /// The stamina that one hop uses (0 to 1). A climb of the same height uses about 0.017.
    #[export]
    #[init(val = 0.04)]
    climb_hop_stamina: f32,

    /// On a wall, a jump with the stick this far away from the wall jumps off (0 to 1).
    #[export]
    #[init(val = 0.5)]
    jump_off_stick: f32,

    /// How long the stamina lasts while the bear jogs on the ground, in seconds.
    #[export]
    #[init(val = 20.0)]
    jog_stamina_seconds: f32,

    /// The time after a jump off or a slide off in which the bear cannot grab, in seconds.
    #[export]
    #[init(val = 0.4)]
    regrab_time: f32,

    /// How long the stamina lasts on a vertical wall, in seconds.
    #[export]
    #[init(val = 30.0)]
    stamina_seconds: f32,

    /// The part of the stamina use on the least steep climb (0 to 1).
    #[export]
    #[init(val = 0.3)]
    gentle_drain_factor: f32,

    /// The time on the ground that makes the stamina full again, in seconds.
    #[export]
    #[init(val = 3.0)]
    stamina_recover_seconds: f32,

    /// After the stamina is gone, the bear can climb again at this part of it (0 to 1).
    #[export]
    #[init(val = 0.3)]
    regrab_stamina_part: f32,

    /// With this grip or more, the bear does not slide on a wall (0 to 1).
    #[export]
    #[init(val = 0.7)]
    firm_grip: f32,

    /// With this grip or less, the bear slides off completely (0 to 1).
    #[export]
    #[init(val = 0.25)]
    loose_grip: f32,

    /// The fastest slow slide on a slippery wall, in m/s.
    #[export]
    #[init(val = 0.35)]
    slow_slide_speed: f32,

    /// If the bear slides faster than this speed, it lets go, in m/s.
    #[export]
    #[init(val = 1.5)]
    let_go_slide_speed: f32,

    /// The time to climb from a wall onto a ledge, in seconds.
    #[export]
    #[init(val = 0.2)]
    mantle_time: f32,

    /// How fast the body turns to the wall direction, in 1/s.
    #[export]
    #[init(val = 12.0)]
    climb_turn_rate: f32,

    /// The speed of a bear with two legs and fewer than two arms, as a part of the normal speed.
    #[export]
    #[init(val = 0.95)]
    armless_walk_speed_scale: f32,

    /// The speed of a hop on one leg with two arms, as a part of the normal speed.
    #[export]
    #[init(val = 0.6)]
    hop_speed_scale: f32,

    /// The hop gets slower by this part for each missing arm (0 to 1).
    #[export]
    #[init(val = 0.15)]
    hop_arm_slowdown: f32,

    /// The jump height on one leg, as a part of the normal jump.
    #[export]
    #[init(val = 0.6)]
    hop_jump_scale: f32,

    /// The speed of an arm-crawl (no legs), as a part of the normal speed.
    #[export]
    #[init(val = 0.3)]
    crawl_speed_scale: f32,

    /// The speed of a one-arm drag (no legs, one arm), as a part of the normal speed.
    #[export]
    #[init(val = 0.18)]
    drag_speed_scale: f32,

    /// The speed of a wriggle (no limbs), as a part of the normal speed.
    #[export]
    #[init(val = 0.1)]
    wriggle_speed_scale: f32,

    /// The steepest surface that a bear with one arm can hold, in degrees.
    #[export]
    #[init(val = 68.0)]
    one_arm_climb_degrees: f32,

    /// The climb stamina use of a bear with one arm is multiplied by this value.
    #[export]
    #[init(val = 1.6)]
    one_arm_stamina_factor: f32,

    /// The climb stamina use of a bear with one leg is multiplied by this value.
    #[export]
    #[init(val = 1.15)]
    one_leg_stamina_factor: f32,

    /// The climb stamina use of a bear with no legs is multiplied by this value.
    #[export]
    #[init(val = 1.3)]
    no_legs_stamina_factor: f32,

    /// The limbs that the bear has.
    limbs: LimbSet,

    timers: JumpTimers,
    jump_was_down: bool,
    let_go_was_down: bool,
    mode: Mode,
    /// The stamina, from 0 (empty) to 1 (full).
    #[init(val = 1.0)]
    stamina: f32,
    /// The stamina was gone. The bear cannot climb until some comes back.
    exhausted: bool,
    /// The time that is left until the bear can grab a wall again.
    regrab_left: f32,
    /// How long the bear on the ground pushes into a wall.
    push_time: f32,
    /// The direction that the body looks at, as an angle around Y.
    yaw: f32,
    /// The move direction of the last physics step (x, z).
    last_wish: Vector2,
    /// The time in the air since the bear left the ground, in seconds.
    air_time: f32,
    /// The bear slides on a wall now.
    was_sliding: bool,
    /// Events for the playtest recorder, since its last `take_events`.
    events: Vec<String>,
    /// One ray query for all ray casts.
    ray_query: Option<Gd<PhysicsRayQueryParameters3D>>,
    /// The visible bear. It turns; the collision shape does not.
    body: Option<Gd<Node3D>>,
    model: Option<Gd<BfBearModel>>,
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
        let Controls {
            wish,
            jog,
            jump: jump_down,
            let_go: let_go_down,
        } = self.read_input();
        let jump_pressed = jump_down && !self.jump_was_down;
        self.jump_was_down = jump_down;
        let let_go_pressed = let_go_down && !self.let_go_was_down;
        self.let_go_was_down = let_go_down;
        self.last_wish = wish;
        self.regrab_left = (self.regrab_left - delta).max(0.0);

        match self.mode {
            Mode::Walk => self.walk(delta, wish, jog, jump_pressed),
            Mode::Climb(climb) => self.climb(delta, climb, wish, jump_pressed, let_go_pressed),
            Mode::Mantle { from, to, time } => self.mantle(delta, from, to, time),
        }
        self.pose_body(delta);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(key) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if !key.is_pressed() || key.is_echo() {
            return;
        }
        // Debug keys: 1 to 4 remove or give back one limb, 0 gives back all.
        let limb = match key.get_keycode() {
            Key::KEY_1 => Some(0),
            Key::KEY_2 => Some(1),
            Key::KEY_3 => Some(2),
            Key::KEY_4 => Some(3),
            Key::KEY_0 => {
                self.change_limbs(LimbSet::ALL);
                None
            }
            _ => None,
        };
        if let Some(i) = limb {
            let has = self.limbs.has(i);
            self.change_limbs(self.limbs.with(i, !has));
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
        self.mode = Mode::Walk;
        self.stamina = 1.0;
        self.exhausted = false;
        self.event("restart");
    }

    /// Removes or gives back a limb: "LeftArm", "RightArm", "LeftLeg"
    /// or "RightLeg". S-114 and the dev menu (S-113) call this.
    #[func]
    pub fn set_limb(&mut self, part: GString, present: bool) {
        let name = part.to_string();
        let Some(i) = LIMBS.iter().position(|l| l.name() == name) else {
            godot_warn!("BfBear: no limb with the name {name}");
            return;
        };
        self.change_limbs(self.limbs.with(i, present));
    }

    /// Gives back all limbs.
    #[func]
    pub fn restore_limbs(&mut self) {
        self.change_limbs(LimbSet::ALL);
    }

    /// Tells if the bear has a limb, for example "LeftLeg".
    #[func]
    pub fn has_limb(&self, part: GString) -> bool {
        let name = part.to_string();
        LIMBS
            .iter()
            .position(|l| l.name() == name)
            .is_some_and(|i| self.limbs.has(i))
    }

    /// Gives the name of the gait now, for example "walk" or "hop".
    #[func]
    pub fn gait_name(&self) -> GString {
        GString::from(self.gait_profile().gait.name())
    }

    /// Kills the bear and starts it again at the start point (R16).
    /// `cause` is a short name for the playtest recorder, for example
    /// "smoke:time". There is no other penalty.
    #[func]
    pub fn die(&mut self, cause: GString) {
        self.event(&format!("death:{cause}"));
        self.go_to_start();
    }

    /// Keeps an event for the playtest recorder. Other nodes use it,
    /// for example the smoke edge ("smoke_enter").
    #[func]
    pub fn report_event(&mut self, name: GString) {
        self.event(&name.to_string());
    }

    /// Gives the stamina, from 0 (empty) to 1 (full).
    #[func]
    pub fn stamina_part(&self) -> f32 {
        self.stamina
    }

    /// Tells if the stamina was gone and the bear cannot climb yet.
    #[func]
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }

    /// Tells if the bear is on a wall now.
    #[func]
    pub fn is_climbing(&self) -> bool {
        matches!(self.mode, Mode::Climb(_))
    }

    /// Gives one line of text for the debug overlay.
    #[func]
    pub fn debug_text(&self) -> GString {
        let stamina = self.stamina * 100.0;
        let mode = match self.mode {
            Mode::Walk if self.exhausted => format!("walk (tired)   stamina {stamina:.0}%"),
            Mode::Walk => format!("walk   stamina {stamina:.0}%"),
            Mode::Mantle { .. } => format!("onto a ledge   stamina {stamina:.0}%"),
            Mode::Climb(c) => format!(
                "climb   slope {:.0} deg   grip {:.2}   slide {:.2} m/s   stamina {stamina:.0}%",
                slope_angle(c.normal).to_degrees(),
                c.grip,
                c.slide_speed
            ),
        };
        let text = format!(
            "{mode}   gait {}   limbs {}",
            self.gait_profile().gait.name(),
            self.limbs.label()
        );
        GString::from(&text)
    }

    /// Gives the state of the bear for the playtest recorder.
    #[func]
    pub fn recorder_state(&self) -> VarDictionary {
        let velocity = self.base().get_velocity();
        let on_floor = self.base().is_on_floor();
        let speed = velocity.length();
        let (state, slope, grip) = match self.mode {
            Mode::Climb(c) => (
                if c.slide_speed > 0.01 {
                    "slide"
                } else {
                    "climb"
                },
                slope_angle(c.normal).to_degrees(),
                c.grip,
            ),
            Mode::Mantle { .. } => ("mantle", 0.0, -1.0),
            Mode::Walk => {
                let state = if !on_floor {
                    if velocity.y > 0.0 {
                        "jump"
                    } else {
                        "fall"
                    }
                } else if speed > (self.walk_speed + self.jog_speed) * 0.5 {
                    "jog"
                } else if speed > 0.05 {
                    "walk"
                } else {
                    "idle"
                };
                let slope = if on_floor {
                    self.base()
                        .get_floor_normal()
                        .angle_to(Vector3::UP)
                        .to_degrees()
                } else {
                    0.0
                };
                (state, slope, -1.0)
            }
        };
        let mut d = VarDictionary::new();
        d.set("pos", self.base().get_global_position());
        d.set("velocity", velocity);
        d.set("speed", speed);
        d.set("state", state);
        d.set("slope_deg", slope);
        // -1: the recorder asks the terrain for the grip below the bear.
        d.set("grip", grip);
        d.set("stamina", self.stamina);
        d.set("exhausted", self.exhausted);
        d.set("limbs", self.limbs.count() as i64);
        d.set("gait", self.gait_profile().gait.name());
        let anim_us = self
            .model
            .as_ref()
            .map_or(0.0, |m| m.bind().anim_microseconds());
        d.set("anim_us", anim_us);
        d.set("stick_held", self.last_wish.length() > 0.01);
        d
    }

    /// Gives the events since the last call and forgets them. Each event
    /// is a short name, for example "jump", "grab" or "let_go:jump_off".
    #[func]
    pub fn take_events(&mut self) -> PackedStringArray {
        let events = std::mem::take(&mut self.events);
        events.iter().map(GString::from).collect()
    }
}

impl BfBear {
    /// One step on the ground or in the air.
    fn walk(&mut self, delta: f32, wish: Vector2, jog: bool, jump_pressed: bool) {
        let max_slope = self.max_slope_degrees.to_radians();
        self.base_mut().set_floor_max_angle(max_slope);
        let on_floor = self.base().is_on_floor();

        let mut velocity = self.base().get_velocity();

        // A jog uses stamina. A tired bear walks.
        let jogging = jog && wish != Vector2::ZERO && !self.exhausted && self.stamina > 0.0;
        if jogging && on_floor {
            self.stamina -= delta / self.jog_stamina_seconds.max(0.1);
            if self.stamina <= 0.0 {
                self.stamina = 0.0;
                self.exhausted = true;
                self.event("tired:jog");
            }
        }

        // Speed on the ground plane. Fewer limbs are slower.
        let gait = self.gait_profile();
        let speed = gait.speed_scale
            * if jogging {
                self.jog_speed
            } else {
                self.walk_speed
            };
        let target = (wish.x * speed, wish.y * speed);
        let rate = if wish == Vector2::ZERO {
            self.deceleration
        } else {
            self.acceleration
        };
        let control = if on_floor { 1.0 } else { self.air_control };
        // Just after the bear leaves a wall, it keeps its speed in the
        // air if the player gives no direction.
        let keeps_speed = !on_floor && wish == Vector2::ZERO && self.regrab_left > 0.0;
        if !keeps_speed {
            let (vx, vz) = approach((velocity.x, velocity.z), target, rate * control * delta);
            velocity.x = vx;
            velocity.z = vz;
        }

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
        // A bear without legs cannot jump.
        if self.timers.update(
            delta,
            on_floor,
            jump_pressed && gait.can_jump(),
            self.coyote_time,
            self.jump_buffer_time,
        ) {
            let height = self.jump_height * gait.jump_scale;
            velocity.y = jump_speed(height, self.fall_acceleration);
            self.event("jump");
        }

        self.base_mut().set_velocity(velocity);
        self.base_mut().move_and_slide();
        self.stay_above_ground();

        // Turn the body to the move direction.
        if wish != Vector2::ZERO {
            let wanted = wish.x.atan2(wish.y);
            let step = self.turn_speed_degrees.to_radians() * delta;
            self.yaw += angle_difference(self.yaw, wanted).clamp(-step, step);
        }

        // A landing after a long time in the air is an event.
        if self.base().is_on_floor() {
            if self.air_time > 0.5 {
                let fall = -velocity.y.min(0.0);
                self.event(&format!("land:{:.1}s:{fall:.1}m/s", self.air_time));
            }
            self.air_time = 0.0;
        } else {
            self.air_time += delta;
        }

        // The stamina comes back on the ground, but not during a jog.
        if self.base().is_on_floor() && !jogging {
            let gain = delta / self.stamina_recover_seconds.max(0.01);
            self.stamina = (self.stamina + gain).min(1.0);
            if self.stamina >= self.regrab_stamina_part.clamp(0.0, 1.0) {
                self.exhausted = false;
            }
        }
        self.try_grab(delta, wish);
    }

    /// Starts a climb if the bear pushes into a surface that is too
    /// steep to walk. This uses 2 ray casts at most.
    fn try_grab(&mut self, delta: f32, wish: Vector2) {
        let velocity = self.base().get_velocity();
        let can_grab = wish != Vector2::ZERO
            && self.gait_profile().climb.can_climb
            && !self.exhausted
            && self.regrab_left <= 0.0
            && velocity.length() <= self.max_grab_speed;
        if !can_grab {
            self.push_time = 0.0;
            return;
        }
        let Some(mut surface) = self.surface() else {
            return;
        };
        let p = self.climb_params();
        let centre = to_glam(self.base().get_global_position()) + Vec3::Y * HALF_HEIGHT;
        let push = Vec3::new(wish.x, 0.0, wish.y).normalize_or_zero();
        let Some(hit) = find_grab(&mut surface, centre, push, &p) else {
            self.push_time = 0.0;
            return;
        };
        // In the air, the bear grabs at once.
        self.push_time += delta;
        if self.base().is_on_floor() && self.push_time < self.grab_delay {
            return;
        }
        self.push_time = 0.0;
        let climb = Climb::start(&mut surface, hit, to_glam(velocity), &p);
        self.base_mut().set_velocity(Vector3::ZERO);
        self.set_centre(climb.centre);
        self.mode = Mode::Climb(climb);
        self.was_sliding = false;
        self.event("grab");
    }

    /// One step on a wall.
    fn climb(
        &mut self,
        delta: f32,
        mut climb: Climb,
        wish: Vector2,
        jump_pressed: bool,
        let_go_pressed: bool,
    ) {
        let p = self.climb_params();
        let gait = self.gait_profile();
        // A bear without arms cannot hold a wall.
        if !gait.climb.can_climb {
            self.event("let_go:no_arms");
            self.let_go(to_godot(climb.slide_velocity()));
            return;
        }
        let Some(mut surface) = self.surface() else {
            self.event("let_go:no_physics");
            self.let_go(Vector3::ZERO);
            return;
        };
        // The let-go button (R21): the bear drops with a small push off the wall.
        if let_go_pressed {
            self.event("let_go:button");
            self.let_go(to_godot(climb.normal * 0.3 + climb.slide_velocity()));
            return;
        }
        let input = wall_input(Vec2::new(wish.x, wish.y), climb.normal);
        if jump_pressed {
            if input.y < -self.jump_off_stick.clamp(0.0, 1.0) {
                // The stick points away from the wall: jump off.
                let up = jump_speed(self.climb_jump_height, self.fall_acceleration);
                let velocity = climb.normal * self.climb_jump_push + Vec3::Y * up;
                self.event("let_go:jump_off");
                self.let_go(to_godot(velocity));
                return;
            }
            // Otherwise: a hop up the wall (R20). It costs stamina.
            if !climb.is_hopping() {
                climb.hop(self.climb_hop_height, self.climb_hop_time);
                self.stamina -= self.climb_hop_stamina.max(0.0) * gait.climb.stamina_factor;
                self.event("hop");
            }
        }
        let sliding_now = climb.slide_speed > 0.01;
        if sliding_now != self.was_sliding {
            self.was_sliding = sliding_now;
            self.event(if sliding_now { "slide" } else { "slide_stop" });
        }

        let moving = input.length() > 0.05;
        let sliding = climb.slide_speed > 0.01;
        let factor = gait.climb.stamina_factor;
        self.stamina -= stamina_drain(climb.angle, moving, sliding, &p) * factor * delta;
        if self.stamina <= 0.0 {
            self.stamina = 0.0;
            self.exhausted = true;
            self.event("let_go:no_stamina");
            self.let_go(to_godot(climb.slide_velocity()));
            return;
        }

        match climb.step(&mut surface, input, delta, &p) {
            Step::Hold => {
                self.set_centre(climb.centre);
                self.mode = Mode::Climb(climb);
            }
            Step::Ground { feet } => {
                // The upright bear looks at the wall that it came from.
                self.yaw = (-climb.normal.x).atan2(-climb.normal.z);
                self.event("mantle");
                self.mode = Mode::Mantle {
                    from: self.base().get_global_position(),
                    to: to_godot(feet) + Vector3::new(0.0, 0.02, 0.0),
                    time: 0.0,
                };
            }
            Step::LetGo { velocity } => {
                self.event("let_go:slide_or_edge");
                self.let_go(to_godot(velocity));
            }
        }
    }

    /// One step of the move from a wall to a place to stand.
    fn mantle(&mut self, delta: f32, from: Vector3, to: Vector3, time: f32) {
        let time = time + delta;
        let t = (time / self.mantle_time.max(0.01)).min(1.0);
        let ease = t * t * (3.0 - 2.0 * t);
        // A small arc lifts the feet above the edge.
        let lift = (t * std::f32::consts::PI).sin() * 0.04;
        let position = from.lerp(to, ease) + Vector3::new(0.0, lift, 0.0);
        self.base_mut().set_global_position(position);
        self.base_mut().set_velocity(Vector3::ZERO);
        if t >= 1.0 {
            self.timers = JumpTimers::default();
            self.mode = Mode::Walk;
        } else {
            self.mode = Mode::Mantle { from, to, time };
        }
    }

    /// Stops the climb. The bear falls with this velocity.
    fn let_go(&mut self, velocity: Vector3) {
        if let Mode::Climb(climb) = self.mode {
            self.yaw = (-climb.normal.x).atan2(-climb.normal.z);
        }
        self.mode = Mode::Walk;
        self.timers = JumpTimers::default();
        self.regrab_left = self.regrab_time;
        self.push_time = 0.0;
        self.base_mut().set_velocity(velocity);
    }

    /// Keeps an event for the playtest recorder. At most 64 events wait;
    /// without a recorder, the oldest go.
    fn event(&mut self, name: &str) {
        if self.events.len() >= 64 {
            self.events.remove(0);
        }
        self.events.push(name.to_string());
    }

    /// Puts the centre of the bear at a position.
    fn set_centre(&mut self, centre: Vec3) {
        let feet = to_godot(centre) - Vector3::new(0.0, HALF_HEIGHT, 0.0);
        self.base_mut().set_global_position(feet);
    }

    /// Gives the physics world as a climbing surface.
    fn surface(&mut self) -> Option<RaySurface> {
        let space = self.base().get_world_3d()?.get_direct_space_state()?;
        if self.ray_query.is_none() {
            let mut query = PhysicsRayQueryParameters3D::new_gd();
            let mut exclude = Array::<Rid>::new();
            exclude.push(self.base().get_rid());
            query.set_exclude(&exclude);
            // A ray that starts below the ground must not hit it.
            query.set_hit_back_faces(false);
            self.ray_query = Some(query);
        }
        Some(RaySurface {
            space,
            query: self.ray_query.clone()?,
            terrain: self
                .base()
                .get_tree()
                .get_first_node_in_group(TERRAIN_GROUP),
        })
    }

    /// Gives what the limbs of the bear can do (the gait table).
    fn gait_profile(&self) -> GaitProfile {
        let tuning = GaitTuning {
            armless_walk_speed: self.armless_walk_speed_scale.clamp(0.05, 1.0),
            hop_speed: self.hop_speed_scale.clamp(0.05, 1.0),
            hop_arm_slowdown: self.hop_arm_slowdown.clamp(0.0, 0.45),
            hop_jump: self.hop_jump_scale.clamp(0.0, 1.0),
            crawl_speed: self.crawl_speed_scale.clamp(0.02, 1.0),
            drag_speed: self.drag_speed_scale.clamp(0.02, 1.0),
            wriggle_speed: self.wriggle_speed_scale.clamp(0.02, 1.0),
            one_arm_max_degrees: self.one_arm_climb_degrees,
            one_arm_stamina: self.one_arm_stamina_factor.max(0.1),
            one_leg_stamina: self.one_leg_stamina_factor.max(0.1),
            no_legs_stamina: self.no_legs_stamina_factor.max(0.1),
        };
        profile(self.limbs, &tuning)
    }

    /// Changes the limbs of the bear. The model hides the lost limbs, and
    /// the recorder gets an event for each change.
    fn change_limbs(&mut self, limbs: LimbSet) {
        if limbs == self.limbs {
            return;
        }
        for (i, limb) in LIMBS.iter().enumerate() {
            if limbs.has(i) != self.limbs.has(i) {
                let what = if limbs.has(i) { "limb_on" } else { "limb_off" };
                self.event(&format!("{what}:{}", limb.name()));
            }
        }
        self.limbs = limbs;
        self.event(&format!("gait:{}", self.gait_profile().gait.name()));
        if let Some(mut model) = self.model.clone() {
            model.bind_mut().show_limbs(limbs);
        }
    }

    /// Gives the tuning values of the climb from the exported fields.
    fn climb_params(&self) -> ClimbParams {
        let start = self.max_slope_degrees;
        // With one arm, the bear holds only less steep surfaces.
        let max = match self.gait_profile().climb.max_degrees {
            Some(limit) => self.climb_max_degrees.min(limit),
            None => self.climb_max_degrees,
        };
        ClimbParams {
            start_angle: start.to_radians(),
            stop_angle: (start - self.climb_stop_margin_degrees.max(0.0)).to_radians(),
            max_angle: max.max(start).to_radians(),
            radius: RADIUS,
            half_height: HALF_HEIGHT,
            reach: self.climb_reach.max(0.02),
            climb_speed: self.climb_speed,
            gravity: self.fall_acceleration,
            firm_grip: self.firm_grip,
            loose_grip: self.loose_grip.min(self.firm_grip),
            slow_slide_speed: self.slow_slide_speed,
            let_go_slide_speed: self.let_go_slide_speed,
            drain_per_second: 1.0 / self.stamina_seconds.max(0.1),
            gentle_drain_factor: self.gentle_drain_factor.clamp(0.0, 1.0),
            ..ClimbParams::default()
        }
    }

    /// Turns the model and poses its parts. On the ground, the model
    /// stands and looks along `yaw`. On a wall, the belly is on the wall
    /// and the head is up. The animation (`anim.rs`) poses the parts.
    fn pose_body(&mut self, delta: f32) {
        let Some(mut body) = self.body.clone() else {
            return;
        };
        let (target, rate, mode) = match self.mode {
            Mode::Climb(climb) => {
                let (_, up) = wall_axes(climb.normal);
                let (up, forward) = (to_godot(up), to_godot(-climb.normal));
                let basis = Basis::from_cols(up.cross(forward), up, forward);
                (
                    basis.get_quaternion(),
                    self.climb_turn_rate,
                    AnimMode::Climb {
                        travel: climb.travel,
                    },
                )
            }
            Mode::Mantle { .. } => (
                Quaternion::from_axis_angle(Vector3::UP, self.yaw),
                self.climb_turn_rate * 2.5,
                AnimMode::Mantle,
            ),
            Mode::Walk => (
                Quaternion::from_axis_angle(Vector3::UP, self.yaw),
                self.climb_turn_rate * 2.5,
                AnimMode::Ground,
            ),
        };
        let now = body.get_basis().get_quaternion();
        let k = 1.0 - (-rate.max(0.0) * delta).exp();
        let turn = now.slerp(target, k).normalized();
        let basis = Basis::from_quaternion(turn);
        // The body turns around the centre of the bear, not around the feet.
        let pivot = Vector3::new(0.0, HALF_HEIGHT, 0.0);
        let place = pivot - basis * pivot;
        body.set_basis(basis);
        body.set_position(place);

        let Some(mut model) = self.model.clone() else {
            return;
        };
        let Some(mut ground) = self.surface() else {
            return;
        };
        let on_floor = self.mode == Mode::Walk && self.base().is_on_floor();
        let floor_normal = if on_floor {
            to_glam(self.base().get_floor_normal())
        } else {
            Vec3::Y
        };
        let input = AnimInput {
            dt: delta,
            model: glam::Affine3A::from_rotation_translation(
                glam::Quat::from_xyzw(turn.x, turn.y, turn.z, turn.w),
                to_glam(self.base().get_global_position() + place),
            ),
            on_floor,
            floor_normal,
            mode,
            limbs: self.limbs,
            profile: self.gait_profile(),
        };
        model.bind_mut().animate(&input, &mut ground);
    }

    /// Reads the keys and the gamepad.
    fn read_input(&self) -> Controls {
        let tree = self.base().get_tree();
        let Some(camera) = tree.get_first_node_in_group(FOLLOW_CAMERA_GROUP) else {
            return Controls::default();
        };
        let Ok(camera) = camera.try_cast::<godot::classes::Camera3D>() else {
            return Controls::default();
        };
        // While a debug camera or the dev menu is active, the bear does not move.
        if !camera.is_current() || crate::dev_menu::is_open() {
            return Controls::default();
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
        let let_go = key(Key::C) || input.is_joy_button_pressed(0, JoyButton::B);
        Controls {
            wish,
            jog,
            jump,
            let_go,
        }
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
            self.event(&format!("back_on_ground:{:.2}m", ground - position.y));
            position.y = ground + 0.05;
            self.base_mut().set_global_position(position);
            let mut velocity = self.base().get_velocity();
            velocity.y = 0.0;
            self.base_mut().set_velocity(velocity);
        }
    }

    /// Makes the collision shape and the visible bear. The origin of
    /// the bear is between its feet.
    fn build_body(&mut self) {
        // The collision shape: a capsule, 0.40 m tall.
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(RADIUS);
        capsule.set_height(HALF_HEIGHT * 2.0);
        let mut shape = CollisionShape3D::new_alloc();
        shape.set_shape(&capsule);
        shape.set_position(Vector3::new(0.0, HALF_HEIGHT, 0.0));
        self.base_mut().add_child(&shape);

        // The model makes its parts when it enters the tree.
        let mut model = BfBearModel::new_alloc();
        model.set_name("Model");
        self.base_mut().add_child(&model);
        self.body = Some(model.clone().upcast());
        self.model = Some(model);
    }
}
