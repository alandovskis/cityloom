//! The map page's area restrictions: the list the Restrictions tab shows, drawing a new outline, and renaming,
//! changing the rule of, redrawing, dragging the corners of and deleting the one selected. What it shows on the map
//! goes through the `Mapper` port (`sync`); what it keeps, through the `RestrictionStore`. Which streets are inside
//! is the map's view-model's business (`MapVm::findings`), which knows the city.
//!
//! Every outline that reaches the store has passed `valid`: a new or redrawn one when it is closed (the drawing
//! machine checks it), a dragged one when its corner is let go (checked here). The store does not check.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::{overlay, style};
use crate::restriction::draw::{Draw, Outcome, Step};
use crate::restriction::model::{LonLat, Restriction, Rule};
use crate::restriction::store::RestrictionStore;
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;
use crate::shared::said::{self, Said};
use crate::shared::units::Units;

pub struct AreasVm {
    store: RestrictionStore,
    ports: Ports,
    i18n: Rc<I18n>,
    list: ArcRwSignal<Vec<Restriction>>,
    /// Whether the document is a newer build's: shown as nothing, never written.
    future: ArcRwSignal<bool>,
    draw: ArcRwSignal<Draw>,
    selected: ArcRwSignal<Option<String>>,
    name: ArcRwSignal<String>,
    rule: ArcRwSignal<Rule>,
}

impl AreasVm {
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<AreasVm> {
        let store = RestrictionStore::new(ports.storage.clone());
        let (list, future) = (store.all(), store.is_future());
        Rc::new(AreasVm {
            store,
            ports,
            i18n,
            list: ArcRwSignal::new(list),
            future: ArcRwSignal::new(future),
            draw: ArcRwSignal::new(Draw::Idle),
            selected: ArcRwSignal::new(None),
            name: ArcRwSignal::new(String::new()),
            rule: ArcRwSignal::new(Rule::CarFree),
        })
    }

    // ---- words ----

    /// The message `key`, for a view.
    pub fn word(&self, key: &str) -> String {
        self.i18n.tr(key, &Args::new())
    }

    /// What a model says, for a view. Nothing an area says has a length in it.
    pub fn say(&self, s: &Said) -> String {
        said::say(&self.i18n, Units::Metres, s)
    }

    fn say_now(&self, s: &Said) -> String {
        said::say_now(&self.i18n, Units::Metres, s)
    }

    fn tell(&self, key: &str, args: Args) {
        self.ports.announcer.say(&self.i18n.tr_now(key, &args));
    }

    fn tell_about(&self, key: &str, r: &Restriction) {
        self.tell(key, Args::new().str("area", self.say_now(&r.title())));
    }

    // ---- properties ----

    pub fn list(&self) -> Vec<Restriction> {
        self.list.get()
    }

    pub fn list_now(&self) -> Vec<Restriction> {
        self.list.get_untracked()
    }

    pub fn find(&self, id: &str) -> Option<Restriction> {
        self.list.with(|l| l.iter().find(|r| r.id == id).cloned())
    }

    pub fn find_now(&self, id: &str) -> Option<Restriction> {
        self.list.with_untracked(|l| l.iter().find(|r| r.id == id).cloned())
    }

    pub fn draw(&self) -> Draw {
        self.draw.get()
    }

    pub fn draw_now(&self) -> Draw {
        self.draw.get_untracked()
    }

    pub fn selected_id(&self) -> Option<String> {
        self.selected.get()
    }

    pub fn selected_id_now(&self) -> Option<String> {
        self.selected.get_untracked()
    }

    pub fn is_future(&self) -> bool {
        self.future.get()
    }

    pub fn form_name(&self) -> String {
        self.name.get()
    }

    pub fn form_rule(&self) -> Rule {
        self.rule.get()
    }

    /// What an unnamed area will be called: its rule.
    pub fn form_placeholder(&self) -> String {
        self.say(&self.rule.get().said())
    }

    /// What the tab says first.
    pub fn lead(&self) -> String {
        if self.future.get() {
            return self.word("restriction-future");
        }
        match self.list.with(Vec::len) {
            0 => self.word("restriction-lead-none"),
            n => self.i18n.tr("restriction-lead-some", &Args::new().num("n", n as i64)),
        }
    }

    /// How many corners the outline being drawn has.
    pub fn corners_text(&self) -> String {
        self.i18n.tr("restriction-corners", &Args::new().num("n", self.draw.with(|d| d.corners().len()) as i64))
    }

    /// The name of the rule `id` among the choices.
    pub fn rule_choice(&self, id: &str) -> String {
        self.word(Rule::choice_key(id))
    }

    /// A speed among the choices: `30 km/h`.
    pub fn kmh_text(&self, kmh: u8) -> String {
        self.i18n.tr("restriction-kmh", &Args::new().num("kmh", kmh.into()))
    }

    // ---- commands ----

    /// The list is read again from where it is kept: another tab wrote it. A selection whose area is gone goes.
    pub fn reload(&self) {
        let all = self.store.all();
        if self.selected.get_untracked().is_some_and(|id| !all.iter().any(|r| r.id == id)) {
            self.selected.set(None);
        }
        self.future.set(self.store.is_future());
        self.list.set(all);
    }

    /// Applies a step to the outline and says what came of it.
    fn step(&self, step: Step) -> Outcome {
        let mut draw = self.draw.get_untracked();
        let outcome = draw.step(step);
        self.draw.set(draw);
        match &outcome {
            Outcome::Added(n) => self.tell("restriction-corner-added", Args::new().num("n", *n as i64)),
            Outcome::Duplicate => self.tell("restriction-refused-duplicate", Args::new()),
            Outcome::Removed(n) => self.tell("restriction-corner-removed", Args::new().num("n", *n as i64)),
            Outcome::Closed => self.closed(),
            Outcome::Cancelled => self.tell("restriction-cancelled", Args::new()),
            Outcome::Refused(why) => self.ports.announcer.say(&self.say_now(why)),
            Outcome::Started | Outcome::Ignored => {}
        }
        outcome
    }

    /// The outline is closed: it waits for its rule and name.
    fn closed(&self) {
        self.name.set(String::new());
        self.rule.set(Rule::CarFree);
        self.tell("restriction-closed", Args::new());
    }

    /// Starts drawing a new area. False, and nothing changes, where the document is a newer build's or an outline
    /// is already being drawn or named (the Declare button is not shown then, so nothing is said).
    pub fn start(&self) -> bool {
        if self.future.get_untracked() || self.draw.get_untracked() != Draw::Idle {
            return false;
        }
        self.selected.set(None);
        let started = self.step(Step::Start) == Outcome::Started;
        if started {
            self.tell("restriction-started", Args::new());
        }
        started
    }

    pub fn add_at(&self, at: LonLat) -> bool {
        matches!(self.step(Step::Add(at)), Outcome::Added(_))
    }

    /// A corner where the middle of the map is: the keyboard's way of placing one.
    pub fn add_at_centre(&self) -> bool {
        self.add_at(self.ports.mapper.centre())
    }

    pub fn press_vertex(&self, i: usize) -> bool {
        self.step(Step::PressVertex(i)) == Outcome::Closed
    }

    pub fn close(&self) -> bool {
        self.step(Step::Close) == Outcome::Closed
    }

    pub fn back(&self) -> bool {
        matches!(self.step(Step::Back), Outcome::Removed(_))
    }

    pub fn cancel(&self) -> bool {
        self.step(Step::Cancel) == Outcome::Cancelled
    }

    /// A press on the map while an outline is drawn. Whatever is under it (a street, an area) it is the outline's:
    /// on the first corner it closes the outline (the press is a little off the corner, so it is never taken as a
    /// corner there), on another corner it is said to be one already, anywhere else it places a corner. Idle or
    /// naming, a press is not the outline's and does nothing here: which of a place or an area a press on both
    /// means is the map's view-model's to decide, as only it knows the places.
    pub fn on_press(&self, at: LonLat, vertex: Option<usize>) {
        if !matches!(self.draw.get_untracked(), Draw::Drawing(_)) {
            return;
        }
        match vertex {
            Some(0) => {
                self.press_vertex(0);
            }
            Some(_) => self.tell("restriction-refused-duplicate", Args::new()),
            None => {
                self.add_at(at);
            }
        }
    }

    pub fn set_form_name(&self, name: &str) {
        self.name.set(name.to_string());
    }

    pub fn set_form_rule(&self, rule: Rule) {
        self.rule.set(rule);
    }

    /// Keeps the closed outline with the rule and name chosen. If storage refuses, the outline stays to try again.
    pub fn save(&self) -> bool {
        let Draw::Naming(ring) = self.draw.get_untracked() else { return false };
        let Some(r) = self.store.add(&self.name.get_untracked(), self.rule.get_untracked(), ring) else {
            self.tell("restriction-not-kept", Args::new());
            return false;
        };
        self.draw.set(Draw::Idle);
        self.reload();
        self.selected.set(Some(r.id.clone()));
        self.tell_about("restriction-saved", &r);
        true
    }

    /// What a key pressed on the map does to the outline being drawn; says whether it was one of its keys. Arrow
    /// keys stay the map's, so the map can be moved between corners. A closed outline takes no key from the map:
    /// its rule and name are in the tab's form, where Escape belongs to the field, and only Cancel lets it go.
    pub fn press_key(&self, key: &str, shift: bool) -> bool {
        let draw = self.draw.get_untracked();
        match (&draw, key) {
            (Draw::Drawing(_), "Enter") if shift => {
                self.close();
            }
            (Draw::Drawing(_), "Enter") => {
                self.add_at_centre();
            }
            (Draw::Drawing(c), "Backspace") if !c.is_empty() => {
                self.back();
            }
            (Draw::Drawing(_), "Escape") => {
                self.cancel();
            }
            _ => return false,
        }
        true
    }

    /// Sends the areas and the outline being drawn to the map, with their layers.
    pub fn sync(&self) {
        let selected = self.selected.get_untracked();
        let list = self.list.get_untracked();
        self.ports.mapper.set_restrictions(&style::area_layers(), &overlay::areas(&list, selected.as_deref(), None));
        self.ports.mapper.set_draft(&style::draft_layers(), &overlay::draft(self.draw.get_untracked().corners()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restriction::store::RESTRICTIONS_KEY;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{FakeMapper, MapCall, MemoryStorage, RecordingAnnouncer, Storage, test_ports};

    const SQUARE: [LonLat; 4] = [[-73.60, 45.52], [-73.59, 45.52], [-73.59, 45.53], [-73.60, 45.53]];

    fn rig() -> (Rc<AreasVm>, Rc<FakeMapper>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
        let (ports, said, storage) = test_ports();
        let mapper = Rc::new(FakeMapper::default());
        let vm = AreasVm::new(Ports { mapper: mapper.clone(), ..ports }, crate::i18n_for(Locale::En));
        (vm, mapper, said, storage)
    }

    /// Draws the square by its corners and closes it on the first.
    fn draw_square(vm: &AreasVm) {
        assert!(vm.start());
        for p in SQUARE {
            assert!(vm.add_at(p));
        }
        assert!(vm.press_vertex(0));
    }

    #[test]
    fn an_area_is_declared_by_its_corners_then_its_rule_and_name_and_each_step_is_said() {
        let (vm, _, said, storage) = rig();
        draw_square(&vm);
        assert_eq!(
            said.take(),
            [
                "Drawing an area. Click the map, or press Enter on it, to place the first corner.",
                "Corner 1 placed.",
                "Corner 2 placed.",
                "Corner 3 placed.",
                "Corner 4 placed.",
                "The outline is closed. Choose its rule and name, then save it.",
            ]
        );
        assert_eq!(vm.draw_now(), Draw::Naming(SQUARE.to_vec()));
        assert_eq!(vm.form_placeholder(), "Car-free", "an unnamed area is called by its rule");
        vm.set_form_rule(Rule::BusAndBikeOnly);
        vm.set_form_name("Mile End");
        assert!(vm.save());
        assert_eq!(said.take(), ["“Mile End” saved."]);
        assert_eq!(vm.draw_now(), Draw::Idle);
        let list = vm.list_now();
        assert_eq!((list.len(), list[0].id.as_str(), list[0].rule), (1, "r1", Rule::BusAndBikeOnly));
        assert_eq!(vm.selected_id_now().as_deref(), Some("r1"), "the new area is selected");
        assert!(storage.recall(RESTRICTIONS_KEY).unwrap().contains("Mile End"));
    }

    #[test]
    fn from_the_keyboard_enter_places_a_corner_at_the_centre_and_shift_enter_closes() {
        let (vm, mapper, said, _) = rig();
        assert!(!vm.press_key("Enter", false), "the key is not the outline's while nothing is drawn");
        assert!(vm.start());
        for p in &SQUARE[..3] {
            mapper.set_centre(*p);
            assert!(vm.press_key("Enter", false));
        }
        assert!(vm.press_key("Enter", true));
        assert_eq!(vm.draw_now(), Draw::Naming(SQUARE[..3].to_vec()));
        assert!(said.take().last().unwrap().starts_with("The outline is closed."));
    }

    #[test]
    fn backspace_removes_the_last_corner_and_escape_cancels() {
        let (vm, _, said, _) = rig();
        vm.start();
        vm.add_at(SQUARE[0]);
        vm.add_at(SQUARE[1]);
        said.take();
        assert!(vm.press_key("Backspace", false));
        assert_eq!(said.take(), ["Corner removed. One corner left."]);
        assert!(vm.press_key("Escape", false));
        assert_eq!(said.take(), ["Drawing cancelled."]);
        assert_eq!(vm.draw_now(), Draw::Idle);
        assert!(!vm.press_key("Backspace", false), "nothing to remove");
    }

    #[test]
    fn a_closed_outline_takes_no_key_from_the_map_so_escape_in_its_name_field_cannot_cancel_it() {
        let (vm, _, said, _) = rig();
        draw_square(&vm);
        said.take();
        for (key, shift) in [("Escape", false), ("Enter", false), ("Enter", true), ("Backspace", false)] {
            assert!(!vm.press_key(key, shift), "{key} {shift}");
        }
        assert_eq!(vm.draw_now(), Draw::Naming(SQUARE.to_vec()), "only the Cancel button lets go of a closed outline");
        assert!(said.take().is_empty());
        assert!(vm.cancel());
        assert_eq!(said.take(), ["Drawing cancelled."]);
    }

    #[test]
    fn a_corner_where_one_already_is_is_not_placed_and_that_is_said() {
        let (vm, _, said, _) = rig();
        vm.start();
        vm.add_at(SQUARE[0]);
        said.take();
        assert!(!vm.add_at(SQUARE[0]), "a double-click is one corner");
        assert_eq!(said.take(), ["A corner is already there. Move the map, then add the next one."]);
        assert_eq!(vm.draw_now().corners(), &SQUARE[..1]);
    }

    #[test]
    fn a_press_on_the_first_corner_closes_the_outline_and_a_press_on_another_corner_is_said_to_be_one() {
        let (vm, _, said, _) = rig();
        vm.start();
        for p in &SQUARE[..3] {
            vm.on_press(*p, None);
        }
        said.take();
        vm.on_press([SQUARE[2][0] + 1e-5, SQUARE[2][1]], Some(2));
        assert_eq!(said.take(), ["A corner is already there. Move the map, then add the next one."]);
        assert_eq!(vm.draw_now().corners().len(), 3, "a press on a corner places none");
        // The press is a little off the first corner, as a pointer is: it closes, it does not add a fourth corner.
        vm.on_press([SQUARE[0][0] + 2e-5, SQUARE[0][1] - 1e-5], Some(0));
        assert_eq!(said.take(), ["The outline is closed. Choose its rule and name, then save it."]);
        assert_eq!(vm.draw_now(), Draw::Naming(SQUARE[..3].to_vec()));
    }

    #[test]
    fn a_press_is_a_corner_only_while_drawing_and_nothing_once_the_outline_is_closed() {
        let (vm, _, said, _) = rig();
        vm.on_press(SQUARE[0], None);
        assert_eq!((vm.draw_now(), said.take()), (Draw::Idle, vec![]), "idle: a press is the map's, not the outline's");
        draw_square(&vm);
        said.take();
        vm.on_press([-73.595, 45.525], None);
        vm.on_press(SQUARE[0], Some(0));
        assert_eq!(vm.draw_now(), Draw::Naming(SQUARE.to_vec()));
        assert!(said.take().is_empty(), "naming: presses are ignored, and quietly");
    }

    #[test]
    fn declaring_again_while_drawing_or_naming_is_ignored_quietly_and_the_outline_is_kept() {
        let (vm, _, said, _) = rig();
        vm.start();
        vm.add_at(SQUARE[0]);
        said.take();
        assert!(!vm.start());
        assert_eq!(vm.draw_now().corners(), &SQUARE[..1]);
        draw_square_from_one(&vm);
        said.take();
        assert!(!vm.start());
        assert_eq!(vm.draw_now(), Draw::Naming(SQUARE.to_vec()));
        assert!(said.take().is_empty(), "the Declare button is not shown then, so there is nothing to tell");
    }

    /// Goes on from the first corner placed to the square closed.
    fn draw_square_from_one(vm: &AreasVm) {
        for p in &SQUARE[1..] {
            assert!(vm.add_at(*p));
        }
        assert!(vm.close());
    }

    #[test]
    fn an_outline_that_crosses_itself_is_refused_with_the_reason_and_drawing_goes_on() {
        let (vm, _, said, _) = rig();
        vm.start();
        for p in [SQUARE[0], SQUARE[2], SQUARE[1], SQUARE[3]] {
            vm.add_at(p);
        }
        said.take();
        assert!(!vm.close());
        assert_eq!(said.take(), ["The outline crosses itself. Move or remove a corner."]);
        assert!(matches!(vm.draw_now(), Draw::Drawing(c) if c.len() == 4));
    }

    #[test]
    fn a_save_the_storage_refuses_keeps_the_outline_and_says_so() {
        let (vm, _, said, storage) = rig();
        draw_square(&vm);
        said.take();
        storage.blocked(true);
        assert!(!vm.save());
        assert_eq!(said.take(), ["The area rules could not be saved in this browser."]);
        assert_eq!(vm.draw_now(), Draw::Naming(SQUARE.to_vec()), "nothing drawn is lost");
        storage.blocked(false);
        assert!(vm.save());
        assert_eq!(vm.list_now().len(), 1);
    }

    #[test]
    fn what_is_sent_to_the_map_is_the_areas_and_the_outline_being_drawn() {
        let (vm, mapper, ..) = rig();
        vm.start();
        vm.add_at(SQUARE[0]);
        vm.add_at(SQUARE[1]);
        vm.sync();
        let calls = mapper.take();
        assert!(matches!(&calls[0], MapCall::Restrictions { layers, data } if layers.contains("areas-fill") && data.contains("\"features\":[]")), "{calls:?}");
        assert!(matches!(&calls[1], MapCall::Draft { layers, data } if layers.contains("draft-vertex") && data.matches("\"Point\"").count() == 2), "{calls:?}");
        vm.close(); // refused: two corners
        vm.add_at(SQUARE[2]);
        vm.close();
        vm.set_form_name("A");
        vm.save();
        vm.sync();
        let calls = mapper.take();
        assert!(matches!(&calls[0], MapCall::Restrictions { data, .. } if data.contains("\"r1\"") && data.contains("car_free")), "{calls:?}");
        assert!(matches!(&calls[1], MapCall::Draft { data, .. } if data.contains("\"features\":[]")), "{calls:?}");
    }

    #[test]
    fn a_newer_document_shows_nothing_and_cannot_be_drawn_on() {
        let (ports, _, storage) = test_ports();
        let newer = r#"{"version":2,"restrictions":{}}"#;
        storage.remember(RESTRICTIONS_KEY, newer);
        let vm = AreasVm::new(ports, crate::i18n_for(Locale::En));
        assert!(vm.is_future());
        assert_eq!(vm.lead(), "These area rules were saved by a newer version of CityLoom, so they cannot be shown or changed here.");
        assert!(!vm.start());
        assert_eq!(storage.recall(RESTRICTIONS_KEY).as_deref(), Some(newer));
    }

    #[test]
    fn the_lead_counts_the_areas() {
        let (vm, ..) = rig();
        assert_eq!(vm.lead(), "No area rules yet. Declare one to check the streets inside it.");
        draw_square(&vm);
        assert!(vm.save());
        assert_eq!(vm.lead(), "One area rule. Select one to see the streets inside it.");
    }

    #[test]
    fn the_words_of_the_form_count_the_corners_and_name_the_rules_and_speeds() {
        let (vm, ..) = rig();
        assert_eq!(vm.corners_text(), "0 corners");
        vm.start();
        vm.add_at(SQUARE[0]);
        assert_eq!(vm.corners_text(), "1 corner");
        assert_eq!(vm.rule_choice("speed_limit"), "Speed limit");
        assert_eq!(vm.rule_choice("car_free"), "Car-free");
        assert_eq!(vm.kmh_text(30), "30 km/h");
        assert_eq!(vm.word("restriction-rule-no-parking"), "No parking or loading");
        let fr = AreasVm::new(test_ports().0, crate::i18n_for(Locale::FrCa));
        assert_eq!(fr.kmh_text(30), "30\u{a0}km/h");
        assert_eq!(fr.lead(), "Aucune règle de zone pour l’instant. Déclarez-en une pour vérifier les rues qu’elle contient.");
    }
}
