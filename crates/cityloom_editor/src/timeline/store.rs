//! One project's milestones as this browser keeps them: a document of records, each the milestone's header and its
//! snapshot, read again and changed one record at a time. Records are held as JSON, so a field a newer build adds
//! survives a rename. Takes the project's storage (`ScopedStorage`), never the root.

use std::collections::BTreeMap;
use std::rc::Rc;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::shared::ports::Storage;
use crate::shared::project_id::{FIRST, split_key};
use crate::sync::model::{Id, Kind, Loaded, load, mark, stamp};
use crate::timeline::model::{Draft, MAX_BYTES, MAX_TOTAL_BYTES, Milestone, Refused, Snapshot, clean_name};

/// The shape of the document. Raised when the shape changes.
pub const VERSION: u32 = 1;
/// Where the document is kept (scoped: `cityloom-milestones@<project>`).
pub const MILESTONES_KEY: &str = "cityloom-milestones";
/// Where a document that could not be read is copied before the first write replaces it.
pub const DAMAGED_KEY: &str = "cityloom-milestones-damaged";

struct Doc {
    records: BTreeMap<String, Value>,
    /// The document's own keys other than the two it is made of, as read.
    others: Map<String, Value>,
}

impl Doc {
    fn to_json(&self) -> String {
        let mut doc = self.others.clone();
        doc.insert("version".into(), VERSION.into());
        doc.insert("milestones".into(), Value::Object(self.records.iter().map(|(k, v)| (k.clone(), v.clone())).collect()));
        Value::Object(doc).to_string()
    }
}

enum Document {
    Empty,
    Current(Doc),
    /// Written by a newer build: not read, and never written over.
    Future,
    /// Not a document of this version; the text is kept so it can be copied aside.
    Damaged(String),
}

fn parse(stored: Option<&str>) -> Document {
    let Some(raw) = stored.filter(|s| !s.trim().is_empty()) else { return Document::Empty };
    let damaged = || Document::Damaged(raw.to_string());
    let Ok(Value::Object(mut whole)) = serde_json::from_str::<Value>(raw) else { return damaged() };
    match whole.remove("version") {
        Some(Value::Number(n)) if n.as_u64() == Some(u64::from(VERSION)) => {}
        Some(Value::Number(n)) if n.as_u64().is_some_and(|v| v > u64::from(VERSION)) => return Document::Future,
        _ => return damaged(),
    }
    let Some(Value::Object(records)) = whole.remove("milestones") else { return damaged() };
    Document::Current(Doc { records: records.into_iter().collect(), others: whole })
}

/// The header of a record, read as the timeline lists it; the key names the milestone.
fn header(id: &str, record: &Value) -> Option<Milestone> {
    let mut m = Milestone::deserialize(record).ok()?;
    m.id = id.to_string();
    Some(m)
}

/// A record as it is kept: the header and the snapshot.
fn record_of(m: &Milestone, snapshot: &Snapshot) -> Value {
    let Value::Object(mut fields) = serde_json::to_value(m).expect("a milestone serialises") else { unreachable!("a milestone is an object") };
    fields.remove("id");
    if let Some(edits) = &snapshot.edits {
        fields.insert("edits".into(), edits.clone());
    }
    if let Some(restrictions) = &snapshot.restrictions {
        fields.insert("restrictions".into(), restrictions.clone());
    }
    Value::Object(fields)
}

/// The milestones of one project.
#[derive(Clone)]
pub struct MilestoneStore {
    storage: Rc<dyn Storage>,
}

impl MilestoneStore {
    pub fn new(storage: Rc<dyn Storage>) -> MilestoneStore {
        MilestoneStore { storage }
    }

    fn document(&self) -> Document {
        parse(self.storage.recall(MILESTONES_KEY).as_deref())
    }

    /// Whether the document can be read and changed: none kept, or one of this version.
    pub fn is_usable(&self) -> bool {
        matches!(self.document(), Document::Empty | Document::Current(_))
    }

    /// Applies `change` to the document as it is kept now, and keeps the result. A damaged document is copied aside
    /// first; a newer build's is left alone.
    fn write<R>(&self, change: impl FnOnce(&mut Doc) -> Result<R, Refused>) -> Result<R, Refused> {
        let mut doc = match self.document() {
            Document::Future => return Err(Refused::Unusable),
            Document::Empty => Doc { records: BTreeMap::new(), others: Map::new() },
            Document::Current(doc) => doc,
            Document::Damaged(raw) => {
                // The first copy may hold what can be recovered: a later one does not replace it.
                if self.storage.recall(DAMAGED_KEY).is_none() && !self.storage.remember(DAMAGED_KEY, &raw) {
                    return Err(Refused::Unusable);
                }
                Doc { records: BTreeMap::new(), others: Map::new() }
            }
        };
        let done = change(&mut doc)?;
        if self.storage.remember(MILESTONES_KEY, &doc.to_json()) { Ok(done) } else { Err(Refused::Unusable) }
    }

    /// Every milestone, oldest first (equal times by id); none where the document is a newer build's or damaged. A
    /// record this build cannot read is left out.
    pub fn list(&self) -> Vec<Milestone> {
        let Document::Current(doc) = self.document() else { return Vec::new() };
        let mut all: Vec<Milestone> = doc.records.iter().filter_map(|(id, record)| header(id, record)).collect();
        all.sort_by(|a, b| (&a.created_at, &a.id).cmp(&(&b.created_at, &b.id)));
        all
    }

    pub fn get(&self, id: &str) -> Option<Milestone> {
        let Document::Current(doc) = self.document() else { return None };
        header(id, doc.records.get(id)?)
    }

    /// What milestone `id` holds. None where there is no such milestone; a snapshot with neither document where it
    /// holds no work.
    pub fn snapshot(&self, id: &str) -> Option<Snapshot> {
        let Document::Current(doc) = self.document() else { return None };
        let record = doc.records.get(id)?;
        Some(Snapshot { edits: record.get("edits").cloned(), restrictions: record.get("restrictions").cloned() })
    }

    /// The id a new milestone takes: random where the storage has randomness, else `m<n>` past the highest number the
    /// document or the sync document has held (a deleted milestone's id stays dead).
    fn fresh_id(&self, doc: &Doc) -> String {
        let mut known: Vec<String> = doc.records.keys().cloned().collect();
        // The sync document is the root's and holds every project's entries: only this project's name ids here.
        let project = self.storage.project().unwrap_or_else(|| FIRST.to_string());
        if let Loaded::Current(sync) = load(&*self.storage) {
            known.extend(sync.records.keys().filter_map(|name| Id::from_name(name)).filter(|id| id.kind == Kind::Milestone).filter_map(|id| {
                let (owner, key) = split_key(&id.key);
                (owner == project).then(|| key.to_string())
            }));
        }
        if let Some(hex) = self.storage.random_hex() {
            let id = format!("m-{}", &hex[..hex.len().min(12)]);
            if !known.contains(&id) {
                return id;
            }
        }
        let highest = known.iter().filter_map(|id| id.strip_prefix('m')?.parse::<u64>().ok()).max().unwrap_or(0);
        format!("m{}", highest.saturating_add(1))
    }

    /// Keeps a new milestone, made now, and marks it for the sync. Refused for a bad name, or a snapshot that passes
    /// one record's size.
    pub fn create(&self, draft: Draft) -> Result<Milestone, Refused> {
        let name = clean_name(&draft.name)?;
        let created = self.write(|doc| {
            let m = Milestone {
                id: self.fresh_id(doc),
                name,
                created_at: stamp(self.storage.now_ms()),
                auto: draft.auto,
                app_version: draft.app_version.clone(),
                author: draft.author.clone(),
                summary: draft.summary,
            };
            let record = record_of(&m, &draft.snapshot);
            let size = record.to_string().len();
            if size > MAX_BYTES {
                return Err(Refused::TooLarge);
            }
            // What the project holds already, and this: the device's storage is shared with the work itself.
            let held: usize = doc.records.values().map(|r| r.to_string().len()).sum();
            if held + size > MAX_TOTAL_BYTES {
                return Err(Refused::Full);
            }
            doc.records.insert(m.id.clone(), record);
            Ok(m)
        })?;
        mark(&*self.storage, Kind::Milestone, &created.id, false);
        Ok(created)
    }

    /// Renames milestone `id`; nothing else of it changes.
    pub fn rename(&self, id: &str, name: &str) -> Result<(), Refused> {
        let name = clean_name(name)?;
        let changed = self.write(|doc| {
            let Some(Value::Object(fields)) = doc.records.get_mut(id) else { return Err(Refused::Unknown) };
            let changed = fields.get("name") != Some(&json!(name));
            fields.insert("name".into(), json!(name));
            Ok(changed)
        })?;
        if changed {
            mark(&*self.storage, Kind::Milestone, id, false);
        }
        Ok(())
    }

    /// Removes milestone `id` and marks its removal for the sync.
    pub fn delete(&self, id: &str) -> Result<(), Refused> {
        self.write(|doc| doc.records.remove(id).map(|_| ()).ok_or(Refused::Unknown))?;
        mark(&*self.storage, Kind::Milestone, id, true);
        Ok(())
    }

    /// Every id kept, the records this build cannot read too. For the sync, which carries records as they are.
    pub fn raw_ids(&self) -> Vec<String> {
        match self.document() {
            Document::Current(doc) => doc.records.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// The record `id` as the document keeps it.
    pub fn raw(&self, id: &str) -> Option<Value> {
        let Document::Current(doc) = self.document() else { return None };
        doc.records.get(id).cloned()
    }

    /// Keeps `record` under `id` as given. Not an edit: nothing is marked. The caller checks `is_usable()` first: this
    /// goes through the store's own write, which copies a damaged document aside and replaces it.
    pub fn put_raw(&self, id: &str, record: Value) -> bool {
        self.write(|doc| {
            doc.records.insert(id.to_string(), record);
            Ok(())
        })
        .is_ok()
    }

    /// Removes the record `id`. Not an edit: nothing is marked. The caller checks `is_usable()` first.
    pub fn remove_raw(&self, id: &str) -> bool {
        self.write(|doc| {
            doc.records.remove(id);
            Ok(())
        })
        .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::{MemoryStorage, test_ports};
    use crate::sync::model::open;
    use crate::timeline::model::Summary;

    fn store() -> (MilestoneStore, Rc<MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        (MilestoneStore::new(ports.storage), storage)
    }

    fn draft(name: &str) -> Draft {
        Draft {
            name: name.into(),
            auto: false,
            app_version: "0.68.0".into(),
            author: None,
            summary: Summary { streets: 1, junctions: 0, rules: 1 },
            snapshot: Snapshot { edits: Some(json!({ "version": 15, "streets": { "w1": { "n": 1 } } })), restrictions: None },
        }
    }

    fn pending(storage: &MemoryStorage, id: &str) -> Option<(bool, bool)> {
        open(storage)?.entry(&Id::new(Kind::Milestone, id)).map(|e| (e.pending, e.deleted))
    }

    #[test]
    fn nothing_kept_is_no_milestone_and_each_made_one_takes_the_next_id_and_waits_to_be_sent() {
        let (s, storage) = store();
        assert!(s.list().is_empty() && s.is_usable());
        storage.set_now(1_800_000_000_000);
        let a = s.create(draft("  Before  ")).unwrap();
        storage.tick(5_000);
        let b = s.create(draft("After")).unwrap();
        assert_eq!((a.id.as_str(), a.name.as_str(), b.id.as_str()), ("m1", "Before", "m2"));
        assert!(a.created_at < b.created_at);
        assert_eq!(s.list().iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["m1", "m2"]);
        assert_eq!((pending(&storage, "m1"), pending(&storage, "m2")), (Some((true, false)), Some((true, false))));
    }

    #[test]
    fn a_snapshot_is_kept_with_the_milestone_and_listing_does_not_carry_it() {
        let (s, _) = store();
        let m = s.create(draft("A")).unwrap();
        assert_eq!(s.snapshot(&m.id).unwrap().edits.unwrap()["streets"]["w1"]["n"], 1);
        assert_eq!(s.snapshot("nope"), None);
        let json = serde_json::to_string(&s.list()[0]).unwrap();
        assert!(!json.contains("streets\":{"), "{json}");
    }

    #[test]
    fn a_bad_name_or_a_snapshot_too_big_for_a_record_is_refused_and_nothing_is_kept() {
        let (s, storage) = store();
        assert_eq!(s.create(draft("  ")), Err(Refused::Empty));
        assert_eq!(s.create(draft(&"x".repeat(81))), Err(Refused::TooLong));
        let mut big = draft("Big");
        big.snapshot.edits = Some(json!({ "x": "y".repeat(MAX_BYTES) }));
        assert_eq!(s.create(big), Err(Refused::TooLarge));
        assert!(s.list().is_empty() && storage.recall(MILESTONES_KEY).is_none() && open(&*storage).is_none_or(|d| d.records.is_empty()));
    }

    #[test]
    fn a_project_keeps_milestones_only_up_to_a_total_size_and_a_deleted_one_makes_room() {
        let (s, _) = store();
        let weighing = |name: &str| {
            let mut d = draft(name);
            d.snapshot.edits = Some(json!({ "x": "y".repeat(MAX_BYTES / 2) }));
            d
        };
        let kept: Vec<_> = (0..MAX_TOTAL_BYTES / (MAX_BYTES / 2) - 1).map(|n| s.create(weighing(&format!("M{n}"))).unwrap()).collect();
        let before = s.list();
        assert_eq!(s.create(weighing("One too many")), Err(Refused::Full));
        assert_eq!(s.list(), before, "nothing was kept");
        s.delete(&kept[0].id).unwrap();
        assert!(s.create(weighing("Room again")).is_ok());
        // The room it made is taken again.
        assert_eq!(s.create(weighing("Too big")), Err(Refused::Full));
    }

    #[test]
    fn a_rename_changes_the_name_only_and_keeps_what_a_newer_build_added() {
        let (s, storage) = store();
        let m = s.create(draft("A")).unwrap();
        let mut raw = s.raw(&m.id).unwrap();
        raw["future"] = json!([1, 2]);
        assert!(s.put_raw(&m.id, raw));
        assert_eq!(s.rename(&m.id, " B "), Ok(()));
        assert_eq!(s.get(&m.id).unwrap().name, "B");
        assert_eq!(s.raw(&m.id).unwrap()["future"], json!([1, 2]));
        assert_eq!(s.snapshot(&m.id).unwrap().edits.unwrap()["version"], 15);
        assert_eq!((s.rename("nope", "C"), s.rename(&m.id, "")), (Err(Refused::Unknown), Err(Refused::Empty)));
        assert_eq!(pending(&storage, &m.id), Some((true, false)));
    }

    #[test]
    fn a_deleted_milestone_is_gone_marked_removed_and_its_id_is_not_issued_again() {
        let (s, storage) = store();
        let a = s.create(draft("A")).unwrap();
        assert_eq!(s.delete(&a.id), Ok(()));
        assert!(s.list().is_empty());
        assert_eq!(pending(&storage, &a.id), Some((true, true)));
        assert_eq!(s.delete(&a.id), Err(Refused::Unknown));
        assert_eq!(s.create(draft("B")).unwrap().id, "m2", "the tombstone keeps m1 dead");
    }

    #[test]
    fn random_ids_where_the_storage_has_randomness() {
        let (s, storage) = store();
        storage.with_randomness(true);
        let id = s.create(draft("A")).unwrap().id;
        assert!(id.starts_with("m-") && id.len() == 14, "{id}");
    }

    #[test]
    fn a_newer_builds_document_is_neither_read_nor_written_and_a_damaged_one_is_copied_aside_once() {
        let (s, storage) = store();
        storage.remember(MILESTONES_KEY, r#"{"version":2,"milestones":{}}"#);
        assert!(!s.is_usable() && s.list().is_empty());
        assert_eq!(s.create(draft("A")), Err(Refused::Unusable));
        assert_eq!(storage.recall(MILESTONES_KEY).unwrap(), r#"{"version":2,"milestones":{}}"#);
        storage.remember(MILESTONES_KEY, "not json");
        assert!(!s.is_usable());
        assert!(s.create(draft("A")).is_ok());
        assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("not json"));
        storage.remember(MILESTONES_KEY, "worse");
        assert!(s.create(draft("B")).is_ok());
        assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("not json"), "the first copy stays");
    }

    #[test]
    fn the_syncs_own_writes_are_never_marked_and_keep_records_as_they_came() {
        let (s, storage) = store();
        assert!(s.put_raw("m-abc", json!({ "name": "From elsewhere", "created_at": "2027-01-01T00:00:00.000Z", "app_version": "9", "summary": {"streets":0,"junctions":0,"rules":0}, "edits": {"x": 1} })));
        assert_eq!(pending(&storage, "m-abc"), None);
        assert_eq!(s.get("m-abc").unwrap().name, "From elsewhere");
        assert_eq!(s.raw_ids(), ["m-abc"]);
        assert!(s.put_raw("m-odd", json!("not a milestone")));
        assert_eq!(s.raw_ids(), ["m-abc", "m-odd"]);
        assert_eq!(s.list().len(), 1, "a record this build cannot read is not listed");
        assert!(s.remove_raw("m-odd") && s.remove_raw("m-odd"));
        assert_eq!(s.raw_ids(), ["m-abc"]);
        assert_eq!(pending(&storage, "m-odd"), None);
    }
}
