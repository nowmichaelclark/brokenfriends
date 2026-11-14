//! The playtest recorder node (R22, `plan/diagnostics.md`).
//!
//! `BfRecorder` is always on in a game with a window. It writes a
//! session folder in `playtests/` next to the game:
//! - `session.json`: the build, the hardware and the settings.
//! - `samples.jsonl`: one line each second.
//! - `events.jsonl`: hitches, low fps, stuck, climb loops, the bear
//!   below the ground, bear state changes, zones, marks, log errors.
//! - `shots/`: JPEG screenshots. `marks/`: the last 10 s of frames for
//!   each mark.
//! - `frametimes.json`: the histogram of all frame times.
//!
//! When the game quits, the node writes `report.md` and packs all of
//! it, with the Godot log, into one zip. A folder without a zip (after
//! a crash) is packed at the next start.
//!
//! The logic without Godot is in `crate::playtest`.

use crate::bear::BEAR_GROUP;
use crate::playtest::detect::{Detectors, Found, FrameInput};
use crate::playtest::report::{make_report, ReportInput};
use crate::playtest::stats::{FrameHistogram, SecondStats};
use crate::playtest::{
    files_to_delete, log_problems, parse_lines, scrub, session_name, BearSample, CameraSample,
    Event, FrameRecord, InputSample, Sample,
};
use crate::terrain::TERRAIN_GROUP;
use godot::classes::image::Interpolation;
use godot::classes::performance::Monitor;
use godot::classes::{
    CanvasLayer, DisplayServer, Engine, INode, Input, InputEvent, InputEventJoypadButton,
    InputEventKey, Label, Node, Os, Performance, ProjectSettings, RenderingServer, Time, ZipPacker,
};
use godot::global::{JoyAxis, JoyButton, Key};
use godot::prelude::*;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// The group that holds the recorder. The dev menu (S-113) finds it here.
pub const RECORDER_GROUP: &str = "bf_recorder";

/// At most this many hitches become events. The histogram counts all.
const MAX_HITCH_EVENTS: usize = 2000;

/// At most this many log errors and warnings become events.
const MAX_LOG_EVENTS: usize = 200;

/// The buttons that a sample lists when they are held.
const PAD_BUTTONS: [(JoyButton, &str); 15] = [
    (JoyButton::A, "pad A"),
    (JoyButton::B, "pad B"),
    (JoyButton::X, "pad X"),
    (JoyButton::Y, "pad Y"),
    (JoyButton::BACK, "pad back"),
    (JoyButton::START, "pad start"),
    (JoyButton::LEFT_STICK, "pad L3"),
    (JoyButton::RIGHT_STICK, "pad R3"),
    (JoyButton::LEFT_SHOULDER, "pad LB"),
    (JoyButton::RIGHT_SHOULDER, "pad RB"),
    (JoyButton::DPAD_UP, "pad up"),
    (JoyButton::DPAD_DOWN, "pad down"),
    (JoyButton::DPAD_LEFT, "pad left"),
    (JoyButton::DPAD_RIGHT, "pad right"),
    (JoyButton::GUIDE, "pad guide"),
];

/// The keys that a sample lists when they are held.
const KEYS: [(Key, &str); 10] = [
    (Key::W, "W"),
    (Key::A, "A"),
    (Key::S, "S"),
    (Key::D, "D"),
    (Key::UP, "up"),
    (Key::DOWN, "down"),
    (Key::LEFT, "left"),
    (Key::RIGHT, "right"),
    (Key::SPACE, "space"),
    (Key::SHIFT, "shift"),
];

/// Records each playtest (R22). Tap Back on the gamepad or push F9 to
/// save a mark. The files go to `playtests/` next to the game.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct BfRecorder {
    /// If this is off, the recorder does nothing.
    #[export]
    #[init(val = true)]
    enabled: bool,

    /// Record also without a window (`--headless`). Screenshots are off
    /// then. The command line `-- --bf-record` does the same.
    #[export]
    record_headless: bool,

    /// The time between two regular screenshots, in seconds.
    #[export]
    #[init(val = 30.0)]
    screenshot_interval: f64,

    /// The shortest time between two screenshots of hitches or "stuck", in seconds.
    #[export]
    #[init(val = 10.0)]
    event_screenshot_gap: f64,

    /// At most this many screenshots of hitches and "stuck" in a session.
    #[export]
    #[init(val = 60)]
    max_event_screenshots: i32,

    /// The width of a screenshot in pixels.
    #[export]
    #[init(val = 960)]
    screenshot_width: i32,

    /// The height of a screenshot in pixels.
    #[export]
    #[init(val = 540)]
    screenshot_height: i32,

    /// The JPEG quality of a screenshot (0 to 1).
    #[export]
    #[init(val = 0.7)]
    jpeg_quality: f32,

    /// A mark saves the frames of this many seconds before it.
    #[export]
    #[init(val = 10.0)]
    mark_seconds: f64,

    /// A press of Back shorter than this is a mark (a tap), in seconds.
    /// A longer hold is for the dev menu (S-113).
    #[export]
    #[init(val = 0.4)]
    tap_seconds: f64,

    /// The playtests folder can hold this many megabytes. Older zips go.
    #[export]
    #[init(val = 200)]
    folder_limit_mb: i32,

    /// A frame longer than this is a hitch, in milliseconds.
    #[export]
    #[init(val = 50.0)]
    hitch_ms: f32,

    /// A hitch must also be this many times longer than the average frame of the last second.
    #[export]
    #[init(val = 2.0)]
    hitch_factor: f32,

    /// Below this fps for `low_fps_seconds`, the recorder writes a "low fps" event.
    #[export]
    #[init(val = 55.0)]
    low_fps: f32,

    /// How long the fps must be low before an event, in seconds.
    #[export]
    #[init(val = 2.0)]
    low_fps_seconds: f64,

    /// After a new zone shows, hitches count as "loading" for this time, in seconds.
    #[export]
    #[init(val = 1.5)]
    loading_seconds: f64,

    active: bool,
    headless: bool,
    /// The playtests folder and the folder of this session.
    root: PathBuf,
    dir: PathBuf,
    name: String,
    session: Value,
    samples_file: Option<File>,
    events_file: Option<File>,
    start_usec: u64,
    last_usec: u64,
    second_start: f64,
    second: SecondStats,
    histogram: FrameHistogram,
    detectors: Detectors,
    frames: VecDeque<FrameRecord>,
    next_shot: f64,
    last_event_shot: f64,
    event_shots: i32,
    shot_number: u32,
    /// The last frame took a screenshot. A hitch in the next frame is ours.
    shot_frame: bool,
    marks: u32,
    back_down_at: Option<f64>,
    loading_until: f64,
    has_zone: bool,
    pipeline_compiles: u32,
    log_path: Option<PathBuf>,
    log_offset: u64,
    log_events: usize,
    hitch_events: usize,
    /// The time of the recorder in this second, in microseconds, and the frames.
    own_usec: u64,
    own_frames: u32,
    log_rest: String,
    next_histogram_save: f64,
    note: Option<Gd<Label>>,
    note_until: f64,
    finished: bool,
    base: Base<Node>,
}

#[godot_api]
impl INode for BfRecorder {
    fn ready(&mut self) {
        self.base_mut().add_to_group(RECORDER_GROUP);
        self.headless = DisplayServer::singleton().get_name() == "headless";
        // `-- --bf-record` on the command line records a headless run (tests).
        let forced = Os::singleton()
            .get_cmdline_user_args()
            .as_slice()
            .iter()
            .any(|a| *a == "--bf-record");
        if !self.enabled || (self.headless && !self.record_headless && !forced) {
            return;
        }
        // Run after the other nodes, so a sample sees the newest state.
        self.base_mut().set_process_priority(1000);
        self.base_mut()
            .set_process_mode(godot::classes::node::ProcessMode::ALWAYS);
        self.start();
    }

    fn process(&mut self, _delta: f64) {
        if self.active {
            self.frame();
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if !self.active {
            return;
        }
        let t = self.now();
        if let Ok(key) = event.clone().try_cast::<InputEventKey>() {
            if key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::F9 {
                self.mark();
            }
            return;
        }
        if let Ok(button) = event.try_cast::<InputEventJoypadButton>() {
            if button.get_button_index() != JoyButton::BACK {
                return;
            }
            if button.is_pressed() {
                self.back_down_at = Some(t);
            } else if let Some(down) = self.back_down_at.take() {
                if t - down <= self.tap_seconds {
                    self.mark();
                }
            }
        }
    }

    fn on_notification(&mut self, what: godot::classes::notify::NodeNotification) {
        if what == godot::classes::notify::NodeNotification::WM_CLOSE_REQUEST {
            self.finish(true);
        }
    }

    fn exit_tree(&mut self) {
        self.finish(true);
    }
}

#[godot_api]
impl BfRecorder {
    /// Saves a mark: a screenshot, the last frames and a "mark" event.
    /// Tap Back on the gamepad or push F9.
    #[func]
    pub fn mark(&mut self) {
        if !self.active {
            return;
        }
        self.marks += 1;
        let n = self.marks;
        let t = self.now();
        // The frames of the last seconds, one line for each frame.
        let mut text = String::new();
        for f in &self.frames {
            if let Ok(line) = serde_json::to_string(f) {
                text.push_str(&line);
                text.push('\n');
            }
        }
        let marks = self.dir.join("marks");
        let _ = fs::create_dir_all(&marks);
        let _ = fs::write(marks.join(format!("mark-{n:02}.jsonl")), text);
        let shot = self.screenshot(&format!("mark-{n:02}"));
        let mut event = self.event_here("mark", format!("Mark {n}"));
        event.value = Some(n as f64);
        event.shot = shot;
        self.write_event(event);
        self.show_note(&format!("Mark {n} saved."), t);
    }

    /// Writes the report and the zip of the session so far. The
    /// recording goes on. The dev menu calls this ("Save playtest").
    #[func]
    pub fn save_playtest(&mut self) -> GString {
        if !self.active {
            return GString::new();
        }
        self.flush();
        let zip = pack_session(&self.dir, &self.root, true, &self.scrub_pairs(), None);
        if let Some(zip) = &zip {
            let t = self.now();
            self.show_note("Playtest saved.", t);
            godot_print!("BfRecorder: saved {}", file_name(zip));
        }
        zip.map(|z| GString::from(&file_name(&z)))
            .unwrap_or_default()
    }

    /// Gives the name of the session folder, or "" if the recorder is off.
    #[func]
    pub fn session_name(&self) -> GString {
        GString::from(&self.name)
    }

    /// Called when the terrain shows a new zone.
    #[func]
    fn on_zone_changed(&mut self) {
        if !self.active {
            return;
        }
        let t = self.now();
        self.has_zone = true;
        self.loading_until = t + self.loading_seconds;
        let Some(mut terrain) = self.terrain() else {
            return;
        };
        let seed = terrain.call("current_seed", &[]).to::<i64>();
        let seconds = terrain.call("generation_seconds", &[]).to::<f32>();
        let import = terrain.call("import_ms", &[]).to::<f32>();
        let mut event = self.event_here(
            "zone",
            format!("seed {seed}, generation {seconds:.2} s, Terrain3D import {import:.0} ms"),
        );
        event.value = Some(seed as f64);
        self.write_event(event);
        let zone = json!({"t": t, "seed": seed, "generation_s": seconds, "import_ms": import});
        if let Some(zones) = self.session["zones"].as_array_mut() {
            zones.push(zone);
        } else {
            self.session["zones"] = json!([zone]);
        }
        self.write_session();
    }
}

impl BfRecorder {
    fn now(&self) -> f64 {
        let usec = Time::singleton().get_ticks_usec();
        usec.saturating_sub(self.start_usec) as f64 / 1_000_000.0
    }

    /// Makes the session folder and starts the recording.
    fn start(&mut self) {
        let Some(root) = playtests_folder() else {
            godot_warn!("BfRecorder: no folder can be written. The recorder is off.");
            return;
        };
        self.root = root;
        self.log_path = godot_log_path();
        // The old log was rotated at this start: pack old sessions first.
        finish_old_sessions(&self.root, &self.scrub_pairs());
        self.cleanup();

        let started = Time::singleton()
            .get_datetime_string_from_system()
            .to_string();
        self.name = session_name(&started);
        self.dir = self.root.join(&self.name);
        if fs::create_dir_all(self.dir.join("shots")).is_err() {
            godot_warn!("BfRecorder: cannot make the session folder. The recorder is off.");
            return;
        }
        let open = |name: &str| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.dir.join(name))
                .ok()
        };
        self.samples_file = open("samples.jsonl");
        self.events_file = open("events.jsonl");
        self.start_usec = Time::singleton().get_ticks_usec();
        self.last_usec = self.start_usec;
        self.session = session_info(&started, self.headless);
        self.write_session();

        self.detectors.params.hitch_ms = self.hitch_ms;
        self.detectors.params.hitch_factor = self.hitch_factor;
        self.detectors.params.low_fps = self.low_fps;
        self.detectors.params.low_fps_seconds = self.low_fps_seconds;
        self.next_shot = 5.0;
        self.last_event_shot = -1.0e9;
        self.active = true;

        // Measure the CPU and GPU time of the main viewport.
        if let Some(viewport) = self.base().get_viewport() {
            RenderingServer::singleton()
                .viewport_set_measure_render_time(viewport.get_viewport_rid(), true);
        }
        // Skip the log of the time before the recorder.
        if let Some(path) = &self.log_path {
            self.log_offset = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        }
        if let Some(mut terrain) = self.terrain() {
            let callable = self.base().callable("on_zone_changed");
            terrain.connect("zone_changed", &callable);
        }
        self.make_note_label();
        let info = if self.headless {
            "recording started (headless: no screenshots)"
        } else {
            "recording started"
        };
        let event = self.event_here("recorder", info.into());
        self.write_event(event);
        godot_print!("BfRecorder: recording {}", self.name);
    }

    /// The work of one frame.
    fn frame(&mut self) {
        let usec = Time::singleton().get_ticks_usec();
        let ms = usec.saturating_sub(self.last_usec) as f32 / 1000.0;
        self.last_usec = usec;
        let t = self.now();
        self.histogram.add(ms);
        self.second.add(ms);

        let bear = self.bear_state();
        let pos = bear
            .as_ref()
            .and_then(|d| d.get("pos")?.try_to::<Vector3>().ok());
        let stick_held = bear
            .as_ref()
            .and_then(|d| d.get("stick_held")?.try_to::<bool>().ok())
            .unwrap_or(false);
        let generating = self
            .terrain()
            .map(|mut t| t.call("is_generating", &[]).booleanize())
            .unwrap_or(false);
        let loading = !self.has_zone || generating || t < self.loading_until;
        // On a wall, the heightmap is not exact enough for this test.
        let on_wall = bear
            .as_ref()
            .and_then(|d| d.get("state"))
            .map(|v| matches!(v.to_string().as_str(), "climb" | "slide" | "mantle"))
            .unwrap_or(false);
        let ground = match (pos, loading || on_wall) {
            (Some(p), false) => self.terrain().map(|mut terrain| {
                terrain
                    .call("height_at", &[p.x.to_variant(), p.z.to_variant()])
                    .to::<f32>()
            }),
            _ => None,
        };
        let pos_arr = pos.map(|p| [p.x, p.y, p.z]);

        // Shader compiles near a hitch tell its cause.
        let compiles = pipeline_compiles();
        let compiled = compiles > self.pipeline_compiles;
        self.pipeline_compiles = compiles;

        let found = self.detectors.frame(FrameInput {
            t,
            frame_ms: ms,
            bear: pos_arr,
            stick_held,
            ground,
        });
        let own = std::mem::take(&mut self.shot_frame);
        for f in found {
            self.found(f, t, loading, own, compiled);
        }
        self.bear_events(t);

        // The frames for the marks.
        let input = Input::singleton();
        let velocity = bear
            .as_ref()
            .and_then(|d| d.get("velocity")?.try_to::<Vector3>().ok());
        let state = bear
            .as_ref()
            .and_then(|d| d.get("state"))
            .map(|v| v.to_string())
            .unwrap_or_default();
        let camera = self.camera_sample().map(|c| c.pos);
        self.frames.push_back(FrameRecord {
            t,
            ms,
            bear: pos_arr,
            velocity: velocity.map(|v| [v.x, v.y, v.z]),
            state,
            camera,
            move_stick: stick(&input, JoyAxis::LEFT_X, JoyAxis::LEFT_Y),
            look: stick(&input, JoyAxis::RIGHT_X, JoyAxis::RIGHT_Y),
        });
        while let Some(old) = self.frames.front() {
            if t - old.t > self.mark_seconds {
                self.frames.pop_front();
            } else {
                break;
            }
        }

        // The own cost, before the work of each second and the screenshots.
        self.own_usec += Time::singleton().get_ticks_usec().saturating_sub(usec);
        self.own_frames += 1;
        if t - self.second_start >= 1.0 {
            self.write_sample(t, bear);
            self.read_log(t);
        }
        if t >= self.next_histogram_save {
            self.next_histogram_save = t + 10.0;
            self.save_histogram();
        }
        // One screenshot in a frame is enough.
        if t >= self.next_shot && !self.shot_frame {
            self.next_shot = t + self.screenshot_interval.max(1.0);
            let shot = self.screenshot("regular");
            if shot.is_some() {
                self.shot_frame = true;
            }
        }
        if self.note.is_some() && t >= self.note_until {
            if let Some(note) = &mut self.note {
                note.set_visible(false);
            }
        }
    }

    /// Writes an event for a find of a detector.
    fn found(&mut self, f: Found, t: f64, loading: bool, own: bool, compiled: bool) {
        if matches!(f, Found::Hitch { .. }) {
            if self.hitch_events >= MAX_HITCH_EVENTS {
                return;
            }
            self.hitch_events += 1;
        }
        let (kind, info, value, shot_tag) = match f {
            Found::Hitch { ms } => {
                let mut info = format!("frame {ms:.0} ms");
                if compiled {
                    info.push_str(", shader compile in this frame");
                }
                if own {
                    info.push_str(", after a recorder screenshot");
                }
                ("hitch", info, Some(ms as f64), Some("hitch"))
            }
            Found::LowFpsStart { fps } => (
                "low_fps",
                format!(
                    "below {:.0} fps for {:.0} s, now {fps:.0} fps",
                    self.low_fps, self.low_fps_seconds
                ),
                Some(fps as f64),
                None,
            ),
            Found::LowFpsEnd { seconds, min_fps } => (
                "low_fps_end",
                format!("fps good again after {seconds:.1} s, lowest {min_fps:.0} fps"),
                Some(seconds),
                None,
            ),
            Found::Stuck => (
                "stuck",
                "the stick is held, but the bear moved less than 5 cm in 2 s".into(),
                None,
                Some("stuck"),
            ),
            Found::ClimbLoop { grabs } => (
                "climb_loop",
                format!("{grabs} grabs in 10 s at one place"),
                Some(grabs as f64),
                None,
            ),
            Found::ThroughGround { depth } => (
                "through_ground",
                format!("the bear is {depth:.2} m below the terrain"),
                Some(depth as f64),
                None,
            ),
        };
        let mut event = self.event_here(kind, info);
        event.value = value;
        event.loading = loading;
        event.own = own;
        // Screenshots of hitches and "stuck", not too many.
        let shot_ok = !loading
            && !own
            && t - self.last_event_shot >= self.event_screenshot_gap
            && self.event_shots < self.max_event_screenshots;
        if let (Some(tag), true) = (shot_tag, shot_ok) {
            event.shot = self.screenshot(tag);
            if event.shot.is_some() {
                self.last_event_shot = t;
                self.event_shots += 1;
                self.shot_frame = true;
            }
        }
        self.write_event(event);
    }

    /// Writes the events of the bear (jump, grab, let go and more).
    fn bear_events(&mut self, t: f64) {
        let Some(mut bear) = self.bear() else {
            return;
        };
        let names = bear.call("take_events", &[]).to::<PackedStringArray>();
        for name in names.as_slice() {
            let name = name.to_string();
            let mut event = self.event_here("bear", name.clone());
            if name == "grab" {
                if let Some(pos) = event.pos {
                    if let Some(found) = self.detectors.grab(t, pos) {
                        let loading = t < self.loading_until;
                        self.found(found, t, loading, false, false);
                    }
                }
            }
            event.t = t;
            self.write_event(event);
        }
    }

    /// Writes one line of `samples.jsonl`.
    fn write_sample(&mut self, t: f64, bear: Option<VarDictionary>) {
        let frames = self.second.take((t - self.second_start) as f32);
        self.second_start = t;
        let own_usec = std::mem::take(&mut self.own_usec);
        let own_frames = std::mem::take(&mut self.own_frames);
        let perf = Performance::singleton();
        let monitor = |m: Monitor| perf.get_monitor(m);
        let (cpu_ms, gpu_ms) = match self.base().get_viewport() {
            Some(viewport) => {
                let rs = RenderingServer::singleton();
                let rid = viewport.get_viewport_rid();
                (
                    rs.viewport_get_measured_render_time_cpu(rid) as f32,
                    rs.viewport_get_measured_render_time_gpu(rid) as f32,
                )
            }
            None => (0.0, 0.0),
        };
        let input = Input::singleton();
        let mut held: Vec<String> = KEYS
            .iter()
            .filter(|(k, _)| input.is_physical_key_pressed(*k))
            .map(|(_, n)| n.to_string())
            .collect();
        held.extend(
            PAD_BUTTONS
                .iter()
                .filter(|(b, _)| input.is_joy_button_pressed(0, *b))
                .map(|(_, n)| n.to_string()),
        );
        let sample = Sample {
            t,
            frames,
            cpu_ms,
            gpu_ms,
            process_ms: (monitor(Monitor::TIME_PROCESS) * 1000.0) as f32,
            physics_ms: (monitor(Monitor::TIME_PHYSICS_PROCESS) * 1000.0) as f32,
            draw_calls: monitor(Monitor::RENDER_TOTAL_DRAW_CALLS_IN_FRAME) as u32,
            primitives: monitor(Monitor::RENDER_TOTAL_PRIMITIVES_IN_FRAME) as u32,
            objects: monitor(Monitor::RENDER_TOTAL_OBJECTS_IN_FRAME) as u32,
            video_mb: (monitor(Monitor::RENDER_VIDEO_MEM_USED) / 1048576.0) as f32,
            static_mb: (monitor(Monitor::MEMORY_STATIC) / 1048576.0) as f32,
            pipeline_compiles: self.pipeline_compiles,
            recorder_ms: own_usec as f32 / 1000.0 / own_frames.max(1) as f32,
            bear: bear.map(|d| self.bear_sample(&d)),
            camera: self.camera_sample(),
            input: InputSample {
                move_stick: stick(&input, JoyAxis::LEFT_X, JoyAxis::LEFT_Y),
                look: stick(&input, JoyAxis::RIGHT_X, JoyAxis::RIGHT_Y),
                held,
            },
        };
        if let (Some(file), Ok(line)) = (&mut self.samples_file, serde_json::to_string(&sample)) {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }

    fn bear_sample(&self, d: &VarDictionary) -> BearSample {
        let get_f = |k: &str| d.get(k).and_then(|v| v.try_to::<f32>().ok()).unwrap_or(0.0);
        let pos = d
            .get("pos")
            .and_then(|v| v.try_to::<Vector3>().ok())
            .unwrap_or_default();
        let mut grip = get_f("grip");
        let mut ground = String::from("none");
        if let Some(mut terrain) = self.terrain() {
            let at = [pos.x.to_variant(), pos.z.to_variant()];
            ground = terrain.call("ground_name_at", &at).to_string();
            if grip < 0.0 {
                grip = terrain.call("grip_at", &at).to::<f32>();
            }
        }
        BearSample {
            pos: [pos.x, pos.y, pos.z],
            speed: get_f("speed"),
            state: d.get("state").map(|v| v.to_string()).unwrap_or_default(),
            slope_deg: get_f("slope_deg"),
            ground,
            grip,
            stamina: get_f("stamina"),
            limbs: d
                .get("limbs")
                .and_then(|v| v.try_to::<i64>().ok())
                .unwrap_or(0) as u32,
            anim_us: get_f("anim_us"),
            gait: d.get("gait").map(|v| v.to_string()).unwrap_or_default(),
        }
    }

    fn camera_sample(&self) -> Option<CameraSample> {
        let camera = self.base().get_viewport()?.get_camera_3d()?;
        let transform = camera.get_global_transform();
        let dir = -transform.basis.col_c();
        let p = transform.origin;
        Some(CameraSample {
            pos: [p.x, p.y, p.z],
            dir: [dir.x, dir.y, dir.z],
            name: camera.get_name().to_string(),
        })
    }

    /// Reads the new lines of the Godot log. Errors and warnings become events.
    fn read_log(&mut self, t: f64) {
        let Some(path) = self.log_path.clone() else {
            return;
        };
        let Ok(mut file) = File::open(&path) else {
            return;
        };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.log_offset {
            self.log_offset = 0;
        }
        if len == self.log_offset || file.seek(SeekFrom::Start(self.log_offset)).is_err() {
            return;
        }
        let mut bytes = Vec::new();
        if file.read_to_end(&mut bytes).is_err() {
            return;
        }
        self.log_offset += bytes.len() as u64;
        let mut text = std::mem::take(&mut self.log_rest);
        text.push_str(&String::from_utf8_lossy(&bytes));
        // Keep a half line for the next time.
        let cut = text.rfind('\n').map_or(0, |i| i + 1);
        self.log_rest = text[cut..].to_string();
        let pairs = self.scrub_pairs();
        for (kind, message) in log_problems(&text[..cut]) {
            if self.log_events >= MAX_LOG_EVENTS {
                break;
            }
            // The recorder's own lines are not problems of the game.
            if message.contains("BfRecorder") {
                continue;
            }
            self.log_events += 1;
            let mut event = self.event_here(kind, scrub(&message, &pairs));
            event.t = t;
            self.write_event(event);
        }
    }

    fn save_histogram(&mut self) {
        self.histogram.pack();
        if let Ok(text) = serde_json::to_string(&self.histogram) {
            let _ = fs::write(self.dir.join("frametimes.json"), text);
        }
    }

    fn flush(&mut self) {
        let t = self.now();
        self.read_log(t);
        self.save_histogram();
        self.write_session();
        for file in [&mut self.samples_file, &mut self.events_file]
            .into_iter()
            .flatten()
        {
            let _ = file.flush();
        }
    }

    /// Ends the session: the report, the zip, and the cleanup.
    fn finish(&mut self, normal: bool) {
        if !self.active || self.finished {
            return;
        }
        self.finished = true;
        let t = self.now();
        let event = self.event_here("recorder", format!("recording stopped after {t:.0} s"));
        self.write_event(event);
        self.flush();
        self.samples_file = None;
        self.events_file = None;
        let log = self.log_path.clone();
        if let Some(zip) = pack_session(&self.dir, &self.root, normal, &self.scrub_pairs(), log) {
            let _ = fs::remove_dir_all(&self.dir);
            godot_print!(
                "BfRecorder: the playtest is in playtests/{}",
                file_name(&zip)
            );
        }
        self.cleanup();
        self.active = false;
    }

    /// Deletes the oldest zips if the folder is too large.
    fn cleanup(&self) {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        let mut zips: Vec<(String, u64)> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_zip = name.starts_with("playtest-") && name.ends_with(".zip");
                is_zip.then(|| (name, e.metadata().map(|m| m.len()).unwrap_or(0)))
            })
            .collect();
        // The names start with the date, so the name order is the time order.
        zips.sort();
        let limit = self.folder_limit_mb.max(1) as u64 * 1024 * 1024;
        for name in files_to_delete(&zips, limit) {
            let _ = fs::remove_file(self.root.join(name));
        }
    }

    /// Makes an event at the place of the bear.
    fn event_here(&self, kind: &str, info: String) -> Event {
        let pos = self.bear().map(|b| {
            let p = b.cast::<Node3D>().get_global_position();
            [p.x, p.y, p.z]
        });
        Event {
            t: self.now(),
            kind: kind.into(),
            info,
            pos,
            ..Default::default()
        }
    }

    fn write_event(&mut self, event: Event) {
        if let (Some(file), Ok(line)) = (&mut self.events_file, serde_json::to_string(&event)) {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }

    fn write_session(&self) {
        if let Ok(text) = serde_json::to_string_pretty(&self.session) {
            let _ = fs::write(self.dir.join("session.json"), text);
        }
    }

    /// Saves a screenshot in `shots/`. Gives its file name.
    fn screenshot(&mut self, tag: &str) -> Option<String> {
        if self.headless {
            return None;
        }
        let texture = self.base().get_viewport()?.get_texture()?;
        let mut image = texture.get_image()?;
        if image.is_empty() {
            return None;
        }
        image
            .resize_ex(
                self.screenshot_width.max(16),
                self.screenshot_height.max(16),
            )
            .interpolation(Interpolation::BILINEAR)
            .done();
        let bytes = image
            .save_jpg_to_buffer_ex()
            .quality(self.jpeg_quality.clamp(0.1, 1.0))
            .done();
        self.shot_number += 1;
        let t = self.now();
        let minutes = (t / 60.0) as u32;
        let seconds = (t % 60.0) as u32;
        let name = format!(
            "{:04}-{minutes:02}m{seconds:02}s-{tag}.jpg",
            self.shot_number
        );
        fs::write(self.dir.join("shots").join(&name), bytes.as_slice()).ok()?;
        Some(name)
    }

    fn make_note_label(&mut self) {
        if self.headless {
            return;
        }
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(110);
        let mut label = Label::new_alloc();
        label.add_theme_font_size_override("font_size", 28);
        label.add_theme_color_override("font_color", Color::from_rgb(1.0, 1.0, 0.85));
        label.add_theme_color_override("font_outline_color", Color::from_rgb(0.0, 0.0, 0.0));
        label.add_theme_constant_override("outline_size", 8);
        label.set_position(Vector2::new(40.0, 160.0));
        label.set_visible(false);
        layer.add_child(&label);
        self.base_mut().add_child(&layer);
        self.note = Some(label);
    }

    fn show_note(&mut self, text: &str, t: f64) {
        self.note_until = t + 2.5;
        if let Some(note) = &mut self.note {
            note.set_text(text);
            note.set_visible(true);
        }
    }

    fn bear(&self) -> Option<Gd<Node>> {
        if !self.base().is_inside_tree() {
            return None;
        }
        self.base().get_tree().get_first_node_in_group(BEAR_GROUP)
    }

    fn terrain(&self) -> Option<Gd<Node>> {
        if !self.base().is_inside_tree() {
            return None;
        }
        self.base()
            .get_tree()
            .get_first_node_in_group(TERRAIN_GROUP)
    }

    fn bear_state(&self) -> Option<VarDictionary> {
        let mut bear = self.bear()?;
        bear.call("recorder_state", &[])
            .try_to::<VarDictionary>()
            .ok()
    }

    /// The paths that must not leave the computer, and their replacements.
    fn scrub_pairs(&self) -> Vec<(String, String)> {
        scrub_pairs(&self.root)
    }
}

fn stick(input: &Gd<Input>, x: JoyAxis, y: JoyAxis) -> [f32; 2] {
    let round = |v: f32| (v * 100.0).round() / 100.0;
    [
        round(input.get_joy_axis(0, x)),
        round(input.get_joy_axis(0, y)),
    ]
}

fn pipeline_compiles() -> u32 {
    let perf = Performance::singleton();
    [
        Monitor::PIPELINE_COMPILATIONS_CANVAS,
        Monitor::PIPELINE_COMPILATIONS_MESH,
        Monitor::PIPELINE_COMPILATIONS_SURFACE,
        Monitor::PIPELINE_COMPILATIONS_DRAW,
        Monitor::PIPELINE_COMPILATIONS_SPECIALIZATION,
    ]
    .into_iter()
    .map(|m| perf.get_monitor(m) as u32)
    .sum()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// The folder of the game program, if the game is an export.
fn game_folder() -> Option<PathBuf> {
    let os = Os::singleton();
    if os.has_feature("editor") {
        return None;
    }
    let exe = PathBuf::from(os.get_executable_path().to_string());
    exe.parent().map(Path::to_path_buf)
}

fn user_folder() -> PathBuf {
    PathBuf::from(Os::singleton().get_user_data_dir().to_string())
}

/// Gives the playtests folder: next to the game if it can be written,
/// else in the Godot user folder. In the editor, the user folder.
fn playtests_folder() -> Option<PathBuf> {
    let can_write = |dir: &Path| {
        if fs::create_dir_all(dir).is_err() {
            return false;
        }
        let probe = dir.join(".bf-write-test");
        let ok = fs::write(&probe, b"ok").is_ok();
        let _ = fs::remove_file(probe);
        ok
    };
    if let Some(game) = game_folder() {
        let dir = game.join("playtests");
        if can_write(&dir) {
            return Some(dir);
        }
    }
    let dir = user_folder().join("playtests");
    can_write(&dir).then_some(dir)
}

/// The path of the current Godot log file, if file logging is on.
fn godot_log_path() -> Option<PathBuf> {
    let settings = ProjectSettings::singleton();
    let on = settings
        .get_setting("debug/file_logging/enable_file_logging.pc")
        .try_to::<bool>()
        .unwrap_or(true);
    if !on {
        return None;
    }
    let path = settings
        .get_setting("debug/file_logging/log_path")
        .try_to::<GString>()
        .unwrap_or_else(|_| GString::from("user://logs/godot.log"));
    let path = settings.globalize_path(&path).to_string();
    Some(PathBuf::from(path))
}

/// The newest rotated Godot log (the log of the last run), if any.
fn previous_godot_log() -> Option<PathBuf> {
    let current = godot_log_path()?;
    let folder = current.parent()?;
    let mut logs: Vec<PathBuf> = fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = file_name(p);
            name.starts_with("godot") && name.ends_with(".log") && *p != current
        })
        .collect();
    logs.sort_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
    logs.pop()
}

fn scrub_pairs(root: &Path) -> Vec<(String, String)> {
    let os = Os::singleton();
    let mut pairs = Vec::new();
    if let Some(game) = game_folder() {
        pairs.push((game.to_string_lossy().to_string(), "<game>".to_string()));
    }
    pairs.push((
        root.to_string_lossy().to_string(),
        "<playtests>".to_string(),
    ));
    pairs.push((
        user_folder().to_string_lossy().to_string(),
        "<user data>".to_string(),
    ));
    for var in ["HOME", "USERPROFILE"] {
        let home = os.get_environment(var).to_string();
        if !home.is_empty() {
            pairs.push((home, "~".to_string()));
        }
    }
    for var in ["USER", "USERNAME"] {
        let user = os.get_environment(var).to_string();
        // A very short name could match normal words.
        if user.len() >= 3 {
            pairs.push((user, "<user>".to_string()));
        }
    }
    // On Windows, the log can use "/" or "\\" in the same path.
    let other: Vec<(String, String)> = pairs
        .iter()
        .filter(|(from, _)| from.contains('\\'))
        .map(|(from, to)| (from.replace('\\', "/"), to.clone()))
        .collect();
    pairs.extend(other);
    pairs
}

/// Packs the session folders that have no finished zip (after a crash).
fn finish_old_sessions(root: &Path, pairs: &[(String, String)]) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = file_name(&path);
        if !path.is_dir() || !name.starts_with("playtest-") {
            continue;
        }
        if !path.join("session.json").exists() {
            continue;
        }
        let log = previous_godot_log();
        if pack_session(&path, root, false, pairs, log).is_some() {
            let _ = fs::remove_dir_all(&path);
            godot_print!("BfRecorder: packed an old session ({name}.zip)");
        }
    }
}

/// Writes `report.md` into a session folder and packs the folder and the
/// Godot log into `<root>/<folder name>.zip`. Gives the path of the zip.
fn pack_session(
    dir: &Path,
    root: &Path,
    normal: bool,
    pairs: &[(String, String)],
    log: Option<PathBuf>,
) -> Option<PathBuf> {
    let read = |name: &str| fs::read_to_string(dir.join(name)).unwrap_or_default();
    let session: Value = serde_json::from_str(&read("session.json")).unwrap_or(json!({}));
    let samples: Vec<Sample> = parse_lines(&read("samples.jsonl"));
    let events: Vec<Event> = parse_lines(&read("events.jsonl"));
    let histogram: Option<FrameHistogram> = serde_json::from_str(&read("frametimes.json")).ok();
    let report = make_report(&ReportInput {
        session: &session,
        samples: &samples,
        events: &events,
        histogram: histogram.as_ref(),
        quit_normally: normal,
    });
    fs::write(dir.join("report.md"), report).ok()?;

    let zip_path = root.join(format!("{}.zip", file_name(dir)));
    let mut zip = ZipPacker::new_gd();
    // Godot paths use "/" also on Windows.
    let zip_text = zip_path.to_string_lossy().replace('\\', "/");
    if zip.open(&zip_text) != godot::global::Error::OK {
        godot_warn!("BfRecorder: cannot write the playtest zip.");
        return None;
    }
    let mut add = |name: &str, bytes: &[u8]| {
        zip.start_file(name);
        zip.write_file(&PackedByteArray::from(bytes));
        zip.close_file();
    };
    // report.md first: it is the file to read first.
    let mut files = Vec::new();
    collect_files(dir, dir, &mut files);
    files.sort_by_key(|(name, _)| (name != "report.md", name.clone()));
    for (name, path) in files {
        if let Ok(bytes) = fs::read(&path) {
            add(&name, &bytes);
        }
    }
    let log = log.or_else(godot_log_path);
    if let Some(text) = log.and_then(|p| fs::read(p).ok()) {
        let text = scrub(&String::from_utf8_lossy(&text), pairs);
        add("godot.log", text.as_bytes());
    }
    zip.close();
    Some(zip_path)
}

/// Lists all files below `dir` as (name in the zip, path).
fn collect_files(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, out);
        } else if let Ok(relative) = path.strip_prefix(base) {
            let name = relative.to_string_lossy().replace('\\', "/");
            out.push((name, path));
        }
    }
}

/// Makes the content of `session.json`: build, hardware and settings.
fn session_info(started: &str, headless: bool) -> Value {
    let os = Os::singleton();
    let rs = RenderingServer::singleton();
    let ds = DisplayServer::singleton();
    let settings = ProjectSettings::singleton();
    let setting = |name: &str| settings.get_setting(name).to_string();
    let build = game_folder()
        .and_then(|dir| fs::read_to_string(dir.join("BUILD.txt")).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "(no BUILD.txt: an editor or local build)".into());
    let memory = os.get_memory_info();
    let ram_gb = memory
        .get("physical")
        .and_then(|v| v.try_to::<i64>().ok())
        .map(|b| (b as f64 / 1073741824.0 * 10.0).round() / 10.0);
    let version = Engine::singleton()
        .get_version_info()
        .get("string")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let screen = ds.screen_get_size();
    let window = ds.window_get_size();
    let mut hardware = json!({
        "cpu": os.get_processor_name().to_string(),
        "cpu_threads": os.get_processor_count(),
        "ram_gb": ram_gb,
        "gpu": rs.get_video_adapter_name().to_string(),
        "gpu_vendor": rs.get_video_adapter_vendor().to_string(),
        "gpu_type": format!("{:?}", rs.get_video_adapter_type()),
        "graphics_api": format!(
            "{} {}",
            rs.get_current_rendering_driver_name(),
            rs.get_video_adapter_api_version()
        ),
        "driver": os
            .get_video_adapter_driver_info()
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(" "),
        "os": format!("{} {} {}", os.get_name(), os.get_distribution_name(), os.get_version()),
        "screen": format!("{} x {} at {:.0} Hz", screen.x, screen.y, ds.screen_get_refresh_rate()),
        "window": format!("{} x {}", window.x, window.y),
    });
    if headless {
        hardware["note"] = json!("headless run: no GPU data");
    }
    let engine = Engine::singleton();
    let settings_json = json!({
        "godot": version,
        "renderer": rs.get_current_rendering_method().to_string(),
        "vsync": format!("{:?}", ds.window_get_vsync_mode()),
        "window_mode": format!("{:?}", ds.window_get_mode()),
        "max_fps": engine.get_max_fps(),
        "physics_ticks": engine.get_physics_ticks_per_second(),
        "scaling_3d_mode": setting("rendering/scaling_3d/mode"),
        "scaling_3d_scale": setting("rendering/scaling_3d/scale"),
        "msaa_3d": setting("rendering/anti_aliasing/quality/msaa_3d"),
        "screen_space_aa": setting("rendering/anti_aliasing/quality/screen_space_aa"),
        "shadow_atlas_size": setting("rendering/lights_and_shadows/directional_shadow/size"),
        "soft_shadow_quality": setting("rendering/lights_and_shadows/directional_shadow/soft_shadow_filter_quality"),
        "debug_build": os.is_debug_build(),
    });
    json!({
        "build": build,
        "started": started,
        "hardware": hardware,
        "settings": settings_json,
        "zones": [],
    })
}
