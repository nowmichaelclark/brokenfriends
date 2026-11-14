//! The GDExtension of Broken Friends.
//!
//! Godot loads this library through `game/bf.gdextension`. Each module
//! registers one node class. The class names start with `Bf`.

use godot::prelude::*;

mod anim;
mod atmosphere;
mod bear;
mod bear_model;
mod bear_shape;
mod climb;
mod fly_camera;
mod follow_camera;
mod limbs;
mod motion;
mod overlay;
mod playtest;
mod recorder;
mod smoke_dose;
mod smoke_edge;
mod spring;
mod stamina;
mod terrain;

struct BfExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BfExtension {}
