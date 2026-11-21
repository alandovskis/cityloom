//! The Speed limit control of the street page's left panel: one labelled select, Not set first. A change is one
//! undoable edit, announced by the view-model; the focus stays on the select.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlSelectElement;

use crate::shared::i18n::Args;
use crate::shared::said::{Arg, Said, say};
use crate::street::vm::watch::SheetWatch;
use crate::street::vm::{SpeedOption, StreetVm};

/// The value of the option "Same as the street".
const STREET: &str = "street";

/// What an option's `value` is: its speed, nothing for Not set, or `STREET`.
fn value_of(o: &SpeedOption) -> String {
    if o.street { STREET.to_string() } else { o.kmh.map(|n| n.to_string()).unwrap_or_default() }
}

/// The value of the chosen option.
fn current_value(options: &[SpeedOption]) -> String {
    options.iter().find(|o| o.selected).map(value_of).unwrap_or_default()
}

#[component]
pub fn SpeedLimit(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    // A refused or unchanged choice leaves the select showing what the street has.
    let show_current = move |select: &HtmlSelectElement| {
        select.set_value(&current_value(&w.speed_options()));
    };
    let select_ref = NodeRef::<leptos::html::Select>::new();
    let shown = view! {
        <div class="zone-row speed-limit">
            <label for="speed-limit" class="zone-l">{move || w.i18n().tr("st-speed-label", &Args::new())}</label>
            <div class="zone-c">
            <select
                node_ref=select_ref
                id="speed-limit"
                on:change=move |ev| {
                    let Some(select) = ev.target().and_then(|t| t.dyn_into::<HtmlSelectElement>().ok()) else { return };
                    let value = select.value();
                    if value == STREET {
                        w.follow_street_speed();
                    } else {
                        w.set_speed(value.parse::<u16>().ok());
                    }
                    show_current(&select);
                }
            >
                {move || {
                    w.speed_options()
                        .into_iter()
                        .map(|o| {
                            let value = value_of(&o);
                            view! { <option value=value selected=o.selected>{o.label}</option> }
                        })
                        .collect_view()
                }}
            </select>
            {move || {
                // With a stretch picked the limit set here is the stretch's, so a line says which; where the picked
                // runs have different limits, the one shown is the shown run's, and a line says it varies.
                let (v, units) = w.now();
                let i18n = w.i18n();
                let stretch = v.plan.pick.map(|(from, to)| {
                    let hint = Said::new("st-speed-stretch-hint").with("from", Arg::Length(from)).with("to", Arg::Length(to));
                    view! { <p class="zone-hint">{say(&i18n, units, &hint)}</p> }
                });
                let varies = v.speed_varies.then(|| view! { <p class="zone-hint">{i18n.tr("st-varies", &Args::new())}</p> });
                (stretch, varies)
            }}
            </div>
        </div>
    };
    // The browser ignores `selected` on an option once the person has picked one, so after Undo, Redo or Start
    // over the select is told its value. Made after the view so that it runs after the options are rebuilt.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let current = current_value(&w.speed_options());
        if let Some(select) = select_ref.get() {
            select.set_value(&current);
        }
    });
    shown
}
