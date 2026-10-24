//! The Speed limit control of the street page's left panel: one labelled select, Not set first. A change is one
//! undoable edit, announced by the view-model; the focus stays on the select.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlSelectElement;

use super::vm::StreetVm;
use super::watch::SheetWatch;
use crate::shared::i18n::Args;

#[component]
pub fn SpeedLimit(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    // A refused or unchanged choice leaves the select showing what the street has.
    let show_current = move |select: &HtmlSelectElement| {
        let current = w.view_now().speed_kmh.map(|n| n.to_string()).unwrap_or_default();
        select.set_value(&current);
    };
    let select_ref = NodeRef::<leptos::html::Select>::new();
    let shown = view! {
        <div class="speed-limit">
            <label for="speed-limit">{move || w.i18n().tr("st-speed-label", &Args::new())}</label>
            <select
                node_ref=select_ref
                id="speed-limit"
                on:change=move |ev| {
                    let Some(select) = ev.target().and_then(|t| t.dyn_into::<HtmlSelectElement>().ok()) else { return };
                    let kmh = select.value().parse::<u16>().ok();
                    w.set_speed(kmh);
                    show_current(&select);
                }
            >
                {move || {
                    w.speed_options()
                        .into_iter()
                        .map(|o| {
                            let value = o.kmh.map(|n| n.to_string()).unwrap_or_default();
                            view! { <option value=value selected=o.selected>{o.label}</option> }
                        })
                        .collect_view()
                }}
            </select>
        </div>
    };
    // The browser ignores `selected` on an option once the person has picked one, so after Undo, Redo or Start
    // over the select is told its value. Made after the view so that it runs after the options are rebuilt.
    Effect::new(move |_| {
        let current = w.speed_options().into_iter().find(|o| o.selected).and_then(|o| o.kmh).map(|n| n.to_string());
        if let Some(select) = select_ref.get() {
            select.set_value(&current.unwrap_or_default());
        }
    });
    shown
}
