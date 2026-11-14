//! The GDExtension of Broken Friends.
//!
//! Godot loads this library through `game/bf.gdextension`. Each module
//! registers one node class. The class names start with `Bf`.

use godot::prelude::*;

mod anim;
mod atmosphere;
mod autoplay;
mod bear;
mod bear_model;
mod bear_shape;
mod climb;
mod dev_menu;
mod fly_camera;
mod follow_camera;
mod limbs;
mod lost_limb;
mod motion;
mod obstacle_sim;
mod obstacles;
mod overlay;
mod playtest;
mod ragdoll;
mod recorder;
mod smoke_dose;
mod smoke_edge;
mod spring;
mod stamina;
mod tear;
mod terrain;
mod test_runner;
mod tumble;
mod verlet;
mod view_distance;

struct BfExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BfExtension {}
