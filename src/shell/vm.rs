//! What every page shares: the account menu (units, region, theme), the details
//! and notes sidebars, the notes tabs. What a choice means to the page's own
//! model is reached through a `Target`; what is remembered, and what is said
//! about it, through the ports.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::catalogue::{REGIONS, Side};
use crate::shared::ports::Ports;
use crate::shared::units::Units;

pub const REGION_KEY: &str = crate::city::store::REGION_KEY;
pub const THEME_KEY: &str = "cityloom-theme";
pub const INSPECTOR_KEY: &str = "cityloom-inspector";
pub const NOTES_KEY: &str = "cityloom-notes";
pub const TAB_KEY: &str = "cityloom-notes-tab";

/// The page the shell is for.
pub trait Target {
    fn set_units(&self, units: Units);
    /// Gives the page's model the region, by index into the catalogue, and says
    /// whether it took it.
    fn apply_region(&self, region: usize) -> bool;
    /// The id of the region the page's model now holds.
    fn region_id(&self) -> String;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn word(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }

    fn parse(word: &str) -> Option<Theme> {
        match word {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}

/// One choice in the region list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionOption {
    pub id: &'static str,
    pub label: String,
}

pub struct ShellVm {
    ports: Ports,
    target: Rc<dyn Target>,
    /// What the left sidebar holds on this page: "piece details", "details", "places".
    details_word: &'static str,
    tabs: Vec<String>,
    units: ArcRwSignal<Units>,
    theme: ArcRwSignal<Option<Theme>>,
    system_dark: ArcRwSignal<bool>,
    region: ArcRwSignal<String>,
    menu_open: ArcRwSignal<bool>,
    inspector_open: ArcRwSignal<bool>,
    notes_open: ArcRwSignal<bool>,
    tab: ArcRwSignal<String>,
}

fn side_word(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

impl ShellVm {
    /// The shell of a page whose notes have `tabs`, by id. What was chosen on an
    /// earlier visit is restored; the region is given to the page's model.
    pub fn new(ports: Ports, target: Rc<dyn Target>, details_word: &'static str, tabs: Vec<String>, system_dark: bool) -> Rc<ShellVm> {
        ShellVm::new_with(ports, target, details_word, tabs, system_dark, true)
    }

    /// As `new`, with whether the notes start open when the person has not chosen:
    /// a page that cannot spare the room for them on a narrow screen says no.
    pub fn new_with(
        ports: Ports,
        target: Rc<dyn Target>,
        details_word: &'static str,
        tabs: Vec<String>,
        system_dark: bool,
        notes_default_open: bool,
    ) -> Rc<ShellVm> {
        let storage = ports.storage.clone();
        let recall = |k: &str| storage.recall(k);
        let theme = recall(THEME_KEY).as_deref().and_then(Theme::parse);
        let tab = recall(TAB_KEY).filter(|t| tabs.contains(t)).or_else(|| tabs.first().cloned()).unwrap_or_default();
        let inspector_open = recall(INSPECTOR_KEY).as_deref() != Some("closed");
        let notes_open = match recall(NOTES_KEY).as_deref() {
            Some("closed") => false,
            Some("open") => true,
            _ => notes_default_open,
        };
        let vm = ShellVm {
            target,
            details_word,
            tabs,
            units: ArcRwSignal::new(Units::default()),
            theme: ArcRwSignal::new(theme),
            system_dark: ArcRwSignal::new(system_dark),
            region: ArcRwSignal::new(String::new()),
            menu_open: ArcRwSignal::new(false),
            inspector_open: ArcRwSignal::new(inspector_open),
            notes_open: ArcRwSignal::new(notes_open),
            tab: ArcRwSignal::new(tab),
            ports,
        };
        vm.take_region(recall(REGION_KEY).as_deref());
        Rc::new(vm)
    }

    // ---- units ----

    pub fn units(&self) -> Units {
        self.units.get()
    }

    pub fn set_units(&self, units: Units) {
        self.units.set(units);
        self.target.set_units(units);
    }

    // ---- region ----

    pub fn regions(&self) -> Vec<RegionOption> {
        REGIONS.iter().map(|r| RegionOption { id: r.id, label: format!("{} ({})", r.name, side_word(r.drive_side)) }).collect()
    }

    /// The id of the region the page's model holds.
    pub fn region(&self) -> String {
        self.region.get()
    }

    /// The person chose a region: the page takes it, it is remembered, and said.
    pub fn choose_region(&self, id: &str) {
        self.take_region(Some(id));
        self.ports.storage.remember(REGION_KEY, id);
        if let Some(r) = REGIONS.iter().find(|r| r.id == self.target.region_id()) {
            self.ports.announcer.say(&format!("{}: traffic keeps {}.", r.name, side_word(r.drive_side)));
        }
    }

    fn take_region(&self, id: Option<&str>) {
        if let Some(i) = REGIONS.iter().position(|r| Some(r.id) == id) {
            self.target.apply_region(i);
        }
        self.region.set(self.target.region_id());
    }

    // ---- theme ----

    /// The theme in use: the one chosen, or else the system's.
    pub fn theme(&self) -> Theme {
        self.theme.get().unwrap_or(if self.system_dark.get() { Theme::Dark } else { Theme::Light })
    }

    /// The theme the person chose, if they have; the page follows the system until then.
    pub fn chosen_theme(&self) -> Option<Theme> {
        self.theme.get()
    }

    pub fn follow_system(&self, dark: bool) {
        self.system_dark.set(dark);
    }

    pub fn choose_theme(&self, theme: Theme) {
        self.theme.set(Some(theme));
        self.ports.storage.remember(THEME_KEY, theme.word());
        self.ports.announcer.say(&format!("{} theme.", theme.name()));
    }

    // ---- the account menu ----

    pub fn menu_open(&self) -> bool {
        self.menu_open.get()
    }

    pub fn toggle_menu(&self) {
        self.menu_open.update(|o| *o = !*o);
    }

    pub fn close_menu(&self) {
        self.menu_open.set(false);
    }

    /// Escape: closes the menu if it is open, and says whether it did.
    pub fn escape(&self) -> bool {
        let was = self.menu_open.get_untracked();
        self.menu_open.set(false);
        was
    }

    // ---- sidebars ----

    pub fn inspector_open(&self) -> bool {
        self.inspector_open.get()
    }

    pub fn notes_open(&self) -> bool {
        self.notes_open.get()
    }

    pub fn toggle_inspector(&self) {
        let open = !self.inspector_open.get_untracked();
        self.inspector_open.set(open);
        self.ports.storage.remember(INSPECTOR_KEY, if open { "open" } else { "closed" });
        let word = capitalised(self.details_word);
        self.ports.announcer.say(&format!("{word} {}.", if open { "shown" } else { "hidden" }));
    }

    pub fn toggle_notes(&self) {
        let open = !self.notes_open.get_untracked();
        self.notes_open.set(open);
        self.ports.storage.remember(NOTES_KEY, if open { "open" } else { "closed" });
        self.ports.announcer.say(if open { "Notes shown." } else { "Notes hidden." });
    }

    // ---- the notes tabs ----

    pub fn tabs(&self) -> &[String] {
        &self.tabs
    }

    /// The id of the tab showing.
    pub fn tab(&self) -> String {
        self.tab.get()
    }

    pub fn show_tab(&self, id: &str) {
        if self.tabs.iter().any(|t| t == id) {
            self.tab.set(id.to_string());
            self.ports.storage.remember(TAB_KEY, id);
        }
    }

    /// A key pressed on a tab: the arrows, Home and End move to another, which
    /// is shown. Gives its id, or nothing when the key is not one of those.
    pub fn tab_key(&self, from: &str, key: &str) -> Option<String> {
        let n = self.tabs.len() as i32;
        let i = self.tabs.iter().position(|t| t == from)? as i32;
        let to = match key {
            "ArrowRight" => i + 1,
            "ArrowLeft" => i - 1,
            "Home" => 0,
            "End" => n - 1,
            _ => return None,
        };
        let id = self.tabs[(to.rem_euclid(n)) as usize].clone();
        self.show_tab(&id);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::{MemoryStorage, RecordingAnnouncer, Storage, test_ports};
    use std::cell::{Cell, RefCell};

    struct Page {
        units: Cell<Units>,
        region: RefCell<String>,
        takes: Cell<bool>,
    }

    struct Fake(Rc<Page>);

    impl Target for Fake {
        fn set_units(&self, units: Units) {
            self.0.units.set(units);
        }
        fn apply_region(&self, region: usize) -> bool {
            if self.0.takes.get() {
                *self.0.region.borrow_mut() = REGIONS[region].id.to_string();
            }
            self.0.takes.get()
        }
        fn region_id(&self) -> String {
            self.0.region.borrow().clone()
        }
    }

    struct Rig {
        vm: Rc<ShellVm>,
        page: Rc<Page>,
        said: Rc<RecordingAnnouncer>,
        storage: Rc<MemoryStorage>,
    }

    fn tabs() -> Vec<String> {
        ["t-space", "t-checks", "t-changes"].map(String::from).to_vec()
    }

    fn rig_with(prepare: impl FnOnce(&MemoryStorage)) -> Rig {
        let (ports, said, storage) = test_ports();
        prepare(&storage);
        let page = Rc::new(Page { units: Cell::new(Units::Metres), region: RefCell::new("canada".into()), takes: Cell::new(true) });
        let vm = ShellVm::new(ports, Rc::new(Fake(page.clone())), "piece details", tabs(), false);
        Rig { vm, page, said, storage }
    }

    fn rig() -> Rig {
        rig_with(|_| {})
    }

    #[test]
    fn units_start_in_metres_and_the_page_is_told_when_they_change() {
        let r = rig();
        assert_eq!(r.vm.units(), Units::Metres);
        r.vm.set_units(Units::Feet);
        assert_eq!((r.vm.units(), r.page.units.get()), (Units::Feet, Units::Feet));
    }

    #[test]
    fn the_regions_are_listed_with_the_side_traffic_keeps() {
        let r = rig();
        let regions = r.vm.regions();
        assert_eq!(regions[0], RegionOption { id: "canada", label: "Canada (right)".into() });
        assert!(regions.iter().any(|o| o.label == "United Kingdom (left)"));
    }

    #[test]
    fn the_region_chosen_before_is_given_to_the_page_without_a_word_said() {
        let r = rig_with(|s| {
            s.remember(REGION_KEY, "germany");
        });
        assert_eq!(r.vm.region(), "germany");
        assert_eq!(r.page.region.borrow().as_str(), "germany");
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn an_unknown_remembered_region_leaves_the_page_s_own() {
        let r = rig_with(|s| {
            s.remember(REGION_KEY, "atlantis");
        });
        assert_eq!(r.vm.region(), "canada");
    }

    #[test]
    fn choosing_a_region_gives_it_to_the_page_remembers_it_and_says_which_side() {
        let r = rig();
        r.vm.choose_region("united-kingdom");
        assert_eq!(r.vm.region(), "united-kingdom");
        assert_eq!(r.storage.recall(REGION_KEY).as_deref(), Some("united-kingdom"));
        assert_eq!(r.said.take(), vec!["United Kingdom: traffic keeps left."]);
    }

    #[test]
    fn a_region_the_page_refuses_is_not_shown_as_chosen() {
        let r = rig();
        r.page.takes.set(false);
        r.vm.choose_region("germany");
        assert_eq!(r.vm.region(), "canada");
    }

    #[test]
    fn the_theme_follows_the_system_until_one_is_chosen() {
        let r = rig();
        assert_eq!((r.vm.theme(), r.vm.chosen_theme()), (Theme::Light, None));
        r.vm.follow_system(true);
        assert_eq!(r.vm.theme(), Theme::Dark);
        r.vm.choose_theme(Theme::Light);
        r.vm.follow_system(true);
        assert_eq!((r.vm.theme(), r.vm.chosen_theme()), (Theme::Light, Some(Theme::Light)));
        assert_eq!(r.storage.recall(THEME_KEY).as_deref(), Some("light"));
        assert_eq!(r.said.take(), vec!["Light theme."]);
    }

    #[test]
    fn a_theme_chosen_before_is_restored_and_a_junk_one_is_not() {
        let r = rig_with(|s| {
            s.remember(THEME_KEY, "dark");
        });
        assert_eq!(r.vm.chosen_theme(), Some(Theme::Dark));
        let r = rig_with(|s| {
            s.remember(THEME_KEY, "purple");
        });
        assert_eq!(r.vm.chosen_theme(), None);
    }

    #[test]
    fn the_menu_opens_and_closes_and_escape_only_counts_when_it_closed_something() {
        let r = rig();
        assert!(!r.vm.menu_open());
        assert!(!r.vm.escape());
        r.vm.toggle_menu();
        assert!(r.vm.menu_open());
        assert!(r.vm.escape());
        assert!(!r.vm.menu_open());
        r.vm.toggle_menu();
        r.vm.close_menu();
        assert!(!r.vm.menu_open());
    }

    #[test]
    fn the_sidebars_are_open_at_first_and_toggling_remembers_and_says() {
        let r = rig();
        assert!(r.vm.inspector_open() && r.vm.notes_open());
        r.vm.toggle_inspector();
        assert!(!r.vm.inspector_open());
        assert_eq!(r.storage.recall(INSPECTOR_KEY).as_deref(), Some("closed"));
        r.vm.toggle_inspector();
        r.vm.toggle_notes();
        assert!(!r.vm.notes_open());
        r.vm.toggle_notes();
        assert_eq!(r.storage.recall(NOTES_KEY).as_deref(), Some("open"));
        assert_eq!(r.said.take(), vec!["Piece details hidden.", "Piece details shown.", "Notes hidden.", "Notes shown."]);
    }

    fn rig_notes(default_open: bool, prepare: impl FnOnce(&MemoryStorage)) -> Rig {
        let (ports, said, storage) = test_ports();
        prepare(&storage);
        let page = Rc::new(Page { units: Cell::new(Units::Metres), region: RefCell::new("canada".into()), takes: Cell::new(true) });
        let vm = ShellVm::new_with(ports, Rc::new(Fake(page.clone())), "details", tabs(), false, default_open);
        Rig { vm, page, said, storage }
    }

    #[test]
    fn on_a_screen_too_narrow_for_them_the_notes_start_closed_until_the_person_has_chosen() {
        let r = rig_notes(false, |_| {});
        assert!(!r.vm.notes_open() && r.vm.inspector_open());
        r.vm.toggle_notes();
        assert!(r.vm.notes_open());
        assert_eq!(r.storage.recall(NOTES_KEY).as_deref(), Some("open"));
    }

    #[test]
    fn what_the_person_chose_before_wins_over_the_default_either_way() {
        let r = rig_notes(false, |s| {
            s.remember(NOTES_KEY, "open");
        });
        assert!(r.vm.notes_open(), "opened on an earlier visit, so open");
        let r = rig_notes(true, |s| {
            s.remember(NOTES_KEY, "closed");
        });
        assert!(!r.vm.notes_open());
    }

    #[test]
    fn a_sidebar_closed_before_stays_closed() {
        let r = rig_with(|s| {
            s.remember(INSPECTOR_KEY, "closed");
            s.remember(NOTES_KEY, "closed");
        });
        assert!(!r.vm.inspector_open() && !r.vm.notes_open());
    }

    #[test]
    fn the_left_sidebar_is_named_for_what_the_page_puts_in_it() {
        let (ports, said, _) = test_ports();
        let page = Rc::new(Page { units: Cell::new(Units::Metres), region: RefCell::new("canada".into()), takes: Cell::new(true) });
        let vm = ShellVm::new(ports, Rc::new(Fake(page)), "places", vec![], false);
        vm.toggle_inspector();
        assert_eq!(said.take(), vec!["Places hidden."]);
    }

    #[test]
    fn the_first_tab_shows_until_another_is_chosen_and_the_choice_is_remembered() {
        let r = rig();
        assert_eq!(r.vm.tab(), "t-space");
        r.vm.show_tab("t-checks");
        assert_eq!(r.vm.tab(), "t-checks");
        assert_eq!(r.storage.recall(TAB_KEY).as_deref(), Some("t-checks"));
        r.vm.show_tab("t-nothing");
        assert_eq!(r.vm.tab(), "t-checks");
        let r2 = rig_with(|s| {
            s.remember(TAB_KEY, "t-changes");
        });
        assert_eq!(r2.vm.tab(), "t-changes");
        let r3 = rig_with(|s| {
            s.remember(TAB_KEY, "t-gone");
        });
        assert_eq!(r3.vm.tab(), "t-space");
    }

    #[test]
    fn the_arrows_move_between_tabs_and_wrap_and_other_keys_do_nothing() {
        let r = rig();
        assert_eq!(r.vm.tab_key("t-space", "ArrowRight").as_deref(), Some("t-checks"));
        assert_eq!(r.vm.tab(), "t-checks");
        assert_eq!(r.vm.tab_key("t-space", "ArrowLeft").as_deref(), Some("t-changes"));
        assert_eq!(r.vm.tab_key("t-changes", "ArrowRight").as_deref(), Some("t-space"));
        assert_eq!(r.vm.tab_key("t-checks", "End").as_deref(), Some("t-changes"));
        assert_eq!(r.vm.tab_key("t-changes", "Home").as_deref(), Some("t-space"));
        assert_eq!(r.vm.tab_key("t-space", "a"), None);
        assert_eq!(r.vm.tab(), "t-space");
    }
}
