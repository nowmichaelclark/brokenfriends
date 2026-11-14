//! The autoplay bot (S-133, `plan/diagnostics.md`, "Autoplay"): the
//! parts without Godot.
//!
//! The bot plays the bear through the gamepad. It does not move the
//! bear itself, except for a teleport at the start of a task and the
//! limb changes of the dev tools. This module has the tasks, the
//! search for steep faces and ledges, the step of each task, the
//! seeded random input of "wander" and the report. The Godot node is
//! `crate::autoplay::BfAutoplay`.

use bf_worldgen::HeightMap;
use serde::Serialize;
use std::collections::BTreeMap;
use std::f32::consts::TAU;

/// The two kinds of autoplay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// A fixed list of tasks for each seed.
    Tour,
    /// Seeded random input for some minutes on each seed.
    Wander,
}

impl Mode {
    /// The name on the command line and in the report.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Tour => "tour",
            Mode::Wander => "wander",
        }
    }

    /// The name on the menu and on the screen.
    pub fn title(self) -> &'static str {
        match self {
            Mode::Tour => "Autoplay tour",
            Mode::Wander => "Autoplay wander",
        }
    }

    /// Gives the mode for a name (`--bf-autoplay=tour`).
    pub fn from_name(name: &str) -> Option<Mode> {
        match name {
            "tour" | "" => Some(Mode::Tour),
            "wander" => Some(Mode::Wander),
            _ => None,
        }
    }
}

/// The seeds of the tour from the menu: the seeds of the benchmark.
pub const TOUR_SEEDS: [i64; 3] = [1, 7, 42];

/// The bear never goes farther than this from the centre in a task
/// that is not about the smoke, in metres. The wall of smoke is more
/// than 590 m away.
pub const SAFE_RADIUS_M: f32 = 520.0;

/// A small seeded random source (SplitMix64). The same seed gives the
/// same numbers on all computers.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 to 1 (1 is not included).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A number from `a` to `b`.
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }

    /// True with the chance `p` (0 to 1).
    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }
}

/// The gamepad for one physics step, in world terms. The node changes
/// `dir` into a stick direction for the camera (`stick_for`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pad {
    /// The wanted move direction on the ground (x, z). Length 0 to 1.
    pub dir: [f32; 2],
    /// Hold the jog button.
    pub jog: bool,
    /// Hold the jump button.
    pub jump: bool,
    /// Hold the let-go button.
    pub let_go: bool,
}

/// Changes a world direction (x, z) into the left stick (x right, y
/// down) for a camera that turns `yaw` radians around Y. It is the
/// inverse of the turn in `BfBear::read_input`.
pub fn stick_for(dir: [f32; 2], yaw: f32) -> [f32; 2] {
    let (sin, cos) = yaw.sin_cos();
    [dir[0] * cos - dir[1] * sin, dir[0] * sin + dir[1] * cos]
}

/// What the bear does, from the `state` of `BfBear::recorder_state`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BearState {
    #[default]
    Ground,
    Air,
    Climb,
    Mantle,
}

impl BearState {
    pub fn from_name(name: &str) -> BearState {
        match name {
            // A tumbling bear is not on its feet.
            "jump" | "fall" | "tumble" => BearState::Air,
            "climb" | "slide" => BearState::Climb,
            "mantle" => BearState::Mantle,
            _ => BearState::Ground,
        }
    }
}

/// What the bot sees in one physics step.
#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    /// The time of the step in seconds.
    pub dt: f32,
    /// The feet of the bear.
    pub pos: [f32; 3],
    pub state: BearState,
    /// The events of the bear in this step ("grab", "jump", ...).
    pub events: &'a [String],
}

/// Something that the node must do for the bot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Request {
    /// Put the bear on the ground at (x, z) (`BfBear::go_to`).
    Teleport { x: f32, z: f32 },
    /// Give the bear these limbs: left arm, right arm, left leg, right leg.
    Limbs([bool; 4]),
}

/// One part of a task.
#[derive(Clone, Debug, PartialEq)]
pub enum Leg {
    /// Put the bear on the ground at (x, z).
    Teleport { x: f32, z: f32 },
    /// Give the bear a set of limbs.
    Limbs([bool; 4]),
    /// Move to a point. A jump comes when the bear does not move for a
    /// time. Done near the point or after `max_s`.
    Go {
        to: [f32; 2],
        jog: bool,
        near: f32,
        max_s: f32,
    },
    /// Push into a wall in the direction `dir` and climb. A hop comes
    /// each 2.5 s. Done after `climb_s` on the wall, after a mantle,
    /// or after `max_s`. At the end, the let-go button drops the bear.
    Climb {
        dir: [f32; 2],
        climb_s: f32,
        max_s: f32,
    },
    /// Jog in `dir` and jump at the edge point. Done after the landing
    /// or after `max_s`.
    LedgeJump {
        dir: [f32; 2],
        edge: [f32; 2],
        max_s: f32,
    },
    /// No input for a time.
    Idle { seconds: f32 },
    /// One press of the jump button.
    Jump,
    /// Jog in `dir` until the bear dies, or for `max_s`.
    UntilDeath { dir: [f32; 2], max_s: f32 },
    /// Seeded random input for a time.
    Wander { seconds: f32, seed: u64 },
}

impl Leg {
    fn name(&self) -> &'static str {
        match self {
            Leg::Teleport { .. } => "teleport",
            Leg::Limbs(_) => "limbs",
            Leg::Go { .. } => "go",
            Leg::Climb { .. } => "climb",
            Leg::LedgeJump { .. } => "ledge_jump",
            Leg::Idle { .. } => "idle",
            Leg::Jump => "jump",
            Leg::UntilDeath { .. } => "until_death",
            Leg::Wander { .. } => "wander",
        }
    }
}

/// The kinds of task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    /// Walk and jog to a point far away.
    FarWalk,
    /// Climb the nearest steep face.
    Climb,
    /// Jump off the nearest ledge.
    LedgeJump,
    /// Go to the next obstacle (one of each type, `KIND_COUNT` tasks).
    Obstacle(u8),
    /// Go into the smoke at the edge, come out, then stay until death.
    Smoke,
    /// Move with each set of limbs.
    Limbs,
    /// A part of a wander run: a teleport and random input.
    Wander(u32),
}

impl TaskKind {
    pub fn name(self) -> String {
        match self {
            TaskKind::FarWalk => "far_walk".into(),
            TaskKind::Climb => "climb".into(),
            TaskKind::LedgeJump => "ledge_jump".into(),
            TaskKind::Obstacle(i) => format!("obstacle_{}", i + 1),
            TaskKind::Smoke => "smoke_edge".into(),
            TaskKind::Limbs => "limb_sets".into(),
            TaskKind::Wander(i) => format!("wander_{}", i + 1),
        }
    }
}

/// The tasks of the tour, for each seed.
pub fn tour_tasks() -> Vec<TaskKind> {
    let mut v = vec![TaskKind::FarWalk, TaskKind::Climb, TaskKind::LedgeJump];
    v.extend((0..bf_worldgen::obstacles::KIND_COUNT as u8).map(TaskKind::Obstacle));
    v.push(TaskKind::Smoke);
    v.push(TaskKind::Limbs);
    v
}

/// The length of one wander task in seconds. Each task starts at a
/// new random place.
pub const WANDER_CHUNK_S: f32 = 40.0;

/// The tasks of a wander run of `minutes` on one seed.
pub fn wander_tasks(minutes: f32) -> Vec<TaskKind> {
    let n = ((minutes.max(0.1) * 60.0) / WANDER_CHUNK_S).ceil() as u32;
    (0..n).map(TaskKind::Wander).collect()
}

/// An obstacle that `BfObstacles::go_to_next` chose.
#[derive(Clone, Debug, PartialEq)]
pub struct ObstacleInfo {
    pub kind: String,
    pub pos: [f32; 3],
    /// The direction of the obstacle on the ground plane (x, z).
    pub dir: [f32; 2],
}

/// What the task builder knows about the game.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'a> {
    pub heights: &'a HeightMap,
    pub seed: i64,
    /// The start point of the zone (x, z).
    pub start: [f32; 2],
    /// The feet of the bear now.
    pub bear: [f32; 3],
    /// The obstacle for an obstacle task (the node asks for it first).
    pub obstacle: Option<&'a ObstacleInfo>,
    /// Angles (radians) and the front of the smoke wall there, in
    /// metres from the centre. The first is at `smoke_angle(seed)`.
    /// Empty: no wall.
    pub smoke_fronts: &'a [(f32, f32)],
}

/// The things that must happen in a task. They do not fail the build:
/// the bot is simple, and the report only tells what it reached.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Expect {
    /// Each prefix must be the start of a bear event in the task.
    pub events: Vec<&'static str>,
    /// The bear must get this far from its place at the start of the
    /// task (after the teleport), in metres.
    pub moved_m: f32,
}

/// One task: a name, its legs and what must happen.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub name: String,
    pub legs: Vec<Leg>,
    pub expect: Expect,
    /// A short note from the builder, for example "no steep face".
    pub note: String,
}

/// The first angle (radians) at which the tour tries to go into the
/// smoke for a seed.
pub fn smoke_angle(seed: i64) -> f32 {
    Rng::new(seed as u64 ^ 0x5A0E).unit() * TAU
}

/// The number of angles at which the node measures the smoke front.
pub const SMOKE_ANGLES: usize = 32;

/// The angles at which the node measures the smoke front for a seed.
pub fn smoke_angles(seed: i64) -> Vec<f32> {
    let first = smoke_angle(seed);
    (0..SMOKE_ANGLES)
        .map(|i| (first + i as f32 * TAU / SMOKE_ANGLES as f32) % TAU)
        .collect()
}

/// Chooses the way into the smoke: the first angle where the ground
/// from 16 m before the front to 20 m after it is walkable (less than
/// 30 degrees). Gives the angle, the front and whether the ground is
/// walkable. If no way is walkable, it gives the first angle.
pub fn smoke_path(h: &HeightMap, fronts: &[(f32, f32)]) -> Option<(f32, f32, bool)> {
    let fronts: Vec<(f32, f32)> = fronts.iter().copied().filter(|f| f.1 > 0.0).collect();
    let walkable = |&(a, front): &(f32, f32)| {
        (-16..=20).all(|i| {
            let r = front + i as f32;
            slope_deg(h, a.cos() * r, a.sin() * r) < 30.0
        })
    };
    match fronts.iter().find(|f| walkable(f)) {
        Some(&(a, front)) => Some((a, front, true)),
        None => fronts.first().map(|&(a, front)| (a, front, false)),
    }
}

fn len2(v: [f32; 2]) -> f32 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

fn norm2(v: [f32; 2]) -> [f32; 2] {
    let l = len2(v);
    if l < 1e-6 {
        [0.0, 0.0]
    } else {
        [v[0] / l, v[1] / l]
    }
}

/// The uphill direction times the slope (rise over run) at a point.
pub fn gradient(h: &HeightMap, x: f32, z: f32) -> [f32; 2] {
    let d = h.spacing.max(0.25);
    [
        (h.sample_world(x + d, z) - h.sample_world(x - d, z)) / (2.0 * d),
        (h.sample_world(x, z + d) - h.sample_world(x, z - d)) / (2.0 * d),
    ]
}

/// The slope at a point in degrees.
pub fn slope_deg(h: &HeightMap, x: f32, z: f32) -> f32 {
    len2(gradient(h, x, z)).atan().to_degrees()
}

/// A steep face to climb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    /// A walkable place below the face (x, z).
    pub approach: [f32; 2],
    /// The foot of the face.
    pub face: [f32; 2],
    /// The direction into the face, on the ground plane.
    pub dir: [f32; 2],
}

/// A ledge to jump off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    /// A walkable place before the edge.
    pub approach: [f32; 2],
    /// The edge.
    pub edge: [f32; 2],
    /// The direction of the jump, on the ground plane.
    pub dir: [f32; 2],
    /// How far the ground drops 1.5 m after the edge, in metres.
    pub drop_m: f32,
}

/// The points of a square grid around `from`, nearest first.
fn search_points(from: [f32; 2], radius: f32, step: f32) -> Vec<[f32; 2]> {
    let n = (radius / step).ceil() as i32;
    let mut points = Vec::new();
    for iz in -n..=n {
        for ix in -n..=n {
            let p = [ix as f32 * step, iz as f32 * step];
            if len2(p) <= radius {
                points.push(p);
            }
        }
    }
    points.sort_by(|a, b| len2(*a).total_cmp(&len2(*b)));
    points
        .into_iter()
        .map(|p| [from[0] + p[0], from[1] + p[1]])
        .filter(|p| len2(*p) <= SAFE_RADIUS_M)
        .collect()
}

/// Finds the nearest steep face (steeper than `min_deg`, at least
/// 2.5 m high) with walkable ground below it.
pub fn find_face(h: &HeightMap, from: [f32; 2], radius: f32, min_deg: f32) -> Option<Face> {
    let min_slope = min_deg.to_radians().tan();
    for p in search_points(from, radius, 2.0) {
        let g = gradient(h, p[0], p[1]);
        if len2(g) < min_slope {
            continue;
        }
        let up = norm2(g);
        let approach = [p[0] - up[0] * 3.0, p[1] - up[1] * 3.0];
        if slope_deg(h, approach[0], approach[1]) > 30.0 {
            continue;
        }
        let above = [p[0] + up[0] * 2.0, p[1] + up[1] * 2.0];
        let rise = h.sample_world(above[0], above[1]) - h.sample_world(approach[0], approach[1]);
        if rise < 2.5 {
            continue;
        }
        return Some(Face {
            approach,
            face: p,
            dir: up,
        });
    }
    None
}

/// Finds the nearest ledge: a walkable edge where the ground 1.5 m
/// further is at least `min_drop` lower, with flat ground before it.
pub fn find_ledge(h: &HeightMap, from: [f32; 2], radius: f32, min_drop: f32) -> Option<Ledge> {
    let dirs: Vec<[f32; 2]> = (0..8)
        .map(|i| {
            let a = i as f32 * TAU / 8.0;
            [a.cos(), a.sin()]
        })
        .collect();
    for p in search_points(from, radius, 2.0) {
        if slope_deg(h, p[0], p[1]) > 25.0 {
            continue;
        }
        let here = h.sample_world(p[0], p[1]);
        for d in &dirs {
            let below = h.sample_world(p[0] + d[0] * 1.5, p[1] + d[1] * 1.5);
            let drop = here - below;
            if drop < min_drop {
                continue;
            }
            let approach = [p[0] - d[0] * 4.0, p[1] - d[1] * 4.0];
            let back = h.sample_world(approach[0], approach[1]);
            if (back - here).abs() > 1.0 || slope_deg(h, approach[0], approach[1]) > 25.0 {
                continue;
            }
            return Some(Ledge {
                approach,
                edge: p,
                dir: *d,
                drop_m: drop,
            });
        }
    }
    None
}

/// Finds a random walkable place near the centre for a wander task.
pub fn wander_place(h: &HeightMap, rng: &mut Rng) -> [f32; 2] {
    let mut best = [0.0, 0.0];
    let mut best_slope = f32::MAX;
    for _ in 0..24 {
        let a = rng.unit() * TAU;
        let r = rng.unit().sqrt() * (SAFE_RADIUS_M - 40.0);
        let p = [a.cos() * r, a.sin() * r];
        let s = slope_deg(h, p[0], p[1]);
        if s < 30.0 {
            return p;
        }
        if s < best_slope {
            best_slope = s;
            best = p;
        }
    }
    best
}

/// The limb sets of the tour: one leg, no legs, one arm, then all.
pub const TOUR_LIMB_SETS: [(&str, [bool; 4]); 4] = [
    ("one_leg", [true, true, true, false]),
    ("no_legs", [true, true, false, false]),
    ("one_arm", [true, false, true, true]),
    ("all", [true, true, true, true]),
];

/// Makes the task of a kind.
pub fn build(kind: TaskKind, ctx: &Ctx) -> Task {
    let here = [ctx.bear[0], ctx.bear[2]];
    let mut rng = Rng::new((ctx.seed as u64).wrapping_mul(1_000_003) ^ kind_salt(kind));
    let mut task = Task {
        name: kind.name(),
        legs: Vec::new(),
        expect: Expect::default(),
        note: String::new(),
    };
    match kind {
        TaskKind::FarWalk => {
            // From the start point, jog to a point 70 m away.
            let a = rng.unit() * TAU;
            let to = clamp_safe([ctx.start[0] + a.cos() * 70.0, ctx.start[1] + a.sin() * 70.0]);
            task.legs = vec![
                Leg::Teleport {
                    x: ctx.start[0],
                    z: ctx.start[1],
                },
                Leg::Go {
                    to,
                    jog: true,
                    near: 1.5,
                    max_s: 20.0,
                },
            ];
            task.expect.moved_m = 20.0;
            task.note = format!("to ({:.0}, {:.0})", to[0], to[1]);
        }
        TaskKind::Climb => {
            let face = find_face(ctx.heights, here, 200.0, 58.0)
                .or_else(|| find_face(ctx.heights, ctx.start, 400.0, 58.0));
            match face {
                Some(f) => {
                    task.legs = vec![
                        Leg::Teleport {
                            x: f.approach[0],
                            z: f.approach[1],
                        },
                        Leg::Idle { seconds: 0.5 },
                        Leg::Climb {
                            dir: f.dir,
                            climb_s: 10.0,
                            max_s: 18.0,
                        },
                        Leg::Idle { seconds: 1.5 },
                    ];
                    task.expect.events = vec!["grab"];
                    task.note = format!("face at ({:.0}, {:.0})", f.face[0], f.face[1]);
                }
                None => task.note = "no steep face found".into(),
            }
        }
        TaskKind::LedgeJump => {
            let ledge = find_ledge(ctx.heights, here, 200.0, 1.2)
                .or_else(|| find_ledge(ctx.heights, ctx.start, 400.0, 1.2));
            match ledge {
                Some(l) => {
                    task.legs = vec![
                        Leg::Teleport {
                            x: l.approach[0],
                            z: l.approach[1],
                        },
                        Leg::Idle { seconds: 0.5 },
                        Leg::LedgeJump {
                            dir: l.dir,
                            edge: l.edge,
                            max_s: 7.0,
                        },
                        Leg::Idle { seconds: 1.0 },
                    ];
                    task.expect.events = vec!["jump"];
                    task.note = format!(
                        "edge at ({:.0}, {:.0}), drop {:.1} m",
                        l.edge[0], l.edge[1], l.drop_m
                    );
                }
                None => task.note = "no ledge found".into(),
            }
        }
        TaskKind::Obstacle(_) => match ctx.obstacle {
            // `go_to_next` put the bear next to the obstacle already.
            Some(o) => {
                // The rock teeth tear only a fast bear; the bot jogs
                // down over them, which is slower than their limit.
                let jog = matches!(
                    o.kind.as_str(),
                    "ash_crust" | "stepping_stones" | "thorn_snag" | "rock_teeth"
                );
                // Wait longer than one cycle of the obstacle (`obstacles.rs`).
                let wait = match o.kind.as_str() {
                    "rolling_stones" => 12.0,
                    "gust" => 7.0,
                    "vent_push" => 6.5,
                    "vent_launch" => 5.0,
                    "sinking_sand" => 4.0,
                    // One full cycle: the tell, the fall and the pin.
                    "falling_rocks" => 14.0,
                    _ => 2.5,
                };
                // Where to go: a push vent hits only a bear on its line
                // (`obstacle_sim.rs`); the stepping stones are crossed.
                let (ahead, near, max_s) = match o.kind.as_str() {
                    "vent_push" => (0.6, 0.12, 4.5),
                    "stepping_stones" => (6.0, 0.6, 8.0),
                    // Through the bush, and down over the teeth.
                    "thorn_snag" => (1.2, 0.4, 6.0),
                    "rock_teeth" => (2.0, 0.5, 6.0),
                    _ => (0.0, 0.4, 4.5),
                };
                let mut legs = vec![
                    Leg::Idle { seconds: 0.5 },
                    Leg::Go {
                        to: [o.pos[0] + o.dir[0] * ahead, o.pos[2] + o.dir[1] * ahead],
                        jog,
                        near,
                        max_s,
                    },
                ];
                if o.kind == "updraft" {
                    // The updraft lifts only a bear in the air.
                    for _ in 0..2 {
                        legs.push(Leg::Jump);
                        legs.push(Leg::Idle { seconds: 1.5 });
                    }
                }
                legs.push(Leg::Idle { seconds: wait });
                task.legs = legs;
                task.expect.events = vec!["obstacle:"];
                task.note = o.kind.clone();
            }
            None => task.note = "no obstacle in the zone".into(),
        },
        TaskKind::Smoke => match smoke_path(ctx.heights, ctx.smoke_fronts) {
            None => task.note = "no smoke wall".into(),
            Some((a, front, flat)) => {
                let out = [a.cos(), a.sin()];
                let at = |r: f32| [out[0] * r, out[1] * r];
                let start = at(front - 8.0);
                let go = |r: f32, max_s: f32| Leg::Go {
                    to: at(r),
                    jog: true,
                    near: 1.0,
                    max_s,
                };
                task.legs = vec![
                    Leg::Teleport {
                        x: start[0],
                        z: start[1],
                    },
                    Leg::Idle { seconds: 0.5 },
                    // The front moves up to 0.35 m/s, so the points have a margin.
                    go(front + 10.0, 9.0),
                    go(front - 14.0, 11.0),
                    go(front + 8.0, 9.0),
                    // Stand in the smoke until the bear dies.
                    Leg::UntilDeath {
                        dir: [0.0, 0.0],
                        max_s: 20.0,
                    },
                    Leg::Idle { seconds: 0.5 },
                ];
                task.expect.events = vec!["smoke_enter", "smoke_leave", "death:smoke"];
                task.note = format!(
                    "angle {:.0} deg, front {:.0} m{}",
                    a.to_degrees(),
                    front,
                    if flat { "" } else { ", steep" }
                );
            }
        },
        TaskKind::Limbs => {
            let mut legs = Vec::new();
            for (i, (_, set)) in TOUR_LIMB_SETS.iter().enumerate() {
                let a = rng.unit() * TAU;
                legs.push(Leg::Limbs(*set));
                if i == TOUR_LIMB_SETS.len() - 1 {
                    break;
                }
                legs.push(Leg::Go {
                    to: clamp_safe([here[0] + a.cos() * 10.0, here[1] + a.sin() * 10.0]),
                    jog: true,
                    near: 0.5,
                    max_s: 4.0,
                });
            }
            legs.push(Leg::Idle { seconds: 0.5 });
            task.legs = legs;
            task.expect.events = vec!["gait:hop", "gait:"];
            task.note = TOUR_LIMB_SETS
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ");
        }
        TaskKind::Wander(_) => {
            let place = wander_place(ctx.heights, &mut rng);
            let limbs = if rng.chance(0.3) {
                let mut set = [true; 4];
                set[(rng.next_u64() % 4) as usize] = false;
                set
            } else {
                [true; 4]
            };
            task.legs = vec![
                Leg::Teleport {
                    x: place[0],
                    z: place[1],
                },
                Leg::Limbs(limbs),
                Leg::Wander {
                    seconds: WANDER_CHUNK_S,
                    seed: rng.next_u64(),
                },
            ];
            task.note = format!("at ({:.0}, {:.0})", place[0], place[1]);
        }
    }
    task
}

fn kind_salt(kind: TaskKind) -> u64 {
    match kind {
        TaskKind::FarWalk => 1,
        TaskKind::Climb => 2,
        TaskKind::LedgeJump => 3,
        TaskKind::Obstacle(i) => 10 + u64::from(i),
        TaskKind::Smoke => 4,
        TaskKind::Limbs => 5,
        TaskKind::Wander(i) => 1000 + u64::from(i),
    }
}

fn clamp_safe(p: [f32; 2]) -> [f32; 2] {
    let l = len2(p);
    if l > SAFE_RADIUS_M {
        [p[0] * SAFE_RADIUS_M / l, p[1] * SAFE_RADIUS_M / l]
    } else {
        p
    }
}

/// A press of the jump or the let-go button. The bear reads a press
/// when the button goes down, so the bot holds it for a short time and
/// then lets it up for a short time.
#[derive(Clone, Copy, Debug, Default)]
struct Pulse {
    /// The time that the button stays down.
    down_left: f32,
    /// The time until the next press can start.
    wait_left: f32,
}

const PULSE_DOWN_S: f32 = 0.1;
const PULSE_UP_S: f32 = 0.12;

impl Pulse {
    fn press(&mut self) {
        if self.down_left <= 0.0 && self.wait_left <= 0.0 {
            self.down_left = PULSE_DOWN_S;
            self.wait_left = PULSE_DOWN_S + PULSE_UP_S;
        }
    }

    /// Gives if the button is down in this step.
    fn step(&mut self, dt: f32) -> bool {
        let down = self.down_left > 0.0;
        self.down_left -= dt;
        self.wait_left -= dt;
        down
    }
}

/// The memory of the leg that runs now.
#[derive(Clone, Debug, Default)]
struct LegState {
    t: f32,
    climbed_s: f32,
    was_climbing: bool,
    jumped: bool,
    airborne: bool,
    next_hop: f32,
    /// For "Go": the place and the time of the last real movement.
    last_moved_pos: [f32; 3],
    last_moved_t: f32,
    /// For "Wander".
    rng: Option<Rng>,
    segment_left: f32,
    segment: Pad,
    jump_rate: f32,
    outcome: &'static str,
}

/// The result of one task.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TaskResult {
    pub name: String,
    pub note: String,
    /// The game seconds of the task.
    pub seconds: f32,
    /// For each leg: its name and how it ended.
    pub legs: Vec<String>,
    /// The bear events that the task expected and saw.
    pub seen: Vec<String>,
    /// The bear events that the task expected and did not see.
    pub missed: Vec<String>,
    /// The largest distance from the place at the start, in metres.
    pub moved_m: f32,
    /// The task reached what it expected.
    pub reached: bool,
}

/// Runs one task step by step.
#[derive(Clone, Debug)]
pub struct TaskRun {
    task: Task,
    leg: usize,
    state: LegState,
    jump: Pulse,
    let_go: Pulse,
    t: f32,
    origin: Option<[f32; 3]>,
    moved_m: f32,
    seen: Vec<bool>,
    leg_log: Vec<String>,
}

impl TaskRun {
    pub fn new(task: Task) -> Self {
        let seen = vec![false; task.expect.events.len()];
        TaskRun {
            task,
            leg: 0,
            state: LegState::default(),
            jump: Pulse::default(),
            let_go: Pulse::default(),
            t: 0.0,
            origin: None,
            moved_m: 0.0,
            seen,
            leg_log: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.task.name
    }

    /// The name of the leg that runs now.
    pub fn leg_name(&self) -> &'static str {
        self.task.legs.get(self.leg).map_or("done", |l| l.name())
    }

    /// Tells if the task has no more legs.
    pub fn is_done(&self) -> bool {
        self.leg >= self.task.legs.len()
    }

    /// One physics step. Gives the gamepad and, at most, one request.
    pub fn step(&mut self, v: &View) -> (Pad, Option<Request>) {
        self.t += v.dt;
        // A restart (death) moves the bear: measure from the new place.
        if v.events
            .iter()
            .any(|e| e == "restart" || e.starts_with("dev:go_to"))
        {
            self.origin = Some(v.pos);
        }
        for (i, prefix) in self.task.expect.events.iter().enumerate() {
            if v.events.iter().any(|e| e.starts_with(prefix)) {
                self.seen[i] = true;
            }
        }
        if let Some(o) = self.origin {
            let d = len2([v.pos[0] - o[0], v.pos[2] - o[2]]);
            self.moved_m = self.moved_m.max(d);
        }
        let mut pad = Pad::default();
        let mut request = None;
        // A teleport or a limb change takes one step; the next leg
        // starts in the next step.
        if let Some(leg) = self.task.legs.get(self.leg).cloned() {
            let (p, r, done) = self.leg_step(&leg, v);
            pad = p;
            request = r;
            if done {
                let outcome = if self.state.outcome.is_empty() {
                    "done"
                } else {
                    self.state.outcome
                };
                self.leg_log
                    .push(format!("{} {} {:.1}s", leg.name(), outcome, self.state.t));
                self.leg += 1;
                self.state = LegState::default();
                if matches!(leg, Leg::Teleport { .. }) {
                    // Measure the movement from the new place.
                    self.origin = None;
                }
            }
        }
        if self.origin.is_none() && !matches!(request, Some(Request::Teleport { .. })) {
            self.origin = Some(v.pos);
        }
        pad.jump |= self.jump.step(v.dt);
        pad.let_go |= self.let_go.step(v.dt);
        (pad, request)
    }

    fn leg_step(&mut self, leg: &Leg, v: &View) -> (Pad, Option<Request>, bool) {
        let s = &mut self.state;
        s.t += v.dt;
        let here = [v.pos[0], v.pos[2]];
        let climbing = matches!(v.state, BearState::Climb | BearState::Mantle);
        match *leg {
            Leg::Teleport { x, z } => (Pad::default(), Some(Request::Teleport { x, z }), true),
            Leg::Limbs(set) => (Pad::default(), Some(Request::Limbs(set)), true),
            Leg::Idle { seconds } => (Pad::default(), None, s.t >= seconds),
            Leg::Jump => {
                self.jump.press();
                (Pad::default(), None, true)
            }
            Leg::Go {
                to,
                jog,
                near,
                max_s,
            } => {
                let to_target = [to[0] - here[0], to[1] - here[1]];
                if len2(to_target) <= near {
                    s.outcome = "reached";
                    return (Pad::default(), None, true);
                }
                if s.t >= max_s {
                    s.outcome = "timeout";
                    return (Pad::default(), None, true);
                }
                // A jump when the bear did not move for 1 s on the ground.
                let moved = {
                    let p = s.last_moved_pos;
                    let d = [v.pos[0] - p[0], v.pos[1] - p[1], v.pos[2] - p[2]];
                    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                };
                if s.t < v.dt * 1.5 || moved > 0.3 {
                    s.last_moved_pos = v.pos;
                    s.last_moved_t = s.t;
                } else if s.t - s.last_moved_t > 1.0 && v.state == BearState::Ground {
                    self.jump.press();
                    s.last_moved_t = s.t;
                }
                let pad = Pad {
                    dir: norm2(to_target),
                    jog,
                    ..Pad::default()
                };
                (pad, None, false)
            }
            Leg::Climb {
                dir,
                climb_s,
                max_s,
            } => {
                if climbing {
                    s.climbed_s += v.dt;
                    s.was_climbing = true;
                    if s.t >= s.next_hop {
                        s.next_hop = s.t + 2.5;
                        if v.state == BearState::Climb {
                            self.jump.press();
                        }
                    }
                } else if s.was_climbing && v.state == BearState::Ground {
                    s.outcome = "mantled_or_off";
                    return (Pad::default(), None, true);
                }
                if s.climbed_s >= climb_s || s.t >= max_s {
                    if v.state == BearState::Climb {
                        self.let_go.press();
                        s.outcome = "let_go";
                    } else if s.was_climbing {
                        s.outcome = "climbed";
                    } else {
                        s.outcome = "no_grab";
                    }
                    return (Pad::default(), None, true);
                }
                let pad = Pad {
                    dir,
                    ..Pad::default()
                };
                (pad, None, false)
            }
            Leg::LedgeJump { dir, edge, max_s } => {
                let ahead = (edge[0] - here[0]) * dir[0] + (edge[1] - here[1]) * dir[1];
                if !s.jumped && ahead < 0.35 && v.state == BearState::Ground {
                    self.jump.press();
                    s.jumped = true;
                }
                if v.state == BearState::Air {
                    s.airborne = true;
                }
                if s.airborne && v.state != BearState::Air {
                    s.outcome = "landed";
                    return (Pad::default(), None, true);
                }
                if s.t >= max_s {
                    s.outcome = if s.jumped { "timeout" } else { "no_jump" };
                    return (Pad::default(), None, true);
                }
                let pad = Pad {
                    dir,
                    jog: true,
                    ..Pad::default()
                };
                (pad, None, false)
            }
            Leg::UntilDeath { dir, max_s } => {
                if v.events.iter().any(|e| e.starts_with("death:")) {
                    s.outcome = "died";
                    return (Pad::default(), None, true);
                }
                if s.t >= max_s {
                    s.outcome = "timeout";
                    return (Pad::default(), None, true);
                }
                let pad = Pad {
                    dir,
                    jog: true,
                    ..Pad::default()
                };
                (pad, None, false)
            }
            Leg::Wander { seconds, seed } => {
                if s.t >= seconds {
                    return (Pad::default(), None, true);
                }
                let rng = s.rng.get_or_insert_with(|| Rng::new(seed));
                s.segment_left -= v.dt;
                if s.segment_left <= 0.0 {
                    s.segment_left = rng.range(0.6, 3.0);
                    s.jump_rate = if rng.chance(0.5) {
                        rng.range(0.2, 1.2)
                    } else {
                        0.0
                    };
                    s.segment = if rng.chance(0.15) {
                        Pad::default()
                    } else {
                        let a = rng.unit() * TAU;
                        let m = rng.range(0.35, 1.0);
                        Pad {
                            dir: [a.cos() * m, a.sin() * m],
                            jog: rng.chance(0.35),
                            ..Pad::default()
                        }
                    };
                    // Stay inside the safe circle: turn back near its edge.
                    if len2(here) > SAFE_RADIUS_M - 20.0 {
                        let back = norm2([-here[0], -here[1]]);
                        s.segment.dir = back;
                    }
                }
                if s.jump_rate > 0.0 && rng.chance(s.jump_rate * v.dt) {
                    self.jump.press();
                }
                if climbing && rng.chance(0.08 * v.dt) {
                    self.let_go.press();
                }
                (s.segment, None, false)
            }
        }
    }

    /// The result so far.
    pub fn result(&self) -> TaskResult {
        let mut seen = Vec::new();
        let mut missed = Vec::new();
        for (i, prefix) in self.task.expect.events.iter().enumerate() {
            if self.seen[i] {
                seen.push(prefix.to_string());
            } else {
                missed.push(prefix.to_string());
            }
        }
        let moved_ok = self.moved_m >= self.task.expect.moved_m;
        let reached = !self.task.legs.is_empty() && missed.is_empty() && moved_ok;
        TaskResult {
            name: self.task.name.clone(),
            note: self.task.note.clone(),
            seconds: self.t,
            legs: self.leg_log.clone(),
            seen,
            missed,
            moved_m: self.moved_m,
            reached,
        }
    }
}

/// A put-back from this far below the ground or more is a fall through
/// the ground, in metres. `BfBear::stay_above_ground` acts at 0.25 m.
pub const DEEP_PUT_BACK_M: f32 = 0.5;

/// The counts of one seed. The node fills them.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Counts {
    /// The bear went below the terrain (outside the time after a teleport).
    pub through_ground: u32,
    /// `stay_above_ground` put the bear back on the ground from
    /// `DEEP_PUT_BACK_M` or more below it: a fall through the ground.
    pub back_on_ground: u32,
    /// The same from less than `DEEP_PUT_BACK_M` below. On steep ground
    /// the collision of Terrain3D and the Rust heightmap do not agree
    /// by some centimetres. These do not fail the build.
    pub small_put_back: u32,
    /// The same, in the short time after a teleport (Terrain3D can be
    /// late with the collision there). These do not fail the build.
    pub after_teleport: u32,
    pub stuck: u32,
    pub climb_loops: u32,
    pub grabs: u32,
    pub jumps: u32,
    pub deaths: u32,
    /// The obstacle events by "what:type", for example "hit:rolling_stones".
    pub obstacles: BTreeMap<String, u32>,
    /// The deepest place below the terrain, in metres.
    pub deepest_m: f32,
}

impl Counts {
    /// Counts one bear event.
    pub fn bear_event(&mut self, name: &str, after_teleport: bool) {
        if let Some(rest) = name.strip_prefix("back_on_ground") {
            // back_on_ground:0.41m
            let depth: f32 = rest
                .trim_start_matches(':')
                .trim_end_matches('m')
                .parse()
                .unwrap_or(f32::MAX);
            if after_teleport {
                self.after_teleport += 1;
            } else if depth >= DEEP_PUT_BACK_M {
                self.back_on_ground += 1;
                self.deepest_m = self.deepest_m.max(depth);
            } else {
                self.small_put_back += 1;
            }
        } else if name == "grab" {
            self.grabs += 1;
        } else if name == "jump" {
            self.jumps += 1;
        } else if name.starts_with("death:") {
            self.deaths += 1;
        } else if let Some(rest) = name.strip_prefix("obstacle:") {
            // obstacle:<what>:<type>:#<number>
            let mut parts = rest.split(':');
            if let (Some(what), Some(kind)) = (parts.next(), parts.next()) {
                *self.obstacles.entry(format!("{what}:{kind}")).or_default() += 1;
            }
        }
    }

    /// The bear fell through the ground: this fails the build.
    pub fn fell_through(&self) -> bool {
        self.through_ground > 0 || self.back_on_ground > 0
    }
}

/// The result of one seed.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SeedResult {
    pub seed: i64,
    /// The zone was made and the tasks ran.
    pub ran: bool,
    pub seconds: f32,
    pub tasks: Vec<TaskResult>,
    pub counts: Counts,
}

/// The result of an autoplay run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AutoReport {
    pub mode: Mode,
    pub seeds: Vec<SeedResult>,
    /// Somebody stopped the run.
    pub cancelled: bool,
    pub headless: bool,
    /// The real time of the run, in seconds.
    pub wall_seconds: f32,
}

impl AutoReport {
    /// Gives the problems that fail the build. Empty means "pass".
    pub fn failures(&self) -> Vec<String> {
        let mut out = Vec::new();
        for s in &self.seeds {
            if !s.ran {
                out.push(format!("seed {}: the zone did not load in time", s.seed));
            }
            if s.counts.fell_through() {
                out.push(format!(
                    "seed {}: the bear fell through the ground ({} times below it, {} put back, deepest {:.2} m)",
                    s.seed, s.counts.through_ground, s.counts.back_on_ground, s.counts.deepest_m
                ));
            }
        }
        out
    }

    pub fn passed(&self) -> bool {
        self.failures().is_empty()
    }

    /// The text for `report.md` and the build log.
    pub fn markdown(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("### {}\n\n", self.mode.title()));
        let failures = self.failures();
        if self.cancelled {
            s.push_str("Stopped before the end.\n\n");
        }
        if failures.is_empty() {
            s.push_str("Result: **pass** (no fall through the ground).\n\n");
        } else {
            s.push_str("Result: **fail**\n\n");
            for f in &failures {
                s.push_str(&format!("- {f}\n"));
            }
            s.push('\n');
        }
        let game_s: f32 = self.seeds.iter().map(|x| x.seconds).sum();
        s.push_str(&format!(
            "Game time {:.0} s, real time {:.0} s{}.\n\n",
            game_s,
            self.wall_seconds,
            if self.headless {
                " (headless: no speed numbers)"
            } else {
                ""
            }
        ));
        s.push_str(
            "| Seed | Time | Tasks reached | Below ground | Put back | Small put-back | After teleport | Stuck | Climb loops | Grabs | Jumps | Deaths |\n",
        );
        s.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");
        for r in &self.seeds {
            let reached = r.tasks.iter().filter(|t| t.reached).count();
            let c = &r.counts;
            s.push_str(&format!(
                "| {} | {:.0} s | {}/{} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                r.seed,
                r.seconds,
                reached,
                r.tasks.len(),
                c.through_ground,
                c.back_on_ground,
                c.small_put_back,
                c.after_teleport,
                c.stuck,
                c.climb_loops,
                c.grabs,
                c.jumps,
                c.deaths
            ));
        }
        s.push('\n');
        for r in &self.seeds {
            if r.tasks.is_empty() {
                continue;
            }
            s.push_str(&format!("Seed {} tasks:\n\n", r.seed));
            for t in &r.tasks {
                let mark = if t.reached { "ok" } else { "missed" };
                let mut line = format!("- {} ({}): {mark}, {:.0} s", t.name, t.note, t.seconds);
                if !t.missed.is_empty() {
                    line.push_str(&format!(", no {}", t.missed.join(", no ")));
                }
                line.push_str(&format!(", moved {:.0} m", t.moved_m));
                s.push_str(&line);
                s.push('\n');
            }
            if !r.counts.obstacles.is_empty() {
                let list: Vec<String> = r
                    .counts
                    .obstacles
                    .iter()
                    .map(|(k, n)| format!("{k} {n}"))
                    .collect();
                s.push_str(&format!("- obstacle events: {}\n", list.join(", ")));
            }
            s.push('\n');
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat zone with a wall: the ground is at 0 m for x < 10 and
    /// rises at 70 degrees after that, up to 20 m.
    fn wall_zone() -> HeightMap {
        let size = 256;
        let mut h = HeightMap::new(size, 1.0);
        for z in 0..size {
            for x in 0..size {
                let wx = x as f32 - size as f32 * 0.5;
                let rise = ((wx - 10.0).max(0.0) * 70f32.to_radians().tan()).min(20.0);
                h.data[z * size + x] = rise;
            }
        }
        h
    }

    /// A zone with a step down: 5 m high for x < 20, 0 m after that.
    fn ledge_zone() -> HeightMap {
        let size = 256;
        let mut h = HeightMap::new(size, 1.0);
        for z in 0..size {
            for x in 0..size {
                let wx = x as f32 - size as f32 * 0.5;
                h.data[z * size + x] = if wx < 20.0 { 5.0 } else { 0.0 };
            }
        }
        h
    }

    #[test]
    fn stick_turns_back() {
        // The bear turns the stick by the camera yaw. The bot must undo it.
        for &yaw in &[0.0f32, 0.7, -2.1, 3.0] {
            let dir = [0.6f32, -0.8];
            let s = stick_for(dir, yaw);
            let (sin, cos) = yaw.sin_cos();
            let wish = [s[0] * cos + s[1] * sin, -s[0] * sin + s[1] * cos];
            assert!((wish[0] - dir[0]).abs() < 1e-5 && (wish[1] - dir[1]).abs() < 1e-5);
        }
    }

    #[test]
    fn rng_is_seeded() {
        let a: Vec<u64> = (0..5)
            .scan(Rng::new(7), |r, _| Some(r.next_u64()))
            .collect();
        let b: Vec<u64> = (0..5)
            .scan(Rng::new(7), |r, _| Some(r.next_u64()))
            .collect();
        assert_eq!(a, b);
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            let u = r.unit();
            assert!((0.0..1.0).contains(&u));
        }
    }

    #[test]
    fn finds_the_wall() {
        let h = wall_zone();
        let face = find_face(&h, [-30.0, 0.0], 100.0, 58.0).expect("a face");
        assert!(face.dir[0] > 0.9, "into the wall is +x: {:?}", face.dir);
        assert!(face.approach[0] < face.face[0]);
        assert!(slope_deg(&h, face.approach[0], face.approach[1]) < 30.0);
        // No face on flat ground.
        let flat = HeightMap::new(128, 1.0);
        assert!(find_face(&flat, [0.0, 0.0], 50.0, 58.0).is_none());
    }

    #[test]
    fn finds_the_ledge() {
        let h = ledge_zone();
        let l = find_ledge(&h, [0.0, 0.0], 100.0, 1.2).expect("a ledge");
        assert!(l.dir[0] > 0.9, "the jump goes to +x: {:?}", l.dir);
        assert!(l.drop_m >= 1.2);
        assert!(l.approach[0] < l.edge[0]);
    }

    #[test]
    fn the_tour_has_all_tasks() {
        let tasks = tour_tasks();
        assert_eq!(tasks.len(), 5 + bf_worldgen::obstacles::KIND_COUNT);
        let h = wall_zone();
        let o = ObstacleInfo {
            kind: "gust".into(),
            pos: [5.0, 0.0, 5.0],
            dir: [1.0, 0.0],
        };
        let ctx = Ctx {
            heights: &h,
            seed: 7,
            start: [-40.0, 0.0],
            bear: [-40.0, 0.0, 0.0],
            obstacle: Some(&o),
            smoke_fronts: &[(1.0, 700.0)],
        };
        for kind in tasks {
            let task = build(kind, &ctx);
            if kind == TaskKind::LedgeJump {
                // The wall zone has no drop that a bear can walk to.
                continue;
            }
            assert!(
                !task.legs.is_empty(),
                "{} has no legs: {}",
                task.name,
                task.note
            );
        }
        // The same seed gives the same tasks.
        assert_eq!(
            build(TaskKind::FarWalk, &ctx),
            build(TaskKind::FarWalk, &ctx)
        );
        assert_eq!(
            build(TaskKind::Wander(3), &ctx),
            build(TaskKind::Wander(3), &ctx)
        );
    }

    /// A very simple bear: it moves with the stick at 3 m/s on flat ground.
    fn simulate(task: Task, seconds: f32, events_at: &[(f32, &str)]) -> TaskResult {
        let mut run = TaskRun::new(task);
        let mut pos = [0.0f32, 0.0, 0.0];
        let dt = 1.0 / 60.0;
        let mut t = 0.0;
        while t < seconds && !run.is_done() {
            let events: Vec<String> = events_at
                .iter()
                .filter(|(at, _)| *at >= t && *at < t + dt)
                .map(|(_, e)| e.to_string())
                .collect();
            let view = View {
                dt,
                pos,
                state: BearState::Ground,
                events: &events,
            };
            let (pad, request) = run.step(&view);
            if let Some(Request::Teleport { x, z }) = request {
                pos = [x, 0.0, z];
            }
            pos[0] += pad.dir[0] * 3.0 * dt;
            pos[2] += pad.dir[1] * 3.0 * dt;
            t += dt;
        }
        run.result()
    }

    #[test]
    fn go_reaches_the_point() {
        let task = Task {
            name: "t".into(),
            legs: vec![
                Leg::Teleport { x: 1.0, z: 1.0 },
                Leg::Go {
                    to: [10.0, 1.0],
                    jog: false,
                    near: 0.5,
                    max_s: 10.0,
                },
            ],
            expect: Expect {
                events: vec![],
                moved_m: 8.0,
            },
            note: String::new(),
        };
        let r = simulate(task, 20.0, &[]);
        assert!(r.reached, "{r:?}");
        assert!(r.legs[1].starts_with("go reached"), "{:?}", r.legs);
        assert!(r.seconds < 4.0);
    }

    #[test]
    fn missed_events_show() {
        let task = Task {
            name: "t".into(),
            legs: vec![Leg::Idle { seconds: 1.0 }],
            expect: Expect {
                events: vec!["grab", "jump"],
                moved_m: 0.0,
            },
            note: String::new(),
        };
        let r = simulate(task, 5.0, &[(0.5, "jump")]);
        assert!(!r.reached);
        assert_eq!(r.seen, vec!["jump".to_string()]);
        assert_eq!(r.missed, vec!["grab".to_string()]);
    }

    #[test]
    fn jump_is_a_short_press() {
        let mut p = Pulse::default();
        p.press();
        let dt = 1.0 / 60.0;
        let downs: Vec<bool> = (0..30).map(|_| p.step(dt)).collect();
        let first_up = downs.iter().position(|d| !d).unwrap();
        assert!((5..=7).contains(&first_up), "{downs:?}");
        assert!(downs[first_up..].iter().all(|d| !d));
    }

    #[test]
    fn wander_is_seeded_and_stays_inside() {
        let task = || Task {
            name: "w".into(),
            legs: vec![Leg::Wander {
                seconds: 30.0,
                seed: 99,
            }],
            expect: Expect::default(),
            note: String::new(),
        };
        let a = simulate(task(), 40.0, &[]);
        let b = simulate(task(), 40.0, &[]);
        assert_eq!(a, b);
        assert!(a.moved_m > 1.0);
    }

    #[test]
    fn counts_and_failures() {
        let mut c = Counts::default();
        c.bear_event("obstacle:hit:rolling_stones:#12", false);
        c.bear_event("obstacle:hit:rolling_stones:#13", false);
        c.bear_event("grab", false);
        c.bear_event("back_on_ground:0.40m", true);
        c.bear_event("back_on_ground:0.26m", false);
        assert_eq!(c.obstacles["hit:rolling_stones"], 2);
        assert_eq!(c.after_teleport, 1);
        assert_eq!(c.small_put_back, 1);
        assert!(!c.fell_through());
        let mut report = AutoReport {
            mode: Mode::Tour,
            seeds: vec![SeedResult {
                seed: 7,
                ran: true,
                counts: c.clone(),
                ..Default::default()
            }],
            cancelled: false,
            headless: true,
            wall_seconds: 10.0,
        };
        assert!(report.passed());
        assert!(report.markdown().contains("**pass**"));
        report.seeds[0]
            .counts
            .bear_event("back_on_ground:0.50m", false);
        assert!(!report.passed());
        assert!(report.markdown().contains("**fail**"));
        report.seeds.push(SeedResult {
            seed: 42,
            ..Default::default()
        });
        assert_eq!(report.failures().len(), 2);
    }

    #[test]
    fn smoke_path_avoids_the_wall() {
        // The wall zone is steep at x = 10 to 17 m (angle 0) and flat at
        // -x (angle pi).
        let h = wall_zone();
        let fronts = [(0.0, 14.0), (std::f32::consts::PI, 14.0)];
        let (a, front, flat) = smoke_path(&h, &fronts).unwrap();
        assert!(flat && (a - std::f32::consts::PI).abs() < 1e-6 && front == 14.0);
        let steep = [(0.0, 14.0)];
        assert_eq!(smoke_path(&h, &steep), Some((0.0, 14.0, false)));
        assert_eq!(smoke_path(&h, &[]), None);
        assert_eq!(smoke_angles(7).len(), SMOKE_ANGLES);
    }

    #[test]
    fn wander_tasks_cover_the_minutes() {
        assert_eq!(wander_tasks(2.0).len(), 3);
        assert_eq!(wander_tasks(0.1).len(), 1);
    }
}
