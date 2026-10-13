//! The street page's view-model: the street being edited, what is said when it
//! changes, and keeping what is made in the city. The views read its view and
//! send it what the person does.

use std::rc::Rc;

use leptos::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::restriction::model::{Finding, Restriction};
use crate::restriction::store::RestrictionStore;
use crate::shared::catalogue::StreetClass;
use crate::shared::core::{Core, Presents};
use crate::shared::i18n::{Args, I18n};
use crate::shared::keeper::Keeper;
use crate::shared::ports::Ports;
use crate::shared::units::Units;
use crate::street::model::{Editor, View};
use crate::street::text;

impl Presents for Editor {
    type View = View;
    fn present(&self) -> View {
        self.view()
    }
}

/// The storage key under which the view chosen for the street's drawing is kept, per browser.
pub const VIEW_KEY: &str = "cityloom-street-view";

/// How the street's drawing shows the street: its cross-section, which is edited, or the street in 2.5D,
/// which is only looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Section,
    Iso,
}

impl ViewMode {
    /// The word it is kept as.
    pub fn word(self) -> &'static str {
        match self {
            ViewMode::Section => "section",
            ViewMode::Iso => "iso",
        }
    }

    pub fn parse(word: &str) -> Option<ViewMode> {
        match word {
            "section" => Some(ViewMode::Section),
            "iso" => Some(ViewMode::Iso),
            _ => None,
        }
    }

    /// The view the toggle goes to from this one.
    pub fn other(self) -> ViewMode {
        match self {
            ViewMode::Section => ViewMode::Iso,
            ViewMode::Iso => ViewMode::Section,
        }
    }
}

pub struct StreetVm {
    core: Core<Editor>,
    ports: Ports,
    i18n: Rc<I18n>,
    keeper: Option<Rc<Keeper>>,
    binding: Option<CityBinding>,
    /// The areas whose outline holds the street, read when the page opened.
    areas: Vec<Restriction>,
    mode: ArcRwSignal<ViewMode>,
}

/// The areas whose outline holds the midpoint of the street a page edits, read once when the page opens: none for
/// a street that is not of the city, or a city that does not say where it lies.
fn areas_holding(ports: &Ports, binding: Option<&CityBinding>) -> Vec<Restriction> {
    let Some(CityBinding { store, place: Place::Street(edge) }) = binding else { return Vec::new() };
    let city = store.open();
    let (Some(shape), Some(projection)) = (city.street_shape(*edge), crate::city::projection::for_store(store, city.origin_m())) else {
        return Vec::new();
    };
    RestrictionStore::new(ports.storage.clone()).all().into_iter().filter(|r| r.holds(&shape, &projection)).collect()
}

impl StreetVm {
    /// A street to edit, saying what it says in the language of `i18n` (the one
    /// instance the shell switches). Bound to a place in the city, what is made
    /// is written back to it; otherwise it is a sandbox.
    pub fn new(ports: Ports, i18n: Rc<I18n>, street: Editor, binding: Option<CityBinding>) -> Rc<StreetVm> {
        let areas = areas_holding(&ports, binding.as_ref());
        Rc::new_cyclic(|me: &std::rc::Weak<StreetVm>| {
            // The view chosen on an earlier visit; the section when there is none, or it is not a view.
            let mode = ports.storage.recall(VIEW_KEY).as_deref().and_then(ViewMode::parse).unwrap_or(ViewMode::Section);
            let keeper = binding.is_some().then(|| {
                let (keep, told) = (me.clone(), me.clone());
                Keeper::new(
                    ports.scheduler.clone(),
                    move || keep.upgrade().is_some_and(|vm| vm.keep_now()),
                    move || {
                        if let Some(vm) = told.upgrade() {
                            vm.ports.announcer.say(&vm.i18n.tr_now("not-kept", &Args::new()));
                        }
                    },
                )
            });
            StreetVm { core: Core::new(street), ports, i18n, keeper, binding, areas, mode: ArcRwSignal::new(mode) }
        })
    }

    /// The words of the page, in the language the shell has chosen.
    pub fn i18n(&self) -> &Rc<I18n> {
        &self.i18n
    }

    // ---- what the views read ----

    /// The view, which a view that reads it draws again when the street changes.
    pub fn view(&self) -> Rc<View> {
        self.core.view()
    }

    /// The same, for a command: nothing is watched.
    pub fn view_now(&self) -> Rc<View> {
        self.core.view_now()
    }

    /// How the street stands against each area it is inside, as it is now: drawn again when the street is edited.
    pub fn area_checks(&self) -> Vec<Finding> {
        if self.areas.is_empty() {
            return Vec::new();
        }
        self.core.version().track();
        let street = self.core.read(|e| e.snapshot());
        self.areas.iter().map(|r| Finding::of(r, &street)).collect()
    }

    pub fn units(&self) -> Units {
        self.core.units()
    }

    pub fn units_now(&self) -> Units {
        self.core.units_now()
    }

    pub fn version(&self) -> RwSignal<u32> {
        self.core.version()
    }

    pub fn read<R>(&self, f: impl FnOnce(&Editor) -> R) -> R {
        self.core.read(f)
    }

    // ---- the view ----

    /// How the drawing shows the street; a view that reads it is drawn again when it changes.
    pub fn mode(&self) -> ViewMode {
        self.mode.get()
    }

    /// The same, for a command: nothing is watched.
    pub fn mode_now(&self) -> ViewMode {
        self.mode.get_untracked()
    }

    /// The person chose how the drawing shows the street: it is kept for the next visit and said. Choosing
    /// the view already shown does nothing. A store that refuses does not stop the change: it holds for this visit.
    pub fn set_mode(&self, mode: ViewMode) {
        if self.mode.get_untracked() == mode {
            return;
        }
        self.mode.set(mode);
        self.ports.storage.remember(VIEW_KEY, mode.word());
        let key = match mode {
            ViewMode::Section => "view-shown-section",
            ViewMode::Iso => "view-shown-iso",
        };
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new()));
    }

    /// The other view: what the toggle's `V` does.
    pub fn toggle_mode(&self) {
        self.set_mode(self.mode_now().other());
    }

    // ---- what the views do ----

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
    }

    /// Changes the street, tells whatever watches it, and sees that it is kept.
    pub fn edit<R>(&self, f: impl FnOnce(&mut Editor) -> R) -> R {
        let r = self.core.edit(f);
        if let Some(k) = &self.keeper {
            k.touch();
        }
        r
    }

    /// An edit the person made: announced by what it was and how the pieces
    /// stand. A refused edit says nothing.
    pub fn apply(&self, f: impl FnOnce(&mut Editor) -> bool) -> bool {
        let before = self.class_now();
        let done = self.edit(f);
        if done {
            self.say_edit(before);
        }
        done
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet<R>(&self, f: impl FnOnce(&mut Editor) -> R) -> R {
        self.edit(f)
    }

    /// A selection the person made, announced by what is selected.
    pub fn select(&self, uid: Option<u32>) {
        self.edit(|e| e.select(uid));
        self.say_selection();
    }

    pub fn select_relative(&self, delta: i32) {
        self.edit(|e| e.select_relative(delta));
        self.say_selection();
    }

    /// The end of a gesture: kept when `commit`, and announced as an edit if it
    /// changed anything.
    pub fn end_gesture(&self, commit: bool) {
        if commit {
            let before = self.class_now();
            if self.edit(|e| e.end_gesture()) {
                self.say_edit(before);
            }
        } else {
            self.edit(|e| e.cancel_gesture());
        }
    }

    pub fn undo(&self) -> bool {
        let before = self.class_now();
        let done = self.edit(|e| e.undo());
        if done {
            let said = text::undone_text(&self.view_now(), &self.i18n, self.units_now());
            self.ports.announcer.say(&self.with_class_note(said, before));
        }
        done
    }

    pub fn redo(&self) -> bool {
        let before = self.class_now();
        let done = self.edit(|e| e.redo());
        if done {
            let said = text::redone_text(&self.view_now(), &self.i18n, self.units_now());
            self.ports.announcer.say(&self.with_class_note(said, before));
        }
        done
    }

    pub fn reset(&self) -> bool {
        let before = self.class_now();
        let done = self.edit(|e| e.reset());
        if done {
            self.ports.announcer.say(&self.with_class_note(text::started_over(&self.i18n), before));
        }
        done
    }

    /// Arranges the roadway as an Atlas measure, or says it does not suit.
    pub fn apply_measure(&self, code: &str) -> bool {
        let before = self.class_now();
        let done = self.edit(|e| e.apply_measure(code));
        if done {
            self.say_edit(before);
        } else {
            self.ports.announcer.say(&text::measure_refused(&self.i18n));
        }
        done
    }

    /// The page is being left: what is waiting to be kept is kept now.
    pub fn flush(&self) {
        if let Some(k) = &self.keeper {
            k.flush();
        }
    }

    /// Writes the street as it now is back to its place in the city.
    fn keep_now(&self) -> bool {
        let Some(binding) = &self.binding else { return true };
        binding.keep_street(self.core.read(|e| e.snapshot()))
    }

    /// The class the street has now, for telling after an edit whether it changed.
    fn class_now(&self) -> StreetClass {
        self.view_now().class
    }

    /// `said`, and after it what the edit did to the street's class, if it changed.
    fn with_class_note(&self, said: String, before: StreetClass) -> String {
        match text::class_change_text(before, &self.view_now(), &self.i18n, self.units_now()) {
            Some(note) => format!("{said} {note}"),
            None => said,
        }
    }

    fn say_edit(&self, before: StreetClass) {
        if let Some(t) = text::edit_text(&self.view_now(), &self.i18n, self.units_now()) {
            self.ports.announcer.say(&self.with_class_note(t, before));
        }
    }

    fn say_selection(&self) {
        if let Some(t) = text::selection_text(&self.view_now(), &self.i18n, self.units_now()) {
            self.ports.announcer.say(&t);
        }
    }
}

impl crate::shell::Target for StreetVm {
    fn set_units(&self, units: Units) {
        StreetVm::set_units(self, units);
    }

    fn apply_region(&self, region: usize) -> bool {
        self.edit(|e| e.set_region(region))
    }

    fn region_id(&self) -> String {
        self.view_now().region.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::binding::Place;
    use crate::city::store::CityStore;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{ManualScheduler, MemoryStorage, RecordingAnnouncer, test_ports_with_time};

    struct Rig {
        vm: Rc<StreetVm>,
        said: Rc<RecordingAnnouncer>,
        time: Rc<ManualScheduler>,
        storage: Rc<MemoryStorage>,
    }

    fn rig(sample: usize) -> Rig {
        let (ports, said, storage, time) = test_ports_with_time();
        Rig { vm: StreetVm::new(ports, crate::i18n_for(Locale::En), Editor::new(sample), None), said, time, storage }
    }

    fn rig_on(street: crate::street::model::Street) -> Rig {
        let (ports, said, storage, time) = test_ports_with_time();
        Rig { vm: StreetVm::new(ports, crate::i18n_for(Locale::En), Editor::from_street(&street, &street, 0), None), said, time, storage }
    }

    #[test]
    fn making_the_street_pedestrian_is_announced_with_what_it_became_and_undo_with_what_it_is_again() {
        use crate::shared::catalogue::kind_index;
        let r = rig(0);
        let sidewalk = kind_index("sidewalk").unwrap();
        assert!(r.vm.apply(|e| e.make_pedestrian(sidewalk)));
        let said = r.said.take();
        assert_eq!(said.len(), 1, "one announcement: the live region keeps only the last one said");
        assert!(said[0].starts_with("Made pedestrian. "), "{said:?}");
        assert!(said[0].ends_with(" Sample Street 1 is now a pedestrian street."), "{said:?}");
        assert!(!r.vm.apply(|e| e.make_pedestrian(sidewalk)), "no piece is a motor piece now");
        assert!(r.said.take().is_empty());
        assert!(r.vm.undo());
        assert!(r.said.take()[0].ends_with(" Sample Street 1 carries traffic again."));
        assert!(r.vm.redo());
        assert!(r.said.take()[0].ends_with(" Sample Street 1 is now a pedestrian street."));
        assert!(r.vm.reset());
        assert!(r.said.take()[0].ends_with(" Sample Street 1 carries traffic again."));
    }

    #[test]
    fn an_edit_that_leaves_the_class_alone_says_nothing_of_it() {
        let r = rig(0);
        let uid = first(&r.vm);
        assert!(r.vm.apply(|e| e.nudge_width(uid, 100)));
        let said = r.said.take();
        assert!(!said[0].contains("pedestrian") && !said[0].contains("traffic again"), "{said:?}");
    }

    #[test]
    fn taking_out_the_last_lane_and_giving_one_back_are_announced_in_the_language_of_the_page() {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::{Piece, Street};
        let piece = |id: &str| Piece { kind: kind_index(id).unwrap(), width_mm: 3000, direction: None, variants: Vec::new() };
        let r = rig_on(Street::imported(StreetClass::Local, Side::Right, &[piece("sidewalk"), piece("travel")]).named("Rue Rachel"));
        let lane = r.vm.view_now().segments[1].uid;
        assert!(r.vm.apply(|e| e.remove(lane)));
        assert!(r.said.take()[0].ends_with(" Rue Rachel is now a pedestrian street."));
        assert!(r.vm.apply(|e| e.add(kind_index("travel").unwrap(), 1) != 0));
        assert!(r.said.take()[0].ends_with(" Rue Rachel carries traffic again."));
        r.vm.i18n().set(Locale::FrCa);
        assert!(r.vm.undo());
        assert!(r.said.take()[0].ends_with(" Rue Rachel est maintenant une rue piétonne."));
    }

    fn city_rig() -> (Rig, CityBinding) {
        let (ports, said, storage, time) = test_ports_with_time();
        let store = CityStore::sample(ports.storage.clone());
        let edge = store.open().view(0).edges[0].uid;
        let street = store.open().street_editor(edge, 0).unwrap();
        let binding = CityBinding { store, place: Place::Street(edge) };
        (Rig { vm: StreetVm::new(ports, crate::i18n_for(Locale::En), street, Some(binding.clone())), said, time, storage }, binding)
    }

    fn first(vm: &StreetVm) -> u32 {
        vm.view_now().segments[0].uid
    }

    #[test]
    fn an_edit_counts_and_a_view_is_built_once_per_edit() {
        let r = rig(0);
        let (a, version, uid) = (r.vm.view_now(), r.vm.version(), first(&r.vm));
        assert!(Rc::ptr_eq(&a, &r.vm.view_now()));
        assert!(r.vm.edit(|e| e.nudge_width(uid, 100)));
        assert_eq!(version.get_untracked(), 1);
        let b = r.vm.view_now();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(b.revisions.len(), 1);
    }

    #[test]
    fn the_units_start_in_metres_and_changing_them_is_not_an_edit() {
        let r = rig(0);
        assert_eq!(r.vm.units_now(), Units::Metres);
        let a = r.vm.view_now();
        r.vm.set_units(Units::Feet);
        assert_eq!(r.vm.units_now(), Units::Feet);
        assert!(Rc::ptr_eq(&a, &r.vm.view_now()));
        assert_eq!(r.vm.version().get_untracked(), 0);
    }

    #[test]
    fn an_edit_is_announced_and_a_refused_one_is_not() {
        let r = rig(0);
        let uid = first(&r.vm);
        assert!(r.vm.apply(|e| e.remove(uid)));
        assert!(r.said.take()[0].ends_with(". 3.3 m left to use."));
        assert!(!r.vm.apply(|e| e.remove(uid)));
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn the_street_says_what_it_says_in_the_language_the_shell_switches() {
        let r = rig(0);
        let uid = first(&r.vm);
        // The shell holds this same instance, so a switch there is a switch here.
        let shell = r.vm.i18n().clone();
        shell.set(Locale::FrCa);
        r.vm.select(Some(uid));
        assert_eq!(r.said.take(), vec!["Trottoir, 3,3\u{a0}m, 1 sur 6"]);
        shell.set(Locale::En);
        r.vm.select(Some(uid));
        assert_eq!(r.said.take(), vec!["Sidewalk, 3.3 m, 1 of 6"]);
    }

    #[test]
    fn a_quiet_change_says_nothing_but_still_counts_as_an_edit() {
        let r = rig(0);
        let uid = first(&r.vm);
        assert!(r.vm.quiet(|e| e.nudge_width(uid, 100)));
        assert!(r.said.take().is_empty());
        assert_eq!(r.vm.version().get_untracked(), 1);
    }

    #[test]
    fn selecting_announces_the_piece_and_clearing_says_nothing() {
        let r = rig(0);
        r.vm.select(Some(first(&r.vm)));
        assert_eq!(r.said.take(), vec!["Sidewalk, 3.3 m, 1 of 6"]);
        r.vm.select(None);
        assert!(r.said.take().is_empty());
        r.vm.select_relative(1);
        assert_eq!(r.said.take().len(), 1);
    }

    #[test]
    fn a_gesture_is_one_edit_when_it_ends_and_nothing_when_it_changed_nothing() {
        let r = rig(0);
        r.vm.select(Some(first(&r.vm)));
        r.said.take();
        r.vm.quiet(|e| e.begin_gesture());
        assert!(r.vm.quiet(|e| e.resize_boundary(0, 200)));
        assert_eq!(r.vm.view_now().revisions.len(), 0);
        r.vm.end_gesture(true);
        assert_eq!(r.vm.view_now().revisions.len(), 1);
        assert!(r.said.take()[0].contains(". "));
        r.vm.quiet(|e| e.begin_gesture());
        r.vm.end_gesture(true);
        assert!(r.said.take().is_empty(), "a gesture that changed nothing says nothing");
        r.vm.quiet(|e| e.begin_gesture());
        r.vm.quiet(|e| e.resize_boundary(0, 300));
        r.vm.end_gesture(false);
        assert_eq!(r.vm.view_now().revisions.len(), 1);
    }

    #[test]
    fn undo_redo_and_start_over_are_announced() {
        let r = rig(0);
        let uid = first(&r.vm);
        r.vm.edit(|e| e.remove(uid));
        assert!(r.vm.undo());
        assert_eq!(r.said.take(), vec!["Undone. Every metre of the street is used."]);
        assert!(r.vm.redo());
        assert_eq!(r.said.take(), vec!["Redone. 3.3 m left to use."]);
        assert!(r.vm.reset());
        assert_eq!(r.said.take(), vec!["Started over from the street as it is today. Undo brings your changes back."]);
        // Starting over can itself be undone, which brings the changes back.
        assert!(r.vm.undo());
        assert_eq!(r.said.take(), vec!["Undone. 3.3 m left to use."]);
    }

    #[test]
    fn a_measure_is_announced_when_it_is_arranged_and_refused_in_words_when_it_is_not() {
        let r = rig(1);
        assert!(r.vm.apply_measure("B1"));
        assert_eq!(r.said.take().len(), 1);
        assert!(!r.vm.apply_measure("B2"));
        assert_eq!(r.said.take(), vec!["That measure does not suit this street."]);
    }

    #[test]
    fn what_is_announced_after_an_edit_and_a_refusal_is_said_in_the_language_of_the_page() {
        let (ports, said, _storage, _time) = test_ports_with_time();
        let i18n = crate::i18n_for(Locale::En);
        let vm = StreetVm::new(ports, i18n.clone(), Editor::new(1), None);
        i18n.set(Locale::FrCa);
        let uid = first(&vm);
        assert!(vm.apply(|e| e.remove(uid)));
        let told = said.take();
        assert!(told[0].starts_with("Retrait\u{a0}: trottoir. Il reste 3,5\u{a0}m à utiliser"), "{told:?}");
        assert!(!vm.apply_measure("B2"));
        assert_eq!(said.take(), vec!["Cette mesure ne convient pas à cette rue."]);
        // The edit was made in French; said again in English, it follows.
        i18n.set(Locale::En);
        assert!(vm.undo() && vm.redo());
        assert_eq!(said.take().last().unwrap(), "Redone. 3.5 m left to use.");
    }

    #[test]
    fn what_is_announced_follows_the_units() {
        let r = rig(0);
        r.vm.set_units(Units::Feet);
        let uid = first(&r.vm);
        r.vm.apply(|e| e.remove(uid));
        assert!(r.said.take()[0].ends_with(". 10.8 ft left to use."));
    }

    #[test]
    fn a_sandbox_keeps_nothing_in_storage() {
        use crate::shared::ports::Storage;
        let r = rig(0);
        let uid = first(&r.vm);
        r.vm.edit(|e| e.nudge_width(uid, 100));
        r.time.advance(1_000);
        assert_eq!(r.time.pending(), 0);
        assert_eq!(r.storage.recall(crate::city::edits::EDITS_KEY), None);
    }

    #[test]
    fn what_is_made_of_a_city_street_is_kept_in_the_city_a_moment_after_the_last_change() {
        let (r, binding) = city_rig();
        let uid = first(&r.vm);
        assert!(r.vm.edit(|e| e.nudge_width(uid, 100)));
        r.time.advance(200);
        assert_eq!(binding.store.open().view(0).edited, 0, "not yet");
        r.time.advance(100);
        assert_eq!(binding.store.open().view(0).edited, 1);
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn leaving_the_page_keeps_what_is_waiting_at_once() {
        let (r, binding) = city_rig();
        let uid = first(&r.vm);
        r.vm.edit(|e| e.nudge_width(uid, 100));
        r.vm.flush();
        assert_eq!(binding.store.open().view(0).edited, 1);
    }

    #[test]
    fn a_city_that_cannot_be_written_is_said_once_and_the_page_goes_on() {
        let (r, _) = city_rig();
        r.storage.blocked(true);
        let uid = first(&r.vm);
        r.vm.edit(|e| e.nudge_width(uid, 100));
        r.time.advance(300);
        r.vm.edit(|e| e.nudge_width(uid, 100));
        r.time.advance(300);
        assert_eq!(r.said.take(), vec![crate::i18n_for(crate::shared::i18n::Locale::En).tr_now("not-kept", &crate::shared::i18n::Args::new())]);
        assert_eq!(r.vm.view_now().revisions.len(), 2);
    }

    #[test]
    fn a_city_that_cannot_be_written_is_said_in_the_language_of_the_page() {
        let (r, _) = city_rig();
        r.vm.i18n().set(Locale::FrCa);
        r.storage.blocked(true);
        let uid = first(&r.vm);
        r.vm.edit(|e| e.nudge_width(uid, 100));
        r.time.advance(300);
        r.vm.edit(|e| e.nudge_width(uid, 100));
        r.time.advance(300);
        assert_eq!(r.said.take(), vec!["Ce navigateur ne conserve pas vos modifications, alors les autres pages ne les verront pas."]);
    }

    use crate::restriction::fixtures::{bent_midpoint, bent_store, square};
    use crate::restriction::model::Rule;
    use crate::restriction::store::RestrictionStore;

    /// Bendville's street on its page, with the areas kept first.
    fn bendville_rig(areas: &[(&str, Rule, f64)]) -> Rig {
        let (ports, said, storage, time) = test_ports_with_time();
        let store = bent_store(storage.clone());
        let (_, mid) = bent_midpoint(&store);
        for (name, rule, east) in areas {
            RestrictionStore::new(storage.clone()).add(name, *rule, square([mid[0] + east, mid[1]], 0.0005)).unwrap();
        }
        let street = store.open().street_editor(1, 0).unwrap();
        let binding = CityBinding { store, place: Place::Street(1) };
        Rig { vm: StreetVm::new(ports, crate::i18n_for(Locale::En), street, Some(binding)), said, time, storage }
    }

    #[test]
    fn a_street_inside_an_area_is_checked_against_its_rule_and_follows_the_edits() {
        let r = bendville_rig(&[("Car zone", Rule::CarFree, 0.0), ("Elsewhere", Rule::CarFree, 0.05)]);
        let checks = r.vm.area_checks();
        assert_eq!(checks.len(), 1, "only the area that holds the street's midpoint");
        assert!(!checks[0].ok());
        assert_eq!(crate::shared::testing::en(&checks[0].label()), "Area rule: Car zone");
        let uid = first(&r.vm);
        assert!(r.vm.apply(|e| e.remove(uid)));
        assert_eq!(crate::shared::testing::en(&r.vm.area_checks()[0].detail()[1]), "It has a driving lane.");
    }

    #[test]
    fn a_street_outside_every_area_or_not_of_the_city_has_no_area_check() {
        assert!(bendville_rig(&[]).vm.area_checks().is_empty());
        assert!(rig(0).vm.area_checks().is_empty(), "a sandbox street lies nowhere");
        assert!(city_rig().0.vm.area_checks().is_empty(), "the sample city lies nowhere");
    }

    #[test]
    fn a_city_with_a_network_but_a_layout_that_lies_nowhere_has_no_area_check_even_under_an_area() {
        let (ports, _, storage, _) = test_ports_with_time();
        let everywhere = vec![[-180.0, -85.0], [180.0, -85.0], [180.0, 85.0], [-180.0, 85.0]];
        RestrictionStore::new(storage.clone()).add("Everywhere", Rule::CarFree, everywhere).unwrap();
        let store = CityStore::sample(ports.storage.clone());
        assert!(store.has_network() && store.open().origin_m().is_none());
        let edge = store.open().view(0).edges[0].uid;
        let street = store.open().street_editor(edge, 0).unwrap();
        let binding = CityBinding { store, place: Place::Street(edge) };
        assert!(StreetVm::new(ports, crate::i18n_for(Locale::En), street, Some(binding)).area_checks().is_empty());
    }

    #[test]
    fn the_drawing_starts_as_the_section_and_toggling_shows_the_2_5d_keeps_the_choice_and_says_it() {
        use crate::shared::ports::Storage;
        let r = rig(0);
        assert_eq!(r.vm.mode_now(), ViewMode::Section);
        r.vm.toggle_mode();
        assert_eq!(r.vm.mode_now(), ViewMode::Iso);
        assert_eq!(r.storage.recall(VIEW_KEY).as_deref(), Some("iso"));
        assert_eq!(r.said.take(), vec!["Showing the street in 2.5D"]);
        r.vm.toggle_mode();
        assert_eq!(r.vm.mode_now(), ViewMode::Section);
        assert_eq!(r.storage.recall(VIEW_KEY).as_deref(), Some("section"));
        assert_eq!(r.said.take(), vec!["Showing the section"]);
        // Changing the view is not an edit.
        assert_eq!(r.vm.version().get_untracked(), 0);
    }

    #[test]
    fn choosing_the_view_already_shown_changes_nothing_and_says_nothing() {
        let r = rig(0);
        r.vm.set_mode(ViewMode::Section);
        assert!(r.said.take().is_empty());
        r.vm.set_mode(ViewMode::Iso);
        r.said.take();
        r.vm.set_mode(ViewMode::Iso);
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn the_view_chosen_is_the_one_the_next_page_opens_with() {
        let (ports, _said, _storage, _time) = test_ports_with_time();
        let first = StreetVm::new(ports.clone(), crate::i18n_for(Locale::En), Editor::new(0), None);
        first.set_mode(ViewMode::Iso);
        let next = StreetVm::new(ports, crate::i18n_for(Locale::En), Editor::new(0), None);
        assert_eq!(next.mode_now(), ViewMode::Iso);
    }

    #[test]
    fn the_view_change_is_said_in_the_language_of_the_page() {
        let r = rig(0);
        r.vm.i18n().set(Locale::FrCa);
        r.vm.toggle_mode();
        assert_eq!(r.said.take(), vec!["Rue affichée en 2,5D"]);
        r.vm.toggle_mode();
        assert_eq!(r.said.take(), vec!["Coupe affichée"]);
    }

    #[test]
    fn a_blocked_store_or_an_unknown_word_starts_the_section_and_the_toggle_still_works() {
        use crate::shared::ports::Storage;
        let (ports, said, storage, _time) = test_ports_with_time();
        storage.remember(VIEW_KEY, "sideways");
        let vm = StreetVm::new(ports, crate::i18n_for(Locale::En), Editor::new(0), None);
        assert_eq!(vm.mode_now(), ViewMode::Section);
        storage.blocked(true);
        vm.toggle_mode();
        assert_eq!(vm.mode_now(), ViewMode::Iso, "it holds for this visit");
        assert_eq!(said.take(), vec!["Showing the street in 2.5D"]);
        // Nothing was kept: the next visit reads what the store held before, and starts on the section.
        assert_eq!(storage.recall(VIEW_KEY).as_deref(), Some("sideways"));
    }

    #[test]
    fn a_view_is_written_as_a_word_and_read_back() {
        for m in [ViewMode::Section, ViewMode::Iso] {
            assert_eq!(ViewMode::parse(m.word()), Some(m));
            assert_ne!(m.other(), m);
            assert_eq!(m.other().other(), m);
        }
        assert_eq!(ViewMode::parse(""), None);
    }
}
