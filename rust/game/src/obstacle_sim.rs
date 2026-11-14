//! The obstacles in motion (R23). This module has no Godot types.
//!
//! `ObstacleSim` moves the obstacles of one zone in each physics step.
//! It reads the place of the bear and tells what the obstacles do to
//! the bear: a wind, a knock, wet fur. It also gives the events for the
//! playtest recorder and the puffs of dust and steam for the screen.
//!
//! Each obstacle has a tell before it acts: the crust shakes, a stone
//! shakes at the top of its gully, dust moves in before a gust, and a
//! vent makes more and more steam before it blows.

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
}

/// The look of a puff.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PuffKind {
    /// Pale dust: gusts, stones, the crust.
    Dust,
    /// Dirty steam: vents.
    Steam,
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
