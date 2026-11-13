use std::cell::Cell;
use std::rc::Rc;

use super::model::{self, ACCESS_LANE_MM, Field, Refusal};
use super::view::Profiles;
use super::vm::{Form, StandardsVm};
use crate::shared::i18n::Locale;
use crate::shared::ports::{MemoryStorage, RecordingAnnouncer, test_ports};
use crate::shared::testing::{count, html};
use crate::shared::units::Units;

struct T {
    vm: Rc<StandardsVm>,
    said: Rc<RecordingAnnouncer>,
    storage: Rc<MemoryStorage>,
    changes: Rc<Cell<u32>>,
    _restore: model::Restore,
}

fn setup() -> T {
    let (ports, said, storage) = test_ports();
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    let vm = StandardsVm::new(ports, crate::i18n_for(Locale::En), Rc::new(|| Units::Metres), Rc::new(move || seen.set(seen.get() + 1)));
    T { vm, said, storage, changes, _restore: model::activate_for_test("", model::Standards::default()) }
}

fn form(vm: &StandardsVm, id: &str) -> Form {
    let f = vm.form().unwrap_or_else(|| panic!("no form for {id}"));
    Form {
        name: f.name,
        lengths: f.lengths.iter().map(|l| l.value.clone()).collect(),
        speeds: f.speeds,
        targets: f.targets.iter().map(|t| t.value.clone()).collect(),
    }
}

#[test]
fn creating_a_profile_lists_it_opens_its_form_and_says_so() {
    let t = setup();
    assert!(t.vm.rows().is_empty());
    assert!(t.vm.create("City of X"));
    let rows = t.vm.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "City of X");
    assert!(!rows[0].in_force);
    assert_eq!(t.vm.editing().as_deref(), Some(rows[0].id.as_str()));
    assert_eq!(t.said.take(), vec!["Created the profile \"City of X\".".to_string()]);
    assert_eq!(t.changes.get(), 1);
}

#[test]
fn a_refusal_says_why_shows_it_and_keeps_nothing() {
    let t = setup();
    assert!(!t.vm.create("   "));
    assert_eq!(t.vm.error().as_deref(), Some("Give the profile a name."));
    assert_eq!(t.said.take(), vec!["Give the profile a name.".to_string()]);
    assert!(t.vm.rows().is_empty());
    assert_eq!(t.changes.get(), 0);
    t.vm.create("A");
    assert_eq!(t.vm.error(), None);
    assert!(!t.vm.create("a"));
    assert_eq!(t.vm.error().as_deref(), Some("Another profile already has that name."));
}

#[test]
fn choosing_a_profile_puts_it_in_force_and_names_it_where_the_page_asks() {
    let t = setup();
    t.vm.create("City of X");
    let id = t.vm.rows()[0].id.clone();
    assert_eq!(t.vm.in_force_text(), "The checks use the built-in standards.");
    assert!(t.vm.select(Some(&id)));
    assert_eq!(t.vm.in_force().as_deref(), Some(id.as_str()));
    assert_eq!(t.vm.in_force_text(), "The checks use the standard \"City of X\".");
    assert_eq!(model::active_name().as_deref(), Some("City of X"));
    assert!(t.vm.select(None));
    assert_eq!(model::active_name(), None);
    assert_eq!(t.vm.in_force_text(), "The checks use the built-in standards.");
}

#[test]
fn saving_reads_the_figures_in_the_pages_units_and_refuses_a_bad_one_with_its_range() {
    let t = setup();
    t.vm.create("X");
    let id = t.vm.rows()[0].id.clone();
    let mut f = form(&t.vm, &id);
    assert_eq!(f.lengths[0], "3.00");
    f.lengths[0] = "3,5".into();
    f.speeds = "20, 40 ,30".into();
    f.targets[3] = "30".into();
    assert!(t.vm.save(&id, &f));
    let saved = t.vm.form().unwrap();
    assert_eq!(saved.lengths[0].value, "3.50");
    assert_eq!(saved.speeds, "20, 30, 40");
    assert_eq!(saved.targets[3].value, "30");
    // not a number
    f.lengths[1] = "far".into();
    assert!(!t.vm.save(&id, &f));
    assert_eq!(t.vm.error().as_deref(), Some("That is not a number."));
    // out of range, with the range
    f.lengths[1] = "5".into();
    f.lengths[0] = "0.5".into();
    assert!(!t.vm.save(&id, &f));
    assert_eq!(t.vm.error().as_deref(), Some("Emergency access lane width must be between 2.0 m and 6.0 m."));
    assert!(!t.vm.save(&id, &Form { speeds: "".into(), ..form(&t.vm, &id) }));
    assert_eq!(t.vm.error().as_deref(), Some("List at least one speed limit."));
    // what was kept before stays
    assert_eq!(t.vm.form().unwrap().lengths[0].value, "3.50");
}

#[test]
fn editing_the_profile_in_force_changes_the_figures_the_checks_read() {
    let t = setup();
    t.vm.create("X");
    let id = t.vm.rows()[0].id.clone();
    t.vm.select(Some(&id));
    let mut f = form(&t.vm, &id);
    f.lengths[0] = "4".into();
    t.vm.save(&id, &f);
    assert_eq!(model::access_lane_mm(), 4000);
    assert_ne!(model::access_lane_mm(), ACCESS_LANE_MM);
}

#[test]
fn a_looser_figure_is_kept_and_the_form_warns_of_it() {
    let t = setup();
    t.vm.create("X");
    let id = t.vm.rows()[0].id.clone();
    let mut f = form(&t.vm, &id);
    assert!(t.vm.form().unwrap().warnings.is_empty());
    f.lengths[0] = "2.5".into();
    assert!(t.vm.save(&id, &f));
    let warnings = t.vm.form().unwrap().warnings;
    assert_eq!(warnings.len(), 1);
    assert_eq!(crate::shared::testing::en(&warnings[0]), "Emergency access lane width: 2.5 m is looser than the built-in 3.0 m.");
}

#[test]
fn widths_are_set_validated_and_reset_for_a_kind() {
    let t = setup();
    t.vm.create("X");
    let id = t.vm.rows()[0].id.clone();
    assert!(t.vm.set_widths(&id, "travel", "2.8", "3.6"));
    let widths = t.vm.form().unwrap().widths;
    assert_eq!(widths.len(), 1);
    assert_eq!(
        crate::shared::testing::en(&widths[0].said),
        "Travel lane: 2.8 m to 3.6 m (built-in 2.7 m to 4.0 m)".replace("2.7 m to 4.0 m", &builtin_travel())
    );
    assert!(!t.vm.set_widths(&id, "travel", "3.6", "2.8"));
    assert_eq!(t.vm.error().as_deref(), Some("The narrowest width of a driving lane must not be over its widest."));
    assert!(!t.vm.set_widths(&id, "travel", "x", "2.8"));
    assert!(t.vm.clear_widths(&id, "travel"));
    assert!(t.vm.form().unwrap().widths.is_empty());
}

fn builtin_travel() -> String {
    let k = &crate::shared::catalogue::KINDS[crate::shared::catalogue::kind_index("travel").unwrap()];
    format!("{:.1} m to {:.1} m", k.min_mm as f64 / 1000.0, k.max_mm as f64 / 1000.0)
}

#[test]
fn removing_asks_first_and_puts_the_built_in_standards_back_if_it_was_in_force() {
    let t = setup();
    t.vm.create("X");
    let id = t.vm.rows()[0].id.clone();
    t.vm.select(Some(&id));
    t.vm.ask_remove(&id);
    assert_eq!(t.vm.removing().as_deref(), Some(id.as_str()));
    t.vm.cancel_remove();
    assert_eq!(t.vm.removing(), None);
    assert_eq!(t.vm.rows().len(), 1);
    t.vm.ask_remove(&id);
    assert!(t.vm.remove(&id));
    assert!(t.vm.rows().is_empty());
    assert_eq!(t.vm.editing(), None);
    assert_eq!(model::active_name(), None);
}

#[test]
fn an_export_is_shown_and_imports_into_another_browser_with_the_same_validation() {
    let t = setup();
    t.vm.create("City of X");
    let id = t.vm.rows()[0].id.clone();
    let mut f = form(&t.vm, &id);
    f.lengths[0] = "3.5".into();
    t.vm.save(&id, &f);
    assert!(t.vm.export(&id));
    let (name, text) = t.vm.exported().unwrap();
    assert_eq!(name, "City of X");
    assert!(text.contains("\"schema_version\": 1"));
    let other = setup();
    assert!(other.vm.import(&text));
    assert_eq!(other.vm.rows()[0].name, "City of X");
    assert!(!other.vm.import("{}"));
    assert_eq!(other.vm.error().as_deref(), Some("That text is not an exported CityLoom standards profile."));
    assert!(!other.vm.import(&text.replace("City of X", "City of Z").replace("\"access_lane_mm\": 3500", "\"access_lane_mm\": 100")));
    assert!(other.vm.error().unwrap().starts_with("Emergency access lane width must be between"));
}

#[test]
fn a_blocked_storage_is_said() {
    let t = setup();
    t.storage.blocked(true);
    assert!(!t.vm.create("X"));
    assert!(t.vm.error().unwrap().starts_with("The browser would not keep the profile"));
}

#[test]
fn the_section_draws_its_controls_and_follows_the_profiles() {
    let t = setup();
    let vm = t.vm.clone();
    let before = html(|| leptos::prelude::IntoAny::into_any(leptos::prelude::view! { <Profiles vm=vm.clone()/> }));
    assert!(before.contains("id=\"standards-in-force\"") && before.contains("id=\"standards-create\"") && before.contains("id=\"standards-import\""));
    assert!(before.contains("No profile yet.") && before.contains("The checks use the built-in standards."));
    assert_eq!(count(&before, "standards-edit-form"), 0);
    t.vm.create("City of X");
    let vm = t.vm.clone();
    let after = html(|| leptos::prelude::IntoAny::into_any(leptos::prelude::view! { <Profiles vm=vm.clone()/> }));
    assert!(after.contains("City of X") && after.contains("id=\"standards-edit-form\"") && after.contains("id=\"standards-save\""));
    assert!(after.contains("Emergency access lane width (m)") && after.contains("Built-in: 3.0 m"));
    assert!(!after.contains("No profile yet."));
}

#[test]
fn the_refusals_have_reasons_in_both_languages() {
    use crate::shared::said::Said;
    let refusals = [
        Refusal::NameEmpty,
        Refusal::NameTooLong { max: 60 },
        Refusal::NameTaken,
        Refusal::Length { field: Field::StopMin, min_mm: 3000, max_mm: 30000 },
        Refusal::Speed { min_kmh: 5, max_kmh: 130 },
        Refusal::SpeedsEmpty,
        Refusal::SpeedsMany { max: 24 },
        Refusal::SpeedsOrder,
        Refusal::UnknownKind("x".into()),
        Refusal::UnknownClass("x".into()),
        Refusal::WidthOrder { kind: "travel" },
        Refusal::WidthDefault { kind: "travel", default_mm: 3300 },
        Refusal::NotANumber,
        Refusal::Unknown,
        Refusal::TooMany { max: 20 },
        Refusal::Storage,
        Refusal::Newer,
        Refusal::ImportUnreadable,
        Refusal::ImportVersion { found: 2 },
    ];
    for r in refusals {
        let said: Said = r.said();
        for locale in Locale::ALL {
            let i18n = crate::i18n_for(locale);
            assert!(i18n.has_message(said.key, locale), "{} in {}", said.key, locale.tag());
        }
        assert!(!crate::shared::testing::en(&said).is_empty());
        assert!(!crate::shared::testing::fr(&said).contains("{"), "{:?}", said);
    }
}
