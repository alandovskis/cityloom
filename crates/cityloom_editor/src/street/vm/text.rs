//! What the street page says in words: the line that says whether the pieces fit
//! the street, and what a screen reader is told after an edit or a selection.
//!
//! Every sentence is a message (`src/street/i18n`); these functions only choose
//! which one and hand it the lengths, written in the language of the page.

use crate::shared::catalogue::{KINDS, StreetClass, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::{Arg, Said};
use crate::shared::units::Units;
use crate::street::model::{
    CalmingStripView, Check, GreenRowView, PlanAxis, PlanView, SideView, StripCell, Template, TemplateRefusal, TemplateReport, View, ZoneRefusal,
};

/// What an edit says of the street becoming pedestrian or carrying traffic again, after what it already said;
/// nothing when the class is the one it was.
pub fn class_change_text(before: StreetClass, v: &View, i18n: &I18n, units: Units) -> Option<String> {
    let street = Args::new().str("street", say_now(i18n, units, &v.name));
    match (before == StreetClass::Pedestrian, v.class == StreetClass::Pedestrian) {
        (false, true) => Some(i18n.tr_now("street-now-pedestrian", &street)),
        (true, false) => Some(i18n.tr_now("street-traffic-again", &street)),
        _ => None,
    }
}

/// What the model said, in words, for a view (see `shared::said`).
pub fn say(i18n: &I18n, units: Units, said: &Said) -> String {
    crate::shared::said::say(i18n, units, said)
}

/// The same, for a command: nothing is watched.
pub fn say_now(i18n: &I18n, units: Units, said: &Said) -> String {
    crate::shared::said::say_now(i18n, units, said)
}

/// Said when the street is put back as it is today.
pub fn started_over(i18n: &I18n) -> String {
    i18n.tr_now("started-over", &Args::new())
}

/// Said when a measure cannot be laid out in this street.
pub fn measure_refused(i18n: &I18n) -> String {
    i18n.tr_now("measure-refused", &Args::new())
}

/// How the pieces stand against the street's width, as a phrase with no full stop.
pub fn fit_phrase(v: &View, i18n: &I18n, units: Units) -> String {
    let amount = |mm: i32| Args::new().str("amount", units.length_fine_in(mm, i18n.locale_now()));
    match v.delta_mm {
        0 => i18n.tr_now("fit-used", &Args::new()),
        d if d < 0 => i18n.tr_now("fit-left", &amount(-d)),
        d => i18n.tr_now("fit-over", &amount(d)),
    }
}

/// The status line under the drawing.
pub fn status_text(v: &View, i18n: &I18n, units: Units) -> String {
    let amount = |mm: i32| Args::new().str("amount", units.length_fine_in(mm, i18n.locale_now()));
    match v.delta_mm {
        0 => i18n.tr_now("status-used", &Args::new()),
        d if d < 0 => i18n.tr_now("status-unused", &amount(-d)),
        d => i18n.tr_now("status-over", &amount(d)),
    }
}

/// What the last edit was, and how the pieces stand.
pub fn edit_text(v: &View, i18n: &I18n, units: Units) -> Option<String> {
    v.revisions.last().map(|r| i18n.tr_now("edit-done", &Args::new().str("label", say_now(i18n, units, &r.label)).str("fit", fit_phrase(v, i18n, units))))
}

pub fn undone_text(v: &View, i18n: &I18n, units: Units) -> String {
    i18n.tr_now("edit-undone", &Args::new().str("fit", fit_phrase(v, i18n, units)))
}

pub fn redone_text(v: &View, i18n: &I18n, units: Units) -> String {
    i18n.tr_now("edit-redone", &Args::new().str("fit", fit_phrase(v, i18n, units)))
}

/// The selected piece, its width and its place among the pieces; nothing when
/// nothing is selected.
pub fn selection_text(v: &View, i18n: &I18n, units: Units) -> Option<String> {
    let uid = v.selected?;
    let i = v.segments.iter().position(|s| s.uid == uid)?;
    Some(piece_text(v, i, i18n, units))
}

/// Piece `i`: its kind, its width and its place among the pieces, which is how a cell of the piece list is named and
/// what is said when the piece is selected.
pub fn piece_text(v: &View, i: usize, i18n: &I18n, units: Units) -> String {
    let s = &v.segments[i];
    let args = Args::new()
        .str("kind", i18n.tr_now(&kind_key(KINDS[s.kind].id), &Args::new()))
        .str("width", units.length_fine_in(s.width_mm, i18n.locale_now()))
        .num("index", i as i64 + 1)
        .num("total", v.segments.len() as i64);
    i18n.tr_now("selection-position", &args)
}

/// How a cell of a strip is named (by the message `key`): what it is, where it lies and how long, and its place on
/// the strip.
pub fn cell_text<C: StripCell>(key: &str, side: &SideView<C>, cell: usize, axis: PlanAxis, i18n: &I18n, units: Units) -> String {
    let locale = i18n.locale_now();
    let Some(c) = side.cells.get(cell) else { return String::new() };
    let (from, to) = axis.left_span(c.from_mm(), c.to_mm());
    let args = Args::new()
        .str("name", say_now(i18n, units, c.label()))
        .str("from", units.length_in(from, locale))
        .str("to", units.length_in(to, locale))
        .str("length", units.length_in(to - from, locale))
        .num("n", cell as i64 + 1)
        .num("total", side.cells.len() as i64);
    i18n.tr_now(key, &args)
}

/// What a screen reader is told on choosing cell `cell` of the raised elements' strip: its kind (or "Free"), where it
/// lies, how long it is and its place on the strip.
pub fn calming_text(strip: &CalmingStripView, cell: usize, axis: PlanAxis, i18n: &I18n, units: Units) -> String {
    let locale = i18n.locale_now();
    let Some(c) = strip.cells.get(cell) else { return String::new() };
    let (from, to) = axis.left_span(c.from_mm, c.to_mm);
    let args = Args::new()
        .str("name", say_now(i18n, units, &c.label))
        .str("from", units.length_in(from, locale))
        .str("to", units.length_in(to, locale))
        .str("length", units.length_in(to - from, locale))
        .num("n", cell as i64 + 1)
        .num("total", strip.cells.len() as i64);
    i18n.tr_now("calming-cell", &args)
}

/// What a screen reader is told of element `cell` of the Green row: its name, where it lies along the street and
/// its place in the row.
pub fn green_cell_text(g: &GreenRowView, cell: usize, axis: PlanAxis, i18n: &I18n, units: Units) -> String {
    let locale = i18n.locale_now();
    let Some(c) = g.cells.get(cell) else { return String::new() };
    let (from, to) = axis.left_span(c.from_mm, c.to_mm);
    let args = Args::new()
        .str("name", say_now(i18n, units, &c.label))
        .str("from", units.length_in(from, locale))
        .str("to", units.length_in(to, locale))
        .str("length", units.length_in(to - from, locale))
        .num("n", cell as i64 + 1)
        .num("total", g.cells.len() as i64);
    i18n.tr_now("green-cell", &args)
}

/// Why a boundary could not be moved (or the street changed while a drag is under way), in the page's language.
pub fn zone_refused(i18n: &I18n, units: Units, refusal: ZoneRefusal) -> String {
    say_now(i18n, units, &refusal.said())
}

/// Why a street template cannot be applied, in the page's language.
pub fn template_refused(i18n: &I18n, units: Units, template: Template, refusal: TemplateRefusal) -> String {
    say_now(i18n, units, &refusal.said(template))
}

/// What a template did, part by part, in the page's language: the parts applied, then those skipped and why.
/// Nothing for a template that has only its pieces.
pub fn template_report(i18n: &I18n, units: Units, report: &TemplateReport) -> Option<String> {
    let parts: Vec<String> = report.applied.iter().chain(&report.skipped).map(|s| say_now(i18n, units, s)).collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// What is said of the pick: the stretch from the plan's left end, with `what` sets it apart from the run beside it
/// (`Editor::pick_differs`) when there is anything, or the whole street. Nothing for a street with no length to
/// measure along.
pub fn picked_said(plan: &PlanView, what: Vec<Said>) -> Option<Said> {
    let Some((from, to)) = plan.pick else {
        return plan.length_mm.map(|length| Said::new("stretch-whole").with("length", Arg::Length(length)));
    };
    let span = |key| Said::new(key).with("from", Arg::Length(from)).with("to", Arg::Length(to)).with("length", Arg::Length(to - from));
    Some(match listed(what) {
        Some(what) => span("stretch-picked-differs").with("what", Arg::Said(Box::new(what))),
        None => span("stretch-picked"),
    })
}

/// Where the plan's keyboard point is, `at_mm` from the plan's left end.
pub fn point_said(at_mm: i32) -> Said {
    Said::new("st-point").with("at", Arg::Length(at_mm))
}

/// The plan's name for a screen reader: the street from end to end, and how long. Nothing for a street with no length.
pub fn plan_label(plan: &PlanView) -> Option<Said> {
    plan.length_mm.map(|length| Said::new("st-plan-label").with("length", Arg::Length(length)))
}

/// The line under the plan: the picked stretch and how long it is. Nothing for the whole street.
pub fn stretch_label(plan: &PlanView) -> Option<Said> {
    plan.pick.map(|(from, to)| Said::new("st-stretch").with("from", Arg::Length(from)).with("to", Arg::Length(to)).with("length", Arg::Length(to - from)))
}

/// What is said under a figure that differs along the pick: along the stretch, or along the street when the whole
/// street is picked.
pub fn varies_said(plan: &PlanView) -> Said {
    Said::new(if plan.pick.is_some() { "st-varies" } else { "st-varies-street" })
}

/// Where an edit did not reach (`spans`, from the plan's left end), each a sentence naming what that stretch lacks
/// (`kind`, a catalogue id).
pub fn missed_said(spans: &[(i32, i32)], kind: &'static str) -> Vec<Said> {
    spans
        .iter()
        .map(|&(from, to)| Said::new("edit-missed").with("from", Arg::Length(from)).with("to", Arg::Length(to)).with("kind", Arg::KindLower(kind)))
        .collect()
}

/// `items` as one list: "a, b and c".
fn listed(items: Vec<Said>) -> Option<Said> {
    let mut items = items.into_iter().rev();
    let last = items.next()?;
    let Some(before) = items.next() else { return Some(last) };
    let pair = |key, a: Said, b: Said| Said::new(key).with("a", Arg::Said(Box::new(a))).with("b", Arg::Said(Box::new(b)));
    Some(items.fold(pair("stretch-what-pair", before, last), |list, item| pair("stretch-what-more", item, list)))
}

/// What a check is called where it is listed: a failing check of a street that changes along its length says where
/// it fails (a stretch, "from 49 m to 82 m", or where the street changes).
pub fn check_label(check: &Check) -> Said {
    match (&check.zone, check.ok) {
        // A curbside row names the use that fails, not a zone.
        (Some(instance), false) if check.id.starts_with("curb-") => {
            Said::new("check-curb-in").with("check", Arg::Said(Box::new(check.label.clone()))).with("instance", Arg::Said(Box::new(instance.clone())))
        }
        // A stop row names the stop that fails.
        (Some(instance), false) if check.id.starts_with("stop-") => {
            Said::new("check-stop-in").with("check", Arg::Said(Box::new(check.label.clone()))).with("instance", Arg::Said(Box::new(instance.clone())))
        }
        // A raised element's row names the element (and the zone, where the zone is what fails).
        (Some(instance), false) if check.id.starts_with("calming-") => {
            Said::new("check-calming-in").with("check", Arg::Said(Box::new(check.label.clone()))).with("instance", Arg::Said(Box::new(instance.clone())))
        }
        // A row of the trees and planting names the element that fails.
        (Some(instance), false) if check.id.starts_with("green-") => {
            Said::new("check-green-in").with("check", Arg::Said(Box::new(check.label.clone()))).with("instance", Arg::Said(Box::new(instance.clone())))
        }
        (Some(stretch), false) => {
            Said::new("check-stretch-in").with("check", Arg::Said(Box::new(check.label.clone()))).with("stretch", Arg::Said(Box::new(stretch.clone())))
        }
        _ => check.label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::street::model::Editor;
    use crate::street::model::StreetExt;

    fn en() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::En)
    }

    fn fr() -> std::rc::Rc<I18n> {
        crate::i18n_for(Locale::FrCa)
    }

    fn uid(e: &Editor, i: usize) -> u32 {
        e.view().segments[i].uid
    }

    #[test]
    fn the_pick_the_plan_and_what_sets_a_stretch_apart_are_worded_in_both_languages() {
        use crate::street::model::{ZoneEnd, ZoneFrame};
        let mut e = Editor::new(0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
        let words = |i18n: &I18n, said: Option<Said>| say_now(i18n, Units::Metres, &said.expect("something to say"));
        let plan = e.view().plan;
        assert_eq!(words(&en(), picked_said(&plan, Vec::new())), "The whole street, 60.0 m long.");
        assert_eq!(words(&en(), plan_label(&plan)), "The street from its first end to its last, 60.0 m long");
        assert_eq!(stretch_label(&plan), None, "no line for the whole street");
        e.pick_stretch(10_000, 30_000).unwrap();
        let plan = e.view().plan;
        assert_eq!(words(&en(), picked_said(&plan, Vec::new())), "From 10.0 m to 30.0 m, 20.0 m long.");
        assert_eq!(words(&fr(), stretch_label(&plan)), "De 10,0\u{a0}m à 30,0\u{a0}m · 20,0\u{a0}m de long");
        let what = || {
            vec![
                Said::new("stretch-what-added").with("kind", Arg::KindLower("bike")).with("side", Arg::Msg("side-word-right")),
                Said::new("stretch-what-dropped").with("kind", Arg::KindLower("parking")).with("side", Arg::Msg("side-word-left")),
                Said::new("stretch-what-speed").with("kmh", Arg::Num(30)),
            ]
        };
        assert_eq!(
            words(&en(), picked_said(&plan, what())),
            "From 10.0 m to 30.0 m, 20.0 m long: bike lane added on the right, no parking on the left and a speed limit of 30 km/h."
        );
        assert_eq!(
            words(&fr(), picked_said(&plan, what())),
            "De 10,0\u{a0}m à 30,0\u{a0}m, 20,0\u{a0}m de long\u{a0}: piste cyclable en plus à droite, sans stationnement à gauche et une limite de 30\u{a0}km/h."
        );
        assert_eq!(words(&en(), Some(point_said(12_300))), "At 12.3 m");
        assert_eq!(words(&fr(), Some(point_said(12_300))), "À 12,3\u{a0}m");
        assert_eq!(words(&en(), missed_said(&[(20_000, 40_000)], "parking").pop()), "Not changed from 20.0 m to 40.0 m: that stretch has no parking.");
        assert_eq!(
            words(&fr(), missed_said(&[(20_000, 40_000)], "parking").pop()),
            "Aucun changement de 20,0\u{a0}m à 40,0\u{a0}m\u{a0}: ce tronçon est sans stationnement."
        );
    }

    #[test]
    fn a_street_that_is_exactly_full_says_every_metre_is_used() {
        let v = Editor::new(0).view();
        assert_eq!(v.delta_mm, 0);
        assert_eq!(fit_phrase(&v, &en(), Units::Metres), "Every metre of the street is used");
        assert_eq!(status_text(&v, &en(), Units::Metres), "Every metre of the street is used.");
    }

    #[test]
    fn a_street_with_room_left_says_how_much() {
        let mut e = Editor::new(0);
        let first = uid(&e, 0);
        assert!(e.remove(first));
        let v = e.view();
        assert_eq!(fit_phrase(&v, &en(), Units::Metres), "3.3 m left to use");
        assert_eq!(status_text(&v, &en(), Units::Metres), "3.3 m of the street is still unused.");
        assert_eq!(fit_phrase(&v, &en(), Units::Feet), "10.8 ft left to use");
    }

    #[test]
    fn a_street_that_is_too_full_says_how_much_too_wide() {
        let mut e = Editor::new(0);
        let first = uid(&e, 0);
        assert!(e.set_width(first, e.view().segments[0].width_mm + 500));
        let v = e.view();
        assert!(v.delta_mm > 0);
        assert_eq!(fit_phrase(&v, &en(), Units::Metres), "0.5 m too wide. Make a piece narrower or remove one");
        assert_eq!(status_text(&v, &en(), Units::Metres), "0.5 m too wide. Make a piece narrower or remove one.");
    }

    #[test]
    fn an_edit_is_announced_by_its_label_and_how_the_pieces_stand() {
        let mut e = Editor::new(0);
        assert_eq!(edit_text(&e.view(), &en(), Units::Metres), None);
        let first = uid(&e, 0);
        e.remove(first);
        let said = edit_text(&e.view(), &en(), Units::Metres).unwrap();
        assert!(said.ends_with(". 3.3 m left to use."), "{said}");
        assert!(said.starts_with(&say_now(&en(), Units::Metres, &e.view().revisions.last().unwrap().label)));
    }

    #[test]
    fn undo_and_redo_are_announced_with_how_the_pieces_stand() {
        let v = Editor::new(0).view();
        assert_eq!(undone_text(&v, &en(), Units::Metres), "Undone. Every metre of the street is used.");
        assert_eq!(redone_text(&v, &en(), Units::Metres), "Redone. Every metre of the street is used.");
    }

    #[test]
    fn the_selected_piece_is_announced_with_its_width_and_place() {
        let mut e = Editor::new(0);
        assert_eq!(selection_text(&e.view(), &en(), Units::Metres), None);
        let second = uid(&e, 1);
        e.select(Some(second));
        let v = e.view();
        let s = &v.segments[1];
        assert_eq!(
            selection_text(&v, &en(), Units::Metres).unwrap(),
            format!("{}, {} m, 2 of {}", en().tr_now(&kind_key(KINDS[s.kind].id), &Args::new()), Units::Metres.fine(s.width_mm), v.segments.len())
        );
    }

    #[test]
    fn the_fit_phrase_and_status_are_said_in_french() {
        let mut e = Editor::new(0);
        let first = uid(&e, 0);
        let v = e.view();
        assert_eq!(fit_phrase(&v, &fr(), Units::Metres), "Chaque mètre de la rue est utilisé");
        assert_eq!(status_text(&v, &fr(), Units::Metres), "Chaque mètre de la rue est utilisé.");
        assert!(e.set_width(first, e.view().segments[0].width_mm + 500));
        let v = e.view();
        assert_eq!(fit_phrase(&v, &fr(), Units::Metres), "0,5\u{a0}m de trop. Rétrécissez un élément ou retirez-en un");
        assert_eq!(status_text(&v, &fr(), Units::Metres), "0,5\u{a0}m de trop. Rétrécissez un élément ou retirez-en un.");
        let mut e = Editor::new(0);
        assert!(e.remove(first));
        let v = e.view();
        assert_eq!(fit_phrase(&v, &fr(), Units::Metres), "Il reste 3,3\u{a0}m à utiliser");
        assert_eq!(status_text(&v, &fr(), Units::Metres), "Il reste 3,3\u{a0}m de la rue à utiliser.");
    }

    #[test]
    fn undo_redo_selection_and_the_two_refusals_are_said_in_french() {
        let mut e = Editor::new(0);
        let v = e.view();
        assert_eq!(undone_text(&v, &fr(), Units::Metres), "Annulé. Chaque mètre de la rue est utilisé.");
        assert_eq!(redone_text(&v, &fr(), Units::Metres), "Rétabli. Chaque mètre de la rue est utilisé.");
        let second = uid(&e, 1);
        e.select(Some(second));
        let said = selection_text(&e.view(), &fr(), Units::Metres).unwrap();
        assert!(said.ends_with(", 2 sur 6"), "{said}");
        assert!(said.contains("\u{a0}m, "), "{said}");
        assert_eq!(started_over(&fr()), "Retour à la rue telle qu’elle est aujourd’hui. Annulez pour retrouver vos modifications.");
        assert_eq!(measure_refused(&fr()), "Cette mesure ne convient pas à cette rue.");
        assert_eq!(measure_refused(&en()), "That measure does not suit this street.");
    }

    fn english_after(sample: usize, edit: impl FnOnce(&mut Editor)) -> String {
        let mut e = Editor::new(sample);
        edit(&mut e);
        say_now(&en(), Units::Metres, &e.view().revisions.last().unwrap().label)
    }

    fn french_after(sample: usize, edit: impl FnOnce(&mut Editor)) -> String {
        let mut e = Editor::new(sample);
        edit(&mut e);
        say_now(&fr(), Units::Metres, &e.view().revisions.last().unwrap().label)
    }

    fn piece(e: &Editor, id: &str) -> u32 {
        e.view().segments.iter().find(|s| KINDS[s.kind].id == id).unwrap().uid
    }

    #[test]
    fn a_revision_is_said_with_the_names_of_the_page_s_language() {
        let remove = |e: &mut Editor| {
            let u = piece(e, "parking");
            assert!(e.remove(u));
        };
        assert_eq!(english_after(0, remove), "Remove parking");
        assert_eq!(french_after(0, remove), "Retrait\u{a0}: stationnement");
        let surface = |e: &mut Editor| {
            let u = piece(e, "parking");
            assert!(e.set_material(u, crate::shared::catalogue::MATERIALS.iter().position(|m| m.id == "permeable").unwrap()));
        };
        assert_eq!(english_after(0, surface), "Parking surface: permeable paving");
        assert_eq!(french_after(0, surface), "Stationnement, surface\u{a0}: revêtement perméable");
        let resize = |e: &mut Editor| {
            let bike = KINDS.iter().position(|k| k.id == "bike").unwrap();
            let u = e.add(bike, 0);
            assert!(e.set_width(u, 2_000));
        };
        assert_eq!(english_after(0, resize), "Resize bike lane");
        assert_eq!(french_after(0, resize), "Redimensionnement\u{a0}: piste cyclable");
    }

    #[test]
    fn a_window_a_vehicle_and_a_reset_are_said_in_both_languages() {
        let window = |e: &mut Editor| {
            let u = piece(e, "parking");
            assert!(e.add_variant(u));
        };
        assert_eq!(english_after(0, window), "Parking is transit lane 07:00-10:00");
        assert_eq!(french_after(0, window), "Stationnement devient voie réservée au transport en commun de 07:00 à 10:00");
        let tram = |e: &mut Editor| {
            let u = piece(e, "parking");
            assert!(e.add_variant(u));
            e.set_time(8 * 60);
            assert!(e.set_tram(u, true));
        };
        assert_eq!(english_after(0, tram), "Transit lane vehicle: tram");
        assert_eq!(french_after(0, tram), "Voie réservée au transport en commun, véhicule\u{a0}: tramway");
        let reset = |e: &mut Editor| {
            let u = piece(e, "parking");
            assert!(e.remove(u));
            assert!(e.reset());
        };
        assert_eq!(english_after(0, reset), "Reset to existing");
        assert_eq!(french_after(0, reset), "Retour à l’état existant");
    }

    #[test]
    fn the_changes_follow_a_switch_of_language_instead_of_freezing_the_one_they_were_made_in() {
        let mut e = Editor::new(0);
        let u = piece(&e, "parking");
        e.remove(u);
        let v = e.view();
        let i18n = crate::i18n_for(Locale::En);
        let en_said = edit_text(&v, &i18n, Units::Metres).unwrap();
        assert!(en_said.starts_with("Remove parking. "), "{en_said}");
        i18n.set(Locale::FrCa);
        let fr_said = edit_text(&v, &i18n, Units::Metres).unwrap();
        assert!(fr_said.starts_with("Retrait\u{a0}: stationnement. "), "{fr_said}");
    }

    fn zoned_editor() -> Editor {
        use crate::shared::catalogue::Side;
        use crate::street::model::{Street, ZoneEnd, ZoneFrame, zoned};
        let street = zoned(Street::sample(0, Side::Right), &[0, 10_000, 30_000], 40_000);
        Editor::from_street(&street, &street, 0).with_frame(Some(ZoneFrame { length_mm: 40_000, from_end: ZoneEnd::First }))
    }

    #[test]
    fn the_words_are_in_both_languages_and_the_french_has_its_typography() {
        use crate::street::model::ZoneRefusal;
        assert_eq!(zone_refused(&en(), Units::Metres, ZoneRefusal::TooShort), "A stretch must be at least 5.0 m long.");
        assert_eq!(zone_refused(&fr(), Units::Metres, ZoneRefusal::TooShort), "Un tronçon doit mesurer au moins 5,0\u{a0}m.");
    }

    #[test]
    fn a_failing_stop_row_reads_with_one_colon_in_french() {
        use crate::street::model::StopKind;
        let mut e = zoned_editor();
        e.add_stop(crate::shared::catalogue::Side::Right, 10_000, 12_000, StopKind::Island).unwrap();
        let checks = e.view().checks;
        let failing: Vec<_> = checks.iter().filter(|c| c.id.starts_with("stop-") && !c.ok).collect();
        assert!(!failing.is_empty());
        for c in &failing {
            let row = say_now(&fr(), Units::Metres, &check_label(c));
            assert_eq!(row.matches(':').count(), 1, "{row}");
        }
        let kind = failing.iter().find(|c| c.id == "stop-kind").unwrap();
        assert_eq!(
            say_now(&fr(), Units::Metres, &check_label(kind)),
            "Arrêts aménagés comme leur type l’exige\u{a0}: îlot d’embarquement à 10,0\u{a0}m, à droite"
        );
    }

    #[test]
    fn a_failing_check_names_its_stretch_only_when_the_street_changes_along_its_length() {
        let mut e = zoned_editor();
        assert!(e.select_zone(1));
        let uid = e.view().segments[0].uid;
        assert!(e.set_width(uid, e.view().segments[0].width_mm + 500));
        let v = e.view();
        let fits = v.checks.iter().find(|c| c.id == "fits").unwrap();
        assert!(!fits.ok);
        let plain = say_now(&en(), Units::Metres, &fits.label);
        assert_eq!(say_now(&en(), Units::Metres, &check_label(fits)), format!("{plain} from 10.0 m to 30.0 m"));
        let plain = say_now(&fr(), Units::Metres, &fits.label);
        assert_eq!(say_now(&fr(), Units::Metres, &check_label(fits)), format!("{plain}, de 10,0\u{a0}m à 30,0\u{a0}m"));
        let passing = v.checks.iter().find(|c| c.ok).unwrap();
        assert_eq!(check_label(passing), passing.label);
        let single = Editor::new(0).view();
        assert!(single.checks.iter().all(|c| check_label(c) == c.label));
    }

    #[test]
    fn a_failing_curbside_row_names_its_use_not_a_zone() {
        use crate::street::model::Check;
        let row = Check {
            id: "curb-length",
            ok: false,
            label: Said::new("check-label-curb-length"),
            detail: Said::new("check-detail-curb-length-ok"),
            zone: Some(
                Said::new("curb-instance").with("use", Arg::Msg("curb-use-loading")).with("at", Arg::Length(42_000)).with("side", Arg::Msg("side-word-right")),
            ),
            standard: None,
        };
        assert_eq!(say_now(&en(), Units::Metres, &check_label(&row)), "Curbside uses are long enough: Loading at 42.0 m, right");
        assert_eq!(say_now(&fr(), Units::Metres, &check_label(&row)), "Usages du bord de rue assez longs\u{a0}: Zone de chargement à 42,0\u{a0}m, à droite");
    }

    #[test]
    fn a_failing_green_row_names_its_element_in_both_languages() {
        use crate::street::model::Check;
        let row = Check {
            id: "green-soil",
            ok: false,
            label: Said::new("check-label-green-soil"),
            detail: Said::new("check-detail-green-soil-ok"),
            zone: Some(
                Said::new("green-instance")
                    .with("kind", Arg::Msg("green-kind-tree"))
                    .with("at", Arg::Length(42_000))
                    .with("across", Arg::KindLower("sidewalk")),
            ),
            standard: None,
        };
        assert_eq!(say_now(&en(), Units::Metres, &check_label(&row)), "Tree pits with enough soil: Tree at 42.0 m, on sidewalk");
        let fr_row = say_now(&fr(), Units::Metres, &check_label(&row));
        assert!(fr_row.starts_with("Fosses d’arbre avec assez de terre\u{a0}: Arbre à 42,0\u{a0}m, sur l’élément "), "{fr_row}");
        let ok = Check { ok: true, zone: None, ..row };
        assert_eq!(check_label(&ok), ok.label);
    }

    #[test]
    fn a_failing_stop_row_names_its_stop_in_both_languages() {
        use crate::shared::catalogue::Side;
        use crate::street::model::{Piece, StopKind, Street, ZoneEnd, ZoneFrame};
        let piece = |id: &str, direction| Piece { kind: crate::shared::catalogue::kind_index(id).unwrap(), width_mm: 3000, direction, variants: Vec::new() };
        let s = Street::imported(
            crate::shared::catalogue::StreetClass::Local,
            Side::Right,
            &[piece("sidewalk", None), piece("travel", Some(1)), piece("travel", Some(1)), piece("sidewalk", None)],
        );
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
        e.add_stop(Side::Right, 42_000, 12_000, StopKind::Curb).unwrap();
        let v = e.view();
        let row = v.checks.iter().find(|c| c.id == "stop-lane").unwrap();
        assert!(!row.ok);
        assert_eq!(say_now(&en(), Units::Metres, &check_label(row)), "Stops have a lane for the bus: curb stop at 42.0 m, right");
        assert_eq!(say_now(&fr(), Units::Metres, &check_label(row)), "Voie pour l’autobus aux arrêts\u{a0}: arrêt au trottoir à 42,0\u{a0}m, à droite");
        e.name_stop(0, Some("Mont-Royal")).unwrap();
        let v = e.view();
        let row = v.checks.iter().find(|c| c.id == "stop-lane").unwrap();
        assert_eq!(say_now(&en(), Units::Metres, &check_label(row)), "Stops have a lane for the bus: Mont-Royal (curb stop, 42.0 m)");
    }

    /// Every key the model can say is a message in both languages. A new constructor in the model must use these literal forms, or this test does not see its key. The model builds a `Said` only with `Said::new("…")`, and puts a word in another's place only with `Arg::Msg("…")`, so its source lists them all.
    #[test]
    fn every_message_the_model_says_exists_in_english_and_in_french() {
        let source = include_str!("../model/mod.rs");
        let source = &source[..source.find("#[cfg(test)]\nmod tests").unwrap()];
        let mut keys = Vec::new();
        for opener in ["Said::new(\"", "Arg::Msg(\""] {
            for part in source.split(opener).skip(1) {
                keys.push(part[..part.find('"').unwrap()].to_string());
            }
        }
        keys.sort();
        keys.dedup();
        assert!(keys.len() >= 20, "{keys:?}");
        for locale in Locale::ALL {
            let i18n = crate::i18n_for(locale);
            // Every argument any of them takes, so that a message is found whatever it is about.
            let args = [
                "kind",
                "a",
                "b",
                "material",
                "curb",
                "vehicle",
                "direction",
                "base",
                "from",
                "to",
                "code",
                "name",
                "amount",
                "side",
                "class",
                "kmh",
                "need",
                "stretch",
                "at",
                "room",
            ]
            .into_iter()
            .fold(Args::new(), |args, name| args.str(name, "x"));
            for key in &keys {
                i18n.tr_now(key, &args);
            }
            assert_eq!(i18n.missing(), Vec::<String>::new(), "{locale:?}");
        }
    }

    #[test]
    fn on_a_street_anchored_at_its_last_end_a_curbside_use_is_said_from_the_plan_s_left_end() {
        use crate::shared::catalogue::Side;
        use crate::street::model::{Street, Use, ZoneEnd, ZoneFrame};
        // 100 m anchored at its last end: a use from 10 m to 16 m from the anchor is from 84 m to 90 m on the plan.
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 100_000, from_end: ZoneEnd::Last }));
        e.add_curb(Side::Right, 10_000, 6_000, Use::Loading).unwrap();
        let v = e.view();
        let row = v.checks.iter().find(|c| c.id == "curb-length").unwrap();
        assert!(
            say_now(&en(), Units::Metres, &check_label(row)).ends_with(": Loading at 84.0 m, right"),
            "{:?}",
            say_now(&en(), Units::Metres, &check_label(row))
        );
        assert_eq!(say_now(&en(), Units::Metres, &v.revisions.last().unwrap().label), "Add loading at 84.0 m");
        let side = v.curb.sides.iter().find(|s| s.cells.iter().any(|c| c.instance.is_some())).unwrap();
        let cell = side.cells.iter().position(|c| c.instance.is_some()).unwrap();
        let said = cell_text("curb-cell", side, cell, v.axis, &en(), Units::Metres);
        assert!(said.contains(", 84.0 m to 90.0 m, 6.0 m long,"), "{said}");
    }

    /// Every street message, in both languages, says "stretch" where it said "zone": only the places that are zones
    /// by name keep the word. Each message is read in every variant of its selectors, then formatted with sample
    /// values; the ids that still say "zone" are listed.
    #[test]
    fn no_street_message_says_zone_except_school_and_loading_zones() {
        use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};
        use std::collections::BTreeSet;
        // "school zone" and "loading zone" are zones by name, as is "zone scolaire". French says "zone de chargement"
        // for the loading zone (the glossary's term), "zone tampon" for a buffer (a piece across the street, not a part
        // along it) and "zones à circulation restreinte" (the name of a kind of area).
        const ALLOWED: [&str; 6] = ["school zone", "loading zone", "zone scolaire", "zone de chargement", "zone tampon", "zones à circulation restreinte"];
        fn says_zone(text: &str) -> bool {
            let mut text = text.to_lowercase();
            for phrase in ALLOWED {
                text = text.replace(phrase, "");
            }
            text.contains("zone")
        }
        fn words(p: &Pattern<&str>, text: &mut String, vars: &mut BTreeSet<&'static str>) {
            for el in &p.elements {
                match el {
                    PatternElement::TextElement { value } => text.push_str(value),
                    PatternElement::Placeable { expression } => expression_words(expression, text, vars),
                }
            }
        }
        fn inline_words(e: &InlineExpression<&str>, text: &mut String, vars: &mut BTreeSet<&'static str>) {
            match e {
                InlineExpression::StringLiteral { value } => text.push_str(value),
                InlineExpression::VariableReference { id } => {
                    vars.insert(String::leak(id.name.to_string()));
                }
                InlineExpression::Placeable { expression } => expression_words(expression, text, vars),
                _ => {}
            }
        }
        fn expression_words(e: &Expression<&str>, text: &mut String, vars: &mut BTreeSet<&'static str>) {
            match e {
                Expression::Inline(i) => inline_words(i, text, vars),
                Expression::Select { selector, variants } => {
                    inline_words(selector, text, vars);
                    for v in variants {
                        text.push(' ');
                        words(&v.value, text, vars);
                    }
                }
            }
        }
        let mut offending = Vec::new();
        for (locale, source) in [(Locale::En, crate::street::RESOURCES.en), (Locale::FrCa, crate::street::RESOURCES.fr)] {
            let i18n = crate::i18n_for(locale);
            let resource = fluent_syntax::parser::parse(source).expect("the street's messages parse");
            for entry in &resource.body {
                let Entry::Message(m) = entry else { continue };
                let Some(pattern) = &m.value else { continue };
                let (mut text, mut vars) = (String::new(), BTreeSet::new());
                words(pattern, &mut text, &mut vars);
                let formatted = [1, 2, 5].map(|n| i18n.tr_now(m.id.name, &vars.iter().fold(Args::new(), |args, v| args.num(v, n))));
                if says_zone(&text) || formatted.iter().any(|f| says_zone(f)) {
                    offending.push(format!("{}: {}", locale.tag(), m.id.name));
                }
            }
        }
        assert_eq!(offending, Vec::<String>::new());
    }
}
