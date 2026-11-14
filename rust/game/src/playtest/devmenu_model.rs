//! The pages and the cursor of the dev menu. No Godot code is here,
//! so the unit tests can check it.

/// A page of the dev menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Main,
    Tests,
}

/// What an entry does when the player pushes A.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    FlyCamera,
    Overlay,
    NewZone,
    /// Left and right change the seed. A makes the zone.
    Seed,
    SavePlaytest,
    OpenTests,
    Close,
    Benchmark,
    DrawDistance,
    BackToMain,
}

const MAIN: [Item; 7] = [
    Item::FlyCamera,
    Item::Overlay,
    Item::NewZone,
    Item::Seed,
    Item::SavePlaytest,
    Item::OpenTests,
    Item::Close,
];

const TESTS: [Item; 3] = [Item::Benchmark, Item::DrawDistance, Item::BackToMain];

impl Page {
    /// The entries of the page, from top to bottom.
    pub fn items(self) -> &'static [Item] {
        match self {
            Page::Main => &MAIN,
            Page::Tests => &TESTS,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Page::Main => "Dev menu",
            Page::Tests => "Tests",
        }
    }
}

impl Item {
    /// The text of the entry. `seed` is the seed that the menu shows.
    pub fn label(self, seed: i64) -> String {
        match self {
            Item::FlyCamera => "Fly camera".into(),
            Item::Overlay => "Debug overlay on or off".into(),
            Item::NewZone => "New zone (next seed)".into(),
            Item::Seed => format!("Seed  < {seed} >   (A: make this zone)"),
            Item::SavePlaytest => "Save playtest".into(),
            Item::OpenTests => "Tests...".into(),
            Item::Close => "Close the menu".into(),
            Item::Benchmark => "Benchmark (seeds 1, 7, 42)".into(),
            Item::DrawDistance => "Draw distance test (300, 800, 3000 m)".into(),
            Item::BackToMain => "Back".into(),
        }
    }
}

/// The page and the entry that the cursor points to.
#[derive(Clone, Debug)]
pub struct MenuState {
    pub page: Page,
    pub cursor: usize,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            page: Page::Main,
            cursor: 0,
        }
    }

    /// The entry under the cursor.
    pub fn current(&self) -> Item {
        let items = self.page.items();
        items[self.cursor.min(items.len() - 1)]
    }

    /// Moves the cursor up. At the top, it goes to the bottom.
    pub fn up(&mut self) {
        let n = self.page.items().len();
        self.cursor = (self.cursor + n - 1) % n;
    }

    /// Moves the cursor down. At the bottom, it goes to the top.
    pub fn down(&mut self) {
        let n = self.page.items().len();
        self.cursor = (self.cursor + 1) % n;
    }

    /// Goes to another page. The cursor goes to the top.
    pub fn go(&mut self, page: Page) {
        self.page = page;
        self.cursor = 0;
    }

    /// The B button: from "Tests" it goes back. From the main page, it
    /// tells the caller to close the menu (returns true).
    pub fn back(&mut self) -> bool {
        if self.page == Page::Main {
            true
        } else {
            self.go(Page::Main);
            false
        }
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_wraps() {
        let mut m = MenuState::new();
        m.up();
        assert_eq!(m.current(), Item::Close);
        m.down();
        assert_eq!(m.current(), Item::FlyCamera);
    }

    #[test]
    fn pages_change_and_go_back() {
        let mut m = MenuState::new();
        m.go(Page::Tests);
        assert_eq!(m.current(), Item::Benchmark);
        m.down();
        assert_eq!(m.current(), Item::DrawDistance);
        assert!(!m.back());
        assert_eq!(m.page, Page::Main);
        assert_eq!(m.cursor, 0);
        assert!(m.back());
    }

    #[test]
    fn each_page_has_a_way_out() {
        assert!(Page::Main.items().contains(&Item::Close));
        assert!(Page::Tests.items().contains(&Item::BackToMain));
    }

    #[test]
    fn seed_label_shows_the_seed() {
        assert!(Item::Seed.label(42).contains("42"));
    }
}
