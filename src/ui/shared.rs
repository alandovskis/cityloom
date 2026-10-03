//! The junction model, shared between the page's Leptos components and the
//! script that still drives the rest of the page. Both edit it through here, so
//! an edit made by either bumps a version the components watch.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::{Junction, Refusal, Target};
use crate::ui::core::Core;
use crate::ui::live;
use crate::vm::junction_text as announce;
use crate::vm::plan_frame::Frame;

pub struct Shared {
    core: Core<Junction>,
    /// How the plan was last drawn, for turning a pointer position into a point on it.
    frame: Cell<Option<Frame>>,
}

impl std::ops::Deref for Shared {
    type Target = Core<Junction>;
    fn deref(&self) -> &Core<Junction> {
        &self.core
    }
}

impl Shared {
    pub fn new(model: Junction) -> Rc<Shared> {
        Rc::new(Shared { core: Core::new(model), frame: Cell::new(None) })
    }

    pub fn frame(&self) -> Option<Frame> {
        self.frame.get()
    }

    pub fn set_frame(&self, frame: Frame) {
        self.frame.set(Some(frame));
    }

    /// An edit made from a component of the page: it is announced by what it
    /// was, and a refusal by its reason.
    pub fn edit_in_page<R: Into<bool> + Copy>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        let r = self.edit(f);
        if r.into() {
            self.say_edit();
        } else {
            self.say_refusal();
        }
        r
    }

    /// A selection made from a component of the page.
    pub fn select_in_page(&self, target: Target) {
        self.edit(|j| j.select(target));
        self.say_selection();
    }

    /// A change made from a component that is not announced, as each step of a
    /// drag is.
    pub fn quiet_in_page<R>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        self.edit(f)
    }

    /// The end of a gesture made from a component: kept when `commit`, and the
    /// edit is announced if it changed anything, or else the selection.
    pub fn end_gesture_in_page(&self, commit: bool) {
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

    /// Moves the selection to the next thing round the junction, and says what.
    pub fn select_relative_in_page(&self, dir: i32) {
        self.edit(|j| j.select_relative(dir));
        self.say_selection();
    }

    pub fn undo_in_page(&self) -> bool {
        let done = self.edit(|j| j.undo());
        if done {
            live::say(announce::UNDONE);
        }
        done
    }

    pub fn redo_in_page(&self) -> bool {
        let done = self.edit(|j| j.redo());
        if done {
            self.say_edit();
        }
        done
    }

    pub fn reset_in_page(&self) -> bool {
        let done = self.edit(|j| j.reset());
        if done {
            live::say(announce::STARTED_OVER);
        }
        done
    }

    pub fn load_sample_in_page(&self, sample: usize) {
        self.edit(|j| j.load_sample(sample));
        live::say(&announce::sample_text(&self.view()));
    }

    fn say_edit(&self) {
        if let Some(text) = announce::edit_text(&self.view()) {
            live::say(&text);
        }
    }

    fn say_refusal(&self) {
        live::say(self.read(|j| j.refusal()).unwrap_or(Refusal::DoesNotFit).message());
    }

    fn say_selection(&self) {
        if let Some(text) = announce::selection_text(&self.view(), self.units().get_untracked()) {
            live::say(&text);
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Units;

    #[test]
    fn an_edit_bumps_the_version_and_a_read_does_not() {
        let shared = Shared::new(Junction::new(0));
        let version = shared.version();
        assert_eq!(version.get_untracked(), 0);
        let arms = shared.read(|j| j.current().arms.len());
        assert_eq!(arms, 4);
        assert_eq!(version.get_untracked(), 0);
        let uid = shared.read(|j| j.current().arms[0].uid);
        assert!(shared.edit(|j| j.set_corner(uid, 7_000)));
        assert_eq!(version.get_untracked(), 1);
        assert_eq!(shared.read(|j| j.arm(uid).unwrap().corner_mm), 7_000);
    }

    #[test]
    fn a_refused_edit_still_bumps_the_version() {
        // The page draws what the model says, so it asks again either way.
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms[0].uid);
        assert!(!shared.edit(|j| j.set_corner(uid, 99_000)));
        assert_eq!(shared.version().get_untracked(), 1);
    }

    #[test]
    fn an_edit_from_the_page_works_without_a_script_to_tell() {
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms[0].uid);
        assert!(shared.edit_in_page(|j| j.set_corner(uid, 7_000)));
    }

    #[test]
    fn the_units_start_in_metres_and_the_page_can_change_them() {
        let shared = Shared::new(Junction::new(0));
        let units = shared.units();
        assert_eq!(units.get_untracked(), Units::Metres);
        shared.set_units(Units::Feet);
        assert_eq!(units.get_untracked(), Units::Feet);
    }

    #[test]
    fn changing_units_is_not_an_edit_of_the_model() {
        let shared = Shared::new(Junction::new(0));
        shared.set_units(Units::Feet);
        assert_eq!(shared.version().get_untracked(), 0);
    }

    #[test]
    fn the_view_is_built_once_per_edit_however_often_it_is_asked_for() {
        let shared = Shared::new(Junction::new(0));
        let a = shared.view();
        assert!(Rc::ptr_eq(&a, &shared.view()));
        let uid = shared.read(|j| j.current().arms[0].uid);
        assert!(shared.edit(|j| j.set_corner(uid, 7_000)));
        let b = shared.view();
        assert!(!Rc::ptr_eq(&a, &b));
        assert!(Rc::ptr_eq(&b, &shared.view()));
        assert_eq!(b.arms.iter().find(|x| x.uid == uid).unwrap().corner_mm, 7_000);
    }

    #[test]
    fn changing_units_keeps_the_view() {
        let shared = Shared::new(Junction::new(0));
        let a = shared.view();
        shared.set_units(Units::Feet);
        assert!(Rc::ptr_eq(&a, &shared.view()));
    }

    #[test]
    fn selecting_from_the_page_changes_the_selection_and_bumps_the_version() {
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms[0].uid);
        shared.select_in_page(Target::Arm(uid));
        assert_eq!(shared.read(|j| j.selected), Target::Arm(uid));
        assert_eq!(shared.version().get_untracked(), 1);
    }

    #[test]
    fn a_dragged_gesture_is_one_revision_when_it_ends_and_none_when_it_is_cancelled() {
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid);
        let revisions = |s: &Shared| s.view().revisions.len();
        shared.quiet_in_page(|j| { j.begin_gesture(); true });
        assert!(shared.quiet_in_page(|j| j.drag_arm_to(uid, 3_000.0, -10_000.0)));
        assert!(shared.quiet_in_page(|j| j.drag_arm_to(uid, 5_000.0, -10_000.0)));
        assert_eq!(revisions(&shared), 0, "nothing is recorded until the gesture ends");
        shared.end_gesture_in_page(true);
        assert_eq!(revisions(&shared), 1);

        shared.quiet_in_page(|j| { j.begin_gesture(); true });
        shared.quiet_in_page(|j| j.drag_arm_to(uid, 9_000.0, -10_000.0));
        shared.end_gesture_in_page(false);
        assert_eq!(revisions(&shared), 1);
        assert_eq!(shared.read(|j| j.arm(uid).unwrap().bearing), 25);
    }

    #[test]
    fn a_gesture_that_changes_nothing_records_nothing() {
        let shared = Shared::new(Junction::new(0));
        shared.quiet_in_page(|j| { j.begin_gesture(); true });
        shared.end_gesture_in_page(true);
        assert_eq!(shared.view().revisions.len(), 0);
    }

    #[test]
    fn the_plan_s_frame_is_kept_for_whoever_needs_to_read_a_point_from_it() {
        let shared = Shared::new(Junction::new(0));
        assert_eq!(shared.frame(), None);
        let frame = Frame::fit([-10.0, -10.0, 10.0, 10.0], 1000.0, 800.0);
        shared.set_frame(frame);
        assert_eq!(shared.frame(), Some(frame));
    }

    fn said() -> Vec<String> {
        crate::ui::live::take_said()
    }

    #[test]
    fn an_edit_from_the_page_is_announced_by_its_label_and_a_refusal_by_its_reason() {
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms[0].uid);
        said();
        assert!(shared.edit_in_page(|j| j.set_corner(uid, 7_000)));
        assert_eq!(said(), vec!["Corner after Sample Avenue 2 (north): 7000 mm radius. 4 streets, traffic signal. Every check passes."]);
        assert!(!shared.edit_in_page(|j| j.set_corner(uid, 99_000)));
        assert_eq!(said(), vec!["That change does not fit."]);
    }

    #[test]
    fn a_selection_from_the_page_is_announced_and_clearing_it_says_nothing() {
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms[0].uid);
        said();
        shared.select_in_page(Target::Arm(uid));
        assert_eq!(said(), vec!["Sample Avenue 2 (north), 0 degrees"]);
        shared.select_in_page(Target::None);
        assert!(said().is_empty());
    }

    #[test]
    fn a_quiet_change_says_nothing() {
        let shared = Shared::new(Junction::new(0));
        said();
        shared.quiet_in_page(|j| j.set_control(2));
        assert!(said().is_empty());
    }

    #[test]
    fn the_end_of_a_gesture_announces_the_edit_or_else_the_selection() {
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid);
        shared.select_in_page(Target::Arm(uid));
        said();
        shared.quiet_in_page(|j| { j.begin_gesture(); true });
        shared.quiet_in_page(|j| j.drag_arm_to(uid, 3_000.0, -10_000.0));
        shared.end_gesture_in_page(true);
        assert!(said()[0].starts_with("Sample Avenue 2 bearing: 15°."));
        shared.quiet_in_page(|j| { j.begin_gesture(); true });
        shared.end_gesture_in_page(true);
        assert_eq!(said(), vec!["Sample Avenue 2 (north), 15 degrees"]);
    }

    #[test]
    fn undo_redo_start_over_and_a_sample_are_announced() {
        let shared = Shared::new(Junction::new(0));
        let uid = shared.read(|j| j.current().arms[0].uid);
        shared.edit(|j| j.set_corner(uid, 7_000));
        said();
        assert!(shared.undo_in_page());
        assert_eq!(said(), vec!["Undone."]);
        assert!(shared.redo_in_page());
        assert!(said()[0].starts_with("Corner after"));
        assert!(shared.reset_in_page());
        assert_eq!(said(), vec!["Started over from the junction as it is today."]);
        assert!(!shared.undo_in_page());
        assert!(said().is_empty());
        shared.load_sample_in_page(1);
        assert_eq!(said(), vec!["Street and lane. 3 streets, side streets stop. Every check passes."]);
    }
}
