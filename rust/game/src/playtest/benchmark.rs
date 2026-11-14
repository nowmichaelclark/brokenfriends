//! The plan, the camera path and the statistics of the dev-menu tests.
//! No Godot code is here, so the unit tests can check it.

use std::f32::consts::TAU;

/// The seeds of the benchmark.
pub const BENCH_SEEDS: [i64; 3] = [1, 7, 42];

/// The far planes of the draw distance test, in metres.
pub const DISTANCES_M: [f32; 3] = [300.0, 800.0, 3000.0];

/// The seed of the draw distance test.
pub const DISTANCE_SEED: i64 = 1;

/// The draw distance of the benchmark, in metres.
pub const BENCH_FAR_M: f32 = 3000.0;

/// The path has this many legs. Each leg gets its own numbers.
pub const LEGS: usize = 4;

/// The radius of the circle that the camera flies, in metres.
pub const PATH_RADIUS_M: f32 = 350.0;

/// The height of the camera above the ground, in metres.
pub const PATH_HEIGHT_M: f32 = 40.0;

/// The kind of a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestKind {
    Benchmark,
    DrawDistance,
}

impl TestKind {
    pub fn name(self) -> &'static str {
        match self {
            TestKind::Benchmark => "benchmark",
            TestKind::DrawDistance => "draw_distance",
        }
    }

    /// Reads the name of a test from the command line.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "benchmark" => Some(TestKind::Benchmark),
            "draw_distance" => Some(TestKind::DrawDistance),
            _ => None,
        }
    }
}

/// One flight along the path: one seed and one far plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    pub seed: i64,
    pub far_m: f32,
}

/// The flights of a test, in order.
pub fn plan(kind: TestKind) -> Vec<Run> {
    match kind {
        TestKind::Benchmark => BENCH_SEEDS
            .iter()
            .map(|&seed| Run {
                seed,
                far_m: BENCH_FAR_M,
            })
            .collect(),
        TestKind::DrawDistance => DISTANCES_M
            .iter()
            .map(|&far_m| Run {
                seed: DISTANCE_SEED,
                far_m,
            })
            .collect(),
    }
}

/// The angle on the circle after `t` seconds of a flight that takes
/// `total` seconds. The flight is one full circle.
pub fn path_angle(t: f32, total: f32) -> f32 {
    TAU * (t / total.max(0.001)).clamp(0.0, 1.0)
}

/// The place on the circle (x, z) at an angle, around the zone centre.
pub fn path_xz(angle: f32) -> [f32; 2] {
    [PATH_RADIUS_M * angle.cos(), PATH_RADIUS_M * angle.sin()]
}

/// The leg of the path at time `t`, and the time that the leg has run.
pub fn leg_at(t: f32, leg_seconds: f32) -> (usize, f32) {
    let leg_seconds = leg_seconds.max(0.001);
    let leg = (t / leg_seconds) as usize;
    (
        leg.min(LEGS - 1),
        t - leg.min(LEGS - 1) as f32 * leg_seconds,
    )
}

/// The frames of one leg.
#[derive(Default, Clone, Debug)]
pub struct LegStats {
    frame_ms: Vec<f32>,
    draw_calls: f64,
    primitives: f64,
}

/// The numbers of one leg.
#[derive(Clone, Debug, PartialEq)]
pub struct LegSummary {
    pub frames: usize,
    pub fps: f32,
    pub frame_avg_ms: f32,
    pub frame_p95_ms: f32,
    pub frame_max_ms: f32,
    pub draw_calls: f32,
    pub primitives: f32,
}

impl LegStats {
    pub fn push(&mut self, frame_ms: f32, draw_calls: f64, primitives: f64) {
        self.frame_ms.push(frame_ms);
        self.draw_calls += draw_calls;
        self.primitives += primitives;
    }

    pub fn summary(&self) -> LegSummary {
        let n = self.frame_ms.len();
        if n == 0 {
            return LegSummary {
                frames: 0,
                fps: 0.0,
                frame_avg_ms: 0.0,
                frame_p95_ms: 0.0,
                frame_max_ms: 0.0,
                draw_calls: 0.0,
                primitives: 0.0,
            };
        }
        let total: f32 = self.frame_ms.iter().sum();
        let avg = total / n as f32;
        let mut sorted = self.frame_ms.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let p95_index = ((n as f32 * 0.95).ceil() as usize).clamp(1, n) - 1;
        LegSummary {
            frames: n,
            fps: if avg > 0.0 { 1000.0 / avg } else { 0.0 },
            frame_avg_ms: avg,
            frame_p95_ms: sorted[p95_index],
            frame_max_ms: sorted[n - 1],
            draw_calls: (self.draw_calls / n as f64) as f32,
            primitives: (self.primitives / n as f64) as f32,
        }
    }
}

impl LegSummary {
    /// One short line for the report.
    pub fn line(&self) -> String {
        format!(
            "{:.0} fps, frame avg {:.1} ms, p95 {:.1} ms, max {:.1} ms, {:.0} draw calls, {:.0} k primitives",
            self.fps,
            self.frame_avg_ms,
            self.frame_p95_ms,
            self.frame_max_ms,
            self.draw_calls,
            self.primitives / 1000.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_have_three_flights() {
        let bench = plan(TestKind::Benchmark);
        assert_eq!(bench.iter().map(|r| r.seed).collect::<Vec<_>>(), [1, 7, 42]);
        let dist = plan(TestKind::DrawDistance);
        assert_eq!(
            dist.iter().map(|r| r.far_m).collect::<Vec<_>>(),
            [300.0, 800.0, 3000.0]
        );
        assert!(dist.iter().all(|r| r.seed == DISTANCE_SEED));
    }

    #[test]
    fn the_path_is_one_full_circle() {
        assert_eq!(path_angle(0.0, 10.0), 0.0);
        assert!((path_angle(10.0, 10.0) - TAU).abs() < 1e-5);
        let [x, z] = path_xz(0.0);
        assert!((x - PATH_RADIUS_M).abs() < 1e-3 && z.abs() < 1e-3);
    }

    #[test]
    fn legs_follow_time() {
        assert_eq!(leg_at(0.0, 8.0).0, 0);
        assert_eq!(leg_at(9.0, 8.0), (1, 1.0));
        assert_eq!(leg_at(31.9, 8.0).0, 3);
        // Past the end, the last leg stays.
        assert_eq!(leg_at(99.0, 8.0).0, LEGS - 1);
    }

    #[test]
    fn summary_gives_average_p95_and_max() {
        let mut s = LegStats::default();
        for i in 1..=100 {
            s.push(i as f32, 10.0, 1000.0);
        }
        let r = s.summary();
        assert_eq!(r.frames, 100);
        assert!((r.frame_avg_ms - 50.5).abs() < 1e-3);
        assert_eq!(r.frame_p95_ms, 95.0);
        assert_eq!(r.frame_max_ms, 100.0);
        assert_eq!(r.draw_calls, 10.0);
        assert!((r.fps - 1000.0 / 50.5).abs() < 1e-3);
    }

    #[test]
    fn an_empty_leg_has_zeros() {
        let r = LegStats::default().summary();
        assert_eq!(r.frames, 0);
        assert_eq!(r.fps, 0.0);
    }

    #[test]
    fn names_round_trip() {
        for k in [TestKind::Benchmark, TestKind::DrawDistance] {
            assert_eq!(TestKind::from_name(k.name()), Some(k));
        }
        assert_eq!(TestKind::from_name("x"), None);
    }
}
