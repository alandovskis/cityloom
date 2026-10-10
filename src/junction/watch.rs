//! What a component needs to draw from, and change, the shared junction.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::keys::{Action, Step};
use crate::junction::model::{Junction, Target};
use crate::junction::read_model::{ArmView, JView};
use crate::junction::vm::JunctionVm;
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::{Said, say};
use crate::shared::units::Units;

/// What a component reads of the junction view-model and tells it. The view-model
/// is kept where only this thread can reach it, so the handle is `Copy` and any
/// closure can hold it.
#[derive(Clone, Copy)]
pub struct Watch {
    vm: StoredValue<Rc<JunctionVm>, LocalStorage>,
}

impl Watch {
    pub fn new(vm: Rc<JunctionVm>) -> Watch {
        Watch { vm: StoredValue::new_local(vm) }
    }

    pub fn i18n(&self) -> Rc<I18n> {
        self.vm.with_value(|v| v.i18n().clone())
    }

    /// What the model said, in words in the language of the page, drawn again when the language or
    /// the units change.
    pub fn say(&self, said: &Said) -> String {
        self.vm.with_value(|v| say(v.i18n(), v.units(), said))
    }

    /// A message of the page, drawn again when the language changes.
    pub fn tr(&self, key: &str) -> String {
        self.vm.with_value(|v| v.i18n().tr(key, &Args::new()))
    }

    pub fn version(&self) -> RwSignal<u32> {
        self.vm.with_value(|v| v.version())
    }

    /// The view, drawing again when the junction is edited.
    pub fn view(&self) -> Rc<JView> {
        self.vm.with_value(|v| v.view())
    }

    /// The view as it is now, for an event handler: nothing is watched.
    pub fn view_now(&self) -> Rc<JView> {
        self.vm.with_value(|v| v.view_now())
    }

    /// The units as they are now, for an event handler: nothing is watched.
    pub fn units_now(&self) -> Units {
        self.vm.with_value(|v| v.units_now())
    }

    /// The units, drawing again when they change.
    pub fn units(&self) -> Units {
        self.vm.with_value(|v| v.units())
    }

    /// The view and the units, drawing again when either changes.
    pub fn now(&self) -> (Rc<JView>, Units) {
        (self.view(), self.units())
    }

    /// What `f` makes of one arm of the view, or None when there is no such arm.
    pub fn arm<R>(&self, uid: u32, f: impl FnOnce(&ArmView) -> R) -> Option<R> {
        self.view().arms.iter().find(|a| a.uid == uid).map(f)
    }

    /// Edits the junction as the person does: announced by what it was, or why it was refused.
    pub fn edit(&self, f: impl FnOnce(&mut Junction) -> bool) -> bool {
        self.vm.with_value(|v| v.apply(f))
    }

    pub fn frame(&self) -> Option<crate::junction::frame::Frame> {
        self.vm.with_value(|v| v.frame())
    }

    pub fn set_frame(&self, frame: crate::junction::frame::Frame) {
        self.vm.with_value(|v| v.set_frame(frame));
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet(&self, f: impl FnOnce(&mut Junction) -> bool) -> bool {
        self.vm.with_value(|v| v.quiet(f))
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

    #[cfg(test)]
    pub fn load_sample(&self, sample: usize) {
        self.vm.with_value(|v| v.load_sample(sample));
    }

    /// What a key on the plan asks for.
    pub fn apply(&self, action: Action) {
        let selected = self.view_now().selected.uid;
        let kind = self.view_now().selected.kind;
        match action {
            Action::Select(dir) => self.vm.with_value(|v| v.select_relative(dir)),
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
        self.vm.with_value(|v| v.select(target));
    }
}
