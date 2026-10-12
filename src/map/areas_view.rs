//! The Area rules panel on the left of the map page: the area rules, declaring one by drawing it on the map, and changing or
//! deleting the one selected. The views show what the map's view-model says and send it what the person does.
//!
//! Where the keyboard's focus goes when the panel changes what it shows is the views' to say, since a control that
//! was pressed may be gone: drawing starts on the map (Enter places a corner there), a closed outline asks for its
//! rule in the form (the map takes no key from it), and back in the list the selected area's row, or Declare, has it
//! if it was lost.

use std::rc::Rc;

use leptos::prelude::*;

use crate::map::vm::{AreaStreet, MapVm};
use crate::restriction::draw::Draw;
use crate::restriction::model::{DEFAULT_SPEED, Rule, SPEEDS};
use crate::shared::bind::Bound;

type Vm = Bound<MapVm>;

/// A message of the panel, for a view: it follows a switch of language.
fn word(vm: Vm, key: &'static str) -> impl Fn() -> String + Copy + 'static {
    move || vm.with(|v| v.areas().word(key))
}

/// What the panel shows: the list, the outline being drawn, or the closed outline's rule and name.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    List,
    Drawing,
    Naming,
}

/// Moves the keyboard's focus to the element `id` once the panel is drawn; with `only_if_lost`, only when nothing has
/// it (the control pressed is gone).
fn focus_later(id: String, only_if_lost: bool) {
    #[cfg(target_arch = "wasm32")]
    request_animation_frame(move || {
        use leptos::wasm_bindgen::JsCast;

        let doc = leptos::prelude::document();
        let lost = doc.active_element().is_none_or(|el| el.tag_name() == "BODY");
        if only_if_lost && !lost {
            return;
        }
        if let Some(el) = doc.get_element_by_id(&id).and_then(|el| el.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
            let _ = el.focus();
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, only_if_lost);
}

/// Where the focus goes back to in the list: the selected area's row, else Declare.
fn focus_list(vm: Vm) {
    match vm.with(|v| v.areas().selected_id_now()) {
        Some(id) => focus_later(format!("area-row-{id}"), true),
        None => focus_later("declare-area".to_string(), true),
    }
}

/// The shell's own control to press so the Area rules panel is shown, given the left toggle's `aria-expanded`: the
/// toggle, if the panel is closed. The map slice does not reach the shell's view-model; it presses what a person would.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn reveal_clicks(panel_expanded: Option<&str>) -> Vec<&'static str> {
    if panel_expanded == Some("false") { vec!["inspector-toggle"] } else { Vec::new() }
}

/// Shows the Area rules panel, where an outline is finished, cancelled and named, wherever the person had gone: a
/// closed outline takes no key from the map, so its form must be reachable.
fn reveal() {
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;

        let doc = leptos::prelude::document();
        let attr = |id: &str, name: &str| doc.get_element_by_id(id).and_then(|el| el.get_attribute(name));
        for id in reveal_clicks(attr("inspector-toggle", "aria-expanded").as_deref()) {
            if let Some(el) = doc.get_element_by_id(id).and_then(|el| el.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
                el.click();
            }
        }
    }
}

/// The panel: the list, or the outline being drawn, or the closed outline's rule and name.
#[component]
pub fn Areas(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;
        use leptos::web_sys::{Element, KeyboardEvent};

        use crate::shared::shortcut::{Shortcut, shortcut};
        // Ctrl or Cmd with Z brings back the area deleted last, outside a field (which has its own undo).
        let handle = window_event_listener(leptos::ev::keydown, move |e: KeyboardEvent| {
            let in_a_field =
                e.target().and_then(|t| t.dyn_into::<Element>().ok()).is_some_and(|el| el.closest("input, select, textarea").ok().flatten().is_some());
            if (e.ctrl_key() || e.meta_key())
                && !in_a_field
                && shortcut(&e.key(), e.shift_key()) == Some(Shortcut::Undo)
                && vm.with(|v| v.areas().undo_delete())
            {
                e.prevent_default();
                // The Undo button it stood for is gone: the area brought back takes the focus if it was there.
                focus_list(vm);
            }
        });
        on_cleanup(move || handle.remove());
    }
    // Only a change of stage draws the panel again: a corner placed or a rule chosen keeps the controls (and the focus).
    let stage = Memo::new(move |_| match vm.with(|v| v.areas().draw()) {
        Draw::Idle => Stage::List,
        Draw::Drawing(_) => Stage::Drawing,
        Draw::Naming(_) => Stage::Naming,
    });
    let first = StoredValue::new(true);
    move || {
        let back = !first.get_value();
        first.set_value(false);
        match stage.get() {
            Stage::List => idle(vm, back).into_any(),
            Stage::Drawing => drawing(vm).into_any(),
            Stage::Naming => naming(vm).into_any(),
        }
    }
}

/// The list. `back` when it comes back after an outline was drawn or named.
fn idle(vm: Vm, back: bool) -> impl IntoView {
    if back {
        // The control pressed (Save, Cancel, Finish) is gone: the area saved, or Declare, takes the focus.
        focus_list(vm);
    }
    view! {
        <p class="hint" id="areas-lead">{move || vm.with(|v| v.areas_lead())}</p>
        <button type="button" id="declare-area" class="btn" disabled=move || !vm.with(|v| v.can_declare()) on:click=move |_| {
            vm.with(|v| v.areas().start());
        }>{word(vm, "restriction-declare")}</button>
        {move || vm.with(|v| v.areas().deleted_note()).map(|note| view! {
            <p class="hint area-undone">
                {note}
                " "
                <button type="button" id="area-undo" class="btn quiet" on:click=move |_| {
                    if vm.with(|v| v.areas().undo_delete()) {
                        focus_list(vm);
                    }
                }>{word(vm, "restriction-undo")}</button>
            </p>
        })}
        <ul class="places-list area-list" id="areas">
            <For each=move || vm.with(|v| v.areas().list()) key=|r| r.id.clone() children=move |r| row(vm, r.id)/>
        </ul>
    }
}

/// An area's row: kept while the area is, so its form keeps the focus while the area changes.
fn row(vm: Vm, id: String) -> impl IntoView {
    let on = {
        let id = id.clone();
        Memo::new(move |_| vm.with(|v| v.areas().selected_id()).as_deref() == Some(id.as_str()))
    };
    let pick = {
        let id = id.clone();
        move |_| {
            vm.with(|v| {
                let was = v.areas().selected_id_now().as_deref() == Some(id.as_str());
                v.areas().select(if was { None } else { Some(id.as_str()) });
            })
        }
    };
    let (title_id, sub_id) = (id.clone(), id.clone());
    let row_id = format!("area-row-{id}");
    let details_id = format!("area-details-{id}");
    view! {
        <li class=move || if on.get() { "area on" } else { "area" }>
            <button
                type="button"
                id=row_id
                class="place-row"
                aria-expanded=move || on.get().to_string()
                aria-controls=move || on.get().then(|| details_id.clone())
                on:click=pick
            >
                <b>{move || vm.with(|v| v.areas().find(&title_id).map(|r| v.areas().say(&r.title())).unwrap_or_default())}</b>
                <small>{move || vm.with(|v| v.areas().find(&sub_id).map(|r| v.area_row_sub(&r)).unwrap_or_default())}</small>
            </button>
            {move || on.get().then(|| details(vm, id.clone()))}
        </li>
    }
}

/// The selected area's form, under its row: its rule and name, Redraw and Delete, and the streets inside.
fn details(vm: Vm, id: String) -> impl IntoView {
    let (name_id, edit_id, rename_id, redraw_id, delete_id, streets_id) = (id.clone(), id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
    let details_id = format!("area-details-{id}");
    view! {
        <div class="area-form" id=details_id>
            {rule_picker(vm, Some(id))}
            <label for="area-name">{word(vm, "restriction-name-label")}</label>
            <input
                type="text"
                id="area-name"
                prop:value=move || vm.with(|v| v.areas().name_field(&name_id))
                placeholder=move || vm.with(|v| v.areas().form_placeholder())
                on:input=move |e| vm.with(|v| v.areas().edit_name(&edit_id, &event_target_value(&e)))
                on:change=move |e| {
                    vm.with(|v| v.areas().rename(&rename_id, &event_target_value(&e)));
                }
            />
            <div class="btns">
                <button type="button" id="area-redraw" class="btn quiet" disabled=move || !vm.with(|v| v.can_declare()) on:click=move |_| {
                    vm.with(|v| v.areas().start_redraw(&redraw_id));
                }>{word(vm, "restriction-redraw")}</button>
                <button type="button" id="area-delete" class="btn quiet" on:click=move |_| {
                    if vm.with(|v| v.areas().delete(&delete_id)) {
                        focus_later("area-undo".to_string(), true);
                    }
                }>{word(vm, "restriction-delete")}</button>
            </div>
            {move || {
                let streets = vm.with(|v| v.area_streets(&streets_id));
                (!streets.is_empty()).then(|| view! {
                    <h3 class="note-h">{word(vm, "restriction-streets-heading")}</h3>
                    <ul class="checks places-list">{streets.into_iter().map(|s| street_row(vm, s)).collect_view()}</ul>
                })
            }}
        </div>
    }
}

fn street_row(vm: Vm, s: AreaStreet) -> impl IntoView {
    let ok = s.conflicts.is_empty();
    let d = if ok { "M2.5 8.5 6.5 12.5 13.5 3.5" } else { "M3 3l10 10M13 3 3 13" };
    let (name, href, detail) = (s.name.clone(), s.href.clone(), s);
    view! {
        <li class=if ok { "ok" } else { "bad" }>
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d=d/></svg>
            <div>
                <b><a href=href>{move || vm.with(|v| v.say(&name))}</a></b>
                <span>{move || vm.with(|v| v.street_detail(&detail))}</span>
            </div>
        </li>
    }
}

/// The rule of the closed outline (`id` None) or of the area `id`: the four rules, and the speed when it is a limit.
fn rule_picker(vm: Vm, id: Option<String>) -> impl IntoView {
    let id = StoredValue::new(id);
    let current = Memo::new(move |_| {
        vm.with(|v| match id.get_value() {
            Some(id) => v.areas().find(&id).map_or(Rule::CarFree, |r| r.rule),
            None => v.areas().form_rule(),
        })
    });
    let set = move |rule: Rule| {
        vm.with(|v| match id.get_value() {
            Some(id) => {
                v.areas().set_rule(&id, rule);
            }
            None => v.areas().set_form_rule(rule),
        })
    };
    let kmh = move || match current.get() {
        Rule::SpeedLimit { kmh } => kmh,
        _ => DEFAULT_SPEED,
    };
    // The speed's control comes and goes with a limit, not with each speed chosen (which would take its focus).
    let is_limit = Memo::new(move |_| matches!(current.get(), Rule::SpeedLimit { .. }));
    let options = Rule::IDS
        .iter()
        .map(|&rid| {
            view! { <option value=rid prop:selected=move || current.get().id() == rid>{move || vm.with(|v| v.areas().rule_choice(rid))}</option> }
        })
        .collect_view();
    view! {
        <label for="area-rule">{word(vm, "restriction-rule-label")}</label>
        <select id="area-rule" on:change=move |e| {
            if let Some(rule) = Rule::from_id(&event_target_value(&e), kmh()) {
                set(rule);
            }
        }>{options}</select>
        {move || is_limit.get().then(|| view! {
            <label for="area-speed">{word(vm, "restriction-speed-label")}</label>
            <select id="area-speed" on:change=move |e| {
                if let Ok(kmh) = event_target_value(&e).parse::<u8>() {
                    set(Rule::SpeedLimit { kmh });
                }
            }>
                {SPEEDS.iter().map(|&s| view! {
                    <option value=s.to_string() prop:selected=move || kmh() == s>{move || vm.with(|v| v.areas().kmh_text(s))}</option>
                }).collect_view()}
            </select>
        })}
    }
}

fn drawing(vm: Vm) -> impl IntoView {
    // Finish and Cancel are in the panel: it is shown. The keyboard places corners on the map: it goes there, where
    // Enter places one at the cross.
    reveal();
    focus_later("basemap".to_string(), false);
    let few = move || vm.with(|v| v.areas().draw().corners().len()) < 3;
    view! {
        <p class="hint">{word(vm, "restriction-drawing-help")}</p>
        <p class="hint"><b>{move || vm.with(|v| v.areas().corners_text())}</b></p>
        <div class="btns">
            <button type="button" id="area-finish" class="btn" disabled=few on:click=move |_| {
                vm.with(|v| v.areas().close());
            }>{word(vm, "restriction-finish")}</button>
            <button type="button" id="area-cancel" class="btn quiet" on:click=move |_| {
                vm.with(|v| v.areas().cancel());
            }>{word(vm, "restriction-cancel")}</button>
        </div>
    }
}

fn naming(vm: Vm) -> impl IntoView {
    // The map takes no key from a closed outline: the form, where its rule is chosen first, does.
    reveal();
    focus_later("area-rule".to_string(), false);
    view! {
        <form class="area-form" on:submit=move |e| {
            e.prevent_default();
            vm.with(|v| v.areas().save());
        }>
            <p class="hint">{word(vm, "restriction-naming-help")}</p>
            {rule_picker(vm, None)}
            <label for="area-name">{word(vm, "restriction-name-label")}</label>
            <input
                type="text"
                id="area-name"
                prop:value=move || vm.with(|v| v.areas().form_name())
                placeholder=move || vm.with(|v| v.areas().form_placeholder())
                on:input=move |e| vm.with(|v| v.areas().set_form_name(&event_target_value(&e)))
            />
            <div class="btns">
                <button type="submit" id="area-save" class="btn">{word(vm, "restriction-save")}</button>
                <button type="button" id="area-cancel" class="btn quiet" on:click=move |_| {
                    vm.with(|v| v.areas().cancel());
                }>{word(vm, "restriction-cancel")}</button>
            </div>
        </form>
    }
}
