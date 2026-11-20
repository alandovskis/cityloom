//! The picked stretch's ends as fields, on the details panel's Street level: a way to pick part of a run with a single
//! pointer, without dragging along the plan (WCAG 2.5.7), and to type a stretch exactly. Positions count from the
//! plan's left end, in the page's units. The view forwards: a change in either field (Enter, or leaving it changed) goes
//! to the view-model's `pick_typed`, which says what was picked or why not.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::Args;
use crate::street::vm::StreetVm;
use crate::street::vm::watch::SheetWatch;

#[component]
pub fn PickFields(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm.clone());
    let vm = StoredValue::new_local(vm);
    let from = NodeRef::<leptos::html::Input>::new();
    let to = NodeRef::<leptos::html::Input>::new();
    let ends = move || vm.with_value(|vm| vm.pick_ends_shown());
    let from_value = move || ends().map(|e| e.0).unwrap_or_default();
    let to_value = move || ends().map(|e| e.1).unwrap_or_default();
    let text = move |key: &'static str| w.i18n().tr(key, &Args::new());
    let unit_text = move |key: &'static str| w.i18n().tr(key, &Args::new().str("unit", w.units().word()));
    let commit = move || {
        let (Some(a), Some(b)) = (from.get_untracked(), to.get_untracked()) else { return };
        vm.with_value(|vm| vm.pick_typed(&a.value(), &b.value()));
        // The fields show the pick as it now is, whether it was taken or refused.
        if let Some((x, y)) = untrack(ends) {
            a.set_value(&x);
            b.set_value(&y);
        }
    };
    view! {
        <Show when=move || ends().is_some()>
            <div class="zone-tools pick-fields" role="group" aria-label=move || text("st-pick-fields")>
                <div class="zone-tool">
                    <label for="pick-from">{move || unit_text("st-pick-from")}</label>
                    <input
                        id="pick-from"
                        type="text"
                        inputmode="decimal"
                        autocomplete="off"
                        node_ref=from
                        value=from_value
                        prop:value=from_value
                        on:change=move |_| commit()
                    />
                </div>
                <div class="zone-tool">
                    <label for="pick-to">{move || unit_text("st-pick-to")}</label>
                    <input
                        id="pick-to"
                        type="text"
                        inputmode="decimal"
                        autocomplete="off"
                        node_ref=to
                        value=to_value
                        prop:value=to_value
                        on:change=move |_| commit()
                    />
                </div>
            </div>
        </Show>
    }
}
