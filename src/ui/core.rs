//! What every page's shared model has in common: the model, a version that
//! changes on every edit, the units lengths are shown in, the view built once per
//! edit, and a callback for the script. A junction and a street each wrap it.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::Junction;
use crate::junction_view::JView;
use crate::model::{Editor, View};
use crate::units::Units;

/// A model the page edits and draws from a view of.
pub trait Model {
    type View;
    fn build_view(&self) -> Self::View;
}

impl Model for Junction {
    type View = JView;
    fn build_view(&self) -> JView {
        self.view()
    }
}

impl Model for Editor {
    type View = View;
    fn build_view(&self) -> View {
        self.view()
    }
}

pub struct Core<M: Model> {
    model: RefCell<M>,
    version: ArcRwSignal<u32>,
    units: ArcRwSignal<Units>,
    on_change: RefCell<Option<js_sys::Function>>,
    /// The view as of a version of the model.
    cached: RefCell<Option<(u32, Rc<M::View>)>>,
}

impl<M: Model> Core<M> {
    pub fn new(model: M) -> Core<M> {
        Core { model: RefCell::new(model), version: ArcRwSignal::new(0), units: ArcRwSignal::new(Units::default()), on_change: RefCell::new(None), cached: RefCell::new(None) }
    }

    /// The model's view, built once for each edit however often it is asked for.
    pub fn view(&self) -> Rc<M::View> {
        let version = self.version.get_untracked();
        if let Some((at, view)) = &*self.cached.borrow() {
            if *at == version {
                return view.clone();
            }
        }
        let view = Rc::new(self.model.borrow().build_view());
        *self.cached.borrow_mut() = Some((version, view.clone()));
        view
    }

    pub fn read<R>(&self, f: impl FnOnce(&M) -> R) -> R {
        f(&self.model.borrow())
    }

    /// Edits the model, and tells whatever watches `version` and the script.
    pub fn edit<R>(&self, f: impl FnOnce(&mut M) -> R) -> R {
        let r = f(&mut self.model.borrow_mut());
        self.version.update(|v| *v += 1);
        if let Some(on_change) = &*self.on_change.borrow() {
            let _ = on_change.call0(&wasm_bindgen::JsValue::NULL);
        }
        r
    }

    /// Says what to call, with no arguments, after any change to the model, by
    /// either side: the script keeps what the city holds.
    pub fn set_on_change(&self, f: js_sys::Function) {
        *self.on_change.borrow_mut() = Some(f);
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
    fn a_street_s_core_counts_edits_and_builds_its_view_once_each() {
        let core = Core::new(Editor::new(0));
        let version = core.version();
        let a = core.view();
        assert!(Rc::ptr_eq(&a, &core.view()));
        assert_eq!(version.get_untracked(), 0);
        let uid = core.read(|e| e.view().segments[0].uid);
        assert!(core.edit(|e| e.nudge_width(uid, 100)));
        assert_eq!(version.get_untracked(), 1);
        let b = core.view();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(b.revisions.len(), 1);
    }

    #[test]
    fn the_units_of_a_street_start_in_metres_and_do_not_rebuild_the_view() {
        let core = Core::new(Editor::new(0));
        assert_eq!(core.units().get_untracked(), Units::Metres);
        let a = core.view();
        core.set_units(Units::Feet);
        assert_eq!(core.units().get_untracked(), Units::Feet);
        assert!(Rc::ptr_eq(&a, &core.view()));
    }
}
