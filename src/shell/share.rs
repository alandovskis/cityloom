//! The Share button of the header: a menu of the ways to take the page out of the app (for now, to print it).

use std::rc::Rc;

use super::popover::PopoverViewModel;
use crate::shared::i18n::I18n;

/// The Share menu: whether it is open, and the words of its button and heading, in the language of the page.
pub struct ShareViewModel {
    popover: Rc<PopoverViewModel>,
}

impl ShareViewModel {
    pub fn new(i18n: Rc<I18n>) -> Rc<ShareViewModel> {
        Rc::new(ShareViewModel { popover: PopoverViewModel::new(i18n, "ui-share", "ui-share") })
    }

    /// The popover the view binds its button and menu to.
    pub fn popover(&self) -> Rc<PopoverViewModel> {
        self.popover.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;

    #[test]
    fn it_is_closed_at_first_and_its_button_and_heading_follow_the_language() {
        let s = ShareViewModel::new(crate::i18n_for(Locale::En));
        assert!(!s.popover().is_open_now());
        assert_eq!((s.popover().label().as_str(), s.popover().heading().as_str()), ("Share", "Share"));
        let s = ShareViewModel::new(crate::i18n_for(Locale::FrCa));
        assert_eq!(s.popover().label(), "Partager");
    }
}
