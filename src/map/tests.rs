//! The map page drawn to HTML on the host.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::testing::{button_tag, count, html};
// ---- the city map ---------------------------------------------------------------------------

use crate::city::store::CityStore;
use crate::map::view as map_ui;
use crate::map::vm::MapVm;
use crate::shared::i18n::{Args, I18n, Locale};
use crate::shared::ports::{MemoryStorage, test_ports};

fn map_vm() -> (Rc<MapVm>, Rc<MemoryStorage>) {
    map_vm_in(Locale::En)
}

fn map_vm_in(locale: Locale) -> (Rc<MapVm>, Rc<MemoryStorage>) {
    let (ports, _, storage) = test_ports();
    (MapVm::on_sample(ports, crate::i18n_for(locale)), storage)
}

fn map_html<V: IntoView + 'static>(f: impl FnOnce() -> V) -> String {
    html(|| f().into_any())
}

/// Another page changes a street and the map reads the city again.
fn change_a_street(vm: &MapVm, storage: &Rc<MemoryStorage>, by_mm: i32) {
    let street = vm.view().edges.iter().find(|e| !e.freeway).unwrap().uid;
    assert!(CityStore::sample(storage.clone()).write(|c| {
        let mut e = c.street_editor(street, 0).unwrap();
        let u = e.view().segments[0].uid;
        e.set_width(u, e.view().segments[0].width_mm + by_mm);
        c.keep_street(street, e.snapshot())
    }));
    vm.reload();
}

#[test]
fn the_map_heading_names_the_city_and_counts_its_places() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::MapHeader vm=vm.clone()/> });
    assert!(h.contains(">Sample city</h1>") && h.contains("City map") && h.contains("id=\"city-count\""));
    assert!(h.contains("9 junctions, 23 streets"));
}

#[test]
fn start_over_is_off_until_something_is_changed_and_then_says_what_it_does() {
    let (vm, storage) = map_vm();
    let h = map_html(|| view! { <map_ui::ResetButton vm=vm.clone()/> });
    assert!(button_tag(&h, "reset").contains("disabled") && h.contains(">Start over<"));
    change_a_street(&vm, &storage, -100);
    let h = map_html(|| view! { <map_ui::ResetButton vm=vm.clone()/> });
    assert!(!button_tag(&h, "reset").contains("disabled"));
    vm.press_reset();
    let h = map_html(|| view! { <map_ui::ResetButton vm=vm.clone()/> });
    assert!(h.contains(">Press again to start over<"));
}

#[test]
fn the_map_tools_zoom_in_out_and_to_the_whole_city() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::MapTools vm=vm.clone()/> });
    for id in ["zoom-out", "zoom-in", "zoom-fit"] {
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}");
    }
    assert!(h.contains("aria-label=\"Zoom out\"") && h.contains("Whole city") && h.contains("title=\"Whole city\""));
}

#[test]
fn a_map_page_with_nothing_to_show_says_why() {
    let (ports, ..) = test_ports();
    let vm = MapVm::new(ports, crate::i18n_for(Locale::En)); // the default area, whose roads are not kept: so no map
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert!(h.contains("role=\"status\"") && h.contains("The roads of this place could not be loaded"), "{h}");
    assert!(!h.contains("hidden"), "{h}");
}

#[test]
fn a_map_page_whose_area_s_roads_were_not_had_lists_nothing_and_names_the_area() {
    let (ports, _, _) = test_ports(); // the default area, whose roads are not kept
    let vm = MapVm::new(ports, crate::i18n_for(Locale::En));
    let area = crate::place::area::default_area().name;
    let h = map_html(|| view! { <map_ui::MapHeader vm=vm.clone()/> });
    assert!(h.contains(&format!(">{area}</h1>")) && h.contains("0 junctions, 0 streets") && !h.contains("Sample city"), "{h}");
    let h = map_html(|| view! { <map_ui::Places vm=vm.clone()/> });
    assert_eq!(count(&h, "place-row"), 0, "{h}");
    let h = map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    assert!(h.contains("class=\"fit bad\"") && h.contains("No roads to show.") && !h.contains("tick"), "{h}");
    let h = map_html(|| view! { <map_ui::TitleBlock vm=vm.clone()/> });
    assert!(h.contains(&format!("id=\"tb-street\">{area}<")) && h.contains("id=\"tb-places\" class=\"fig\">0<"), "{h}");
    assert!(button_tag(&map_html(|| view! { <map_ui::ResetButton vm=vm.clone()/> }), "reset").contains("disabled"));
    assert_eq!(count(&map_html(|| view! { <map_ui::Checks vm=vm.clone()/> }), "<li"), 0);
    assert_eq!(count(&map_html(|| view! { <map_ui::Changes vm=vm.clone()/> }), "<li"), 0);
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert!(h.contains("The roads of this place could not be loaded"), "{h}");
}

#[test]
fn the_key_says_what_the_colours_of_the_places_mean() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::Legend vm=vm.clone()/> });
    assert_eq!(count(&h, "<li>"), 3);
    for words in ["Works", "Changed", "Needs attention"] {
        assert!(h.contains(words), "{words}");
    }
    assert!(h.contains("dot-bad") && h.contains("dot-ok") && h.contains("dot-changed"));
}

#[test]
fn the_map_s_status_line_says_whether_every_place_works() {
    let (vm, storage) = map_vm();
    let h = map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    assert!(h.contains("class=\"fit\"") && h.contains("32 places. Every check passes.") && h.contains("tick"));
    change_a_street(&vm, &storage, 1_000);
    let h = map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    assert!(h.contains("class=\"fit bad\"") && h.contains("1 place needs attention: ") && !h.contains("tick"));
}

#[test]
fn the_places_panel_lists_each_junction_and_street_as_a_link_with_what_is_wrong() {
    let (vm, storage) = map_vm();
    let h = map_html(|| view! { <map_ui::Places vm=vm.clone()/> });
    assert_eq!(count(&h, "class=\"place-row\""), 32);
    assert!(h.contains("href=\"intersection.html?junction=") && h.contains("href=\"street.html?street="));
    assert!(h.contains(">Junctions</h3>") && h.contains(">Streets</h3>") && !h.contains("class=\"st"));
    assert!(h.contains("Press a junction to open its plan. Press a street to open its cross-section."));
    change_a_street(&vm, &storage, 1_000);
    let h = map_html(|| view! { <map_ui::Places vm=vm.clone()/> });
    assert_eq!(count(&h, "class=\"st bad\">Needs attention"), 1);
}

#[test]
fn the_row_of_the_place_the_pointer_is_on_is_marked() {
    let (vm, _) = map_vm();
    let n = vm.view().nodes.iter().find(|n| n.junction).unwrap().uid;
    vm.set_hot(Some(format!("j-{n}")));
    let h = map_html(|| view! { <map_ui::Places vm=vm.clone()/> });
    assert_eq!(count(&h, "class=\"place-row on\""), 1);
}

#[test]
fn the_checks_lead_and_list_say_what_needs_attention() {
    let (vm, storage) = map_vm();
    assert!(map_html(|| view! { <map_ui::ChecksLead vm=vm.clone()/> }).contains("Every check passes in all 32 places."));
    assert_eq!(count(&map_html(|| view! { <map_ui::Checks vm=vm.clone()/> }), "<li"), 0);
    change_a_street(&vm, &storage, 1_000);
    assert!(map_html(|| view! { <map_ui::ChecksLead vm=vm.clone()/> }).contains("1 place needs attention. Open one to see what is wrong and fix it."));
    let h = map_html(|| view! { <map_ui::Checks vm=vm.clone()/> });
    assert_eq!(count(&h, "<li class=\"bad\">"), 1);
    assert!(h.contains("href=\"street.html?street=") && h.contains("Fits the street width"));
}

#[test]
fn the_changes_lead_and_list_say_what_was_changed_and_whether_it_still_works() {
    let (vm, storage) = map_vm();
    assert!(map_html(|| view! { <map_ui::ChangesLead vm=vm.clone()/> }).contains("Nothing changed yet."));
    change_a_street(&vm, &storage, -100);
    assert!(map_html(|| view! { <map_ui::ChangesLead vm=vm.clone()/> }).contains("1 place changed from the city as first laid out."));
    let h = map_html(|| view! { <map_ui::Changes vm=vm.clone()/> });
    assert_eq!(count(&h, "<li class=\"\">"), 1);
    assert!(h.contains("Still works"));
}

#[test]
fn the_map_s_title_block_gives_the_city_its_places_and_what_changed() {
    let (vm, storage) = map_vm();
    change_a_street(&vm, &storage, -100);
    let h = map_html(|| view! { <map_ui::TitleBlock vm=vm.clone()/> });
    assert!(
        h.contains("id=\"tb-street\">Sample city<") && h.contains("id=\"tb-places\" class=\"fig\">32<") && h.contains("id=\"tb-changes\" class=\"fig\">1<")
    );
}

#[test]
fn the_search_box_is_a_labelled_combobox_that_starts_closed() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::SearchBox vm=vm.clone()/> });
    assert!(h.contains("role=\"search\"") && h.contains("id=\"search\"") && h.contains("aria-label=\"Search places\""));
    assert!(h.contains("role=\"combobox\"") && h.contains("aria-expanded=\"false\"") && h.contains("placeholder=\"Search places\""));
    assert_eq!(count(&h, "role=\"option\""), 0);
    assert!(h.contains("type=\"submit\"") && h.contains(">Search<"), "a button as well as Enter, for those who look for one");
}

#[test]
fn a_search_lists_the_places_found_as_options_with_what_is_wrong_and_says_how_many() {
    let (vm, storage) = map_vm();
    change_a_street(&vm, &storage, 1_000);
    vm.set_search("avenue");
    let h = map_html(|| view! { <map_ui::SearchBox vm=vm.clone()/> });
    assert!(h.contains("aria-expanded=\"true\""));
    let shown = vm.results().len().min(crate::map::vm::MapVm::SHOWN);
    assert_eq!(count(&h, "role=\"option\""), shown);
    assert!(h.contains("Sample Avenue") && h.contains("href=\"street.html?street="));
    assert!(h.contains("Needs attention"), "a place that does not work says so in the results");
    assert!(h.contains("place") && h.contains("match"));
}

#[test]
fn the_option_the_arrow_keys_are_on_is_selected_and_named_by_the_box() {
    let (vm, _) = map_vm();
    vm.set_search("junction");
    vm.move_active(1);
    vm.move_active(1);
    let h = map_html(|| view! { <map_ui::SearchBox vm=vm.clone()/> });
    assert!(h.contains("aria-activedescendant=\"sr-1\""));
    assert_eq!(count(&h, "aria-selected=\"true\""), 1);
}

#[test]
fn a_search_that_finds_nothing_says_so_and_how_to_get_the_places_back() {
    let (vm, _) = map_vm();
    vm.set_search("zzz");
    let h = map_html(|| view! { <map_ui::SearchBox vm=vm.clone()/> });
    assert_eq!(count(&h, "role=\"option\""), 0);
    assert!(h.contains("No places match") && h.contains("Clear the search to see all 32."));
}

use crate::map::home;

#[test]
fn the_hero_map_draws_every_junction_but_cannot_be_reached_or_read_out() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <home::HeroMap vm=vm.clone()/> });
    assert_eq!(count(&h, "class=\"m-jc\""), 9);
    assert!(h.contains("aria-hidden=\"true\"") && h.contains("inert"));
}

#[test]
fn the_hero_facts_name_the_city_and_say_whether_it_works() {
    let (vm, storage) = map_vm();
    let h = map_html(|| view! { <home::HeroFacts vm=vm.clone()/> });
    assert!(h.contains("class=\"hero-facts\"") && h.contains("Sample city: 9 junctions, 23 streets.") && h.contains("Every check passes."));
    assert!(h.contains("tick"));
    change_a_street(&vm, &storage, 1_000);
    let h = map_html(|| view! { <home::HeroFacts vm=vm.clone()/> });
    assert!(h.contains("class=\"hero-facts bad\"") && h.contains("needs attention") && !h.contains("tick"));
}

// ---- in French ------------------------------------------------------------------------------

/// Every view of the map page, drawn to HTML, with a street changed so it fails and the search open.
fn every_view(vm: &Rc<MapVm>) -> String {
    let mut h = String::new();
    h += &map_html(|| view! { <map_ui::MapHeader vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::ResetButton vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::MapTools vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::Legend vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::Places vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::ChecksLead vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::Checks vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::ChangesLead vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::Changes vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::TitleBlock vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::SearchBox vm=vm.clone()/> });
    h
}

/// English words the map page says, none of which a French page may show.
const ENGLISH: [&str; 23] = [
    "Does it work",
    "Key to the map",
    "Opens the",
    "Needs attention",
    "City map",
    "Start over",
    "Zoom out",
    "Zoom in",
    "Whole city",
    "Search places",
    "Places found",
    "Press a junction",
    "Junctions",
    "Streets",
    "Places",
    "Changed",
    "Still works",
    "Works",
    "place needs",
    "places match",
    "place matches",
    "Every check",
    " junction",
];

#[test]
fn every_view_of_the_map_page_speaks_french_with_no_english_left() {
    let (vm, storage) = map_vm_in(Locale::FrCa);
    change_a_street(&vm, &storage, 1_000);
    vm.set_search("rue");
    let h = every_view(&vm);
    for english in ENGLISH {
        assert!(!h.contains(english), "{english:?} in {h}");
    }
    for french in ["Carte de la ville", "Jonctions", "Rues", "Lieux", "À corriger", "Recommencer", "Toute la ville", "Rechercher des lieux"] {
        assert!(h.contains(french), "{french:?} in {h}");
    }
    assert!(h.contains("1 lieu à corriger\u{a0}: "), "the status line, with a no-break space before the colon: {h}");
    assert_eq!(vm.i18n().missing(), Vec::<String>::new());
}

#[test]
fn a_french_map_page_with_nothing_to_show_says_why_in_french() {
    let (ports, ..) = test_ports();
    let vm = MapVm::new(ports, crate::i18n_for(Locale::FrCa));
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert!(h.contains("Les rues de ce lieu n’ont pas pu être chargées"), "{h}");
    let h = map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    assert!(h.contains("Aucune rue à afficher."), "{h}");
}

#[test]
fn the_places_list_and_the_status_follow_a_switch_of_language_without_a_new_view_model() {
    let (vm, storage) = map_vm();
    change_a_street(&vm, &storage, 1_000);
    let owner = Owner::new();
    owner.set();
    let v = StoredValue::new_local(vm.clone());
    let status = Memo::new(move |_| v.with_value(|vm| vm.status().text));
    let first_row = Memo::new(move |_| v.with_value(|vm| vm.junction_label(vm.view().nodes.iter().find(|n| n.junction).unwrap().uid)));
    assert!(status.get().starts_with("1 place needs attention: "));
    assert!(first_row.get().ends_with("Opens the junction plan."));
    vm.i18n().set(Locale::FrCa);
    assert!(status.get().starts_with("1 lieu à corriger"), "{}", status.get());
    assert!(first_row.get().ends_with("Ouvre le plan de la jonction."), "{}", first_row.get());
}

// ---- the messages ---------------------------------------------------------------------------

#[test]
fn the_map_ftl_files_have_the_same_messages_and_variables() {
    use crate::shared::i18n::tests::parity_problems;
    assert_eq!(parity_problems(crate::map::RESOURCES.en, crate::map::RESOURCES.fr), Vec::<String>::new());
}

/// A plural message said for 0, 1 and 2 in one language.
fn counted(i18n: &I18n, key: &str, extra: fn(Args) -> Args) -> [String; 3] {
    [0, 1, 2].map(|n| i18n.tr_now(key, &extra(Args::new().num("n", n))))
}

#[test]
fn zero_and_one_are_singular_in_french_and_zero_is_plural_in_english_in_every_count() {
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let none = |a: Args| a;
    let names = |a: Args| a.str("names", "X");
    let shown = |a: Args| a.num("shown", 8);
    let cases: [(&str, fn(Args) -> Args, [&str; 3], [&str; 3]); 9] = [
        ("map-streets", none, ["0 streets", "1 street", "2 streets"], ["0 rue", "1 rue", "2 rues"]),
        ("map-junction-count", none, ["0 junctions", "1 junction", "2 junctions"], ["0 jonction", "1 jonction", "2 jonctions"]),
        (
            "map-search-found",
            none,
            ["0 places match", "1 place matches", "2 places match"],
            ["0 lieu correspond", "1 lieu correspond", "2 lieux correspondent"],
        ),
        (
            "map-search-found-more",
            shown,
            ["0 places match · showing the first 8", "1 place matches · showing the first 8", "2 places match · showing the first 8"],
            ["0 lieu correspond · 8 premiers affichés", "1 lieu correspond · 8 premiers affichés", "2 lieux correspondent · 8 premiers affichés"],
        ),
        (
            "map-checks-failing",
            none,
            [
                "0 places need attention. Open one to see what is wrong and fix it.",
                "1 place needs attention. Open one to see what is wrong and fix it.",
                "2 places need attention. Open one to see what is wrong and fix it.",
            ],
            [
                "0 lieu est à corriger. Ouvrez-le pour voir ce qui ne va pas et le corriger.",
                "1 lieu est à corriger. Ouvrez-le pour voir ce qui ne va pas et le corriger.",
                "2 lieux sont à corriger. Ouvrez-en un pour voir ce qui ne va pas et le corriger.",
            ],
        ),
        (
            "map-checks-pass",
            none,
            ["Every check passes in all 0 places.", "Every check passes in all 1 places.", "Every check passes in all 2 places."],
            [
                "Toutes les vérifications réussissent dans l’unique lieu.",
                "Toutes les vérifications réussissent dans l’unique lieu.",
                "Toutes les vérifications réussissent dans les 2 lieux.",
            ],
        ),
        (
            "map-changes-some",
            none,
            [
                "0 places changed from the city as first laid out.",
                "1 place changed from the city as first laid out.",
                "2 places changed from the city as first laid out.",
            ],
            [
                "0 lieu modifié par rapport à la ville telle que tracée au départ.",
                "1 lieu modifié par rapport à la ville telle que tracée au départ.",
                "2 lieux modifiés par rapport à la ville telle que tracée au départ.",
            ],
        ),
        (
            "map-status-ok",
            none,
            ["0 places. Every check passes.", "1 places. Every check passes.", "2 places. Every check passes."],
            [
                "0 lieu. Toutes les vérifications réussissent.",
                "1 lieu. Toutes les vérifications réussissent.",
                "2 lieux. Toutes les vérifications réussissent.",
            ],
        ),
        (
            "map-status-bad",
            names,
            ["0 places need attention: X.", "1 place needs attention: X.", "2 places need attention: X."],
            ["0 lieu à corriger\u{a0}: X.", "1 lieu à corriger\u{a0}: X.", "2 lieux à corriger\u{a0}: X."],
        ),
    ];
    for (key, extra, english, french) in cases {
        assert_eq!(counted(&en, key, extra), english.map(String::from), "{key} in English");
        assert_eq!(counted(&fr, key, extra), french.map(String::from), "{key} in French");
    }
    let more = |a: Args| a.str("names", "X");
    assert_eq!(counted(&fr, "map-status-bad-more", more)[1], "1 lieu à corriger\u{a0}: X et d’autres.");
    assert_eq!(counted(&en, "map-status-bad-more", more)[2], "2 places need attention: X and more.");
}

// ---- the page's markup ----------------------------------------------------------------------

const MAP_HTML: &str = include_str!("../../web/map.html");

#[test]
fn every_data_i18n_key_in_map_html_exists_in_both_languages() {
    use crate::shell::view::i18n_keys;
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let keys = i18n_keys(MAP_HTML);
    assert!(keys.len() > 35, "the page should have been converted: {}", keys.len());
    for (key, _) in keys {
        en.tr_now(&key, &Args::new());
        fr.tr_now(&key, &Args::new());
    }
    assert_eq!(en.missing(), Vec::<String>::new(), "missing in en");
    assert_eq!(fr.missing(), Vec::<String>::new(), "missing in fr");
}

#[test]
fn the_english_text_in_map_html_is_the_english_message() {
    let en = crate::i18n_for(Locale::En);
    for (key, fallback) in crate::shell::view::i18n_keys(MAP_HTML) {
        assert_eq!(fallback, en.tr_now(&key, &Args::new()), "{key}");
    }
}

#[test]
fn map_html_has_a_language_row_and_sets_the_language_in_the_head() {
    assert!(MAP_HTML.contains(r#"data-lang="en" lang="en" aria-pressed="true">English<"#));
    assert!(MAP_HTML.contains(r#"data-lang="fr-CA" lang="fr" aria-pressed="false">Français<"#));
    assert_eq!(MAP_HTML.matches("<script>").count(), 1);
    assert!(MAP_HTML.contains(r#"localStorage.getItem("cityloom-lang")"#));
}

#[test]
fn the_window_title_of_the_map_page_is_its_header_s_message_in_english() {
    // The header sets the title in the language of the page; the markup's own is the English message.
    let en = crate::i18n_for(Locale::En);
    assert!(MAP_HTML.contains(&format!("<title>{}</title>", en.tr_now("map-page-title", &Args::new()))));
    assert!(!MAP_HTML.contains("<title data-i18n"), "the shell does not write the title");
}
