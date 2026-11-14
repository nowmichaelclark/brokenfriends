//! The limbs that the bear still has, and the gait table (R12, R18),
//! with no Godot types. Unit tests cover this module.
//!
//! A [`LimbSet`] tells which arms and legs the bear has. The gait table
//! ([`profile`]) changes the limb set into a [`GaitProfile`]: the way
//! the bear moves on the ground, its speed, its jump and its climb
//! limits. The movement code (`bear.rs`) and the animation (`anim.rs`)
//! both read the profile.

/// The arms and the legs that the bear has. Bit `i` is the limb
/// `LIMBS[i]`: left arm, right arm, left leg, right leg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimbSet(u8);

impl Default for LimbSet {
    fn default() -> LimbSet {
        LimbSet::ALL
    }
}

impl LimbSet {
    /// All four limbs.
    pub const ALL: LimbSet = LimbSet(0b1111);

    /// Tells if the bear has the limb `LIMBS[i]`.
    pub fn has(self, i: usize) -> bool {
        i < 4 && self.0 & (1 << i) != 0
    }

    /// Gives the set with the limb `LIMBS[i]` added or removed.
    pub fn with(self, i: usize, present: bool) -> LimbSet {
        if i >= 4 {
            return self;
        }
        if present {
            LimbSet(self.0 | (1 << i))
        } else {
            LimbSet(self.0 & !(1 << i))
        }
    }

    /// The number of arms (0 to 2).
    pub fn arms(self) -> usize {
        self.has(0) as usize + self.has(1) as usize
    }

    /// The number of legs (0 to 2).
    pub fn legs(self) -> usize {
        self.has(2) as usize + self.has(3) as usize
    }

    /// The number of limbs (0 to 4).
    pub fn count(self) -> usize {
        self.arms() + self.legs()
    }

    /// A short text for the debug overlay and the recorder, for example
    /// "LA RA LL --": the missing limbs are "--".
    pub fn label(self) -> String {
        let names = ["LA", "RA", "LL", "RL"];
        let parts: Vec<&str> = (0..4)
            .map(|i| if self.has(i) { names[i] } else { "--" })
            .collect();
        parts.join(" ")
    }
}

/// The way the bear moves on the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gait {
    /// Two legs: walk, jog and jump.
    Walk,
    /// One leg: the bear hops. `leg` is 0 (left) or 1 (right).
    Hop { leg: usize },
    /// No legs, two arms: the bear lies on its belly and pulls with
    /// the arms in turn.
    ArmCrawl,
    /// No legs, one arm: pull, rest, and roll a little. `arm` is 0
    /// (left) or 1 (right).
    OneArmDrag { arm: usize },
    /// No limbs: the bear wriggles and rolls, very slowly.
    Wriggle,
}

impl Gait {
    /// A short name for the debug overlay and the recorder.
    pub fn name(self) -> &'static str {
        match self {
            Gait::Walk => "walk",
            Gait::Hop { .. } => "hop",
            Gait::ArmCrawl => "arm_crawl",
            Gait::OneArmDrag { .. } => "one_arm_drag",
            Gait::Wriggle => "wriggle",
        }
    }

    /// Tells if the bear lies on its belly in this gait.
    pub fn lies(self) -> bool {
        matches!(
            self,
            Gait::ArmCrawl | Gait::OneArmDrag { .. } | Gait::Wriggle
        )
    }
}

/// The climb limits of a limb set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClimbProfile {
    /// The bear can hold a wall.
    pub can_climb: bool,
    /// The steepest surface that the bear can hold, in degrees. `None`
    /// is the full limit of the bear (`climb_max_degrees`).
    pub max_degrees: Option<f32>,
    /// The stamina use is multiplied by this value.
    pub stamina_factor: f32,
}

/// What a limb set can do: one cell of the gait table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaitProfile {
    pub gait: Gait,
    /// The walk and jog speeds are multiplied by this value.
    pub speed_scale: f32,
    /// The jump height is multiplied by this value. 0: no jump.
    pub jump_scale: f32,
    /// The length of the strides is multiplied by this value.
    pub stride_scale: f32,
    pub climb: ClimbProfile,
}

impl GaitProfile {
    /// Tells if the bear can jump.
    pub fn can_jump(&self) -> bool {
        self.jump_scale > 0.0
    }
}

/// The tuning values of the gait table. The bear copies them from its
/// exported fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaitTuning {
    /// The speed of a bear with two legs and fewer than two arms.
    pub armless_walk_speed: f32,
    /// The speed of a hop on one leg, with two arms.
    pub hop_speed: f32,
    /// The hop gets slower by this part for each missing arm.
    pub hop_arm_slowdown: f32,
    /// The jump of a bear on one leg.
    pub hop_jump: f32,
    /// The speed of an arm-crawl.
    pub crawl_speed: f32,
    /// The speed of a one-arm drag.
    pub drag_speed: f32,
    /// The speed of a wriggle.
    pub wriggle_speed: f32,
    /// The steepest surface that a bear with one arm can hold, in degrees.
    pub one_arm_max_degrees: f32,
    /// The stamina factor of a bear with one arm.
    pub one_arm_stamina: f32,
    /// The stamina factor of a bear with one leg.
    pub one_leg_stamina: f32,
    /// The stamina factor of a bear with no legs.
    pub no_legs_stamina: f32,
}

impl Default for GaitTuning {
    fn default() -> GaitTuning {
        GaitTuning {
            armless_walk_speed: 0.95,
            hop_speed: 0.6,
            hop_arm_slowdown: 0.15,
            hop_jump: 0.6,
            crawl_speed: 0.3,
            drag_speed: 0.18,
            wriggle_speed: 0.1,
            one_arm_max_degrees: 68.0,
            one_arm_stamina: 1.6,
            one_leg_stamina: 1.15,
            no_legs_stamina: 1.3,
        }
    }
}

/// The gait table. It gives what a limb set can do.
///
/// | Legs | 2 arms | 1 arm | 0 arms |
/// |---|---|---|---|
/// | 2 | walk; full climb | walk; limited climb | walk; no climb |
/// | 1 | hop | slower hop; limited climb | slow hop, small jumps; no climb |
/// | 0 | arm-crawl; full climb | one-arm drag; limited climb | wriggle; no climb |
pub fn profile(limbs: LimbSet, t: &GaitTuning) -> GaitProfile {
    let arms = limbs.arms();
    let legs = limbs.legs();
    let missing_arms = (2 - arms) as f32;
    let (gait, speed_scale, jump_scale, stride_scale) = match legs {
        2 => {
            let speed = if arms == 2 { 1.0 } else { t.armless_walk_speed };
            (Gait::Walk, speed, 1.0, 1.0)
        }
        1 => {
            let leg = if limbs.has(2) { 0 } else { 1 };
            let slow = (1.0 - t.hop_arm_slowdown * missing_arms).max(0.1);
            // Without arms, the bear cannot keep its balance in a big jump.
            let jump = if arms == 0 {
                0.6 * t.hop_jump
            } else {
                t.hop_jump
            };
            (Gait::Hop { leg }, t.hop_speed * slow, jump, 1.25)
        }
        _ => match arms {
            2 => (Gait::ArmCrawl, t.crawl_speed, 0.0, 0.8),
            1 => {
                let arm = if limbs.has(0) { 0 } else { 1 };
                (Gait::OneArmDrag { arm }, t.drag_speed, 0.0, 0.9)
            }
            _ => (Gait::Wriggle, t.wriggle_speed, 0.0, 0.6),
        },
    };
    let mut stamina_factor = match legs {
        2 => 1.0,
        1 => t.one_leg_stamina,
        _ => t.no_legs_stamina,
    };
    if arms == 1 {
        stamina_factor *= t.one_arm_stamina;
    }
    let climb = ClimbProfile {
        can_climb: arms > 0,
        max_degrees: (arms == 1).then_some(t.one_arm_max_degrees),
        stamina_factor,
    };
    GaitProfile {
        gait,
        speed_scale,
        jump_scale,
        stride_scale,
        climb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(arms: [bool; 2], legs: [bool; 2]) -> LimbSet {
        LimbSet::ALL
            .with(0, arms[0])
            .with(1, arms[1])
            .with(2, legs[0])
            .with(3, legs[1])
    }

    #[test]
    fn a_limb_set_adds_and_removes_limbs() {
        let s = LimbSet::ALL.with(1, false).with(3, false);
        assert!(s.has(0) && !s.has(1) && s.has(2) && !s.has(3));
        assert_eq!((s.arms(), s.legs(), s.count()), (1, 1, 2));
        assert_eq!(s.label(), "LA -- LL --");
        assert_eq!(s.with(1, true).with(3, true), LimbSet::ALL);
        assert_eq!(s.with(7, false), s);
    }

    #[test]
    fn the_table_gives_the_gait_of_each_limb_set() {
        let t = GaitTuning::default();
        let gait = |arms, legs| profile(set(arms, legs), &t).gait;
        let both = [true, true];
        let none = [false, false];
        assert_eq!(gait(both, both), Gait::Walk);
        assert_eq!(gait(none, both), Gait::Walk);
        assert_eq!(gait(both, [false, true]), Gait::Hop { leg: 1 });
        assert_eq!(gait(both, [true, false]), Gait::Hop { leg: 0 });
        assert_eq!(gait(both, none), Gait::ArmCrawl);
        assert_eq!(gait([false, true], none), Gait::OneArmDrag { arm: 1 });
        assert_eq!(gait(none, none), Gait::Wriggle);
        assert!(Gait::Wriggle.lies() && !Gait::Hop { leg: 0 }.lies());
    }

    #[test]
    fn fewer_limbs_are_slower_and_climb_less() {
        let t = GaitTuning::default();
        let full = profile(LimbSet::ALL, &t);
        assert_eq!(full.speed_scale, 1.0);
        assert_eq!(full.climb.max_degrees, None);
        assert_eq!(full.climb.stamina_factor, 1.0);

        // Each step down the table is slower.
        let order = [
            set([true, true], [true, true]),
            set([true, true], [true, false]),
            set([true, false], [true, false]),
            set([false, false], [true, false]),
            set([true, true], [false, false]),
            set([true, false], [false, false]),
            set([false, false], [false, false]),
        ];
        let speeds: Vec<f32> = order.iter().map(|&s| profile(s, &t).speed_scale).collect();
        assert!(speeds.windows(2).all(|w| w[1] < w[0]), "{speeds:?}");

        // One arm: a limited climb that costs more. No arms: no climb.
        let one_arm = profile(set([false, true], [true, true]), &t);
        assert!(one_arm.climb.can_climb);
        assert_eq!(one_arm.climb.max_degrees, Some(t.one_arm_max_degrees));
        assert!((one_arm.climb.stamina_factor - 1.6).abs() < 1e-6);
        assert!(
            !profile(set([false, false], [true, true]), &t)
                .climb
                .can_climb
        );
        // No legs: the arms climb, but at a higher cost.
        let crawl = profile(set([true, true], [false, false]), &t);
        assert!(crawl.climb.can_climb && crawl.climb.max_degrees.is_none());
        assert!((crawl.climb.stamina_factor - 1.3).abs() < 1e-6);
        assert!(!crawl.can_jump());

        // One leg: smaller jumps, and even smaller without arms.
        let hop = profile(set([true, true], [true, false]), &t);
        let hop_no_arms = profile(set([false, false], [true, false]), &t);
        assert!(hop.can_jump() && hop.jump_scale < 1.0);
        assert!(hop_no_arms.jump_scale < hop.jump_scale);
    }
}
