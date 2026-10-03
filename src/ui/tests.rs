//! The map page drawn to HTML on the host.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::testing::{button_tag, count, html};
use crate::shared::units::Units;
// ---- the city map ---------------------------------------------------------------------------

use crate::ui::map as map_ui;
use crate::city::store::CityStore;
use crate::vm::map::MapVm;
use crate::shared::ports::{MemoryStorage, test_ports};

fn map_vm() -> (Rc<MapVm>, Rc<MemoryStorage>) {
    let (ports, _, storage) = test_ports();
    (MapVm::new(ports), storage)
}

fn map_html<V: IntoView + 'static>(f: impl FnOnce() -> V) -> String {
    html(|| f().into_any())
}

/// Another page changes a street and the map reads the city again.
fn change_a_street(vm: &MapVm, storage: &Rc<MemoryStorage>, by_mm: i32) {
    let street = vm.view().edges.iter().find(|e| !e.freeway).unwrap().uid;
    assert!(CityStore::new(storage.clone()).write(|c| {
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
    assert!(h.contains("aria-label=\"Zoom out\"") && h.contains("Whole city") && h.contains("Press a junction to open its plan."));
}

#[test]
fn the_map_is_a_labelled_group_holding_every_place_as_a_link_and_a_scale_bar() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert!(h.contains("id=\"map-view\"") && h.contains("tabindex=\"0\"") && h.contains("aria-label=\"Map of the city, north up\""));
    assert!(h.contains("viewBox=\""));
    assert_eq!(count(&h, "class=\"place\""), 9 + 23);
    assert!(h.contains("class=\"sb-ink\"") && h.contains("class=\"north\""));
    assert!(!h.contains("m-hl"), "nothing is hot");
    assert!(!h.contains("panning"));
}

#[test]
fn the_place_the_pointer_is_on_is_outlined_on_the_map() {
    let (vm, _) = map_vm();
    let e = vm.view().edges[0].uid;
    vm.set_hot(Some(format!("s-{e}")));
    let h = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert_eq!(count(&h, "class=\"m-hl on\""), 1);
    assert!(h.contains(&format!("id=\"hl-s-{e}\"")));
}

#[test]
fn the_map_follows_the_zoom_and_the_units() {
    let (vm, _) = map_vm();
    let before = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    vm.zoom_in();
    let after = map_html(|| view! { <map_ui::MapView vm=vm.clone()/> });
    assert_ne!(before, after);
    vm.set_units(Units::Feet);
    assert!(map_html(|| view! { <map_ui::MapView vm=vm.clone()/> }).contains(" ft</text>"));
}

#[test]
fn the_key_lists_what_the_streets_are_made_of() {
    let (vm, _) = map_vm();
    let h = map_html(|| view! { <map_ui::Legend vm=vm.clone()/> });
    assert_eq!(count(&h, "<li>"), vm.legend().len());
    assert!(h.contains("Sidewalk") && h.contains("url(#h-sidewalk)"));
}

#[test]
fn the_map_s_status_line_says_whether_every_place_works() {
    let (vm, storage) = map_vm();
    let h = map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    assert!(h.contains("class=\"fit\"") && h.contains("32 places. Every check passes."));
    change_a_street(&vm, &storage, 1_000);
    let h = map_html(|| view! { <map_ui::Status vm=vm.clone()/> });
    assert!(h.contains("class=\"fit bad\"") && h.contains("1 place needs attention: "));
}

#[test]
fn the_places_panel_lists_each_junction_and_street_as_a_link_with_what_is_wrong() {
    let (vm, storage) = map_vm();
    let h = map_html(|| view! { <map_ui::Places vm=vm.clone()/> });
    assert_eq!(count(&h, "class=\"place-row\""), 32);
    assert!(h.contains("href=\"intersection.html?junction=") && h.contains("href=\"index.html?street="));
    assert!(h.contains(">Junctions</h3>") && h.contains(">Streets</h3>") && !h.contains("class=\"st"));
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
    assert!(h.contains("href=\"index.html?street=") && h.contains("Fits the street width"));
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
    assert!(h.contains("id=\"tb-street\">Sample city<") && h.contains("id=\"tb-places\" class=\"fig\">32<") && h.contains("id=\"tb-changes\" class=\"fig\">1<"));
}
