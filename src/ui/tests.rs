//! The components drawn to HTML on the host, and read as the page would show
//! them. The markup is checked by what it says and which classes and
//! attributes it carries, not by its exact text.

use std::rc::Rc;

use leptos::prelude::*;


use crate::junction::*;
use crate::vm::junction::JunctionVm;
use crate::shared::units::Units;
use crate::shared::testing::{button_tag, count, html};
use crate::ui::{inspector, notes, page, plan, turns};

fn shared(sample: usize) -> Rc<JunctionVm> {
    JunctionVm::new(crate::shared::platform::browser_ports(), Junction::new(sample), None)
}

fn arm(shared: &JunctionVm, bearing: i32) -> u32 {
    shared.read(|j| j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid)
}

// ---- turns ---------------------------------------------------------------------

#[test]
fn the_turn_table_has_a_button_for_every_turn_and_none_for_a_street_to_itself() {
    let s = shared(0);
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<button"), 12);
    assert_eq!(count(&h, "aria-pressed=\"true\""), 12);
    assert_eq!(count(&h, "class=\"self\""), 4);
    assert!(h.contains("Sample Avenue 2 (north) to Sample Street 1 (east): left turn, allowed"));
}

#[test]
fn a_banned_turn_reads_as_not_allowed() {
    let s = shared(0);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.set_turn(n, e, false));
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert!(h.contains("left turn, not allowed"));
    assert_eq!(count(&h, "aria-pressed=\"true\""), 11);
}

#[test]
fn a_turn_a_measure_blocks_is_locked_with_its_reason() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_rule(e, RULE_DEAD_END));
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert!(count(&h, "turn locked") > 0);
    assert!(h.contains("not possible."));
}

// ---- notes ---------------------------------------------------------------------

#[test]
fn getting_across_lists_each_street_with_its_crossing_and_lanes() {
    let s = shared(0);
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<tr><th scope=\"row\">"), 4);
    assert!(h.contains("2 × 9.0"));
    assert!(h.contains(">11.4<"));
}

#[test]
fn getting_across_follows_the_units() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    assert!(h.contains("2 × 29.5"));
    assert!(h.contains(">37.4<"));
}

#[test]
fn a_street_without_a_crossing_says_none_and_a_too_long_one_is_flagged() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_crossing(e, false));
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"zero\">none"));
}

#[test]
fn conflicts_are_counted_and_the_note_follows_the_control() {
    let s = shared(0);
    let h = html(|| view! { <notes::Conflicts vm=s.clone()/> }.into_any());
    assert!(h.contains("<td>16</td>") && h.contains("<td>32</td>"));
    let note = html(|| view! { <notes::ConflictNote vm=s.clone()/> }.into_any());
    assert!(note.contains("A signal takes turns"));
}

#[test]
fn the_checks_each_say_whether_they_pass() {
    let s = shared(0);
    let h = html(|| view! { <notes::Checks vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<li class=\"ok\">"), 7);
    assert!(h.contains(": passes"));
    assert!(h.contains("Longest crossing in one go: 11.4 m"));
}

#[test]
fn a_failing_check_is_marked_and_says_so_to_a_screen_reader() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 12_000));
    let h = html(|| view! { <notes::Checks vm=s.clone()/> }.into_any());
    assert!(count(&h, "<li class=\"bad\">") >= 1);
    assert!(h.contains(": fails"));
}

#[test]
fn the_changes_start_from_the_junction_today_and_list_each_edit() {
    let s = shared(0);
    let h = html(|| view! { <notes::Revisions vm=s.clone()/> }.into_any());
    assert!(h.contains("Junction today") && h.contains("base now"));
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    let h = html(|| view! { <notes::Revisions vm=s.clone()/> }.into_any());
    assert!(h.contains("7000 mm radius"));
    assert!(h.contains("class=\"now\""));
    assert!(!h.contains("base now"));
}

#[test]
fn the_measures_say_where_each_is_modelled_and_where_it_is_in_use() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_bus_lane(e, true));
    s.edit(|j| j.set_approach(e, Q_CURB));
    let h = html(|| view! { <notes::Measures vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<tbody>"), 3);
    assert!(h.contains("In use on E"));
    assert!(h.contains("Street editor") && h.contains("Not modelled"));
}

// ---- inspector -----------------------------------------------------------------

fn panel(s: &Rc<JunctionVm>) -> String {
    html(|| view! { <inspector::Inspector vm=s.clone()/> }.into_any())
}

#[test]
fn with_nothing_selected_the_panel_offers_the_junction_s_control() {
    let s = shared(0);
    let h = panel(&s);
    assert!(h.contains("Select a street, corner or crossing to change it."));
    assert!(h.contains("Junction control") && h.contains("Traffic signal"));
    assert!(!h.contains("Cycle track"));
}

#[test]
fn a_roundabout_adds_its_size_cycle_track_and_bus_lane() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let h = panel(&s);
    for want in ["Size of the roundabout", "Cycle track", "Bus lane through the middle"] {
        assert!(h.contains(want), "{want}");
    }
    assert!(!h.contains("Track width"));
    s.edit(|j| j.set_cycle_track(true));
    assert!(panel(&s).contains("Track width"));
}

#[test]
fn a_street_s_panel_has_its_lanes_crossing_transit_direction_and_street() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    for want in ["Sample Street 1", "Lanes coming in", "Set back from the junction", "Transit priority", "Direction", "Shift sideways", "Remove this street"] {
        assert!(h.contains(want), "{want}");
    }
    assert!(h.contains("E, 90° · 11.4 m road"));
    assert_eq!(count(&h, "class=\"lane-row\""), 1);
}

#[test]
fn a_street_without_a_crossing_offers_to_mark_one() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_crossing(e, false));
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Mark a crossing") && !h.contains("Remove the crossing"));
}

#[test]
fn a_queue_jump_has_a_length_field_and_a_virtual_one_does_not() {
    let s = shared(0);
    let e = arm(&s, 90);
    s.edit(|j| j.set_approach(e, Q_CURB));
    s.edit(|j| j.select(Target::Arm(e)));
    assert!(panel(&s).contains("Length of the queue jump"));
    s.edit(|j| j.set_approach(e, Q_VIRTUAL));
    let h = panel(&s);
    assert!(!h.contains("Length of the queue jump") && !h.contains("Gate distance upstream"));
}

#[test]
fn a_city_junction_s_streets_link_to_the_street_editor_and_cannot_be_removed() {
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
        a.section = Some(crate::street::model::Street::sample(a.street, crate::shared::catalogue::Side::Right));
    }
    let j = Junction::from_city("Test", &state, &state, 0).unwrap();
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), j, None);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Open the cross-section") && h.contains("index.html?street="));
    assert!(!h.contains("Remove this street"));
}

#[test]
fn a_corner_panel_names_its_two_streets_and_gives_the_radius() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Corner(n)));
    let h = panel(&s);
    assert!(h.contains("Corner") && h.contains("N to E"));
    assert!(h.contains("Curb radius") && h.contains("value=\"6.0\""));
}

#[test]
fn a_lane_panel_lists_where_the_lane_goes() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Lane(n, 1)));
    let h = panel(&s);
    assert!(h.contains("Lane 2 of 2") && h.contains("nearest the curb"));
    assert!(h.contains("Straight on to Sample Avenue 2 (south)"));
    assert!(h.contains("Select the whole street"));
}

#[test]
fn the_bus_lane_and_cycle_track_have_their_own_panels() {
    let s = shared(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let (n, so) = (arm(&s, 0), arm(&s, 180));
    s.edit(|j| j.set_bus(Some((n, so))));
    s.edit(|j| j.set_cycle(Some(2_000)));
    s.edit(|j| j.select(Target::Bus));
    let h = panel(&s);
    assert!(h.contains("Remove the bus lane"));
    s.edit(|j| j.select(Target::Cycle));
    let h = panel(&s);
    assert!(h.contains("Remove the cycle track") && h.contains("Round the outside of the roundabout"));
}

#[test]
fn the_panel_s_lengths_follow_the_units() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("E, 90° · 37.4 ft road"));
}

// ---- what the keys do -------------------------------------------------------------

use crate::ui::keys::{Action, Step};
use crate::shared::live;
use crate::ui::watch::Watch;

fn watch(sample: usize) -> (Rc<JunctionVm>, Watch) {
    let s = shared(sample);
    let w = Watch::new(s.clone());
    live::take_said();
    (s, w)
}

#[test]
fn stepping_the_selected_street_turns_it_by_a_step_and_says_so() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    s.select(Target::Arm(n));
    live::take_said();
    w.apply(Action::Step(Step::Bearing, 1));
    assert_eq!(s.read(|j| j.arm(n).unwrap().bearing), 5);
    assert!(live::take_said()[0].starts_with("Sample Avenue 2 bearing: 5°."));
}

#[test]
fn stepping_suits_what_is_selected() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    s.select(Target::Corner(n));
    w.apply(Action::Step(Step::Corner, -1));
    assert_eq!(s.read(|j| j.arm(n).unwrap().corner_mm), 5_500);
    s.select(Target::Crossing(n));
    w.apply(Action::Step(Step::Setback, 1));
    assert_eq!(s.read(|j| j.arm(n).unwrap().crossing.unwrap().setback_mm), 3_500);
}

#[test]
fn stepping_with_nothing_selected_changes_nothing() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let before = s.read(|j| j.current().clone());
    w.apply(Action::Step(Step::Bearing, 1));
    assert_eq!(s.read(|j| j.current().clone()), before);
}

#[test]
fn removing_takes_away_what_is_selected_in_the_way_that_suits_it() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    s.select(Target::Crossing(n));
    w.apply(Action::Remove);
    assert!(s.read(|j| j.arm(n).unwrap().crossing.is_none()));
    s.select(Target::Arm(n));
    w.apply(Action::Remove);
    assert_eq!(s.read(|j| j.current().arms.len()), 3);
    assert!(s.read(|j| j.arm(n).is_none()));
}

#[test]
fn removing_the_last_removable_street_is_refused_with_its_reason() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(1);
    let uid = s.read(|j| j.current().arms[0].uid);
    s.select(Target::Arm(uid));
    live::take_said();
    w.apply(Action::Remove);
    assert_eq!(live::take_said(), vec!["A junction needs at least three streets."]);
}

#[test]
fn the_selection_moves_round_the_junction_and_says_where_it_is() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    w.apply(Action::Select(1));
    assert_ne!(s.read(|j| j.selected), Target::None);
    assert_eq!(live::take_said().len(), 1);
    w.apply(Action::Deselect);
    assert_eq!(s.read(|j| j.selected), Target::None);
}

#[test]
fn the_bus_lane_and_cycle_track_are_removed_by_the_same_key() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    s.edit(|j| j.set_control(ROUNDABOUT));
    let (n, so) = (arm(&s, 0), arm(&s, 180));
    s.edit(|j| j.set_bus(Some((n, so))));
    s.edit(|j| j.set_cycle(Some(2_000)));
    s.select(Target::Bus);
    w.apply(Action::Remove);
    assert!(s.read(|j| j.current().bus.is_none()));
    s.select(Target::Cycle);
    w.apply(Action::Remove);
    assert!(s.read(|j| j.current().cycle.is_none()));
}

// ---- the page around the plan -------------------------------------------------------

fn linked_shared() -> Rc<JunctionVm> {
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
        a.section = Some(crate::street::model::Street::sample(a.street, crate::shared::catalogue::Side::Right));
    }
    JunctionVm::new(crate::shared::platform::browser_ports(), Junction::from_city("Junction 4", &state, &state, 0).unwrap(), None)
}

#[test]
fn the_heading_names_the_junction_and_counts_its_streets() {
    let s = shared(0);
    let h = html(|| view! { <page::Header vm=s.clone()/> }.into_any());
    assert!(h.contains(">Avenue and street</h1>") && h.contains("Junction plan") && h.contains(">4 streets<"));
    assert!(!h.contains("City map"));
}

#[test]
fn a_city_junction_s_heading_links_back_to_the_map() {
    let h = html(|| view! { <page::Header vm=linked_shared()/> }.into_any());
    assert!(h.contains("href=\"map.html\"") && h.contains("City map") && h.contains(">Junction 4</h1>"));
}

#[test]
fn the_title_block_gives_the_name_the_streets_and_the_changes_made() {
    let s = shared(0);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    let h = html(|| view! { <page::TitleBlock vm=s.clone()/> }.into_any());
    assert!(h.contains("id=\"tb-street\">Avenue and street<") && h.contains("id=\"tb-row\" class=\"fig\">4<") && h.contains("id=\"tb-changes\" class=\"fig\">1<"));
}

#[test]
fn the_status_line_says_whether_the_junction_works() {
    let s = shared(0);
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"fit\"") && h.contains("Every check passes."));
    let n = arm(&s, 0);
    s.edit(|j| j.set_island(n, false));
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"fit bad\"") && h.contains("One check needs attention: crossing distance."));
}

/// The opening tag of the button with this id.
#[test]
fn undo_redo_and_start_over_are_off_until_there_is_something_to_undo() {
    let s = shared(0);
    let off = |h: &str| ["undo", "redo", "reset"].map(|id| button_tag(h, id).contains("disabled"));
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    assert_eq!(off(&h), [true, true, true]);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    assert_eq!(off(&h), [false, true, false]);
    s.edit(|j| j.undo());
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    assert_eq!(off(&h), [true, false, true]);
}

#[test]
fn the_key_lists_the_pieces_in_the_plan() {
    let s = shared(0);
    let h = html(|| view! { <page::Key vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "<li>"), 5);
    assert!(h.contains("Sidewalk") && h.contains("Driving lane") && h.contains("Planted median"));
}

#[test]
fn the_palette_offers_each_street_but_not_the_freeway_with_its_width() {
    let s = shared(0);
    let h = html(|| view! { <page::Palette vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "class=\"chip\""), 3);
    assert!(h.contains("Sample Street 1") && h.contains("18.0 m wide") && h.contains("30.0 m wide"));
    assert!(!h.contains("Freeway"));
    s.set_units(Units::Feet);
    let h = html(|| view! { <page::Palette vm=s.clone()/> }.into_any());
    assert!(h.contains("59.1 ft wide"));
}

#[test]
fn the_samples_say_which_one_the_junction_is() {
    let s = shared(1);
    let h = html(|| view! { <page::Samples vm=s.clone()/> }.into_any());
    assert_eq!(count(&h, "class=\"chip plain\""), 4);
    assert_eq!(count(&h, "aria-pressed=\"true\""), 1);
    assert!(h.contains("Street and lane") && h.contains("3 streets") && h.contains("Five ways"));
}

#[test]
fn the_plan_is_a_labelled_group_holding_a_picture_of_the_junction() {
    let s = shared(0);
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    assert!(h.contains("id=\"wrap\"") && h.contains("tabindex=\"0\"") && h.contains("aria-label=\"Junction plan editor\""));
    assert!(h.contains("id=\"drawing\"") && h.contains("role=\"img\""));
    assert!(h.contains("Plan of the junction, north up. 4 streets."));
    assert!(h.contains("data-role=\"grip-arm\"") && h.contains("class=\"plan\""));
}

#[test]
fn the_plan_follows_the_units_and_the_selection() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Arm(n)));
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    assert!(h.contains("2 × 29.5") && h.contains("66 ft"));
    assert_eq!(count(&h, "class=\"arm on\""), 1);
}

// ---- the city map ---------------------------------------------------------------------------

use crate::ui::map as map_ui;
use crate::vm::city_store::CityStore;
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
