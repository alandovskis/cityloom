//! The commands beside the curbside strips: the use, its length, add a use into the chosen free stretch and remove the
//! chosen use. One named group, like the zone commands; every control is a tab stop.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use super::model::Use;
use super::vm::StreetVm;
use super::watch::SheetWatch;
use super::zone_tools::focus_later;
use crate::shared::catalogue::Side;
use crate::shared::i18n::Args;
use crate::shared::units::parse_length_mm;

/// The use the select shows as a value, and reads back.
fn use_value(u: Use) -> String {
    Use::ALL.iter().position(|x| *x == u).unwrap_or(0).to_string()
}

fn use_of(value: &str) -> Use {
    value.parse::<usize>().ok().and_then(|n| Use::ALL.get(n).copied()).unwrap_or(Use::Parking)
}

/// The strip to give the focus back to after a command.
fn strip_to_focus(w: SheetWatch) -> Option<&'static str> {
    w.view_now().curb.sides.iter().find(|s| s.selected.is_some()).map(|s| if s.side == Side::Left { "curb-strip-left" } else { "curb-strip-right" })
}

/// Adds what the controls say, at the start of the chosen free stretch: the length typed, else the use's minimum.
/// False when the length is not a number (said) or the add was refused (said by the view-model).
pub fn commit_add(w: SheetWatch, use_: Use, typed: &str) -> bool {
    let length = if typed.trim().is_empty() {
        use_.min_mm()
    } else {
        match parse_length_mm(typed, w.units_now()) {
            Some(mm) => mm,
            None => {
                w.say_bad_number();
                return false;
            }
        }
    };
    w.add_curb(use_, length)
}

/// Gives the chosen use the length that was typed.
pub fn commit_length(w: SheetWatch, typed: &str) -> bool {
    match parse_length_mm(typed, w.units_now()) {
        Some(mm) => w.resize_curb(mm),
        None => {
            w.say_bad_number();
            false
        }
    }
}

#[component]
pub fn CurbTools(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let text = move |key: &'static str| w.i18n().tr(key, &Args::new());
    let unit_text = move |key: &'static str| w.i18n().tr(key, &Args::new().str("unit", w.units().word()));
    // What the select offers to add while a free stretch (or nothing) is chosen.
    let pick = RwSignal::new(Use::Parking);
    let chosen = move || {
        let v = w.view();
        v.curb.sides.iter().find_map(|s| s.cells.get(s.selected?)).map(|c| (c.use_, c.from_mm, c.to_mm))
    };
    let chosen_use = move || chosen().and_then(|c| c.0);
    let shown_use = move || chosen_use().unwrap_or_else(|| pick.get());
    let length_value = move || chosen().and_then(|c| c.0.map(|_| w.units().number_in(c.2 - c.1, w.i18n().locale()))).unwrap_or_default();
    let any = move || !w.view().curb.sides.is_empty();
    let options = move || {
        Use::ALL
            .iter()
            .map(|u| {
                let u = *u;
                view! {
                    <option value=use_value(u)>
                        {move || w.i18n().tr(u.name_key(), &Args::new())}
                    </option>
                }
            })
            .collect_view()
    };
    let typed = move |id: &str| document().get_element_by_id(id).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default();
    view! {
        <Show when=any>
            <section class="insp-sec" aria-labelledby="h-curb-tools">
                <h3 class="note-h" id="h-curb-tools">{move || text("st-curb-tools")}</h3>
                <div class="zone-tools curb-tools" role="group" aria-label=move || text("st-curb-tools")>
                    <div class="zone-tool">
                        <label for="curb-use">{move || text("st-curb-use")}</label>
                        <select
                            id="curb-use"
                            prop:value=move || use_value(shown_use())
                            on:change=move |e| {
                                let u = use_of(&event_target::<HtmlSelectElement>(&e).value());
                                pick.set(u);
                                if chosen_use().is_some() {
                                    w.set_curb_use(u);
                                }
                            }
                        >
                            {options}
                        </select>
                    </div>
                    <div class="zone-tool">
                        <label for="curb-length">{move || unit_text("st-curb-length")}</label>
                        <input
                            id="curb-length"
                            type="text"
                            inputmode="decimal"
                            autocomplete="off"
                            prop:value=length_value
                            on:keydown=move |e| {
                                if e.key() == "Enter" {
                                    e.prevent_default();
                                    let input = event_target::<HtmlInputElement>(&e);
                                    if chosen_use().is_some() {
                                        if !commit_length(w, &input.value()) {
                                            input.set_value(&length_value());
                                        }
                                    } else if commit_add(w, shown_use(), &input.value()) {
                                        input.set_value("");
                                    }
                                }
                            }
                        />
                    </div>
                    <div class="zone-tool">
                        <button
                            type="button"
                            id="curb-add"
                            aria-disabled=move || chosen().is_none_or(|c| c.0.is_some()).then_some("true")
                            on:click=move |_| {
                                let t = typed("curb-length");
                                if commit_add(w, shown_use(), &t) {
                                    if let Some(id) = strip_to_focus(w) {
                                        focus_later(id);
                                    }
                                }
                            }
                        >
                            {move || text("st-curb-add")}
                        </button>
                        <button
                            type="button"
                            id="curb-remove"
                            aria-disabled=move || chosen_use().is_none().then_some("true")
                            on:click=move |_| {
                                let side = strip_to_focus(w);
                                if w.remove_curb() {
                                    if let Some(id) = side {
                                        focus_later(id);
                                    }
                                }
                            }
                        >
                            {move || text("st-curb-remove")}
                        </button>
                    </div>
                </div>
            </section>
        </Show>
    }
}
