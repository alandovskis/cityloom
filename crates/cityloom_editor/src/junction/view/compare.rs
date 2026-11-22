//! The junction half of the comparison page: each design's 2.5D, side by side, at one scale.

use std::rc::Rc;

use leptos::prelude::*;

use super::iso::{IsoSvg, iso_label, iso_pair};
use crate::junction::vm::compare::JunctionCompareVm;
use crate::shared::compare::CompareHost;
use crate::shared::compare_view::CompareFrame;

/// The width both drawings are laid out for; the page's styles scale them to the column (`width: 100%`).
const DRAWN_WIDTH: f64 = 560.0;

/// The height they are fitted in: a junction is about as tall as it is wide, and a column is narrower than a page.
const DRAWN_HEIGHT: f64 = 420.0;

/// One design's drawing: what it is, for a screen reader, and its markup.
struct Column {
    iso: IsoSvg,
    iso_label: String,
}

/// The two designs' columns, drawn together (they share a scale), again when a side is chosen again, the units
/// change or the language does (the names are asked with the tracked `tr`).
fn columns(vm: &JunctionCompareVm) -> [Column; 2] {
    let i18n = vm.i18n();
    let (a, b) = vm.views();
    let (iso_a, iso_b) = iso_pair(&a, &b, &i18n, DRAWN_WIDTH, DRAWN_HEIGHT, vm.units());
    [(a, iso_a), (b, iso_b)].map(|(v, iso)| Column { iso_label: iso_label(&v, &i18n), iso })
}

/// The junction comparison page's body: the frame, and in each column the 2.5D of its design.
#[component]
pub fn JunctionCompare(vm: Rc<JunctionCompareVm>) -> impl IntoView {
    // The view-model is not `Send`: the memo holds it in a local store.
    let held = StoredValue::new_local(vm.clone());
    // Always changed (`changed` answers true): the memo runs again only when a side, the units or the language change,
    // and the pair is then new, so the columns are told every time. Answering false would keep the first drawing.
    let both = Memo::new_with_compare(move |_| held.with_value(|vm| columns(vm)), |_, _| true);
    let column = move |at: usize| {
        move || {
            let (iso_label, frame, markup) = both.with(|both| (both[at].iso_label.clone(), both[at].iso.frame, both[at].iso.markup.clone()));
            view! {
                <figure class="compare-iso" role="img" aria-label=iso_label>
                    <svg
                        class="iso"
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox=format!("0 0 {} {}", frame.width, frame.height)
                        inner_html=markup
                    ></svg>
                </figure>
            }
            .into_any()
        }
    };
    view! { <CompareFrame host=vm column_a=column(0) column_b=column(1)/> }
}
