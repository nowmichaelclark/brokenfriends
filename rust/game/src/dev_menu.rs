//! The dev menu (R13, S-113). The gamepad opens it: hold Back for
//! 0.6 s. A tap of Back is the playtest mark, so a hold is free.
//! On the keyboard, push F10. The menu has the dev tools that were keys
//! before: fly camera, overlay, new zone, seed. It also has "Save
//! playtest" and the "Tests" page.

use crate::autoplay::AUTOPLAY_GROUP;
use crate::bear::BEAR_GROUP;
use crate::fly_camera::FLY_CAMERA_GROUP;
use crate::obstacles::OBSTACLES_GROUP;
use crate::overlay::OVERLAY_GROUP;
use crate::playtest::menu::{Action, MenuState, LIMB_NAMES};
use crate::recorder::RECORDER_GROUP;
use crate::terrain::TERRAIN_GROUP;
use crate::test_runner::TEST_RUNNER_GROUP;
use crate::view_distance::VIEW_DISTANCE_GROUP;
use godot::classes::{
    CanvasLayer, ColorRect, ICanvasLayer, Input, InputEvent, InputEventJoypadButton, InputEventKey,
    Label,
};
use godot::global::{JoyButton, Key};
use godot::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// The menu is open. The bear and the cameras read this and ignore the
/// input, so a button of the menu does not move the bear.
static OPEN: AtomicBool = AtomicBool::new(false);

/// Tells if the dev menu is open.
pub fn is_open() -> bool {
    OPEN.load(Ordering::Relaxed)
}

/// The group that holds the dev menu.
pub const DEV_MENU_GROUP: &str = "bf_dev_menu";

/// The dev menu. Hold Back on the gamepad (or push F10) to open it.
/// D-pad or arrow keys: choose and change. A or Enter: do. B or Esc:
/// back and close.
#[derive(GodotClass)]
#[class(base = CanvasLayer, init)]
pub struct BfDevMenu {
    /// How long to hold Back to open the menu, in seconds. A tap of Back
    /// is the playtest mark (it must be shorter than 0.4 s).
    #[export]
    #[init(val = 0.6)]
    hold_seconds: f64,

    /// The size of the text in pixels.
    #[export]
    #[init(val = 26)]
    font_size: i32,

    state: MenuState,
    panel: Option<Gd<ColorRect>>,
    title: Option<Gd<Label>>,
    body: Option<Gd<Label>>,
    /// The time that Back is held now, in seconds. After the menu opens,
    /// it stays above the limit until the button is let go.
    back_held: f64,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for BfDevMenu {
    fn ready(&mut self) {
        self.base_mut().add_to_group(DEV_MENU_GROUP);
        // Under the recorder notes (layer 110), over the overlay (100).
        self.base_mut().set_layer(105);
        self.make_panel();
        self.close();
    }

    fn exit_tree(&mut self) {
        OPEN.store(false, Ordering::Relaxed);
    }

    fn process(&mut self, delta: f64) {
        let held = Input::singleton().is_joy_button_pressed(0, JoyButton::BACK);
        if !held {
            self.back_held = 0.0;
            return;
        }
        let before = self.back_held;
        self.back_held += delta;
        if before < self.hold_seconds && self.back_held >= self.hold_seconds && !is_open() {
            self.open();
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if !key.is_pressed() || key.is_echo() {
                return;
            }
            let code = key.get_keycode();
            if code == Key::F10 {
                if is_open() {
                    self.close();
                } else {
                    self.open();
                }
                self.handled();
                return;
            }
            if !is_open() {
                return;
            }
            match code {
                Key::UP => self.state.up(),
                Key::DOWN => self.state.down(),
                Key::LEFT => {
                    let a = self.state.adjust(-1);
                    self.run(a);
                }
                Key::RIGHT => {
                    let a = self.state.adjust(1);
                    self.run(a);
                }
                Key::ENTER | Key::KP_ENTER => {
                    let a = self.state.activate();
                    self.run(a);
                }
                Key::ESCAPE | Key::BACKSPACE => {
                    let a = self.state.back();
                    self.run(a);
                }
                _ => return,
            }
            self.refresh();
            self.handled();
            return;
        }
        let Ok(button) = event.try_cast::<InputEventJoypadButton>() else {
            return;
        };
        if !is_open() || !button.is_pressed() {
            return;
        }
        match button.get_button_index() {
            JoyButton::DPAD_UP => self.state.up(),
            JoyButton::DPAD_DOWN => self.state.down(),
            JoyButton::DPAD_LEFT => {
                let a = self.state.adjust(-1);
                self.run(a);
            }
            JoyButton::DPAD_RIGHT => {
                let a = self.state.adjust(1);
                self.run(a);
            }
            JoyButton::A => {
                let a = self.state.activate();
                self.run(a);
            }
            JoyButton::B => {
                let a = self.state.back();
                self.run(a);
            }
            _ => return,
        }
        self.refresh();
        self.handled();
    }
}

#[godot_api]
impl BfDevMenu {
    /// Opens the menu.
    #[func]
    pub fn open(&mut self) {
        self.state.open();
        self.read_game_state();
        OPEN.store(true, Ordering::Relaxed);
        if let Some(panel) = &mut self.panel {
            panel.set_visible(true);
        }
        self.refresh();
    }

    /// Closes the menu.
    #[func]
    pub fn close(&mut self) {
        OPEN.store(false, Ordering::Relaxed);
        if let Some(panel) = &mut self.panel {
            panel.set_visible(false);
        }
    }
}

impl BfDevMenu {
    fn handled(&self) {
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.set_input_as_handled();
        }
    }

    fn make_panel(&mut self) {
        let mut panel = ColorRect::new_alloc();
        panel.set_position(Vector2::new(48.0, 48.0));
        panel.set_size(Vector2::new(1100.0, 560.0));
        panel.set_color(Color::from_rgba(0.06, 0.05, 0.04, 0.86));

        let mut title = Label::new_alloc();
        title.set_position(Vector2::new(24.0, 16.0));
        title.add_theme_font_size_override("font_size", self.font_size + 6);
        title.add_theme_color_override("font_color", Color::from_rgb(1.0, 0.92, 0.7));
        panel.add_child(&title);

        let mut body = Label::new_alloc();
        body.set_position(Vector2::new(24.0, 72.0));
        body.add_theme_font_size_override("font_size", self.font_size);
        body.add_theme_color_override("font_color", Color::from_rgb(1.0, 1.0, 1.0));
        panel.add_child(&body);

        self.base_mut().add_child(&panel);
        self.panel = Some(panel);
        self.title = Some(title);
        self.body = Some(body);
    }

    /// Reads the state of the game into the menu.
    fn read_game_state(&mut self) {
        if let Some(mut terrain) = self.node(TERRAIN_GROUP) {
            self.state.seed = terrain.call("current_seed", &[]).to::<i64>();
        }
        if let Some(mut fly) = self.node(FLY_CAMERA_GROUP) {
            self.state.fly_camera = fly.call("is_active", &[]).booleanize();
        }
        if let Some(mut overlay) = self.node(OVERLAY_GROUP) {
            self.state.overlay = overlay.call("is_shown", &[]).booleanize();
        }
        if let Some(mut view) = self.node(VIEW_DISTANCE_GROUP) {
            let forced = view.call("get_forced_distance", &[]).to::<f32>();
            self.state
                .set_view_distance((forced > 0.0).then_some(forced));
            self.state.smog_distance_m = view.call("smog_distance_m", &[]).to::<f32>();
        }
        self.state.test_running = self.any_test_running();
        if let Some(mut bear) = self.node(BEAR_GROUP) {
            for (i, name) in LIMB_NAMES.iter().enumerate() {
                self.state.limbs[i] = bear.call("has_limb", &[name.to_variant()]).booleanize();
            }
        }
    }

    fn node(&self, group: &str) -> Option<Gd<Node>> {
        if !self.base().is_inside_tree() {
            return None;
        }
        self.base().get_tree().get_first_node_in_group(group)
    }

    /// Does what a button press asks.
    fn run(&mut self, action: Action) {
        match action {
            Action::None => {}
            Action::Close => self.close(),
            Action::ToggleFlyCamera => {
                if let Some(mut fly) = self.node(FLY_CAMERA_GROUP) {
                    fly.call("toggle", &[]);
                }
                self.read_game_state();
            }
            Action::ToggleOverlay => {
                if let Some(mut overlay) = self.node(OVERLAY_GROUP) {
                    overlay.call("toggle_shown", &[]);
                }
                self.read_game_state();
            }
            Action::NewZone => {
                if let Some(mut terrain) = self.node(TERRAIN_GROUP) {
                    terrain.call("new_zone", &[]);
                }
                self.read_game_state();
            }
            Action::MakeZone(seed) => {
                if let Some(mut terrain) = self.node(TERRAIN_GROUP) {
                    terrain.call("make_zone", &[seed.to_variant()]);
                }
                self.read_game_state();
            }
            Action::SetViewDistance(metres) => {
                if let Some(mut view) = self.node(VIEW_DISTANCE_GROUP) {
                    view.call("set_forced_distance", &[metres.unwrap_or(0.0).to_variant()]);
                }
            }
            Action::NextObstacle => {
                if let Some(mut obstacles) = self.node(OBSTACLES_GROUP) {
                    obstacles.call("go_to_next", &[]);
                }
                // The bear moves at once; close the menu so Mikey sees it.
                self.close();
            }
            Action::SavePlaytest => {
                if let Some(mut recorder) = self.node(RECORDER_GROUP) {
                    let name = recorder.call("save_playtest", &[]).to_string();
                    if name.is_empty() {
                        godot_warn!("BfDevMenu: the recorder is off, so no playtest was saved.");
                    }
                }
            }
            Action::StartTest(kind) => {
                if let Some(mut runner) = self.node(TEST_RUNNER_GROUP) {
                    let started = runner
                        .call("start_test", &[kind.name().to_variant()])
                        .booleanize();
                    if started {
                        self.close();
                    }
                }
                self.read_game_state();
            }
            Action::SetLimb(name, present) => {
                if let Some(mut bear) = self.node(BEAR_GROUP) {
                    bear.call("set_limb", &[name.to_variant(), present.to_variant()]);
                }
                self.read_game_state();
            }
            Action::Tumble(hard) => {
                if let Some(mut bear) = self.node(BEAR_GROUP) {
                    bear.call("tumble_now", &[hard.to_variant()]);
                }
                self.close();
            }
            Action::RestoreLimbs => {
                if let Some(mut bear) = self.node(BEAR_GROUP) {
                    bear.call("restore_limbs", &[]);
                }
                self.read_game_state();
            }
            Action::StartAutoplay => {
                if let Some(mut autoplay) = self.node(AUTOPLAY_GROUP) {
                    let started = autoplay
                        .call("start_autoplay", &["tour".to_variant()])
                        .booleanize();
                    if started {
                        self.close();
                    }
                }
                self.read_game_state();
            }
            Action::StopTest => {
                if let Some(mut runner) = self.node(TEST_RUNNER_GROUP) {
                    runner.call("stop_test", &[]);
                }
                if let Some(mut autoplay) = self.node(AUTOPLAY_GROUP) {
                    autoplay.call("stop_autoplay", &[]);
                }
                self.read_game_state();
            }
        }
    }

    /// Tells if a test of the runner or the autoplay bot runs now.
    fn any_test_running(&self) -> bool {
        [TEST_RUNNER_GROUP, AUTOPLAY_GROUP].iter().any(|group| {
            self.node(group)
                .map(|mut n| n.call("is_running", &[]).booleanize())
                .unwrap_or(false)
        })
    }

    /// Shows the page.
    fn refresh(&mut self) {
        self.state.test_running = self.any_test_running();
        let (lines, cursor) = self.state.lines();
        let mut text = String::new();
        for (i, line) in lines.iter().enumerate() {
            text.push_str(if i == cursor { "> " } else { "   " });
            text.push_str(line);
            text.push('\n');
        }
        text.push_str(
            "\nD-pad: choose and change   A: do   B: back   (keyboard: arrows, Enter, Esc, F10)",
        );
        if let Some(title) = &mut self.title {
            title.set_text(self.state.title());
        }
        if let Some(body) = &mut self.body {
            body.set_text(&text);
        }
    }
}
