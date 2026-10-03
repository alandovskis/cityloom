//! The city map page: views bound to the map's view-model. They show what it
//! says and send it what the person does; nothing here decides anything.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{Element, KeyboardEvent, PointerEvent, WheelEvent};

use crate::shared::bind::Bound;
use crate::ui::map_svg::{hot_layer, map_svg, overlay_svg};
use crate::vm::map::{MapVm, NoteItem, PlaceRow, ResetOutcome};

type Vm = Bound<MapVm>;

/// How long a first press of "start over" waits to be pressed again, in milliseconds.
#[cfg(target_arch = "wasm32")]
const ARMED_MS: u64 = 4000;

#[component]
pub fn MapHeader(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <h1 id="street-name">{move || vm.with(|v| v.title())}</h1>
        <p class="street-sub" id="street-sub">
            <span>"City map"</span>
            " "
            <span aria-hidden="true">"\u{b7}"</span>
            " "
            <span id="city-count" class="fig">{move || vm.with(|v| v.counts())}</span>
        </p>
    }
}

#[component]
pub fn ResetButton(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    #[cfg(target_arch = "wasm32")]
    let timer = StoredValue::new_local(None::<leptos::leptos_dom::helpers::TimeoutHandle>);
    let press = move |_| {
        let outcome = vm.with(|v| v.press_reset());
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(h) = timer.try_update_value(|t| t.take()).flatten() {
                h.clear();
            }
            if outcome == ResetOutcome::Armed {
                let h = set_timeout_with_handle(move || vm.with(|v| v.disarm()), std::time::Duration::from_millis(ARMED_MS)).ok();
                timer.set_value(h);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = outcome == ResetOutcome::Armed;
    };
    view! {
        <div class="btns" role="toolbar" aria-label="Sheet tools">
            <button type="button" id="reset" class="btn" disabled=move || !vm.with(|v| v.can_reset()) on:click=press on:blur=move |_| vm.with(|v| v.disarm())>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M2.5 8a5.5 5.5 0 1 0 1.8-4.1M2.5 2.5v3h3"/></svg>
                <span id="reset-label">{move || vm.with(|v| v.reset_label())}</span>
            </button>
        </div>
    }
}

#[component]
pub fn MapTools(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <div class="map-tools" role="toolbar" aria-label="Map view">
            <button type="button" id="zoom-out" class="btn" aria-label="Zoom out" on:click=move |_| vm.with(|v| v.zoom_out())>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3.5 8h9"/></svg>
            </button>
            <button type="button" id="zoom-in" class="btn" aria-label="Zoom in" on:click=move |_| vm.with(|v| v.zoom_in())>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3.5 8h9M8 3.5v9"/></svg>
            </button>
            <button type="button" id="zoom-fit" class="btn" on:click=move |_| vm.with(|v| v.fit_camera())>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10"/></svg>
                "Whole city"
            </button>
            <p class="map-hint">"Press a junction to open its plan. Press a street to open its cross-section."</p>
        </div>
    }
}

/// The place a pointer or the focus is on, from the nearest thing under it that names one.
fn hot_of(target: Option<leptos::web_sys::EventTarget>) -> Option<String> {
    target.and_then(|t| t.dyn_into::<Element>().ok()).and_then(|t| t.closest("[data-hl]").ok().flatten()).and_then(|t| t.get_attribute("data-hl"))
}

/// The window the map fills, and the point at its middle, in page pixels.
fn window_of(el: &Element) -> (f64, f64, (f64, f64)) {
    let r = el.get_bounding_client_rect();
    (r.width(), r.height(), (r.left() + r.width() / 2.0, r.top() + r.height() / 2.0))
}

#[component]
pub fn MapView(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    let frame = NodeRef::<leptos::html::Div>::new();
    // Widths that stay a few pixels wide are set in metres, so a new zoom draws again.
    let k = Memo::new(move |_| vm.with(|v| v.camera().k));
    let places = move || vm.with(|v| map_svg(&v.view(), k.get(), v.units()));
    let highlight = move || vm.with(|v| hot_layer(&v.view(), k.get(), v.hot().as_deref()));
    let overlay = move || vm.with(|v| overlay_svg(&v.scale_bar(), v.camera().height));

    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::closure::Closure;
        Effect::new(move |_| {
            let Some(el) = frame.get() else { return };
            let el: Element = el.into();
            let (w, h, _) = window_of(&el);
            vm.with(|v| v.resize(w, h));
            let target = el.clone();
            let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| {
                let (w, h, _) = window_of(&target);
                if w > 0.0 && h > 0.0 {
                    vm.with(|v| v.resize(w, h));
                }
            });
            if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
                o.observe(&el);
            }
            observer.forget();
            // A drag that ends over a place must not open it.
            let swallow = Closure::<dyn FnMut(leptos::web_sys::Event)>::new(move |e: leptos::web_sys::Event| {
                if vm.with(|v| v.swallow_click()) {
                    e.prevent_default();
                    e.stop_propagation();
                }
            });
            let _ = el.add_event_listener_with_callback_and_bool("click", swallow.as_ref().unchecked_ref(), true);
            swallow.forget();
        });
        // What the other pages wrote is read again when the page is shown, and when another tab writes.
        window_event_listener(leptos::ev::storage, move |e: leptos::web_sys::StorageEvent| {
            if e.key().is_none_or(|k| k == crate::city::store::CITY_KEY) {
                vm.with(|v| v.reload());
            }
        });
        window_event_listener(leptos::ev::pageshow, move |e: leptos::web_sys::PageTransitionEvent| {
            if e.persisted() {
                vm.with(|v| v.reload());
            }
        });
    }

    let down = move |e: PointerEvent| {
        if e.pointer_type() == "mouse" && e.button() != 0 {
            return;
        }
        vm.with(|v| v.pointer_down(e.pointer_id(), e.client_x() as f64, e.client_y() as f64));
    };
    let moved = move |e: PointerEvent| {
        let Some(el) = frame.get_untracked() else { return };
        let el: Element = el.into();
        let (_, _, origin) = window_of(&el);
        if vm.with(|v| v.pointer_move(e.pointer_id(), e.client_x() as f64, e.client_y() as f64, origin)) {
            let _ = el.set_pointer_capture(e.pointer_id());
        }
    };
    let up = move |e: PointerEvent| vm.with(|v| v.pointer_up(e.pointer_id()));
    let wheel = move |e: WheelEvent| {
        e.prevent_default();
        let Some(el) = frame.get_untracked() else { return };
        let (_, _, origin) = window_of(&Element::from(el));
        vm.with(|v| v.wheel(e.delta_y(), e.ctrl_key(), e.client_x() as f64 - origin.0, e.client_y() as f64 - origin.1));
    };
    let key = move |e: KeyboardEvent| {
        if e.meta_key() || e.ctrl_key() || e.alt_key() {
            return;
        }
        if vm.with(|v| v.press_key(&e.key())) {
            e.prevent_default();
        }
    };
    let hot_pointer = move |e: PointerEvent| {
        let h = hot_of(e.target());
        vm.with(|v| v.set_hot(h));
    };
    let hot_focus = move |e: leptos::web_sys::FocusEvent| {
        let h = hot_of(e.target());
        vm.with(|v| v.set_hot(h));
    };
    view! {
        <div
            class=move || if vm.with(|v| v.panning()) { "map-view panning" } else { "map-view" }
            id="map-view"
            tabindex="0"
            role="group"
            aria-label="Map of the city, north up"
            aria-describedby="keys"
            node_ref=frame
            on:pointerdown=down
            on:pointermove=moved
            on:pointerup=up
            on:pointercancel=up
            on:wheel=wheel
            on:keydown=key
            on:pointerover=hot_pointer
            on:pointerleave=move |_| vm.with(|v| v.set_hot(None))
            on:focusin=hot_focus
            on:focusout=move |_| vm.with(|v| v.set_hot(None))
        >
            <svg id="map" xmlns="http://www.w3.org/2000/svg" viewBox=move || vm.with(|v| v.view_box())>
                <g inner_html=highlight></g>
                <g inner_html=places></g>
            </svg>
            <svg class="map-overlay" id="map-overlay" aria-hidden="true" focusable="false" inner_html=overlay></svg>
        </div>
    }
}

/// A key to the strips: the tint and hatch of each kind of piece in the streets.
#[component]
pub fn Legend(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || {
        vm.with(|v| v.legend())
            .into_iter()
            .map(|(id, name)| {
                view! {
                    <li>
                        <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">
                            <rect class=format!("k-{id}") width="44" height="22" stroke="none"/>
                            <rect width="44" height="22" fill=format!("url(#h-{id})") stroke="none"/>
                        </svg>
                        {name}
                    </li>
                }
            })
            .collect_view()
    }
}

/// Whether the city works, in one line.
#[component]
pub fn Status(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <p id="fit" class=move || if vm.with(|v| v.status().bad) { "fit bad" } else { "fit" } role="status">
            {move || vm.with(|v| v.status().text)}
        </p>
    }
}

fn place_row(vm: Vm, row: PlaceRow) -> impl IntoView {
    let on = {
        let hot = row.hot.clone();
        move || vm.with(|v| v.hot().as_deref() == Some(hot.as_str()))
    };
    let (enter, focus) = (row.hot.clone(), row.hot.clone());
    let tag = row.tag.map(|t| view! { <span class=if t.bad { "st bad" } else { "st" }>{t.text}</span> });
    view! {
        <li>
            <a
                class=move || if on() { "place-row on" } else { "place-row" }
                href=row.href
                data-hl=row.hot
                on:pointerenter=move |_| vm.with(|v| v.set_hot(Some(enter.clone())))
                on:pointerleave=move |_| vm.with(|v| v.set_hot(None))
                on:focus=move |_| vm.with(|v| v.set_hot(Some(focus.clone())))
                on:blur=move |_| vm.with(|v| v.set_hot(None))
            >
                <b>{row.name}</b>
                <small>{row.sub}</small>
                {tag}
            </a>
        </li>
    }
}

/// The places in the city to open: the junctions, then the streets.
#[component]
pub fn Places(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <div class="insp-head">
            <div><h2 class="insp-name">"Places"</h2><p class="insp-sub">{move || vm.with(|v| v.counts())}</p></div>
        </div>
        <section class="insp-sec">
            <h3 class="note-h">"Junctions"</h3>
            <ul class="places">{move || vm.with(|v| v.junction_rows()).into_iter().map(|r| place_row(vm, r)).collect_view()}</ul>
        </section>
        <section class="insp-sec">
            <h3 class="note-h">"Streets"</h3>
            <ul class="places">{move || vm.with(|v| v.street_rows()).into_iter().map(|r| place_row(vm, r)).collect_view()}</ul>
        </section>
    }
}

fn note_item(i: NoteItem, class: &'static str, icon: AnyView) -> impl IntoView {
    view! {
        <li class=class>
            {icon}
            <div><b><a href=i.href>{i.name}</a></b><span>{i.detail}</span></div>
        </li>
    }
}

fn bad_icon() -> AnyView {
    view! { <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 3l10 10M13 3 3 13"/></svg> }.into_any()
}

fn changed_icon() -> AnyView {
    view! { <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="4.5" y="4.5" width="7" height="7"/></svg> }.into_any()
}

#[component]
pub fn ChecksLead(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || vm.with(|v| v.checks_lead())
}

#[component]
pub fn Checks(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || vm.with(|v| v.failing_items()).into_iter().map(|i| note_item(i, "bad", bad_icon())).collect_view()
}

#[component]
pub fn ChangesLead(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || vm.with(|v| v.changes_lead())
}

#[component]
pub fn Changes(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || vm.with(|v| v.changed_items()).into_iter().map(|i| note_item(i, "", changed_icon())).collect_view()
}

#[component]
pub fn TitleBlock(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <div class="tb-cell tb-wide"><span>"City"</span><b id="tb-street">{move || vm.with(|v| v.title())}</b></div>
        <div class="tb-cell"><span>"Places"</span><b id="tb-places" class="fig">{move || vm.with(|v| v.places().to_string())}</b></div>
        <div class="tb-cell"><span>"Changed"</span><b id="tb-changes" class="fig">{move || vm.with(|v| v.changes().to_string())}</b></div>
    }
}
