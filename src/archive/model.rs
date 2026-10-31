//! The portable design archive: one JSON document that carries a person's design work (the street and junction edits
//! and the area rules) out of a browser and into another, with the rules of reading one safely. No I/O of its own: it
//! reads and writes through the `Storage` port, and everything it refuses it says as `Said` data.
//!
//! **The format** (`format_version` 1; UNVERIFIED, decided for issue #74 and not reviewed against any outside
//! convention):
//!
//! ```json
//! { "kind": "cityloom-design", "format_version": 1, "app_version": "0.43.0", "created_at": "2026-10-31T12:00:00.000Z",
//!   "source": { "area_key": "45.5230,-73.5810", "tile_checksum": "…+i7", "importer_version": 7 },
//!   "area": "area=45.5230,-73.5810&name=Plateau", "edits": { …the `cityloom-edits` document… },
//!   "restrictions": { …the `cityloom-restrictions` document… } }
//! ```
//!
//! `edits` and `restrictions` are the stores' own documents, taken as raw JSON, so a record this build cannot read and
//! a field it does not know travel unchanged. The edits are written at the version their content needs
//! (`Draft::to_json`) and read from `edits::VERSION - 1` to `edits::NEWEST`; the restrictions are version 1.
//!
//! **Versioning.** `format_version` is an integer. A reader accepts exactly the versions it knows (1) and refuses a
//! higher one as "from a newer CityLoom"; a change that old readers could misread must raise it, a change that only adds
//! optional fields does not. **Unknown fields** anywhere in the envelope are ignored on read, never an error, which is
//! how a later 1.x archive stays readable. `notes` and `scenarios` are reserved names for per-place notes and design
//! scenarios, which do not exist yet: version 1 neither writes nor reads them.
//!
//! **What is not in it:** the sync document (`cityloom-sync`), anything of the account or the session, the language and
//! theme, and each area's street network (`cityloom-network:<key>`, which the area's roads re-derive from OpenStreetMap).
//! `area` and `source` say what the design was made on; importing never changes the chosen area.
//!
//! **Importing never overwrites silently.** `plan_import` compares the archive with local work place by place:
//! `New` (not here), `Same`, or `Conflict` (here, and different). `apply` writes the new ones, and a conflict only when
//! the person chose to use the archive's; the choice is made once for the whole import (UNVERIFIED: not per place).
//! Everything is checked before the first write, a write that fails is rolled back, and a local document that is
//! damaged or from a newer build (and so cannot be merged into safely) stops the import with local data untouched.
//! Written records are marked for the sync like any edit.

use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::{Map, Value};

use crate::city::edits::{self, Document, Draft, EDITS_KEY, JUNCTIONS, NEWEST, STREETS, VERSION as EDITS_VERSION};
use crate::city::store::{CHECKSUM_KEY, CityStore};
use crate::place::tiles::IMPORTER_VERSION;
use crate::restriction::store::{self as rules, RESTRICTIONS_KEY, RestrictionStore, Restrictions};
use crate::shared::ports::Storage;
use crate::shared::said::{Arg, Said};
use crate::sync::model::{Kind, mark, valid_key};

/// The `kind` field of every archive.
pub const MAGIC: &str = "cityloom-design";
/// The only format this build writes, and the newest it reads.
pub const FORMAT_VERSION: u32 = 1;
/// The suffix of an archive's file name. Not a bare `.cityloom`: that is the area archive `cityloom_prepare` builds.
pub const FILE_SUFFIX: &str = ".cityloom-design.json";
/// The most text an archive may hold, in bytes (UNVERIFIED): the browser's storage cannot hold more than this of work,
/// so a larger file is not an archive of ours.
pub const MAX_BYTES: usize = 10_000_000;
/// The oldest edits document a reader takes (the oldest this build reads from its own storage).
const OLDEST_EDITS: u64 = (EDITS_VERSION - 1) as u64;
/// The restrictions document's version.
const RULES_VERSION: u64 = rules::VERSION as u64;

/// Why an archive was not read, or not imported. Nothing was changed in any of these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchiveError {
    /// Larger than `MAX_BYTES`.
    TooLarge,
    NotJson,
    /// JSON, but not a CityLoom design archive.
    NotAnArchive,
    /// An archive that does not hold together; the part says where (a message key: the header, the edits, the rules).
    Damaged(&'static str),
    /// `format_version` above what this build knows.
    FutureFormat(u64),
    /// The embedded edits are from a newer build than this one.
    EditsNewer(u64),
    /// The embedded edits are older than this build reads.
    EditsOld(u64),
    /// The embedded restrictions are from a newer build.
    RulesNewer(u64),
    /// This browser's edits cannot take new records: damaged, or written by a newer build (`newer`).
    LocalEdits {
        newer: bool,
    },
    /// The same for this browser's area rules.
    LocalRules {
        newer: bool,
    },
    /// The browser's storage refused a write; what was written was put back.
    StorageRefused,
    /// There is nothing of this browser's work to put in an archive, or it cannot be read.
    ExportUnreadable,
}

pub const PART_HEADER: &str = "archive-part-header";
pub const PART_EDITS: &str = "archive-part-edits";
pub const PART_RULES: &str = "archive-part-rules";

impl ArchiveError {
    /// What to tell the person. Each message says that local work was left as it was.
    pub fn said(&self) -> Said {
        match self {
            ArchiveError::TooLarge => Said::new("archive-error-too-large").with("megabytes", Arg::Num((MAX_BYTES / 1_000_000) as i64)),
            ArchiveError::NotJson => Said::new("archive-error-not-json"),
            ArchiveError::NotAnArchive => Said::new("archive-error-not-archive"),
            ArchiveError::Damaged(part) => Said::new("archive-error-damaged").with("part", Arg::MsgLower(part)),
            ArchiveError::FutureFormat(v) => Said::new("archive-error-future-format").with("version", Arg::Num(*v as i64)),
            ArchiveError::EditsNewer(v) => Said::new("archive-error-edits-newer").with("version", Arg::Num(*v as i64)),
            ArchiveError::EditsOld(v) => Said::new("archive-error-edits-old").with("version", Arg::Num(*v as i64)),
            ArchiveError::RulesNewer(v) => Said::new("archive-error-rules-newer").with("version", Arg::Num(*v as i64)),
            ArchiveError::LocalEdits { newer: true } => Said::new("archive-error-local-edits-newer"),
            ArchiveError::LocalEdits { newer: false } => Said::new("archive-error-local-edits-damaged"),
            ArchiveError::LocalRules { newer: true } => Said::new("archive-error-local-rules-newer"),
            ArchiveError::LocalRules { newer: false } => Said::new("archive-error-local-rules-damaged"),
            ArchiveError::StorageRefused => Said::new("archive-error-storage"),
            ArchiveError::ExportUnreadable => Said::new("archive-error-export"),
        }
    }
}

/// What the design was made on. All of it is informational: importing never acts on it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    pub area_key: Option<String>,
    pub tile_checksum: Option<String>,
    pub importer_version: Option<u32>,
}

/// A design archive that was read and found whole. Its records are held as raw JSON.
#[derive(Clone, Debug, PartialEq)]
pub struct Archive {
    pub format_version: u32,
    pub app_version: String,
    pub created_at: String,
    pub source: Source,
    /// The area the person had chosen, as its query string.
    pub area: Option<String>,
    streets: BTreeMap<String, Value>,
    junctions: BTreeMap<String, Value>,
    rules: BTreeMap<String, Value>,
}

impl Archive {
    pub fn streets(&self) -> usize {
        self.streets.len()
    }

    pub fn junctions(&self) -> usize {
        self.junctions.len()
    }

    pub fn rules(&self) -> usize {
        self.rules.len()
    }

    /// Whether it carries any work at all.
    pub fn is_empty(&self) -> bool {
        self.streets.is_empty() && self.junctions.is_empty() && self.rules.is_empty()
    }

    fn records(&self, kind: Kind) -> &BTreeMap<String, Value> {
        match kind {
            Kind::Street => &self.streets,
            Kind::Junction => &self.junctions,
            Kind::Rule | Kind::Area => &self.rules,
        }
    }
}

// ---- export ----

/// The archive of this browser's work as text. Fails only where the edits or the rules cannot be read (damaged, or
/// from a newer build): an archive made then would not hold the work.
pub fn export(storage: &Rc<dyn Storage>) -> Result<String, ArchiveError> {
    let mut doc = Map::new();
    doc.insert("kind".into(), MAGIC.into());
    doc.insert("format_version".into(), FORMAT_VERSION.into());
    doc.insert("app_version".into(), env!("CARGO_PKG_VERSION").into());
    doc.insert("created_at".into(), iso(storage.now_ms()).into());
    let area = CityStore::current_area(&**storage);
    let mut source = Map::new();
    source.insert("area_key".into(), area.key().into());
    if let Some(sum) = storage.recall(&format!("{CHECKSUM_KEY}:{}", area.key())).filter(|s| !s.is_empty()) {
        source.insert("tile_checksum".into(), sum.into());
    }
    source.insert("importer_version".into(), IMPORTER_VERSION.into());
    doc.insert("source".into(), Value::Object(source));
    doc.insert("area".into(), area.to_query().into());
    match edits::draft(storage.recall(EDITS_KEY).as_deref()) {
        Ok(draft) if !(draft.raw_keys(STREETS).is_empty() && draft.raw_keys(JUNCTIONS).is_empty()) => {
            doc.insert("edits".into(), serde_json::from_str(&draft.to_json()).map_err(|_| ArchiveError::ExportUnreadable)?);
        }
        Ok(_) | Err(Document::Empty) => {}
        Err(_) => return Err(ArchiveError::ExportUnreadable),
    }
    match rules::parse(storage.recall(RESTRICTIONS_KEY).as_deref()) {
        rules::Document::Current(r) if !RestrictionStore::new(storage.clone()).raw_ids().is_empty() => {
            doc.insert("restrictions".into(), serde_json::from_str(&r.to_json()).map_err(|_| ArchiveError::ExportUnreadable)?);
        }
        rules::Document::Current(_) | rules::Document::Empty => {}
        _ => return Err(ArchiveError::ExportUnreadable),
    }
    Ok(serde_json::to_string_pretty(&Value::Object(doc)).expect("an archive serialises"))
}

/// The file name an archive made at `now_ms` is offered under.
pub fn file_name(now_ms: u64) -> String {
    let day: String = iso(now_ms).chars().take(10).collect();
    format!("cityloom-{day}{FILE_SUFFIX}")
}

/// `ms` since 1970 as `2026-10-31T12:00:00.000Z`.
fn iso(ms: u64) -> String {
    let (secs, millis) = (ms / 1000, ms % 1000);
    let (days, rest) = ((secs / 86_400) as i64, secs % 86_400);
    // civil-from-days (H. Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{millis:03}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

// ---- reading ----

/// Reads `text` as an archive and checks it whole: nothing is kept until every part has been found sound.
pub fn parse_and_validate(text: &str) -> Result<Archive, ArchiveError> {
    if text.len() > MAX_BYTES {
        return Err(ArchiveError::TooLarge);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| ArchiveError::NotJson)?;
    let Some(doc) = value.as_object() else { return Err(ArchiveError::NotAnArchive) };
    if doc.get("kind").and_then(Value::as_str) != Some(MAGIC) {
        return Err(ArchiveError::NotAnArchive);
    }
    let version = match doc.get("format_version").and_then(Value::as_u64) {
        Some(0) | None => return Err(ArchiveError::Damaged(PART_HEADER)),
        Some(v) if v > u64::from(FORMAT_VERSION) => return Err(ArchiveError::FutureFormat(v)),
        Some(v) => v as u32,
    };
    let text_of = |name: &str| match doc.get(name) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(ArchiveError::Damaged(PART_HEADER)),
    };
    let (app_version, created_at) = (text_of("app_version")?, text_of("created_at")?);
    let area = Some(text_of("area")?).filter(|a| !a.is_empty());
    let source = match doc.get("source") {
        None | Some(Value::Null) => Source::default(),
        Some(Value::Object(s)) => Source {
            area_key: s.get("area_key").and_then(Value::as_str).map(str::to_string),
            tile_checksum: s.get("tile_checksum").and_then(Value::as_str).map(str::to_string),
            importer_version: s.get("importer_version").and_then(Value::as_u64).and_then(|v| u32::try_from(v).ok()),
        },
        Some(_) => return Err(ArchiveError::Damaged(PART_HEADER)),
    };
    let (streets, junctions) = read_edits(doc.get("edits"))?;
    let rules = read_rules(doc.get("restrictions"))?;
    Ok(Archive { format_version: version, app_version, created_at, source, area, streets, junctions, rules })
}

type Records = BTreeMap<String, Value>;

/// The records of one section, each of which has to be an object under a key the sync can carry.
fn section(draft: &Draft, name: &str, part: &'static str) -> Result<Records, ArchiveError> {
    let mut out = Records::new();
    for key in draft.raw_keys(name) {
        let record = draft.get_raw(name, &key).filter(|r| r.is_object()).ok_or(ArchiveError::Damaged(part))?;
        if !valid_key(&key) {
            return Err(ArchiveError::Damaged(part));
        }
        out.insert(key, record.clone());
    }
    Ok(out)
}

fn read_edits(value: Option<&Value>) -> Result<(Records, Records), ArchiveError> {
    let Some(value) = value.filter(|v| !v.is_null()) else { return Ok((Records::new(), Records::new())) };
    let damaged = ArchiveError::Damaged(PART_EDITS);
    let Some(doc) = value.as_object() else { return Err(damaged) };
    match doc.get("version").and_then(Value::as_u64) {
        None => return Err(damaged),
        Some(v) if v > u64::from(NEWEST) => return Err(ArchiveError::EditsNewer(v)),
        Some(v) if v < OLDEST_EDITS => return Err(ArchiveError::EditsOld(v)),
        Some(_) => {}
    }
    match edits::draft(Some(&value.to_string())) {
        Ok(draft) => Ok((section(&draft, STREETS, PART_EDITS)?, section(&draft, JUNCTIONS, PART_EDITS)?)),
        Err(Document::Future) => Err(ArchiveError::EditsNewer(NEWEST as u64 + 1)),
        Err(_) => Err(damaged),
    }
}

fn read_rules(value: Option<&Value>) -> Result<Records, ArchiveError> {
    let Some(value) = value.filter(|v| !v.is_null()) else { return Ok(Records::new()) };
    let damaged = ArchiveError::Damaged(PART_RULES);
    let Some(doc) = value.as_object() else { return Err(damaged) };
    match doc.get("version").and_then(Value::as_u64) {
        None => return Err(damaged),
        Some(v) if v > RULES_VERSION => return Err(ArchiveError::RulesNewer(v)),
        Some(v) if v < RULES_VERSION => return Err(damaged),
        Some(_) => {}
    }
    if !matches!(rules::parse(Some(&value.to_string())), rules::Document::Current(_)) {
        return Err(damaged);
    }
    let mut out = Records::new();
    for (id, record) in doc["restrictions"].as_object().into_iter().flatten() {
        if !record.is_object() || !valid_key(id) {
            return Err(damaged);
        }
        out.insert(id.clone(), record.clone());
    }
    Ok(out)
}

// ---- comparing ----

/// How an archived place stands against local work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not in this browser: importing adds it.
    New,
    /// Here already, the same: nothing to do.
    Same,
    /// Here already, and different: importing leaves it unless the person chooses the archive's.
    Conflict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    pub key: String,
    pub status: Status,
}

/// Every record of an archive against local work. Computed from the storage as it is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportPlan {
    pub entries: Vec<Entry>,
}

impl ImportPlan {
    pub fn count(&self, status: Status) -> usize {
        self.entries.iter().filter(|e| e.status == status).count()
    }

    /// The keys that differ, for the confirmation to name (streets and junctions by their keys, rules by id).
    pub fn conflicts(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| e.status == Status::Conflict)
    }
}

/// The local edits to compare with and write into; an error where they cannot take new records.
fn local_edits(storage: &Rc<dyn Storage>) -> Result<Draft, ArchiveError> {
    match edits::draft(storage.recall(EDITS_KEY).as_deref()) {
        Ok(draft) => Ok(draft),
        Err(Document::Empty) => Ok(Draft::new()),
        Err(Document::Future) => Err(ArchiveError::LocalEdits { newer: true }),
        Err(_) => Err(ArchiveError::LocalEdits { newer: false }),
    }
}

fn local_rules(storage: &Rc<dyn Storage>) -> Result<RestrictionStore, ArchiveError> {
    let store = RestrictionStore::new(storage.clone());
    if store.is_usable() { Ok(store) } else { Err(ArchiveError::LocalRules { newer: store.is_future() }) }
}

/// Compares `archive` with this browser's work. A local document that cannot take records is an error only when the
/// archive has records for it.
pub fn plan_import(archive: &Archive, storage: &Rc<dyn Storage>) -> Result<ImportPlan, ArchiveError> {
    let mut entries = Vec::new();
    let wants_edits = !archive.streets.is_empty() || !archive.junctions.is_empty();
    if wants_edits {
        let draft = local_edits(storage)?;
        for (kind, section) in [(Kind::Street, STREETS), (Kind::Junction, JUNCTIONS)] {
            for (key, record) in archive.records(kind) {
                entries.push(entry(kind, key, draft.get_raw(section, key), record));
            }
        }
    }
    if !archive.rules.is_empty() {
        let store = local_rules(storage)?;
        for (id, record) in &archive.rules {
            entries.push(entry(Kind::Rule, id, store.raw(id).as_ref(), record));
        }
    }
    Ok(ImportPlan { entries })
}

fn entry(kind: Kind, key: &str, local: Option<&Value>, archived: &Value) -> Entry {
    let status = match local {
        None => Status::New,
        Some(here) if here == archived => Status::Same,
        Some(_) => Status::Conflict,
    };
    Entry { kind, key: key.to_string(), status }
}

// ---- applying ----

/// What the person decided about the places that differ, once for the whole import.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Add the new ones; leave every place that differs as it is here.
    KeepMine,
    /// Add the new ones, and replace the places that differ with the archive's.
    UseArchive,
}

/// What an import did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub added: usize,
    pub replaced: usize,
    /// Places that differ and were left as they are here.
    pub kept: usize,
    pub same: usize,
}

/// Imports `archive` as `choice` says. The plan is made again from the storage as it is now; every document is
/// checked before the first write, and if a write fails the ones made are put back.
pub fn apply(archive: &Archive, storage: &Rc<dyn Storage>, choice: Choice) -> Result<Applied, ArchiveError> {
    let plan = plan_import(archive, storage)?;
    let mut done = Applied { same: plan.count(Status::Same), ..Applied::default() };
    let mut writes: Vec<&Entry> = Vec::new();
    for e in &plan.entries {
        match (e.status, choice) {
            (Status::New, _) => {
                done.added += 1;
                writes.push(e);
            }
            (Status::Conflict, Choice::UseArchive) => {
                done.replaced += 1;
                writes.push(e);
            }
            (Status::Conflict, Choice::KeepMine) => done.kept += 1,
            (Status::Same, _) => {}
        }
    }
    if writes.is_empty() {
        return Ok(done);
    }
    let record = |e: &Entry| archive.records(e.kind)[&e.key].clone();
    let (before_rules, before_edits) = (storage.recall(RESTRICTIONS_KEY), storage.recall(EDITS_KEY));
    let rule_writes: Vec<&&Entry> = writes.iter().filter(|e| e.kind == Kind::Rule).collect();
    let edit_writes: Vec<&&Entry> = writes.iter().filter(|e| e.kind != Kind::Rule).collect();
    let put_back = |key: &str, before: &Option<String>, empty: String| {
        storage.remember(key, before.as_deref().unwrap_or(&empty));
    };
    if !rule_writes.is_empty() {
        let store = local_rules(storage)?;
        for e in &rule_writes {
            if !store.put_raw(&e.key, record(e)) {
                put_back(RESTRICTIONS_KEY, &before_rules, Restrictions::new().to_json());
                return Err(ArchiveError::StorageRefused);
            }
        }
    }
    if !edit_writes.is_empty() {
        let mut draft = local_edits(storage)?;
        let kept = edit_writes.iter().all(|e| draft.put_raw(if e.kind == Kind::Street { STREETS } else { JUNCTIONS }, &e.key, record(e)));
        if !(kept && storage.remember(EDITS_KEY, &draft.to_json())) {
            if !rule_writes.is_empty() {
                put_back(RESTRICTIONS_KEY, &before_rules, Restrictions::new().to_json());
            }
            put_back(EDITS_KEY, &before_edits, Draft::new().to_json());
            return Err(ArchiveError::StorageRefused);
        }
    }
    for e in &writes {
        mark(&**storage, e.kind, &e.key, false);
    }
    Ok(done)
}
