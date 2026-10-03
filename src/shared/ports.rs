//! What a view-model asks of the platform it runs on. The page supplies the
//! browser's; tests supply fakes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Says something to a person who cannot see the page.
pub trait Announcer {
    fn say(&self, text: &str);
}

/// Keeps strings between visits. Storage can be blocked, which `remember` says.
pub trait Storage {
    fn recall(&self, key: &str) -> Option<String>;
    fn remember(&self, key: &str, value: &str) -> bool;
}

/// Runs something after a delay, which can be called off.
pub trait Scheduler {
    /// Calls `f` once, `ms` milliseconds from now. The id can be given to `cancel`.
    fn after(&self, ms: u32, f: Box<dyn FnOnce()>) -> u32;
    fn cancel(&self, id: u32);
}

/// The platform services a view-model is given.
#[derive(Clone)]
pub struct Ports {
    pub announcer: Rc<dyn Announcer>,
    pub storage: Rc<dyn Storage>,
    pub scheduler: Rc<dyn Scheduler>,
}

/// An announcer that keeps what it was told.
#[derive(Default)]
pub struct RecordingAnnouncer {
    said: RefCell<Vec<String>>,
}

impl RecordingAnnouncer {
    /// What was said since this was last called.
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.said.borrow_mut())
    }
}

impl Announcer for RecordingAnnouncer {
    fn say(&self, text: &str) {
        self.said.borrow_mut().push(text.to_string());
    }
}

/// Storage in memory, which can be made to refuse as a blocked one does.
#[derive(Default)]
pub struct MemoryStorage {
    kept: RefCell<HashMap<String, String>>,
    blocked: std::cell::Cell<bool>,
}

impl MemoryStorage {
    pub fn blocked(&self, blocked: bool) {
        self.blocked.set(blocked);
    }
}

impl Storage for MemoryStorage {
    fn recall(&self, key: &str) -> Option<String> {
        self.kept.borrow().get(key).cloned()
    }

    fn remember(&self, key: &str, value: &str) -> bool {
        if self.blocked.get() {
            return false;
        }
        self.kept.borrow_mut().insert(key.to_string(), value.to_string());
        true
    }
}

/// A scheduler that runs what is due only when told that time has passed.
#[derive(Default)]
pub struct ManualScheduler {
    now: std::cell::Cell<u32>,
    next: std::cell::Cell<u32>,
    tasks: RefCell<Vec<(u32, u32, Box<dyn FnOnce()>)>>,
}

impl ManualScheduler {
    /// Lets `ms` milliseconds pass, running what falls due in the order it was due.
    pub fn advance(&self, ms: u32) {
        let until = self.now.get() + ms;
        loop {
            let due = {
                let mut tasks = self.tasks.borrow_mut();
                tasks.sort_by_key(|t| (t.0, t.1));
                tasks.iter().position(|t| t.0 <= until).map(|i| tasks.remove(i))
            };
            let Some((at, _, f)) = due else { break };
            self.now.set(at.max(self.now.get()));
            f();
        }
        self.now.set(until);
    }

    /// How many things are waiting.
    pub fn pending(&self) -> usize {
        self.tasks.borrow().len()
    }
}

impl Scheduler for ManualScheduler {
    fn after(&self, ms: u32, f: Box<dyn FnOnce()>) -> u32 {
        let id = self.next.get() + 1;
        self.next.set(id);
        self.tasks.borrow_mut().push((self.now.get() + ms, id, f));
        id
    }

    fn cancel(&self, id: u32) {
        self.tasks.borrow_mut().retain(|t| t.1 != id);
    }
}

/// Ports for a test, with the fakes kept so the test can look at them.
pub fn test_ports() -> (Ports, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
    let (ports, announcer, storage, _) = test_ports_with_time();
    (ports, announcer, storage)
}

/// The same, with the scheduler too.
pub fn test_ports_with_time() -> (Ports, Rc<RecordingAnnouncer>, Rc<MemoryStorage>, Rc<ManualScheduler>) {
    let announcer = Rc::new(RecordingAnnouncer::default());
    let storage = Rc::new(MemoryStorage::default());
    let scheduler = Rc::new(ManualScheduler::default());
    (Ports { announcer: announcer.clone(), storage: storage.clone(), scheduler: scheduler.clone() }, announcer, storage, scheduler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_recording_announcer_keeps_what_it_is_told_until_asked() {
        let a = RecordingAnnouncer::default();
        a.say("one");
        a.say("two");
        assert_eq!(a.take(), vec!["one", "two"]);
        assert!(a.take().is_empty());
    }

    #[test]
    fn memory_storage_keeps_and_recalls_and_can_refuse() {
        let s = MemoryStorage::default();
        assert_eq!(s.recall("k"), None);
        assert!(s.remember("k", "v"));
        assert_eq!(s.recall("k").as_deref(), Some("v"));
        s.blocked(true);
        assert!(!s.remember("k", "w"));
        assert_eq!(s.recall("k").as_deref(), Some("v"));
    }

    #[test]
    fn the_manual_scheduler_runs_what_is_due_in_order_and_not_what_is_cancelled() {
        let s = ManualScheduler::default();
        let log = Rc::new(RefCell::new(Vec::new()));
        let (a, b, c) = (log.clone(), log.clone(), log.clone());
        s.after(30, Box::new(move || a.borrow_mut().push("late")));
        s.after(10, Box::new(move || b.borrow_mut().push("early")));
        let cancelled = s.after(20, Box::new(move || c.borrow_mut().push("never")));
        s.cancel(cancelled);
        assert_eq!(s.pending(), 2);
        s.advance(5);
        assert!(log.borrow().is_empty());
        s.advance(10);
        assert_eq!(*log.borrow(), vec!["early"]);
        s.advance(100);
        assert_eq!(*log.borrow(), vec!["early", "late"]);
        assert_eq!(s.pending(), 0);
    }

    #[test]
    fn a_task_that_schedules_another_has_it_run_when_that_falls_due() {
        let s = Rc::new(ManualScheduler::default());
        let log = Rc::new(RefCell::new(Vec::new()));
        let (s2, l2) = (s.clone(), log.clone());
        s.after(
            10,
            Box::new(move || {
                l2.borrow_mut().push("first");
                let l3 = l2.clone();
                s2.after(10, Box::new(move || l3.borrow_mut().push("second")));
            }),
        );
        s.advance(15);
        assert_eq!(*log.borrow(), vec!["first"]);
        s.advance(10);
        assert_eq!(*log.borrow(), vec!["first", "second"]);
    }
}
