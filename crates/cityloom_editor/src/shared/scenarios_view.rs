//! The scenarios of the place a street or junction page edits (`scenarios.rs`): a menu of the designs, the baseline
//! first, that shows one; a line that says which is shown and where its edits go; and, under a disclosure, a name
//! field and the actions. A refused name is said under the field, which it describes (`aria-describedby`), and in the
//! page's live region by the view-model. Every control is a native one, so all of it works from the keyboard.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::i18n::Args;
use crate::shared::locked::locked;
use crate::shared::said::say;
use crate::shared::scenarios::ScenarioHost;
use crate::shared::units::Units;

/// The scenarios section; nothing for a page whose place keeps none (a sandbox, a place with no key of its own).
#[component]
pub fn ScenarioPanel(host: Rc<dyn ScenarioHost>) -> impl IntoView {
    if !host.scenarios_offered() {
        return ().into_any();
    }
    let words = StoredValue::new_local(host.i18n());
    // shown read-only (#222), the designs can be looked at, and the name and the actions are offered locked
    let read_only = host.read_only();
    let host = StoredValue::new_local(host);
    // the name typed, kept apart from the model: an action that is done clears it
    let name = RwSignal::new(String::new());
    let picker = NodeRef::<leptos::html::Select>::new();
    // A delete shows the baseline, on which the pressed button is disabled: the focus goes to the menu of designs, so
    // the person goes on from where they were rather than from the start of the page.
    let delete = move |_| {
        if host.with_value(|h| h.delete_scenario())
            && let Some(select) = picker.get_untracked()
        {
            let _ = select.focus();
        }
    };
    let tr = move |key: &'static str| move || words.with_value(|i18n| i18n.tr(key, &Args::new()));
    let shown = move || host.with_value(|h| h.scenario_state().active());
    let baseline_shown = move || shown().is_none();
    let options = move || {
        host.with_value(|h| h.scenario_state().items())
            .into_iter()
            .map(|item| {
                let id = item.id;
                view! { <option value=id.to_string() prop:selected=move || shown() == Some(id)>{item.name}</option> }
            })
            .collect_view()
    };
    let mode = move || match shown() {
        None => words.with_value(|i18n| i18n.tr("scenario-showing-baseline", &Args::new())),
        Some(id) => {
            let name = host.with_value(|h| h.scenario_state().items()).into_iter().find(|i| i.id == id).map(|i| i.name).unwrap_or_default();
            words.with_value(|i18n| i18n.tr("scenario-showing", &Args::new().str("name", name)))
        }
    };
    let refusal = move || host.with_value(|h| h.scenario_state().refusal()).map(|why| words.with_value(|i18n| say(i18n, Units::Metres, &why)));
    let refused = move || refusal().is_some().then_some("true");
    let asking = move || host.with_value(|h| h.scenario_state().asking_delete());
    // The label shown (`swap-on`) and the other (`swap-off`, hidden, and so in no accessible name), in one cell: the
    // button is always as wide as the longer, so asking to confirm moves nothing.
    let swap = move |confirming: bool| move || if asking() == confirming { "swap-on" } else { "swap-off" };
    // An action on the name typed: the field is emptied when it is done.
    let with_name = move |act: fn(&dyn ScenarioHost, &str) -> bool| {
        move |_| {
            let typed = name.get_untracked();
            if host.with_value(|h| act(h.as_ref(), &typed)) {
                name.set(String::new());
            }
        }
    };
    let switch = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        let id = value.parse::<u32>().ok();
        host.with_value(|h| h.switch_scenario(id));
    };
    view! {
        <section class="scenarios" aria-labelledby="scenario-h">
            <h2 id="scenario-h" class="scenario-h">{tr("scenario-heading")}</h2>
            <div class="scenario-row">
                <label for="scenario-pick">{tr("scenario-pick")}</label>
                <select node_ref=picker id="scenario-pick" aria-describedby="scenario-mode" prop:value=move || shown().map(|id| id.to_string()).unwrap_or_default() on:change=switch>
                    <option value="" prop:selected=baseline_shown>{tr("scenario-baseline")}</option>
                    {options}
                </select>
            </div>
            <p id="scenario-mode" class="scenario-mode" class:in-scenario=move || !baseline_shown()>{mode}</p>
            <details class="scenario-manage">
                <summary>{tr("scenario-manage")}</summary>
                {locked(read_only, view! {
                <label for="scenario-name">{tr("scenario-name")}</label>
                <input
                    id="scenario-name"
                    type="text"
                    autocomplete="off"
                    aria-describedby="scenario-refusal"
                    aria-invalid=refused
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
                <p id="scenario-refusal" class="scenario-refusal">{refusal}</p>
                <div class="scenario-actions">
                    <button type="button" class="btn" id="scenario-new" on:click=with_name(|h, n| h.create_scenario(n))>{tr("scenario-new")}</button>
                    <button type="button" class="btn" id="scenario-duplicate" disabled=baseline_shown on:click=with_name(|h, n| h.duplicate_scenario(n))>
                        {tr("scenario-duplicate")}
                    </button>
                    <button type="button" class="btn" id="scenario-rename" disabled=baseline_shown on:click=with_name(|h, n| h.rename_scenario(n))>
                        {tr("scenario-rename")}
                    </button>
                    <button type="button" class="btn" id="scenario-delete" disabled=baseline_shown on:click=delete>
                        <span class="swap">
                            <span class=swap(false)>{tr("scenario-delete")}</span>
                            <span class=swap(true)>{tr("scenario-delete-confirm")}</span>
                        </span>
                    </button>
                </div>
                })}
            </details>
        </section>
    }
    .into_any()
}
