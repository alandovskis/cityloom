//! What the street page says in words: the line that says whether the pieces fit
//! the street, and what a screen reader is told after an edit or a selection.

use crate::shared::catalogue::KINDS;
use crate::model::View;
use crate::shared::units::Units;

pub const STARTED_OVER: &str = "Started over from the street as it is today. Undo brings your changes back.";
pub const MEASURE_REFUSED: &str = "That measure does not suit this street.";

/// How the pieces stand against the street's width, as a phrase with no full stop.
pub fn fit_phrase(v: &View, units: Units) -> String {
    match v.delta_mm {
        0 => "Every metre of the street is used".to_string(),
        d if d < 0 => format!("{} left to use", units.length_fine(-d)),
        d => format!("{} too wide. Make a piece narrower or remove one", units.length_fine(d)),
    }
}

/// The status line under the drawing.
pub fn status_text(v: &View, units: Units) -> String {
    match v.delta_mm {
        0 => "Every metre of the street is used.".to_string(),
        d if d < 0 => format!("{} of the street is still unused.", units.length_fine(-d)),
        d => format!("{} too wide. Make a piece narrower or remove one.", units.length_fine(d)),
    }
}

/// What the last edit was, and how the pieces stand.
pub fn edit_text(v: &View, units: Units) -> Option<String> {
    v.revisions.last().map(|r| format!("{}. {}.", r.label, fit_phrase(v, units)))
}

pub fn undone_text(v: &View, units: Units) -> String {
    format!("Undone. {}.", fit_phrase(v, units))
}

pub fn redone_text(v: &View, units: Units) -> String {
    format!("Redone. {}.", fit_phrase(v, units))
}

/// The selected piece, its width and its place among the pieces; nothing when
/// nothing is selected.
pub fn selection_text(v: &View, units: Units) -> Option<String> {
    let uid = v.selected?;
    let i = v.segments.iter().position(|s| s.uid == uid)?;
    let s = &v.segments[i];
    Some(format!("{}, {}, {} of {}", KINDS[s.kind].name, units.length_fine(s.width_mm), i + 1, v.segments.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Editor;

    fn uid(e: &Editor, i: usize) -> u32 {
        e.view().segments[i].uid
    }

    #[test]
    fn a_street_that_is_exactly_full_says_every_metre_is_used() {
        let v = Editor::new(0).view();
        assert_eq!(v.delta_mm, 0);
        assert_eq!(fit_phrase(&v, Units::Metres), "Every metre of the street is used");
        assert_eq!(status_text(&v, Units::Metres), "Every metre of the street is used.");
    }

    #[test]
    fn a_street_with_room_left_says_how_much() {
        let mut e = Editor::new(0);
        let first = uid(&e, 0);
        assert!(e.remove(first));
        let v = e.view();
        assert_eq!(fit_phrase(&v, Units::Metres), "3.3 m left to use");
        assert_eq!(status_text(&v, Units::Metres), "3.3 m of the street is still unused.");
        assert_eq!(fit_phrase(&v, Units::Feet), "10.8 ft left to use");
    }

    #[test]
    fn a_street_that_is_too_full_says_how_much_too_wide() {
        let mut e = Editor::new(0);
        let first = uid(&e, 0);
        assert!(e.set_width(first, e.view().segments[0].width_mm + 500));
        let v = e.view();
        assert!(v.delta_mm > 0);
        assert_eq!(fit_phrase(&v, Units::Metres), "0.5 m too wide. Make a piece narrower or remove one");
        assert_eq!(status_text(&v, Units::Metres), "0.5 m too wide. Make a piece narrower or remove one.");
    }

    #[test]
    fn an_edit_is_announced_by_its_label_and_how_the_pieces_stand() {
        let mut e = Editor::new(0);
        assert_eq!(edit_text(&e.view(), Units::Metres), None);
        let first = uid(&e, 0);
        e.remove(first);
        let said = edit_text(&e.view(), Units::Metres).unwrap();
        assert!(said.ends_with(". 3.3 m left to use."), "{said}");
        assert!(said.starts_with(&e.view().revisions.last().unwrap().label));
    }

    #[test]
    fn undo_and_redo_are_announced_with_how_the_pieces_stand() {
        let v = Editor::new(0).view();
        assert_eq!(undone_text(&v, Units::Metres), "Undone. Every metre of the street is used.");
        assert_eq!(redone_text(&v, Units::Metres), "Redone. Every metre of the street is used.");
    }

    #[test]
    fn the_selected_piece_is_announced_with_its_width_and_place() {
        let mut e = Editor::new(0);
        assert_eq!(selection_text(&e.view(), Units::Metres), None);
        let second = uid(&e, 1);
        e.select(Some(second));
        let v = e.view();
        let s = &v.segments[1];
        assert_eq!(selection_text(&v, Units::Metres).unwrap(), format!("{}, {} m, 2 of {}", KINDS[s.kind].name, Units::Metres.fine(s.width_mm), v.segments.len()));
    }
}
