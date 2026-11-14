//! The view distance: how far the game draws (S-113, S-118).
//!
//! The smog hides all objects that are far away. Thus, the game does
//! not need to draw them. `BfViewDistance` finds the distance at which
//! the smog is almost solid and gives it to the cameras and to the
//! terrain. The dev menu can force a different distance (for the tests).

use crate::atmosphere::ATMOSPHERE_GROUP;
use crate::terrain::TERRAIN_GROUP;
use godot::classes::{Camera3D, INode, Node};
use godot::prelude::*;

/// The group that holds the `BfViewDistance` node.
pub const VIEW_DISTANCE_GROUP: &str = "bf_view_distance";

/// The group of the cameras that get the far plane. Each camera of
/// the game joins it.
pub const VIEW_CAMERA_GROUP: &str = "bf_view_cameras";

/// Gives the distance at which the exponential smog has this part of
/// its full strength, in metres. Godot makes the smog of an object at
/// distance `d` as `1 - exp(-d * density)`.
pub fn smog_distance(density: f32, cutoff: f32, min_m: f32, max_m: f32) -> f32 {
    let cutoff = cutoff.clamp(0.5, 0.9999);
    let d = if density > 1.0e-6 {
        -(1.0 - cutoff).ln() / density
    } else {
        max_m
    };
    d.clamp(min_m, max_m.max(min_m))
}

/// Gives the number of Terrain3D mesh LODs that reach this distance.
/// A LOD is one ring of the clipmap. Each ring has the same number of
/// vertices and twice the size of the ring before it. The ring `n` (from
/// 0) reaches `2 * mesh_size * 2^n * spacing` metres from the camera
/// [unverified: S-118 measures this with the primitive count].
pub fn terrain_lods(distance_m: f32, mesh_size: i32, spacing: f32, max_lods: i32) -> i32 {
    let first_ring = 2.0 * mesh_size.max(2) as f32 * spacing.max(0.1);
    let mut lods = 1;
    while first_ring * (1u32 << (lods - 1)) as f32 <= distance_m && lods < max_lods {
        lods += 1;
    }
    lods
}

/// Gives the far plane to the cameras and the draw range to the
/// terrain. The distance comes from the smog, or from a forced value.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct BfViewDistance {
    /// If this is on, the view distance follows the smog. If it is off,
    /// the view distance is the longest one (the camera draws all).
    #[export]
    #[init(val = false)]
    follow_smog: bool,

    /// The part of the smog (0.5 to 0.9999) at which the game stops to
    /// draw. At 0.99 the smog hides 99% of an object. The object is
    /// then almost the colour of the smog, and nobody sees it go.
    #[export]
    #[init(val = 0.99)]
    smog_cutoff: f32,

    /// The shortest view distance in metres.
    #[export]
    #[init(val = 100.0)]
    min_distance_m: f32,

    /// The longest view distance in metres.
    #[export]
    #[init(val = 3000.0)]
    max_distance_m: f32,

    /// The far plane of a camera is the view distance multiplied by
    /// this value. A value above 1 gives a margin for the camera angle.
    #[export]
    #[init(val = 1.05)]
    far_margin: f32,

    /// The time between two reads of the smog, in seconds.
    #[export]
    #[init(val = 0.5)]
    update_interval: f64,

    /// A forced view distance in metres. Zero uses the smog. The dev
    /// menu and the tests set this.
    #[export]
    #[init(val = 0.0)]
    forced_distance_m: f32,

    current_m: f32,
    time_to_update: f64,
    base: Base<Node>,
}

#[godot_api]
impl INode for BfViewDistance {
    fn ready(&mut self) {
        self.base_mut().add_to_group(VIEW_DISTANCE_GROUP);
    }

    fn process(&mut self, delta: f64) {
        self.time_to_update -= delta;
        if self.time_to_update > 0.0 {
            return;
        }
        self.time_to_update = self.update_interval;
        self.apply();
    }
}

#[godot_api]
impl BfViewDistance {
    /// Forces a view distance in metres. Give 0 to use the smog.
    #[func]
    pub fn set_forced_distance(&mut self, metres: f32) {
        self.forced_distance_m = metres.max(0.0);
        self.apply();
    }

    /// Gives the forced view distance in metres, or 0.
    #[func]
    pub fn get_forced_distance(&self) -> f32 {
        self.forced_distance_m
    }

    /// Gives the distance that the smog sets, in metres.
    #[func]
    pub fn smog_distance_m(&self) -> f32 {
        let density = self
            .atmosphere_value("fog_density")
            .unwrap_or(0.006)
            .max(0.0);
        smog_distance(
            density,
            self.smog_cutoff,
            self.min_distance_m,
            self.max_distance_m,
        )
    }

    /// Gives the view distance that the game uses now, in metres.
    /// Objects with a visibility range use this as the end of the range.
    #[func]
    pub fn view_distance_m(&self) -> f32 {
        if self.forced_distance_m > 0.0 {
            self.forced_distance_m
        } else if self.follow_smog {
            self.smog_distance_m()
        } else {
            self.max_distance_m
        }
    }

    /// Gives the distance to the cameras and to the terrain. A camera
    /// that was made after the last call gets it at the next call.
    #[func]
    pub fn apply(&mut self) {
        if !self.base().is_inside_tree() {
            return;
        }
        let distance = self.view_distance_m();
        self.current_m = distance;
        let far = (distance * self.far_margin.max(1.0)).max(1.0);
        let tree = self.base().get_tree();
        for node in tree.get_nodes_in_group(VIEW_CAMERA_GROUP).iter_shared() {
            if let Ok(mut camera) = node.try_cast::<Camera3D>() {
                if (camera.get_far() - far).abs() > 0.5 {
                    camera.set_far(far);
                }
            }
        }
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            terrain.call("set_draw_distance", &[far.to_variant()]);
        }
    }
}

impl BfViewDistance {
    fn atmosphere_value(&self, name: &str) -> Option<f32> {
        if !self.base().is_inside_tree() {
            return None;
        }
        let node = self
            .base()
            .get_tree()
            .get_first_node_in_group(ATMOSPHERE_GROUP)?;
        node.get(name).try_to::<f32>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smog_distance_of_the_normal_smog() {
        // Density 0.006: the smog is 99% at 767 m.
        let d = smog_distance(0.006, 0.99, 100.0, 3000.0);
        assert!((d - 767.5).abs() < 1.0, "{d}");
    }

    #[test]
    fn thicker_smog_gives_a_shorter_distance() {
        let thin = smog_distance(0.003, 0.99, 100.0, 3000.0);
        let thick = smog_distance(0.012, 0.99, 100.0, 3000.0);
        assert!(thick < thin);
        assert!((thick * 4.0 - thin).abs() < 1.0);
    }

    #[test]
    fn smog_distance_has_limits() {
        assert_eq!(smog_distance(0.0, 0.99, 100.0, 3000.0), 3000.0);
        assert_eq!(smog_distance(0.00001, 0.99, 100.0, 3000.0), 3000.0);
        assert_eq!(smog_distance(5.0, 0.99, 100.0, 3000.0), 100.0);
        // A cutoff of 1 would be infinite: the value is clamped.
        assert!(smog_distance(0.006, 1.0, 100.0, 3000.0) <= 3000.0);
    }

    #[test]
    fn terrain_lods_reach_the_distance() {
        // Mesh size 48 at 1 m: the rings reach 96, 192, 384, 768, 1536 ...
        assert_eq!(terrain_lods(50.0, 48, 1.0, 7), 1);
        assert_eq!(terrain_lods(300.0, 48, 1.0, 7), 3);
        assert_eq!(terrain_lods(500.0, 48, 1.0, 7), 4);
        assert_eq!(terrain_lods(767.5, 48, 1.0, 7), 4);
        assert_eq!(terrain_lods(800.0, 48, 1.0, 7), 5);
        assert_eq!(terrain_lods(3000.0, 48, 1.0, 7), 6);
        assert_eq!(terrain_lods(100000.0, 48, 1.0, 7), 7);
    }

    #[test]
    fn terrain_lods_follow_the_spacing() {
        assert_eq!(terrain_lods(300.0, 48, 2.0, 7), 2);
    }
}
