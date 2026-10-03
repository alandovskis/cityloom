//! What a component of the street page needs to draw from, and change, the
//! shared street.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::units::Units;
use crate::street::keys::Action;
use crate::street::model::{Editor, SegView, View};
use crate::street::vm::StreetVm;

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

    /// The view, drawing again when the street is edited.
    pub fn view(&self) -> Rc<View> {
        self.vm.with_value(|v| v.view())
    }

    /// The view as it is now, for an event handler: nothing is watched.
    pub fn view_now(&self) -> Rc<View> {
        self.vm.with_value(|v| v.view_now())
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

    pub fn undo(&self) -> bool {
        self.vm.with_value(|v| v.undo())
    }

    pub fn redo(&self) -> bool {
        self.vm.with_value(|v| v.redo())
    }

    pub fn reset(&self) -> bool {
        self.vm.with_value(|v| v.reset())
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
