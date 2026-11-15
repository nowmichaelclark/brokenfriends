//! The tests of the dev menu: the camera path, the statistics and the
//! text for `report.md` (S-113, `plan/diagnostics.md`). This module has
//! no Godot code. `crate::test_runner::BfTestRunner` flies the camera.
//!
//! A test is a list of runs. Each run makes a zone from a seed, sets a
//! view distance and flies the camera along a fixed path. The path has
//! parts. The result has one row for each part. The same path on each
//! laptop gives numbers that we can compare.

use super::stats::FrameHistogram;
use serde_json::{json, Value};
use std::fmt::Write;

/// The tests that the "Tests" page can start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestKind {
    /// The camera flies the path over seeds 1, 7 and 42.
    Benchmark,
    /// The camera flies the path over seed 7 at three view distances
    /// and at the distance of the smog (the normal game).
    DrawDistance,
    /// The camera stops at fixed views of the water over seeds 1, 7 and
    /// 42 (S-160, `demo-2.md`, "Speed tests").
    WaterView,
    /// The camera flies into the widest cave mouth, around a chamber and
    /// back, over seeds 1, 7 and 42 (S-164, `cave_tour.rs`).
    CaveTour,
}

/// The views of the Water view test: the name for `BfWater::test_view`
/// and the text for the report. S-165 adds the tallest and the widest
/// fall.
pub const WATER_VIEWS: [(&str, &str); 2] = [
    ("largest_lake", "the largest lake"),
    ("river_rapids", "a river with rapids"),
];

/// The time of each view of the Water view test, in seconds.
pub const VIEW_SECONDS: f32 = 6.0;

/// The frames of the first seconds of each view are not measured.
pub const VIEW_WARM_SECONDS: f32 = 2.0;

/// Where the Water view test is after this time in a run: the view and
/// if the frame counts. `None` after the last view.
pub fn view_at(elapsed_s: f32, views: usize) -> Option<(usize, bool)> {
    let index = (elapsed_s.max(0.0) / VIEW_SECONDS) as usize;
    (index < views).then(|| {
        let t = elapsed_s - index as f32 * VIEW_SECONDS;
        (index, t >= VIEW_WARM_SECONDS)
    })
}

impl TestKind {
    /// The name in the report and in the file names.
    pub fn name(self) -> &'static str {
        match self {
            TestKind::Benchmark => "benchmark",
            TestKind::DrawDistance => "draw-distance",
            TestKind::WaterView => "water-view",
            TestKind::CaveTour => "cave-tour",
        }
    }

    /// The name on the menu.
    pub fn title(self) -> &'static str {
        match self {
            TestKind::Benchmark => "Benchmark",
            TestKind::DrawDistance => "Draw distance test",
            TestKind::WaterView => "Water view",
            TestKind::CaveTour => "Cave tour",
        }
    }

    /// Gives the kind for a name (the command line `--bf-test=...`).
    pub fn from_name(name: &str) -> Option<TestKind> {
        match name {
            "benchmark" | "bench" => Some(TestKind::Benchmark),
            "draw-distance" | "draw" => Some(TestKind::DrawDistance),
            "water-view" | "water" => Some(TestKind::WaterView),
            "cave-tour" | "caves" => Some(TestKind::CaveTour),
            _ => None,
        }
    }
}

/// One run of a test: a seed and a view distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunSpec {
    pub seed: i64,
    /// The view distance in metres. `None`: the distance of the smog
    /// (the normal value of the game).
    pub view_distance_m: Option<f32>,
}

/// Gives the runs of a test, in order.
pub fn plan(kind: TestKind) -> Vec<RunSpec> {
    match kind {
        TestKind::Benchmark | TestKind::WaterView | TestKind::CaveTour => [1, 7, 42]
            .into_iter()
            .map(|seed| RunSpec {
                seed,
                view_distance_m: None,
            })
            .collect(),
        TestKind::DrawDistance => [Some(300.0), Some(800.0), Some(3000.0), None]
            .into_iter()
            .map(|d| RunSpec {
                seed: 7,
                view_distance_m: d,
            })
            .collect(),
    }
}

/// The fixed path of the camera: a closed polygon around the centre of
/// the zone. Each side is one part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenchPath {
    /// The distance of the corners from the centre, in metres.
    pub radius_m: f32,
    /// The number of sides.
    pub parts: usize,
    /// The speed of the camera in metres per second.
    pub speed_mps: f32,
}

/// A point of the path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPoint {
    pub x: f32,
    pub z: f32,
    /// The direction of the path as a unit vector in x and z.
    pub dir: [f32; 2],
    /// The side that the point is on, from 0.
    pub part: usize,
}

impl BenchPath {
    fn corner(&self, k: usize) -> [f32; 2] {
        let a = std::f32::consts::TAU * (k % self.parts.max(3)) as f32 / self.parts.max(3) as f32;
        [self.radius_m * a.cos(), self.radius_m * a.sin()]
    }

    /// The length of one side in metres.
    pub fn part_length(&self) -> f32 {
        let a = self.corner(0);
        let b = self.corner(1);
        ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
    }

    /// The time to fly the full path, in seconds.
    pub fn duration_s(&self) -> f32 {
        self.part_length() * self.parts.max(3) as f32 / self.speed_mps.max(0.1)
    }

    /// The time to fly one side, in seconds.
    pub fn part_seconds(&self) -> f32 {
        self.part_length() / self.speed_mps.max(0.1)
    }

    /// Gives the point after this time, in seconds. After the end, the
    /// last point stays.
    pub fn at_time(&self, seconds: f32) -> PathPoint {
        let n = self.parts.max(3);
        let length = self.part_length().max(0.001);
        let along = (seconds.max(0.0) * self.speed_mps).min(length * n as f32 - 0.001);
        let part = ((along / length) as usize).min(n - 1);
        let f = (along - part as f32 * length) / length;
        let a = self.corner(part);
        let b = self.corner(part + 1);
        let dx = b[0] - a[0];
        let dz = b[1] - a[1];
        let l = (dx * dx + dz * dz).sqrt().max(0.001);
        PathPoint {
            x: a[0] + dx * f,
            z: a[1] + dz * f,
            dir: [dx / l, dz / l],
            part,
        }
    }
}

/// Gives the height of the camera: above the highest ground in a few
/// places along the path, so the camera does not go into a hill.
pub fn camera_height(ground: impl Fn(f32, f32) -> f32, point: &PathPoint, clearance_m: f32) -> f32 {
    let mut highest = f32::MIN;
    for d in [-30.0f32, 0.0, 30.0, 60.0] {
        highest = highest.max(ground(
            point.x + point.dir[0] * d,
            point.z + point.dir[1] * d,
        ));
    }
    highest + clearance_m
}

/// The data of one frame in a run.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameData {
    pub ms: f32,
    pub draw_calls: f32,
    pub primitives: f32,
    /// The render time on the CPU and on the GPU, in milliseconds.
    /// Zero if the engine does not measure it.
    pub cpu_ms: f32,
    pub gpu_ms: f32,
    /// The water meshes in the view: each is one draw call (S-160).
    pub water_draws: f32,
}

#[derive(Clone, Debug, Default)]
struct PartStats {
    frames: FrameHistogram,
    draw_calls: f64,
    primitives: f64,
    cpu_ms: f64,
    gpu_ms: f64,
    water_draws: f64,
}

/// Collects the frames of one run.
#[derive(Clone, Debug)]
pub struct RunStats {
    parts: Vec<PartStats>,
}

/// The numbers of one part of one run.
#[derive(Clone, Debug, PartialEq)]
pub struct PartResult {
    pub part: usize,
    pub frames: u64,
    pub avg_fps: f32,
    pub low_1pct_fps: f32,
    pub p95_ms: f32,
    pub worst_ms: f32,
    pub draw_calls: f32,
    pub primitives: f32,
    pub cpu_ms: f32,
    pub gpu_ms: f32,
    pub water_draws: f32,
}

/// The result of one run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunResult {
    pub spec: RunSpec,
    /// The view distance that the game used, in metres.
    pub used_view_distance_m: f32,
    /// The number of Terrain3D mesh rings in this run (0: not known).
    pub terrain_lods: i32,
    pub parts: Vec<PartResult>,
    /// The names of the parts (the views of the Water view test). Empty
    /// for a path: the parts have numbers.
    pub part_names: Vec<String>,
}

impl RunStats {
    pub fn new(parts: usize) -> Self {
        Self {
            parts: vec![PartStats::default(); parts.max(1)],
        }
    }

    /// Adds one frame to a part.
    pub fn add(&mut self, part: usize, frame: FrameData) {
        let last = self.parts.len() - 1;
        let p = &mut self.parts[part.min(last)];
        p.frames.add(frame.ms);
        p.draw_calls += frame.draw_calls as f64;
        p.primitives += frame.primitives as f64;
        p.cpu_ms += frame.cpu_ms as f64;
        p.gpu_ms += frame.gpu_ms as f64;
        p.water_draws += frame.water_draws as f64;
    }

    /// The numbers of each part. A part without frames is left out.
    pub fn results(&self) -> Vec<PartResult> {
        self.parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.frames.frames > 0)
            .map(|(i, p)| {
                let n = p.frames.frames as f64;
                PartResult {
                    part: i,
                    frames: p.frames.frames,
                    avg_fps: p.frames.average_fps(),
                    low_1pct_fps: p.frames.one_percent_low_fps(),
                    p95_ms: p.frames.percentile_ms(0.95),
                    worst_ms: p.frames.worst_ms,
                    draw_calls: (p.draw_calls / n) as f32,
                    primitives: (p.primitives / n) as f32,
                    cpu_ms: (p.cpu_ms / n) as f32,
                    gpu_ms: (p.gpu_ms / n) as f32,
                    water_draws: (p.water_draws / n) as f32,
                }
            })
            .collect()
    }

    /// The numbers of the full run: the same statistics over all parts.
    pub fn total(&self) -> Option<PartResult> {
        let mut all = FrameHistogram::default();
        let mut sums = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for p in &self.parts {
            all.merge(&p.frames);
            sums.0 += p.draw_calls;
            sums.1 += p.primitives;
            sums.2 += p.cpu_ms;
            sums.3 += p.gpu_ms;
            sums.4 += p.water_draws;
        }
        if all.frames == 0 {
            return None;
        }
        let n = all.frames as f64;
        Some(PartResult {
            part: usize::MAX,
            frames: all.frames,
            avg_fps: all.average_fps(),
            low_1pct_fps: all.one_percent_low_fps(),
            p95_ms: all.percentile_ms(0.95),
            worst_ms: all.worst_ms,
            draw_calls: (sums.0 / n) as f32,
            primitives: (sums.1 / n) as f32,
            cpu_ms: (sums.2 / n) as f32,
            gpu_ms: (sums.3 / n) as f32,
            water_draws: (sums.4 / n) as f32,
        })
    }
}

/// What a test needs to write its result into the report.
pub struct TestReport<'a> {
    pub kind: TestKind,
    pub path: BenchPath,
    pub clearance_m: f32,
    /// The run results and, for each run, the total.
    pub runs: &'a [(RunResult, Option<PartResult>)],
    /// The test stopped before the end.
    pub cancelled: bool,
    /// The game has no window or no GPU: the numbers do not tell the
    /// speed.
    pub headless: bool,
    /// The settings of the process that change the cost, for example
    /// "water on" (each setting runs in a new process).
    pub settings: String,
}

fn row(out: &mut String, name: &str, r: &PartResult) {
    let _ = writeln!(
        out,
        "| {name} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.0} | {:.1} | {:.0} | {:.1} | {:.1} |",
        r.frames,
        r.avg_fps,
        r.low_1pct_fps,
        r.p95_ms,
        r.worst_ms,
        r.draw_calls,
        r.water_draws,
        r.primitives / 1000.0,
        r.cpu_ms,
        r.gpu_ms
    );
}

/// Makes the text of the test for `report.md`.
pub fn make_markdown(t: &TestReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "### Test: {}\n", t.kind.title());
    if t.kind == TestKind::WaterView {
        let names: Vec<&str> = WATER_VIEWS.iter().map(|v| v.1).collect();
        let _ = writeln!(
            out,
            "The camera stops at fixed views of the water: {}. Each view lasts {:.0} s; \
             the numbers start after {:.0} s. At the lake, the camera is 4 m above the water \
             near the shore and looks across the longest stretch of water. At the river, the \
             camera is 3 m above the water about 14 m below a rapid and looks up the river.",
            names.join(", "),
            VIEW_SECONDS,
            VIEW_WARM_SECONDS
        );
    } else if t.kind == TestKind::CaveTour {
        out.push_str(super::cave_tour::INTRO);
        out.push('\n');
    } else {
        let _ = writeln!(
            out,
            "The camera flies a closed path of {} parts around the centre of the zone: \
             radius {:.0} m, speed {:.0} m/s, {:.0} m above the ground, flat look-down of 12 degrees. \
             Each part is about {:.1} s.",
            t.path.parts,
            t.path.radius_m,
            t.path.speed_mps,
            t.clearance_m,
            t.path.part_seconds()
        );
    }
    if !t.settings.is_empty() {
        let _ = writeln!(out, "\nSettings of this process: {}.", t.settings);
    }
    if t.cancelled {
        out.push_str("\nThe test stopped before the end. The list is not full.\n");
    }
    if t.headless {
        out.push_str(
            "\nThis run had no window. The times are not valid for the speed of the game.\n",
        );
    }
    for (run, total) in t.runs {
        let distance = match run.spec.view_distance_m {
            Some(d) => format!("{d:.0} m (forced)"),
            None => format!("{:.0} m (from the smog)", run.used_view_distance_m),
        };
        let rings = if run.terrain_lods > 0 {
            format!(", {} terrain rings", run.terrain_lods)
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "\n#### Seed {}, view distance {distance}{rings}\n",
            run.spec.seed
        );
        let first = if run.part_names.is_empty() {
            "part"
        } else {
            "view"
        };
        let _ = writeln!(
            out,
            "| {first} | frames | avg fps | 1% low fps | p95 ms | worst ms | draw calls | water draws | primitives (k) | render cpu ms | render gpu ms |\n\
             |---|---|---|---|---|---|---|---|---|---|---|"
        );
        for p in &run.parts {
            let name = run
                .part_names
                .get(p.part)
                .cloned()
                .unwrap_or_else(|| format!("{}", p.part + 1));
            row(&mut out, &name, p);
        }
        if let Some(total) = total {
            row(&mut out, "**all**", total);
        }
        // A view with no frames: the zone has no such place.
        for (i, name) in run.part_names.iter().enumerate() {
            if !run.parts.iter().any(|p| p.part == i) {
                let _ = writeln!(out, "\nNo frames for the view \"{name}\".");
            }
        }
    }
    out
}

/// Makes the data of the test for the file `tests/<name>.json`.
pub fn make_json(t: &TestReport) -> Value {
    let part = |p: &PartResult| {
        json!({
            "part": if p.part == usize::MAX { Value::Null } else { json!(p.part + 1) },
            "frames": p.frames,
            "avg_fps": p.avg_fps,
            "low_1pct_fps": p.low_1pct_fps,
            "p95_ms": p.p95_ms,
            "worst_ms": p.worst_ms,
            "draw_calls": p.draw_calls,
            "primitives": p.primitives,
            "render_cpu_ms": p.cpu_ms,
            "render_gpu_ms": p.gpu_ms,
            "water_draw_calls": p.water_draws,
        })
    };
    let runs: Vec<Value> = t
        .runs
        .iter()
        .map(|(run, total)| {
            json!({
                "seed": run.spec.seed,
                "view_distance_forced_m": run.spec.view_distance_m,
                "view_distance_used_m": run.used_view_distance_m,
                "terrain_lods": run.terrain_lods,
                "part_names": run.part_names,
                "parts": run.parts.iter().map(part).collect::<Vec<_>>(),
                "total": total.as_ref().map(part),
            })
        })
        .collect();
    json!({
        "test": t.kind.name(),
        "path": {
            "parts": t.path.parts,
            "radius_m": t.path.radius_m,
            "speed_mps": t.path.speed_mps,
            "clearance_m": t.clearance_m,
        },
        "cancelled": t.cancelled,
        "headless": t.headless,
        "settings": t.settings,
        "runs": runs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> BenchPath {
        BenchPath {
            radius_m: 500.0,
            parts: 8,
            speed_mps: 60.0,
        }
    }

    #[test]
    fn plans_have_the_seeds_and_distances() {
        let b = plan(TestKind::Benchmark);
        assert_eq!(b.iter().map(|r| r.seed).collect::<Vec<_>>(), [1, 7, 42]);
        assert!(b.iter().all(|r| r.view_distance_m.is_none()));
        let d = plan(TestKind::DrawDistance);
        assert_eq!(
            d.iter().map(|r| r.view_distance_m).collect::<Vec<_>>(),
            [Some(300.0), Some(800.0), Some(3000.0), None]
        );
        assert!(d.iter().all(|r| r.seed == 7));
    }

    #[test]
    fn names_round_trip() {
        for k in [
            TestKind::Benchmark,
            TestKind::DrawDistance,
            TestKind::WaterView,
            TestKind::CaveTour,
        ] {
            assert_eq!(TestKind::from_name(k.name()), Some(k));
        }
        assert_eq!(TestKind::from_name("nothing"), None);
    }

    #[test]
    fn path_is_closed_and_fixed() {
        let p = path();
        let start = p.at_time(0.0);
        assert!((start.x - 500.0).abs() < 0.01 && start.z.abs() < 0.01);
        // The same time always gives the same point.
        assert_eq!(p.at_time(12.3), p.at_time(12.3));
        // After the full time the camera is at the end of the last side.
        let end = p.at_time(p.duration_s() + 5.0);
        assert!((end.x - 500.0).abs() < 1.0 && end.z.abs() < 1.0);
        assert_eq!(end.part, 7);
        // Each side has the same length.
        assert!((p.part_length() - 382.68).abs() < 0.1);
    }

    #[test]
    fn parts_follow_time() {
        let p = path();
        let s = p.part_seconds();
        assert_eq!(p.at_time(0.5 * s).part, 0);
        assert_eq!(p.at_time(1.5 * s).part, 1);
        assert_eq!(p.at_time(7.5 * s).part, 7);
        let d = p.at_time(0.5 * s).dir;
        assert!((d[0] * d[0] + d[1] * d[1] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn camera_stays_above_a_hill_ahead() {
        let point = PathPoint {
            x: 0.0,
            z: 0.0,
            dir: [1.0, 0.0],
            part: 0,
        };
        // A hill 60 m ahead.
        let ground = |x: f32, _z: f32| if x >= 59.0 { 100.0 } else { 10.0 };
        assert!((camera_height(ground, &point, 25.0) - 125.0).abs() < 0.01);
    }

    #[test]
    fn statistics_of_a_run() {
        let mut s = RunStats::new(2);
        for _ in 0..100 {
            s.add(
                0,
                FrameData {
                    ms: 10.0,
                    draw_calls: 50.0,
                    primitives: 20000.0,
                    cpu_ms: 2.0,
                    gpu_ms: 6.0,
                    water_draws: 0.0,
                },
            );
        }
        for _ in 0..50 {
            s.add(
                1,
                FrameData {
                    ms: 20.0,
                    draw_calls: 100.0,
                    ..FrameData::default()
                },
            );
        }
        let r = s.results();
        assert_eq!(r.len(), 2);
        assert!((r[0].avg_fps - 100.0).abs() < 1.0);
        assert!((r[0].draw_calls - 50.0).abs() < 0.01);
        assert!((r[1].avg_fps - 50.0).abs() < 1.0);
        let total = s.total().unwrap();
        assert_eq!(total.frames, 150);
        // 100 frames of 10 ms and 50 of 20 ms: 2000 ms for 150 frames.
        assert!((total.avg_fps - 75.0).abs() < 1.0);
        assert!((total.draw_calls - 66.67).abs() < 0.1);
        assert!(total.worst_ms >= 20.0);
    }

    #[test]
    fn empty_parts_are_left_out() {
        let mut s = RunStats::new(3);
        s.add(2, FrameData::default());
        assert_eq!(s.results().len(), 1);
        assert_eq!(s.results()[0].part, 2);
        assert!(RunStats::new(3).total().is_none());
    }

    #[test]
    fn report_text_and_json() {
        let mut s = RunStats::new(1);
        for _ in 0..10 {
            s.add(
                0,
                FrameData {
                    ms: 16.0,
                    ..FrameData::default()
                },
            );
        }
        let run = RunResult {
            spec: RunSpec {
                seed: 7,
                view_distance_m: Some(300.0),
            },
            used_view_distance_m: 300.0,
            terrain_lods: 3,
            parts: s.results(),
            part_names: Vec::new(),
        };
        let runs = vec![(run, s.total())];
        let t = TestReport {
            kind: TestKind::DrawDistance,
            path: path(),
            clearance_m: 25.0,
            runs: &runs,
            cancelled: true,
            headless: true,
            settings: String::new(),
        };
        let text = make_markdown(&t);
        assert!(text.contains("### Test: Draw distance test"));
        assert!(text.contains("Seed 7, view distance 300 m (forced), 3 terrain rings"));
        assert!(text.contains("stopped before the end"));
        assert!(text.contains("no window"));
        assert!(text.contains("**all**"));
        let j = make_json(&t);
        assert_eq!(j["test"], "draw-distance");
        assert_eq!(j["runs"][0]["seed"], 7);
        assert!(j["runs"][0]["total"]["avg_fps"].as_f64().unwrap() > 50.0);
    }

    #[test]
    fn water_views_follow_time() {
        assert_eq!(plan(TestKind::WaterView).len(), 3);
        assert_eq!(view_at(0.5, 2), Some((0, false)));
        assert_eq!(view_at(2.5, 2), Some((0, true)));
        assert_eq!(view_at(6.5, 2), Some((1, false)));
        assert_eq!(view_at(9.0, 2), Some((1, true)));
        assert_eq!(view_at(12.0, 2), None);
    }

    #[test]
    fn water_view_report_names_the_views() {
        let mut s = RunStats::new(2);
        for _ in 0..10 {
            s.add(
                0,
                FrameData {
                    ms: 16.0,
                    water_draws: 2.0,
                    ..FrameData::default()
                },
            );
        }
        let run = RunResult {
            spec: RunSpec {
                seed: 1,
                view_distance_m: None,
            },
            used_view_distance_m: 767.0,
            terrain_lods: 4,
            parts: s.results(),
            part_names: vec!["the largest lake".into(), "a river".into()],
        };
        let runs = vec![(run, s.total())];
        let t = TestReport {
            kind: TestKind::WaterView,
            path: path(),
            clearance_m: 25.0,
            runs: &runs,
            cancelled: false,
            headless: false,
            settings: "water on".into(),
        };
        let text = make_markdown(&t);
        assert!(text.contains("### Test: Water view"));
        assert!(text.contains("| the largest lake | 10 |"));
        assert!(text.contains("No frames for the view \"a river\"."));
        assert!(text.contains("Settings of this process: water on."));
        let j = make_json(&t);
        assert_eq!(j["runs"][0]["total"]["water_draw_calls"], 2.0);
        assert_eq!(j["settings"], "water on");
    }
}
