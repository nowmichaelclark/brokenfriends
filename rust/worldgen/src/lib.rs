//! World generation for Broken Friends.
//!
//! This crate does not use Godot. All functions are pure: the same seed
//! always gives the same result. The code uses no `sin`, `cos` or `powf`
//! in generation, so the results are the same on all computers.

pub mod detmath;
pub mod drainage;
pub mod erosion_filter;
pub mod heightmap;
pub mod highlands;
pub mod noise;

pub use heightmap::HeightMap;
pub use highlands::{Ground, HighlandsParams, Zone};
