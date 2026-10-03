//! The street model, shared between the page's Leptos components and the
//! script. Edits made from a component are announced as they happen.

use std::rc::Rc;

use leptos::prelude::*;

use crate::model::Editor;
use crate::ui::core::Core;
use crate::ui::{live, street_text};

pub struct SharedSheet {
    core: Core<Editor>,
}

impl std::ops::Deref for SharedSheet {
    type Target = Core<Editor>;
    fn deref(&self) -> &Core<Editor> {
        &self.core
    }
}

impl SharedSheet {
    pub fn new(model: Editor) -> Rc<SharedSheet> {
        Rc::new(SharedSheet { core: Core::new(model) })
    }

    /// An edit made from a component of the page: announced by what it was,
    /// and by how the pieces stand. A refused edit says nothing.
    pub fn edit_in_page(&self, f: impl FnOnce(&mut Editor) -> bool) -> bool {
        let done = self.edit(f);
        if done {
            self.say_edit();
        }
        done
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet_in_page<R>(&self, f: impl FnOnce(&mut Editor) -> R) -> R {
        self.edit(f)
    }

    /// A selection made from a component, announced by what is selected.
    pub fn select_in_page(&self, uid: Option<u32>) {
        self.edit(|e| e.select(uid));
        self.say_selection();
    }

    pub fn select_relative_in_page(&self, delta: i32) {
        self.edit(|e| e.select_relative(delta));
        self.say_selection();
    }

    /// The end of a gesture: kept when `commit`, announced as an edit if it
    /// changed anything, or else by what is selected.
    pub fn end_gesture_in_page(&self, commit: bool) {
        if commit {
            if self.edit(|e| e.end_gesture()) {
                self.say_edit();
            } else {
                self.say_selection();
            }
        } else {
            self.edit(|e| e.cancel_gesture());
        }
    }

    pub fn undo_in_page(&self) -> bool {
        let done = self.edit(|e| e.undo());
        if done {
            live::say(&street_text::undone_text(&self.view(), self.units().get_untracked()));
        }
        done
    }

    pub fn redo_in_page(&self) -> bool {
        let done = self.edit(|e| e.redo());
        if done {
            live::say(&street_text::redone_text(&self.view(), self.units().get_untracked()));
        }
        done
    }

    pub fn reset_in_page(&self) -> bool {
        let done = self.edit(|e| e.reset());
        if done {
            live::say(street_text::STARTED_OVER);
        }
        done
    }

    /// Arranges the roadway as an Atlas measure, or says it does not suit.
    pub fn apply_measure_in_page(&self, code: &str) -> bool {
        let done = self.edit(|e| e.apply_measure(code));
        if done {
            self.say_edit();
        } else {
            live::say(street_text::MEASURE_REFUSED);
        }
        done
    }

    fn say_edit(&self) {
        if let Some(text) = street_text::edit_text(&self.view(), self.units().get_untracked()) {
            live::say(&text);
        }
    }

    fn say_selection(&self) {
        if let Some(text) = street_text::selection_text(&self.view(), self.units().get_untracked()) {
            live::say(&text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Units;

    fn said() -> Vec<String> {
        live::take_said()
    }

    fn first(s: &SharedSheet) -> u32 {
        s.view().segments[0].uid
    }

    #[test]
    fn an_edit_from_the_page_is_announced_and_a_refused_one_is_not() {
        let s = SharedSheet::new(Editor::new(0));
        let uid = first(&s);
        said();
        assert!(s.edit_in_page(|e| e.remove(uid)));
        assert!(said()[0].ends_with(". 3.3 m left to use."));
        assert!(!s.edit_in_page(|e| e.remove(uid)));
        assert!(said().is_empty());
    }

    #[test]
    fn a_quiet_change_says_nothing_but_still_counts_as_an_edit() {
        let s = SharedSheet::new(Editor::new(0));
        let uid = first(&s);
        said();
        assert!(s.quiet_in_page(|e| e.nudge_width(uid, 100)));
        assert!(said().is_empty());
        assert_eq!(s.version().get_untracked(), 1);
    }

    #[test]
    fn selecting_announces_the_piece_and_clearing_says_nothing() {
        let s = SharedSheet::new(Editor::new(0));
        let uid = first(&s);
        said();
        s.select_in_page(Some(uid));
        assert_eq!(said(), vec!["Sidewalk, 3.3 m, 1 of 6"]);
        s.select_in_page(None);
        assert!(said().is_empty());
        s.select_relative_in_page(1);
        assert_eq!(said().len(), 1);
    }

    #[test]
    fn a_gesture_is_one_edit_when_it_ends_or_the_selection_when_it_changed_nothing() {
        let s = SharedSheet::new(Editor::new(0));
        s.select_in_page(Some(first(&s)));
        said();
        s.quiet_in_page(|e| e.begin_gesture());
        assert!(s.quiet_in_page(|e| e.resize_boundary(0, 200)));
        assert_eq!(s.view().revisions.len(), 0);
        s.end_gesture_in_page(true);
        assert_eq!(s.view().revisions.len(), 1);
        assert!(said()[0].contains(". "));
        s.quiet_in_page(|e| e.begin_gesture());
        s.end_gesture_in_page(true);
        assert_eq!(said().len(), 1, "the selection is announced");
        s.quiet_in_page(|e| e.begin_gesture());
        s.quiet_in_page(|e| e.resize_boundary(0, 300));
        s.end_gesture_in_page(false);
        assert_eq!(s.view().revisions.len(), 1);
    }

    #[test]
    fn undo_redo_and_start_over_are_announced() {
        let s = SharedSheet::new(Editor::new(0));
        let uid = first(&s);
        s.edit(|e| e.remove(uid));
        said();
        assert!(s.undo_in_page());
        assert_eq!(said(), vec!["Undone. Every metre of the street is used."]);
        assert!(s.redo_in_page());
        assert_eq!(said(), vec!["Redone. 3.3 m left to use."]);
        assert!(s.reset_in_page());
        assert_eq!(said(), vec!["Started over from the street as it is today. Undo brings your changes back."]);
        // Starting over can itself be undone, which brings the changes back.
        assert!(s.undo_in_page());
        assert_eq!(said(), vec!["Undone. 3.3 m left to use."]);
    }

    #[test]
    fn a_measure_is_announced_when_it_is_arranged_and_refused_in_words_when_it_is_not() {
        let s = SharedSheet::new(Editor::new(1));
        said();
        assert!(s.apply_measure_in_page("B1"));
        assert_eq!(said().len(), 1);
        assert!(!s.apply_measure_in_page("B2"));
        assert_eq!(said(), vec!["That measure does not suit this street."]);
    }

    #[test]
    fn what_is_announced_follows_the_units() {
        let s = SharedSheet::new(Editor::new(0));
        s.set_units(Units::Feet);
        let uid = first(&s);
        said();
        s.edit_in_page(|e| e.remove(uid));
        assert!(said()[0].ends_with(". 10.8 ft left to use."));
    }
}
