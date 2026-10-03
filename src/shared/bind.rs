//! How a view holds a view-model: a handle that is `Copy`, so any closure a
//! view builds can read the view-model's properties or send it commands.

use std::rc::Rc;

use leptos::prelude::*;

pub struct Bound<T: 'static>(StoredValue<Rc<T>, LocalStorage>);

impl<T: 'static> Bound<T> {
    pub fn new(vm: Rc<T>) -> Bound<T> {
        Bound(StoredValue::new_local(vm))
    }

    /// Reads a property, or sends a command.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.0.with_value(|vm| f(vm))
    }
}

impl<T: 'static> Clone for Bound<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Bound<T> {}
