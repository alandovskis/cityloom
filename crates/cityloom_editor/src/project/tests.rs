use std::rc::Rc;

use super::store::VERSION;
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
