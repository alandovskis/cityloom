//! The street page's keys and components drawn to HTML on the host, and read as
//! the page would show them.

use std::rc::Rc;

use leptos::prelude::*;

use crate::city::model::EndView;
use crate::shared::catalogue::KINDS;
use crate::shared::i18n::{Args, Locale};
use crate::shared::live;
use crate::shared::testing::{button_tag, count, html};
use crate::shared::units::{Units, group_thousands};
use crate::street::model::Editor;
use crate::street::model::StreetExt;
use crate::street::view::{self, add_menu, inspector, notes, page, transform};
use crate::street::vm::StreetVm;
use crate::street::vm::keys::Action as StreetAction;
use crate::street::vm::transform::{Choice, DialogState, Group, Outcome};
use crate::street::vm::watch::SheetWatch;
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

#[test]
fn the_piece_pane_moves_and_removes_its_piece_with_buttons_that_stop_at_the_ends() {
    let owner = Owner::new();
    owner.set();
    let s = street_shared(0);
    let first = segment(&s, 0);
    s.select(Some(first));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    let (left, right, remove) = (button_tag(&h, "piece-move-left"), button_tag(&h, "piece-move-right"), button_tag(&h, "piece-remove"));
    assert!(left.contains("disabled"), "the first piece cannot go further left: {left}");
    assert!(!right.contains("disabled") && !remove.contains("disabled"), "{right} {remove}");
    assert!(h.contains(">Move left<") && h.contains(">Move right<") && h.contains(">Remove<"), "{h}");
    assert!(h.find("piece-move-left").unwrap() < h.find("i-h-width").unwrap(), "under the name, above the settings: {h}");
    let last = segment(&s, s.view().segments.len() - 1);
    s.select(Some(last));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(button_tag(&h, "piece-move-right").contains("disabled"), "the last cannot go further right");
    let fr = street_shared_in(0, Locale::FrCa);
    fr.select(Some(segment(&fr, 0)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=fr.clone()/> });
    assert!(h.contains(">Déplacer à gauche<") && h.contains(">Déplacer à droite<") && h.contains(">Retirer<"), "{h}");
}

#[test]
fn the_piece_pane_says_where_the_piece_is_counted_from_and_where_its_width_sits_in_its_range() {
    let s = street_shared(0);
    let u = segment(&s, 1);
    s.select(Some(u));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(" · 2 of 6 from the left<"), "{h}");
    let v = s.view();
    let seg = v.segments.iter().find(|x| x.uid == u).unwrap();
    let pct = (seg.width_mm - seg.min_mm) as f64 * 100.0 / (seg.max_mm - seg.min_mm) as f64;
    let meter = &h[h.find("insp-meter").expect("a meter of the width in its range")..];
    assert!(meter.contains(&format!("width: {:.0}%", pct)), "{meter}");
    assert!(h[..h.find("insp-meter").unwrap()].rfind("aria-hidden=\"true\"").is_some(), "the range is said in words beside it");
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
        EndView { name: Said::new("city-the-edge-of-the-map"), title: Said::new("city-the-edge-of-the-map"), junction: false, uid: 0 },
        EndView {
            name: Said::new("city-junction-number").with("n", Arg::Num(4)),
            title: Said::new("city-junction-number").with("n", Arg::Num(4)),
            junction: true,
            uid: 2,
        },
    ]
}

fn street_in(locale: Locale) -> Rc<StreetVm> {
    let (ports, _, _) = crate::shared::ports::test_ports();
    StreetVm::new(ports, crate::i18n_for(locale), Editor::new(0), None)
}

#[test]
fn the_drawing_is_the_street_in_depth_with_no_handles_and_the_keys_reach_it() {
    let s = street_in(Locale::En);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    // The keyboard edits the street through the group around the drawing, which is named for what it shows.
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
fn the_group_keeps_the_height_the_drawing_is_held_to_and_fits_the_room_it_has() {
    use crate::street::view::iso::{iso_height, max_height};
    let s = street_in(Locale::En);
    let v = s.view();
    assert!(iso_height(&v, 1050.0) <= max_height(1050.0));
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains(&format!("min-height:{}px", max_height(1050.0))), "{h}");
    // No 600 px minimum: the drawing is scaled to the group's width.
    assert!(h.contains("iso-view"), "{h}");
}

#[test]
fn in_french_the_group_is_named_for_the_2_5d() {
    let s = street_in(Locale::FrCa);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("aria-label=\"Éditeur de la rue, vue 2,5D\"") && !h.contains("coupe"), "{h}");
}

#[test]
fn the_drawing_s_heading_names_the_street_in_2_5d_in_each_language() {
    for (locale, iso) in [(Locale::En, "Street in 2.5D"), (Locale::FrCa, "Rue en 2,5D")] {
        let s = street_in(locale);
        assert_eq!(street_html(|| view! { <page::DrawingHeading vm=s.clone()/> }), iso);
    }
}

#[test]
fn the_2_5d_drawing_follows_the_selection() {
    let s = street_in(Locale::En);
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
    let s = street_in(Locale::En);
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <view::StreetDrawing vm=s.clone()/> });
    assert!(h.contains("<title>Trottoir</title>") && h.contains("en 2,5D, sur un tronçon de"), "{h}");
    assert!(!h.contains("<title>Sidewalk</title>"), "{h}");
}

#[test]
fn the_street_heading_names_the_street_and_no_more() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(h.contains(">Sample Street 1</h1>"));
    assert!(!h.contains("City map") && !h.contains("Street cross-section") && !h.contains(" wide") && !h.contains("18.0 m") && !h.contains("between"), "{h}");
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
    assert!(d.contains("Unnamed local street in 2.5D"), "{d}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(h.contains(">Rue locale sans nom</h1>"), "{h}");
}

#[test]
fn a_city_street_s_heading_links_to_the_junctions_it_runs_between() {
    let s = street_shared(1);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=Some(street_ends())/> });
    assert!(!h.contains("href=\"map.html\"") && !h.contains("City map") && !h.contains("Street cross-section") && !h.contains(" wide"), "{h}");
    assert!(h.contains("between ") && h.contains("the edge of the map") && h.contains(" and <a href=\"intersection.html?junction=2\">Junction 4</a>"));
    assert_eq!(count(&h, "href=\"intersection.html"), 1, "the edge of the map is not a link");
}

#[test]
fn in_a_project_the_heading_s_links_open_the_places_of_that_project() {
    let (ports, ..) = crate::shared::ports::test_ports();
    let s = StreetVm::new(crate::shared::scoped::in_project(ports, "p1"), crate::i18n_for(Locale::En), Editor::new(1), None);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=Some(street_ends()) neighbours=Some([Some(7), Some(3)])/> });
    assert!(h.contains("href=\"intersection.html?junction=2&amp;project=p1\""), "{h}");
    assert!(h.contains("href=\"street.html?street=7&amp;project=p1\"") && h.contains("href=\"street.html?street=3&amp;project=p1\""), "{h}");
    assert_eq!(count(&h, "project=p1"), 3, "{h}");
}

/// A browser holding Bendville's roads in project `p1`, whose record names Bendville, with the page at `path`.
fn bendville_in_p1(path: &str) -> crate::shared::ports::TestPorts {
    let t = crate::shared::ports::test_ports_full();
    let projects = crate::project::ProjectStore::new(t.ports.storage.clone());
    let p = projects.create("Bendville").unwrap();
    assert_eq!(p.id, "p1");
    assert!(projects.put_raw(crate::project::model::Project { area: crate::restriction::fixtures::bendville().to_query(), ..p }));
    crate::restriction::fixtures::bent_store(crate::shared::scoped::in_project(t.ports.clone(), "p1").storage);
    *t.page.path.borrow_mut() = path.into();
    t
}

#[test]
fn a_street_opened_in_a_project_keeps_its_edits_in_that_project_only() {
    use crate::shared::ports::Storage;
    crate::sync::hub::reset();
    let t = bendville_in_p1("/street.html?street=1&project=p1");
    let sheet = crate::street::open_street_in(t.ports.clone(), 1).expect("Bendville's street, in p1");
    assert!(t.navigator.take().is_empty());
    let vm = sheet.vm();
    let u = vm.view_now().segments[0].uid;
    assert!(vm.apply(|e| e.nudge_width(u, -100)));
    vm.flush();
    assert!(t.storage.recall("cityloom-edits@p1").is_some_and(|v| !v.is_empty()));
    for other in ["cityloom-edits", "cityloom-edits@first"] {
        assert_eq!(t.storage.recall(other), None, "{other}");
    }
    crate::sync::hub::reset();
}

#[test]
fn once_a_sync_removes_the_street_s_project_its_last_edit_is_neither_kept_nor_marked() {
    use crate::shared::ports::Storage;
    use crate::sync::model::{Change, Id, Kind, Loaded, Origin, load};
    crate::sync::hub::reset();
    let t = bendville_in_p1("/street.html?street=1&project=p1");
    let sheet = crate::street::open_street_in(t.ports.clone(), 1).expect("Bendville's street, in p1");
    let vm = sheet.vm();
    let u = vm.view_now().segments[0].uid;
    // an edit still waiting in the keeper when the project is deleted on another device
    assert!(vm.apply(|e| e.nudge_width(u, -100)));
    assert!(crate::project::ProjectStore::new(t.ports.storage.clone()).remove_raw("p1"));
    crate::sync::hub::publish(&[Change { id: Id::new(Kind::Project, "p1"), origin: Origin::Remote }]);
    assert_eq!(t.navigator.take_replaced(), [crate::project::context::MISSING_HREF]);
    vm.flush();
    assert!(t.storage.recall("cityloom-edits@p1").is_none_or(|v| v.is_empty()), "the gone project's work does not come back");
    if let Loaded::Current(doc) = load(&*t.storage) {
        // (the project's own record and its area were marked when the test made them)
        assert!(doc.records.keys().all(|name| !name.starts_with("street/")), "the street is not marked: {:?}", doc.records.keys().collect::<Vec<_>>());
    }
    crate::sync::hub::reset();
}

#[test]
fn a_street_page_for_a_project_that_is_not_here_opens_nothing_and_goes_to_the_list() {
    use crate::shared::ports::Storage;
    crate::sync::hub::reset();
    let t = bendville_in_p1("/street.html?street=1&project=gone");
    assert!(crate::street::open_street_in(t.ports.clone(), 1).is_none());
    assert_eq!(t.navigator.take_replaced(), [crate::project::context::MISSING_HREF]);
    assert!(t.storage.keys().iter().all(|k| !k.ends_with("@gone")), "{:?}", t.storage.keys());
    crate::sync::hub::reset();
}

#[test]
fn a_street_page_for_no_street_of_the_city_goes_to_the_map_of_its_project() {
    crate::sync::hub::reset();
    let t = bendville_in_p1("/street.html?street=99&project=p1");
    assert!(crate::street::open_street_in(t.ports.clone(), 99).is_none());
    assert!(crate::street::open_street_in(t.ports.clone(), 0).is_none());
    assert_eq!(t.navigator.take_replaced(), ["map.html?project=p1", "map.html?project=p1"]);
    crate::sync::hub::reset();
}

#[test]
fn a_city_street_has_arrows_to_the_streets_that_carry_on_from_its_ends() {
    let s = street_shared(1);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=Some(street_ends()) neighbours=Some([None, Some(3)])/> });
    assert!(h.contains("aria-label=\"Along the street\""), "{h}");
    // The end with a street to go on to is a link that says which way it leads; the other says why it is not.
    assert!(h.contains("href=\"street.html?street=3\"") && h.contains("Next street") && h.contains("toward Junction 4"), "{h}");
    assert_eq!(count(&h, "href=\"street.html"), 1, "{h}");
    // Both are drawn as the page's secondary buttons, the one with nowhere to go as a disabled one.
    assert_eq!(count(&h, "class=\"btn street-step"), 2, "{h}");
    assert_eq!(count(&h, "class=\"btn-ico\""), 2, "{h}");
    assert!(h.contains("Previous street") && h.contains("aria-disabled=\"true\"") && h.contains("No street carries on from this end."), "{h}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=Some(street_ends()) neighbours=Some([Some(7), None])/> });
    assert!(h.contains("aria-label=\"Le long de la rue\"") && h.contains("href=\"street.html?street=7\"") && h.contains("Rue précédente"), "{h}");
    assert!(
        h.contains("vers la limite de la carte") && h.contains("Rue suivante") && h.contains("Aucune rue ne se poursuit à partir de cette extrémité."),
        "{h}"
    );
}

#[test]
fn a_street_not_of_the_city_has_no_arrows() {
    let s = street_shared(1);
    let h = street_html(|| view! { <page::StreetHeader vm=s.clone() ends=None/> });
    assert!(!h.contains("Along the street") && !h.contains("street.html"), "{h}");
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

/// A street of project `p1` on a page that shows the project read-only.
fn read_only_street() -> Rc<StreetVm> {
    use crate::shared::ports::Storage;
    let (ports, ..) = crate::shared::ports::test_ports();
    let storage: Rc<dyn Storage> = Rc::new(crate::shared::scoped::ScopedStorage::new(ports.storage.clone(), "p1").only_to_read());
    StreetVm::new(crate::shared::ports::Ports { storage, ..ports }, crate::i18n_for(Locale::En), Editor::new(0), None)
}

#[test]
fn a_street_shown_read_only_offers_no_undo_no_add_menu_and_no_transform_menu() {
    let s = read_only_street();
    let h = street_html(|| view! { <page::History vm=s.clone()/> });
    assert!(!h.contains("id=\"undo\"") && !h.contains("id=\"redo\"") && !h.contains("id=\"reset\""), "{h}");
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(!h.contains("add-btn") && !h.contains("add-menu"), "{h}");
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    assert!(!h.contains("transform-btn"), "{h}");
    // a street that can be edited offers all three
    let s = street_shared(0);
    assert!(street_html(|| view! { <page::History vm=s.clone()/> }).contains("id=\"undo\""));
    assert!(street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> }).contains("id=\"add-btn\""));
    assert!(street_html(|| view! { <transform::TransformDialog vm=s.clone()/> }).contains("id=\"transform-btn\""));
}

#[test]
fn a_street_shown_read_only_offers_its_piece_s_details_disabled_and_its_pieces_to_choose() {
    let s = read_only_street();
    let u = segment(&s, 1);
    s.select(Some(u));
    let read_only = s.read_only();
    let h = html(|| crate::shared::locked::locked(read_only, view! { <inspector::StreetInspector vm=s.clone()/> }));
    assert!(h.starts_with(r#"<fieldset disabled class="read-only-controls""#), "{h}");
    assert!(h.contains("piece-remove"), "the details are still shown: {h}");
    let h = street_html(|| view! { <view::piece_list::PieceList vm=s.clone()/> });
    assert!(!h.contains("disabled"), "the pieces can still be chosen: {h}");
}

/// The opening tag of the first element whose markup holds `needle`.
fn tag_with(h: &str, needle: &str) -> String {
    let at = h.find(needle).unwrap_or_else(|| panic!("{needle}: {h}"));
    let start = h[..at].rfind('<').unwrap();
    let end = h[at..].find('>').unwrap() + at;
    h[start..=end].to_string()
}

/// The opening tag of the element with `id`.
fn tag_of(h: &str, id: &str) -> String {
    tag_with(h, &format!("id=\"{id}\""))
}

/// The Add menu's panel of the chosen topic: from its tag to the end of the menu.
fn add_panel(h: &str) -> &str {
    &h[h.find("id=\"add-panel\"").unwrap_or_else(|| panic!("no panel: {h}"))..]
}

/// A street of `pieces` with its first piece of kind `id` selected, so that the Add menu opens on that kind's topic.
fn add_menu_on(pieces: &[(&str, i32, Option<usize>)], id: &str, locale: Locale) -> Rc<StreetVm> {
    let s = stop_street(pieces, locale);
    let u = s.view().segments.iter().find(|g| KINDS[g.kind].id == id).unwrap_or_else(|| panic!("no {id}")).uid;
    s.select(Some(u));
    s
}

/// Sidewalks and a bike lane: no lane a bus can stop in. The bus shelter is there to be selected, so the menu opens on
/// Transit.
const NO_BUS_LANE: [(&str, i32, Option<usize>); 4] = [("sidewalk", 3000, None), ("bike", 1800, None), ("busshelter", 2000, None), ("sidewalk", 3000, None)];

#[test]
fn the_add_menu_has_a_search_and_eight_topics() {
    use crate::street::vm::add_menu::Topic;
    let s = street_shared(0);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    let menu = tag_of(&h, "add-menu");
    assert!(menu.contains(" hidden") && menu.contains("class=\"menu add-menu\""), "{menu}");
    // A disclosure of a panel, not a menu: no popup role.
    let button = button_tag(&h, "add-btn");
    assert!(button.contains("aria-expanded=\"false\"") && button.contains("aria-controls=\"add-menu\"") && !button.contains("aria-haspopup"), "{button}");
    assert_eq!(count(&h, "id=\"add-search\""), 1, "{h}");
    assert!(tag_of(&h, "add-search").contains("type=\"search\""), "{h}");
    assert!(h.contains("for=\"add-search\"") && h.contains(">Search what to add</label>"), "{h}");
    let topics = tag_with(&h, "role=\"tablist\"");
    assert!(topics.contains("aria-orientation=\"vertical\"") && topics.contains("aria-label=\"Topics\""), "{topics}");
    assert_eq!(count(&h, "role=\"tab\""), 8, "{h}");
    let names = ["Walking", "Greenery", "Cycling", "Transit", "Roadway", "Traffic calming", "Furniture", "Utilities"];
    let mut last = 0;
    for (t, name) in Topic::ALL.into_iter().zip(names) {
        let at = h.find(&format!("id=\"add-topic-{}\"", t.id())).unwrap_or_else(|| panic!("{name}: {h}"));
        assert!(at > last, "{name} in its place: {h}");
        last = h[at..].find(&format!(">{name}</button>")).unwrap_or_else(|| panic!("{name}: {h}")) + at;
    }
    assert!(!h.contains("Transit stops") && !h.contains("Green infrastructure"), "{h}");
    // The menu opens on Walking with nothing selected: its tab is chosen and the only one in the tab order.
    let walking = tag_of(&h, "add-topic-walking");
    assert!(walking.contains("role=\"tab\"") && walking.contains("aria-selected=\"true\"") && walking.contains("tabindex=\"0\""), "{walking}");
    assert!(walking.contains("aria-controls=\"add-panel\""), "{walking}");
    let cycling = tag_of(&h, "add-topic-cycling");
    assert!(cycling.contains("aria-selected=\"false\"") && cycling.contains("tabindex=\"-1\""), "{cycling}");
    let panel = tag_of(&h, "add-panel");
    assert!(panel.contains("role=\"tabpanel\"") && panel.contains("aria-labelledby=\"add-topic-walking\""), "{panel}");
    let panel = add_panel(&h);
    assert_eq!(count(panel, "class=\"add-item\""), 1, "{panel}");
    assert_eq!(count(panel, "class=\"add-icon\""), 1, "{panel}");
    assert!(panel.contains("data-item=\"kind-sidewalk\"") && !panel.contains("aria-disabled"), "{panel}");
    let hint = panel.find("id=\"add-hint-pieces\"").unwrap_or_else(|| panic!("{panel}"));
    let said = panel.find(">Goes after the selected piece.<").unwrap_or_else(|| panic!("{panel}"));
    assert!(hint < said && said < panel.find("<b>Sidewalk</b>").unwrap(), "the hint above the item: {panel}");
    let sidewalk = KINDS.iter().find(|k| k.id == "sidewalk").unwrap().default_mm;
    assert!(panel.contains(&format!("class=\"dw\">{}<", Units::Metres.length_fine_in(sidewalk, Locale::En))), "{panel}");
    s.set_units(Units::Feet);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(h.contains(&format!("class=\"dw\">{}<", Units::Feet.length_fine_in(sidewalk, Locale::En))), "{h}");
}

#[test]
fn the_add_menu_opens_on_the_topic_of_the_chosen_piece() {
    let s = add_menu_on(&[("sidewalk", 3000, None), ("bike", 1800, None), ("travel", 3300, Some(0)), ("sidewalk", 3000, None)], "bike", Locale::En);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(tag_of(&h, "add-topic-cycling").contains("aria-selected=\"true\""), "{h}");
    assert!(tag_of(&h, "add-topic-walking").contains("aria-selected=\"false\""), "{h}");
    assert!(tag_of(&h, "add-panel").contains("aria-labelledby=\"add-topic-cycling\""), "{h}");
    let panel = add_panel(&h);
    assert_eq!(count(panel, "class=\"add-item\""), 3, "{panel}");
    for name in ["Bike lane", "Bike rack", "Bikeshare station"] {
        assert!(panel.contains(&format!("<b>{name}</b>")), "{name}: {panel}");
    }
    assert!(!panel.contains("<b>Sidewalk</b>"), "{panel}");
}

#[test]
fn unavailable_items_are_greyed_with_their_reason() {
    use crate::street::model::{StopKind, StopKindExt};
    let s = add_menu_on(&NO_BUS_LANE, "busshelter", Locale::En);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(tag_of(&h, "add-topic-transit").contains("aria-selected=\"true\""), "{h}");
    let panel = add_panel(&h);
    assert_eq!(count(panel, "class=\"add-item\""), 6, "{panel}");
    assert_eq!(count(panel, "aria-disabled=\"true\""), 3, "{panel}");
    assert_eq!(count(panel, "aria-describedby=\"add-reason-stops\""), 3, "{panel}");
    for k in StopKind::ALL {
        let item = tag_with(panel, &format!("data-item=\"{}\"", k.name_key()));
        assert!(item.contains("aria-disabled=\"true\"") && item.contains("aria-describedby=\"add-reason-stops\""), "{item}");
    }
    // A piece is never greyed.
    assert!(!tag_with(panel, "data-item=\"kind-bus\"").contains("aria-disabled"), "{panel}");
    assert_eq!(count(panel, "id=\"add-reason-stops\""), 1, "the reason once, under its run: {panel}");
    assert!(tag_of(panel, "add-reason-stops").contains("class=\"add-reason\""), "{panel}");
    let at = |text: &str| panel.find(text).unwrap_or_else(|| panic!("{text}: {panel}"));
    let reason = at(">Stops need a lane a bus can stop in on this street.<");
    let (pieces, stops) = (at(">Goes after the selected piece.<"), at(">Goes into the chosen free stretch of its row.<"));
    assert!(pieces < at("<b>Transit lane</b>") && at("<b>Bus station</b>") < stops && stops < at("<b>Curb stop</b>"), "{panel}");
    assert!(at("<b>Boarding island</b>") < reason, "the reason under the run it applies to: {panel}");
    assert!(tag_of(panel, "add-hint-stops").contains("class=\"hint\""), "{panel}");
    // Where a bus can stop, nothing in Transit is greyed.
    let s = add_menu_on(&PARKED, "bus", Locale::En);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    let panel = add_panel(&h);
    assert!(tag_of(&h, "add-topic-transit").contains("aria-selected=\"true\""), "{h}");
    assert_eq!(count(panel, "class=\"add-item\""), 6, "{panel}");
    assert!(!panel.contains("aria-disabled") && !panel.contains("add-reason"), "{panel}");
}

#[test]
fn the_add_menu_follows_the_language_and_the_street() {
    use crate::street::vm::add_menu::Topic;
    let s = add_menu_on(&NO_BUS_LANE, "busshelter", Locale::En);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(h.contains(">Transit</button>") && h.contains(">Stops need a lane a bus can stop in on this street.<"), "{h}");
    // What the menu draws from is watched: the street and the language change it. (Under an owner of its own, the
    // one before put back after.)
    Owner::new().with(|| {
        let w = SheetWatch::new(s.clone());
        let greyed = Memo::new(move |_| w.add_entries(Topic::Transit).iter().filter(|e| e.unavailable.is_some()).count());
        let found = Memo::new(move |_| w.add_search("avancée").len());
        assert_eq!((greyed.get(), found.get()), (3, 0));
        let parking = crate::shared::catalogue::kind_index("parking").unwrap();
        assert!(s.edit(|e| e.add(parking, 2) != 0));
        assert_eq!(greyed.get(), 0, "a curbside lane makes room for stops");
        s.i18n().set(Locale::FrCa);
        assert_eq!(found.get(), 1, "« Avancée de trottoir » is found in French");
    });
    // And the same view-model, switched, draws the menu in French.
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(h.contains(">Transport en commun</button>") && h.contains(">Chercher quoi ajouter</label>") && !h.contains(">Transit</button>"), "{h}");
}

#[test]
fn the_add_menu_offers_every_kind_of_green_greyed_where_green_cannot_go() {
    let planted = [("sidewalk", 3000, None), ("planting", 1500, None), ("bus", 3300, Some(1)), ("bus", 3300, Some(0)), ("sidewalk", 3000, None)];
    let s = add_menu_on(&planted, "planting", Locale::En);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(tag_of(&h, "add-topic-greenery").contains("aria-selected=\"true\""), "{h}");
    let panel = add_panel(&h);
    assert_eq!(count(panel, "class=\"add-item\""), 6, "{panel}");
    let at = |name: &str| panel.find(&format!("<b>{name}</b>")).unwrap_or_else(|| panic!("{name}: {panel}"));
    let order = ["Planting strip", "Planted median", "Tree", "Planting bed", "Rain garden", "Bioswale"];
    assert!(order.windows(2).all(|p| at(p[0]) < at(p[1])), "{panel}");
    assert!(panel.contains(">Goes at the first free place along the street.<") && !panel.contains("aria-disabled"), "{panel}");
    // A street with no end to measure from: the green elements stay, greyed, with the reason.
    let s = street_shared(1);
    let u = s.view().segments.iter().find(|g| KINDS[g.kind].id == "planting").unwrap().uid;
    s.select(Some(u));
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    let panel = add_panel(&h);
    assert_eq!(count(panel, "aria-disabled=\"true\""), 4, "{panel}");
    assert_eq!(count(panel, "aria-describedby=\"add-reason-green\""), 4, "{panel}");
    let reason = crate::shared::testing::en(&crate::shared::said::Said::new("green-refused-no-anchor"));
    let at = panel.find("id=\"add-reason-green\"").unwrap_or_else(|| panic!("{panel}"));
    assert!(panel[at..].contains(&format!(">{reason}</p>")) && at > panel.find("<b>Bioswale</b>").unwrap(), "{reason}: {panel}");
    let s = add_menu_on(&planted, "planting", Locale::FrCa);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(h.contains(">Verdure</button>") && h.contains("<b>Arbre</b>") && h.contains("<b>Noue végétalisée</b>"), "{h}");
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
fn the_width_bars_draw_today_and_after_a_change_the_design_with_a_legend_of_the_uses_the_street_has() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::SpaceBars vm=s.clone()/> });
    assert!(
        h.starts_with(r#"<div aria-hidden="true" class="space-bars""#) || h.contains(r#"aria-hidden="true""#),
        "a drawing of the table, which is read instead: {h}"
    );
    assert_eq!(count(&h, r#"class="space-bar""#), 1, "today alone until the design differs: {h}");
    assert!(h.contains(">Today<") && !h.contains(">Your design<"), "{h}");
    let v = s.view();
    let used: Vec<_> = v.outcomes.share.iter().filter(|u| u.mm > 0).collect();
    assert_eq!(count(&h, "class=\"seg "), used.len(), "a segment per use with width: {h}");
    for u in &used {
        assert!(h.contains(&format!("flex-grow: {}", u.mm)), "{} at {} mm: {h}", u.pct, u.mm);
    }
    assert!(!h.contains("Biking") && h.contains("Walking"), "a use with no width is left out of the legend: {h}");
    let w = segment(&s, 0);
    s.edit(|e| e.nudge_width(w, 100));
    let h = street_html(|| view! { <notes::SpaceBars vm=s.clone()/> });
    assert_eq!(count(&h, r#"class="space-bar""#), 2, "{h}");
    assert!(h.find(">Today<").unwrap() < h.find(">Your design<").unwrap(), "{h}");
    assert!(h.contains(r#"<span class="up">+0.1</span>"#), "the change beside the use's width: {h}");
    let walking = s.view().outcomes.share.iter().find(|u| u.mode == crate::shared::catalogue::Mode::Foot).unwrap().mm;
    assert!(h.contains(&format!("<b>{}</b>", Units::Metres.length_fine_in(walking, Locale::En))), "each width with its unit: {h}");
}

#[test]
fn people_moved_is_one_figure_with_its_change_from_today() {
    let s = street_shared(0);
    let v = s.view();
    let today = group_thousands(v.outcomes.existing_capacity_pph.into(), Locale::En);
    let h = street_html(|| view! { <notes::CapacityFigure vm=s.clone()/> });
    assert!(h.contains(r#"aria-hidden="true""#), "a drawing of the table, which is read instead: {h}");
    assert!(h.contains(&format!("{today} per hour")) && h.contains("As today"), "{h}");
    let u = segment(&s, 2);
    s.edit(|e| e.set_width(u, 3_000));
    let v = s.view();
    if v.outcomes.capacity_pph != v.outcomes.existing_capacity_pph {
        let h = street_html(|| view! { <notes::CapacityFigure vm=s.clone()/> });
        assert!(h.contains(&format!(" from {today} today")) && !h.contains("As today"), "{h}");
    }
    let fr = street_shared_in(0, Locale::FrCa);
    let h = street_html(|| view! { <notes::CapacityFigure vm=fr.clone()/> });
    assert!(h.contains("à l’heure") && h.contains("Comme aujourd’hui"), "{h}");
}

#[test]
fn the_impact_tab_is_one_band_of_the_last_change_the_width_and_the_people_moved() {
    let (s, _) = street_watch();
    let h = html(|| view! { <view::notes_tray::StreetNotes vm=s.clone()/> }.into_any());
    let panel = &h[h.find(r#"id="p-space""#).unwrap()..h.find(r#"id="p-checks""#).unwrap()];
    let at = |s: &str| panel.find(s).unwrap_or_else(|| panic!("{s}: {panel}"));
    assert!(at(r#"class="impact-band""#) < at(r#"id="impact-card""#), "{panel}");
    assert!(at(r#"id="impact-card""#) < at(r#"id="h-space""#) && at(r#"id="h-space""#) < at(r#"id="h-cap""#), "{panel}");
    assert!(at(r#"id="space-bars""#) < at(r#"id="time-note""#) && at(r#"id="time-note""#) < at(r#"id="h-cap""#), "the time of day under the bars: {panel}");
    assert!(at(r#"id="cap-figure""#) > at(r#"id="h-cap""#), "{panel}");
    for table in ["space", "cap"] {
        let tag = &panel[panel[..at(&format!(r#"id="{table}""#))].rfind('<').unwrap()..];
        let tag = &tag[..tag.find('>').unwrap()];
        assert!(tag.contains("sr-only"), "the table is read, the drawing seen: {tag}");
    }
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
    assert_eq!(count(&h, "class=\"ok\""), 4);
    assert!(h.contains(": passes") && h.contains("Every metre is used"));
    let u = segment(&s, 0);
    s.edit(|e| e.set_width(u, 3_800));
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(h.contains("<li class=\"bad\">") && h.contains(": fails") && h.contains("0.5 m too wide. Narrow or remove a piece."));
}

#[test]
fn the_transit_measures_cite_the_atlas_toolbox_for_their_names_and_say_the_rest_is_a_placeholder() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::AtlasSource vm=s.clone()/> });
    assert!(h.contains("Source: named, figures not checked") && h.contains("Marron Institute"), "{h}");
    assert!(h.contains("What each one does here") && h.contains("CityLoom's own placeholder"), "{h}");
    assert!(h.contains("href=\"https://tpa.transitcosts.com/atlas/toolbox\"") && h.contains("target=\"_blank\""), "{h}");
}

#[test]
fn every_check_carries_its_sources_or_says_it_is_a_placeholder() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert_eq!(count(&h, "<details class=\"src"), count(&h, "<li "));
    assert!(h.contains("Source: placeholder, no source"));
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

/// The cards of the changes list, in order: each card's opening tag and its text.
fn change_cards(h: &str) -> Vec<(String, String)> {
    h.split("<li ")
        .skip(1)
        .map(|c| {
            let tag = c[..c.find('>').unwrap()].to_string();
            let body = &c[c.find('>').unwrap() + 1..c.find("</li>").unwrap()];
            let mut text = String::new();
            let mut inside = false;
            for ch in body.chars() {
                match ch {
                    '<' => {
                        inside = true;
                        if !text.is_empty() && !text.ends_with(' ') {
                            text.push(' ');
                        }
                    }
                    '>' => inside = false,
                    c if !inside => text.push(c),
                    _ => {}
                }
            }
            (tag, text.trim().to_string())
        })
        .collect()
}

#[test]
fn the_street_s_changes_are_a_card_each_newest_first_over_the_street_today() {
    let s = street_shared(0);
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    let cards = change_cards(&h);
    assert_eq!(cards.len(), 1, "{h}");
    assert_eq!(cards[0].1, "Street today");
    assert!(cards[0].0.contains("rev-card base now") && cards[0].0.contains(r#"aria-current="step""#), "{:?}", cards[0]);
    let u = segment(&s, 0);
    s.edit(|e| e.remove(u));
    let v = segment(&s, 0);
    s.edit(|e| e.nudge_width(v, 100));
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    let cards = change_cards(&h);
    assert_eq!(cards.len(), 3, "{h}");
    assert!(cards[0].1.starts_with("Step 2 ") && cards[1].1 == "Step 1 Remove sidewalk" && cards[2].1 == "Street today", "{cards:?}");
    assert!(cards[0].0.contains("rev-card now") && cards[0].0.contains(r#"aria-current="step""#), "the newest is the street now: {cards:?}");
    for (tag, _) in &cards[1..] {
        assert!(!tag.contains("now") && !tag.contains("aria-current"), "{tag}");
    }
    assert!(h.contains(r#"<span class="rev-step">Step 1</span>"#) && h.contains(r#"<span class="rev-what">Remove sidewalk</span>"#), "{h}");
}

#[test]
fn the_changes_list_is_drawn_again_in_the_language_the_shell_switches() {
    let owner = Owner::new();
    owner.set();
    let s = street_shared_in(0, Locale::En);
    let u = segment(&s, 0);
    s.edit(|e| e.remove(u));
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(h.contains(">Remove sidewalk<") && h.contains(">Step 1<") && h.contains("Street today"), "{h}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(h.contains(">Retrait\u{a0}: trottoir<") && h.contains(">Étape 1<") && h.contains("La rue aujourd’hui"), "{h}");
    assert!(!h.contains("Remove sidewalk"), "{h}");
}

#[test]
fn nothing_selected_the_street_level_says_how_to_begin_and_the_inspector_is_empty() {
    let s = street_shared(0);
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(!h.contains("<p"), "the inspector shows only at the piece level, with a piece: {h}");
    let page = include_str!("../../../../../web/street.html");
    let street = &page[page.find("id=\"p-street\"").unwrap()..page.find("id=\"p-piece\"").unwrap()];
    assert!(street.contains("data-i18n=\"st-path-hint\""), "the hint is the Street level's");
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
fn a_selected_piece_s_panel_has_its_width_surface_and_place_among_the_pieces() {
    let s = street_shared(0);
    let u = segment(&s, 2);
    s.edit(|e| e.select(Some(u)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains(">Driving lane</h2>") && h.contains("3.3 m wide \u{b7} 3 of 6 from the left"));
    for want in ["Width", "Surface", "What it is paved with.", "Allowed "] {
        assert!(h.contains(want), "{want}");
    }
    // Moved by the pane's buttons (the drawing has no drag since the 2.5D became the only view) or Shift and an arrow.
    assert!(!h.contains("Position") && h.contains("Move left") && h.contains("Move right"), "{h}");
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
fn the_title_block_ends_with_whether_the_street_was_changed_and_the_osm_ways_it_came_from() {
    let s = osm_street_shared(Locale::En);
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains("href=\"https://www.openstreetmap.org/way/4687530\"") && h.contains(">way 4687530 v49<"), "{h}");
    assert!(h.contains(">way 7<") && h.contains("id=\"tb-state\">As imported<"), "{h}");
    // Street, Width, Changes made, then Data and OpenStreetMap, each a label over its value
    let at = |id: &str| h.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("{id}: {h}"));
    assert!(at("tb-street") < at("tb-row") && at("tb-row") < at("tb-changes") && at("tb-changes") < at("tb-state"), "{h}");
    assert!(at("tb-state") < h.find("openstreetmap.org").unwrap(), "{h}");
    assert!(h.contains("<span>Data</span>") && h.contains("<span>OpenStreetMap</span>"), "{h}");
    assert!(h.contains(r#"class="tb-cell tb-src""#), "the sources wrap in a cell of their own: {h}");
    let u = segment(&s, 0);
    s.edit(|e| e.nudge_width(u, 100));
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains("id=\"tb-state\">Edited<"), "{h}");
}

#[test]
fn a_street_with_no_source_has_no_source_in_its_title_block() {
    let s = street_shared(0);
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains("id=\"tb-street\"") && h.contains("id=\"tb-changes\""), "{h}");
    assert!(!h.contains("openstreetmap.org") && !h.contains("tb-state") && !h.contains("OpenStreetMap"), "{h}");
}

#[test]
fn the_title_block_s_source_follows_the_language_switch() {
    let owner = Owner::new();
    owner.set();
    let s = osm_street_shared(Locale::En);
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains(">Data<") && h.contains("As imported"), "{h}");
    s.i18n().set(Locale::FrCa);
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains(">Données<") && h.contains("Tel qu’importé") && !h.contains("As imported"), "{h}");
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
    let u = segment(&s, 0);
    s.edit(|e| e.nudge_width(u, 100));
    let h = street_html(|| view! { <notes::Revisions vm=s.clone()/> });
    assert!(h.contains("La rue aujourd’hui") && h.contains(">Étape 1<") && !h.contains("Street today") && !h.contains("Step"), "{h}");
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
    assert!(!h.contains("Carte de la ville") && !h.contains("Coupe transversale de la rue") && !h.contains("de large"), "{h}");
    assert!(h.contains("entre ") && h.contains("la limite de la carte") && h.contains(" et <a href=\"intersection.html?junction=2\">Jonction 4</a>"), "{h}");
    assert!(!h.contains("City map") && !h.contains("Street cross-section") && !h.contains("between") && !h.contains(" wide"), "{h}");
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains(">Rue<") && h.contains(">Largeur<") && h.contains("Modifications apportées"), "{h}");
    assert!(!h.contains(">Width<") && !h.contains("Changes made"), "{h}");
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
    let s = add_menu_on(&PARKED, "bus", Locale::FrCa);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    for tab in ["Verdure", "Transport en commun", "Apaisement de la circulation"] {
        assert!(h.contains(&format!(">{tab}</button>")), "{tab}: {h}");
    }
    assert!(h.contains("<b>Avancée de trottoir</b>"), "{h}");
    let stop = Units::Metres.length_fine_in(crate::standards::model::stop_min_mm(), Locale::FrCa);
    assert!(stop.starts_with("8,0") && h.contains(&format!("class=\"dw\">{stop}<")), "{stop}: {h}");
    assert!(h.contains("Ajouter un élément") && h.contains(">Chercher quoi ajouter</label>") && h.contains("aria-label=\"Thèmes\""), "{h}");
    assert!(h.contains(">Se place après l’élément sélectionné.<") && h.contains(">Se place dans le tronçon libre choisi de sa rangée.<"), "{h}");
    assert!(!h.contains("Add a piece") && !h.contains("Goes after") && !h.contains(">Walking<") && !h.contains("Search what to add"), "{h}");
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
fn the_vehicle_and_the_other_settings_are_in_french() {
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
fn the_group_ids_are_those_of_the_add_menu() {
    use crate::street::vm::add_menu::Topic;
    assert_eq!(Topic::ALL.map(Topic::id), crate::shared::catalogue::GROUP_IDS);
}

#[test]
fn the_lengths_around_the_section_follow_the_language() {
    let s = street_shared_in(1, Locale::FrCa);
    let row = s.view().row_mm;
    let fr = Units::Metres.length_fine_in(row, Locale::FrCa);
    let en = Units::Metres.length_fine_in(row, Locale::En);
    assert!(fr.contains(',') && fr.contains('\u{a0}') && en.contains('.'), "{fr} {en}");
    let h = street_html(|| view! { <page::TitleBlock vm=s.clone()/> });
    assert!(h.contains(&format!(">{fr}<")) && !h.contains(&en), "{h}");
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
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

#[test]
fn the_street_french_writes_an_ordinary_space_before_a_colon() {
    // The engine puts the no-break space in; one typed in the file would be a second source of it.
    let typed: Vec<&str> = crate::street::RESOURCES.fr.lines().filter(|l| l.contains("\u{a0}:") || l.contains("\u{202f}:")).collect();
    assert_eq!(typed, Vec::<&str>::new());
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

// ---- the Transform dialog -------------------------------------------------------------

/// The choices of one group of the dialog drawn to HTML, `chosen` chosen.
fn choices_html(s: &Rc<StreetVm>, group: Group, chosen: Option<Choice>) -> String {
    let s = s.clone();
    street_html(move || {
        let w = SheetWatch::new(s);
        let choices = w.transform_choices();
        let state = RwSignal::new(DialogState { group, chosen, pointed: None, active: chosen });
        transform::choices_markup(w, &choices, group, state)
    })
}

fn preview_html(s: &Rc<StreetVm>, shown: Option<Choice>) -> String {
    let shown = shown.map(|c| (c, Rc::new(s.preview(c))));
    let s = s.clone();
    street_html(move || transform::preview_markup(SheetWatch::new(s), shown))
}

#[test]
fn the_transform_button_opens_a_dialog() {
    let s = street_shared(0);
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    let button = button_tag(&h, "transform-btn");
    assert!(button.contains("aria-haspopup=\"dialog\"") && button.contains("class=\"btn\""), "{button}");
    assert!(h.contains(">Transform<"), "{h}");
    let dialog = tag_of(&h, "transform-dialog");
    assert!(dialog.starts_with("<dialog") && dialog.contains("aria-labelledby=\"transform-dialog-h\""), "{dialog}");
    assert!(!dialog.contains(" open"), "closed until the button opens it: {dialog}");
    assert!(tag_of(&h, "transform-dialog-h").starts_with("<h2"));
    assert!(h.contains(">Transform this street<"), "{h}");
    assert!(button_tag(&h, "transform-close").contains("aria-label=\"Close\""));
    assert!(h.contains(">Cancel<") && h.contains(">Apply<"), "{h}");
}

#[test]
fn the_dialog_has_a_tab_per_group_with_its_count() {
    let s = street_shared(0);
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    let choices = s.transform_choices();
    let list = tag_of(&h, "transform-groups");
    assert!(list.contains("role=\"tablist\"") && list.contains("aria-orientation=\"vertical\""), "{list}");
    assert_eq!(count(&h, "role=\"tab\""), 3, "{h}");
    let transit: usize = choices.transit.iter().map(|(_, codes)| codes.len()).sum();
    let fit = choices.templates.iter().filter(|t| t.refusal.is_none()).count();
    for (id, name, n, selected) in [
        ("ped", "Make pedestrian", choices.pedestrian.len().to_string(), true),
        ("transit", "Transit priority", transit.to_string(), false),
        ("templates", "Street templates", format!("{fit} of 5"), false),
    ] {
        let tab = button_tag(&h, &format!("transform-tab-{id}"));
        assert!(tab.contains("role=\"tab\"") && tab.contains("aria-controls=\"transform-panel\""), "{tab}");
        assert!(tab.contains(&format!("aria-selected=\"{selected}\"")), "{tab}");
        let body = &h[h.find(&tab).unwrap()..];
        let body = &body[..body.find("</button>").unwrap()];
        assert!(body.contains(name) && body.contains(&format!("class=\"count\">{n}<")), "{body}");
    }
    assert_eq!(choices.pedestrian.len(), 3);
    assert!(transit > 0, "the sample is offered a measure");
    // The first group is open: its tab names the panel, which holds its hint and its choices.
    let panel = tag_of(&h, "transform-panel");
    assert!(panel.contains("role=\"tabpanel\"") && panel.contains("aria-labelledby=\"transform-tab-ped\""), "{panel}");
    let inside = &h[h.find("id=\"transform-panel\"").unwrap()..];
    assert!(inside.find("id=\"transform-hint\"").unwrap() < inside.find("id=\"transform-choices\"").unwrap());
    let list = tag_of(&h, "transform-choices");
    assert!(list.contains("role=\"listbox\"") && list.contains("aria-labelledby=\"transform-tab-ped\""), "{list}");
    assert_eq!(count(&h, "ped-item"), 3);
}

#[test]
fn a_street_without_traffic_has_no_make_pedestrian_tab() {
    use crate::shared::catalogue::kind_index;
    let s = street_shared(1);
    assert!(s.apply(|e| e.make_pedestrian(kind_index("sidewalk").unwrap())));
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    assert!(!h.contains("transform-tab-ped") && !h.contains("ped-item"), "{h}");
    let first = if s.transform_choices().transit.is_empty() { "templates" } else { "transit" };
    assert!(button_tag(&h, &format!("transform-tab-{first}")).contains("aria-selected=\"true\""));
}

#[test]
fn the_transit_choices_are_grouped_by_family() {
    let s = street_shared(1);
    let choices = s.transform_choices();
    let h = choices_html(&s, Group::Transit, None);
    assert_eq!(count(&h, "role=\"group\""), choices.transit.len(), "{h}");
    let i18n = crate::i18n_for(Locale::En);
    for (family, codes) in &choices.transit {
        let heading = format!("transform-family-{family}-h");
        assert!(h.contains(&format!("role=\"group\" aria-labelledby=\"{heading}\"")), "{family}: {h}");
        let name = i18n.tr_now(&format!("transform-family-{family}"), &Args::new());
        assert!(tag_of(&h, &heading).contains("aria-hidden=\"true\""), "read as the group's name only");
        let at = h.find(&format!("id=\"{heading}\"")).unwrap();
        assert!(h[at..at + h[at..].find("</p>").unwrap()].ends_with(&format!(">{name}")), "{name}: {h}");
        for code in codes {
            let option = tag_of(&h, &format!("transform-opt-measure-{}", code.to_lowercase()));
            assert!(option.contains("role=\"option\"") && option.contains("transit-item"), "{option}");
            assert!(h.contains(&format!("<span class=\"dirtag\">{code}</span>")), "{code}");
        }
    }
    let flat: usize = choices.transit.iter().map(|(_, codes)| codes.len()).sum();
    assert_eq!(count(&h, "role=\"option\""), flat);
    assert!(h.contains("Center-Running Transit Lanes"), "{h}");
    // A measure that does not fit, a junction measure and a freeway measure are not offered.
    for m in s.view_now().measures.iter().filter(|m| !m.can_apply) {
        assert!(!h.contains(&format!(">{}<", m.code)), "{} will not fit and is not listed", m.code);
    }
    assert!(!h.contains("Offset Queue-Jump Lanes") && !h.contains("Transit Signal Priority") && !h.contains(">B2<"), "{h}");
    // The hint comes first, outside the list, which it describes.
    let hint = h.find("Arrange the street as one of these transit measures.").expect("the hint");
    assert!(hint < h.find("role=\"listbox\"").unwrap());
    let list = tag_of(&h, "transform-choices");
    assert!(list.contains("aria-labelledby=\"transform-tab-transit\"") && list.contains("tabindex=\"0\""), "{list}");
    assert!(list.contains("aria-describedby=\"transform-hint\""), "{list}");
    // A measure the street already is, is no longer offered.
    assert!(s.edit(|e| e.apply_measure("B3")));
    assert!(!choices_html(&s, Group::Transit, None).contains("transform-opt-measure-b3"));
}

#[test]
fn every_family_has_a_heading_in_both_languages() {
    for locale in Locale::ALL {
        let i18n = crate::i18n_for(locale);
        for family in 'a'..='f' {
            assert!(i18n.has_message(&format!("transform-family-{family}"), locale), "{family} in {}", locale.tag());
        }
    }
    let fr = crate::i18n_for(Locale::FrCa);
    assert_eq!(fr.tr_now("transform-family-a", &Args::new()), "Rues de transport en commun");
}

#[test]
fn every_option_is_named_and_a_disabled_template_says_why() {
    // Sample 2 is 12 m wide: the bus corridor needs 13.8 m.
    let s = street_shared(2);
    let h = choices_html(&s, Group::Templates, None);
    assert_eq!(count(&h, "role=\"option\""), 5, "{h}");
    assert_eq!(count(&h, "id=\"transform-opt-template-"), 5, "{h}");
    for name in ["Commercial main street", "School street", "Neighbourhood greenway", "Bus corridor", "Industrial access street"] {
        assert!(h.contains(&format!("<b>{name}</b>")), "{name}");
    }
    let refused = s.transform_choices().templates.iter().filter(|t| t.refusal.is_some()).count();
    assert_eq!(count(&h, "aria-disabled=\"true\""), refused);
    let bus = tag_of(&h, "transform-opt-template-bus-corridor");
    assert!(bus.contains("aria-disabled=\"true\"") && bus.contains("template-disabled") && bus.contains("data-template=\"bus-corridor\""), "{bus}");
    // Named by what it shows, its name and its reason (WCAG 2.5.3), so the reason is not also its description.
    assert!(!bus.contains("aria-label") && !bus.contains("aria-describedby"), "{bus}");
    let option = &h[h.find("id=\"transform-opt-template-bus-corridor\"").unwrap()..];
    let option = &option[..option.find("</div>").unwrap()];
    assert!(option.contains("<b>Bus corridor</b>") && option.contains("id=\"template-reason-bus-corridor\""), "{option}");
    let reason = &h[h.find("id=\"template-reason-bus-corridor\"").unwrap()..];
    assert!(reason.contains("Bus corridor needs a street at least 13.8 m wide. This street is 12.0 m wide."), "{reason}");
    assert!(tag_of(&h, "template-reason-bus-corridor").contains("class=\"template-reason\""));
    let school = tag_of(&h, "transform-opt-template-school");
    assert!(!school.contains("aria-disabled") && !h.contains("template-reason-school"), "{school}");
    // Every option of every group has its id, and the chosen one is the list's active descendant.
    let s = street_shared(0);
    let sidewalk = Choice::Pedestrian(crate::shared::catalogue::kind_index("sidewalk").unwrap());
    let h = choices_html(&s, Group::Pedestrian, Some(sidewalk));
    assert_eq!(count(&h, "role=\"option\""), 3);
    for (id, name) in [("sidewalk", "Sidewalk"), ("planting", "Planting strip"), ("terrace", "Café terrace")] {
        let option = tag_of(&h, &format!("transform-opt-ped-{id}"));
        assert!(option.contains("ped-item") && option.contains("transform-item"), "{option}");
        assert!(option.contains(&format!("aria-selected=\"{}\"", id == "sidewalk")), "{option}");
        assert!(h.contains(&format!("<b>{name}</b>")), "{name}");
    }
    assert!(tag_of(&h, "transform-choices").contains("aria-activedescendant=\"transform-opt-ped-sidewalk\""));
}

#[test]
fn the_list_s_first_choice_is_active_when_it_takes_the_focus_but_not_chosen() {
    // The APG listbox: a screen reader hears an option as the list takes the focus, and nothing is chosen, previewed
    // or applicable until Space, Enter or a click chooses it.
    let s = street_shared(0);
    let choices = s.transform_choices();
    let mut state = transform::opening(&choices);
    let h = choices_html(&s, state.group, None);
    assert!(!tag_of(&h, "transform-choices").contains("aria-activedescendant"), "{h}");
    state.focus(&choices);
    let focused = state;
    let w = s.clone();
    let h = street_html(move || {
        let w = SheetWatch::new(w);
        let choices = w.transform_choices();
        transform::choices_markup(w, &choices, focused.group, RwSignal::new(focused))
    });
    assert!(tag_of(&h, "transform-choices").contains("aria-activedescendant=\"transform-opt-ped-sidewalk\""), "{h}");
    let first = tag_of(&h, "transform-opt-ped-sidewalk");
    assert!(first.contains("transform-active") && first.contains("aria-selected=\"false\""), "{first}");
    assert_eq!(count(&h, "aria-selected=\"true\""), 0, "{h}");
    assert_eq!(count(&h, "transform-active"), 1, "{h}");
    assert_eq!(focused.shown(), None);
    assert!(preview_html(&s, focused.shown()).contains("transform-empty"));
}

#[test]
fn the_dialog_makes_a_choice_s_preview_once_until_the_street_changes() {
    let s = street_shared(0);
    let sidewalk = Choice::Pedestrian(crate::shared::catalogue::kind_index("sidewalk").unwrap());
    let w = s.clone();
    street_html(move || {
        let previews = transform::Previews::new(SheetWatch::new(w.clone()));
        let (a, b) = (previews.of(sidewalk), previews.of(sidewalk));
        assert!(Rc::ptr_eq(&a, &b), "the live line, the drawing and Apply share one preview");
        assert!(!Rc::ptr_eq(&a, &previews.of(Choice::Measure("B1"))));
        assert!(Rc::ptr_eq(&a, &previews.of(sidewalk)), "kept while other choices are previewed");
        // The street changes (a reload from another device): it is made again, from the street as it now is.
        assert!(w.apply(|e| e.make_pedestrian(crate::shared::catalogue::kind_index("planting").unwrap())));
        let c = previews.of(sidewalk);
        assert!(!Rc::ptr_eq(&a, &c));
        assert!(!c.before.has_motor, "the street as it now is");
        previews.clear();
        assert!(!Rc::ptr_eq(&c, &previews.of(sidewalk)), "closing lets go of them");
        view! { <p></p> }
    });
}

#[test]
fn the_preview_is_empty_until_a_choice_is_shown() {
    let s = street_shared(1);
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    let preview = tag_of(&h, "transform-preview");
    assert!(preview.starts_with("<section") && !preview.contains("aria-live"), "the figures and details are not read out on every move: {preview}");
    let said = tag_of(&h, "transform-said");
    assert!(said.contains("aria-live=\"polite\"") && said.contains("aria-atomic=\"true\""), "{said}");
    assert!(h.find("id=\"transform-preview\"").unwrap() < h.find("id=\"transform-said\"").unwrap(), "inside the preview");
    assert_eq!(count(&h, "aria-live"), 1, "the said line is the dialog's only live region: {h}");
    // Closed (as it opens and after it closes), the dialog holds no drawing.
    assert!(!h.contains("<svg class=\"iso\""), "{h}");
    assert!(h.contains("class=\"transform-empty\">Choose a change to see what it does to the street.<"), "{h}");
    assert!(!h.contains("transform-figure"), "{h}");
    // A kind the motor pieces become: Now and After, at one scale, and what changes.
    let sidewalk = Choice::Pedestrian(crate::shared::catalogue::kind_index("sidewalk").unwrap());
    let h = preview_html(&s, Some(sidewalk));
    assert!(!h.contains("transform-empty"));
    assert!(tag_of(&h, "transform-preview-h").starts_with("<h3") && h.contains(">Sidewalk</h3>"), "{h}");
    assert_eq!(count(&h, "class=\"transform-figure\""), 2, "{h}");
    assert_eq!(count(&h, "role=\"img\""), 2, "{h}");
    let preview = s.preview(sidewalk);
    let Outcome::Changed { after, .. } = &preview.outcome else { panic!("the sample can be made pedestrian") };
    let i18n = crate::i18n_for(Locale::En);
    for v in [&preview.before, after] {
        let alt = view::drawing::alt(v, &i18n, Units::Metres);
        assert!(h.contains(&format!("aria-label=\"{alt}\"")), "{alt}: {h}");
    }
    assert!(h.contains(">Now<") && h.contains(">After<"), "{h}");
    assert!(h.contains("<h4>What changes</h4>"), "{h}");
    assert!(h.contains("impact-uses") || h.contains("impact-none"), "{h}");
    assert!(!h.contains("transform-report"), "{h}");
    // A measure is named by its code and name.
    let h = preview_html(&s, Some(Choice::Measure("B1")));
    assert!(h.contains(">B1 · Center-Running Transit Lanes</h3>"), "{h}");
    // A template that does not fit: the street now, and why.
    let s = street_shared(2);
    let h = preview_html(&s, Some(Choice::Template(crate::street::model::Template::BusCorridor)));
    assert_eq!(count(&h, "class=\"transform-figure\""), 1, "{h}");
    assert!(h.contains(">Now<") && !h.contains(">After<") && !h.contains("What changes"), "{h}");
    assert!(h.contains("class=\"transform-no\">Bus corridor needs a street at least 13.8 m wide."), "{h}");
    // A template that has nothing to report lists nothing; School says what it did.
    let h = preview_html(&s, Some(Choice::Template(crate::street::model::Template::Greenway)));
    assert!(h.contains("What changes") && !h.contains("transform-report"), "{h}");
    let h = preview_html(&s, Some(Choice::Template(crate::street::model::Template::School)));
    assert!(h.contains("class=\"transform-report\"") && h.contains("30 km/h"), "{h}");
}

#[test]
fn the_live_line_says_the_shown_choice_and_its_outcome_and_nothing_else() {
    let s = street_shared(2);
    let said = |c: Option<Choice>| {
        let s = s.clone();
        let mut out = None;
        street_html(|| {
            out = transform::said_line(SheetWatch::new(s.clone()), c.map(|c| (c, Rc::new(s.preview(c)))));
            view! { <p></p> }
        });
        out
    };
    assert_eq!(said(None), None);
    let sidewalk = Choice::Pedestrian(crate::shared::catalogue::kind_index("sidewalk").unwrap());
    let Outcome::Changed { impact, .. } = s.preview(sidewalk).outcome else { panic!("the sample can be made pedestrian") };
    assert!(!impact.is_empty());
    assert_eq!(said(Some(sidewalk)), Some(("Sidewalk".to_string(), crate::shared::testing::en(&impact.verdict_said()))));
    let (name, reason) = said(Some(Choice::Template(crate::street::model::Template::BusCorridor))).unwrap();
    assert_eq!(name, "Bus corridor");
    assert!(reason.starts_with("Bus corridor needs a street at least 13.8 m wide."), "{reason}");
    // In the dialog, the live line holds those two and no figure or detail.
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    let at = h.find("id=\"transform-said\"").unwrap();
    let line = &h[at..at + h[at..].find("</p>").unwrap()];
    assert!(!line.contains("<svg") && !line.contains("<ul") && !line.contains("<h"), "{line}");
}

#[test]
fn the_preview_s_drawings_have_ids_of_their_own() {
    // The page's drawing keeps its ids; each preview figure carries its own, so neither shadows the other.
    assert_eq!(
        transform::scoped_ids(r#"<pattern id="iso-m-brick"/><path data-uid="3" fill="url(#iso-m-brick)" filter="url(#h-x)"/>"#, "transform-now-"),
        r#"<pattern id="transform-now-iso-m-brick"/><path data-uid="3" fill="url(#transform-now-iso-m-brick)" filter="url(#h-x)"/>"#
    );
    let s = street_shared(0);
    let h = preview_html(&s, Some(Choice::Pedestrian(crate::shared::catalogue::kind_index("sidewalk").unwrap())));
    assert!(h.contains(" id=\"transform-now-iso-") && h.contains(" id=\"transform-after-iso-"), "{h}");
    assert!(!h.contains(" id=\"iso-") && !h.contains("url(#iso-"), "{h}");
    for reference in h.split("url(#").skip(1).map(|r| &r[..r.find(')').unwrap()]).filter(|r| r.starts_with("transform-")) {
        assert!(h.contains(&format!(" id=\"{reference}\"")), "{reference} is defined in the preview");
    }
}

#[test]
fn the_dialog_opens_and_closes_with_nothing_chosen_so_it_keeps_no_drawing() {
    let s = street_shared(0);
    let choices = s.transform_choices();
    let state = transform::opening(&choices);
    assert_eq!((state.chosen, state.pointed, state.shown()), (None, None, None));
    assert!(!preview_html(&s, state.shown()).contains("<svg"));
}

#[test]
fn undo_and_redo_keys_wait_while_a_dialog_is_open() {
    use crate::shared::shortcut::Shortcut;
    assert_eq!(view::page_shortcut(true, false, false, "z", false), Some(Shortcut::Undo));
    assert_eq!(view::page_shortcut(true, false, false, "y", false), Some(Shortcut::Redo));
    assert_eq!(view::page_shortcut(true, false, true, "z", false), None, "the Transform dialog is open");
    assert_eq!(view::page_shortcut(true, false, true, "y", false), None);
    assert_eq!(view::page_shortcut(false, false, false, "z", false), None, "no Control or Command");
    assert_eq!(view::page_shortcut(true, true, false, "z", false), None, "typing in a field");
}

#[test]
fn apply_is_disabled_until_a_choice_that_fits_is_chosen() {
    use crate::street::model::Template;
    let s = street_shared(2);
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    let apply = button_tag(&h, "transform-apply");
    assert!(apply.contains("disabled") && apply.contains("class=\"btn primary\""), "{apply}");
    assert!(!button_tag(&h, "transform-cancel").contains("disabled"));
    let can = |c: Option<Choice>| {
        let s = s.clone();
        let mut out = false;
        street_html(|| {
            out = transform::can_apply(transform::Previews::new(SheetWatch::new(s)), c);
            view! { <p></p> }
        });
        out
    };
    assert!(!can(None));
    assert!(can(Some(Choice::Template(Template::School))));
    assert!(!can(Some(Choice::Template(Template::BusCorridor))), "a template that does not fit");
}

#[test]
fn the_transform_dialog_speaks_french() {
    let s = street_shared_in(1, Locale::FrCa);
    let h = street_html(|| view! { <transform::TransformDialog vm=s.clone()/> });
    assert!(h.contains(">Transformer<") && h.contains(">Transformer cette rue<"), "{h}");
    assert!(h.contains("Rendre piétonne") && h.contains("Priorité au transport en commun") && h.contains("Modèles de rue"), "{h}");
    assert!(h.contains("Trottoir") && h.contains("Terrasse de café"), "{h}");
    assert!(h.contains(">Annuler<") && h.contains(">Appliquer<") && h.contains("aria-label=\"Fermer\""), "{h}");
    let fit = s.transform_choices().templates.iter().filter(|t| t.refusal.is_none()).count();
    assert!(h.contains(&format!("class=\"count\">{fit} sur 5<")), "{h}");
    let h = choices_html(&s, Group::Transit, None);
    assert!(h.contains("Voies centrales réservées au transport en commun") && !h.contains("Center-Running Transit Lanes"), "{h}");
    let h = preview_html(&s, Some(Choice::Measure("B1")));
    assert!(h.contains(">B1 · Voies centrales réservées au transport en commun</h3>") && h.contains(">Maintenant<") && h.contains(">Après<"), "{h}");
    let s = street_shared_in(2, Locale::FrCa);
    let h = choices_html(&s, Group::Templates, None);
    for name in ["Rue commerciale", "Rue scolaire", "Voie verte de quartier", "Corridor d’autobus", "Rue d’accès industriel"] {
        assert!(h.contains(name), "{name}");
    }
    assert!(h.contains("Le modèle «\u{a0}Corridor d’autobus\u{a0}» demande une rue d’au moins 13,8\u{a0}m"), "{h}");
}

#[test]
fn the_left_panel_holds_the_inspector_and_no_transit_table() {
    let page = include_str!("../../../../../web/street.html");
    assert!(page.contains("id=\"inspector\""));
    assert!(!page.contains("transit-priority") && !page.contains("h-transit"), "the Transform dialog is where the measures are arranged");
}

#[test]
fn the_transform_messages_exist_in_both_languages() {
    let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
    for id in [
        "transform-button",
        "transform-dialog-title",
        "transform-close",
        "transform-cancel",
        "transform-apply",
        "transform-preview-empty",
        "transform-now",
        "transform-after",
        "transform-what-changes",
        "transform-templates-fit",
        "transform-measure-title",
    ] {
        assert!(en.has_message(id, Locale::En) && fr.has_message(id, Locale::FrCa), "{id}");
    }
    assert_eq!(
        (en.tr_now("transform-button", &Args::new()), fr.tr_now("transform-button", &Args::new())),
        ("Transform".to_string(), "Transformer".to_string())
    );
    let fit = Args::new().num("fit", 4).num("all", 5);
    assert_eq!((en.tr_now("transform-templates-fit", &fit), fr.tr_now("transform-templates-fit", &fit)), ("4 of 5".to_string(), "4 sur 5".to_string()));
}

#[test]
fn a_bike_lane_is_offered_no_other_times() {
    let s = street_shared_in(0, Locale::En);
    let bike = s.edit(|e| e.add(crate::shared::catalogue::kind_index("bike").unwrap(), 1));
    s.edit(|e| e.select(Some(bike)));
    let h = street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    assert!(h.contains("Bike lane") && !h.contains("Other times") && !h.contains("Add other times"), "{h}");
}

/// The sample street divided at `starts` (drawn on 40 m), loaded with a frame `frame_mm` long anchored at its first end.
fn zoned_street(starts: &[i32], frame_mm: Option<i32>, locale: Locale) -> Rc<StreetVm> {
    use crate::shared::catalogue::Side;
    use crate::street::model::{Street, ZoneEnd, ZoneFrame, zoned};
    let street = zoned(Street::sample(0, Side::Right), starts, 40_000);
    let frame = frame_mm.map(|length_mm| ZoneFrame { length_mm, from_end: ZoneEnd::First });
    let (ports, _, _) = crate::shared::ports::test_ports();
    StreetVm::new(ports, crate::i18n_for(locale), Editor::from_street(&street, &street, 0).with_frame(frame), None)
}

// ---- the plan: the whole street from above, in place of the zone strip ----------------------------------------

fn plan_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::view::plan::Plan vm=s.clone()/> })
}

/// The opening tag of the element with id `id`.
fn plan_tag<'a>(h: &'a str, id: &str) -> &'a str {
    let at = h.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("#{id} in {h}"));
    &h[h[..at].rfind('<').unwrap()..at + h[at..].find('>').unwrap() + 1]
}

#[test]
fn the_plan_mounts_in_place_of_the_zone_strip() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En);
    let h = plan_html(&s);
    assert_eq!(count(&h, "id=\"plan\""), 1, "{h}");
    assert!(!h.contains("zone-strip") && !h.contains("zone-cell"), "{h}");
    let group = plan_tag(&h, "plan");
    assert!(group.contains("role=\"group\""), "{group}");
    assert!(group.contains("aria-label=\"The street from its first end to its last, 40.0 m long\""), "{group}");
    assert!(h.contains("<svg class=\"plan\"") && h.contains("data-length-mm=\"40000\""), "the plan's drawing: {h}");
    // The drawing is said by the group's label (with its cut ends) and the point, so a screen reader skips it.
    let svg = &h[h.find("<svg class=\"plan\"").unwrap()..];
    assert!(svg[..svg.find('>').unwrap()].contains("aria-hidden=\"true\""), "{svg}");
    // The page mounts it where the zone strip was.
    let page = include_str!("../mod.rs");
    assert!(page.contains("mount(\"zones-slot\", view! { <view::plan::Plan vm=vm.clone()/> }"), "{page}");
    assert!(!page.contains("ZoneStrip"));
}

/// The point's `aria-keyshortcuts` name only keys that do something: on a street shown read-only the Alt+arrows that
/// move a boundary are always refused, so they are not named; picking keys are.
#[test]
fn the_plan_point_names_the_boundary_keys_only_where_a_boundary_may_move() {
    let keys = |s: &Rc<StreetVm>| {
        let h = plan_html(s);
        let point = plan_tag(&h, "plan-point");
        let at = point.find("aria-keyshortcuts=\"").expect("the point names its keys") + "aria-keyshortcuts=\"".len();
        point[at..at + point[at..].find('"').unwrap()].split(' ').map(str::to_string).collect::<Vec<_>>()
    };
    let editable = keys(&zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En));
    for k in ["Alt+ArrowLeft", "Alt+ArrowRight", "Shift+Alt+ArrowLeft", "Shift+Alt+ArrowRight", "Shift+ArrowRight", "Enter", "Escape"] {
        assert!(editable.iter().any(|e| e == k), "{k} in {editable:?}");
    }
    let read_only = keys(&read_only_street());
    assert!(read_only.iter().all(|k| !k.contains("Alt+")), "no boundary key read-only: {read_only:?}");
    for k in ["Shift+ArrowLeft", "Shift+ArrowRight", "Shift+PageUp", "Shift+PageDown", "Shift+Home", "Shift+End", "Enter", "Escape"] {
        assert!(read_only.iter().any(|e| e == k), "{k} still picks read-only: {read_only:?}");
    }
}

#[test]
fn the_plan_point_is_one_tab_stop_with_slider_semantics() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En);
    let h = plan_html(&s);
    assert_eq!(count(&h, "tabindex="), 1, "one tab stop: {h}");
    let point = plan_tag(&h, "plan-point");
    for attr in [
        "role=\"slider\"",
        "tabindex=\"0\"",
        "aria-valuemin=\"0\"",
        "aria-valuemax=\"40\"",
        "aria-valuenow=\"0\"",
        "aria-valuetext=\"The whole street, 40.0 m long.\"",
        "aria-label=\"Point along the street\"",
        "aria-keyshortcuts=\"",
    ] {
        assert!(point.contains(attr), "{attr} in {point}");
    }
    assert!(!point.contains("aria-labelledby"), "its own short name, not the plan's again: {point}");
    // The point is inside the plan; the stretch picked is the line under it.
    assert!(h.find("id=\"plan\"").unwrap() < h.find("id=\"plan-point\"").unwrap());
    assert!(h.contains("The whole street, 40.0 m long."), "{h}");
    s.pick_stretch(10_000, 20_000);
    let h = plan_html(&s);
    let point = plan_tag(&h, "plan-point");
    assert!(point.contains("aria-valuetext=\"From 10.0 m to 20.0 m, 10.0 m long.\""), "{point}");
    let picked = plan_tag(&h, "plan-picked");
    let line = &h[h.find(picked).unwrap() + picked.len()..];
    assert!(line.starts_with("From 10.0 m to 20.0 m · 10.0 m long<"), "{line}");
    // Moving the point alone, it says where it is.
    s.plan_key(crate::street::vm::keys::PlanKey::Move(12_300));
    let h = plan_html(&s);
    let point = plan_tag(&h, "plan-point");
    assert!(point.contains("aria-valuenow=\"12.3\"") && point.contains("aria-valuetext=\"At 12.3 m\""), "{point}");
}

#[test]
fn the_plan_shows_the_ends_and_a_notice_when_the_zones_were_drawn_elsewhere() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En);
    let h = plan_html(&s);
    assert!(h.contains("role=\"status\" class=\"plan-notice\"><!></p>"), "an empty live region, to be filled: {h}");
    let at = h.find("class=\"plan-ends\"").expect("the ends");
    let ends = &h[h[..at].rfind('<').unwrap()..];
    assert!(ends[..ends.find('>').unwrap()].contains("aria-hidden=\"true\""), "said by the slider instead: {ends}");
    assert!(ends.contains(">0.0 m<") && ends.contains(">40.0 m<"), "{ends}");
    // Drawn on 40 m, loaded 20 m.
    let s = zoned_street(&[0, 10_000, 30_000], Some(20_000), Locale::En);
    let h = plan_html(&s);
    assert!(h.contains("The changes along this street were drawn when it was 40.0 m long; here it is 20.0 m."), "{h}");
}

/// A street with no end to measure from (both cut by the map's edge) is drawn schematic, with no positions: the plan
/// names neither end by a length ("0.0 m" twice), leaves no empty line under it, and its name says both ends are cut.
#[test]
fn a_plan_with_no_end_to_measure_from_names_no_positions_and_says_its_ends_are_cut() {
    for (locale, name) in
        [(Locale::En, "The street, both its ends cut by the edge of the map"), (Locale::FrCa, "La rue, ses deux extrémités coupées par le bord de la carte")]
    {
        let s = zoned_street(&[0, 10_000, 30_000], None, locale);
        let h = plan_html(&s);
        assert!(!h.contains("plan-ends"), "no end labels: {h}");
        assert!(!h.contains("0.0 m") && !h.contains("0,0"), "no position anywhere: {h}");
        assert!(!h.contains("id=\"plan-picked\""), "no empty line under the plan: {h}");
        assert!(plan_tag(&h, "plan").contains(&format!("aria-label=\"{name}\"")), "{h}");
        let svg = &h[h.find("<svg class=\"plan\"").expect("the drawing")..];
        assert_eq!(svg.matches("class=\"plan-cut\"").count(), 2, "both ends drawn cut");
        assert!(!svg[..svg.find('>').unwrap()].contains("data-length-mm"), "schematic: no length");
    }
    // With an end to measure from, the ends and the line are there.
    let h = plan_html(&zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::En));
    assert!(h.contains("plan-ends") && h.contains("id=\"plan-picked\""), "{h}");
}

#[test]
fn a_plan_with_no_length_leaves_the_point_with_no_words_rather_than_empty_ones() {
    let s = zoned_street(&[0, 10_000, 30_000], None, Locale::En);
    let h = plan_html(&s);
    let point = plan_tag(&h, "plan-point");
    assert!(!point.contains("aria-valuetext"), "{point}");
    assert!(point.contains("aria-label=\"Point along the street\""), "{point}");
}

#[test]
fn the_picked_stretch_s_ends_are_fields_on_the_street_level_in_the_page_s_language() {
    let html = |s: &Rc<StreetVm>| street_html(|| view! { <crate::street::view::plan::PickFields vm=s.clone()/> });
    let s = zoned_street(&[0], Some(40_000), Locale::En);
    let h = html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Picked stretch\""), "{h}");
    assert!(plan_tag(&h, "pick-from").contains("value=\"0.0\"") && plan_tag(&h, "pick-to").contains("value=\"40.0\""), "the whole street: {h}");
    assert!(h.contains("for=\"pick-from\">From (m)</label>") && h.contains("for=\"pick-to\">To (m)</label>"), "{h}");
    assert!(plan_tag(&h, "pick-from").contains("inputmode=\"decimal\""), "{h}");
    assert!(s.pick_stretch(10_000, 25_500));
    let h = html(&s);
    assert!(plan_tag(&h, "pick-from").contains("value=\"10.0\"") && plan_tag(&h, "pick-to").contains("value=\"25.5\""), "{h}");
    let fr = zoned_street(&[0], Some(40_000), Locale::FrCa);
    assert!(fr.pick_stretch(10_000, 25_500));
    let h = html(&fr);
    assert!(h.contains("aria-label=\"Tronçon choisi\"") && h.contains(">De (m)</label>") && h.contains(">À (m)</label>"), "{h}");
    assert!(plan_tag(&h, "pick-to").contains("value=\"25,5\""), "{h}");
    // No end to measure from: no positions to type.
    assert!(!html(&zoned_street(&[0], None, Locale::En)).contains("pick-from"));
    // The details panel's Street level holds them, under the street's head.
    let page = include_str!("../../../../../web/street.html");
    let head = page.find("<div id=\"street-head-slot\"").expect("the head's slot");
    let slot = page.find("<div id=\"pick-slot\"").expect("the fields' slot");
    assert!(page.find("id=\"p-street\"").unwrap() < head && head < slot && slot < page.find("<div id=\"speed-slot\"").unwrap());
    assert!(include_str!("../mod.rs").contains("mount(\"pick-slot\", view! { <view::plan::PickFields vm=vm.clone()/> }"));
}

#[test]
fn a_boundary_chosen_on_the_plan_has_a_move_to_field_on_the_street_level() {
    use crate::street::vm::plan::PlanPress;
    let html = |s: &Rc<StreetVm>| street_html(|| view! { <crate::street::view::plan::PickFields vm=s.clone()/> });
    // Two runs: an edit of a stretch from 10 m to the end makes a boundary at 10 m.
    let s = zoned_street(&[0], Some(40_000), Locale::En);
    assert!(s.pick_stretch(10_000, 40_000));
    let uid = s.view_now().segments[0].uid;
    assert!(s.apply(|e| e.set_width(uid, 2_500)));
    assert!(!html(&s).contains("boundary-to"), "nothing chosen, no field");
    let zone = s.view_now().plan.boundaries[0].zone;
    let g = s.plan_press(PlanPress::Boundary { zone }, 100.0);
    s.plan_release(g, 10_000);
    let h = html(&s);
    assert!(plan_tag(&h, "boundary-to").contains("value=\"10.0\""), "{h}");
    assert!(h.contains("for=\"boundary-to\">Move the change to (m)</label>"), "{h}");
    let fr = zoned_street(&[0], Some(40_000), Locale::FrCa);
    assert!(fr.pick_stretch(10_000, 40_000));
    let uid = fr.view_now().segments[0].uid;
    assert!(fr.apply(|e| e.set_width(uid, 2_500)));
    let g = fr.plan_press(PlanPress::Boundary { zone }, 100.0);
    fr.plan_release(g, 10_000);
    let h = html(&fr);
    assert!(h.contains(">Déplacer le changement à (m)</label>") && plan_tag(&h, "boundary-to").contains("value=\"10,0\""), "{h}");
}

#[test]
fn the_plan_follows_the_language() {
    let s = zoned_street(&[0, 10_000, 30_000], Some(40_000), Locale::FrCa);
    let h = plan_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(plan_tag(&h, "plan").contains("aria-label=\"La rue de sa première extrémité à sa dernière, 40,0 m de long\""), "{h}");
    assert!(plan_tag(&h, "plan-point").contains("aria-valuetext=\"Toute la rue, 40,0 m de long.\""), "{h}");
    assert!(plan_tag(&h, "plan-point").contains("aria-label=\"Point le long de la rue\""), "{h}");
}

#[test]
fn the_rows_sit_under_the_plan_in_a_plan_rows_wrapper() {
    let page = include_str!("../../../../../web/street.html");
    let zones = page.find("<div id=\"zones-slot\"").expect("the plan's slot");
    let rows = page.find("<div class=\"plan-rows\">").expect("the rows' wrapper");
    assert!(zones < rows, "under the plan");
    let wrapper = &page[rows..rows + page[rows..].find("\n        </div>").expect("the wrapper closes")];
    for slot in ["curb-slot", "calming-slot", "green-slot"] {
        assert!(wrapper.contains(&format!("<div id=\"{slot}\"")), "{slot} in {wrapper}");
    }
    // Out of sight until the keyboard is in one of them, then shown as they were.
    let editor = include_str!("../../../../../web/editor.css");
    let rule = |sel: &str| {
        let i = editor.find(&format!("\n{sel} {{")).unwrap_or_else(|| panic!("{sel}"));
        &editor[i..i + editor[i..].find('}').unwrap()]
    };
    let hidden = rule(".plan-rows:not(:focus-within)");
    assert!(hidden.contains("clip-path: inset(50%);") && hidden.contains("position: absolute;"), "{hidden}");
    assert!(rule(".plan-rows").contains("display: contents;"));
}

#[test]
fn the_plan_scrolls_inside_itself_and_keeps_its_point_in_sight() {
    let editor = include_str!("../../../../../web/editor.css");
    let rule = |sel: &str| {
        let i = editor.find(&format!("\n{sel} {{")).unwrap_or_else(|| panic!("{sel}"));
        &editor[i..i + editor[i..].find('}').unwrap()]
    };
    assert!(rule(".plan-box").contains("overflow-x: auto;"));
    assert!(rule(".plan-section").contains("min-width: 0;"));
    // The point takes no press from the plan beneath it and is a target wide; its line is always seen, and its focus
    // shows as a ring in the plan's two tones.
    let point = rule("#plan-point");
    assert!(point.contains("pointer-events: none;") && point.contains("width: 24px;"), "{point}");
    assert!(!point.contains("opacity"), "{point}");
    let focused = rule("#plan-point:focus-visible");
    assert!(focused.contains("var(--plan-pick-edge)") && focused.contains("var(--plan-pick-casing)"), "{focused}");
}

#[test]
fn the_point_s_focus_ring_is_seen_whole_at_either_end_of_the_plan() {
    let editor = include_str!("../../../../../web/editor.css");
    let rule = |sel: &str| {
        let i = editor.find(&format!("\n{sel} {{")).unwrap_or_else(|| panic!("{sel}"));
        &editor[i..i + editor[i..].find('}').unwrap()]
    };
    // The point is 24 px wide about its line, so at the plan's ends half of it lies past the drawing: the box that
    // clips the plan's scroll keeps that much room on each side.
    assert!(rule("#plan-point").contains("width: 24px;") && rule("#plan-point").contains("margin-left: -12px;"));
    assert!(rule(".plan-box").contains("padding: 4px 12px;"), "{}", rule(".plan-box"));
    // The plan is fitted to the box less that room, so it still fits without a scroll.
    let component = include_str!("plan/component.rs");
    assert!(component.contains("const POINT_ROOM_PX: f64 = 12.0;"), "the room, as the CSS has it");
    assert!(component.contains("el.client_width()) - 2.0 * POINT_ROOM_PX"), "the plan is fitted inside it");
}

// ---- the street level of the details panel ------------------------------------------

#[test]
fn the_street_level_opens_on_the_street_s_name_class_width_and_length_and_then_the_picked_stretch() {
    let s = zoned_street(&[0, 20_000], Some(40_000), Locale::En);
    let summary = |vm: &Rc<StreetVm>| street_html(|| view! { <crate::street::view::page::StreetSummary vm=vm.clone()/> });
    let h = summary(&s);
    assert!(h.contains(r#"<h2 class="insp-name">Sample Street 1</h2>"#), "{h}");
    let v = s.view();
    let width = Units::Metres.length_fine_in(v.row_mm, Locale::En);
    assert!(h.contains(&format!(">Local street · {width} wide · 40.0 m long<")), "{h}");
    assert!(!h.contains("zone") && !h.contains("street-stretch"), "no count of zones, no strip of them, no line for the whole street: {h}");
    assert!(s.pick_stretch(10_000, 30_000));
    let h = summary(&s);
    assert!(h.contains(r#"<p class="insp-sub street-stretch">From 10.0 m to 30.0 m · 20.0 m long</p>"#), "{h}");
    let one = zoned_street(&[], Some(40_000), Locale::FrCa);
    let h = summary(&one).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("Rue locale ·") && h.contains("40,0 m de long<"), "{h}");
    // A street with no end to measure from has no length to give.
    let free = street_shared(0);
    let h = summary(&free);
    assert!(h.contains(" wide<") && !h.contains(" long"), "{h}");
}

#[test]
fn with_a_stretch_picked_the_speed_limit_says_whose_it_is_and_whether_it_varies() {
    let s = zoned_street(&[0, 20_000], Some(40_000), Locale::En);
    let speed = |vm: &Rc<StreetVm>| street_html(|| view! { <crate::street::view::speed_view::SpeedLimit vm=vm.clone()/> });
    let h = speed(&s);
    assert!(h.contains(r#"<label for="speed-limit" class="zone-l">Speed limit</label>"#), "{h}");
    assert!(!h.contains("zone-hint"), "the whole street's limit needs no line: {h}");
    assert!(s.pick_stretch(10_000, 30_000));
    assert!(s.set_speed(Some(30)));
    let h = speed(&s);
    assert!(h.contains(">For the stretch from 10.0 m to 30.0 m.<"), "{h}");
    assert!(!h.contains("Varies"), "{h}");
    // Across the stretch at 30 km/h and the street's own beside it, the limits differ.
    assert!(s.pick_stretch(0, 20_000));
    let h = speed(&s);
    assert!(h.contains(">For the stretch from 0.0 m to 20.0 m.<") && h.contains(">Varies along the stretch<"), "{h}");
    // The whole street picked: it is the street whose limit varies.
    s.pick_whole();
    let h = speed(&s);
    assert!(h.contains(">Varies along the street<") && !h.contains("For the stretch"), "{h}");
    let fr = zoned_street(&[0, 20_000], Some(40_000), Locale::FrCa);
    assert!(fr.pick_stretch(10_000, 30_000) && fr.set_speed(Some(30)));
    fr.pick_whole();
    assert!(speed(&fr).contains(">Varie le long de la rue<"), "{}", speed(&fr));
    let fr = zoned_street(&[0, 20_000], Some(40_000), Locale::FrCa);
    assert!(fr.pick_stretch(10_000, 30_000));
    let h = speed(&fr).replace('\u{a0}', " ");
    assert!(h.contains(">Pour le tronçon de 10,0 m à 30,0 m.<"), "{h}");
}

#[test]
fn a_piece_whose_width_varies_along_the_pick_says_so_under_its_width() {
    let s = zoned_street(&[0, 20_000], Some(40_000), Locale::En);
    let uid = segment(&s, 0);
    let width = s.view().segments[0].width_mm;
    assert!(s.pick_stretch(0, 20_000));
    assert!(s.edit(|e| e.set_width(uid, width + 100)));
    s.select(Some(uid));
    let inspector = || street_html(|| view! { <inspector::StreetInspector vm=s.clone()/> });
    let one = inspector();
    assert!(!one.contains(r#"id="i-width-varies""#), "one run picked: one width: {one}");
    assert!(one.contains(r#"aria-describedby="i-width-range""#), "described by what is there: {one}");
    s.pick_whole();
    s.select(Some(uid));
    let h = inspector();
    assert!(h.contains(r#"<p id="i-width-varies" class="insp-range">Varies along the street</p>"#), "the whole street is picked: {h}");
    assert!(h.contains(r#"aria-describedby="i-width-range i-width-varies""#), "{h}");
}

#[test]
fn the_details_panel_has_no_zone_level_markup() {
    use crate::street::vm::level::Level;
    let page = include_str!("../../../../../web/street.html");
    for gone in [r#"id="p-zone""#, "zone-entry", "zone-tools-slot", "st-row-zones"] {
        assert!(!page.contains(gone), "{gone} is still in the page");
    }
    // Each level of the panel is a part of the page, and the page has no other.
    assert_eq!(page.matches(r#"class="panel-level""#).count(), Level::ALL.len(), "{page}");
    for level in Level::ALL {
        assert!(page.contains(&format!(r#"class="panel-level" id="{}""#, crate::street::view::panel_path::level_id(level))), "{level:?}");
    }
}

#[test]
fn the_street_level_is_rows_with_the_tip_at_its_foot() {
    let page = include_str!("../../../../../web/street.html");
    let level = &page[page.find(r#"id="p-street""#).unwrap()..page.find(r#"id="p-piece""#).unwrap()];
    let at = |s: &str| level.find(s).unwrap_or_else(|| panic!("{s}: {level}"));
    assert!(at("street-head-slot") < at("speed-slot") && at("speed-slot") < at("standards-slot"), "{level}");
    assert!(at("standards-slot") < at("st-path-hint"), "the tip last: {level}");
}

// ---- the details panel's path ------------------------------------------------------

fn path_html(vm: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::view::panel_path::PanelPath vm=vm.clone()/> })
}

#[test]
fn with_nothing_chosen_the_path_is_the_street_alone() {
    let vm = street_in(Locale::En);
    let h = path_html(&vm);
    assert!(h.contains("<nav") && h.contains("aria-label=\"What the details are of\""), "{h}");
    assert_eq!(count(&h, "<li"), 1, "{h}");
    assert!(h.contains("aria-current=\"location\"") && h.contains(">Street</span>"), "{h}");
    assert!(!h.contains("<button"), "the level shown is no button: {h}");
}

#[test]
fn a_chosen_piece_has_a_path_from_the_street_whose_first_step_is_a_button() {
    let vm = street_in(Locale::En);
    vm.select(Some(vm.view_now().segments[0].uid));
    let h = path_html(&vm);
    assert_eq!(count(&h, "<li"), 2, "{h}");
    assert_eq!(count(&h, "<button"), 1, "{h}");
    assert!(h.contains("data-level=\"street\"") && h.contains(">Street</button>"), "{h}");
    assert!(!h.contains("data-level=\"zone\"") && !h.contains("Zone 1"), "{h}");
    let current = &h[h.find("aria-current").unwrap()..];
    assert!(current.contains(">Sidewalk</span>"), "{h}");
}

#[test]
fn the_path_is_in_french_too() {
    let vm = street_in(Locale::FrCa);
    vm.select(Some(vm.view_now().segments[0].uid));
    let h = path_html(&vm);
    assert!(h.contains("aria-label=\"Ce que montrent les détails\"") && h.contains(">Rue</button>") && h.contains(">Trottoir</span>"), "{h}");
}

// ---- the speed limit control -------------------------------------------------------

fn speed_html(locale: Locale) -> String {
    use crate::shared::catalogue::Side;
    use crate::street::model::{Speed, Street};
    let mut street = Street::sample(0, Side::Right);
    street.speed = Speed::Kmh(50);
    let (ports, _, _) = crate::shared::ports::test_ports();
    let s = StreetVm::new(ports, crate::i18n_for(locale), Editor::from_street(&street, &street, 0), None);
    street_html(|| view! { <crate::street::view::speed_view::SpeedLimit vm=s.clone()/> })
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
    street_html(|| view! { <crate::street::view::curb_strip::CurbStrips vm=s.clone()/> })
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
fn on_a_street_anchored_at_its_last_end_the_unfolded_row_runs_as_the_plan_does() {
    use crate::shared::catalogue::Side;
    use crate::street::model::{Instance, PlanThing, Street, Use, ZoneEnd, ZoneFrame};
    // Parking at anchored 0 to 6 m on 40 m: 34 m to 40 m from the plan's left end.
    let street = Street {
        from_end: ZoneEnd::Last,
        length_mm: Some(40_000),
        curb: vec![Instance { side: Side::Right, from_mm: 0, length_mm: 6_000, use_: Use::Parking }],
        ..Street::sample(0, Side::Right)
    };
    let (ports, _, _) = crate::shared::ports::test_ports();
    let editor = Editor::from_street(&street, &street, 0).with_frame(Some(ZoneFrame { length_mm: 40_000, from_end: ZoneEnd::Last }));
    let s = StreetVm::new(ports, crate::i18n_for(Locale::En), editor, None);
    let parking = s.view_now().plan.objects.iter().find(|o| matches!(o.thing, PlanThing::Curb { side: Side::Right, .. })).map(|o| (o.from_mm, o.to_mm));
    assert_eq!(parking, Some((34_000, 40_000)), "the plan draws it at its right end");
    let h = curb_html(&s);
    let at = |needle: &str| h.find(needle).unwrap_or_else(|| panic!("{needle} in {h}"));
    let free = at("aria-label=\"Free, 0.0 m to 34.0 m, 34.0 m long, 1 of 2\"");
    let use_ = at("aria-label=\"Parking, 34.0 m to 40.0 m, 6.0 m long, 2 of 2\"");
    assert!(free < use_, "the free stretch first, on the left, as on the plan: {h}");
    assert!(at("id=\"curb-right-0\"") < free && free < at("id=\"curb-right-1\""), "{h}");
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
    street_html(|| view! { <crate::street::view::curb_tools::CurbTools vm=s.clone()/> })
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
    let editor = include_str!("../../../../../web/editor.css");
    let rule = |head: &str| {
        let i = editor.find(head).unwrap_or_else(|| panic!("{head}"));
        &editor[i..i + editor[i..].find('}').unwrap()]
    };
    assert!(rule(".curb-cell,\n.stop-cell,\n.calming-cell {").contains("min-width: 24px;"));
    assert!(rule(".curb-strip,\n.stop-strip,\n.calming-strip,\n.green-strip {").contains("overflow-x: auto;"), "the Green row has the same strip box");
    assert!(
        rule(".curb-cell.failing,\n.stop-cell.failing,\n.calming-cell.failing {").contains("border-bottom-width: 5px;"),
        "a failing use differs by more than colour"
    );
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
    street_html(|| view! { <crate::street::view::stop_tools::StopTools vm=s.clone()/> })
}

#[test]
fn the_stop_tools_render_labelled_controls() {
    let s = parked_with_a_stop(Locale::En);
    let h = stop_tools_html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Stop commands\"") && !h.contains("toolbar"), "{h}");
    for (id, label) in [("stop-name", "Name"), ("stop-length", "Length (m)")] {
        assert!(h.contains(&format!("<label for=\"{id}\"")) && h.contains(&format!(">{label}</label>")), "{id}: {h}");
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    // the shelter is not set here
    assert!(!h.contains("stop-shelter") && !h.contains("checkbox") && !h.contains("Shelter"), "{h}");
    // the kind is chosen in the Add a piece menu, when the stop is added: no kind control here
    assert!(!h.contains("stop-kind") && !h.contains("<select") && !h.contains("<option") && !h.contains("Curb extension"), "{h}");
    assert!(h.contains("id=\"stop-remove\"") && h.contains(">Remove stop</button>"), "{h}");
    // adding is in the Add a piece menu, not here
    assert!(!h.contains("stop-add") && !h.contains("Add stop"), "{h}");
    // a stop is chosen: it can be removed
    assert!(!h.contains("id=\"stop-remove\" aria-disabled"), "{h}");
    // a street without a lane for the bus or a curbside has no commands
    let none = stop_street(&[("sidewalk", 3000, None), ("bike", 1800, None), ("sidewalk", 3000, None)], Locale::En);
    assert_eq!(count(&stop_tools_html(&none), "stop-name"), 0);
}

#[test]
fn the_stop_tools_show_only_while_a_stop_is_chosen() {
    use crate::shared::catalogue::Side;
    let s = parked_with_a_stop(Locale::En);
    // the free stretch after the stop is chosen: no commands, a stop is added from the Add a piece menu
    s.select_stop(Side::Right, 1);
    let h = stop_tools_html(&s);
    assert!(!h.contains("stop-") && !h.contains("Stop commands"), "{h}");
    s.select_stop(Side::Right, 0);
    assert!(stop_tools_html(&s).contains("id=\"stop-name\""));
}

#[test]
fn the_stop_tools_follow_the_language() {
    let s = parked_with_a_stop(Locale::FrCa);
    let h = stop_tools_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("aria-label=\"Commandes des arrêts\"") && h.contains(">Nom</label>") && h.contains(">Longueur (m)</label>"), "{h}");
    assert!(!h.contains("Abribus") && h.contains(">Retirer un arrêt</button>"), "{h}");
    assert!(!h.contains(">Type</label>") && !h.contains("<option"), "{h}");
}

// ---- the green row and its commands -------------------------------------------------

fn green_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::view::green_strip::GreenStrip vm=s.clone()/> })
}

fn green_tools_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::view::green_tools::GreenTools vm=s.clone()/> })
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
    use crate::street::model::{GreenForm, GreenKind};
    let s = stop_street(&AT_THE_CURB, Locale::En);
    let tree = |at_mm| GreenForm { at_mm: Some(at_mm), host: Some(0), ..GreenForm::new(GreenKind::Tree) };
    assert!(s.add_green(tree(10_000)));
    let h = green_html(&s);
    assert!(!h.contains("Fails a check") && !h.contains("failing"), "one tree on a 3 m sidewalk passes every check: {h}");
    // A second one 4 m on stands closer than their crowns allow (8 m between centres); the pair is recorded against
    // the first.
    assert!(s.add_green(tree(14_000)));
    let h = green_html(&s);
    assert_eq!(count(&h, "Fails a check. See the checks."), 1, "{h}");
    assert_eq!(count(&h, "failing"), 1, "{h}");
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
fn the_green_tools_show_only_while_an_element_is_chosen() {
    // a street that can take green but has none chosen has no tools: an element is added from the Add a piece menu
    let s = stop_street(&AT_THE_CURB, Locale::En);
    let h = green_tools_html(&s);
    assert!(!h.contains("green-") && !h.contains("Tree and planting commands"), "{h}");
    // one with no length cannot take green, and nothing is drawn
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
        ("green-fine", "Across, from the middle of that piece (cm)"),
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
    for (id, label) in [("green-change", "Change"), ("green-remove", "Remove")] {
        assert!(h.contains(&format!("id=\"{id}\"")) && h.contains(&format!(">{label}</button>")), "{id}: {h}");
    }
    assert!(!h.contains("green-add") && !h.contains(">Add</button>"), "adding is in the Add a piece menu: {h}");
    // the chosen rain garden's own values are in the fields
    assert!(h.contains("id=\"green-at\" type=\"text\" inputmode=\"decimal\" autocomplete=\"off\" value=\"20.0\"") && h.contains("value=\"6.0\""), "{h}");
    assert!(!h.contains("id=\"green-change\" aria-disabled") && !h.contains("id=\"green-remove\" aria-disabled"), "{h}");
    // choosing a tree shows its size
    s.select_green(0);
    let t = green_tools_html(&s);
    assert!(t.contains("<label for=\"green-size\"") && t.contains(">Medium</option>"), "{t}");
}

#[test]
fn the_green_tools_follow_the_language() {
    let h = green_tools_html(&green_street(Locale::FrCa)).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains("aria-label=\"Commandes des arbres et plantations\"") && h.contains(">Type</label>") && h.contains(">Longueur (m)</label>"), "{h}");
    assert!(h.contains(">Jardin de pluie</option>") && h.contains(">Plate-bande</option>") && h.contains(">Noue végétalisée</option>"), "{h}");
    assert!(!h.contains(">Ajouter</button>") && h.contains(">Modifier</button>") && h.contains(">Retirer</button>"), "{h}");
}

// ---- the piece list ----------------------------------------------------------------

fn pieces_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::view::piece_list::PieceList vm=s.clone()/> })
}

#[test]
fn the_piece_list_is_a_listbox_with_one_tab_stop_and_a_cell_per_piece_with_its_kind_and_width() {
    let s = street_in(Locale::En);
    let v = s.view_now();
    let h = pieces_html(&s);
    assert_eq!(count(&h, "role=\"listbox\""), 1, "{h}");
    assert!(h.contains("id=\"piece-list\"") && h.contains("tabindex=\"0\"") && h.contains("aria-label=\"Pieces, left to right\""), "{h}");
    assert_eq!(count(&h, "role=\"option\""), v.segments.len(), "{h}");
    assert_eq!(count(&h, "tabindex"), 1, "the cells are not tab stops");
    for (i, g) in v.segments.iter().enumerate() {
        let name = crate::street::vm::text::piece_text(&v, i, &s.i18n(), s.units_now());
        assert!(h.contains(&format!("id=\"piece-cell-{}\"", g.uid)), "{h}");
        assert!(h.contains(&format!("aria-label=\"{name}\"")), "{name}: {h}");
    }
    // Nothing selected: none is marked, and the keyboard lands on the first.
    assert_eq!(count(&h, "aria-selected=\"true\""), 0);
    assert!(h.contains(&format!("aria-activedescendant=\"piece-cell-{}\"", v.segments[0].uid)), "{h}");
}

#[test]
fn the_selected_piece_is_the_one_marked_and_the_active_descendant() {
    let s = street_in(Locale::En);
    let uid = s.view_now().segments[2].uid;
    s.select(Some(uid));
    let h = pieces_html(&s);
    assert_eq!(count(&h, "aria-selected=\"true\""), 1, "{h}");
    assert!(h.contains(&format!("aria-activedescendant=\"piece-cell-{uid}\"")), "{h}");
    let cell = &h[h.find(&format!("id=\"piece-cell-{uid}\"")).unwrap()..];
    assert!(cell[..cell.find("</div>").unwrap()].contains("aria-selected=\"true\""));
}

#[test]
fn the_piece_list_is_worded_in_french_with_the_glossary_s_word() {
    let h = pieces_html(&street_in(Locale::FrCa));
    assert!(h.contains("aria-label=\"Éléments, de gauche à droite\""), "{h}");
    assert!(h.contains("Éléments de la rue"), "{h}");
}

// ---- the summary of the last change (#64) --------------------------------------------------

/// A street page whose clock the test holds, so the summary can be left to settle.
fn timed_street(locale: Locale) -> (Rc<StreetVm>, Rc<crate::shared::ports::ManualScheduler>) {
    let (ports, _, _, time) = crate::shared::ports::test_ports_with_time();
    (StreetVm::new(ports, crate::i18n_for(locale), Editor::new(0), None), time)
}

fn last_change(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <notes::LastChange vm=s.clone()/> })
}

#[test]
fn the_card_says_there_is_no_change_yet_until_the_street_is_changed() {
    let (s, time) = timed_street(Locale::En);
    let h = last_change(&s);
    assert!(h.contains("impact-empty") && h.contains("No change yet."), "{h}");
    assert!(!h.contains("impact-what"));
    let u = segment(&s, 0);
    s.select(Some(u));
    time.advance(2_000);
    assert!(last_change(&s).contains("impact-empty"), "selecting is not a change");
}

#[test]
fn an_edit_that_widens_a_sidewalk_shows_the_space_it_took_and_the_check_it_broke_apart() {
    let (s, time) = timed_street(Locale::En);
    let u = segment(&s, 0);
    assert!(s.apply(|e| e.set_width(u, 3_800)));
    assert!(last_change(&s).contains("impact-empty"), "not until the edits settle");
    time.advance(600);
    let h = last_change(&s);
    assert!(h.contains("Resize sidewalk") && h.contains("Street space"), "{h}");
    assert!(h.contains("mc mc-foot") && h.contains("Walking: 6.6 m \u{2192} 7.1 m (0.5 m more)"), "{h}");
    assert!(h.contains("Newly failing") && !h.contains("Newly passing"), "{h}");
    let failing = &h[h.find("Newly failing").unwrap()..];
    assert!(failing.contains("<li class=\"bad\">") && failing.contains("Fits the street width"), "{h}");
    assert!(h.contains("impact-verdict regressed") && h.contains("Regressed: 1 check now fails."), "{h}");
    assert!(h.find("impact-verdict").unwrap() < h.find("impact-what").unwrap(), "the verdict leads: {h}");
    assert_eq!(count(&h, "<li class=\"bad\">"), 1);
    // Units follow the page.
    s.set_units(Units::Feet);
    assert!(last_change(&s).contains("ft"));
}

#[test]
fn undo_shows_what_undoing_changed_and_a_check_that_recovers_is_newly_passing() {
    let (s, time) = timed_street(Locale::En);
    let u = segment(&s, 0);
    assert!(s.apply(|e| e.set_width(u, 3_800)));
    time.advance(600);
    assert!(s.undo());
    time.advance(600);
    let h = last_change(&s);
    assert!(h.contains("Undone: Resize sidewalk"), "{h}");
    assert!(h.contains("Walking: 7.1 m \u{2192} 6.6 m (0.5 m less)"), "{h}");
    assert!(h.contains("Newly passing") && !h.contains("Newly failing"), "{h}");
    assert!(h.contains("<li class=\"ok\">") && h.contains("impact-verdict improved"), "{h}");
    assert!(s.redo());
    time.advance(600);
    assert!(last_change(&s).contains("Redone: Resize sidewalk"));
}

#[test]
fn a_change_that_touches_neither_space_nor_checks_says_so_and_has_no_lists() {
    let (s, time) = timed_street(Locale::En);
    assert!(s.set_speed(Some(30)));
    time.advance(600);
    let h = last_change(&s);
    assert!(h.contains("No effect on space or checks") && h.contains("Speed limit 30 km/h"), "{h}");
    assert!(!h.contains("impact-uses") && !h.contains("impact-verdict") && !h.contains("<ul"), "{h}");
}

#[test]
fn a_change_that_leaves_every_check_as_it_was_says_so_beside_the_space_it_moved() {
    let (s, time) = timed_street(Locale::En);
    let u = segment(&s, 0);
    assert!(s.apply(|e| e.set_width(u, 3_000)));
    time.advance(600);
    let h = last_change(&s);
    assert!(h.contains("Walking: 6.6 m \u{2192} 6.3 m (0.3 m less)") && !h.contains("Newly"), "{h}");
    assert!(h.contains("impact-verdict unchanged") && h.contains("No change in the checks."), "{h}");
}

#[test]
fn making_the_street_pedestrian_is_one_change_that_says_what_it_became() {
    let (s, time) = timed_street(Locale::En);
    let sidewalk = crate::shared::catalogue::kind_index("sidewalk").unwrap();
    assert!(s.apply(|e| e.make_pedestrian(sidewalk)));
    time.advance(600);
    let h = last_change(&s);
    assert!(h.contains("Made pedestrian") && h.contains("mc mc-vehicle") && h.contains("is now a pedestrian street."), "{h}");
}

#[test]
fn the_card_is_in_french_with_plurals_and_follows_a_switch_of_language() {
    let (s, time) = timed_street(Locale::FrCa);
    let u = segment(&s, 0);
    assert!(s.apply(|e| e.set_width(u, 3_800)));
    time.advance(600);
    let h = last_change(&s);
    assert!(h.contains("Marche\u{a0}: 6,6\u{a0}m \u{2192} 7,1\u{a0}m (0,5\u{a0}m de plus)"), "{h}");
    assert!(h.contains("Nouveaux échecs") && h.contains("Dégradation\u{a0}: 1 vérification échoue maintenant."), "{h}");
    assert!(!h.contains("Newly") && !h.contains("Walking"), "{h}");
    s.i18n().set(Locale::En);
    assert!(last_change(&s).contains("Newly failing"));
}

#[test]
fn the_card_is_a_polite_live_region_in_the_page_and_the_first_tab_is_called_impact() {
    let (vm, _) = street_watch();
    let page = html(|| view! { <view::notes_tray::StreetNotes vm=vm.clone()/> }.into_any());
    let card = &page[..page.find("id=\"impact-card\"").unwrap()];
    let card = &page[card.rfind('<').unwrap()..];
    let card = &card[..card.find('>').unwrap()];
    assert!(card.contains("aria-live=\"polite\"") && !card.contains("assertive") && card.contains("aria-atomic=\"true\""), "{card}");
    let first = button_tag(&page, "t-space");
    assert!(first.contains("aria-controls=\"p-space\"") && first.contains("aria-selected=\"true\""), "{first}");
    assert!(page.contains("<span>Impact</span>"), "{page}");
    // The card is in the first tab's panel, above the width table.
    assert!(page.find("id=\"impact-card\"").unwrap() < page.find("id=\"h-space\"").unwrap());
    let en = crate::i18n_for(Locale::En);
    let fr = crate::i18n_for(Locale::FrCa);
    let say = |i: &crate::shared::i18n::I18n| i.tr_now("ui-tab-impact", &crate::shared::i18n::Args::new());
    assert_eq!((say(&en).as_str(), say(&fr).as_str()), ("Impact", "Incidence"));
}

// ---- the raised elements' strip and tools -------------------------------------------

fn calming_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::view::calming_strip::CalmingStrip vm=s.clone()/> })
}

fn calming_tools_html(s: &Rc<StreetVm>) -> String {
    street_html(|| view! { <crate::street::view::calming_tools::CalmingTools vm=s.clone()/> })
}

/// A street of 40 m with a 7 m table at 0, chosen.
fn calming_street(locale: Locale) -> Rc<StreetVm> {
    use crate::street::model::CalmingKind;
    let s = zoned_street(&[0], Some(40_000), locale);
    s.select_calming(0);
    assert!(s.add_calming(CalmingKind::Table, 7_000));
    s
}

#[test]
fn the_calming_strip_has_a_cell_per_element_and_stretch_one_tab_stop_and_the_chosen_cell_active() {
    let s = calming_street(Locale::En);
    let h = calming_html(&s);
    assert!(h.contains("id=\"calming-strip\"") && h.contains("class=\"calming-strip\"") && h.contains("role=\"listbox\""), "{h}");
    assert!(h.contains("aria-orientation=\"horizontal\""), "{h}");
    assert_eq!(count(&h, "tabindex=\"0\""), 1, "{h}");
    assert_eq!(count(&h, "role=\"option\""), 2, "an element and the free stretch after it: {h}");
    assert!(h.contains("id=\"calming-0\"") && h.contains("id=\"calming-1\""), "{h}");
    assert!(h.contains("aria-activedescendant=\"calming-0\""), "{h}");
    assert_eq!(count(&h, "aria-selected=\"true\""), 1, "{h}");
    assert!(h.contains("aria-label=\"Raised table, 0.0 m to 7.0 m, 7.0 m long, 1 of 2\""), "{h}");
    assert!(h.contains("aria-label=\"Free, 7.0 m to 40.0 m, 33.0 m long, 2 of 2\""), "{h}");
    assert!(h.contains(">Raised tables, crossings, chicanes and modal filters along the street</h2>") && h.contains("class=\"sr-only\""), "{h}");
    assert!(h.contains("class=\"calming-cell gap\"") || h.contains("calming-cell gap"), "the free stretch is marked: {h}");
}

#[test]
fn the_calming_strip_shows_a_free_stretch_chosen_and_nothing_without_a_length() {
    let s = zoned_street(&[0], Some(40_000), Locale::En);
    let h = calming_html(&s);
    assert_eq!(count(&h, "role=\"option\""), 1, "{h}");
    assert!(h.contains("aria-activedescendant") == false || !h.contains("aria-activedescendant=\"calming-0\""), "nothing chosen: {h}");
    s.select_calming(0);
    assert!(calming_html(&s).contains("aria-activedescendant=\"calming-0\""));
    let bare = zoned_street(&[0], None, Locale::En);
    assert!(!calming_html(&bare).contains("calming-strip"), "no anchor, no strip");
}

#[test]
fn the_calming_strip_follows_the_language() {
    let s = calming_street(Locale::FrCa);
    let h = calming_html(&s).replace(['\u{a0}', '\u{202f}'], " ");
    assert!(h.contains(">Plateaux, passages surélevés, chicanes et filtres modaux le long de la rue</h2>"), "{h}");
    assert!(h.contains("aria-label=\"Plateau surélevé, de 0,0 m à 7,0 m, 7,0 m de long, 1 sur 2\""), "{h}");
}

#[test]
fn a_failing_element_is_marked_and_described_on_its_cell() {
    use crate::street::model::CalmingKind;
    let s = zoned_street(&[0], Some(40_000), Locale::En);
    s.select_calming(0);
    assert!(s.add_calming(CalmingKind::Table, 7_000));
    assert!(s.set_speed(Some(60)));
    let h = calming_html(&s);
    assert!(h.contains("failing") && h.contains("aria-description=\"Fails a check. See the checks.\""), "{h}");
}

#[test]
fn the_calming_tools_render_labelled_controls_for_the_chosen_element_with_the_kinds_in_order() {
    let s = calming_street(Locale::En);
    s.select_calming(0);
    let h = calming_tools_html(&s);
    assert!(h.contains("role=\"group\"") && h.contains("aria-label=\"Raised table commands\"") && !h.contains("toolbar"), "{h}");
    for (id, label) in [("calming-kind", "Kind"), ("calming-length", "Length (m)")] {
        assert!(h.contains(&format!("<label for=\"{id}\"")) && h.contains(&format!(">{label}</label>")), "{id}: {h}");
        assert!(h.contains(&format!("id=\"{id}\"")), "{id}: {h}");
    }
    assert!(h.contains("<select id=\"calming-kind\""), "{h}");
    assert_eq!(count(&h, "<option"), 4, "{h}");
    let table = h.find("value=\"0\"").unwrap();
    let crossing = h.find("value=\"1\"").unwrap();
    let chicane = h.find("value=\"2\"").unwrap();
    let filter = h.find("value=\"3\"").unwrap();
    assert!(table < crossing && crossing < chicane && chicane < filter, "{h}");
    assert!(
        h.contains(">Raised table</option>")
            && h.contains(">Raised crossing</option>")
            && h.contains(">Chicane</option>")
            && h.contains(">Modal filter</option>"),
        "{h}"
    );
    assert!(!h.contains("calming-first"), "only a chicane has a side to start on: {h}");
    assert!(h.contains("id=\"calming-remove\"") && h.contains(">Remove raised element</button>"), "{h}");
    assert!(!h.contains("calming-add") && !h.contains("Add raised element"), "adding is in the Add a piece menu: {h}");
    assert!(!h.contains("aria-disabled"), "an element is chosen: remove is on: {h}");
}

#[test]
fn the_calming_tools_show_only_while_an_element_is_chosen() {
    let s = calming_street(Locale::En);
    s.select_calming(1);
    let h = calming_tools_html(&s);
    assert!(!h.contains("calming-") && !h.contains("Raised table commands"), "a free stretch is chosen: {h}");
    s.select_calming(0);
    assert!(calming_tools_html(&s).contains("id=\"calming-length\""));
}

#[test]
fn the_calming_tools_offer_the_starting_side_for_a_chicane_only() {
    use crate::street::model::CalmingKind;
    let chicane_street = |locale| {
        let s = calming_street(locale);
        s.select_calming(1);
        assert!(s.add_calming(CalmingKind::Chicane, 24_000));
        s.select_calming(1);
        s
    };
    let s = chicane_street(Locale::En);
    let h = calming_tools_html(&s);
    assert!(h.contains("<label for=\"calming-first\"") && h.contains(">Starts on</label>"), "{h}");
    assert!(h.contains("<select id=\"calming-first\"") && h.contains(">Left</option>") && h.contains(">Right</option>"), "{h}");
    let s = chicane_street(Locale::FrCa);
    let h = calming_tools_html(&s);
    assert!(h.contains(">Commence à</label>") && h.contains(">Gauche</option>") && h.contains(">Chicane</option>"), "{h}");
}

#[test]
fn the_calming_tools_can_remove_the_chosen_element() {
    let s = calming_street(Locale::En);
    let h = calming_tools_html(&s);
    assert!(!h.contains("id=\"calming-remove\" aria-disabled"), "an element is chosen: remove is on: {h}");
}

#[test]
fn the_add_menu_lists_traffic_calming_where_an_element_can_go() {
    let s = calming_street(Locale::En);
    s.select_calming(1);
    let h = street_html(|| view! { <add_menu::AddMenu vm=s.clone()/> });
    assert!(tag_of(&h, "add-topic-calming").contains("aria-selected=\"true\"") && h.contains(">Traffic calming</button>"), "{h}");
    let panel = add_panel(&h);
    assert_eq!(count(panel, "class=\"add-item\""), 4, "{panel}");
    let at = |name: &str| panel.find(&format!("<b>{name}</b>")).unwrap_or_else(|| panic!("{name}: {panel}"));
    assert!(at("Raised table") < at("Raised crossing") && at("Raised crossing") < at("Chicane") && at("Chicane") < at("Modal filter"), "{panel}");
    assert!(panel.contains("7.0 m") && panel.contains("24.0 m") && panel.contains("2.0 m"), "each kind's default length: {panel}");
    assert!(!panel.contains("aria-disabled"), "{panel}");
    // A street with no stretch to anchor an element on still has the topic.
    let none = street_shared(0);
    let h = street_html(|| view! { <add_menu::AddMenu vm=none.clone()/> });
    assert!(h.contains("id=\"add-topic-calming\"") && h.contains(">Traffic calming</button>"), "{h}");
    let fr = calming_street(Locale::FrCa);
    fr.select_calming(1);
    let h = street_html(|| view! { <add_menu::AddMenu vm=fr.clone()/> });
    assert!(h.contains(">Apaisement de la circulation</button>") && h.contains("<b>Plateau surélevé</b>"), "{h}");
}

#[test]
fn the_calming_tools_follow_the_language_and_the_units() {
    let s = calming_street(Locale::FrCa);
    s.select_calming(0);
    let h = calming_tools_html(&s);
    assert!(h.contains("aria-label=\"Commandes des éléments surélevés\"") && h.contains(">Plateau surélevé</option>"), "{h}");
}

// ---- the flat figure of the 2.5D --------------------------------------------------------

fn en() -> std::rc::Rc<crate::shared::i18n::I18n> {
    crate::i18n_for(Locale::En)
}

fn in_box(f: &cityloom_report::Figure, w: f32, h: f32) -> bool {
    use cityloom_report::Item;
    let eps = 0.01;
    f.items.iter().all(|i| match i {
        Item::Poly { pts, .. } | Item::Line { pts, .. } => pts.iter().all(|p| p.0 >= -eps && p.0 <= w + eps && p.1 >= -eps && p.1 <= h + eps),
        Item::Text { .. } => true,
    })
}

#[test]
fn the_street_figure_has_polygons_inside_its_box() {
    use cityloom_report::Item;
    let f = crate::street::view::drawing::figure(&Editor::new(0).view(), &en(), 252.0, 200.0);
    assert!(f.items.iter().any(|i| matches!(i, Item::Poly { .. })));
    assert!(in_box(&f, 252.0, 200.0), "{f:?}");
    assert!(f.width <= 252.01 && f.height <= 200.01);
}

#[test]
fn the_street_figure_has_no_selection_outline() {
    let mut e = Editor::new(0);
    let plain = crate::street::view::drawing::figure(&e.view(), &en(), 252.0, 200.0);
    let uid = e.view().segments[0].uid;
    e.select(Some(uid));
    assert_eq!(crate::street::view::drawing::figure(&e.view(), &en(), 252.0, 200.0), plain);
}

#[test]
fn two_streets_with_different_widths_give_different_figures() {
    let (a, b) = (Editor::new(0), Editor::new(1));
    assert_ne!(a.view().total_mm, b.view().total_mm);
    assert_ne!(crate::street::view::drawing::figure(&a.view(), &en(), 252.0, 200.0), crate::street::view::drawing::figure(&b.view(), &en(), 252.0, 200.0));
}

#[test]
fn the_street_alt_text_names_what_is_drawn() {
    let v = Editor::new(0).view();
    let (en, fr) = (
        crate::street::view::drawing::alt(&v, &crate::i18n_for(Locale::En), crate::shared::units::Units::Metres),
        crate::street::view::drawing::alt(&v, &crate::i18n_for(Locale::FrCa), crate::shared::units::Units::Metres),
    );
    assert!(!en.is_empty() && !fr.is_empty() && en != fr, "{en} / {fr}");
}

#[test]
fn the_neighbouring_zone_is_not_drawn_in_the_figure() {
    use crate::street::model::two_zones;
    // Two zones, the boundary near: the page draws two sections, the figure the shown one.
    let z = two_zones(true);
    let v = z.view();
    assert!(v.zones.joint.is_some(), "the fixture's boundary is near");
    let (all, p) = crate::street::view::iso::diagram_and_projection(&v, &en(), 252.0);
    assert_eq!(all.sections.len(), 2);
    let both = crate::shared::flat::flatten(&all, &p, 252.0, 200.0);
    let one = crate::street::view::drawing::figure(&v, &en(), 252.0, 200.0);
    assert!(one.items.len() < both.items.len(), "{} {}", one.items.len(), both.items.len());
}

#[test]
fn a_fitted_street_figure_leaves_out_the_speed_signs_words() {
    use cityloom_report::Item;
    let mut e = Editor::new(0);
    e.set_speed(Some(50));
    let f = crate::street::view::drawing::figure(&e.view(), &en(), 252.0, 200.0);
    // Only street names are kept in a fitted figure, so the sign's number and word are not drawn.
    assert!(!f.items.iter().any(|i| matches!(i, Item::Text { .. })), "{:?}", f.items.iter().filter(|i| matches!(i, Item::Text { .. })).collect::<Vec<_>>());
}

#[test]
fn the_checks_list_what_fails_first_and_fold_what_passes_until_asked() {
    let s = street_shared(0);
    let u = segment(&s, 0);
    s.edit(|e| e.nudge_width(u, 100)); // too wide: the street no longer fits
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    let rows: Vec<&str> = h.split("<li ").skip(1).collect();
    assert!(rows.len() > 2, "{h}");
    assert!(rows[0].starts_with(r#"class="bad""#) && rows[0].contains("Fits the street width"), "the failure first: {h}");
    let (failing, passing) = (rows.iter().filter(|r| r.contains(r#"class="bad""#)).count(), rows.iter().filter(|r| r.contains(r#"class="ok""#)).count());
    assert!(rows[..failing].iter().all(|r| r.contains(r#"class="bad""#)), "every failure before every pass: {h}");
    assert!(rows.iter().filter(|r| r.contains(r#"class="ok""#)).all(|r| r[..r.find('>').unwrap()].contains("hidden")), "passes folded: {h}");
    let toggle = street_html(|| view! { <notes::PassingChecks vm=s.clone()/> });
    assert!(toggle.contains(&format!(">Show the {passing} passing checks<")), "{toggle}");
    s.passes().toggle();
    let h = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(!h.contains(" hidden"), "shown once asked: {h}");
    assert!(street_html(|| view! { <notes::PassingChecks vm=s.clone()/> }).contains(r#"aria-expanded="true""#));
}

#[test]
fn choosing_a_goal_marks_the_checks_that_bear_on_it_and_leaves_the_street_as_it_was() {
    use crate::shared::goals::Goal;
    let s = street_shared(0);
    let before = serde_json::to_string(&*s.view_now()).unwrap();
    let plain = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(!plain.contains("goal-mark"));
    s.goal().set(Some(Goal::SchoolRoute));
    let marked = street_html(|| view! { <notes::Checks vm=s.clone()/> });
    assert!(marked.contains("goal-mark") && marked.contains("Bears on your goal"), "{marked}");
    let guide = street_html(|| view! { <notes::GoalGuide vm=s.clone()/> });
    assert!(guide.contains("Improve a school route"), "{guide}");
    s.goal().set(None);
    assert_eq!(serde_json::to_string(&*s.view_now()).unwrap(), before, "a goal is a preference, not an edit");
    assert!(!street_html(|| view! { <notes::Checks vm=s.clone()/> }).contains("goal-mark"));
}

// ---- the notes' tray of the street page ------------------------------------------------------

#[test]
fn the_street_s_notes_are_the_shell_s_tray_with_its_three_tabs_and_the_slots_the_page_fills() {
    let (s, _) = street_watch();
    let h = html(|| view! { <view::notes_tray::StreetNotes vm=s.clone()/> }.into_any());
    assert!(h.starts_with("<aside ") && h.contains(r#"class="notes tray""#), "{h}");
    for (tab, panel, label) in [("t-space", "p-space", "Impact"), ("t-checks", "p-checks", "Checks"), ("t-changes", "p-changes", "Changes")] {
        let b = button_tag(&h, tab);
        assert!(b.contains(&format!(r#"aria-controls="{panel}""#)), "{b}");
        let at = h.find(&format!(r#"id="{panel}""#)).unwrap_or_else(|| panic!("{panel}"));
        let tag = &h[h[..at].rfind('<').unwrap()..at + h[at..].find('>').unwrap()];
        assert!(tag.contains(r#"role="tabpanel""#) && tag.contains(&format!(r#"aria-labelledby="{tab}""#)), "{tag}");
        assert!(h.contains(&format!("<span>{label}</span>")), "{label}");
    }
    assert_eq!(count(&h, r#"role="tabpanel""#), 3);
    assert_eq!(count(&h, " hidden"), 2, "only the first panel shows until the shell shows the chosen one");
    for gone in ["t-provenance", "p-provenance", r#"id="provenance""#, "Provenance", "Where this street came from"] {
        assert!(!h.contains(gone), "where the street came from is in the title block, under every tab: {gone}");
    }
    // every slot `mount_street_page` fills, once each, inside the tray
    for id in [
        "fit-slot",
        "fit-checks-slot",
        "checks-n-slot",
        "impact",
        "space",
        "cap",
        "time-note",
        "goal-slot",
        "checks",
        "checks-fold-slot",
        "atlas-source",
        "revs",
        "title-block",
    ] {
        assert_eq!(count(&h, &format!(r#"id="{id}""#)), 1, "{id}");
    }
    for heading in [
        "Last change",
        "Where the width goes",
        "People moved",
        "sample numbers",
        "Guided goals",
        "Does it work?",
        "sample limits",
        "Transit priority",
        "Your changes",
    ] {
        assert!(h.contains(heading), "{heading}: {h}");
    }
    let at = h.find(r#"id="impact-card""#).expect("the Last change card");
    let card = &h[h[..at].rfind('<').unwrap()..at + h[at..].find('>').unwrap()];
    for a in [r#"class="impact""#, r#"aria-labelledby="h-impact""#, r#"aria-live="polite""#, r#"aria-atomic="true""#] {
        assert!(card.contains(a), "{a}: {card}");
    }
    assert!(h.contains(r#"aria-label="About this street""#), "{h}");
}

#[test]
fn the_checks_tab_puts_the_checks_first_and_the_goal_and_its_measures_in_a_column_beside_them() {
    let (s, _) = street_watch();
    let h = html(|| view! { <view::notes_tray::StreetNotes vm=s.clone()/> }.into_any());
    let panel = &h[h.find(r#"id="p-checks""#).unwrap()..h.find(r#"id="p-changes""#).unwrap()];
    let at = |s: &str| panel.find(s).unwrap_or_else(|| panic!("{s}: {panel}"));
    assert!(at(r#"class="checks-split""#) < at(r#"id="h-checks""#), "{panel}");
    assert!(at(r#"id="checks""#) < at(r#"id="checks-fold-slot""#), "the fold's button under the list: {panel}");
    assert!(at(r#"id="checks-fold-slot""#) < at(r#"class="checks-side""#), "{panel}");
    assert!(
        at(r#"class="checks-side""#) < at(r#"id="h-goal""#) && at(r#"id="h-goal""#) < at(r#"id="h-atlas""#),
        "the goal, then where the measures' names come from: {panel}"
    );
}

#[test]
fn the_street_page_leaves_a_slot_for_its_notes_and_writes_none_of_the_tray_itself() {
    let page = include_str!("../../../../../web/street.html");
    assert!(page.contains(r#"<div id="notes-slot" style="display: contents"></div>"#));
    for gone in ["tray-handle", "notes-toggle", "notes-body", r#"id="t-space""#, r#"id="impact""#] {
        assert!(!page.contains(gone), "{gone} is drawn by the tray, not written in the page");
    }
    assert!(page.contains(r#"href="v/{{build}}/tray.css""#), "the tray's own stylesheet");
}
