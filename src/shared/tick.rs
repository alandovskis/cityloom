//! A checker's tick beside a status line that says the sheet works. It is drawn in
//! once, when the sheet comes to work after not working: the last piece of a
//! street fitting its width, the last failed check of a junction or the city
//! passing again. A sheet that worked when it was opened just carries the tick.

use leptos::prelude::*;

/// Whether the sheet has just come to work: it did not before, and it does now.
/// `before` is nothing the first time it is asked.
pub fn comes_to_work(before: Option<bool>, now: bool) -> bool {
    before == Some(false) && now
}

/// The tick, while `works`. It draws itself in when `works` turns from false to true.
#[component]
pub fn Tick(#[prop(into)] works: Signal<bool>) -> impl IntoView {
    // A memo, so that an edit that leaves the sheet working does not draw it again.
    let works = Memo::new(move |_| works.get());
    let before = StoredValue::new(None::<bool>);
    move || {
        let now = works.get();
        let fresh = comes_to_work(before.get_value(), now);
        before.set_value(Some(now));
        now.then(|| {
            view! {
                <svg class="btn-ico tick" class:fresh=fresh viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" focusable="false">
                    <path d="M3 8.6 6.1 11.7 13 4.6" pathLength="1"></path>
                </svg>
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_that_works_on_arrival_is_not_celebrated() {
        assert!(!comes_to_work(None, true));
    }

    #[test]
    fn a_sheet_that_comes_to_work_after_not_working_is() {
        assert!(comes_to_work(Some(false), true));
    }

    #[test]
    fn a_sheet_that_goes_on_working_or_does_not_work_is_not() {
        assert!(!comes_to_work(Some(true), true));
        assert!(!comes_to_work(Some(false), false));
        assert!(!comes_to_work(Some(true), false));
        assert!(!comes_to_work(None, false));
    }
}
