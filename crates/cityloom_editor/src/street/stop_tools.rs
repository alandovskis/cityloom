//! The commands beside the stops rows: the shelter, the name and the length of the chosen stop (on a free
//! stretch, the shelter and name of the stop the Add a piece menu will add there), and remove the chosen stop. The kind is
//! chosen in that menu, when the stop is added. One named group, like the curbside commands; every control is a
//! tab stop.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::HtmlInputElement;

use super::curb_tools::typed_length;
use super::vm::StreetVm;
use super::watch::SheetWatch;
use super::zone_tools::focus_later;
use crate::shared::catalogue::Side;
use crate::shared::i18n::Args;

/// The cell of the stops rows that is chosen: its side and whether it is a stop (else a gap).
struct Chosen {
    side: Side,
    from_mm: i32,
    to_mm: i32,
    is_stop: bool,
}

fn chosen(w: SheetWatch) -> Option<Chosen> {
    let v = w.view();
    v.stops.sides.iter().find_map(|s| {
        let c = s.cells.get(s.selected?)?;
        Some(Chosen { side: s.side, from_mm: c.from_mm, to_mm: c.to_mm, is_stop: c.kind.is_some() })
    })
}

/// The strip to give the focus back to after a command.
fn strip_to_focus(w: SheetWatch) -> Option<&'static str> {
    chosen(w).map(|c| if c.side == Side::Left { "stop-strip-left" } else { "stop-strip-right" })
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
    let is_stop = move || chosen(w).is_some_and(|c| c.is_stop);
    let shelter_value = move || w.stop_shelter_shown();
    let name_value = move || w.stop_name_shown();
    let length_value = move || chosen(w).filter(|c| c.is_stop).map(|c| w.units().number_in(c.to_mm - c.from_mm, w.i18n().locale())).unwrap_or_default();
    let any = move || !w.view().stops.sides.is_empty();
    view! {
        <Show when=any>
            <section class="insp-sec" aria-labelledby="h-stop-tools">
                <h3 class="note-h" id="h-stop-tools">{move || text("st-stop-tools")}</h3>
                <div class="zone-tools curb-tools" role="group" aria-label=move || text("st-stop-tools")>
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
                                    if is_stop() && !commit_length(w, &input.value()) {
                                        input.set_value(&length_value());
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
