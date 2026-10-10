//! A popover of the header (the settings, the help): whether it is open, and the words of its button and
//! heading, in the language of the page. The view only shows what this says.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};

pub struct PopoverViewModel {
    i18n: Rc<I18n>,
    label: &'static str,
    heading: &'static str,
    open: ArcRwSignal<bool>,
}

impl PopoverViewModel {
    /// A popover whose button says the message `label` and whose heading says `heading`.
    pub fn new(i18n: Rc<I18n>, label: &'static str, heading: &'static str) -> Rc<PopoverViewModel> {
        Rc::new(PopoverViewModel { i18n, label, heading, open: ArcRwSignal::new(false) })
    }

    pub fn open(&self) -> bool {
        self.open.get()
    }

    pub fn is_open_now(&self) -> bool {
        self.open.get_untracked()
    }

    pub fn toggle(&self) {
        self.open.update(|o| *o = !*o);
    }

    pub fn close(&self) {
        self.open.set(false);
    }

    /// What its button says; it follows a switch of language.
    pub fn label(&self) -> String {
        self.i18n.tr(self.label, &Args::new())
    }

    /// What its heading says; it follows a switch of language.
    pub fn heading(&self) -> String {
        self.i18n.tr(self.heading, &Args::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;

    fn help(locale: Locale) -> Rc<PopoverViewModel> {
        PopoverViewModel::new(crate::i18n_for(locale), "ui-help", "ui-keyboard")
    }

    #[test]
    fn it_is_closed_at_first_and_opens_and_closes() {
        let p = help(Locale::En);
        assert!(!p.is_open_now());
        p.toggle();
        assert!(p.is_open_now());
        p.toggle();
        assert!(!p.is_open_now());
        p.toggle();
        p.close();
        assert!(!p.is_open_now());
    }

    #[test]
    fn its_button_and_heading_are_worded_in_the_language_of_the_page() {
        let p = help(Locale::En);
        assert_eq!((p.label().as_str(), p.heading().as_str()), ("Help", "Keyboard"));
        let p = help(Locale::FrCa);
        assert_eq!((p.label().as_str(), p.heading().as_str()), ("Aide", "Clavier"));
    }

    #[test]
    fn it_follows_a_switch_of_language() {
        let i18n = crate::i18n_for(Locale::En);
        let p = PopoverViewModel::new(i18n.clone(), "ui-settings", "ui-settings");
        assert_eq!(p.label(), "Settings");
        i18n.set(Locale::FrCa);
        assert_eq!(p.label(), "Réglages");
    }
}
