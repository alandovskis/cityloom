//! The map page drawn to HTML on the host.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::testing::{count, html};
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
    let store = CityStore::sample(storage.clone());
    let mut e = store.open().street_editor(street, 0).unwrap();
    let u = e.view().segments[0].uid;
    e.set_width(u, e.view().segments[0].width_mm + by_mm);
    assert!(store.write_street(street, e.snapshot()));
    vm.reload();
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
fn a_map_page_whose_area_s_roads_were_not_had_names_the_area() {
    let (ports, _, _) = test_ports(); // the default area, whose roads are not kept
    let vm = MapVm::new(ports, crate::i18n_for(Locale::En));
    let area = crate::place::area::default_area().name;
    let h = map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    assert!(h.contains("class=\"fit bad\"") && h.contains("No roads to show.") && !h.contains("tick"), "{h}");
    let h = map_html(|| view! { <map_ui::TitleBlock vm=vm.clone()/> });
    assert!(h.contains(&format!("id=\"tb-street\">{area}<")) && h.contains("id=\"tb-places\" class=\"fig\">0<"), "{h}");
    assert_eq!(count(&map_html(|| view! { <map_ui::Checks vm=vm.clone()/> }), "<li"), 0);
    assert_eq!(count(&map_html(|| view! { <map_ui::Changes vm=vm.clone()/> }), "<li"), 0);
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert!(h.contains("The roads of this place could not be loaded"), "{h}");
}

#[test]
fn the_key_says_what_the_colours_of_the_places_mean() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::Legend vm=vm.clone()/> });
    assert_eq!(count(&h, "<li>"), 4);
    for words in ["Works", "Changed", "Needs attention", "Conflicts with an area rule"] {
        assert!(h.contains(words), "{words}");
    }
    assert!(h.contains("dot-bad") && h.contains("dot-ok") && h.contains("dot-changed") && h.contains("dot-conflict"));
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

#[test]
fn the_hero_speaks_french_with_no_english_left() {
    let (vm, storage) = map_vm_in(Locale::FrCa);
    let h = map_html(|| view! { <home::HeroFacts vm=vm.clone()/> });
    assert!(h.contains("Sample city\u{a0}: 9 jonctions, 23 rues."), "the city's name is data, the rest French: {h}");
    assert!(h.contains("Toutes les vérifications réussissent."), "{h}");
    change_a_street(&vm, &storage, 1_000);
    let map = map_html(|| view! { <home::HeroMap vm=vm.clone()/> });
    assert!(map.contains(" \u{b7} modifié<") || map.contains(">modifié<"), "a changed place is tagged in French: {map}");
    for english in ["changed", "Changed", "junctions", "streets", "Every check", "Opens the", " wide.", "Traffic signal", "traffic signal", "stop"] {
        assert!(!map.contains(english), "{english:?} in the hero map");
    }
    for english in ["junctions", "streets", "Every check"] {
        assert!(!h.contains(english), "{english:?} in the hero facts");
    }
}

// ---- in French ------------------------------------------------------------------------------

/// Every view of the map page, drawn to HTML, with a street changed so it fails and the search open.
fn every_view(vm: &Rc<MapVm>) -> String {
    let mut h = String::new();
    h += &map_html(|| view! { <map_ui::MapTools vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::Legend vm=vm.clone()/> });
    h += &map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
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
    for french in ["Lieux", "À corriger", "Toute la ville", "Rechercher des lieux"] {
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
    let cases: [(&str, fn(Args) -> Args, [&str; 3], [&str; 3]); 8] = [
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
            ["0 places. Every check passes.", "1 place. Every check passes.", "2 places. Every check passes."],
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
    // `map-checks-pass` is never said of 0 places (a city with none says `map-checks-no-places`), so only 1 and 2.
    let pass = counted(&fr, "map-checks-pass", none);
    assert_eq!(pass[1..], ["Toutes les vérifications réussissent dans l’unique lieu.", "Toutes les vérifications réussissent dans les 2 lieux."]);
    let pass_en = counted(&en, "map-checks-pass", none);
    assert_eq!(pass_en[1..], ["Every check passes in the one place.", "Every check passes in all 2 places."]);
    // `map-search-none` counts the places under `places`, not `n`.
    let none_found = |i18n: &I18n, places: i64| i18n.tr_now("map-search-none", &Args::new().str("query", "zz").num("places", places));
    assert_eq!(none_found(&en, 1), "No places match “zz”. Clear the search to see the one place.");
    assert_eq!(none_found(&en, 2), "No places match “zz”. Clear the search to see all 2.");
    assert_eq!(none_found(&fr, 1), "Aucun lieu ne correspond à «\u{a0}zz\u{a0}». Effacez la recherche pour revoir l’unique lieu.");
    assert_eq!(none_found(&fr, 2), "Aucun lieu ne correspond à «\u{a0}zz\u{a0}». Effacez la recherche pour revoir les 2 lieux.");
    let more = |a: Args| a.str("names", "X");
    assert_eq!(counted(&fr, "map-status-bad-more", more)[1], "1 lieu à corriger\u{a0}: X et d’autres.");
    assert_eq!(counted(&en, "map-status-bad-more", more)[2], "2 places need attention: X and more.");
}

// ---- the page's markup ----------------------------------------------------------------------

const MAP_HTML: &str = include_str!("../../../../web/map.html");

#[test]
fn every_data_i18n_key_in_map_html_exists_in_both_languages() {
    use crate::shell::view::i18n_keys;
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let keys = i18n_keys(MAP_HTML);
    assert!(keys.len() > 32, "the page should have been converted: {}", keys.len());
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

// ---- the home page's markup -----------------------------------------------------------------

const INDEX_HTML: &str = include_str!("../../../../web/index.html");

#[test]
fn every_data_i18n_key_in_index_html_exists_in_both_languages() {
    use crate::shell::view::i18n_keys;
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let keys = i18n_keys(INDEX_HTML);
    assert!(keys.len() >= 17, "the page should have been converted: {}", keys.len());
    for (key, _) in keys {
        en.tr_now(&key, &Args::new());
        fr.tr_now(&key, &Args::new());
    }
    assert_eq!(en.missing(), Vec::<String>::new(), "missing in en");
    assert_eq!(fr.missing(), Vec::<String>::new(), "missing in fr");
}

#[test]
fn the_english_text_in_index_html_is_the_english_message() {
    let en = crate::i18n_for(Locale::En);
    for (key, fallback) in crate::shell::view::i18n_keys(INDEX_HTML) {
        assert_eq!(fallback, en.tr_now(&key, &Args::new()), "{key}");
    }
}

#[test]
fn index_html_has_a_language_row_and_sets_the_language_in_the_head() {
    assert!(INDEX_HTML.contains(r#"data-lang="en" lang="en" aria-pressed="true">English<"#));
    assert!(INDEX_HTML.contains(r#"data-lang="fr-CA" lang="fr" aria-pressed="false">Français<"#));
    assert_eq!(INDEX_HTML.matches("<script>").count(), 1);
    assert!(INDEX_HTML.contains(r#"localStorage.getItem("cityloom-lang")"#));
}

#[test]
fn the_window_title_of_the_home_page_is_its_message_in_english() {
    // mount_home sets the title in the language of the page; the markup's own is the English message.
    let en = crate::i18n_for(Locale::En);
    assert!(INDEX_HTML.contains(&format!("<title>{}</title>", en.tr_now("map-home-title", &Args::new()))));
    assert!(!INDEX_HTML.contains("<title data-i18n"), "the shell does not write the title");
}

#[test]
fn the_openstreetmap_credit_stays_a_link_and_reads_naturally_in_both_languages() {
    assert!(INDEX_HTML.contains(r#"<a href="https://www.openstreetmap.org/copyright" data-i18n="map-home-osm-contributors">"#));
    let credit = |i18n: &I18n| ["map-home-streets-by", "map-home-osm-contributors"].map(|k| i18n.tr_now(k, &Args::new())).join(" ");
    assert_eq!(credit(&crate::i18n_for(Locale::En)), "Streets © OpenStreetMap contributors");
    assert_eq!(credit(&crate::i18n_for(Locale::FrCa)), "Rues © les contributeurs d’OpenStreetMap");
}

#[test]
fn map_html_has_one_h1_the_city_map() {
    let html = include_str!("../../../../web/map.html");
    assert_eq!(html.matches("<h1").count(), 1);
    assert!(html.contains(r#"<h1 id="h-drawing" class="sr-only" data-i18n="map-city-map">City map</h1>"#));
}

// ---- the Area rules panel ------------------------------------------------------------------

use crate::map::areas_view::Areas;
use crate::restriction::fixtures::{bent_midpoint, bent_store, square};
use crate::restriction::model::Rule;
use crate::restriction::store::RestrictionStore;
use crate::shared::ports::{FakeMapper, MapEvent, Ports};
use crate::shared::testing::button_tag;

/// Bendville's map, its tiles answered, with the areas kept in storage first, round its street's midpoint.
fn ready_areas_vm(locale: Locale, areas: &[(&str, Rule)]) -> Rc<MapVm> {
    ready_areas_vm_off(locale, areas, 0.0)
}

/// The same, the areas `east` degrees east of the street's midpoint.
fn ready_areas_vm_off(locale: Locale, areas: &[(&str, Rule)], east: f64) -> Rc<MapVm> {
    let (ports, _, storage) = test_ports();
    let store = bent_store(storage.clone());
    let (_, mid) = bent_midpoint(&store);
    for (name, rule) in areas {
        RestrictionStore::new(storage.clone()).add(name, *rule, square([mid[0] + east, mid[1]], 0.0005)).unwrap();
    }
    let mapper = Rc::new(FakeMapper::default());
    let vm = MapVm::new(Ports { mapper: mapper.clone(), ..ports }, crate::i18n_for(locale));
    vm.attach();
    mapper.say(MapEvent::Ready { bounds: [-74.0, 45.0, -73.0, 46.0] });
    vm
}

fn areas_html(vm: &Rc<MapVm>) -> String {
    map_html(|| view! { <Areas vm=vm.clone()/> })
}

/// Draws a triangle and closes it: the outline then waits for its rule and name.
fn close_a_triangle(vm: &MapVm) {
    assert!(vm.areas().start());
    for p in [[-73.6, 45.5], [-73.59, 45.5], [-73.59, 45.51]] {
        assert!(vm.areas().add_at(p));
    }
    assert!(vm.areas().close());
}

#[test]
fn the_tab_with_no_area_says_how_to_begin_and_offers_to_declare_one() {
    let vm = ready_areas_vm(Locale::En, &[]);
    let h = areas_html(&vm);
    assert!(h.contains("No area rules yet. Declare one to check the streets inside it."), "{h}");
    assert!(h.contains("id=\"declare-area\"") && h.contains("Declare an area"), "{h}");
    assert!(!button_tag(&h, "declare-area").contains("disabled"));
    assert_eq!(count(&h, "<li"), 0);
}

#[test]
fn declaring_is_disabled_until_there_is_a_map() {
    let (vm, _) = map_vm(); // the sample city: its tiles never answer here
    assert!(button_tag(&areas_html(&vm), "declare-area").contains("disabled"));
}

#[test]
fn each_area_is_a_row_with_its_rule_and_streets_and_the_selected_one_opens_its_details() {
    let vm = ready_areas_vm(Locale::En, &[("Car zone", Rule::CarFree)]);
    let h = areas_html(&vm);
    assert!(h.contains("Car zone") && h.contains("Car-free · One street inside; one conflicts with the rule."), "{h}");
    assert!(h.contains("aria-expanded=\"false\"") && !h.contains("aria-controls") && !h.contains("id=\"area-delete\""), "{h}");
    vm.areas().select(Some("r1"));
    let h = areas_html(&vm);
    assert!(h.contains("aria-expanded=\"true\"") && h.contains("aria-controls=\"area-details-r1\"") && h.contains("id=\"area-details-r1\""), "{h}");
    assert!(!h.contains("aria-pressed"), "{h}");
    for id in ["area-name", "area-rule", "area-redraw", "area-delete"] {
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}");
    }
    assert!(h.contains("Streets inside") && h.contains("href=\"street.html?street=1\"") && h.contains("It has 2 driving lanes."), "{h}");
    assert!(!h.contains("id=\"area-speed\""), "no speed for a car-free area");
    assert!(vm.areas().set_rule("r1", Rule::SpeedLimit { kmh: 30 }));
    assert!(areas_html(&vm).contains("id=\"area-speed\""));
}

#[test]
fn while_drawing_the_tab_says_how_and_counts_the_corners_and_finish_needs_three() {
    let vm = ready_areas_vm(Locale::En, &[]);
    vm.areas().start();
    vm.areas().add_at([-73.6, 45.5]);
    let h = areas_html(&vm);
    assert!(h.contains("Click the map to place each corner") && h.contains("1 corner"), "{h}");
    assert!(button_tag(&h, "area-finish").contains("disabled"));
    vm.areas().add_at([-73.59, 45.5]);
    vm.areas().add_at([-73.59, 45.51]);
    assert!(!button_tag(&areas_html(&vm), "area-finish").contains("disabled"));
}

#[test]
fn a_closed_outline_asks_for_its_rule_and_name() {
    let vm = ready_areas_vm(Locale::En, &[]);
    close_a_triangle(&vm);
    let h = areas_html(&vm);
    assert!(
        h.contains("id=\"area-rule\"") && h.contains("id=\"area-name\"") && h.contains("id=\"area-save\"") && h.contains("placeholder=\"Car-free\""),
        "{h}"
    );
    assert_eq!(count(&h, "<option"), 4, "the four rules");
}

#[test]
fn a_closed_outline_promises_no_key_on_the_map_only_its_cancel_button() {
    // The map gives a closed outline no key (its rule and name are typed in the form), so Escape cannot cancel it.
    for locale in [Locale::En, Locale::FrCa] {
        let vm = ready_areas_vm(locale, &[]);
        close_a_triangle(&vm);
        let h = areas_html(&vm);
        for key in ["Escape", "Échap", "Enter", "Entrée"] {
            assert!(!h.contains(key), "{key} in {h}");
        }
        assert!(h.contains("id=\"area-cancel\""), "{h}");
    }
}

#[test]
fn while_an_area_is_redrawn_it_cannot_be_deleted() {
    let vm = ready_areas_vm(Locale::En, &[("Car zone", Rule::CarFree)]);
    vm.areas().select(Some("r1"));
    assert!(vm.areas().start_redraw("r1"));
    let h = areas_html(&vm);
    assert!(!h.contains("id=\"area-delete\"") && !h.contains("id=\"declare-area\""), "{h}");
    assert!(h.contains("id=\"area-finish\"") && h.contains("id=\"area-cancel\""), "{h}");
}

#[test]
fn a_deleted_area_offers_undo() {
    let vm = ready_areas_vm(Locale::En, &[("Car zone", Rule::CarFree)]);
    vm.areas().delete("r1");
    let h = areas_html(&vm);
    assert!(h.contains("“Car zone” deleted.") && h.contains("id=\"area-undo\""), "{h}");
}

#[test]
fn the_tab_speaks_french_with_no_english_left() {
    let vm = ready_areas_vm(Locale::FrCa, &[("Zone A", Rule::CarFree)]);
    vm.areas().select(Some("r1"));
    let h = areas_html(&vm);
    assert!(
        h.contains("Zone sans voitures") && h.contains("Supprimer") && h.contains("Rues à l’intérieur") && h.contains("Elle a 2 voies de circulation."),
        "{h}"
    );
    for english in ["Delete", "Redraw", "Streets inside", "Car-free", "driving lanes", "Declare an area"] {
        assert!(!h.contains(english), "{english} in {h}");
    }
}

#[test]
fn the_drawing_and_the_naming_speak_french_with_no_english_left() {
    let vm = ready_areas_vm(Locale::FrCa, &[]);
    vm.areas().start();
    vm.areas().add_at([-73.6, 45.5]);
    let h = areas_html(&vm);
    assert!(h.contains("Cliquez sur la carte") && h.contains("1 coin") && h.contains("Terminer") && h.contains("Annuler"), "{h}");
    vm.areas().add_at([-73.59, 45.5]);
    vm.areas().add_at([-73.59, 45.51]);
    vm.areas().close();
    let h = areas_html(&vm);
    assert!(h.contains("Choisissez la règle") && h.contains("Enregistrer") && h.contains("placeholder=\"Zone sans voitures\""), "{h}");
    for english in ["Click", "Finish", "Cancel", "Save", "Name", "Rule", "Car-free", "Speed limit"] {
        assert!(!h.contains(&format!(">{english}")) && !h.contains(&format!("\"{english}")), "{english} in {h}");
    }
}

#[test]
fn map_html_has_the_area_rules_in_a_left_panel_and_not_in_the_notes() {
    assert!(!MAP_HTML.contains("id=\"t-areas\"") && !MAP_HTML.contains("id=\"p-areas\"") && !MAP_HTML.contains("data-left=\"none\""));
    assert!(
        MAP_HTML.contains(
            r#"id="inspector-toggle" class="edge-tab left" data-covers="left" aria-controls="areas-panel" aria-expanded="true" aria-keyshortcuts="[""#
        )
    );
    assert!(MAP_HTML.contains(r#"<aside class="panel left" id="areas-panel""#));
    let panel = &MAP_HTML[MAP_HTML.find("id=\"areas-panel\"").unwrap()..];
    assert!(panel[..panel.find("</aside>").unwrap()].contains("id=\"areas-slot\""));
    assert!(MAP_HTML.contains(r#"localStorage.getItem("cityloom-inspector")==="closed""#), "the head script restores a closed panel");
}

#[test]
fn the_keyboard_help_lists_the_key_for_the_area_rules_beside_the_notes() {
    // The map takes no Tab or Enter for the places (Tab leaves it): they are opened from the search, which is what
    // the help says.
    assert!(!MAP_HTML.contains("map-key-next") && !MAP_HTML.contains("next place"));
    assert!(MAP_HTML.contains(r#"<kbd>↓</kbd> <kbd data-i18n="ui-key-enter">Enter</kbd> <span data-i18n="map-key-open">open a place found</span>"#));
    assert!(MAP_HTML.contains(r#"<kbd>[</kbd> <span data-i18n="map-key-areas">area rules</span>"#));
    assert!(MAP_HTML.contains(r#"<kbd>]</kbd> <span data-i18n="map-key-panels">notes</span>"#));
}

#[test]
fn an_area_with_no_street_inside_shows_no_streets_heading() {
    let vm = ready_areas_vm_off(Locale::En, &[("Far", Rule::CarFree)], 0.05);
    vm.areas().select(Some("r1"));
    let h = areas_html(&vm);
    assert!(h.contains("No loaded street is inside.") && h.contains("id=\"area-delete\""), "{h}");
    assert!(!h.contains("Streets inside") && count(&h, "<ul") == 1, "{h}");
}

#[test]
fn the_details_and_the_naming_form_ask_for_the_rule_then_the_name() {
    let vm = ready_areas_vm(Locale::En, &[("Car zone", Rule::CarFree)]);
    vm.areas().select(Some("r1"));
    let h = areas_html(&vm);
    assert!(h.find("id=\"area-rule\"").unwrap() < h.find("id=\"area-name\"").unwrap(), "details: {h}");
    vm.areas().select(None);
    close_a_triangle(&vm);
    let h = areas_html(&vm);
    assert!(h.find("id=\"area-rule\"").unwrap() < h.find("id=\"area-name\"").unwrap(), "naming: {h}");
}

#[test]
fn a_name_that_looks_like_markup_is_shown_as_text() {
    let name = r#"<img src=x onerror=alert(1)> & "Zone""#;
    let vm = ready_areas_vm(Locale::En, &[(name, Rule::CarFree)]);
    vm.areas().select(Some("r1"));
    let h = areas_html(&vm);
    assert!(!h.contains("<img"), "{h}");
    assert!(h.contains("&lt;img src=x onerror=alert(1)&gt; &amp; "), "the row: {h}");
}

#[test]
fn the_panel_is_opened_through_the_shell_s_own_toggle_only_where_it_is_closed() {
    use crate::map::areas_view::reveal_clicks;
    assert_eq!(reveal_clicks(Some("true")), Vec::<&str>::new());
    assert_eq!(reveal_clicks(Some("false")), ["inspector-toggle"]);
    assert_eq!(reveal_clicks(None), Vec::<&str>::new(), "no toggle on the page: nothing to press");
}

/// The home page is built before the area's roads are read, so that its search does not wait for them.
#[test]
fn a_city_whose_roads_arrive_after_the_page_was_made_is_drawn_and_fitted_on_reopening() {
    use cityloom_model::{
        Control, Direction, Edge, Junction, Lane, LaneKind, Network, NodeId, NodeRef, Point, Provenance, Road, RoadId, RoadType, WayRef, Zone,
    };
    let (ports, _, storage) = crate::shared::ports::test_ports();
    let vm = MapVm::new(ports, crate::i18n_for(Locale::En));
    assert!(vm.view().edges.is_empty(), "no roads are kept yet");
    let node = |id, x_m| Junction {
        id: NodeId(id),
        provenance: Provenance::of_nodes([NodeRef::new(id as i64)]),
        position: Point::new(x_m, 0.0),
        junction: false,
        control: Control::None,
    };
    let lane = |way| Lane { kind: LaneKind::Driving, direction: way, width_m: 3.0, buffer_type: None, windows: vec![], shape: vec![] };
    let network = Network {
        left_hand: false,
        corridors: vec![],
        junctions: vec![node(1, 0.0), node(2, 100.0)],
        roads: vec![Road {
            edge: Edge { id: RoadId(1), from: NodeId(1), to: NodeId(2) },
            provenance: Provenance::of_ways([WayRef::new(7)]),
            name: "Only Street".into(),
            road_type: RoadType::Local,
            zones: vec![Zone::whole(vec![lane(Direction::Forward), lane(Direction::Backward)])],
            points: vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0)],
        }],
    };
    assert!(CityStore::current(storage.clone()).keep_network_from(&network, None));
    vm.zoom_in();
    vm.reopen();
    assert_eq!(vm.view().edges.len(), 1);
    let camera = vm.camera();
    assert!(!camera.moved, "the page shows the whole city");
    assert_eq!(camera.k, camera.fit_k);
    assert!(camera.world.x1 - camera.world.x0 > 100.0, "the world is the city's, not the empty one: {:?}", camera.world);
}

// ---- the network checks ---------------------------------------------------------------------

use crate::map::network_view;

/// Another page takes a crossing off one arm of a junction of the city, and the map reads the city again.
fn take_a_crossing_away(vm: &MapVm, storage: &Rc<MemoryStorage>) {
    let store = CityStore::sample(storage.clone());
    let mut j = store.open().junction_editor(2, 0).unwrap();
    let arm = j.snapshot().arms.iter().find(|a| a.crossing.is_some()).map(|a| a.uid).unwrap();
    assert!(j.set_crossing(arm, false));
    assert!(store.write_junction(2, j.snapshot()));
    vm.reload();
}

#[test]
fn the_network_checks_are_a_section_per_check_with_its_count_in_the_heading() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <network_view::NetworkChecks vm=vm.clone()/> });
    assert_eq!(count(&h, "<section"), 0, "a check is not a landmark: {h}");
    assert!(!h.contains("aria-labelledby"), "{h}");
    assert_eq!(count(&h, "<h3"), 6);
    // the name and the count are read apart: "Crossings: no findings"
    assert!(h.contains("Crossings</span><span class=\"sr-only\">: </span><span class=\"fig net-count\">no findings</span>"), "{h}");
    for (id, label) in [
        ("cycling", "Cycling links"),
        ("access", "Pedestrian street access"),
        ("crossings", "Crossings"),
        ("lanes", "Lane counts"),
        ("transit", "Transit corridors"),
        ("walking", "Walking routes"),
    ] {
        assert!(h.contains(&format!("id=\"net-h-{id}\"")) && h.contains(&format!("data-rule=\"{id}\"")) && h.contains(label), "{id}: {h}");
    }
    assert_eq!(count(&h, "no findings"), 6);
    assert_eq!(count(&h, "<li"), 0);
    let lead = map_html(|| view! { <network_view::NetworkLead vm=vm.clone()/> });
    assert!(lead.contains("found nothing"), "{lead}");
}

#[test]
fn a_finding_says_why_and_links_the_street_and_the_junction() {
    let (vm, storage) = map_vm();
    take_a_crossing_away(&vm, &storage);
    let h = map_html(|| view! { <network_view::NetworkChecks vm=vm.clone()/> });
    assert_eq!(count(&h, "<li data-severity"), 1, "{h}");
    assert!(h.contains("1 finding") && h.contains("has no pedestrian crossing") && h.contains("Problem"), "{h}");
    assert!(h.contains("href=\"street.html?street=") && h.contains("href=\"intersection.html?junction=2\""), "{h}");
    assert!(h.contains("data-severity=\"problem\"") && h.contains("class=\"bad\"") && h.contains("data-rule=\"crossings\""), "{h}");
    // the places are a list of their own, so a screen reader says how many there are
    assert!(h.contains("<ul class=\"net-links\"") && count(&h, "<ul class=\"net-links\"") == 1, "{h}");
    assert_eq!(count(&h, "class=\"net-link\""), 2, "{h}");
    let lead = map_html(|| view! { <network_view::NetworkLead vm=vm.clone()/> });
    assert!(lead.contains("found 1 finding"), "{lead}");
}

#[test]
fn the_network_checks_are_worded_in_french() {
    let (vm, storage) = map_vm_in(Locale::FrCa);
    take_a_crossing_away(&vm, &storage);
    let h = map_html(|| view! { <network_view::NetworkChecks vm=vm.clone()/> });
    assert!(h.contains("Passages pour piétons") && h.contains("1 constat") && h.contains("passage pour piétons") && h.contains("Problème"), "{h}");
}

#[test]
fn the_provenance_tab_lists_a_group_per_kind_with_its_count_source_and_impact() {
    use crate::map::source_view;
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <source_view::SourceFindings vm=vm.clone()/> });
    assert_eq!(count(&h, "<h3"), 5, "{h}");
    for (id, label) in
        [("name", "Street names"), ("disconnected", "Disconnected streets"), ("ends", "Dead ends"), ("width", "Widths"), ("speed", "Speed limits")]
    {
        assert!(h.contains(&format!("id=\"src-h-{id}\"")) && h.contains(&format!("data-kind=\"{id}\"")) && h.contains(label), "{id}: {h}");
    }
    // the sample city's streets have no name from OpenStreetMap: each finding says so, where it comes from and what it could mislead
    assert!(h.contains("is not in OpenStreetMap") && h.contains("Source:") && h.contains("Design impact:") && h.contains("The road has no name tag."), "{h}");
    assert!(h.contains("href=\"street.html?street="), "{h}");
    // a count in the heading, read apart from the name, and no mark or word of a design failure
    assert!(h.contains("Street names</span><span class=\"sr-only\">: </span><span class=\"fig net-count\">"), "{h}");
    assert!(!h.contains("class=\"bad\"") && !h.contains("data-severity") && !h.contains("Problem"), "{h}");
    let lead = map_html(|| view! { <source_view::SourceLead vm=vm.clone()/> });
    assert!(lead.contains("may be missing or wrong"), "{lead}");
}

#[test]
fn source_findings_do_not_reach_the_status_line_the_checks_lead_or_the_network_checks() {
    let (vm, _) = map_vm();
    assert!(vm.source_count() > 0, "the sample city has unnamed streets");
    // the same numbers as a city whose findings were never worked out
    let status = vm.status();
    let lead = vm.checks_lead();
    let network = vm.network_count();
    let _ = vm.source_groups();
    assert_eq!((vm.status(), vm.checks_lead(), vm.network_count()), (status.clone(), lead.clone(), network));
    assert!(!status.bad || !status.text.contains("source"), "{status:?}");
    assert!(!lead.contains("possible gap") && !lead.contains("source data"), "{lead}");
    assert!(vm.network_rules().iter().all(|c| c.items.iter().all(|i| !i.text.contains("OpenStreetMap"))));
}

#[test]
fn the_provenance_tab_is_worded_in_french() {
    use crate::map::source_view;
    let (vm, _) = map_vm_in(Locale::FrCa);
    let h = map_html(|| view! { <source_view::SourceFindings vm=vm.clone()/> });
    assert!(h.contains("Noms de rues") && h.contains("lacune") && h.contains("Effet sur la conception") && h.contains("n’a pas d’étiquette de nom"), "{h}");
}

// ---- the design review report ----------------------------------------------------------------

mod report {
    use super::*;
    use crate::shared::ports::{Ports, test_ports_with_pdf};
    use cityloom_report::{Block, ReportData};

    fn sample_in(locale: Locale) -> (Rc<MapVm>, Rc<crate::shared::ports::FakePdfWriter>, Rc<crate::shared::ports::FakeDownloads>, Rc<MemoryStorage>) {
        let (ports, pdf, downloads) = test_ports_with_pdf();
        let storage = Rc::new(MemoryStorage::default());
        storage.set_now(1_793_966_400_000); // 2026-11-06 12:00:00 UTC
        let ports = Ports { storage: storage.clone(), ..ports };
        (MapVm::on_sample(ports, crate::i18n_for(locale)), pdf, downloads, storage)
    }

    fn texts(page: &[Block]) -> Vec<String> {
        page.iter()
            .flat_map(|b| match b {
                Block::Heading { text, .. } | Block::Paragraph { text } => vec![text.clone()],
                Block::Table { caption, head, rows } => {
                    let mut v = vec![caption.clone()];
                    v.extend(head.clone());
                    v.extend(rows.iter().flatten().cloned());
                    v
                }
                Block::Figures { items } => items.iter().flat_map(|c| [c.caption.clone(), c.note.clone()]).collect(),
            })
            .collect()
    }

    fn texts_of(pages: &[Vec<Block>]) -> String {
        pages.iter().flat_map(|p| texts(p)).collect::<Vec<_>>().join("\n")
    }

    fn table(data: &ReportData) -> Vec<Vec<String>> {
        data.pages[1].iter().find_map(|b| if let Block::Table { rows, .. } = b { Some(rows.clone()) } else { None }).expect("the summary has a table")
    }

    #[test]
    fn the_report_has_a_cover_a_summary_and_the_sources_pages() {
        let (vm, ..) = sample_in(Locale::En);
        let data = vm.report_data();
        assert_eq!(data.lang, "en");
        let cover = texts(&data.pages[0]);
        assert!(matches!(&data.pages[0][0], Block::Heading { level: 1, text } if *text == vm.title()));
        let joined = cover.join("\n");
        assert!(joined.contains("Design review report"), "{joined}");
        assert!(joined.contains("2026-11-06 12:00 UTC"), "{joined}");
        assert!(joined.contains(env!("CARGO_PKG_VERSION")), "{joined}");
        assert!(joined.contains("Whole city"), "{joined}");
        assert!(joined.contains("design study"), "{joined}");
        assert!(joined.contains("placeholders"), "{joined}");
    }

    #[test]
    fn the_summary_counts_come_from_the_city_view() {
        let (vm, ..) = sample_in(Locale::En);
        let v = vm.view();
        let rows = table(&vm.report_data());
        assert_eq!(rows.len(), 5);
        let junctions = v.nodes.iter().filter(|n| n.junction).count();
        assert!(rows[0][1].contains(&v.edges.len().to_string()) && rows[0][1].contains(&junctions.to_string()), "{:?}", rows[0]);
        assert_eq!(rows[1][1], v.edited.to_string());
        assert_eq!(rows[2][1], v.failing.to_string());
    }

    /// The network effects with the area rules, the limits of the data and the appendix, after the changed places.
    const REVIEW_PAGES: usize = 3;

    fn headings(data: &ReportData) -> Vec<String> {
        data.pages.iter().flatten().filter_map(|b| if let Block::Heading { level: 2, text } = b { Some(text.clone()) } else { None }).collect()
    }

    #[test]
    fn the_report_ends_with_the_network_the_areas_the_data_limits_and_the_appendix() {
        for (locale, last) in [
            (Locale::En, ["Network effects", "Area rules", "Limits of the OpenStreetMap data", "Appendix"]),
            (Locale::FrCa, ["Effets sur le réseau", "Règles de zone", "Limites des données OpenStreetMap", "Annexe"]),
        ] {
            let (vm, ..) = sample_in(locale);
            let mut data = vm.report_data();
            data.pages = without_sources(data.pages);
            assert_eq!(data.pages.len(), 2 + REVIEW_PAGES);
            let heads = headings(&data);
            assert_eq!(&heads[heads.len() - 4..], &last.map(String::from), "{heads:?}");
        }
    }

    #[test]
    fn the_review_pages_come_after_the_changed_places() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        let street = vm.view().edges.iter().find(|e| !e.freeway).unwrap().uid;
        widen_street(&storage, street, 1000);
        vm.reload();
        let mut data = vm.report_data();
        data.pages = without_sources(data.pages);
        assert_eq!(data.pages.len(), 3 + REVIEW_PAGES);
        assert!(matches!(&data.pages[2][0], Block::Heading { level: 2, text } if text != "Network effects"), "{:?}", data.pages[2][0]);
        assert_eq!(headings(&data).iter().filter(|h| h.as_str() == "Network effects").count(), 1);
    }

    #[test]
    fn a_city_with_no_places_has_no_review_pages_and_no_appendix() {
        let (ports, ..) = test_ports_with_pdf();
        let vm = MapVm::new(ports, crate::i18n_for(Locale::En));
        assert_eq!(vm.report_data().pages.len(), 2);
    }

    #[test]
    fn a_declared_area_and_its_streets_are_in_the_report() {
        let vm = ready_areas_vm(Locale::En, &[("Old town", Rule::CarFree)]);
        let data = vm.report_data();
        let areas = data.pages.iter().find(|p| p.iter().any(|b| matches!(b, Block::Heading { text, .. } if text == "Area rules"))).expect("an Area rules page");
        let all = texts(areas).join("\n");
        assert!(all.contains("Old town"), "{all}");
        assert!(!all.contains("No area rules"), "{all}");
    }

    #[test]
    fn the_network_findings_of_the_city_are_in_the_report() {
        let (vm, ..) = sample_in(Locale::En);
        let found = vm.network_rules().iter().map(|r| r.count).sum::<usize>();
        let page = vm.report_data().pages.into_iter().find(|p| matches!(&p[0], Block::Heading { text, .. } if text == "Network effects")).unwrap();
        let (_, _, rows) = page.iter().find_map(|b| if let Block::Table { caption, head, rows } = b { Some((caption, head, rows)) } else { None }).unwrap();
        assert_eq!(rows.len(), 6);
        let listed: usize = rows.iter().map(|r| r[1].split(' ').next().unwrap().parse::<usize>().unwrap_or(0)).sum();
        assert_eq!(listed, found, "{rows:?}");
    }

    #[test]
    fn pass_and_fail_are_words() {
        let (vm, ..) = sample_in(Locale::En);
        let rows = table(&vm.report_data());
        assert_eq!(rows[2][2], "Pass");
        assert_eq!(rows[3][2], "Pass");
        assert_eq!(rows[4][2], "Pass");
    }

    #[test]
    fn a_failing_place_is_a_fail() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        let street = vm.view().edges.iter().find(|e| !e.freeway).unwrap().uid;
        let store = CityStore::sample(storage);
        let mut e = store.open().street_editor(street, 0).unwrap();
        let u = e.view().segments[0].uid;
        e.set_width(u, e.view().segments[0].width_mm + 8000);
        assert!(store.write_street(street, e.snapshot()));
        vm.reload();
        assert!(vm.view().failing > 0, "the edit must break a check");
        let rows = table(&vm.report_data());
        assert_eq!((rows[1][1].as_str(), rows[2][1].as_str(), rows[2][2].as_str()), ("1", vm.view().failing.to_string().as_str(), "Fail"));
    }

    #[test]
    fn the_sources_page_lists_each_cited_rule_with_its_references_and_says_the_rest_are_placeholders() {
        let (vm, ..) = sample_in(Locale::En);
        let page = texts_of(&vm.report_data().pages[2..]);
        assert!(page.contains("Room for the transition") && page.contains("Lane drops"), "{page}");
        assert!(page.contains("Regional Municipality of York") && page.contains("Ministère des Transports du Québec"), "{page}");
        assert!(page.contains("placeholder"), "{page}");
        let (fr, ..) = sample_in(Locale::FrCa);
        let page = texts_of(&fr.report_data().pages[2..]);
        assert!(page.contains("Place pour la transition") && page.contains("provisoire"), "{page}");
    }

    #[test]
    fn the_report_follows_the_interface_language() {
        let (en, ..) = sample_in(Locale::En);
        let (fr, ..) = sample_in(Locale::FrCa);
        let (en, fr) = (en.report_data(), fr.report_data());
        assert_eq!((en.lang.as_str(), fr.lang.as_str()), ("en", "fr-CA"));
        assert_ne!(texts(&en.pages[0]), texts(&fr.pages[0]));
        assert_ne!(en.title, fr.title);
        assert!(texts(&fr.pages[0]).join("\n").contains("Ville entière"));
    }

    #[test]
    fn a_city_with_no_places_says_there_is_nothing_to_report() {
        let (ports, ..) = test_ports_with_pdf();
        let vm = MapVm::new(ports, crate::i18n_for(Locale::En)); // the default area, whose roads are not kept
        let data = vm.report_data();
        assert!(!data.pages[1].iter().any(|b| matches!(b, Block::Table { .. })));
        assert!(texts(&data.pages[1]).join("\n").contains("There is nothing to report"));
    }

    #[test]
    fn no_problems_and_no_notices_says_none_found_with_pass() {
        let (vm, ..) = sample_in(Locale::En);
        let page = texts(&vm.report_data().pages[1]).join("\n");
        assert!(page.contains("No network problems and no notices were found: Pass"), "{page}");
    }

    #[test]
    fn a_long_area_name_is_kept_whole() {
        let name = "A very long area name that goes on and on past any sensible width, Saint-Jean-Baptiste-de-la-Salle-des-Neiges, Québec, Canada";
        let built =
            crate::map::report::build(&crate::map::report::Facts { area: name.to_string(), ..Default::default() }, "1.0.0", 0, &crate::i18n_for(Locale::En));
        assert!(matches!(&built.pages[0][0], Block::Heading { level: 1, text } if text == name));
    }

    // ---- the pages of the changed places ----

    /// A street of the sample city with its first piece made `by` wider, written as another page would.
    fn widen_street(storage: &Rc<MemoryStorage>, uid: u32, by: i32) {
        let store = CityStore::sample(storage.clone());
        let mut e = store.open().street_editor(uid, 0).unwrap();
        let u = e.view().segments[0].uid;
        e.set_width(u, e.view().segments[0].width_mm + by);
        assert!(store.write_street(uid, e.snapshot()));
    }

    fn change_junction(storage: &Rc<MemoryStorage>, uid: u32) {
        let store = CityStore::sample(storage.clone());
        let mut j = store.open().junction_editor(uid, 0).unwrap();
        let next = (j.snapshot().control + 1) % crate::junction::model::CONTROLS.len();
        assert!(j.set_control(next));
        assert!(store.write_junction(uid, j.snapshot()));
    }

    fn a_street(vm: &MapVm) -> u32 {
        vm.view().edges.iter().find(|e| !e.freeway).unwrap().uid
    }

    /// The pages of the changed places: after the cover and the summary, before the review pages and the appendix.
    fn without_sources(pages: Vec<Vec<Block>>) -> Vec<Vec<Block>> {
        pages.into_iter().filter(|p| !matches!(p.first(), Some(Block::Heading { level: 2, text }) if text.starts_with("Sources"))).collect()
    }

    fn pages(vm: &MapVm) -> Vec<Vec<Block>> {
        let all = without_sources(vm.report_data().pages);
        let end = all.len().saturating_sub(REVIEW_PAGES).max(2);
        all[2..end].to_vec()
    }

    fn figures(page: &[Block]) -> Vec<&cityloom_report::Captioned> {
        page.iter().flat_map(|b| if let Block::Figures { items } = b { items.iter().collect() } else { Vec::new() }).collect()
    }

    fn tables(page: &[Block]) -> Vec<(&Vec<String>, &Vec<Vec<String>>)> {
        page.iter().filter_map(|b| if let Block::Table { head, rows, .. } = b { Some((head, rows)) } else { None }).collect()
    }

    #[test]
    fn one_page_per_edited_place_and_none_for_unedited() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        assert!(pages(&vm).is_empty());
        let street = a_street(&vm);
        widen_street(&storage, street, 300);
        vm.reload();
        assert_eq!(pages(&vm).len(), 1);
        let junction = vm.view().nodes.iter().find(|n| n.junction).unwrap().uid;
        change_junction(&storage, junction);
        vm.reload();
        let p = pages(&vm);
        assert_eq!(p.len(), vm.view().edited);
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn no_edit_gives_no_place_pages() {
        let (vm, ..) = sample_in(Locale::En);
        assert_eq!(without_sources(vm.report_data().pages).len(), 2 + REVIEW_PAGES);
    }

    #[test]
    fn the_street_page_has_before_and_after_figures_with_captions() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        widen_street(&storage, a_street(&vm), 300);
        vm.reload();
        let page = &pages(&vm)[0];
        assert!(matches!(&page[0], Block::Heading { .. }));
        let f = figures(page);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].caption.as_str(), f[1].caption.as_str()), ("Before", "After"));
        assert!(f.iter().all(|c| !c.alt.is_empty() && !c.figure.items.is_empty()));
        assert_ne!(f[0].figure, f[1].figure);
        assert!(texts(page).join("\n").contains("Width:"), "{:?}", texts(page));
    }

    #[test]
    fn the_width_table_gives_before_and_after_shares() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        widen_street(&storage, a_street(&vm), 300);
        vm.reload();
        let page = &pages(&vm)[0];
        let (head, rows) = tables(page)[0];
        assert_eq!(head.as_slice(), ["Mode", "Before", "After"]);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0][0], "Walking");
        assert!(rows.iter().all(|r| r[1].contains('%') && r[2].contains('%')));
        assert!(rows.iter().any(|r| r[1] != r[2]), "{rows:?}");
    }

    #[test]
    fn a_failing_check_is_a_fail_row_and_a_passing_place_says_all_pass() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        let street = a_street(&vm);
        widen_street(&storage, street, 8000);
        vm.reload();
        assert!(vm.view().failing > 0);
        let page = &pages(&vm)[0];
        let (head, fails) = tables(page).into_iter().find(|(h, _)| h[0] == "Check").expect("a checks table");
        assert_eq!(head.as_slice(), ["Check", "Before", "After", "Detail"]);
        assert!(fails.iter().any(|r| r[1] == "Pass" && r[2] == "Fail" && !r[3].is_empty()), "an edit that breaks a check: {fails:?}");
        assert!(fails.iter().all(|r| r[1] == "Fail" || r[2] == "Fail"), "{fails:?}");
        assert!(texts(page).join("\n").contains("Before is the layout as imported"));
        let (vm, _, _, storage) = sample_in(Locale::En);
        let store = CityStore::sample(storage.clone());
        let street = a_street(&vm);
        let mut e = store.open().street_editor(street, 0).unwrap();
        assert!(e.set_speed(Some(30)));
        assert!(store.write_street(street, e.snapshot()));
        vm.reload();
        assert_eq!(vm.view().failing, 0);
        let page = &pages(&vm)[0];
        assert!(!tables(page).iter().any(|(h, _)| h[0] == "Check"));
        assert!(texts(page).join("\n").contains("pass: Pass."), "{:?}", texts(page));
    }

    fn pairs<'a>(said: &'a crate::shared::said::Said, states: &[(Option<bool>, Option<bool>)]) -> Vec<crate::map::report::CheckPair<'a>> {
        states.iter().map(|&(before, after)| crate::map::report::CheckPair { id: "x", before, after, label: said, detail: said }).collect()
    }

    #[test]
    fn more_than_eight_failing_checks_are_cut_with_a_count() {
        let said = crate::shared::said::Said::new("map-report-pass");
        let checks = pairs(&said, &[(Some(true), Some(false)); 11]);
        let blocks = crate::map::report::checks_block(&checks, &crate::i18n_for(Locale::En), crate::shared::units::Units::Metres);
        let (_, rows) = tables(&blocks)[0];
        assert_eq!(rows.len(), 9);
        assert_eq!(rows[8][0], "3 more failing checks");
        assert_eq!(rows[..8].iter().filter(|r| r[1] == "Pass" && r[2] == "Fail").count(), 8);
    }

    #[test]
    fn the_checks_table_puts_before_beside_after_and_lists_a_check_failing_in_either() {
        let said = crate::shared::said::Said::new("map-report-pass");
        let checks =
            pairs(&said, &[(Some(false), Some(true)), (Some(true), Some(false)), (Some(false), Some(false)), (Some(true), Some(true)), (Some(false), None)]);
        let blocks = crate::map::report::checks_block(&checks, &crate::i18n_for(Locale::En), crate::shared::units::Units::Metres);
        let (_, rows) = tables(&blocks)[0];
        let states: Vec<(&str, &str)> = rows.iter().map(|r| (r[1].as_str(), r[2].as_str())).collect();
        // those failing now first, then those that only failed before; the passing one is not listed
        assert_eq!(states, [("Pass", "Fail"), ("Fail", "Fail"), ("Fail", "Pass"), ("Fail", "n/a")]);
        assert!(!rows[0][3].is_empty() && !rows[1][3].is_empty(), "a failing After result has its detail");
        assert!(rows[2][3].is_empty() && rows[3][3].is_empty());
    }

    #[test]
    fn the_plural_messages_read_right_in_french_for_zero_one_and_many() {
        let fr = crate::i18n_for(Locale::FrCa);
        let line =
            |added: i64, removed: i64| fr.tr_now("map-report-place-pieces", &crate::shared::i18n::Args::new().num("added", added).num("removed", removed));
        assert_eq!(line(2, 0), "Éléments\u{a0}: 2 ajoutés, 0 retiré.");
        assert_eq!(line(0, 1), "Éléments\u{a0}: 0 ajouté, 1 retiré.");
        assert_eq!(line(1, 1), "Éléments\u{a0}: 1 ajouté, 1 retiré.");
        let pass = |n: i64| fr.tr_now("map-report-place-all-pass", &crate::shared::i18n::Args::new().num("n", n).str("result", "Réussite"));
        assert_eq!(pass(0), "0 vérification réussie\u{a0}: Réussite.");
        assert_eq!(pass(1), "1 vérification réussie\u{a0}: Réussite.");
        assert_eq!(pass(5), "5 vérifications réussies\u{a0}: Réussite.");
    }

    #[test]
    fn a_junction_that_cannot_be_drawn_gets_a_text_page() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        let junction = vm.view().nodes.iter().find(|n| n.junction).unwrap().uid;
        change_junction(&storage, junction);
        let store = CityStore::sample(storage.clone());
        let city = store.open();
        let mut view = city.view(0);
        let plain = view.nodes.iter().position(|n| !n.junction).expect("a node that is no junction");
        view.nodes[plain].junction = true;
        view.nodes[plain].edited = true;
        let (pages, left) = crate::map::report::place_pages(&city, &view, 0, &vm.i18n(), vm.units());
        assert_eq!(left, 0);
        let undrawn = pages.iter().find(|p| texts(p).join("\n").contains("could not be drawn")).expect("a text page");
        assert!(matches!(&undrawn[0], Block::Heading { .. }));
        assert!(figures(undrawn).is_empty() && tables(undrawn).is_empty());
        assert!(pages.iter().any(|p| !figures(p).is_empty()), "the junction that can be drawn has figures");
    }

    #[test]
    fn a_junction_page_says_the_control_before_and_after() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        change_junction(&storage, vm.view().nodes.iter().find(|n| n.junction).unwrap().uid);
        vm.reload();
        let page = &pages(&vm)[0];
        assert_eq!(figures(page).len(), 2);
        assert!(texts(page).join("\n").contains("Control:"), "{:?}", texts(page));
    }

    /// The page with its checks replaced by the worst case: `MAX_FAILING_ROWS` plus three failing checks, each with a long
    /// detail, so that the table is as long as a page's table gets.
    fn with_worst_checks(mut page: Vec<Block>, i18n: &crate::shared::i18n::I18n) -> Vec<Block> {
        use crate::shared::said::{Arg, Said};
        let keep = page.iter().rposition(|b| matches!(b, Block::Figures { .. }) || matches!(b, Block::Table { head, .. } if head.len() == 3)).unwrap();
        page.truncate(keep + 1);
        let long = |n: usize| {
            Said::new("city-name").with("name", Arg::Text(format!("Détail {n}\u{a0}: ").to_string() + &"un texte très long qui continue ".repeat(8)))
        };
        let (label, detail): (Vec<Said>, Vec<Said>) = (0..crate::map::report::MAX_FAILING_ROWS + 3).map(|n| (long(n), long(n))).unzip();
        let checks: Vec<_> = label
            .iter()
            .zip(&detail)
            .map(|(l, d)| crate::map::report::CheckPair { id: "x", before: Some(true), after: Some(false), label: l, detail: d })
            .collect();
        page.extend(crate::map::report::checks_block(&checks, i18n, crate::shared::units::Units::Metres));
        page
    }

    fn written_text(pages: Vec<Vec<Block>>, lang: &str) -> String {
        let data = ReportData { lang: lang.to_string(), title: "Rapport".to_string(), pages };
        let bytes = cityloom_report_pdf::write_pdf(
            &data,
            include_bytes!("../../../../web/fonts/NotoSans-Regular.ttf"),
            include_bytes!("../../../../web/fonts/NotoSans-Bold.ttf"),
        )
        .expect("a place page is written whole: the writer fails on a line it would cut");
        let doc = lopdf::Document::load_mem(&bytes).unwrap();
        let numbers: Vec<u32> = doc.get_pages().keys().copied().collect();
        doc.extract_text(&numbers).unwrap()
    }

    #[test]
    fn the_whole_report_of_the_sample_city_is_written_in_both_languages() {
        for (locale, tag, heading) in [(Locale::En, "en", "Appendix"), (Locale::FrCa, "fr-CA", "Annexe")] {
            let (vm, ..) = sample_in(locale);
            let text = written_text(vm.report_data().pages, tag);
            assert!(text.contains(heading), "{text}");
        }
    }

    #[test]
    fn review_pages_full_of_findings_are_written_whole_in_both_languages() {
        use crate::map::network::{Check, NetworkFinding, NetworkReport, Place, PlaceKind, Severity};
        use crate::map::source_quality::{Kind, SourceFinding, SourceReport};
        use crate::map::vm::AreaStreet;
        use crate::restriction::model::{Conflict, Restriction};
        use crate::shared::said::Said;
        for (locale, tag) in [(Locale::En, "en"), (Locale::FrCa, "fr-CA")] {
            let i18n = crate::i18n_for(locale);
            let place = |uid| Place { kind: PlaceKind::Street, uid, name: Said::new("city-unnamed") };
            let network = NetworkReport {
                findings: Check::ALL
                    .iter()
                    .flat_map(|&rule| {
                        (0..crate::map::report_review::MAX_ROWS + 3).map(move |_| NetworkFinding {
                            rule,
                            severity: Severity::Problem,
                            said: Said::new("net-crossing-missing"),
                            places: (0..6).map(place).collect(),
                        })
                    })
                    .collect(),
                unreadable: 7,
            };
            let source = SourceReport {
                findings: Kind::ALL
                    .iter()
                    .flat_map(|&kind| {
                        (0..crate::map::report_review::MAX_ROWS + 3).map(move |_| SourceFinding {
                            kind,
                            said: Said::new("prov-ends").with("street", crate::shared::said::Arg::Text("Rue Très Longue-Saint-Jean-Baptiste".into())),
                            source: Said::new("prov-source-width"),
                            impact: Said::new("prov-impact-width"),
                            places: vec![place(1)],
                        })
                    })
                    .collect(),
            };
            let rule = Rule::SpeedLimit { kmh: 30 };
            let areas: Vec<_> = (0..5)
                .map(|i| {
                    let streets = (0..crate::map::report_review::MAX_AREA_STREETS as u32 + 4)
                        .map(|u| AreaStreet {
                            area: format!("r{i}"),
                            uid: u,
                            href: String::new(),
                            name: Said::new("city-unnamed"),
                            rule,
                            conflicts: vec![Conflict::Speed { kmh: 60 }],
                            speed_unknown: false,
                        })
                        .collect();
                    (Restriction { id: format!("r{i}"), name: "Zone de la très longue rue Saint-Jean-Baptiste".into(), rule, ring: vec![] }, streets)
                })
                .collect();
            let units = crate::shared::units::Units::Metres;
            let pages = vec![
                [crate::map::report_review::network_page(&network, &i18n, units), crate::map::report_review::area_page(&areas, &i18n, units)].concat(),
                crate::map::report_review::limits_page(&source, &i18n, units),
                crate::map::report_review::appendix(&i18n),
            ];
            let text = written_text(pages, tag);
            assert!(!text.is_empty());
        }
    }

    #[test]
    fn the_worst_street_page_and_junction_page_are_written_whole_in_fr_ca() {
        let (vm, _, _, storage) = sample_in(Locale::FrCa);
        let street = a_street(&vm);
        widen_street(&storage, street, 8000);
        change_junction(&storage, vm.view().nodes.iter().find(|n| n.junction).unwrap().uid);
        let city = CityStore::sample(storage.clone()).open();
        let mut view = city.view(0);
        view.edges.iter_mut().find(|e| e.uid == street).unwrap().zones = 3;
        let (pages, _) = crate::map::report::place_pages(&city, &view, 0, &vm.i18n(), vm.units());
        assert_eq!(pages.len(), 2);
        let i18n = vm.i18n();
        let worst: Vec<Vec<Block>> = pages.into_iter().map(|p| with_worst_checks(p, &i18n)).collect();
        for page in worst {
            let text = written_text(vec![page], "fr-CA");
            assert!(text.contains("Détail 0"), "{text}");
            assert!(text.contains("Détail 7"), "the eighth row is kept");
            assert!(text.contains("3 autres vérifications"), "the count of the rest is kept: {text}");
        }
    }

    #[test]
    fn more_than_forty_edited_places_give_forty_pages_and_a_note() {
        assert_eq!(crate::map::report::MAX_PLACES, 40);
        let (vm, _, _, storage) = sample_in(Locale::En);
        let streets: Vec<u32> = vm.view().edges.iter().take(3).map(|e| e.uid).collect();
        for s in &streets {
            widen_street(&storage, *s, 100);
        }
        vm.reload();
        let city = CityStore::sample(storage.clone()).open();
        let view = city.view(0);
        let (pages, left) = crate::map::report::place_pages_up_to(2, &city, &view, 0, &vm.i18n(), vm.units());
        assert_eq!((pages.len(), left), (2, view.edited - 2));
        let facts = crate::map::report::Facts { places: 5, edited: 3, place_pages: pages, left_out: left, ..Default::default() };
        let data = crate::map::report::build(&facts, "1.0.0", 0, &crate::i18n_for(Locale::En));
        // The facts carry no review pages, so only the appendix follows the two changed places.
        assert_eq!(without_sources(data.pages.clone()).len(), 4 + 1);
        assert!(texts(&data.pages[1]).join("\n").contains("not shown"), "{:?}", texts(&data.pages[1]));
    }

    #[test]
    fn a_zoned_street_says_it_is_drawn_at_its_first_zone() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        let street = a_street(&vm);
        widen_street(&storage, street, 300);
        let city = CityStore::sample(storage.clone()).open();
        let mut view = city.view(0);
        view.edges.iter_mut().find(|e| e.uid == street).unwrap().zones = 3;
        let (pages, _) = crate::map::report::place_pages(&city, &view, 0, &vm.i18n(), vm.units());
        assert!(texts(&pages[0]).join("\n").contains("drawn at its first zone"));
        let (pages, _) = crate::map::report::place_pages(&city, &city.view(0), 0, &vm.i18n(), vm.units());
        assert!(!texts(&pages[0]).join("\n").contains("first zone"));
    }

    #[test]
    fn a_long_street_name_is_kept_whole_in_the_heading() {
        let (vm, _, _, storage) = sample_in(Locale::En);
        let street = a_street(&vm);
        widen_street(&storage, street, 300);
        let city = CityStore::sample(storage.clone()).open();
        let mut view = city.view(0);
        let long = "Avenue du Très Long Nom de Rue Qui Continue Encore et Encore Jusqu’à Dépasser La Largeur De La Page";
        view.edges.iter_mut().find(|e| e.uid == street).unwrap().called =
            crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text(long.to_string()));
        let (pages, _) = crate::map::report::place_pages(&city, &view, 0, &vm.i18n(), vm.units());
        assert!(matches!(&pages[0][0], Block::Heading { text, .. } if text.contains(long)), "{:?}", pages[0][0]);
    }

    #[test]
    fn the_pages_are_in_fr_ca_when_the_page_is_french() {
        let (vm, _, _, storage) = sample_in(Locale::FrCa);
        widen_street(&storage, a_street(&vm), 300);
        change_junction(&storage, vm.view().nodes.iter().find(|n| n.junction).unwrap().uid);
        vm.reload();
        let p = pages(&vm);
        let all: String = p.iter().flat_map(|pg| texts(pg)).collect::<Vec<_>>().join("\n");
        assert!(all.contains("Avant") && all.contains("Après") && all.contains("Largeur") && all.contains("Signalisation"), "{all}");
        assert!(!all.contains("Before") && !all.contains("Width"), "{all}");
        let f = figures(&p[1]);
        assert!(f.iter().all(|c| c.alt.contains("Avant") || c.alt.contains("Après")));
    }

    #[test]
    fn download_report_saves_the_pdf_it_is_given() {
        let (vm, pdf, downloads, _) = sample_in(Locale::En);
        vm.download_report();
        let json = pdf.asked().remove(0);
        assert_eq!(serde_json::from_str::<ReportData>(&json).unwrap(), vm.report_data());
        assert!(pdf.answer_with(Ok(b"%PDF".to_vec())));
        let saved = downloads.saved();
        assert_eq!(saved.len(), 1);
        assert_eq!((saved[0].1.as_str(), saved[0].2.as_slice()), ("application/pdf", &b"%PDF"[..]));
        assert!(saved[0].0.starts_with("cityloom-") && saved[0].0.ends_with(".pdf") && !saved[0].0.contains(' '), "{}", saved[0].0);
    }

    #[test]
    fn download_report_says_when_the_writer_fails() {
        let (ports, pdf, downloads) = test_ports_with_pdf();
        let said = Rc::new(crate::shared::ports::RecordingAnnouncer::default());
        let ports = Ports { announcer: said.clone(), ..ports };
        let vm = MapVm::on_sample(ports, crate::i18n_for(Locale::En));
        vm.download_report();
        pdf.answer_with(Err("boom".to_string()));
        assert!(downloads.saved().is_empty());
        let heard = said.take();
        assert_eq!(heard.len(), 1);
        assert!(heard[0].contains("could not be made"), "{heard:?}");
        // and the next try is allowed
        vm.download_report();
        assert_eq!(pdf.asked().len(), 1);
    }

    #[test]
    fn download_report_says_when_the_file_cannot_be_saved() {
        let (ports, pdf, downloads) = test_ports_with_pdf();
        let said = Rc::new(crate::shared::ports::RecordingAnnouncer::default());
        let ports = Ports { announcer: said.clone(), ..ports };
        let vm = MapVm::on_sample(ports, crate::i18n_for(Locale::En));
        downloads.fail();
        vm.download_report();
        pdf.answer_with(Ok(b"%PDF".to_vec()));
        let heard = said.take();
        assert_eq!(heard.len(), 1);
        assert!(heard[0].contains("could not be made"), "{heard:?}");
    }

    #[test]
    fn a_failed_report_shows_a_notice_that_the_next_attempt_clears() {
        let (ports, pdf, _downloads) = test_ports_with_pdf();
        let vm = MapVm::on_sample(ports, crate::i18n_for(Locale::En));
        assert!(!vm.report_failed());
        vm.download_report();
        assert!(!vm.report_failed());
        pdf.answer_with(Err("boom".to_string()));
        assert!(vm.report_failed());
        vm.download_report();
        assert!(!vm.report_failed(), "a new attempt starts with no notice");
        pdf.answer_with(Ok(b"%PDF".to_vec()));
        assert!(!vm.report_failed());
    }

    #[test]
    fn a_report_that_cannot_be_saved_shows_the_notice_and_dismissing_clears_it() {
        let (ports, pdf, downloads) = test_ports_with_pdf();
        let vm = MapVm::on_sample(ports, crate::i18n_for(Locale::En));
        downloads.fail();
        vm.download_report();
        pdf.answer_with(Ok(b"%PDF".to_vec()));
        assert!(vm.report_failed());
        vm.dismiss_report_notice();
        assert!(!vm.report_failed());
    }

    #[test]
    fn the_notice_is_drawn_as_visible_text_with_a_named_close_button_and_follows_the_language() {
        let (ports, pdf, _d) = test_ports_with_pdf();
        let vm = MapVm::on_sample(ports, crate::i18n_for(Locale::En));
        let none = map_html(|| view! { <map_ui::ReportNotice vm=vm.clone()/> });
        assert!(!none.contains("could not be made"), "{none}");
        vm.download_report();
        pdf.answer_with(Err("boom".to_string()));
        let h = map_html(|| view! { <map_ui::ReportNotice vm=vm.clone()/> });
        assert!(h.contains("role=\"status\"") && h.contains("The report could not be made. Try again."), "{h}");
        assert!(h.contains("<button") && h.contains("aria-label=\"Dismiss\""), "{h}");
        vm.i18n().set(Locale::FrCa);
        let h = map_html(|| view! { <map_ui::ReportNotice vm=vm.clone()/> });
        assert!(h.contains("n’a pas pu être produit") && h.contains("aria-label=\"Fermer\""), "{h}");
    }

    #[test]
    fn the_french_cover_gives_the_export_date_and_time_in_the_french_way() {
        let (vm, ..) = sample_in(Locale::FrCa);
        let joined = texts(&vm.report_data().pages[0]).join("\n");
        assert!(
            joined.contains("Date d’exportation\u{202f}: 2026-11-06 12\u{a0}h\u{a0}00 UTC")
                || joined.contains("Date d’exportation\u{a0}: 2026-11-06 12\u{a0}h\u{a0}00 UTC"),
            "{joined}"
        );
    }

    #[test]
    fn a_second_download_while_one_runs_saves_one_file() {
        let (vm, pdf, downloads, _) = sample_in(Locale::En);
        vm.download_report();
        vm.download_report();
        assert_eq!(pdf.asked().len(), 1);
        pdf.answer_with(Ok(vec![1]));
        assert_eq!(downloads.saved().len(), 1);
    }

    fn facts() -> crate::map::report::Facts {
        crate::map::report::Facts { area: "Bendville".into(), streets: 2, junctions: 1, places: 3, problems: 2, notices: 3, ..Default::default() }
    }

    fn built(facts: &crate::map::report::Facts, locale: Locale) -> ReportData {
        crate::map::report::build(facts, "1.0.0", 0, &crate::i18n_for(locale))
    }

    #[test]
    fn problems_fail_notices_notice_and_the_none_found_line_is_absent() {
        let en = built(&facts(), Locale::En);
        let rows = table(&en);
        assert_eq!((rows[3][2].as_str(), rows[4][2].as_str()), ("Fail", "Notice"));
        assert_eq!((rows[3][1].as_str(), rows[4][1].as_str()), ("2", "3"));
        assert!(!texts(&en.pages[1]).join("\n").contains("were found"));
        let fr = built(&facts(), Locale::FrCa);
        let rows = table(&fr);
        assert_eq!((rows[3][2].as_str(), rows[4][2].as_str()), ("Échec", "À noter"));
    }

    #[test]
    fn the_city_figure_uses_singular_and_plural() {
        let one = crate::map::report::Facts { streets: 1, junctions: 1, ..facts() };
        assert_eq!(table(&built(&one, Locale::En))[0][1], "1 street, 1 junction");
        assert_eq!(table(&built(&facts(), Locale::En))[0][1], "2 streets, 1 junction");
    }
}
