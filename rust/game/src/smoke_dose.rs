//! The rule for the smoke at the edge of the zone (R14, R16).
//!
//! This module has no Godot code. The bear can go into the smoke a short
//! distance. A timer counts the seconds in the smoke. If the timer is
//! full, or the bear is too deep, the bear dies. Outside the smoke, the
//! timer goes down.

/// Why the bear died.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The bear stayed in the smoke for too long.
    Time,
    /// The bear went too far into the smoke.
    Depth,
}

impl Cause {
    /// A short name for the playtest events.
    pub fn name(self) -> &'static str {
        match self {
            Cause::Time => "time",
            Cause::Depth => "depth",
        }
    }
}

/// What happened in one step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DoseStep {
    /// The bear went into the smoke in this step.
    pub entered: bool,
    /// The bear came out of the smoke in this step.
    pub left: bool,
    /// The bear died in this step.
    pub died: Option<Cause>,
}

/// The smoke that the bear has taken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmokeDose {
    /// The seconds in the smoke that count now.
    time_in: f32,
    /// The bear is in the smoke now.
    inside: bool,
    /// The last depth in metres (0 outside the smoke).
    depth: f32,
}

impl Default for SmokeDose {
    fn default() -> Self {
        Self::new()
    }
}

impl SmokeDose {
    /// Makes a clean dose.
    pub const fn new() -> Self {
        Self {
            time_in: 0.0,
            inside: false,
            depth: 0.0,
        }
    }

    /// Forgets the dose (after a death or a new zone).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Does one step. `depth_m` is the distance into the smoke in metres
    /// (zero or less: no smoke). `max_seconds` is the time that kills.
    /// `max_depth_m` is the depth that kills. `recovery` is how many times
    /// faster than real time the timer goes down outside the smoke.
    pub fn step(
        &mut self,
        delta: f32,
        depth_m: f32,
        max_seconds: f32,
        max_depth_m: f32,
        recovery: f32,
    ) -> DoseStep {
        let was_inside = self.inside;
        self.inside = depth_m > 0.0;
        self.depth = depth_m.max(0.0);
        if self.inside {
            self.time_in += delta;
        } else {
            self.time_in = (self.time_in - delta * recovery).max(0.0);
        }
        let mut step = DoseStep {
            entered: self.inside && !was_inside,
            left: !self.inside && was_inside,
            died: None,
        };
        if self.time_in >= max_seconds {
            step.died = Some(Cause::Time);
        } else if self.depth >= max_depth_m {
            step.died = Some(Cause::Depth);
        }
        step
    }

    /// Gives how near the bear is to death, from 0 (safe) to 1 (dead). It
    /// is the larger of the time part and the depth part.
    pub fn danger(&self, max_seconds: f32, max_depth_m: f32) -> f32 {
        let by_time = self.time_in / max_seconds.max(0.01);
        let by_depth = self.depth / max_depth_m.max(0.01);
        by_time.max(by_depth).clamp(0.0, 1.0)
    }

    /// The seconds in the smoke that count now.
    pub fn seconds(&self) -> f32 {
        self.time_in
    }

    /// The bear is in the smoke now.
    pub fn is_inside(&self) -> bool {
        self.inside
    }

    /// The depth in metres, or 0 outside the smoke.
    pub fn depth_m(&self) -> f32 {
        self.depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn run(dose: &mut SmokeDose, seconds: f32, depth: f32) -> DoseStep {
        let mut last = DoseStep::default();
        for _ in 0..(seconds / DT) as usize {
            last = dose.step(DT, depth, 12.0, 45.0, 2.0);
            if last.died.is_some() {
                break;
            }
        }
        last
    }

    #[test]
    fn no_smoke_no_danger() {
        let mut d = SmokeDose::new();
        let s = run(&mut d, 30.0, -50.0);
        assert_eq!(s.died, None);
        assert_eq!(d.danger(12.0, 45.0), 0.0);
        assert!(!d.is_inside());
    }

    #[test]
    fn a_short_visit_is_safe() {
        let mut d = SmokeDose::new();
        assert!(run(&mut d, 5.0, 10.0).died.is_none());
        assert!(d.is_inside());
        assert!(d.danger(12.0, 45.0) > 0.3);
        // The bear comes out and the timer goes down twice as fast.
        run(&mut d, 2.6, -5.0);
        assert!(d.seconds() < 0.5, "{}", d.seconds());
    }

    #[test]
    fn a_long_stay_kills_by_time() {
        let mut d = SmokeDose::new();
        let mut cause = None;
        for _ in 0..(20.0 / DT) as usize {
            let s = d.step(DT, 10.0, 12.0, 45.0, 2.0);
            if s.died.is_some() {
                cause = s.died;
                break;
            }
        }
        assert_eq!(cause, Some(Cause::Time));
        assert!(d.seconds() > 11.9 && d.seconds() < 12.1);
    }

    #[test]
    fn going_too_deep_kills_at_once() {
        let mut d = SmokeDose::new();
        d.step(DT, 5.0, 12.0, 45.0, 2.0);
        let s = d.step(DT, 46.0, 12.0, 45.0, 2.0);
        assert_eq!(s.died, Some(Cause::Depth));
    }

    #[test]
    fn enter_and_leave_are_reported_once() {
        let mut d = SmokeDose::new();
        let a = d.step(DT, 1.0, 12.0, 45.0, 2.0);
        let b = d.step(DT, 1.0, 12.0, 45.0, 2.0);
        let c = d.step(DT, -1.0, 12.0, 45.0, 2.0);
        let e = d.step(DT, -1.0, 12.0, 45.0, 2.0);
        assert!(a.entered && !b.entered);
        assert!(c.left && !e.left);
    }

    #[test]
    fn repeated_short_visits_do_not_add_up() {
        let mut d = SmokeDose::new();
        for _ in 0..20 {
            assert!(run(&mut d, 3.0, 8.0).died.is_none());
            run(&mut d, 2.0, -8.0);
        }
        assert!(d.seconds() < 6.0);
    }

    #[test]
    fn reset_clears_the_dose() {
        let mut d = SmokeDose::new();
        run(&mut d, 6.0, 10.0);
        d.reset();
        assert_eq!(d, SmokeDose::new());
    }
}
