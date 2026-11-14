//! The dev menu for the gamepad (R13, R22, `plan/diagnostics.md`).
//!
//! Hold Back (Select) to open it. A tap of Back is the playtest mark
//! (see `recorder.rs`). The d-pad and A use the menu. B goes back.
//! Keyboard: F10 opens it, the arrow keys and Enter use it.
//!
//! The menu pauses the game while it is open. The tests run with the
//! menu closed. They fly the fly camera along a fixed path and write
//! the numbers into the playtest report.

use crate::fly_camera::FLY_CAMERA_GROUP;
use crate::overlay::OVERLAY_GROUP;
use crate::playtest::benchmark::{
    leg_at, path_angle, path_xz, plan, LegStats, LegSummary, Run, TestKind, LEGS, PATH_HEIGHT_M,
};
use crate::playtest::devmenu_model::{Item, MenuState, Page};
use crate::recorder::RECORDER_GROUP;
use crate::terrain::TERRAIN_GROUP;
use godot::classes::node::ProcessMode;
use godot::classes::performance::Monitor;
use godot::classes::{
    Camera3D, CanvasLayer, ICanvasLayer, Input, InputEvent, InputEventJoypadButton, InputEventKey,
    Label, Os, PanelContainer, Performance, SceneTree,
};
use godot::global::{JoyButton, Key};
use godot::prelude::*;
use serde_json::{json, Value};

/// The group that holds the dev menu.
pub const DEVMENU_GROUP: &str = "bf_devmenu";

/// A press on the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nav {
    Up,
    Down,
    Left,
    Right,
    Accept,
    Back,
}

/// The buttons of the menu. The bear must not see them as new presses
/// when the menu closes.
const MENU_BUTTONS: [JoyButton; 6] = [
    JoyButton::A,
    JoyButton::B,
    JoyButton::DPAD_UP,
    JoyButton::DPAD_DOWN,
    JoyButton::DPAD_LEFT,
    JoyButton::DPAD_RIGHT,
];

/// The dev menu. Hold Back on the gamepad (or push F10) to open it.
#[derive(GodotClass)]
#[class(base = CanvasLayer, init)]
pub struct BfDevMenu {
    /// Hold Back this long to open the menu, in seconds. This must be
    /// longer than the tap time of the recorder (a tap is a mark).
    #[export]
    #[init(val = 0.6)]
    hold_seconds: f64,

    /// The time that each leg of a test flight takes, in seconds.
    #[export]
    #[init(val = 8.0)]
    leg_seconds: f32,

    /// The time before a test flight starts, in seconds. The terrain
    /// and the textures load in this time.
    #[export]
    #[init(val = 2.0)]
    settle_seconds: f32,

    /// The size of the menu text in pixels.
    #[export]
    #[init(val = 26)]
    font_size: i32,

    menu: MenuState,
    open: bool,
    /// The menu is closed, but the game waits until the buttons of the
    /// menu are up again.
    closing: bool,
    /// The test to start when the closing ends.
    start_after_close: Option<TestKind>,
    hold: f64,
    hold_armed: bool,
    shown_seed: i64,
    footer: String,
    zone_ready: bool,
    runner: Option<Runner>,
    /// After a test, put this seed back.
    restore_seed: Option<i64>,
    /// A test from the command line. The game quits after it.
    auto_test: Option<TestKind>,
    quit_after_test: bool,
    fast: bool,
    note_until: f64,
    time: f64,
    panel: Option<Gd<PanelContainer>>,
    label: Option<Gd<Label>>,
    status: Option<Gd<Label>>,
    base: Base<CanvasLayer>,
}

#[godot_api]
impl ICanvasLayer for BfDevMenu {
    fn ready(&mut self) {
        self.base_mut().add_to_group(DEVMENU_GROUP);
        self.base_mut().set_layer(120);
        self.base_mut().set_process_mode(ProcessMode::ALWAYS);
        self.hold_armed = true;

        let mut panel = PanelContainer::new_alloc();
        panel.set_position(Vector2::new(60.0, 60.0));
        let mut label = Label::new_alloc();
        label.add_theme_font_size_override("font_size", self.font_size);
        label.add_theme_color_override("font_color", Color::from_rgb(1.0, 1.0, 1.0));
        panel.add_child(&label);
        panel.set_visible(false);
        self.base_mut().add_child(&panel);
        self.panel = Some(panel);
        self.label = Some(label);

        let mut status = Label::new_alloc();
        status.set_position(Vector2::new(60.0, 20.0));
        status.add_theme_font_size_override("font_size", 22);
        status.add_theme_color_override("font_color", Color::from_rgb(1.0, 1.0, 0.7));
        status.add_theme_color_override("font_outline_color", Color::from_rgb(0.0, 0.0, 0.0));
        status.add_theme_constant_override("outline_size", 6);
        status.set_visible(false);
        self.base_mut().add_child(&status);
        self.status = Some(status);

        let tree = self.base().get_tree();
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("on_zone_changed");
            terrain.connect("zone_changed", &on_zone);
        }

        // `-- --bf-run-test=benchmark` runs a test and quits (for the
        // headless check). `--bf-test-fast` makes the flights short.
        for arg in Os::singleton().get_cmdline_user_args().as_slice() {
            let arg = arg.to_string();
            if let Some(name) = arg.strip_prefix("--bf-run-test=") {
                self.auto_test = TestKind::from_name(name);
                self.quit_after_test = true;
            } else if arg == "--bf-test-fast" {
                self.fast = true;
            }
        }
        if self.fast {
            self.leg_seconds = 0.5;
            self.settle_seconds = 0.3;
        }
    }

    fn process(&mut self, delta: f64) {
        self.time += delta;
        self.watch_back_button(delta);
        self.finish_closing();

        if let Some(kind) = self.auto_test {
            if self.zone_ready && self.runner.is_none() {
                self.auto_test = None;
                self.begin_test(kind);
            }
        }
        if self.runner.is_some() {
            self.step_test(delta as f32);
        } else if let Some(seed) = self.restore_seed {
            if self
                .terrain_call("make_zone", &[seed.to_variant()])
                .booleanize()
            {
                self.restore_seed = None;
            }
        }
        self.update_status();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if !key.is_pressed() || key.is_echo() {
                return;
            }
            let nav = match key.get_keycode() {
                Key::F10 => {
                    if self.open {
                        self.close_menu();
                    } else if self.runner.is_none() {
                        self.open_menu();
                    }
                    return;
                }
                Key::UP => Nav::Up,
                Key::DOWN => Nav::Down,
                Key::LEFT => Nav::Left,
                Key::RIGHT => Nav::Right,
                Key::ENTER | Key::KP_ENTER => Nav::Accept,
                Key::BACKSPACE => Nav::Back,
                _ => return,
            };
            self.navigate(nav);
            return;
        }
        let Ok(button) = event.try_cast::<InputEventJoypadButton>() else {
            return;
        };
        if !button.is_pressed() {
            return;
        }
        let nav = match button.get_button_index() {
            JoyButton::DPAD_UP => Nav::Up,
            JoyButton::DPAD_DOWN => Nav::Down,
            JoyButton::DPAD_LEFT => Nav::Left,
            JoyButton::DPAD_RIGHT => Nav::Right,
            JoyButton::A => Nav::Accept,
            JoyButton::B => Nav::Back,
            _ => return,
        };
        self.navigate(nav);
    }
}

#[godot_api]
impl BfDevMenu {
    /// The terrain sends this when a zone is on the screen.
    #[func]
    fn on_zone_changed(&mut self) {
        self.zone_ready = true;
    }

    /// Starts a test by name ("benchmark" or "draw_distance"). The menu
    /// uses this. A script can use it too.
    #[func]
    pub fn run_test(&mut self, name: GString) {
        if let Some(kind) = TestKind::from_name(&name.to_string()) {
            if self.runner.is_none() {
                self.begin_test(kind);
            }
        }
    }

    /// Gives true while a test runs.
    #[func]
    pub fn is_testing(&self) -> bool {
        self.runner.is_some()
    }

    /// Gives true while the menu is open.
    #[func]
    pub fn is_open(&self) -> bool {
        self.open
    }
}

impl BfDevMenu {
    // ---- opening and closing ----

    /// Hold Back for `hold_seconds` to open the menu.
    fn watch_back_button(&mut self, delta: f64) {
        let down = Input::singleton().is_joy_button_pressed(0, JoyButton::BACK);
        if !down {
            self.hold = 0.0;
            self.hold_armed = true;
            return;
        }
        if !self.hold_armed || self.open || self.closing || self.runner.is_some() {
            return;
        }
        self.hold += delta;
        if self.hold >= self.hold_seconds {
            // One hold opens the menu one time.
            self.hold_armed = false;
            self.open_menu();
        }
    }

    fn open_menu(&mut self) {
        if self.open {
            return;
        }
        self.open = true;
        self.closing = false;
        self.menu = MenuState::new();
        self.footer.clear();
        self.shown_seed = self
            .terrain_call("current_seed", &[])
            .try_to::<i64>()
            .unwrap_or(0);
        self.base().get_tree().set_pause(true);
        self.refresh_text();
        if let Some(panel) = &mut self.panel {
            panel.set_visible(true);
        }
    }

    /// Hides the menu. The game runs again when the buttons are up.
    fn close_menu(&mut self) {
        if !self.open {
            return;
        }
        self.open = false;
        self.closing = true;
        if let Some(panel) = &mut self.panel {
            panel.set_visible(false);
        }
    }

    /// If the menu is hidden and no menu button is down, unpause. A
    /// held A then does not become a jump of the bear.
    fn finish_closing(&mut self) {
        if !self.closing {
            return;
        }
        let input = Input::singleton();
        if MENU_BUTTONS
            .iter()
            .any(|b| input.is_joy_button_pressed(0, *b))
        {
            return;
        }
        self.closing = false;
        self.base().get_tree().set_pause(false);
        if let Some(kind) = self.start_after_close.take() {
            self.begin_test(kind);
        }
    }

    // ---- the menu ----

    fn navigate(&mut self, nav: Nav) {
        if self.runner.is_some() {
            if nav == Nav::Back {
                self.cancel_test("cancelled");
            }
            return;
        }
        if !self.open {
            return;
        }
        match nav {
            Nav::Up => self.menu.up(),
            Nav::Down => self.menu.down(),
            Nav::Left if self.menu.current() == Item::Seed => self.shown_seed -= 1,
            Nav::Right if self.menu.current() == Item::Seed => self.shown_seed += 1,
            Nav::Left | Nav::Right => {}
            Nav::Back => {
                if self.menu.back() {
                    self.close_menu();
                }
            }
            Nav::Accept => self.choose(self.menu.current()),
        }
        if self.open {
            self.refresh_text();
        }
    }

    fn choose(&mut self, item: Item) {
        self.footer.clear();
        match item {
            Item::FlyCamera => {
                self.close_menu();
                if let Some(mut camera) = self.group_node(FLY_CAMERA_GROUP) {
                    camera.call("toggle", &[]);
                }
            }
            Item::Overlay => {
                if let Some(mut overlay) = self.group_node(OVERLAY_GROUP) {
                    overlay.call("toggle", &[]);
                }
            }
            Item::NewZone => {
                self.close_menu();
                self.terrain_call("new_zone", &[]);
            }
            Item::Seed => {
                let started = self.terrain_call("make_zone", &[self.shown_seed.to_variant()]);
                if started.booleanize() {
                    self.close_menu();
                } else {
                    self.footer = "A zone is in work. Try again in a moment.".into();
                }
            }
            Item::SavePlaytest => {
                let name = self
                    .group_node(RECORDER_GROUP)
                    .map(|mut r| r.call("save_playtest", &[]).to_string())
                    .unwrap_or_default();
                self.footer = if name.is_empty() {
                    "The recorder is off. Nothing was saved.".into()
                } else {
                    format!("Saved {name}")
                };
            }
            Item::OpenTests => self.menu.go(Page::Tests),
            Item::Close => self.close_menu(),
            Item::Benchmark => self.start_test_from_menu(TestKind::Benchmark),
            Item::DrawDistance => self.start_test_from_menu(TestKind::DrawDistance),
            Item::BackToMain => {
                self.menu.back();
            }
        }
    }

    fn start_test_from_menu(&mut self, kind: TestKind) {
        self.start_after_close = Some(kind);
        self.close_menu();
    }

    fn refresh_text(&mut self) {
        let mut text = format!("{}\n\n", self.menu.page.title());
        for (i, item) in self.menu.page.items().iter().enumerate() {
            let mark = if i == self.menu.cursor { ">" } else { " " };
            text.push_str(&format!("{mark} {}\n", item.label(self.shown_seed)));
        }
        text.push_str("\nD-pad: move   A: choose   B: back");
        if !self.footer.is_empty() {
            text.push_str(&format!("\n{}", self.footer));
        }
        if let Some(label) = &mut self.label {
            label.set_text(&text);
        }
    }

    // ---- the tests ----

    fn begin_test(&mut self, kind: TestKind) {
        let tree = self.base().get_tree();
        let Some(camera) = tree
            .get_first_node_in_group(FLY_CAMERA_GROUP)
            .and_then(|n| n.try_cast::<Camera3D>().ok())
        else {
            godot_error!("BfDevMenu: no fly camera. The test did not start.");
            return;
        };
        let start_seed = self
            .terrain_call("current_seed", &[])
            .try_to::<i64>()
            .unwrap_or(0);
        let was_current = camera.is_current();
        let old_far = camera.get_far();
        let mut camera = camera;
        if !was_current {
            camera.make_current();
        }
        self.restore_seed = None;
        self.runner = Some(Runner {
            kind,
            runs: plan(kind),
            run: 0,
            phase: Phase::Prepare,
            t: 0.0,
            leg: 0,
            stats: LegStats::default(),
            legs: Vec::new(),
            done: Vec::new(),
            start_seed,
            was_current,
            old_far,
            camera,
            camera_y: 0.0,
        });
        self.show_note("Test started. B stops it.");
        godot_print!("BfDevMenu: test {} started.", kind.name());
    }

    fn step_test(&mut self, delta: f32) {
        let Some(mut runner) = self.runner.take() else {
            return;
        };
        let tree = self.base().get_tree();
        let generating = self.terrain_call("is_generating", &[]).booleanize();
        let events = runner.step(
            delta,
            &tree,
            Frame {
                zone_ready: self.zone_ready,
                generating,
                leg_seconds: self.leg_seconds,
                settle_seconds: self.settle_seconds,
            },
        );
        if events.request_zone {
            self.zone_ready = false;
            let seed = runner.runs[runner.run].seed;
            if !self
                .terrain_call("make_zone", &[seed.to_variant()])
                .booleanize()
            {
                // A zone is in work. Try again in the next frame.
                runner.phase = Phase::Prepare;
                self.zone_ready = false;
            }
        }
        for (name, info, data) in runner.done.drain(..) {
            if let Some(mut recorder) = self.group_node(RECORDER_GROUP) {
                recorder.call(
                    "test_result",
                    &[
                        name.to_variant(),
                        info.to_variant(),
                        data.to_string().to_variant(),
                    ],
                );
            }
        }
        if events.finished {
            self.end_test(runner, "done");
        } else {
            self.runner = Some(runner);
        }
    }

    fn cancel_test(&mut self, why: &str) {
        if let Some(runner) = self.runner.take() {
            self.end_test(runner, why);
        }
    }

    /// Gives the camera and the zone back.
    fn end_test(&mut self, mut runner: Runner, why: &str) {
        runner.camera.set_far(runner.old_far);
        if !runner.was_current {
            runner.camera.clear_current();
        }
        let seed = self
            .terrain_call("current_seed", &[])
            .try_to::<i64>()
            .unwrap_or(0);
        if seed != runner.start_seed {
            self.restore_seed = Some(runner.start_seed);
        }
        godot_print!("BfDevMenu: test {} {why}.", runner.kind.name());
        self.show_note(&format!("Test {why}."));
        if self.quit_after_test {
            self.base().get_tree().quit();
        }
    }

    // ---- small helpers ----

    fn show_note(&mut self, text: &str) {
        self.note_until = self.time + 4.0;
        if let Some(status) = &mut self.status {
            status.set_text(text);
            status.set_visible(true);
        }
    }

    fn update_status(&mut self) {
        let text = self.runner.as_ref().map(Runner::status);
        if let (Some(text), Some(status)) = (text, &mut self.status) {
            status.set_text(&text);
            status.set_visible(true);
        } else if self.time > self.note_until {
            if let Some(status) = &mut self.status {
                status.set_visible(false);
            }
        }
    }

    fn group_node(&self, group: &str) -> Option<Gd<Node>> {
        if !self.base().is_inside_tree() {
            return None;
        }
        self.base().get_tree().get_first_node_in_group(group)
    }

    fn terrain_call(&self, method: &str, args: &[Variant]) -> Variant {
        match self.group_node(TERRAIN_GROUP) {
            Some(mut terrain) => terrain.call(method, args),
            None => Variant::nil(),
        }
    }
}

// ---- the test runner ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Wait until the terrain is free, then ask for the zone.
    Prepare,
    /// Wait until the zone shows.
    WaitZone,
    /// The camera is at the start. The terrain loads.
    Settle,
    /// The camera flies. The frames count.
    Fly,
}

struct Frame {
    zone_ready: bool,
    generating: bool,
    leg_seconds: f32,
    settle_seconds: f32,
}

#[derive(Default)]
struct StepEvents {
    /// The runner wants the terrain to make the zone of the current run.
    request_zone: bool,
    finished: bool,
}

struct Runner {
    kind: TestKind,
    runs: Vec<Run>,
    run: usize,
    phase: Phase,
    t: f32,
    leg: usize,
    stats: LegStats,
    legs: Vec<LegSummary>,
    /// Results for the recorder: name, short text, data.
    done: Vec<(String, String, Value)>,
    start_seed: i64,
    was_current: bool,
    old_far: f32,
    camera: Gd<Camera3D>,
    camera_y: f32,
}

impl Runner {
    fn status(&self) -> String {
        let run = format!("run {}/{}", self.run + 1, self.runs.len());
        let what = match self.phase {
            Phase::Prepare | Phase::WaitZone => "making the zone".to_string(),
            Phase::Settle => "loading".to_string(),
            Phase::Fly => format!("leg {}/{}", self.leg + 1, LEGS),
        };
        format!("Test {}: {run}, {what}   (B: stop)", self.kind.name())
    }

    fn step(&mut self, delta: f32, tree: &Gd<SceneTree>, f: Frame) -> StepEvents {
        let mut out = StepEvents::default();
        match self.phase {
            Phase::Prepare => {
                if !f.generating {
                    out.request_zone = true;
                    self.phase = Phase::WaitZone;
                }
            }
            Phase::WaitZone => {
                if f.zone_ready && !f.generating {
                    self.camera.set_far(self.runs[self.run].far_m);
                    self.camera_y = f32::NAN;
                    self.t = 0.0;
                    self.phase = Phase::Settle;
                }
            }
            Phase::Settle => {
                self.place(tree, 0.0, delta, f.leg_seconds);
                self.t += delta;
                if self.t >= f.settle_seconds {
                    self.t = 0.0;
                    self.leg = 0;
                    self.stats = LegStats::default();
                    self.legs.clear();
                    self.phase = Phase::Fly;
                }
            }
            Phase::Fly => {
                let total = f.leg_seconds * LEGS as f32;
                self.t += delta;
                let (leg, _) = leg_at(self.t.min(total - 0.0001), f.leg_seconds);
                if leg != self.leg {
                    self.legs.push(self.stats.summary());
                    self.stats = LegStats::default();
                    self.leg = leg;
                }
                self.place(tree, self.t, delta, f.leg_seconds);
                let perf = Performance::singleton();
                self.stats.push(
                    delta * 1000.0,
                    perf.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
                    perf.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME),
                );
                if self.t >= total {
                    self.legs.push(self.stats.summary());
                    self.finish_run();
                    self.run += 1;
                    if self.run >= self.runs.len() {
                        out.finished = true;
                    } else {
                        self.phase = Phase::Prepare;
                    }
                }
            }
        }
        out
    }

    /// Puts the camera on the path at time `t` and looks ahead.
    fn place(&mut self, tree: &Gd<SceneTree>, t: f32, delta: f32, leg_seconds: f32) {
        let total = leg_seconds * LEGS as f32;
        let angle = path_angle(t, total);
        let [x, z] = path_xz(angle);
        let [lx, lz] = path_xz(angle + 0.3);
        let (ground, ahead) = match tree.get_first_node_in_group(TERRAIN_GROUP) {
            Some(mut terrain) => {
                let mut h = |x: f32, z: f32| {
                    terrain
                        .call("height_at", &[x.to_variant(), z.to_variant()])
                        .try_to::<f32>()
                        .unwrap_or(0.0)
                };
                (h(x, z), h(lx, lz))
            }
            None => (0.0, 0.0),
        };
        let wanted = ground + PATH_HEIGHT_M;
        // The height follows the ground with a small delay.
        self.camera_y = if self.camera_y.is_nan() {
            wanted
        } else {
            self.camera_y + (wanted - self.camera_y) * (delta * 3.0).min(1.0)
        };
        self.camera
            .set_global_position(Vector3::new(x, self.camera_y, z));
        self.camera.look_at(Vector3::new(lx, ahead, lz));
    }

    fn finish_run(&mut self) {
        let run = self.runs[self.run];
        let lines: Vec<String> = self
            .legs
            .iter()
            .enumerate()
            .map(|(i, l)| format!("leg {}: {}", i + 1, l.line()))
            .collect();
        let info = format!(
            "seed {}, far {:.0} m. {}",
            run.seed,
            run.far_m,
            lines.join(" | ")
        );
        let legs: Vec<Value> = self
            .legs
            .iter()
            .map(|l| {
                json!({
                    "frames": l.frames,
                    "fps": l.fps,
                    "frame_avg_ms": l.frame_avg_ms,
                    "frame_p95_ms": l.frame_p95_ms,
                    "frame_max_ms": l.frame_max_ms,
                    "draw_calls": l.draw_calls,
                    "primitives": l.primitives,
                })
            })
            .collect();
        let data = json!({
            "test": self.kind.name(),
            "seed": run.seed,
            "far_m": run.far_m,
            "legs": legs,
        });
        self.done.push((self.kind.name().to_string(), info, data));
    }
}
