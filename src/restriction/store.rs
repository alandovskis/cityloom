//! The area restrictions as this browser keeps them: one document for every area, whatever tile is loaded,
//! read again and changed one record at a time, so another tab's restrictions survive a write.

use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::Value;

use crate::restriction::model::{LonLat, Restriction, Rule, number_of};
use crate::shared::ports::Storage;

/// The shape of the document. Raised when the shape changes.
pub const VERSION: u32 = 1;
/// Where the document is kept.
pub const RESTRICTIONS_KEY: &str = "cityloom-restrictions";
/// Where a document that could not be read is copied before the first write replaces it.
pub const DAMAGED_KEY: &str = "cityloom-restrictions-damaged";

/// The document. A record this build cannot read (one a newer build wrote under the same version) is held as it
/// was, written back unchanged and never listed.
#[derive(Clone, Debug, PartialEq)]
pub struct Restrictions {
    pub version: u32,
    /// The number the next restriction takes at least: it only rises, so an id that was deleted is not issued again.
    pub next_id: u32,
    /// By id, `r<n>`.
    pub restrictions: BTreeMap<String, Restriction>,
    /// What could not be read, by id.
    unreadable: BTreeMap<String, Value>,
    /// The record objects as they were read, so that a field this build does not know survives a change to it.
    raw: BTreeMap<String, serde_json::Map<String, Value>>,
    /// The document's own keys other than the three above, as read.
    others: serde_json::Map<String, Value>,
}

impl Default for Restrictions {
    fn default() -> Self {
        Restrictions::new()
    }
}

impl Restrictions {
    /// An empty document of the current version.
    pub fn new() -> Self {
        Restrictions {
            version: VERSION,
            next_id: 1,
            restrictions: BTreeMap::new(),
            unreadable: BTreeMap::new(),
            raw: BTreeMap::new(),
            others: serde_json::Map::new(),
        }
    }

    pub fn to_json(&self) -> String {
        let mut records = serde_json::Map::new();
        for (id, r) in &self.restrictions {
            // As it was read, but for the fields that changed: what a newer build added, at any depth, stays.
            let mut record = self.raw.get(id).cloned().unwrap_or_default();
            let was = serde_json::from_value::<Restriction>(Value::Object(record.clone())).ok().map(|w| Restriction { id: id.clone(), ..w });
            if was.as_ref() != Some(r) {
                let Value::Object(known) = serde_json::to_value(r).expect("a restriction serialises") else { unreachable!("a record is an object") };
                let old = was.and_then(|w| serde_json::to_value(w).ok()).unwrap_or_default();
                for (field, value) in known {
                    if old.get(&field) != Some(&value) {
                        record.insert(field, value);
                    }
                }
            }
            records.insert(id.clone(), Value::Object(record));
        }
        for (id, raw) in &self.unreadable {
            records.insert(id.clone(), raw.clone());
        }
        let mut doc = self.others.clone();
        doc.insert("version".into(), self.version.into());
        doc.insert("next_id".into(), self.next_id.into());
        doc.insert("restrictions".into(), Value::Object(records));
        Value::Object(doc).to_string()
    }

    /// Every restriction with its id, in the order of their numbers (an id of another shape last).
    pub fn list(&self) -> Vec<Restriction> {
        let mut all: Vec<Restriction> = self.restrictions.iter().map(|(id, r)| Restriction { id: id.clone(), ..r.clone() }).collect();
        all.sort_by_key(|r| (number_of(&r.id).unwrap_or(u32::MAX), r.id.clone()));
        all
    }

    fn holds(&self, id: &str) -> bool {
        self.restrictions.contains_key(id) || self.unreadable.contains_key(id)
    }

    /// The id the next restriction takes: the number after the largest ever issued or kept.
    pub fn next_id(&self) -> String {
        let kept = self.restrictions.keys().chain(self.unreadable.keys()).filter_map(|id| number_of(id)).max().unwrap_or(0);
        format!("r{}", self.next_id.max(kept.saturating_add(1)))
    }

    /// Keeps `r` under its id, and never issues that number or a lower one again.
    fn insert(&mut self, r: Restriction) {
        if let Some(n) = number_of(&r.id) {
            self.next_id = self.next_id.max(n.saturating_add(1));
        }
        self.restrictions.insert(r.id.clone(), r);
    }
}

/// What the store held.
#[derive(Debug)]
pub enum Document {
    /// Nothing was kept.
    Empty,
    Current(Restrictions),
    /// Written by a newer build: not read, and never written over.
    Future,
    /// Not a document of this version; the text is kept so it can be copied aside.
    Damaged(String),
}

pub fn parse(stored: Option<&str>) -> Document {
    let Some(raw) = stored.filter(|s| !s.trim().is_empty()) else { return Document::Empty };
    let damaged = || Document::Damaged(raw.to_string());
    let Ok(value) = serde_json::from_str::<Value>(raw) else { return damaged() };
    // A version that is a number of another kind than an integer, or a string, is some later build's.
    match &value["version"] {
        Value::Null => return damaged(),
        Value::Number(n) if n.as_u64() == Some(u64::from(VERSION)) => {}
        Value::Number(n) if n.as_u64() == Some(0) => return damaged(),
        _ => return Document::Future,
    }
    let Some(records) = value["restrictions"].as_object() else { return damaged() };
    let mut doc = Restrictions::new();
    doc.next_id = value["next_id"].as_u64().and_then(|n| u32::try_from(n).ok()).unwrap_or(1);
    for (key, kept) in value.as_object().into_iter().flatten() {
        if !matches!(key.as_str(), "version" | "next_id" | "restrictions") {
            doc.others.insert(key.clone(), kept.clone());
        }
    }
    for (id, record) in records {
        match serde_json::from_value::<Restriction>(record.clone()) {
            Ok(r) => {
                doc.restrictions.insert(id.clone(), Restriction { id: id.clone(), ..r });
                if let Value::Object(fields) = record {
                    doc.raw.insert(id.clone(), fields.clone());
                }
            }
            Err(_) => {
                doc.unreadable.insert(id.clone(), record.clone());
            }
        }
    }
    Document::Current(doc)
}

fn finite(ring: &[LonLat]) -> bool {
    ring.iter().flatten().all(|c| c.is_finite())
}

/// The restrictions kept in this browser.
#[derive(Clone)]
pub struct RestrictionStore {
    storage: Rc<dyn Storage>,
}

impl RestrictionStore {
    pub fn new(storage: Rc<dyn Storage>) -> RestrictionStore {
        RestrictionStore { storage }
    }

    fn document(&self) -> Document {
        parse(self.storage.recall(RESTRICTIONS_KEY).as_deref())
    }

    /// Every restriction kept, in the order of their numbers; none when the document is a newer build's or damaged.
    pub fn all(&self) -> Vec<Restriction> {
        match self.document() {
            Document::Current(doc) => doc.list(),
            _ => Vec::new(),
        }
    }

    /// Whether the document was written by a newer build, which this one must leave alone.
    pub fn is_future(&self) -> bool {
        matches!(self.document(), Document::Future)
    }

    /// Applies `change` to the document as it is kept now, and keeps the result. None when the document is a newer
    /// build's, the change is not taken, or storage refuses. A damaged document is copied aside first.
    fn write<R>(&self, change: impl FnOnce(&mut Restrictions) -> Option<R>) -> Option<R> {
        let mut doc = match self.document() {
            Document::Future => return None,
            Document::Empty => Restrictions::new(),
            Document::Current(doc) => doc,
            Document::Damaged(raw) => {
                // The first copy may hold what can be recovered: a later one does not replace it.
                if self.storage.recall(DAMAGED_KEY).is_none() && !self.storage.remember(DAMAGED_KEY, &raw) {
                    return None;
                }
                Restrictions::new()
            }
        };
        let done = change(&mut doc)?;
        self.storage.remember(RESTRICTIONS_KEY, &doc.to_json()).then_some(done)
    }

    /// Keeps a new restriction under the next id, and gives it back with that id.
    pub fn add(&self, name: &str, rule: Rule, ring: Vec<LonLat>) -> Option<Restriction> {
        if !finite(&ring) {
            return None;
        }
        self.write(|doc| {
            let r = Restriction { id: doc.next_id(), name: name.to_string(), rule, ring };
            if doc.holds(&r.id) {
                return None;
            }
            doc.insert(r.clone());
            Some(r)
        })
    }

    /// Changes the restriction `id` as it is kept now. False when it is no longer kept, or was not kept again.
    pub fn update(&self, id: &str, change: impl FnOnce(&mut Restriction)) -> bool {
        self.write(|doc| {
            let r = doc.restrictions.get_mut(id)?;
            change(r);
            finite(&r.ring).then_some(())
        })
        .is_some()
    }

    pub fn remove(&self, id: &str) -> bool {
        self.write(|doc| doc.restrictions.remove(id).map(|_| ())).is_some()
    }

    /// Keeps a deleted restriction again: under its own id if that is free, else under the next. Gives it back with
    /// the id it has now.
    pub fn restore(&self, r: &Restriction) -> Option<Restriction> {
        if !finite(&r.ring) {
            return None;
        }
        self.write(|doc| {
            let id = if doc.holds(&r.id) { doc.next_id() } else { r.id.clone() };
            let back = Restriction { id, ..r.clone() };
            doc.insert(back.clone());
            Some(back)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::shared::ports::{MemoryStorage, test_ports};

    const RING: [LonLat; 3] = [[-73.60, 45.52], [-73.59, 45.52], [-73.59, 45.53]];

    fn store() -> (RestrictionStore, Rc<MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        (RestrictionStore::new(ports.storage), storage)
    }

    fn names(s: &RestrictionStore) -> Vec<(String, String)> {
        s.all().into_iter().map(|r| (r.id, r.name)).collect()
    }

    #[test]
    fn nothing_kept_is_no_restriction_and_each_added_one_takes_the_next_id() {
        let (s, storage) = store();
        assert!(s.all().is_empty() && !s.is_future());
        let a = s.add("A", Rule::CarFree, RING.to_vec()).unwrap();
        let b = s.add("", Rule::SpeedLimit { kmh: 30 }, RING.to_vec()).unwrap();
        assert_eq!((a.id.as_str(), b.id.as_str()), ("r1", "r2"));
        assert_eq!(names(&s), [("r1".to_string(), "A".to_string()), ("r2".to_string(), String::new())]);
        let Document::Current(doc) = parse(storage.recall(RESTRICTIONS_KEY).as_deref()) else { panic!("not read back") };
        assert_eq!(doc.version, VERSION);
        assert_eq!(doc.list(), s.all(), "the document round-trips, its records given back their ids");
    }

    #[test]
    fn the_list_is_in_the_order_of_the_numbers_and_the_next_id_is_one_more_than_the_largest() {
        let mut doc = Restrictions::new();
        for id in ["r10", "r9", "r2"] {
            doc.restrictions.insert(id.into(), Restriction { id: id.into(), name: id.into(), rule: Rule::CarFree, ring: RING.to_vec() });
        }
        assert_eq!(doc.list().iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["r2", "r9", "r10"]);
        assert_eq!(doc.next_id(), "r11");
        assert_eq!(Restrictions::new().next_id(), "r1");
    }

    #[test]
    fn a_newer_version_is_neither_listed_nor_written_over() {
        let (s, storage) = store();
        let newer = r#"{"version":2,"restrictions":{}}"#;
        storage.remember(RESTRICTIONS_KEY, newer);
        assert!(s.all().is_empty() && s.is_future());
        assert_eq!(s.add("A", Rule::CarFree, RING.to_vec()), None);
        assert_eq!(storage.recall(RESTRICTIONS_KEY).as_deref(), Some(newer));
    }

    #[test]
    fn a_damaged_document_is_copied_aside_before_the_first_write_replaces_it() {
        let (s, storage) = store();
        storage.remember(RESTRICTIONS_KEY, "{not json");
        assert!(s.all().is_empty() && !s.is_future());
        assert_eq!(s.add("A", Rule::CarFree, RING.to_vec()).unwrap().id, "r1");
        assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("{not json"));
        assert_eq!(names(&s), [("r1".to_string(), "A".to_string())]);
        assert!(matches!(parse(Some(r#"{"version":1,"restrictions":5}"#)), Document::Damaged(_)), "an old shape is damaged too");
        assert!(matches!(parse(Some("  ")), Document::Empty));
    }

    #[test]
    fn two_tabs_each_change_only_their_own_record() {
        let (ports, ..) = test_ports();
        let (one, two) = (RestrictionStore::new(ports.storage.clone()), RestrictionStore::new(ports.storage));
        one.add("A", Rule::CarFree, RING.to_vec()).unwrap();
        two.add("B", Rule::CarFree, RING.to_vec()).unwrap();
        assert!(one.update("r1", |r| r.name = "A2".into()));
        assert_eq!(names(&two), [("r1".to_string(), "A2".to_string()), ("r2".to_string(), "B".to_string())]);
        assert!(two.remove("r1"));
        assert!(!one.update("r1", |r| r.name = "back".into()), "a change to one another tab removed is not taken");
        assert!(!one.remove("r1"));
        assert_eq!(names(&one), [("r2".to_string(), "B".to_string())]);
    }

    #[test]
    fn a_restored_restriction_keeps_its_id_unless_another_took_it() {
        let (s, _) = store();
        s.add("A", Rule::CarFree, RING.to_vec()).unwrap();
        let b = s.add("B", Rule::CarFree, RING.to_vec()).unwrap();
        assert!(s.remove("r2"));
        assert_eq!(s.restore(&b).unwrap().id, "r2", "free again");
        assert_eq!(s.restore(&b).unwrap().id, "r3", "taken now");
        assert_eq!(names(&s).len(), 3);
    }

    #[test]
    fn a_removed_id_is_not_issued_again_so_a_stale_tab_cannot_hit_another_record() {
        let (ports, ..) = test_ports();
        let (a, b) = (RestrictionStore::new(ports.storage.clone()), RestrictionStore::new(ports.storage));
        for n in 1..=5 {
            a.add(&format!("n{n}"), Rule::CarFree, RING.to_vec()).unwrap();
        }
        assert!(b.remove("r5"));
        assert_eq!(b.add("new", Rule::CarFree, RING.to_vec()).unwrap().id, "r6");
        assert!(!a.update("r5", |r| r.name = "stale".into()));
        assert!(!a.remove("r5"));
        assert_eq!(names(&a).last().unwrap(), &("r6".to_string(), "new".to_string()));
    }

    #[test]
    fn a_record_that_cannot_be_read_is_left_out_but_kept_unchanged() {
        let (s, storage) = store();
        let odd = r#"{"name":"later","ring":[[1.0,2.0]],"rule":{"kind":"school_street"}}"#;
        storage.remember(
            RESTRICTIONS_KEY,
            &format!(r#"{{"version":1,"restrictions":{{"r1":{odd},"r2":{{"name":"G","rule":{{"kind":"car_free"}},"ring":[[-73.6,45.5]]}}}}}}"#),
        );
        assert_eq!(names(&s), [("r2".to_string(), "G".to_string())]);
        assert!(!s.is_future());
        let kept = |storage: &MemoryStorage| storage.recall(RESTRICTIONS_KEY).unwrap();
        assert_eq!(s.add("N", Rule::CarFree, RING.to_vec()).unwrap().id, "r3");
        assert!(kept(&storage).contains(odd));
        assert!(s.update("r2", |r| r.name = "G2".into()));
        assert!(kept(&storage).contains(odd));
        assert!(s.remove("r3"));
        assert!(kept(&storage).contains(odd));
        assert_eq!(names(&s), [("r2".to_string(), "G2".to_string())]);
        assert_eq!(storage.recall(DAMAGED_KEY), None, "nothing was damaged");
        assert!(s.restore(&Restriction { id: "r1".into(), name: "x".into(), rule: Rule::CarFree, ring: RING.to_vec() }).unwrap().id != "r1");
        assert!(kept(&storage).contains(odd), "an unreadable record's id is taken");
    }

    #[test]
    fn a_second_damage_keeps_the_first_backup() {
        let (s, storage) = store();
        storage.remember(RESTRICTIONS_KEY, "{first");
        s.add("A", Rule::CarFree, RING.to_vec()).unwrap();
        storage.remember(RESTRICTIONS_KEY, "{second");
        s.add("B", Rule::CarFree, RING.to_vec()).unwrap();
        assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("{first"));
        assert_eq!(names(&s).len(), 1);
    }

    struct NoBackup(Rc<MemoryStorage>);
    impl Storage for NoBackup {
        fn recall(&self, key: &str) -> Option<String> {
            self.0.recall(key)
        }
        fn remember(&self, key: &str, value: &str) -> bool {
            key != DAMAGED_KEY && self.0.remember(key, value)
        }
        fn keys(&self) -> Vec<String> {
            self.0.keys()
        }
        fn now_ms(&self) -> u64 {
            self.0.now_ms()
        }
        fn random_hex(&self) -> Option<String> {
            self.0.random_hex()
        }
        fn identity(&self) -> usize {
            self.0.identity()
        }
    }

    #[test]
    fn a_damaged_document_that_cannot_be_backed_up_is_not_replaced() {
        let (_, _, storage) = test_ports();
        let s = RestrictionStore::new(Rc::new(NoBackup(storage.clone())));
        storage.remember(RESTRICTIONS_KEY, "{not json");
        assert_eq!(s.add("A", Rule::CarFree, RING.to_vec()), None);
        assert_eq!(storage.recall(RESTRICTIONS_KEY).as_deref(), Some("{not json"));
    }

    #[test]
    fn a_vertex_that_is_not_finite_is_not_kept() {
        let (s, _) = store();
        let bad = vec![[f64::NAN, 1.0], [2.0, 3.0], [4.0, f64::INFINITY]];
        assert_eq!(s.add("A", Rule::CarFree, bad.clone()), None);
        let a = s.add("A", Rule::CarFree, RING.to_vec()).unwrap();
        assert!(!s.update("r1", |r| r.ring = bad.clone()));
        assert_eq!(s.restore(&Restriction { id: "r9".into(), ring: bad, ..a.clone() }), None);
        assert_eq!(s.all(), [a]);
    }

    #[test]
    fn a_newer_document_is_kept_byte_for_byte_after_a_change_that_is_refused() {
        let (s, storage) = store();
        let newer = r#"{"version":2,"restrictions":{"r1":{"x":1}}}"#;
        storage.remember(RESTRICTIONS_KEY, newer);
        assert!(!s.update("r1", |r| r.name = "x".into()));
        assert!(!s.remove("r1"));
        assert_eq!(s.restore(&Restriction { id: "r7".into(), name: String::new(), rule: Rule::CarFree, ring: RING.to_vec() }), None);
        assert_eq!(storage.recall(RESTRICTIONS_KEY).as_deref(), Some(newer));
    }

    #[test]
    fn a_blocked_storage_keeps_nothing_and_says_so() {
        let (s, storage) = store();
        storage.blocked(true);
        assert_eq!(s.add("A", Rule::CarFree, RING.to_vec()), None);
        assert!(s.all().is_empty());
    }

    const MIXED: &str = r#"{"version":1,"next_id":3,"zone":{"a":1},"restrictions":{"r1":{"name":"A","rule":{"kind":"car_free"},"ring":[[-73.6,45.52],[-73.59,45.52],[-73.59,45.53]],"schedule":"weekdays"},"r2":{"name":"B","rule":{"kind":"car_free"},"ring":[[-73.6,45.52],[-73.59,45.52],[-73.59,45.53]],"tag":[1,2]}}}"#;

    fn mixed() -> (RestrictionStore, Rc<MemoryStorage>) {
        let (s, storage) = store();
        storage.remember(RESTRICTIONS_KEY, MIXED);
        (s, storage)
    }

    fn kept(storage: &MemoryStorage) -> serde_json::Value {
        serde_json::from_str(&storage.recall(RESTRICTIONS_KEY).unwrap()).unwrap()
    }

    #[test]
    fn what_this_build_does_not_know_survives_every_kind_of_write() {
        let (s, storage) = mixed();
        let start = kept(&storage);
        // add: the other records and the top level are untouched
        let c = s.add("C", Rule::CarFree, RING.to_vec()).unwrap();
        let v = kept(&storage);
        assert_eq!(
            (&v["zone"], &v["restrictions"]["r1"]["schedule"], &v["restrictions"]["r2"]["tag"]),
            (&start["zone"], &start["restrictions"]["r1"]["schedule"], &start["restrictions"]["r2"]["tag"])
        );
        // update: the changed record keeps its own unknown field, takes the new known ones
        assert!(s.update("r1", |r| r.name = "A2".into()));
        let v = kept(&storage);
        assert_eq!((v["restrictions"]["r1"]["schedule"].as_str(), v["restrictions"]["r1"]["name"].as_str()), (Some("weekdays"), Some("A2")));
        assert_eq!(v["restrictions"]["r2"], start["restrictions"]["r2"]);
        assert_eq!(v["zone"], start["zone"]);
        // remove
        assert!(s.remove(&c.id));
        assert!(s.remove("r2"));
        let v = kept(&storage);
        assert_eq!((v["restrictions"]["r1"]["schedule"].as_str(), &v["zone"]), (Some("weekdays"), &start["zone"]));
        assert!(v["restrictions"].get("r2").is_none());
        // restore
        let back = s.restore(&Restriction { id: "r2".into(), name: "B".into(), rule: Rule::CarFree, ring: RING.to_vec() }).unwrap();
        assert_eq!(back.id, "r2");
        let v = kept(&storage);
        assert_eq!((v["restrictions"]["r1"]["schedule"].as_str(), &v["zone"]), (Some("weekdays"), &start["zone"]));
    }

    #[test]
    fn a_version_that_is_not_an_integer_is_a_newer_document_and_left_alone() {
        for text in [r#"{"version":1.0,"restrictions":{}}"#, r#"{"version":"2","restrictions":{}}"#] {
            let (s, storage) = store();
            storage.remember(RESTRICTIONS_KEY, text);
            assert!(s.is_future(), "{text}");
            assert_eq!(s.add("A", Rule::CarFree, RING.to_vec()), None);
            assert_eq!(storage.recall(RESTRICTIONS_KEY).as_deref(), Some(text));
            assert_eq!(storage.recall(DAMAGED_KEY), None);
        }
    }

    #[test]
    fn the_largest_id_does_not_overflow() {
        let (s, storage) = store();
        storage.remember(RESTRICTIONS_KEY, &format!(r#"{{"version":1,"next_id":{max},"restrictions":{{"r{max}":{{"name":"x"}}}}}}"#, max = u32::MAX));
        let before = storage.recall(RESTRICTIONS_KEY);
        assert_eq!(s.add("A", Rule::CarFree, RING.to_vec()), None, "no id is left, and the one kept is not written over");
        assert_eq!(storage.recall(RESTRICTIONS_KEY), before);
    }

    #[test]
    fn what_a_newer_build_added_inside_a_rule_survives_writes_to_other_records_and_to_its_own() {
        let (s, storage) = store();
        let ring = r#"[[-73.6,45.52],[-73.59,45.52],[-73.59,45.53]]"#;
        let r1 = format!(r#"{{"name":"A","ring":{ring},"rule":{{"hours":"7-9","kind":"car_free"}}}}"#);
        let r3 = format!(r#"{{"name":"S","ring":{ring},"rule":{{"kind":"speed_limit","kmh":30,"zone":"school"}}}}"#);
        storage.remember(RESTRICTIONS_KEY, &format!(r#"{{"version":1,"next_id":4,"restrictions":{{"r1":{r1},"r3":{r3}}}}}"#));
        let rule_of = |id: &str| kept(&storage)["restrictions"][id]["rule"].clone();
        let (hours, zone) = (serde_json::json!({"hours":"7-9","kind":"car_free"}), serde_json::json!({"kind":"speed_limit","kmh":30,"zone":"school"}));
        let other = s.add("B", Rule::CarFree, RING.to_vec()).unwrap();
        assert_eq!((rule_of("r1"), rule_of("r3")), (hours.clone(), zone.clone()), "an add");
        assert!(s.update(&other.id, |r| r.name = "B2".into()));
        assert_eq!((rule_of("r1"), rule_of("r3")), (hours.clone(), zone.clone()), "a rename of another record");
        assert!(s.remove(&other.id));
        assert_eq!((rule_of("r1"), rule_of("r3")), (hours.clone(), zone.clone()), "a removal");
        assert!(s.update("r1", |r| r.name = "A2".into()));
        assert!(s.update("r3", |r| r.name = "S2".into()));
        assert_eq!((rule_of("r1"), rule_of("r3")), (hours.clone(), zone.clone()), "a rename of its own record");
        assert_eq!(kept(&storage)["restrictions"]["r1"]["name"], "A2");
        s.restore(&Restriction { id: other.id.clone(), name: "B".into(), rule: Rule::CarFree, ring: RING.to_vec() }).unwrap();
        assert_eq!((rule_of("r1"), rule_of("r3")), (hours, zone), "a restore");
        assert!(s.update("r3", |r| r.rule = Rule::SpeedLimit { kmh: 20 }));
        assert_eq!(rule_of("r3"), serde_json::json!({"kind":"speed_limit","kmh":20}), "a changed rule replaces the rule whole");
    }
}
