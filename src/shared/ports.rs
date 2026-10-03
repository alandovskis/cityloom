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

/// What came back from a request: the body, or what went wrong, in words.
pub type Fetched = Result<Vec<u8>, String>;

/// Asks a server for something. The answer arrives later, once, to `done`.
pub trait Fetcher {
    /// `body` makes it a POST of that text; without one it is a GET.
    fn fetch(&self, url: &str, body: Option<&str>, done: Box<dyn FnOnce(Fetched)>);
}

/// Turns OpenStreetMap data into the street network JSON the city is made from. The reader is
/// a module of its own that the page loads when it is first asked.
pub trait Importer {
    fn import(&self, osm: Vec<u8>, done: Box<dyn FnOnce(Result<String, String>)>);
}

/// Opens another page of the site.
pub trait Navigator {
    fn go(&self, href: &str);
}

/// The platform services a view-model is given.
#[derive(Clone)]
pub struct Ports {
    pub navigator: Rc<dyn Navigator>,
    pub importer: Rc<dyn Importer>,
    pub fetcher: Rc<dyn Fetcher>,
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

/// A fetcher that keeps its requests until the test answers them.
#[derive(Default)]
pub struct FakeFetcher {
    asked: RefCell<Vec<(String, Option<String>, Box<dyn FnOnce(Fetched)>)>>,
}

impl FakeFetcher {
    /// The url and body of each request not yet answered, oldest first.
    pub fn asked(&self) -> Vec<(String, Option<String>)> {
        self.asked.borrow().iter().map(|(u, b, _)| (u.clone(), b.clone())).collect()
    }

    /// Answers the oldest request not yet answered. False when there is none.
    pub fn answer(&self, result: Fetched) -> bool {
        let first = {
            let mut asked = self.asked.borrow_mut();
            if asked.is_empty() { None } else { Some(asked.remove(0)) }
        };
        match first {
            Some((_, _, done)) => {
                done(result);
                true
            }
            None => false,
        }
    }
}

impl Fetcher for FakeFetcher {
    fn fetch(&self, url: &str, body: Option<&str>, done: Box<dyn FnOnce(Fetched)>) {
        self.asked.borrow_mut().push((url.to_string(), body.map(str::to_string), done));
    }
}

/// An importer that keeps what it is asked to read until the test answers.
#[derive(Default)]
pub struct FakeImporter {
    asked: RefCell<Vec<(Vec<u8>, Box<dyn FnOnce(Result<String, String>)>)>>,
}

impl FakeImporter {
    /// What each request not yet answered asked to read, oldest first.
    pub fn asked(&self) -> Vec<Vec<u8>> {
        self.asked.borrow().iter().map(|(b, _)| b.clone()).collect()
    }

    /// Answers the oldest request not yet answered. False when there is none.
    pub fn answer(&self, result: Result<String, String>) -> bool {
        let first = {
            let mut asked = self.asked.borrow_mut();
            if asked.is_empty() { None } else { Some(asked.remove(0)) }
        };
        match first {
            Some((_, done)) => {
                done(result);
                true
            }
            None => false,
        }
    }
}

impl Importer for FakeImporter {
    fn import(&self, osm: Vec<u8>, done: Box<dyn FnOnce(Result<String, String>)>) {
        self.asked.borrow_mut().push((osm, done));
    }
}

/// A navigator that keeps where it was sent.
#[derive(Default)]
pub struct RecordingNavigator {
    gone: RefCell<Vec<String>>,
}

impl RecordingNavigator {
    /// Where it was sent since this was last called.
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.gone.borrow_mut())
    }
}

impl Navigator for RecordingNavigator {
    fn go(&self, href: &str) {
        self.gone.borrow_mut().push(href.to_string());
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
    let fetcher = Rc::new(FakeFetcher::default());
    (
        Ports {
            navigator: Rc::new(RecordingNavigator::default()),
            importer: Rc::new(FakeImporter::default()),
            fetcher,
            announcer: announcer.clone(),
            storage: storage.clone(),
            scheduler: scheduler.clone(),
        },
        announcer,
        storage,
        scheduler,
    )
}

/// The same, with the fetcher and the importer too.
pub fn test_ports_with_fetcher() -> (Ports, Rc<FakeFetcher>, Rc<FakeImporter>) {
    let (ports, fetcher, importer, _) = test_ports_with_services();
    (ports, fetcher, importer)
}

/// The same, with the navigator too.
pub fn test_ports_with_services() -> (Ports, Rc<FakeFetcher>, Rc<FakeImporter>, Rc<RecordingNavigator>) {
    let (mut ports, ..) = test_ports_with_time();
    let fetcher = Rc::new(FakeFetcher::default());
    let importer = Rc::new(FakeImporter::default());
    let navigator = Rc::new(RecordingNavigator::default());
    ports.fetcher = fetcher.clone();
    ports.importer = importer.clone();
    ports.navigator = navigator.clone();
    (ports, fetcher, importer, navigator)
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
    fn the_fake_fetcher_keeps_requests_and_answers_the_oldest_first() {
        let f = FakeFetcher::default();
        let got = Rc::new(RefCell::new(Vec::new()));
        let (a, b) = (got.clone(), got.clone());
        f.fetch("u1", None, Box::new(move |r| a.borrow_mut().push(("one", r))));
        f.fetch("u2", Some("q"), Box::new(move |r| b.borrow_mut().push(("two", r))));
        assert_eq!(f.asked(), vec![("u1".to_string(), None), ("u2".to_string(), Some("q".to_string()))]);
        assert!(f.answer(Ok(b"x".to_vec())));
        assert!(f.answer(Err("down".into())));
        assert!(!f.answer(Ok(vec![])));
        assert_eq!(*got.borrow(), vec![("one", Ok(b"x".to_vec())), ("two", Err("down".to_string()))]);
        assert!(f.asked().is_empty());
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
