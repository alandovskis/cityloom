//! The work a person keeps on the server: the rules that decide what a batch of records does, over metadata only.
//! Sealing and storage are the Postgres `WorkRepo`'s business (`db/work.rs`).

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use time::OffsetDateTime;
use time::format_description::BorrowedFormatItem;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

pub const MAX_KEY: usize = 200;
pub const MAX_RECORD_BYTES: usize = 64 * 1024;
pub const MAX_RECORDS: usize = 5_000;
pub const MAX_SEALED_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_BATCH: usize = 100;
pub const PAGE: usize = 500;
pub const TOMBSTONE_DAYS: i64 = 90;
/// What sealing adds to a body: version byte, nonce, tag (`crypto::seal`). A test in `tests/work_db.rs` pins it.
pub const SEAL_OVERHEAD: usize = 1 + 12 + 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Street,
    Junction,
    Rule,
    Area,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Street => "street",
            Kind::Junction => "junction",
            Kind::Rule => "rule",
            Kind::Area => "area",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "street" => Kind::Street,
            "junction" => Kind::Junction,
            "rule" => Kind::Rule,
            "area" => Kind::Area,
            _ => return None,
        })
    }
}

/// At most 200 characters of `[A-Za-z0-9~_.:-]`, and not empty.
pub fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= MAX_KEY && key.bytes().all(|b| b.is_ascii_alphanumeric() || b"~_.:-".contains(&b))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    pub kind: Kind,
    pub key: String,
}

/// The associated data a body is sealed with: `work.<kind>.<key>.<user_id>`.
pub fn aad(id: &Id, user: Uuid) -> String {
    crate::crypto::aad("work", id.kind.as_str(), &format!("{}.{user}", id.key))
}

const STAMP: &[BorrowedFormatItem<'static>] = time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

/// A time as the browser compares them as text: UTC, to the millisecond, `2027-01-15T08:00:00.000Z`.
pub fn stamp(at: OffsetDateTime) -> String {
    at.to_offset(time::UtcOffset::UTC).format(STAMP).unwrap_or_default()
}

pub fn parse_stamp(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    Record,
    Records,
    Bytes,
    Batch,
}

impl Limit {
    pub fn as_str(self) -> &'static str {
        match self {
            Limit::Record => "record",
            Limit::Records => "records",
            Limit::Bytes => "bytes",
            Limit::Batch => "batch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    TooLarge(Limit),
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    Kind,
    Key,
    Time,
    Body,
    BaseRev,
}

/// A record a device sends. `body` is the JSON text as the browser sent it (plaintext here; sealed on its way in).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Put {
    pub id: Id,
    pub base_rev: i64,
    pub edited_at: OffsetDateTime,
    pub deleted: bool,
    pub body: Option<Vec<u8>>,
}

impl Put {
    /// Checks the parts of a record the server must know; the size limits are `plan_batch`'s.
    pub fn from_wire(kind: &str, key: &str, base_rev: i64, edited_at: &str, deleted: bool, body: Option<&str>) -> Result<Put, Invalid> {
        let kind = Kind::parse(kind).ok_or(Invalid::Kind)?;
        if !valid_key(key) {
            return Err(Invalid::Key);
        }
        if base_rev < 0 {
            return Err(Invalid::BaseRev);
        }
        let edited_at = parse_stamp(edited_at).ok_or(Invalid::Time)?;
        let body = match (deleted, body) {
            (true, None) => None,
            (false, Some(text)) => Some(text.as_bytes().to_vec()),
            _ => return Err(Invalid::Body),
        };
        Ok(Put { id: Id { kind, key: key.to_string() }, base_rev, edited_at, deleted, body })
    }

    /// What the body weighs once sealed; a tombstone has none.
    pub fn sealed_len(&self) -> usize {
        self.body.as_ref().map_or(0, |b| b.len() + SEAL_OVERHEAD)
    }
}

/// What the server holds of a row, without its body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub rev: i64,
    pub edited_at: OffsetDateTime,
    pub deleted: bool,
    /// The sealed body's length, 0 for a tombstone.
    pub size: usize,
}

/// What an account holds in all: every row (tombstones too) and the sealed bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub rows: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Accepted,
    Superseded,
}

/// What became of one record: its revision (the new one if accepted, the row's if superseded) and, if accepted,
/// the time it is stored at (the sent time capped at the server's clock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub verdict: Verdict,
    pub rev: i64,
    pub at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The account's revision after the batch.
    pub rev: i64,
    pub items: Vec<Item>,
}

/// Decides a whole batch against the rows it names. `rev` is the account's revision now, `totals` what the account
/// holds, `current` the metadata of the rows the batch names. Nothing is applied: a refusal covers the whole batch.
pub fn plan_batch(rev: i64, totals: Totals, current: &BTreeMap<Id, Meta>, batch: &[Put], now: OffsetDateTime) -> Result<Plan, Refusal> {
    if batch.len() > MAX_BATCH {
        return Err(Refusal::TooLarge(Limit::Batch));
    }
    let mut seen = BTreeSet::new();
    for put in batch {
        if !seen.insert(&put.id) {
            return Err(Refusal::Duplicate);
        }
        if put.body.as_ref().is_some_and(|b| b.len() > MAX_RECORD_BYTES) {
            return Err(Refusal::TooLarge(Limit::Record));
        }
    }
    let (mut rev, mut rows, mut bytes) = (rev, totals.rows, totals.bytes);
    let mut items = Vec::with_capacity(batch.len());
    for put in batch {
        let at = put.edited_at.min(now);
        let row = current.get(&put.id);
        if let Some(m) = row.filter(|m| m.rev != put.base_rev && at <= m.edited_at) {
            items.push(Item { verdict: Verdict::Superseded, rev: m.rev, at: m.edited_at });
            continue;
        }
        rev += 1;
        match row {
            None => rows += 1,
            Some(m) => bytes -= m.size,
        }
        bytes += put.sealed_len();
        items.push(Item { verdict: Verdict::Accepted, rev, at });
    }
    if rows > MAX_RECORDS {
        return Err(Refusal::TooLarge(Limit::Records));
    }
    if bytes > MAX_SEALED_BYTES {
        return Err(Refusal::TooLarge(Limit::Bytes));
    }
    Ok(Plan { rev, items })
}

/// Why a work call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WorkError {
    #[error("the store is unavailable")]
    Unavailable,
    #[error("a stored value could not be read")]
    Corrupt,
    #[error("the account no longer exists")]
    Gone,
    #[error("the account does not store its work on the server")]
    StorageOff,
    #[error("the device is too far behind: pull everything")]
    Resync,
    #[error("the account's copy was turned off and on again: the device's revisions mean nothing to it")]
    Reset,
    #[error("over a limit")]
    TooLarge(Limit),
    #[error("a place appears twice in the batch")]
    Duplicate,
}

impl From<Refusal> for WorkError {
    fn from(r: Refusal) -> WorkError {
        match r {
            Refusal::TooLarge(limit) => WorkError::TooLarge(limit),
            Refusal::Duplicate => WorkError::Duplicate,
        }
    }
}

impl From<crate::accounts::RepoError> for WorkError {
    fn from(e: crate::accounts::RepoError) -> WorkError {
        use crate::accounts::RepoError;
        match e {
            RepoError::Unavailable => WorkError::Unavailable,
            RepoError::Gone => WorkError::Gone,
            RepoError::Corrupt | RepoError::Duplicate => WorkError::Corrupt,
        }
    }
}

/// A row as served: the body opened (plaintext JSON), or `unreadable` when it did not open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: Id,
    pub rev: i64,
    pub edited_at: OffsetDateTime,
    pub deleted: bool,
    pub body: Option<Vec<u8>>,
    pub unreadable: bool,
}

/// What a pull asks. `since` moves on from page to page; `start` is where the whole pull began, and it is what the
/// server judges the device's right to its revisions by (a tombstone purged while a long pull pages must not make
/// the later pages start over). `epoch` is the account's copy the device last read, if it knows one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ask {
    pub since: i64,
    pub start: i64,
    pub epoch: Option<Uuid>,
}

impl Ask {
    /// The first page of a pull, from revision `since`.
    pub fn first(since: i64) -> Ask {
        Ask { since, start: since, epoch: None }
    }

    /// The page after one that said `more`, from revision `since`; the pull still began where it did.
    pub fn continued(self, since: i64) -> Ask {
        Ask { since, ..self }
    }

    pub fn with_epoch(self, epoch: Uuid) -> Ask {
        Ask { epoch: Some(epoch), ..self }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pulled {
    /// The account's copy: new each time storage is turned on.
    pub epoch: Uuid,
    /// The last revision in `records` when `more`, else the account's revision.
    pub rev: i64,
    pub more: bool,
    pub records: Vec<Record>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushedItem {
    pub id: Id,
    pub verdict: Verdict,
    pub rev: i64,
    /// The server's record, when superseded.
    pub record: Option<Record>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pushed {
    pub rev: i64,
    pub items: Vec<PushedItem>,
}

/// The account's work, sealed at rest. Every call needs `store_on_server`, checked inside its transaction.
#[async_trait]
pub trait WorkRepo: Send + Sync {
    /// The records changed after `ask.since`, in revision order, at most `PAGE`. A pull that began at 0 (`ask.start`)
    /// gets every live record and no tombstone. `Reset` when `ask.epoch` is not the account's; `Resync` when
    /// `ask.start` is older than a purged tombstone or `ask.since` is above the account's revision.
    async fn pull(&self, user: Uuid, ask: Ask, now: OffsetDateTime) -> Result<Pulled, WorkError>;
    /// Applies a batch whole or not at all.
    async fn push(&self, user: Uuid, batch: Vec<Put>, now: OffsetDateTime) -> Result<Pushed, WorkError>;
    /// Deletes every row and resets the revision.
    async fn clear(&self, user: Uuid) -> Result<(), WorkError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_800_000_000 + secs).unwrap()
    }

    fn id(key: &str) -> Id {
        Id { kind: Kind::Street, key: key.into() }
    }

    fn put(key: &str, base_rev: i64, edited: OffsetDateTime) -> Put {
        Put { id: id(key), base_rev, edited_at: edited, deleted: false, body: Some(b"{}".to_vec()) }
    }

    fn meta(rev: i64, edited: OffsetDateTime) -> Meta {
        Meta { rev, edited_at: edited, deleted: false, size: 2 + SEAL_OVERHEAD }
    }

    fn plan_at(rows: &[(&str, Meta)], batch: &[Put], now: OffsetDateTime) -> Result<Plan, Refusal> {
        let current: BTreeMap<Id, Meta> = rows.iter().map(|(k, m)| (id(k), m.clone())).collect();
        let totals = Totals { rows: current.len(), bytes: current.values().map(|m| m.size).sum() };
        let rev = current.values().map(|m| m.rev).max().unwrap_or(0);
        plan_batch(rev, totals, &current, batch, now)
    }

    fn plan(rows: &[(&str, Meta)], batch: &[Put]) -> Result<Plan, Refusal> {
        plan_at(rows, batch, at(100))
    }

    #[test]
    fn a_record_with_no_row_is_accepted_and_takes_the_next_revision() {
        let p = plan(&[], &[put("a", 0, at(0)), put("b", 0, at(0))]).unwrap();
        assert_eq!(p.rev, 2);
        assert_eq!(p.items.iter().map(|i| (i.verdict, i.rev)).collect::<Vec<_>>(), [(Verdict::Accepted, 1), (Verdict::Accepted, 2)]);
    }

    /// Every case of `tests/data/accept_rules.json`, which the app's `ServerSim` reads too.
    #[derive(serde::Deserialize)]
    struct TableRow {
        rev: i64,
        edited_at: i64,
    }
    #[derive(serde::Deserialize)]
    struct TablePut {
        base_rev: i64,
        edited_at: i64,
    }
    #[derive(serde::Deserialize)]
    struct TableCase {
        name: String,
        row: Option<TableRow>,
        put: TablePut,
        now: i64,
        verdict: String,
        rev: i64,
        at: i64,
    }

    #[test]
    fn the_shared_truth_table_decides_every_case() {
        let cases: Vec<TableCase> = serde_json::from_str(include_str!("../tests/data/accept_rules.json")).unwrap();
        assert!(cases.len() >= 7, "the table is not empty");
        for c in cases {
            let rows: Vec<(&str, Meta)> = c.row.iter().map(|r| ("a", meta(r.rev, at(r.edited_at)))).collect();
            let p = plan_at(&rows, &[put("a", c.put.base_rev, at(c.put.edited_at))], at(c.now)).unwrap();
            let verdict = match p.items[0].verdict {
                Verdict::Accepted => "accepted",
                Verdict::Superseded => "superseded",
            };
            assert_eq!((verdict, p.items[0].rev, p.items[0].at), (c.verdict.as_str(), c.rev, at(c.at)), "{}", c.name);
        }
    }

    #[test]
    fn a_superseded_item_does_not_take_a_revision() {
        let p = plan(&[("a", meta(7, at(50)))], &[put("a", 3, at(10)), put("b", 0, at(10))]).unwrap();
        assert_eq!(p.items.iter().map(|i| (i.verdict, i.rev)).collect::<Vec<_>>(), [(Verdict::Superseded, 7), (Verdict::Accepted, 8)]);
        assert_eq!(p.rev, 8);
    }

    #[test]
    fn deleting_a_place_the_server_never_had_leaves_a_tombstone() {
        let mut d = put("a", 0, at(0));
        d.deleted = true;
        d.body = None;
        let p = plan(&[], &[d]).unwrap();
        assert_eq!((p.items[0].verdict, p.rev), (Verdict::Accepted, 1));
    }

    #[test]
    fn a_batch_is_refused_whole_for_the_limit_it_passes() {
        let big = |n: usize| Put { body: Some(vec![b' '; n]), ..put("a", 0, at(0)) };
        assert_eq!(plan(&[], &[big(MAX_RECORD_BYTES + 1)]).unwrap_err(), Refusal::TooLarge(Limit::Record));
        assert!(plan(&[], &[big(MAX_RECORD_BYTES)]).is_ok(), "exactly 64 KB is allowed");
        let many: Vec<Put> = (0..MAX_BATCH + 1).map(|i| put(&format!("k{i}"), 0, at(0))).collect();
        assert_eq!(plan(&[], &many).unwrap_err(), Refusal::TooLarge(Limit::Batch));
        assert_eq!(plan(&[], &[put("a", 0, at(0)), put("a", 0, at(1))]).unwrap_err(), Refusal::Duplicate);
        // 5,000 rows already: a new place passes the record limit, a change to an old one does not
        let full: Vec<(String, Meta)> = (0..MAX_RECORDS).map(|i| (format!("k{i}"), meta(i as i64 + 1, at(0)))).collect();
        let rows: Vec<(&str, Meta)> = full.iter().map(|(k, m)| (k.as_str(), m.clone())).collect();
        assert_eq!(plan(&rows, &[put("new", 0, at(1))]).unwrap_err(), Refusal::TooLarge(Limit::Records));
        assert!(plan(&rows, &[put("k0", 1, at(1))]).is_ok());
        // 2 MB of sealed bodies: 35 records of 60,000 bytes do not fit
        let sixty: Vec<Put> = (0..35).map(|i| Put { body: Some(vec![b' '; 60_000]), ..put(&format!("s{i}"), 0, at(0)) }).collect();
        assert_eq!(plan(&[], &sixty[..34]).map(|p| p.rev), Ok(34));
        assert_eq!(plan(&[], &sixty).unwrap_err(), Refusal::TooLarge(Limit::Bytes));
    }

    #[test]
    fn the_wire_form_is_checked_for_what_the_server_must_know() {
        let ok = |kind: &str, key: &str, time: &str, deleted: bool, body: Option<&str>| Put::from_wire(kind, key, 0, time, deleted, body);
        let t = "2027-01-15T08:00:00.000Z";
        assert!(ok("street", "w2000~n1~n2", t, false, Some("{\"a\":1}")).is_ok());
        assert!(ok("area", "current", t, false, Some("{}")).is_ok());
        assert!(ok("rule", "r-0123456789abcdef0123456789abcdef", t, true, None).is_ok());
        assert_eq!(ok("shape", "k", t, false, Some("{}")).unwrap_err(), Invalid::Kind);
        for key in ["", "../etc", "a/b", "a b", "a\u{e9}", &"k".repeat(MAX_KEY + 1)] {
            assert_eq!(ok("street", key, t, false, Some("{}")).unwrap_err(), Invalid::Key, "{key:?}");
        }
        assert!(ok("street", &"k".repeat(MAX_KEY), t, false, Some("{}")).is_ok());
        assert_eq!(ok("street", "k", "yesterday", false, Some("{}")).unwrap_err(), Invalid::Time);
        assert_eq!(ok("street", "k", t, false, None).unwrap_err(), Invalid::Body, "a live record has a body");
        assert_eq!(ok("street", "k", t, true, Some("{}")).unwrap_err(), Invalid::Body, "a tombstone has none");
        assert_eq!(Put::from_wire("street", "k", -1, t, true, None).unwrap_err(), Invalid::BaseRev);
    }

    #[test]
    fn times_are_read_with_any_offset_and_written_in_utc_to_the_millisecond() {
        let parsed = parse_stamp("2027-01-15T03:00:00.5-05:00").unwrap();
        assert_eq!(stamp(parsed), "2027-01-15T08:00:00.500Z");
        assert_eq!(parse_stamp("2027-01-15"), None);
    }

    #[test]
    fn a_pull_goes_on_from_where_it_began() {
        let first = Ask::first(7);
        assert_eq!((first.since, first.start, first.epoch), (7, 7, None));
        let epoch = Uuid::parse_str("3f2b0000-0000-4000-8000-000000000002").unwrap();
        let next = first.with_epoch(epoch).continued(507);
        assert_eq!((next.since, next.start, next.epoch), (507, 7, Some(epoch)));
    }

    #[test]
    fn the_associated_data_names_the_kind_the_key_and_the_account() {
        let user = Uuid::parse_str("3f2b0000-0000-4000-8000-000000000001").unwrap();
        assert_eq!(aad(&id("w1~n2~n3"), user), "work.street.w1~n2~n3.3f2b0000-0000-4000-8000-000000000001");
    }
}
