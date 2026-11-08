//! The commands beside the raised elements' strip: the kind, the length of the chosen element, add an element into the
//! chosen free stretch and remove the chosen one. One named group, like the curbside commands; every control is a tab
//! stop.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use super::curb_tools::typed_length;
use super::model::CalmingKind;
use super::row::CalmingRow;
use super::vm::StreetVm;
use super::watch::SheetWatch;
use super::zone_tools::focus_later;
use crate::shared::catalogue::Side;
use crate::shared::i18n::Args;

/// The kind the select shows as a value, and reads back.
fn kind_value(k: CalmingKind) -> String {
    CalmingKind::ALL.iter().position(|x| *x == k).unwrap_or(0).to_string()
}

fn kind_of(value: &str) -> CalmingKind {
    value.parse::<usize>().ok().and_then(|n| CalmingKind::ALL.get(n).copied()).unwrap_or(CalmingKind::Table)
}

fn side_value(side: Side) -> &'static str {
    if side == Side::Left { "left" } else { "right" }
}

fn side_of(value: &str) -> Side {
    if value == "left" { Side::Left } else { Side::Right }
}

/// The chosen cell: where it starts and ends and whether it is an element (else a free stretch).
fn chosen(w: SheetWatch) -> Option<(i32, i32, bool)> {
    let v = w.view();
    let c = v.calming_strip.cells.get(v.calming_strip.selected?)?;
    Some((c.from_mm, c.to_mm, c.element.is_some()))
}

/// Adds an element of `kind` into the chosen free stretch: the length typed, else the default. False when the length
/// is not a number (said) or the add was refused (said by the view-model).
pub fn commit_add(w: SheetWatch, kind: CalmingKind, typed: &str) -> bool {
    typed_length(w, typed, Some(kind.default_mm())).is_some_and(|mm| w.add_calming(kind, mm))
}

/// Gives the chosen element the length that was typed.
pub fn commit_length(w: SheetWatch, typed: &str) -> bool {
    typed_length(w, typed, None).is_some_and(|mm| w.set_calming_length(mm))
}

#[component]
pub fn CalmingTools(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let text = move |key: &'static str| w.i18n().tr(key, &Args::new());
    let unit_text = move |key: &'static str| w.i18n().tr(key, &Args::new().str("unit", w.units().word()));
    let is_element = move || chosen(w).is_some_and(|c| c.2);
    // The length shown: the chosen element's; else what an add would use (the default), as a hint to type over.
    let length_value = move || match chosen(w) {
        Some((from, to, true)) => w.units().number_in(to - from, w.i18n().locale()),
        _ => String::new(),
    };
    let placeholder = move || w.units().number_in(w.calming_kind_shown().default_mm(), w.i18n().locale());
    let is_chicane = move || w.calming_kind_shown() == CalmingKind::Chicane;
    let any = move || !w.view().calming_strip.cells.is_empty();
    let options = move || {
        CalmingKind::ALL
            .iter()
            .map(|k| {
                let k = *k;
                view! {
                    <option value=kind_value(k)>{move || w.i18n().tr(k.name_key(), &Args::new())}</option>
                }
            })
            .collect_view()
    };
    let typed =
        move || document().get_element_by_id("calming-length").and_then(|e| e.dyn_into::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default();
    view! {
        <Show when=any>
            <section class="insp-sec" aria-labelledby="h-calming-tools">
                <h3 class="note-h" id="h-calming-tools">{move || text("st-calming-tools")}</h3>
                <div class="zone-tools calming-tools" role="group" aria-label=move || text("st-calming-tools")>
                    <div class="zone-tool">
                        <label for="calming-kind">{move || text("st-calming-kind")}</label>
                        <select
                            id="calming-kind"
                            prop:value=move || kind_value(w.calming_kind_shown())
                            on:change=move |e| w.pick_calming_kind(kind_of(&event_target::<HtmlSelectElement>(&e).value()))
                        >
                            {options}
                        </select>
                    </div>
                    <Show when=is_chicane>
                        <div class="zone-tool">
                            <label for="calming-first">{move || text("st-calming-first")}</label>
                            <select
                                id="calming-first"
                                prop:value=move || side_value(w.calming_first_shown())
                                on:change=move |e| w.pick_calming_first(side_of(&event_target::<HtmlSelectElement>(&e).value()))
                            >
                                <option value="left">{move || text("st-calming-first-left")}</option>
                                <option value="right">{move || text("st-calming-first-right")}</option>
                            </select>
                        </div>
                    </Show>
                    <div class="zone-tool">
                        <label for="calming-length">{move || unit_text("st-calming-length")}</label>
                        <input
                            id="calming-length"
                            type="text"
                            inputmode="decimal"
                            autocomplete="off"
                            placeholder=placeholder
                            prop:value=length_value
                            on:keydown=move |e| {
                                if e.key() == "Enter" {
                                    e.prevent_default();
                                    let input = event_target::<HtmlInputElement>(&e);
                                    if is_element() {
                                        if !commit_length(w, &input.value()) {
                                            input.set_value(&length_value());
                                        }
                                    } else if commit_add(w, w.calming_kind_shown(), &input.value()) {
                                        input.set_value("");
                                    }
                                }
                            }
                        />
                    </div>
                    <div class="zone-tool">
                        <button
                            type="button"
                            id="calming-add"
                            aria-disabled=move || is_element().then_some("true")
                            on:click=move |_| {
                                if commit_add(w, w.calming_kind_shown(), &typed()) {
                                    focus_later(CalmingRow::STRIP_ID);
                                }
                            }
                        >
                            {move || text("st-calming-add")}
                        </button>
                        <button
                            type="button"
                            id="calming-remove"
                            aria-disabled=move || (!is_element()).then_some("true")
                            on:click=move |_| {
                                if w.remove_calming() {
                                    focus_later(CalmingRow::STRIP_ID);
                                }
                            }
                        >
                            {move || text("st-calming-remove")}
                        </button>
                    </div>
                </div>
            </section>
        </Show>
    }
}
