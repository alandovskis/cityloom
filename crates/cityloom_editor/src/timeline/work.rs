//! A project's work as a milestone sees it: taken now (`capture`), set beside a milestone's (`compare`) and put back
//! (`restore`). Records are held as the stores keep them, raw JSON by kind and key, so what a newer build adds is
//! compared and restored with the rest. Takes the project's storage (`ScopedStorage`). No wording here.

use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::Value;

use crate::city::edits::{self, Document as EditsDocument, EDITS_KEY, JUNCTIONS, STREETS};
use crate::restriction::store::{self as rules, RESTRICTIONS_KEY, RestrictionStore};
use crate::shared::ports::Storage;
use crate::shared::project_id::{FIRST, wire_key};
use crate::sync::model::{Id, Kind, mark_all, open, save};
use crate::timeline::model::{Draft, Milestone, Refused, Snapshot, Summary};
use crate::timeline::store::MilestoneStore;

/// The records of a snapshot, by kind (`Street`, `Junction`, `Rule`) and the key the project's documents hold them under.
pub type Records = BTreeMap<(Kind, String), Value>;

fn section(snapshot: &Value, name: &str) -> impl Iterator<Item = (String, Value)> + use<> {
    snapshot.get(name).and_then(Value::as_object).cloned().unwrap_or_default().into_iter()
}

/// Every record the snapshot holds.
pub fn records_of(snapshot: &Snapshot) -> Records {
    let mut out = Records::new();
    if let Some(edits) = &snapshot.edits {
        out.extend(section(edits, STREETS).map(|(key, record)| ((Kind::Street, key), record)));
        out.extend(section(edits, JUNCTIONS).map(|(key, record)| ((Kind::Junction, key), record)));
    }
    if let Some(restrictions) = &snapshot.restrictions {
        out.extend(section(restrictions, "restrictions").map(|(key, record)| ((Kind::Rule, key), record)));
    }
    out
}

/// How many streets, junctions and rules the records hold.
pub fn summary_of(records: &Records) -> Summary {
    let count = |kind: Kind| records.keys().filter(|(k, _)| *k == kind).count();
    Summary { streets: count(Kind::Street), junctions: count(Kind::Junction), rules: count(Kind::Rule) }
}

/// The project's work now. Refused where a document cannot be read (damaged or a newer build's): what it holds is not
/// known, so nothing is taken from it and nothing is put over it.
pub fn capture(storage: &Rc<dyn Storage>) -> Result<Snapshot, Refused> {
    let kept = |key: &str| storage.recall(key).filter(|text| !text.trim().is_empty());
    let edits = match kept(EDITS_KEY) {
        None => None,
        Some(text) => {
            edits::draft(Some(&text)).map_err(|_: EditsDocument| Refused::Unusable)?;
            Some(serde_json::from_str::<Value>(&text).map_err(|_| Refused::Unusable)?)
        }
    };
    let restrictions = match kept(RESTRICTIONS_KEY) {
        None => None,
        Some(text) => {
            if !matches!(rules::parse(Some(&text)), rules::Document::Current(_)) {
                return Err(Refused::Unusable);
            }
            Some(serde_json::from_str::<Value>(&text).map_err(|_| Refused::Unusable)?)
        }
    };
    Ok(Snapshot { edits, restrictions })
}

/// How a place stands now, against a milestone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum How {
    /// Made since the milestone.
    Added,
    /// In the milestone, gone now.
    Removed,
    /// In both, differently.
    Changed,
}

/// What about a changed place is different: the parts of its design a person would name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Aspect {
    Name,
    Class,
    Width,
    Lanes,
    Speed,
    Zones,
    Arms,
    Control,
    Rule,
    Outline,
    /// Something else in the record that differs.
    Other,
}

/// Where each aspect lives in a record, by kind: a path of object keys.
fn aspect_paths(kind: Kind) -> &'static [(&'static [&'static str], Aspect)] {
    match kind {
        Kind::Street => &[
            (&["street", "name"], Aspect::Name),
            (&["street", "class"], Aspect::Class),
            (&["street", "row_mm"], Aspect::Width),
            (&["street", "segments"], Aspect::Lanes),
            (&["street", "speed_kmh"], Aspect::Speed),
            (&["street", "zones"], Aspect::Zones),
        ],
        Kind::Junction => &[(&["state", "arms"], Aspect::Arms), (&["state", "control"], Aspect::Control)],
        Kind::Rule => &[(&["name"], Aspect::Name), (&["rule"], Aspect::Rule), (&["ring"], Aspect::Outline)],
        _ => &[],
    }
}

fn at<'a>(record: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(record, |value, key| value.get(*key))
}

fn without(record: &Value, path: &[&str]) -> Value {
    let mut copy = record.clone();
    if let Some((last, parents)) = path.split_last() {
        if let Some(object) = parents.iter().try_fold(&mut copy, |value, key| value.get_mut(*key)).and_then(Value::as_object_mut) {
            object.remove(*last);
        }
    }
    copy
}

/// What differs between two records of one place, in the order of the table, `Other` last for what no aspect names.
fn aspects(kind: Kind, before: &Value, after: &Value) -> Vec<Aspect> {
    let table = aspect_paths(kind);
    let mut out: Vec<Aspect> = table.iter().filter(|(path, _)| at(before, path) != at(after, path)).map(|(_, aspect)| *aspect).collect();
    let rest = |record: &Value| table.iter().fold(record.clone(), |left, (path, _)| without(&left, path));
    if rest(before) != rest(after) {
        out.push(Aspect::Other);
    }
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct Difference {
    pub kind: Kind,
    pub key: String,
    pub how: How,
    /// What about the place differs, for a `Changed` one; empty for a place that was added or removed.
    pub aspects: Vec<Aspect>,
}

/// What differs between a milestone and the work now, places in kind and key order. Nothing where they are the same.
pub fn compare(milestone: &Snapshot, now: &Snapshot) -> Vec<Difference> {
    let (then, now) = (records_of(milestone), records_of(now));
    let mut out = Vec::new();
    for ((kind, key), before) in &then {
        match now.get(&(*kind, key.clone())) {
            None => out.push(Difference { kind: *kind, key: key.clone(), how: How::Removed, aspects: Vec::new() }),
            Some(after) if after != before => out.push(Difference { kind: *kind, key: key.clone(), how: How::Changed, aspects: aspects(*kind, before, after) }),
            Some(_) => {}
        }
    }
    out.extend(now.keys().filter(|id| !then.contains_key(id)).map(|(kind, key)| Difference {
        kind: *kind,
        key: key.clone(),
        how: How::Added,
        aspects: Vec::new(),
    }));
    out.sort_by(|a, b| (a.kind, &a.key).cmp(&(b.kind, &b.key)));
    out
}

/// What the person sees of a made milestone: the header, and what it differs by now (the work taken before the
/// restore as a milestone of its own, when there was any).
#[derive(Clone, Debug, PartialEq)]
pub struct Restored {
    /// How many places changed to be as the milestone had them.
    pub changed: usize,
    /// The milestone made first so the work being replaced can be had back; none when nothing was replaced.
    pub backup: Option<Milestone>,
}

/// Puts the project's work back as milestone `id` held it. Work that would be replaced or lost is first kept as a
/// milestone of its own (named `backup_name`, `auto`), so a restore can be undone by restoring that one; where that
/// cannot be done nothing is changed. Changed places are marked for the sync, removed ones as removals.
pub fn restore(storage: &Rc<dyn Storage>, id: &str, backup_name: &str, app_version: &str, author: Option<&str>) -> Result<Restored, Refused> {
    let milestones = MilestoneStore::new(storage.clone());
    let target = milestones.snapshot(id).ok_or(Refused::Unknown)?;
    let now = capture(storage)?;
    let differences = compare(&target, &now);
    if differences.is_empty() {
        return Ok(Restored { changed: 0, backup: None });
    }
    let backup = milestones.create(Draft {
        name: backup_name.to_string(),
        auto: true,
        app_version: app_version.to_string(),
        author: author.map(str::to_string),
        summary: summary_of(&records_of(&now)),
        snapshot: now.clone(),
    })?;
    let (kept_edits, kept_rules) = (storage.recall(EDITS_KEY), storage.recall(RESTRICTIONS_KEY));
    let then = records_of(&target);
    if let Err(e) = put_back(storage, &then, &differences) {
        // Neither document is left half way: both are put as they were (one there was none of is blanked: an empty
        // value reads as absent), and the backup made first is dropped, with the mark that told the sync of it.
        storage.remember(EDITS_KEY, kept_edits.as_deref().unwrap_or(""));
        storage.remember(RESTRICTIONS_KEY, kept_rules.as_deref().unwrap_or(""));
        milestones.remove_raw(&backup.id);
        forget_mark(storage, &backup.id);
        return Err(e);
    }
    let removed: Vec<(Kind, String)> = differences.iter().filter(|d| d.how == How::Added).map(|d| (d.kind, d.key.clone())).collect();
    let back: Vec<(Kind, String)> = differences.iter().filter(|d| d.how != How::Added).map(|d| (d.kind, d.key.clone())).collect();
    mark_all(&**storage, &removed, true);
    mark_all(&**storage, &back, false);
    Ok(Restored { changed: differences.len(), backup: Some(backup) })
}

/// Takes milestone `id`'s entry out of the sync document: it was marked when made and never existed anywhere else, so
/// a removal sent for it would name a record the server never had.
fn forget_mark(storage: &Rc<dyn Storage>, id: &str) {
    if let Some(mut doc) = open(&**storage) {
        // The entry is named by the wire key, as `mark` composes it: `<project>:<id>` outside the first project.
        let project = storage.project();
        doc.forget(&Id::new(Kind::Milestone, &wire_key(project.as_deref().unwrap_or(FIRST), id)));
        save(&**storage, &doc);
    }
}

fn put_back(storage: &Rc<dyn Storage>, then: &Records, differences: &[Difference]) -> Result<(), Refused> {
    let mut draft = match storage.recall(EDITS_KEY).filter(|text| !text.trim().is_empty()) {
        Some(text) => edits::draft(Some(&text)).map_err(|_| Refused::Unusable)?,
        None => edits::Draft::new(),
    };
    let rules = RestrictionStore::new(storage.clone());
    let sections = [(Kind::Street, STREETS), (Kind::Junction, JUNCTIONS)];
    let mut edited = false;
    for d in differences {
        let record = then.get(&(d.kind, d.key.clone()));
        if let Some((_, name)) = sections.iter().find(|(kind, _)| *kind == d.kind) {
            match record {
                Some(record) => {
                    if !draft.put_raw(name, &d.key, record.clone()) {
                        return Err(Refused::Unusable);
                    }
                }
                None => draft.remove_raw(name, &d.key),
            }
            edited = true;
        } else if d.kind == Kind::Rule {
            let done = match record {
                Some(record) => rules.put_raw(&d.key, record.clone()),
                None => rules.remove_raw(&d.key),
            };
            if !done {
                return Err(Refused::Unusable);
            }
        }
    }
    if edited && !storage.remember(EDITS_KEY, &draft.to_json()) {
        return Err(Refused::Unusable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::{MemoryStorage, test_ports};
    use crate::sync::model::Id;
    use serde_json::json;

    fn setup() -> (Rc<dyn Storage>, Rc<MemoryStorage>) {
        let (ports, _, memory) = test_ports();
        (ports.storage, memory)
    }

    fn work(streets: &[(&str, i64)], rules: &[(&str, &str)]) -> (String, String) {
        let streets: serde_json::Map<String, Value> = streets.iter().map(|(k, n)| ((*k).to_string(), json!({ "ways": [1], "street": { "n": n } }))).collect();
        let ring = json!([[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]]);
        let restrictions: serde_json::Map<String, Value> =
            rules.iter().map(|(id, name)| ((*id).to_string(), json!({ "name": name, "rule": { "kind": "car_free" }, "ring": ring }))).collect();
        (json!({ "version": 5, "streets": streets }).to_string(), json!({ "version": 1, "next_id": 9, "restrictions": restrictions }).to_string())
    }

    fn keep(storage: &Rc<dyn Storage>, streets: &[(&str, i64)], rules: &[(&str, &str)]) {
        let (edits, restrictions) = work(streets, rules);
        assert!(storage.remember(EDITS_KEY, &edits) && storage.remember(RESTRICTIONS_KEY, &restrictions));
    }

    fn marked(memory: &MemoryStorage, kind: Kind, key: &str) -> Option<(bool, bool)> {
        open(memory)?.entry(&Id::new(kind, key)).map(|e| (e.pending, e.deleted))
    }

    #[test]
    fn nothing_kept_is_an_empty_snapshot_and_work_is_counted_by_kind() {
        let (storage, _) = setup();
        let none = capture(&storage).unwrap();
        assert_eq!((none.edits.clone(), none.restrictions.clone()), (None, None));
        assert!(summary_of(&records_of(&none)).is_empty());
        keep(&storage, &[("w1", 1), ("w2", 1)], &[("r1", "Zone")]);
        let summary = summary_of(&records_of(&capture(&storage).unwrap()));
        assert_eq!((summary.streets, summary.junctions, summary.rules), (2, 0, 1));
    }

    #[test]
    fn a_document_that_cannot_be_read_is_not_captured() {
        let (storage, _) = setup();
        storage.remember(EDITS_KEY, "not json");
        assert_eq!(capture(&storage), Err(Refused::Unusable));
        storage.remember(EDITS_KEY, "");
        storage.remember(RESTRICTIONS_KEY, r#"{"version":99,"restrictions":{}}"#);
        assert_eq!(capture(&storage), Err(Refused::Unusable));
    }

    #[test]
    fn the_same_work_has_no_difference_and_places_made_removed_or_changed_are_told_apart() {
        let (storage, _) = setup();
        keep(&storage, &[("w1", 1), ("w2", 1), ("w3", 1)], &[("r1", "Zone")]);
        let then = capture(&storage).unwrap();
        assert_eq!(compare(&then, &then), []);
        keep(&storage, &[("w1", 1), ("w2", 2), ("w4", 1)], &[("r1", "Renamed")]);
        let now = capture(&storage).unwrap();
        let seen: Vec<(Kind, String, How)> = compare(&then, &now).into_iter().map(|d| (d.kind, d.key, d.how)).collect();
        let want = |kind, key: &str, how| (kind, key.to_string(), how);
        assert_eq!(
            seen,
            [
                want(Kind::Street, "w2", How::Changed),
                want(Kind::Street, "w3", How::Removed),
                want(Kind::Street, "w4", How::Added),
                want(Kind::Rule, "r1", How::Changed),
            ]
        );
    }

    #[test]
    fn a_changed_place_says_what_about_it_changed() {
        let street = |street: Value| json!({ "ways": [1], "street": street });
        let rule = |name: &str, kind: &str, kmh: i64| json!({ "name": name, "rule": { "kind": kind, "kmh": kmh }, "ring": [[0, 0]] });
        let edits =
            |a: Value, b: Value| Snapshot { edits: Some(json!({ "version": 5, "streets": { "w1": a }, "junctions": { "j1": b } })), restrictions: None };
        let before = edits(
            street(json!({ "name": "A", "class": "local", "row_mm": 9000, "segments": [1], "speed_kmh": 30 })),
            json!({ "nodes": [1], "state": { "arms": [1, 2], "control": 0 } }),
        );
        let after = edits(
            street(json!({ "name": "B", "class": "local", "row_mm": 12000, "segments": [1, 2], "speed_kmh": 30, "note": 1 })),
            json!({ "nodes": [1], "state": { "arms": [1, 2], "control": 2 } }),
        );
        let seen = compare(&before, &after);
        assert_eq!(seen[0].aspects, [Aspect::Name, Aspect::Width, Aspect::Lanes, Aspect::Other]);
        assert_eq!(seen[1].aspects, [Aspect::Control]);
        let rules = |r: Value| Snapshot { edits: None, restrictions: Some(json!({ "version": 1, "restrictions": { "r1": r } })) };
        let seen = compare(&rules(rule("Z", "car_free", 0)), &rules(rule("Z", "speed_limit", 20)));
        assert_eq!(seen[0].aspects, [Aspect::Rule]);
        let added = compare(&Snapshot::default(), &before);
        assert!(added.iter().all(|d| d.aspects.is_empty()), "only a changed place has aspects");
    }

    fn milestone(storage: &Rc<dyn Storage>, name: &str) -> Milestone {
        let snapshot = capture(storage).unwrap();
        let summary = summary_of(&records_of(&snapshot));
        MilestoneStore::new(storage.clone()).create(Draft { name: name.into(), auto: false, app_version: "1".into(), author: None, summary, snapshot }).unwrap()
    }

    #[test]
    fn restoring_puts_the_work_back_after_keeping_what_it_replaces_as_a_milestone_of_its_own() {
        let (storage, memory) = setup();
        keep(&storage, &[("w1", 1), ("w2", 1)], &[("r1", "Zone")]);
        let before = milestone(&storage, "Before");
        let original = capture(&storage).unwrap();
        keep(&storage, &[("w2", 2), ("w3", 1)], &[("r2", "Other")]);
        let edited = capture(&storage).unwrap();
        memory.tick(1_000);
        let done = restore(&storage, &before.id, "Before restoring Before", "2", Some("ada")).unwrap();
        assert_eq!(done.changed, 5, "w1 back, w2 changed, w3 gone, r1 back, r2 gone");
        assert_eq!(compare(&original, &capture(&storage).unwrap()), [], "the work is as the milestone had it");
        let backup = done.backup.unwrap();
        assert!(backup.auto && backup.name == "Before restoring Before" && backup.author.as_deref() == Some("ada"));
        assert_eq!((backup.summary.streets, backup.summary.rules), (2, 1));
        let kept = MilestoneStore::new(storage.clone()).snapshot(&backup.id).unwrap();
        assert_eq!(compare(&kept, &edited), [], "the backup holds the work that was replaced");
        let again = restore(&storage, &backup.id, "Before restoring", "2", None).unwrap();
        assert_eq!(compare(&edited, &capture(&storage).unwrap()), [], "restoring the backup undoes the restore");
        assert_eq!(again.changed, 5);
        // marks: a place put back is a write, one that was only added is a removal
        assert_eq!(marked(&memory, Kind::Street, "w3"), Some((true, false)), "restored by the second restore");
    }

    #[test]
    fn restoring_marks_places_put_back_as_written_and_places_taken_away_as_removed() {
        let (storage, memory) = setup();
        keep(&storage, &[("w1", 1)], &[]);
        let before = milestone(&storage, "Before");
        keep(&storage, &[("w1", 2), ("w9", 1)], &[]);
        restore(&storage, &before.id, "Backup", "1", None).unwrap();
        assert_eq!(marked(&memory, Kind::Street, "w1"), Some((true, false)));
        assert_eq!(marked(&memory, Kind::Street, "w9"), Some((true, true)));
    }

    #[test]
    fn restoring_work_that_is_already_as_the_milestone_had_it_changes_and_keeps_nothing() {
        let (storage, memory) = setup();
        keep(&storage, &[("w1", 1)], &[]);
        let m = milestone(&storage, "Same");
        let done = restore(&storage, &m.id, "Backup", "1", None).unwrap();
        assert_eq!(done, Restored { changed: 0, backup: None });
        assert_eq!(MilestoneStore::new(storage.clone()).list().len(), 1);
        assert_eq!(marked(&memory, Kind::Street, "w1"), None);
    }

    #[test]
    fn a_restore_that_cannot_keep_the_work_it_replaces_changes_nothing() {
        let (storage, _) = setup();
        keep(&storage, &[("w1", 1)], &[]);
        let before = milestone(&storage, "Before");
        let big = "y".repeat(crate::timeline::model::MAX_BYTES);
        let streets = json!({ "version": 5, "streets": { "w1": { "ways": [1], "street": { "n": 2 } }, "w2": { "ways": [2], "street": { "pad": big } } } });
        storage.remember(EDITS_KEY, &streets.to_string());
        let now = storage.recall(EDITS_KEY);
        assert_eq!(restore(&storage, &before.id, "Backup", "1", None), Err(Refused::TooLarge));
        assert_eq!(storage.recall(EDITS_KEY), now, "the work stands");
        assert_eq!(MilestoneStore::new(storage.clone()).list().len(), 1, "and no backup was made");
        assert_eq!(restore(&storage, "nope", "Backup", "1", None), Err(Refused::Unknown));
    }

    #[test]
    fn a_restore_that_fails_half_way_leaves_no_document_it_made_and_no_mark_for_the_backup_it_dropped() {
        let (storage, memory) = setup();
        keep(&storage, &[("w1", 1)], &[("r1", "Zone")]);
        let before = milestone(&storage, "Before");
        // The project now has streets only: no rules document at all.
        storage.remember(RESTRICTIONS_KEY, "");
        keep_streets_only(&storage, &[("w1", 2)]);
        memory.refuse(EDITS_KEY);
        assert_eq!(restore(&storage, &before.id, "Backup", "1", None), Err(Refused::Unusable));
        memory.refuse_nothing();
        assert!(storage.recall(RESTRICTIONS_KEY).is_none_or(|text| text.trim().is_empty()), "the rules the restore began to put back are gone again");
        assert_eq!(MilestoneStore::new(storage.clone()).list().len(), 1, "no backup is kept");
        let any_backup_mark =
            open(&*memory).is_some_and(|doc| doc.records.keys().any(|name| name.starts_with("milestone/") && name != &format!("milestone/{}", before.id)));
        assert!(!any_backup_mark, "no mark is left for the backup that was dropped");
    }

    #[test]
    fn the_mark_of_a_dropped_backup_is_found_under_the_project_s_wire_key_in_any_project() {
        let (ports, _, memory) = test_ports();
        let storage: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(ports.storage, "p1"));
        keep(&storage, &[("w1", 1)], &[("r1", "Zone")]);
        let before = milestone(&storage, "Before");
        storage.remember(RESTRICTIONS_KEY, "");
        keep_streets_only(&storage, &[("w1", 2)]);
        memory.refuse(&format!("{EDITS_KEY}@p1"));
        assert_eq!(restore(&storage, &before.id, "Backup", "1", None), Err(Refused::Unusable));
        memory.refuse_nothing();
        let marks: Vec<String> = open(&*memory).unwrap().records.keys().filter(|name| name.starts_with("milestone/")).cloned().collect();
        assert_eq!(marks, [format!("milestone/p1:{}", before.id)], "only the milestone that was kept is marked");
    }

    fn keep_streets_only(storage: &Rc<dyn Storage>, streets: &[(&str, i64)]) {
        let streets: serde_json::Map<String, Value> = streets.iter().map(|(k, n)| ((*k).to_string(), json!({ "ways": [1], "street": { "n": n } }))).collect();
        assert!(storage.remember(EDITS_KEY, &json!({ "version": 5, "streets": streets }).to_string()));
    }

    #[test]
    fn a_restore_over_a_document_that_cannot_be_read_is_refused() {
        let (storage, _) = setup();
        keep(&storage, &[("w1", 1)], &[]);
        let before = milestone(&storage, "Before");
        storage.remember(EDITS_KEY, "garbage");
        assert_eq!(restore(&storage, &before.id, "Backup", "1", None), Err(Refused::Unusable));
        assert_eq!(storage.recall(EDITS_KEY).as_deref(), Some("garbage"));
    }
}
