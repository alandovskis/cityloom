//! The junction page's view-model: the junction being edited, what is said when
//! it changes, and keeping what is made in the city. The views read its view and
//! send it what the person does.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::model::{Junction, Refusal, Target};
use crate::junction::read_model::JView;
use crate::shared::units::Units;
use crate::city::binding::CityBinding;
use crate::shared::core::{Core, Presents};
use crate::junction::text as text;
use crate::shared::keeper::{Keeper, NOT_KEPT};
use crate::junction::frame::Frame;
use crate::shared::ports::Ports;

impl Presents for Junction {
    type View = JView;
    fn present(&self) -> JView {
        self.view()
    }
}

pub struct JunctionVm {
    core: Core<Junction>,
    ports: Ports,
    keeper: Option<Rc<Keeper>>,
    binding: Option<CityBinding>,
    /// How the plan was last drawn, for turning a pointer position into a point on it.
    frame: Cell<Option<Frame>>,
}

impl JunctionVm {
    /// A junction to edit. Bound to a place in the city, what is made is written
    /// back to it; otherwise it is a sandbox.
    pub fn new(ports: Ports, junction: Junction, binding: Option<CityBinding>) -> Rc<JunctionVm> {
        Rc::new_cyclic(|me: &std::rc::Weak<JunctionVm>| {
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
            JunctionVm { core: Core::new(junction), ports, keeper, binding, frame: Cell::new(None) }
        })
    }

    // ---- what the views read ----

    /// The view, which a view that reads it draws again when the junction changes.
    pub fn view(&self) -> Rc<JView> {
        self.core.view()
    }

    /// The same, for a command: nothing is watched.
    pub fn view_now(&self) -> Rc<JView> {
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

    pub fn read<R>(&self, f: impl FnOnce(&Junction) -> R) -> R {
        self.core.read(f)
    }

    pub fn frame(&self) -> Option<Frame> {
        self.frame.get()
    }

    // ---- what the views do ----

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
    }

    pub fn set_frame(&self, frame: Frame) {
        self.frame.set(Some(frame));
    }

    /// Changes the junction, tells whatever watches it, and sees that it is kept.
    pub fn edit<R>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        let r = self.core.edit(f);
        if let Some(k) = &self.keeper {
            k.touch();
        }
        r
    }

    /// An edit the person made: it is announced by what it was, and a refusal by
    /// its reason.
    pub fn apply<R: Into<bool> + Copy>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        let r = self.edit(f);
        if r.into() {
            self.say_edit();
        } else {
            self.say_refusal();
        }
        r
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet<R>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        self.edit(f)
    }

    /// A selection the person made, announced by what is selected.
    pub fn select(&self, target: Target) {
        self.edit(|j| j.select(target));
        self.say_selection();
    }

    /// Moves the selection to the next thing round the junction, and says what.
    pub fn select_relative(&self, dir: i32) {
        self.edit(|j| j.select_relative(dir));
        self.say_selection();
    }

    /// The end of a gesture: kept when `commit`, and announced as an edit if it
    /// changed anything, or else by what is selected.
    pub fn end_gesture(&self, commit: bool) {
        if commit {
            if self.edit(|j| j.end_gesture()) {
                self.say_edit();
            } else {
                self.say_selection();
            }
        } else {
            self.edit(|j| j.cancel_gesture());
        }
    }

    pub fn undo(&self) -> bool {
        let done = self.edit(|j| j.undo());
        if done {
            self.ports.announcer.say(text::UNDONE);
        }
        done
    }

    pub fn redo(&self) -> bool {
        let done = self.edit(|j| j.redo());
        if done {
            self.say_edit();
        }
        done
    }

    pub fn reset(&self) -> bool {
        let done = self.edit(|j| j.reset());
        if done {
            self.ports.announcer.say(text::STARTED_OVER);
        }
        done
    }

    pub fn load_sample(&self, sample: usize) {
        self.edit(|j| j.load_sample(sample));
        self.ports.announcer.say(&text::sample_text(&self.view_now()));
    }

    /// The page is being left: what is waiting to be kept is kept now.
    pub fn flush(&self) {
        if let Some(k) = &self.keeper {
            k.flush();
        }
    }

    /// Writes the junction as it now is back to its place in the city.
    fn keep_now(&self) -> bool {
        let Some(binding) = &self.binding else { return true };
        binding.keep_junction(self.core.read(|j| j.snapshot()))
    }

    fn say_edit(&self) {
        if let Some(t) = text::edit_text(&self.view_now()) {
            self.ports.announcer.say(&t);
        }
    }

    fn say_refusal(&self) {
        let why = self.core.read(|j| j.refusal()).unwrap_or(Refusal::DoesNotFit);
        self.ports.announcer.say(why.message());
    }

    fn say_selection(&self) {
        if let Some(t) = text::selection_text(&self.view_now(), self.units_now()) {
            self.ports.announcer.say(&t);
        }
    }
}

impl crate::vm::shell::Target for JunctionVm {
    fn set_units(&self, units: Units) {
        JunctionVm::set_units(self, units);
    }

    fn apply_region(&self, region: usize) -> bool {
        self.edit(|j| j.set_region(region))
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
    use crate::shared::ports::{ManualScheduler, MemoryStorage, RecordingAnnouncer, Storage, test_ports_with_time};

    struct Rig {
        vm: Rc<JunctionVm>,
        said: Rc<RecordingAnnouncer>,
        time: Rc<ManualScheduler>,
        storage: Rc<MemoryStorage>,
    }

    fn rig(sample: usize) -> Rig {
        let (ports, said, storage, time) = test_ports_with_time();
        Rig { vm: JunctionVm::new(ports, Junction::new(sample), None), said, time, storage }
    }

    fn city_rig() -> (Rig, CityBinding) {
        let (ports, said, storage, time) = test_ports_with_time();
        let store = CityStore::new(ports.storage.clone());
        let node = store.open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let junction = store.open().junction_editor(node, 0).unwrap();
        let binding = CityBinding { store, place: Place::Junction(node) };
        (Rig { vm: JunctionVm::new(ports, junction, Some(binding.clone())), said, time, storage }, binding)
    }

    fn arm(vm: &JunctionVm, bearing: i32) -> u32 {
        vm.read(|j| j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid)
    }

    #[test]
    fn an_edit_changes_the_view_and_the_version_and_a_view_is_built_once_per_edit() {
        let r = rig(0);
        let (a, version) = (r.vm.view_now(), r.vm.version());
        assert!(Rc::ptr_eq(&a, &r.vm.view_now()));
        assert_eq!(version.get_untracked(), 0);
        let n = arm(&r.vm, 0);
        assert!(r.vm.edit(|j| j.set_corner(n, 7_000)));
        assert_eq!(version.get_untracked(), 1);
        let b = r.vm.view_now();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(b.arms.iter().find(|x| x.uid == n).unwrap().corner_mm, 7_000);
    }

    #[test]
    fn the_units_start_in_metres_and_changing_them_is_not_an_edit() {
        let r = rig(0);
        assert_eq!(r.vm.units_now(), Units::Metres);
        let a = r.vm.view_now();
        r.vm.set_units(Units::Feet);
        assert_eq!(r.vm.units_now(), Units::Feet);
        assert_eq!(r.vm.version().get_untracked(), 0);
        assert!(Rc::ptr_eq(&a, &r.vm.view_now()));
    }

    #[test]
    fn the_plan_s_frame_is_kept_for_whoever_needs_to_read_a_point_from_it() {
        let r = rig(0);
        assert_eq!(r.vm.frame(), None);
        let f = Frame::fit([-10.0, -10.0, 10.0, 10.0], 1000.0, 800.0);
        r.vm.set_frame(f);
        assert_eq!(r.vm.frame(), Some(f));
    }

    #[test]
    fn an_edit_is_announced_by_its_label_and_a_refusal_by_its_reason() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        assert!(r.vm.apply(|j| j.set_corner(n, 7_000)));
        assert_eq!(r.said.take(), vec!["Corner after Sample Avenue 2 (north): 7000 mm radius. 4 streets, traffic signal. Every check passes."]);
        assert!(!r.vm.apply(|j| j.set_corner(n, 99_000)));
        assert_eq!(r.said.take(), vec!["That change does not fit."]);
        assert_eq!(r.vm.version().get_untracked(), 2, "a refused edit still asks the views to look again");
    }

    #[test]
    fn a_selection_is_announced_and_clearing_it_says_nothing() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.select(Target::Arm(n));
        assert_eq!(r.said.take(), vec!["Sample Avenue 2 (north), 0 degrees"]);
        assert_eq!(r.vm.read(|j| j.selected), Target::Arm(n));
        r.vm.select(Target::None);
        assert!(r.said.take().is_empty());
        r.vm.select_relative(1);
        assert_eq!(r.said.take().len(), 1);
    }

    #[test]
    fn a_quiet_change_says_nothing() {
        let r = rig(0);
        r.vm.quiet(|j| j.set_control(2));
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn a_gesture_is_one_revision_announced_when_it_ends_and_none_when_cancelled_or_unchanged() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.select(Target::Arm(n));
        r.said.take();
        r.vm.quiet(|j| j.begin_gesture());
        assert!(r.vm.quiet(|j| j.drag_arm_to(n, 3_000.0, -10_000.0)));
        r.vm.quiet(|j| j.drag_arm_to(n, 5_000.0, -10_000.0));
        assert_eq!(r.vm.view_now().revisions.len(), 0);
        r.vm.end_gesture(true);
        assert_eq!(r.vm.view_now().revisions.len(), 1);
        assert!(r.said.take()[0].starts_with("Sample Avenue 2 bearing: 25°."));
        // Nothing changed: the selection is said instead.
        r.vm.quiet(|j| j.begin_gesture());
        r.vm.end_gesture(true);
        assert_eq!(r.said.take(), vec!["Sample Avenue 2 (north-east), 25 degrees"]);
        r.vm.quiet(|j| j.begin_gesture());
        r.vm.quiet(|j| j.drag_arm_to(n, 9_000.0, -10_000.0));
        r.vm.end_gesture(false);
        assert_eq!((r.vm.view_now().revisions.len(), r.vm.read(|j| j.arm(n).unwrap().bearing)), (1, 25));
    }

    #[test]
    fn undo_redo_start_over_and_a_sample_are_announced() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.edit(|j| j.set_corner(n, 7_000));
        assert!(r.vm.undo());
        assert_eq!(r.said.take(), vec!["Undone."]);
        assert!(r.vm.redo());
        assert!(r.said.take()[0].starts_with("Corner after"));
        assert!(r.vm.reset());
        assert_eq!(r.said.take(), vec!["Started over from the junction as it is today."]);
        assert!(!r.vm.undo());
        assert!(r.said.take().is_empty());
        r.vm.load_sample(1);
        assert_eq!(r.said.take(), vec!["Street and lane. 3 streets, side streets stop. Every check passes."]);
    }

    #[test]
    fn a_sandbox_keeps_nothing_in_storage() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.edit(|j| j.set_corner(n, 7_000));
        r.time.advance(1_000);
        assert_eq!(r.time.pending(), 0);
        assert_eq!(r.storage.recall(crate::city::store::CITY_KEY), None);
    }

    #[test]
    fn what_is_made_of_a_city_junction_is_kept_in_the_city_a_moment_after_the_last_change() {
        let (r, binding) = city_rig();
        let n = r.vm.view_now().arms[0].uid;
        assert!(r.vm.edit(|j| j.set_corner(n, 7_000)));
        r.time.advance(200);
        assert_eq!(binding.store.open().view(0).edited, 0, "not yet");
        assert!(r.vm.edit(|j| j.set_corner(n, 7_500)));
        r.time.advance(200);
        assert_eq!(binding.store.open().view(0).edited, 0);
        r.time.advance(100);
        assert_eq!(binding.store.open().view(0).edited, 1);
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn leaving_the_page_keeps_what_is_waiting_at_once() {
        let (r, binding) = city_rig();
        let n = r.vm.view_now().arms[0].uid;
        r.vm.edit(|j| j.set_corner(n, 7_000));
        r.vm.flush();
        assert_eq!(binding.store.open().view(0).edited, 1);
    }

    #[test]
    fn a_city_that_cannot_be_written_is_said_once_and_the_page_goes_on() {
        let (r, _) = city_rig();
        r.storage.blocked(true);
        let n = r.vm.view_now().arms[0].uid;
        r.vm.edit(|j| j.set_corner(n, 7_000));
        r.time.advance(300);
        r.vm.edit(|j| j.set_corner(n, 7_500));
        r.time.advance(300);
        assert_eq!(r.said.take(), vec![NOT_KEPT]);
        assert_eq!(r.vm.read(|j| j.arm(n).unwrap().corner_mm), 7_500);
    }
}
