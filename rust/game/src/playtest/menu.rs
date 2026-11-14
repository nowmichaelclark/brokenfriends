//! The model of the dev menu (S-113): the pages, the items, the cursor
//! and the actions. It has no Godot code, so it has unit tests.
//! `crate::dev_menu::BfDevMenu` shows it and carries out the actions.

use super::bench::TestKind;

/// The view distances that the menu can force, in metres. `None` is the
/// normal value: the distance of the smog.
pub const VIEW_STEPS: [Option<f32>; 4] = [None, Some(300.0), Some(800.0), Some(3000.0)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Main,
    Tests,
    Limbs,
}

/// The names of the limbs, in the order of the "Limbs" page. They are
/// the names that `BfBear::set_limb` takes.
pub const LIMB_NAMES: [&str; 4] = ["LeftArm", "RightArm", "LeftLeg", "RightLeg"];

/// What the game must do after a button press.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    None,
    /// Close the menu.
    Close,
    ToggleFlyCamera,
    ToggleOverlay,
    /// Make a new zone with the next seed.
    NewZone,
    /// Make the zone of this seed.
    MakeZone(i64),
    /// Force a view distance (`None`: from the smog).
    SetViewDistance(Option<f32>),
    SavePlaytest,
    StartTest(TestKind),
    /// Start the autoplay tour (S-133).
    StartAutoplay,
    StopTest,
    /// Take a limb away or give it back (the name is in `LIMB_NAMES`).
    SetLimb(&'static str, bool),
    /// Give back all limbs.
    RestoreLimbs,
    /// Put the bear at the next obstacle (the same as F6).
    NextObstacle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Close,
    Fly,
    Overlay,
    NewZone,
    Seed,
    View,
    Save,
    TestsPage,
    LimbsPage,
    NextObstacle,
    Limb(usize),
    RestoreLimbs,
    Run(TestKind),
    Autoplay,
    Stop,
    Back,
}

/// What the menu shows and what the game tells it.
#[derive(Clone, Debug)]
pub struct MenuState {
    pub page: Page,
    cursor: usize,
    /// The seed that the "Seed" item shows.
    pub seed: i64,
    pub fly_camera: bool,
    pub overlay: bool,
    /// The index in `VIEW_STEPS`.
    view: usize,
    /// The distance of the smog in metres, for the label of "auto".
    pub smog_distance_m: f32,
    pub test_running: bool,
    /// The limbs that the bear has: left arm, right arm, left leg, right leg.
    pub limbs: [bool; 4],
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            page: Page::Main,
            cursor: 0,
            seed: 1,
            fly_camera: false,
            overlay: true,
            view: 0,
            smog_distance_m: 0.0,
            test_running: false,
            limbs: [true; 4],
        }
    }
}

impl MenuState {
    /// Starts at the first item of the main page.
    pub fn open(&mut self) {
        self.page = Page::Main;
        self.cursor = 0;
    }

    fn kinds(&self) -> Vec<Kind> {
        match self.page {
            Page::Main => vec![
                Kind::Close,
                Kind::Fly,
                Kind::Overlay,
                Kind::NewZone,
                Kind::Seed,
                Kind::View,
                Kind::LimbsPage,
                Kind::Save,
                Kind::TestsPage,
                Kind::NextObstacle,
            ],
            Page::Limbs => vec![
                Kind::Limb(0),
                Kind::Limb(1),
                Kind::Limb(2),
                Kind::Limb(3),
                Kind::RestoreLimbs,
                Kind::Back,
            ],
            Page::Tests => {
                let mut v = vec![
                    Kind::Run(TestKind::Benchmark),
                    Kind::Run(TestKind::DrawDistance),
                    Kind::Autoplay,
                ];
                if self.test_running {
                    v.push(Kind::Stop);
                }
                v.push(Kind::Back);
                v
            }
        }
    }

    fn selected(&self) -> Kind {
        let kinds = self.kinds();
        kinds[self.cursor.min(kinds.len() - 1)]
    }

    /// The forced view distance, or `None` for the smog.
    pub fn view_distance(&self) -> Option<f32> {
        VIEW_STEPS[self.view]
    }

    /// Sets the forced view distance. A value that is not in the list
    /// becomes "from the smog".
    pub fn set_view_distance(&mut self, value: Option<f32>) {
        self.view = VIEW_STEPS.iter().position(|v| *v == value).unwrap_or(0);
    }

    fn label(&self, kind: Kind) -> String {
        let on = |b: bool| if b { "on" } else { "off" };
        match kind {
            Kind::Close => "Close the menu".into(),
            Kind::Fly => format!("Fly camera: {}", on(self.fly_camera)),
            Kind::Overlay => format!("Debug overlay: {}", on(self.overlay)),
            Kind::NewZone => "New zone (next seed)".into(),
            Kind::NextObstacle => "Go to the next obstacle (each time: the next type)".into(),
            Kind::Seed => format!(
                "Seed: {}   (left, right: change. A: make the zone)",
                self.seed
            ),
            Kind::View => match self.view_distance() {
                None => format!(
                    "View distance: from the smog ({:.0} m)   (left, right: change)",
                    self.smog_distance_m
                ),
                Some(d) => format!("View distance: {d:.0} m, forced   (left, right: change)"),
            },
            Kind::Save => "Save playtest".into(),
            Kind::TestsPage => "Tests ...".into(),
            Kind::LimbsPage => "Limbs ...".into(),
            Kind::Limb(i) => {
                let name = ["Left arm", "Right arm", "Left leg", "Right leg"][i];
                format!("{name}: {}", if self.limbs[i] { "there" } else { "gone" })
            }
            Kind::RestoreLimbs => "Give back all limbs".into(),
            Kind::Run(TestKind::Benchmark) => {
                if self.test_running {
                    "Benchmark (a test is running)".into()
                } else {
                    "Benchmark: seeds 1, 7 and 42".into()
                }
            }
            Kind::Run(TestKind::DrawDistance) => {
                if self.test_running {
                    "Draw distance test (a test is running)".into()
                } else {
                    "Draw distance test: 300, 800, 3000 m and the smog".into()
                }
            }
            Kind::Autoplay => {
                if self.test_running {
                    "Autoplay tour (a test is running)".into()
                } else {
                    "Autoplay tour: the bot plays seeds 1, 7 and 42 (about 9 min)".into()
                }
            }
            Kind::Stop => "Stop the test".into(),
            Kind::Back => "Back".into(),
        }
    }

    /// The title of the page.
    pub fn title(&self) -> &'static str {
        match self.page {
            Page::Main => "Dev menu",
            Page::Tests => "Dev menu: tests",
            Page::Limbs => "Dev menu: limbs",
        }
    }

    /// The lines of the page and the index of the line with the cursor.
    pub fn lines(&self) -> (Vec<String>, usize) {
        let kinds = self.kinds();
        let cursor = self.cursor.min(kinds.len() - 1);
        (kinds.into_iter().map(|k| self.label(k)).collect(), cursor)
    }

    pub fn up(&mut self) {
        let n = self.kinds().len();
        self.cursor = (self.cursor.min(n - 1) + n - 1) % n;
    }

    pub fn down(&mut self) {
        let n = self.kinds().len();
        self.cursor = (self.cursor.min(n - 1) + 1) % n;
    }

    /// Left (-1) or right (+1) on the item. Changes a value.
    pub fn adjust(&mut self, direction: i32) -> Action {
        match self.selected() {
            Kind::Seed => {
                self.seed = (self.seed + direction as i64).max(0);
                Action::None
            }
            Kind::View => {
                let n = VIEW_STEPS.len() as i32;
                self.view = (self.view as i32 + direction).rem_euclid(n) as usize;
                Action::SetViewDistance(self.view_distance())
            }
            _ => Action::None,
        }
    }

    /// The A button on the item.
    pub fn activate(&mut self) -> Action {
        match self.selected() {
            Kind::Close => Action::Close,
            Kind::Fly => Action::ToggleFlyCamera,
            Kind::Overlay => Action::ToggleOverlay,
            Kind::NewZone => Action::NewZone,
            Kind::NextObstacle => Action::NextObstacle,
            Kind::Seed => Action::MakeZone(self.seed),
            Kind::View => self.adjust(1),
            Kind::Save => Action::SavePlaytest,
            Kind::TestsPage => {
                self.page = Page::Tests;
                self.cursor = 0;
                Action::None
            }
            Kind::LimbsPage => {
                self.page = Page::Limbs;
                self.cursor = 0;
                Action::None
            }
            Kind::Limb(i) => {
                self.limbs[i] = !self.limbs[i];
                Action::SetLimb(LIMB_NAMES[i], self.limbs[i])
            }
            Kind::RestoreLimbs => {
                self.limbs = [true; 4];
                Action::RestoreLimbs
            }
            Kind::Run(kind) => {
                if self.test_running {
                    Action::None
                } else {
                    Action::StartTest(kind)
                }
            }
            Kind::Autoplay => {
                if self.test_running {
                    Action::None
                } else {
                    Action::StartAutoplay
                }
            }
            Kind::Stop => Action::StopTest,
            Kind::Back => {
                self.page = Page::Main;
                self.cursor = 0;
                Action::None
            }
        }
    }

    /// The B button: back to the main page, or close the menu.
    pub fn back(&mut self) -> Action {
        if self.page != Page::Main {
            self.page = Page::Main;
            self.cursor = 0;
            Action::None
        } else {
            Action::Close
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_wraps() {
        let mut m = MenuState::default();
        let (lines, cursor) = m.lines();
        assert_eq!(cursor, 0);
        m.up();
        assert_eq!(m.lines().1, lines.len() - 1);
        m.down();
        assert_eq!(m.lines().1, 0);
    }

    #[test]
    fn next_obstacle_is_the_last_main_item() {
        let mut m = MenuState::default();
        m.up();
        assert_eq!(m.activate(), Action::NextObstacle);
        assert_eq!(m.page, Page::Main);
    }

    #[test]
    fn main_page_actions() {
        let mut m = MenuState::default();
        assert_eq!(m.activate(), Action::Close);
        m.down();
        assert_eq!(m.activate(), Action::ToggleFlyCamera);
        m.down();
        assert_eq!(m.activate(), Action::ToggleOverlay);
        m.down();
        assert_eq!(m.activate(), Action::NewZone);
        m.down();
        // The seed item: left and right change it, A makes the zone.
        m.seed = 5;
        assert_eq!(m.adjust(1), Action::None);
        assert_eq!(m.adjust(1), Action::None);
        assert_eq!(m.activate(), Action::MakeZone(7));
        assert_eq!(m.adjust(-1), Action::None);
        assert_eq!(m.seed, 6);
        m.down();
        m.down();
        // The "Limbs" page opens, and B goes back.
        assert_eq!(m.activate(), Action::None);
        assert_eq!(m.page, Page::Limbs);
        assert_eq!(m.back(), Action::None);
        assert_eq!(m.page, Page::Main);
        // B puts the cursor at the top again.
        for _ in 0..7 {
            m.down();
        }
        assert_eq!(m.activate(), Action::SavePlaytest);
    }

    #[test]
    fn limbs_page() {
        let mut m = MenuState::default();
        for _ in 0..6 {
            m.down();
        }
        m.activate();
        assert_eq!(m.page, Page::Limbs);
        assert_eq!(m.activate(), Action::SetLimb("LeftArm", false));
        assert!(m.lines().0[0].contains("gone"));
        assert_eq!(m.activate(), Action::SetLimb("LeftArm", true));
        m.down();
        m.down();
        m.down();
        assert_eq!(m.activate(), Action::SetLimb("RightLeg", false));
        m.down();
        assert_eq!(m.activate(), Action::RestoreLimbs);
        assert_eq!(m.limbs, [true; 4]);
        m.down();
        assert_eq!(m.activate(), Action::None);
        assert_eq!(m.page, Page::Main);
    }

    #[test]
    fn tests_page_has_the_autoplay() {
        let mut m = MenuState {
            page: Page::Tests,
            ..Default::default()
        };
        m.down();
        m.down();
        assert_eq!(m.activate(), Action::StartAutoplay);
        m.test_running = true;
        assert_eq!(m.activate(), Action::None);
        assert!(m.lines().0[2].contains("a test is running"));
    }

    #[test]
    fn the_seed_does_not_go_below_zero() {
        let mut m = MenuState::default();
        for _ in 0..4 {
            m.down();
        }
        m.seed = 0;
        m.adjust(-1);
        assert_eq!(m.seed, 0);
    }

    #[test]
    fn view_distance_cycles() {
        let mut m = MenuState::default();
        for _ in 0..5 {
            m.down();
        }
        assert_eq!(m.view_distance(), None);
        assert_eq!(m.adjust(1), Action::SetViewDistance(Some(300.0)));
        assert_eq!(m.adjust(1), Action::SetViewDistance(Some(800.0)));
        assert_eq!(m.activate(), Action::SetViewDistance(Some(3000.0)));
        assert_eq!(m.adjust(1), Action::SetViewDistance(None));
        assert_eq!(m.adjust(-1), Action::SetViewDistance(Some(3000.0)));
        m.set_view_distance(Some(800.0));
        assert_eq!(m.view_distance(), Some(800.0));
        m.set_view_distance(Some(123.0));
        assert_eq!(m.view_distance(), None);
    }

    #[test]
    fn tests_page() {
        let mut m = MenuState::default();
        for _ in 0..8 {
            m.down();
        }
        assert_eq!(m.activate(), Action::None);
        assert_eq!(m.page, Page::Tests);
        assert_eq!(m.lines().1, 0);
        assert_eq!(m.activate(), Action::StartTest(TestKind::Benchmark));
        m.down();
        assert_eq!(m.activate(), Action::StartTest(TestKind::DrawDistance));
        // B goes back to the main page, a second B closes the menu.
        assert_eq!(m.back(), Action::None);
        assert_eq!(m.page, Page::Main);
        assert_eq!(m.back(), Action::Close);
    }

    #[test]
    fn a_running_test_adds_a_stop_item_and_blocks_a_start() {
        let mut m = MenuState {
            page: Page::Tests,
            ..MenuState::default()
        };
        assert_eq!(m.lines().0.len(), 4);
        m.test_running = true;
        let (lines, _) = m.lines();
        assert_eq!(lines.len(), 5);
        assert_eq!(m.activate(), Action::None);
        m.down();
        m.down();
        m.down();
        assert_eq!(m.activate(), Action::StopTest);
        assert!(lines[3].contains("Stop"));
    }

    #[test]
    fn open_goes_to_the_top() {
        let mut m = MenuState {
            page: Page::Tests,
            ..MenuState::default()
        };
        m.down();
        m.open();
        assert_eq!(m.page, Page::Main);
        assert_eq!(m.lines().1, 0);
    }
}
