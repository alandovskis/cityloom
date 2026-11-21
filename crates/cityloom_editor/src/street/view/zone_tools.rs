//! The commands beside the zone strip: split (at a typed distance, else the middle), the selected zone's start,
//! merge with a neighbour (asking which section to keep when they differ), its name, and the two that act on a zone
//! with a section of its own. One named group (not a toolbar: it holds text fields, which take the arrow keys); every control is a tab stop.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlInputElement;

use crate::shared::i18n::Args;
use crate::shared::said::{say, say_now};
use crate::shared::units::parse_length_mm;
use crate::street::model::Keep;
use crate::street::vm::StreetVm;
use crate::street::vm::watch::SheetWatch;

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
pub(super) fn focus_later(id: &'static str) {
    set_timeout(
        move || {
            if let Some(e) = document().get_element_by_id(id).and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()) {
                let _ = e.focus();
            }
        },
        std::time::Duration::ZERO,
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn focus_later(_id: &'static str) {}

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
            focus_later("plan-point");
        }
        open
    });

    // A merge button names the zone it merges with ("With Zone 1", read as "Merge with Zone 1"); with none on that side
    // it says which side, stays unavailable, and a line under it says why.
    let neighbour = move |next: bool| {
        let v = w.view();
        let at = if next { Some(v.zones.selected + 1) } else { v.zones.selected.checked_sub(1) };
        at.and_then(|i| v.zones.zones.get(i)).map(|z| say(&w.i18n(), w.units(), &z.label_mid))
    };
    let merge_button = move |next: bool| {
        let (id, why_id, key) = if next {
            ("zone-merge-next", "zone-merge-next-why", "st-zone-merge-next")
        } else {
            ("zone-merge-prev", "zone-merge-prev-why", "st-zone-merge-prev")
        };
        let with = move |key: &'static str| neighbour(next).filter(|_| !no_neighbour(next)).map(|zone| w.i18n().tr(key, &Args::new().str("zone", zone)));
        view! {
            <button
                type="button"
                class="btn"
                id=id
                aria-label=move || with("st-zone-merge-with")
                aria-disabled=move || no_neighbour(next).then_some("true")
                aria-describedby=move || no_neighbour(next).then_some(why_id)
                on:click=move |_| {
                    if !no_neighbour(next) && w.begin_merge(next) && w.pending_merge().is_none() {
                        focus_later("plan-point");
                    }
                }
            >
                {move || with("st-zone-with").unwrap_or_else(|| text(key))}
            </button>
        }
    };
    let merge_why = move |next: bool| {
        let why_id = if next { "zone-merge-next-why" } else { "zone-merge-prev-why" };
        view! { <span id=why_id class="zone-why">{move || no_neighbour(next).then(no_neighbour_why)}</span> }
    };
    // Where the zone lies, and a strip of the zones with this one filled (the line says it; the strip is only seen).
    let sub = move || {
        let v = w.view();
        let (units, i18n) = (w.units(), w.i18n());
        let locale = i18n.locale();
        let z = &v.zones;
        let Some(zone) = z.zones.get(z.selected) else { return String::new() };
        if !z.several {
            return i18n.tr("st-zone-sub-whole", &Args::new().str("length", units.length_in(z.length_mm, locale)));
        }
        let args = Args::new()
            .str("from", units.length_in(zone.from_mm, locale))
            .str("to", units.length_in(zone.to_mm, locale))
            .str("length", units.length_in(zone.to_mm - zone.from_mm, locale))
            .num("at", z.selected as i64 + 1)
            .num("total", z.zones.len() as i64);
        i18n.tr("st-zone-sub", &args)
    };
    let name_shown = move || {
        let v = w.view();
        v.zones.zones.get(v.zones.selected).map(|z| say(&w.i18n(), w.units(), &z.label)).unwrap_or_default()
    };
    let mini = move || {
        let v = w.view();
        v.zones
            .zones
            .iter()
            .enumerate()
            .map(|(i, z)| {
                view! { <span class=(i == v.zones.selected).then_some("on") style=format!("flex-grow: {}", (z.to_mm - z.from_mm).max(1))></span> }
            })
            .collect_view()
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

    let row = |label: AnyView, body: AnyView| view! { <div class="zone-row">{label}<div class="zone-c">{body}</div></div> };
    let span_label = move |key: &'static str| view! { <span class="zone-l">{move || text(key)}</span> }.into_any();
    let field_label = move |id: &'static str, key: &'static str| view! { <label for=id class="zone-l">{move || text(key)}</label> }.into_any();

    view! {
        <section class="zone-pane" aria-labelledby="h-zone-tools">
        <div class="insp-head">
            <span class="zone-badge" aria-hidden="true">{move || selected() + 1}</span>
            <div>
                <h2 class="insp-name">{name_shown}</h2>
                <p class="insp-sub">{sub}</p>
            </div>
        </div>
        <div class="zone-mini" aria-hidden="true">{mini}</div>
        <h3 class="sr-only" id="h-zone-tools">{move || text("st-zone-tools")}</h3>
        <div class="zone-tools" role="group" aria-label=move || text("st-zone-tools")>
            {move || {
                w.pending_merge()
                    .map(|_| {
                        view! {
                            <div class="zone-merge-choice" role="group" aria-label=move || text("st-zone-merge-choice")>
                                <p class="zone-ask">{move || text("st-zone-merge-choice")}</p>
                                {keep_button("zone-keep-this", Keep::Selected)}
                                {keep_button("zone-keep-other", Keep::Neighbour)}
                                <button type="button" class="btn" id="zone-keep-cancel" on:click=move |_| w.cancel_merge()>
                                    {move || text("st-zone-keep-cancel")}
                                </button>
                            </div>
                        }
                    })
            }}
            {move || {
                several()
                    .then(|| {
                        row(
                            field_label("zone-name", "st-zone-row-name"),
                            view! {
                                <input
                                    id="zone-name"
                                    type="text"
                                    autocomplete="off"
                                    aria-label=move || text("st-zone-name")
                                    placeholder=move || {
                                        let v = w.view();
                                        v.zones.zones.get(v.zones.selected).map(|z| say(&w.i18n(), w.units(), &z.label)).unwrap_or_default()
                                    }
                                    prop:value=name_value
                                    on:keydown=move |e| {
                                        if e.key() == "Enter" {
                                            e.prevent_default();
                                            apply_name(event_target::<HtmlInputElement>(&e));
                                        }
                                    }
                                    on:blur=move |e| apply_name(event_target::<HtmlInputElement>(&e))
                                />
                            }
                            .into_any(),
                        )
                    })
            }}
            {move || {
                (several() && selected() > 0)
                    .then(|| {
                        row(
                            field_label("zone-start", "st-zone-row-start"),
                            view! {
                                <span class="zone-field">
                                    <input
                                        id="zone-start"
                                        type="text"
                                        inputmode="decimal"
                                        autocomplete="off"
                                        aria-label=move || unit_text("st-zone-start")
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
                                    <span class="unit-tag" aria-hidden="true">{move || w.units().word()}</span>
                                </span>
                            }
                            .into_any(),
                        )
                    })
            }}
            {move || {
                several()
                    .then(|| {
                        row(
                            span_label("st-zone-row-section"),
                            view! {
                                <span class=move || if own() { "zone-chip own" } else { "zone-chip" }>
                                    {move || text(if own() { "st-zone-own" } else { "st-zone-same" })}
                                </span>
                                {move || {
                                    own()
                                        .then(|| {
                                            view! {
                                                <div class="zone-btns">
                                                    <button type="button" class="btn" id="zone-reset" on:click=move |_| { w.reset_zone(); }>
                                                        {move || text("st-zone-reset")}
                                                    </button>
                                                    <button type="button" class="btn" id="zone-use" on:click=move |_| { w.use_for_street(); }>
                                                        {move || text("st-zone-use")}
                                                    </button>
                                                </div>
                                            }
                                        })
                                }}
                            }
                            .into_any(),
                        )
                    })
            }}
            {row(
                field_label("zone-split-at", "st-zone-row-split"),
                view! {
                    <div class="zone-btns">
                        <span class="zone-field">
                            <input
                                id="zone-split-at"
                                type="text"
                                inputmode="decimal"
                                autocomplete="off"
                                aria-label=move || unit_text("st-zone-split-at")
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
                            <span class="unit-tag" aria-hidden="true">{move || w.units().word()}</span>
                        </span>
                        <button
                            type="button"
                            class=move || if several() { "btn" } else { "btn primary" }
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
                    </div>
                    {move || split_why().map(|why| view! { <p id="zone-split-why" class="zone-why">{why}</p> })}
                    {move || (!several() && split_why().is_none()).then(|| view! { <p class="zone-hint">{move || text("st-zone-split-hint")}</p> })}
                }
                .into_any(),
            )}
            {move || {
                several()
                    .then(|| {
                        row(
                            span_label("st-zone-row-merge"),
                            view! {
                                <div class="zone-btns">{merge_button(false)} {merge_button(true)}</div>
                                {merge_why(false)}
                                {merge_why(true)}
                            }
                            .into_any(),
                        )
                    })
            }}
        </div>
        </section>
    }
}
