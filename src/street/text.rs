//! What the street page says in words: the line that says whether the pieces fit
//! the street, and what a screen reader is told after an edit or a selection.
//!
//! Every sentence is a message (`src/street/i18n`); these functions only choose
//! which one and hand it the lengths, written in the language of the page.

use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::Said;
use crate::shared::units::Units;
use crate::street::model::View;

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
            let args = ["kind", "a", "b", "material", "curb", "vehicle", "direction", "base", "from", "to", "code", "name", "amount", "side", "class"]
                .into_iter()
                .fold(Args::new(), |args, name| args.str(name, "x"));
            for key in &keys {
                i18n.tr_now(key, &args);
            }
            assert_eq!(i18n.missing(), Vec::<String>::new(), "{locale:?}");
        }
    }
}
