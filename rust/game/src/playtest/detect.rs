//! The event detectors of the playtest recorder.
//!
//! Each detector looks at one frame at a time and tells when something
//! is wrong: a hitch, low fps, a stuck bear, a climb loop or a bear
//! below the ground. The rules come from `plan/diagnostics.md`.

use std::collections::VecDeque;

/// The limits of the detectors.
#[derive(Clone, Debug)]
pub struct DetectParams {
    /// A frame longer than this is a hitch, in milliseconds.
    pub hitch_ms: f32,
    /// A hitch must also be this many times longer than the average
    /// frame of the last second. Thus a slow computer does not make a
    /// hitch event in each frame.
    pub hitch_factor: f32,
    /// Below this fps, the fps is low.
    pub low_fps: f32,
    /// The fps must be low for this time before an event, in seconds.
    pub low_fps_seconds: f64,
    /// A held stick and less movement than this is "stuck", in metres.
    pub stuck_distance: f32,
    /// The time window of the stuck test, in seconds.
    pub stuck_seconds: f64,
    /// More grabs than this in the window is a climb loop.
    pub loop_grabs: usize,
    /// The time window of the climb loop test, in seconds.
    pub loop_seconds: f64,
    /// Grabs nearer than this are at the same place, in metres.
    pub loop_radius: f32,
    /// The bear is "through the ground" this far below the terrain, in metres.
    pub ground_tolerance: f32,
}

impl Default for DetectParams {
    fn default() -> Self {
        Self {
            hitch_ms: 50.0,
            hitch_factor: 2.0,
            low_fps: 55.0,
            low_fps_seconds: 2.0,
            stuck_distance: 0.05,
            stuck_seconds: 2.0,
            loop_grabs: 4,
            loop_seconds: 10.0,
            loop_radius: 1.0,
            ground_tolerance: 0.3,
        }
    }
}

/// What the detectors know about one frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameInput {
    /// The time since the start of the session, in seconds.
    pub t: f64,
    /// The time of the frame in milliseconds.
    pub frame_ms: f32,
    /// The feet of the bear, if there is a bear.
    pub bear: Option<[f32; 3]>,
    /// The player pushes the stick (or a move key) and the bear gets it.
    pub stick_held: bool,
    /// The terrain height below the bear, if there is a zone.
    pub ground: Option<f32>,
}

/// Something that a detector found.
#[derive(Clone, Debug, PartialEq)]
pub enum Found {
    /// One frame took this many milliseconds.
    Hitch { ms: f32 },
    /// The fps was low for `low_fps_seconds`. `fps` is the fps now.
    LowFpsStart { fps: f32 },
    /// The fps is good again after this time, in seconds.
    LowFpsEnd { seconds: f64, min_fps: f32 },
    /// The stick was held, but the bear did not move.
    Stuck,
    /// The bear grabbed this many times at one place in the window.
    ClimbLoop { grabs: usize },
    /// The bear is this far below the terrain, in metres.
    ThroughGround { depth: f32 },
}

/// All detectors and their memory.
#[derive(Clone, Debug, Default)]
pub struct Detectors {
    pub params: DetectParams,
    /// The frame times of the last second: (time, ms).
    recent: VecDeque<(f64, f32)>,
    /// The time when the fps became low, if it is low now.
    low_since: Option<f64>,
    low_reported: bool,
    low_min_fps: f32,
    /// Bear positions while the stick is held: (time, position).
    held: VecDeque<(f64, [f32; 3])>,
    stuck_reported: bool,
    /// The times and places of recent grabs.
    grabs: VecDeque<(f64, [f32; 3])>,
    loop_quiet_until: f64,
    ground_quiet_until: f64,
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

impl Detectors {
    /// Looks at one frame. Gives all that the detectors found in it.
    pub fn frame(&mut self, f: FrameInput) -> Vec<Found> {
        let mut found = Vec::new();
        let recent_avg = if self.recent.is_empty() {
            0.0
        } else {
            self.recent.iter().map(|(_, ms)| ms).sum::<f32>() / self.recent.len() as f32
        };
        if f.frame_ms > self.params.hitch_ms && f.frame_ms > recent_avg * self.params.hitch_factor {
            found.push(Found::Hitch { ms: f.frame_ms });
        }
        self.low_fps(&f, &mut found);
        self.stuck(&f, &mut found);
        self.ground(&f, &mut found);
        found
    }

    fn low_fps(&mut self, f: &FrameInput, found: &mut Vec<Found>) {
        self.recent.push_back((f.t, f.frame_ms));
        while let Some(&(t, _)) = self.recent.front() {
            if f.t - t > 1.0 {
                self.recent.pop_front();
            } else {
                break;
            }
        }
        let total_ms: f32 = self.recent.iter().map(|(_, ms)| ms).sum();
        // Wait for nearly one full second of frames.
        if total_ms < 900.0 {
            return;
        }
        let fps = self.recent.len() as f32 * 1000.0 / total_ms;
        if fps < self.params.low_fps {
            let since = *self.low_since.get_or_insert(f.t);
            if !self.low_reported {
                self.low_min_fps = fps;
            }
            self.low_min_fps = self.low_min_fps.min(fps);
            if !self.low_reported && f.t - since >= self.params.low_fps_seconds {
                self.low_reported = true;
                found.push(Found::LowFpsStart { fps });
            }
        } else if let Some(since) = self.low_since.take() {
            if self.low_reported {
                found.push(Found::LowFpsEnd {
                    seconds: f.t - since,
                    min_fps: self.low_min_fps,
                });
            }
            self.low_reported = false;
        }
    }

    fn stuck(&mut self, f: &FrameInput, found: &mut Vec<Found>) {
        let (Some(bear), true) = (f.bear, f.stick_held) else {
            self.held.clear();
            self.stuck_reported = false;
            return;
        };
        self.held.push_back((f.t, bear));
        // Keep one position older than the window.
        while self.held.len() > 2 && f.t - self.held[1].0 >= self.params.stuck_seconds {
            self.held.pop_front();
        }
        let Some(&(start, _)) = self.held.front() else {
            return;
        };
        if f.t - start < self.params.stuck_seconds {
            return;
        }
        let window_start = f.t - self.params.stuck_seconds;
        let moved = self
            .held
            .iter()
            .filter(|(t, _)| *t >= window_start)
            .map(|(_, p)| distance(*p, bear))
            .fold(distance(self.held[0].1, bear), f32::max);
        if moved < self.params.stuck_distance {
            if !self.stuck_reported {
                self.stuck_reported = true;
                found.push(Found::Stuck);
            }
        } else {
            self.stuck_reported = false;
        }
    }

    fn ground(&mut self, f: &FrameInput, found: &mut Vec<Found>) {
        let (Some(bear), Some(ground)) = (f.bear, f.ground) else {
            return;
        };
        let depth = ground - bear[1];
        if depth > self.params.ground_tolerance && f.t >= self.ground_quiet_until {
            // At most one event each second.
            self.ground_quiet_until = f.t + 1.0;
            found.push(Found::ThroughGround { depth });
        }
    }

    /// Tells the detectors that the bear grabbed a wall at a place.
    pub fn grab(&mut self, t: f64, at: [f32; 3]) -> Option<Found> {
        self.grabs.push_back((t, at));
        while let Some(&(old, _)) = self.grabs.front() {
            if t - old > self.params.loop_seconds {
                self.grabs.pop_front();
            } else {
                break;
            }
        }
        let near = self
            .grabs
            .iter()
            .filter(|(_, p)| distance(*p, at) <= self.params.loop_radius)
            .count();
        if near > self.params.loop_grabs && t >= self.loop_quiet_until {
            // One event for each loop, not one for each grab.
            self.loop_quiet_until = t + self.params.loop_seconds;
            return Some(Found::ClimbLoop { grabs: near });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs frames of a fixed time for some seconds. Gives all finds.
    fn run(
        d: &mut Detectors,
        t: &mut f64,
        seconds: f64,
        ms: f32,
        make: impl Fn(f64) -> FrameInput,
    ) -> Vec<Found> {
        let mut found = Vec::new();
        let end = *t + seconds;
        while *t < end {
            *t += ms as f64 / 1000.0;
            let mut f = make(*t);
            f.t = *t;
            f.frame_ms = ms;
            found.extend(d.frame(f));
        }
        found
    }

    #[test]
    fn hitch() {
        let mut d = Detectors::default();
        let f = FrameInput {
            t: 1.0,
            frame_ms: 80.0,
            ..Default::default()
        };
        assert_eq!(d.frame(f), vec![Found::Hitch { ms: 80.0 }]);
        let f = FrameInput {
            frame_ms: 40.0,
            ..f
        };
        assert!(d.frame(f).is_empty());
    }

    #[test]
    fn slow_frames_are_not_all_hitches() {
        let mut d = Detectors::default();
        let mut t = 0.0;
        let none = |_| FrameInput::default();
        // 10 fps: each frame is longer than 50 ms, but none is a spike.
        // (The first frame has no history, so it can be a hitch.)
        run(&mut d, &mut t, 0.2, 100.0, none);
        let found = run(&mut d, &mut t, 3.0, 100.0, none);
        assert!(!found.iter().any(|f| matches!(f, Found::Hitch { .. })));
        // A frame of 300 ms among them is a hitch.
        let f = FrameInput {
            t: t + 0.3,
            frame_ms: 300.0,
            ..Default::default()
        };
        assert!(d.frame(f).contains(&Found::Hitch { ms: 300.0 }));
    }

    #[test]
    fn low_fps_needs_two_seconds() {
        let mut d = Detectors::default();
        let mut t = 0.0;
        let none = |_| FrameInput::default();
        assert!(run(&mut d, &mut t, 5.0, 16.0, none).is_empty());
        // 40 fps for 1.5 s: no event yet.
        assert!(run(&mut d, &mut t, 1.5, 25.0, none).is_empty());
        // Two more seconds: one start event.
        let found = run(&mut d, &mut t, 2.0, 25.0, none);
        assert_eq!(found.len(), 1);
        assert!(matches!(found[0], Found::LowFpsStart { fps } if fps < 45.0));
        // Good fps again: one end event.
        let found = run(&mut d, &mut t, 2.0, 16.0, none);
        assert_eq!(found.len(), 1);
        let Found::LowFpsEnd { seconds, min_fps } = found[0] else {
            panic!("{found:?}");
        };
        assert!(seconds > 3.0, "{seconds}");
        assert!((min_fps - 40.0).abs() < 1.0, "{min_fps}");
    }

    #[test]
    fn short_dip_is_not_low_fps() {
        let mut d = Detectors::default();
        let mut t = 0.0;
        let none = |_| FrameInput::default();
        run(&mut d, &mut t, 3.0, 16.0, none);
        assert!(run(&mut d, &mut t, 1.0, 30.0, none).is_empty());
        assert!(run(&mut d, &mut t, 3.0, 16.0, none).is_empty());
    }

    #[test]
    fn stuck_once_per_push() {
        let mut d = Detectors::default();
        let mut t = 0.0;
        let still = |_| FrameInput {
            bear: Some([1.0, 2.0, 3.0]),
            stick_held: true,
            ..Default::default()
        };
        assert!(run(&mut d, &mut t, 1.9, 16.0, still).is_empty());
        assert_eq!(run(&mut d, &mut t, 3.0, 16.0, still), vec![Found::Stuck]);
        // Let go of the stick, then push again: a new event.
        run(&mut d, &mut t, 0.1, 16.0, |_| FrameInput::default());
        assert_eq!(run(&mut d, &mut t, 2.5, 16.0, still), vec![Found::Stuck]);
    }

    #[test]
    fn a_moving_bear_is_not_stuck() {
        let mut d = Detectors::default();
        let mut t = 0.0;
        let walk = |t: f64| FrameInput {
            bear: Some([t as f32 * 1.4, 0.0, 0.0]),
            stick_held: true,
            ..Default::default()
        };
        assert!(run(&mut d, &mut t, 5.0, 16.0, walk).is_empty());
        // No stick: no event, also if the bear stands still.
        let rest = |_| FrameInput {
            bear: Some([0.0; 3]),
            ..Default::default()
        };
        assert!(run(&mut d, &mut t, 5.0, 16.0, rest).is_empty());
    }

    #[test]
    fn climb_loop() {
        let mut d = Detectors::default();
        let at = [5.0, 10.0, 5.0];
        for i in 0..4 {
            assert_eq!(d.grab(i as f64, at), None);
        }
        // The fifth grab at the same place in 10 s is a loop.
        assert_eq!(d.grab(4.0, at), Some(Found::ClimbLoop { grabs: 5 }));
        // The next grabs in the same loop give no new event.
        assert_eq!(d.grab(5.0, at), None);
        // Grabs at different places are not a loop.
        let mut d = Detectors::default();
        for i in 0..10 {
            assert_eq!(d.grab(i as f64, [i as f32 * 3.0, 0.0, 0.0]), None);
        }
        // Slow grabs are not a loop.
        let mut d = Detectors::default();
        for i in 0..10 {
            assert_eq!(d.grab(i as f64 * 3.0, at), None);
        }
    }

    #[test]
    fn through_the_ground() {
        let mut d = Detectors::default();
        let f = FrameInput {
            t: 1.0,
            bear: Some([0.0, 9.0, 0.0]),
            ground: Some(10.0),
            ..Default::default()
        };
        assert_eq!(d.frame(f), vec![Found::ThroughGround { depth: 1.0 }]);
        // Not again in the same second.
        assert!(d.frame(FrameInput { t: 1.5, ..f }).is_empty());
        assert_eq!(d.frame(FrameInput { t: 2.1, ..f }).len(), 1);
        // A small difference is normal.
        let f = FrameInput {
            t: 5.0,
            bear: Some([0.0, 9.8, 0.0]),
            ..f
        };
        assert!(d.frame(f).is_empty());
    }
}
