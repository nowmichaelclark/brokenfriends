//! The stamina ring on the screen.

use crate::bear::BEAR_GROUP;
use godot::classes::{Control, IControl};
use godot::prelude::*;

/// A ring near the bear that shows the stamina. The ring shows only
/// while the stamina is not full. It is red while the bear is too
/// tired to climb.
#[derive(GodotClass)]
#[class(base = Control, init)]
pub struct BfStaminaRing {
    /// The radius of the ring in pixels.
    #[export]
    #[init(val = 26.0)]
    radius: f32,

    /// The width of the ring line in pixels.
    #[export]
    #[init(val = 8.0)]
    width: f32,

    /// The position of the ring, from the head of the bear, in pixels.
    #[export]
    #[init(val = Vector2::new(90.0, -20.0))]
    ring_offset: Vector2,

    /// The colour of the stamina that is left.
    #[export]
    #[init(val = Color::from_rgb(0.93, 0.85, 0.45))]
    colour: Color,

    /// The colour while the bear is too tired to climb.
    #[export]
    #[init(val = Color::from_rgb(0.85, 0.25, 0.15))]
    tired_colour: Color,

    /// The stamina from 0 to 1.
    #[init(val = 1.0)]
    part: f32,
    tired: bool,
    /// The centre of the ring on the screen.
    centre: Vector2,
    base: Base<Control>,
}

#[godot_api]
impl IControl for BfStaminaRing {
    fn process(&mut self, _delta: f64) {
        let Some(mut bear) = self.base().get_tree().get_first_node_in_group(BEAR_GROUP) else {
            return;
        };
        let part = bear
            .call("stamina_part", &[])
            .try_to::<f32>()
            .unwrap_or(1.0);
        let tired = bear.call("is_exhausted", &[]).booleanize();
        let mut centre = self.centre;
        if let Some(camera) = self.base().get_viewport().and_then(|v| v.get_camera_3d()) {
            if let Ok(bear) = bear.try_cast::<Node3D>() {
                let head = bear.get_global_position() + Vector3::new(0.0, 0.4, 0.0);
                if !camera.is_position_behind(head) {
                    centre = camera.unproject_position(head) + self.ring_offset;
                }
            }
        }
        if part != self.part || tired != self.tired || centre != self.centre {
            self.part = part;
            self.tired = tired;
            self.centre = centre;
            self.base_mut().queue_redraw();
        }
    }

    fn draw(&mut self) {
        if self.part >= 1.0 {
            return;
        }
        let (centre, radius, width) = (self.centre, self.radius, self.width);
        let colour = if self.tired {
            self.tired_colour
        } else {
            self.colour
        };
        let top = -std::f32::consts::FRAC_PI_2;
        let end = top + std::f32::consts::TAU * self.part.clamp(0.0, 1.0);
        let back = Color::from_rgba(0.0, 0.0, 0.0, 0.45);
        let mut base = self.base_mut();
        base.draw_arc_ex(centre, radius, 0.0, std::f32::consts::TAU, 48, back)
            .width(width + 4.0)
            .antialiased(true)
            .done();
        if end > top {
            base.draw_arc_ex(centre, radius, top, end, 48, colour)
                .width(width)
                .antialiased(true)
                .done();
        }
    }
}
