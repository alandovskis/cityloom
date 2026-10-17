//! Cloud edits, the rules with no I/O: the document `cityloom-sync` that says which records wait to be sent and
//! the last revision the account's server copy was read up to, marking a record, deciding what a pulled record does,
//! and cutting a batch. The stores mark through `mark` (they hold only a `Storage`), so this file depends on nothing
//! but `shared`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::shared::ports::{Storage, WorkPut, WorkRecord};

/// Where the sync state is kept.
pub const SYNC_KEY: &str = "cityloom-sync";
/// Where a sync document that could not be read is copied before the first write replaces it.
pub const DAMAGED_KEY: &str = "cityloom-sync-damaged";
/// The shape of the document. Raised when the shape changes.
pub const VERSION: u32 = 1;
/// Records in one `PUT`.
pub const BATCH: usize = 100;
/// A browser lets a `keepalive` request carry 64 KiB; the batch of a page being left stays under this, envelope included.
pub const KEEPALIVE_BYTES: usize = 60_000;
/// The key of the one record of kind `Area`: the area the person chose.
pub const AREA_RECORD: &str = "current";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Street,
    Junction,
    Rule,
    Area,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Street, Kind::Junction, Kind::Rule, Kind::Area];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Street => "street",
            Kind::Junction => "junction",
            Kind::Rule => "rule",
            Kind::Area => "area",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// A record: its kind and its key (the place key, the rule's id, or `current`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    pub kind: Kind,
    pub key: String,
}

impl Id {
    pub fn new(kind: Kind, key: &str) -> Id {
        Id { kind, key: key.to_string() }
    }

    /// How the sync document names it: `kind/key` (a key never holds `/`).
    pub fn name(&self) -> String {
        format!("{}/{}", self.kind.as_str(), self.key)
    }

    pub fn from_name(name: &str) -> Option<Id> {
        let (kind, key) = name.split_once('/')?;
        Some(Id { kind: Kind::parse(kind)?, key: key.to_string() })
    }

    /// None for a kind this build does not know.
    pub fn of_record(r: &WorkRecord) -> Option<Id> {
        Some(Id { kind: Kind::parse(&r.kind)?, key: r.key.clone() })
    }
}

/// What the device knows of one record.
fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// When the edit was made on a device (`stamp`), or the server's time for the edit it holds.
    pub edited_at: String,
    /// Waiting to be sent.
    pub pending: bool,
    /// The edit is a removal.
    pub deleted: bool,
    /// The server's revision of the record this device last saw; 0 if it never did.
    pub rev: i64,
    /// The server refused this record for its size on its own: it stays on this device and is left out of batches
    /// until the person edits the place again, which marks it afresh.
    #[serde(default, skip_serializing_if = "is_false")]
    pub blocked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyncDoc {
    pub version: u32,
    /// The account this state belongs to; another account starts over (`switch_user`).
    pub user: Option<String>,
    /// The copy of the account's work the revisions below belong to (the server's `epoch`); None before a first pull.
    #[serde(default)]
    pub epoch: Option<String>,
    /// The account's revision pulled up to.
    pub rev: i64,
    /// By `Id::name`.
    pub records: BTreeMap<String, Entry>,
}

impl Default for SyncDoc {
    fn default() -> Self {
        SyncDoc::new()
    }
}

impl SyncDoc {
    pub fn new() -> SyncDoc {
        SyncDoc { version: VERSION, user: None, epoch: None, rev: 0, records: BTreeMap::new() }
    }

    pub fn entry(&self, id: &Id) -> Option<&Entry> {
        self.records.get(&id.name())
    }

    /// Another account signed in on this browser: its server copy has its own revisions, so nothing of the last one's
    /// counts. What waits to be sent stays (with its time, as a place the new account has never seen); what was
    /// already synced to the other account is dropped, and is claimed again from the stores as never marked.
    pub fn switch_user(&mut self, user: &str) {
        self.user = Some(user.to_string());
        self.epoch = None;
        self.rev = 0;
        self.records.retain(|_, e| e.pending);
        for e in self.records.values_mut() {
            e.rev = 0;
            e.blocked = false;
        }
    }

    /// The server's copy is a new one (storage was turned off and on again while this device was away): none of its
    /// revisions are this device's. Every record the device holds waits to be sent again with its own time, to compete
    /// with whatever the new copy holds as a record made before storage was turned on does, and nothing is dropped.
    pub fn restart(&mut self) {
        self.epoch = None;
        self.rev = 0;
        for e in self.records.values_mut() {
            e.rev = 0;
            e.pending = true;
            e.blocked = false;
        }
    }

    pub fn forget(&mut self, id: &Id) {
        self.records.remove(&id.name());
    }

    /// Whether anything of a usable kind waits to be sent.
    pub fn has_pending(&self, usable: &dyn Fn(Kind) -> bool) -> bool {
        self.records.iter().any(|(name, e)| e.pending && Id::from_name(name).is_some_and(|id| usable(id.kind)))
    }
}

/// What the store held.
#[derive(Debug)]
pub enum Loaded {
    Absent,
    Current(SyncDoc),
    /// Written by a newer build: not read, and never written over.
    Future,
    /// Not a document of this version; the text is kept so it can be copied aside.
    Damaged(String),
}

pub fn load(storage: &dyn Storage) -> Loaded {
    let Some(raw) = storage.recall(SYNC_KEY).filter(|s| !s.trim().is_empty()) else { return Loaded::Absent };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else { return Loaded::Damaged(raw) };
    match value.get("version").and_then(Value::as_u64) {
        Some(v) if v > u64::from(VERSION) => Loaded::Future,
        Some(v) if v == u64::from(VERSION) => match serde_json::from_value::<SyncDoc>(value) {
            Ok(doc) => Loaded::Current(doc),
            Err(_) => Loaded::Damaged(raw),
        },
        _ => Loaded::Damaged(raw),
    }
}

/// The document to work on: the one kept, a new one where none is (a damaged one is copied aside first, unless a
/// copy is already kept), None where it is a newer build's or cannot be copied aside.
pub fn open(storage: &dyn Storage) -> Option<SyncDoc> {
    match load(storage) {
        Loaded::Absent => Some(SyncDoc::new()),
        Loaded::Current(doc) => Some(doc),
        Loaded::Future => None,
        Loaded::Damaged(raw) => {
            if storage.recall(DAMAGED_KEY).is_none() && !storage.remember(DAMAGED_KEY, &raw) {
                return None;
            }
            Some(SyncDoc::new())
        }
    }
}

pub fn save(storage: &dyn Storage, doc: &SyncDoc) -> bool {
    storage.remember(SYNC_KEY, &serde_json::to_string(doc).expect("the sync document serialises"))
}

/// The time as the server writes it, so that two compare as text: UTC to the millisecond.
pub fn stamp(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // civil date from days since 1970-01-01 (H. Hinnant's algorithm)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z", rest / 3600, rest % 3600 / 60, rest % 60, ms % 1000)
}

/// A store wrote a record (or removed it, with `deleted`): it waits to be sent, edited now. Signed in or not. Says
/// whether the mark was kept (not where the sync document is a newer build's); an edit is never held up by it.
pub fn mark(storage: &dyn Storage, kind: Kind, key: &str, deleted: bool) -> bool {
    let Some(mut doc) = open(storage) else { return false };
    let id = Id::new(kind, key);
    let rev = doc.entry(&id).map_or(0, |e| e.rev);
    doc.records.insert(id.name(), Entry { edited_at: stamp(storage.now_ms()), pending: true, deleted, rev, blocked: false });
    let kept = save(storage, &doc);
    crate::sync::hub::marked(storage.identity());
    kept
}

/// Marks every record of the stores that has no entry as edited at `at`: work made before anything was marked (while
/// signed out, with storage off or on an older build) counts as edited when storage is turned on. Says how many.
pub fn claim_unmarked(doc: &mut SyncDoc, local: &[Id], at: &str) -> usize {
    let mut claimed = 0;
    for id in local {
        if doc.entry(id).is_none() {
            doc.records.insert(id.name(), Entry { edited_at: at.to_string(), pending: true, deleted: false, rev: 0, blocked: false });
            claimed += 1;
        }
    }
    claimed
}

/// What a pulled record does to the device's copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Not read (the server could not open it): left as it is.
    Skip,
    /// The local copy waits with a later time: it stays, and is sent on top of the server's revision.
    KeepLocal,
    /// The server's record replaces the local copy. `changed`: the copy was different; `replaced`: it was waiting to
    /// be sent, so the person's own change lost.
    Apply { changed: bool, replaced: bool },
}

pub fn decide(entry: Option<&Entry>, remote: &WorkRecord, local: Option<&Value>) -> Decision {
    if remote.error.is_some() {
        return Decision::Skip;
    }
    if entry.is_some_and(|e| e.pending && e.edited_at > remote.edited_at) {
        return Decision::KeepLocal;
    }
    let changed = if remote.deleted { local.is_some() } else { local != remote.body.as_ref() };
    Decision::Apply { changed, replaced: changed && entry.is_some_and(|e| e.pending) }
}

/// The entry now says the server's record is what the device has.
pub fn adopted(doc: &mut SyncDoc, id: &Id, remote: &WorkRecord) {
    doc.records.insert(id.name(), Entry { edited_at: remote.edited_at.clone(), pending: false, deleted: remote.deleted, rev: remote.rev, blocked: false });
}

/// The local copy stays; it is sent on top of the revision the server has now.
pub fn kept_local(doc: &mut SyncDoc, id: &Id, rev: i64) {
    if let Some(e) = doc.records.get_mut(&id.name()) {
        e.rev = rev;
    }
}

/// The server took `put` as its record `rev`: the place stops waiting, unless it was edited again since `put` was cut.
pub fn accepted(doc: &mut SyncDoc, put: &WorkPut, rev: i64) {
    let Some(id) = Kind::parse(&put.kind).map(|kind| Id::new(kind, &put.key)) else { return };
    if let Some(e) = doc.records.get_mut(&id.name()) {
        e.rev = rev;
        if e.edited_at == put.edited_at {
            e.pending = false;
        }
    }
}

/// The server refused `put` for its size when it was sent alone: the record stays on this device and is left out of
/// later batches (`next_batch`), unless the person edited the place again since `put` was cut.
pub fn block(doc: &mut SyncDoc, put: &WorkPut) {
    let Some(id) = Kind::parse(&put.kind).map(|kind| Id::new(kind, &put.key)) else { return };
    if let Some(e) = doc.records.get_mut(&id.name())
        && e.edited_at == put.edited_at
    {
        e.blocked = true;
    }
}

/// The records that wait to be sent, up to a batch: with their bodies read by `body_of`, or as removals (a record
/// whose place has gone from the store is sent as a removal). `usable` says which kinds may be sent (a kind whose
/// document is damaged or newer is not). A `keepalive` batch stops where the browser's 64 KiB would be passed; a
/// record too big for that waits for the next page. A blocked record is left out.
pub fn next_batch(doc: &SyncDoc, body_of: &dyn Fn(&Id) -> Option<Value>, usable: &dyn Fn(Kind) -> bool, keepalive: bool) -> Vec<WorkPut> {
    let mut out = Vec::new();
    let mut bytes = 0;
    for (name, e) in &doc.records {
        if !e.pending || e.blocked {
            continue;
        }
        let Some(id) = Id::from_name(name).filter(|id| usable(id.kind)) else { continue };
        let body = if e.deleted { None } else { body_of(&id) };
        let put = WorkPut { kind: id.kind.as_str().into(), key: id.key, base_rev: e.rev, edited_at: e.edited_at.clone(), deleted: body.is_none(), body };
        if keepalive {
            let size = serde_json::to_string(&put).map_or(usize::MAX, |s| s.len()) + 1;
            if bytes + size > KEEPALIVE_BYTES {
                break;
            }
            bytes += size;
        }
        out.push(put);
        if out.len() == BATCH {
            break;
        }
    }
    out
}

/// Where a record changed on this device, for whoever shows the person: the server's record replaced the copy
/// (`Remote`), or the person's own waiting change lost to a later one (`Replaced`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Remote,
    Replaced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub id: Id,
    pub origin: Origin,
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::shared::ports::{MemoryStorage, test_ports};
    use serde_json::json;

    fn storage() -> Rc<MemoryStorage> {
        let (_, _, s) = test_ports();
        s.set_now(1_800_000_000_000);
        s
    }

    fn id(kind: Kind, key: &str) -> Id {
        Id::new(kind, key)
    }

    fn remote(kind: &str, key: &str, rev: i64, at: &str, body: Option<serde_json::Value>) -> WorkRecord {
        WorkRecord { kind: kind.into(), key: key.into(), rev, edited_at: at.into(), deleted: body.is_none(), body, error: None }
    }

    fn entry(at: &str, pending: bool, deleted: bool, rev: i64) -> Entry {
        Entry { edited_at: at.into(), pending, deleted, rev, blocked: false }
    }

    const T1: &str = "2027-01-15T08:00:00.000Z";
    const T2: &str = "2027-01-15T08:00:01.000Z";
    const T3: &str = "2027-01-15T08:00:02.000Z";

    #[test]
    fn times_are_written_as_the_server_writes_them() {
        assert_eq!(stamp(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(stamp(1_800_000_000_000), "2027-01-15T08:00:00.000Z");
        assert_eq!(stamp(951_782_400_000 + 43_200_000 + 7), "2000-02-29T12:00:00.007Z");
        assert!(stamp(1_800_000_000_999) > stamp(1_800_000_000_000), "they compare as text");
    }

    #[test]
    fn an_id_round_trips_through_its_name() {
        let i = id(Kind::Street, "w1~n2~x~e");
        assert_eq!(i.name(), "street/w1~n2~x~e");
        assert_eq!(Id::from_name(&i.name()), Some(i));
        assert_eq!(Id::from_name("shape/k"), None);
        assert_eq!(Id::from_name("nonsense"), None);
        assert_eq!(Id::of_record(&remote("rule", "r-1", 1, T1, None)), Some(id(Kind::Rule, "r-1")));
        assert_eq!(Id::of_record(&remote("shape", "k", 1, T1, None)), None, "a kind this build does not know is left alone");
    }

    #[test]
    fn marking_a_record_stamps_it_with_the_time_and_keeps_the_revision_last_seen() {
        let s = storage();
        assert!(mark(&*s, Kind::Street, "w1", false));
        let mut doc = open(&*s).unwrap();
        assert_eq!(doc.entry(&id(Kind::Street, "w1")), Some(&entry("2027-01-15T08:00:00.000Z", true, false, 0)));
        doc.records.get_mut("street/w1").unwrap().rev = 9;
        doc.records.get_mut("street/w1").unwrap().pending = false;
        save(&*s, &doc);
        s.tick(2_000);
        assert!(mark(&*s, Kind::Street, "w1", true));
        assert_eq!(open(&*s).unwrap().entry(&id(Kind::Street, "w1")), Some(&entry("2027-01-15T08:00:02.000Z", true, true, 9)));
    }

    #[test]
    fn a_sync_document_from_a_newer_build_is_not_written_and_a_damaged_one_is_copied_aside_once() {
        let s = storage();
        s.remember(SYNC_KEY, r#"{"version":99,"user":"x"}"#);
        assert!(!mark(&*s, Kind::Area, AREA_RECORD, false));
        assert_eq!(open(&*s), None);
        assert_eq!(s.recall(SYNC_KEY).as_deref(), Some(r#"{"version":99,"user":"x"}"#), "left alone");
        s.remember(SYNC_KEY, "{broken");
        assert!(mark(&*s, Kind::Area, AREA_RECORD, false));
        assert_eq!(s.recall(DAMAGED_KEY).as_deref(), Some("{broken"));
        s.remember(SYNC_KEY, "{broken again");
        assert!(mark(&*s, Kind::Area, AREA_RECORD, false));
        assert_eq!(s.recall(DAMAGED_KEY).as_deref(), Some("{broken"), "the first copy is kept");
    }

    #[test]
    fn switching_account_forgets_the_old_revisions_keeps_what_waits_and_drops_what_was_synced() {
        // switching_account_... : Review Focus 1
        let mut doc = SyncDoc::new();
        doc.user = Some("ada".into());
        doc.epoch = Some("e1".into());
        doc.rev = 40;
        doc.records.insert("street/a".into(), entry(T1, false, false, 12)); // synced to Ada's account
        doc.records.insert("street/b".into(), entry(T2, true, false, 30)); // waiting, made after Ada's last sync
        doc.records.insert("rule/c".into(), entry(T2, false, true, 31)); // a deletion Ada's account already knows
        doc.switch_user("bob");
        assert_eq!((doc.user.as_deref(), doc.rev, doc.epoch.as_deref()), (Some("bob"), 0, None), "Ada's copy is not Bob's");
        assert_eq!(doc.records.keys().collect::<Vec<_>>(), ["street/b"]);
        assert_eq!(doc.records["street/b"], entry(T2, true, false, 0), "it keeps its time and is a new place to Bob's account");
    }

    #[test]
    fn a_local_record_never_marked_is_claimed_at_the_given_time_and_a_marked_one_is_left() {
        let mut doc = SyncDoc::new();
        doc.records.insert("street/old".into(), entry(T1, false, false, 3));
        let n = claim_unmarked(&mut doc, &[id(Kind::Street, "old"), id(Kind::Street, "new"), id(Kind::Rule, "r-1")], T3);
        assert_eq!(n, 2);
        assert_eq!(doc.records["street/new"], entry(T3, true, false, 0));
        assert_eq!(doc.records["street/old"], entry(T1, false, false, 3));
    }

    #[test]
    fn a_pulled_record_is_applied_unless_the_local_one_waits_with_a_later_time() {
        let theirs = remote("street", "k", 5, T2, Some(json!({"w": 1})));
        let mine = json!({"w": 2});
        assert_eq!(decide(None, &theirs, None), Decision::Apply { changed: true, replaced: false });
        assert_eq!(decide(Some(&entry(T1, false, false, 4)), &theirs, Some(&mine)), Decision::Apply { changed: true, replaced: false });
        assert_eq!(decide(Some(&entry(T3, true, false, 4)), &theirs, Some(&mine)), Decision::KeepLocal, "mine is later");
        assert_eq!(
            decide(Some(&entry(T1, true, false, 4)), &theirs, Some(&mine)),
            Decision::Apply { changed: true, replaced: true },
            "mine was earlier: replaced"
        );
        assert_eq!(
            decide(Some(&entry(T2, true, false, 4)), &theirs, Some(&mine)),
            Decision::Apply { changed: true, replaced: true },
            "a tie goes to the server"
        );
    }

    #[test]
    fn the_echo_of_ones_own_record_is_applied_without_a_change() {
        // the_echo_of_ones_own_record_...: Review Focus 5
        let body = json!({"w": 1});
        let theirs = remote("street", "k", 5, T2, Some(body.clone()));
        assert_eq!(decide(Some(&entry(T2, false, false, 5)), &theirs, Some(&body)), Decision::Apply { changed: false, replaced: false });
        assert_eq!(
            decide(Some(&entry(T2, true, false, 4)), &theirs, Some(&body)),
            Decision::Apply { changed: false, replaced: false },
            "a push whose answer was lost"
        );
    }

    #[test]
    fn a_pulled_deletion_changes_a_place_that_exists_and_nothing_that_does_not() {
        let gone = remote("rule", "r-1", 6, T2, None);
        assert_eq!(decide(Some(&entry(T1, false, false, 5)), &gone, Some(&json!({}))), Decision::Apply { changed: true, replaced: false });
        assert_eq!(decide(None, &gone, None), Decision::Apply { changed: false, replaced: false });
        assert_eq!(decide(Some(&entry(T3, true, false, 5)), &gone, Some(&json!({}))), Decision::KeepLocal, "re-made later than the deletion");
    }

    #[test]
    fn a_record_the_server_could_not_open_is_left_alone() {
        let mut broken = remote("street", "k", 5, T2, None);
        broken.deleted = false;
        broken.error = Some("corrupt".into());
        assert_eq!(decide(Some(&entry(T1, false, false, 4)), &broken, Some(&json!({}))), Decision::Skip);
    }

    fn doc_with(pending: &[(&str, &str, bool)]) -> SyncDoc {
        let mut doc = SyncDoc::new();
        for (name, at, deleted) in pending {
            doc.records.insert(name.to_string(), entry(at, true, *deleted, 2));
        }
        doc
    }

    #[test]
    fn a_batch_takes_what_waits_with_its_body_or_as_a_deletion() {
        let doc = doc_with(&[("street/a", T1, false), ("rule/r-1", T2, true), ("street/gone", T3, false)]);
        let batch = next_batch(&doc, &|i| (i.key == "a").then(|| json!({"a": 1})), &|_| true, false);
        let shape: Vec<_> = batch.iter().map(|p| (p.kind.as_str(), p.key.as_str(), p.deleted, p.body.is_some(), p.base_rev)).collect();
        assert_eq!(shape, [("rule", "r-1", true, false, 2), ("street", "a", false, true, 2), ("street", "gone", true, false, 2)]);
        assert_eq!(batch[1].edited_at, T1);
    }

    #[test]
    fn a_batch_holds_at_most_100_and_skips_what_is_not_usable_or_not_waiting() {
        let mut doc = SyncDoc::new();
        for i in 0..150 {
            doc.records.insert(format!("street/k{i:03}"), entry(T1, true, false, 0));
        }
        doc.records.insert("rule/r-1".into(), entry(T1, true, false, 0));
        doc.records.insert("street/synced".into(), entry(T1, false, false, 1));
        let batch = next_batch(&doc, &|_| Some(json!({})), &|k| k != Kind::Rule, false);
        assert_eq!(batch.len(), BATCH);
        assert!(batch.iter().all(|p| p.kind == "street" && p.key != "synced"));
    }

    #[test]
    fn a_keepalive_batch_is_cut_to_fit_and_the_rest_waits_for_the_next_page() {
        // a_keepalive_batch_...: Review Focus 5
        let mut doc = SyncDoc::new();
        for i in 0..5 {
            doc.records.insert(format!("street/k{i}"), entry(T1, true, false, 0));
        }
        let big = json!({"pad": "x".repeat(20_000)});
        let batch = next_batch(&doc, &|_| Some(big.clone()), &|_| true, true);
        assert_eq!(batch.len(), 2, "three of 20 KB would pass the 60,000 bytes the browser allows a keepalive body");
        let total: usize = batch.iter().map(|p| serde_json::to_string(p).unwrap().len()).sum();
        assert!(total <= KEEPALIVE_BYTES);
        let alone = next_batch(&doc, &|_| Some(json!({"pad": "x".repeat(70_000)})), &|_| true, true);
        assert!(alone.is_empty(), "one record too big for a keepalive request waits for the next open");
    }

    #[test]
    fn what_the_server_took_stops_waiting_unless_the_person_changed_the_place_meanwhile() {
        let mut doc = doc_with(&[("street/a", T1, false), ("street/b", T1, false)]);
        let put =
            |key: &str, at: &str| WorkPut { kind: "street".into(), key: key.into(), base_rev: 2, edited_at: at.into(), deleted: false, body: Some(json!({})) };
        accepted(&mut doc, &put("a", T1), 7);
        assert_eq!(doc.records["street/a"], entry(T1, false, false, 7));
        doc.records.get_mut("street/b").unwrap().edited_at = T3.into(); // edited again while the request was out
        accepted(&mut doc, &put("b", T1), 8);
        assert_eq!(doc.records["street/b"], entry(T3, true, false, 8), "still waiting, on top of the server's revision");
    }

    #[test]
    fn adopting_the_servers_record_and_keeping_ones_own_set_the_entry() {
        let mut doc = doc_with(&[("street/a", T1, false)]);
        adopted(&mut doc, &id(Kind::Street, "a"), &remote("street", "a", 9, T2, Some(json!({}))));
        assert_eq!(doc.records["street/a"], entry(T2, false, false, 9));
        adopted(&mut doc, &id(Kind::Rule, "r-1"), &remote("rule", "r-1", 10, T3, None));
        assert_eq!(doc.records["rule/r-1"], entry(T3, false, true, 10));
        doc.records.insert("street/m".into(), entry(T3, true, false, 1));
        kept_local(&mut doc, &id(Kind::Street, "m"), 12);
        assert_eq!(doc.records["street/m"], entry(T3, true, false, 12));
    }

    #[test]
    fn forgetting_a_record_and_asking_about_what_waits() {
        let mut doc = doc_with(&[("street/a", T1, false), ("rule/r-1", T1, false)]);
        assert!(doc.has_pending(&|_| true));
        assert!(doc.has_pending(&|k| k == Kind::Rule));
        assert!(!doc.has_pending(&|k| k == Kind::Junction));
        doc.forget(&id(Kind::Rule, "r-1"));
        assert!(!doc.has_pending(&|k| k == Kind::Rule));
    }

    #[test]
    fn a_record_the_server_refused_for_its_size_is_left_out_of_batches_until_the_place_is_edited_again() {
        let mut doc = doc_with(&[("street/big", T1, false), ("street/small", T1, false)]);
        let put =
            |key: &str, at: &str| WorkPut { kind: "street".into(), key: key.into(), base_rev: 2, edited_at: at.into(), deleted: false, body: Some(json!({})) };
        block(&mut doc, &put("big", T1));
        assert!(doc.records["street/big"].blocked);
        let batch = next_batch(&doc, &|_| Some(json!({})), &|_| true, false);
        assert_eq!(batch.iter().map(|p| p.key.as_str()).collect::<Vec<_>>(), ["small"]);
        assert!(doc.has_pending(&|_| true), "it is not synced, and the menu still says so");
        // an edit made since the request was cut is not blocked
        doc.records.get_mut("street/small").unwrap().edited_at = T3.into();
        block(&mut doc, &put("small", T1));
        assert!(!doc.records["street/small"].blocked);
        // the person edits the place again: it is marked afresh, and tried again
        let s = storage();
        assert!(mark(&*s, Kind::Street, "big", false));
        let mut doc = open(&*s).unwrap();
        let edited = doc.records["street/big"].edited_at.clone();
        block(&mut doc, &put("big", &edited));
        assert!(save(&*s, &doc) && open(&*s).unwrap().records["street/big"].blocked);
        s.tick(1_000);
        assert!(mark(&*s, Kind::Street, "big", false));
        assert!(!open(&*s).unwrap().records["street/big"].blocked);
        assert!(!s.recall(SYNC_KEY).unwrap().contains("blocked"), "an ordinary record does not carry the flag");
    }

    #[test]
    fn a_new_server_copy_makes_every_record_wait_again_with_its_own_time_and_forgets_its_revisions() {
        let mut doc = SyncDoc::new();
        doc.rev = 40;
        doc.epoch = Some("e1".into());
        doc.records.insert("street/a".into(), entry(T1, false, false, 12));
        doc.records.insert("rule/c".into(), entry(T2, false, true, 31));
        let mut refused = entry(T3, true, false, 5);
        refused.blocked = true;
        doc.records.insert("street/b".into(), refused);
        doc.restart();
        assert_eq!((doc.rev, doc.epoch.clone()), (0, None));
        assert_eq!(doc.records["street/a"], entry(T1, true, false, 0), "still edited when it was");
        assert_eq!(doc.records["rule/c"], entry(T2, true, true, 0), "a removal competes with what the new copy holds too");
        assert_eq!(doc.records["street/b"], entry(T3, true, false, 0), "and a refusal is forgotten: the new copy has its own limits");
    }
}
