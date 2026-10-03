//! The junction model, shared between the page's Leptos components and the
//! script that still drives the rest of the page. Both edit it through here, so
//! an edit made by either bumps a version the components watch.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::{Junction, Target};
use crate::junction_view::JView;
use crate::ui::plan_svg::Frame;
use crate::units::Units;

pub struct Shared {
    model: RefCell<Junction>,
    version: ArcRwSignal<u32>,
    units: ArcRwSignal<Units>,
    on_edit: RefCell<Option<js_sys::Function>>,
    /// The view as of a version of the model.
    cached: RefCell<Option<(u32, Rc<JView>)>>,
    /// How the plan was last drawn, for turning a pointer position into a point on it.
    frame: Cell<Option<Frame>>,
}

impl Shared {
    pub fn new(model: Junction) -> Rc<Shared> {
        Rc::new(Shared { model: RefCell::new(model), version: ArcRwSignal::new(0), units: ArcRwSignal::new(Units::default()), on_edit: RefCell::new(None), cached: RefCell::new(None), frame: Cell::new(None) })
    }

    /// The model's view, built once for each edit however often it is asked for.
    pub fn view(&self) -> Rc<JView> {
        let version = self.version.get_untracked();
        if let Some((at, view)) = &*self.cached.borrow() {
            if *at == version {
                return view.clone();
            }
        }
        let view = Rc::new(self.model.borrow().view());
        *self.cached.borrow_mut() = Some((version, view.clone()));
        view
    }

    pub fn frame(&self) -> Option<Frame> {
        self.frame.get()
    }

    pub fn set_frame(&self, frame: Frame) {
        self.frame.set(Some(frame));
    }

    pub fn read<R>(&self, f: impl FnOnce(&Junction) -> R) -> R {
        f(&self.model.borrow())
    }

    /// Edits the model, and tells whatever watches `version`.
    pub fn edit<R>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        let r = f(&mut self.model.borrow_mut());
        self.version.update(|v| *v += 1);
        r
    }

    /// An edit made from a component of the page: the script is told afterwards,
    /// with whether the model took it, so it can draw what it draws and announce it.
    pub fn edit_in_page<R: Into<bool> + Copy>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        let r = self.edit(f);
        self.tell("edit", r.into());
        r
    }

    /// A selection made from a component of the page.
    pub fn select_in_page(&self, target: Target) {
        self.edit(|j| j.select(target));
        self.tell("select", true);
    }

    /// A change made from a component that the script need only redraw for, as
    /// each step of a drag is.
    pub fn quiet_in_page<R>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        let r = self.edit(f);
        self.tell("refresh", true);
        r
    }

    /// The end of a gesture made from a component: kept when `commit`, and the
    /// script announces the edit if it changed anything, or else the selection.
    pub fn end_gesture_in_page(&self, commit: bool) {
        if commit {
            let changed = self.edit(|j| j.end_gesture());
            self.tell(if changed { "edit" } else { "select" }, changed);
        } else {
            self.edit(|j| j.cancel_gesture());
            self.tell("refresh", true);
        }
    }

    fn tell(&self, what: &str, ok: bool) {
        if let Some(on_edit) = &*self.on_edit.borrow() {
            let _ = on_edit.call2(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_bool(ok), &wasm_bindgen::JsValue::from_str(what));
        }
    }

    /// Says what to call, with whether the model took it and `"edit"`,
    /// `"select"` or `"refresh"`, after a change made from a component.
    pub fn set_on_edit(&self, f: js_sys::Function) {
        *self.on_edit.borrow_mut() = Some(f);
    }

    /// The units the page shows lengths in; not part of the model.
    pub fn units(&self) -> RwSignal<Units> {
        self.units.clone().into()
    }

    pub fn set_units(&self, units: Units) {
        self.units.set(units);
    }

    /// Changes whenever the model is edited, by either side.
    pub fn version(&self) -> RwSignal<u32> {
        self.version.clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
