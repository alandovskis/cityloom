//! What a view-model asks of the platform it runs on. The page supplies the
//! browser's; tests supply fakes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use serde::Deserialize;

/// Says something to a person who cannot see the page.
pub trait Announcer {
    fn say(&self, text: &str);
}

/// Keeps strings between visits. Storage can be blocked, which `remember` says.
pub trait Storage {
    fn recall(&self, key: &str) -> Option<String>;
    fn remember(&self, key: &str, value: &str) -> bool;
    /// Every key the storage holds.
    fn keys(&self) -> Vec<String>;
    /// Whether `what` is asked for the first time of this storage on this page, which marks it asked. Held in
    /// memory, not kept: every store a page makes on the same storage shares it, and the next page starts afresh.
    fn first_time(&self, what: &str) -> bool;
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
    /// `bounds` is the box to keep, south, west, north, east in degrees, or None to keep all of it.
    fn import(&self, osm: Vec<u8>, bounds: Option<[f64; 4]>, done: Box<dyn FnOnce(Result<String, String>)>);
}

/// Opens another page of the site.
pub trait Navigator {
    fn go(&self, href: &str);
}

/// The page itself: the language the browser is set to, and the document's `lang`.
pub trait Page {
    /// The browser's preferred language tag, such as `fr-CA`, if it tells.
    fn language(&self) -> Option<String>;
    /// Sets the `lang` of the document.
    fn set_lang(&self, tag: &str);
}

/// What the map tells the page.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MapEvent {
    /// The tiles are there. `bounds` is the area they cover: west, south, east, north, in degrees.
    Ready { bounds: [f64; 4] },
    /// The tiles could not be had.
    Failed,
    /// The map was pressed at `at` (longitude, latitude). `hot` is the place there (`s-7`, `j-3`), `area` the
    /// restriction whose outline holds it, `vertex` the corner of the outline being drawn there: the page decides
    /// what the press means.
    Press { at: [f64; 2], hot: Option<String>, area: Option<String>, vertex: Option<usize> },
    /// A corner (`index`) of the selected restriction `area` is being dragged to `at`; `done` once it is let go.
    Vertex { area: String, index: usize, at: [f64; 2], done: bool },
    /// The pointer is over a place, or is no longer.
    Hover { hot: Option<String> },
    /// The person moved the map themselves, with the pointer or the wheel.
    Moved,
    /// The part of the map the panels leave open, after every move, resize and change of insets: west, south,
    /// east, north, in degrees. Not said while the map has no size. When the insets together cover the whole
    /// width or height of the map (panels stacked on a phone), the adapter reports the whole map as open.
    View { bounds: [f64; 4] },
}

/// The map the page shows the city on. It answers by telling `listen`'s function what happened.
pub trait Mapper {
    /// Shows the places: `layers` is a MapLibre layer list over a GeoJSON source named `places`, whose
    /// features `data` holds. Called again whenever the places change.
    fn set_places(&self, layers: &str, data: &str);
    /// Shows the restrictions: `layers` is a MapLibre layer list over a GeoJSON source named `restrictions`, whose
    /// features `data` holds, drawn under the places. Called again whenever they change.
    fn set_restrictions(&self, layers: &str, data: &str);
    /// Shows the outline being drawn: `layers` over a source named `draft` holding `data`, drawn over everything.
    fn set_draft(&self, layers: &str, data: &str);
    /// Where the middle of the view is: longitude, latitude.
    fn centre(&self) -> [f64; 2];
    /// Moves the view to hold `bounds` (west, south, east, north in degrees), clear of `insets`
    /// (top, right, bottom, left, in pixels).
    fn fit(&self, bounds: [f64; 4], insets: [f64; 4]);
    /// Tells the map what floats over it, in pixels from each edge (top, right, bottom, left), so that it can say
    /// which part of it is open (`MapEvent::View`). Insets that together cover the whole width or height are
    /// ignored by the adapter: the whole map counts as open.
    fn set_insets(&self, insets: [f64; 4]);
    /// Says which places can be edited: the ones named are drawn as they are, every other place is muted. Called
    /// again whenever the set changes; the map keeps it for places it is sent later, and for a new style.
    fn set_editable(&self, hot: &[String]);
    /// Zooms in by `factor` (more than 1), or out (less than 1).
    fn zoom_by(&self, factor: f64);
    /// Moves the view by pixels: a positive `dx` moves it east, a positive `dy` south.
    fn pan_by(&self, dx: f64, dy: f64);
    /// Marks the place `hot` (as in `s-7`) as the one the pointer or focus is on, and clears any other.
    fn highlight(&self, hot: Option<&str>);
    /// Whether the scale is shown in feet and miles.
    fn set_imperial(&self, imperial: bool);
    /// Has `on` called with what the map tells the page, from now on.
    fn listen(&self, on: Box<dyn Fn(MapEvent)>);
}

/// The map of a page that has none.
pub struct NoMapper;

impl Mapper for NoMapper {
    fn set_places(&self, _: &str, _: &str) {}
    fn set_restrictions(&self, _: &str, _: &str) {}
    fn set_draft(&self, _: &str, _: &str) {}
    fn centre(&self) -> [f64; 2] {
        [0.0, 0.0]
    }
    fn fit(&self, _: [f64; 4], _: [f64; 4]) {}
    fn set_insets(&self, _: [f64; 4]) {}
    fn set_editable(&self, _: &[String]) {}
    fn zoom_by(&self, _: f64) {}
    fn pan_by(&self, _: f64, _: f64) {}
    fn highlight(&self, _: Option<&str>) {}
    fn set_imperial(&self, _: bool) {}
    fn listen(&self, _: Box<dyn Fn(MapEvent)>) {}
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
    pub mapper: Rc<dyn Mapper>,
    pub page: Rc<dyn Page>,
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
    /// Keys it refuses to keep anything under.
    refused: RefCell<std::collections::HashSet<String>>,
    /// What was asked for the first time (`Storage::first_time`).
    asked: RefCell<std::collections::HashSet<String>>,
}

impl MemoryStorage {
    pub fn blocked(&self, blocked: bool) {
        self.blocked.set(blocked);
    }

    /// Refuses to keep anything under `key` from now on, as a storage near its quota refuses a large value.
    #[cfg(test)]
    pub fn refuse(&self, key: &str) {
        self.refused.borrow_mut().insert(key.to_string());
    }

    #[cfg(test)]
    pub fn refuse_nothing(&self) {
        self.refused.borrow_mut().clear();
    }

    /// Forgets what was asked for the first time, as a new page of the same browser storage would.
    #[cfg(test)]
    pub fn next_page(&self) {
        self.asked.borrow_mut().clear();
    }

    /// Removes what is kept under `key`, as clearing a browser's site data would.
    #[cfg(test)]
    pub fn forget(&self, key: &str) {
        self.kept.borrow_mut().remove(key);
    }
}

impl Storage for MemoryStorage {
    fn recall(&self, key: &str) -> Option<String> {
        self.kept.borrow().get(key).cloned()
    }

    fn remember(&self, key: &str, value: &str) -> bool {
        if self.blocked.get() || self.refused.borrow().contains(key) {
            return false;
        }
        self.kept.borrow_mut().insert(key.to_string(), value.to_string());
        true
    }

    fn keys(&self) -> Vec<String> {
        self.kept.borrow().keys().cloned().collect()
    }

    fn first_time(&self, what: &str) -> bool {
        self.asked.borrow_mut().insert(what.to_string())
    }
}

/// A task a `ManualScheduler` holds: its id, when it falls due, and what it runs.
type Task = (u32, u32, Box<dyn FnOnce()>);

/// A scheduler that runs what is due only when told that time has passed.
#[derive(Default)]
pub struct ManualScheduler {
    now: std::cell::Cell<u32>,
    next: std::cell::Cell<u32>,
    tasks: RefCell<Vec<Task>>,
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

/// A request a `FakeFetcher` keeps: the address, the body and what is told the answer.
type FetchRequest = (String, Option<String>, Box<dyn FnOnce(Fetched)>);

/// A fetcher that keeps its requests until the test answers them.
#[derive(Default)]
pub struct FakeFetcher {
    asked: RefCell<Vec<FetchRequest>>,
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

/// What a `FakeImporter` keeps of a request: the file, the bounds and what is told the result.
type ImportRequest = (Vec<u8>, Option<[f64; 4]>, Box<dyn FnOnce(Result<String, String>)>);

/// An importer that keeps what it is asked to read until the test answers.
#[derive(Default)]
pub struct FakeImporter {
    asked: RefCell<Vec<ImportRequest>>,
}

impl FakeImporter {
    /// What each request not yet answered asked to read, oldest first.
    pub fn asked(&self) -> Vec<Vec<u8>> {
        self.asked.borrow().iter().map(|(b, ..)| b.clone()).collect()
    }

    /// The box each request not yet answered asked to keep, oldest first.
    pub fn bounds_asked(&self) -> Vec<Option<[f64; 4]>> {
        self.asked.borrow().iter().map(|(_, b, _)| *b).collect()
    }

    /// Answers the oldest request not yet answered. False when there is none.
    pub fn answer(&self, result: Result<String, String>) -> bool {
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

impl Importer for FakeImporter {
    fn import(&self, osm: Vec<u8>, bounds: Option<[f64; 4]>, done: Box<dyn FnOnce(Result<String, String>)>) {
        self.asked.borrow_mut().push((osm, bounds, done));
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

/// A page whose language a test sets, and whose `lang` it keeps.
#[derive(Default)]
pub struct FakePage {
    pub language: RefCell<Option<String>>,
    pub lang: RefCell<String>,
}

impl Page for FakePage {
    fn language(&self) -> Option<String> {
        self.language.borrow().clone()
    }
    fn set_lang(&self, tag: &str) {
        *self.lang.borrow_mut() = tag.to_string();
    }
}

/// What a `FakeMapper` was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum MapCall {
    Places { layers: String, data: String },
    Restrictions { layers: String, data: String },
    Draft { layers: String, data: String },
    Fit { bounds: [f64; 4], insets: [f64; 4] },
    Insets([f64; 4]),
    Editable(Vec<String>),
    Zoom(f64),
    Pan(f64, f64),
    Highlight(Option<String>),
    Imperial(bool),
}

/// What listens to a map's events.
type MapListener = Box<dyn Fn(MapEvent)>;

/// A map that keeps what it is asked, and passes on what a test has it say.
#[derive(Default)]
pub struct FakeMapper {
    calls: RefCell<Vec<MapCall>>,
    listener: RefCell<Option<MapListener>>,
    centre: Cell<[f64; 2]>,
}

impl FakeMapper {
    /// What it was asked since this was last called.
    pub fn take(&self) -> Vec<MapCall> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }

    /// Moves the middle of the view, as the person panning would.
    pub fn set_centre(&self, at: [f64; 2]) {
        self.centre.set(at);
    }

    /// The map tells the page something.
    pub fn say(&self, event: MapEvent) {
        if let Some(on) = &*self.listener.borrow() {
            on(event);
        }
    }
}

impl Mapper for FakeMapper {
    fn set_places(&self, layers: &str, data: &str) {
        self.calls.borrow_mut().push(MapCall::Places { layers: layers.into(), data: data.into() });
    }
    fn set_restrictions(&self, layers: &str, data: &str) {
        self.calls.borrow_mut().push(MapCall::Restrictions { layers: layers.into(), data: data.into() });
    }
    fn set_draft(&self, layers: &str, data: &str) {
        self.calls.borrow_mut().push(MapCall::Draft { layers: layers.into(), data: data.into() });
    }
    fn centre(&self) -> [f64; 2] {
        self.centre.get()
    }
    fn fit(&self, bounds: [f64; 4], insets: [f64; 4]) {
        self.calls.borrow_mut().push(MapCall::Fit { bounds, insets });
    }
    fn set_insets(&self, insets: [f64; 4]) {
        self.calls.borrow_mut().push(MapCall::Insets(insets));
    }
    fn set_editable(&self, hot: &[String]) {
        self.calls.borrow_mut().push(MapCall::Editable(hot.to_vec()));
    }
    fn zoom_by(&self, factor: f64) {
        self.calls.borrow_mut().push(MapCall::Zoom(factor));
    }
    fn pan_by(&self, dx: f64, dy: f64) {
        self.calls.borrow_mut().push(MapCall::Pan(dx, dy));
    }
    fn highlight(&self, hot: Option<&str>) {
        self.calls.borrow_mut().push(MapCall::Highlight(hot.map(String::from)));
    }
    fn set_imperial(&self, imperial: bool) {
        self.calls.borrow_mut().push(MapCall::Imperial(imperial));
    }
    fn listen(&self, on: Box<dyn Fn(MapEvent)>) {
        *self.listener.borrow_mut() = Some(on);
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
            mapper: Rc::new(NoMapper),
            page: Rc::new(FakePage::default()),
        },
        announcer,
        storage,
        scheduler,
    )
}

/// The same, with the page too, so a test can set the browser's language.
pub fn test_ports_with_page() -> (Ports, Rc<FakePage>, Rc<MemoryStorage>) {
    let (mut ports, _, storage, _) = test_ports_with_time();
    let page = Rc::new(FakePage::default());
    ports.page = page.clone();
    (ports, page, storage)
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
    fn a_storage_lists_the_keys_it_holds() {
        let s = MemoryStorage::default();
        assert!(s.keys().is_empty());
        s.remember("a", "1");
        s.remember("b:2", "2");
        let mut keys = s.keys();
        keys.sort();
        assert_eq!(keys, ["a", "b:2"]);
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

#[cfg(test)]
mod map_tests {
    use super::*;

    #[test]
    fn the_adapters_events_are_read_from_json() {
        let read = |s: &str| serde_json::from_str::<MapEvent>(s).unwrap();
        assert_eq!(read(r#"{"kind":"ready","bounds":[1,2,3,4]}"#), MapEvent::Ready { bounds: [1.0, 2.0, 3.0, 4.0] });
        assert_eq!(read(r#"{"kind":"failed"}"#), MapEvent::Failed);
        assert_eq!(
            read(r#"{"kind":"press","at":[1,2],"hot":"s-7","area":null,"vertex":null}"#),
            MapEvent::Press { at: [1.0, 2.0], hot: Some("s-7".into()), area: None, vertex: None }
        );
        assert_eq!(
            read(r#"{"kind":"press","at":[1,2],"area":"r3","vertex":0}"#),
            MapEvent::Press { at: [1.0, 2.0], hot: None, area: Some("r3".into()), vertex: Some(0) }
        );
        assert_eq!(
            read(r#"{"kind":"vertex","area":"r1","index":2,"at":[3,4],"done":true}"#),
            MapEvent::Vertex { area: "r1".into(), index: 2, at: [3.0, 4.0], done: true }
        );
        assert_eq!(read(r#"{"kind":"hover","hot":"j-3"}"#), MapEvent::Hover { hot: Some("j-3".into()) });
        assert_eq!(read(r#"{"kind":"hover","hot":null}"#), MapEvent::Hover { hot: None });
        assert_eq!(read(r#"{"kind":"moved"}"#), MapEvent::Moved);
    }

    #[test]
    fn the_open_view_of_the_map_is_read_from_its_json() {
        let read = |s: &str| serde_json::from_str::<MapEvent>(s).unwrap();
        assert_eq!(read(r#"{"kind":"view","bounds":[1,2,3,4]}"#), MapEvent::View { bounds: [1.0, 2.0, 3.0, 4.0] });
    }

    #[test]
    fn the_fake_map_keeps_the_editable_set_and_the_insets_it_is_given() {
        let fake = FakeMapper::default();
        fake.set_editable(&["s-1".to_string(), "j-2".to_string()]);
        fake.set_insets([1.0, 2.0, 3.0, 4.0]);
        assert_eq!(fake.take(), vec![MapCall::Editable(vec!["s-1".into(), "j-2".into()]), MapCall::Insets([1.0, 2.0, 3.0, 4.0])]);
    }

    #[test]
    fn the_fake_records_what_it_is_asked_and_passes_on_what_it_is_told() {
        let fake = FakeMapper::default();
        fake.zoom_by(1.4);
        fake.pan_by(-80.0, 0.0);
        fake.highlight(Some("s-1"));
        fake.set_imperial(true);
        fake.fit([1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]);
        fake.set_places("[]", "{}");
        assert_eq!(
            fake.take(),
            vec![
                MapCall::Zoom(1.4),
                MapCall::Pan(-80.0, 0.0),
                MapCall::Highlight(Some("s-1".into())),
                MapCall::Imperial(true),
                MapCall::Fit { bounds: [1.0, 2.0, 3.0, 4.0], insets: [5.0, 6.0, 7.0, 8.0] },
                MapCall::Places { layers: "[]".into(), data: "{}".into() },
            ]
        );
        fake.set_restrictions("[1]", "{1}");
        fake.set_draft("[2]", "{2}");
        assert_eq!(
            fake.take(),
            vec![MapCall::Restrictions { layers: "[1]".into(), data: "{1}".into() }, MapCall::Draft { layers: "[2]".into(), data: "{2}".into() }]
        );
        assert_eq!(fake.centre(), [0.0, 0.0]);
        fake.set_centre([-73.6, 45.5]);
        assert_eq!(fake.centre(), [-73.6, 45.5]);
        assert_eq!(NoMapper.centre(), [0.0, 0.0]);
        assert!(fake.take().is_empty(), "taken once");

        let heard = Rc::new(RefCell::new(Vec::new()));
        let sink = heard.clone();
        fake.listen(Box::new(move |e| sink.borrow_mut().push(e)));
        fake.say(MapEvent::Moved);
        assert_eq!(*heard.borrow(), vec![MapEvent::Moved]);
    }
}
