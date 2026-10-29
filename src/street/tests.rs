//! The street page's keys and components drawn to HTML on the host, and read as
//! the page would show them.

use std::rc::Rc;

use leptos::prelude::*;

use crate::city::model::EndView;
use crate::shared::catalogue::KINDS;
use crate::shared::i18n::Locale;
use crate::shared::live;
use crate::shared::testing::{button_tag, count, html};
use crate::shared::units::{Units, group_thousands};
use crate::street::keys::Action as StreetAction;
use crate::street::model::Editor;
use crate::street::vm::{StreetVm, ViewMode};
use crate::street::watch::SheetWatch;
use crate::street::{inspector, notes, page, view};
// ---- what the keys do on the street ---------------------------------------------------

fn street_watch() -> (Rc<StreetVm>, SheetWatch) {
    let s = StreetVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::En), Editor::new(0), None);
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
    StreetVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::En), Editor::new(sample), None)
}

fn street_html<V: IntoView + 'static>(f: impl FnOnce() -> V) -> String {
    html(|| f().into_any())
}

fn street_ends() -> Vec<EndView> {
    use crate::shared::said::{Arg, Said};
    vec![
        EndView { name: Said::new("city-the-edge-of-the-map"), junction: false, uid: 0 },
        EndView { name: Said::new("city-junction-number").with("n", Arg::Num(4)), junction: true, uid: 2 },
    ]
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

fn street_in(mode: ViewMode, locale: Locale) -> Rc<StreetVm> {
    let (ports, _, _) = crate::shared::ports::test_ports();
    let s = StreetVm::new(ports, crate::i18n_for(locale), Editor::new(0), None);
    s.set_mode(mode);
    s
}

#[test]
fn in_2_5d_the_drawing_is_the_street_in_depth_with_no_handles_and_the_keys_still_reach_it() {
    let s = street_in(ViewMode::Iso, Locale::En);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    // The keyboard still edits the street through the group around the drawing.
    // ...named for the view it shows, not as a cross-section.
    assert!(h.contains("id=\"wrap\"") && h.contains("tabindex=\"0\"") && h.contains("aria-label=\"Street editor, 2.5D view\""), "{h}");
    assert!(!h.to_lowercase().contains("cross-section"), "{h}");
    assert!(h.contains("id=\"drawing\"") && h.contains("role=\"img\""));
    assert!(h.contains("aria-label=\"Sample Street 1 in 2.5D, along a 40 m stretch. 6 segments.\""), "{h}");
    assert_eq!(count(&h, "class=\"iso-top "), 6);
    for none in ["data-role=\"handle\"", "data-role=\"edge\"", "class=\"seg\"", "class=\"sel-box\""] {
        assert_eq!(count(&h, none), 0, "{none}");
    }
}

#[test]
fn the_group_is_as_tall_as_the_section_in_both_views_so_switching_does_not_make_the_card_jump() {
    use crate::street::iso::iso_height;
    use crate::street::svg::Geometry;
    for mode in [ViewMode::Section, ViewMode::Iso] {
        let s = street_in(mode, Locale::En);
        let v = s.view();
        let section = Geometry::fit(1050.0, v.row_mm).rows.height;
        // The group keeps the taller of the two, whichever view is shown, and the 2.5D is never the taller.
        assert!(iso_height(&v, 1050.0) <= section);
        let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
        assert!(h.contains(&format!("min-height:{section}px")), "{mode:?} {section} {h}");
    }
}

#[test]
fn only_the_section_keeps_the_group_600_px_wide_the_2_5d_fits_the_room_it_has() {
    let section = street_html(|| view! { <view::StreetDrawing vm=street_in(ViewMode::Section, Locale::En)/> });
    let iso = street_html(|| view! { <view::StreetDrawing vm=street_in(ViewMode::Iso, Locale::En)/> });
    assert!(!section.contains("iso-view"), "{section}");
    assert!(iso.contains("iso-view"), "{iso}");
}

#[test]
fn in_french_the_2_5d_group_is_named_for_the_2_5d() {
    let s = street_in(ViewMode::Iso, Locale::FrCa);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("aria-label=\"Éditeur de la rue, vue 2,5D\"") && !h.contains("coupe"), "{h}");
}

#[test]
fn the_drawing_s_heading_names_the_view_shown_in_each_language() {
    for (locale, section, iso) in [(Locale::En, "Cross-section", "Street in 2.5D"), (Locale::FrCa, "Coupe transversale", "Rue en 2,5D")] {
        let s = street_in(ViewMode::Section, locale);
        assert_eq!(street_html(|| view! { <page::DrawingHeading vm=s.clone()/> }), section);
        s.set_mode(ViewMode::Iso);
        assert_eq!(street_html(|| view! { <page::DrawingHeading vm=s.clone()/> }), iso);
    }
}

#[test]
fn in_the_section_the_drawing_is_unchanged() {
    let s = street_in(ViewMode::Section, Locale::En);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert_eq!(count(&h, "data-role=\"handle\""), 5);
    assert_eq!(count(&h, "class=\"iso-top "), 0);
}

#[test]
fn the_2_5d_drawing_follows_the_selection() {
    let s = street_in(ViewMode::Iso, Locale::En);
    let draw = || street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    let outline = |h: &str| h.find("class=\"iso-sel\"").map(|at| h[at..at + h[at..].find("/>").unwrap()].to_string());
    // Nothing selected: no outline.
    assert_eq!(outline(&draw()), None);
    // A piece selected: its outline; another: the outline moves to it; none: it goes.
    s.edit(|e| e.select(Some(segment(&s, 2))));
    let second = outline(&draw()).expect("the selected piece is outlined");
    s.edit(|e| e.select(Some(segment(&s, 4))));
    let fourth = outline(&draw()).expect("the newly selected piece is outlined");
    assert_ne!(second, fourth);
    s.edit(|e| e.select(None));
    assert_eq!(count(&draw(), "class=\"iso-sel\""), 0);
}

#[test]
fn the_2_5d_drawing_follows_the_language_the_shell_switches() {
    let s = street_in(ViewMode::Iso, Locale::En);
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("<title>Trottoir</title>") && h.contains("en 2,5D, sur un tronçon de"), "{h}");
    assert!(!h.contains("<title>Sidewalk</title>"), "{h}");
}

#[test]
fn the_view_toggle_is_a_group_of_two_buttons_and_the_view_shown_is_pressed() {
    let s = street_in(ViewMode::Section, Locale::En);
    let h = street_html(|| view! { <page::ViewToggle vm=s.clone()/> });
    assert!(h.contains("<fieldset") && h.contains("id=\"view-mode\""), "{h}");
    assert!(h.contains(">Show the street as</legend>"), "{h}");
    assert!(button_tag(&h, "view-section").contains("aria-pressed=\"true\""));
    assert!(button_tag(&h, "view-iso").contains("aria-pressed=\"false\""));
    // V goes to the view not shown, so only that button carries it.
    assert!(button_tag(&h, "view-iso").contains("aria-keyshortcuts=\"V\""));
    assert!(!button_tag(&h, "view-section").contains("aria-keyshortcuts"));
    assert!(h.contains(">Section</button>") && h.contains(">2.5D</button>"), "{h}");
    s.set_mode(ViewMode::Iso);
    let h = street_html(|| view! { <page::ViewToggle vm=s.clone()/> });
    assert!(button_tag(&h, "view-iso").contains("aria-pressed=\"true\""));
    assert!(button_tag(&h, "view-section").contains("aria-pressed=\"false\""));
    assert!(button_tag(&h, "view-section").contains("aria-keyshortcuts=\"V\""));
    assert!(!button_tag(&h, "view-iso").contains("aria-keyshortcuts"));
}

#[test]
fn the_view_toggle_reads_coupe_and_2_5d_in_french() {
    let s = street_in(ViewMode::Section, Locale::FrCa);
    let h = street_html(|| view! { <page::ViewToggle vm=s.clone()/> });
    assert!(h.contains(">Coupe</button>") && h.contains(">2,5D</button>") && h.contains(">Afficher la rue en</legend>"), "{h}");
}

#[test]
fn the_street_heading_names_the_street_and_says_how_wide_it_is() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(h.contains(">Sample Street 1</h1>") && h.contains("Street cross-section") && h.contains(">18.0 m</b>"));
    assert!(!h.contains("City map") && !h.contains("between"));
}

#[test]
fn a_street_without_a_name_is_headed_by_its_class_in_the_language_of_the_page() {
    use crate::shared::catalogue::Side;
    use crate::street::model::Street;
    let unnamed = Street { name: None, ..Street::sample(0, Side::Right) };
    let s = StreetVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::En), Editor::from_street(&unnamed, &unnamed, 0), None);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(h.contains(">Unnamed local street</h1>"), "{h}");
    let b = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(b.contains(">Unnamed local street</b>"), "{b}");
    let d = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(d.contains("Cross-section of Unnamed local street."), "{d}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(h.contains(">Rue locale sans nom</h1>"), "{h}");
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
    // A disclosure of a panel of buttons, not a menu: no popup role, and the palette is groups of lists (no
    // list that holds anything but items).
    assert!(!button_tag(&h, "add-btn").contains("aria-haspopup"));
    assert!(!h.contains("role=\"presentation\"") && !h.contains("<ul id=\"palette\""));
    assert_eq!(count(&h, "role=\"group\""), 7);
    assert_eq!(count(&h, "class=\"add-item\""), 18);
    assert_eq!(count(&h, "class=\"add-icon\""), 18);
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
fn the_space_table_has_a_buffer_row_only_on_a_street_that_has_a_buffer() {
    let s = street_shared(0);
    let none = street_html(|| view! { <notes::Space vm=s.clone()/> });
    assert!(!none.contains("mc-neutral") && !none.contains("Buffer"));
    s.edit(|e| {
        e.add(crate::shared::catalogue::kind_index("buffer").unwrap(), 3);
        true
    });
    let h = street_html(|| view! { <notes::Space vm=s.clone()/> });
    assert!(h.contains("mc-neutral") && h.contains("Buffer"), "{h}");
    assert!(h.contains("class=\"up\">+0.6<"), "the design has 0.6 m more of it than today");
}

#[test]
fn the_capacity_table_says_how_many_people_move_and_how_that_changed() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::Capacity vm=s.clone()/> });
    assert!(h.contains("People per hour") && h.contains("class=\"zero\">0<"));
    // No header cell is empty: the first column is the measure.
    assert!(h.contains(">Measure<") && !h.contains("<th scope=\"col\"></th>"), "{h}");
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
    assert!(!h.contains("base now") && h.contains("class=\"now\"") && h.contains("<td>Remove sidewalk</td>"), "{h}");
}

#[test]
fn the_changes_list_is_drawn_again_in_the_language_the_shell_switches() {
    let owner = Owner::new();
    owner.set();
    let s = street_shared_in(0, Locale::En);
    let u = segment(&s, 0);
    s.edit(|e| e.remove(u));
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(h.contains("<td>Remove sidewalk</td>") && h.contains("Street today"), "{h}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(h.contains("<td>Retrait\u{a0}: trottoir</td>") && h.contains("La rue aujourd’hui"), "{h}");
    assert!(!h.contains("Remove sidewalk"), "{h}");
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
fn the_arrow_keys_step_the_width_field() {
    assert_eq!(inspector::width_key("ArrowUp"), Some(1));
    assert_eq!(inspector::width_key("ArrowDown"), Some(-1));
    assert_eq!(inspector::width_key("Enter"), None);
}

#[test]
fn enter_and_escape_end_the_width_field_and_give_the_keys_back_to_the_drawing() {
    use inspector::FieldEnd;
    assert_eq!(inspector::width_end("Enter"), Some(FieldEnd::Commit));
    assert_eq!(inspector::width_end("Escape"), Some(FieldEnd::Revert));
    assert_eq!(inspector::width_end("ArrowUp"), None);
    assert_eq!(inspector::width_end("a"), None);
}

#[test]
fn the_width_field_is_text_so_a_comma_reaches_it() {
    let s = street_shared(0);
    let u = segment(&s, 2);
    s.edit(|e| e.select(Some(u)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("id=\"width\"") && h.contains("type=\"text\"") && h.contains("inputmode=\"decimal\""));
    assert!(!h.contains("type=\"number\""));
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
        let s = StreetVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::En), Editor::from_street(&street, &street, 0), None);
        let u = segment(&s, 0);
        s.edit(|e| e.select(Some(u)));
        street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> })
    };
    let h = rows(Some("Mo-Fr"));
    assert!(h.contains("class=\"var-days\"") && h.contains("Mo-Fr"), "{h}");
    assert!(!rows(None).contains("var-days"));
}

fn osm_street_shared(locale: Locale) -> Rc<StreetVm> {
    use crate::shared::catalogue::{Side, StreetClass, kind_index};
    use crate::shared::provenance::OsmRef;
    use crate::street::model::{Piece, Street};
    let travel = kind_index("travel").unwrap();
    let mut street = Street::imported(StreetClass::Local, Side::Right, &[Piece { kind: travel, width_mm: 3200, direction: Some(0), variants: Vec::new() }]);
    street.source = vec![OsmRef { id: 4687530, version: Some(49) }, OsmRef { id: 7, version: None }];
    StreetVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(locale), Editor::from_street(&street, &street, 0), None)
}

#[test]
fn the_provenance_links_the_osm_ways_and_says_whether_the_street_was_changed() {
    let s = osm_street_shared(Locale::En);
    let h = street_html(|| view! { <page::Provenance vm=s.clone()/> });
    assert!(h.contains("href=\"https://www.openstreetmap.org/way/4687530\"") && h.contains(">way 4687530 v49<"), "{h}");
    assert!(h.contains(">way 7<") && h.contains("id=\"tb-state\">As imported<"), "{h}");
    let u = segment(&s, 0);
    s.edit(|e| e.nudge_width(u, 100));
    let h = street_html(|| view! { <page::Provenance vm=s.clone()/> });
    assert!(h.contains("id=\"tb-state\">Edited<"), "{h}");
}

#[test]
fn a_street_with_no_source_says_so() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::Provenance vm=s.clone()/> });
    assert!(h.contains("This street was not made from OpenStreetMap."), "{h}");
    assert!(!h.contains("openstreetmap.org") && !h.contains("tb-state"), "{h}");
    let fr = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <page::Provenance vm=fr.clone()/> });
    assert!(h.contains("Cette rue ne vient pas d’OpenStreetMap.") && !h.contains("was not made"), "{h}");
}

#[test]
fn the_provenance_follows_the_language_switch() {
    let owner = Owner::new();
    owner.set();
    let s = osm_street_shared(Locale::En);
    let h = street_html(|| view! { <page::Provenance vm=s.clone()/> });
    assert!(h.contains(">Data<") && h.contains("As imported"), "{h}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <page::Provenance vm=s.clone()/> });
    assert!(h.contains(">Données<") && h.contains("Tel qu’importé") && !h.contains("As imported"), "{h}");
}

#[test]
fn the_title_block_no_longer_holds_the_sources() {
    let s = osm_street_shared(Locale::En);
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(!h.contains("openstreetmap.org") && !h.contains("tb-state") && !h.contains("OpenStreetMap"), "{h}");
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
    assert!(h.contains("10.8 ft wide") && h.contains("aria-label=\"Narrower by 1.0 ft\""));
    assert!(h.contains("Allowed ") && h.contains(" ft"));
}

// ---- the notes in French ----------------------------------------------------------------

fn street_shared_in(sample: usize, locale: Locale) -> Rc<StreetVm> {
    StreetVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(locale), Editor::new(sample), None)
}

#[test]
fn the_space_and_capacity_tables_are_in_french() {
    let s = street_shared_in(0, Locale::FrCa);
    let u = segment(&s, 0);
    s.edit(|e| e.remove(u));
    let h = street_html(|| view! { <notes::Space vm=s.clone()/> });
    assert!(h.contains("Largeur par usage, en mètres") && h.contains("Votre aménagement") && h.contains("Marche"));
    assert!(!h.contains("Your design") && !h.contains("Width by use"));
    let h = street_html(|| view! { <notes::Capacity vm=s.clone()/> });
    assert!(h.contains("Personnes par heure") && !h.contains("People per hour"));
}

#[test]
fn the_checks_and_their_count_are_in_french() {
    let s = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(h.contains("\u{a0}: réussite") && h.contains("Chaque mètre est utilisé") && !h.contains(": passes"));
    let u = segment(&s, 0);
    s.edit(|e| e.set_width(u, 3_800));
    let pill = street_html(|| view! { <notes::FitChecks vm=s.clone()/> });
    assert!(pill.contains("1 vérification échoue") && !pill.contains("1 check fails"));
    let badge = street_html(|| view! { <notes::ChecksBadge vm=s.clone()/> });
    assert!(badge.contains("en échec") && !badge.contains(" fail"));
}

#[test]
fn the_changes_are_in_french() {
    let s = street_shared_in(1, Locale::FrCa);
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(h.contains("La rue aujourd’hui") && h.contains("Étape") && !h.contains("Street today"));
}

#[test]
fn the_notes_are_drawn_again_in_the_language_the_shell_switches() {
    let owner = Owner::new();
    owner.set();
    let s = street_shared_in(0, Locale::En);
    let i18n = s.i18n().clone();
    // The same instance the view-model holds, so switching it changes what the notes say.
    let h = street_html(|| view! { <notes::Capacity vm=s.clone()/> });
    assert!(h.contains("People per hour"));
    i18n.set(Locale::FrCa);
    let h = street_html(|| view! { <notes::Capacity vm=s.clone()/> });
    assert!(h.contains("Personnes par heure"));
}

#[test]
fn the_checks_and_their_count_are_said_in_english_with_no_stray_space() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(h.contains("<span class=\"sr-only\">: passes</span>"), "{h}");
    let u = segment(&s, 0);
    s.edit(|e| e.set_width(u, 3_800));
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(h.contains("<span class=\"sr-only\">: fails</span>"), "{h}");
    let badge = street_html(|| view! { <notes::ChecksBadge vm=s.clone()/> });
    assert!(badge.contains("<span class=\"sr-only\"> fail</span>"), "{badge}");
}

#[test]
fn the_checks_are_said_in_french_and_follow_the_switch() {
    let owner = Owner::new();
    owner.set();
    let s = street_shared_in(0, Locale::En);
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(h.contains("Fits the street width") && h.contains("Both sides") && h.contains("Room for emergency vehicles"), "{h}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(h.contains("Respecte la largeur de la rue") && h.contains("Des deux côtés") && h.contains("Place pour les véhicules d’urgence"), "{h}");
    assert!(!h.contains("Fits the street width") && !h.contains("Both sides"), "{h}");
}

// ---- the page around the section, in French ------------------------------------------------

#[test]
fn the_heading_and_the_title_block_are_in_french() {
    let s = street_shared_in(1, Locale::FrCa);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=Some(street_ends())/> });
    assert!(h.contains("Carte de la ville") && h.contains("Coupe transversale de la rue") && h.contains("de large"), "{h}");
    assert!(h.contains("entre ") && h.contains("la limite de la carte") && h.contains(" et <a href=\"intersection.html?junction=2\">Jonction 4</a>"), "{h}");
    assert!(!h.contains("City map") && !h.contains("Street cross-section") && !h.contains("between") && !h.contains(" wide"), "{h}");
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains(">Rue<") && h.contains(">Largeur<") && h.contains("Modifications apportées"), "{h}");
    assert!(!h.contains(">Width<") && !h.contains("Changes made"), "{h}");
}

#[test]
fn the_drawing_s_group_is_named_in_the_language_of_the_page() {
    let s = street_shared(0);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("aria-label=\"Street cross-section editor\""), "{h}");
    let s = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("aria-label=\"Éditeur de coupe transversale de la rue\"") && !h.contains("Street cross-section editor"), "{h}");
}

#[test]
fn the_history_buttons_are_in_french() {
    let s = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <page::History vm=s.clone()/> });
    assert!(h.contains("Annuler") && h.contains("Rétablir") && h.contains("Recommencer") && h.contains("Outils de la feuille"), "{h}");
    assert!(!h.contains(">Undo<") && !h.contains("Start over") && !h.contains("Sheet tools"), "{h}");
}

#[test]
fn the_add_menu_is_in_french() {
    let s = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <page::AddMenu vm=s.clone()/> });
    assert!(h.contains("Ajouter un élément") && h.contains("Se place après l’élément sélectionné."), "{h}");
    assert!(h.contains("class=\"add-group-h\">Marche<") && h.contains("<b>Trottoir</b>"), "{h}");
    assert!(!h.contains("Add a piece") && !h.contains("Goes after") && !h.contains(">Walking<") && !h.contains(">Sidewalk<"), "{h}");
}

#[test]
fn the_clock_and_the_welcome_are_in_french() {
    let s = street_shared_in(1, Locale::FrCa);
    s.edit(|e| e.apply_measure("B3"));
    let timed = s.view().segments.iter().find(|x| !x.variants.is_empty()).unwrap().uid;
    s.edit(|e| e.select(Some(timed)));
    let h = street_html(|| view! { <page::Clock vm=s.clone()/> });
    assert!(h.contains("Heure de la journée") && h.contains(" sauf "), "{h}");
    assert!(!h.contains("Time of day") && !h.contains("except"), "{h}");
    let t = street_html(|| view! { <page::TimeNote vm=s.clone()/> });
    assert!(t.contains("Les chiffres valent pour 12:00."), "{t}");
    let h = street_html(|| view! { <page::Welcome vm=s.clone()/> });
    assert!(h.contains("Ajoutez un élément") && h.contains("Compris") && !h.contains("Got it"), "{h}");
}

#[test]
fn the_clock_note_names_the_days_in_each_language() {
    let en = crate::i18n_for(Locale::En);
    let fr = crate::i18n_for(Locale::FrCa);
    let mut e = Editor::new(1);
    assert!(e.apply_measure("B3"));
    let mut seg = e.view().segments.into_iter().find(|x| !x.variants.is_empty()).unwrap();
    seg.base_kind = KINDS.iter().position(|k| k.id == "sidewalk").unwrap();
    seg.variants[0].kind = KINDS.iter().position(|k| k.id == "bike").unwrap();
    seg.variants[0].from_min = 420;
    seg.variants[0].to_min = 600;
    seg.variants[0].days = Some("Mo-Fr,Su".into());
    seg.variants.truncate(1);
    assert_eq!(page::clock_note(&seg, &en), "Sidewalk except bike lane Mo-Fr,Su 07:00\u{2013}10:00");
    assert_eq!(page::clock_note(&seg, &fr), "Trottoir sauf piste cyclable lun.-ven.,dim. 07:00\u{2013}10:00");
}

#[test]
fn the_window_title_names_the_street_and_where_it_runs_in_each_language() {
    let en = crate::i18n_for(Locale::En);
    let fr = crate::i18n_for(Locale::FrCa);
    let ends = ["Rue A".to_string(), "Rue B".to_string()];
    assert_eq!(page::page_title(&en, "Main St", &ends), "Main St between Rue A and Rue B \u{b7} CityLoom");
    assert_eq!(page::page_title(&fr, "Main St", &ends), "Main St entre Rue A et Rue B \u{b7} CityLoom");
    assert_eq!(page::page_title(&en, "Main St", &[]), "CityLoom street editor");
    assert_eq!(page::page_title(&fr, "Main St", &[]), "Éditeur de rues CityLoom");
}

// ---- the piece panel -------------------------------------------------------------------

fn select_in(sample: usize, locale: Locale, pick: impl Fn(&StreetVm) -> u32) -> Rc<StreetVm> {
    let s = street_shared_in(sample, locale);
    let u = pick(&s);
    s.edit(|e| e.select(Some(u)));
    s
}

#[test]
fn the_piece_panel_is_in_french() {
    let s = select_in(0, Locale::FrCa, |s| segment(s, 2));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Voie de circulation</h2>") && h.contains("3,3\u{a0}m de large · 3 sur 6"), "{h}");
    assert!(h.contains(">Largeur</h3>") && h.contains(">Revêtement</h3>") && h.contains("Le matériau qui le recouvre."), "{h}");
    assert!(h.contains("Asphalte") && h.contains("Valeurs permises\u{a0}: de ") && h.contains(">Direction</h3>"), "{h}");
    assert!(h.contains("aria-label=\"Rétrécir de 0,1\u{a0}m\"") && h.contains("aria-label=\"Élargir de 0,1\u{a0}m\""), "{h}");
    assert!(h.contains("S’éloigne de vous"), "{h}");
    for en in ["Width", "Surface", "What it is paved with.", "Allowed ", "Narrower by", "Driving lane", "Away from you", "wide"] {
        assert!(!h.contains(en), "{en}");
    }
}

#[test]
fn the_width_field_and_its_range_follow_the_decimal_mark_of_the_language_and_round_trip() {
    let owner = Owner::new();
    owner.set();
    let s = select_in(0, Locale::FrCa, |s| segment(s, 2));
    let w = SheetWatch::new(s.clone());
    let u = segment(&s, 2);
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("value=\"3,30\""), "{h}");
    let (min, max) = (s.view().segments[2].min_mm, s.view().segments[2].max_mm);
    let (lo, hi) = (Units::Metres.fixed_in(min, 2, Locale::FrCa), Units::Metres.fixed_in(max, 2, Locale::FrCa));
    assert!(lo.contains(',') && hi.contains(','));
    assert!(h.contains(&format!("de {lo} à {hi}\u{a0}m")), "{h}");
    // What the field holds, typed back, is the same width.
    let typed = Units::Metres.fixed_in(2_700, 2, Locale::FrCa);
    assert_eq!(typed, "2,70");
    assert!(inspector::commit_width(w, u, &typed));
    assert_eq!(s.view().segments[2].width_mm, 2_700);
    assert_eq!(Units::Metres.fixed_in(s.view().segments[2].width_mm, 2, Locale::FrCa), typed);
}

#[test]
fn the_empty_panel_the_vehicle_and_the_other_settings_are_in_french() {
    let s = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("Sélectionnez un élément pour modifier sa largeur et son revêtement.") && !h.contains("Select a piece"), "{h}");
    let s = street_shared_in(1, Locale::FrCa);
    assert!(s.edit(|e| e.apply_measure("B1")));
    let bus = s.view().segments.iter().find(|x| KINDS[x.kind].id == "bus").unwrap().uid;
    s.edit(|e| e.select(Some(bus)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Véhicule</h3>") && h.contains(">Autobus</button>") && h.contains(">Tramway</button>") && !h.contains("Vehicle"), "{h}");
    let s = select_in(1, Locale::FrCa, |s| segment(s, 3));
    let u = segment(&s, 3);
    s.edit(|e| e.add_variant(u));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Autres périodes</h3>") && h.contains("Ajouter d’autres périodes") && h.contains("le reste de la journée."), "{h}");
    assert!(h.contains("aria-label=\"Type 1\"") && h.contains("aria-label=\"De\"") && h.contains("aria-label=\"À\""), "{h}");
    assert!(!h.contains("Other times") && !h.contains("Add other times") && !h.contains("aria-label=\"From\""), "{h}");
    let walk = select_in(0, Locale::FrCa, |s| segment(s, 0));
    let h = street_html(|| view! { <inspector::StreetInspector vm=walk.clone()/> });
    assert!(h.contains(">Bordure</h3>") && h.contains("Aucune (au ras du sol)") && h.contains("Granit") && !h.contains("None (flush)"), "{h}");
}

#[test]
fn the_panel_is_drawn_again_in_the_language_the_shell_switches() {
    let owner = Owner::new();
    owner.set();
    let s = select_in(0, Locale::En, |s| segment(s, 2));
    let i18n = s.i18n().clone();
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Driving lane</h2>") && h.contains("value=\"3.30\""));
    i18n.set(Locale::FrCa);
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Voie de circulation</h2>") && h.contains("value=\"3,30\"") && !h.contains("value=\"3.30\""), "{h}");
}

#[test]
fn the_drawing_is_in_french_when_the_page_is() {
    let s = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("Aujourd’hui") && h.contains("Votre aménagement") && h.contains("Coupe transversale de"), "{h}");
    assert!(!h.contains(">Today<") && !h.contains("Cross-section of"), "{h}");
    let e = street_shared(0);
    let h = street_html(|| view! { <view::StreetDrawing vm=e.clone()/> });
    assert!(h.contains(">Today<"), "{h}");
    e.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <view::StreetDrawing vm=e.clone()/> });
    assert!(h.contains(">Aujourd’hui<") && !h.contains(">Today<"), "{h}");
}

#[test]
fn the_group_ids_are_those_of_the_add_menu() {
    let menu: Vec<&str> = page::ADD_GROUPS.iter().map(|(id, _)| *id).collect();
    assert_eq!(menu, crate::shared::catalogue::GROUP_IDS);
}

#[test]
fn the_lengths_around_the_section_follow_the_language() {
    let s = street_shared_in(1, Locale::FrCa);
    let row = s.view().row_mm;
    let fr = Units::Metres.length_fine_in(row, Locale::FrCa);
    let en = Units::Metres.length_fine_in(row, Locale::En);
    assert!(fr.contains(',') && fr.contains('\u{a0}') && en.contains('.'), "{fr} {en}");
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(h.contains(&format!(">{fr}<")) && !h.contains(&en), "{h}");
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains(&format!(">{fr}<")) && !h.contains(&en), "{h}");
    let h = street_html(|| view! { <page::AddMenu vm=s.clone()/> });
    let sidewalk = KINDS.iter().find(|k| k.id == "sidewalk").expect("a sidewalk kind").default_mm;
    let fr_dw = Units::Metres.length_fine_in(sidewalk, Locale::FrCa);
    let en_dw = Units::Metres.length_fine_in(sidewalk, Locale::En);
    assert!(h.contains(&format!("class=\"dw\">{fr_dw}<")) && !h.contains(&format!("class=\"dw\">{en_dw}<")), "{h}");
    // the English page keeps its point
    let e = street_shared(1);
    let h = street_html(|| view! { <page::TitleBlock vm=e.clone()/> });
    assert!(h.contains(&format!(">{en}<")), "{h}");
}

#[test]
fn the_street_ftl_files_have_the_same_messages_and_variables() {
    use crate::shared::i18n::tests::parity_problems;
    assert_eq!(parity_problems(crate::street::RESOURCES.en, crate::street::RESOURCES.fr), Vec::<String>::new());
}

fn bendville_street(areas: &[(&str, crate::restriction::model::Rule)], locale: Locale) -> Rc<StreetVm> {
    use crate::restriction::fixtures::{bent_midpoint, bent_store, square};
    let (ports, _, storage) = crate::shared::ports::test_ports();
    let store = bent_store(storage.clone());
    let (_, mid) = bent_midpoint(&store);
    for (name, rule) in areas {
        crate::restriction::store::RestrictionStore::new(storage.clone()).add(name, *rule, square(mid, 0.0005)).unwrap();
    }
    let street = store.open().street_editor(1, 0).unwrap();
    let binding = crate::city::binding::CityBinding { store, place: crate::city::binding::Place::Street(1) };
    StreetVm::new(ports, crate::i18n_for(locale), street, Some(binding))
}

#[test]
fn each_area_holding_the_street_is_a_check_and_a_failing_one_is_counted() {
    use crate::restriction::model::Rule;
    let vm = bendville_street(&[("Car zone", Rule::CarFree), ("Slow zone", Rule::SpeedLimit { kmh: 30 })], Locale::En);
    let h = street_html(|| view! { <notes::Checks vm=vm.clone()/> });
    assert!(h.contains("Area rule: Car zone") && h.contains("Car-free: the street does not keep to it.") && h.contains("It has 2 driving lanes."), "{h}");
    assert!(h.contains("Area rule: Slow zone") && h.contains("Speed not known: OpenStreetMap gives none and none is set."), "{h}");
    let editor_failing = vm.view_now().checks.iter().filter(|c| !c.ok).count();
    let n = editor_failing + 1;
    let pill = street_html(|| view! { <notes::FitChecks vm=vm.clone()/> });
    assert!(pill.contains(&format!("{n} check")), "the car-free row counts, a speed limit with no speed does not: {pill}");
    let badge = street_html(|| view! { <notes::ChecksBadge vm=vm.clone()/> });
    assert!(badge.contains(&format!("id=\"checks-n\"")) && badge.contains(&format!(">{n}<span class=\"sr-only\">")), "{badge}");
}

#[test]
fn the_area_rows_are_said_in_french_too() {
    use crate::restriction::model::Rule;
    let vm = bendville_street(&[("Zone piétonne", Rule::CarFree), ("Zone lente", Rule::SpeedLimit { kmh: 30 })], Locale::FrCa);
    let h = street_html(|| view! { <notes::Checks vm=vm.clone()/> });
    assert!(h.contains("Règle de zone\u{a0}: Zone piétonne") && h.contains("la rue ne la respecte pas."), "{h}");
    assert!(h.contains("<span class=\"sr-only\">\u{a0}: échec</span>") && h.contains("Vitesse inconnue"), "{h}");
    assert!(!h.contains("Area rule"), "{h}");
    let editor_failing = vm.view_now().checks.iter().filter(|c| !c.ok).count();
    let n = editor_failing + 1;
    let pill = street_html(|| view! { <notes::FitChecks vm=vm.clone()/> });
    assert!(pill.contains(&n.to_string()), "{pill}");
    let badge = street_html(|| view! { <notes::ChecksBadge vm=vm.clone()/> });
    assert!(badge.contains(&format!(">{n}<span class=\"sr-only\">")), "{badge}");
}

#[test]
fn a_street_in_no_area_has_no_area_row() {
    let vm = bendville_street(&[], Locale::En);
    assert!(!street_html(|| view! { <notes::Checks vm=vm.clone()/> }).contains("Area rule"));
}

#[test]
fn the_pedestrian_street_messages_exist_in_both_languages() {
    use crate::shared::i18n::{Args, Locale};
    for locale in Locale::ALL {
        let i18n = crate::i18n_for(locale);
        for id in [
            "make-pedestrian-button",
            "make-pedestrian-hint",
            "rev-made-pedestrian",
            "street-now-pedestrian",
            "street-traffic-again",
            "check-access-ped-ok",
            "check-access-ped-bad",
            "check-detail-no-edges-pedestrian",
        ] {
            assert!(i18n.has_message(id, locale), "{id} in {}", locale.tag());
        }
    }
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    let none = Args::new();
    assert_eq!(
        (en.tr_now("rev-made-pedestrian", &none), fr.tr_now("rev-made-pedestrian", &none)),
        ("Made pedestrian".to_string(), "Rendue piétonne".to_string())
    );
    assert_eq!(fr.tr_now("make-pedestrian-button", &none), "Rendre piétonne");
    let street = Args::new().str("street", "Rue Rachel");
    assert_eq!(en.tr_now("street-now-pedestrian", &street), "Rue Rachel is now a pedestrian street.");
    assert_eq!(fr.tr_now("street-traffic-again", &street), "Rue Rachel accueille de nouveau la circulation.");
}

#[test]
fn the_transform_menu_has_a_group_for_making_pedestrian_and_one_for_the_measures_that_fit() {
    use crate::shared::catalogue::kind_index;
    let s = street_shared(1);
    let h = street_html(|| view! { <page::Transform vm=s.clone()/> });
    let button = button_tag(&h, "transform-btn");
    assert!(
        button.contains("aria-haspopup=\"menu\"") && button.contains("aria-controls=\"transform-menu\"") && button.contains("aria-expanded=\"false\""),
        "{button}"
    );
    assert!(h.contains(">Transform<") && h.contains("id=\"transform-menu\"") && h.contains("role=\"menu\"") && h.contains("hidden"), "{h}");
    assert_eq!(count(&h, "role=\"group\""), 2, "{h}");
    assert!(h.contains("Make pedestrian") && h.contains("Transit priority"), "{h}");
    for kind in ["Sidewalk", "Planting strip", "Café terrace"] {
        assert!(h.contains(kind), "{kind}");
    }
    assert_eq!(count(&h, "role=\"menuitem\""), count(&h, "transform-item"));
    // The mark and the name are apart in the text, so that a visible label read as one word is not what the
    // item's name has to contain (WCAG 2.5.3).
    assert_eq!(count(&h, "</span> <b>"), count(&h, "role=\"menuitem\""), "{h}");
    // A menu holds only menu items and groups: each group's heading and hint are hidden from assistive technology and
    // reach it as the group's name and description.
    for (group, heading, hint) in [("ped", "transform-ped-h", "transform-ped-hint"), ("transit", "transform-transit-h", "transform-transit-hint")] {
        let tag = &h[h[..h.find(&format!("aria-labelledby=\"{heading}\"")).unwrap()].rfind('<').unwrap()..];
        assert!(tag[..tag.find('>').unwrap()].contains(&format!("aria-describedby=\"{hint}\"")), "{group}: {tag}");
        for id in [heading, hint] {
            let at = h.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("{id}: {h}"));
            let el = &h[h[..at].rfind('<').unwrap()..];
            assert!(el[..el.find('>').unwrap()].contains("aria-hidden=\"true\""), "{id}: {el}");
        }
    }
    assert_eq!(count(&h, "ped-item"), 3);
    let fits = s.transform_menu().transit;
    assert!(!fits.is_empty());
    assert_eq!(count(&h, "transit-item"), fits.len());
    for code in &fits {
        assert!(h.contains(&format!(">{code}<")), "{code}");
    }
    let view = s.view_now();
    for m in view.measures.iter().filter(|m| !m.can_apply) {
        assert!(!h.contains(&format!(">{}<", m.code)), "{} will not fit and is not listed", m.code);
    }
    assert!(h.contains("Arrange the street as B1 Center-Running Transit Lanes"), "{h}");
    assert!(!h.contains("Offset Queue-Jump Lanes"), "a junction measure is not here: {h}");
    assert!(!h.contains("Transit Signal Priority"), "a measure that is not modelled is not here: {h}");
    assert!(!h.contains("B2"), "a freeway measure is not offered on a street: {h}");
    // A measure the street already is, is no longer offered.
    assert!(s.edit(|e| e.apply_measure("B3")));
    let h = street_html(|| view! { <page::Transform vm=s.clone()/> });
    assert!(!h.contains("Arrange the street as B3"), "{h}");
    // Made pedestrian, the street has no motor piece: only the measures are left.
    assert!(s.apply(|e| e.make_pedestrian(kind_index("sidewalk").unwrap())));
    let h = street_html(|| view! { <page::Transform vm=s.clone()/> });
    assert!(!h.contains("Make pedestrian") && !h.contains("ped-item"), "a street without traffic has nothing to make pedestrian");
}

#[test]
fn the_transform_menu_names_the_measures_in_french() {
    let s = street_shared_in(1, Locale::FrCa);
    let h = street_html(|| view! { <page::Transform vm=s.clone()/> });
    assert!(h.contains("Voies centrales réservées au transport en commun"), "{h}");
    assert!(h.contains("Réorganiser la rue selon B1 Voies centrales réservées au transport en commun"), "{h}");
    assert!(h.contains("Priorité au transport en commun"), "{h}");
    assert!(!h.contains("Center-Running Transit Lanes"), "{h}");
}

#[test]
fn the_left_panel_holds_the_inspector_and_no_transit_table() {
    let page = include_str!("../../web/street.html");
    assert!(page.contains("id=\"inspector\""));
    assert!(!page.contains("transit-priority") && !page.contains("h-transit"), "the Transform menu is where the measures are arranged");
}

#[test]
fn the_transform_menu_leaves_out_a_group_with_nothing_in_it_and_is_not_shown_empty() {
    use crate::shared::catalogue::kind_index;
    let s = street_shared(1);
    let h = street_html(|| view! { <page::Transform vm=s.clone()/> });
    assert_eq!(count(&h, "role=\"group\""), 2);
    assert!(s.apply(|e| e.make_pedestrian(kind_index("sidewalk").unwrap())));
    let menu = s.transform_menu();
    let h = street_html(|| view! { <page::Transform vm=s.clone()/> });
    if menu.is_empty() {
        assert!(!h.contains("transform-btn"), "{h}");
    } else {
        assert_eq!(count(&h, "role=\"group\""), 1, "{h}");
    }
}

#[test]
fn the_transform_menu_speaks_french() {
    let s = street_shared_in(1, Locale::FrCa);
    let h = street_html(|| view! { <page::Transform vm=s.clone()/> });
    assert!(h.contains(">Transformer<") && h.contains("Rendre piétonne") && h.contains("Priorité au transport en commun"), "{h}");
    assert!(h.contains("Trottoir") && h.contains("Terrasse de café"), "{h}");
    assert!(h.contains("Réorganiser la rue selon"), "{h}");
}

#[test]
fn the_transform_messages_exist_in_both_languages() {
    use crate::shared::i18n::{Args, Locale};
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    assert!(en.has_message("transform-button", Locale::En) && fr.has_message("transform-button", Locale::FrCa));
    assert_eq!(
        (en.tr_now("transform-button", &Args::new()), fr.tr_now("transform-button", &Args::new())),
        ("Transform".to_string(), "Transformer".to_string())
    );
}

#[test]
fn a_bike_lane_is_offered_no_other_times() {
    let s = street_shared_in(0, Locale::En);
    let bike = s.edit(|e| e.add(crate::shared::catalogue::kind_index("bike").unwrap(), 1));
    s.edit(|e| e.select(Some(bike)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("Bike lane") && !h.contains("Other times") && !h.contains("Add other times"), "{h}");
}

// ---- the zone strip ---------------------------------------------------------------

fn zoned_street(starts: &[i32], frame_mm: Option<i32>, locale: Locale) -> Rc<StreetVm> {
    use crate::shared::catalogue::Side;
    use crate::street::model::{Street, ZoneEnd, ZoneFrame, zoned};
    let street = zoned(Street::sample(0, Side::Right), starts, 40_000);
    let frame = frame_mm.map(|length_mm| ZoneFrame { length_mm, from_end: ZoneEnd::First });
    let (ports, _, _) = crate::shared::ports::test_ports();
    StreetVm::new(ports, crate::i18n_for(locale), Editor::from_street(&street, &street, 0).with_frame(frame), None)
}

fn strip_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::strip::ZoneStrip vm=s.clone() ends=None/> })
}

#[test]
fn the_strip_has_one_cell_per_zone_in_a_listbox_with_one_tab_stop_and_the_selected_cell_active() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En);
    s.select_zone(1);
    let h = strip_html(&s);
    assert!(h.contains("id=\"zone-strip\"") && h.contains("role=\"listbox\"") && h.contains("aria-orientation=\"horizontal\""), "{h}");
    assert_eq!(count(&h, "tabindex=\"0\""), 1, "{h}");
    assert_eq!(count(&h, "role=\"option\""), 3, "{h}");
    assert!(h.contains("aria-activedescendant=\"zone-1\""), "{h}");
    assert_eq!(count(&h, "aria-selected=\"true\""), 1, "{h}");
    assert_eq!(count(&h, "aria-selected=\"false\""), 2, "{h}");
    assert!(h.contains("id=\"zone-0\"") && h.contains("id=\"zone-1\"") && h.contains("id=\"zone-2\""), "{h}");
    assert_eq!(count(&h, "aria-hidden=\"true\" class=\"zone-handle\""), 2, "one handle between each two cells: {h}");
    assert!(h.contains(">Zones along the street</h2>") && h.contains("Measured from the first end"), "{h}");
    assert!(!h.contains("were drawn"), "no notice while the zones were drawn on this road: {h}");
}

#[test]
fn each_cell_is_named_with_its_label_range_length_and_section() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En);
    let h = strip_html(&s);
    assert!(h.contains("aria-label=\"Zone 1, 0.0 m to 10.0 m, 10.0 m long, the base section, 1 of 3\""), "{h}");
    assert!(h.contains("aria-label=\"Zone 2, 10.0 m to 30.0 m, 20.0 m long, the base section, 2 of 3\""), "{h}");
    assert!(h.contains("aria-label=\"Zone 3, 30.0 m to 40.0 m, 10.0 m long, the base section, 3 of 3\""), "{h}");
    // The text on a cell may be cut to fit; the whole name is its title.
    assert!(h.contains("title=\"Zone 2\""), "{h}");
}

#[test]
fn the_caption_names_the_end_the_strip_is_measured_from() {
    let s = zoned_street(&[0, 20_000], Some(40_000), Locale::En);
    let h = street_html(|| view! { <crate::street::strip::ZoneStrip vm=s.clone() ends=Some(street_ends())/> });
    assert!(h.contains("Measured from the edge of the map"), "{h}");
}

#[test]
fn the_strip_follows_the_language() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::FrCa);
    // The language puts no-break spaces before units; the words are the same.
    let h = strip_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains(">Zones le long de la rue</h2>"), "{h}");
    assert!(h.contains("aria-label=\"Zone 2, de 10,0 m à 30,0 m, 20,0 m de long, la coupe de base, 2 sur 3\""), "{h}");
}

#[test]
fn a_cut_off_zone_is_disabled_and_the_notice_is_shown() {
    // Drawn on 40 m, loaded 20 m: the last zone starts past the loaded end.
    let s = zoned_street(&[0, 10_000, 30_000], Some(20_000), Locale::En);
    let h = strip_html(&s);
    assert_eq!(count(&h, "aria-disabled=\"true\""), 1, "{h}");
    assert!(h.contains("Cut off by the edge of the map"), "{h}");
    assert!(h.contains("role=\"status\"") && h.contains("The zones were drawn on this street when it was 40.0 m long; here it is 20.0 m."), "{h}");
}

#[test]
fn a_one_zone_street_shows_one_cell_and_an_empty_status_that_a_notice_can_fill() {
    let s = street_shared(0);
    let h = strip_html(&s);
    assert_eq!(count(&h, "role=\"option\""), 1, "{h}");
    assert_eq!(count(&h, "zone-handle"), 0, "{h}");
    assert!(!h.contains("aria-disabled") && !h.contains("were drawn"), "{h}");
    // The live region is always in the page, so what is put in it later is announced.
    assert!(h.contains("role=\"status\" class=\"zone-notice\"><!></p>"), "{h}");
}

#[test]
fn every_cell_has_a_minimum_width_and_the_strip_scrolls_inside_itself() {
    let editor = include_str!("../../web/editor.css");
    let at = |sel: &str| {
        let i = editor.find(&format!("{sel} {{")).unwrap_or_else(|| panic!("{sel}"));
        &editor[i..i + editor[i..].find('}').unwrap()]
    };
    assert!(at(".zone-cell").contains("min-width: 24px;"));
    assert!(at(".zone-strip").contains("overflow-x: auto;"));
    assert!(at(".zone-strip-section").contains("min-width: 0;"));
    // An own section and a cut-off zone differ by more than tint (WCAG 1.4.1), in forced colours too.
    assert!(at(".zone-cell.own").contains("border-top-width: 5px;"));
    assert!(at(".zone-cell[aria-disabled=\"true\"]").contains("border-style: dashed;"));
    let forced = &editor[editor.find("@media (forced-colors: active) {\n  .zone-cell").unwrap()..];
    let forced = &forced[..forced.find("\n}\n").unwrap()];
    assert!(forced.contains(".zone-cell.own {\n    border-top-width: 5px;"), "{forced}");
    assert!(forced.contains("border-style: dashed;"), "{forced}");
}

// ---- the zone commands -------------------------------------------------------------

fn tools_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::zone_tools::ZoneTools vm=s.clone()/> })
}

/// The street with its second zone given a section of its own, the first zone selected.
fn street_with_own_second_zone(locale: Locale) -> Rc<StreetVm> {
    let s = zoned_street(&[0, 20_000], Some(40_000), locale);
    s.select_zone(1);
    let uid = s.view().segments[0].uid;
    assert!(s.edit(|e| e.nudge_width(uid, 100)));
    s.select_zone(0);
    s
}

#[test]
fn the_commands_are_a_named_group_with_a_labelled_field_for_each_input() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En);
    s.select_zone(1);
    let h = tools_html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Zone commands\"") && !h.contains("toolbar"), "{h}");
    for (id, label) in [("zone-split-at", "Split at (m)"), ("zone-start", "Start of this zone (m)"), ("zone-name", "Zone name")] {
        assert!(h.contains(&format!("<label for=\"{id}\"")) && h.contains(&format!(">{label}</label>")), "{id}: {h}");
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    for id in ["zone-split", "zone-merge-prev", "zone-merge-next"] {
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    // The first zone has no start to move.
    s.select_zone(0);
    let h = tools_html(&s);
    assert!(!h.contains("id=\"zone-start\""), "{h}");
}

#[test]
fn a_street_of_one_zone_offers_split_and_nothing_else() {
    let s = zoned_street(&[], Some(40_000), Locale::En);
    let h = tools_html(&s);
    for id in ["zone-split-at", "zone-split"] {
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    for id in ["zone-name", "zone-start", "zone-merge-prev", "zone-merge-next", "zone-reset", "zone-use"] {
        assert!(!h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    // Splitting it brings the rest.
    assert!(s.split_zone(None));
    let h = tools_html(&s);
    for id in ["zone-name", "zone-merge-prev", "zone-merge-next"] {
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
}

#[test]
fn the_choice_names_each_zone_in_the_middle_of_a_french_sentence_in_lower_case() {
    let s = street_with_own_second_zone(Locale::FrCa);
    assert!(s.begin_merge(true));
    let h = tools_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("Garder la coupe : zone 1") && h.contains("Garder la coupe : zone 2"), "{h}");
}

#[test]
fn split_says_why_it_is_unavailable_for_a_street_cut_at_both_ends() {
    let s = zoned_street(&[0, 20_000], None, Locale::En);
    let h = tools_html(&s);
    let b = button_tag(&h, "zone-split");
    assert!(b.contains("aria-disabled=\"true\"") && b.contains("aria-describedby=\"zone-split-why\""), "{b}");
    assert!(h.contains("id=\"zone-split-why\""), "{h}");
    assert!(h.contains("Both ends of this street are cut by the edge of the map, so zones cannot be placed along it."), "{h}");
    let ok = tools_html(&zoned_street(&[0, 20_000], Some(40_000), Locale::En));
    assert!(!button_tag(&ok, "zone-split").contains("aria-disabled"), "{ok}");
    assert!(!ok.contains("zone-split-why"), "{ok}");
}

#[test]
fn merge_buttons_are_unavailable_at_the_ends_with_a_reason() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En);
    let h = tools_html(&s);
    assert!(button_tag(&h, "zone-merge-prev").contains("aria-disabled=\"true\""), "{h}");
    assert!(button_tag(&h, "zone-merge-prev").contains("aria-describedby=\"zone-merge-prev-why\""), "{h}");
    assert!(h.contains("There is no zone on that side to merge with."), "{h}");
    assert!(!button_tag(&h, "zone-merge-next").contains("aria-disabled"), "{h}");
    s.select_zone(1);
    let h = tools_html(&s);
    assert!(!h.contains("aria-disabled") && !h.contains("There is no zone"), "{h}");
    s.select_zone(2);
    let h = tools_html(&s);
    assert!(button_tag(&h, "zone-merge-next").contains("aria-disabled=\"true\""), "{h}");
    assert!(!button_tag(&h, "zone-merge-prev").contains("aria-disabled"), "{h}");
}

#[test]
fn the_merge_choice_group_appears_only_while_a_merge_is_pending() {
    let s = street_with_own_second_zone(Locale::En);
    let h = tools_html(&s);
    assert!(!h.contains("zone-merge-choice") && !h.contains("zone-keep-this"), "{h}");
    assert!(s.begin_merge(true));
    let h = tools_html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Which section to keep\""), "{h}");
    for id in ["zone-keep-this", "zone-keep-other", "zone-keep-cancel"] {
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    assert!(h.contains("Keep the section of Zone 1") && h.contains("Keep the section of Zone 2"), "{h}");
    s.cancel_merge();
    assert!(!tools_html(&s).contains("zone-keep-this"));
}

#[test]
fn reset_and_use_for_the_street_are_offered_only_for_a_zone_with_its_own_section() {
    let s = street_with_own_second_zone(Locale::En);
    let h = tools_html(&s);
    assert!(!h.contains("id=\"zone-reset\"") && !h.contains("id=\"zone-use\""), "{h}");
    s.select_zone(1);
    let h = tools_html(&s);
    assert!(h.contains("id=\"zone-reset\"") && h.contains(">Reset to the street&#x27;s section<") || h.contains(">Reset to the street's section<"), "{h}");
    assert!(h.contains("id=\"zone-use\"") && h.contains("Use this section for the whole street"), "{h}");
}

#[test]
fn the_fields_take_metres_in_the_pages_units_and_say_a_bad_number() {
    use crate::street::zone_tools::{commit_split, commit_start};
    // In French: the comma and the unit word are both read.
    let (ports, said, _) = crate::shared::ports::test_ports();
    let street = crate::street::model::zoned(crate::street::model::Street::sample(0, crate::shared::catalogue::Side::Right), &[0, 20_000], 40_000);
    let frame = Some(crate::street::model::ZoneFrame { length_mm: 40_000, from_end: crate::street::model::ZoneEnd::First });
    let s = StreetVm::new(ports, crate::i18n_for(Locale::FrCa), Editor::from_street(&street, &street, 0).with_frame(frame), None);
    let w = SheetWatch::new(s.clone());
    s.select_zone(1);
    said.take();
    assert!(!commit_start(w, "vingt"));
    assert_eq!(said.take(), ["Ce n\u{2019}est pas un nombre."]);
    assert!(commit_start(w, "25,5 m"));
    assert_eq!(s.view_now().zones.zones[1].from_mm, 25_500);
    said.take();
    assert!(commit_split(w, "32"));
    assert_eq!(s.view_now().zones.zones.len(), 3);
    assert_eq!(s.view_now().zones.zones[2].from_mm, 32_000);
    // An empty field splits in the middle of the selected zone.
    s.select_zone(0);
    assert!(commit_split(w, ""));
    assert_eq!(s.view_now().zones.zones.len(), 4);
    said.take();
    assert!(!commit_split(w, "x"));
    assert_eq!(said.take().len(), 1);
}

#[test]
fn the_zone_commands_are_worded_in_both_languages() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::FrCa);
    s.select_zone(1);
    let h = tools_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("aria-label=\"Commandes des zones\""), "{h}");
    assert!(h.contains(">Diviser à (m)</label>") && h.contains(">Début de cette zone (m)</label>") && h.contains(">Nom de la zone</label>"), "{h}");
    assert!(!h.contains("Zone commands") && !h.contains("Split"), "{h}");
}

// ---- the speed limit control -------------------------------------------------------

fn speed_html(locale: Locale) -> String {
    use crate::shared::catalogue::Side;
    use crate::street::model::{Speed, Street};
    let mut street = Street::sample(0, Side::Right);
    street.speed = Speed::Kmh(50);
    let (ports, _, _) = crate::shared::ports::test_ports();
    let s = StreetVm::new(ports, crate::i18n_for(locale), Editor::from_street(&street, &street, 0), None);
    street_html(|| view! { <crate::street::speed_view::SpeedLimit vm=s.clone()/> })
}

#[test]
fn the_speed_control_renders_a_labelled_select() {
    let h = speed_html(Locale::En);
    assert!(h.contains("<label for=\"speed-limit\"") && h.contains(">Speed limit</label>"), "{h}");
    assert!(h.contains("<select") && h.contains("id=\"speed-limit\""), "{h}");
    assert_eq!(count(&h, "<option"), 13, "{h}");
    assert!(h.contains(">Not set</option>") && h.contains(">50 km/h</option>"), "{h}");
    assert_eq!(count(&h, "selected"), 1, "{h}");
    let at = h.find(">50 km/h</option>").unwrap();
    assert!(h[..at].rsplit("<option").next().unwrap().contains("selected"), "{h}");
}

#[test]
fn the_speed_control_is_in_french_too() {
    let h = speed_html(Locale::FrCa);
    assert!(h.contains(">Limite de vitesse</label>") && h.contains(">Non indiquée</option>"), "{h}");
}

#[test]
fn a_street_over_the_limit_of_an_area_fails_its_row() {
    use crate::restriction::model::Rule;
    let vm = bendville_street(&[("Slow zone", Rule::SpeedLimit { kmh: 30 })], Locale::En);
    assert!(vm.set_speed(Some(50)));
    let h = street_html(|| view! { <notes::Checks vm=vm.clone()/> });
    assert!(h.contains("Area rule: Slow zone") && h.contains("Speed limit 50 km/h, over the area\u{2019}s 30 km/h.") && h.contains("class=\"bad\""), "{h}");
    let n = vm.view_now().checks.iter().filter(|c| !c.ok).count() + 1;
    let pill = street_html(|| view! { <notes::FitChecks vm=vm.clone()/> });
    assert!(pill.contains(&format!("{n} check")), "{pill}");
}

// ---- the curbside strips and their commands ----------------------------------------

fn curb_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::curb_strip::CurbStrips vm=s.clone()/> })
}

/// The sample street (parking on both sides) with a 6 m parking space at the start of the right side, chosen.
fn curb_street(locale: Locale) -> Rc<StreetVm> {
    use crate::shared::catalogue::Side;
    let s = zoned_street(&[0], Some(40_000), locale);
    s.select_curb(Side::Right, 0);
    assert!(s.add_curb(crate::street::model::Use::Parking, 6_000));
    s
}

#[test]
fn the_curb_strip_is_a_listbox_with_one_tab_stop() {
    let s = curb_street(Locale::En);
    let h = curb_html(&s);
    for side in ["left", "right"] {
        assert!(h.contains(&format!("id=\"curb-strip-{side}\"")), "{side}: {h}");
    }
    // the sample has a stops row under each curbside strip, which have their own test
    assert_eq!(count(&h, "id=\"curb-strip-"), 2, "{h}");
    assert_eq!(count(&h, "role=\"listbox\""), 4, "{h}");
    assert_eq!(count(&h, "aria-orientation=\"horizontal\""), 4, "{h}");
    assert_eq!(count(&h, "tabindex=\"0\""), 4, "one tab stop on each strip: {h}");
    assert_eq!(count(&h, "role=\"option\""), 3 + 2, "a gap on the left, a use and a gap on the right, and a gap in each stops row: {h}");
    assert!(h.contains("aria-activedescendant=\"curb-right-0\""), "{h}");
    assert_eq!(count(&h, "aria-selected=\"true\""), 1, "{h}");
    assert!(h.contains("id=\"curb-left-0\"") && h.contains("id=\"curb-right-0\"") && h.contains("id=\"curb-right-1\""), "{h}");
    assert!(h.contains(">Curbside uses and stops along the street</h2>") && h.contains("Curbside, left") && h.contains("Curbside, right"), "{h}");
    assert!(h.contains("aria-label=\"Curbside uses, right\"") && h.contains("aria-label=\"Curbside uses, left\""), "{h}");
    assert!(h.contains("aria-label=\"Parking, 0.0 m to 6.0 m, 6.0 m long, 1 of 2\""), "{h}");
    assert!(h.contains("aria-label=\"Free, 0.0 m to 40.0 m, 40.0 m long, 1 of 1\""), "{h}");
    // 6 m is long enough for parking (5.5 m): no cell says it fails
    assert!(!h.contains("Fails a check"), "{h}");
}

#[test]
fn a_use_that_fails_a_check_says_so_and_a_street_without_a_curbside_lane_has_no_strip() {
    let s = curb_street(Locale::En);
    s.select_curb(crate::shared::catalogue::Side::Right, 0);
    assert!(s.resize_curb(3_000), "shorter than a parking space is allowed and fails its check");
    let h = curb_html(&s);
    assert!(h.contains("Fails a check. See the checks.") && h.contains("failing"), "{h}");
    let none = street_shared(3);
    assert_eq!(count(&curb_html(&none), "role=\"listbox\""), 0);
}

#[test]
fn no_strip_is_drawn_while_the_streets_length_is_unknown() {
    // The sample street has parking on both sides but no length and no frame: there is nothing to lay uses along.
    let s = street_shared(0);
    assert!(s.view_now().curb.sides.is_empty());
    let h = curb_html(&s);
    assert_eq!((count(&h, "role=\"listbox\""), count(&h, "tabindex")), (0, 0), "{h}");
}

#[test]
fn the_curb_strip_follows_the_language() {
    let s = curb_street(Locale::FrCa);
    let h = curb_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains(">Usages du bord de rue et arrêts le long de la rue</h2>"), "{h}");
    assert!(h.contains("aria-label=\"Stationnement, de 0,0 m à 6,0 m, 6,0 m de long, 1 sur 2\""), "{h}");
    assert!(h.contains("aria-label=\"Usages du bord de rue, à droite\"") && h.contains("Bord de rue, à gauche"), "{h}");
}

fn curb_tools_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::curb_tools::CurbTools vm=s.clone()/> })
}

#[test]
fn the_curb_tools_render_labelled_controls() {
    let s = curb_street(Locale::En);
    let h = curb_tools_html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Curbside commands\"") && !h.contains("toolbar"), "{h}");
    for (id, label) in [("curb-use", "Use"), ("curb-length", "Length (m)")] {
        assert!(h.contains(&format!("<label for=\"{id}\"")) && h.contains(&format!(">{label}</label>")), "{id}: {h}");
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    assert!(h.contains("<select id=\"curb-use\""), "{h}");
    assert_eq!(count(&h, "<option"), 8, "{h}");
    for name in ["Parking", "Loading", "Street tree", "Planting", "Rain garden", "Patio", "Bike parking", "Transit platform"] {
        assert!(h.contains(&format!(">{name}</option>")), "{name}: {h}");
    }
    for (id, label) in [("curb-add", "Add use"), ("curb-remove", "Remove use")] {
        assert!(h.contains(&format!("id=\"{id}\"")) && h.contains(&format!(">{label}</button>")), "{id}: {h}");
    }
    // a use is chosen: it can be removed, and a use cannot be added into it
    assert!(h.contains("id=\"curb-add\" aria-disabled=\"true\"") && !h.contains("id=\"curb-remove\" aria-disabled"), "{h}");
    // a street without a curbside lane has no commands
    assert_eq!(count(&curb_tools_html(&street_shared(3)), "curb-use"), 0);
}

#[test]
fn the_curb_tools_follow_the_language() {
    let s = curb_street(Locale::FrCa);
    let h = curb_tools_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("aria-label=\"Commandes du bord de rue\"") && h.contains(">Usage</label>") && h.contains(">Longueur (m)</label>"), "{h}");
    assert!(h.contains(">Ajouter un usage</button>") && h.contains(">Retirer un usage</button>"), "{h}");
    assert!(h.contains(">Fosse d’arbre</option>") && h.contains(">Supports à vélos</option>") && h.contains(">Terrasse</option>"), "{h}");
}

#[test]
fn the_curb_cells_share_the_zone_cells_minimum_width_and_scrolling() {
    let editor = include_str!("../../web/editor.css");
    let rule = |head: &str| {
        let i = editor.find(head).unwrap_or_else(|| panic!("{head}"));
        &editor[i..i + editor[i..].find('}').unwrap()]
    };
    assert!(rule(".curb-cell,\n.stop-cell,\n.zone-cell {").contains("min-width: 24px;"));
    assert!(rule(".curb-strip,\n.stop-strip,\n.green-strip,\n.zone-strip {").contains("overflow-x: auto;"), "the Green row has the same strip box");
    assert!(rule(".curb-cell.failing,\n.stop-cell.failing {").contains("border-bottom-width: 5px;"), "a failing use differs by more than colour");
    assert!(rule(".green-cell {").contains("height: 24px;") && rule(".green-cell {").contains("min-width: 24px;"), "a cell is a target of 24 px");
    assert!(rule(".green-cell.failing {").contains("border-bottom-width: 5px;"), "a failing element differs by more than colour");
}

// ---- the stops rows and their commands ----------------------------------------------

/// A street of (kind, width, direction) pieces, 40 m long, anchored at its first end, with traffic on the right.
fn stop_street(pieces: &[(&str, i32, Option<usize>)], locale: Locale) -> Rc<StreetVm> {
    use crate::shared::catalogue::{REGIONS, Side, StreetClass, kind_index};
    use crate::street::model::{Piece, Street, ZoneEnd, ZoneFrame};
    let pieces: Vec<Piece> =
        pieces.iter().map(|&(id, width_mm, direction)| Piece { kind: kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() }).collect();
    let street = Street::imported(StreetClass::Local, Side::Right, &pieces);
    let region = REGIONS.iter().position(|r| r.drive_side == Side::Right).unwrap();
    let frame = Some(ZoneFrame { length_mm: 40_000, from_end: ZoneEnd::First });
    let (ports, _, _) = crate::shared::ports::test_ports();
    StreetVm::new(ports, crate::i18n_for(locale), Editor::from_street(&street, &street, region).with_frame(frame), None)
}

const PARKED: [(&str, i32, Option<usize>); 6] =
    [("sidewalk", 3000, None), ("parking", 2400, None), ("bus", 3300, Some(1)), ("bus", 3300, Some(0)), ("parking", 2400, None), ("sidewalk", 3000, None)];
const AT_THE_CURB: [(&str, i32, Option<usize>); 4] = [("sidewalk", 3000, None), ("bus", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", 3000, None)];

/// Bus lane beside parking, with a 12 m curb extension at the start of the right side, chosen.
fn parked_with_a_stop(locale: Locale) -> Rc<StreetVm> {
    use crate::shared::catalogue::Side;
    let s = stop_street(&PARKED, locale);
    s.select_stop(Side::Right, 0);
    assert!(s.add_stop(crate::street::model::StopKind::Bulb, 12_000));
    s
}

#[test]
fn the_stops_row_is_a_listbox_with_one_tab_stop() {
    let s = parked_with_a_stop(Locale::En);
    let h = curb_html(&s);
    for side in ["left", "right"] {
        assert!(h.contains(&format!("id=\"stop-strip-{side}\"")), "{side}: {h}");
    }
    assert_eq!(count(&h, "role=\"listbox\""), 4, "a curbside strip and a stops row on each side: {h}");
    assert!(h.contains(">Curbside uses and stops along the street</h2>"), "both are under one heading: {h}");
    assert_eq!(count(&h, "tabindex=\"0\""), 4, "one tab stop on each row: {h}");
    assert_eq!(count(&h, "role=\"option\""), 2 + 1 + 2, "the curbside gaps, a stops gap on the left, a stop and a gap on the right: {h}");
    assert!(h.contains("aria-activedescendant=\"stop-right-0\""), "{h}");
    assert!(h.contains("id=\"stop-left-0\"") && h.contains("id=\"stop-right-0\"") && h.contains("id=\"stop-right-1\""), "{h}");
    assert!(h.contains("aria-label=\"Stops, right\"") && h.contains("aria-label=\"Stops, left\"") && h.contains(">Stops, right</p>"), "{h}");
    assert!(h.contains("aria-label=\"Curb extension, 0.0 m to 12.0 m, 12.0 m long, 1 of 2\""), "{h}");
    assert!(h.contains("aria-label=\"Free, 0.0 m to 40.0 m, 40.0 m long, 1 of 1\""), "{h}");
    // the same classes as the curbside rows, each stops row directly under its side's curbside row
    let at = |id: &str| h.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("{id}: {h}"));
    assert!(
        at("curb-strip-left") < at("stop-strip-left") && at("stop-strip-left") < at("curb-strip-right") && at("curb-strip-right") < at("stop-strip-right"),
        "{h}"
    );
    assert_eq!((count(&h, "class=\"curb-strip\""), count(&h, "class=\"stop-strip\"")), (2, 2), "{h}");
    // with parking on both sides each stops row is directly under its side's curbside row
    let both = curb_html(&curb_street(Locale::En));
    let at = |id: &str| both.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("{id}: {both}"));
    assert!(
        at("curb-strip-left") < at("stop-strip-left") && at("stop-strip-left") < at("curb-strip-right") && at("curb-strip-right") < at("stop-strip-right"),
        "{both}"
    );
    // a curb extension beside the bus lane here fails nothing it is checked for; a curb stop beside parking fails its kind
    assert!(!h.contains("Fails a check"), "{h}");
    assert!(s.set_stop_kind(crate::street::model::StopKind::Curb));
    let h = curb_html(&s);
    assert!(h.contains("Fails a check. See the checks.") && h.contains("failing"), "{h}");
    // a sheltered stop says so
    assert!(s.set_stop_shelter(true));
    assert!(curb_html(&s).contains("aria-label=\"Curb stop, with a shelter, 0.0 m to 12.0 m, 12.0 m long, 1 of 2\""));
}

#[test]
fn a_street_with_a_lane_for_the_bus_and_no_curbside_has_only_the_stops_rows() {
    let s = stop_street(&AT_THE_CURB, Locale::En);
    let h = curb_html(&s);
    assert_eq!((count(&h, "id=\"stop-strip-"), count(&h, "id=\"curb-strip-")), (2, 0), "{h}");
    assert!(h.contains(">Stops along the street</h2>"), "{h}");
    let f = curb_html(&stop_street(&AT_THE_CURB, Locale::FrCa));
    assert!(f.contains(">Arrêts le long de la rue</h2>"), "{f}");
    // a street with neither has no row
    let none = stop_street(&[("sidewalk", 3000, None), ("bike", 1800, None), ("sidewalk", 3000, None)], Locale::En);
    assert_eq!(count(&curb_html(&none), "role=\"listbox\""), 0);
}

#[test]
fn a_street_of_plain_travel_lanes_has_the_stops_rows() {
    let s = stop_street(&[("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("travel", 3300, Some(0)), ("sidewalk", 3000, None)], Locale::En);
    let h = curb_html(&s);
    assert_eq!((count(&h, "id=\"stop-strip-"), count(&h, "id=\"curb-strip-")), (2, 0), "{h}");
}

#[test]
fn the_stops_row_follows_the_language() {
    let s = parked_with_a_stop(Locale::FrCa);
    let h = curb_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("aria-label=\"Avancée de trottoir, de 0,0 m à 12,0 m, 12,0 m de long, 1 sur 2\""), "{h}");
    assert!(h.contains("aria-label=\"Arrêts, à droite\"") && h.contains(">Arrêts, à gauche</p>"), "{h}");
}

fn stop_tools_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::stop_tools::StopTools vm=s.clone()/> })
}

#[test]
fn the_stop_tools_render_labelled_controls() {
    let s = parked_with_a_stop(Locale::En);
    let h = stop_tools_html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Stop commands\"") && !h.contains("toolbar"), "{h}");
    for (id, label) in [("stop-kind", "Kind"), ("stop-name", "Name"), ("stop-length", "Length (m)"), ("stop-shelter", "Shelter")] {
        assert!(h.contains(&format!("<label for=\"{id}\"")) && h.contains(&format!(">{label}</label>")), "{id}: {h}");
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    assert!(h.contains("<select id=\"stop-kind\"") && h.contains("id=\"stop-shelter\" type=\"checkbox\""), "{h}");
    assert_eq!(count(&h, "<option"), 3, "{h}");
    for name in ["Curb stop", "Curb extension", "Boarding island"] {
        assert!(h.contains(&format!(">{name}</option>")), "{name}: {h}");
    }
    for (id, label) in [("stop-add", "Add stop"), ("stop-remove", "Remove stop")] {
        assert!(h.contains(&format!("id=\"{id}\"")) && h.contains(&format!(">{label}</button>")), "{id}: {h}");
    }
    // a stop is chosen: it can be removed, and a stop cannot be added into it
    assert!(h.contains("id=\"stop-add\" aria-disabled=\"true\"") && !h.contains("id=\"stop-remove\" aria-disabled"), "{h}");
    // a street without a lane for the bus or a curbside has no commands
    let none = stop_street(&[("sidewalk", 3000, None), ("bike", 1800, None), ("sidewalk", 3000, None)], Locale::En);
    assert_eq!(count(&stop_tools_html(&none), "stop-kind"), 0);
}

#[test]
fn the_stop_tools_follow_the_language() {
    let s = parked_with_a_stop(Locale::FrCa);
    let h = stop_tools_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(
        h.contains("aria-label=\"Commandes des arrêts\"") && h.contains(">Type</label>") && h.contains(">Nom</label>") && h.contains(">Longueur (m)</label>"),
        "{h}"
    );
    assert!(h.contains(">Abribus</label>") && h.contains(">Ajouter un arrêt</button>") && h.contains(">Retirer un arrêt</button>"), "{h}");
    for name in ["Arrêt au trottoir", "Avancée de trottoir", "Îlot d’embarquement"] {
        assert!(h.contains(&format!(">{name}</option>")), "{name}: {h}");
    }
}

// ---- the green row and its commands -------------------------------------------------

fn green_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::green_strip::GreenStrip vm=s.clone()/> })
}

fn green_tools_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::green_tools::GreenTools vm=s.clone()/> })
}

/// Two trees and a rain garden on a street of two sidewalks and a roadway, the rain garden chosen.
fn green_street(locale: Locale) -> Rc<StreetVm> {
    use crate::street::model::{GreenForm, GreenKind};
    let s = stop_street(&AT_THE_CURB, locale);
    let at = |kind, at_mm, host| GreenForm { at_mm: Some(at_mm), host: Some(host), ..GreenForm::new(kind) };
    assert!(s.add_green(at(GreenKind::Tree, 10_000, 0)));
    assert!(s.add_green(at(GreenKind::Tree, 10_000, 3)));
    assert!(s.add_green(at(GreenKind::RainGarden, 20_000, 0)));
    s
}

#[test]
fn the_green_row_is_a_listbox_with_one_tab_stop() {
    let s = green_street(Locale::En);
    let h = green_html(&s);
    assert_eq!(count(&h, "role=\"listbox\""), 1, "{h}");
    assert!(h.contains("id=\"green-strip\"") && h.contains("aria-orientation=\"horizontal\""), "{h}");
    assert_eq!(count(&h, "tabindex=\"0\""), 1, "one tab stop: {h}");
    assert_eq!(count(&h, "role=\"option\""), 3, "{h}");
    assert!(h.contains("id=\"green-0\"") && h.contains("id=\"green-1\"") && h.contains("id=\"green-2\""), "{h}");
    assert!(h.contains("aria-activedescendant=\"green-2\""), "the last one added is chosen: {h}");
    assert_eq!(count(&h, "aria-selected=\"true\""), 1, "{h}");
    assert!(h.contains(">Trees and planting along the street</h2>") && h.contains("aria-label=\"Trees and planting along the street. "), "{h}");
    assert!(h.contains("aria-label=\"Tree at 10.0 m, on sidewalk, 10.0 m to 13.0 m, 3.0 m long, 1 of 3\""), "{h}");
    assert!(h.contains("aria-label=\"Rain garden at 20.0 m, on sidewalk, 20.0 m to 26.0 m, 6.0 m long, 3 of 3\""), "{h}");
    // two elements side by side across lie on two tracks, which the row's height follows
    assert!(h.contains("--tracks: 2"), "{h}");
}

#[test]
fn a_green_cell_that_fails_a_check_says_so() {
    let s = green_street(Locale::En);
    let h = green_html(&s);
    // a 1.9 m pit on a 3 m sidewalk leaves under the 1.8 m walkway
    assert!(h.contains("Fails a check. See the checks.") && h.contains("failing"), "{h}");
}

#[test]
fn the_green_row_follows_the_language() {
    let h = green_html(&green_street(Locale::FrCa)).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains(">Arbres et plantations le long de la rue</h2>"), "{h}");
    assert!(h.contains("aria-label=\"Arbre à 10,0 m, sur l’élément trottoir, de 10,0 m à 13,0 m, 3,0 m de long, 1 sur 3\""), "{h}");
}

#[test]
fn a_street_without_green_has_no_green_row() {
    let s = stop_street(&AT_THE_CURB, Locale::En);
    let h = green_html(&s);
    assert_eq!((count(&h, "role=\"listbox\""), count(&h, "tabindex"), count(&h, "green")), (0, 0, 0), "{h}");
}

#[test]
fn the_green_tools_are_offered_where_green_can_be_added_and_not_elsewhere() {
    // a street with sidewalks and a known length can take green before it has any
    let s = stop_street(&AT_THE_CURB, Locale::En);
    assert!(green_tools_html(&s).contains("id=\"green-add\""));
    // one with no length cannot, and nothing is drawn
    let street = crate::street::model::Street::sample(0, crate::shared::catalogue::Side::Right);
    let unknown =
        StreetVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::En), Editor::from_street(&street, &street, 0).with_frame(None), None);
    let h = green_tools_html(&unknown);
    assert_eq!(count(&h, "green"), 0, "{h}");
}

#[test]
fn the_green_tools_render_labelled_controls() {
    let s = green_street(Locale::En);
    let h = green_tools_html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Tree and planting commands\"") && !h.contains("toolbar"), "{h}");
    for (id, label) in [
        ("green-kind", "Kind"),
        ("green-at", "Along the street, from the start (m)"),
        ("green-host", "Across, on"),
        ("green-fine", "Across, from the middle of that piece (m)"),
        ("green-length", "Length (m)"),
        ("green-width", "Width (m)"),
    ] {
        assert!(h.contains(&format!("<label for=\"{id}\"")) && h.contains(&format!(">{label}</label>")), "{id}: {h}");
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    assert_eq!(count(&h, "id=\"green-size\""), 0, "the rain garden chosen has no size: {h}");
    assert_eq!(count(&h, "<option"), 4 + 2, "four kinds and the two sidewalks: {h}");
    for name in ["Tree", "Planting bed", "Rain garden", "Bioswale", "Sidewalk, 3.0 m"] {
        assert!(h.contains(&format!(">{name}</option>")), "{name}: {h}");
    }
    for (id, label) in [("green-add", "Add"), ("green-change", "Change"), ("green-remove", "Remove")] {
        assert!(h.contains(&format!("id=\"{id}\"")) && h.contains(&format!(">{label}</button>")), "{id}: {h}");
    }
    // the chosen rain garden's own values are in the fields
    assert!(h.contains("id=\"green-at\" type=\"text\" inputmode=\"decimal\" autocomplete=\"off\" value=\"20.0\"") && h.contains("value=\"6.0\""), "{h}");
    assert!(!h.contains("id=\"green-change\" aria-disabled") && !h.contains("id=\"green-remove\" aria-disabled"), "{h}");
    // choosing a tree shows its size
    s.select_green(0);
    let t = green_tools_html(&s);
    assert!(t.contains("<label for=\"green-size\"") && t.contains(">Medium</option>"), "{t}");
}

#[test]
fn the_green_tools_without_a_chosen_element_cannot_change_or_remove() {
    let s = stop_street(&AT_THE_CURB, Locale::En);
    let h = green_tools_html(&s);
    assert!(h.contains("id=\"green-change\" aria-disabled=\"true\"") && h.contains("id=\"green-remove\" aria-disabled=\"true\""), "{h}");
    assert!(!h.contains("id=\"green-add\" aria-disabled"), "{h}");
}

#[test]
fn the_green_tools_follow_the_language() {
    let h = green_tools_html(&green_street(Locale::FrCa)).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("aria-label=\"Commandes des arbres et plantations\"") && h.contains(">Type</label>") && h.contains(">Longueur (m)</label>"), "{h}");
    assert!(h.contains(">Jardin de pluie</option>") && h.contains(">Plate-bande</option>") && h.contains(">Noue végétalisée</option>"), "{h}");
    assert!(h.contains(">Ajouter</button>") && h.contains(">Modifier</button>") && h.contains(">Retirer</button>"), "{h}");
}
