//! The street half of the comparison page: each design's plan and 2.5D, side by side, at one scale.

use std::rc::Rc;

use leptos::prelude::*;

use super::iso::{IsoSvg, iso_label, iso_pair};
use super::plan::plan_pair;
use crate::shared::compare::CompareHost;
use crate::shared::compare_view::CompareFrame;
use crate::shared::i18n::Args;
use crate::shared::said::say;
use crate::street::vm::compare::StreetCompareVm;
use crate::street::vm::text::plan_label;

/// The width both drawings are laid out for; the page's styles scale them to the column (`width: 100%`).
const DRAWN_WIDTH: f64 = 560.0;

/// One design's drawings: what the plan and the 2.5D are, for a screen reader, and their markup.
#[derive(Clone)]
struct Column {
    plan: String,
    plan_label: String,
    iso: IsoSvg,
    iso_label: String,
}

/// The two designs' columns, drawn together (they share a scale), again when a side is chosen again, the units
/// change or the language does (the names are asked with the tracked `tr`).
fn columns(vm: &StreetCompareVm) -> [Column; 2] {
    let i18n = vm.i18n();
    let units = vm.units();
    let (a, b) = vm.views();
    let (plan_a, plan_b) = plan_pair(&a.plan, &b.plan, DRAWN_WIDTH, &i18n, units);
    let (iso_a, iso_b) = iso_pair(&a, &b, &i18n, DRAWN_WIDTH);
    // A plan with no length has no end to measure from, and is named without one.
    let label = |v: &crate::street::model::View| plan_label(&v.plan).map_or_else(|| i18n.tr("cmp-plan-label", &Args::new()), |said| say(&i18n, units, &said));
    [(a, plan_a, iso_a), (b, plan_b, iso_b)].map(|(v, plan, iso)| Column { plan_label: label(&v), iso_label: iso_label(&v, &i18n, units), plan, iso })
}

/// The street comparison page's body: the frame, and in each column the plan and then the 2.5D of its design.
#[component]
pub fn StreetCompare(vm: Rc<StreetCompareVm>) -> impl IntoView {
    // The view-model is not `Send`: the memo holds it in a local store.
    let held = StoredValue::new_local(vm.clone());
    // Never equal: the memo runs again only when a side, the units or the language change, and the pair is then new.
    let both = Memo::new_with_compare(move |_| held.with_value(|vm| columns(vm)), |_, _| false);
    let column = move |at: usize| {
        move || {
            let c = both.with(|both| both[at].clone());
            view! {
                <figure class="compare-plan" role="img" aria-label=c.plan_label inner_html=c.plan></figure>
                <figure class="compare-iso" role="img" aria-label=c.iso_label>
                    <svg
                        class="iso"
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox=format!("0 0 {} {}", c.iso.width, c.iso.height)
                        inner_html=c.iso.markup
                    ></svg>
                </figure>
            }
            .into_any()
        }
    };
    view! { <CompareFrame host=vm column_a=column(0) column_b=column(1)/> }
}
