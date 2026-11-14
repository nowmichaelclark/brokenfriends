//! The tests of the dev menu (S-113, `plan/diagnostics.md`): the
//! benchmark and the draw distance test.
//!
//! The runner makes the zone of a seed, flies a camera along a fixed
//! path and measures each frame. The numbers for each part of the path
//! go into the report of the playtest. The pure parts (the path, the
//! statistics and the text) are in `crate::playtest::bench`.

use crate::playtest::bench::{
    camera_height, make_json, make_markdown, plan, BenchPath, FrameData, PartResult, PathPoint,
    RunResult, RunSpec, RunStats, TestKind, TestReport,
};
use crate::recorder::RECORDER_GROUP;
use crate::terrain::TERRAIN_GROUP;
use crate::view_distance::{VIEW_CAMERA_GROUP, VIEW_DISTANCE_GROUP};
use godot::classes::performance::Monitor;
use godot::classes::{
    Camera3D, CanvasLayer, DisplayServer, INode, InputEvent, InputEventJoypadButton, InputEventKey,
    Label, Os, Performance, RenderingServer, Time,
};
use godot::global::{JoyButton, Key};
use godot::prelude::*;

/// The group that holds the test runner.
pub const TEST_RUNNER_GROUP: &str = "bf_test_runner";

/// What the runner does now.
enum Phase {
    /// Wait until the zone of the seed shows.
    Zone { requested: bool },
    /// The camera is at the start. Wait, so the load hitches pass.
    Settle { left: f32 },
    /// The camera flies. The time is in seconds.
    Fly { elapsed: f32 },
}

/// A test that runs now.
struct Active {
    kind: TestKind,
    specs: Vec<RunSpec>,
    run: usize,
    phase: Phase,
    stats: RunStats,
    results: Vec<(RunResult, Option<PartResult>)>,
    saved_seed: i64,
    saved_forced_m: f32,
    previous_camera: Option<Gd<Camera3D>>,
    camera: Gd<Camera3D>,
    last_usec: u64,
    /// The command line started this test. The game quits at the end.
    quit_at_end: bool,
}

/// Runs the tests of the dev menu.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct BfTestRunner {
    /// The distance of the corners of the camera path from the centre
    /// of the zone, in metres.
    #[export]
    #[init(val = 500.0)]
    path_radius_m: f32,

    /// The number of sides (parts) of the camera path.
    #[export]
    #[init(val = 8)]
    path_parts: i32,

    /// The speed of the camera in metres per second.
    #[export]
    #[init(val = 60.0)]
    path_speed_mps: f32,

    /// The height of the camera above the highest ground nearby, in metres.
    #[export]
    #[init(val = 25.0)]
    camera_clearance_m: f32,

    /// The time to wait at the start of each run, in seconds. The frames
    /// of this time are not measured.
    #[export]
    #[init(val = 2.5)]
    settle_seconds: f32,

    /// The field of view of the test camera in degrees.
    #[export]
    #[init(val = 75.0)]
    fov_degrees: f32,

    /// The angle of the camera below the horizon, in degrees.
    #[export]
    #[init(val = 12.0)]
    look_down_degrees: f32,

    active: Option<Active>,
    /// A test from the command line (`-- --bf-test=benchmark`). It starts
    /// when the first zone shows.
    pending_start: Option<TestKind>,
    label: Option<Gd<Label>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for BfTestRunner {
    fn ready(&mut self) {
        self.base_mut().add_to_group(TEST_RUNNER_GROUP);
        // A command line `-- --bf-test=draw-distance` starts a test (cloud runs).
        for arg in Os::singleton().get_cmdline_user_args().as_slice() {
            let arg = arg.to_string();
            if let Some(name) = arg.strip_prefix("--bf-test=") {
                match TestKind::from_name(name) {
                    Some(kind) => self.pending_start = Some(kind),
                    None => godot_error!("BfTestRunner: unknown test '{name}'"),
                }
            }
        }
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(104);
        let mut label = Label::new_alloc();
        label.set_position(Vector2::new(12.0, 150.0));
        label.add_theme_font_size_override("font_size", 20);
        label.add_theme_color_override("font_color", Color::from_rgb(1.0, 1.0, 0.8));
        label.add_theme_color_override("font_outline_color", Color::from_rgb(0.0, 0.0, 0.0));
        label.add_theme_constant_override("outline_size", 6);
        label.set_visible(false);
        layer.add_child(&label);
        self.base_mut().add_child(&layer);
        self.label = Some(label);
    }

    fn process(&mut self, delta: f64) {
        if self.active.is_none() {
            self.start_pending();
            return;
        }
        self.step(delta as f32);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if self.active.is_none() {
            return;
        }
        // B or Esc stops the test.
        let stop = if let Ok(button) = event.clone().try_cast::<InputEventJoypadButton>() {
            button.is_pressed() && button.get_button_index() == JoyButton::B
        } else if let Ok(key) = event.try_cast::<InputEventKey>() {
            key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::ESCAPE
        } else {
            false
        };
        if stop && !crate::dev_menu::is_open() {
            self.stop_test();
        }
    }

    fn exit_tree(&mut self) {
        // Put back what the test changed, if the game quits in a test.
        self.finish(true);
    }
}

#[godot_api]
impl BfTestRunner {
    /// Starts a test by its name ("benchmark" or "draw-distance").
    /// Gives true if the test started.
    #[func]
    pub fn start_test(&mut self, name: GString) -> bool {
        let Some(kind) = TestKind::from_name(&name.to_string()) else {
            godot_error!("BfTestRunner: unknown test '{name}'");
            return false;
        };
        self.start(kind, false)
    }

    /// Stops the test that runs now. The result so far goes into the report.
    #[func]
    pub fn stop_test(&mut self) {
        self.finish(true);
    }

    /// Tells if a test runs now.
    #[func]
    pub fn is_running(&self) -> bool {
        self.active.is_some()
    }
}

impl BfTestRunner {
    fn path(&self) -> BenchPath {
        BenchPath {
            radius_m: self.path_radius_m.max(50.0),
            parts: self.path_parts.clamp(3, 64) as usize,
            speed_mps: self.path_speed_mps.max(1.0),
        }
    }

    fn node(&self, group: &str) -> Option<Gd<Node>> {
        if !self.base().is_inside_tree() {
            return None;
        }
        self.base().get_tree().get_first_node_in_group(group)
    }

    /// Starts the test from the command line when the first zone shows.
    fn start_pending(&mut self) {
        let Some(kind) = self.pending_start else {
            return;
        };
        let Some(mut terrain) = self.node(TERRAIN_GROUP) else {
            return;
        };
        let ready = !terrain.call("is_generating", &[]).booleanize()
            && terrain.call("generation_seconds", &[]).to::<f32>() > 0.0;
        if ready {
            self.pending_start = None;
            self.start(kind, true);
        }
    }

    fn start(&mut self, kind: TestKind, quit_at_end: bool) -> bool {
        if self.active.is_some() || !self.base().is_inside_tree() {
            return false;
        }
        let Some(mut terrain) = self.node(TERRAIN_GROUP) else {
            godot_error!("BfTestRunner: there is no terrain.");
            return false;
        };
        let saved_seed = terrain.call("current_seed", &[]).to::<i64>();
        let saved_forced_m = self
            .node(VIEW_DISTANCE_GROUP)
            .map(|mut v| v.call("get_forced_distance", &[]).to::<f32>())
            .unwrap_or(0.0);
        let previous_camera = self.base().get_viewport().and_then(|v| v.get_camera_3d());

        let mut camera = Camera3D::new_alloc();
        camera.set_name("TestCamera");
        camera.set_fov(self.fov_degrees);
        camera.set_near(0.1);
        camera.add_to_group(VIEW_CAMERA_GROUP);
        self.base_mut().add_child(&camera);
        camera.make_current();

        if let Some(viewport) = self.base().get_viewport() {
            RenderingServer::singleton()
                .viewport_set_measure_render_time(viewport.get_viewport_rid(), true);
        }
        if let Some(mut recorder) = self.node(RECORDER_GROUP) {
            recorder.call("set_test", &[kind.name().to_variant()]);
        }
        let specs = plan(kind);
        let parts = self.path().parts;
        godot_print!("BfTestRunner: start {}", kind.title());
        self.active = Some(Active {
            kind,
            specs,
            run: 0,
            phase: Phase::Zone { requested: false },
            stats: RunStats::new(parts),
            results: Vec::new(),
            saved_seed,
            saved_forced_m,
            previous_camera,
            camera,
            last_usec: Time::singleton().get_ticks_usec(),
            quit_at_end,
        });
        self.begin_run();
        true
    }

    /// The number of mesh rings that the terrain draws now.
    fn terrain_lods(&self) -> i32 {
        self.node(TERRAIN_GROUP)
            .map(|mut t| t.call("mesh_lods", &[]).to::<i32>())
            .unwrap_or(0)
    }

    /// Sets the view distance and asks for the zone of the run.
    fn begin_run(&mut self) {
        let Some(active) = &mut self.active else {
            return;
        };
        let spec = active.specs[active.run];
        active.stats = RunStats::new(self.path_parts.clamp(3, 64) as usize);
        active.phase = Phase::Zone { requested: false };
        let forced = spec.view_distance_m.unwrap_or(0.0);
        if let Some(mut view) = self.node(VIEW_DISTANCE_GROUP) {
            view.call("set_forced_distance", &[forced.to_variant()]);
        }
    }

    /// The work of one frame.
    fn step(&mut self, delta: f32) {
        let usec = Time::singleton().get_ticks_usec();
        let path = self.path();
        let clearance = self.camera_clearance_m;
        let pitch = (-self.look_down_degrees).to_radians();
        let settle = self.settle_seconds.max(0.0);

        let Some(mut terrain) = self.node(TERRAIN_GROUP) else {
            self.finish(true);
            return;
        };
        let generating = terrain.call("is_generating", &[]).booleanize();
        let current_seed = terrain.call("current_seed", &[]).to::<i64>();

        // The state machine. `next` says what to do after the borrow.
        enum Next {
            Nothing,
            MakeZone(i64),
            RunDone,
        }
        let mut next = Next::Nothing;
        let mut frame_to_add: Option<(usize, f32)> = None;
        let text: String;
        let mut aim: Option<PathPoint> = None;
        {
            let Some(active) = &mut self.active else {
                return;
            };
            let spec = active.specs[active.run];
            let runs = active.specs.len();
            let title = active.kind.title();
            match &mut active.phase {
                Phase::Zone { requested } => {
                    if !*requested {
                        if !generating && current_seed == spec.seed {
                            active.phase = Phase::Settle { left: settle };
                        } else if !generating {
                            *requested = true;
                            next = Next::MakeZone(spec.seed);
                        }
                    } else if !generating {
                        active.phase = Phase::Settle { left: settle };
                    }
                    text = format!(
                        "{title}: run {}/{runs}, seed {}: the zone is in work ...",
                        active.run + 1,
                        spec.seed
                    );
                }
                Phase::Settle { left } => {
                    *left -= delta;
                    let p = path.at_time(0.0);
                    aim = Some(p);
                    text = format!(
                        "{title}: run {}/{runs}, seed {}: wait ...",
                        active.run + 1,
                        spec.seed
                    );
                    if *left <= 0.0 {
                        active.phase = Phase::Fly { elapsed: 0.0 };
                    }
                }
                Phase::Fly { elapsed } => {
                    *elapsed += delta;
                    let p = path.at_time(*elapsed);
                    aim = Some(p);
                    frame_to_add = Some((p.part, delta));
                    text = format!(
                        "{title}: run {}/{runs}, seed {}, part {}/{}. B or Esc stops the test.",
                        active.run + 1,
                        spec.seed,
                        p.part + 1,
                        path.parts
                    );
                    if *elapsed >= path.duration_s() {
                        next = Next::RunDone;
                    }
                }
            }
        }

        // Move the camera. The camera looks along the path.
        if let Some(point) = aim {
            // `camera_height` wants `Fn`: keep the node in a cell.
            let terrain_cell = std::cell::RefCell::new(&mut terrain);
            let height = camera_height(
                |x, z| {
                    terrain_cell
                        .borrow_mut()
                        .call("height_at", &[x.to_variant(), z.to_variant()])
                        .to::<f32>()
                },
                &point,
                clearance,
            );
            if let Some(active) = &mut self.active {
                let yaw = (-point.dir[0]).atan2(-point.dir[1]);
                active
                    .camera
                    .set_global_position(Vector3::new(point.x, height, point.z));
                active.camera.set_rotation(Vector3::new(pitch, yaw, 0.0));
            }
        }

        // Measure the frame. The camera has the new place now.
        let render_ms = self.base().get_viewport().map(|viewport| {
            let rs = RenderingServer::singleton();
            let rid = viewport.get_viewport_rid();
            (
                rs.viewport_get_measured_render_time_cpu(rid) as f32,
                rs.viewport_get_measured_render_time_gpu(rid) as f32,
            )
        });
        if let (Some((part, _)), Some(active)) = (frame_to_add, &mut self.active) {
            let ms = usec.saturating_sub(active.last_usec) as f32 / 1000.0;
            let perf = Performance::singleton();
            let (cpu_ms, gpu_ms) = render_ms.unwrap_or((0.0, 0.0));
            active.stats.add(
                part,
                FrameData {
                    ms,
                    draw_calls: perf.get_monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME) as f32,
                    primitives: perf.get_monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME) as f32,
                    cpu_ms,
                    gpu_ms,
                },
            );
        }
        if let Some(active) = &mut self.active {
            active.last_usec = usec;
        }

        if let Some(label) = &mut self.label {
            label.set_text(&text);
            label.set_visible(true);
        }

        match next {
            Next::Nothing => {}
            Next::MakeZone(seed) => {
                terrain.call("make_zone", &[seed.to_variant()]);
            }
            Next::RunDone => self.run_done(),
        }
    }

    /// Saves the numbers of the run. Starts the next run, or ends the test.
    fn run_done(&mut self) {
        let used = self
            .node(VIEW_DISTANCE_GROUP)
            .map(|mut v| v.call("view_distance_m", &[]).to::<f32>())
            .unwrap_or(0.0);
        let lods = self.terrain_lods();
        let Some(active) = &mut self.active else {
            return;
        };
        let spec = active.specs[active.run];
        let parts = active.stats.results();
        let total = active.stats.total();
        if let Some(t) = &total {
            godot_print!(
                "BfTestRunner: seed {}, view distance {:.0} m: {:.0} fps, 1% low {:.0} fps, {:.0} draw calls",
                spec.seed,
                used,
                t.avg_fps,
                t.low_1pct_fps,
                t.draw_calls
            );
        }
        active.results.push((
            RunResult {
                spec,
                used_view_distance_m: used,
                terrain_lods: lods,
                parts,
            },
            total,
        ));
        active.run += 1;
        if active.run >= active.specs.len() {
            self.finish(false);
        } else {
            self.begin_run();
        }
    }

    /// Ends the test: writes the result and puts back the game state.
    fn finish(&mut self, cancelled: bool) {
        let Some(mut active) = self.active.take() else {
            return;
        };
        // The run that was in work: keep the frames so far.
        if cancelled {
            let parts = active.stats.results();
            if !parts.is_empty() {
                let spec = active.specs[active.run.min(active.specs.len() - 1)];
                let total = active.stats.total();
                let lods = self.terrain_lods();
                let used = self
                    .node(VIEW_DISTANCE_GROUP)
                    .map(|mut v| v.call("view_distance_m", &[]).to::<f32>())
                    .unwrap_or(0.0);
                active.results.push((
                    RunResult {
                        spec,
                        used_view_distance_m: used,
                        terrain_lods: lods,
                        parts,
                    },
                    total,
                ));
            }
        }

        let headless = DisplayServer::singleton().get_name() == "headless";
        let report = TestReport {
            kind: active.kind,
            path: self.path(),
            clearance_m: self.camera_clearance_m,
            runs: &active.results,
            cancelled,
            headless,
        };
        let markdown = make_markdown(&report);
        let data = make_json(&report).to_string();
        godot_print!("BfTestRunner: {}", markdown);
        if let Some(mut recorder) = self.node(RECORDER_GROUP) {
            recorder.call(
                "add_test_result",
                &[
                    active.kind.name().to_variant(),
                    markdown.to_variant(),
                    data.to_variant(),
                ],
            );
            recorder.call("set_test", &["".to_variant()]);
        }

        // Put back the camera, the view distance and the zone.
        if active.camera.is_instance_valid() {
            active.camera.clear_current();
            active.camera.queue_free();
        }
        if let Some(mut previous) = active.previous_camera.take() {
            if previous.is_instance_valid() {
                previous.make_current();
            }
        }
        if let Some(mut view) = self.node(VIEW_DISTANCE_GROUP) {
            view.call("set_forced_distance", &[active.saved_forced_m.to_variant()]);
        }
        if let Some(mut terrain) = self.node(TERRAIN_GROUP) {
            if terrain.call("current_seed", &[]).to::<i64>() != active.saved_seed {
                terrain.call("make_zone", &[active.saved_seed.to_variant()]);
            }
        }
        if let Some(label) = &mut self.label {
            label.set_visible(false);
        }
        godot_print!("BfTestRunner: the test is done");
        if active.quit_at_end && self.base().is_inside_tree() {
            self.base().get_tree().quit();
        }
    }
}
