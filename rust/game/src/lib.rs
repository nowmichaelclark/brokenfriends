//! The GDExtension of Broken Friends.
//!
//! Godot loads this library through `game/bf.gdextension`. Each module
//! registers one node class. The class names start with `Bf`.

use godot::prelude::*;

mod atmosphere;
mod bear;
mod climb;
mod fly_camera;
mod follow_camera;
mod motion;
mod overlay;
mod stamina;
mod terrain;

struct BfExtension;

#[gdextension]
unsafe impl ExtensionLibrary for BfExtension {}
