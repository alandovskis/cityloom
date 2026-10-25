//! What a component of the street page needs to draw from, and change, the
//! shared street.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::catalogue::Side;
use crate::shared::i18n::I18n;
use crate::shared::units::Units;
use crate::street::keys::Action;
use crate::street::model::{Editor, SegView, StopKind, Use, View};
use crate::street::row::Row;
use crate::street::vm::{StreetVm, ViewMode};

/// What a component reads of the street view-model and tells it. The view-model
/// is kept where only this thread can reach it, so the handle is `Copy` and any
/// closure can hold it.
#[derive(Clone, Copy)]
pub struct SheetWatch {
    vm: StoredValue<Rc<StreetVm>, LocalStorage>,
}

impl SheetWatch {
    pub fn new(vm: Rc<StreetVm>) -> SheetWatch {
        SheetWatch { vm: StoredValue::new_local(vm) }
    }

    pub fn version(&self) -> RwSignal<u32> {
        self.vm.with_value(|v| v.version())
    }

    /// How the drawing shows the street, drawing again when it changes.
    pub fn mode(&self) -> ViewMode {
        self.vm.with_value(|v| v.mode())
    }

    pub fn set_mode(&self, mode: ViewMode) {
        self.vm.with_value(|v| v.set_mode(mode));
    }

    pub fn toggle_mode(&self) {
        self.vm.with_value(|v| v.toggle_mode());
    }

    /// The view, drawing again when the street is edited.
    pub fn view(&self) -> Rc<View> {
        self.vm.with_value(|v| v.view())
    }

    /// The view as it is now, for an event handler: nothing is watched.
    pub fn view_now(&self) -> Rc<View> {
        self.vm.with_value(|v| v.view_now())
    }

    /// How the street stands against each area it is inside, drawing again when the street is edited.
    pub fn area_checks(&self) -> Vec<crate::restriction::model::Finding> {
        self.vm.with_value(|v| v.area_checks())
    }

    /// The page's words, for a view: ask with `tr` so it is drawn again on a language switch.
    pub fn i18n(&self) -> Rc<I18n> {
        self.vm.with_value(|v| v.i18n().clone())
    }

    pub fn units(&self) -> Units {
        self.vm.with_value(|v| v.units())
    }

    pub fn units_now(&self) -> Units {
        self.vm.with_value(|v| v.units_now())
    }

    pub fn now(&self) -> (Rc<View>, Units) {
        (self.view(), self.units())
    }

    /// What `f` makes of one piece of the view, or None when there is no such piece.
    pub fn segment<R>(&self, uid: u32, f: impl FnOnce(&SegView) -> R) -> Option<R> {
        self.view().segments.iter().find(|s| s.uid == uid).map(f)
    }

    /// Edits the street as the page does, announcing what was done.
    pub fn edit(&self, f: impl FnOnce(&mut Editor) -> bool) -> bool {
        self.vm.with_value(|v| v.apply(f))
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet<R>(&self, f: impl FnOnce(&mut Editor) -> R) -> R {
        self.vm.with_value(|v| v.quiet(f))
    }

    pub fn select(&self, uid: Option<u32>) {
        self.vm.with_value(|v| v.select(uid));
    }

    pub fn finish_gesture(&self, commit: bool) {
        self.vm.with_value(|v| v.end_gesture(commit));
    }

    pub fn select_zone(&self, zone: usize) {
        self.vm.with_value(|v| v.select_zone(zone));
    }

    pub fn select_zone_relative(&self, delta: i32) {
        self.vm.with_value(|v| v.select_zone_relative(delta));
    }

    pub fn select_zone_edge(&self, last: bool) {
        self.vm.with_value(|v| v.select_zone_edge(last));
    }

    pub fn split_zone(&self, at_mm: Option<i32>) -> bool {
        self.vm.with_value(|v| v.split_zone(at_mm))
    }

    pub fn begin_merge(&self, next: bool) -> bool {
        self.vm.with_value(|v| v.begin_merge(next))
    }

    /// The merge waiting for the choice of the section to keep, drawing again when it changes.
    pub fn pending_merge(&self) -> Option<bool> {
        self.vm.with_value(|v| v.pending_merge())
    }

    pub fn finish_merge(&self, keep: crate::street::model::Keep) -> bool {
        self.vm.with_value(|v| v.finish_merge(keep))
    }

    pub fn cancel_merge(&self) {
        self.vm.with_value(|v| v.cancel_merge());
    }

    pub fn nudge_boundary(&self, delta_mm: i32) -> bool {
        self.vm.with_value(|v| v.nudge_boundary(delta_mm))
    }

    pub fn move_boundary(&self, to_mm: i32) -> bool {
        self.vm.with_value(|v| v.move_boundary(to_mm))
    }

    pub fn begin_boundary_drag(&self) -> bool {
        self.vm.with_value(|v| v.begin_boundary_drag())
    }

    pub fn drag_boundary_to(&self, to_mm: i32) {
        self.vm.with_value(|v| v.drag_boundary_to(to_mm));
    }

    pub fn end_boundary_drag(&self, commit: bool) {
        self.vm.with_value(|v| v.end_boundary_drag(commit));
    }

    pub fn select_cell<R: Row>(&self, side: Side, cell: usize) {
        self.vm.with_value(|v| v.select_cell::<R>(side, cell));
    }

    pub fn select_cell_relative<R: Row>(&self, side: Side, delta: i32) {
        self.vm.with_value(|v| v.select_cell_relative::<R>(side, delta));
    }

    pub fn select_cell_edge<R: Row>(&self, side: Side, last: bool) {
        self.vm.with_value(|v| v.select_cell_edge::<R>(side, last));
    }

    pub fn nudge_cell<R: Row>(&self, delta_mm: i32) -> bool {
        self.vm.with_value(|v| v.nudge_cell::<R>(delta_mm))
    }

    pub fn begin_cell_drag<R: Row>(&self) -> bool {
        self.vm.with_value(|v| v.begin_cell_drag::<R>())
    }

    pub fn drag_cell_to<R: Row>(&self, from_mm: i32) {
        self.vm.with_value(|v| v.drag_cell_to::<R>(from_mm));
    }

    pub fn end_cell_drag(&self, keep: bool) {
        self.vm.with_value(|v| v.end_cell_drag(keep));
    }

    pub fn add_curb(&self, use_: Use, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.add_curb(use_, length_mm))
    }

    pub fn set_curb_use(&self, use_: Use) -> bool {
        self.vm.with_value(|v| v.set_curb_use(use_))
    }

    pub fn resize_curb(&self, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.resize_curb(length_mm))
    }

    pub fn remove_curb(&self) -> bool {
        self.vm.with_value(|v| v.remove_curb())
    }

    pub fn default_stop_kind(&self) -> StopKind {
        self.vm.with_value(|v| v.default_stop_kind())
    }

    pub fn add_stop(&self, kind: StopKind, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.add_stop(kind, length_mm))
    }

    pub fn set_stop_kind(&self, kind: StopKind) -> bool {
        self.vm.with_value(|v| v.set_stop_kind(kind))
    }

    pub fn set_stop_shelter(&self, on: bool) -> bool {
        self.vm.with_value(|v| v.set_stop_shelter(on))
    }

    pub fn name_stop(&self, name: &str) -> bool {
        self.vm.with_value(|v| v.name_stop(name))
    }

    pub fn resize_stop(&self, length_mm: i32) -> bool {
        self.vm.with_value(|v| v.resize_stop(length_mm))
    }

    pub fn remove_stop(&self) -> bool {
        self.vm.with_value(|v| v.remove_stop())
    }

    pub fn say_bad_number(&self) {
        self.vm.with_value(|v| v.say_bad_number());
    }

    pub fn name_zone(&self, name: &str) -> bool {
        self.vm.with_value(|v| v.name_zone(name))
    }

    pub fn speed_options(&self) -> Vec<crate::street::vm::SpeedOption> {
        self.vm.with_value(|v| v.speed_options())
    }

    pub fn set_speed(&self, kmh: Option<u16>) -> bool {
        self.vm.with_value(|v| v.set_speed(kmh))
    }

    pub fn reset_zone(&self) -> bool {
        self.vm.with_value(|v| v.reset_zone())
    }

    pub fn use_for_street(&self) -> bool {
        self.vm.with_value(|v| v.use_for_street())
    }

    pub fn undo(&self) -> bool {
        self.vm.with_value(|v| v.undo())
    }

    pub fn redo(&self) -> bool {
        self.vm.with_value(|v| v.redo())
    }

    pub fn reset(&self) -> bool {
        self.vm.with_value(|v| v.reset())
    }

    /// What the Transform menu offers, drawing again when the street is edited.
    pub fn transform_menu(&self) -> crate::street::vm::TransformMenu {
        self.vm.with_value(|v| v.transform_menu())
    }

    pub fn apply_measure(&self, code: &str) -> bool {
        self.vm.with_value(|v| v.apply_measure(code))
    }

    /// What a key on the section asks for, as far as it changes the street.
    /// Going to the width field, and letting go of a drag, are for the page.
    pub fn apply(&self, action: Action) {
        let v = self.view_now();
        let selected = v.selected;
        match action {
            Action::Select(dir) => self.vm.with_value(|v| v.select_relative(dir)),
            Action::Move(dir) => {
                let Some(uid) = selected else { return };
                let Some(i) = v.segments.iter().position(|s| s.uid == uid) else { return };
                let j = i as i32 + dir;
                if j >= 0 && (j as usize) < v.segments.len() {
                    self.edit(|e| e.move_to(uid, j as usize));
                }
            }
            Action::Nudge(mm) => {
                if let Some(uid) = selected {
                    self.edit(|e| e.nudge_width(uid, mm));
                }
            }
            Action::Remove => {
                if let Some(uid) = selected {
                    self.edit(|e| e.remove(uid));
                }
            }
            Action::EditWidth => {}
            Action::Escape => self.select(None),
        }
    }
}
