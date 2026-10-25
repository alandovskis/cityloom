//! What the street page says in words: the line that says whether the pieces fit
//! the street, and what a screen reader is told after an edit or a selection.
//!
//! Every sentence is a message (`src/street/i18n`); these functions only choose
//! which one and hand it the lengths, written in the language of the page.

use crate::shared::catalogue::{KINDS, StreetClass, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::{Arg, Said};
use crate::shared::units::Units;
use crate::street::model::{Check, SideView, StripCell, View, ZoneRefusal};

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
    let s = &v.segments[i];
    let args = Args::new()
        .str("kind", i18n.tr_now(&kind_key(KINDS[s.kind].id), &Args::new()))
        .str("width", units.length_fine_in(s.width_mm, i18n.locale_now()))
        .num("index", i as i64 + 1)
        .num("total", v.segments.len() as i64);
    Some(i18n.tr_now("selection-position", &args))
}

/// The selected zone: its name, where it lies, how long it is, whether it has a section of its own and its place
/// among the zones. Said whenever a zone is chosen.
pub fn zone_selection_text(v: &View, i18n: &I18n, units: Units) -> String {
    zone_text(v, v.zones.selected, i18n, units)
}

/// The same for zone `index` (the last when past the end), which is how a cell of the strip is named.
pub fn zone_text(v: &View, index: usize, i18n: &I18n, units: Units) -> String {
    let locale = i18n.locale_now();
    let zone = &v.zones.zones[index.min(v.zones.zones.len().saturating_sub(1))];
    let args = Args::new()
        .str("zone", say_now(i18n, units, &zone.label))
        .str("from", units.length_in(zone.from_mm, locale))
        .str("to", units.length_in(zone.to_mm, locale))
        .str("length", units.length_in(zone.to_mm - zone.from_mm, locale))
        .str("section", if zone.own { "own" } else { "base" })
        .num("n", zone.index as i64 + 1)
        .num("total", v.zones.zones.len() as i64);
    i18n.tr_now("zone-selected", &args)
}

/// How a cell of a strip is named (by the message `key`): what it is, where it lies and how long, and its place on
/// the strip.
pub fn cell_text<C: StripCell>(key: &str, side: &SideView<C>, cell: usize, i18n: &I18n, units: Units) -> String {
    let locale = i18n.locale_now();
    let Some(c) = side.cells.get(cell) else { return String::new() };
    let args = Args::new()
        .str("name", say_now(i18n, units, c.label()))
        .str("from", units.length_in(c.from_mm(), locale))
        .str("to", units.length_in(c.to_mm(), locale))
        .str("length", units.length_in(c.to_mm() - c.from_mm(), locale))
        .num("n", cell as i64 + 1)
        .num("total", side.cells.len() as i64);
    i18n.tr_now(key, &args)
}

/// Why a zone operation was refused, in the page's language.
pub fn zone_refused(i18n: &I18n, units: Units, refusal: ZoneRefusal) -> String {
    say_now(i18n, units, &refusal.said())
}

/// What a check is called where it is listed: a failing check of a street with several zones says which zone
/// it fails in.
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
        (Some(zone), false) => {
            Said::new("check-zone-in").with("check", Arg::Said(Box::new(check.label.clone()))).with("zone", Arg::Said(Box::new(zone.clone())))
        }
        _ => check.label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::street::model::Editor;

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
        let mut e = zoned_editor();
        assert!(e.select_zone(1));
        let v = e.view();
        assert_eq!(zone_selection_text(&v, &en(), Units::Metres), "Zone 2, 10.0 m to 30.0 m, 20.0 m long, the base section, 2 of 3");
        assert_eq!(zone_selection_text(&v, &fr(), Units::Metres), "Zone 2, de 10,0\u{a0}m à 30,0\u{a0}m, 20,0\u{a0}m de long, la coupe de base, 2 sur 3");
        assert_eq!(zone_refused(&en(), Units::Metres, ZoneRefusal::TooShort), "A zone must be at least 5.0 m long.");
        assert_eq!(zone_refused(&fr(), Units::Metres, ZoneRefusal::TooShort), "Une zone doit mesurer au moins 5,0\u{a0}m.");
        assert_eq!(zone_refused(&fr(), Units::Feet, ZoneRefusal::NoNeighbour), "Il n’y a pas de zone de ce côté avec laquelle fusionner.");
        assert!(e.name_zone(Some("Market")));
        let v = e.view();
        assert_eq!(zone_selection_text(&v, &en(), Units::Metres), "Market, 10.0 m to 30.0 m, 20.0 m long, the base section, 2 of 3");
    }

    #[test]
    fn a_merge_is_said_in_both_languages_by_the_zone_whose_section_it_kept() {
        use crate::street::model::Keep;
        let mut e = zoned_editor();
        let uid = e.view().segments[0].uid;
        assert!(e.select_zone(1));
        assert!(e.set_width(uid, e.view().segments[0].width_mm + 100));
        assert!(e.select_zone(0));
        assert!(e.merge_zone(true, Keep::Neighbour).is_ok());
        let label = e.view().revisions.last().unwrap().label.clone();
        assert_eq!(say_now(&en(), Units::Metres, &label), "Merge zones, keeping Zone 2");
        assert_eq!(say_now(&fr(), Units::Metres, &label), "Fusion des zones; coupe gardée\u{a0}: zone 2");
    }

    #[test]
    fn a_failing_check_names_its_zone_only_when_the_street_has_several() {
        let mut e = zoned_editor();
        assert!(e.select_zone(1));
        let uid = e.view().segments[0].uid;
        assert!(e.set_width(uid, e.view().segments[0].width_mm + 500));
        let v = e.view();
        let fits = v.checks.iter().find(|c| c.id == "fits").unwrap();
        assert!(!fits.ok);
        let plain = say_now(&en(), Units::Metres, &fits.label);
        assert_eq!(say_now(&en(), Units::Metres, &check_label(fits)), format!("{plain} in Zone 2"));
        assert_eq!(say_now(&fr(), Units::Metres, &check_label(fits)), format!("{}\u{a0}: Zone 2", say_now(&fr(), Units::Metres, &fits.label)));
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
        };
        assert_eq!(say_now(&en(), Units::Metres, &check_label(&row)), "Curbside uses are long enough: Loading at 42.0 m, right");
        assert_eq!(say_now(&fr(), Units::Metres, &check_label(&row)), "Usages du bord de rue assez longs\u{a0}: Zone de chargement à 42,0\u{a0}m, à droite");
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
        assert_eq!(say_now(&fr(), Units::Metres, &check_label(row)), "Arrêts\u{a0}: une voie pour l’autobus\u{a0}: arrêt au trottoir à 42,0\u{a0}m, à droite");
        e.name_stop(0, Some("Mont-Royal")).unwrap();
        let v = e.view();
        let row = v.checks.iter().find(|c| c.id == "stop-lane").unwrap();
        assert_eq!(say_now(&en(), Units::Metres, &check_label(row)), "Stops have a lane for the bus: Mont-Royal (curb stop, 42.0 m)");
    }

    /// Every key the model can say is a message in both languages. A new constructor in the model must use these literal forms, or this test does not see its key. The model builds a `Said` only with `Said::new("…")`, and puts a word in another's place only with `Arg::Msg("…")`, so its source lists them all.
    #[test]
    fn every_message_the_model_says_exists_in_english_and_in_french() {
        let source = include_str!("model.rs");
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
                "zone",
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
}
