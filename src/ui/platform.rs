//! The browser's side of the ports a view-model is given.

use std::rc::Rc;

use crate::ui::{live, store};
use crate::vm::ports::{Announcer, Ports, Storage};

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

/// What a view-model gets on a page: the live region and local storage.
pub fn browser_ports() -> Ports {
    Ports { announcer: Rc::new(BrowserAnnouncer), storage: Rc::new(BrowserStorage) }
}
