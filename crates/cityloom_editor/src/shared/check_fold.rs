//! The passing checks of a Checks tab (the street's and the junction's), folded under the failing ones: shown only
//! once asked for, so the tab opens on what fails. Whether they are shown is the page's, not remembered.

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};

/// Whether the passing checks are shown; folded at first.
#[derive(Clone, Default)]
pub struct PassFold(ArcRwSignal<bool>);

impl PassFold {
    /// Whether the passing checks are shown, watched.
    pub fn shown(&self) -> bool {
        self.0.get()
    }

    /// Shows the passing checks, or folds them again.
    pub fn toggle(&self) {
        self.0.update(|shown| *shown = !*shown);
    }
}

/// Under the list (`#checks`): when every check passes, a line that says so; then the button that shows or folds the
/// passing ones. Nothing when none passes.
pub fn passing_toggle(fold: PassFold, passing: usize, all: usize, i18n: &I18n) -> impl IntoView + use<> {
    let shown = fold.shown();
    let label = if shown { i18n.tr("ui-checks-passing-hide", &Args::new()) } else { i18n.tr("ui-checks-passing", &Args::new().num("n", passing as i64)) };
    let every = (passing == all).then(|| view! { <p class="checks-all-pass">{i18n.tr("ui-checks-all-pass", &Args::new().num("n", all as i64))}</p> });
    let fold = StoredValue::new_local(fold);
    (passing > 0).then(|| {
        view! {
            {every}
            <button
                type="button"
                id="checks-passing"
                class="checks-passing"
                aria-controls="checks"
                aria-expanded=if shown { "true" } else { "false" }
                on:click=move |_| fold.with_value(PassFold::toggle)
            >
                {label}
            </button>
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::shared::testing::html;

    #[test]
    fn the_passing_checks_are_folded_at_first_and_shown_and_folded_again_by_a_press() {
        let fold = PassFold::default();
        assert!(!fold.shown());
        fold.toggle();
        assert!(fold.shown());
        fold.toggle();
        assert!(!fold.shown());
    }

    fn drawn(fold: &PassFold, passing: usize, all: usize, locale: Locale) -> String {
        let (fold, i18n) = (fold.clone(), crate::i18n_for(locale));
        html(move || passing_toggle(fold.clone(), passing, all, &i18n).into_any())
    }

    #[test]
    fn the_button_counts_the_passing_checks_and_says_whether_they_are_shown() {
        let fold = PassFold::default();
        let h = drawn(&fold, 9, 11, Locale::En);
        assert!(h.contains(r#"id="checks-passing""#) && h.contains(r#"aria-controls="checks""#) && h.contains(r#"aria-expanded="false""#), "{h}");
        assert!(h.contains(">Show the 9 passing checks<") && !h.contains("checks-all-pass"), "{h}");
        assert!(drawn(&fold, 1, 2, Locale::En).contains(">Show the passing check<"));
        fold.toggle();
        let h = drawn(&fold, 9, 11, Locale::En);
        assert!(h.contains(r#"aria-expanded="true""#) && h.contains(">Hide the passing checks<"), "{h}");
    }

    #[test]
    fn when_every_check_passes_a_line_says_so_and_with_none_passing_there_is_no_button() {
        let fold = PassFold::default();
        let h = drawn(&fold, 11, 11, Locale::En);
        assert!(h.contains(">All 11 checks pass.<") && h.contains("checks-passing"), "{h}");
        assert!(!drawn(&fold, 0, 3, Locale::En).contains("checks-passing"));
    }

    #[test]
    fn the_button_is_worded_in_french() {
        let fold = PassFold::default();
        let h = drawn(&fold, 9, 9, Locale::FrCa);
        assert!(h.contains(">Afficher les 9 vérifications réussies<") && h.contains(">Les 9 vérifications réussissent.<"), "{h}");
        fold.toggle();
        assert!(drawn(&fold, 9, 9, Locale::FrCa).contains(">Masquer les vérifications réussies<"));
    }
}
