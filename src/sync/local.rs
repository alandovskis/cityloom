//! The stores' records as the sync carries them: raw JSON by kind and key, read and written with nothing added or
//! dropped (a newer build's fields pass through), and never marked (a marked record would be sent back). A kind whose
//! document is damaged or from a newer build is not `usable`: nothing is read, listed, written or replaced for it.

use std::rc::Rc;

use serde_json::{Value, json};

use crate::city::edits::{self, Document, Draft, EDITS_KEY, JUNCTIONS, STREETS};
use crate::city::store::AREA_KEY;
use crate::place::area::Area;
use crate::restriction::store::RestrictionStore;
use crate::shared::ports::Storage;
use crate::sync::model::{AREA_RECORD, Id, Kind, valid_key};

type Shared = Rc<dyn Storage>;

fn edits_draft(storage: &Shared) -> Option<Draft> {
    match edits::draft(storage.recall(EDITS_KEY).as_deref()) {
        Ok(draft) => Some(draft),
        Err(Document::Empty) => Some(Draft::new()),
        Err(_) => None,
    }
}

fn section(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Street => Some(STREETS),
        Kind::Junction => Some(JUNCTIONS),
        Kind::Rule | Kind::Area => None,
    }
}

/// Whether this kind's document can be read and changed.
pub fn usable(storage: &Shared, kind: Kind) -> bool {
    match kind {
        Kind::Street | Kind::Junction => edits_draft(storage).is_some(),
        Kind::Rule => RestrictionStore::new(storage.clone()).is_usable(),
        Kind::Area => true,
    }
}

/// Every record the stores hold whose kind is usable and whose key the server would take.
pub fn ids(storage: &Shared) -> Vec<Id> {
    let mut out = Vec::new();
    if let Some(draft) = edits_draft(storage) {
        for kind in [Kind::Street, Kind::Junction] {
            let section = section(kind).expect("a street and a junction have a section");
            out.extend(draft.raw_keys(section).into_iter().map(|key| Id { kind, key }));
        }
    }
    let rules = RestrictionStore::new(storage.clone());
    out.extend(rules.raw_ids().into_iter().map(|key| Id { kind: Kind::Rule, key }));
    if storage.recall(AREA_KEY).is_some_and(|q| Area::from_query(&q).is_some()) {
        out.push(Id::new(Kind::Area, AREA_RECORD));
    }
    out.retain(|id| valid_key(&id.key));
    out
}

pub fn read(storage: &Shared, id: &Id) -> Option<Value> {
    match id.kind {
        Kind::Street | Kind::Junction => edits_draft(storage)?.get_raw(section(id.kind)?, &id.key).cloned(),
        Kind::Rule => RestrictionStore::new(storage.clone()).raw(&id.key),
        Kind::Area => storage.recall(AREA_KEY).filter(|q| Area::from_query(q).is_some()).map(|query| json!({ "query": query })),
    }
}

/// Keeps `body` as the record. False where the kind's document is unusable (damaged or a newer build's: it is not
/// written or replaced), the body is not what the kind holds (an area that is not an area) or storage refused.
pub fn write(storage: &Shared, id: &Id, body: &Value) -> bool {
    match id.kind {
        Kind::Street | Kind::Junction => {
            let (Some(mut draft), Some(section)) = (edits_draft(storage), section(id.kind)) else { return false };
            draft.put_raw(section, &id.key, body.clone()) && storage.remember(EDITS_KEY, &draft.to_json())
        }
        Kind::Rule => {
            // `put_raw` goes through the store's own write, which copies a damaged document aside and replaces it; the
            // sync never does that (the editors do, when the person next edits), so the document must be usable.
            let rules = RestrictionStore::new(storage.clone());
            rules.is_usable() && rules.put_raw(&id.key, body.clone())
        }
        Kind::Area => match body.get("query").and_then(Value::as_str) {
            Some(query) if Area::from_query(query).is_some() => storage.remember(AREA_KEY, query),
            _ => false,
        },
    }
}

/// Removes the record; false where the kind's document is unusable. The chosen area is never removed (there is always
/// one to open): that is a no-op that says yes.
pub fn remove(storage: &Shared, id: &Id) -> bool {
    match id.kind {
        Kind::Street | Kind::Junction => {
            let (Some(mut draft), Some(section)) = (edits_draft(storage), section(id.kind)) else { return false };
            draft.remove_raw(section, &id.key);
            storage.remember(EDITS_KEY, &draft.to_json())
        }
        Kind::Rule => {
            let rules = RestrictionStore::new(storage.clone());
            rules.is_usable() && rules.remove_raw(&id.key)
        }
        Kind::Area => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::{MemoryStorage, test_ports};
    use serde_json::json;

    fn storage() -> (Rc<dyn Storage>, Rc<MemoryStorage>) {
        let (ports, _, memory) = test_ports();
        (ports.storage, memory)
    }

    #[test]
    fn each_kind_is_listed_read_written_and_removed_without_marking() {
        let (s, memory) = storage();
        let street = Id::new(Kind::Street, "w1~n2~n3");
        let junction = Id::new(Kind::Junction, "n7");
        let rule = Id::new(Kind::Rule, "r-abc");
        let area = Id::new(Kind::Area, AREA_RECORD);
        assert_eq!(ids(&s), Vec::<Id>::new());
        let ring = json!([[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]]);
        let bodies = [
            (street.clone(), json!({"ways": [1], "street": {"x": 1}, "newer": {"z": [1]}})),
            (junction.clone(), json!({"nodes": [7]})),
            (rule.clone(), json!({"name": "R", "rule": {"kind": "car_free"}, "ring": ring})),
            (area.clone(), json!({"query": "area=45.5000,-73.6000&name=Home"})),
        ];
        for (id, body) in &bodies {
            assert!(write(&s, id, body), "{id:?}");
            assert_eq!(read(&s, id).as_ref(), Some(body), "{id:?}");
        }
        let mut listed = ids(&s);
        listed.sort();
        assert_eq!(listed, [street.clone(), junction.clone(), rule.clone(), area.clone()]);
        for (id, _) in &bodies {
            assert!(remove(&s, id), "{id:?}");
        }
        assert_eq!(read(&s, &street), None);
        assert_eq!(read(&s, &junction), None);
        assert_eq!(read(&s, &rule), None);
        assert!(read(&s, &area).is_some(), "the chosen area is never removed, only replaced");
        assert_eq!(memory.recall("cityloom-sync"), None, "reading and writing for the sync is not an edit");
    }

    fn many_zone_record(zones: usize) -> Value {
        use crate::city::edits::StreetRecord;
        use crate::shared::catalogue::Side;
        use crate::street::model::{Street, zoned};
        let starts: Vec<i32> = (0..zones as i32).map(|i| i * 5_000).collect();
        let mut street = zoned(Street::sample(0, Side::Right), &starts, zones as i32 * 5_000);
        for (i, zone) in street.zones.iter_mut().enumerate() {
            let mut own = street.segments.clone();
            own[0].width_mm += 10 * i as i32;
            zone.segments = Some(own);
            zone.name = Some(format!("Zone {i}"));
        }
        let record = StreetRecord { ways: [7].into(), nodes: [1, 2].into(), leaving: [(1, 90)].into(), street };
        serde_json::to_value(record).unwrap()
    }

    #[test]
    fn writing_a_zoned_street_through_the_sync_makes_the_document_version_6_so_an_older_build_will_not_touch_it() {
        let (s, memory) = storage();
        let version = || serde_json::from_str::<Value>(&memory.recall(EDITS_KEY).unwrap()).unwrap()["version"].clone();
        let plain = Id::new(Kind::Street, "w1~n2~n3");
        assert!(write(&s, &plain, &json!({"ways": [1], "street": {"x": 1}})));
        assert_eq!(version(), 5);
        let zoned = Id::new(Kind::Street, "w7~n1~n2");
        let body = many_zone_record(2);
        assert!(write(&s, &zoned, &body));
        assert_eq!(version(), 6);
        assert_eq!(read(&s, &zoned), Some(body), "and the record is read back whole");
        assert!(remove(&s, &zoned));
        assert_eq!(version(), 5, "the last zoned record is gone");
    }

    #[test]
    fn a_many_zone_street_record_fits_a_keepalive_batch() {
        use crate::sync::model::{Entry, KEEPALIVE_BYTES, SyncDoc, next_batch};
        let body = many_zone_record(20);
        let mut doc = SyncDoc::new();
        doc.records
            .insert("street/w7~n1~n2".into(), Entry { edited_at: "2027-01-15T08:00:00.000Z".into(), pending: true, deleted: false, rev: 0, blocked: false });
        let batch = next_batch(&doc, &|_| Some(body.clone()), &|_| true, true);
        assert_eq!(batch.len(), 1, "a 20-zone record is sent in a keepalive batch");
        let size = serde_json::to_string(&batch[0]).unwrap().len();
        assert!(size < KEEPALIVE_BYTES, "{size} bytes");
    }

    #[test]
    fn an_area_that_is_not_an_area_is_refused() {
        let (s, _) = storage();
        let area = Id::new(Kind::Area, AREA_RECORD);
        assert!(!write(&s, &area, &json!({"query": "area=nonsense"})));
        assert!(!write(&s, &area, &json!({"other": 1})));
        assert_eq!(read(&s, &area), None);
    }

    #[test]
    fn a_document_from_a_newer_build_or_a_damaged_one_is_unusable_and_never_written_or_replaced() {
        // sync_..._unusable: Review Focus 4
        let (s, memory) = storage();
        memory.remember(EDITS_KEY, r#"{"version":99,"streets":{}}"#);
        memory.remember("cityloom-restrictions", "{broken");
        assert!(!usable(&s, Kind::Street) && !usable(&s, Kind::Junction) && !usable(&s, Kind::Rule));
        assert!(usable(&s, Kind::Area));
        assert!(!write(&s, &Id::new(Kind::Street, "k"), &json!({})));
        assert!(!write(&s, &Id::new(Kind::Rule, "r-1"), &json!({"name": "x", "rule": {"kind": "car_free"}, "ring": []})));
        assert!(!remove(&s, &Id::new(Kind::Street, "k")));
        assert!(!remove(&s, &Id::new(Kind::Rule, "r-1")));
        assert_eq!(ids(&s), Vec::<Id>::new());
        assert_eq!(memory.recall(EDITS_KEY).as_deref(), Some(r#"{"version":99,"streets":{}}"#));
        assert_eq!(memory.recall("cityloom-restrictions").as_deref(), Some("{broken"));
        assert_eq!(memory.recall("cityloom-restrictions-damaged"), None, "no copy aside either: the editors do that when the person next edits");
    }

    #[test]
    fn a_key_the_server_would_refuse_is_not_listed() {
        let (s, _) = storage();
        assert!(write(&s, &Id::new(Kind::Street, "has space"), &json!({})));
        assert!(write(&s, &Id::new(Kind::Street, "fine"), &json!({})));
        assert_eq!(ids(&s), [Id::new(Kind::Street, "fine")]);
    }
}
