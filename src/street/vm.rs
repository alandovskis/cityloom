//! The street page's view-model: the street being edited, what is said when it
//! changes, and keeping what is made in the city. The views read its view and
//! send it what the person does.

use std::rc::Rc;

use leptos::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::restriction::model::{Finding, Restriction};
use crate::restriction::store::RestrictionStore;
use crate::shared::catalogue::{PEDESTRIAN_KINDS, StreetClass, kind_index};
use crate::shared::core::{Core, Presents};
use crate::shared::i18n::{Args, I18n};
use crate::shared::keeper::Keeper;
use crate::shared::ports::Ports;
use crate::shared::units::Units;
use crate::street::model::{Editor, Keep, View, ZoneRefusal};
use crate::street::text;
use crate::sync::model::{Change, Origin};

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
    /// A merge waiting for the choice of the section to keep: with the next zone (true) or the previous one.
    merge: ArcRwSignal<Option<bool>>,
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

/// The choices of the Transform menu (`StreetVm::transform_menu`).
#[derive(Clone, Debug, PartialEq)]
pub struct TransformMenu {
    /// Catalogue indices of the kinds the motor pieces can become.
    pub pedestrian: Vec<usize>,
    /// Atlas codes of the measures the street can be arranged as.
    pub transit: Vec<&'static str>,
}

impl TransformMenu {
    /// Nothing to offer: the button is not shown.
    pub fn is_empty(&self) -> bool {
        self.pedestrian.is_empty() && self.transit.is_empty()
    }
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
            StreetVm { core: Core::new(street), ports, i18n, keeper, binding, areas, mode: ArcRwSignal::new(mode), merge: ArcRwSignal::new(None) }
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

    /// What the Transform menu offers, drawn again when the street is edited: the kinds the motor pieces can become
    /// (while the street has a motor piece) and the transit measures the street is not already and can be arranged
    /// as, in the Notes table's order: the table's Arrange rows. A measure that will not fit is not listed.
    pub fn transform_menu(&self) -> TransformMenu {
        let v = self.view();
        TransformMenu {
            pedestrian: if v.has_motor { PEDESTRIAN_KINDS.iter().filter_map(|id| kind_index(id)).collect() } else { Vec::new() },
            transit: v.measures.iter().filter(|m| m.can_apply && !m.present).map(|m| m.code).collect(),
        }
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

    // ---- zones ----

    /// A zone edit the person made: announced like any edit, and a refusal is said in the page's words and
    /// changes nothing (not even what is kept).
    fn zone_edit(&self, f: impl FnOnce(&mut Editor) -> Result<(), ZoneRefusal>) -> bool {
        let before = self.class_now();
        match self.core.edit(f) {
            Ok(()) => {
                if let Some(k) = &self.keeper {
                    k.touch();
                }
                self.merge.set(None);
                self.say_edit(before);
                true
            }
            Err(refusal) => {
                self.ports.announcer.say(&text::zone_refused(&self.i18n, self.units_now(), refusal));
                false
            }
        }
    }

    /// Chooses a zone, announcing it. A zone that is not there, or is cut off, is left alone and nothing is said.
    pub fn select_zone(&self, zone: usize) {
        if self.edit(|e| e.select_zone(zone)) {
            self.merge.set(None);
            self.say_zone();
        }
    }

    /// The zone `delta` places along from the selected one, if there is one.
    pub fn select_zone_relative(&self, delta: i32) {
        let target = self.view_now().zones.selected as i64 + delta as i64;
        if let Ok(zone) = usize::try_from(target) {
            self.select_zone(zone);
        }
    }

    /// The first zone, or the last one that is not cut off (Home and End).
    pub fn select_zone_edge(&self, last: bool) {
        let zones = &self.view_now().zones.zones;
        let target = if last { zones.iter().rev().find(|z| !z.cut_off).map(|z| z.index) } else { Some(0) };
        if let Some(zone) = target {
            self.select_zone(zone);
        }
    }

    pub fn split_zone(&self, at_mm: Option<i32>) -> bool {
        self.zone_edit(|e| e.split_zone(at_mm))
    }

    /// Merges the selected zone with the next (or previous) one at once when the two have the same section; else
    /// waits for the choice of the one to keep (`finish_merge`, or `cancel_merge`). False when it was refused.
    pub fn begin_merge(&self, next: bool) -> bool {
        if self.core.read(|e| e.merge_needs_choice(next)) {
            // A choice that could not be carried out is not asked for: the refusal comes first.
            if let Some(refusal) = self.core.read(|e| e.structure_blocked()) {
                self.ports.announcer.say(&text::zone_refused(&self.i18n, self.units_now(), refusal));
                return false;
            }
            self.merge.set(Some(next));
            return true;
        }
        self.zone_edit(|e| e.merge_zone(next, Keep::Selected))
    }

    /// The merge waiting for a choice, if there is one: with the next zone (true) or the previous one.
    pub fn pending_merge(&self) -> Option<bool> {
        self.merge.get()
    }

    /// The same, for a command: nothing is watched.
    pub fn pending_merge_now(&self) -> Option<bool> {
        self.merge.get_untracked()
    }

    pub fn finish_merge(&self, keep: Keep) -> bool {
        let Some(next) = self.merge.get_untracked() else { return false };
        self.merge.set(None);
        self.zone_edit(|e| e.merge_zone(next, keep))
    }

    pub fn cancel_merge(&self) {
        self.merge.set(None);
    }

    /// Moves the selected zone's start by `delta_mm`, saying why when it cannot.
    pub fn nudge_boundary(&self, delta_mm: i32) -> bool {
        let view = self.view_now();
        let Some(from) = view.zones.zones.get(view.zones.selected).map(|z| z.from_mm) else { return false };
        self.move_boundary(from + delta_mm)
    }

    pub fn move_boundary(&self, to_mm: i32) -> bool {
        self.zone_edit(|e| e.move_boundary(to_mm))
    }

    pub fn begin_boundary_drag(&self) {
        self.edit(|e| e.begin_gesture());
    }

    /// One step of a drag of the boundary: shown, not announced.
    pub fn drag_boundary_to(&self, to_mm: i32) {
        self.quiet(|e| e.move_boundary(to_mm).is_ok());
    }

    /// The end of a boundary drag: kept as one edit, and announced, when `commit`.
    pub fn end_boundary_drag(&self, commit: bool) {
        self.end_gesture(commit);
    }

    /// Says that a typed length for a zone is not a number.
    pub fn say_bad_number(&self) {
        self.ports.announcer.say(&self.i18n.tr_now("zone-bad-number", &Args::new()));
    }

    pub fn name_zone(&self, name: &str) -> bool {
        self.apply(|e| e.name_zone(Some(name)))
    }

    pub fn reset_zone(&self) -> bool {
        self.apply(|e| e.reset_zone())
    }

    pub fn use_for_street(&self) -> bool {
        self.apply(|e| e.use_for_street())
    }

    fn say_zone(&self) {
        self.ports.announcer.say(&text::zone_selection_text(&self.view_now(), &self.i18n, self.units_now()));
    }

    /// Hears the sync, once, when the page opens: before a sync reads the stores this page writes what it holds, and
    /// when a sync changed the city underneath it reloads the street (and says so).
    pub fn watch_sync(self: &Rc<Self>) {
        crate::sync::hub::watch(self, Self::flush, Self::on_synced);
    }

    /// A sync changed streets. If this one now differs from the city, the city's is shown (the selected piece stays
    /// selected if it is still there); the street's own record losing to a later edit is said as that, anything else
    /// as an update. The editor is replaced, so what the person could undo in this place is gone, and so is a drag
    /// under way (a drag was flushed before the sync read the stores, so it is lost only when the city's street
    /// differs from it).
    fn on_synced(&self, changes: &[Change]) {
        let Some(binding) = &self.binding else { return };
        let Place::Street(edge) = binding.place else { return };
        // Only news of this very street is about it.
        let Some(own) = binding.record() else { return };
        if !changes.iter().any(|c| c.id == own) {
            return;
        }
        // What the person made in the last moments is later than anything pulled: it is written (and marked, to be
        // sent on the next run) before the editor is compared with the city.
        self.flush();
        let Some(mut fresh) = binding.store.open().street_editor(edge, binding.store.region()) else { return };
        if fresh.snapshot() == self.core.read(|e| e.snapshot()) {
            return;
        }
        fresh.select(self.view_now().selected);
        // The zone stays selected if it is still there; the frame came with the editor.
        let zone = self.core.read(|e| e.zone());
        fresh.select_zone(zone);
        self.merge.set(None);
        let replaced = changes.iter().any(|c| c.id == own && c.origin == Origin::Replaced);
        self.core.edit(|e| *e = fresh);
        self.ports.announcer.say(&self.i18n.tr_now(if replaced { "sync-replaced" } else { "sync-updated" }, &Args::new()));
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

    /// Another device's edit of the rig's street, applied to the city.
    fn remote_edit(binding: &CityBinding) {
        let Place::Street(edge) = binding.place else { unreachable!() };
        let mut s = binding.store.open().street_editor(edge, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(binding.store.write_street(edge, s));
    }

    #[test]
    fn a_street_changed_by_a_sync_is_reloaded_and_announced_and_one_that_did_not_change_is_left_alone() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Id, Kind, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let before = r.vm.core.read(|e| e.snapshot().segments[0].width_mm);
        remote_edit(&binding);
        let own = binding.record().unwrap();
        r.said.take();
        hub::publish(&[Change { id: own.clone(), origin: Origin::Remote }]);
        assert_eq!(r.vm.core.read(|e| e.snapshot().segments[0].width_mm), before - 100);
        assert_eq!(r.said.take(), ["Updated from another device."]);
        // the same news again changes nothing and says nothing
        hub::publish(&[Change { id: own.clone(), origin: Origin::Remote }]);
        assert!(r.said.take().is_empty());
        // a change of another kind is none of this page's business
        hub::publish(&[Change { id: Id::new(Kind::Rule, "r-1"), origin: Origin::Remote }]);
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn a_street_whose_own_change_lost_says_so_and_in_french_when_the_page_is() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        remote_edit(&binding);
        r.vm.i18n().set(Locale::FrCa);
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Replaced }]);
        assert_eq!(r.said.take(), ["Votre modification de ce lieu a été remplacée par une plus récente."]);
    }

    #[test]
    fn a_sync_asking_the_editor_to_flush_has_what_is_waiting_written_first() {
        use crate::sync::hub;
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let uid = first(&r.vm);
        assert!(r.vm.apply(|e| e.nudge_width(uid, 100)));
        assert_eq!(binding.store.open().view(0).edited, 0, "the save waits for its moment");
        hub::flush();
        assert_eq!(binding.store.open().view(0).edited, 1, "a sync is about to read the stores");
    }

    #[test]
    fn a_reload_keeps_the_selected_piece_and_a_drag_under_way_that_the_sync_did_not_touch_goes_on() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let uid = first(&r.vm);
        r.vm.select(Some(uid));
        remote_edit(&binding);
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.vm.view_now().selected, Some(uid), "the selection survives the reload");
        // a drag under way, not yet written: a sync of this very street writes it first, so nothing differs
        r.vm.edit(|e| e.begin_gesture());
        assert!(r.vm.quiet(|e| e.nudge_width(uid, 100)));
        let dragged = r.vm.core.read(|e| e.snapshot());
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert!(r.said.take().is_empty());
        assert_eq!(r.vm.core.read(|e| e.snapshot()), dragged);
        r.vm.end_gesture(true);
        assert!(r.vm.view_now().can_undo, "the drag is still one undoable edit");
    }

    #[test]
    fn an_edit_made_a_moment_before_a_sync_is_not_lost_and_nothing_is_announced() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let uid = first(&r.vm);
        assert!(r.vm.apply(|e| e.nudge_width(uid, 100)));
        let edited = r.vm.core.read(|e| e.snapshot());
        assert_eq!(binding.store.open().view(0).edited, 0, "still waiting in the keeper");
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.vm.core.read(|e| e.snapshot()), edited, "the later edit wins");
        assert!(r.said.take().is_empty());
        assert_eq!(binding.store.open().view(0).edited, 1, "and it was written, to be sent on the next run");
    }

    #[test]
    fn another_streets_change_does_not_touch_this_editor() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Id, Kind, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let uid = first(&r.vm);
        assert!(r.vm.apply(|e| e.nudge_width(uid, 100)));
        r.said.take();
        hub::publish(&[Change { id: Id::new(Kind::Street, "another"), origin: Origin::Remote }]);
        assert!(r.said.take().is_empty());
        assert_eq!(binding.store.open().view(0).edited, 0, "not even written: the news is not about this street");
    }

    #[test]
    fn a_reload_during_a_drag_when_the_city_differs_refuses_the_drag_that_follows() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        r.vm.edit(|e| e.begin_gesture());
        r.time.advance(1_000); // nothing waits to be written
        assert_eq!(r.time.pending(), 0);
        remote_edit(&binding);
        // the drag has not changed anything yet, so the city's newer street wins
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert!(!r.vm.quiet(|e| e.resize_boundary(0, 50)), "the drag's next step is refused");
        let reloaded = r.vm.core.read(|e| e.snapshot());
        r.said.take();
        r.vm.end_gesture(true);
        assert_eq!(r.vm.core.read(|e| e.snapshot()), reloaded, "ending the dead drag changes nothing");
        assert!(r.said.take().is_empty());
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
    fn the_transform_menu_offers_the_pedestrian_kinds_and_only_the_measures_that_fit() {
        use crate::shared::catalogue::{PEDESTRIAN_KINDS, kind_index};
        let r = rig(1);
        let menu = r.vm.transform_menu();
        let kinds: Vec<usize> = PEDESTRIAN_KINDS.iter().filter_map(|id| kind_index(id)).collect();
        assert_eq!(menu.pedestrian, kinds);
        let arrangeable: Vec<&str> = r.vm.view_now().measures.iter().filter(|m| m.can_apply && !m.present).map(|m| m.code).collect();
        assert!(!arrangeable.is_empty());
        assert_eq!(menu.transit, arrangeable);
        let view = r.vm.view_now();
        for m in view.measures.iter().filter(|m| !m.can_apply) {
            assert!(!menu.transit.contains(&m.code), "{} will not fit", m.code);
        }
        // Choosing one arranges it as one undoable edit, and it is then this street's and no longer offered.
        let code = menu.transit[0];
        let steps = r.vm.view_now().revisions.len();
        assert!(r.vm.apply_measure(code));
        assert_eq!(r.vm.view_now().revisions.len(), steps + 1);
        assert!(!r.vm.transform_menu().transit.contains(&code));
        assert!(r.vm.undo());
        assert!(r.vm.transform_menu().transit.contains(&code));
    }

    #[test]
    fn a_street_with_no_motor_piece_has_no_pedestrian_choices_and_the_menu_is_empty_when_nothing_is_offered() {
        use crate::shared::catalogue::kind_index;
        let r = rig(0);
        assert!(r.vm.apply(|e| e.make_pedestrian(kind_index("sidewalk").unwrap())));
        let menu = r.vm.transform_menu();
        assert!(menu.pedestrian.is_empty());
        let fits: Vec<&str> = r.vm.view_now().measures.iter().filter(|m| m.can_apply && !m.present).map(|m| m.code).collect();
        assert_eq!(menu.transit, fits, "the menu offers the measures that fit on a pedestrian street too");
        assert_eq!(menu.is_empty(), fits.is_empty());
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

    #[test]
    fn only_selecting_a_piece_of_a_place_never_edited_marks_nothing_for_the_sync() {
        let (r, _) = city_rig();
        let uid = first(&r.vm);
        r.vm.select(Some(uid));
        r.vm.flush();
        r.time.advance(1_000);
        assert_eq!(crate::shared::ports::Storage::recall(&*r.storage, crate::sync::model::SYNC_KEY), None, "looking is not editing");
    }

    // ---- zones ----

    use crate::shared::catalogue::{Side, StreetClass, kind_index};
    use crate::street::model::{Keep, Piece, Street, ZoneEnd, ZoneFrame, zoned};

    const LENGTH: i32 = 40_000;

    fn frame() -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LENGTH, from_end: ZoneEnd::First })
    }

    /// The sample street divided at `starts` (a 40 m street anchored at its first end), on a page of its own.
    fn zoned_rig(street: Street, starts: &[i32]) -> Rig {
        let street = zoned(street, starts, LENGTH);
        let (ports, said, storage, time) = test_ports_with_time();
        let editor = Editor::from_street(&street, &street, 0).with_frame(frame());
        Rig { vm: StreetVm::new(ports, crate::i18n_for(Locale::En), editor, None), said, time, storage }
    }

    fn sample_rig(starts: &[i32]) -> Rig {
        zoned_rig(Street::sample(0, Side::Right), starts)
    }

    #[test]
    fn splitting_announces_what_was_done_and_how_the_pieces_stand_and_selects_the_new_zone() {
        let (ports, said, _, _) = test_ports_with_time();
        let vm = StreetVm::new(ports, crate::i18n_for(Locale::En), Editor::new(0).with_frame(frame()), None);
        assert!(vm.split_zone(None));
        assert_eq!(said.take(), ["Split the street at 20.0 m. Every metre of the street is used."]);
        let v = vm.view_now();
        assert_eq!((v.zones.zones.len(), v.zones.selected), (2, 1));
        vm.i18n().set(Locale::FrCa);
        assert!(vm.split_zone(Some(30_000)));
        assert_eq!(said.take(), ["Division de la rue à 30,0\u{a0}m. Chaque mètre de la rue est utilisé."]);
        assert_eq!(vm.view_now().zones.selected, 2);
    }

    #[test]
    fn a_refused_split_says_why_in_the_page_language_and_changes_nothing() {
        let (ports, said, storage, time) = test_ports_with_time();
        let store = CityStore::sample(ports.storage.clone());
        let edge = store.open().view(0).edges[0].uid;
        let street = store.open().street_editor(edge, 0).unwrap().with_frame(None);
        let binding = CityBinding { store, place: Place::Street(edge) };
        let vm = StreetVm::new(ports, crate::i18n_for(Locale::En), street, Some(binding.clone()));
        assert!(!vm.split_zone(None));
        assert_eq!(said.take(), ["Both ends of this street are cut by the edge of the map, so zones cannot be placed along it."]);
        assert!(vm.view_now().revisions.is_empty());
        vm.i18n().set(Locale::FrCa);
        assert!(!vm.split_zone(None));
        assert_eq!(said.take(), ["Les deux extrémités de cette rue sont coupées par le bord de la carte\u{a0}: on ne peut pas y placer de zones."]);
        time.advance(1_000);
        assert_eq!(binding.store.open().view(0).edited, 0, "nothing was kept");
        assert_eq!(crate::shared::ports::Storage::recall(&*storage, crate::sync::model::SYNC_KEY), None);
        // a zone too short carries the minimum in the page's units
        let r = sample_rig(&[]);
        assert!(!r.vm.split_zone(Some(2_000)));
        assert_eq!(r.said.take(), ["A zone must be at least 5.0 m long."]);
    }

    #[test]
    fn selecting_a_zone_announces_its_name_range_length_and_whether_it_has_its_own_section() {
        let r = sample_rig(&[0, 10_000, 30_000]);
        let uid = first(&r.vm);
        r.vm.select(Some(uid));
        r.said.take();
        r.vm.select_zone(1);
        assert_eq!(r.said.take(), ["Zone 2, 10.0 m to 30.0 m, 20.0 m long, the base section, 2 of 3"]);
        assert_eq!(r.vm.view_now().selected, None, "the selected piece is let go");
        assert!(r.vm.apply(|e| e.nudge_width(first(&r.vm), 100)));
        r.said.take();
        r.vm.select_zone_relative(1);
        assert_eq!(r.said.take(), ["Zone 3, 30.0 m to 40.0 m, 10.0 m long, the base section, 3 of 3"]);
        r.vm.select_zone_edge(false);
        assert_eq!(r.said.take(), ["Zone 1, 0.0 m to 10.0 m, 10.0 m long, the base section, 1 of 3"]);
        r.vm.select_zone(1);
        r.said.take();
        assert!(r.vm.name_zone("Market"));
        r.said.take();
        r.vm.select_zone_edge(true);
        r.vm.select_zone_relative(-1);
        assert_eq!(r.said.take().last().unwrap(), "Market, 10.0 m to 30.0 m, 20.0 m long, its own section, 2 of 3");
        r.vm.i18n().set(Locale::FrCa);
        r.vm.select_zone(2);
        assert_eq!(r.said.take(), ["Zone 3, de 30,0\u{a0}m à 40,0\u{a0}m, 10,0\u{a0}m de long, la section de base, 3 sur 3"]);
    }

    #[test]
    fn choosing_a_zone_beyond_the_ends_does_nothing_and_says_nothing() {
        let r = sample_rig(&[0, 10_000]);
        r.vm.select_zone(5);
        r.vm.select_zone_relative(-1);
        assert!(r.said.take().is_empty());
        assert_eq!(r.vm.view_now().zones.selected, 0);
        r.vm.select_zone_relative(1);
        r.said.take();
        r.vm.select_zone_relative(1);
        assert!(r.said.take().is_empty());
        assert_eq!(r.vm.view_now().zones.selected, 1);
    }

    #[test]
    fn merging_zones_with_different_sections_asks_first_and_the_choice_is_said() {
        let r = sample_rig(&[0, 20_000]);
        // zone 2 gets a section of its own
        r.vm.select_zone(1);
        let uid = first(&r.vm);
        assert!(r.vm.apply(|e| e.nudge_width(uid, 100)));
        r.vm.select_zone(0);
        r.said.take();
        let before = r.vm.view_now().revisions.len();
        assert!(r.vm.begin_merge(true), "it asks which section to keep");
        assert_eq!(r.vm.pending_merge_now(), Some(true));
        assert_eq!(r.vm.view_now().revisions.len(), before);
        assert!(r.said.take().is_empty());
        r.vm.cancel_merge();
        assert_eq!(r.vm.pending_merge_now(), None);
        assert!(r.vm.begin_merge(true));
        assert!(r.vm.finish_merge(Keep::Neighbour));
        assert_eq!(r.vm.pending_merge_now(), None);
        let said = r.said.take();
        assert!(said[0].starts_with("Merge zones, keeping Zone 1. "), "{said:?}");
        assert_eq!(r.vm.view_now().zones.zones.len(), 1);
        assert!(!r.vm.finish_merge(Keep::Selected), "nothing is pending");
    }

    #[test]
    fn a_merge_that_cannot_happen_is_refused_at_once_instead_of_asking_which_section_to_keep() {
        let street = zoned(Street::sample(0, Side::Right), &[0, 20_000], LENGTH);
        let mut other = street.clone();
        other.zones[1].segments = Some(street.segments.iter().map(|g| crate::street::model::Segment { width_mm: g.width_mm + 100, ..g.clone() }).collect());
        let (ports, said, _, _) = test_ports_with_time();
        let editor = Editor::from_street(&other, &other, 0).with_frame(None);
        let vm = StreetVm::new(ports, crate::i18n_for(Locale::En), editor, None);
        assert!(vm.read(|e| e.merge_needs_choice(true)), "the sections differ");
        assert!(!vm.begin_merge(true));
        assert_eq!(vm.pending_merge_now(), None);
        assert_eq!(said.take(), ["Both ends of this street are cut by the edge of the map, so zones cannot be placed along it."]);
        assert!(vm.view_now().revisions.is_empty());
        assert_eq!(vm.view_now().zones.zones.len(), 2);
    }

    #[test]
    fn merging_zones_with_the_same_section_happens_at_once_and_a_missing_neighbour_is_refused_in_words() {
        let r = sample_rig(&[0, 20_000]);
        assert!(r.vm.begin_merge(true));
        assert_eq!(r.vm.pending_merge_now(), None);
        assert_eq!(r.vm.view_now().zones.zones.len(), 1);
        assert_eq!(r.said.take().len(), 1);
        let r = sample_rig(&[0, 20_000]);
        assert!(!r.vm.begin_merge(false));
        assert_eq!(r.said.take(), ["There is no zone on that side to merge with."]);
    }

    #[test]
    fn a_boundary_drag_is_quiet_while_it_goes_and_one_announced_edit_when_it_ends() {
        let r = sample_rig(&[0, 20_000]);
        r.vm.select_zone(1);
        r.said.take();
        r.vm.begin_boundary_drag();
        r.vm.drag_boundary_to(22_000);
        r.vm.drag_boundary_to(25_000);
        assert!(r.said.take().is_empty());
        assert!(r.vm.view_now().revisions.is_empty());
        r.vm.end_boundary_drag(true);
        assert_eq!(r.said.take(), ["Move the start of Zone 2 to 25.0 m. Every metre of the street is used."]);
        assert_eq!(r.vm.view_now().revisions.len(), 1);
        assert_eq!(r.vm.view_now().zones.zones[1].from_mm, 25_000);
        // cancelled: back as it was, nothing said
        r.vm.begin_boundary_drag();
        r.vm.drag_boundary_to(30_000);
        r.vm.end_boundary_drag(false);
        assert_eq!(r.vm.view_now().zones.zones[1].from_mm, 25_000);
        assert!(r.said.take().is_empty());
        // a nudge says why it stopped
        assert!(!r.vm.nudge_boundary(-25_000));
        assert_eq!(r.said.take(), ["A zone must be at least 5.0 m long."]);
        assert!(r.vm.nudge_boundary(1_000));
        assert_eq!(r.said.take().len(), 1);
        r.said.take();
        assert!(r.vm.move_boundary(27_000));
        assert_eq!(r.said.take().len(), 1);
    }

    #[test]
    fn making_the_last_motor_zone_pedestrian_announces_the_class_change_and_a_zone_of_several_does_not() {
        let piece = |id: &str| Piece { kind: kind_index(id).unwrap(), width_mm: 3000, direction: None, variants: Vec::new() };
        let street = Street::imported(StreetClass::Local, Side::Right, &[piece("sidewalk"), piece("travel")]).named("Rue Rachel");
        let r = zoned_rig(street, &[0, 20_000]);
        let sidewalk = kind_index("sidewalk").unwrap();
        r.vm.select_zone(1);
        r.said.take();
        assert!(r.vm.apply(|e| e.make_pedestrian(sidewalk)));
        let said = r.said.take();
        assert!(!said[0].contains("pedestrian street"), "zone 1 still carries traffic: {said:?}");
        r.vm.select_zone(0);
        r.said.take();
        assert!(r.vm.apply(|e| e.make_pedestrian(sidewalk)));
        assert!(r.said.take()[0].ends_with(" Rue Rachel is now a pedestrian street."));
    }

    #[test]
    fn a_zone_edit_is_kept_in_the_city_after_the_debounce() {
        let (r, binding) = city_rig();
        let Place::Street(edge) = binding.place else { unreachable!() };
        assert!(r.vm.split_zone(None));
        r.said.take();
        assert_eq!(binding.store.open().view(0).edited, 0, "not yet");
        r.time.advance(300);
        assert_eq!(binding.store.open().view(0).edited, 1);
        let kept = binding.store.open().street_editor(edge, 0).unwrap();
        assert_eq!(kept.view().zones.zones.len(), 2);
    }

    #[test]
    fn a_sync_reload_keeps_the_selected_zone_and_the_frame() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let Place::Street(edge) = binding.place else { unreachable!() };
        assert!(r.vm.split_zone(None));
        assert!(r.vm.split_zone(None));
        r.vm.select_zone(1);
        r.vm.flush();
        // another device changed a width, keeping the zones
        let mut s = binding.store.open().street_editor(edge, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(binding.store.write_street(edge, s));
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.vm.read(|e| e.zone()), 1);
        assert_eq!(r.vm.read(|e| e.frame()), binding.store.open().street_editor(edge, 0).unwrap().frame());
        assert!(r.vm.read(|e| e.frame()).is_some());
        // the zones are gone from the city: the selection falls back to the first
        let mut s = binding.store.open().street_editor(edge, 0).unwrap().snapshot();
        s.zones.clear();
        s.length_mm = None;
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(binding.store.write_street(edge, s));
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.vm.read(|e| e.zone()), 0);
    }
}
