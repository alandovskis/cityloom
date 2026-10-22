//! The commands beside the zone strip: split (at a typed distance, else the middle), the selected zone's start,
//! merge with a neighbour (asking which section to keep when they differ), its name, and the two that act on a zone
//! with a section of its own. One named group (not a toolbar: it holds text fields, which take the arrow keys); every control is a tab stop.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlInputElement;

use super::model::Keep;
use super::vm::StreetVm;
use super::watch::SheetWatch;
use crate::shared::i18n::Args;
use crate::shared::said::{say, say_now};
use crate::shared::units::parse_length_mm;

/// Splits at what was typed, or in the middle of the selected zone when the field is empty. False when it is not a
/// number (said) or the split was refused (said by the view-model).
pub fn commit_split(w: SheetWatch, typed: &str) -> bool {
    if typed.trim().is_empty() {
        return w.split_zone(None);
    }
    match parse_length_mm(typed, w.units_now()) {
        Some(mm) => w.split_zone(Some(mm)),
        None => {
            w.say_bad_number();
            false
        }
    }
}

/// Moves the selected zone's start to what was typed.
pub fn commit_start(w: SheetWatch, typed: &str) -> bool {
    match parse_length_mm(typed, w.units_now()) {
        Some(mm) => w.move_boundary(mm),
        None => {
            w.say_bad_number();
            false
        }
    }
}

/// Puts the focus on the element, once the page has drawn what was just changed.
#[cfg(target_arch = "wasm32")]
fn focus_later(id: &'static str) {
    set_timeout(
        move || {
            if let Some(e) = document().get_element_by_id(id).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
                let _ = e.focus();
            }
        },
        std::time::Duration::ZERO,
    );
}

#[component]
pub fn ZoneTools(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let text = move |key: &'static str| w.i18n().tr(key, &Args::new());
    let unit_text = move |key: &'static str| w.i18n().tr(key, &Args::new().str("unit", w.units().word()));
    let selected = move || w.view().zones.selected;
    let several = move || w.view().zones.several;
    let own = move || w.view().zones.zones.get(selected()).is_some_and(|z| z.own);
    let split_why = move || w.view().zones.split.as_ref().map(|s| say(&w.i18n(), w.units(), s));
    let no_neighbour = move |next: bool| {
        let v = w.view();
        !if next { v.zones.can_merge_next } else { v.zones.can_merge_prev }
    };
    let no_neighbour_why = move || text("zone-refused-no-neighbour");

    let start_value = move || {
        let v = w.view();
        v.zones.zones.get(v.zones.selected).map(|z| w.units().number_in(z.from_mm, w.i18n().locale())).unwrap_or_default()
    };
    // What the selected zone is called when a person named it; empty for the number it is given otherwise.
    let name_value = move || {
        let v = w.view();
        v.zones.zones.get(v.zones.selected).filter(|z| z.named).map(|z| say(&w.i18n(), w.units(), &z.label)).unwrap_or_default()
    };
    let name_now = move || {
        let v = w.view_now();
        v.zones.zones.get(v.zones.selected).filter(|z| z.named).map(|z| say_now(&w.i18n(), w.units_now(), &z.label)).unwrap_or_default()
    };
    // Applies what was typed; the field then shows the name the zone has, so text that was not taken (a refusal, which
    // is said) is not left looking as if it had been.
    let apply_name = move |input: HtmlInputElement| {
        let typed = input.value();
        if typed.trim() != name_now() {
            w.name_zone(&typed);
        }
        input.set_value(&name_now());
    };

    // The choice takes the focus when it opens and gives it back to the strip when it closes.
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |was_open: Option<bool>| {
        let open = w.pending_merge().is_some();
        if open {
            focus_later("zone-keep-this");
        } else if was_open == Some(true) {
            focus_later("zone-strip");
        }
        open
    });

    let merge_button = move |next: bool| {
        let (id, why_id, key) = if next {
            ("zone-merge-next", "zone-merge-next-why", "st-zone-merge-next")
        } else {
            ("zone-merge-prev", "zone-merge-prev-why", "st-zone-merge-prev")
        };
        view! {
            <button
                type="button"
                id=id
                aria-disabled=move || no_neighbour(next).then_some("true")
                aria-describedby=move || no_neighbour(next).then_some(why_id)
                on:click=move |_| {
                    if !no_neighbour(next) {
                        w.begin_merge(next);
                    }
                }
            >
                {move || text(key)}
            </button>
            <span id=why_id class="sr-only">{move || no_neighbour(next).then(no_neighbour_why)}</span>
        }
    };
    let keep_button = move |id: &'static str, keep: Keep| {
        let name = move || {
            let v = w.view();
            let at = match (keep, w.pending_merge()) {
                (Keep::Selected, _) => Some(v.zones.selected),
                (Keep::Neighbour, Some(true)) => Some(v.zones.selected + 1),
                (Keep::Neighbour, _) => v.zones.selected.checked_sub(1),
            };
            let zone = at.and_then(|i| v.zones.zones.get(i)).map(|z| say(&w.i18n(), w.units(), &z.label_mid)).unwrap_or_default();
            w.i18n().tr("st-zone-keep", &Args::new().str("zone", zone))
        };
        view! { <button type="button" id=id on:click=move |_| { w.finish_merge(keep); }>{name}</button> }
    };

    view! {
        <section class="insp-sec" aria-labelledby="h-zone-tools">
        <h3 class="note-h" id="h-zone-tools">{move || text("st-zone-tools")}</h3>
        <div class="zone-tools" role="group" aria-label=move || text("st-zone-tools")>
            <div class="zone-tool">
                <label for="zone-split-at">{move || unit_text("st-zone-split-at")}</label>
                <input
                    id="zone-split-at"
                    type="text"
                    inputmode="decimal"
                    autocomplete="off"
                    on:keydown=move |e| {
                        if e.key() == "Enter" {
                            e.prevent_default();
                            let input = event_target::<HtmlInputElement>(&e);
                            if commit_split(w, &input.value()) {
                                input.set_value("");
                            }
                        }
                    }
                />
                <button
                    type="button"
                    id="zone-split"
                    aria-disabled=move || split_why().map(|_| "true")
                    aria-describedby=move || split_why().map(|_| "zone-split-why")
                    on:click=move |_| {
                        let typed = document()
                            .get_element_by_id("zone-split-at")
                            .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                            .map(|i| i.value())
                            .unwrap_or_default();
                        commit_split(w, &typed);
                    }
                >
                    {move || text("st-zone-split")}
                </button>
                {move || split_why().map(|why| view! { <p id="zone-split-why" class="zone-why">{why}</p> })}
            </div>
            {move || {
                (several() && selected() > 0)
                    .then(|| {
                        view! {
                            <div class="zone-tool">
                                <label for="zone-start">{move || unit_text("st-zone-start")}</label>
                                <input
                                    id="zone-start"
                                    type="text"
                                    inputmode="decimal"
                                    autocomplete="off"
                                    prop:value=start_value
                                    on:keydown=move |e| {
                                        if e.key() == "Enter" {
                                            e.prevent_default();
                                            let input = event_target::<HtmlInputElement>(&e);
                                            if !commit_start(w, &input.value()) {
                                                input.set_value(&start_value());
                                            }
                                        }
                                    }
                                />
                            </div>
                        }
                    })
            }}
            {move || several().then(|| view! { <div class="zone-tool">{merge_button(false)} {merge_button(true)}</div> })}
            {move || {
                w.pending_merge()
                    .map(|_| {
                        view! {
                            <div class="zone-tool zone-merge-choice" role="group" aria-label=move || text("st-zone-merge-choice")>
                                {keep_button("zone-keep-this", Keep::Selected)}
                                {keep_button("zone-keep-other", Keep::Neighbour)}
                                <button type="button" id="zone-keep-cancel" on:click=move |_| w.cancel_merge()>
                                    {move || text("st-zone-keep-cancel")}
                                </button>
                            </div>
                        }
                    })
            }}
            {move || {
                several()
                    .then(|| {
                        view! {
                            <div class="zone-tool">
                                <label for="zone-name">{move || text("st-zone-name")}</label>
                                <input
                                    id="zone-name"
                                    type="text"
                                    autocomplete="off"
                                    prop:value=name_value
                                    on:keydown=move |e| {
                                        if e.key() == "Enter" {
                                            e.prevent_default();
                                            apply_name(event_target::<HtmlInputElement>(&e));
                                        }
                                    }
                                    on:blur=move |e| apply_name(event_target::<HtmlInputElement>(&e))
                                />
                            </div>
                        }
                    })
            }}
            {move || {
                own()
                    .then(|| {
                        view! {
                            <div class="zone-tool">
                                <button type="button" id="zone-reset" on:click=move |_| { w.reset_zone(); }>{move || text("st-zone-reset")}</button>
                                <button type="button" id="zone-use" on:click=move |_| { w.use_for_street(); }>{move || text("st-zone-use")}</button>
                            </div>
                        }
                    })
            }}
        </div>
        </section>
    }
}
