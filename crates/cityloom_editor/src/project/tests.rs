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

// ---- migration of the work kept before projects existed ----

mod migration {
    use std::cell::Cell;

    use super::super::migrate_legacy;
    use super::*;
    use crate::city::store::CityStore;
    use crate::place::area::{Area, default_area};
    use crate::shared::ports::Ports;
    use crate::shared::project_id::FIRST;
    use crate::shared::scoped::SCOPED_KEYS;
    use crate::sync::model::stamp;
    use crate::sync::tests::{ServerSim, T0, ada, device};

    const NAME: &str = "My first project";

    fn world() -> (Ports, Rc<MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        (ports, storage)
    }

    fn legacy_all(storage: &MemoryStorage) {
        storage.remember("cityloom-edits", "{\"edits\":1}");
        storage.remember("cityloom-edits-damaged", "{not json");
        storage.remember("cityloom-restrictions", "{\"rules\":1}");
        storage.remember("cityloom-restrictions-damaged", "{also not");
        storage.remember("cityloom-area", &default_area().to_query());
    }

    fn snapshot(storage: &MemoryStorage) -> Vec<(String, Option<String>)> {
        let mut keys = storage.keys();
        keys.sort();
        keys.into_iter().map(|k| (k.clone(), storage.recall(&k))).collect()
    }

    fn montreal_elsewhere() -> String {
        Area::new("Verdun", 45.4586, -73.5719).to_query()
    }

    #[test]
    fn legacy_work_moves_into_the_first_project_and_the_old_keys_are_blank() {
        let (ports, storage) = world();
        legacy_all(&storage);
        assert!(migrate_legacy(&ports.storage, NAME));
        let list = ProjectStore::new(ports.storage.clone()).list();
        assert_eq!(list, vec![Project { id: "first".into(), name: NAME.into(), area: default_area().to_query(), created_at: stamp(0), placeholder: false }]);
        assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{\"edits\":1}"));
        assert_eq!(storage.recall("cityloom-edits-damaged@first").as_deref(), Some("{not json"));
        assert_eq!(storage.recall("cityloom-restrictions@first").as_deref(), Some("{\"rules\":1}"));
        assert_eq!(storage.recall("cityloom-restrictions-damaged@first").as_deref(), Some("{also not"));
        assert_eq!(storage.recall("cityloom-area@first"), Some(default_area().to_query()));
        for key in SCOPED_KEYS {
            assert!(storage.recall(key).is_none_or(|v| v.is_empty()), "{key} is blank");
        }
    }

    #[test]
    fn the_stored_area_is_the_projects_when_it_is_in_montreal() {
        let (ports, storage) = world();
        storage.remember("cityloom-edits", "{}");
        storage.remember("cityloom-area", &montreal_elsewhere());
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(ProjectStore::new(ports.storage.clone()).get(FIRST).unwrap().area, montreal_elsewhere());
        assert_eq!(storage.recall("cityloom-area@first"), Some(montreal_elsewhere()));
    }

    #[test]
    fn restrictions_alone_trigger_the_move() {
        let (ports, storage) = world();
        storage.remember("cityloom-restrictions", "{\"rules\":1}");
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-restrictions@first").as_deref(), Some("{\"rules\":1}"));
        assert_eq!(ProjectStore::new(ports.storage.clone()).get(FIRST).unwrap().area, default_area().to_query());
    }

    #[test]
    fn an_area_alone_creates_nothing() {
        let (ports, storage) = world();
        storage.remember("cityloom-area", &montreal_elsewhere());
        let before = snapshot(&storage);
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert!(ProjectStore::new(ports.storage.clone()).list().is_empty());
        assert_eq!(snapshot(&storage), before);
    }

    #[test]
    fn no_legacy_data_or_only_blank_data_creates_nothing() {
        let (ports, storage) = world();
        assert!(!migrate_legacy(&ports.storage, NAME));
        storage.remember("cityloom-edits", "");
        storage.remember("cityloom-restrictions", "  ");
        let before = snapshot(&storage);
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(snapshot(&storage), before);
        assert!(ProjectStore::new(ports.storage.clone()).list().is_empty());
    }

    #[test]
    fn a_second_run_returns_false_and_changes_nothing() {
        let (ports, storage) = world();
        legacy_all(&storage);
        assert!(migrate_legacy(&ports.storage, NAME));
        let before = snapshot(&storage);
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(snapshot(&storage), before);
    }

    #[test]
    fn with_a_project_already_listed_a_stray_legacy_key_stays_put() {
        let (ports, storage) = world();
        let s = ProjectStore::new(ports.storage.clone());
        s.create("Mine").unwrap();
        storage.remember("cityloom-edits", "{\"stray\":1}");
        let before = snapshot(&storage);
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(snapshot(&storage), before);
        assert_eq!(s.list().len(), 1);
    }

    #[test]
    fn a_project_that_is_a_real_first_is_not_adopted_again() {
        let (ports, storage) = world();
        let s = ProjectStore::new(ports.storage.clone());
        s.put_raw(Project { id: "first".into(), name: "Named".into(), area: default_area().to_query(), created_at: stamp(0), placeholder: false });
        storage.remember("cityloom-edits", "{\"stray\":1}");
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-edits").as_deref(), Some("{\"stray\":1}"));
        assert_eq!(s.get(FIRST).unwrap().name, "Named");
    }

    #[test]
    fn an_area_outside_montreal_or_unreadable_is_ignored_and_left_alone() {
        for bad in [Area::new("Berlin", 52.5, 13.4).to_query(), "area=nonsense".to_string(), "sample".to_string()] {
            let (ports, storage) = world();
            storage.remember("cityloom-edits", "{}");
            storage.remember("cityloom-area", &bad);
            assert!(migrate_legacy(&ports.storage, NAME), "{bad}");
            assert_eq!(ProjectStore::new(ports.storage.clone()).get(FIRST).unwrap().area, default_area().to_query(), "{bad}");
            assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{}"));
            assert!(storage.recall("cityloom-area@first").is_none(), "{bad}");
            assert_eq!(storage.recall("cityloom-area").as_deref(), Some(bad.as_str()), "not lost");
        }
    }

    #[test]
    fn a_copy_the_storage_refuses_leaves_everything_in_place_and_the_next_run_finishes() {
        let (ports, storage) = world();
        legacy_all(&storage);
        storage.refuse("cityloom-restrictions@first");
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert!(ProjectStore::new(ports.storage.clone()).list().is_empty(), "no project while the work is not all copied");
        assert_eq!(storage.recall("cityloom-edits").as_deref(), Some("{\"edits\":1}"));
        assert_eq!(storage.recall("cityloom-restrictions").as_deref(), Some("{\"rules\":1}"));
        assert_eq!(storage.recall("cityloom-area"), Some(default_area().to_query()));
        storage.refuse_nothing();
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-restrictions@first").as_deref(), Some("{\"rules\":1}"));
        assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{\"edits\":1}"));
        assert_eq!(ProjectStore::new(ports.storage.clone()).list().len(), 1);
        assert!(storage.recall("cityloom-edits").is_none_or(|v| v.is_empty()));
    }

    #[test]
    fn a_storage_that_refuses_everything_loses_nothing() {
        let (ports, storage) = world();
        legacy_all(&storage);
        storage.blocked(true);
        let before = snapshot(&storage);
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(snapshot(&storage), before);
    }

    #[test]
    fn a_list_that_cannot_be_kept_leaves_the_legacy_keys_and_the_next_run_finishes() {
        let (ports, storage) = world();
        legacy_all(&storage);
        storage.refuse(PROJECTS_KEY);
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-edits").as_deref(), Some("{\"edits\":1}"));
        assert!(storage.recall(PROJECTS_KEY).is_none());
        storage.refuse_nothing();
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{\"edits\":1}"));
        assert_eq!(ProjectStore::new(ports.storage.clone()).list().len(), 1);
    }

    #[test]
    fn an_old_key_the_storage_will_not_blank_is_reported_and_the_work_is_in_the_project() {
        let (ports, storage) = world();
        legacy_all(&storage);
        storage.refuse("cityloom-edits");
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{\"edits\":1}"));
        assert_eq!(ProjectStore::new(ports.storage.clone()).list().len(), 1);
    }

    #[test]
    fn a_damaged_or_newer_list_is_not_touched_and_nothing_moves() {
        for raw in ["{not json", "{\"version\":99,\"projects\":[]}"] {
            let (ports, storage) = world();
            legacy_all(&storage);
            storage.remember(PROJECTS_KEY, raw);
            let before = snapshot(&storage);
            assert!(!migrate_legacy(&ports.storage, NAME), "{raw}");
            assert_eq!(snapshot(&storage), before, "{raw}");
        }
    }

    /// A document of this version with the given street records, as the sync or an editor writes it.
    fn edits_doc(streets: &[(&str, &str)]) -> String {
        let mut draft = crate::city::edits::Draft::new();
        for (key, v) in streets {
            assert!(draft.put_raw(crate::city::edits::STREETS, key, serde_json::json!({ "v": v })));
        }
        draft.to_json()
    }

    fn street_of(storage: &MemoryStorage, key: &str, id: &str) -> Option<String> {
        let draft = crate::city::edits::draft(storage.recall(key).as_deref()).ok()?;
        draft.get_raw(crate::city::edits::STREETS, id)?["v"].as_str().map(str::to_string)
    }

    #[test]
    fn pulled_rows_and_legacy_work_are_merged_record_by_record() {
        let (ports, storage) = world();
        let s = ProjectStore::new(ports.storage.clone());
        // the sync pulled server rows into a placeholder `first` before the legacy work was moved
        assert!(s.ensure(FIRST));
        storage.remember("cityloom-edits@first", &edits_doc(&[("shared", "pulled"), ("settled", "pulled"), ("only-pulled", "pulled")]));
        storage.remember("cityloom-edits", &edits_doc(&[("shared", "legacy"), ("settled", "legacy"), ("only-legacy", "legacy")]));
        // an unsent local edit of `shared` and of `only-legacy`; `settled` was synced
        assert!(mark(&*ports.storage, Kind::Street, "shared", false));
        assert!(mark(&*ports.storage, Kind::Street, "only-legacy", false));
        assert!(migrate_legacy(&ports.storage, NAME));
        let kept = "cityloom-edits@first";
        assert_eq!(street_of(&storage, kept, "shared").as_deref(), Some("legacy"), "the pending local edit wins");
        assert_eq!(street_of(&storage, kept, "settled").as_deref(), Some("pulled"), "no pending edit: the pulled one stands");
        assert_eq!(street_of(&storage, kept, "only-pulled").as_deref(), Some("pulled"));
        assert_eq!(street_of(&storage, kept, "only-legacy").as_deref(), Some("legacy"), "nothing stranded");
        assert!(storage.recall("cityloom-edits").is_none_or(|v| v.is_empty()));
        assert!(!s.get(FIRST).unwrap().placeholder);
    }

    #[test]
    fn rules_are_merged_by_id_the_same_way() {
        let (ports, storage) = world();
        let s = ProjectStore::new(ports.storage.clone());
        assert!(s.ensure(FIRST));
        let scoped: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(ports.storage.clone(), FIRST));
        let pulled = crate::restriction::store::RestrictionStore::new(scoped.clone());
        let legacy = crate::restriction::store::RestrictionStore::new(ports.storage.clone());
        for id in ["r-shared", "r-settled", "r-pulled"] {
            assert!(pulled.put_raw(id, serde_json::json!({ "v": "pulled" })));
        }
        for id in ["r-shared", "r-settled", "r-legacy"] {
            assert!(legacy.put_raw(id, serde_json::json!({ "v": "legacy" })));
        }
        assert!(mark(&*ports.storage, Kind::Rule, "r-shared", false));
        assert!(mark(&*ports.storage, Kind::Rule, "r-legacy", false));
        assert!(migrate_legacy(&ports.storage, NAME));
        let v = |id: &str| pulled.raw(id).map(|r| r["v"].clone());
        assert_eq!(v("r-shared"), Some("legacy".into()));
        assert_eq!(v("r-settled"), Some("pulled".into()));
        assert_eq!(v("r-pulled"), Some("pulled".into()));
        assert_eq!(v("r-legacy"), Some("legacy".into()));
        assert!(storage.recall("cityloom-restrictions").is_none_or(|x| x.is_empty()));
    }

    #[test]
    fn a_scoped_area_is_kept_over_the_legacy_one() {
        let (ports, storage) = world();
        let s = ProjectStore::new(ports.storage.clone());
        assert!(s.ensure(FIRST));
        storage.remember("cityloom-edits", &edits_doc(&[("a", "x")]));
        storage.remember("cityloom-area@first", &default_area().to_query());
        storage.remember("cityloom-area", &montreal_elsewhere());
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-area@first"), Some(default_area().to_query()));
    }

    #[test]
    fn a_scoped_document_that_cannot_be_merged_leaves_everything_and_the_project_unlisted() {
        let newer = "{\"version\":99,\"streets\":{}}";
        for scoped in ["{not json", newer] {
            let (ports, storage) = world();
            let s = ProjectStore::new(ports.storage.clone());
            assert!(s.ensure(FIRST));
            storage.remember("cityloom-edits@first", scoped);
            storage.remember("cityloom-edits", &edits_doc(&[("a", "x")]));
            storage.remember("cityloom-restrictions", "{\"rules\":1}");
            let before = snapshot(&storage);
            assert!(!migrate_legacy(&ports.storage, NAME), "{scoped}");
            assert_eq!(snapshot(&storage), before, "{scoped}: nothing written, nothing blanked");
            assert!(s.get(FIRST).unwrap().placeholder, "{scoped}: still a placeholder, so a later run can retry");
        }
    }

    #[test]
    fn a_legacy_document_that_cannot_be_read_is_not_merged_over_a_pulled_one() {
        let (ports, storage) = world();
        let s = ProjectStore::new(ports.storage.clone());
        assert!(s.ensure(FIRST));
        storage.remember("cityloom-edits@first", &edits_doc(&[("a", "pulled")]));
        storage.remember("cityloom-edits", "{not json");
        let before = snapshot(&storage);
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(snapshot(&storage), before);
    }

    /// A storage that says it kept `key` and keeps something else.
    struct Lying {
        inner: Rc<MemoryStorage>,
        key: &'static str,
    }

    impl Storage for Lying {
        fn recall(&self, key: &str) -> Option<String> {
            self.inner.recall(key)
        }
        fn remember(&self, key: &str, value: &str) -> bool {
            self.inner.remember(key, if key == self.key && !value.is_empty() { "garbage" } else { value })
        }
        fn keys(&self) -> Vec<String> {
            self.inner.keys()
        }
        fn now_ms(&self) -> u64 {
            self.inner.now_ms()
        }
        fn random_hex(&self) -> Option<String> {
            self.inner.random_hex()
        }
        fn identity(&self) -> usize {
            self.inner.identity()
        }
    }

    #[test]
    fn a_copy_that_reads_back_differently_is_undone_so_the_next_run_is_not_blocked() {
        let (_, storage) = world();
        legacy_all(&storage);
        let lying: Rc<dyn Storage> = Rc::new(Lying { inner: storage.clone(), key: "cityloom-restrictions@first" });
        assert!(!migrate_legacy(&lying, NAME));
        assert!(storage.recall("cityloom-restrictions@first").is_none_or(|v| v.is_empty()), "the bad copy is not left behind");
        assert_eq!(storage.recall("cityloom-restrictions").as_deref(), Some("{\"rules\":1}"));
        assert!(ProjectStore::new(lying.clone()).list().is_empty());
        let honest: Rc<dyn Storage> = storage.clone();
        assert!(migrate_legacy(&honest, NAME));
        assert_eq!(storage.recall("cityloom-restrictions@first").as_deref(), Some("{\"rules\":1}"));
    }

    #[test]
    fn a_conflicting_salvage_copy_is_left_alone_and_reported() {
        let (ports, storage) = world();
        legacy_all(&storage);
        storage.remember("cityloom-edits-damaged@first", "{older damaged");
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage.recall("cityloom-edits-damaged@first").as_deref(), Some("{older damaged"));
        assert_eq!(storage.recall("cityloom-edits-damaged").as_deref(), Some("{not json"));
        assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{\"edits\":1}"), "the rest moved");
    }

    #[test]
    fn a_copy_an_earlier_run_made_is_recognised_and_the_old_key_blanked() {
        let (ports, storage) = world();
        storage.remember("cityloom-edits", "{\"edits\":1}");
        storage.remember("cityloom-edits@first", "{\"edits\":1}");
        assert!(migrate_legacy(&ports.storage, NAME));
        assert!(storage.recall("cityloom-edits").is_none_or(|v| v.is_empty()));
        assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{\"edits\":1}"));
    }

    #[test]
    fn a_placeholder_first_the_sync_made_is_adopted() {
        let (ports, storage) = world();
        storage.set_now(777_000);
        let s = ProjectStore::new(ports.storage.clone());
        assert!(s.ensure(FIRST));
        assert!(s.get(FIRST).unwrap().placeholder);
        // a pulled bare-key record landed in the project; this device also has legacy work
        storage.remember("cityloom-edits", "{\"edits\":1}");
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(
            s.list(),
            vec![Project { id: "first".into(), name: NAME.into(), area: default_area().to_query(), created_at: stamp(0), placeholder: false }]
        );
        assert_eq!(storage.recall("cityloom-edits@first").as_deref(), Some("{\"edits\":1}"));
        assert!(storage.recall("cityloom-edits").is_none_or(|v| v.is_empty()));
        assert!(entry_of(&storage, &Id::new(Kind::Project, "first")).is_some_and(|e| e.pending), "now sent");
    }

    #[test]
    fn a_placeholder_alone_does_not_start_a_migration_and_another_placeholder_is_not_adopted() {
        let (ports, _) = world();
        let s = ProjectStore::new(ports.storage.clone());
        assert!(s.ensure(FIRST));
        assert!(!migrate_legacy(&ports.storage, NAME), "no legacy data");
        assert!(s.get(FIRST).unwrap().placeholder);
        let (ports, storage2) = world();
        let s = ProjectStore::new(ports.storage.clone());
        assert!(s.ensure("p1"));
        storage2.remember("cityloom-edits", "{}");
        assert!(!migrate_legacy(&ports.storage, NAME));
        assert_eq!(storage2.recall("cityloom-edits").as_deref(), Some("{}"));
    }

    fn legacy_street(d: &crate::sync::tests::Device, by: i32) -> String {
        let store = CityStore::sample(d.ports.storage.clone());
        let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
        s.segments[0].width_mm -= by;
        s.segments[1].width_mm += by;
        assert!(store.write_street(1, s));
        store.open().street_record(1).unwrap().0
    }

    fn first_width(d: &crate::sync::tests::Device) -> i32 {
        let scoped: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(d.ports.storage.clone(), FIRST));
        CityStore::sample(scoped).open().street_editor(1, 0).unwrap().snapshot().segments[0].width_mm
    }

    #[test]
    fn migration_creates_no_entry_for_the_records_it_moves_and_leaves_them_to_the_claim() {
        let (ports, storage) = world();
        storage.set_now(9_000_000);
        let store = CityStore::sample(ports.storage.clone());
        let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(store.write_street(1, s));
        let street = store.open().street_record(1).unwrap().0;
        storage.remember("cityloom-area", &default_area().to_query());
        // work an older build wrote with no mark
        storage.remember(crate::sync::model::SYNC_KEY, "");
        assert!(migrate_legacy(&ports.storage, NAME));
        for id in [Id::new(Kind::Street, &street), Id::new(Kind::Area, crate::sync::model::AREA_RECORD)] {
            assert!(entry_of(&storage, &id).is_none(), "{} left for the claim, which keeps its own time", id.name());
        }
        let project = entry_of(&storage, &Id::new(Kind::Project, "first")).expect("the project is claimed");
        assert!(project.pending && !project.deleted);
        assert_eq!(project.edited_at, stamp(0), "older than any real edit or rename of it");
    }

    #[test]
    fn a_synced_records_entry_is_untouched_by_the_migration() {
        let (ports, storage) = world();
        storage.set_now(9_000_000);
        let store = CityStore::sample(ports.storage.clone());
        let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(store.write_street(1, s));
        let id = Id::new(Kind::Street, &store.open().street_record(1).unwrap().0);
        let mut doc = crate::sync::model::open(&*storage).unwrap();
        let synced = crate::sync::model::Entry { edited_at: stamp(1_000), pending: false, deleted: false, rev: 4, blocked: false };
        doc.records.insert(id.name(), synced.clone());
        assert!(crate::sync::model::save(&*storage, &doc));
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(entry_of(&storage, &id), Some(synced));
    }

    #[test]
    fn the_project_is_claimed_only_where_the_sync_document_has_no_entry_for_it() {
        let (ports, storage) = world();
        storage.set_now(9_000_000);
        storage.remember("cityloom-edits", "{}");
        let id = Id::new(Kind::Project, "first");
        let mut doc = crate::sync::model::open(&*storage).unwrap();
        let renamed = crate::sync::model::Entry { edited_at: stamp(5_000_000), pending: false, deleted: false, rev: 3, blocked: false };
        doc.records.insert(id.name(), renamed.clone());
        assert!(crate::sync::model::save(&*storage, &doc));
        assert!(migrate_legacy(&ports.storage, NAME));
        assert_eq!(entry_of(&storage, &id), Some(renamed), "a rename or a pulled record keeps its entry");
    }

    #[test]
    fn a_newer_edit_made_elsewhere_stands_on_both_devices_after_the_migration() {
        crate::sync::hub::reset();
        let server = ServerSim::new();
        let laptop = device(&server, T0 + 100_000);
        let phone = device(&server, T0 + 100_000);
        // both worked before projects and had synced: the street is the same, the phone's entry says so
        let key = legacy_street(&laptop, 100);
        assert_eq!(legacy_street(&phone, 100), key);
        laptop.vm.known(&ada());
        let row = server.row("street", &key).unwrap();
        let mut doc = crate::sync::model::open(&*phone.storage).unwrap();
        doc.user = Some("ada".into());
        doc.epoch = Some("e1".into());
        doc.rev = row.rev;
        let id = Id::new(Kind::Street, &key);
        doc.records
            .insert(id.name(), crate::sync::model::Entry { edited_at: row.edited_at.clone(), pending: false, deleted: false, rev: row.rev, blocked: false });
        assert!(crate::sync::model::save(&*phone.storage, &doc));
        // the laptop migrates, edits the street later and sends it
        assert!(migrate_legacy(&laptop.ports.storage, NAME));
        laptop.storage.set_now(T0 + 150_000);
        let scoped: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(laptop.ports.storage.clone(), FIRST));
        let store = CityStore::sample(scoped);
        let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(store.write_street(1, s));
        laptop.vm.sync();
        let newer = first_width(&laptop);
        // the phone migrates later, with its stale copy, and syncs
        phone.storage.set_now(T0 + 300_000);
        assert!(migrate_legacy(&phone.ports.storage, NAME));
        phone.vm.known(&ada());
        laptop.vm.sync();
        assert_eq!(first_width(&phone), newer, "the laptop's later edit reached the phone");
        assert_eq!(first_width(&laptop), newer, "and was not overwritten by the phone's stale copy");
    }

    #[test]
    fn two_devices_that_migrate_apart_end_with_one_first_project_and_no_duplicate_places() {
        crate::sync::hub::reset();
        let server = ServerSim::new();
        let laptop = device(&server, T0 + 100_000);
        let phone = device(&server, T0 + 200_000);
        let mut keys = Vec::new();
        for (d, by) in [(&laptop, 100), (&phone, 150)] {
            let store = CityStore::sample(d.ports.storage.clone());
            let mut s = store.open().street_editor(1, 0).unwrap().snapshot();
            s.segments[0].width_mm -= by;
            s.segments[1].width_mm += by;
            assert!(store.write_street(1, s));
            keys.push(store.open().street_record(1).unwrap().0);
            d.storage.remember("cityloom-area", &default_area().to_query());
        }
        assert_eq!(keys[0], keys[1]);
        assert!(migrate_legacy(&laptop.ports.storage, NAME));
        assert!(migrate_legacy(&phone.ports.storage, NAME));
        laptop.vm.known(&ada());
        let report = Rc::new(Cell::new(crate::sync::model::Report::default()));
        let r = report.clone();
        phone.vm.enable("ada", Box::new(move |x| r.set(x)));
        assert!(report.get().ok);
        laptop.vm.sync();
        let mut live = server.live();
        live.sort();
        assert_eq!(live, [format!("area/{}", crate::sync::model::AREA_RECORD), "project/first".to_string(), format!("street/{}", keys[0])]);
        for d in [&laptop, &phone] {
            let list = ProjectStore::new(d.ports.storage.clone()).list();
            assert_eq!(list.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(), ["first"]);
            assert!(!list[0].placeholder);
            let streets = crate::project::store::records(&d.ports.storage, FIRST).into_iter().filter(|(k, _)| *k == Kind::Street).count();
            assert_eq!(streets, 1);
            assert!(d.storage.recall("cityloom-edits").is_none_or(|v| v.is_empty()));
        }
        // the later edit won on both
        let w = |d: &crate::sync::tests::Device| {
            let scoped: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(d.ports.storage.clone(), FIRST));
            CityStore::sample(scoped).open().street_editor(1, 0).unwrap().snapshot().segments[0].width_mm
        };
        assert_eq!(w(&laptop), w(&phone));
    }
}

mod context {
    use crate::project::context::{MISSING_HREF, Resolved, enter, leave_when_removed, open_page, resolve};
    use crate::project::{LAST_KEY, Project, ProjectStore};
    use crate::shared::ports::{Storage, TestPorts, test_ports_full};
    use crate::sync::hub;
    use crate::sync::model::{Change, Id, Kind, Origin, stamp};

    /// A browser with the projects `first` and `p1`.
    fn two_projects() -> TestPorts {
        let t = test_ports_full();
        let store = ProjectStore::new(t.ports.storage.clone());
        let first =
            Project { id: "first".into(), name: "First".into(), area: crate::place::area::default_area().to_query(), created_at: stamp(0), placeholder: false };
        assert!(store.put_raw(first));
        assert_eq!(store.create("Plateau").unwrap().id, "p1");
        t
    }

    #[test]
    fn the_project_the_address_names_is_opened() {
        let t = two_projects();
        assert_eq!(resolve(&t.ports.storage, "?project=p1"), Resolved::Project("p1".into()));
        assert_eq!(resolve(&t.ports.storage, "street=7&project=p1"), Resolved::Project("p1".into()));
    }

    #[test]
    fn a_project_that_is_not_here_is_missing_even_when_another_was_opened_last() {
        let t = two_projects();
        assert!(ProjectStore::new(t.ports.storage.clone()).open("first"));
        assert_eq!(resolve(&t.ports.storage, "?project=zzz"), Resolved::Missing);
        assert_eq!(resolve(&t.ports.storage, "?street=7&project="), Resolved::Missing);
        assert_eq!(resolve(&t.ports.storage, "?street=7&project"), Resolved::Missing);
    }

    #[test]
    fn with_no_project_named_the_one_opened_last_is_opened() {
        let t = two_projects();
        assert!(ProjectStore::new(t.ports.storage.clone()).open("first"));
        assert_eq!(resolve(&t.ports.storage, ""), Resolved::Project("first".into()));
        assert_eq!(resolve(&t.ports.storage, "?street=7"), Resolved::Project("first".into()));
    }

    #[test]
    fn with_no_project_named_and_none_opened_it_is_missing() {
        let t = two_projects();
        assert_eq!(resolve(&t.ports.storage, ""), Resolved::Missing);
        // one opened last and deleted since is none
        t.storage.remember(LAST_KEY, "p9");
        assert_eq!(resolve(&t.ports.storage, "?street=7"), Resolved::Missing);
    }

    #[test]
    fn entering_a_project_scopes_the_storage_and_remembers_it() {
        let t = two_projects();
        *t.page.path.borrow_mut() = "/street.html?street=7&project=p1".into();
        let ports = enter(t.ports.clone()).expect("p1 is here");
        assert_eq!(ports.storage.project().as_deref(), Some("p1"));
        assert_eq!(t.storage.recall(LAST_KEY).as_deref(), Some("p1"));
        assert!(t.navigator.take().is_empty());
        ports.storage.remember("cityloom-edits", "x");
        assert_eq!(t.storage.recall("cityloom-edits@p1").as_deref(), Some("x"));
        assert_eq!(t.storage.recall("cityloom-edits"), None);
    }

    #[test]
    fn a_page_for_a_deleted_project_goes_to_the_list_and_writes_nothing() {
        let t = two_projects();
        assert!(ProjectStore::new(t.ports.storage.clone()).delete("p1"));
        let before: Vec<(String, Option<String>)> = t.storage.keys().into_iter().map(|k| (k.clone(), t.storage.recall(&k))).collect();
        *t.page.path.borrow_mut() = "/street.html?street=7&project=p1".into();
        assert!(enter(t.ports.clone()).is_none());
        assert_eq!(t.navigator.take(), [MISSING_HREF]);
        let after: Vec<(String, Option<String>)> = t.storage.keys().into_iter().map(|k| (k.clone(), t.storage.recall(&k))).collect();
        assert_eq!(after, before, "nothing is written");
    }

    #[test]
    fn a_look_at_the_page_s_project_neither_sends_the_page_away_nor_remembers_it() {
        use crate::project::context::peek;
        let t = two_projects();
        *t.page.path.borrow_mut() = "/map.html?project=p1".into();
        assert_eq!(peek(t.ports.clone()).storage.project().as_deref(), Some("p1"));
        *t.page.path.borrow_mut() = "/map.html?project=gone".into();
        assert_eq!(peek(t.ports.clone()).storage.project(), None, "the page's own storage");
        assert!(t.navigator.take().is_empty());
        assert_eq!(t.storage.recall(LAST_KEY), None);
    }

    fn news(kind: Kind, key: &str, origin: Origin) -> Change {
        Change { id: Id::new(kind, key), origin }
    }

    #[test]
    fn the_page_leaves_for_the_list_when_a_sync_removes_its_project() {
        hub::reset();
        let t = two_projects();
        *t.page.path.borrow_mut() = "/map.html?project=p1".into();
        assert!(open_page(t.ports.clone()).is_some());
        // another project's news, a place's, or its own while it is still listed: the page stays
        hub::publish(&[news(Kind::Project, "first", Origin::Remote)]);
        hub::publish(&[news(Kind::Project, "p1", Origin::Remote)]);
        hub::publish(&[news(Kind::Street, "p1:w1~n1~n2", Origin::Remote)]);
        assert!(t.navigator.take().is_empty());
        assert!(ProjectStore::new(t.ports.storage.clone()).remove_raw("p1"));
        hub::publish(&[news(Kind::Project, "p1", Origin::Remote)]);
        assert_eq!(t.navigator.take(), [MISSING_HREF]);
        hub::reset();
    }

    #[test]
    fn a_replaced_record_that_takes_the_project_away_sends_the_page_away_too() {
        hub::reset();
        let t = two_projects();
        leave_when_removed(&t.ports, "first");
        assert!(ProjectStore::new(t.ports.storage.clone()).remove_raw("first"));
        hub::publish(&[news(Kind::Project, "first", Origin::Replaced)]);
        assert_eq!(t.navigator.take(), [MISSING_HREF]);
        hub::reset();
    }
}
