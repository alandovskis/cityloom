//! The stores' records as the sync carries them: raw JSON by kind and key, read and written with nothing added or
//! dropped (a newer build's fields pass through), and never marked (a marked record would be sent back). A kind whose
//! document is damaged or from a newer build, in any project, is not `usable`: nothing is read, listed, written or
//! replaced for it.
//!
//! Every function takes the ROOT storage. A record's key is its wire key (`shared::project_id`): the project is read
//! from it and the record is kept in that project's own documents (`ScopedStorage`). A `project` record is the
//! project's entry in the list (`ProjectStore`), its key the project id.

use std::rc::Rc;

use serde_json::{Value, json};

use crate::city::edits::{self, Document, Draft, EDITS_KEY, JUNCTIONS, STREETS};
use crate::city::store::AREA_KEY;
use crate::place::area::Area;
use crate::project::model::Project;
use crate::project::store::{self as projects, ProjectStore};
use crate::restriction::store::RestrictionStore;
use crate::shared::ports::Storage;
use crate::shared::project_id::{FIRST, split_key, valid, wire_key};
use crate::shared::scoped::ScopedStorage;
use crate::sync::model::{Id, Kind, valid_key};

type Shared = Rc<dyn Storage>;

/// The first project, always, and every listed one.
pub fn projects(storage: &Shared) -> Vec<String> {
    let mut out = vec![FIRST.to_string()];
    out.extend(ProjectStore::new(storage.clone()).list().into_iter().map(|p| p.id).filter(|id| id != FIRST));
    out
}

/// `project`'s own view of the root storage.
pub fn scoped(storage: &Shared, project: &str) -> Shared {
    Rc::new(ScopedStorage::new(storage.clone(), project))
}

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
        Kind::Rule | Kind::Area | Kind::Project => None,
    }
}

/// Whether one project's document of this kind can be read and changed.
fn usable_in(scoped: &Shared, kind: Kind) -> bool {
    match kind {
        Kind::Street | Kind::Junction => edits_draft(scoped).is_some(),
        Kind::Rule => RestrictionStore::new(scoped.clone()).is_usable(),
        Kind::Area | Kind::Project => true,
    }
}

/// Whether this kind's documents can be read and changed: the list for projects, else the kind's document in every
/// project (one damaged project pauses the kind, so nothing is ever written over it).
pub fn usable(storage: &Shared, kind: Kind) -> bool {
    match kind {
        Kind::Project => ProjectStore::new(storage.clone()).is_usable(),
        _ => projects(storage).iter().all(|project| usable_in(&scoped(storage, project), kind)),
    }
}

/// Every record the stores hold, in every project, whose kind is usable there and whose wire key the server would
/// take, and every listed project's own record. A key the server would refuse stays on this device, readable.
pub fn ids(storage: &Shared) -> Vec<Id> {
    let mut out = Vec::new();
    for project in projects(storage) {
        out.extend(projects::records(storage, &project).into_iter().map(|(kind, key)| Id { kind, key: wire_key(&project, &key) }));
    }
    out.extend(ProjectStore::new(storage.clone()).list().into_iter().filter(|p| !p.placeholder).map(|p| Id::new(Kind::Project, &p.id)));
    out.retain(|id| valid_key(&id.key));
    out
}

/// The record as kept. A placeholder project is not one: it never leaves the device.
pub fn read(storage: &Shared, id: &Id) -> Option<Value> {
    if id.kind == Kind::Project {
        return ProjectStore::new(storage.clone()).get(&id.key).filter(|p| !p.placeholder).and_then(|p| serde_json::to_value(p).ok());
    }
    let (project, key) = split_key(&id.key);
    let storage = &scoped(storage, project);
    match id.kind {
        Kind::Street | Kind::Junction => edits_draft(storage)?.get_raw(section(id.kind)?, key).cloned(),
        Kind::Rule => RestrictionStore::new(storage.clone()).raw(key),
        Kind::Area => storage.recall(AREA_KEY).filter(|q| Area::from_query(q).is_some()).map(|query| json!({ "query": query })),
        Kind::Project => None,
    }
}

/// Keeps `body` as the record. False where the kind's document is unusable (damaged or a newer build's: it is not
/// written or replaced), the body is not what the kind holds (an area that is not an area) or storage refused. A record
/// of a project the list does not hold adds a placeholder project, so it can be reached; the project's own record
/// replaces the placeholder.
pub fn write(storage: &Shared, id: &Id, body: &Value) -> bool {
    let list = ProjectStore::new(storage.clone());
    if id.kind == Kind::Project {
        // The key names the project, whatever the body says.
        let mut body = body.clone();
        let Some(fields) = body.as_object_mut() else { return false };
        fields.insert("id".into(), json!(id.key));
        let Ok(project) = serde_json::from_value::<Project>(body) else { return false };
        let project = Project { placeholder: false, ..project };
        return valid(&project.id) && Area::from_query(&project.area).is_some() && list.put_raw(project);
    }
    let (project, key) = split_key(&id.key);
    // The list is checked first: a record must not be kept where no project could reach it.
    let listed = list.get(project).is_some();
    if !listed && !list.is_usable() {
        return false;
    }
    write_in(&scoped(storage, project), id.kind, key, body) && (listed || list.ensure(project))
}

fn write_in(storage: &Shared, kind: Kind, key: &str, body: &Value) -> bool {
    match kind {
        Kind::Street | Kind::Junction => {
            let (Some(mut draft), Some(section)) = (edits_draft(storage), section(kind)) else { return false };
            draft.put_raw(section, key, body.clone()) && storage.remember(EDITS_KEY, &draft.to_json())
        }
        Kind::Rule => {
            // `put_raw` goes through the store's own write, which copies a damaged document aside and replaces it; the
            // sync never does that (the editors do, when the person next edits), so the document must be usable.
            let rules = RestrictionStore::new(storage.clone());
            rules.is_usable() && rules.put_raw(key, body.clone())
        }
        Kind::Area => match body.get("query").and_then(Value::as_str) {
            Some(query) if Area::from_query(query).is_some() => storage.remember(AREA_KEY, query),
            _ => false,
        },
        Kind::Project => false,
    }
}

/// Removes the record; false where the kind's document is unusable. A record that is not kept is removed already
/// (nothing is written, so no project's documents appear for it). The chosen area is never removed (there is always
/// one to open): that is a no-op that says yes. Removing a `project` record removes the project's places with it.
pub fn remove(storage: &Shared, id: &Id) -> bool {
    if id.kind == Kind::Project {
        return ProjectStore::new(storage.clone()).remove_raw(&id.key);
    }
    let (project, key) = split_key(&id.key);
    let storage = &scoped(storage, project);
    match id.kind {
        Kind::Street | Kind::Junction => {
            let (Some(mut draft), Some(section)) = (edits_draft(storage), section(id.kind)) else { return false };
            if draft.get_raw(section, key).is_none() {
                return true;
            }
            draft.remove_raw(section, key);
            storage.remember(EDITS_KEY, &draft.to_json())
        }
        Kind::Rule => {
            let rules = RestrictionStore::new(storage.clone());
            rules.is_usable() && (!rules.raw_ids().iter().any(|held| held == key) || rules.remove_raw(key))
        }
        Kind::Area | Kind::Project => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::store::PLACEHOLDER_NAME;
    use crate::shared::ports::{MemoryStorage, test_ports};
    use crate::sync::model::AREA_RECORD;
    use serde_json::json;

    /// Where the first project keeps its edits.
    const FIRST_EDITS: &str = "cityloom-edits@first";

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
        let version = || serde_json::from_str::<Value>(&memory.recall(FIRST_EDITS).unwrap()).unwrap()["version"].clone();
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
        memory.remember(FIRST_EDITS, r#"{"version":99,"streets":{}}"#);
        memory.remember("cityloom-restrictions@first", "{broken");
        assert!(!usable(&s, Kind::Street) && !usable(&s, Kind::Junction) && !usable(&s, Kind::Rule));
        assert!(usable(&s, Kind::Area));
        assert!(!write(&s, &Id::new(Kind::Street, "k"), &json!({})));
        assert!(!write(&s, &Id::new(Kind::Rule, "r-1"), &json!({"name": "x", "rule": {"kind": "car_free"}, "ring": []})));
        assert!(!remove(&s, &Id::new(Kind::Street, "k")));
        assert!(!remove(&s, &Id::new(Kind::Rule, "r-1")));
        assert_eq!(ids(&s), Vec::<Id>::new());
        assert_eq!(memory.recall(FIRST_EDITS).as_deref(), Some(r#"{"version":99,"streets":{}}"#));
        assert_eq!(memory.recall("cityloom-restrictions@first").as_deref(), Some("{broken"));
        assert_eq!(memory.recall("cityloom-restrictions-damaged@first"), None, "no copy aside either: the editors do that when the person next edits");
    }

    fn project_body(name: &str) -> Value {
        json!({"id": "ignored", "name": name, "area": crate::place::area::default_area().to_query(), "created_at": "2027-01-15T08:00:00.000Z"})
    }

    fn list_ids(s: &Rc<dyn Storage>) -> Vec<String> {
        ProjectStore::new(s.clone()).list().into_iter().map(|p| p.id).collect()
    }

    #[test]
    fn two_projects_keep_the_same_place_apart_under_their_wire_keys() {
        let (s, memory) = storage();
        assert!(write(&s, &Id::new(Kind::Project, "p1"), &project_body("Plateau")));
        let first = Id::new(Kind::Street, "w1");
        let other = Id::new(Kind::Street, "p1:w1");
        assert!(write(&s, &first, &json!({"mine": "first"})));
        assert!(write(&s, &other, &json!({"mine": "p1"})));
        let mut listed = ids(&s);
        listed.sort();
        assert_eq!(listed, [other.clone(), first.clone(), Id::new(Kind::Project, "p1")]);
        assert!(memory.recall("cityloom-edits@p1").unwrap().contains("\"p1\""));
        assert!(!memory.recall("cityloom-edits@p1").unwrap().contains("\"first\""));
        assert!(memory.recall("cityloom-edits@first").unwrap().contains("\"first\""));
        assert_eq!(memory.recall(EDITS_KEY), None, "the first project's records are its own too");
        assert_eq!(read(&s, &first), Some(json!({"mine": "first"})));
        assert_eq!(read(&s, &other), Some(json!({"mine": "p1"})));
        assert!(remove(&s, &other));
        assert_eq!(read(&s, &other), None);
        assert_eq!(read(&s, &first), Some(json!({"mine": "first"})), "removing it from p1 leaves the first project's");
        assert_eq!(memory.recall("cityloom-sync"), None);
    }

    #[test]
    fn rules_and_the_area_go_by_project_too() {
        let (s, memory) = storage();
        assert!(write(&s, &Id::new(Kind::Project, "p1"), &project_body("Plateau")));
        let ring = json!([[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]]);
        let rule = Id::new(Kind::Rule, "p1:r-abc");
        let area = Id::new(Kind::Area, "p1:current");
        assert!(write(&s, &rule, &json!({"name": "R", "rule": {"kind": "car_free"}, "ring": ring})));
        assert!(write(&s, &area, &json!({"query": "area=45.5000,-73.6000&name=Home"})));
        assert!(memory.recall("cityloom-restrictions@p1").is_some_and(|d| d.contains("r-abc")));
        assert_eq!(memory.recall("cityloom-area@p1").as_deref(), Some("area=45.5000,-73.6000&name=Home"));
        assert_eq!(memory.recall("cityloom-restrictions"), None);
        assert_eq!(read(&s, &Id::new(Kind::Rule, "r-abc")), None, "the first project has no such rule");
        let mut listed = ids(&s);
        listed.sort();
        assert_eq!(listed, [rule.clone(), area.clone(), Id::new(Kind::Project, "p1")]);
    }

    #[test]
    fn a_project_record_is_kept_in_the_list_without_a_mark_and_replaced_in_place() {
        let (s, memory) = storage();
        assert!(write(&s, &Id::new(Kind::Project, "p1"), &project_body("Plateau")));
        assert!(write(&s, &Id::new(Kind::Project, "p2"), &project_body("Rosemont")));
        assert!(write(&s, &Id::new(Kind::Project, "p1"), &project_body("Le Plateau")));
        let list = ProjectStore::new(s.clone()).list();
        assert_eq!(list.iter().map(|p| (p.id.as_str(), p.name.as_str())).collect::<Vec<_>>(), [("p1", "Le Plateau"), ("p2", "Rosemont")]);
        assert_eq!(read(&s, &Id::new(Kind::Project, "p1")).unwrap()["name"], "Le Plateau");
        assert_eq!(read(&s, &Id::new(Kind::Project, "p1")).unwrap()["id"], "p1", "the key names the project, not the body");
        assert_eq!(read(&s, &Id::new(Kind::Project, "p9")), None);
        assert_eq!(memory.recall("cityloom-sync"), None, "the sync's own writes are never marked");
    }

    #[test]
    fn a_project_record_that_is_not_a_project_is_refused() {
        let (s, _) = storage();
        let p1 = Id::new(Kind::Project, "p1");
        assert!(!write(&s, &p1, &json!({"name": "No area"})));
        assert!(!write(&s, &p1, &json!({"name": "X", "area": "area=nonsense", "created_at": "2027-01-15T08:00:00.000Z"})));
        assert!(!write(&s, &Id::new(Kind::Project, "Not An Id"), &project_body("X")));
        assert_eq!(list_ids(&s), Vec::<String>::new());
    }

    #[test]
    fn removing_a_project_record_removes_its_places_and_its_entry_without_a_mark() {
        let (s, memory) = storage();
        assert!(write(&s, &Id::new(Kind::Project, "p1"), &project_body("Plateau")));
        assert!(write(&s, &Id::new(Kind::Project, "p2"), &project_body("Rosemont")));
        assert!(write(&s, &Id::new(Kind::Street, "p1:w1"), &json!({"x": 1})));
        assert!(write(&s, &Id::new(Kind::Street, "p2:w1"), &json!({"x": 2})));
        assert!(ProjectStore::new(s.clone()).open("p1"));
        assert!(remove(&s, &Id::new(Kind::Project, "p1")));
        assert_eq!(list_ids(&s), ["p2"]);
        assert!(memory.recall("cityloom-edits@p1").is_none_or(|v| v.is_empty()));
        assert_eq!(read(&s, &Id::new(Kind::Street, "p1:w1")), None);
        assert_eq!(read(&s, &Id::new(Kind::Street, "p2:w1")), Some(json!({"x": 2})));
        assert_eq!(ProjectStore::new(s.clone()).last_opened(), None);
        assert_eq!(memory.recall("cityloom-sync"), None);
        assert!(remove(&s, &Id::new(Kind::Project, "p1")), "removing what is gone is done");
    }

    #[test]
    fn a_record_for_a_project_not_in_the_list_brings_a_placeholder_that_its_own_record_replaces() {
        let (s, memory) = storage();
        memory.set_now(1_800_000_000_000);
        assert!(write(&s, &Id::new(Kind::Street, "p9:w1"), &json!({"x": 1})));
        let placeholder = ProjectStore::new(s.clone()).get("p9").expect("its places are reachable");
        assert_eq!(placeholder.name, PLACEHOLDER_NAME);
        assert_eq!(placeholder.area, crate::place::area::default_area().to_query());
        assert_eq!(placeholder.created_at, crate::sync::model::stamp(1_800_000_000_000));
        assert!(placeholder.placeholder);
        assert!(write(&s, &Id::new(Kind::Street, "p9:w2"), &json!({"x": 2})));
        assert_eq!(ProjectStore::new(s.clone()).list().len(), 1, "one placeholder");
        // the placeholder never leaves the device: it would replace the project's name elsewhere
        assert_eq!(read(&s, &Id::new(Kind::Project, "p9")), None);
        assert!(!ids(&s).contains(&Id::new(Kind::Project, "p9")));
        assert!(ids(&s).contains(&Id::new(Kind::Street, "p9:w1")), "its places do go");
        assert!(write(
            &s,
            &Id::new(Kind::Project, "p9"),
            &json!({"name": "Verdun", "area": crate::place::area::default_area().to_query(), "created_at": "2027-01-15T08:00:00.000Z", "placeholder": true})
        ));
        let list = ProjectStore::new(s.clone()).list();
        assert_eq!(list.iter().map(|p| (p.id.as_str(), p.name.as_str(), p.placeholder)).collect::<Vec<_>>(), [("p9", "Verdun", false)]);
        assert!(ids(&s).contains(&Id::new(Kind::Project, "p9")));
        assert_eq!(read(&s, &Id::new(Kind::Street, "p9:w1")), Some(json!({"x": 1})));
        assert_eq!(memory.recall("cityloom-sync"), None);
    }

    #[test]
    fn removing_a_record_of_a_project_not_in_the_list_adds_nothing() {
        let (s, memory) = storage();
        assert!(remove(&s, &Id::new(Kind::Street, "p8:w1")));
        assert!(remove(&s, &Id::new(Kind::Rule, "p8:r-abc")));
        assert_eq!(list_ids(&s), Vec::<String>::new());
        assert_eq!(memory.recall("cityloom-edits@p8"), None);
        assert_eq!(memory.recall("cityloom-restrictions@p8"), None);
    }

    #[test]
    fn a_record_for_a_project_is_refused_while_the_list_cannot_be_changed() {
        let (s, memory) = storage();
        memory.remember(crate::project::PROJECTS_KEY, r#"{"version":99,"projects":[]}"#);
        assert!(!usable(&s, Kind::Project));
        assert!(!write(&s, &Id::new(Kind::Street, "p9:w1"), &json!({"x": 1})));
        assert!(!write(&s, &Id::new(Kind::Project, "p9"), &project_body("Verdun")));
        assert!(!remove(&s, &Id::new(Kind::Project, "p9")));
        assert_eq!(memory.recall("cityloom-edits@p9"), None);
    }

    #[test]
    fn a_damaged_document_in_any_project_pauses_its_kind() {
        let (s, memory) = storage();
        assert!(write(&s, &Id::new(Kind::Project, "p1"), &project_body("Plateau")));
        assert!(usable(&s, Kind::Street) && usable(&s, Kind::Rule) && usable(&s, Kind::Project));
        memory.remember("cityloom-edits@p1", "{broken");
        assert!(!usable(&s, Kind::Street) && !usable(&s, Kind::Junction));
        assert!(usable(&s, Kind::Rule));
        assert!(!write(&s, &Id::new(Kind::Street, "p1:w1"), &json!({})));
        assert_eq!(memory.recall("cityloom-edits@p1").as_deref(), Some("{broken"));
    }

    #[test]
    fn a_wire_key_over_two_hundred_characters_is_not_listed_but_stays_readable() {
        // Review Focus 1. A random project id has 14 characters: with its `:` a 190-character key becomes 205.
        let (s, _) = storage();
        let project = "p-0123456789ab";
        assert!(write(&s, &Id::new(Kind::Project, project), &project_body("Plateau")));
        let long = format!("w{}", "1".repeat(189));
        assert_eq!(long.len(), 190);
        let id = Id::new(Kind::Street, &format!("{project}:{long}"));
        assert_eq!(id.key.len(), 205);
        assert!(write(&s, &id, &json!({"x": 1})));
        assert!(write(&s, &Id::new(Kind::Street, &long), &json!({"x": 2})));
        let listed = ids(&s);
        assert!(!listed.contains(&id), "{} characters", id.key.len());
        assert!(listed.contains(&Id::new(Kind::Street, &long)), "the first project's bare key fits");
        assert_eq!(read(&s, &id), Some(json!({"x": 1})));
    }

    #[test]
    fn a_key_the_server_would_refuse_is_not_listed() {
        let (s, _) = storage();
        assert!(write(&s, &Id::new(Kind::Street, "has space"), &json!({})));
        assert!(write(&s, &Id::new(Kind::Street, "fine"), &json!({})));
        assert_eq!(ids(&s), [Id::new(Kind::Street, "fine")]);
    }
}
