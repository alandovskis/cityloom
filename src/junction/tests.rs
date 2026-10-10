//! The components drawn to HTML on the host, and read as the page would show
//! them. The markup is checked by what it says and which classes and
//! attributes it carries, not by its exact text.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::model::*;
use crate::junction::vm::JunctionVm;
use crate::junction::{inspector, notes, page, plan, turns};
use crate::shared::testing::{button_tag, count, html};
use crate::shared::units::Units;

fn shared(sample: usize) -> Rc<JunctionVm> {
    JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), Junction::new(sample), None)
}

fn arm(shared: &JunctionVm, bearing: i32) -> u32 {
    shared.read(|j| j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid)
}

// ---- the messages ----------------------------------------------------------------

#[test]
fn the_junction_ftl_files_have_the_same_messages_and_variables() {
    use crate::shared::i18n::tests::parity_problems;
    assert_eq!(parity_problems(super::RESOURCES.en, super::RESOURCES.fr), Vec::<String>::new());
}

#[test]
fn the_checks_and_the_history_of_a_junction_speak_french_with_no_english_left() {
    use crate::shared::i18n::Locale;
    use crate::shared::said::say_now;
    let fr = crate::i18n_for(Locale::FrCa);
    let say = |s: &crate::shared::said::Said| say_now(&fr, Units::Metres, s);
    // A junction that fails every check it can, with a history of each kind of edit.
    let mut j = Junction::new(3); // five ways: too many streets for a signal
    let (n, e) = (j.current().arms[0].uid, j.current().arms[1].uid);
    assert!(j.set_control(SIGNAL));
    let _ = j.set_corner(n, 9_000);
    assert!(j.set_approach(n, GATE_SIGNAL));
    assert!(j.set_stop(e, STOP_PLATFORM));
    assert!(j.set_filter(e, true));
    assert!(j.set_rule(e, RULE_RIRO));
    let lane_to = j.arm(n).unwrap().lanes[0].to[0];
    let _ = j.set_lane_dest(n, 0, lane_to, false);
    let v = j.view();
    assert!(v.checks.iter().filter(|c| !c.ok).count() >= 3, "{:?}", v.checks.iter().map(|c| (c.id, c.ok)).collect::<Vec<_>>());
    let mut text: Vec<String> = v.checks.iter().flat_map(|c| [say(&c.label), say(&c.detail)]).collect();
    text.extend(v.revisions.iter().map(|r| say(&r.label)));
    text.extend(v.arms.iter().flat_map(|a| a.transit.problems.iter().map(say)));
    text.extend(v.movements.iter().filter_map(|m| m.blocked.as_ref().map(say)));
    let all = text.join("\n");
    for english in [
        "Lanes",
        "Crossing",
        "crossing",
        "Sidewalk",
        "Signal has",
        "one signal",
        "Transit",
        " to ",
        "Corner",
        "Control",
        "north",
        "south",
        "east",
        "west",
        "needs",
        "streets",
        "turn",
        "lane",
        "Left",
        "Right-in",
        "dead end",
        "modal filter lets",
    ] {
        assert!(!all.contains(english), "{english:?} in:\n{all}");
    }
    assert!(all.contains("Signalisation\u{a0}: feux de circulation"), "{all}");
    assert_eq!(fr.missing(), Vec::<String>::new());
}

#[test]
fn the_turn_table_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::FrCa), Junction::new(0), None);
    let e = arm(&s, 90);
    s.edit(|j| j.set_rule(e, RULE_DEAD_END));
    let h = html(|| view! { <turns::Turns vm=s.clone()/> }.into_any());
    assert!(h.contains("Sample Avenue 2 (nord) vers Sample Street 1 (est)\u{a0}: virage à gauche"), "{h}");
    assert!(h.contains("Virages permis.") && h.contains(">O<") && h.contains("impossible. Un cul-de-sac"), "{h}");
    for english in ["Turns allowed", "From", " to ", " turn", "allowed", "north", ">W<"] {
        assert!(!h.contains(english), "{english:?} in {h}");
    }
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
    assert!(h.contains("7.0 m radius"));
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
    }
    let j = Junction::from_city(crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Test".into())), &state, &state, 0)
        .unwrap();
    let s = JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(crate::shared::i18n::Locale::En), j, None);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    let h = panel(&s);
    assert!(h.contains("Open the cross-section") && h.contains("street.html?street="));
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

use crate::junction::keys::{Action, Step};
use crate::junction::watch::Watch;
use crate::shared::live;

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
    }
    JunctionVm::new(
        crate::shared::platform::browser_ports(),
        crate::i18n_for(crate::shared::i18n::Locale::En),
        Junction::from_city(crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Junction 4".into())), &state, &state, 0)
            .unwrap(),
        None,
    )
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
    assert!(
        h.contains("id=\"tb-street\">Avenue and street<") && h.contains("id=\"tb-row\" class=\"fig\">4<") && h.contains("id=\"tb-changes\" class=\"fig\">1<")
    );
}

#[test]
fn the_title_block_links_the_osm_nodes_and_says_whether_the_junction_was_changed() {
    use crate::shared::provenance::OsmRef;
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
    }
    state.source = vec![OsmRef { id: 29796354, version: Some(5) }, OsmRef { id: 9, version: None }];
    let s = JunctionVm::new(
        crate::shared::platform::browser_ports(),
        crate::i18n_for(crate::shared::i18n::Locale::En),
        Junction::from_city(crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Junction 4".into())), &state, &state, 0)
            .unwrap(),
        None,
    );
    let h = html(|| view! { <page::TitleBlock vm=s.clone()/> }.into_any());
    assert!(h.contains("href=\"https://www.openstreetmap.org/node/29796354\"") && h.contains(">node 29796354 v5<"), "{h}");
    assert!(h.contains(">node 9<") && h.contains("id=\"tb-state\">As imported<"), "{h}");
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    let h = html(|| view! { <page::TitleBlock vm=s.clone()/> }.into_any());
    assert!(h.contains("id=\"tb-state\">Edited<"), "{h}");
}

#[test]
fn the_title_block_of_a_junction_with_no_source_does_not_link_to_openstreetmap() {
    let h = html(|| view! { <page::TitleBlock vm=shared(0)/> }.into_any());
    assert!(!h.contains("openstreetmap.org"), "{h}");
}

#[test]
fn the_status_line_says_whether_the_junction_works() {
    let s = shared(0);
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"fit\"") && h.contains("Every check passes.") && h.contains("tick"));
    let n = arm(&s, 0);
    s.edit(|j| j.set_island(n, false));
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    assert!(h.contains("class=\"fit bad\"") && h.contains("One check needs attention: crossing distance.") && !h.contains("tick"));
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

// ---- the page in French ---------------------------------------------------------------

fn shared_in(sample: usize, locale: crate::shared::i18n::Locale) -> Rc<JunctionVm> {
    JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(locale), Junction::new(sample), None)
}

fn linked_in(locale: crate::shared::i18n::Locale) -> Rc<JunctionVm> {
    let mut state = Junction::new(0).current().clone();
    for a in &mut state.arms {
        a.edge = a.uid + 10;
    }
    state.source = vec![crate::shared::provenance::OsmRef { id: 9, version: None }];
    let name = crate::shared::said::Said::new("city-name").with("name", crate::shared::said::Arg::Text("Jonction 4".into()));
    JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(locale), Junction::from_city(name, &state, &state, 0).unwrap(), None)
}

/// Fails on the first of `english` found in `h`, and on the first of `french` missing from it.
fn speaks_french(h: &str, english: &[&str], french: &[&str]) {
    for e in english {
        assert!(!h.contains(e), "{e:?} in:\n{h}");
    }
    for f in french {
        assert!(h.contains(f), "{f:?} missing from:\n{h}");
    }
}

/// English that must not be left on a French junction page, whatever the view.
const ENGLISH: &[&str] = &["Junction plan", "Does it work", "Your changes", "Turns allowed", "Where paths meet", "streets", "Every check"];

#[test]
fn the_page_around_the_plan_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = linked_in(Locale::FrCa);
    let h = html(|| view! { <page::Header vm=s.clone()/> }.into_any());
    speaks_french(&h, &[ENGLISH, &["City map"]].concat(), &["Carte de la ville", "Plan de la jonction", ">4 rues<", ">Jonction 4</h1>"]);
    let h = html(|| view! { <page::TitleBlock vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &["Junction<", "Streets", "Changes made", "Data", "As imported"],
        &[">Jonction<", ">Rues<", "Modifications apportées", "Données", "Telle qu’importée"],
    );
    let h = html(|| view! { <page::Fit vm=s.clone()/> }.into_any());
    speaks_french(&h, ENGLISH, &["4 rues, feux de circulation. Toutes les vérifications réussissent."]);
    let h = html(|| view! { <page::History vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Undo", "Redo", "Start over"], &["Annuler", "Rétablir", "Recommencer"]);
}

#[test]
fn the_notes_speak_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.set_bus_lane(e, true));
    s.edit(|j| j.set_approach(e, Q_CURB));
    s.edit(|j| j.set_corner(n, 7_000));
    s.edit(|j| j.set_crossing(e, false));
    let h = html(|| view! { <notes::Across vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &["How far", ">Street<", "To cross", "Lanes in", ">none<", ">W<"],
        &["Rue", "À traverser", "Voies entrantes", ">aucun<", ">O<", "2 × 9,0"],
    );
    let h = html(|| view! { <notes::Conflicts vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Points where", "Kind", "Merging", "Splitting", ">All<"], &["Type", "Croisement", "Convergence", "Divergence", "Total"]);
    let h = html(|| view! { <notes::ConflictNote vm=s.clone()/> }.into_any());
    speaks_french(&h, &["A signal takes turns"], &["Les feux"]);
    let h = html(|| view! { <notes::Checks vm=s.clone()/> }.into_any());
    speaks_french(&h, &[": passes", "Longest crossing", " m<"], &["\u{a0}: réussite", "Plus longue traversée d’un seul coup\u{a0}: 11,4\u{a0}m"]);
    let h = html(|| view! { <notes::Measures vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &["In use on", "Street editor", "Not modelled", "Set on a street", "Localized measures", "Curbside Queue-Jump Lanes", "under development"],
        &[
            "Utilisée sur E",
            "Éditeur de rue",
            "Non modélisée",
            "Mesures ponctuelles",
            "Voies de dépassement de file en bordure de trottoir",
            "en développement",
        ],
    );
    let h = html(|| view! { <notes::Revisions vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &["Step", "What changed", "Junction today", "Corner after"],
        &["Étape", "Ce qui a changé", "Jonction aujourd’hui", "Coin après Sample Avenue 2 (nord)"],
    );
}

#[test]
fn the_plan_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.set_offset(e, 500));
    s.edit(|j| j.select(Target::Corner(n)));
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    speaks_french(
        &h,
        &[ENGLISH, &["Plan of the junction", "north up", "Scale", " road", "shifted", "Turn ", "Change the corner radius", "Lane 1 of"]].concat(),
        &[
            "aria-label=\"Éditeur du plan de la jonction\"",
            "Plan de la jonction, nord en haut. 4 rues.",
            ">Échelle<",
            "E · chaussée de 11,4\u{a0}m · décalée de 0,5\u{a0}m",
            "Faire pivoter Sample Street 1 (est)",
            "Modifier le rayon du coin",
            "Voie 1 sur 2, Sample Avenue 2 (nord)",
            ">20\u{a0}m<",
            ">2 × 9,0<",
        ],
    );
    s.edit(|j| j.select(Target::Crossing(n)));
    s.edit(|j| j.set_control(ROUNDABOUT));
    let so = arm(&s, 180);
    s.edit(|j| j.set_bus(Some((n, so))));
    let h = html(|| view! { <plan::PlanDrawing vm=s.clone()/> }.into_any());
    speaks_french(&h, &["Move the crossing", "Bus only", "Roundabout."], &["Déplacer le passage pour piétons", "Autobus seulement", "Carrefour giratoire."]);
}

#[test]
fn a_switch_of_language_draws_the_plan_its_label_and_the_status_again() {
    use crate::junction::plan_svg::{plan_label, plan_svg};
    use crate::junction::text::{arms_text, fit_text};
    use crate::shared::i18n::Locale;
    let owner = Owner::new();
    owner.set();
    let i18n = crate::i18n_for(Locale::En);
    // What the views' closures do: they word the junction from the shared words, so they track them.
    let shared = StoredValue::new_local((i18n.clone(), Junction::new(0).view()));
    let markup = Memo::new(move |_| shared.with_value(|(i18n, v)| plan_svg(v, i18n, 1000.0, 1000.0, Units::Metres).markup));
    let label = Memo::new(move |_| shared.with_value(|(i18n, v)| plan_label(v, i18n)));
    let status = Memo::new(move |_| shared.with_value(|(i18n, v)| format!("{} / {}", arms_text(v, i18n), fit_text(v, i18n))));
    assert!(markup.get().contains(">Scale<") && label.get().starts_with("Plan of the junction") && status.get().starts_with("4 streets"));
    i18n.set(Locale::FrCa);
    assert!(markup.get().contains(">Échelle<") && !markup.get().contains(">Scale<"), "{}", markup.get());
    assert!(label.get().starts_with("Plan de la jonction, nord en haut."), "{}", label.get());
    assert!(status.get().starts_with("4 rues / 4 rues, feux de circulation."), "{}", status.get());
}

#[test]
fn the_junction_page_asks_for_no_message_that_is_missing_in_french() {
    use crate::shared::i18n::Locale;
    let s = linked_in(Locale::FrCa);
    let n = arm(&s, 0);
    s.edit(|j| j.set_corner(n, 7_000));
    s.edit(|j| j.select(Target::Arm(n)));
    let _ = html(|| {
        view! {
            <page::Header vm=s.clone()/><page::TitleBlock vm=s.clone()/><page::Fit vm=s.clone()/><page::History vm=s.clone()/><page::Key vm=s.clone()/>
            <notes::Across vm=s.clone()/><notes::Conflicts vm=s.clone()/><notes::ConflictNote vm=s.clone()/><notes::Checks vm=s.clone()/>
            <notes::Measures vm=s.clone()/><notes::Revisions vm=s.clone()/><plan::PlanDrawing vm=s.clone()/><turns::Turns vm=s.clone()/>
        }
        .into_any()
    });
    assert_eq!(s.i18n().missing(), Vec::<String>::new());
}

// ---- the panel in French ----------------------------------------------------------------

/// English that must not be left in the panel beside the plan, whatever is selected. The sample streets are
/// named "Sample Street 1", "Sample Avenue 2"…, so "Street" alone is not on the list.
const PANEL_ENGLISH: &[&str] = &[
    "Select a",
    "Junction control",
    "Size of the",
    "across",
    "Cycle track",
    "Track",
    "Bus lane",
    "Remove",
    "Corner",
    "Curb",
    "curb",
    "Smaller",
    "Larger",
    "Tighter",
    "Wider",
    "Narrower",
    "Closer",
    "Farther",
    "Shorter",
    "Longer",
    "Shift",
    "clockwise",
    ">Lane ",
    "\"Lane ",
    "Where this",
    "Set back",
    "Crossing",
    "crossing",
    "Halfway",
    "Refuge",
    "Shorten",
    "Mark a",
    "Transit",
    "At the approach",
    "Bus stop",
    "Turns",
    "Length of",
    "Gate distance",
    "Direction",
    "Clockwise",
    "neighbours",
    "Open the",
    "belongs to",
    "One way",
    "one way",
    "coming in",
    " road",
    " to ",
    "stages",
    "Straight on",
    "Left",
    "Right",
    "nearest",
    "only lane",
    "Cars turn",
    "wide",
    "parking",
    "needs at least",
    "No bus",
    "Lets buses",
    "Round the",
];

/// The panel of `s` in French, checked against the English list and for the French phrases.
fn panel_speaks_french(s: &Rc<JunctionVm>, french: &[&str]) {
    speaks_french(&panel(s), PANEL_ENGLISH, french);
    assert_eq!(s.i18n().missing(), Vec::<String>::new());
}

#[test]
fn the_panel_with_nothing_selected_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    panel_speaks_french(&s, &["Sélectionnez une rue, un coin ou un passage pour piétons à modifier.", "Signalisation de la jonction", "Feux de circulation"]);
}

#[test]
fn the_panel_of_a_roundabout_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    s.edit(|j| j.set_control(ROUNDABOUT));
    panel_speaks_french(
        &s,
        &[
            "Taille du carrefour giratoire",
            "m de diamètre",
            "Diamètre extérieur. Au moins ",
            "aria-label=\"Réduire de 0,5\u{a0}m\"",
            "aria-label=\"Agrandir de 0,5\u{a0}m\"",
            "Piste cyclable",
            "Piste tout autour, à l’extérieur",
            "Voie d’autobus par le centre",
            "Aucune voie d’autobus",
            "Permet aux autobus de couper à travers l’îlot entre deux rues.",
        ],
    );
    let (n, so) = (arm(&s, 0), arm(&s, 180));
    s.edit(|j| j.set_cycle_track(true));
    s.edit(|j| j.set_bus(Some((n, so))));
    panel_speaks_french(
        &s,
        &[
            "Largeur de la piste",
            "De 1,5\u{a0}m à 3,0\u{a0}m. Elle prend de la place à la chaussée, dans le même cercle.",
            "aria-label=\"Rétrécir de 0,5\u{a0}m\"",
            "aria-label=\"Élargir de 0,5\u{a0}m\"",
            "Une voie de 3,5\u{a0}m réservée aux autobus, tout droit à travers l’îlot.",
        ],
    );
    s.edit(|j| j.select(Target::Bus));
    panel_speaks_french(&s, &[">Voie d’autobus<", "Retirer la voie d’autobus", "Une voie de 3,5\u{a0}m réservée aux autobus"]);
    s.edit(|j| j.select(Target::Cycle));
    panel_speaks_french(
        &s,
        &[">Piste cyclable<", "Tout autour du carrefour giratoire, à l’extérieur", "Les cyclistes traversent chaque rue", "Retirer la piste cyclable"],
    );
}

#[test]
fn the_panel_of_a_corner_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Corner(n)));
    panel_speaks_french(
        &s,
        &[
            ">Coin<",
            ">N vers E<",
            "Rayon de la bordure",
            "aria-label=\"Resserrer de 0,5\u{a0}m\"",
            "aria-label=\"Élargir de 0,5\u{a0}m\"",
            "De 1,0\u{a0}m à 15,0\u{a0}m. Les voitures tournent ici à environ ",
            "\u{a0}km/h.",
            "Un coin serré ralentit les voitures",
        ],
    );
}

#[test]
fn the_panel_of_a_crossing_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let (n, e) = (arm(&s, 0), arm(&s, 90));
    s.edit(|j| j.select(Target::Crossing(e)));
    panel_speaks_french(
        &s,
        &[
            "E, 90° · chaussée de 11,4\u{a0}m",
            ">Passage pour piétons<",
            "11,4\u{a0}m à traverser",
            "Retirer le passage pour piétons",
            "Retrait par rapport à la jonction",
            "aria-label=\"Rapprocher de 0,5\u{a0}m\"",
            "aria-label=\"Éloigner de 0,5\u{a0}m\"",
            "De 2,0\u{a0}m à 8,0\u{a0}m",
            "Largeur du passage pour piétons",
            "De 2,0\u{a0}m à 6,0\u{a0}m",
            "Îlot à mi-traversée",
            "Îlot refuge",
            "Raccourcir la traversée",
            "Avancée de trottoir à gauche",
            "Avancée de trottoir à droite",
        ],
    );
    s.edit(|j| j.select(Target::Crossing(n)));
    panel_speaks_french(&s, &["2 traversées de 9,0\u{a0}m"]);
}

#[test]
fn the_panel_of_a_street_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    panel_speaks_french(
        &s,
        &[
            "Sample Street 1",
            "E, 90° · chaussée de 11,4\u{a0}m",
            "Voies entrantes",
            ">Voie 1<",
            "aria-label=\"Voie 1 vers ",
            "Priorité au transport en commun",
            "Voie réservée aux autobus en entrée",
            "À l’approche",
            "Arrêt d’autobus",
            ">Virages<",
            "N1 Filtre modal pour le transport en commun",
            ">Orientation<",
            "aria-label=\"Faire pivoter de 5° dans le sens antihoraire\"",
            "aria-label=\"Faire pivoter de 5° dans le sens horaire\"",
            "Dans le sens horaire à partir du nord. Au moins 30° de ses voisines.",
            "Décalage latéral",
            "aria-label=\"Décaler de 0,1\u{a0}m vers la gauche\"",
            "aria-label=\"Décaler de 0,1\u{a0}m vers la droite\"",
            "Jusqu’à ",
            "Retirer cette rue",
        ],
    );
    s.edit(|j| j.set_crossing(e, false));
    panel_speaks_french(&s, &["Marquer un passage pour piétons"]);
    s.edit(|j| j.set_approach(e, Q_CURB));
    panel_speaks_french(&s, &["Longueur de la voie de dépassement de file", "aria-label=\"Raccourcir de ", "aria-label=\"Allonger de ", "G2 Voie"]);
    s.edit(|j| j.set_approach(e, GATE_SIGNAL));
    panel_speaks_french(&s, &["Distance de la porte d’autobus en amont"]);
}

#[test]
fn the_panel_of_a_street_that_cannot_be_removed_says_why_in_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let w = arm(&s, 270);
    s.edit(|j| j.remove_arm(w));
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    panel_speaks_french(&s, &["Une jonction compte au moins 3 rues."]);
}

#[test]
fn the_panel_of_a_city_street_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = linked_in(Locale::FrCa);
    let e = arm(&s, 90);
    s.edit(|j| j.select(Target::Arm(e)));
    panel_speaks_french(&s, &[">Rue<", "Ouvrir la coupe transversale", "Cette rue appartient à la ville."]);
}

#[test]
fn the_panel_of_a_lane_speaks_french() {
    use crate::shared::i18n::Locale;
    let s = shared_in(0, Locale::FrCa);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Lane(n, 1)));
    panel_speaks_french(
        &s,
        &[
            "Voie 2 sur 2",
            "Sample Avenue 2 (nord) · la plus près de la bordure",
            "Où mène cette voie",
            "Tout droit vers Sample Avenue 2 (sud)",
            "Sélectionner toute la rue",
            "\u{a0}m de large. Une voie doit mener à au moins une rue.",
        ],
    );
    s.edit(|j| j.select(Target::Lane(n, 0)));
    panel_speaks_french(&s, &["la plus près du centre"]);
}

/// The first sample junction with an arm that `pick` chooses, made in French, and that arm.
fn sample_with(pick: impl Fn(&crate::junction::read_model::ArmView) -> bool) -> (Rc<JunctionVm>, u32) {
    use crate::shared::i18n::Locale;
    for sample in 0..JUNCTION_SAMPLES.len() {
        let s = shared_in(sample, Locale::FrCa);
        if let Some(a) = s.view_now().arms.iter().find(|a| pick(a)) {
            return (s.clone(), a.uid);
        }
    }
    panic!("no sample junction has such an arm");
}

#[test]
fn the_panel_of_a_narrow_road_speaks_french() {
    // "Sample Lane 3" is too narrow for an island and has parking on one side only.
    let (s, uid) = sample_with(|a| a.crossing.is_some() && !a.can_island);
    s.edit(|j| j.select(Target::Crossing(uid)));
    panel_speaks_french(&s, &["Îlot refuge (chaussée trop étroite)", "Avancée de trottoir à gauche (pas de stationnement de ce côté)"]);
}

#[test]
fn a_one_way_street_s_words_speak_french() {
    // No sample junction has a one-way street, so the words are checked as the panel asks for them.
    use crate::junction::inspector::goes_text;
    use crate::shared::i18n::{Args, Locale};
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    assert_eq!(goes_text(&en, LEFT, "X".into(), false), "Left to X (one way in)");
    assert_eq!(goes_text(&fr, LEFT, "X".into(), false), "À gauche vers X (sens unique vers la jonction)");
    assert_eq!(goes_text(&fr, RIGHT, "X".into(), true), "À droite vers X");
    assert_eq!(fr.tr_now("jn-insp-one-way-out", &Args::new()), "Sens unique sortant. Aucune voie n’entre.");
}

#[test]
fn a_crossing_too_far_for_one_go_says_so_in_french() {
    use crate::junction::inspector::crossing_range;
    use crate::shared::i18n::Locale;
    let fr = crate::i18n_for(Locale::FrCa);
    let mut c = Junction::new(0).view().arms.into_iter().find(|a| a.bearing == 90).unwrap().crossing.unwrap();
    c.too_far = true;
    assert_eq!(crossing_range(&c, &fr, Units::Metres), "11,4\u{a0}m à traverser. Trop long à traverser d’un seul coup.");
}

#[test]
fn the_panel_s_plurals_are_singular_at_zero_and_one_in_french() {
    use crate::shared::i18n::{Args, Locale};
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let stages = |i: &crate::shared::i18n::I18n, n: i64| i.tr_now("jn-insp-crossing-stages", &Args::new().num("n", n).str("length", "9"));
    assert_eq!([0, 1, 2].map(|n| stages(&en, n)), ["0 stages of 9", "1 stage of 9", "2 stages of 9"]);
    assert_eq!([0, 1, 2].map(|n| stages(&fr, n)), ["0 traversée de 9", "1 traversée de 9", "2 traversées de 9"]);
    let arms = |i: &crate::shared::i18n::I18n, n: i64| i.tr_now("jn-insp-min-arms", &Args::new().num("n", n));
    assert_eq!(
        [0, 1, 2].map(|n| arms(&en, n)),
        ["A junction needs at least 0 streets.", "A junction needs at least 1 street.", "A junction needs at least 2 streets."]
    );
    assert_eq!(
        [0, 1, 2].map(|n| arms(&fr, n)),
        ["Une jonction compte au moins 0 rue.", "Une jonction compte au moins 1 rue.", "Une jonction compte au moins 2 rues."]
    );
}

#[test]
fn the_panel_s_steppers_follow_the_units() {
    let s = shared(0);
    s.set_units(Units::Feet);
    let n = arm(&s, 0);
    s.edit(|j| j.select(Target::Corner(n)));
    let h = panel(&s);
    assert!(h.contains("aria-label=\"Tighter by 1.6 ft\""), "{h}");
}

// ---- a junction that cannot be drawn ---------------------------------------------------------

fn junction_four() -> crate::shared::said::Said {
    crate::shared::said::Said::new("city-junction-number").with("n", crate::shared::said::Arg::Num(4))
}

#[test]
fn a_junction_that_cannot_be_drawn_says_so_in_english() {
    use crate::shared::i18n::Locale;
    let i18n = crate::i18n_for(Locale::En);
    let h = html(|| view! { <page::StuckHeader i18n=i18n.clone() name=junction_four()/><page::Stuck i18n=i18n.clone()/> }.into_any());
    for want in [
        "<h1 id=\"street-name\">Junction 4</h1>",
        "class=\"stuck\"",
        ">This junction cannot be drawn</h2>",
        "The streets that meet here have been changed so that they no longer make a junction.",
        "href=\"map.html\"",
        ">City map</a>",
    ] {
        assert!(h.contains(want), "{want:?} missing from:\n{h}");
    }
}

#[test]
fn a_junction_that_cannot_be_drawn_says_so_in_french() {
    use crate::shared::i18n::Locale;
    let i18n = crate::i18n_for(Locale::FrCa);
    let h = html(|| view! { <page::StuckHeader i18n=i18n.clone() name=junction_four()/><page::Stuck i18n=i18n.clone()/> }.into_any());
    speaks_french(
        &h,
        &["Junction", "cannot be drawn", "The streets", "City map"],
        &[">Jonction 4</h1>", ">Cette jonction ne peut pas être dessinée</h2>", "Redonnez de la place aux rues", ">Carte de la ville</a>"],
    );
    assert_eq!(i18n.missing(), Vec::<String>::new());
}

#[test]
fn a_junction_that_cannot_be_drawn_follows_a_switch_of_language() {
    use crate::junction::page::stuck_words;
    use crate::shared::i18n::Locale;
    let owner = Owner::new();
    owner.set();
    let i18n = crate::i18n_for(Locale::En);
    let held = StoredValue::new_local(i18n.clone());
    let words = Memo::new(move |_| held.with_value(|i| stuck_words(i, &junction_four())));
    assert_eq!(words.get().0, "Junction 4 · CityLoom");
    i18n.set(Locale::FrCa);
    assert_eq!(words.get(), ("Jonction 4 · CityLoom".to_string(), "Jonction 4".to_string()));
}

#[test]
fn the_junction_page_and_the_page_of_one_that_cannot_be_drawn_start_in_the_stored_language() {
    // open_junction and mount_bare_shell both take their words from page_i18n.
    use crate::shared::i18n::{LANG_KEY, Locale};
    let (ports, page, _) = crate::shared::ports::test_ports_with_page();
    ports.storage.remember(LANG_KEY, "fr-CA");
    let i18n = super::page_i18n(&ports);
    assert_eq!(i18n.locale_now(), Locale::FrCa);
    assert_eq!(*page.lang.borrow(), "fr-CA", "the document says the language the page is in");
    let h = html(|| view! { <page::Stuck i18n=i18n.clone()/> }.into_any());
    assert!(h.contains("Cette jonction ne peut pas être dessinée"), "{h}");
}

// ---- the page's markup ----------------------------------------------------------------------

const INTERSECTION_HTML: &str = include_str!("../../web/intersection.html");

#[test]
fn every_data_i18n_key_in_intersection_html_exists_in_both_languages() {
    use crate::shared::i18n::{Args, Locale};
    use crate::shell::view::i18n_keys;
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let keys = i18n_keys(INTERSECTION_HTML);
    assert!(keys.len() > 45, "the page should have been converted: {}", keys.len());
    for (key, _) in keys {
        en.tr_now(&key, &Args::new());
        fr.tr_now(&key, &Args::new());
    }
    assert_eq!(en.missing(), Vec::<String>::new(), "missing in en");
    assert_eq!(fr.missing(), Vec::<String>::new(), "missing in fr");
}

#[test]
fn the_english_text_in_intersection_html_is_the_english_message() {
    use crate::shared::i18n::{Args, Locale};
    let en = crate::i18n_for(Locale::En);
    for (key, fallback) in crate::shell::view::i18n_keys(INTERSECTION_HTML) {
        assert_eq!(fallback, en.tr_now(&key, &Args::new()), "{key}");
    }
}

#[test]
fn intersection_html_has_a_language_row_and_sets_the_language_in_the_head() {
    assert!(INTERSECTION_HTML.contains(r#"data-lang="en" lang="en" aria-pressed="true">English<"#));
    assert!(INTERSECTION_HTML.contains(r#"data-lang="fr-CA" lang="fr" aria-pressed="false">Français<"#));
    assert_eq!(INTERSECTION_HTML.matches("<script>").count(), 1);
    assert!(INTERSECTION_HTML.contains(r#"localStorage.getItem("cityloom-lang")"#));
    // The header sets the window title from the junction's name, so the shell does not write it.
    assert!(!INTERSECTION_HTML.contains("<title data-i18n"));
}

#[test]
fn intersection_html_leaves_no_text_without_its_message() {
    // Every text between tags in the body that holds a letter is a message, unless it is a unit, a key cap, a
    // language's own name or the Atlas's name.
    use crate::shell::view::i18n_keys;
    let body = &INTERSECTION_HTML[INTERSECTION_HTML.find("<body>").unwrap()..];
    let translated: Vec<String> = i18n_keys(body).into_iter().map(|(_, text)| text).collect();
    let own = ["m", "ft", "Z", "English", "Français", "Transit Priority Atlas"];
    for piece in body.split('>').filter_map(|p| p.split('<').next()).map(str::trim).filter(|t| !t.is_empty()) {
        if piece.chars().any(char::is_alphabetic) && !own.contains(&piece) {
            assert!(translated.iter().any(|t| t == piece), "{piece:?} has no message");
        }
    }
    // The text people read in attributes is translated too.
    for attr in ["aria-label=\"", "title=\""] {
        assert_eq!(INTERSECTION_HTML.matches(&format!(" {attr}")).count(), INTERSECTION_HTML.matches(&format!(" data-i18n-{attr}")).count(), "{attr}");
    }
    assert!(INTERSECTION_HTML.contains(r#"<meta name="description" data-i18n-content="jn-page-description""#));
}

// ---- the number fields ----------------------------------------------------------------

fn cycle_width_mm(s: &JunctionVm) -> Option<i32> {
    s.read(|j| j.current().cycle)
}

fn roundabout_with_cycle_track(s: &JunctionVm) {
    s.edit(|j| j.set_control(ROUNDABOUT));
    s.edit(|j| j.set_cycle_track(true));
}

#[test]
fn a_length_typed_with_a_comma_or_a_point_is_read_as_a_decimal() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    s.edit(|j| j.set_cycle(Some(1_500)));
    assert!(inspector::commit_cycle(w, "2,5"));
    assert_eq!(cycle_width_mm(&s), Some(2_500));
    s.edit(|j| j.set_cycle(Some(1_500)));
    assert!(inspector::commit_cycle(w, "2.5"));
    assert_eq!(cycle_width_mm(&s), Some(2_500));
}

#[test]
fn a_length_typed_in_feet_is_converted() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    s.set_units(Units::Feet);
    assert!(inspector::commit_cycle(w, "8,2"));
    assert_eq!(cycle_width_mm(&s), Some(2_500));
}

#[test]
fn what_is_not_a_number_is_not_committed() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    let before = cycle_width_mm(&s);
    assert!(!inspector::commit_cycle(w, "abc"));
    assert!(!inspector::commit_cycle(w, "1,2,3"));
    assert_eq!(cycle_width_mm(&s), before);
}

#[test]
fn a_typed_length_outside_what_fits_is_refused_by_the_model() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    roundabout_with_cycle_track(&s);
    let before = cycle_width_mm(&s);
    crate::shared::live::take_said();
    assert!(inspector::commit_cycle(w, "9,5"));
    assert_eq!(cycle_width_mm(&s), before);
    assert!(!crate::shared::live::take_said().is_empty(), "the refusal is announced");
    // What the field is made to hold again is the model's value.
    assert!(panel(&s).contains(&format!("value=\"{}\"", Units::Metres.fixed(before.unwrap(), 1))));
}

#[test]
fn a_bearing_typed_with_a_comma_is_rounded_and_snapped_like_any_other() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = watch(0);
    let n = arm(&s, 0);
    assert!(inspector::commit_bearing(w, n, "20"));
    let ninety = s.read(|j| j.arm(n).unwrap().bearing);
    assert_eq!(ninety, 20);
    assert!(inspector::commit_bearing(w, n, "20,5"));
    assert_eq!(s.read(|j| j.arm(n).unwrap().bearing), 20, "20,5 is 21, and the nearest step is 20");
    assert!(inspector::commit_bearing(w, n, "22,5"));
    assert_eq!(s.read(|j| j.arm(n).unwrap().bearing), 25);
    assert!(!inspector::commit_bearing(w, n, "north"));
}

#[test]
fn a_field_with_no_hint_is_not_described_by_one() {
    let s = shared(0);
    s.select(Target::Arm(arm(&s, 0)));
    let h = panel(&s);
    for id in h.split("aria-describedby=\"").skip(1) {
        let id = id.split('"').next().unwrap();
        assert!(h.contains(&format!("id=\"{id}\"")), "{id} is described by nothing");
    }
}

#[test]
fn the_arrow_keys_step_a_number_field() {
    assert_eq!(inspector::step_key("ArrowUp"), Some(1));
    assert_eq!(inspector::step_key("ArrowDown"), Some(-1));
    assert_eq!(inspector::step_key("Enter"), None);
}

#[test]
fn every_number_field_is_text_that_asks_for_a_decimal_keyboard() {
    let s = shared_in(0, crate::shared::i18n::Locale::FrCa);
    roundabout_with_cycle_track(&s);
    let ring = panel(&s);
    s.select(Target::Arm(arm(&s, 0)));
    let street = panel(&s);
    s.select(Target::Corner(arm(&s, 0)));
    let corner = panel(&s);
    for h in [&ring, &street, &corner] {
        assert!(!h.contains("type=\"number\""), "{h}");
        assert!(count(h, "inputmode=\"decimal\"") >= 1, "{h}");
        assert_eq!(count(h, "<input"), count(h, "type=\"text\""), "{h}");
    }
    assert!(count(&ring, "type=\"text\"") >= 2, "the ring's size and the track's width");
}

#[test]
fn a_length_is_held_with_the_decimal_mark_of_the_language() {
    let s = shared_in(0, crate::shared::i18n::Locale::FrCa);
    roundabout_with_cycle_track(&s);
    s.edit(|j| j.set_cycle(Some(2_500)));
    assert!(panel(&s).contains("value=\"2,5\""));
    let en = shared(0);
    roundabout_with_cycle_track(&en);
    en.edit(|j| j.set_cycle(Some(2_500)));
    assert!(panel(&en).contains("value=\"2.5\""));
}
