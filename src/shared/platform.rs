//! The browser's side of the ports a view-model is given.

use std::rc::Rc;

use crate::shared::{live, store};
use crate::shared::ports::{Announcer, Ports, Scheduler, Storage};

struct BrowserAnnouncer;

impl Announcer for BrowserAnnouncer {
    fn say(&self, text: &str) {
        live::say(text);
    }
}

struct BrowserStorage;

impl Storage for BrowserStorage {
    fn recall(&self, key: &str) -> Option<String> {
        store::recall(key)
    }

    fn remember(&self, key: &str, value: &str) -> bool {
        store::remember(key, value)
    }
}

/// Timers, kept by id so that they can be called off.
#[derive(Default)]
struct BrowserScheduler {
    next: std::cell::Cell<u32>,
    #[cfg(target_arch = "wasm32")]
    handles: std::rc::Rc<std::cell::RefCell<std::collections::HashMap<u32, leptos::leptos_dom::helpers::TimeoutHandle>>>,
}

impl Scheduler for BrowserScheduler {
    #[cfg(target_arch = "wasm32")]
    fn after(&self, ms: u32, f: Box<dyn FnOnce()>) -> u32 {
        let id = self.next.get() + 1;
        self.next.set(id);
        let handles = self.handles.clone();
        let done = leptos::prelude::set_timeout_with_handle(
            move || {
                handles.borrow_mut().remove(&id);
                f();
            },
            std::time::Duration::from_millis(ms as u64),
        );
        if let Ok(h) = done {
            self.handles.borrow_mut().insert(id, h);
        }
        id
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn after(&self, _ms: u32, _f: Box<dyn FnOnce()>) -> u32 {
        let id = self.next.get() + 1;
        self.next.set(id);
        id
    }

    #[cfg(target_arch = "wasm32")]
    fn cancel(&self, id: u32) {
        if let Some(h) = self.handles.borrow_mut().remove(&id) {
            h.clear();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn cancel(&self, _id: u32) {}
}

/// What a view-model gets on a page: the live region and local storage.
pub fn browser_ports() -> Ports {
    Ports { announcer: Rc::new(BrowserAnnouncer), storage: Rc::new(BrowserStorage), scheduler: Rc::new(BrowserScheduler::default()) }
}
