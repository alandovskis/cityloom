use std::rc::Rc;

use super::store::{PLACEHOLDER_NAME, VERSION};
use super::*;
use crate::shared::ports::{MemoryStorage, Storage, test_ports};
use crate::shared::project_id::valid;
use crate::sync::model::{Id, Kind, Loaded, load, mark};

fn store() -> (ProjectStore, Rc<MemoryStorage>) {
    let (ports, _, storage) = test_ports();
    (ProjectStore::new(ports.storage), storage)
}

fn entry_of(storage: &MemoryStorage, id: &Id) -> Option<crate::sync::model::Entry> {
    let Loaded::Current(doc) = load(storage) else { return None };
    doc.entry(id).cloned()
}

#[test]
fn create_trims_keeps_and_gives_the_default_area() {
    let (s, storage) = store();
    storage.set_now(1_000);
    let p = s.create("  Plateau  ").unwrap();
    assert_eq!(p.name, "Plateau");
    assert_eq!(p.area, crate::place::area::default_area().to_query());
    assert_eq!(p.created_at, crate::sync::model::stamp(1_000));
    assert!(valid(&p.id));
    assert_eq!(s.get(&p.id), Some(p.clone()));
    assert_eq!(s.list(), vec![p]);
}

#[test]
fn ids_are_random_where_storage_has_randomness_and_numbered_where_it_has_none() {
    let (s, storage) = store();
    let a = s.create("A").unwrap();
    let b = s.create("B").unwrap();
    assert_eq!((a.id.as_str(), b.id.as_str()), ("p1", "p2"));
    storage.with_randomness(true);
    let c = s.create("C").unwrap();
    assert!(c.id.starts_with("p-") && c.id.len() == 14 && valid(&c.id), "{}", c.id);
}

#[test]
fn a_name_that_is_empty_or_over_eighty_characters_is_refused() {
    let (s, storage) = store();
    assert_eq!(s.create("  "), Err(Refused::Empty));
    assert_eq!(s.create(&"é".repeat(81)), Err(Refused::TooLong));
    assert!(s.create(&"é".repeat(80)).is_ok());
    assert_eq!(s.list().len(), 1);
    assert!(storage.recall(DAMAGED_KEY).is_none());
}

#[test]
fn the_list_keeps_creation_order() {
    let (s, _) = store();
    for name in ["B", "A", "C"] {
        s.create(name).unwrap();
    }
    assert_eq!(s.list().iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["B", "A", "C"]);
}

#[test]
fn rename_changes_only_the_name() {
    let (s, _) = store();
    let a = s.create("A").unwrap();
    s.create("B").unwrap();
    assert_eq!(s.rename(&a.id, " Z "), Ok(()));
    let now = s.get(&a.id).unwrap();
    assert_eq!(now, Project { name: "Z".into(), ..a.clone() });
    assert_eq!(s.rename("nope", "X"), Err(Refused::Unknown));
    assert_eq!(s.rename(&a.id, ""), Err(Refused::Empty));
    assert_eq!(s.list().len(), 2);
}

#[test]
fn delete_removes_the_record_and_its_scoped_keys_and_no_other_projects() {
    let (s, storage) = store();
    let a = s.create("A").unwrap();
    let b = s.create("B").unwrap();
    for p in [&a, &b] {
        for key in crate::shared::scoped::SCOPED_KEYS {
            storage.remember(&format!("{key}@{}", p.id), "{\"x\":1}");
        }
    }
    storage.remember("cityloom-lang", "fr");
    assert!(s.open(&a.id));
    assert!(s.delete(&a.id));
    assert_eq!(s.list(), vec![b.clone()]);
    for key in crate::shared::scoped::SCOPED_KEYS {
        assert!(storage.recall(&format!("{key}@{}", a.id)).is_none_or(|v| v.is_empty()), "{key}");
        assert_eq!(storage.recall(&format!("{key}@{}", b.id)).as_deref(), Some("{\"x\":1}"));
    }
    assert_eq!(storage.recall("cityloom-lang").as_deref(), Some("fr"));
    assert_eq!(s.last_opened(), None);
    let tomb = entry_of(&storage, &Id::new(Kind::Project, &a.id)).unwrap();
    assert!(tomb.deleted && tomb.pending);
    assert!(!s.delete(&a.id), "already gone");
}

#[test]
fn open_remembers_a_project_that_exists_and_refuses_one_that_does_not() {
    let (s, _) = store();
    let a = s.create("A").unwrap();
    let b = s.create("B").unwrap();
    assert_eq!(s.last_opened(), None);
    assert!(s.open(&a.id));
    assert!(!s.open("nope"));
    assert_eq!(s.last_opened(), Some(a.id.clone()));
    assert!(s.open(&b.id));
    assert_eq!(s.last_opened(), Some(b.id));
}

#[test]
fn creating_marks_the_project_record_pending() {
    let (s, storage) = store();
    let p = s.create("A").unwrap();
    let e = entry_of(&storage, &Id::new(Kind::Project, &p.id)).unwrap();
    assert!(e.pending && !e.deleted);
}

#[test]
fn last_changed_is_the_newest_entry_of_the_project() {
    let (s, storage) = store();
    storage.set_now(1_000);
    let a = s.create("A").unwrap();
    let b = s.create("B").unwrap();
    assert_eq!(last_changed(&*storage, &a), a.created_at);
    storage.set_now(2_000);
    mark(&*storage, Kind::Street, &format!("{}:w1~n2~n3", a.id), false);
    storage.set_now(3_000);
    mark(&*storage, Kind::Rule, &format!("{}:r-abc", a.id), false);
    storage.set_now(9_000);
    mark(&*storage, Kind::Street, &format!("{}:w7~n8", b.id), false);
    assert_eq!(last_changed(&*storage, &a), crate::sync::model::stamp(3_000));
    assert_eq!(last_changed(&*storage, &b), crate::sync::model::stamp(9_000));
}

#[test]
fn a_newer_document_is_left_alone() {
    let (s, storage) = store();
    let newer = r#"{"version":99,"projects":[]}"#;
    storage.remember(PROJECTS_KEY, newer);
    assert!(!s.is_usable());
    assert_eq!(s.create("A"), Err(Refused::Unusable));
    assert_eq!(s.rename("x", "A"), Err(Refused::Unusable));
    assert!(!s.delete("x"));
    assert_eq!(storage.recall(PROJECTS_KEY).as_deref(), Some(newer));
    assert!(storage.recall(DAMAGED_KEY).is_none());
}

#[test]
fn a_damaged_document_is_copied_aside_only_by_the_next_successful_create() {
    let (s, storage) = store();
    storage.remember(PROJECTS_KEY, "{broken");
    assert!(!s.is_usable());
    assert!(s.list().is_empty());
    assert_eq!(s.create(""), Err(Refused::Empty));
    assert!(storage.recall(DAMAGED_KEY).is_none(), "a refused change copies nothing");
    assert_eq!(storage.recall(PROJECTS_KEY).as_deref(), Some("{broken"));
    let p = s.create("A").unwrap();
    assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("{broken"));
    assert_eq!(s.list(), vec![p]);
    assert!(s.is_usable());
    let doc: serde_json::Value = serde_json::from_str(&storage.recall(PROJECTS_KEY).unwrap()).unwrap();
    assert_eq!(doc["version"], VERSION);
}

#[test]
fn a_placeholder_is_listed_without_a_mark_and_renaming_it_makes_it_the_persons() {
    let (s, storage) = store();
    assert!(s.ensure("p9"));
    assert!(s.ensure("p9"), "already there");
    assert!(!s.ensure("Not An Id"));
    let p = s.get("p9").unwrap();
    assert!(p.placeholder && p.name == PLACEHOLDER_NAME);
    assert_eq!(s.list().len(), 1);
    assert_eq!(entry_of(&storage, &Id::new(Kind::Project, "p9")), None);
    assert_eq!(s.rename("p9", PLACEHOLDER_NAME), Ok(()));
    assert!(!s.get("p9").unwrap().placeholder);
    assert!(entry_of(&storage, &Id::new(Kind::Project, "p9")).is_some_and(|e| e.pending), "the same name, but now chosen: it is sent");
}

fn scoped(storage: &Rc<MemoryStorage>, project: &str) -> Rc<dyn Storage> {
    Rc::new(crate::shared::scoped::ScopedStorage::new(storage.clone(), project))
}

#[test]
fn delete_marks_every_record_of_the_project_deleted_and_no_other_projects() {
    // Ruling 1: the server and other devices drop the deleted project's places too.
    use crate::city::store::CityStore;
    use crate::restriction::model::Rule;
    use crate::restriction::store::RestrictionStore;
    let (s, storage) = store();
    let a = s.create("A").unwrap();
    let b = s.create("B").unwrap();
    let ring = vec![[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]];
    let mut keys = Vec::new();
    for p in [&a, &b] {
        let city = CityStore::sample(scoped(&storage, &p.id));
        let mut street = city.open().street_editor(1, 0).unwrap().snapshot();
        street.segments[0].width_mm -= 100;
        street.segments[1].width_mm += 100;
        assert!(city.write_street(1, street));
        keys.push(city.open().street_record(1).unwrap().0);
        RestrictionStore::new(scoped(&storage, &p.id)).add("Zone", Rule::CarFree, ring.clone()).unwrap();
        assert!(CityStore::choose(&*scoped(&storage, &p.id), &crate::place::area::default_area()));
    }
    // A junction known only to the sync document (its own document no longer holds it): marked deleted too.
    mark(&*storage, Kind::Junction, &format!("{}:n7", a.id), false);
    storage.set_now(5_000);
    assert!(s.delete(&a.id));
    let wire = |kind: Kind, key: &str| Id::new(kind, &crate::shared::project_id::wire_key(&a.id, key));
    for id in [wire(Kind::Street, &keys[0]), wire(Kind::Rule, "r1"), wire(Kind::Area, "current"), wire(Kind::Junction, "n7"), Id::new(Kind::Project, &a.id)] {
        let e = entry_of(&storage, &id).unwrap_or_else(|| panic!("{id:?} has an entry"));
        assert!(e.deleted && e.pending, "{id:?}");
        assert_eq!(e.edited_at, crate::sync::model::stamp(5_000), "{id:?}");
    }
    let other = |kind: Kind, key: &str| Id::new(kind, &crate::shared::project_id::wire_key(&b.id, key));
    for id in [other(Kind::Street, &keys[1]), other(Kind::Rule, "r1"), other(Kind::Area, "current")] {
        assert!(!entry_of(&storage, &id).unwrap_or_else(|| panic!("{id:?} has an entry")).deleted, "{id:?}");
    }
    let listed = crate::sync::local::ids(&(storage.clone() as Rc<dyn Storage>));
    assert!(listed.iter().all(|id| !id.key.starts_with(&format!("{}:", a.id))), "{listed:?}");
    assert!(listed.contains(&other(Kind::Street, &keys[1])));
}

#[test]
fn rename_and_delete_on_a_damaged_list_copy_nothing_aside_and_leave_it() {
    let (s, storage) = store();
    storage.remember(PROJECTS_KEY, "{broken");
    assert_eq!(s.rename("p1", "A"), Err(Refused::Unknown));
    assert!(!s.delete("p1"));
    assert!(storage.recall(DAMAGED_KEY).is_none(), "a refused change copies nothing");
    assert_eq!(storage.recall(PROJECTS_KEY).as_deref(), Some("{broken"));
}

#[test]
fn a_numbered_id_is_never_used_again_after_its_project_was_deleted() {
    let (s, storage) = store();
    let a = s.create("A").unwrap();
    let b = s.create("B").unwrap();
    assert!(s.delete(&b.id));
    assert_eq!(s.create("C").unwrap().id, "p3", "p2 was deleted: its tombstone must not come back to life");
    assert!(s.delete(&a.id));
    // a project the sync document knows of but the list does not
    mark(&*storage, Kind::Project, "p7", true);
    assert_eq!(s.create("D").unwrap().id, "p8");
}

#[test]
fn last_changed_of_the_first_project_counts_bare_keys_and_not_other_projects() {
    let (s, storage) = store();
    storage.set_now(1_000);
    let p1 = s.create("A").unwrap();
    let first = Project {
        id: crate::shared::project_id::FIRST.into(),
        name: "F".into(),
        area: p1.area.clone(),
        created_at: crate::sync::model::stamp(0),
        placeholder: false,
    };
    assert_eq!(last_changed(&*storage, &first), first.created_at, "the p1 project record is not the first project's");
    storage.set_now(2_000);
    mark(&*storage, Kind::Street, "w1~n2~n3", false);
    storage.set_now(4_000);
    mark(&*storage, Kind::Street, &format!("{}:w1~n2~n3", p1.id), false);
    assert_eq!(last_changed(&*storage, &first), crate::sync::model::stamp(2_000));
}

#[test]
fn last_changed_counts_the_projects_own_record() {
    let (s, storage) = store();
    storage.set_now(1_000);
    let a = s.create("A").unwrap();
    storage.set_now(2_000);
    mark(&*storage, Kind::Street, &format!("{}:w1", a.id), false);
    storage.set_now(7_000);
    s.rename(&a.id, "B").unwrap();
    assert_eq!(last_changed(&*storage, &a), crate::sync::model::stamp(7_000));
}
