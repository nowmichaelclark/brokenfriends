//! The seams of the limbs and how they tear (R12), with no Godot types.
//! Unit tests cover this module.
//!
//! Each limb has a seam health from 0 to 1. A hard hit lowers it. The
//! health gives the stage of the seam:
//! 1. Intact (above 0.7).
//! 2. Frayed (0.4 to 0.7): the joint is a little looser.
//! 3. Torn (0.1 to 0.4): the limb droops, and stuffing comes out on hits.
//! 4. Dangling (above 0 and below 0.1): the limb hangs on a thread. It
//!    counts as missing for the gait. The thread wears while the bear
//!    moves.
//! 5. Detached (0): the limb falls off and stays in the world.
//!
//! Sewing gives a limb a new seam with full health.

use crate::anim::LIMBS;
use crate::limbs::LimbSet;

/// The stage of a seam, from its health.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Intact,
    Frayed,
    Torn,
    Dangling,
    Detached,
}

impl Stage {
    /// Gives the stage of a health value.
    pub fn of(health: f32) -> Stage {
        if health <= 0.0 {
            Stage::Detached
        } else if health < 0.1 {
            Stage::Dangling
        } else if health < 0.4 {
            Stage::Torn
        } else if health <= 0.7 {
            Stage::Frayed
        } else {
            Stage::Intact
        }
    }

    /// A short name for the recorder, for example "torn".
    pub fn name(self) -> &'static str {
        match self {
            Stage::Intact => "intact",
            Stage::Frayed => "frayed",
            Stage::Torn => "torn",
            Stage::Dangling => "dangling",
            Stage::Detached => "detached",
        }
    }

    /// Tells if the limb is on the body and works (intact, frayed or
    /// torn). A dangling limb does not work.
    pub fn works(self) -> bool {
        self < Stage::Dangling
    }
}

/// The tuning values of the seams. The bear copies them from its
/// exported fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TearParams {
    /// A hit on a limb slower than this speed does no damage, in m/s.
    pub damage_speed: f32,
    /// The health that each m/s above `damage_speed` takes away.
    pub damage_per_speed: f32,
    /// The health that a dangling thread loses for each metre that the
    /// bear moves.
    pub thread_wear_per_m: f32,
}

impl Default for TearParams {
    fn default() -> TearParams {
        TearParams {
            damage_speed: 7.0,
            damage_per_speed: 0.09,
            thread_wear_per_m: 0.012,
        }
    }
}

/// A change of the stage of one seam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Change {
    /// The limb, in the order of [`LIMBS`].
    pub limb: usize,
    pub from: Stage,
    pub to: Stage,
}

/// The seams of the four limbs, in the order of [`LIMBS`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seams {
    health: [f32; 4],
}

impl Default for Seams {
    fn default() -> Seams {
        Seams { health: [1.0; 4] }
    }
}

impl Seams {
    /// The health of a seam (0 to 1).
    pub fn health(&self, limb: usize) -> f32 {
        self.health.get(limb).copied().unwrap_or(0.0)
    }

    pub fn stage(&self, limb: usize) -> Stage {
        Stage::of(self.health(limb))
    }

    /// Sets the health of a seam and tells if the stage changed.
    pub fn set(&mut self, limb: usize, health: f32) -> Option<Change> {
        if limb >= 4 {
            return None;
        }
        let from = self.stage(limb);
        self.health[limb] = health.clamp(0.0, 1.0);
        let to = self.stage(limb);
        (from != to).then_some(Change { limb, from, to })
    }

    /// Takes health away from a seam. A detached limb takes no damage.
    pub fn damage(&mut self, limb: usize, amount: f32) -> Option<Change> {
        if amount <= 0.0 || self.stage(limb) == Stage::Detached {
            return None;
        }
        self.set(limb, self.health(limb) - amount)
    }

    /// A hit on a limb at a speed in m/s. Gives the health that the hit
    /// takes away (0 below the damage speed) and the change of stage.
    pub fn hit(&mut self, limb: usize, speed: f32, p: &TearParams) -> (f32, Option<Change>) {
        let amount = damage_of(speed, p);
        if amount <= 0.0 {
            return (0.0, None);
        }
        (amount, self.damage(limb, amount))
    }

    /// The bear moved some metres: each dangling thread wears.
    pub fn wear(&mut self, metres: f32, p: &TearParams) -> Vec<Change> {
        let dangling: Vec<usize> = (0..4)
            .filter(|&i| self.stage(i) == Stage::Dangling)
            .collect();
        dangling
            .into_iter()
            .filter_map(|i| self.damage(i, metres.max(0.0) * p.thread_wear_per_m))
            .collect()
    }

    /// Sews a limb on again: the seam is new.
    pub fn mend(&mut self, limb: usize) -> Option<Change> {
        self.set(limb, 1.0)
    }

    /// The limbs that work (not dangling and not detached).
    pub fn working(&self) -> LimbSet {
        (0..4).fold(LimbSet::ALL, |s, i| s.with(i, self.stage(i).works()))
    }

    /// How loose a limb is on its seam, from 0 (intact) to 1 (dangling).
    pub fn looseness(&self, limb: usize) -> f32 {
        match self.stage(limb) {
            Stage::Intact => 0.0,
            // From 0 at 0.7 to 0.4 at 0.4, then to 1 at 0.1.
            Stage::Frayed => (0.7 - self.health(limb)) / 0.3 * 0.4,
            Stage::Torn => 0.4 + (0.4 - self.health(limb)) / 0.3 * 0.6,
            Stage::Dangling | Stage::Detached => 1.0,
        }
    }

    /// The name of a limb, for example "LeftArm".
    pub fn limb_name(limb: usize) -> &'static str {
        LIMBS.get(limb).map_or("", |l| l.name())
    }
}

/// The health that a hit at a speed takes away.
pub fn damage_of(speed: f32, p: &TearParams) -> f32 {
    (speed - p.damage_speed).max(0.0) * p.damage_per_speed.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_health_gives_the_stage() {
        assert_eq!(Stage::of(1.0), Stage::Intact);
        assert_eq!(Stage::of(0.71), Stage::Intact);
        assert_eq!(Stage::of(0.7), Stage::Frayed);
        assert_eq!(Stage::of(0.39), Stage::Torn);
        assert_eq!(Stage::of(0.09), Stage::Dangling);
        assert_eq!(Stage::of(0.0), Stage::Detached);
        assert!(Stage::Torn.works() && !Stage::Dangling.works());
    }

    #[test]
    fn a_slow_hit_does_nothing_and_a_hard_hit_tears() {
        let p = TearParams::default();
        let mut s = Seams::default();
        assert_eq!(s.hit(0, p.damage_speed, &p), (0.0, None));
        assert_eq!(s.health(0), 1.0);
        // 15 m/s: 8 m/s too fast, 0.72 of the health.
        let (amount, change) = s.hit(2, 15.0, &p);
        assert!((amount - 0.72).abs() < 1e-5);
        assert_eq!(
            change,
            Some(Change {
                limb: 2,
                from: Stage::Intact,
                to: Stage::Torn
            })
        );
        // A second hit like it takes the leg off.
        let (_, change) = s.hit(2, 15.0, &p);
        assert_eq!(change.map(|c| c.to), Some(Stage::Detached));
        assert_eq!(s.working(), LimbSet::ALL.with(2, false));
        // A detached limb takes no more damage.
        assert_eq!(s.hit(2, 30.0, &p).1, None);
    }

    #[test]
    fn a_dangling_thread_wears_and_breaks() {
        let p = TearParams::default();
        let mut s = Seams::default();
        s.set(1, 0.05);
        assert_eq!(s.stage(1), Stage::Dangling);
        assert!(!s.working().has(1));
        // The other seams do not wear.
        assert!(s.wear(1.0, &p).is_empty());
        assert_eq!(s.health(0), 1.0);
        let metres = 0.05 / p.thread_wear_per_m;
        let changes = s.wear(metres + 0.01, &p);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].to, Stage::Detached);
    }

    #[test]
    fn looseness_grows_as_the_seam_tears_and_mending_resets_it() {
        let mut s = Seams::default();
        let mut last = -1.0;
        for h in [1.0, 0.7, 0.55, 0.4, 0.25, 0.1, 0.05] {
            s.set(3, h);
            let l = s.looseness(3);
            assert!(l >= last, "{h}: {l} < {last}");
            assert!((0.0..=1.0).contains(&l));
            last = l;
        }
        assert_eq!(s.looseness(3), 1.0);
        let change = s.mend(3).unwrap();
        assert_eq!((change.from, change.to), (Stage::Dangling, Stage::Intact));
        assert_eq!(s.looseness(3), 0.0);
        assert_eq!(Seams::limb_name(3), "RightLeg");
    }
}
