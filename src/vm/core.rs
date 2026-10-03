//! What every view-model shares: the model, a version that changes on every
//! edit, the units lengths are shown in, and the model's view built once per
//! edit. Properties read through it, so a view that reads one draws again when
//! it changes.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::units::Units;

/// A model a view-model presents a view of.
pub trait Presents {
    type View;
    fn present(&self) -> Self::View;
}

pub struct Core<M: Presents> {
    model: RefCell<M>,
    version: ArcRwSignal<u32>,
    units: ArcRwSignal<Units>,
    cached: RefCell<Option<(u32, Rc<M::View>)>>,
}

impl<M: Presents> Core<M> {
    pub fn new(model: M) -> Core<M> {
        Core { model: RefCell::new(model), version: ArcRwSignal::new(0), units: ArcRwSignal::new(Units::default()), cached: RefCell::new(None) }
    }

    /// The model's view, built once for each edit however often it is asked for.
    /// Reading it in a view makes the view draw again when the model is edited.
    pub fn view(&self) -> Rc<M::View> {
        self.version.track();
        self.view_now()
    }

    /// The same, for a command: nothing is watched.
    pub fn view_now(&self) -> Rc<M::View> {
        let version = self.version.get_untracked();
        if let Some((at, view)) = &*self.cached.borrow() {
            if *at == version {
                return view.clone();
            }
        }
        let view = Rc::new(self.model.borrow().present());
        *self.cached.borrow_mut() = Some((version, view.clone()));
        view
    }

    pub fn read<R>(&self, f: impl FnOnce(&M) -> R) -> R {
        f(&self.model.borrow())
    }

    /// Edits the model, and tells whatever watches it.
    pub fn edit<R>(&self, f: impl FnOnce(&mut M) -> R) -> R {
        let r = f(&mut self.model.borrow_mut());
        self.version.update(|v| *v += 1);
        r
    }

    /// The units, read so that a view that reads them draws again when they change.
    pub fn units(&self) -> Units {
        self.units.get()
    }

    pub fn units_now(&self) -> Units {
        self.units.get_untracked()
    }

    pub fn set_units(&self, units: Units) {
        self.units.set(units);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter(i32);

    impl Presents for Counter {
        type View = i32;
        fn present(&self) -> i32 {
            self.0
        }
    }

    #[test]
    fn the_view_is_built_once_per_edit_and_rebuilt_after_one() {
        let core = Core::new(Counter(1));
        let a = core.view_now();
        assert!(Rc::ptr_eq(&a, &core.view_now()));
        core.edit(|c| c.0 = 2);
        let b = core.view_now();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!((*a, *b), (1, 2));
    }

    #[test]
    fn the_units_start_in_metres_and_changing_them_does_not_rebuild_the_view() {
        let core = Core::new(Counter(1));
        assert_eq!(core.units_now(), Units::Metres);
        let a = core.view_now();
        core.set_units(Units::Feet);
        assert_eq!(core.units_now(), Units::Feet);
        assert!(Rc::ptr_eq(&a, &core.view_now()));
    }

    #[test]
    fn a_read_gives_what_is_in_the_model() {
        let core = Core::new(Counter(7));
        assert_eq!(core.read(|c| c.0), 7);
    }
}
