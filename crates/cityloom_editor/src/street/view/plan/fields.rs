//! The picked stretch's ends as fields, and the Move to field of a boundary a click on the plan chose, on the details
//! panel's Street level: a way to pick part of a run and to move a boundary with a single pointer, without dragging
//! along the plan (WCAG 2.5.7), and to type either exactly. Positions count from the plan's left end, in the page's
//! units. The view forwards: a change in a pick field (Enter, or leaving it changed) goes to the view-model's
//! `type_pick_from` or `type_pick_to`, which keep what was typed until it makes a stretch, Enter in the Move to field to `move_chosen_boundary` and Escape there to `clear_boundary`; each says
//! what it did or why not.

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
    let commit = move |is_from: bool| {
        let (Some(a), Some(b)) = (from.get_untracked(), to.get_untracked()) else { return };
        vm.with_value(|vm| if is_from { vm.type_pick_from(&a.value()) } else { vm.type_pick_to(&b.value()) });
        // The fields show what the view-model says they hold: the pick, or what was typed and is waiting.
        if let Some((x, y)) = untrack(ends) {
            a.set_value(&x);
            b.set_value(&y);
        }
    };
    let boundary_to = move || vm.with_value(|vm| vm.boundary_to_shown());
    let boundary_value = move || boundary_to().unwrap_or_default();
    let on_boundary_key = move |e: leptos::ev::KeyboardEvent| match e.key().as_str() {
        "Enter" => {
            e.prevent_default();
            let input = event_target::<leptos::web_sys::HtmlInputElement>(&e);
            vm.with_value(|vm| vm.move_chosen_boundary(&input.value()));
            // The field shows where the boundary now is, whether it moved or not.
            input.set_value(&untrack(boundary_value));
        }
        "Escape" => {
            e.prevent_default();
            vm.with_value(|vm| vm.clear_boundary());
            // The field goes with the choice: the keys go back to the plan's point.
            crate::street::view::focus_later("plan-point");
        }
        _ => {}
    };
    view! {
        <Show when=move || boundary_to().is_some()>
            <div class="zone-tools boundary-fields">
                <div class="zone-tool">
                    <label for="boundary-to">{move || unit_text("st-boundary-to")}</label>
                    <input
                        id="boundary-to"
                        type="text"
                        inputmode="decimal"
                        autocomplete="off"
                        value=boundary_value
                        prop:value=boundary_value
                        on:keydown=on_boundary_key
                    />
                </div>
            </div>
        </Show>
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
                        on:change=move |_| commit(true)
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
                        on:change=move |_| commit(false)
                    />
                </div>
            </div>
        </Show>
    }
}
