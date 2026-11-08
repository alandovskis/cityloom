//! A popover of the header (the settings, the help): whether it is open, and the words of its button and
//! heading, in the language of the page. The view only shows what this says.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};

pub struct PopoverViewModel {
    i18n: Rc<I18n>,
    label: &'static str,
    heading: &'static str,
    open: ArcRwSignal<bool>,
    /// The `open` of the popovers that close when this one opens.
    others: RefCell<Vec<ArcRwSignal<bool>>>,
}

impl PopoverViewModel {
    /// A popover whose button says the message `label` and whose heading says `heading`.
    pub fn new(i18n: Rc<I18n>, label: &'static str, heading: &'static str) -> Rc<PopoverViewModel> {
        Rc::new(PopoverViewModel { i18n, label, heading, open: ArcRwSignal::new(false), others: RefCell::new(Vec::new()) })
    }

    pub fn open(&self) -> bool {
        self.open.get()
    }

    pub fn is_open_now(&self) -> bool {
        self.open.get_untracked()
    }

    /// Opens it, closing the popovers it is exclusive with, or closes it.
    pub fn toggle(&self) {
        if !self.is_open_now() {
            self.others.borrow().iter().for_each(|o| o.set(false));
        }
        self.open.update(|o| *o = !*o);
    }

    /// Makes `popovers` open one at a time: opening any of them closes the others.
    pub fn exclusive(popovers: &[&Rc<PopoverViewModel>]) {
        for p in popovers {
            *p.others.borrow_mut() = popovers.iter().filter(|q| !Rc::ptr_eq(p, q)).map(|q| q.open.clone()).collect();
        }
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
    fn popovers_made_exclusive_are_never_open_together_whichever_is_toggled() {
        let (a, b, c) = (help(Locale::En), help(Locale::En), help(Locale::En));
        PopoverViewModel::exclusive(&[&a, &b, &c]);
        a.toggle();
        b.toggle();
        assert!((a.is_open_now(), b.is_open_now(), c.is_open_now()) == (false, true, false), "opening b closed a");
        c.toggle();
        assert!((a.is_open_now(), b.is_open_now(), c.is_open_now()) == (false, false, true), "opening c closed b");
        c.toggle();
        assert!(!c.is_open_now() && !a.is_open_now() && !b.is_open_now(), "closing one opens none");
        let alone = help(Locale::En);
        alone.toggle();
        b.toggle();
        assert!(alone.is_open_now() && b.is_open_now(), "a popover outside the group is left alone");
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
