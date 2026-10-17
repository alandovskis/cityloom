//! Runs the sync. Signed in with storage on the server, it pulls, merges and pushes: when a page opens (once the
//! account says storage is on), shortly after a write (debounced like the save, and not more than once every two
//! seconds), when the tab shows again, when the network returns, and, with `keepalive`, when the page is left. A
//! failure leaves everything marked to send. It never blocks editing, and it does not keep a connection open.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use leptos::prelude::*;

use crate::restriction::store::RestrictionStore;
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::{Me, Ports, WorkAsk, WorkError, WorkPushed, WorkPut, WorkRecord, WorkStatus};
use crate::sync::model::{self, Change, Decision, Entry, Id, Kind, Origin, Report, SyncDoc};
use crate::sync::{hub, local, text};

/// After the last write, before a run.
const DEBOUNCE_MS: u32 = 250;
/// Between two runs that writes start: the server's limit for these routes is about one call a second.
const MIN_GAP_MS: u64 = 2_000;
/// A run does at most this many calls; a server that never says it is done does not hold the page.
const MAX_ROUNDS: u32 = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Nobody is signed in, or the person did not choose to store their work on the server.
    Off,
    Synced,
    /// Something waits to be sent, or the last run failed.
    NotSynced,
    /// A document of the browser is damaged or from a newer build: its records are neither sent nor changed.
    Paused,
}

pub struct SyncVm {
    ports: Ports,
    i18n: Rc<I18n>,
    /// The account whose work is synced, while storage on the server is chosen and someone is signed in.
    user: RefCell<Option<String>>,
    status: ArcRwSignal<Status>,
    running: Cell<bool>,
    /// A write or a request came while a run was going: run again when it ends.
    again: Cell<bool>,
    timer: Cell<Option<u32>>,
    last_end: Cell<u64>,
    told_paused: Cell<bool>,
    told_large: Cell<bool>,
    /// The server refused the account's work for an account-wide limit (records or bytes): nothing is pushed on this
    /// page any more (the next page open tries again, and so does turning storage on), though pulls go on.
    capped: Cell<bool>,
    /// Those waiting for the end of the run (the Account page, to count what it merged).
    waiting: RefCell<Vec<Box<dyn FnOnce(Report)>>>,
}

impl SyncVm {
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<SyncVm> {
        let key = ports.storage.identity();
        let vm = Rc::new(SyncVm {
            ports,
            i18n,
            user: RefCell::new(None),
            status: ArcRwSignal::new(Status::Off),
            running: Cell::new(false),
            again: Cell::new(false),
            timer: Cell::new(None),
            last_end: Cell::new(0),
            told_paused: Cell::new(false),
            told_large: Cell::new(false),
            capped: Cell::new(false),
            waiting: RefCell::new(Vec::new()),
        });
        let weak = Rc::downgrade(&vm);
        hub::on_marked(key, move || {
            if let Some(vm) = weak.upgrade() {
                vm.touch();
            }
        });
        hub::install(&vm);
        vm
    }

    /// The state, drawn again when it changes.
    pub fn status(&self) -> Status {
        self.status.get()
    }

    /// What the menu says of it, in the page's language; None when nothing is synced.
    pub fn status_text(&self) -> Option<String> {
        text::status_text(&self.i18n, self.status.get())
    }

    fn say(&self, key: &str) {
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new()));
    }

    /// The account says who is signed in: with storage on the server, sync starts.
    pub fn known(self: &Rc<Self>, me: &Me) {
        if me.store_on_server == Some(true) {
            *self.user.borrow_mut() = Some(me.id.clone());
            self.status.set(Status::NotSynced);
            self.sync();
        } else {
            self.stop();
        }
    }

    /// The person just chose to store their work on the server: merge it with the account's, then tell `done`.
    pub fn enable(self: &Rc<Self>, user: &str, done: Box<dyn FnOnce(Report)>) {
        *self.user.borrow_mut() = Some(user.to_string());
        self.status.set(Status::NotSynced);
        self.told_paused.set(false);
        self.told_large.set(false);
        self.capped.set(false);
        self.sync_with(Some(done));
    }

    /// Sync stops (signed out, or storage not on the server); what waits to be sent stays marked.
    pub fn stop(&self) {
        self.user.borrow_mut().take();
        self.cancel_timer();
        self.status.set(Status::Off);
    }

    /// Storage on the server was turned off (here, or on another device): sync stops and its state is cleared. The
    /// browser's work stays; it is claimed again, as never marked, if storage is turned on later.
    pub fn disable(&self) {
        self.stop();
        if model::open(&*self.ports.storage).is_some() {
            model::save(&*self.ports.storage, &SyncDoc::new());
        }
    }

    /// The tab shows again, or the network returned: run at once.
    pub fn refresh(self: &Rc<Self>) {
        if self.user.borrow().is_some() {
            self.sync();
        }
    }

    /// A store marked a record.
    pub fn touch(self: &Rc<Self>) {
        if self.user.borrow().is_none() {
            return;
        }
        if self.status.get_untracked() == Status::Synced {
            self.status.set(Status::NotSynced);
        }
        if self.running.get() {
            self.again.set(true);
        } else {
            self.schedule();
        }
    }

    /// The page is being left (the editors have already written what they hold): send what waits, as a request that
    /// outlives the page, cut to what the browser allows. Nothing is pulled and nothing waits for the answer.
    pub fn leave(self: &Rc<Self>) {
        if self.user.borrow().is_none() || self.capped.get() {
            return;
        }
        self.cancel_timer();
        let storage = self.ports.storage.clone();
        let Some(doc) = model::open(&*storage) else { return };
        let usable = |kind: Kind| local::usable(&storage, kind);
        let batch = model::next_batch(&doc, &|id| local::read(&storage, id), &usable, true);
        if !batch.is_empty() {
            self.ports.work.push(batch, true, Box::new(|_| {}));
        }
    }

    pub fn sync(self: &Rc<Self>) {
        self.sync_with(None);
    }

    fn cancel_timer(&self) {
        if let Some(id) = self.timer.take() {
            self.ports.scheduler.cancel(id);
        }
    }

    fn schedule(self: &Rc<Self>) {
        if self.timer.get().is_some() {
            return;
        }
        let last = self.last_end.get();
        let gap = self.ports.storage.now_ms().saturating_sub(last);
        let wait = if last > 0 && gap < MIN_GAP_MS { (MIN_GAP_MS - gap) as u32 } else { DEBOUNCE_MS };
        let me = Rc::downgrade(self);
        let id = self.ports.scheduler.after(
            wait.max(DEBOUNCE_MS),
            Box::new(move || {
                if let Some(vm) = me.upgrade() {
                    vm.timer.set(None);
                    vm.sync();
                }
            }),
        );
        self.timer.set(Some(id));
    }

    fn sync_with(self: &Rc<Self>, done: Option<Box<dyn FnOnce(Report)>>) {
        if let Some(done) = done {
            self.waiting.borrow_mut().push(done);
        }
        let Some(user) = self.user.borrow().clone() else {
            self.release(Report::default());
            return;
        };
        if self.running.replace(true) {
            self.again.set(true);
            return;
        }
        self.cancel_timer();
        // The editors write what they hold, so it is marked before the stores are read.
        hub::flush();
        let storage = self.ports.storage.clone();
        let Some(mut doc) = model::open(&*storage) else {
            self.tell_paused();
            self.end(Report::default(), Status::Paused);
            return;
        };
        if doc.user.as_deref() != Some(user.as_str()) {
            doc.switch_user(&user);
        }
        self.rename_rules(&mut doc);
        model::claim_unmarked(&mut doc, &local::ids(&storage), &model::stamp(storage.now_ms()));
        model::save(&*storage, &doc);
        let run = Rc::new(Run {
            vm: self.clone(),
            full: Cell::new(doc.rev == 0),
            pulled: RefCell::new(Vec::new()),
            rev: Cell::new(doc.rev),
            sent: Cell::new(0),
            received: Cell::new(0),
            changes: RefCell::new(Vec::new()),
            epoch: RefCell::new(doc.epoch.clone()),
            paused: Cell::new(false),
            skipped: Cell::new(false),
            failed_apply: Cell::new(false),
            single: Cell::new(false),
            rounds: Cell::new(0),
        });
        run.pull(WorkAsk::first(doc.rev).with_epoch(doc.epoch.clone()));
    }

    /// Gives the `r<n>` rules random ids (once: later there are none). A rule that had been sent under its old id is
    /// removed there; the new ids are claimed as never marked.
    fn rename_rules(&self, doc: &mut SyncDoc) {
        let storage = self.ports.storage.clone();
        if !local::usable(&storage, Kind::Rule) {
            return;
        }
        let renamed = RestrictionStore::new(storage.clone()).rename_numbered(&mut || storage.random_hex());
        for (old, _) in renamed {
            let id = Id::new(Kind::Rule, &old);
            let seen = doc.entry(&id).map_or(0, |e| e.rev);
            if seen > 0 {
                doc.records.insert(id.name(), Entry { edited_at: model::stamp(storage.now_ms()), pending: true, deleted: true, rev: seen, blocked: false });
            } else {
                doc.forget(&id);
            }
        }
    }

    fn release(&self, report: Report) {
        for done in self.waiting.take() {
            done(report);
        }
    }

    fn end(self: &Rc<Self>, report: Report, status: Status) {
        self.running.set(false);
        self.last_end.set(self.ports.storage.now_ms());
        self.status.set(if self.user.borrow().is_some() { status } else { Status::Off });
        self.release(report);
        if self.again.replace(false) {
            self.schedule();
        }
    }

    fn tell_paused(&self) {
        if !self.told_paused.replace(true) {
            self.say("sync-paused");
        }
    }

    fn tell_large(&self, limit: &str) {
        if !self.told_large.replace(true) {
            self.say(text::too_large_key(limit));
        }
    }

    /// The state a finished run leaves.
    fn status_after(&self, ok: bool, paused: bool) -> Status {
        if self.user.borrow().is_none() {
            return Status::Off;
        }
        let storage = &self.ports.storage;
        if paused || Kind::ALL.into_iter().any(|kind| !local::usable(storage, kind)) {
            self.tell_paused();
            return Status::Paused;
        }
        let waiting = model::open(&**storage).is_some_and(|doc| doc.has_pending(&|_| true));
        if ok && !waiting { Status::Synced } else { Status::NotSynced }
    }
}

/// One pass: pull everything above the revision seen, merge it, push what waits.
struct Run {
    vm: Rc<SyncVm>,
    /// The pull is from revision 0: what comes is everything the server has, so what it does not list is gone.
    full: Cell<bool>,
    pulled: RefCell<Vec<WorkRecord>>,
    rev: Cell<i64>,
    sent: Cell<usize>,
    received: Cell<usize>,
    changes: RefCell<Vec<Change>>,
    /// The copy of the account's work the pages came from (None until the first page, and after a reset).
    epoch: RefCell<Option<String>>,
    /// A kind was not usable: the status says syncing is paused.
    paused: Cell<bool>,
    /// A pulled record was left alone (a paused kind, or a body the server could not open): the revision stays where
    /// it was, so the record is pulled again by the next run instead of being missed for good.
    skipped: Cell<bool>,
    failed_apply: Cell<bool>,
    /// A batch was refused for its size: the records go one at a time, to find the one that is too big.
    single: Cell<bool>,
    rounds: Cell<u32>,
}

impl Run {
    fn round(&self) -> bool {
        self.rounds.set(self.rounds.get() + 1);
        self.rounds.get() <= MAX_ROUNDS
    }

    fn pull(self: &Rc<Self>, ask: WorkAsk) {
        if !self.round() {
            return self.finish(false);
        }
        let run = self.clone();
        self.vm.ports.work.pull(
            ask.clone(),
            Box::new(move |answer| match answer {
                Ok(page) => {
                    run.pulled.borrow_mut().extend(page.records);
                    *run.epoch.borrow_mut() = Some(page.epoch.clone());
                    if page.more {
                        // The next page goes on from this one's revision and still says where the pull began.
                        run.pull(ask.continued(page.rev).with_epoch(Some(page.epoch)));
                    } else {
                        run.rev.set(page.rev);
                        run.merge();
                    }
                }
                // The server purged a tombstone this device never saw (or its counter was reset): take everything.
                Err(WorkError::Resync) if ask.start != 0 => {
                    run.pulled.borrow_mut().clear();
                    run.full.set(true);
                    run.pull(WorkAsk::first(0).with_epoch(ask.epoch));
                }
                // Storage was turned off and on again while this device was away: merge as when storage is turned on.
                Err(WorkError::Reset) => run.reset(),
                Err(e) => run.fail(e),
            }),
        );
    }

    /// The account's server copy was turned off and on again while this device was away: its revisions mean nothing to
    /// the new copy. Merge as when storage is turned on: every record this device holds competes again with its own
    /// time (`SyncDoc::restart`), the new copy is pulled whole, and nothing is removed locally (the ghost removal is
    /// for a purged tombstone only).
    fn reset(self: &Rc<Self>) {
        let storage = self.vm.ports.storage.clone();
        let Some(mut doc) = model::open(&*storage) else {
            self.paused.set(true);
            return self.finish(false);
        };
        doc.restart();
        model::save(&*storage, &doc);
        self.pulled.borrow_mut().clear();
        self.full.set(false);
        *self.epoch.borrow_mut() = None;
        self.rev.set(0);
        self.pull(WorkAsk::first(0));
    }

    /// Applies a record from the server to the stores and the document. False where a store refused it.
    fn apply(&self, doc: &mut SyncDoc, remote: &WorkRecord) -> bool {
        let Some(id) = Id::of_record(remote) else { return true };
        let storage = &self.vm.ports.storage;
        if !local::usable(storage, id.kind) {
            self.paused.set(true);
            self.skipped.set(true);
            return true;
        }
        if !remote.deleted && remote.body.is_none() {
            return true;
        }
        let here = local::read(storage, &id);
        match model::decide(doc.entry(&id), remote, here.as_ref()) {
            Decision::Skip => {
                self.skipped.set(true);
                true
            }
            Decision::KeepLocal => {
                model::kept_local(doc, &id, remote.rev);
                true
            }
            Decision::Apply { changed, replaced } => {
                if changed {
                    let kept = match &remote.body {
                        Some(body) if !remote.deleted => local::write(storage, &id, body),
                        _ => local::remove(storage, &id),
                    };
                    if !kept {
                        return false;
                    }
                    self.received.set(self.received.get() + 1);
                    self.changes.borrow_mut().push(Change { id: id.clone(), origin: if replaced { Origin::Replaced } else { Origin::Remote } });
                }
                model::adopted(doc, &id, remote);
                true
            }
        }
    }

    fn merge(self: &Rc<Self>) {
        let storage = self.vm.ports.storage.clone();
        // Read again: another tab may have written since the run began.
        let Some(mut doc) = model::open(&*storage) else {
            self.paused.set(true);
            return self.finish(false);
        };
        let pulled = self.pulled.take();
        let mut listed = BTreeSet::new();
        for remote in &pulled {
            if let Some(id) = Id::of_record(remote) {
                listed.insert(id.name());
            }
            if !self.apply(&mut doc, remote) {
                self.failed_apply.set(true);
            }
        }
        if self.full.get() {
            // The server listed everything it has: a place this device holds as synced that is not among them is gone.
            let ghosts: Vec<Id> = doc
                .records
                .iter()
                .filter(|(name, e)| !e.pending && !e.deleted && e.rev > 0 && !listed.contains(*name))
                .filter_map(|(name, _)| Id::from_name(name))
                .filter(|id| id.kind != Kind::Area && local::usable(&storage, id.kind))
                .collect();
            for id in ghosts {
                let here = local::read(&storage, &id).is_some();
                if here && !local::remove(&storage, &id) {
                    self.failed_apply.set(true);
                    continue;
                }
                if here {
                    self.received.set(self.received.get() + 1);
                    self.changes.borrow_mut().push(Change { id: id.clone(), origin: Origin::Remote });
                }
                doc.forget(&id);
            }
        }
        if self.failed_apply.get() {
            // The revision stays where it was, so the page is pulled again; nothing is sent over what could not be kept.
            model::save(&*storage, &doc);
            return self.finish(false);
        }
        // A run that skipped a record leaves the revision where it was: the next run pulls the record again.
        if !self.skipped.get() {
            doc.rev = self.rev.get();
        }
        doc.epoch = self.epoch.borrow().clone();
        model::save(&*storage, &doc);
        hub::publish(&self.changes.borrow());
        self.push();
    }

    fn push(self: &Rc<Self>) {
        if !self.round() {
            return self.finish(false);
        }
        let storage = self.vm.ports.storage.clone();
        let Some(doc) = model::open(&*storage) else {
            self.paused.set(true);
            return self.finish(false);
        };
        let usable = |kind: Kind| local::usable(&storage, kind);
        let mut batch = model::next_batch(&doc, &|id| local::read(&storage, id), &usable, false);
        if self.single.get() {
            batch.truncate(1);
        }
        if batch.is_empty() {
            return self.finish(!self.failed_apply.get());
        }
        if self.vm.capped.get() {
            // The account is full: what waits stays marked on this device.
            return self.finish(false);
        }
        let sent = batch.clone();
        let run = self.clone();
        self.vm.ports.work.push(
            batch,
            false,
            Box::new(move |answer| match answer {
                Ok(pushed) => {
                    run.absorb(&sent, pushed);
                    run.push();
                }
                // The account is full (records or bytes): no record is at fault, so none is blocked. Nothing more is
                // pushed on this page; it is said once.
                Err(WorkError::TooLarge(limit)) if limit == "records" || limit == "bytes" => {
                    run.vm.capped.set(true);
                    run.vm.tell_large(&limit);
                    run.finish(false);
                }
                // Too big for the server: the batch goes again one record at a time, to find the one that is.
                Err(WorkError::TooLarge(_)) if sent.len() > 1 => {
                    run.single.set(true);
                    run.push();
                }
                // A record refused on its own stays on this device: it is left out of what follows and said once.
                Err(WorkError::TooLarge(limit)) => {
                    run.block(&sent[0], &limit);
                    run.push();
                }
                Err(e) => run.fail(e),
            }),
        );
    }

    /// The server refused this record for its size on its own: it is blocked (`model::block`), which keeps it on this
    /// device and out of later batches until the person edits the place again, and the limit is said once.
    fn block(&self, put: &WorkPut, limit: &str) {
        let storage = self.vm.ports.storage.clone();
        if let Some(mut doc) = model::open(&*storage) {
            model::block(&mut doc, put);
            model::save(&*storage, &doc);
        }
        self.vm.tell_large(limit);
    }

    /// What the server said of a batch: the records it took stop waiting; for each it did not take, the server's
    /// record (a later edit) replaces the device's.
    fn absorb(&self, sent: &[WorkPut], pushed: WorkPushed) {
        let storage = self.vm.ports.storage.clone();
        let Some(mut doc) = model::open(&*storage) else { return };
        for result in pushed.results {
            let Some(put) = sent.iter().find(|p| p.kind == result.kind && p.key == result.key) else { continue };
            match result.status {
                WorkStatus::Accepted => {
                    model::accepted(&mut doc, put, result.rev);
                    self.sent.set(self.sent.get() + 1);
                }
                WorkStatus::Superseded => {
                    if let Some(remote) = result.record
                        && !self.apply(&mut doc, &remote)
                    {
                        self.failed_apply.set(true);
                    }
                }
            }
        }
        model::save(&*storage, &doc);
    }

    fn fail(self: &Rc<Self>, error: WorkError) {
        match error {
            WorkError::SignedOut => {
                self.vm.stop();
                self.finish(false);
            }
            WorkError::StorageOff => {
                self.vm.disable();
                self.finish(false);
            }
            // A 413 is `push`'s own business, and a pull is not told `Resync` or `Reset` twice: they end the run here.
            WorkError::TooLarge(_) | WorkError::Resync | WorkError::Reset | WorkError::Unavailable | WorkError::Refused => self.finish(false),
        }
    }

    fn finish(self: &Rc<Self>, ok: bool) {
        let report = Report { ok, sent: self.sent.get(), received: self.received.get() };
        let status = self.vm.status_after(ok, self.paused.get());
        self.vm.end(report, status);
    }
}
