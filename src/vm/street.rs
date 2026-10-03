//! The street page's view-model: the street being edited, what is said when it
//! changes, and keeping what is made in the city. The views read its view and
//! send it what the person does.

use std::rc::Rc;

use leptos::prelude::*;

use crate::model::{Editor, View};
use crate::shared::units::Units;
use crate::vm::binding::CityBinding;
use crate::shared::core::{Core, Presents};
use crate::shared::keeper::Keeper;
use crate::shared::ports::Ports;
use crate::vm::street_text as text;

/// Said once when what a page makes cannot be kept.
pub const NOT_KEPT: &str = crate::vm::junction::NOT_KEPT;

impl Presents for Editor {
    type View = View;
    fn present(&self) -> View {
        self.view()
    }
}

pub struct StreetVm {
    core: Core<Editor>,
    ports: Ports,
    keeper: Option<Rc<Keeper>>,
    binding: Option<CityBinding>,
}

impl StreetVm {
    /// A street to edit. Bound to a place in the city, what is made is written
    /// back to it; otherwise it is a sandbox.
    pub fn new(ports: Ports, street: Editor, binding: Option<CityBinding>) -> Rc<StreetVm> {
        Rc::new_cyclic(|me: &std::rc::Weak<StreetVm>| {
            let keeper = binding.is_some().then(|| {
                let (keep, told) = (me.clone(), me.clone());
                Keeper::new(
                    ports.scheduler.clone(),
                    move || keep.upgrade().is_some_and(|vm| vm.keep_now()),
                    move || {
                        if let Some(vm) = told.upgrade() {
                            vm.ports.announcer.say(NOT_KEPT);
                        }
                    },
                )
            });
            StreetVm { core: Core::new(street), ports, keeper, binding }
        })
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
        let done = self.edit(f);
        if done {
            self.say_edit();
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
            if self.edit(|e| e.end_gesture()) {
                self.say_edit();
            }
        } else {
            self.edit(|e| e.cancel_gesture());
        }
    }

    pub fn undo(&self) -> bool {
        let done = self.edit(|e| e.undo());
        if done {
            self.ports.announcer.say(&text::undone_text(&self.view_now(), self.units_now()));
        }
        done
    }

    pub fn redo(&self) -> bool {
        let done = self.edit(|e| e.redo());
        if done {
            self.ports.announcer.say(&text::redone_text(&self.view_now(), self.units_now()));
        }
        done
    }

    pub fn reset(&self) -> bool {
        let done = self.edit(|e| e.reset());
        if done {
            self.ports.announcer.say(text::STARTED_OVER);
        }
        done
    }

    /// Arranges the roadway as an Atlas measure, or says it does not suit.
    pub fn apply_measure(&self, code: &str) -> bool {
        let done = self.edit(|e| e.apply_measure(code));
        if done {
            self.say_edit();
        } else {
            self.ports.announcer.say(text::MEASURE_REFUSED);
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

    fn say_edit(&self) {
        if let Some(t) = text::edit_text(&self.view_now(), self.units_now()) {
            self.ports.announcer.say(&t);
        }
    }

    fn say_selection(&self) {
        if let Some(t) = text::selection_text(&self.view_now(), self.units_now()) {
            self.ports.announcer.say(&t);
        }
    }
}

impl crate::vm::shell::Target for StreetVm {
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
    use crate::vm::binding::Place;
    use crate::vm::city_store::CityStore;
    use crate::shared::ports::{ManualScheduler, MemoryStorage, RecordingAnnouncer, test_ports_with_time};

    struct Rig {
        vm: Rc<StreetVm>,
        said: Rc<RecordingAnnouncer>,
        time: Rc<ManualScheduler>,
        storage: Rc<MemoryStorage>,
    }

    fn rig(sample: usize) -> Rig {
        let (ports, said, storage, time) = test_ports_with_time();
        Rig { vm: StreetVm::new(ports, Editor::new(sample), None), said, time, storage }
    }

    fn city_rig() -> (Rig, CityBinding) {
        let (ports, said, storage, time) = test_ports_with_time();
        let store = CityStore::new(ports.storage.clone());
        let edge = store.open().view(0).edges[0].uid;
        let street = store.open().street_editor(edge, 0).unwrap();
        let binding = CityBinding { store, place: Place::Street(edge) };
        (Rig { vm: StreetVm::new(ports, street, Some(binding.clone())), said, time, storage }, binding)
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
        assert_eq!(r.storage.recall(crate::vm::city_store::CITY_KEY), None);
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
        assert_eq!(r.said.take(), vec![NOT_KEPT]);
        assert_eq!(r.vm.view_now().revisions.len(), 2);
    }
}
