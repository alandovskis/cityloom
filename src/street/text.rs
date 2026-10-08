//! What the street page says in words: the line that says whether the pieces fit
//! the street, and what a screen reader is told after an edit or a selection.
//!
//! Every sentence is a message (`src/street/i18n`); these functions only choose
//! which one and hand it the lengths, written in the language of the page.

use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::units::Units;
use crate::street::model::View;

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

/// What the last edit was, and how the pieces stand. The label is the model's.
pub fn edit_text(v: &View, i18n: &I18n, units: Units) -> Option<String> {
    v.revisions.last().map(|r| i18n.tr_now("edit-done", &Args::new().str("label", r.label.clone()).str("fit", fit_phrase(v, i18n, units))))
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
        assert!(said.starts_with(&e.view().revisions.last().unwrap().label));
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
            format!("{}, {} m, 2 of {}", KINDS[s.kind].name, Units::Metres.fine(s.width_mm), v.segments.len())
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
}
