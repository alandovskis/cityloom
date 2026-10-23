//! What a view-model asks of the platform it runs on. The page supplies the
//! browser's; tests supply fakes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

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
    /// The time, in milliseconds since 1970. The stores that keep a person's edits stamp them with it when they mark
    /// them for syncing, and they hold only this port.
    fn now_ms(&self) -> u64;
    /// 128 random bits as 32 lowercase hex digits (the ids of area rules), or None where the storage has no randomness
    /// (a test fake): rule ids are then the sequential `r<n>`.
    fn random_hex(&self) -> Option<String>;
    /// Which storage this is, for whoever hears of a mark: the same number for every handle to the same storage. Every
    /// `BrowserStorage` is the page's one `localStorage` (each mount makes a handle of its own, so the object behind
    /// a handle cannot name it); a fake gives each instance a number of its own, so two simulated devices stay apart.
    fn identity(&self) -> usize;
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
    /// Starts loading the reader, because it will be asked to read soon (the data it reads is being fetched).
    /// Nothing waits for this, and asking again does nothing.
    fn prepare(&self) {}

    /// `bounds` is the box to keep, south, west, north, east in degrees, or None to keep all of it.
    fn import(&self, osm: Vec<u8>, bounds: Option<[f64; 4]>, done: Box<dyn FnOnce(Result<String, String>)>);
}

/// Opens another page of the site.
pub trait Navigator {
    fn go(&self, href: &str);
}

/// What a person's account looks like to the page, as the server's `GET /api/me` says it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Me {
    pub id: String,
    pub display_name: String,
    /// None until the person has chosen whether the server keeps their edits.
    pub store_on_server: Option<bool>,
    pub identities: Vec<LinkedIdentity>,
}

/// One sign-in method linked to the account.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LinkedIdentity {
    pub id: String,
    pub provider: String,
    pub label: Option<String>,
    pub created_at: String,
}

/// A provider the person can sign in with.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ProviderInfo {
    pub slug: String,
    pub name: String,
}

/// Why an account request failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountError {
    /// 401: nobody is signed in.
    SignedOut,
    /// 5xx, no network, or an answer that cannot be read.
    Unavailable,
    /// 409: the identity is the account's last one.
    LastIdentity,
    /// 404.
    NotFound,
    /// Any other refusal.
    Refused,
}

/// What an HTTP status means for an account request.
pub fn account_error(status: u16) -> AccountError {
    match status {
        401 => AccountError::SignedOut,
        404 => AccountError::NotFound,
        409 => AccountError::LastIdentity,
        0 | 500..=599 => AccountError::Unavailable,
        _ => AccountError::Refused,
    }
}

impl Me {
    pub fn parse(body: &str) -> Result<Me, AccountError> {
        serde_json::from_str(body).map_err(|_| AccountError::Unavailable)
    }
}

impl ProviderInfo {
    /// Reads the server's `{"providers":[..]}`.
    pub fn parse_list(body: &str) -> Result<Vec<ProviderInfo>, AccountError> {
        #[derive(Deserialize)]
        struct List {
            providers: Vec<ProviderInfo>,
        }
        serde_json::from_str::<List>(body).map(|l| l.providers).map_err(|_| AccountError::Unavailable)
    }
}

/// What an account request answers with, once.
pub type Answer<T> = Box<dyn FnOnce(Result<T, AccountError>)>;

/// The person's account on the server. Each answer arrives later, once, to `done`.
pub trait Account {
    fn me(&self, done: Answer<Me>);
    fn providers(&self, done: Answer<Vec<ProviderInfo>>);
    fn set_storage(&self, store: bool, done: Answer<()>);
    fn unlink(&self, identity_id: &str, done: Answer<()>);
    fn delete(&self, done: Answer<()>);
    fn sign_out(&self, done: Answer<()>);
}

/// One record the server keeps for the person, as `GET /api/work` and `PUT /api/work` carry it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkRecord {
    /// `street`, `junction`, `rule` or `area`.
    pub kind: String,
    pub key: String,
    pub rev: i64,
    /// UTC to the millisecond (`2027-01-15T08:00:00.000Z`), so that two compare as text.
    pub edited_at: String,
    pub deleted: bool,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    /// Set by the server when the body could not be opened (`"corrupt"`): the record is then left alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WorkPage {
    /// The copy of the account's work the page was read from: new each time storage is turned on.
    pub epoch: String,
    pub rev: i64,
    pub more: bool,
    pub records: Vec<WorkRecord>,
    /// The account the server answered for (the session cookie's), if it says. A page that believes in another account
    /// is out of date: it must not sync under this one.
    #[serde(default)]
    pub user: Option<String>,
}

/// What a pull asks of the server. `since` is the revision to go on from; `start` is the revision the whole pull began
/// from (the server judges the device's right to its revisions by where the pull began, so a tombstone purged while a
/// long pull pages does not make the later pages start over); `epoch` is the copy the device last read, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkAsk {
    pub since: i64,
    pub start: i64,
    pub epoch: Option<String>,
}

impl WorkAsk {
    /// The first page of a pull from revision `since`.
    pub fn first(since: i64) -> WorkAsk {
        WorkAsk { since, start: since, epoch: None }
    }

    pub fn with_epoch(self, epoch: Option<String>) -> WorkAsk {
        WorkAsk { epoch, ..self }
    }

    /// The page after one that said `more`, from revision `since`; the pull still began where it did.
    pub fn continued(&self, since: i64) -> WorkAsk {
        WorkAsk { since, ..self.clone() }
    }

    /// The query string of `GET /api/work`.
    pub fn query(&self) -> String {
        let mut q = format!("since={}&start={}", self.since, self.start);
        if let Some(epoch) = &self.epoch {
            q.push_str("&epoch=");
            q.push_str(epoch);
        }
        q
    }
}

/// A record the device sends: `base_rev` is the revision it last saw of it (0 for none).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WorkPut {
    pub kind: String,
    pub key: String,
    pub base_rev: i64,
    pub edited_at: String,
    pub deleted: bool,
    pub body: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkStatus {
    Accepted,
    Superseded,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WorkResult {
    pub kind: String,
    pub key: String,
    pub status: WorkStatus,
    pub rev: i64,
    /// The server's record, when superseded.
    #[serde(default)]
    pub record: Option<WorkRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WorkPushed {
    pub rev: i64,
    pub results: Vec<WorkResult>,
}

/// Why a work request failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkError {
    /// 401: nobody is signed in.
    SignedOut,
    /// 409: the account does not store its work on the server (any more).
    StorageOff,
    /// 410 `resync`: pull everything, and drop what the server no longer lists.
    Resync,
    /// 410 `reset`: the account's server copy was turned off and on again since this device read it; merge as when
    /// storage is turned on.
    Reset,
    /// 413: over a limit, named (`record`, `records`, `bytes`, `batch`; empty if the server did not say).
    TooLarge(String),
    /// 429, 5xx, no network, or an answer that cannot be read.
    Unavailable,
    /// Any other refusal.
    Refused,
}

/// What an HTTP status (and, for 413, the body) means for a work request.
pub fn work_error(status: u16, body: &str) -> WorkError {
    match status {
        401 => WorkError::SignedOut,
        409 => WorkError::StorageOff,
        410 => {
            if serde_json::from_str::<serde_json::Value>(body).ok().is_some_and(|v| v["error"] == "reset") {
                WorkError::Reset
            } else {
                WorkError::Resync
            }
        }
        413 => {
            WorkError::TooLarge(serde_json::from_str::<serde_json::Value>(body).ok().and_then(|v| v["limit"].as_str().map(str::to_string)).unwrap_or_default())
        }
        0 | 429 | 500..=599 => WorkError::Unavailable,
        _ => WorkError::Refused,
    }
}

impl WorkPage {
    pub fn parse(body: &str) -> Result<WorkPage, WorkError> {
        serde_json::from_str(body).map_err(|_| WorkError::Unavailable)
    }
}

impl WorkPushed {
    pub fn parse(body: &str) -> Result<WorkPushed, WorkError> {
        serde_json::from_str(body).map_err(|_| WorkError::Unavailable)
    }
}

/// What a work request answers with, once.
pub type WorkAnswer<T> = Box<dyn FnOnce(Result<T, WorkError>)>;

/// The person's work on the server. Each answer arrives later, once, to `done`.
pub trait WorkSync {
    /// One page of the records changed after `ask.since`; `more` says to ask again from the page's `rev` with
    /// `ask.continued(page.rev)`.
    fn pull(&self, ask: WorkAsk, done: WorkAnswer<WorkPage>);
    /// Sends a batch. `keepalive` lets the request outlive the page (the page is being left); the browser caps such a
    /// body at 64 KiB, so a keepalive batch is cut to fit by the caller.
    fn push(&self, batch: Vec<WorkPut>, keepalive: bool, done: WorkAnswer<WorkPushed>);
    /// Deletes the account's server copy.
    fn clear(&self, done: WorkAnswer<()>);
}

/// Whether the text after `#` is one the account's sign-in puts there; any other fragment is the page's own (an
/// anchor) and is neither read nor removed.
pub fn is_account_fragment(fragment: &str) -> bool {
    fragment.starts_with("account=")
}

/// The page itself: the language the browser is set to, and the document's `lang`.
pub trait Page {
    /// The address within the site, path and query, such as `/street.html?street=7`.
    fn path(&self) -> String;
    /// The text after `#` when it is the account's (`account=...`), else None and the address is left as it is; the
    /// fragment is removed from the address, so a second call gives None.
    fn take_fragment(&self) -> Option<String>;
    /// The browser's preferred language tags, most preferred first, such as `["de", "fr-CA"]`; empty if it tells none.
    fn languages(&self) -> Vec<String>;
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
    /// Shows the zone boundaries: `layers` over a source named `zone-ticks` holding `data`, drawn over the places and
    /// under the outline being drawn.
    fn set_zone_ticks(&self, layers: &str, data: &str);
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
    fn set_zone_ticks(&self, _: &str, _: &str) {}
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
    pub account: Rc<dyn Account>,
    pub work: Rc<dyn WorkSync>,
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
    now: std::cell::Cell<u64>,
    randomness: std::cell::Cell<bool>,
    drawn: std::cell::Cell<u64>,
    identity: std::cell::OnceCell<usize>,
}

impl MemoryStorage {
    /// Sets the time `now_ms` tells.
    pub fn set_now(&self, ms: u64) {
        self.now.set(ms);
    }

    /// Moves the time on.
    pub fn tick(&self, ms: u64) {
        self.now.set(self.now.get() + ms);
    }

    /// Whether `random_hex` gives digits or None. The digits are a counter, so a test can predict them, started
    /// apart for each storage so that two storages (two devices) never draw the same ones.
    pub fn with_randomness(&self, on: bool) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.randomness.set(on);
        if on && self.drawn.get() == 0 {
            self.drawn.set(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) << 32);
        }
    }

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

    fn now_ms(&self) -> u64 {
        self.now.get()
    }

    fn random_hex(&self) -> Option<String> {
        if !self.randomness.get() {
            return None;
        }
        self.drawn.set(self.drawn.get() + 1);
        Some(format!("{:032x}", self.drawn.get()))
    }

    fn identity(&self) -> usize {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        *self.identity.get_or_init(|| NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
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
    prepared: Cell<u32>,
}

impl FakeImporter {
    /// How many times the reader was told to get ready.
    pub fn prepared(&self) -> u32 {
        self.prepared.get()
    }

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
    fn prepare(&self) {
        self.prepared.set(self.prepared.get() + 1);
    }

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
pub struct FakePage {
    pub languages: RefCell<Vec<String>>,
    pub lang: RefCell<String>,
    pub path: RefCell<String>,
    pub fragment: RefCell<Option<String>>,
}

impl Default for FakePage {
    fn default() -> Self {
        FakePage { languages: RefCell::new(Vec::new()), lang: RefCell::new(String::new()), path: RefCell::new("/".into()), fragment: RefCell::new(None) }
    }
}

impl Page for FakePage {
    fn path(&self) -> String {
        self.path.borrow().clone()
    }
    fn take_fragment(&self) -> Option<String> {
        let mut fragment = self.fragment.borrow_mut();
        if fragment.as_deref().is_some_and(is_account_fragment) { fragment.take() } else { None }
    }
    fn languages(&self) -> Vec<String> {
        self.languages.borrow().clone()
    }
    fn set_lang(&self, tag: &str) {
        *self.lang.borrow_mut() = tag.to_string();
    }
}

/// What a `FakeAccount` was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum AccountCall {
    Me,
    Providers,
    SetStorage(bool),
    Unlink(String),
    Delete,
    SignOut,
}

/// An answer for `FakeAccount::answer`.
pub enum AccountReply {
    Me(Result<Me, AccountError>),
    Providers(Result<Vec<ProviderInfo>, AccountError>),
    Done(Result<(), AccountError>),
}

enum Pending {
    Me(Answer<Me>),
    Providers(Answer<Vec<ProviderInfo>>),
    Done(Answer<()>),
}

/// An account that keeps what it was asked until a test answers.
#[derive(Default)]
pub struct FakeAccount {
    asked: RefCell<Vec<(AccountCall, Pending)>>,
}

impl FakeAccount {
    /// The calls not yet answered, oldest first.
    pub fn calls(&self) -> Vec<AccountCall> {
        self.asked.borrow().iter().map(|(c, _)| c.clone()).collect()
    }

    /// Answers the oldest pending call of that kind (`Me`, `Providers`, or any of the calls that answer with nothing);
    /// false if there is none.
    pub fn answer(&self, reply: AccountReply) -> bool {
        let at = self.asked.borrow().iter().position(|(_, p)| {
            matches!(
                (p, &reply),
                (Pending::Me(_), AccountReply::Me(_)) | (Pending::Providers(_), AccountReply::Providers(_)) | (Pending::Done(_), AccountReply::Done(_))
            )
        });
        let Some(at) = at else { return false };
        let (_, pending) = self.asked.borrow_mut().remove(at);
        match (pending, reply) {
            (Pending::Me(done), AccountReply::Me(r)) => done(r),
            (Pending::Providers(done), AccountReply::Providers(r)) => done(r),
            (Pending::Done(done), AccountReply::Done(r)) => done(r),
            _ => unreachable!("matched above"),
        }
        true
    }
}

impl Account for FakeAccount {
    fn me(&self, done: Answer<Me>) {
        self.asked.borrow_mut().push((AccountCall::Me, Pending::Me(done)));
    }
    fn providers(&self, done: Answer<Vec<ProviderInfo>>) {
        self.asked.borrow_mut().push((AccountCall::Providers, Pending::Providers(done)));
    }
    fn set_storage(&self, store: bool, done: Answer<()>) {
        self.asked.borrow_mut().push((AccountCall::SetStorage(store), Pending::Done(done)));
    }
    fn unlink(&self, identity_id: &str, done: Answer<()>) {
        self.asked.borrow_mut().push((AccountCall::Unlink(identity_id.into()), Pending::Done(done)));
    }
    fn delete(&self, done: Answer<()>) {
        self.asked.borrow_mut().push((AccountCall::Delete, Pending::Done(done)));
    }
    fn sign_out(&self, done: Answer<()>) {
        self.asked.borrow_mut().push((AccountCall::SignOut, Pending::Done(done)));
    }
}

/// What a `FakeWorkSync` was asked.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkCall {
    Pull(WorkAsk),
    Push { batch: Vec<WorkPut>, keepalive: bool },
    Clear,
}

/// An answer for `FakeWorkSync::answer`.
pub enum WorkReply {
    Page(Result<WorkPage, WorkError>),
    Pushed(Result<WorkPushed, WorkError>),
    Cleared(Result<(), WorkError>),
}

enum WorkPending {
    Page(WorkAnswer<WorkPage>),
    Pushed(WorkAnswer<WorkPushed>),
    Cleared(WorkAnswer<()>),
}

/// A work store that keeps what it was asked until a test answers.
#[derive(Default)]
pub struct FakeWorkSync {
    asked: RefCell<Vec<(WorkCall, WorkPending)>>,
}

impl FakeWorkSync {
    /// The calls not yet answered, oldest first.
    pub fn calls(&self) -> Vec<WorkCall> {
        self.asked.borrow().iter().map(|(c, _)| c.clone()).collect()
    }

    /// Answers the oldest pending call of that kind; false if there is none.
    pub fn answer(&self, reply: WorkReply) -> bool {
        let at = self.asked.borrow().iter().position(|(_, p)| {
            matches!(
                (p, &reply),
                (WorkPending::Page(_), WorkReply::Page(_)) | (WorkPending::Pushed(_), WorkReply::Pushed(_)) | (WorkPending::Cleared(_), WorkReply::Cleared(_))
            )
        });
        let Some(at) = at else { return false };
        let (_, pending) = self.asked.borrow_mut().remove(at);
        match (pending, reply) {
            (WorkPending::Page(done), WorkReply::Page(r)) => done(r),
            (WorkPending::Pushed(done), WorkReply::Pushed(r)) => done(r),
            (WorkPending::Cleared(done), WorkReply::Cleared(r)) => done(r),
            _ => unreachable!("matched above"),
        }
        true
    }
}

impl WorkSync for FakeWorkSync {
    fn pull(&self, ask: WorkAsk, done: WorkAnswer<WorkPage>) {
        self.asked.borrow_mut().push((WorkCall::Pull(ask), WorkPending::Page(done)));
    }
    fn push(&self, batch: Vec<WorkPut>, keepalive: bool, done: WorkAnswer<WorkPushed>) {
        self.asked.borrow_mut().push((WorkCall::Push { batch, keepalive }, WorkPending::Pushed(done)));
    }
    fn clear(&self, done: WorkAnswer<()>) {
        self.asked.borrow_mut().push((WorkCall::Clear, WorkPending::Cleared(done)));
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
    zone_ticks: RefCell<(String, String)>,
    listener: RefCell<Option<MapListener>>,
    centre: Cell<[f64; 2]>,
}

impl FakeMapper {
    /// What it was asked since this was last called.
    pub fn take(&self) -> Vec<MapCall> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }

    /// The last zone ticks it was asked to show (layers, data); not among `take`'s calls.
    pub fn zone_ticks(&self) -> (String, String) {
        self.zone_ticks.borrow().clone()
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
    fn set_zone_ticks(&self, layers: &str, data: &str) {
        *self.zone_ticks.borrow_mut() = (layers.into(), data.into());
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
            account: Rc::new(FakeAccount::default()),
            work: Rc::new(FakeWorkSync::default()),
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

/// The fakes behind `test_ports_full`.
pub struct TestPorts {
    pub ports: Ports,
    pub account: Rc<FakeAccount>,
    pub navigator: Rc<RecordingNavigator>,
    pub page: Rc<FakePage>,
    pub announcer: Rc<RecordingAnnouncer>,
    pub storage: Rc<MemoryStorage>,
    pub work: Rc<FakeWorkSync>,
}

/// The same, with the account, the navigator and the page too.
pub fn test_ports_full() -> TestPorts {
    let (mut ports, announcer, storage, _) = test_ports_with_time();
    let (account, navigator, page) = (Rc::new(FakeAccount::default()), Rc::new(RecordingNavigator::default()), Rc::new(FakePage::default()));
    ports.account = account.clone();
    ports.navigator = navigator.clone();
    ports.page = page.clone();
    let work = Rc::new(FakeWorkSync::default());
    ports.work = work.clone();
    TestPorts { ports, account, navigator, page, announcer, storage, work }
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

    fn me_json() -> &'static str {
        r#"{"id":"3f2b","display_name":"Ada","store_on_server":null,"identities":[{"id":"ab12","provider":"google","label":"ada@example.org","created_at":"2026-10-14T12:00:00Z"},{"id":"cd34","provider":"osm","label":null,"created_at":"2026-10-15T12:00:00Z"}]}"#
    }

    #[test]
    fn the_status_of_an_answer_says_what_went_wrong() {
        for (status, error) in [
            (401, AccountError::SignedOut),
            (404, AccountError::NotFound),
            (409, AccountError::LastIdentity),
            (503, AccountError::Unavailable),
            (500, AccountError::Unavailable),
            (502, AccountError::Unavailable),
            (0, AccountError::Unavailable),
            (400, AccountError::Refused),
            (403, AccountError::Refused),
        ] {
            assert_eq!(account_error(status), error, "{status}");
        }
    }

    #[test]
    fn me_reads_the_servers_json_and_anything_else_is_unavailable() {
        let me = Me::parse(me_json()).unwrap();
        assert_eq!((me.display_name.as_str(), me.store_on_server, me.identities.len()), ("Ada", None, 2));
        assert_eq!(me.identities[1].label, None);
        assert_eq!(Me::parse("<html>").unwrap_err(), AccountError::Unavailable);
        assert_eq!(Me::parse(&me_json().replace("null,\"identities", "true,\"identities")).unwrap().store_on_server, Some(true));
    }

    #[test]
    fn the_providers_are_read_from_their_list() {
        let list = ProviderInfo::parse_list(r#"{"providers":[{"slug":"google","name":"Google"},{"slug":"osm","name":"OpenStreetMap"}]}"#).unwrap();
        assert_eq!(list.iter().map(|p| p.slug.as_str()).collect::<Vec<_>>(), ["google", "osm"]);
        assert_eq!(ProviderInfo::parse_list("{}").unwrap_err(), AccountError::Unavailable);
    }

    #[test]
    fn the_fake_account_keeps_calls_until_answered_by_kind_oldest_first() {
        let t = test_ports_full();
        let got = Rc::new(RefCell::new(Vec::<String>::new()));
        let g = got.clone();
        t.ports.account.me(Box::new(move |r| g.borrow_mut().push(format!("me {:?}", r.map(|m| m.display_name)))));
        let g = got.clone();
        t.ports.account.set_storage(true, Box::new(move |r| g.borrow_mut().push(format!("storage {r:?}"))));
        let g = got.clone();
        t.ports.account.unlink("ab", Box::new(move |r| g.borrow_mut().push(format!("unlink {r:?}"))));
        assert_eq!(t.account.calls(), [AccountCall::Me, AccountCall::SetStorage(true), AccountCall::Unlink("ab".into())]);
        assert!(!t.account.answer(AccountReply::Providers(Ok(vec![]))), "no providers call is pending");
        assert!(t.account.answer(AccountReply::Done(Ok(()))));
        assert!(t.account.answer(AccountReply::Me(Err(AccountError::SignedOut))));
        assert_eq!(t.account.calls(), [AccountCall::Unlink("ab".into())]);
        assert_eq!(*got.borrow(), ["storage Ok(())", "me Err(SignedOut)"]);
    }

    #[test]
    fn the_fake_page_gives_its_fragment_once_and_has_a_path() {
        let t = test_ports_full();
        assert_eq!(t.ports.page.path(), "/");
        *t.page.fragment.borrow_mut() = Some("account=new".into());
        assert_eq!(t.ports.page.take_fragment().as_deref(), Some("account=new"));
        assert_eq!(t.ports.page.take_fragment(), None);
    }

    #[test]
    fn the_fake_page_leaves_a_fragment_that_is_not_the_accounts() {
        let t = test_ports_full();
        for other in ["street=7", "section-2", "accounts", ""] {
            *t.page.fragment.borrow_mut() = Some(other.into());
            assert_eq!(t.ports.page.take_fragment(), None, "{other:?}");
            assert_eq!(t.page.fragment.borrow().as_deref(), Some(other), "{other:?} is left in the address");
        }
    }
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

    #[test]
    fn the_status_of_a_work_answer_says_what_went_wrong() {
        assert_eq!(work_error(401, ""), WorkError::SignedOut);
        assert_eq!(work_error(409, r#"{"error":"storage-off"}"#), WorkError::StorageOff);
        assert_eq!(work_error(410, r#"{"error":"resync"}"#), WorkError::Resync);
        assert_eq!(work_error(410, r#"{"error":"reset"}"#), WorkError::Reset);
        assert_eq!(work_error(410, ""), WorkError::Resync);
        assert_eq!(work_error(413, r#"{"error":"too-large","limit":"record"}"#), WorkError::TooLarge("record".into()));
        assert_eq!(work_error(413, "not json"), WorkError::TooLarge(String::new()));
        for unavailable in [0, 429, 500, 502, 503] {
            assert_eq!(work_error(unavailable, ""), WorkError::Unavailable, "{unavailable}");
        }
        assert_eq!(work_error(400, ""), WorkError::Refused);
    }

    #[test]
    fn a_pull_asks_by_query_and_goes_on_from_where_it_began() {
        let first = WorkAsk::first(3);
        assert_eq!((first.since, first.start, first.epoch.clone()), (3, 3, None));
        assert_eq!(first.query(), "since=3&start=3");
        let next = first.with_epoch(Some("e-1".into())).continued(503);
        assert_eq!((next.since, next.start), (503, 3));
        assert_eq!(next.query(), "since=503&start=3&epoch=e-1");
    }

    #[test]
    fn a_pulled_page_is_read_with_its_tombstones_and_its_unreadable_records() {
        let page = WorkPage::parse(
            r#"{"epoch":"e-1","rev":9,"more":true,"records":[
                {"kind":"street","key":"w1~n2~n3","rev":7,"edited_at":"2027-01-15T08:00:00.000Z","deleted":false,"body":{"street":{"a":1},"future":[1]}},
                {"kind":"rule","key":"r-1","rev":8,"edited_at":"2027-01-15T08:00:01.000Z","deleted":true,"body":null},
                {"kind":"street","key":"w9","rev":9,"edited_at":"2027-01-15T08:00:02.000Z","deleted":false,"body":null,"error":"corrupt"}]}"#,
        )
        .unwrap();
        assert_eq!((page.epoch.as_str(), page.rev, page.more, page.records.len()), ("e-1", 9, true, 3));
        assert_eq!(page.records[0].body.as_ref().unwrap()["future"][0], 1);
        assert!(page.records[1].deleted && page.records[1].body.is_none());
        assert_eq!(page.records[2].error.as_deref(), Some("corrupt"));
        assert_eq!(WorkPage::parse("<html>").unwrap_err(), WorkError::Unavailable);
    }

    #[test]
    fn a_push_answer_is_read_and_a_put_is_written_with_null_for_a_deletion() {
        let pushed = WorkPushed::parse(
            r#"{"rev":4,"results":[
                {"kind":"street","key":"a","status":"accepted","rev":4},
                {"kind":"rule","key":"b","status":"superseded","rev":2,"record":{"kind":"rule","key":"b","rev":2,"edited_at":"2027-01-15T08:00:00.000Z","deleted":false,"body":{"name":"X"}}}]}"#,
        )
        .unwrap();
        assert_eq!((pushed.rev, pushed.results[0].status, pushed.results[1].status), (4, WorkStatus::Accepted, WorkStatus::Superseded));
        assert!(pushed.results[0].record.is_none());
        assert_eq!(pushed.results[1].record.as_ref().unwrap().body.as_ref().unwrap()["name"], "X");
        let put = WorkPut { kind: "rule".into(), key: "b".into(), base_rev: 2, edited_at: "2027-01-15T08:00:00.000Z".into(), deleted: true, body: None };
        assert_eq!(serde_json::to_value(&put).unwrap()["body"], serde_json::Value::Null);
    }

    #[test]
    fn the_fake_work_sync_keeps_calls_until_answered_by_kind_oldest_first() {
        let work = FakeWorkSync::default();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let s = seen.clone();
        work.pull(WorkAsk::first(3), Box::new(move |r| s.borrow_mut().push(format!("pull {:?}", r.map(|p| p.rev)))));
        let s = seen.clone();
        work.push(vec![], true, Box::new(move |r| s.borrow_mut().push(format!("push {:?}", r.map(|p| p.rev)))));
        let s = seen.clone();
        work.clear(Box::new(move |r| s.borrow_mut().push(format!("clear {r:?}"))));
        assert_eq!(work.calls(), [WorkCall::Pull(WorkAsk::first(3)), WorkCall::Push { batch: vec![], keepalive: true }, WorkCall::Clear]);
        assert!(work.answer(WorkReply::Pushed(Ok(WorkPushed { rev: 1, results: vec![] }))));
        assert!(!work.answer(WorkReply::Pushed(Ok(WorkPushed { rev: 1, results: vec![] }))), "none left of that kind");
        assert!(work.answer(WorkReply::Page(Err(WorkError::Resync))));
        assert!(work.answer(WorkReply::Cleared(Ok(()))));
        assert_eq!(*seen.borrow(), ["push Ok(1)", "pull Err(Resync)", "clear Ok(())"]);
    }

    #[test]
    fn memory_storage_tells_the_time_it_is_given_and_has_no_randomness_unless_asked() {
        let s = MemoryStorage::default();
        assert_eq!((s.now_ms(), s.random_hex()), (0, None));
        s.set_now(1_800_000_000_000);
        s.tick(5);
        assert_eq!(s.now_ms(), 1_800_000_000_005);
        s.with_randomness(true);
        let (a, b) = (s.random_hex().unwrap(), s.random_hex().unwrap());
        assert_eq!((a.len(), b.len()), (32, 32));
        assert_ne!(a, b);
        let other = MemoryStorage::default();
        other.with_randomness(true);
        assert_ne!(other.random_hex().unwrap(), a, "two devices never draw the same digits");
    }

    #[test]
    fn a_memory_storage_has_an_identity_of_its_own_that_every_handle_to_it_shares() {
        let one: Rc<dyn Storage> = Rc::new(MemoryStorage::default());
        let same = one.clone();
        let other: Rc<dyn Storage> = Rc::new(MemoryStorage::default());
        assert_eq!(one.identity(), same.identity());
        assert_ne!(one.identity(), other.identity());
        assert!(one.identity() > 0 && other.identity() > 0, "0 is the browser's");
    }

    #[test]
    fn the_fake_mapper_records_the_zone_ticks() {
        let fake = FakeMapper::default();
        assert_eq!(fake.zone_ticks(), (String::new(), String::new()));
        fake.set_zone_ticks("[3]", "{3}");
        assert_eq!(fake.zone_ticks(), ("[3]".into(), "{3}".into()));
        assert!(fake.take().is_empty(), "the ticks are not among the calls the other tests list");
    }
}
