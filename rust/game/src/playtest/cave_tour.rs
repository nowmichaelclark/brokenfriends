//! The Cave tour test (S-164, `plan/demo-2.md`, "Speed tests"). This
//! module has no Godot code. `crate::test_runner::BfTestRunner` flies the
//! camera along `bf_worldgen::caves::Caves::tour`.
//!
//! The frame times, the draw calls and the GPU time go into the normal
//! table of `bench.rs`, with one row for each part of the tour. This
//! module adds a second table with the numbers of the cave chunks.

use bf_worldgen::caves::TOUR_PARTS;
use serde_json::{json, Value};
use std::fmt::Write;

/// The text at the top of the test in `report.md`.
pub const INTRO: &str = "The camera flies from 50 m outside the widest hillside mouth to the \
mouth, down the tunnels to the widest chamber within 400 m, once around that chamber at half \
of its radius, and the same way back. The parts are: outside, tunnel, chamber, back. The cave \
table gives the cave chunks of each part. The cave light and the deep-cave cut come in S-166, \
so this test has one setting only.";

/// A frame longer than this is a hitch, in milliseconds.
pub const HITCH_MS: f32 = 50.0;

/// The cave numbers of one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaveFrame {
    /// The frame time in milliseconds.
    pub ms: f32,
    /// Chunks with a mesh that shows.
    pub shown: f32,
    /// Chunks with collision.
    pub bodies: f32,
    /// Chunks that wait for a worker or an upload.
    pub queue: f32,
    /// The CPU time of the cave work in the frame, in milliseconds.
    pub upload_ms: f32,
    /// The triangles of the shown cave meshes.
    pub triangles: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Part {
    frames: u64,
    shown: f64,
    bodies: f64,
    max_queue: f32,
    upload_ms: f64,
    max_upload_ms: f32,
    triangles: f64,
    hitches: u32,
}

/// The cave numbers of one run, for each part of the tour.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaveStats {
    parts: Vec<Part>,
}

impl CaveStats {
    pub fn new() -> Self {
        Self {
            parts: vec![Part::default(); TOUR_PARTS.len()],
        }
    }

    /// Adds one frame to a part.
    pub fn add(&mut self, part: usize, f: CaveFrame) {
        let last = self.parts.len() - 1;
        let p = &mut self.parts[part.min(last)];
        p.frames += 1;
        p.shown += f.shown as f64;
        p.bodies += f.bodies as f64;
        p.max_queue = p.max_queue.max(f.queue);
        p.upload_ms += f.upload_ms as f64;
        p.max_upload_ms = p.max_upload_ms.max(f.upload_ms);
        p.triangles += f.triangles as f64;
        if f.ms >= HITCH_MS {
            p.hitches += 1;
        }
    }
}

/// The cave numbers of all runs, for the report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaveRun {
    pub seed: i64,
    /// A short text about the tour: the mouth and the chamber, or why
    /// the zone has no tour.
    pub about: String,
    pub stats: CaveStats,
}

/// Makes the cave table of the test for `report.md`.
pub fn make_markdown(runs: &[CaveRun]) -> String {
    let mut out = String::from("\n#### Cave chunks\n");
    for run in runs {
        let _ = writeln!(out, "\nSeed {}: {}\n", run.seed, run.about);
        out.push_str(
            "| part | frames | meshes shown | collision | largest queue | cave ms (avg) | cave ms (max) | cave triangles (k) | hitches |\n\
             |---|---|---|---|---|---|---|---|---|\n",
        );
        for (i, p) in run.stats.parts.iter().enumerate() {
            if p.frames == 0 {
                continue;
            }
            let n = p.frames as f64;
            let _ = writeln!(
                out,
                "| {} | {} | {:.1} | {:.1} | {:.0} | {:.2} | {:.2} | {:.0} | {} |",
                TOUR_PARTS.get(i).unwrap_or(&"?"),
                p.frames,
                p.shown / n,
                p.bodies / n,
                p.max_queue,
                p.upload_ms / n,
                p.max_upload_ms,
                p.triangles / n / 1000.0,
                p.hitches
            );
        }
    }
    out
}

/// Makes the cave data of the test for the JSON file.
pub fn make_json(runs: &[CaveRun]) -> Value {
    let runs: Vec<Value> = runs
        .iter()
        .map(|run| {
            let parts: Vec<Value> = run
                .stats
                .parts
                .iter()
                .enumerate()
                .filter(|(_, p)| p.frames > 0)
                .map(|(i, p)| {
                    let n = p.frames as f64;
                    json!({
                        "part": TOUR_PARTS.get(i).copied().unwrap_or("?"),
                        "frames": p.frames,
                        "meshes_shown": p.shown / n,
                        "collision_chunks": p.bodies / n,
                        "largest_queue": p.max_queue,
                        "cave_ms_avg": p.upload_ms / n,
                        "cave_ms_max": p.max_upload_ms,
                        "cave_triangles": p.triangles / n,
                        "hitches": p.hitches,
                    })
                })
                .collect();
            json!({ "seed": run.seed, "about": run.about, "parts": parts })
        })
        .collect();
    Value::Array(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cave_table_has_one_row_for_each_part_with_frames() {
        let mut stats = CaveStats::new();
        for i in 0..10 {
            stats.add(
                1,
                CaveFrame {
                    ms: if i == 0 { 80.0 } else { 16.0 },
                    shown: 20.0,
                    bodies: 3.0,
                    queue: i as f32,
                    upload_ms: 0.5,
                    triangles: 50_000.0,
                },
            );
        }
        let runs = vec![CaveRun {
            seed: 7,
            about: "mouth at (1, 2)".into(),
            stats,
        }];
        let text = make_markdown(&runs);
        assert!(text.contains("Seed 7: mouth at (1, 2)"));
        assert!(text.contains("| tunnel | 10 | 20.0 | 3.0 | 9 | 0.50 | 0.50 | 50 | 1 |"));
        assert!(!text.contains("| outside |"));
        let j = make_json(&runs);
        assert_eq!(j[0]["parts"][0]["part"], "tunnel");
        assert_eq!(j[0]["parts"][0]["hitches"], 1);
    }
}
