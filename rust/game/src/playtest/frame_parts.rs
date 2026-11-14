//! Where the time of one frame went (S-136).
//!
//! The recorder reads the clock at five points of each frame. The
//! times between them tell if a long frame came from the game code or
//! from the graphics driver. On Mikey's laptop, freezes of 1 s came
//! from the present call of the driver, not from the game.

use serde::{Deserialize, Serialize};

/// The clock at the points of one frame, in microseconds. A frame of
/// the recorder goes from the end of the scripts of one frame to the
/// end of the scripts of the next frame. 0 means "not seen".
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameMarks {
    /// The end of the scripts of the frame before (the recorder runs last).
    pub start: u64,
    /// `RenderingServer.frame_pre_draw`: the draw starts.
    pub pre_draw: u64,
    /// `RenderingServer.frame_post_draw`: the draw and the present are done.
    pub post_draw: u64,
    /// The first `SceneTree.physics_frame` after the draw (0: no physics step).
    pub physics: u64,
    /// `SceneTree.process_frame`: the scripts start.
    pub process: u64,
    /// The recorder runs: the scripts are done.
    pub end: u64,
}

/// The parts of one frame, in milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameParts {
    /// The draw and the present. The present waits for the driver and
    /// the compositor (vsync).
    pub draw_ms: f32,
    /// After the draw, before the physics and the scripts: window
    /// events, input, and the sleep of an fps limit.
    pub idle_ms: f32,
    /// All physics steps of the frame (Jolt and `_physics_process`).
    pub physics_ms: f32,
    /// All `_process` functions of the frame.
    pub script_ms: f32,
    /// The rest: the recorder itself, screenshots, queued calls.
    pub other_ms: f32,
}

impl FrameMarks {
    /// The parts of the frame. `None` if a point is missing or the
    /// points are not in order (the first frame, or no draw).
    pub fn parts(&self) -> Option<FrameParts> {
        let m = self;
        let after_draw = if m.physics >= m.post_draw && m.physics > 0 {
            m.physics
        } else {
            m.process
        };
        let ordered = m.start > 0
            && m.start <= m.pre_draw
            && m.pre_draw <= m.post_draw
            && m.post_draw <= after_draw
            && after_draw <= m.process
            && m.process <= m.end;
        if !ordered {
            return None;
        }
        let ms = |from: u64, to: u64| (to - from) as f32 / 1000.0;
        Some(FrameParts {
            draw_ms: ms(m.pre_draw, m.post_draw),
            idle_ms: ms(m.post_draw, after_draw),
            physics_ms: ms(after_draw, m.process),
            script_ms: ms(m.process, m.end),
            other_ms: ms(m.start, m.pre_draw),
        })
    }
}

impl FrameParts {
    /// True if the game code (scripts and physics) used less than half
    /// of the frame. Then the cause is the driver, the compositor or
    /// the operating system.
    pub fn outside_game(&self) -> bool {
        let game = self.physics_ms + self.script_ms + self.other_ms;
        game < self.draw_ms + self.idle_ms
    }

    /// A short text for the event: "draw and present 945, idle 1,
    /// physics 2, script 2, other 0 ms".
    pub fn text(&self) -> String {
        format!(
            "draw and present {:.0}, idle {:.0}, physics {:.0}, script {:.0}, other {:.0} ms",
            self.draw_ms, self.idle_ms, self.physics_ms, self.script_ms, self.other_ms
        )
    }

    /// Adds a frame to a sum.
    pub fn add(&mut self, o: &FrameParts) {
        self.draw_ms += o.draw_ms;
        self.idle_ms += o.idle_ms;
        self.physics_ms += o.physics_ms;
        self.script_ms += o.script_ms;
        self.other_ms += o.other_ms;
    }

    /// The sum divided by the number of frames.
    pub fn divided(&self, n: u32) -> FrameParts {
        let n = n.max(1) as f32;
        FrameParts {
            draw_ms: self.draw_ms / n,
            idle_ms: self.idle_ms / n,
            physics_ms: self.physics_ms / n,
            script_ms: self.script_ms / n,
            other_ms: self.other_ms / n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks() -> FrameMarks {
        FrameMarks {
            start: 1_000,
            pre_draw: 1_500,
            post_draw: 951_500,
            physics: 952_000,
            process: 954_000,
            end: 955_000,
        }
    }

    #[test]
    fn a_present_freeze_shows_in_the_draw_part() {
        let p = marks().parts().unwrap();
        assert_eq!(p.draw_ms, 950.0);
        assert_eq!(p.idle_ms, 0.5);
        assert_eq!(p.physics_ms, 2.0);
        assert_eq!(p.script_ms, 1.0);
        assert_eq!(p.other_ms, 0.5);
        assert!(p.outside_game());
        assert!(p.text().starts_with("draw and present 950, idle 0"));
    }

    #[test]
    fn a_frame_without_a_physics_step_has_no_physics_time() {
        let mut m = marks();
        m.physics = 500; // from an older frame
        let p = m.parts().unwrap();
        assert_eq!(p.physics_ms, 0.0);
        assert_eq!(p.idle_ms, 2.5);
    }

    #[test]
    fn a_slow_script_is_inside_the_game() {
        let mut m = marks();
        m.post_draw = 2_000;
        m.physics = 2_100;
        m.process = 2_200;
        let p = m.parts().unwrap();
        assert_eq!(p.script_ms, 952.8);
        assert!(!p.outside_game());
    }

    #[test]
    fn missing_or_mixed_points_give_none() {
        assert!(FrameMarks::default().parts().is_none());
        let mut m = marks();
        m.pre_draw = 0;
        assert!(m.parts().is_none());
        let mut m = marks();
        m.process = 100;
        assert!(m.parts().is_none());
    }

    #[test]
    fn the_average_divides_the_sum() {
        let p = marks().parts().unwrap();
        let mut sum = FrameParts::default();
        sum.add(&p);
        sum.add(&p);
        assert_eq!(sum.divided(2), p);
    }
}
