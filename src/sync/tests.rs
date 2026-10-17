//! The sync view-model against a stand-in for the server that keeps the server's rules (revisions, accept or
//! supersede, tombstones, a floor for 410), and against the manual fake for the order of calls.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::{Value, json};

use crate::city::edits::EDITS_KEY;
use crate::city::store::CityStore;
use crate::restriction::model::Rule;
use crate::restriction::store::RestrictionStore;
use crate::shared::i18n::{Args, Locale};
use crate::shared::ports::{
    FakeWorkSync, ManualScheduler, Me, MemoryStorage, Ports, RecordingAnnouncer, Storage, WorkAnswer, WorkAsk, WorkCall, WorkError, WorkPage, WorkPushed,
    WorkPut, WorkRecord, WorkReply, WorkResult, WorkStatus, WorkSync, test_ports_with_time,
};
use crate::sync::model::{self, Change, Id, Kind, Origin, Report};
use crate::sync::vm::{Status, SyncVm};
use crate::sync::{RESOURCES, hub, local, text};

#[derive(Clone)]
struct Row {
    rev: i64,
    edited_at: String,
    deleted: bool,
    body: Option<Value>,
    /// The server could not open the body: it comes with `error: "corrupt"` and none.
    corrupt: bool,
}

/// The server's rules in memory: one account.
struct ServerSim {
    rows: RefCell<BTreeMap<(String, String), Row>>,
    rev: Cell<i64>,
    floor: Cell<i64>,
    storing: Cell<bool>,
    down: Cell<bool>,
    /// The copy of the account's work: another one after `cycle`.
    epoch: RefCell<String>,
    /// The server's clock, as a stamp; None means no time is ahead of it.
    now: RefCell<Option<String>>,
    calls: RefCell<Vec<String>>,
    /// The account the session cookie in this browser belongs to, as a pull says it; None says nothing.
    user: RefCell<Option<String>>,
}

impl ServerSim {
    fn new() -> Rc<ServerSim> {
        Rc::new(ServerSim {
            rows: RefCell::new(BTreeMap::new()),
            rev: Cell::new(0),
            floor: Cell::new(0),
            storing: Cell::new(true),
            down: Cell::new(false),
            epoch: RefCell::new("e1".into()),
            now: RefCell::new(None),
            calls: RefCell::new(Vec::new()),
            user: RefCell::new(None),
        })
    }

    /// Another device turned storage off and on again: the copy is a new one, with its counter back at 0.
    fn cycle(&self) {
        self.rows.borrow_mut().clear();
        self.rev.set(0);
        self.floor.set(0);
        let next = format!("{}+", self.epoch.borrow());
        *self.epoch.borrow_mut() = next;
    }

    fn record(key: &(String, String), row: &Row) -> WorkRecord {
        WorkRecord {
            kind: key.0.clone(),
            key: key.1.clone(),
            rev: row.rev,
            edited_at: row.edited_at.clone(),
            deleted: row.deleted,
            body: row.body.clone(),
            error: row.corrupt.then(|| "corrupt".to_string()),
        }
    }

    /// The live records as `kind/key`.
    fn live(&self) -> Vec<String> {
        self.rows.borrow().iter().filter(|(_, r)| !r.deleted).map(|((k, key), _)| format!("{k}/{key}")).collect()
    }

    fn tombstones(&self) -> usize {
        self.rows.borrow().values().filter(|r| r.deleted).count()
    }

    fn row(&self, kind: &str, key: &str) -> Option<Row> {
        self.rows.borrow().get(&(kind.to_string(), key.to_string())).cloned()
    }

    fn seed(&self, kind: &str, key: &str, body: Value, at: &str) {
        self.rev.set(self.rev.get() + 1);
        self.rows
            .borrow_mut()
            .insert((kind.into(), key.into()), Row { rev: self.rev.get(), edited_at: at.into(), deleted: false, body: Some(body), corrupt: false });
    }

    /// A record whose body the server cannot open (a key that was lost).
    fn seed_corrupt(&self, kind: &str, key: &str, at: &str) {
        self.rev.set(self.rev.get() + 1);
        self.rows.borrow_mut().insert((kind.into(), key.into()), Row { rev: self.rev.get(), edited_at: at.into(), deleted: false, body: None, corrupt: true });
    }

    /// A tombstone that was purged: the row is gone and a device from before it must resync.
    fn purge(&self, kind: &str, key: &str) {
        self.rows.borrow_mut().remove(&(kind.to_string(), key.to_string()));
        self.rev.set(self.rev.get() + 1);
        self.floor.set(self.rev.get());
    }

    fn pulled(&self, ask: WorkAsk) -> Result<WorkPage, WorkError> {
        if self.down.get() {
            return Err(WorkError::Unavailable);
        }
        if !self.storing.get() {
            return Err(WorkError::StorageOff);
        }
        if ask.epoch.as_ref().is_some_and(|epoch| *epoch != *self.epoch.borrow()) {
            return Err(WorkError::Reset);
        }
        if (ask.start > 0 && ask.start < self.floor.get()) || ask.since > self.rev.get() {
            return Err(WorkError::Resync);
        }
        let mut records: Vec<WorkRecord> =
            self.rows.borrow().iter().filter(|(_, r)| r.rev > ask.since && (!r.deleted || ask.start > 0)).map(|(k, r)| ServerSim::record(k, r)).collect();
        records.sort_by_key(|r| r.rev);
        let more = records.len() > 500;
        records.truncate(500);
        let rev = if more { records.last().map_or(self.rev.get(), |r| r.rev) } else { self.rev.get() };
        Ok(WorkPage { epoch: self.epoch.borrow().clone(), rev, more, records, user: self.user.borrow().clone() })
    }

    fn pushed(&self, batch: Vec<WorkPut>) -> Result<WorkPushed, WorkError> {
        if self.down.get() {
            return Err(WorkError::Unavailable);
        }
        if !self.storing.get() {
            return Err(WorkError::StorageOff);
        }
        let mut results = Vec::new();
        for put in batch {
            let key = (put.kind.clone(), put.key.clone());
            let existing = self.rows.borrow().get(&key).cloned();
            // a time ahead of the server's clock counts as the clock
            let at = match &*self.now.borrow() {
                Some(now) if put.edited_at > *now => now.clone(),
                _ => put.edited_at.clone(),
            };
            let accept = existing.as_ref().is_none_or(|row| row.rev == put.base_rev || at > row.edited_at);
            if accept {
                self.rev.set(self.rev.get() + 1);
                let row = Row { rev: self.rev.get(), edited_at: at, deleted: put.deleted, body: put.body.clone(), corrupt: false };
                self.rows.borrow_mut().insert(key, row.clone());
                results.push(WorkResult { kind: put.kind, key: put.key, status: WorkStatus::Accepted, rev: row.rev, record: None });
            } else {
                let row = existing.expect("superseded means there is a row");
                results.push(WorkResult {
                    kind: put.kind,
                    key: put.key,
                    status: WorkStatus::Superseded,
                    rev: row.rev,
                    record: Some(ServerSim::record(&key, &row)),
                });
            }
        }
        Ok(WorkPushed { rev: self.rev.get(), results })
    }
}

impl WorkSync for ServerSim {
    fn pull(&self, ask: WorkAsk, done: WorkAnswer<WorkPage>) {
        let call = if ask.since == ask.start { format!("pull {}", ask.since) } else { format!("pull {} from {}", ask.since, ask.start) };
        self.calls.borrow_mut().push(call);
        done(self.pulled(ask));
    }
    fn push(&self, batch: Vec<WorkPut>, keepalive: bool, done: WorkAnswer<WorkPushed>) {
        self.calls.borrow_mut().push(format!("push {} keepalive={keepalive}", batch.len()));
        done(self.pushed(batch));
    }
    fn clear(&self, done: WorkAnswer<()>) {
        self.calls.borrow_mut().push("clear".into());
        self.rows.borrow_mut().clear();
        self.rev.set(0);
        self.floor.set(0);
        done(Ok(()));
    }
}

/// A page of the server's answer with nothing in it.
fn page(rev: i64) -> WorkPage {
    WorkPage { epoch: "e1".into(), rev, more: false, records: vec![], user: None }
}

/// A browser: its storage, its clock, and a sync view-model talking to `server`.
struct Device {
    ports: Ports,
    storage: Rc<MemoryStorage>,
    scheduler: Rc<ManualScheduler>,
    announcer: Rc<RecordingAnnouncer>,
    vm: Rc<SyncVm>,
}

fn device(server: &Rc<ServerSim>, now: u64) -> Device {
    let (mut ports, announcer, storage, scheduler) = test_ports_with_time();
    storage.set_now(now);
    ports.work = server.clone();
    let vm = SyncVm::new(ports.clone(), crate::i18n_for(Locale::En));
    Device { ports, storage, scheduler, announcer, vm }
}

/// The time the tests start from: 2027-01-15T08:00:00Z.
const T0: u64 = 1_800_000_000_000;

fn ada() -> Me {
    Me { id: "ada".into(), display_name: "Ada".into(), store_on_server: Some(true), identities: vec![] }
}

/// Moves `by` mm from the first piece of street `edge` to the second and writes it, as the editor does. Gives the key.
fn edit_street(d: &Device, edge: u32, by: i32) -> String {
    let store = CityStore::sample(d.ports.storage.clone());
    let mut s = store.open().street_editor(edge, 0).unwrap().snapshot();
    s.segments[0].width_mm -= by;
    s.segments[1].width_mm += by;
    assert!(store.write_street(edge, s));
    store.open().street_record(edge).unwrap().0
}

fn width(d: &Device, edge: u32) -> i32 {
    CityStore::sample(d.ports.storage.clone()).open().street_editor(edge, 0).unwrap().snapshot().segments[0].width_mm
}

fn street_id(key: &str) -> Id {
    Id::new(Kind::Street, key)
}

fn record_of(d: &Device, id: &Id) -> Option<Value> {
    local::read(&d.ports.storage, id)
}

#[test]
fn the_ftl_files_have_the_same_messages_and_variables() {
    assert_eq!(crate::shared::i18n::tests::parity_problems(RESOURCES.en, RESOURCES.fr), Vec::<String>::new());
}

#[test]
fn enabling_merges_this_browsers_work_with_the_servers_and_counts_each_way() {
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    let key1 = edit_street(&laptop, 1, 100);
    laptop.vm.known(&ada());
    assert_eq!(server.live(), [format!("street/{key1}")]);
    // the phone worked signed out: street 1 earlier than the laptop did, street 2 on its own
    let phone = device(&server, T0 + 50_000);
    edit_street(&phone, 1, 150);
    phone.storage.set_now(T0 + 60_000);
    let key2 = edit_street(&phone, 2, 100);
    let report = Rc::new(Cell::new(Report::default()));
    let r = report.clone();
    phone.vm.enable("ada", Box::new(move |x| r.set(x)));
    assert_eq!((report.get().ok, report.get().sent, report.get().received), (true, 1, 1), "street 2 went up, street 1 came down");
    assert_eq!(width(&phone, 1), width(&laptop, 1), "the later edit won on the phone");
    assert_eq!(server.live().len(), 2);
    assert!(server.live().contains(&format!("street/{key2}")));
    assert_eq!(phone.vm.status(), Status::Synced);
}

#[test]
fn a_local_record_never_marked_counts_as_edited_when_storage_is_turned_on() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0 + 10_000);
    // work from before anything was marked (an older build's): written to the store with no mark
    let key = "w9~n1~n2";
    assert!(local::write(&d.ports.storage, &street_id(key), &json!({"old": true})));
    server.seed("street", key, json!({"newer": true}), "2027-01-15T07:00:00.000Z");
    d.vm.known(&ada());
    let row = server.row("street", key).unwrap();
    assert_eq!(row.body, Some(json!({"old": true})), "edited at the moment storage was turned on, so later than the server's");
    assert_eq!(row.edited_at, model::stamp(T0 + 10_000));
}

#[test]
fn an_edit_on_one_device_reaches_the_other_when_it_opens_and_the_editor_is_told() {
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    let key = edit_street(&laptop, 1, 100);
    laptop.vm.known(&ada());
    let told = Rc::new(RefCell::new(Vec::new()));
    let t = told.clone();
    hub::register(|| {}, move |changes: &[Change]| t.borrow_mut().extend(changes.iter().map(|c| (c.id.name(), c.origin))));
    let phone = device(&server, T0 + 200_000);
    phone.vm.known(&ada());
    assert_eq!(width(&phone, 1), width(&laptop, 1));
    assert_eq!(*told.borrow(), [(format!("street/{key}"), Origin::Remote)]);
}

#[test]
fn the_later_edit_wins_per_place_and_the_device_whose_change_lost_is_told() {
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    edit_street(&laptop, 1, 100);
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 110_000);
    phone.vm.known(&ada());
    // both edit the same place while apart: the phone first, the laptop later
    phone.storage.set_now(T0 + 120_000);
    edit_street(&phone, 1, 120);
    laptop.storage.set_now(T0 + 130_000);
    edit_street(&laptop, 1, 140);
    let told = Rc::new(RefCell::new(Vec::new()));
    let t = told.clone();
    hub::register(|| {}, move |changes: &[Change]| t.borrow_mut().extend(changes.iter().map(|c| c.origin)));
    laptop.vm.sync();
    phone.vm.sync();
    assert_eq!(width(&phone, 1), width(&laptop, 1), "both keep the later edit");
    assert_eq!(*told.borrow(), [Origin::Replaced], "the phone's own change lost, and it is told so");
}

#[test]
fn a_second_account_on_the_same_browser_starts_at_revision_zero_and_sends_no_deletion() {
    // a_second_account_on_the_same_browser_...: Review Focus 1
    hub::reset();
    let ada_server = ServerSim::new();
    let d = device(&ada_server, T0 + 100_000);
    let own = edit_street(&d, 1, 100);
    d.vm.known(&ada());
    ada_server.seed("street", "w5~n1~n2", json!({}), "2027-01-15T07:00:00.000Z");
    d.vm.refresh();
    assert_eq!(model::open(&*d.storage).unwrap().rev, 2, "Ada's revision");
    d.vm.stop();
    let bob_server = ServerSim::new();
    for key in ["w9~n1~n2", "w9~n2~n3", "w9~n3~n4"] {
        bob_server.seed("street", key, json!({"bob": key}), "2027-01-15T07:00:00.000Z");
    }
    let calls_before = ada_server.calls.borrow().len();
    let bobs = SyncVm::new(Ports { work: bob_server.clone(), ..d.ports.clone() }, crate::i18n_for(Locale::En));
    bobs.known(&Me { id: "bob".into(), ..ada() });
    assert_eq!(bob_server.calls.borrow()[0], "pull 0", "Ada's revision 2 is not Bob's");
    assert_eq!(bob_server.tombstones(), 0);
    assert!(bob_server.live().contains(&format!("street/{own}")), "what was in the browser is merged into the account that turns storage on");
    assert_eq!(bob_server.live().len(), 5, "Bob's three, the one Ada made here, and the one Ada's account sent here");
    assert_eq!(ada_server.calls.borrow().len(), calls_before, "nothing more went to Ada's account");
    let doc = model::open(&*d.storage).unwrap();
    assert_eq!(doc.user.as_deref(), Some("bob"));
}

#[test]
fn storage_off_from_the_server_stops_syncing_and_clears_the_state_but_keeps_the_work() {
    // storage_off_from_the_server_...: Review Focus 2
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    let key = edit_street(&d, 1, 100);
    d.vm.known(&ada());
    server.rows.borrow_mut().clear();
    server.storing.set(false); // another device turned storage off
    edit_street(&d, 1, 150);
    d.vm.refresh();
    assert_eq!(d.vm.status(), Status::Off);
    let doc = model::open(&*d.storage).unwrap();
    assert_eq!((doc.user, doc.epoch, doc.rev), (None, None, 0), "the server's numbers are forgotten");
    assert!(doc.records.values().all(|e| e.rev == 0), "and so are the records'");
    assert!(record_of(&d, &street_id(&key)).is_some(), "the browser's work stays");
    assert_eq!(server.live().len(), 0, "and nothing was resurrected on the server");
    assert_eq!(*server.calls.borrow().last().unwrap(), "pull 0", "no push followed the refusal");
}

#[test]
fn a_resync_removes_what_the_server_no_longer_lists_and_keeps_what_waits() {
    // a_resync_removes_...: Review Focus 3
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    let key1 = edit_street(&laptop, 1, 100);
    laptop.storage.set_now(T0 + 101_000);
    let key2 = edit_street(&laptop, 2, 100);
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 200_000);
    phone.vm.known(&ada());
    assert!(record_of(&phone, &street_id(&key1)).is_some());
    server.purge("street", &key1); // gone, with the tombstone nobody saw
    phone.storage.set_now(T0 + 300_000);
    let key3 = edit_street(&phone, 3, 100);
    let told = Rc::new(RefCell::new(Vec::new()));
    let t = told.clone();
    hub::register(|| {}, move |changes: &[Change]| t.borrow_mut().extend(changes.iter().map(|c| (c.id.name(), c.origin))));
    let before = server.calls.borrow().len();
    phone.vm.sync();
    assert_eq!(
        server.calls.borrow()[before..],
        ["pull 2", "pull 0", "push 1 keepalive=false"],
        "the stale pull is refused, the full one follows, then what waited"
    );
    assert_eq!(record_of(&phone, &street_id(&key1)), None, "a place the server no longer lists is not kept as a ghost");
    assert!(record_of(&phone, &street_id(&key2)).is_some());
    assert!(record_of(&phone, &street_id(&key3)).is_some(), "what waited is kept");
    assert!(server.live().contains(&format!("street/{key3}")), "and sent");
    assert_eq!(*told.borrow(), [(format!("street/{key1}"), Origin::Remote)]);
}

#[test]
fn a_newer_edits_document_pauses_the_streets_but_the_rules_still_sync_and_it_is_said_once() {
    // a_newer_edits_document_pauses_...: Review Focus 4
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    let newer = r#"{"version":99,"streets":{"w1":{"from":"the future"}}}"#;
    d.storage.remember(EDITS_KEY, newer);
    let ring = vec![[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]];
    RestrictionStore::new(d.ports.storage.clone()).add("Zone", Rule::CarFree, ring).unwrap();
    server.seed("street", "w2~n1~n2", json!({"theirs": true}), "2027-01-15T07:00:00.000Z");
    d.vm.known(&ada());
    assert_eq!(d.vm.status(), Status::Paused);
    assert_eq!(server.live(), ["rule/r1", "street/w2~n1~n2"], "the rule went up");
    assert_eq!(d.storage.recall(EDITS_KEY).as_deref(), Some(newer), "the newer document is neither read, applied to, nor replaced");
    let said = d.announcer.take();
    assert_eq!(said.iter().filter(|s| s.starts_with("Syncing is paused")).count(), 1, "{said:?}");
    d.vm.refresh();
    assert!(d.announcer.take().iter().all(|s| !s.starts_with("Syncing is paused")), "said once");
    assert_eq!(d.vm.status_text().as_deref(), Some("Syncing is paused"));
}

#[test]
fn the_echo_of_ones_own_record_is_not_announced_and_does_not_reload_an_editor() {
    // the_echo_of_ones_own_record_...: Review Focus 5
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    edit_street(&d, 1, 100);
    d.vm.known(&ada());
    let told = Rc::new(Cell::new(0));
    let t = told.clone();
    hub::register(|| {}, move |_: &[Change]| t.set(t.get() + 1));
    d.vm.refresh(); // pulls the record it just sent
    d.vm.refresh();
    assert_eq!(told.get(), 0);
    assert!(d.announcer.take().is_empty());
}

#[test]
fn an_unsaved_edit_is_flushed_first_and_goes_in_the_same_run() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    let store = CityStore::sample(d.ports.storage.clone());
    hub::register(
        move || {
            let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
            s.segments[0].width_mm -= 100;
            s.segments[1].width_mm += 100;
            store.write_street(1, s);
        },
        |_: &[Change]| {},
    );
    d.vm.known(&ada());
    assert_eq!(server.live().len(), 1, "what the editor held was written, marked and sent by the sync that asked for it");
}

#[test]
fn while_offline_changes_stay_marked_and_go_when_the_network_returns() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    server.down.set(true);
    edit_street(&d, 1, 100);
    d.vm.known(&ada());
    assert_eq!(d.vm.status(), Status::NotSynced);
    assert_eq!(d.vm.status_text().as_deref(), Some("Not synced yet"));
    assert_eq!(server.live().len(), 0);
    assert!(model::open(&*d.storage).unwrap().has_pending(&|_| true));
    server.down.set(false);
    d.vm.refresh(); // the tab shows again, or the network returns
    assert_eq!((d.vm.status(), server.live().len()), (Status::Synced, 1));
    assert_eq!(d.vm.status_text().as_deref(), Some("Synced"));
}

#[test]
fn a_write_is_synced_after_the_debounce_and_a_burst_is_one_run_at_most_every_two_seconds() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    d.vm.known(&ada());
    d.storage.tick(10_000); // well past the last run
    let before = server.calls.borrow().len();
    edit_street(&d, 1, 100);
    edit_street(&d, 1, 50);
    edit_street(&d, 1, 25);
    assert_eq!(d.scheduler.pending(), 1, "a burst of writes waits for one run");
    assert_eq!(d.vm.status(), Status::NotSynced);
    d.scheduler.advance(249);
    assert_eq!(server.calls.borrow().len(), before, "not before the debounce");
    d.scheduler.advance(1);
    assert_eq!(server.live().len(), 1);
    assert_eq!(d.vm.status(), Status::Synced);
    // the next write comes at once: it waits out the two seconds between runs
    let after = server.calls.borrow().len();
    edit_street(&d, 2, 100);
    d.scheduler.advance(1_000);
    assert_eq!(server.calls.borrow().len(), after);
    d.scheduler.advance(1_000);
    assert_eq!(server.live().len(), 2);
}

#[test]
fn leaving_the_page_pushes_what_waits_with_keepalive_and_pulls_nothing() {
    hub::reset();
    let (mut ports, _, storage, scheduler) = test_ports_with_time();
    let work = Rc::new(FakeWorkSync::default());
    ports.work = work.clone();
    storage.set_now(T0);
    let vm = SyncVm::new(ports.clone(), crate::i18n_for(Locale::En));
    vm.known(&ada());
    assert_eq!(work.calls(), [WorkCall::Pull(WorkAsk::first(0))]);
    assert!(work.answer(WorkReply::Page(Ok(page(0)))));
    let store = CityStore::sample(ports.storage.clone());
    let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
    s.segments[0].width_mm -= 100;
    s.segments[1].width_mm += 100;
    assert!(store.write_street(1, s));
    assert_eq!(scheduler.pending(), 1);
    vm.leave();
    assert_eq!(scheduler.pending(), 0, "the debounced run is cancelled: the page is going");
    match work.calls().as_slice() {
        [WorkCall::Push { batch, keepalive: true }] => assert_eq!((batch.len(), batch[0].kind.as_str()), (1, "street")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_413_is_retried_one_record_at_a_time_and_the_refused_record_is_blocked_and_said_once() {
    hub::reset();
    let (mut ports, announcer, storage, _) = test_ports_with_time();
    let work = Rc::new(FakeWorkSync::default());
    ports.work = work.clone();
    storage.set_now(T0);
    let store = CityStore::sample(ports.storage.clone());
    for edge in [1, 2] {
        let mut s = store.open().street_editor(edge, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(store.write_street(edge, s));
    }
    let vm = SyncVm::new(ports.clone(), crate::i18n_for(Locale::En));
    vm.known(&ada());
    assert!(work.answer(WorkReply::Page(Ok(page(0)))));
    let sent = |work: &FakeWorkSync| match work.calls().as_slice() {
        [WorkCall::Push { batch, .. }] => batch.iter().map(|p| p.key.clone()).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    let both = sent(&work);
    assert_eq!(both.len(), 2);
    // the server refuses the batch for its size: the same records go again, one at a time, to find the one
    assert!(work.answer(WorkReply::Pushed(Err(WorkError::TooLarge("batch".into())))));
    assert_eq!(sent(&work), [both[0].clone()]);
    // the first is too big: it stays on this device, and what follows leaves it out
    assert!(work.answer(WorkReply::Pushed(Err(WorkError::TooLarge("record".into())))));
    assert_eq!(sent(&work), [both[1].clone()]);
    let taken = WorkResult { kind: "street".into(), key: both[1].clone(), status: WorkStatus::Accepted, rev: 1, record: None };
    assert!(work.answer(WorkReply::Pushed(Ok(WorkPushed { rev: 1, results: vec![taken] }))));
    assert_eq!(vm.status(), Status::NotSynced, "one place is still only on this device");
    let doc = model::open(&*storage).unwrap();
    let refused: Vec<String> = doc.records.iter().filter(|(_, e)| e.blocked).map(|(name, _)| name.clone()).collect();
    assert_eq!(refused, [format!("street/{}", both[0])]);
    assert!(doc.entry(&street_id(&both[1])).is_some_and(|e| !e.pending), "the other one was taken");
    let too_big = |announcer: &RecordingAnnouncer| announcer.take().into_iter().filter(|s| s.contains("too big")).collect::<Vec<_>>();
    assert_eq!(too_big(&announcer), ["A place is too big for the server (64 KB), so it stays on this device only."]);
    // the next run sends nothing of it and does not say it again
    vm.refresh();
    assert!(work.answer(WorkReply::Page(Ok(page(0)))));
    assert!(work.calls().is_empty(), "nothing is sent: one record is blocked, the other was taken");
    assert!(too_big(&announcer).is_empty());
}

/// Two places edited in a browser whose server answers with a fake the test drives; the first pull answered empty.
fn two_waiting_places() -> (Rc<FakeWorkSync>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>, Rc<ManualScheduler>, Rc<SyncVm>, Ports) {
    hub::reset();
    let (mut ports, announcer, storage, scheduler) = test_ports_with_time();
    let work = Rc::new(FakeWorkSync::default());
    ports.work = work.clone();
    storage.set_now(T0);
    let store = CityStore::sample(ports.storage.clone());
    for edge in [1, 2] {
        let mut s = store.open().street_editor(edge, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(store.write_street(edge, s));
    }
    let vm = SyncVm::new(ports.clone(), crate::i18n_for(Locale::En));
    vm.known(&ada());
    assert!(work.answer(WorkReply::Page(Ok(page(0)))));
    (work, announcer, storage, scheduler, vm, ports)
}

#[test]
fn a_batch_refused_over_a_record_limit_goes_one_at_a_time_and_the_refused_one_is_blocked_and_said_once() {
    let (work, announcer, storage, ..) = two_waiting_places();
    assert!(work.answer(WorkReply::Pushed(Err(WorkError::TooLarge("batch".into())))));
    let first = match work.calls().as_slice() {
        [WorkCall::Push { batch, .. }] => batch.iter().map(|p| p.key.clone()).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(first.len(), 1, "one at a time now");
    assert!(work.answer(WorkReply::Pushed(Err(WorkError::TooLarge("record".into())))));
    let doc = model::open(&*storage).unwrap();
    assert!(doc.entry(&street_id(&first[0])).is_some_and(|e| e.blocked));
    assert_eq!(announcer.take().iter().filter(|s| s.contains("too big")).count(), 1);
    assert!(work.answer(WorkReply::Pushed(Ok(WorkPushed { rev: 1, results: vec![] }))), "the other record goes next");
}

#[test]
fn an_account_wide_limit_blocks_nothing_stops_pushing_for_the_page_and_is_said_once() {
    for (limit, said) in [
        ("records", "The server keeps at most 5,000 records per account, so the rest stays on this device only."),
        ("bytes", "The server keeps at most 2 MB of work per account, so the rest stays on this device only."),
    ] {
        let (work, announcer, storage, scheduler, vm, ports) = two_waiting_places();
        assert!(work.answer(WorkReply::Pushed(Err(WorkError::TooLarge(limit.into())))));
        assert!(work.calls().is_empty(), "{limit}: no retry, one at a time or otherwise");
        let doc = model::open(&*storage).unwrap();
        assert!(doc.records.values().all(|e| !e.blocked && e.pending), "{limit}: nothing is blocked, both still wait");
        assert_eq!(vm.status(), Status::NotSynced);
        assert_eq!(announcer.take(), [said]);
        // a later write and a tab shown again pull but never push again, and say nothing more
        let store = CityStore::sample(ports.storage.clone());
        let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 50;
        s.segments[1].width_mm += 50;
        assert!(store.write_street(1, s));
        scheduler.advance(10_000);
        vm.refresh();
        while work.answer(WorkReply::Page(Ok(page(0)))) {}
        assert!(work.calls().is_empty(), "{limit}: {:?}", work.calls());
        assert!(announcer.take().is_empty(), "{limit}: said once");
        assert_eq!(vm.status(), Status::NotSynced);
        // storage turned on again on this page (or the next page open, which is a new view-model) tries again
        vm.enable("ada", Box::new(|_| {}));
        assert!(work.answer(WorkReply::Page(Ok(page(0)))));
        assert!(matches!(work.calls().as_slice(), [WorkCall::Push { .. }]), "{limit}");
    }
}

#[test]
fn a_pull_of_several_pages_is_not_started_over_by_a_tombstone_purged_meanwhile() {
    hub::reset();
    let server = ServerSim::new();
    for i in 0..600 {
        server.seed("street", &format!("w{i}~n1~n2"), json!({"n": i}), "2027-01-15T07:00:00.000Z");
    }
    server.seed("rule", "old", json!({}), "2027-01-15T07:00:00.000Z");
    server.purge("rule", "old"); // its tombstone was purged: the floor (602) is above every revision the pages go on from
    let d = device(&server, T0);
    d.vm.known(&ada());
    assert_eq!(*server.calls.borrow(), ["pull 0", "pull 500 from 0"], "the second page is judged by where the pull began");
    assert_eq!(model::open(&*d.storage).unwrap().rev, 602);
    assert_eq!(local::ids(&d.ports.storage).len(), 600);
}

#[test]
fn a_run_that_skipped_a_record_does_not_move_the_revision_on_so_the_record_comes_back() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    d.storage.remember(EDITS_KEY, r#"{"version":99,"streets":{}}"#);
    server.seed("street", "w2~n1~n2", json!({"theirs": true}), "2027-01-15T07:00:00.000Z");
    d.vm.known(&ada());
    assert_eq!(d.vm.status(), Status::Paused);
    assert_eq!(model::open(&*d.storage).unwrap().rev, 0, "the street was skipped, so the revision stays where it was");
    // the document can be read again
    d.storage.forget(EDITS_KEY);
    d.vm.refresh();
    assert!(record_of(&d, &street_id("w2~n1~n2")).is_some(), "it comes back with the next run");
    assert_eq!((d.vm.status(), model::open(&*d.storage).unwrap().rev), (Status::Synced, 1));
}

#[test]
fn a_device_that_missed_an_off_and_on_again_merges_keeps_its_own_records_and_removes_nothing() {
    // a_device_that_missed_an_off_and_on_...: Review Focus 6
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    let mine = edit_street(&laptop, 1, 100);
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 200_000);
    phone.vm.known(&ada());
    assert!(record_of(&phone, &street_id(&mine)).is_some());
    // another device turns storage off and on again and makes a place: the counter is at 1 again, as the phone's revision is
    server.cycle();
    let tablet = device(&server, T0 + 300_000);
    let theirs = edit_street(&tablet, 2, 100);
    tablet.vm.known(&ada());
    assert_eq!(server.live(), [format!("street/{theirs}")]);
    let told = Rc::new(RefCell::new(Vec::new()));
    let t = told.clone();
    hub::register(|| {}, move |changes: &[Change]| t.borrow_mut().extend(changes.iter().map(|c| (c.id.name(), c.origin))));
    let before = server.calls.borrow().len();
    phone.vm.sync();
    assert_eq!(
        server.calls.borrow()[before..],
        ["pull 1", "pull 0", "push 1 keepalive=false"],
        "reset, the whole new copy, then what the phone has and it lacks"
    );
    assert!(record_of(&phone, &street_id(&mine)).is_some(), "what the phone had is kept ...");
    assert!(record_of(&phone, &street_id(&theirs)).is_some(), "... and what the other device made arrives");
    assert_eq!(server.live().len(), 2, "the phone's record is sent to the new copy");
    assert_eq!(*told.borrow(), [(format!("street/{theirs}"), Origin::Remote)], "nothing the phone had was removed or replaced");
    assert_eq!(phone.vm.status(), Status::Synced);
}

#[test]
fn the_server_stand_in_decides_as_the_shared_truth_table_says() {
    // The file the server's own unit test reads (cityloom_server/src/work.rs): the stand-in cannot drift from the rule.
    let cases: Vec<Value> = serde_json::from_str(include_str!("../../cityloom_server/tests/data/accept_rules.json")).unwrap();
    assert!(cases.len() >= 7, "the table is not empty");
    let secs = |v: &Value| model::stamp(T0 + v.as_i64().unwrap() as u64 * 1000);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let server = ServerSim::new();
        if let Some(row) = case["row"].as_object() {
            let rev = row["rev"].as_i64().unwrap();
            server.rev.set(rev);
            server.rows.borrow_mut().insert(
                ("street".into(), "a".into()),
                Row { rev, edited_at: secs(&row["edited_at"]), deleted: false, body: Some(json!({"was": true})), corrupt: false },
            );
        }
        *server.now.borrow_mut() = Some(secs(&case["now"]));
        let put = WorkPut {
            kind: "street".into(),
            key: "a".into(),
            base_rev: case["put"]["base_rev"].as_i64().unwrap(),
            edited_at: secs(&case["put"]["edited_at"]),
            deleted: false,
            body: Some(json!({"is": true})),
        };
        let got = server.pushed(vec![put]).unwrap();
        let verdict = if got.results[0].status == WorkStatus::Accepted { "accepted" } else { "superseded" };
        assert_eq!((verdict, got.results[0].rev), (case["verdict"].as_str().unwrap(), case["rev"].as_i64().unwrap()), "{name}");
        assert_eq!(server.row("street", "a").unwrap().edited_at, secs(&case["at"]), "{name}");
    }
}

#[test]
fn a_failed_apply_does_not_advance_the_revision_and_nothing_is_sent_over_it() {
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    edit_street(&laptop, 1, 100);
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 200_000);
    phone.storage.refuse(EDITS_KEY); // the browser's storage is full
    phone.vm.known(&ada());
    assert_eq!(phone.vm.status(), Status::NotSynced);
    assert_eq!(model::open(&*phone.storage).unwrap().rev, 0, "it will pull that page again");
    assert_eq!(server.calls.borrow().iter().filter(|c| c.starts_with("push") && c.contains("keepalive")).count(), 1, "only the laptop's push");
    phone.storage.refuse_nothing();
    phone.vm.refresh();
    assert_eq!(width(&phone, 1), width(&laptop, 1));
}

#[test]
fn rules_made_before_syncing_are_renamed_once_so_two_devices_r1_never_meet() {
    hub::reset();
    let server = ServerSim::new();
    let ring = vec![[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]];
    let laptop = device(&server, T0 + 1_000);
    let phone = device(&server, T0 + 2_000);
    for (d, name) in [(&laptop, "Laptop"), (&phone, "Phone")] {
        let r = RestrictionStore::new(d.ports.storage.clone()).add(name, Rule::CarFree, ring.clone()).unwrap();
        assert_eq!(r.id, "r1", "both made their first rule before syncing");
        d.storage.with_randomness(true);
    }
    laptop.vm.known(&ada());
    phone.vm.known(&ada());
    laptop.vm.refresh();
    let live = server.live();
    assert_eq!(live.len(), 2, "{live:?}: two rules, not one");
    assert!(live.iter().all(|k| k.starts_with("rule/r-")), "{live:?}");
    for d in [&laptop, &phone] {
        let names: Vec<String> = RestrictionStore::new(d.ports.storage.clone()).all().into_iter().map(|r| r.name).collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(RestrictionStore::new(d.ports.storage.clone()).raw("r1").is_none());
    }
}

#[test]
fn a_rule_removed_on_one_device_is_removed_on_the_other() {
    hub::reset();
    let server = ServerSim::new();
    let ring = vec![[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]];
    let laptop = device(&server, T0 + 1_000);
    laptop.storage.with_randomness(true);
    let rule = RestrictionStore::new(laptop.ports.storage.clone()).add("Zone", Rule::CarFree, ring).unwrap();
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 2_000);
    phone.vm.known(&ada());
    assert_eq!(RestrictionStore::new(phone.ports.storage.clone()).all().len(), 1);
    laptop.storage.set_now(T0 + 3_000);
    assert!(RestrictionStore::new(laptop.ports.storage.clone()).remove(&rule.id));
    laptop.vm.refresh();
    assert_eq!(server.tombstones(), 1);
    phone.vm.refresh();
    assert_eq!(RestrictionStore::new(phone.ports.storage.clone()).all().len(), 0);
}

#[test]
fn signing_out_stops_syncing_but_keeps_the_marks_and_disabling_clears_them() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    d.vm.known(&ada());
    d.vm.stop();
    assert_eq!(d.vm.status(), Status::Off);
    edit_street(&d, 1, 100); // signed out: marked, not sent
    assert_eq!(d.scheduler.pending(), 0);
    assert_eq!(server.live().len(), 0);
    assert!(model::open(&*d.storage).unwrap().has_pending(&|_| true));
    d.vm.known(&ada()); // signs in again
    assert_eq!(server.live().len(), 1, "the work made signed out goes up");
    d.vm.disable();
    let doc = model::open(&*d.storage).unwrap();
    assert_eq!((doc.user.as_deref(), doc.rev, doc.epoch.as_deref()), (None, 0, None), "the server's numbers are forgotten");
    assert_eq!(d.vm.status_text(), None);
}

#[test]
fn stopping_releases_a_request_queued_behind_a_run_that_is_going() {
    use crate::shared::ports::test_ports_full;
    hub::reset();
    let t = test_ports_full();
    let vm = SyncVm::new(t.ports.clone(), crate::i18n_for(Locale::En));
    vm.known(&ada()); // a run starts and waits for the server
    let told = Rc::new(Cell::new(false));
    let c = told.clone();
    vm.enable("ada", Box::new(move |_| c.set(true))); // queued behind that run
    assert!(!told.get(), "still waiting for the next run");
    vm.stop();
    assert!(told.get(), "signing out does not leave the Account page waiting for ever");
}

#[test]
fn a_person_who_never_chose_server_storage_is_never_synced() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    edit_street(&d, 1, 100);
    for choice in [None, Some(false)] {
        d.vm.known(&Me { store_on_server: choice, ..ada() });
        assert_eq!(d.vm.status(), Status::Off);
    }
    assert!(server.calls.borrow().is_empty());
}

#[test]
fn the_status_and_the_announcements_are_said_in_both_languages() {
    let en = crate::i18n_for(Locale::En);
    let fr = crate::i18n_for(Locale::FrCa);
    assert_eq!(text::status_text(&en, Status::Off), None);
    assert_eq!(text::status_text(&en, Status::Synced).as_deref(), Some("Synced"));
    assert_eq!(text::status_text(&fr, Status::NotSynced).as_deref(), Some("Pas encore synchronisé"));
    let merged = |i: &crate::shared::i18n::I18n, sent, received| i.tr_now("sync-merged", &Args::new().num("sent", sent).num("received", received));
    assert_eq!(merged(&en, 1, 0), "Your work was merged with your account: 1 place sent, 0 places received.");
    assert_eq!(merged(&en, 3, 2), "Your work was merged with your account: 3 places sent, 2 places received.");
    assert_eq!(merged(&fr, 0, 1), "Votre travail a été fusionné avec votre compte\u{a0}: 0 lieu envoyé, 1 lieu reçu.");
    assert!(en.missing().is_empty() && fr.missing().is_empty());
}

#[test]
fn a_record_the_server_superseded_is_published_so_an_open_editor_reloads_and_is_told_its_change_lost() {
    let (work, _, _, _, _, _) = two_waiting_places();
    let told = Rc::new(RefCell::new(Vec::new()));
    let t = told.clone();
    hub::register(|| {}, move |changes: &[Change]| t.borrow_mut().extend(changes.iter().map(|c| (c.id.name(), c.origin))));
    let batch = match work.calls().as_slice() {
        [WorkCall::Push { batch, .. }] => batch.clone(),
        other => panic!("{other:?}"),
    };
    let theirs = WorkRecord {
        kind: "street".into(),
        key: batch[0].key.clone(),
        rev: 5,
        edited_at: model::stamp(T0 + 60_000),
        deleted: false,
        body: Some(json!({"theirs": true})),
        error: None,
    };
    let results = vec![
        WorkResult { kind: "street".into(), key: batch[0].key.clone(), status: WorkStatus::Superseded, rev: 5, record: Some(theirs) },
        WorkResult { kind: "street".into(), key: batch[1].key.clone(), status: WorkStatus::Accepted, rev: 6, record: None },
    ];
    assert!(work.answer(WorkReply::Pushed(Ok(WorkPushed { rev: 6, results }))));
    assert_eq!(*told.borrow(), [(format!("street/{}", batch[0].key), Origin::Replaced)]);
}

#[test]
fn what_was_applied_before_a_store_refused_a_record_is_still_published() {
    hub::reset();
    let server = ServerSim::new();
    let ring = vec![[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]];
    let laptop = device(&server, T0 + 100_000);
    edit_street(&laptop, 1, 100);
    RestrictionStore::new(laptop.ports.storage.clone()).add("Zone", Rule::CarFree, ring).unwrap();
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 200_000);
    phone.storage.refuse(EDITS_KEY); // the street cannot be kept, the rule can
    let told = Rc::new(RefCell::new(Vec::new()));
    let t = told.clone();
    hub::register(|| {}, move |changes: &[Change]| t.borrow_mut().extend(changes.iter().map(|c| c.id.name())));
    phone.vm.known(&ada());
    assert_eq!(*told.borrow(), ["rule/r1"]);
}

#[test]
fn a_batch_413_does_not_send_the_rest_of_the_backlog_one_record_per_call() {
    hub::reset();
    let (mut ports, _, storage, _) = test_ports_with_time();
    let work = Rc::new(FakeWorkSync::default());
    ports.work = work.clone();
    storage.set_now(T0);
    let store = CityStore::sample(ports.storage.clone());
    for edge in [1, 2, 3] {
        let mut s = store.open().street_editor(edge, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(store.write_street(edge, s));
    }
    let vm = SyncVm::new(ports.clone(), crate::i18n_for(Locale::En));
    vm.known(&ada());
    assert!(work.answer(WorkReply::Page(Ok(page(0)))));
    assert!(work.answer(WorkReply::Pushed(Err(WorkError::TooLarge("batch".into())))));
    let one = match work.calls().as_slice() {
        [WorkCall::Push { batch, .. }] => batch.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(one.len(), 1);
    let taken = WorkResult { kind: "street".into(), key: one[0].key.clone(), status: WorkStatus::Accepted, rev: 1, record: None };
    assert!(work.answer(WorkReply::Pushed(Ok(WorkPushed { rev: 1, results: vec![taken] }))));
    match work.calls().as_slice() {
        [WorkCall::Push { batch, .. }] => assert_eq!(batch.len(), 2, "the rest goes together again"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn enabling_during_a_run_waits_for_the_run_it_triggers() {
    hub::reset();
    let (mut ports, _, storage, scheduler) = test_ports_with_time();
    let work = Rc::new(FakeWorkSync::default());
    ports.work = work.clone();
    storage.set_now(T0);
    let vm = SyncVm::new(ports.clone(), crate::i18n_for(Locale::En));
    vm.known(&ada()); // a run is going: its pull is not answered yet
    let reports = Rc::new(RefCell::new(Vec::new()));
    let r = reports.clone();
    vm.enable("ada", Box::new(move |x| r.borrow_mut().push(x)));
    assert!(work.answer(WorkReply::Page(Ok(page(0)))));
    assert!(reports.borrow().is_empty(), "the run that was going is not the merge enable asked for");
    scheduler.advance(10_000);
    assert!(work.answer(WorkReply::Page(Ok(page(0)))));
    assert_eq!(reports.borrow().len(), 1);
}

#[test]
fn the_numbers_and_units_of_the_french_messages_do_not_break_across_lines() {
    let fr = crate::i18n_for(Locale::FrCa);
    assert_eq!(
        fr.tr_now("sync-too-large-bytes", &Args::new()),
        "Le serveur conserve au plus 2\u{a0}Mo de travail par compte; le reste demeure sur cet appareil seulement."
    );
    assert!(fr.tr_now("sync-too-large-records", &Args::new()).contains("5\u{a0}000\u{a0}enregistrements"));
    assert!(fr.tr_now("sync-too-large-record", &Args::new()).contains("64\u{a0}Ko"));
}

#[test]
fn a_device_that_saw_storage_go_off_keeps_its_edit_times_so_it_cannot_overwrite_a_later_edit_when_it_returns() {
    // T1: both devices have S; T2: storage off; T3: the laptop edits S; T4: storage on; T5: the phone returns.
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    let key = edit_street(&laptop, 1, 100);
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 110_000);
    phone.vm.known(&ada());
    assert_eq!(width(&phone, 1), width(&laptop, 1));
    // storage goes off; the phone's open page learns it from a 409
    server.rows.borrow_mut().clear();
    server.storing.set(false);
    phone.vm.refresh();
    assert_eq!(phone.vm.status(), Status::Off);
    laptop.vm.disable();
    // the laptop edits S with storage off
    laptop.storage.set_now(T0 + 300_000);
    edit_street(&laptop, 1, 200);
    let edited = width(&laptop, 1);
    // storage is turned on again, on the laptop
    server.storing.set(true);
    server.cycle();
    laptop.storage.set_now(T0 + 400_000);
    laptop.vm.enable("ada", Box::new(|_| {}));
    assert!(server.row("street", &key).unwrap().body.is_some());
    // the phone returns, later still
    phone.storage.set_now(T0 + 500_000);
    phone.vm.known(&ada());
    assert_eq!(width(&phone, 1), edited, "the laptop's later edit reached the phone");
    assert_eq!(width(&laptop, 1), edited, "and the phone's older copy did not overwrite it on the server");
    assert_eq!(server.row("street", &key).unwrap().edited_at, model::stamp(T0 + 300_000));
}

#[test]
fn a_damaged_sync_document_is_copied_aside_once_and_its_records_are_claimed_so_that_the_servers_copy_wins() {
    hub::reset();
    let server = ServerSim::new();
    let laptop = device(&server, T0 + 100_000);
    let key = edit_street(&laptop, 1, 100);
    laptop.vm.known(&ada());
    let phone = device(&server, T0 + 110_000);
    phone.vm.known(&ada());
    // the laptop edits again, later; the phone's sync document is damaged
    laptop.storage.set_now(T0 + 200_000);
    edit_street(&laptop, 1, 150);
    laptop.vm.refresh();
    let theirs = width(&laptop, 1);
    phone.storage.remember(model::SYNC_KEY, "{damaged");
    // work the server does not have: it goes up
    phone.storage.set_now(T0 + 300_000);
    let own = edit_street(&phone, 2, 100);
    phone.vm.refresh();
    assert_eq!(phone.storage.recall(model::DAMAGED_KEY).as_deref(), Some("{damaged"), "copied aside");
    assert_eq!(width(&phone, 1), theirs, "the server's later copy won over the phone's stale one");
    assert!(server.live().contains(&format!("street/{own}")), "what only the phone had went up");
    assert!(server.row("street", &key).unwrap().edited_at > model::stamp(T0 + 100_000));
}

#[test]
fn a_corrupt_record_in_a_pull_does_not_move_the_revision_past_it_and_comes_back_once_the_server_can_open_it() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    server.seed("street", "w1~n1~n2", json!({"fine": true}), "2027-01-15T07:00:00.000Z");
    server.seed_corrupt("street", "w2~n1~n2", "2027-01-15T07:00:01.000Z");
    d.vm.known(&ada());
    assert!(record_of(&d, &street_id("w1~n1~n2")).is_some(), "the readable record is applied");
    assert!(record_of(&d, &street_id("w2~n1~n2")).is_none());
    assert_eq!(model::open(&*d.storage).unwrap().rev, 0, "the revision stays before the record that was skipped");
    assert_eq!(server.calls.borrow().iter().filter(|c| c.starts_with("push")).count(), 0, "nothing is sent over it");
    // the server can open it now
    server.rows.borrow_mut().get_mut(&("street".to_string(), "w2~n1~n2".to_string())).map(|r| {
        r.corrupt = false;
        r.body = Some(json!({"repaired": true}));
    });
    d.vm.refresh();
    assert_eq!(record_of(&d, &street_id("w2~n1~n2")), Some(json!({"repaired": true})));
    assert_eq!(model::open(&*d.storage).unwrap().rev, 2);
}

#[test]
fn a_superseded_record_that_cannot_be_applied_is_not_sent_again_in_the_same_run() {
    hub::reset();
    let server = ServerSim::new();
    let d = device(&server, T0);
    let key = edit_street(&d, 1, 100); // marked at T0
    // the server holds a later edit of the place that it cannot open
    server.seed_corrupt("street", &key, "2027-01-15T09:00:00.000Z");
    d.vm.known(&ada());
    let pushes = server.calls.borrow().iter().filter(|c| c.starts_with("push")).count();
    assert!(pushes <= 1, "the same record was sent {pushes} times in one run");
    assert!(server.row("street", &key).unwrap().corrupt, "the server's record was not overwritten");
    assert_eq!(d.vm.status(), Status::NotSynced, "the place still waits");
    assert!(model::open(&*d.storage).unwrap().has_pending(&|_| true), "and stays marked");
}

#[test]
fn a_tab_that_still_believes_in_the_previous_account_does_not_sync_under_the_new_ones_cookie() {
    hub::reset();
    let server = ServerSim::new();
    *server.user.borrow_mut() = Some("bob".into()); // the browser's cookie is Bob's
    let tab2 = device(&server, T0 + 100_000);
    edit_street(&tab2, 1, 100);
    let bob = Me { id: "bob".into(), display_name: "Bob".into(), store_on_server: Some(true), identities: vec![] };
    tab2.vm.known(&bob);
    // tab 1 was open as Ada and is shown again
    let tab1 = SyncVm::new(tab2.ports.clone(), crate::i18n_for(Locale::En));
    let asked = Rc::new(Cell::new(0));
    let a = asked.clone();
    tab1.on_account_changed(Box::new(move || a.set(a.get() + 1)));
    let before = model::open(&*tab2.storage).unwrap();
    let calls = server.calls.borrow().len();
    tab1.known(&ada());
    assert_eq!(model::open(&*tab2.storage).unwrap(), before, "Ada's tab leaves the sync document as Bob's tab left it");
    assert_eq!(*server.calls.borrow().last().unwrap(), "pull 0", "it asked and stopped");
    assert_eq!(server.calls.borrow().len(), calls + 1, "and sent nothing");
    assert_eq!((asked.get(), tab1.status()), (1, Status::Off), "the page is told to read the session again");
    tab2.vm.refresh();
    assert_eq!(server.calls.borrow().iter().filter(|c| c.starts_with("push")).count(), 1, "the store is not sent again");
    assert_eq!(model::open(&*tab2.storage).unwrap().user.as_deref(), Some("bob"));
}
