//! A `Storage` for one project: the keys that hold a project's own data get the project as a suffix, so two
//! projects never share them; every other key (the language, the sync document, ...) is the person's and is shared.

use std::rc::Rc;

use crate::shared::ports::Storage;

/// The keys that hold one project's data. Each is stored as `<key>@<project>`.
pub const SCOPED_KEYS: [&str; 5] = ["cityloom-edits", "cityloom-edits-damaged", "cityloom-restrictions", "cityloom-restrictions-damaged", "cityloom-area"];

pub struct ScopedStorage {
    inner: Rc<dyn Storage>,
    project: String,
}

impl ScopedStorage {
    pub fn new(inner: Rc<dyn Storage>, project: &str) -> ScopedStorage {
        ScopedStorage { inner, project: project.to_string() }
    }

    fn name(&self, key: &str) -> String {
        if SCOPED_KEYS.contains(&key) { format!("{key}@{}", self.project) } else { key.to_string() }
    }
}

impl Storage for ScopedStorage {
    fn recall(&self, key: &str) -> Option<String> {
        self.inner.recall(&self.name(key))
    }

    fn remember(&self, key: &str, value: &str) -> bool {
        self.inner.remember(&self.name(key), value)
    }

    /// Every key of the inner storage, scoped ones under their suffixed names: nothing is hidden.
    fn keys(&self) -> Vec<String> {
        self.inner.keys()
    }

    fn now_ms(&self) -> u64 {
        self.inner.now_ms()
    }

    fn random_hex(&self) -> Option<String> {
        self.inner.random_hex()
    }

    fn identity(&self) -> usize {
        self.inner.identity()
    }

    fn project(&self) -> Option<String> {
        Some(self.project.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::MemoryStorage;

    fn pair() -> (Rc<MemoryStorage>, ScopedStorage, ScopedStorage) {
        let inner = Rc::new(MemoryStorage::default());
        let a = ScopedStorage::new(inner.clone(), "a");
        let b = ScopedStorage::new(inner.clone(), "b");
        (inner, a, b)
    }

    #[test]
    fn projects_do_not_see_each_others_data() {
        let (inner, a, b) = pair();
        assert!(a.remember("cityloom-edits", "A"));
        assert_eq!(b.recall("cityloom-edits"), None);
        assert!(b.remember("cityloom-edits", "B"));
        assert_eq!(a.recall("cityloom-edits"), Some("A".into()));
        assert_eq!(b.recall("cityloom-edits"), Some("B".into()));
        assert_eq!(inner.recall("cityloom-edits@a"), Some("A".into()));
        for key in SCOPED_KEYS {
            a.remember(key, "x");
            assert_eq!(inner.recall(&format!("{key}@a")), Some("x".into()));
            assert_eq!(inner.recall(key), None);
        }
    }

    #[test]
    fn the_persons_own_keys_are_shared() {
        let (_, a, b) = pair();
        a.remember("cityloom-lang", "fr");
        a.remember("cityloom-sync", "{}");
        assert_eq!(b.recall("cityloom-lang"), Some("fr".into()));
        assert_eq!(b.recall("cityloom-sync"), Some("{}".into()));
    }

    #[test]
    fn it_passes_the_rest_through() {
        let (inner, a, _) = pair();
        inner.set_now(42);
        assert_eq!(a.identity(), inner.identity());
        assert_eq!(a.now_ms(), 42);
        assert_eq!(a.random_hex(), None);
        a.remember("cityloom-edits", "A");
        assert_eq!(a.keys(), inner.keys());
        assert_eq!(a.project(), Some("a".into()));
        assert_eq!(inner.project(), None);
    }
}
