//! The map page drawn to HTML on the host.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::testing::{button_tag, count, html};
// ---- the city map ---------------------------------------------------------------------------

use crate::city::store::CityStore;
use crate::map::view as map_ui;
use crate::map::vm::MapVm;
use crate::shared::ports::{MemoryStorage, test_ports};

fn map_vm() -> (Rc<MapVm>, Rc<MemoryStorage>) {
    let (ports, _, storage) = test_ports();
    (MapVm::on_sample(ports), storage)
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
    let vm = MapVm::new(ports); // the default area, whose roads are not kept: so no map
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert!(h.contains("role=\"status\"") && h.contains("The roads of this place could not be loaded"), "{h}");
    assert!(!h.contains("hidden"), "{h}");
}

#[test]
fn a_map_page_whose_area_s_roads_were_not_had_lists_nothing_and_names_the_area() {
    let (ports, _, _) = test_ports(); // the default area, whose roads are not kept
    let vm = MapVm::new(ports);
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
