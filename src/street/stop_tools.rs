//! The commands beside the stops rows: the kind, the shelter, the name and the length of the chosen stop, add a stop into
//! the chosen free stretch, and remove the chosen stop. One named group, like the curbside commands; every control is a
//! tab stop.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use super::curb_tools::typed_length;
use super::model::{MIN_STOP_MM, StopKind};
use super::vm::StreetVm;
use super::watch::SheetWatch;
use super::zone_tools::focus_later;
use crate::shared::catalogue::Side;
use crate::shared::i18n::Args;

/// The kind the select shows as a value, and reads back.
fn kind_value(k: StopKind) -> String {
    StopKind::ALL.iter().position(|x| *x == k).unwrap_or(0).to_string()
}

fn kind_of(value: &str) -> StopKind {
    value.parse::<usize>().ok().and_then(|n| StopKind::ALL.get(n).copied()).unwrap_or(StopKind::Curb)
}

/// The cell of the stops rows that is chosen: its side, whether it is a stop (else a gap), and what the stop is.
struct Chosen {
    side: Side,
    from_mm: i32,
    to_mm: i32,
    stop: Option<(StopKind, bool, Option<String>)>,
}

fn chosen(w: SheetWatch) -> Option<Chosen> {
    let v = w.view();
    v.stops.sides.iter().find_map(|s| {
        let c = s.cells.get(s.selected?)?;
        Some(Chosen { side: s.side, from_mm: c.from_mm, to_mm: c.to_mm, stop: c.kind.map(|k| (k, c.shelter, c.name.clone())) })
    })
}

/// The strip to give the focus back to after a command.
fn strip_to_focus(w: SheetWatch) -> Option<&'static str> {
    chosen(w).map(|c| if c.side == Side::Left { "stop-strip-left" } else { "stop-strip-right" })
}

/// Adds a stop of `kind` into the chosen free stretch: the length typed, else the shortest a bus needs. False when the
/// length is not a number (said) or the add was refused (said by the view-model).
pub fn commit_add(w: SheetWatch, kind: StopKind, typed: &str) -> bool {
    typed_length(w, typed, Some(MIN_STOP_MM)).is_some_and(|mm| w.add_stop(kind, mm))
}

/// Gives the chosen stop the length that was typed.
pub fn commit_length(w: SheetWatch, typed: &str) -> bool {
    typed_length(w, typed, None).is_some_and(|mm| w.resize_stop(mm))
}

#[component]
pub fn StopTools(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let text = move |key: &'static str| w.i18n().tr(key, &Args::new());
    let unit_text = move |key: &'static str| w.i18n().tr(key, &Args::new().str("unit", w.units().word()));
    let shown_kind = move || w.stop_kind_shown();
    let is_stop = move || chosen(w).is_some_and(|c| c.stop.is_some());
    let shelter_value = move || chosen(w).and_then(|c| c.stop).is_some_and(|s| s.1);
    let name_value = move || chosen(w).and_then(|c| c.stop).and_then(|s| s.2).unwrap_or_default();
    let length_value = move || chosen(w).filter(|c| c.stop.is_some()).map(|c| w.units().number_in(c.to_mm - c.from_mm, w.i18n().locale())).unwrap_or_default();
    let any = move || !w.view().stops.sides.is_empty();
    let options = move || {
        StopKind::ALL
            .iter()
            .map(|k| {
                let k = *k;
                view! {
                    <option value=kind_value(k)>
                        {move || w.i18n().tr(k.name_key(), &Args::new())}
                    </option>
                }
            })
            .collect_view()
    };
    let typed = move |id: &str| document().get_element_by_id(id).and_then(|e| e.dyn_into::<HtmlInputElement>().ok()).map(|i| i.value()).unwrap_or_default();
    view! {
        <Show when=any>
            <section class="insp-sec" aria-labelledby="h-stop-tools">
                <h3 class="note-h" id="h-stop-tools">{move || text("st-stop-tools")}</h3>
                <div class="zone-tools curb-tools" role="group" aria-label=move || text("st-stop-tools")>
                    <div class="zone-tool">
                        <label for="stop-kind">{move || text("st-stop-kind")}</label>
                        <select
                            id="stop-kind"
                            prop:value=move || kind_value(shown_kind())
                            on:change=move |e| {
                                w.pick_stop_kind(kind_of(&event_target::<HtmlSelectElement>(&e).value()));
                            }
                        >
                            {options}
                        </select>
                    </div>
                    <div class="zone-tool">
                        <label for="stop-name">{move || text("st-stop-name")}</label>
                        <input
                            id="stop-name"
                            type="text"
                            autocomplete="off"
                            prop:value=name_value
                            on:change=move |e| {
                                let input = event_target::<HtmlInputElement>(&e);
                                if !w.name_stop(&input.value()) {
                                    input.set_value(&name_value());
                                }
                            }
                        />
                    </div>
                    <div class="zone-tool">
                        <label for="stop-length">{move || unit_text("st-stop-length")}</label>
                        <input
                            id="stop-length"
                            type="text"
                            inputmode="decimal"
                            autocomplete="off"
                            prop:value=length_value
                            on:keydown=move |e| {
                                if e.key() == "Enter" {
                                    e.prevent_default();
                                    let input = event_target::<HtmlInputElement>(&e);
                                    if is_stop() {
                                        if !commit_length(w, &input.value()) {
                                            input.set_value(&length_value());
                                        }
                                    } else if commit_add(w, shown_kind(), &input.value()) {
                                        input.set_value("");
                                    }
                                }
                            }
                        />
                    </div>
                    <div class="zone-tool zone-check">
                        <input
                            id="stop-shelter"
                            type="checkbox"
                            prop:checked=shelter_value
                            on:change=move |e| {
                                let input = event_target::<HtmlInputElement>(&e);
                                if !w.set_stop_shelter(input.checked()) {
                                    input.set_checked(shelter_value());
                                }
                            }
                        />
                        <label for="stop-shelter">{move || text("st-stop-shelter")}</label>
                    </div>
                    <div class="zone-tool">
                        <button
                            type="button"
                            id="stop-add"
                            aria-disabled=move || chosen(w).is_none_or(|c| c.stop.is_some()).then_some("true")
                            on:click=move |_| {
                                let t = typed("stop-length");
                                if commit_add(w, shown_kind(), &t) {
                                    if let Some(id) = strip_to_focus(w) {
                                        focus_later(id);
                                    }
                                }
                            }
                        >
                            {move || text("st-stop-add")}
                        </button>
                        <button
                            type="button"
                            id="stop-remove"
                            aria-disabled=move || (!is_stop()).then_some("true")
                            on:click=move |_| {
                                let side = strip_to_focus(w);
                                if w.remove_stop() {
                                    if let Some(id) = side {
                                        focus_later(id);
                                    }
                                }
                            }
                        >
                            {move || text("st-stop-remove")}
                        </button>
                    </div>
                </div>
            </section>
        </Show>
    }
}
