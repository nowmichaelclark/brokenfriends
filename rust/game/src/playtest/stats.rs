//! Frame-time statistics for the playtest recorder.
//!
//! The recorder keeps two kinds of statistics:
//! - `SecondStats` collects the frame times of one second. Each line of
//!   `samples.jsonl` comes from it.
//! - `FrameHistogram` counts all frame times of the session in bins of
//!   0.1 ms. The report takes the average fps, the "1% low" and the
//!   worst frame from it. The memory does not grow with the play time.

use serde::{Deserialize, Serialize};

/// The width of one histogram bin in milliseconds.
const BIN_MS: f32 = 0.1;
/// The number of bins. Frames longer than 500 ms go into the last bin.
const BIN_COUNT: usize = 5000;

/// Frame times of one second.
#[derive(Clone, Debug, Default)]
pub struct SecondStats {
    frames_ms: Vec<f32>,
}

/// The summary of one second of frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameSummary {
    /// The number of frames in the second.
    pub fps: f32,
    /// The average frame time in milliseconds.
    pub avg_ms: f32,
    /// 95% of the frames are this fast or faster, in milliseconds.
    pub p95_ms: f32,
    /// The longest frame in milliseconds.
    pub max_ms: f32,
}

impl SecondStats {
    /// Adds the time of one frame.
    pub fn add(&mut self, ms: f32) {
        self.frames_ms.push(ms.max(0.0));
    }

    /// Gives the summary of the frames since the last `take` and starts
    /// a new second. `seconds` is the real time that the frames used.
    pub fn take(&mut self, seconds: f32) -> FrameSummary {
        let frames = std::mem::take(&mut self.frames_ms);
        summary(frames, seconds)
    }
}

fn summary(mut frames: Vec<f32>, seconds: f32) -> FrameSummary {
    if frames.is_empty() {
        return FrameSummary::default();
    }
    let n = frames.len();
    let sum: f32 = frames.iter().sum();
    frames.sort_by(f32::total_cmp);
    // The nearest-rank percentile.
    let rank = ((0.95 * n as f32).ceil() as usize).clamp(1, n);
    FrameSummary {
        fps: if seconds > 0.0 {
            n as f32 / seconds
        } else {
            0.0
        },
        avg_ms: sum / n as f32,
        p95_ms: frames[rank - 1],
        max_ms: frames[n - 1],
    }
}

/// All frame times of a session, in bins of 0.1 ms.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameHistogram {
    /// Pairs of (bin, count). Only bins with frames are stored.
    bins: Vec<(u32, u64)>,
    /// The longest frame in milliseconds.
    pub worst_ms: f32,
    /// The sum of all frame times in milliseconds.
    pub total_ms: f64,
    /// The number of frames.
    pub frames: u64,
    #[serde(skip)]
    dense: Vec<u64>,
}

impl Default for FrameHistogram {
    fn default() -> Self {
        Self {
            bins: Vec::new(),
            worst_ms: 0.0,
            total_ms: 0.0,
            frames: 0,
            dense: vec![0; BIN_COUNT],
        }
    }
}

impl FrameHistogram {
    /// Adds the time of one frame.
    pub fn add(&mut self, ms: f32) {
        let ms = ms.max(0.0);
        if self.dense.len() != BIN_COUNT {
            self.densify();
        }
        let bin = ((ms / BIN_MS) as usize).min(BIN_COUNT - 1);
        self.dense[bin] += 1;
        self.frames += 1;
        self.total_ms += ms as f64;
        self.worst_ms = self.worst_ms.max(ms);
    }

    /// Makes the stored form (`bins`) from the counts. Call this before
    /// the histogram is written to a file.
    pub fn pack(&mut self) {
        if self.dense.len() != BIN_COUNT {
            return;
        }
        self.bins = self
            .dense
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(i, &c)| (i as u32, c))
            .collect();
    }

    /// Makes the counts from the stored form, after a file was read.
    fn densify(&mut self) {
        self.dense = vec![0; BIN_COUNT];
        for &(bin, count) in &self.bins {
            if let Some(slot) = self.dense.get_mut(bin as usize) {
                *slot += count;
            }
        }
    }

    fn counts(&self) -> Vec<u64> {
        if self.dense.len() == BIN_COUNT {
            return self.dense.clone();
        }
        let mut copy = self.clone();
        copy.densify();
        copy.dense
    }

    /// The average fps of the session: frames divided by time.
    pub fn average_fps(&self) -> f32 {
        if self.total_ms <= 0.0 {
            return 0.0;
        }
        (self.frames as f64 * 1000.0 / self.total_ms) as f32
    }

    /// The "1% low": the fps of the slowest 1% of the frames. It is
    /// 1000 divided by the average time of those frames.
    pub fn one_percent_low_fps(&self) -> f32 {
        let worst = ((self.frames as f64 * 0.01).ceil() as u64).max(1);
        if self.frames == 0 {
            return 0.0;
        }
        let counts = self.counts();
        let mut left = worst;
        let mut sum_ms = 0.0f64;
        for (bin, &count) in counts.iter().enumerate().rev() {
            if left == 0 {
                break;
            }
            let take = count.min(left);
            // The middle of the bin. The last bin uses the worst frame.
            let ms = if bin == BIN_COUNT - 1 {
                self.worst_ms as f64
            } else {
                (bin as f64 + 0.5) * BIN_MS as f64
            };
            sum_ms += ms * take as f64;
            left -= take;
        }
        let avg = sum_ms / (worst - left) as f64;
        if avg > 0.0 {
            (1000.0 / avg) as f32
        } else {
            0.0
        }
    }

    /// Gives the frame time that this part of the frames (0 to 1) does
    /// not pass, in milliseconds.
    pub fn percentile_ms(&self, part: f32) -> f32 {
        if self.frames == 0 {
            return 0.0;
        }
        let rank = ((part.clamp(0.0, 1.0) as f64 * self.frames as f64).ceil() as u64).max(1);
        let mut seen = 0;
        for (bin, &count) in self.counts().iter().enumerate() {
            seen += count;
            if seen >= rank {
                return ((bin as f32 + 1.0) * BIN_MS).min(self.worst_ms);
            }
        }
        self.worst_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_summary() {
        let mut s = SecondStats::default();
        for _ in 0..19 {
            s.add(10.0);
        }
        s.add(50.0);
        let sum = s.take(0.25);
        assert_eq!(sum.fps, 80.0);
        assert!((sum.avg_ms - 12.0).abs() < 1e-4);
        assert_eq!(sum.p95_ms, 10.0);
        assert_eq!(sum.max_ms, 50.0);
        // The next second starts empty.
        assert_eq!(s.take(1.0), FrameSummary::default());
    }

    #[test]
    fn histogram_average_and_low() {
        let mut h = FrameHistogram::default();
        // 99 frames of 10 ms and 1 frame of 100 ms.
        for _ in 0..99 {
            h.add(10.0);
        }
        h.add(100.0);
        assert_eq!(h.frames, 100);
        assert!((h.average_fps() - 1000.0 / 10.9).abs() < 0.01);
        // The worst 1% is the one 100 ms frame: 10 fps.
        assert!((h.one_percent_low_fps() - 10.0).abs() < 0.1);
        assert_eq!(h.worst_ms, 100.0);
        assert!((h.percentile_ms(0.5) - 10.1).abs() < 0.01);
    }

    #[test]
    fn histogram_survives_a_file() {
        let mut h = FrameHistogram::default();
        for i in 0..1000 {
            h.add(16.0 + (i % 7) as f32);
        }
        h.add(900.0);
        h.pack();
        let text = serde_json::to_string(&h).unwrap();
        let back: FrameHistogram = serde_json::from_str(&text).unwrap();
        assert_eq!(back.frames, h.frames);
        assert_eq!(back.worst_ms, 900.0);
        assert!((back.one_percent_low_fps() - h.one_percent_low_fps()).abs() < 1e-3);
        let mut more = back;
        more.add(16.0);
        assert_eq!(more.frames, 1002);
    }

    #[test]
    fn empty_histogram() {
        let h = FrameHistogram::default();
        assert_eq!(h.average_fps(), 0.0);
        assert_eq!(h.one_percent_low_fps(), 0.0);
        assert_eq!(h.percentile_ms(0.99), 0.0);
    }
}
