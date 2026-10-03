//! What a component needs to draw from, and change, the shared junction.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::{Junction, Target};
use crate::junction_view::{ArmView, JView};
use crate::ui::keys::{Action, Step};
use crate::ui::shared::Shared;
use crate::units::Units;

/// The signals a component watches, and the junction itself, which is kept
/// where only this thread can reach it. It is `Copy`, so any closure can hold it.
#[derive(Clone, Copy)]
pub struct Watch {
    version: RwSignal<u32>,
    units: RwSignal<Units>,
    shared: StoredValue<Rc<Shared>, LocalStorage>,
}

impl Watch {
    pub fn new(shared: Rc<Shared>) -> Watch {
        Watch { version: shared.version(), units: shared.units(), shared: StoredValue::new_local(shared) }
    }

    pub fn version(&self) -> RwSignal<u32> {
        self.version
    }

    /// The view, drawing again when the junction is edited.
    pub fn view(&self) -> Rc<JView> {
        self.version.track();
        self.shared.with_value(|s| s.view())
    }

    /// The view as it is now, for an event handler: nothing is watched.
    pub fn view_now(&self) -> Rc<JView> {
        self.shared.with_value(|s| s.view())
    }

    /// The units as they are now, for an event handler: nothing is watched.
    pub fn units_now(&self) -> Units {
        self.units.get_untracked()
    }

    /// The units, drawing again when they change.
    pub fn units(&self) -> Units {
        self.units.get()
    }

    /// The view and the units, drawing again when either changes.
    pub fn now(&self) -> (Rc<JView>, Units) {
        (self.view(), self.units())
    }

    /// What `f` makes of one arm of the view, or None when there is no such arm.
    pub fn arm<R>(&self, uid: u32, f: impl FnOnce(&ArmView) -> R) -> Option<R> {
        self.view().arms.iter().find(|a| a.uid == uid).map(f)
    }

    /// Edits the junction as the page does; the script draws and announces it.
    pub fn edit(&self, f: impl FnOnce(&mut Junction) -> bool) -> bool {
        self.shared.with_value(|s| s.edit_in_page(f))
    }

    pub fn frame(&self) -> Option<crate::ui::plan_svg::Frame> {
        self.shared.with_value(|s| s.frame())
    }

    pub fn set_frame(&self, frame: crate::ui::plan_svg::Frame) {
        self.shared.with_value(|s| s.set_frame(frame));
    }

    /// A change the script need only redraw for.
    pub fn quiet(&self, f: impl FnOnce(&mut Junction) -> bool) -> bool {
        self.shared.with_value(|s| s.quiet_in_page(f))
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

    pub fn load_sample(&self, sample: usize) {
        self.shared.with_value(|s| s.load_sample_in_page(sample));
    }

    /// What a key on the plan asks for.
    pub fn apply(&self, action: Action) {
        let selected = self.view_now().selected.uid;
        let kind = self.view_now().selected.kind;
        match action {
            Action::Select(dir) => self.shared.with_value(|s| s.select_relative_in_page(dir)),
            Action::Deselect => self.select(Target::None),
            Action::Step(step, dir) => {
                if kind.is_some() {
                    self.edit(|j| match step {
                        Step::Bearing => j.step_bearing(selected, dir),
                        Step::Corner => j.step_corner(selected, dir),
                        Step::Setback => j.step_setback(selected, dir),
                        Step::Cycle => j.step_cycle(dir),
                    });
                }
            }
            Action::Remove => {
                match kind {
                    Some("bus") => self.edit(|j| j.set_bus(None)),
                    Some("cycle") => self.edit(|j| j.set_cycle_track(false)),
                    Some("crossing") => self.edit(|j| j.set_crossing(selected, false)),
                    Some("arm") => self.edit(|j| j.remove_arm(selected)),
                    _ => false,
                };
            }
        }
    }

    pub fn select(&self, target: Target) {
        self.shared.with_value(|s| s.select_in_page(target));
    }
}
