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
    /// What was deleted this session, the last on top: Undo brings it back.
    deleted: ArcRwSignal<Vec<Restriction>>,
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
            deleted: ArcRwSignal::new(Vec::new()),
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

    /// What was deleted last, while it can be brought back.
    pub fn deleted_note(&self) -> Option<String> {
        let last = self.deleted.with(|d| d.last().cloned())?;
        Some(self.i18n.tr("restriction-deleted", &Args::new().str("area", self.say(&last.title()))))
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

    /// Selects an area (from the list, or by a press on the map), or none. Says which, and its rule; letting go is
    /// not said.
    pub fn select(&self, id: Option<&str>) {
        let found = id.and_then(|id| self.find_now(id));
        let id = found.as_ref().map(|r| r.id.clone());
        if self.selected.get_untracked() == id {
            return;
        }
        self.selected.set(id);
        if let Some(r) = found {
            self.tell("restriction-selected", Args::new().str("area", self.say_now(&r.title())).str("rule", self.say_now(&r.rule.said())));
        }
    }

    /// Changes an area as it is kept now, then says so with `said` (given the area as it is now); says it was not
    /// kept when it could not be (gone, or storage refused).
    fn change(&self, id: &str, change: impl FnOnce(&mut Restriction), said: impl FnOnce(&Restriction)) -> bool {
        let kept = self.store.update(id, change);
        self.reload();
        match self.find_now(id).filter(|_| kept) {
            Some(r) => {
                said(&r);
                true
            }
            None => {
                self.tell("restriction-not-kept", Args::new());
                false
            }
        }
    }

    pub fn rename(&self, id: &str, name: &str) -> bool {
        let name = name.to_string();
        self.change(id, |r| r.name = name, |r| self.tell_about("restriction-renamed", r))
    }

    pub fn set_rule(&self, id: &str, rule: Rule) -> bool {
        self.change(
            id,
            |r| r.rule = rule,
            |r| self.tell("restriction-rule-changed", Args::new().str("area", self.say_now(&r.title())).str("rule", self.say_now(&r.rule.said()))),
        )
    }

    /// Deletes an area; it can be brought back for the session (`undo_delete`).
    pub fn delete(&self, id: &str) -> bool {
        let Some(r) = self.find_now(id) else { return false };
        if !self.store.remove(id) {
            self.reload();
            self.tell("restriction-not-kept", Args::new());
            return false;
        }
        self.deleted.update(|d| d.push(r.clone()));
        self.reload();
        self.tell_about("restriction-deleted", &r);
        true
    }

    /// Brings back the area deleted last, selected. If storage refuses, it stays there to bring back.
    pub fn undo_delete(&self) -> bool {
        let Some(r) = self.deleted.with_untracked(|d| d.last().cloned()) else { return false };
        let Some(back) = self.store.restore(&r) else {
            self.tell("restriction-not-kept", Args::new());
            return false;
        };
        self.deleted.update(|d| {
            d.pop();
        });
        self.reload();
        self.selected.set(Some(back.id.clone()));
        self.tell_about("restriction-restored", &back);
        true
    }

    /// What a key pressed on the map does to the outline being drawn, or to the selection; says whether it was one of its keys. Arrow
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
            (Draw::Idle, "Escape") if self.selected.get_untracked().is_some() => self.select(None),
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
    use crate::restriction::store::{RESTRICTIONS_KEY, RestrictionStore};
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

    fn declare(vm: &AreasVm, name: &str) {
        draw_square(vm);
        vm.set_form_name(name);
        assert!(vm.save());
    }

    #[test]
    fn an_area_is_renamed_given_another_rule_and_selected_with_words() {
        let (vm, _, said, _) = rig();
        declare(&vm, "A");
        vm.select(None);
        said.take();
        vm.select(Some("r1"));
        assert_eq!(said.take(), ["“A” selected: Car-free."]);
        vm.select(Some("r1"));
        assert!(said.take().is_empty(), "selected already: nothing to say");
        assert!(vm.rename("r1", "Plateau"));
        assert_eq!(said.take(), ["Renamed “Plateau”."]);
        assert!(vm.set_rule("r1", Rule::SpeedLimit { kmh: 30 }));
        assert_eq!(said.take(), ["The rule of “Plateau” is now: Speed limit of 30 km/h."]);
        assert_eq!(vm.find_now("r1").unwrap().rule, Rule::SpeedLimit { kmh: 30 });
        assert!(vm.press_key("Escape", false), "Escape lets go of the selection");
        assert_eq!(vm.selected_id_now(), None);
        assert!(!vm.press_key("Escape", false), "with nothing selected Escape is not the areas'");
        assert!(said.take().is_empty(), "letting go is not said");
    }

    #[test]
    fn an_area_named_with_nothing_is_called_by_its_rule() {
        let (vm, _, said, _) = rig();
        declare(&vm, "A");
        said.take();
        assert!(vm.rename("r1", "  "));
        assert_eq!(said.take(), ["Renamed “Car-free”."]);
        assert_eq!(vm.find_now("r1").unwrap().name, "  ", "kept as typed");
    }

    #[test]
    fn a_change_the_storage_refuses_is_said_and_not_shown() {
        let (vm, _, said, storage) = rig();
        declare(&vm, "A");
        said.take();
        storage.blocked(true);
        assert!(!vm.rename("r1", "B"));
        assert!(!vm.set_rule("r1", Rule::NoParkingOrLoading));
        assert!(!vm.delete("r1"));
        assert_eq!(said.take(), ["The area rules could not be saved in this browser."; 3]);
        let r = vm.find_now("r1").unwrap();
        assert_eq!((r.name.as_str(), r.rule), ("A", Rule::CarFree));
        assert_eq!(vm.deleted_note(), None, "what was not deleted cannot be brought back");
    }

    #[test]
    fn a_deleted_area_can_be_brought_back_for_the_session() {
        let (vm, _, said, _) = rig();
        declare(&vm, "A");
        said.take();
        assert!(vm.delete("r1"));
        assert_eq!(said.take(), ["“A” deleted."]);
        assert!(vm.list_now().is_empty() && vm.selected_id_now().is_none());
        assert_eq!(vm.deleted_note().as_deref(), Some("“A” deleted."));
        assert!(vm.undo_delete());
        assert_eq!(said.take(), ["“A” restored."]);
        assert_eq!(vm.list_now()[0].id, "r1");
        assert_eq!(vm.selected_id_now().as_deref(), Some("r1"));
        assert_eq!(vm.deleted_note(), None);
        assert!(!vm.undo_delete(), "nothing more to bring back");
        assert!(said.take().is_empty());
    }

    #[test]
    fn undo_brings_back_the_area_deleted_last_first_and_a_refused_one_stays_to_bring_back() {
        let (vm, _, said, storage) = rig();
        declare(&vm, "A");
        declare(&vm, "B");
        assert!(vm.delete("r1") && vm.delete("r2"));
        assert_eq!(vm.deleted_note().as_deref(), Some("“B” deleted."));
        said.take();
        storage.blocked(true);
        assert!(!vm.undo_delete());
        assert_eq!(said.take(), ["The area rules could not be saved in this browser."]);
        assert_eq!(vm.deleted_note().as_deref(), Some("“B” deleted."), "still there to bring back");
        storage.blocked(false);
        assert!(vm.undo_delete());
        assert_eq!(vm.deleted_note().as_deref(), Some("“A” deleted."));
        assert!(vm.undo_delete());
        assert_eq!(vm.list_now().iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["r1", "r2"]);
    }

    #[test]
    fn another_tab_removing_the_selected_area_clears_the_selection_and_a_change_to_it_is_refused() {
        let (vm, _, said, storage) = rig();
        declare(&vm, "A");
        said.take();
        assert!(RestrictionStore::new(storage.clone()).remove("r1"));
        vm.reload();
        assert!(vm.list_now().is_empty() && vm.selected_id_now().is_none());
        assert!(!vm.rename("r1", "back"));
        assert_eq!(said.take(), ["The area rules could not be saved in this browser."]);
        assert!(vm.list_now().is_empty(), "not brought back by the change");
    }

    #[test]
    fn the_selected_area_is_marked_on_the_map_with_its_corners() {
        let (vm, mapper, ..) = rig();
        declare(&vm, "A");
        vm.sync();
        let calls = mapper.take();
        assert!(matches!(&calls[0], MapCall::Restrictions { data, .. } if data.contains("\"selected\":true") && data.contains("\"i\":3")), "{calls:?}");
        vm.select(None);
        vm.sync();
        let calls = mapper.take();
        assert!(matches!(&calls[0], MapCall::Restrictions { data, .. } if !data.contains("\"selected\":true") && !data.contains("\"i\":")), "{calls:?}");
    }
}
