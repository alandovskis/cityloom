//! The design archive: the format and its compatibility rules, the import's guarantees (nothing overwritten without a
//! choice, nothing changed by a bad archive), the commands, the section's markup and its words.

use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::{Value, json};

use crate::archive::RESOURCES;
use crate::archive::model::{self, Applied, ArchiveError, Choice, Status};
use crate::archive::text::Outcome;
use crate::archive::view::ArchiveView;
use crate::archive::vm::ArchiveVm;
use crate::city::edits::{self, EDITS_KEY, NEWEST};
use crate::city::store::CityStore;
use crate::restriction::model::Rule;
use crate::restriction::store::{RESTRICTIONS_KEY, RestrictionStore};
use crate::shared::i18n::Locale;
use crate::shared::i18n::tests::parity_problems;
use crate::shared::ports::{MemoryStorage, Storage, TestPorts, test_ports_full};
use crate::shared::testing::html;
use crate::sync::model::{self as sync, Id, Kind};

const RING: [[f64; 2]; 3] = [[-73.60, 45.52], [-73.59, 45.52], [-73.59, 45.53]];

fn device() -> TestPorts {
    let t = test_ports_full();
    t.storage.set_now(1_000_000_000_000);
    t
}

/// Work made through the stores' own write paths: a street, a junction and an area rule.
fn design(storage: &Rc<dyn Storage>) {
    let city = CityStore::sample(storage.clone());
    let view = city.open().view(0);
    let street = view.edges[0].uid;
    let mut s = city.open().street_editor(street, 0).unwrap().snapshot();
    s.segments[0].width_mm -= 100;
    s.segments[1].width_mm += 100;
    assert!(city.write_street(street, s));
    let node = view.nodes.iter().find(|n| n.junction).unwrap().uid;
    let mut j = city.open().junction_editor(node, 0).unwrap();
    assert!(j.set_corner(j.current().arms[0].uid, 7_000));
    assert!(city.write_junction(node, j.snapshot()));
    assert!(RestrictionStore::new(storage.clone()).add("Calm", Rule::CarFree, RING.to_vec()).is_some());
}

fn stored(storage: &MemoryStorage, key: &str) -> Value {
    serde_json::from_str(&storage.recall(key).unwrap_or_else(|| panic!("{key} is not kept"))).unwrap()
}

/// Every key and value the storage holds.
fn snapshot(storage: &MemoryStorage) -> BTreeMap<String, String> {
    storage.keys().into_iter().map(|k| (k.clone(), storage.recall(&k).unwrap())).collect()
}

fn archive_of(t: &TestPorts) -> String {
    model::export(&t.ports.storage).expect("exports")
}

fn raw(edits: Value, rules: Value) -> String {
    let mut doc = json!({"kind": "cityloom-design", "format_version": 1});
    if !edits.is_null() {
        doc["edits"] = edits;
    }
    if !rules.is_null() {
        doc["restrictions"] = rules;
    }
    doc.to_string()
}

fn edits_with(version: u64, streets: Value) -> Value {
    json!({"version": version, "streets": streets, "junctions": {}})
}

// ---- the round trip ----

#[test]
fn the_design_survives_export_then_import_into_an_empty_device() {
    let (a, b) = (device(), device());
    design(&a.ports.storage);
    let text = archive_of(&a);
    let archive = model::parse_and_validate(&text).expect("an archive this build made is whole");
    assert_eq!((archive.streets(), archive.junctions(), archive.rules()), (1, 1, 1));
    let done = model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap();
    assert_eq!(done, Applied { added: 3, replaced: 0, kept: 0, same: 0 });
    assert_eq!(stored(&b.storage, EDITS_KEY), stored(&a.storage, EDITS_KEY));
    assert_eq!(stored(&b.storage, RESTRICTIONS_KEY)["restrictions"], stored(&a.storage, RESTRICTIONS_KEY)["restrictions"]);
    // the imported work is an edit like any other: marked, so that a device that syncs sends it
    let doc = sync::open(&*b.ports.storage).expect("a sync document was written");
    for id in
        [Kind::Street, Kind::Junction, Kind::Rule].iter().flat_map(|k| crate::sync::local::ids(&b.ports.storage).into_iter().filter(move |i| i.kind == *k))
    {
        assert!(doc.entry(&id).is_some_and(|e| e.pending), "{id:?} waits to be sent");
    }
}

#[test]
fn importing_an_unchanged_archive_again_finds_everything_the_same_and_changes_nothing() {
    let (a, b) = (device(), device());
    design(&a.ports.storage);
    let archive = model::parse_and_validate(&archive_of(&a)).unwrap();
    model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap();
    let before = snapshot(&b.storage);
    let plan = model::plan_import(&archive, &b.ports.storage).unwrap();
    assert_eq!((plan.count(Status::Same), plan.count(Status::New), plan.count(Status::Conflict)), (3, 0, 0));
    let done = model::apply(&archive, &b.ports.storage, Choice::UseArchive).unwrap();
    assert_eq!(done, Applied { added: 0, replaced: 0, kept: 0, same: 3 });
    assert_eq!(snapshot(&b.storage), before);
    // and the archive of the device that imported it is the same design
    let again = model::parse_and_validate(&archive_of(&b)).unwrap();
    assert_eq!(model::plan_import(&again, &a.ports.storage).unwrap().count(Status::Same), 3);
}

#[test]
fn an_export_holds_the_design_and_what_made_it_and_nothing_of_the_account_or_the_network() {
    let a = device();
    design(&a.ports.storage);
    a.storage.remember("cityloom-lang", "fr-CA");
    a.storage.remember("cityloom-theme", "dark");
    a.storage.remember("cityloom-network", "{\"big\":true}");
    let doc: Value = serde_json::from_str(&archive_of(&a)).unwrap();
    let keys: Vec<&str> = doc.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["app_version", "area", "created_at", "edits", "format_version", "kind", "restrictions", "source"]);
    assert_eq!((doc["kind"].as_str(), doc["format_version"].as_u64()), (Some("cityloom-design"), Some(1)));
    assert_eq!(doc["created_at"], "2001-09-09T01:46:40.000Z");
    assert_eq!(doc["app_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(doc["source"]["importer_version"], crate::place::tiles::IMPORTER_VERSION);
    assert!(!doc.to_string().contains("fr-CA") && !doc.to_string().contains("big"));
}

#[test]
fn the_file_is_named_by_the_day_and_is_never_a_bare_dot_cityloom() {
    assert_eq!(model::file_name(1_000_000_000_000), "cityloom-2001-09-09.cityloom-design.json");
    assert!(!model::file_name(0).ends_with(".cityloom"));
}

// ---- nothing is overwritten without a choice ----

/// A device whose street is edited one way, and an archive whose same street is edited another, with one new rule.
fn conflict() -> (TestPorts, String, BTreeMap<String, String>) {
    let (a, b) = (device(), device());
    design(&a.ports.storage);
    design(&b.ports.storage);
    let city = CityStore::sample(a.ports.storage.clone());
    let street = city.open().view(0).edges[0].uid;
    let mut s = city.open().street_editor(street, 0).unwrap().snapshot();
    s.segments[0].width_mm -= 200;
    s.segments[1].width_mm += 200;
    assert!(city.write_street(street, s));
    assert!(RestrictionStore::new(a.ports.storage.clone()).add("Slow", Rule::SpeedLimit { kmh: 30 }, RING.to_vec()).is_some());
    let text = archive_of(&a);
    let before = snapshot(&b.storage);
    (b, text, before)
}

#[test]
fn a_place_that_differs_is_a_conflict_and_nothing_is_written_until_the_person_chooses() {
    let (b, text, before) = conflict();
    let vm = ArchiveVm::new(b.ports.clone(), crate::i18n_for(Locale::En));
    vm.import_text(&text);
    assert!(vm.confirming());
    assert_eq!(snapshot(&b.storage), before, "asking writes nothing");
    assert!(vm.summary_text().unwrap().contains("1 differ"), "{:?}", vm.summary_text());
    assert_eq!(b.announcer.take().len(), 1, "the question is announced");
    vm.cancel();
    assert_eq!((snapshot(&b.storage), vm.confirming()), (before, false));
    assert_eq!(vm.result_text().unwrap(), "Import cancelled. Nothing on this device was changed.");
}

#[test]
fn keeping_mine_adds_only_what_is_new_and_leaves_the_place_that_differs() {
    let (b, text, before) = conflict();
    let vm = ArchiveVm::new(b.ports.clone(), crate::i18n_for(Locale::En));
    vm.import_text(&text);
    vm.choose(Choice::KeepMine);
    assert_eq!(b.storage.recall(EDITS_KEY), before.get(EDITS_KEY).cloned(), "the street here is as it was");
    assert_eq!(RestrictionStore::new(b.ports.storage.clone()).all().len(), 2, "the new rule was added");
    assert!(vm.result_text().unwrap().starts_with("Import finished. Added: 1. Replaced: 0. Kept as they were here: 1."), "{:?}", vm.result_text());
}

#[test]
fn using_the_archive_replaces_the_place_that_differs_and_marks_it() {
    let (b, text, before) = conflict();
    let archive = model::parse_and_validate(&text).unwrap();
    let done = model::apply(&archive, &b.ports.storage, Choice::UseArchive).unwrap();
    assert_eq!((done.added, done.replaced, done.kept), (1, 1, 0));
    assert_ne!(b.storage.recall(EDITS_KEY), before.get(EDITS_KEY).cloned());
    let key = archive.plan_keys(Kind::Street).remove(0);
    assert_eq!(stored(&b.storage, EDITS_KEY)["streets"][&key], serde_json::from_str::<Value>(&text).unwrap()["edits"]["streets"][&key]);
    assert!(sync::open(&*b.ports.storage).unwrap().entry(&Id::new(Kind::Street, &key)).is_some_and(|e| e.pending));
}

impl model::Archive {
    fn plan_keys(&self, kind: Kind) -> Vec<String> {
        let b = device();
        model::plan_import(self, &b.ports.storage).unwrap().entries.into_iter().filter(|e| e.kind == kind).map(|e| e.key).collect()
    }
}

// ---- an archive that cannot be taken leaves everything as it was ----

fn valid() -> Value {
    serde_json::from_str(&{
        let a = device();
        design(&a.ports.storage);
        archive_of(&a)
    })
    .unwrap()
}

#[test]
fn every_kind_of_bad_archive_gets_its_own_error_and_leaves_storage_byte_identical() {
    let good = valid();
    let truncated = good.to_string()[..good.to_string().len() / 2].to_string();
    let with = |f: &dyn Fn(&mut Value)| {
        let mut v = good.clone();
        f(&mut v);
        v.to_string()
    };
    let cases: Vec<(&str, String, ArchiveError)> = vec![
        ("empty", String::new(), ArchiveError::NotJson),
        ("garbage", "not json at all".into(), ArchiveError::NotJson),
        ("binary", "\u{0}\u{1}\u{2}PK".into(), ArchiveError::NotJson),
        ("truncated", truncated, ArchiveError::NotJson),
        ("array", "[1,2]".into(), ArchiveError::NotAnArchive),
        ("area archive", r#"{"kind":"cityloom-area","format_version":1}"#.into(), ArchiveError::NotAnArchive),
        ("some other json", r#"{"streets":{}}"#.into(), ArchiveError::NotAnArchive),
        ("no format version", with(&|v| drop(v.as_object_mut().unwrap().remove("format_version"))), ArchiveError::Damaged(model::PART_HEADER)),
        ("format version zero", with(&|v| v["format_version"] = json!(0)), ArchiveError::Damaged(model::PART_HEADER)),
        ("format version as text", with(&|v| v["format_version"] = json!("1")), ArchiveError::Damaged(model::PART_HEADER)),
        ("future format", with(&|v| v["format_version"] = json!(2)), ArchiveError::FutureFormat(2)),
        ("far future format", with(&|v| v["format_version"] = json!(99)), ArchiveError::FutureFormat(99)),
        ("newer edits", with(&|v| v["edits"]["version"] = json!(NEWEST + 1)), ArchiveError::EditsNewer(u64::from(NEWEST) + 1)),
        ("old edits", with(&|v| v["edits"]["version"] = json!(3)), ArchiveError::EditsOld(3)),
        ("edits without a version", with(&|v| drop(v["edits"].as_object_mut().unwrap().remove("version"))), ArchiveError::Damaged(model::PART_EDITS)),
        ("edits that are not an object", with(&|v| v["edits"] = json!([1])), ArchiveError::Damaged(model::PART_EDITS)),
        ("streets that are not an object", with(&|v| v["edits"]["streets"] = json!(5)), ArchiveError::Damaged(model::PART_EDITS)),
        ("a street that is not an object", with(&|v| v["edits"]["streets"] = json!({"w1~n2~n3": 7})), ArchiveError::Damaged(model::PART_EDITS)),
        ("a key the sync cannot carry", with(&|v| v["edits"]["streets"] = json!({"a b": {}})), ArchiveError::Damaged(model::PART_EDITS)),
        ("newer rules", with(&|v| v["restrictions"]["version"] = json!(2)), ArchiveError::RulesNewer(2)),
        ("rules without a version", with(&|v| drop(v["restrictions"].as_object_mut().unwrap().remove("version"))), ArchiveError::Damaged(model::PART_RULES)),
        ("rules that are not a table", with(&|v| v["restrictions"]["restrictions"] = json!("x")), ArchiveError::Damaged(model::PART_RULES)),
        ("a rule that is not an object", with(&|v| v["restrictions"]["restrictions"] = json!({"r1": 1})), ArchiveError::Damaged(model::PART_RULES)),
        ("a header field of the wrong type", with(&|v| v["created_at"] = json!(5)), ArchiveError::Damaged(model::PART_HEADER)),
        ("too large", format!("{{\"kind\":\"cityloom-design\",\"pad\":\"{}\"}}", "x".repeat(model::MAX_BYTES)), ArchiveError::TooLarge),
    ];
    for (name, text, expected) in cases {
        assert_eq!(model::parse_and_validate(&text).unwrap_err(), expected, "{name}");
        // the same through the command: a specific message, and storage is untouched, on a device that has work
        let t = device();
        design(&t.ports.storage);
        let before = snapshot(&t.storage);
        let vm = ArchiveVm::new(t.ports.clone(), crate::i18n_for(Locale::En));
        t.announcer.take();
        vm.import_text(&text);
        assert_eq!(snapshot(&t.storage), before, "{name}");
        let said = vm.result_text().unwrap();
        assert!(said.ends_with("Nothing on this device was changed."), "{name}: {said}");
        assert!(vm.result_is_error() && !vm.confirming(), "{name}");
        assert_eq!(t.announcer.take(), vec![said], "{name}: the error is announced");
    }
}

#[test]
fn the_errors_say_what_is_wrong_in_both_languages() {
    let said = |locale, e: ArchiveError| crate::shared::said::say_now(&crate::i18n_for(locale), Default::default(), &e.said());
    assert!(said(Locale::En, ArchiveError::FutureFormat(2)).contains("format version 2, made by a newer CityLoom"));
    assert!(said(Locale::FrCa, ArchiveError::FutureFormat(2)).contains("version 2 du format"));
    assert!(said(Locale::En, ArchiveError::Damaged(model::PART_RULES)).contains("its area rules cannot be read"));
    assert!(said(Locale::FrCa, ArchiveError::Damaged(model::PART_EDITS)).contains("modifications des rues et des intersections"));
    assert!(said(Locale::En, ArchiveError::TooLarge).contains("10 MB"));
}

#[test]
fn local_work_that_cannot_take_records_stops_the_import_and_is_left_byte_identical() {
    let text = archive_of(&{
        let a = device();
        design(&a.ports.storage);
        a
    });
    let archive = model::parse_and_validate(&text).unwrap();
    for (key, bad, expected) in [
        (EDITS_KEY, "{ not json", ArchiveError::LocalEdits { newer: false }),
        (EDITS_KEY, r#"{"version":99,"streets":{}}"#, ArchiveError::LocalEdits { newer: true }),
        (EDITS_KEY, r#"{"version":2,"streets":{}}"#, ArchiveError::LocalEdits { newer: false }),
        (RESTRICTIONS_KEY, "garbage", ArchiveError::LocalRules { newer: false }),
        (RESTRICTIONS_KEY, r#"{"version":2,"restrictions":{}}"#, ArchiveError::LocalRules { newer: true }),
    ] {
        let b = device();
        b.storage.remember(key, bad);
        let before = snapshot(&b.storage);
        assert_eq!(model::plan_import(&archive, &b.ports.storage).unwrap_err(), expected, "{bad}");
        assert_eq!(model::apply(&archive, &b.ports.storage, Choice::UseArchive).unwrap_err(), expected, "{bad}");
        assert_eq!(snapshot(&b.storage), before, "{bad}: no copy aside, no marks, nothing written");
    }
}

#[test]
fn a_document_that_the_archive_has_nothing_for_does_not_stop_the_import() {
    let rules_only = raw(Value::Null, json!({"version": 1, "restrictions": {"r-1": {"name": "A", "rule": {"kind": "car_free"}, "ring": RING}}}));
    let b = device();
    b.storage.remember(EDITS_KEY, "damaged");
    let archive = model::parse_and_validate(&rules_only).unwrap();
    assert_eq!(model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap().added, 1);
    assert_eq!(b.storage.recall(EDITS_KEY).as_deref(), Some("damaged"));
}

#[test]
fn a_write_that_fails_puts_back_what_was_written() {
    let a = device();
    design(&a.ports.storage);
    let archive = model::parse_and_validate(&archive_of(&a)).unwrap();
    // the rules are written first; the edits then fail: the rules go back to what they were
    let b = device();
    assert!(RestrictionStore::new(b.ports.storage.clone()).add("Mine", Rule::CarFree, RING.to_vec()).is_some());
    let rules_before = b.storage.recall(RESTRICTIONS_KEY);
    b.storage.refuse(EDITS_KEY);
    assert_eq!(model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap_err(), ArchiveError::StorageRefused);
    assert_eq!(b.storage.recall(RESTRICTIONS_KEY), rules_before);
    assert_eq!(b.storage.recall(EDITS_KEY), None);
    let marked = sync::open(&*b.ports.storage).unwrap();
    assert!(marked.records.keys().all(|name| name.starts_with("rule/")), "only the rule b added itself is marked: {:?}", marked.records.keys());
}

// ---- compatibility ----

#[test]
fn fields_this_build_does_not_know_are_ignored_and_reserved_ones_are_neither_read_nor_written() {
    let text = json!({
        "kind": "cityloom-design", "format_version": 1, "app_version": "9.9.9", "created_at": "2030-01-01T00:00:00.000Z",
        "source": {"area_key": "1,2", "something_new": [1]}, "future_top_level": {"a": 1},
        "notes": {"w1~n2~n3": "a note"}, "scenarios": [{"name": "later"}],
        "edits": edits_with(5, json!({"w1~n2~n3": {"ways": [1], "street": {}, "from_a_newer_build": {"x": [1, 2]}}})),
    })
    .to_string();
    let archive = model::parse_and_validate(&text).expect("unknown fields are not an error");
    assert_eq!((archive.format_version, archive.app_version.as_str(), archive.source.area_key.as_deref()), (1, "9.9.9", Some("1,2")));
    let b = device();
    assert_eq!(model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap().added, 1);
    assert_eq!(stored(&b.storage, EDITS_KEY)["streets"]["w1~n2~n3"]["from_a_newer_build"], json!({"x": [1, 2]}), "a record travels as it is");
    // this build's own export of that device writes neither reserved field
    let out: Value = serde_json::from_str(&archive_of(&b)).unwrap();
    assert!(out.get("notes").is_none() && out.get("scenarios").is_none() && out.get("future_top_level").is_none());
}

#[test]
fn a_minimal_version_1_archive_is_read() {
    let archive = model::parse_and_validate(r#"{"kind":"cityloom-design","format_version":1}"#).unwrap();
    assert!(archive.is_empty());
    let b = device();
    let done = model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap();
    assert_eq!(done, Applied::default());
    assert!(b.storage.keys().is_empty());
    let vm = ArchiveVm::new(b.ports.clone(), crate::i18n_for(Locale::En));
    vm.import_text(r#"{"kind":"cityloom-design","format_version":1}"#);
    assert_eq!(vm.result_text().unwrap(), "Nothing to import: everything in this archive is already on this device.");
}

#[test]
fn edits_of_every_version_this_build_reads_are_accepted_and_written_at_the_version_their_content_needs() {
    // (content of one street record, the version a document holding it is written as)
    let plain = json!({"street": {}});
    let cases = [
        (4, plain.clone(), 5),
        (5, plain.clone(), 5),
        (6, json!({"street": {"zones": [{"from_mm": 0}]}}), 6),
        (7, json!({"street": {"speed_kmh": null}}), 7),
        (8, json!({"street": {"curb": [{"side": "right"}]}}), 8),
        (9, json!({"street": {"stops": [{"side": "right"}]}}), 9),
        (NEWEST.into(), plain.clone(), 5),
    ];
    assert_eq!(NEWEST, 10, "a new edits version needs a row here");
    for (version, record, written) in cases {
        let text = raw(edits_with(version, json!({"w1~n2~n3": record})), Value::Null);
        let archive = model::parse_and_validate(&text).unwrap_or_else(|e| panic!("version {version}: {e:?}"));
        let b = device();
        assert_eq!(model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap().added, 1, "version {version}");
        assert_eq!(stored(&b.storage, EDITS_KEY)["version"], written, "version {version}");
    }
    // a signal plan needs the newest
    let signal = json!({"state": {"signal_plan": {"phases": []}}});
    let doc = json!({"version": 5, "streets": {}, "junctions": {"n7": signal}});
    let b = device();
    let archive = model::parse_and_validate(&raw(doc, Value::Null)).unwrap();
    assert_eq!(model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap().added, 1);
    assert_eq!(stored(&b.storage, EDITS_KEY)["version"], NEWEST);
    for refused in [3, NEWEST + 1] {
        assert!(model::parse_and_validate(&raw(edits_with(refused.into(), json!({})), Value::Null)).is_err(), "version {refused}");
    }
}

#[test]
fn new_records_join_a_document_that_holds_others_and_the_others_are_not_touched() {
    let b = device();
    b.storage.remember(EDITS_KEY, &edits_with(5, json!({"w9~n8~n7": {"street": {}, "theirs": 1}})).to_string());
    let archive = model::parse_and_validate(&raw(edits_with(5, json!({"w1~n2~n3": {"street": {}}})), Value::Null)).unwrap();
    model::apply(&archive, &b.ports.storage, Choice::KeepMine).unwrap();
    let doc = stored(&b.storage, EDITS_KEY);
    assert_eq!(doc["streets"]["w9~n8~n7"], json!({"street": {}, "theirs": 1}));
    assert!(doc["streets"]["w1~n2~n3"].is_object());
    assert!(matches!(edits::parse(b.storage.recall(EDITS_KEY).as_deref()), edits::Document::Current(_)));
}

// ---- the commands ----

#[test]
fn export_offers_a_file_and_says_so_and_with_no_work_makes_none() {
    let t = device();
    let vm = ArchiveVm::new(t.ports.clone(), crate::i18n_for(Locale::En));
    vm.export();
    assert_eq!(t.files.take_saved(), vec![]);
    assert_eq!(vm.result_text().unwrap(), "You have no design work to export yet, so no file was made.");
    design(&t.ports.storage);
    t.announcer.take();
    vm.export();
    let saved = t.files.take_saved();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].0, "cityloom-2001-09-09.cityloom-design.json");
    assert!(model::parse_and_validate(&saved[0].1).is_ok());
    assert_eq!(vm.result_text().unwrap(), "Saved cityloom-2001-09-09.cityloom-design.json with 3 records of your design work.");
    assert_eq!(t.announcer.take(), vec![vm.result_text().unwrap()]);
    t.files.refuse_saves(true);
    vm.export();
    assert!(vm.result_text().unwrap().starts_with("The file could not be offered"));
}

#[test]
fn export_refuses_work_it_cannot_read_rather_than_make_an_archive_that_lacks_it() {
    let t = device();
    t.storage.remember(EDITS_KEY, "damaged");
    let vm = ArchiveVm::new(t.ports.clone(), crate::i18n_for(Locale::En));
    vm.export();
    assert_eq!(t.files.take_saved(), vec![]);
    assert!(vm.result_is_error());
    assert_eq!(t.storage.recall(EDITS_KEY).as_deref(), Some("damaged"));
}

#[test]
fn import_reads_the_file_the_person_picked_and_a_cancelled_chooser_does_nothing() {
    let a = device();
    design(&a.ports.storage);
    let text = archive_of(&a);
    let b = device();
    let vm = ArchiveVm::new(b.ports.clone(), crate::i18n_for(Locale::En));
    vm.import();
    assert_eq!((vm.result_text(), b.storage.keys().len()), (None, 0), "no file chosen: nothing happens");
    b.files.stage("mine.cityloom-design.json", &text);
    vm.import();
    assert_eq!(vm.result_text().unwrap(), "Import finished. Added: 3. Replaced: 0. Kept as they were here: 0. Already identical: 0.");
    assert_eq!(model::plan_import(&model::parse_and_validate(&text).unwrap(), &b.ports.storage).unwrap().count(Status::Same), 3);
}

#[test]
fn the_result_follows_a_language_switch() {
    let a = device();
    let i18n = crate::i18n_for(Locale::En);
    let vm = ArchiveVm::new(a.ports.clone(), i18n.clone());
    vm.import_text("nonsense");
    let english = vm.result_text().unwrap();
    i18n.set(Locale::FrCa);
    assert_ne!(vm.result_text().unwrap(), english);
    assert!(vm.result_text().unwrap().contains("Rien n’a été modifié"));
    assert!(matches!(Outcome::Cancelled, Outcome::Cancelled));
}

// ---- the section and its words ----

fn section(vm: &Rc<ArchiveVm>) -> String {
    let vm = vm.clone();
    html(move || view_of(vm))
}

fn view_of(vm: Rc<ArchiveVm>) -> leptos::prelude::AnyView {
    use leptos::prelude::*;
    view! { <ArchiveView vm=vm/> }.into_any()
}

#[test]
fn the_section_has_labelled_buttons_and_shows_the_question_and_the_result() {
    let (b, text, _) = conflict();
    let vm = ArchiveVm::new(b.ports.clone(), crate::i18n_for(Locale::En));
    let idle = section(&vm);
    for id in ["archive-export", "archive-import"] {
        assert!(idle.contains(&format!("id=\"{id}\"")), "{id}");
    }
    assert!(idle.contains("aria-labelledby=\"h-archive\"") && idle.contains("Design archive") && idle.contains("Export design"));
    assert!(idle.contains("aria-describedby=\"archive-import-help\"") && idle.contains("id=\"archive-import-help\""));
    assert!(!idle.contains("archive-confirm"));
    vm.import_text(&text);
    let asking = section(&vm);
    for needle in
        ["role=\"group\"", "id=\"archive-keep\"", "id=\"archive-use\"", "id=\"archive-cancel\"", "aria-describedby=\"archive-confirm-summary\"", "Keep mine"]
    {
        assert!(asking.contains(needle), "{needle}");
    }
    vm.choose(Choice::UseArchive);
    let done = section(&vm);
    assert!(done.contains("id=\"archive-result\"") && done.contains("tabindex=\"-1\"") && done.contains("Import finished"));
    assert!(!done.contains("archive-confirm"));
    vm.import_text("nope");
    assert!(section(&vm).contains("archive-error"));
}

#[test]
fn the_ftl_files_have_the_same_messages_and_variables() {
    assert_eq!(parity_problems(RESOURCES.en, RESOURCES.fr), Vec::<String>::new());
}

#[test]
fn the_french_section_is_in_french() {
    let t = device();
    let vm = ArchiveVm::new(t.ports.clone(), crate::i18n_for(Locale::FrCa));
    let page = section(&vm);
    assert!(page.contains("Archive de conception") && page.contains("Exporter la conception"));
}
