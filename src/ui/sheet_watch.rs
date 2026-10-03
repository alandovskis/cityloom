//! What a component of the street page needs to draw from, and change, the
//! shared street.

use std::rc::Rc;

use leptos::prelude::*;

use crate::model::{Editor, SegView, View};
use crate::ui::sheet::SharedSheet;
use crate::ui::street_keys::Action;
use crate::units::Units;

/// The signals a component watches, and the street itself, kept where only
/// this thread can reach it. It is `Copy`, so any closure can hold it.
#[derive(Clone, Copy)]
pub struct SheetWatch {
    version: RwSignal<u32>,
    units: RwSignal<Units>,
    shared: StoredValue<Rc<SharedSheet>, LocalStorage>,
}

impl SheetWatch {
    pub fn new(shared: Rc<SharedSheet>) -> SheetWatch {
        SheetWatch { version: shared.version(), units: shared.units(), shared: StoredValue::new_local(shared) }
    }

    pub fn version(&self) -> RwSignal<u32> {
        self.version
    }

    /// The view, drawing again when the street is edited.
    pub fn view(&self) -> Rc<View> {
        self.version.track();
        self.shared.with_value(|s| s.view())
    }

    /// The view as it is now, for an event handler: nothing is watched.
    pub fn view_now(&self) -> Rc<View> {
        self.shared.with_value(|s| s.view())
    }

    pub fn units(&self) -> Units {
        self.units.get()
    }

    pub fn units_now(&self) -> Units {
        self.units.get_untracked()
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
        self.shared.with_value(|s| s.edit_in_page(f))
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet<R>(&self, f: impl FnOnce(&mut Editor) -> R) -> R {
        self.shared.with_value(|s| s.quiet_in_page(f))
    }

    pub fn select(&self, uid: Option<u32>) {
        self.shared.with_value(|s| s.select_in_page(uid));
    }

    pub fn finish_gesture(&self, commit: bool) {
        self.shared.with_value(|s| s.end_gesture_in_page(commit));
    }

    pub fn undo(&self) -> bool {
        self.shared.with_value(|s| s.undo_in_page())
    }

    pub fn redo(&self) -> bool {
        self.shared.with_value(|s| s.redo_in_page())
    }

    pub fn reset(&self) -> bool {
        self.shared.with_value(|s| s.reset_in_page())
    }

    pub fn apply_measure(&self, code: &str) -> bool {
        self.shared.with_value(|s| s.apply_measure_in_page(code))
    }

    /// What a key on the section asks for, as far as it changes the street.
    /// Going to the width field, and letting go of a drag, are for the page.
    pub fn apply(&self, action: Action) {
        let v = self.view_now();
        let selected = v.selected;
        match action {
            Action::Select(dir) => self.shared.with_value(|s| s.select_relative_in_page(dir)),
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
