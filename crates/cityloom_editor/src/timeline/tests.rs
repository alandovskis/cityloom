use std::rc::Rc;

use serde_json::{Value, json};

use super::store::MilestoneStore;
use super::vm::{Context, Focus, TimelineVm};
use crate::city::edits::{EDITS_KEY, IMPORT_STAMP_VERSION, STALE_BEFORE_IMPORT};
use crate::restriction::store::RESTRICTIONS_KEY;
use crate::shared::i18n::Locale;
use crate::shared::ports::{AccountCall, AccountError, AccountReply, FakeAccount, Me, MemoryStorage, RecordingAnnouncer, Storage, test_ports, test_ports_full};

fn page(locale: Locale) -> (Rc<TimelineVm>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
    let (ports, announcer, storage) = test_ports();
    let context = Context { project_name: "Plateau".into(), app_version: "0.70.0".into() };
    (TimelineVm::new(ports, crate::i18n_for(locale), context), announcer, storage)
}

fn en() -> (Rc<TimelineVm>, Rc<RecordingAnnouncer>, Rc<MemoryStorage>) {
    page(Locale::En)
}

/// Keeps streets `(key, name, width)` as the project's edits.
fn keep(storage: &MemoryStorage, streets: &[(&str, &str, i64)]) {
    let streets: serde_json::Map<String, serde_json::Value> = streets
        .iter()
        .map(|(k, name, w)| ((*k).to_string(), json!({ "ways": [1], "import": STALE_BEFORE_IMPORT, "street": { "name": name, "row_mm": w } })))
        .collect();
    assert!(storage.remember(EDITS_KEY, &json!({ "version": IMPORT_STAMP_VERSION, "streets": streets }).to_string()));
}

#[test]
fn the_heading_names_the_project_and_nothing_is_listed_before_a_milestone_is_saved() {
    let (vm, _, _) = en();
    assert_eq!(vm.heading(), "Timeline of Plateau");
    assert!(vm.rows().is_empty());
}

#[test]
fn a_milestone_is_saved_from_the_work_now_and_listed_with_who_when_and_what() {
    let (vm, said, storage) = en();
    storage.set_now(1_794_495_840_000); // 2026-11-12T15:04:00Z
    keep(&storage, &[("w1", "Rue A", 9000), ("w2", "Rue B", 9000)]);
    vm.set_author(Some("Ada".into()));
    assert!(vm.create("  First design "));
    let rows = vm.rows();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.name, "First design");
    assert_eq!(row.summary, "2 streets, 0 junctions, 0 rules");
    assert_eq!(row.author, "By Ada");
    assert_eq!(row.version, "CityLoom 0.70.0");
    assert!(row.when.ends_with(" UTC") && row.when.contains("2026"), "{}", row.when);
    assert!(!row.auto);
    assert_eq!(said.take(), ["Milestone First design saved."]);
}

#[test]
fn milestones_are_listed_in_the_order_they_were_made() {
    let (vm, _, storage) = en();
    keep(&storage, &[("w1", "A", 1)]);
    storage.set_now(1_000);
    vm.create("One");
    storage.set_now(2_000);
    vm.create("Two");
    assert_eq!(vm.rows().iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["One", "Two"]);
}

#[test]
fn a_bad_name_is_refused_beside_the_field_and_said_and_nothing_is_kept() {
    let (vm, said, _) = en();
    assert!(!vm.create("  "));
    assert_eq!(vm.create_error().as_deref(), Some("Give the milestone a name."));
    assert_eq!(said.take(), ["Give the milestone a name."]);
    assert_eq!(vm.focus_request().1, Focus::CreateField);
    assert!(vm.rows().is_empty());
    assert!(!vm.create(&"x".repeat(81)));
    assert_eq!(vm.create_error().as_deref(), Some("A milestone name has at most 80 characters."));
    assert!(vm.create("Fine"));
    assert_eq!(vm.create_error(), None);
}

#[test]
fn milestones_stop_where_they_would_crowd_the_device_and_a_restore_that_cannot_keep_its_backup_is_refused() {
    let (vm, said, storage) = en();
    let big = "y".repeat(700_000);
    keep(&storage, &[("w1", &big, 9000)]);
    for name in ["A", "B", "C", "D"] {
        assert!(vm.create(name), "{name}");
    }
    assert!(!vm.create("E"));
    let full = "The milestones of this project take all the room they may on this device. Delete one to make room.";
    assert_eq!(vm.create_error().as_deref(), Some(full));
    keep(&storage, &[("w1", &big, 12000)]);
    let work = storage.recall(EDITS_KEY);
    let id = vm.rows()[0].id.clone();
    said.take();
    vm.ask_restore(&id);
    assert!(!vm.confirm_restore());
    assert_eq!(said.take(), [full]);
    assert_eq!(storage.recall(EDITS_KEY), work, "the work stands");
    assert_eq!(vm.rows().len(), 4);
    // Deleting one makes room for the backup.
    vm.ask_delete(&vm.rows()[1].id.clone());
    vm.confirm_delete();
    vm.ask_restore(&id);
    assert!(vm.confirm_restore());
}

#[test]
fn work_that_cannot_be_read_is_refused() {
    let (vm, _, storage) = en();
    storage.remember(EDITS_KEY, "garbage");
    assert!(!vm.create("Never"));
    assert_eq!(vm.create_error().as_deref(), Some("The milestones or the work on this device could not be read or changed."));
}

#[test]
fn comparing_lists_the_places_that_differ_with_names_and_what_changed() {
    let (vm, said, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000), ("w2", "Rue B", 9000), ("w3", "Rue C", 9000)]);
    vm.create("Base");
    let id = vm.rows()[0].id.clone();
    keep(&storage, &[("w1", "Rue A", 9000), ("w2", "Rue B", 12000), ("w4", "Rue D", 9000)]);
    said.take();
    assert!(vm.compare(&id));
    assert_eq!(said.take(), ["3 places differ from Base."], "opening the comparison is said: focus stays on the button");
    let c = vm.comparison().unwrap();
    assert_eq!(c.heading, "Since Base");
    assert_eq!(c.summary, "3 places differ from Base.");
    let seen: Vec<(String, String, Vec<String>)> = c.rows.iter().map(|r| (r.label.clone(), r.how.clone(), r.aspects.clone())).collect();
    assert_eq!(
        seen,
        [
            ("Rue B".to_string(), "Changed".to_string(), vec!["width".to_string()]),
            ("Rue C".to_string(), "Removed since".to_string(), vec![]),
            ("Rue D".to_string(), "Added since".to_string(), vec![]),
        ]
    );
    vm.close_compare();
    assert!(vm.comparison().is_none());
    assert_eq!(vm.focus_request().1, Focus::CompareButton(id));
}

#[test]
fn comparing_the_same_work_says_so_and_an_unknown_milestone_is_refused() {
    let (vm, _, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    vm.create("Base");
    let id = vm.rows()[0].id.clone();
    assert!(vm.compare(&id));
    let c = vm.comparison().unwrap();
    assert!(c.rows.is_empty());
    assert_eq!(c.summary, "The work is the same as in Base.");
    assert!(!vm.compare("nope"));
}

#[test]
fn restoring_is_asked_first_and_puts_the_work_back_after_saving_what_it_replaces() {
    let (vm, said, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    vm.create("Base");
    let id = vm.rows()[0].id.clone();
    let before = storage.recall(EDITS_KEY);
    keep(&storage, &[("w1", "Rue A", 12000), ("w2", "Rue B", 9000)]);
    let edited = storage.recall(EDITS_KEY);
    said.take();
    vm.ask_restore(&id);
    assert_eq!(
        vm.restore_question().as_deref(),
        Some("Put the work back as it was in Base? The work as it is now is saved first as a milestone, so you can come back to it.")
    );
    assert_eq!(vm.focus_request().1, Focus::RestoreSafe);
    assert_eq!(storage.recall(EDITS_KEY), edited, "nothing changed by asking");
    assert!(vm.confirm_restore());
    assert!(vm.restore_question().is_none());
    assert_eq!(storage.recall(EDITS_KEY), before);
    let rows = vm.rows();
    assert_eq!(rows.len(), 2);
    let backup = rows.iter().find(|r| r.auto).unwrap();
    assert_eq!(backup.name, "Before restoring Base");
    assert_eq!(said.take(), ["Restored Base. The earlier work is saved as Before restoring Base."]);
    // restoring the backup brings the edits back
    vm.ask_restore(&backup.id.clone());
    assert!(vm.confirm_restore());
    assert_eq!(storage.recall(EDITS_KEY), edited);
}

#[test]
fn cancelling_a_restore_changes_nothing_and_returns_focus() {
    let (vm, _, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    vm.create("Base");
    let id = vm.rows()[0].id.clone();
    keep(&storage, &[("w1", "Rue A", 1)]);
    let edited = storage.recall(EDITS_KEY);
    vm.ask_restore(&id);
    vm.cancel_restore();
    assert!(vm.restore_question().is_none());
    assert_eq!(vm.focus_request().1, Focus::RestoreButton(id));
    assert!(!vm.confirm_restore(), "nothing is being asked");
    assert_eq!(storage.recall(EDITS_KEY), edited);
    assert_eq!(vm.rows().len(), 1);
}

#[test]
fn restoring_work_that_is_already_the_same_says_so_and_saves_no_backup() {
    let (vm, said, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    vm.create("Base");
    let id = vm.rows()[0].id.clone();
    said.take();
    vm.ask_restore(&id);
    assert!(vm.confirm_restore());
    assert_eq!(vm.rows().len(), 1);
    assert_eq!(said.take(), ["The work already is as it was in Base."]);
}

#[test]
fn deleting_is_asked_first_and_leaves_the_work_alone() {
    let (vm, said, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    vm.create("Base");
    let id = vm.rows()[0].id.clone();
    let work = storage.recall(EDITS_KEY);
    said.take();
    vm.ask_delete(&id);
    assert_eq!(vm.delete_question().as_deref(), Some("Delete the milestone Base? The work itself is not changed."));
    vm.cancel_delete();
    assert_eq!(vm.rows().len(), 1);
    assert_eq!(vm.focus_request().1, Focus::DeleteButton(id.clone()));
    vm.ask_delete(&id);
    assert!(vm.confirm_delete());
    assert!(vm.rows().is_empty());
    assert_eq!(storage.recall(EDITS_KEY), work);
    assert_eq!(said.take(), ["Milestone Base deleted."]);
    assert_eq!(vm.focus_request().1, Focus::Heading);
    assert!(!vm.confirm_delete());
}

#[test]
fn asking_about_a_milestone_that_is_gone_does_nothing() {
    let (vm, _, _) = en();
    vm.ask_restore("nope");
    assert!(vm.restore_question().is_none());
    vm.ask_delete("nope");
    assert!(vm.delete_question().is_none());
}

#[test]
fn a_milestone_made_before_a_restore_is_marked_as_the_apps() {
    let (vm, _, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    vm.create("Base");
    keep(&storage, &[("w1", "Rue A", 1)]);
    let id = vm.rows()[0].id.clone();
    vm.ask_restore(&id);
    vm.confirm_restore();
    let rows = vm.rows();
    assert_eq!(rows.iter().filter(|r| r.auto).count(), 1);
    assert_eq!(vm.auto_label(), "Saved before a restore");
}

#[test]
fn the_words_follow_the_language() {
    let (vm, _, storage) = page(Locale::FrCa);
    storage.set_now(1_794_495_840_000);
    keep(&storage, &[("w1", "Rue A", 9000)]);
    assert_eq!(vm.heading(), "Chronologie de Plateau");
    vm.create("Base");
    let row = &vm.rows()[0];
    assert_eq!(row.summary, "1 rue, 0 jonction, 0 règle");
    assert!(row.when.contains("novembre 2026"), "{}", row.when);
}

mod view {
    use super::*;
    use crate::shared::testing::{button_tag, count, html};
    use crate::timeline::view::TimelineView;
    use leptos::prelude::*;

    fn draw(vm: &Rc<TimelineVm>) -> String {
        let vm = vm.clone();
        html(move || view! { <TimelineView vm=vm/> }.into_any())
    }

    #[test]
    fn an_empty_timeline_has_a_heading_the_form_and_a_word_about_having_none() {
        let (vm, _, _) = en();
        let page = draw(&vm);
        assert!(page.contains("Timeline of Plateau"), "{page}");
        assert!(page.contains("No milestones yet"), "{page}");
        assert_eq!(count(&page, "<form"), 1);
        assert!(page.contains("class=\"timeline-back\"") && page.contains("Back to the map"), "{page}");
        assert!(page.contains("aria-invalid=\"false\""), "{page}");
        assert!(page.contains("id=\"timeline-heading\"") && page.contains("tabindex=\"-1\""), "{page}");
    }

    #[test]
    fn each_milestone_is_an_item_named_by_its_name_and_its_details_with_buttons_named_after_it() {
        let (vm, _, storage) = en();
        keep(&storage, &[("w1", "Rue A", 9000)]);
        vm.set_author(Some("Ada".into()));
        vm.create("Base");
        let id = vm.rows()[0].id.clone();
        let page = draw(&vm);
        assert_eq!(count(&page, "class=\"timeline-row\""), 1, "{page}");
        assert!(page.contains(&format!("aria-labelledby=\"timeline-name-{id} timeline-meta-{id}\"")), "{page}");
        assert!(page.contains("By Ada") && page.contains("CityLoom 0.70.0") && page.contains("1 street"), "{page}");
        assert!(button_tag(&page, &format!("timeline-restore-{id}")).contains("aria-label=\"Restore Base\""));
        assert!(button_tag(&page, &format!("timeline-delete-{id}")).contains("aria-label=\"Delete Base\""));
        assert!(button_tag(&page, &format!("timeline-compare-{id}")).contains("aria-label=\"Compare with now: Base\""));
    }

    #[test]
    fn a_question_is_a_group_with_the_safe_button_first_and_a_comparison_is_a_named_section() {
        let (vm, _, storage) = en();
        keep(&storage, &[("w1", "Rue A", 9000)]);
        vm.create("Base");
        let id = vm.rows()[0].id.clone();
        keep(&storage, &[("w1", "Rue A", 12000)]);
        vm.ask_restore(&id);
        let page = draw(&vm);
        assert!(page.contains("role=\"group\"") && page.contains("id=\"timeline-restore-safe\""), "{page}");
        assert!(page.find("timeline-restore-safe").unwrap() < page.find("timeline-restore-yes").unwrap());
        vm.cancel_restore();
        vm.compare(&id);
        let page = draw(&vm);
        assert!(page.contains("class=\"timeline-compare\"") && page.contains("Since Base") && page.contains("1 place differs from Base."), "{page}");
        assert!(page.contains("width"), "{page}");
    }
}

#[test]
fn the_link_back_to_the_map_keeps_the_project_and_a_read_only_view() {
    use crate::shared::ports::Ports;
    use crate::shared::scoped::ScopedStorage;
    let (ports, _, storage) = test_ports();
    let context = || Context { project_name: "Plateau".into(), app_version: "0.70.0".into() };
    let own: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), "p1"));
    let vm = TimelineVm::new(Ports { storage: own, ..ports.clone() }, crate::i18n_for(Locale::En), context());
    assert_eq!(vm.back_href(), "map.html?project=p1");
    let look: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage, "p1").only_to_read());
    let vm = TimelineVm::new(Ports { storage: look, ..ports }, crate::i18n_for(Locale::En), context());
    assert_eq!(vm.back_href(), "map.html?project=p1&view=1", "a timeline shown read-only leads back to its map read-only");
}

#[test]
fn a_stale_milestone_has_its_own_refusal_in_both_languages() {
    assert_eq!(super::text::refused_key(super::model::Refused::Stale), "timeline-refused-stale");
    for locale in [Locale::En, Locale::FrCa] {
        assert!(!crate::i18n_for(locale).tr_now("timeline-refused-stale", &Default::default()).is_empty());
    }
}

/// A timeline page whose account can be answered.
fn signed_page() -> (Rc<TimelineVm>, Rc<FakeAccount>, Rc<MemoryStorage>) {
    let t = test_ports_full();
    let context = Context { project_name: "Plateau".into(), app_version: "0.70.0".into() };
    (TimelineVm::new(t.ports, crate::i18n_for(Locale::En), context), t.account, t.storage)
}

fn me(display_name: &str) -> AccountReply {
    AccountReply::Me(Ok(Me { id: "u1".into(), display_name: display_name.into(), store_on_server: None, identities: vec![] }))
}

/// A street's record holding `street`.
fn street(street: Value) -> Value {
    json!({ "ways": [1], "import": STALE_BEFORE_IMPORT, "street": street })
}

/// Keeps `streets` and `junctions` (records by key) as the project's edits.
fn keep_work(storage: &MemoryStorage, streets: Value, junctions: Value) {
    assert!(storage.remember(EDITS_KEY, &json!({ "version": IMPORT_STAMP_VERSION, "streets": streets, "junctions": junctions }).to_string()));
}

/// A rule's record named `name`.
fn rule(name: Value) -> Value {
    json!({ "name": name, "rule": { "kind": "car_free" }, "ring": [[-73.6, 45.52], [-73.59, 45.52], [-73.59, 45.53]] })
}

/// Keeps `rules` (records by id) as the project's rules.
fn keep_rules(storage: &MemoryStorage, rules: Value) {
    assert!(storage.remember(RESTRICTIONS_KEY, &json!({ "version": 1, "next_id": 9, "restrictions": rules }).to_string()));
}

#[test]
fn the_author_is_whoever_is_signed_in_asked_once() {
    let (vm, account, storage) = signed_page();
    vm.load_author();
    assert_eq!(account.calls(), [AccountCall::Me]);
    assert!(account.answer(me("Ada")));
    keep(&storage, &[("w1", "Rue A", 9000)]);
    assert!(vm.create("Base"));
    assert_eq!(vm.rows()[0].author, "By Ada");
}

#[test]
fn a_blank_name_or_nobody_signed_in_leaves_the_author_empty() {
    let (vm, account, storage) = signed_page();
    vm.load_author();
    assert!(account.answer(me("  ")));
    keep(&storage, &[("w1", "Rue A", 9000)]);
    assert!(vm.create("A"));
    assert_eq!(vm.rows()[0].author, "");

    let (vm, account, storage) = signed_page();
    vm.load_author();
    assert!(account.answer(AccountReply::Me(Err(AccountError::SignedOut))));
    keep(&storage, &[("w1", "Rue A", 9000)]);
    assert!(vm.create("B"));
    assert_eq!(vm.rows()[0].author, "");
}

#[test]
fn a_comparison_names_streets_junctions_and_rules_from_their_records() {
    let (vm, _, storage) = en();
    keep_rules(&storage, json!({ "r1": rule(json!("Old Zone")) }));
    assert!(vm.create("Base"));
    let id = vm.rows()[0].id.clone();
    keep_work(
        &storage,
        json!({ "w1": street(json!({ "row_mm": 9000 })), "w2": street(json!({ "name": "  ", "row_mm": 9000 })) }),
        json!({ "j1": { "nodes": [30, 12, 25], "state": {} }, "j2": { "state": {} } }),
    );
    keep_rules(&storage, json!({ "r2": rule(json!("Quiet")), "r3": rule(json!("  ")) }));
    assert!(vm.compare(&id));
    let c = vm.comparison().unwrap();
    let labels: Vec<&str> = c.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, ["Unnamed street", "Unnamed street", "Junction at node 12", "Junction", "Rule Old Zone", "Rule Quiet", "Area rule"]);
    assert_eq!(c.rows[4].how, "Removed since", "the rule gone is named from the milestone's record");
}

#[test]
fn a_changed_place_says_each_aspect_that_changed_in_words() {
    let (vm, _, storage) = en();
    keep_work(
        &storage,
        json!({ "w1": street(json!({ "name": "Rue A", "class": "local", "row_mm": 9000, "segments": [1], "speed_kmh": 30, "zones": [1] })) }),
        json!({ "j1": { "nodes": [5], "state": { "arms": [1, 2], "control": 0 } } }),
    );
    keep_rules(&storage, json!({ "r1": rule(json!("Zone")) }));
    assert!(vm.create("Base"));
    let id = vm.rows()[0].id.clone();
    keep_work(
        &storage,
        json!({ "w1": street(json!({ "name": "Rue B", "class": "arterial", "row_mm": 9000, "segments": [1, 2], "speed_kmh": 40, "zones": [2], "note": 1 })) }),
        json!({ "j1": { "nodes": [5], "state": { "arms": [1, 2, 3], "control": 2 } } }),
    );
    keep_rules(&storage, json!({ "r1": { "name": "Zone B", "rule": { "kind": "speed_limit" }, "ring": [[-73.6, 45.52], [-73.58, 45.52], [-73.59, 45.53]] } }));
    assert!(vm.compare(&id));
    let c = vm.comparison().unwrap();
    let seen: Vec<(&str, &str, Vec<&str>)> =
        c.rows.iter().map(|r| (r.label.as_str(), r.how.as_str(), r.aspects.iter().map(String::as_str).collect())).collect();
    assert_eq!(
        seen,
        [
            ("Rue B", "Changed", vec!["name", "street class", "lanes", "speed limit", "zones", "other details"]),
            ("Junction at node 5", "Changed", vec!["arms", "control"]),
            ("Rule Zone B", "Changed", vec!["name", "rule", "outline"]),
        ]
    );
}

#[test]
fn a_milestone_of_no_work_says_nothing_was_edited() {
    let (vm, _, _) = en();
    assert!(vm.create("Empty"));
    assert_eq!(vm.rows()[0].summary, "No streets, junctions or rules edited");
}

#[test]
fn work_that_cannot_be_read_now_cannot_be_compared() {
    let (vm, said, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    assert!(vm.create("Base"));
    let id = vm.rows()[0].id.clone();
    storage.remember(EDITS_KEY, "garbage");
    said.take();
    assert!(vm.compare(&id));
    let unusable = "The work on this device cannot be read, so it cannot be compared.";
    assert_eq!(said.take(), [unusable]);
    let c = vm.comparison().unwrap();
    assert_eq!(c.heading, "Since Base");
    assert_eq!(c.summary, unusable);
    assert!(c.rows.is_empty());
}

#[test]
fn restoring_a_milestone_deleted_meanwhile_says_it_is_gone_and_changes_nothing() {
    let (vm, said, storage) = en();
    keep(&storage, &[("w1", "Rue A", 9000)]);
    assert!(vm.create("Base"));
    let id = vm.rows()[0].id.clone();
    keep(&storage, &[("w1", "Rue A", 12000)]);
    let edited = storage.recall(EDITS_KEY);
    vm.ask_restore(&id);
    let elsewhere: Rc<dyn Storage> = storage.clone();
    MilestoneStore::new(elsewhere).delete(&id).unwrap();
    said.take();
    assert!(!vm.confirm_restore());
    assert_eq!(said.take(), ["That milestone is no longer here."]);
    assert_eq!(storage.recall(EDITS_KEY), edited, "the work stands");
    assert!(vm.rows().is_empty(), "no backup was made");
}
