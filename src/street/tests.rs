//! The street page's keys and components drawn to HTML on the host, and read as
//! the page would show them.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::catalogue::KINDS;
use crate::shared::i18n::Locale;
use crate::shared::live;
use crate::shared::testing::{button_tag, count, html};
use crate::shared::units::{Units, group_thousands};
use crate::street::keys::Action as StreetAction;
use crate::street::model::Editor;
use crate::street::vm::StreetVm;
use crate::street::watch::SheetWatch;
use crate::street::{inspector, notes, page, view};
// ---- what the keys do on the street ---------------------------------------------------

fn street_watch() -> (Rc<StreetVm>, SheetWatch) {
    let s = StreetVm::new(crate::shared::platform::browser_ports(), Editor::new(0), None);
    let w = SheetWatch::new(s.clone());
    live::take_said();
    (s, w)
}

fn segment(s: &StreetVm, i: usize) -> u32 {
    s.view().segments[i].uid
}

#[test]
fn nudging_changes_the_selected_piece_s_width_and_says_so() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = street_watch();
    let u = segment(&s, 2);
    s.select(Some(u));
    live::take_said();
    w.apply(StreetAction::Nudge(100));
    assert_eq!(s.view().segments[2].width_mm, 3_400);
    assert!(live::take_said()[0].contains(". "));
    w.apply(StreetAction::Nudge(-500));
    assert_eq!(s.view().segments[2].width_mm, 2_900);
}

#[test]
fn nudging_with_nothing_selected_changes_nothing() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = street_watch();
    w.apply(StreetAction::Nudge(100));
    w.apply(StreetAction::Remove);
    w.apply(StreetAction::Move(1));
    assert_eq!(s.view().revisions.len(), 0);
}

#[test]
fn a_piece_moves_one_place_along_and_stops_at_the_ends() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = street_watch();
    let first = segment(&s, 0);
    s.select(Some(first));
    w.apply(StreetAction::Move(-1));
    assert_eq!(segment(&s, 0), first, "already first");
    w.apply(StreetAction::Move(1));
    assert_eq!(segment(&s, 1), first);
    assert_eq!(s.view().revisions.len(), 1);
}

#[test]
fn the_selection_moves_along_the_pieces_and_escape_clears_it() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = street_watch();
    w.apply(StreetAction::Select(1));
    assert!(s.view().selected.is_some());
    assert_eq!(live::take_said().len(), 1);
    w.apply(StreetAction::Escape);
    assert_eq!(s.view().selected, None);
}

#[test]
fn removing_takes_the_selected_piece_away() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = street_watch();
    let u = segment(&s, 0);
    s.select(Some(u));
    w.apply(StreetAction::Remove);
    assert_eq!(s.view().segments.len(), 5);
    assert!(s.view().segments.iter().all(|x| x.uid != u));
}

fn street_shared(sample: usize) -> Rc<StreetVm> {
    StreetVm::new(crate::shared::platform::browser_ports(), Editor::new(sample), None)
}

fn street_html<V: IntoView + 'static>(f: impl FnOnce() -> V) -> String {
    html(|| f().into_any())
}

fn street_ends() -> Vec<page::StreetEnd> {
    vec![page::StreetEnd { name: "the edge of the map".into(), junction: false, uid: 0 }, page::StreetEnd { name: "Junction 4".into(), junction: true, uid: 2 }]
}

#[test]
fn the_street_section_is_a_labelled_group_holding_a_picture_of_the_street() {
    let s = street_shared(0);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("id=\"wrap\"") && h.contains("aria-label=\"Street cross-section editor\"") && h.contains("tabindex=\"0\""));
    assert!(h.contains("id=\"drawing\"") && h.contains("role=\"img\""));
    assert!(h.contains("Cross-section of Sample Street 1. 6 segments, 18.0 m of 18.0 m. Every metre of the street is used."));
    assert_eq!(count(&h, "data-role=\"handle\""), 5);
    assert_eq!(count(&h, "class=\"seg\""), 6);
}

#[test]
fn the_section_follows_the_units_and_the_selection() {
    let s = street_shared(0);
    s.set_units(Units::Feet);
    let u = segment(&s, 2);
    s.edit(|e| e.select(Some(u)));
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("Street width 59.1 ft") && count(&h, "class=\"sel-box\"") == 1);
}

#[test]
fn the_street_heading_names_the_street_and_says_how_wide_it_is() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(h.contains(">Sample Street 1</h1>") && h.contains("Street cross-section") && h.contains(">18.0 m</b>"));
    assert!(!h.contains("City map") && !h.contains("between"));
}

#[test]
fn a_city_street_s_heading_links_back_and_to_the_junctions_it_runs_between() {
    let s = street_shared(1);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=Some(street_ends())/> });
    assert!(h.contains("href=\"map.html\"") && h.contains("City map"));
    assert!(h.contains("between ") && h.contains("the edge of the map") && h.contains(" and <a href=\"intersection.html?junction=2\">Junction 4</a>"));
    assert_eq!(count(&h, "href=\"intersection.html"), 1, "the edge of the map is not a link");
}

#[test]
fn the_street_s_title_block_gives_its_name_width_and_changes() {
    let s = street_shared(0);
    let u = segment(&s, 0);
    s.edit(|e| e.nudge_width(u, 100));
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(
        h.contains("id=\"tb-street\">Sample Street 1<")
            && h.contains("id=\"tb-row\" class=\"fig\">18.0 m<")
            && h.contains("id=\"tb-changes\" class=\"fig\">1<")
    );
}

#[test]
fn the_street_s_status_line_says_whether_the_pieces_fit() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::Fit vm=s.clone()/> });
    assert!(h.contains("class=\"fit\"") && h.contains("Every metre of the street is used."));
    assert!(h.contains("tick"), "a street whose width is all used carries the tick");
    let u = segment(&s, 0);
    s.edit(|e| e.set_width(u, 3_800));
    let h = street_html(|| view! { <page::Fit vm=s.clone()/> });
    assert!(h.contains("class=\"fit bad\"") && h.contains("0.5 m too wide. Make a piece narrower or remove one."));
    assert!(!h.contains("tick"));
}

#[test]
fn a_street_with_width_left_over_does_not_carry_the_tick() {
    let s = street_shared(0);
    let u = segment(&s, 0);
    s.edit(|e| e.remove(u));
    let h = street_html(|| view! { <page::Fit vm=s.clone()/> });
    assert!(h.contains("class=\"fit\"") && h.contains("still unused") && !h.contains("tick"));
}

#[test]
fn the_street_s_history_buttons_are_off_until_there_is_something_to_undo() {
    let s = street_shared(0);
    let off = |h: &str| ["undo", "redo", "reset"].map(|id| button_tag(h, id).contains("disabled"));
    let h = street_html(|| view! { <page::History vm=s.clone()/> });
    assert_eq!(off(&h), [true, true, true]);
    let u = segment(&s, 0);
    s.edit(|e| e.nudge_width(u, 100));
    let h = street_html(|| view! { <page::History vm=s.clone()/> });
    assert_eq!(off(&h), [false, true, false]);
}

#[test]
fn the_add_menu_is_closed_and_lists_every_piece_in_seven_groups() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::AddMenu vm=s.clone()/> });
    assert!(h.contains("aria-expanded=\"false\"") && h.contains("id=\"add-menu\" hidden class=\"menu add-menu\""));
    assert!(button_tag(&h, "add-btn").contains("aria-haspopup"));
    assert_eq!(count(&h, "class=\"add-item\""), 17);
    assert_eq!(count(&h, "class=\"add-icon\""), 17);
    assert!(h.contains("viewBox=\"0 0 200 160\"") && !h.contains("class=\"swatch\""));
    for (g, group) in ["Walking", "Greenery", "Cycling", "Transit", "Roadway", "Furniture", "Utilities"].into_iter().enumerate() {
        assert!(h.contains(&format!("id=\"add-g-{g}\" class=\"add-group-h\">{group}<")), "{group}");
    }
    assert!(h.contains("3.3 m") && h.contains("Goes after the selected piece."));
    s.set_units(Units::Feet);
    let h = street_html(|| view! { <page::AddMenu vm=s.clone()/> });
    assert!(h.contains("10.8 ft"));
}

#[test]
fn the_clock_is_hidden_until_a_piece_changes_through_the_day() {
    let s = street_shared(1);
    let h = street_html(|| view! { <page::Clock vm=s.clone()/> });
    assert!(h.contains("id=\"clock\" hidden"), "{h}");
    let t = street_html(|| view! { <page::TimeNote vm=s.clone()/> });
    assert!(!t.contains("Numbers are"));
    assert!(s.edit(|e| e.apply_measure("B3")));
    let h = street_html(|| view! { <page::Clock vm=s.clone()/> });
    assert!(!h.contains("id=\"clock\" hidden") && h.contains("id=\"clock\""));
    assert!(h.contains("id=\"time\"") && h.contains("aria-valuetext=\"12:00\"") && h.contains(">12:00</output>"));
    let t = street_html(|| view! { <page::TimeNote vm=s.clone()/> });
    assert!(t.contains("Numbers are for 12:00."));
}

#[test]
fn the_clock_marks_when_the_selected_piece_is_something_else() {
    let s = street_shared(1);
    s.edit(|e| e.apply_measure("B3"));
    let timed = s.view().segments.iter().find(|x| !x.variants.is_empty()).unwrap().uid;
    s.edit(|e| e.select(Some(timed)));
    let h = street_html(|| view! { <page::Clock vm=s.clone()/> });
    assert!(h.contains("class=\"clock-bar on\"") && h.contains("<i style=\"left:"));
    assert!(h.contains("except"));
}

#[test]
fn the_cue_says_how_to_begin_and_can_be_dismissed() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::Welcome vm=s.clone()/> });
    assert!(h.contains("id=\"welcome\"") && h.contains("Add a piece, then drag to arrange.") && h.contains("id=\"welcome-dismiss\""));
}

#[test]
fn the_space_table_compares_each_use_today_and_in_the_design() {
    let s = street_shared(0);
    let u = segment(&s, 0);
    s.edit(|e| e.remove(u));
    let h = street_html(|| view! { <notes::Space vm=s.clone()/> });
    assert!(h.contains("Width by use, in metres") && h.contains("Your design"));
    assert!(h.contains("class=\"down\">\u{2212}3.3<") && h.contains("class=\"zero\">0<"));
    s.set_units(Units::Feet);
    assert!(street_html(|| view! { <notes::Space vm=s.clone()/> }).contains("in feet"));
}

#[test]
fn the_capacity_table_says_how_many_people_move_and_how_that_changed() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::Capacity vm=s.clone()/> });
    assert!(h.contains("People per hour") && h.contains("class=\"zero\">0<"));
    let v = s.view();
    assert!(h.contains(&group_thousands(v.outcomes.capacity_pph.into(), Locale::En)));
    let u = segment(&s, 2);
    s.edit(|e| e.set_width(u, 3_000));
    let h = street_html(|| view! { <notes::Capacity vm=s.clone()/> });
    assert!(h.contains("class=\"down\"") || h.contains("class=\"up\"") || h.contains("class=\"zero\""));
}

#[test]
fn the_checks_each_say_whether_they_pass_in_words() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert_eq!(count(&h, "<li class=\"ok\">"), 4);
    assert!(h.contains(": passes") && h.contains("Every metre is used"));
    let u = segment(&s, 0);
    s.edit(|e| e.set_width(u, 3_800));
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(h.contains("<li class=\"bad\">") && h.contains(": fails") && h.contains("0.5 m too wide. Narrow or remove a piece."));
}

#[test]
fn the_count_of_failing_checks_shows_under_the_drawing_and_on_the_tab_only_when_there_are_some() {
    let s = street_shared(0);
    let pill = street_html(|| view! { <notes::FitChecks vm=s.clone()/> });
    assert!(button_tag(&pill, "fit-checks").contains("hidden"));
    let badge = street_html(|| view! { <notes::ChecksBadge vm=s.clone()/> });
    assert!(badge.contains("hidden") && !badge.contains("sr-only"));
    let u = segment(&s, 0);
    s.edit(|e| e.set_width(u, 3_800));
    let pill = street_html(|| view! { <notes::FitChecks vm=s.clone()/> });
    assert!(!button_tag(&pill, "fit-checks").contains("hidden") && pill.contains("1 check fails"));
    let badge = street_html(|| view! { <notes::ChecksBadge vm=s.clone()/> });
    assert!(badge.contains(">1<") && badge.contains("sr-only"));
}

#[test]
fn the_street_s_changes_start_from_the_street_today_and_list_each_edit() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(h.contains("Street today") && h.contains("base now"));
    let u = segment(&s, 0);
    s.edit(|e| e.remove(u));
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(!h.contains("base now") && h.contains("class=\"now\"") && h.contains(&s.view().revisions[0].label));
}

#[test]
fn the_measures_offer_to_arrange_what_fits_and_say_what_is_present_or_why_not() {
    let s = street_shared(1);
    let h = street_html(|| view! { <notes::Measures vm=s.clone()/> });
    assert_eq!(count(&h, "<tbody>"), 3);
    assert!(count(&h, "class=\"btn m-apply\"") >= 10);
    assert!(h.contains("Arrange the street as B1 Center-Running Transit Lanes"));
    assert!(h.contains("Freeways only") && h.contains("Not modelled") && h.contains("Set at a junction"));
    assert!(h.contains("Lane arrangements along the street."));
    assert!(s.edit(|e| e.apply_measure("B3")));
    let h = street_html(|| view! { <notes::Measures vm=s.clone()/> });
    assert!(h.contains("This street") && !h.contains("Arrange the street as B3"));
}

#[test]
fn nothing_selected_the_street_panel_says_how_to_begin() {
    let s = street_shared(0);
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("Select a piece to change its width and surface."));
}

#[test]
fn a_width_typed_with_a_comma_is_committed() {
    let owner = Owner::new();
    owner.set();
    let (s, w) = street_watch();
    let at = s.view().segments.iter().position(|g| g.min_mm <= 2_500 && g.max_mm >= 2_500 && g.width_mm != 2_500).expect("a piece 2.5 m can fit");
    let u = segment(&s, at);
    assert!(inspector::commit_width(w, u, "2,5"));
    assert_eq!(s.view().segments[at].width_mm, 2_500);
    assert!(!inspector::commit_width(w, u, "abc"));
    assert_eq!(s.view().segments[at].width_mm, 2_500);
}

#[test]
fn a_selected_piece_s_panel_has_its_width_and_surface_and_no_buttons_to_move_it() {
    let s = street_shared(0);
    let u = segment(&s, 2);
    s.edit(|e| e.select(Some(u)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Driving lane</h2>") && h.contains("3.3 m wide \u{b7} 3 of 6"));
    for want in ["Width", "Surface", "What it is paved with.", "Allowed "] {
        assert!(h.contains(want), "{want}");
    }
    for not in ["Position", "Move left", "Move right"] {
        assert!(!h.contains(not), "{not}");
    }
    assert!(h.contains("id=\"width\"") && h.contains("value=\"3.30\"") && h.contains("aria-label=\"Narrower by 0.1 m\""));
    assert_eq!(count(&h, "aria-checked=\"true\""), 2, "a surface and a direction are chosen");
}

#[test]
fn a_planting_strip_is_about_planting_and_a_transit_lane_can_carry_trams() {
    let s = street_shared(1);
    let planting = s.view().segments.iter().find(|x| KINDS[x.kind].id == "planting").unwrap().uid;
    s.edit(|e| e.select(Some(planting)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Planting</h3>") && h.contains("What is planted in it.") && h.contains("Street trees"));
    assert!(!h.contains("Vehicle"));
    assert!(s.edit(|e| e.apply_measure("B1")));
    let bus = s.view().segments.iter().find(|x| KINDS[x.kind].id == "bus").unwrap().uid;
    s.edit(|e| e.select(Some(bus)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("Vehicle") && h.contains(">Bus</button>") && h.contains(">Tram</button>"));
}

#[test]
fn every_setting_of_a_piece_is_shown_with_nothing_to_open() {
    let s = street_shared(1);
    let u = segment(&s, 3);
    s.edit(|e| e.select(Some(u)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(!h.contains("<details") && !h.contains("insp-more") && !h.contains("More about this piece"), "{h}");
    assert!(h.contains(">Other times</h3>") && h.contains("Add other times") && h.contains(">Direction</h3>"));
    s.edit(|e| e.add_variant(u));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert_eq!(count(&h, "class=\"var now\"") + count(&h, "class=\"var\""), 1);
    assert!(h.contains("aria-label=\"Type 1\"") && h.contains("aria-label=\"From\"") && h.contains("aria-label=\"To\""));
}

#[test]
fn a_sidewalk_shows_its_curb_with_the_rest_and_a_planting_strip_has_no_direction() {
    let s = street_shared(0);
    let walk = segment(&s, 0);
    s.edit(|e| e.select(Some(walk)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Curb</h3>") && !h.contains("<details"));
    let s = street_shared(1);
    let planting = s.view().segments.iter().find(|x| KINDS[x.kind].id == "planting").unwrap().uid;
    s.edit(|e| e.select(Some(planting)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(!h.contains(">Direction</h3>") && !h.contains("<details"));
}

#[test]
fn the_days_of_a_window_are_said_in_its_row_and_a_window_of_every_day_says_none() {
    use crate::shared::catalogue::{Side, StreetClass, kind_index};
    use crate::street::model::{Piece, Street, Window};
    let kind = |id| kind_index(id).unwrap();
    let rows = |days: Option<&str>| {
        let window = Window { kind: kind("bus"), from_min: 360, to_min: 600, days: days.map(str::to_string) };
        let street =
            Street::imported(StreetClass::Local, Side::Right, &[Piece { kind: kind("parking"), width_mm: 3000, direction: Some(0), variants: vec![window] }]);
        let s = StreetVm::new(crate::shared::platform::browser_ports(), Editor::from_street(&street, &street, 0), None);
        let u = segment(&s, 0);
        s.edit(|e| e.select(Some(u)));
        street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> })
    };
    let h = rows(Some("Mo-Fr"));
    assert!(h.contains("class=\"var-days\"") && h.contains("Mo-Fr"), "{h}");
    assert!(!rows(None).contains("var-days"));
}

#[test]
fn the_title_block_links_the_osm_ways_and_says_whether_the_street_was_changed() {
    use crate::shared::catalogue::{Side, StreetClass, kind_index};
    use crate::shared::provenance::OsmRef;
    use crate::street::model::{Piece, Street};
    let travel = kind_index("travel").unwrap();
    let mut street = Street::imported(StreetClass::Local, Side::Right, &[Piece { kind: travel, width_mm: 3200, direction: Some(0), variants: Vec::new() }]);
    street.source = vec![OsmRef { id: 4687530, version: Some(49) }, OsmRef { id: 7, version: None }];
    let s = StreetVm::new(crate::shared::platform::browser_ports(), Editor::from_street(&street, &street, 0), None);
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains("href=\"https://www.openstreetmap.org/way/4687530\"") && h.contains(">way 4687530 v49<"), "{h}");
    assert!(h.contains(">way 7<") && h.contains("id=\"tb-state\">As imported<"), "{h}");
    let u = segment(&s, 0);
    s.edit(|e| e.nudge_width(u, 100));
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains("id=\"tb-state\">Edited<"), "{h}");
}

#[test]
fn the_title_block_of_a_sandbox_street_does_not_link_to_openstreetmap() {
    let h = street_html(|| view! { <page::TitleBlock vm=street_shared(0)/> });
    assert!(!h.contains("openstreetmap.org"), "{h}");
}

#[test]
fn a_sidewalk_has_a_curb_to_choose_and_a_driving_lane_has_none() {
    let s = street_shared(0);
    let walk = segment(&s, 0);
    s.edit(|e| e.select(Some(walk)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Curb</h3>") && h.contains("None (flush)") && h.contains("Granite"));
    let lane = segment(&s, 2);
    s.edit(|e| e.select(Some(lane)));
    assert!(!street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> }).contains(">Curb</h3>"));
}

#[test]
fn the_panel_s_lengths_and_steps_follow_the_units() {
    let s = street_shared(0);
    s.set_units(Units::Feet);
    let u = segment(&s, 2);
    s.edit(|e| e.select(Some(u)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("10.8 ft wide") && h.contains("aria-label=\"Narrower by 1.0 ft\"") && h.contains("step=\"0.25\""));
    assert!(h.contains("Allowed ") && h.contains(" ft"));
}
