//! The obstacles in motion (R23). This module has no Godot types.
//!
//! `ObstacleSim` moves the obstacles of one zone in each physics step.
//! It reads the place of the bear and tells what the obstacles do to
//! the bear: a wind, a knock, wet fur. It also gives the events for the
//! playtest recorder and the puffs of dust and steam for the screen.
//!
//! Each obstacle has a tell before it acts: the crust shakes, a stone
//! shakes at the top of its gully, dust moves in before a gust, and a
//! vent makes more and more steam before it blows. A stepping stone
//! blinks before it vanishes, and warm air rises before an updraft gets
//! strong. Sinking sand has ripples.

use bf_worldgen::obstacles::{Kind, Obstacle};
use bf_worldgen::HeightMap;
use glam::{Vec2, Vec3};

/// The tuning values of the obstacles.
#[derive(Clone, Debug, PartialEq)]
pub struct SimParams {
    /// How long an easy crust shakes before it falls, in seconds.
    pub crust_shake_easy_s: f32,
    /// How long a hard crust shakes before it falls, in seconds.
    pub crust_shake_hard_s: f32,
    /// How long a stone shakes at the top before it rolls, in seconds.
    pub stone_tell_s: f32,
    /// The time between two stones of one wave, in seconds.
    pub stone_gap_s: f32,
    /// The gravity on a stone in m/s each second.
    pub stone_gravity: f32,
    /// The rolling friction of a stone (0.06: it rolls far).
    pub stone_friction: f32,
    /// The highest speed of a stone in m/s.
    pub stone_max_speed: f32,
    /// The part of the speed of a stone that the bear gets in a hit.
    pub stone_knock_part: f32,
    /// The speed up that the bear gets in a hit, in m/s.
    pub stone_knock_up: f32,
    /// Stones start only if the bear is nearer than this, in metres.
    pub stone_wake_m: f32,
    /// The wind speed of an easy gust in m/s.
    pub gust_speed_easy: f32,
    /// The wind speed of a hard gust in m/s.
    pub gust_speed_hard: f32,
    /// How long a gust blows in each cycle, in seconds.
    pub gust_blow_s: f32,
    /// How long dust shows before a gust blows, in seconds.
    pub gust_tell_s: f32,
    /// The wind on a bear in the air is multiplied by this value.
    pub gust_air_factor: f32,
    /// How long a vent makes steam before it blows, in seconds.
    pub vent_tell_s: f32,
    /// How long a vent blows, in seconds.
    pub vent_blast_s: f32,
    /// The speed to the side that an easy push vent gives, in m/s.
    pub vent_push_easy: f32,
    /// The speed to the side that a hard push vent gives, in m/s.
    pub vent_push_hard: f32,
    /// The speed up that a push vent gives, in m/s.
    pub vent_push_up: f32,
    /// The bear must be this near to a launch vent, in metres.
    pub vent_radius: f32,
    /// A launch goes this far above the ledge, in metres.
    pub launch_margin_m: f32,
    /// The gravity on the bear while it goes up, in m/s each second.
    pub bear_gravity: f32,
    /// The gravity on the bear is multiplied by this while it falls.
    pub bear_fall_factor: f32,
    /// Puffs show only for obstacles nearer than this, in metres.
    pub puff_range_m: f32,
    /// The part of a cycle in which an easy stepping stone is solid.
    pub step_on_easy: f32,
    /// The part of a cycle in which a hard stepping stone is solid.
    pub step_on_hard: f32,
    /// How long a stepping stone blinks before it vanishes, in seconds.
    pub step_tell_s: f32,
    /// How long a stepping stone shows as a ghost before it is solid, in
    /// seconds.
    pub step_ghost_s: f32,
    /// The speed up of the air in an updraft at full strength, in m/s.
    pub updraft_speed: f32,
    /// How fast the air in an updraft changes the speed of the bear, in
    /// m/s each second. It must be more than the gravity to lift the bear.
    pub updraft_accel: f32,
    /// The part of a cycle in which an updraft is strong.
    pub updraft_strong_part: f32,
    /// The strength of an updraft between its strong phases (0 to 1).
    pub updraft_weak: f32,
    /// The lift is full up to this part of the height of the column. Above
    /// it, the lift goes to 0 at the top.
    pub updraft_fade_part: f32,
    /// How fast an easy patch of sand sinks under a standing bear, in
    /// m/s.
    pub sand_sink_easy: f32,
    /// How fast a hard patch of sand sinks under a standing bear, in m/s.
    pub sand_sink_hard: f32,
    /// A bear that walks makes the sink this part as fast (0 to 1).
    pub sand_walk_part: f32,
    /// How fast sand comes back up when nobody stands on it, in m/s.
    pub sand_rise: f32,
    /// How deep the bear can sink, in metres. The bear is 0.4 m tall.
    pub sand_max_sink_m: f32,
    /// The slow down at once on sand (0 to 1).
    pub sand_slow_base: f32,
    /// The slow down at the largest sink (0 to 1).
    pub sand_slow_max: f32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            crust_shake_easy_s: 0.9,
            crust_shake_hard_s: 0.45,
            stone_tell_s: 1.0,
            stone_gap_s: 0.9,
            stone_gravity: 9.8,
            stone_friction: 0.06,
            stone_max_speed: 9.0,
            stone_knock_part: 0.9,
            stone_knock_up: 2.2,
            stone_wake_m: 90.0,
            gust_speed_easy: 1.2,
            gust_speed_hard: 2.6,
            gust_blow_s: 2.2,
            gust_tell_s: 1.2,
            gust_air_factor: 1.5,
            vent_tell_s: 1.0,
            vent_blast_s: 0.45,
            vent_push_easy: 3.0,
            vent_push_hard: 5.5,
            vent_push_up: 2.5,
            vent_radius: 0.45,
            launch_margin_m: 0.9,
            bear_gravity: 14.0,
            bear_fall_factor: 1.4,
            puff_range_m: 140.0,
            step_on_easy: 0.75,
            step_on_hard: 0.66,
            step_tell_s: 0.7,
            step_ghost_s: 0.5,
            updraft_speed: 2.2,
            updraft_accel: 34.0,
            updraft_strong_part: 0.62,
            updraft_weak: 0.3,
            updraft_fade_part: 0.55,
            sand_sink_easy: 0.09,
            sand_sink_hard: 0.17,
            sand_walk_part: 0.4,
            sand_rise: 0.2,
            sand_max_sink_m: 0.3,
            sand_slow_base: 0.12,
            sand_slow_max: 0.65,
        }
    }
}

/// The distance from the feet of the bear to its centre in metres.
const BEAR_CENTRE: f32 = 0.2;
/// The radius of the bear for a hit by a stone in metres.
const BEAR_RADIUS: f32 = 0.17;

/// What the simulation reads from the bear.
#[derive(Clone, Copy, Debug, Default)]
pub struct BearIn {
    /// The place of the feet.
    pub feet: Vec3,
    pub on_floor: bool,
    /// The bear is on a wall.
    pub climbing: bool,
    /// The speed on the ground plane in m/s.
    pub speed: f32,
}

/// The look of a puff.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PuffKind {
    /// Pale dust: gusts, stones, the crust.
    Dust,
    /// Dirty steam: vents.
    Steam,
    /// Pale, warm air over hot rock: updrafts.
    Heat,
}

/// A request for one puff of dust or steam on the screen.
#[derive(Clone, Copy, Debug)]
pub struct Puff {
    pub pos: Vec3,
    pub vel: Vec3,
    /// The size at the start in metres.
    pub size: f32,
    /// The time that the puff lives in seconds.
    pub life: f32,
    pub kind: PuffKind,
}

/// What one step of the simulation gives.
#[derive(Clone, Debug, Default)]
pub struct StepOut {
    /// The wind on the bear: a velocity that carries it, in m/s.
    pub wind: Vec3,
    /// A hit or a launch: the new velocity of the bear.
    pub knock: Option<Vec3>,
    /// Dirty steam hit the bear.
    pub wet: bool,
    /// Events for the playtest recorder, for example
    /// "obstacle:hit:rolling_stones:#12".
    pub events: Vec<String>,
    pub puffs: Vec<Puff>,
    /// The obstacles (index) whose crust fell in this step.
    pub broke: Vec<usize>,
    /// The air lifts the bear: the speed up that it goes to and how fast
    /// it changes, in m/s and m/s each second. Only for a bear in the air.
    pub lift: Option<(f32, f32)>,
    /// The sand slows the bear on the ground: 0 (no slow down) to 1.
    pub slow: f32,
}

/// The state of a stepping stone at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepPhase {
    /// The stone is solid.
    Solid,
    /// The stone blinks and vanishes soon. `part` goes from 0 to 1.
    Tell { part: f32 },
    /// The stone is not there.
    Gone,
    /// The stone shows as a ghost and gets solid soon. `part` goes from
    /// 0 to 1.
    Ghost { part: f32 },
}

impl StepPhase {
    /// Tells if the bear can stand on the stone.
    pub fn solid(self) -> bool {
        matches!(self, StepPhase::Solid | StepPhase::Tell { .. })
    }
}

/// Gives the state of stone `k` of a stepping-stone path at a time.
///
/// The stones of the path take turns: the stones with an even number are
/// solid in the first part of the cycle. The stones with an odd number
/// start half a cycle later. Thus, two stones next to each other are
/// solid together for a short time. A bear can hop from one to the other
/// then. The rhythm is the same in all directions.
pub fn step_phase(o: &Obstacle, k: u32, time: f64, p: &SimParams) -> StepPhase {
    let period = f64::from(o.period.max(1.0));
    let shift = if k % 2 == 1 { period * 0.5 } else { 0.0 };
    let t = (time + f64::from(o.phase) - shift).rem_euclid(period) as f32;
    let period = period as f32;
    let on_part = p.step_on_easy + (p.step_on_hard - p.step_on_easy) * o.difficulty;
    let on = (period * on_part.clamp(0.5, 0.95)).min(period - p.step_ghost_s - 0.2);
    let tell = p.step_tell_s.clamp(0.0, on - 0.2);
    if t < on - tell {
        StepPhase::Solid
    } else if t < on {
        StepPhase::Tell {
            part: (t - (on - tell)) / tell.max(0.01),
        }
    } else if t < period - p.step_ghost_s {
        StepPhase::Gone
    } else {
        StepPhase::Ghost {
            part: (t - (period - p.step_ghost_s)) / p.step_ghost_s.max(0.01),
        }
    }
}

/// Gives the strength of an updraft at a time, from 0 to 1. It is strong
/// in the first part of its cycle and weak in the rest.
pub fn updraft_strength(o: &Obstacle, time: f64, p: &SimParams) -> f32 {
    let period = f64::from(o.period.max(1.0));
    let t = (time + f64::from(o.phase)).rem_euclid(period) as f32;
    let period = period as f32;
    let strong = (period * p.updraft_strong_part).clamp(1.0, period - 0.5);
    let weak = p.updraft_weak.clamp(0.0, 1.0);
    let up = smoothstep(0.0, 0.8, t);
    let down = 1.0 - smoothstep(strong - 0.5, strong, t);
    weak + (1.0 - weak) * up * down
}

/// The state of an ash crust.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Crust {
    Intact,
    /// The crust shakes. `left` is the time until it falls.
    Shaking {
        left: f32,
        total: f32,
    },
    Broken,
}

/// One stone of a rolling-stones obstacle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stone {
    /// The index of the obstacle that made the stone.
    pub owner: usize,
    /// The centre of the stone.
    pub pos: Vec3,
    /// The velocity on the ground plane (x, z).
    pub vel: Vec2,
    pub radius: f32,
    /// The time until the stone starts to roll. More than 0: it shakes.
    pub wait: f32,
    /// How far the stone turned, in radians.
    pub roll: f32,
    travelled: f32,
    age: f32,
    /// The time until the stone can hit the bear again.
    cooldown: f32,
}

/// A launch that waits for its result.
#[derive(Clone, Copy, Debug)]
struct Launch {
    id: u32,
    /// The height that counts as "on the ledge".
    reached_y: f32,
    time: f32,
}

/// What the bear does at a stepping-stone path.
#[derive(Clone, Copy, Debug, Default)]
struct StepTrack {
    /// The bear is in the area of the trench.
    inside: bool,
    /// The side of the trench that the bear came from: 1 or -1.
    from_side: f32,
    /// The bear fell into the trench in this visit.
    fell: bool,
    /// The bear is near the path (for the "tell" event).
    near: bool,
}

/// What the bear does on a patch of sinking sand.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum SandVisit {
    #[default]
    Away,
    /// The bear stands on the sand.
    On,
    /// The sink is full.
    Stuck,
}

/// Gives the velocity (up, forward) that throws the bear from a launch
/// vent onto a ledge. The ledge is `rise` metres up and `reach` metres
/// away. The top of the flight is `margin` metres above the ledge.
pub fn launch_velocity(rise: f32, reach: f32, margin: f32, gravity: f32, fall: f32) -> (f32, f32) {
    let gravity = gravity.max(0.1);
    let margin = margin.max(0.1);
    let up = (2.0 * gravity * (rise.max(0.0) + margin)).sqrt();
    let time_up = up / gravity;
    let time_down = (2.0 * margin / (gravity * fall.max(0.1))).sqrt();
    (up, reach / (time_up + time_down))
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn xz(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

fn dir3(d: [f32; 2]) -> Vec3 {
    Vec3::new(d[0], 0.0, d[1])
}

/// The obstacles of one zone, in motion.
pub struct ObstacleSim {
    obstacles: Vec<Obstacle>,
    crusts: Vec<Crust>,
    /// For each obstacle: the cycle in which it last acted on the bear.
    acted: Vec<i64>,
    /// For each obstacle: the cycle of its stones, and how many stones
    /// of this cycle exist.
    wave: Vec<(i64, u32)>,
    /// For each obstacle: the part of a puff that is left to make.
    puff_due: Vec<f32>,
    stones: Vec<Stone>,
    launch: Option<Launch>,
    /// For each obstacle: how deep the sand sank, in metres.
    sink: Vec<f32>,
    sand_visit: Vec<SandVisit>,
    steps: Vec<StepTrack>,
    time: f64,
    random: u32,
}

impl ObstacleSim {
    pub fn new(obstacles: Vec<Obstacle>) -> Self {
        let n = obstacles.len();
        Self {
            obstacles,
            crusts: vec![Crust::Intact; n],
            acted: vec![i64::MIN; n],
            wave: vec![(i64::MIN, 0); n],
            puff_due: vec![0.0; n],
            stones: Vec::new(),
            launch: None,
            sink: vec![0.0; n],
            sand_visit: vec![SandVisit::Away; n],
            steps: vec![StepTrack::default(); n],
            time: 0.0,
            random: 0x2F6E_2B1D,
        }
    }

    pub fn obstacles(&self) -> &[Obstacle] {
        &self.obstacles
    }

    pub fn stones(&self) -> &[Stone] {
        &self.stones
    }

    /// Gives how deep the sand of an obstacle sank, in metres.
    pub fn sink(&self, index: usize) -> f32 {
        self.sink.get(index).copied().unwrap_or(0.0)
    }

    /// Gives the time of the simulation in seconds.
    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn crust(&self, index: usize) -> Crust {
        self.crusts.get(index).copied().unwrap_or(Crust::Broken)
    }

    /// A random number from 0 to 1. It is for the look only (puffs).
    fn rand(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        (self.random >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// A random number from -1 to 1.
    fn rand2(&mut self) -> f32 {
        self.rand() * 2.0 - 1.0
    }

    /// Tells how many puffs an obstacle makes in this step, for a rate
    /// in puffs per second.
    fn puffs_due(&mut self, index: usize, rate: f32, dt: f32) -> u32 {
        let due = self.puff_due[index] + rate * dt;
        let whole = due.floor();
        self.puff_due[index] = due - whole;
        whole as u32
    }

    /// Moves all obstacles by one step.
    pub fn step(&mut self, dt: f32, bear: &BearIn, heights: &HeightMap, p: &SimParams) -> StepOut {
        self.time += f64::from(dt);
        let mut out = StepOut::default();
        let centre = bear.feet + Vec3::Y * BEAR_CENTRE;

        // The list moves out of `self` for the loop: the loop reads the
        // obstacles and changes the other fields.
        let obstacles = std::mem::take(&mut self.obstacles);
        let asleep_m = p.puff_range_m.max(p.stone_wake_m) + 30.0;
        for (i, o) in obstacles.iter().enumerate() {
            let pos = Vec3::from_array(o.pos);
            let near = xz(bear.feet).distance(xz(pos));
            // An obstacle far from the bear does nothing.
            if near > asleep_m {
                continue;
            }
            let shows = near <= p.puff_range_m;
            // The place in the cycle: the number of the cycle and the
            // time since its start.
            let period = f64::from(o.period.max(0.5));
            let clock = self.time + f64::from(o.phase);
            let cycle = (clock / period).floor() as i64;
            let t = (clock - cycle as f64 * period) as f32;
            let period = period as f32;
            let name = o.kind.name();

            match &o.kind {
                Kind::AshCrust { radius, lid, .. } => {
                    let lid_y = lid.height(bear.feet.x, bear.feet.z);
                    let above = (bear.feet.y - lid_y).abs();
                    match self.crusts[i] {
                        Crust::Intact => {
                            if bear.on_floor && near <= radius - 0.3 && above <= 0.35 {
                                let total = p.crust_shake_easy_s
                                    + (p.crust_shake_hard_s - p.crust_shake_easy_s) * o.difficulty;
                                let total = total.max(0.05);
                                self.crusts[i] = Crust::Shaking { left: total, total };
                                out.events.push(format!("obstacle:tell:{name}:#{}", o.id));
                            }
                        }
                        Crust::Shaking { left, total } => {
                            let left = left - dt;
                            for _ in 0..self.puffs_due(i, 25.0, dt) {
                                let at = Vec2::new(self.rand2(), self.rand2()) * radius * 0.7;
                                let x = pos.x + at.x;
                                let z = pos.z + at.y;
                                out.puffs.push(Puff {
                                    pos: Vec3::new(x, lid.height(x, z) + 0.05, z),
                                    vel: Vec3::new(0.0, 0.5 + 0.4 * self.rand(), 0.0),
                                    size: 0.25,
                                    life: 0.6,
                                    kind: PuffKind::Dust,
                                });
                            }
                            if left > 0.0 {
                                self.crusts[i] = Crust::Shaking { left, total };
                            } else {
                                self.crusts[i] = Crust::Broken;
                                out.broke.push(i);
                                let on_it = near <= radius + 0.2 && above <= 0.6;
                                let what = if on_it { "fall" } else { "escape" };
                                out.events.push(format!("obstacle:{what}:{name}:#{}", o.id));
                                for _ in 0..28 {
                                    let at = Vec2::new(self.rand2(), self.rand2()) * *radius;
                                    out.puffs.push(Puff {
                                        pos: pos + Vec3::new(at.x, 0.1, at.y),
                                        vel: Vec3::new(at.x * 0.4, 0.6 + self.rand(), at.y * 0.4),
                                        size: 0.6 + 0.5 * self.rand(),
                                        life: 1.4,
                                        kind: PuffKind::Dust,
                                    });
                                }
                            }
                        }
                        Crust::Broken => {}
                    }
                }

                Kind::RollingStones { radius, count, .. } => {
                    let (wave_cycle, made) = self.wave[i];
                    let awake = near <= p.stone_wake_m;
                    let gap = p.stone_gap_s.max(0.1);
                    // The stones of this wave whose time has come.
                    let due = ((t / gap).floor() as u32 + 1).min(*count);
                    if wave_cycle != cycle {
                        // A new wave. The bear must be near from its
                        // start, or the wave does not come.
                        let skip = if awake && wave_cycle == cycle - 1 {
                            0
                        } else {
                            *count
                        };
                        self.wave[i] = (cycle, skip);
                    } else if awake && made < due {
                        let side = Vec3::new(-o.dir[1], 0.0, o.dir[0]);
                        let offset = (made as f32 - (*count as f32 - 1.0) * 0.5) * radius * 2.4;
                        let at = pos + side * offset;
                        let ground = heights.sample_world(at.x, at.z);
                        self.stones.push(Stone {
                            owner: i,
                            pos: Vec3::new(at.x, ground + radius, at.z),
                            vel: Vec2::ZERO,
                            radius: *radius,
                            wait: p.stone_tell_s.max(0.0),
                            roll: 0.0,
                            travelled: 0.0,
                            age: 0.0,
                            cooldown: 0.0,
                        });
                        self.wave[i].1 = made + 1;
                        if made == 0 {
                            out.events.push(format!("obstacle:tell:{name}:#{}", o.id));
                        }
                    }
                }

                Kind::Gust {
                    half_length,
                    half_width,
                } => {
                    let blow = p.gust_blow_s.clamp(0.2, period * 0.8);
                    let blow_start = period - blow;
                    let tell_start = blow_start - p.gust_tell_s.max(0.0);
                    let speed =
                        p.gust_speed_easy + (p.gust_speed_hard - p.gust_speed_easy) * o.difficulty;
                    let ramp = smoothstep(blow_start, blow_start + 0.4, t)
                        * (1.0 - smoothstep(period - 0.4, period, t));
                    let dir = dir3(o.dir);
                    let side = Vec3::new(-o.dir[1], 0.0, o.dir[0]);
                    let rel = bear.feet - pos;
                    let along = rel.dot(dir).abs();
                    let across = rel.dot(side).abs();
                    let inside = along <= *half_length
                        && across <= *half_width
                        && rel.y > -3.0
                        && rel.y < 6.0;
                    if inside && ramp > 0.0 && !bear.climbing {
                        let edge = (1.0 - smoothstep(half_length * 0.8, *half_length, along))
                            * (1.0 - smoothstep(half_width * 0.7, *half_width, across));
                        let air = if bear.on_floor {
                            1.0
                        } else {
                            p.gust_air_factor
                        };
                        out.wind += dir * speed * ramp * edge * air;
                        if ramp * edge > 0.5 && self.acted[i] != cycle {
                            self.acted[i] = cycle;
                            out.events.push(format!("obstacle:push:{name}:#{}", o.id));
                        }
                    }
                    if shows && t >= tell_start {
                        // Dust comes in at the end where the wind starts.
                        let rate = if t >= blow_start { 34.0 } else { 9.0 };
                        let puff_speed = if t >= blow_start {
                            speed * 2.2
                        } else {
                            speed * 0.5
                        };
                        for _ in 0..self.puffs_due(i, rate, dt) {
                            let back = if t >= blow_start {
                                -half_length * (0.2 + 0.8 * self.rand())
                            } else {
                                -half_length * (0.8 + 0.3 * self.rand())
                            };
                            let at = pos + dir * back + side * (self.rand2() * half_width * 0.9);
                            let ground = heights.sample_world(at.x, at.z);
                            let life = 1.0 + 0.6 * self.rand();
                            out.puffs.push(Puff {
                                pos: Vec3::new(at.x, ground + 0.1 + 0.9 * self.rand(), at.z),
                                vel: dir * puff_speed
                                    + Vec3::new(0.0, 0.25 * self.rand2(), 0.0)
                                    + side * (0.3 * self.rand2()),
                                size: 0.5 + 0.5 * self.rand(),
                                life,
                                kind: PuffKind::Dust,
                            });
                        }
                    }
                }

                Kind::SteppingStones {
                    count,
                    spacing,
                    radius,
                    half_length,
                    half_width,
                    ys,
                    ..
                } => {
                    let dir = dir3(o.dir);
                    let side = Vec3::new(-o.dir[1], 0.0, o.dir[0]);
                    let rel = bear.feet - pos;
                    let across = rel.dot(dir);
                    let along = rel.dot(side).abs();
                    let in_box = along <= *half_length
                        && across.abs() <= half_width + 1.2
                        && rel.y > -4.0
                        && rel.y < 3.0;
                    let floor_y = ys.first().copied().unwrap_or(pos.y);
                    let mut track = self.steps[i];
                    let close = along <= half_length + 6.0 && across.abs() <= half_width + 8.0;
                    if close && !track.near {
                        out.events.push(format!("obstacle:tell:{name}:#{}", o.id));
                    }
                    track.near = close;
                    if in_box && !track.inside {
                        track.from_side = across.signum();
                        track.fell = false;
                    }
                    if !in_box && track.inside && !track.fell && across.signum() != track.from_side
                    {
                        out.events.push(format!("obstacle:cross:{name}:#{}", o.id));
                    }
                    if in_box && !track.fell && bear.feet.y < floor_y - 0.8 {
                        track.fell = true;
                        out.events.push(format!("obstacle:fall:{name}:#{}", o.id));
                    }
                    track.inside = in_box;
                    self.steps[i] = track;

                    if shows {
                        // Dust at the stones that blink and at the ghosts.
                        let mut marks: Vec<(Vec3, f32)> = Vec::new();
                        for k in 0..*count {
                            let off = (k as f32 - (*count as f32 - 1.0) * 0.5) * spacing;
                            let at = pos + dir * off;
                            let y = ys.get(k as usize).copied().unwrap_or(pos.y);
                            match step_phase(o, k, self.time, p) {
                                StepPhase::Tell { part } => {
                                    marks.push((Vec3::new(at.x, y, at.z), 12.0 + 16.0 * part))
                                }
                                StepPhase::Ghost { .. } => {
                                    marks.push((Vec3::new(at.x, y, at.z), 6.0))
                                }
                                _ => {}
                            }
                        }
                        let rate: f32 = marks.iter().map(|m| m.1).sum();
                        for n in 0..self.puffs_due(i, rate, dt) as usize {
                            let (at, _) = marks[n % marks.len().max(1)];
                            let wobble = Vec2::new(self.rand2(), self.rand2()) * radius * 0.8;
                            out.puffs.push(Puff {
                                pos: at + Vec3::new(wobble.x, 0.05, wobble.y),
                                vel: Vec3::new(0.0, 0.4 + 0.3 * self.rand(), 0.0),
                                size: 0.22,
                                life: 0.6,
                                kind: PuffKind::Dust,
                            });
                        }
                    }
                }

                Kind::Updraft { radius, height } => {
                    let strength = updraft_strength(o, self.time, p);
                    let rel = bear.feet - pos;
                    let r = xz(rel).length();
                    let edge = 1.0 - smoothstep(radius * 0.6, *radius, r);
                    let fade = 1.0 - smoothstep(height * p.updraft_fade_part, *height, rel.y);
                    if !bear.on_floor
                        && !bear.climbing
                        && edge > 0.0
                        && rel.y > -0.6
                        && rel.y < *height
                    {
                        let speed = p.updraft_speed * strength * edge * fade;
                        let best = out.lift.map_or(0.0, |l| l.0);
                        if speed > best {
                            out.lift = Some((speed, p.updraft_accel));
                        }
                        if self.acted[i] != cycle {
                            self.acted[i] = cycle;
                            out.events.push(format!("obstacle:ride:{name}:#{}", o.id));
                        }
                    }
                    if shows {
                        // Warm air rises: more puffs when it is strong.
                        let rate = 2.0 + 14.0 * strength * strength;
                        for _ in 0..self.puffs_due(i, rate, dt) {
                            let at = Vec2::new(self.rand2(), self.rand2()) * radius * 0.7;
                            let ground = heights.sample_world(pos.x + at.x, pos.z + at.y);
                            out.puffs.push(Puff {
                                pos: Vec3::new(pos.x + at.x, ground + 0.1, pos.z + at.y),
                                vel: Vec3::new(0.0, 1.2 + 1.6 * strength, 0.0),
                                size: 0.5 + 0.3 * self.rand(),
                                life: 0.9 + 0.5 * self.rand(),
                                kind: PuffKind::Heat,
                            });
                        }
                    }
                }

                Kind::SinkingSand { radius, lid, .. } => {
                    let lid_y = lid.height(bear.feet.x, bear.feet.z);
                    let sunk = self.sink[i];
                    let above = bear.feet.y - (lid_y - sunk);
                    let on = bear.on_floor && near <= radius - 0.2 && (-0.2..=0.4).contains(&above);
                    let rate =
                        p.sand_sink_easy + (p.sand_sink_hard - p.sand_sink_easy) * o.difficulty;
                    let max = p.sand_max_sink_m.max(0.01);
                    let moving = bear.speed > 0.3;
                    let now = if on {
                        let part = if moving { p.sand_walk_part } else { 1.0 };
                        let now = (sunk + rate * part * dt).min(max);
                        let full = now / max;
                        let slow = p.sand_slow_base + (p.sand_slow_max - p.sand_slow_base) * full;
                        out.slow = out.slow.max(slow.clamp(0.0, 0.95));
                        now
                    } else {
                        (sunk - p.sand_rise * dt).max(0.0)
                    };
                    self.sink[i] = now;
                    // Events: the bear steps on, gets stuck, or gets out.
                    let visit = match (on, now >= max - 0.005) {
                        (false, _) => SandVisit::Away,
                        (true, false) => SandVisit::On,
                        (true, true) => SandVisit::Stuck,
                    };
                    let before = self.sand_visit[i];
                    if visit != before {
                        match (before, visit) {
                            (SandVisit::Away, _) => {
                                out.events.push(format!("obstacle:sink:{name}:#{}", o.id));
                            }
                            (_, SandVisit::Stuck) => {
                                out.events.push(format!("obstacle:stuck:{name}:#{}", o.id));
                            }
                            (_, SandVisit::Away) if sunk > max * 0.3 => {
                                out.events.push(format!("obstacle:escape:{name}:#{}", o.id));
                            }
                            _ => {}
                        }
                        // Stuck to On (the sink went down a little) is not new.
                        if !(before == SandVisit::Stuck && visit == SandVisit::On) {
                            self.sand_visit[i] = visit;
                        }
                    }
                    if shows && on && moving {
                        for _ in 0..self.puffs_due(i, 7.0, dt) {
                            out.puffs.push(Puff {
                                pos: bear.feet + Vec3::Y * 0.05,
                                vel: Vec3::new(self.rand2() * 0.3, 0.3, self.rand2() * 0.3),
                                size: 0.16,
                                life: 0.5,
                                kind: PuffKind::Dust,
                            });
                        }
                    }
                }

                Kind::VentPush { .. } | Kind::VentLaunch { .. } => {
                    let blast = p.vent_blast_s.clamp(0.05, period * 0.5);
                    let blast_start = period - blast;
                    let tell_start = blast_start - p.vent_tell_s.max(0.0);
                    let dir = dir3(o.dir);
                    let rel = centre - pos;
                    let push_speed =
                        p.vent_push_easy + (p.vent_push_hard - p.vent_push_easy) * o.difficulty;
                    // The direction and the speed of the steam.
                    let (jet, jet_speed) = match o.kind {
                        Kind::VentPush { .. } => ((dir + Vec3::Y * 0.35).normalize(), push_speed),
                        _ => (Vec3::Y, 7.0),
                    };
                    if t >= blast_start && self.acted[i] != cycle {
                        match o.kind {
                            Kind::VentPush { range } => {
                                let along = rel.dot(dir);
                                let across = (xz(rel) - xz(dir) * along).length();
                                if along >= -0.3
                                    && along <= range
                                    && across <= 0.35 + 0.25 * along.max(0.0)
                                    && rel.y.abs() <= 0.9
                                {
                                    self.acted[i] = cycle;
                                    out.knock = Some(dir * push_speed + Vec3::Y * p.vent_push_up);
                                    out.wet = true;
                                    out.events.push(format!("obstacle:hit:{name}:#{}", o.id));
                                }
                            }
                            Kind::VentLaunch { rise, reach }
                                if xz(rel).length() <= p.vent_radius
                                    && rel.y > -0.3
                                    && rel.y < 1.0 =>
                            {
                                self.acted[i] = cycle;
                                let (up, forward) = launch_velocity(
                                    rise,
                                    reach,
                                    p.launch_margin_m,
                                    p.bear_gravity,
                                    p.bear_fall_factor,
                                );
                                out.knock = Some(dir * forward + Vec3::Y * up);
                                out.wet = true;
                                out.events.push(format!("obstacle:launch:{name}:#{}", o.id));
                                self.launch = Some(Launch {
                                    id: o.id,
                                    reached_y: pos.y + rise - 0.6,
                                    time: 0.0,
                                });
                            }
                            _ => {}
                        }
                    }
                    if shows {
                        // A little steam always. More and more before
                        // the blast. A jet in the blast.
                        let (rate, speed, size) = if t >= blast_start {
                            (70.0, jet_speed, 0.45)
                        } else if t >= tell_start {
                            let k = (t - tell_start) / (blast_start - tell_start).max(0.01);
                            (5.0 + 22.0 * k, 0.5 + 0.8 * k, 0.16 + 0.12 * k)
                        } else {
                            (1.2, 0.3, 0.14)
                        };
                        for _ in 0..self.puffs_due(i, rate, dt) {
                            let wobble =
                                Vec3::new(self.rand2(), self.rand2() * 0.3, self.rand2()) * 0.12;
                            out.puffs.push(Puff {
                                pos: pos + Vec3::Y * 0.06 + wobble * 0.5,
                                vel: (jet + wobble) * speed * (0.7 + 0.5 * self.rand()),
                                size,
                                life: 0.5 + 0.4 * self.rand(),
                                kind: PuffKind::Steam,
                            });
                        }
                    }
                }
            }
        }

        self.obstacles = obstacles;
        self.step_stones(dt, centre, bear, heights, p, &mut out);
        self.step_launch(dt, bear, &mut out);
        out
    }

    /// Moves the stones. A stone that hits the bear knocks it away.
    fn step_stones(
        &mut self,
        dt: f32,
        centre: Vec3,
        bear: &BearIn,
        heights: &HeightMap,
        p: &SimParams,
        out: &mut StepOut,
    ) {
        let half = heights.world_size() * 0.5 - 2.0;
        let mut stones = std::mem::take(&mut self.stones);
        stones.retain_mut(|s| {
            let o = &self.obstacles[s.owner];
            let Kind::RollingStones { run_m, .. } = o.kind else {
                return false;
            };
            let (owner, id, dir) = (s.owner, o.id, o.dir);
            let shows = xz(bear.feet).distance(xz(s.pos)) <= p.puff_range_m;
            s.cooldown = (s.cooldown - dt).max(0.0);
            if s.wait > 0.0 {
                // The tell: the stone shakes, and dust comes up.
                s.wait -= dt;
                if shows {
                    for _ in 0..self.puffs_due(owner, 12.0, dt) {
                        let vel = Vec3::new(self.rand2() * 0.4, 0.5, self.rand2() * 0.4);
                        out.puffs.push(Puff {
                            pos: s.pos - Vec3::Y * s.radius * 0.6,
                            vel,
                            size: 0.3,
                            life: 0.7,
                            kind: PuffKind::Dust,
                        });
                    }
                }
                if s.wait <= 0.0 {
                    s.vel = Vec2::from_array(dir) * 0.8;
                }
                return true;
            }

            // The slope pulls the stone. The friction slows it.
            let h = |x: f32, z: f32| heights.sample_world(x, z);
            let (x, z) = (s.pos.x, s.pos.z);
            let grad = Vec2::new(h(x + 0.5, z) - h(x - 0.5, z), h(x, z + 0.5) - h(x, z - 0.5));
            let mut accel = -grad * p.stone_gravity / (1.0 + grad.length_squared());
            accel -= s.vel.normalize_or_zero() * p.stone_friction * p.stone_gravity;
            s.vel = (s.vel + accel * dt).clamp_length_max(p.stone_max_speed);
            let speed = s.vel.length();
            let (x, z) = (x + s.vel.x * dt, z + s.vel.y * dt);
            s.pos = Vec3::new(x, h(x, z) + s.radius, z);
            s.roll += speed * dt / s.radius.max(0.01);
            s.travelled += speed * dt;
            s.age += dt;

            if s.cooldown <= 0.0 && speed >= 0.8 && centre.distance(s.pos) <= s.radius + BEAR_RADIUS
            {
                s.cooldown = 1.0;
                let flat = Vec3::new(s.vel.x, 0.0, s.vel.y);
                out.knock = Some(flat * p.stone_knock_part + Vec3::Y * p.stone_knock_up);
                out.events
                    .push(format!("obstacle:hit:rolling_stones:#{id}"));
            }
            if shows && speed > 2.0 {
                for _ in 0..self.puffs_due(owner, 8.0, dt) {
                    out.puffs.push(Puff {
                        pos: s.pos - Vec3::Y * s.radius * 0.8,
                        vel: Vec3::new(0.0, 0.4, 0.0),
                        size: s.radius * 1.6,
                        life: 0.6,
                        kind: PuffKind::Dust,
                    });
                }
            }

            let stopped = s.age > 1.5 && speed < 0.35;
            let done = stopped
                || s.travelled > run_m * 1.3
                || s.age > 14.0
                || x.abs() > half
                || z.abs() > half;
            if done && shows {
                // The stone breaks into dust.
                for _ in 0..6 {
                    let vel = Vec3::new(self.rand2(), 0.6 + self.rand(), self.rand2()) * 0.8;
                    out.puffs.push(Puff {
                        pos: s.pos,
                        vel,
                        size: s.radius * 2.5,
                        life: 0.9,
                        kind: PuffKind::Dust,
                    });
                }
            }
            !done
        });
        self.stones = stones;
    }

    /// Looks where the bear lands after a launch, and makes an event.
    fn step_launch(&mut self, dt: f32, bear: &BearIn, out: &mut StepOut) {
        let Some(mut launch) = self.launch else {
            return;
        };
        launch.time += dt;
        let result = if launch.time < 0.3 {
            None
        } else if bear.on_floor {
            Some(if bear.feet.y >= launch.reached_y {
                "launch_reached"
            } else {
                "launch_missed"
            })
        } else if bear.climbing {
            Some("launch_wall")
        } else if launch.time > 10.0 {
            Some("launch_lost")
        } else {
            None
        };
        if let Some(result) = result {
            out.events
                .push(format!("obstacle:{result}:vent_launch:#{}", launch.id));
            self.launch = None;
        } else {
            self.launch = Some(launch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_worldgen::obstacles::Lid;

    const DT: f32 = 1.0 / 60.0;

    fn obstacle(kind: Kind, period: f32) -> Obstacle {
        Obstacle {
            id: 5,
            kind,
            pos: [0.0, 0.0, 0.0],
            dir: [1.0, 0.0],
            period,
            phase: 0.0,
            difficulty: 0.0,
        }
    }

    fn flat() -> HeightMap {
        HeightMap::new(128, 1.0)
    }

    fn bear_at(x: f32, y: f32, z: f32) -> BearIn {
        BearIn {
            feet: Vec3::new(x, y, z),
            on_floor: true,
            climbing: false,
            speed: 0.0,
        }
    }

    /// Runs the simulation for some seconds. Gives all events and the
    /// largest wind along x.
    fn run(sim: &mut ObstacleSim, bear: &BearIn, map: &HeightMap, seconds: f32) -> StepOut {
        let p = SimParams::default();
        let mut all = StepOut::default();
        for _ in 0..(seconds / DT) as usize {
            let out = sim.step(DT, bear, map, &p);
            all.events.extend(out.events);
            all.broke.extend(out.broke);
            all.puffs.extend(out.puffs);
            all.wet |= out.wet;
            if out.wind.x.abs() > all.wind.x.abs() {
                all.wind = out.wind;
            }
            if out.knock.is_some() {
                all.knock = out.knock;
            }
            all.slow = all.slow.max(out.slow);
            if out.lift.is_some() {
                all.lift = out.lift;
            }
        }
        all
    }

    #[test]
    fn a_gust_shows_dust_first_and_then_pushes_along_its_direction() {
        let gust = Kind::Gust {
            half_length: 10.0,
            half_width: 4.0,
        };
        let map = flat();
        let p = SimParams::default();
        // Calm: 8 - 2.2 - 1.2 = 4.6 s. No wind and no dust.
        let mut sim = ObstacleSim::new(vec![obstacle(gust.clone(), 8.0)]);
        let calm = run(&mut sim, &bear_at(0.0, 0.0, 0.0), &map, 4.5);
        assert_eq!(calm.wind, Vec3::ZERO);
        assert!(calm.puffs.is_empty());
        // The tell: dust, but no wind yet.
        let tell = run(&mut sim, &bear_at(0.0, 0.0, 0.0), &map, 1.2);
        assert_eq!(tell.wind, Vec3::ZERO);
        assert!(!tell.puffs.is_empty());
        // The blow: the wind has the speed of an easy gust, along +x.
        let blow = run(&mut sim, &bear_at(0.0, 0.0, 0.0), &map, 2.0);
        assert!((blow.wind.x - p.gust_speed_easy).abs() < 0.01, "{blow:?}");
        assert_eq!(blow.events, vec!["obstacle:push:gust:#5".to_string()]);

        // A bear outside the box gets no wind.
        let mut sim = ObstacleSim::new(vec![obstacle(gust, 8.0)]);
        let out = run(&mut sim, &bear_at(0.0, 0.0, 6.0), &map, 8.0);
        assert_eq!(out.wind, Vec3::ZERO);
        assert!(out.events.is_empty());
    }

    #[test]
    fn a_push_vent_hits_one_time_in_each_blast() {
        let map = flat();
        let mut sim = ObstacleSim::new(vec![obstacle(Kind::VentPush { range: 1.5 }, 4.0)]);
        // The bear stands in the jet for two cycles.
        let out = run(&mut sim, &bear_at(0.8, 0.0, 0.0), &map, 8.0);
        let hits = out.events.iter().filter(|e| e.contains(":hit:")).count();
        assert_eq!(hits, 2);
        assert!(out.wet);
        let knock = out.knock.expect("a knock");
        assert!(knock.x > 2.5 && knock.y > 2.0 && knock.z.abs() < 1e-5);

        // Behind the vent or far to the side: no hit.
        for (x, z) in [(-1.0, 0.0), (0.8, 1.2), (2.5, 0.0)] {
            let mut sim = ObstacleSim::new(vec![obstacle(Kind::VentPush { range: 1.5 }, 4.0)]);
            let out = run(&mut sim, &bear_at(x, 0.0, z), &map, 8.0);
            assert!(out.events.is_empty(), "hit at {x}, {z}");
        }
    }

    #[test]
    fn a_launch_puts_the_bear_on_the_ledge() {
        let p = SimParams::default();
        let (rise, reach) = (4.0, 3.0);
        let (up, forward) = launch_velocity(
            rise,
            reach,
            p.launch_margin_m,
            p.bear_gravity,
            p.bear_fall_factor,
        );
        // Move the bear as `BfBear::walk` does.
        let (mut x, mut y, mut vy) = (0.0f32, 0.0f32, up);
        let mut top = 0.0f32;
        loop {
            let factor = if vy < 0.0 { p.bear_fall_factor } else { 1.0 };
            vy -= p.bear_gravity * factor * DT;
            x += forward * DT;
            y += vy * DT;
            top = top.max(y);
            if vy < 0.0 && y <= rise {
                break;
            }
        }
        assert!(top > rise + 0.6 && top < rise + 1.0, "top {top}");
        assert!((x - reach).abs() < 0.25, "the bear lands at {x} m");
    }

    #[test]
    fn a_launch_vent_throws_a_bear_that_stands_on_it_and_logs_the_result() {
        let map = flat();
        let vent = Kind::VentLaunch {
            rise: 3.0,
            reach: 2.0,
        };
        let mut sim = ObstacleSim::new(vec![obstacle(vent, 4.0)]);
        let out = run(&mut sim, &bear_at(0.2, 0.0, 0.1), &map, 3.7);
        assert_eq!(
            out.events,
            vec!["obstacle:launch:vent_launch:#5".to_string()]
        );
        let knock = out.knock.expect("a launch");
        assert!(knock.y > 9.0 && knock.x > 0.5);
        // The bear lands on the ledge.
        let out = run(&mut sim, &bear_at(2.0, 3.0, 0.0), &map, 0.5);
        assert_eq!(
            out.events,
            vec!["obstacle:launch_reached:vent_launch:#5".to_string()]
        );
    }

    fn crust() -> Obstacle {
        let lid = Lid {
            corner: [-5.0, -5.0],
            n: 11,
            spacing: 1.0,
            heights: vec![0.0; 121],
        };
        obstacle(
            Kind::AshCrust {
                radius: 2.5,
                depth: 1.0,
                lid,
            },
            1.0,
        )
    }

    #[test]
    fn a_crust_shakes_and_then_falls() {
        let map = flat();
        let p = SimParams::default();
        let mut sim = ObstacleSim::new(vec![crust()]);
        // A bear near the crust does not start it.
        run(&mut sim, &bear_at(3.0, 0.0, 0.0), &map, 1.0);
        assert_eq!(sim.crust(0), Crust::Intact);
        // A bear on the crust starts the shake.
        let out = run(
            &mut sim,
            &bear_at(1.0, 0.0, 0.0),
            &map,
            p.crust_shake_easy_s - 0.1,
        );
        assert!(matches!(sim.crust(0), Crust::Shaking { .. }));
        assert!(out.broke.is_empty());
        assert!(!out.puffs.is_empty(), "the shake makes dust");
        let out = run(&mut sim, &bear_at(1.0, 0.0, 0.0), &map, 0.2);
        assert_eq!(sim.crust(0), Crust::Broken);
        assert_eq!(out.broke, vec![0]);
        assert_eq!(out.events, vec!["obstacle:fall:ash_crust:#5".to_string()]);
    }

    #[test]
    fn a_bear_that_leaves_the_crust_in_time_escapes() {
        let map = flat();
        let mut sim = ObstacleSim::new(vec![crust()]);
        run(&mut sim, &bear_at(1.0, 0.0, 0.0), &map, 0.2);
        let out = run(&mut sim, &bear_at(4.0, 0.0, 0.0), &map, 1.0);
        assert_eq!(out.events, vec!["obstacle:escape:ash_crust:#5".to_string()]);
    }

    fn stepping_stones() -> Obstacle {
        // The path runs along +x. The trench is along z.
        obstacle(
            Kind::SteppingStones {
                count: 4,
                spacing: 1.6,
                radius: 0.62,
                half_length: 9.0,
                half_width: 2.4,
                depth: 1.8,
                ys: vec![0.0; 4],
            },
            4.0,
        )
    }

    #[test]
    fn stepping_stones_take_turns_and_two_neighbours_are_solid_together() {
        let p = SimParams::default();
        let mut o = stepping_stones();
        o.phase = 0.0;
        let mut both = 0.0f32;
        let mut none = 0.0f32;
        let mut tells = 0usize;
        let mut ghosts = 0usize;
        for step in 0..(8.0 / DT) as usize {
            let t = f64::from(step as f32 * DT);
            let a = step_phase(&o, 0, t, &p);
            let b = step_phase(&o, 1, t, &p);
            // Stones 0 and 2 are the same, stones 1 and 3 are the same.
            assert_eq!(a, step_phase(&o, 2, t, &p));
            assert_eq!(b, step_phase(&o, 3, t, &p));
            if a.solid() && b.solid() {
                both += DT;
            }
            if !a.solid() && !b.solid() {
                none += DT;
            }
            tells += matches!(a, StepPhase::Tell { .. }) as usize;
            ghosts += matches!(a, StepPhase::Ghost { .. }) as usize;
        }
        // Easy: the on part is 0.75 of 4 s, so each stone is solid for 3 s.
        // Two cycles: the two stones are solid together for 2 x 2 s.
        assert!((both - 4.0).abs() < 0.1, "both {both}");
        assert_eq!(none, 0.0, "a bear can always stand on one of them");
        assert!(tells > 0 && ghosts > 0);
    }

    #[test]
    fn a_harder_path_gives_less_time_to_hop() {
        let p = SimParams::default();
        let mut easy = stepping_stones();
        let mut hard = stepping_stones();
        easy.difficulty = 0.0;
        hard.difficulty = 1.0;
        let both = |o: &Obstacle| {
            (0..(4.0 / DT) as usize)
                .filter(|&s| {
                    let t = f64::from(s as f32 * DT);
                    step_phase(o, 0, t, &p).solid() && step_phase(o, 1, t, &p).solid()
                })
                .count() as f32
                * DT
        };
        assert!(both(&hard) > 0.3, "the hop is possible");
        assert!(both(&hard) < both(&easy));
    }

    #[test]
    fn a_bear_that_falls_into_the_trench_or_crosses_it_is_logged() {
        let map = flat();
        let mut sim = ObstacleSim::new(vec![stepping_stones()]);
        // The bear comes near the path: a tell.
        let out = run(&mut sim, &bear_at(-8.0, 0.0, 0.0), &map, 0.1);
        assert_eq!(
            out.events,
            vec!["obstacle:tell:stepping_stones:#5".to_string()]
        );
        // The bear enters on the -x side, falls and climbs out.
        let out = run(&mut sim, &bear_at(-2.0, 0.0, 0.0), &map, 0.1);
        assert!(out.events.is_empty(), "{:?}", out.events);
        let out = run(&mut sim, &bear_at(-0.5, -1.5, 0.0), &map, 0.1);
        assert_eq!(
            out.events,
            vec!["obstacle:fall:stepping_stones:#5".to_string()]
        );
        // The bear leaves on the other side after a fall: no "cross".
        let out = run(&mut sim, &bear_at(5.0, 0.0, 0.0), &map, 0.1);
        assert!(out.events.is_empty(), "{:?}", out.events);
        // A new visit that ends on the other side is a cross.
        run(&mut sim, &bear_at(3.0, 0.0, 0.0), &map, 0.1);
        let out = run(&mut sim, &bear_at(-5.0, 0.0, 0.0), &map, 0.1);
        assert_eq!(
            out.events,
            vec!["obstacle:cross:stepping_stones:#5".to_string()]
        );
    }

    fn updraft() -> Obstacle {
        obstacle(
            Kind::Updraft {
                radius: 3.0,
                height: 8.0,
            },
            8.0,
        )
    }

    fn airborne(x: f32, y: f32, z: f32) -> BearIn {
        BearIn {
            on_floor: false,
            ..bear_at(x, y, z)
        }
    }

    #[test]
    fn an_updraft_lifts_a_bear_in_the_air_and_fades_at_the_top() {
        let map = flat();
        let p = SimParams::default();
        // The strength is high at the start of the cycle (after 1 s).
        let mut sim = ObstacleSim::new(vec![updraft()]);
        run_for(&mut sim, &airborne(0.0, 0.5, 0.0), &map, 1.0);
        let out = sim.step(DT, &airborne(0.0, 0.5, 0.0), &map, &p);
        let (speed, accel) = out.lift.expect("a lift");
        assert!(
            speed > p.updraft_speed * 0.95 && accel > 14.0 * 1.4,
            "{speed}"
        );
        // High in the column, the lift is weaker.
        let out = sim.step(DT, &airborne(0.0, 7.5, 0.0), &map, &p);
        let high = out.lift.map_or(0.0, |l| l.0);
        assert!(high < speed * 0.4, "{high}");
        // On the ground, outside the column or above it: no lift.
        for bear in [
            bear_at(0.0, 0.0, 0.0),
            airborne(5.0, 0.5, 0.0),
            airborne(0.0, 8.5, 0.0),
        ] {
            let out = sim.step(DT, &bear, &map, &p);
            assert!(out.lift.is_none(), "{bear:?}");
        }
    }

    #[test]
    fn an_updraft_is_weaker_between_its_strong_phases_and_logs_a_ride_once() {
        let map = flat();
        let p = SimParams::default();
        let o = updraft();
        let strong = updraft_strength(&o, 2.0, &p);
        let weak = updraft_strength(&o, 7.0, &p);
        assert!(strong > 0.99 && (weak - p.updraft_weak).abs() < 0.01);
        let mut sim = ObstacleSim::new(vec![o]);
        let out = run(&mut sim, &airborne(0.0, 2.0, 0.0), &map, 8.0);
        let rides = out.events.iter().filter(|e| e.contains(":ride:")).count();
        assert_eq!(rides, 1);
    }

    /// Runs the simulation for some seconds and gives nothing back.
    fn run_for(sim: &mut ObstacleSim, bear: &BearIn, map: &HeightMap, seconds: f32) {
        run(sim, bear, map, seconds);
    }

    fn sand() -> Obstacle {
        let lid = Lid {
            corner: [-6.0, -6.0],
            n: 13,
            spacing: 1.0,
            heights: vec![0.0; 169],
        };
        obstacle(
            Kind::SinkingSand {
                radius: 3.0,
                depth: 0.55,
                lid,
            },
            1.0,
        )
    }

    #[test]
    fn a_standing_bear_sinks_and_slows_down_and_a_walking_bear_sinks_less() {
        let map = flat();
        let p = SimParams::default();
        let mut sim = ObstacleSim::new(vec![sand()]);
        let out = run(&mut sim, &bear_at(0.5, 0.0, 0.0), &map, 1.0);
        assert!(sim.sink(0) > 0.07 && sim.sink(0) < 0.11, "{}", sim.sink(0));
        assert!(out
            .events
            .contains(&"obstacle:sink:sinking_sand:#5".to_string()));
        // The bear is on the sand at its new height. After some seconds,
        // the sink is full, and the slow down is large.
        let low = |sink: f32| BearIn {
            feet: Vec3::new(0.5, -sink, 0.0),
            ..bear_at(0.5, 0.0, 0.0)
        };
        let mut last = StepOut::default();
        let mut events = Vec::new();
        for _ in 0..(5.0 / DT) as usize {
            last = sim.step(DT, &low(sim.sink(0)), &map, &p);
            events.extend(last.events.clone());
        }
        assert!((sim.sink(0) - p.sand_max_sink_m).abs() < 0.01);
        assert!((last.slow - p.sand_slow_max).abs() < 0.02, "{}", last.slow);
        assert_eq!(events, vec!["obstacle:stuck:sinking_sand:#5".to_string()]);

        // A walking bear sinks slower.
        let mut walker = ObstacleSim::new(vec![sand()]);
        let moving = BearIn {
            speed: 1.2,
            ..bear_at(0.5, 0.0, 0.0)
        };
        run(&mut walker, &moving, &map, 1.0);
        assert!(walker.sink(0) < sim_after_stand(&map) * 0.5);
    }

    fn sim_after_stand(map: &HeightMap) -> f32 {
        let mut sim = ObstacleSim::new(vec![sand()]);
        run(&mut sim, &bear_at(0.5, 0.0, 0.0), map, 1.0);
        sim.sink(0)
    }

    #[test]
    fn sand_comes_back_up_and_a_bear_that_gets_out_is_logged() {
        let map = flat();
        let p = SimParams::default();
        let mut sim = ObstacleSim::new(vec![sand()]);
        for _ in 0..(4.0 / DT) as usize {
            let at = BearIn {
                feet: Vec3::new(0.5, -sim.sink(0), 0.0),
                ..bear_at(0.5, 0.0, 0.0)
            };
            sim.step(DT, &at, &map, &p);
        }
        let deep = sim.sink(0);
        assert!(deep > 0.25);
        // The bear leaves: the sand comes up and the escape is logged.
        let out = run(&mut sim, &bear_at(5.0, 0.0, 0.0), &map, 0.1);
        assert_eq!(
            out.events,
            vec!["obstacle:escape:sinking_sand:#5".to_string()]
        );
        run(&mut sim, &bear_at(5.0, 0.0, 0.0), &map, 2.0);
        assert_eq!(sim.sink(0), 0.0);
        assert_eq!(out.slow, 0.0);
    }

    #[test]
    fn sand_does_nothing_to_a_bear_in_the_air_or_beside_it() {
        let map = flat();
        let mut sim = ObstacleSim::new(vec![sand()]);
        let out = run(&mut sim, &airborne(0.5, 1.5, 0.0), &map, 2.0);
        assert_eq!((sim.sink(0), out.slow), (0.0, 0.0));
        let out = run(&mut sim, &bear_at(4.0, 0.0, 0.0), &map, 2.0);
        assert_eq!((sim.sink(0), out.slow), (0.0, 0.0));
    }

    /// A slope down to +x: 0.4 m for each metre.
    fn slope() -> HeightMap {
        HeightMap::from_fn(256, 1.0, |x, _| (256 - x) as f32 * 0.4)
    }

    fn stones() -> Obstacle {
        let map = slope();
        let mut o = obstacle(
            Kind::RollingStones {
                radius: 0.2,
                count: 2,
                run_m: 30.0,
            },
            10.0,
        );
        o.pos = [-20.0, map.sample_world(-20.0, 0.0), 0.0];
        o
    }

    #[test]
    fn stones_shake_and_then_roll_down_and_stop_after_their_run() {
        let map = slope();
        let far = bear_at(0.0, 50.0, 30.0);
        let mut sim = ObstacleSim::new(vec![stones()]);
        // The first two cycles make no stones: the bear was not near at
        // the start of the first one. Go to the third cycle.
        run(&mut sim, &far, &map, 10.0);
        assert!(sim.stones().is_empty());
        run(&mut sim, &far, &map, 10.0);
        assert!(sim.stones().is_empty());
        let out = run(&mut sim, &far, &map, 0.5);
        assert_eq!(sim.stones().len(), 1);
        assert!(sim.stones()[0].wait > 0.0, "the stone shakes first");
        assert_eq!(sim.stones()[0].pos.x, -20.0);
        assert_eq!(
            out.events,
            vec!["obstacle:tell:rolling_stones:#5".to_string()]
        );
        run(&mut sim, &far, &map, 3.0);
        assert_eq!(sim.stones().len(), 2);
        for s in sim.stones() {
            assert!(s.pos.x > -19.0 && s.vel.x > 1.0, "the stone rolls: {s:?}");
            let ground = map.sample_world(s.pos.x, s.pos.z);
            assert!((s.pos.y - ground - s.radius).abs() < 1e-3);
        }
        // After the run, the stones are gone.
        run(&mut sim, &far, &map, 6.0);
        assert!(sim.stones().is_empty());
    }

    #[test]
    fn a_stone_knocks_a_bear_in_its_path() {
        let map = slope();
        let x = -8.0;
        let in_path = bear_at(x, map.sample_world(x, -0.24), -0.24);
        let mut sim = ObstacleSim::new(vec![stones()]);
        let out = run(&mut sim, &in_path, &map, 26.0);
        let hits = out.events.iter().filter(|e| e.contains(":hit:")).count();
        assert!(hits >= 1, "{:?}", out.events);
        let knock = out.knock.expect("a knock");
        assert!(knock.x > 2.0 && knock.y > 2.0, "{knock:?}");
    }

    #[test]
    fn no_stones_start_while_the_bear_is_far() {
        let map = slope();
        let far = bear_at(100.0, 0.0, 100.0);
        let mut sim = ObstacleSim::new(vec![stones()]);
        run(&mut sim, &far, &map, 25.0);
        assert!(sim.stones().is_empty());
    }

    #[test]
    fn the_same_steps_give_the_same_stones() {
        let map = slope();
        let bear = bear_at(0.0, 50.0, 30.0);
        let mut a = ObstacleSim::new(vec![stones()]);
        let mut b = ObstacleSim::new(vec![stones()]);
        run(&mut a, &bear, &map, 23.0);
        run(&mut b, &bear, &map, 23.0);
        assert!(!a.stones().is_empty());
        assert_eq!(a.stones(), b.stones());
    }
}
