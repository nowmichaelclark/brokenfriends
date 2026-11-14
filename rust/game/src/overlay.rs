//! The F3 debug overlay.

use crate::bear::BEAR_GROUP;
use crate::terrain::TERRAIN_GROUP;
use godot::classes::performance::Monitor;
use godot::classes::{CanvasLayer, ICanvasLayer, InputEvent, InputEventKey, Label, Performance};
use godot::global::Key;
use godot::prelude::*;

/// Shows fps, frame time, draw calls, primitives, video memory, the
/// seed and the state of the bear. Push F3 to show or hide it.
#[derive(GodotClass)]
#[class(base = CanvasLayer, init)]
pub struct BfDebugOverlay {
    /// If this is on, the overlay shows when the game starts.
    #[export]
    #[init(val = true)]
    show_at_start: bool,

    /// The time between two updates of the text, in seconds.
    #[export]
    #[init(val = 0.25)]
    update_interval: f64,

    /// The size of the text in pixels.
    #[export]
    #[init(val = 16)]
    font_size: i32,

    label: Option<Gd<Label>>,
    time_to_update: f64,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for BfDebugOverlay {
    fn ready(&mut self) {
        let mut label = Label::new_alloc();
        label.set_position(Vector2::new(12.0, 8.0));
        label.add_theme_font_size_override("font_size", self.font_size);
        label.add_theme_color_override("font_color", Color::from_rgb(1.0, 1.0, 1.0));
        label.add_theme_color_override("font_outline_color", Color::from_rgb(0.0, 0.0, 0.0));
        label.add_theme_constant_override("outline_size", 6);
        self.base_mut().add_child(&label);
        self.label = Some(label);
        let show = self.show_at_start;
        self.base_mut().set_layer(100);
        self.base_mut().set_visible(show);
    }

    fn process(&mut self, delta: f64) {
        if !self.base().is_visible() {
            return;
        }
        self.time_to_update -= delta;
        if self.time_to_update > 0.0 {
            return;
        }
        self.time_to_update = self.update_interval;
        let text = self.make_text();
        if let Some(label) = &mut self.label {
            label.set_text(&text);
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(key) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::F3 {
            let visible = !self.base().is_visible();
            self.base_mut().set_visible(visible);
            self.time_to_update = 0.0;
        }
    }
}

impl BfDebugOverlay {
    fn make_text(&self) -> String {
        let perf = Performance::singleton();
        let fps = perf.get_monitor(Monitor::TIME_FPS);
        let frame_ms = if fps > 0.0 { 1000.0 / fps } else { 0.0 };
        let cpu_ms = perf.get_monitor(Monitor::TIME_PROCESS) * 1000.0;
        let draw_calls = perf.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME);
        let primitives = perf.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME);
        let video_mb = perf.get_monitor(Monitor::RENDER_VIDEO_MEM_USED) / (1024.0 * 1024.0);

        let mut seed = String::from("no terrain");
        let mut bear = String::from("no bear");
        if self.base().is_inside_tree() {
            let tree = self.base().get_tree();
            if let Some(mut node) = tree.get_first_node_in_group(BEAR_GROUP) {
                bear = node.call("debug_text", &[]).to_string();
            }
            if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
                seed = terrain.call("current_seed", &[]).to_string();
                if terrain.call("is_generating", &[]).booleanize() {
                    seed.push_str(" (a new zone is in work)");
                }
            }
        }

        format!(
            "{fps:.0} fps   {frame_ms:.1} ms (script {cpu_ms:.1} ms)\n\
             draw calls {draw_calls:.0}   primitives {primitives:.0}\n\
             video memory {video_mb:.0} MB\n\
             seed {seed}\n\
             bear: {bear}\n\
             F2: fly camera   F3: hide   F4: new zone   F9: mark"
        )
    }
}
