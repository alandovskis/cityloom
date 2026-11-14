//! The streets selected on the map, in the order they were added: shift-click on the map or Shift+Enter on a
//! search result adds or drops one, the Selected streets panel lists them. A selection is only a list for now;
//! nothing is changed by it. It lives for the page (it is not kept).

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;

pub struct SelectionVm {
    ports: Ports,
    i18n: Rc<I18n>,
    streets: ArcRwSignal<Vec<u32>>,
}

impl SelectionVm {
    pub fn new(ports: Ports, i18n: Rc<I18n>) -> Rc<SelectionVm> {
        Rc::new(SelectionVm { ports, i18n, streets: ArcRwSignal::new(Vec::new()) })
    }

    /// The selected streets' uids, first added first. Tracked.
    pub fn streets(&self) -> Vec<u32> {
        self.streets.get()
    }

    /// The same, untracked.
    pub fn streets_now(&self) -> Vec<u32> {
        self.streets.get_untracked()
    }

    pub fn contains(&self, uid: u32) -> bool {
        self.streets.read_untracked().contains(&uid)
    }

    /// What the map is told: the places selected, as `s-<uid>`.
    pub fn hot(&self) -> Vec<String> {
        self.streets.read_untracked().iter().map(|u| format!("s-{u}")).collect()
    }

    /// Adds the street, or drops it when it is selected; says which, and how many are selected. `name` is how the
    /// street is called on the page. Returns whether the street is selected now.
    pub fn toggle(&self, uid: u32, name: &str) -> bool {
        if self.contains(uid) {
            self.remove(uid, name);
            false
        } else {
            self.streets.update(|s| s.push(uid));
            self.say("sel-added", name);
            true
        }
    }

    /// Drops the street from the selection; nothing when it is not in it.
    pub fn remove(&self, uid: u32, name: &str) {
        if !self.contains(uid) {
            return;
        }
        self.streets.update(|s| s.retain(|&u| u != uid));
        self.say("sel-removed", name);
    }

    /// Empties the selection, and says so when there was one.
    pub fn clear(&self) {
        if self.streets.read_untracked().is_empty() {
            return;
        }
        self.streets.set(Vec::new());
        self.ports.announcer.say(&self.i18n.tr_now("sel-cleared", &Args::new()));
    }

    fn say(&self, key: &str, name: &str) {
        let args = Args::new().str("name", name).num("count", self.streets.read_untracked().len() as i64);
        self.ports.announcer.say(&self.i18n.tr_now(key, &args));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::test_ports;

    fn selection(locale: Locale) -> (Rc<SelectionVm>, Rc<crate::shared::ports::RecordingAnnouncer>) {
        let (ports, said, _) = test_ports();
        (SelectionVm::new(ports, crate::i18n_for(locale)), said)
    }

    #[test]
    fn a_street_is_added_then_dropped_by_the_same_toggle_and_each_is_said_with_the_count() {
        let (s, said) = selection(Locale::En);
        assert!(s.toggle(4, "Rue Rachel"));
        assert!(s.toggle(9, "Avenue du Parc"));
        assert_eq!(s.streets(), vec![4, 9], "in the order they were added");
        assert_eq!(s.hot(), vec!["s-4", "s-9"]);
        assert!(!s.toggle(4, "Rue Rachel"));
        assert_eq!(s.streets(), vec![9]);
        assert_eq!(
            said.take(),
            vec![
                "Rue Rachel added to the selection. One street selected.",
                "Avenue du Parc added to the selection. 2 streets selected.",
                "Rue Rachel removed from the selection. One street selected.",
            ]
        );
    }

    #[test]
    fn dropping_the_last_street_says_none_is_left_and_dropping_one_that_is_not_there_does_nothing() {
        let (s, said) = selection(Locale::En);
        s.toggle(1, "Main");
        said.take();
        s.remove(2, "Elsewhere");
        assert_eq!(s.streets(), vec![1]);
        assert!(said.take().is_empty());
        s.remove(1, "Main");
        assert_eq!(said.take(), vec!["Main removed from the selection. No streets selected."]);
    }

    #[test]
    fn clearing_empties_it_and_says_so_only_when_there_was_a_selection() {
        let (s, said) = selection(Locale::En);
        s.clear();
        assert!(said.take().is_empty());
        s.toggle(1, "Main");
        s.toggle(2, "Side");
        said.take();
        s.clear();
        assert!(s.streets().is_empty() && s.hot().is_empty());
        assert_eq!(said.take(), vec!["Selection cleared."]);
    }

    #[test]
    fn it_speaks_french_too() {
        let (s, said) = selection(Locale::FrCa);
        s.toggle(1, "Rue Rachel");
        s.toggle(2, "Avenue du Parc");
        s.remove(1, "Rue Rachel");
        s.clear();
        assert_eq!(
            said.take(),
            vec![
                "Ajout à la sélection\u{a0}: Rue Rachel. Une rue sélectionnée.",
                "Ajout à la sélection\u{a0}: Avenue du Parc. 2 rues sélectionnées.",
                "Retrait de la sélection\u{a0}: Rue Rachel. Une rue sélectionnée.",
                "Sélection effacée.",
            ]
        );
    }
}
