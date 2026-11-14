//! The autoplay bot (S-133, `plan/diagnostics.md`, "Autoplay").
//!
//! The bot plays the bear through virtual gamepad input: it sends
//! joypad events to Godot's `Input`, as a real gamepad does. The bear
//! reads them in `BfBear::read_input`, so the bot tests the real
//! controls. Only the teleport at the start of a task and the limb
//! changes use the dev tools. The tasks, the search and the report are
//! in `crate::playtest::autoplay`, with unit tests.
//!
//! - **Tour:** fixed tasks for each seed (a far walk, a climb, a ledge
//!   jump, one obstacle of each type, the smoke edge, each limb set).
//! - **Wander:** seeded random input at random places.
//!
//! The dev menu starts the tour ("Tests" page). The command line
//! `-- --bf-autoplay=tour` (or `wander`) starts it when the first zone
//! shows and quits at the end, with the exit code 1 if the bear fell
//! through the ground or a zone did not load.

use crate::bear::{BfBear, BEAR_GROUP};
use crate::follow_camera::FOLLOW_CAMERA_GROUP;
use crate::obstacles::OBSTACLES_GROUP;
use crate::playtest::autoplay::{
    build, smoke_angles, stick_for, tour_tasks, wander_tasks, AutoReport, BearState, Ctx, Mode,
    ObstacleInfo, Pad, Request, SeedResult, TaskKind, TaskRun, View, TOUR_SEEDS,
};
use crate::playtest::detect::{Detectors, Found, FrameInput};
use crate::playtest::menu::LIMB_NAMES;
use crate::recorder::RECORDER_GROUP;
use crate::smoke_edge::SMOKE_GROUP;
use crate::terrain::{BfTerrain, TERRAIN_GROUP};
use godot::classes::{
    Camera3D, CanvasLayer, DisplayServer, INode, Input, InputEvent, InputEventJoypadButton,
    InputEventJoypadMotion, InputEventKey, Label, Os, Time,
};
use godot::global::{JoyAxis, JoyButton, Key};
use godot::prelude::*;

/// The group that holds the autoplay bot.
pub const AUTOPLAY_GROUP: &str = "bf_autoplay";

/// What the bot does now.
enum Phase {
    /// Wait until the zone of the seed shows.
    Zone { requested: bool, waited: f32 },
    /// The zone shows. Wait, so the collision and the load hitches pass.
    Settle { left: f32 },
    /// Run the tasks of the seed.
    Tasks,
}

/// A run of the bot.
struct Active {
    mode: Mode,
    seeds: Vec<i64>,
    seed_index: usize,
    minutes: f32,
    phase: Phase,
    tasks: Vec<TaskKind>,
    task_index: usize,
    run: Option<TaskRun>,
    seed: SeedResult,
    results: Vec<SeedResult>,
    detectors: Detectors,
    /// The game time of this seed, in seconds.
    seed_t: f64,
    /// The time after a teleport, in which a bear below the ground
    /// does not fail the build.
    grace_left: f32,
    /// The pad that Godot has now (to send only changes).
    sent: Pad,
    sent_stick: [f32; 2],
    start_usec: u64,
    saved_seed: i64,
    quit_at_end: bool,
    out_path: Option<String>,
}

/// The autoplay bot. It plays the bear through the gamepad.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct BfAutoplay {
    /// The time to wait after a zone shows, before the first task, in seconds.
    #[export]
    #[init(val = 2.0)]
    settle_seconds: f32,

    /// After a teleport or a restart, a bear below the ground does not
    /// fail the run for this time, in seconds. Terrain3D makes the
    /// collision only near the camera, so it can be late there.
    #[export]
    #[init(val = 2.0)]
    teleport_grace_seconds: f32,

    /// The longest wait for a zone, in seconds of game time.
    #[export]
    #[init(val = 120.0)]
    zone_timeout_seconds: f32,

    /// The minutes of a wander run on each seed (the command line
    /// `--bf-autoplay-minutes=` changes it).
    #[export]
    #[init(val = 3.0)]
    wander_minutes: f32,

    active: Option<Active>,
    /// A run from the command line. It starts when the first zone shows.
    pending: Option<(Mode, Vec<i64>, f32, Option<String>)>,
    label: Option<Gd<Label>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for BfAutoplay {
    fn ready(&mut self) {
        self.base_mut().add_to_group(AUTOPLAY_GROUP);
        self.read_command_line();
        let mut layer = CanvasLayer::new_alloc();
        layer.set_layer(104);
        let mut label = Label::new_alloc();
        // Below the lines of the F3 overlay.
        label.set_position(Vector2::new(12.0, 230.0));
        label.add_theme_font_size_override("font_size", 20);
        label.add_theme_color_override("font_color", Color::from_rgb(0.8, 1.0, 0.9));
        label.add_theme_color_override("font_outline_color", Color::from_rgb(0.0, 0.0, 0.0));
        label.add_theme_constant_override("outline_size", 6);
        label.set_visible(false);
        layer.add_child(&label);
        self.base_mut().add_child(&layer);
        self.label = Some(label);
    }

    fn physics_process(&mut self, delta: f64) {
        if self.active.is_none() {
            self.start_pending();
            return;
        }
        self.step(delta as f32);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if self.active.is_none() || crate::dev_menu::is_open() {
            return;
        }
        // Esc or the Start button stops the bot. B is a button of the
        // bot (let go), so it cannot stop it.
        let stop = if let Ok(button) = event.clone().try_cast::<InputEventJoypadButton>() {
            button.is_pressed() && button.get_button_index() == JoyButton::START
        } else if let Ok(key) = event.try_cast::<InputEventKey>() {
            key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::ESCAPE
        } else {
            false
        };
        if stop {
            self.finish(true);
        }
    }

    fn exit_tree(&mut self) {
        self.finish(true);
    }
}

#[godot_api]
impl BfAutoplay {
    /// Starts the bot: "tour" on seeds 1, 7 and 42, or "wander" on the
    /// seed of the zone now. Gives true if it started.
    #[func]
    pub fn start_autoplay(&mut self, mode: GString) -> bool {
        let Some(mode) = Mode::from_name(&mode.to_string()) else {
            godot_error!("BfAutoplay: unknown mode '{mode}'");
            return false;
        };
        let seeds = match mode {
            Mode::Tour => TOUR_SEEDS.to_vec(),
            Mode::Wander => {
                let seed = self
                    .node(TERRAIN_GROUP)
                    .map(|mut t| t.call("current_seed", &[]).to::<i64>())
                    .unwrap_or(1);
                vec![seed]
            }
        };
        let minutes = self.wander_minutes;
        self.start(mode, seeds, minutes, None, false)
    }

    /// Stops the bot. The result so far goes into the report.
    #[func]
    pub fn stop_autoplay(&mut self) {
        self.finish(true);
    }

    /// Tells if the bot runs now.
    #[func]
    pub fn is_running(&self) -> bool {
        self.active.is_some()
    }
}

impl BfAutoplay {
    fn node(&self, group: &str) -> Option<Gd<Node>> {
        if !self.base().is_inside_tree() {
            return None;
        }
        self.base().get_tree().get_first_node_in_group(group)
    }

    fn bear(&self) -> Option<Gd<BfBear>> {
        self.node(BEAR_GROUP)?.try_cast::<BfBear>().ok()
    }

    fn terrain(&self) -> Option<Gd<BfTerrain>> {
        self.node(TERRAIN_GROUP)?.try_cast::<BfTerrain>().ok()
    }

    /// Reads `--bf-autoplay=tour`, `--bf-autoplay-seeds=1,7,42`,
    /// `--bf-autoplay-minutes=3` and `--bf-autoplay-out=<file.md>`.
    fn read_command_line(&mut self) {
        let mut mode = None;
        let mut seeds = None;
        let mut minutes = self.wander_minutes;
        let mut out = None;
        for arg in Os::singleton().get_cmdline_user_args().as_slice() {
            let arg = arg.to_string();
            if let Some(name) = arg.strip_prefix("--bf-autoplay=") {
                match Mode::from_name(name) {
                    Some(m) => mode = Some(m),
                    None => godot_error!("BfAutoplay: unknown mode '{name}'"),
                }
            } else if arg == "--bf-autoplay" {
                mode = Some(Mode::Tour);
            } else if let Some(list) = arg.strip_prefix("--bf-autoplay-seeds=") {
                let parsed: Vec<i64> = list
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if !parsed.is_empty() {
                    seeds = Some(parsed);
                }
            } else if let Some(m) = arg.strip_prefix("--bf-autoplay-minutes=") {
                if let Ok(m) = m.parse::<f32>() {
                    minutes = m.max(0.1);
                }
            } else if let Some(path) = arg.strip_prefix("--bf-autoplay-out=") {
                out = Some(path.to_string());
            }
        }
        if let Some(mode) = mode {
            let seeds = seeds.unwrap_or_else(|| TOUR_SEEDS.to_vec());
            self.pending = Some((mode, seeds, minutes, out));
        }
    }

    /// Starts the run from the command line when the first zone shows.
    fn start_pending(&mut self) {
        if self.pending.is_none() {
            return;
        }
        let Some(mut terrain) = self.node(TERRAIN_GROUP) else {
            return;
        };
        let ready = !terrain.call("is_generating", &[]).booleanize()
            && terrain.call("generation_seconds", &[]).to::<f32>() > 0.0;
        if ready {
            if let Some((mode, seeds, minutes, out)) = self.pending.take() {
                self.start(mode, seeds, minutes, out, true);
            }
        }
    }

    fn start(
        &mut self,
        mode: Mode,
        seeds: Vec<i64>,
        minutes: f32,
        out_path: Option<String>,
        quit_at_end: bool,
    ) -> bool {
        if self.active.is_some() || seeds.is_empty() || !self.base().is_inside_tree() {
            return false;
        }
        let (Some(mut terrain), Some(mut bear)) = (self.node(TERRAIN_GROUP), self.bear()) else {
            godot_error!("BfAutoplay: there is no terrain or no bear.");
            return false;
        };
        // The bear reads the gamepad only through the follow camera.
        if let Some(camera) = self.node(FOLLOW_CAMERA_GROUP) {
            if let Ok(mut camera) = camera.try_cast::<Camera3D>() {
                camera.make_current();
            }
        }
        bear.bind_mut().set_event_tap(true);
        let saved_seed = terrain.call("current_seed", &[]).to::<i64>();
        godot_print!(
            "BfAutoplay: start {} on seeds {:?}{}",
            mode.name(),
            seeds,
            if mode == Mode::Wander {
                format!(", {minutes} min each")
            } else {
                String::new()
            }
        );
        self.active = Some(Active {
            mode,
            seeds,
            seed_index: 0,
            minutes,
            phase: Phase::Zone {
                requested: false,
                waited: 0.0,
            },
            tasks: Vec::new(),
            task_index: 0,
            run: None,
            seed: SeedResult::default(),
            results: Vec::new(),
            detectors: Detectors::default(),
            seed_t: 0.0,
            grace_left: 0.0,
            sent: Pad::default(),
            sent_stick: [0.0, 0.0],
            start_usec: Time::singleton().get_ticks_usec(),
            saved_seed,
            quit_at_end,
            out_path,
        });
        self.begin_seed();
        true
    }

    /// Starts the work on the seed at `seed_index`.
    fn begin_seed(&mut self) {
        let Some(a) = &mut self.active else {
            return;
        };
        let seed = a.seeds[a.seed_index];
        a.seed = SeedResult {
            seed,
            ..Default::default()
        };
        a.tasks = match a.mode {
            Mode::Tour => tour_tasks(),
            Mode::Wander => wander_tasks(a.minutes),
        };
        a.task_index = 0;
        a.run = None;
        a.detectors = Detectors::default();
        a.seed_t = 0.0;
        a.phase = Phase::Zone {
            requested: false,
            waited: 0.0,
        };
    }

    /// The work of one physics step.
    fn step(&mut self, dt: f32) {
        let Some(mut terrain) = self.node(TERRAIN_GROUP) else {
            self.finish(true);
            return;
        };
        let generating = terrain.call("is_generating", &[]).booleanize();
        let current_seed = terrain.call("current_seed", &[]).to::<i64>();
        let zone_timeout = self.zone_timeout_seconds;
        let settle = self.settle_seconds.max(0.0);

        enum Next {
            Nothing,
            MakeZone(i64),
            Tasks,
            SeedDone,
        }
        let mut next = Next::Nothing;
        let text;
        {
            let Some(a) = &mut self.active else {
                return;
            };
            let seed = a.seeds[a.seed_index];
            let title = a.mode.title();
            let of = format!("seed {} ({}/{})", seed, a.seed_index + 1, a.seeds.len());
            match &mut a.phase {
                Phase::Zone { requested, waited } => {
                    *waited += dt;
                    if !generating && current_seed == seed && (*requested || *waited > 0.2) {
                        a.phase = Phase::Settle { left: settle };
                    } else if !generating && !*requested {
                        *requested = true;
                        next = Next::MakeZone(seed);
                    } else if *waited > zone_timeout {
                        godot_warn!("BfAutoplay: the zone of seed {seed} did not load in time.");
                        next = Next::SeedDone;
                    }
                    text = format!("{title}: {of}: the zone is in work ...");
                }
                Phase::Settle { left } => {
                    *left -= dt;
                    if *left <= 0.0 {
                        a.seed.ran = true;
                        a.phase = Phase::Tasks;
                    }
                    text = format!("{title}: {of}: wait ...");
                }
                Phase::Tasks => {
                    next = Next::Tasks;
                    let task = a.run.as_ref().map_or("next", |r| r.name());
                    let leg = a.run.as_ref().map_or("", |r| r.leg_name());
                    text = format!(
                        "{title}: {of}, task {} ({}/{}), {leg}. Esc or Start stops.",
                        task,
                        (a.task_index + 1).min(a.tasks.len()),
                        a.tasks.len()
                    );
                }
            }
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
            Next::Tasks => self.task_step(dt),
            Next::SeedDone => self.seed_done(),
        }
    }

    /// One step of the tasks: look, count, act.
    fn task_step(&mut self, dt: f32) {
        let (Some(mut bear), Some(terrain)) = (self.bear(), self.terrain()) else {
            self.finish(true);
            return;
        };
        // Start the next task if there is none.
        let need_task = self
            .active
            .as_ref()
            .is_some_and(|a| a.run.as_ref().is_none_or(|r| r.is_done()));
        if need_task && !self.next_task() {
            self.seed_done();
            return;
        }
        // The task can move the bear (an obstacle), so look after it.
        let grace = self.teleport_grace_seconds.max(0.0);
        let events = bear.bind_mut().take_tap_events();
        let state_dict = bear.bind().recorder_state();
        let pos = bear.get_global_position();
        let state_name = state_dict
            .get("state")
            .map(|v| v.to_string())
            .unwrap_or_default();
        let state = BearState::from_name(&state_name);
        let stick_held = state_dict
            .get("stick_held")
            .and_then(|v| v.try_to::<bool>().ok())
            .unwrap_or(false);
        let ground = terrain.bind().height_at(pos.x, pos.z);

        let Some(a) = &mut self.active else {
            return;
        };
        a.seed_t += f64::from(dt);
        a.grace_left = (a.grace_left - dt).max(0.0);
        for e in &events {
            if e == "restart" || e.starts_with("dev:go_to") {
                a.grace_left = grace;
            }
            let in_grace = a.grace_left > 0.0;
            a.seed.counts.bear_event(e, in_grace);
            if e.starts_with("back_on_ground") {
                godot_print!(
                    "BfAutoplay: seed {} {e}{} at ({:.1}, {:.1}, {:.1}), task {} ({})",
                    a.seed.seed,
                    if in_grace { " after a teleport" } else { "" },
                    pos.x,
                    pos.y,
                    pos.z,
                    a.run.as_ref().map_or("?", |r| r.name()),
                    a.run.as_ref().map_or("", |r| r.leg_name())
                );
            }
            if e == "grab" {
                if let Some(Found::ClimbLoop { .. }) =
                    a.detectors.grab(a.seed_t, [pos.x, pos.y, pos.z])
                {
                    a.seed.counts.climb_loops += 1;
                }
            }
        }
        let on_wall = matches!(state, BearState::Climb | BearState::Mantle);
        let found = a.detectors.frame(FrameInput {
            t: a.seed_t,
            // Not a real frame time: the bot does not look for hitches.
            frame_ms: 1000.0 / 60.0,
            bear: Some([pos.x, pos.y, pos.z]),
            stick_held,
            ground: (!on_wall).then_some(ground),
        });
        for f in found {
            match f {
                Found::Stuck => a.seed.counts.stuck += 1,
                Found::ThroughGround { depth } => {
                    if a.grace_left > 0.0 {
                        a.seed.counts.after_teleport += 1;
                    } else {
                        a.seed.counts.through_ground += 1;
                        a.seed.counts.deepest_m = a.seed.counts.deepest_m.max(depth);
                        godot_print!(
                            "BfAutoplay: seed {} below the ground by {depth:.2} m at ({:.1}, {:.1}, {:.1}), task {}",
                            a.seed.seed,
                            pos.x,
                            pos.y,
                            pos.z,
                            a.run.as_ref().map_or("?", |r| r.name())
                        );
                    }
                }
                _ => {}
            }
        }

        let Some(run) = &mut a.run else {
            return;
        };
        let view = View {
            dt,
            pos: [pos.x, pos.y, pos.z],
            state,
            events: &events,
        };
        let (pad, request) = run.step(&view);
        match request {
            Some(Request::Teleport { x, z }) => {
                let h = terrain.bind().height_at(x, z);
                a.grace_left = grace;
                bear.bind_mut().go_to(Vector3::new(x, h + 0.3, z));
            }
            Some(Request::Limbs(set)) => {
                for (name, present) in LIMB_NAMES.iter().zip(set) {
                    bear.bind_mut().set_limb(GString::from(*name), present);
                }
            }
            None => {}
        }
        self.send_pad(pad);
    }

    /// Builds the next task of the seed. Gives false if there is none.
    fn next_task(&mut self) -> bool {
        // Keep the result of the task that ended.
        if let Some(a) = &mut self.active {
            if let Some(run) = a.run.take() {
                a.seed.tasks.push(run.result());
                a.task_index += 1;
            }
            if a.task_index >= a.tasks.len() {
                return false;
            }
        }
        let Some(kind) = self.active.as_ref().map(|a| a.tasks[a.task_index]) else {
            return false;
        };
        let seed = self.active.as_ref().map_or(0, |a| a.seeds[a.seed_index]);
        let (Some(bear), Some(terrain)) = (self.bear(), self.terrain()) else {
            return false;
        };

        // The parts that need Godot: the obstacle and the smoke wall.
        let mut obstacle = None;
        if let TaskKind::Obstacle(_) = kind {
            if let Some(mut node) = self.node(OBSTACLES_GROUP) {
                let found = node.call("go_to_next", &[]).to::<VarDictionary>();
                let kind = found.get("kind").map(|v| v.to_string());
                let pos = found.get("pos").and_then(|v| v.try_to::<Vector3>().ok());
                let dir = found
                    .get("dir")
                    .and_then(|v| v.try_to::<Vector3>().ok())
                    .unwrap_or(Vector3::ZERO);
                if let (Some(kind), Some(pos)) = (kind, pos) {
                    obstacle = Some(ObstacleInfo {
                        kind,
                        pos: [pos.x, pos.y, pos.z],
                        dir: [dir.x, dir.z],
                    });
                    if let Some(a) = &mut self.active {
                        a.grace_left = self.teleport_grace_seconds.max(0.0);
                    }
                }
            }
        }
        let mut smoke_fronts = Vec::new();
        if kind == TaskKind::Smoke {
            if let Some(mut smoke) = self.node(SMOKE_GROUP) {
                for a in smoke_angles(seed) {
                    let front = smoke.call("front_m_at", &[a.to_variant()]).to::<f32>();
                    smoke_fronts.push((a, front));
                }
            }
        }
        let pos = bear.get_global_position();
        let task = {
            let t = terrain.bind();
            let Some(zone) = t.zone() else {
                return false;
            };
            let start = t.start_point();
            let ctx = Ctx {
                heights: &zone.heights,
                seed,
                start: [start.x, start.y],
                bear: [pos.x, pos.y, pos.z],
                obstacle: obstacle.as_ref(),
                smoke_fronts: &smoke_fronts,
            };
            build(kind, &ctx)
        };
        godot_print!(
            "BfAutoplay: seed {seed}, task {} ({})",
            task.name,
            task.note
        );
        if let Some(mut b) = self.bear() {
            b.bind_mut()
                .report_event(GString::from(&format!("autoplay:{}", task.name)));
        }
        if let Some(a) = &mut self.active {
            a.run = Some(TaskRun::new(task));
        }
        true
    }

    /// Ends the seed: keeps its result and starts the next seed or ends the run.
    fn seed_done(&mut self) {
        self.send_pad(Pad::default());
        if let Some(mut bear) = self.bear() {
            bear.bind_mut().restore_limbs();
        }
        let Some(a) = &mut self.active else {
            return;
        };
        if let Some(run) = a.run.take() {
            a.seed.tasks.push(run.result());
        }
        a.seed.seconds = a.seed_t as f32;
        let done = std::mem::take(&mut a.seed);
        let c = &done.counts;
        godot_print!(
            "BfAutoplay: seed {} done in {:.0} s: {} of {} tasks reached, below ground {}, put back {}, small put-back {}, after teleport {}, stuck {}, climb loops {}",
            done.seed,
            done.seconds,
            done.tasks.iter().filter(|t| t.reached).count(),
            done.tasks.len(),
            c.through_ground,
            c.back_on_ground,
            c.small_put_back,
            c.after_teleport,
            c.stuck,
            c.climb_loops
        );
        a.results.push(done);
        a.seed_index += 1;
        if a.seed_index >= a.seeds.len() {
            self.finish(false);
        } else {
            self.begin_seed();
        }
    }

    /// Sends the pad to Godot as joypad events (device 0). Only changes go.
    fn send_pad(&mut self, pad: Pad) {
        let yaw = self
            .node(FOLLOW_CAMERA_GROUP)
            .and_then(|n| n.try_cast::<Node3D>().ok())
            .map_or(0.0, |c| c.get_global_rotation().y);
        let stick = stick_for(pad.dir, yaw);
        let Some(a) = &mut self.active else {
            return;
        };
        let mut input = Input::singleton();
        for (axis, value, old) in [
            (JoyAxis::LEFT_X, stick[0], a.sent_stick[0]),
            (JoyAxis::LEFT_Y, stick[1], a.sent_stick[1]),
        ] {
            if (value - old).abs() > 1e-4 {
                let mut e = InputEventJoypadMotion::new_gd();
                e.set_device(0);
                e.set_axis(axis);
                e.set_axis_value(value);
                input.parse_input_event(&e);
            }
        }
        for (button, down, old) in [
            (JoyButton::A, pad.jump, a.sent.jump),
            (JoyButton::B, pad.let_go, a.sent.let_go),
            (JoyButton::RIGHT_SHOULDER, pad.jog, a.sent.jog),
        ] {
            if down != old {
                let mut e = InputEventJoypadButton::new_gd();
                e.set_device(0);
                e.set_button_index(button);
                e.set_pressed(down);
                input.parse_input_event(&e);
            }
        }
        a.sent = pad;
        a.sent_stick = stick;
    }

    /// Ends the run: writes the report and puts back the game state.
    fn finish(&mut self, cancelled: bool) {
        if self.active.is_none() {
            return;
        }
        self.send_pad(Pad::default());
        let Some(mut a) = self.active.take() else {
            return;
        };
        if cancelled && a.seed_index < a.seeds.len() {
            if let Some(run) = a.run.take() {
                a.seed.tasks.push(run.result());
            }
            a.seed.seconds = a.seed_t as f32;
            a.results.push(std::mem::take(&mut a.seed));
        }
        if let Some(mut bear) = self.bear() {
            let mut b = bear.bind_mut();
            b.set_event_tap(false);
            b.restore_limbs();
        }
        let wall = Time::singleton()
            .get_ticks_usec()
            .saturating_sub(a.start_usec) as f32
            / 1e6;
        let report = AutoReport {
            mode: a.mode,
            seeds: a.results,
            cancelled,
            headless: DisplayServer::singleton().get_name() == "headless",
            wall_seconds: wall,
        };
        let markdown = report.markdown();
        let data = serde_json::to_string_pretty(&report).unwrap_or_default();
        godot_print!("BfAutoplay: report\n{markdown}");
        let name = format!("autoplay-{}", a.mode.name());
        if let Some(mut recorder) = self.node(RECORDER_GROUP) {
            recorder.call(
                "add_test_result",
                &[name.to_variant(), markdown.to_variant(), data.to_variant()],
            );
        }
        if let Some(path) = &a.out_path {
            let json_path = match path.strip_suffix(".md") {
                Some(stem) => format!("{stem}.json"),
                None => format!("{path}.json"),
            };
            if std::fs::write(path, &markdown).is_err()
                || std::fs::write(&json_path, &data).is_err()
            {
                godot_warn!("BfAutoplay: could not write {path}");
            }
        }
        let passed = report.passed();
        godot_print!(
            "BfAutoplay: result {}",
            if passed { "pass" } else { "fail" }
        );
        for f in report.failures() {
            godot_print!("BfAutoplay: failure: {f}");
        }
        if let Some(label) = &mut self.label {
            label.set_visible(false);
        }
        if !self.base().is_inside_tree() {
            return;
        }
        if a.quit_at_end {
            let code = if passed { 0 } else { 1 };
            self.base().get_tree().quit_ex().exit_code(code).done();
        } else if let Some(mut terrain) = self.node(TERRAIN_GROUP) {
            if terrain.call("current_seed", &[]).to::<i64>() != a.saved_seed {
                terrain.call("make_zone", &[a.saved_seed.to_variant()]);
            }
        }
    }
}
