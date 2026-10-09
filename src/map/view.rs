//! The city map page: views bound to the map's view-model. They show what it
//! says and send it what the person does; nothing here decides anything.

use std::rc::Rc;

use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use leptos::web_sys::Element;
use leptos::web_sys::KeyboardEvent;

#[cfg(target_arch = "wasm32")]
use crate::map::camera::Insets;
use crate::map::vm::{BasemapState, MapVm, NoteItem, PlaceRow, StateTag};
use crate::shared::bind::Bound;
use crate::shared::said::Said;
use crate::shared::tick::Tick;

type Vm = Bound<MapVm>;

/// A message of the page, for a view: it follows a switch of language.
fn word(vm: Vm, key: &'static str) -> impl Fn() -> String + Copy + 'static {
    move || vm.with(|v| v.word(key))
}

#[component]
pub fn MapTools(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    view! {
        <div class="map-tools" role="toolbar" aria-label=word(vm, "map-view-tools")>
            <button type="button" id="zoom-out" class="btn" aria-label=word(vm, "map-zoom-out") on:click=move |_| vm.with(|v| v.zoom_out())>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3.5 8h9"/></svg>
            </button>
            <button type="button" id="zoom-in" class="btn" aria-label=word(vm, "map-zoom-in") on:click=move |_| vm.with(|v| v.zoom_in())>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3.5 8h9M8 3.5v9"/></svg>
            </button>
            <button type="button" id="zoom-fit" class="btn" title=word(vm, "map-whole-city") on:click=move |_| vm.with(|v| v.fit_camera())>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10"/></svg>
                <span class="lbl">{word(vm, "map-whole-city")}</span>
            </button>
        </div>
    }
}

/// Has `observer` watch everything that floats over the map, so it fits again
/// when a panel appears, goes or changes size.
#[cfg(target_arch = "wasm32")]
pub(crate) fn observe_covers(observer: &leptos::web_sys::ResizeObserver) {
    if let Ok(covers) = leptos::prelude::document().query_selector_all("[data-covers]") {
        for i in 0..covers.length() {
            if let Some(c) = covers.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                observer.observe(&c);
            }
        }
    }
}

/// How much of the map's box the floating panels and bars (marked `data-covers`)
/// cover along each edge, plus a gap. Anything that does not overlap the map, as
/// when the panels stack below it on a phone, covers nothing.
///
/// An element can say which edge it covers, `data-covers="left"`, when its size
/// and place do not make that plain.
#[cfg(target_arch = "wasm32")]
pub(crate) fn insets_of(map: &Element) -> Insets {
    const GAP: f64 = 16.0;
    let m = map.get_bounding_client_rect();
    let mut insets = Insets::default();
    let Ok(covers) = leptos::prelude::document().query_selector_all("[data-covers]") else { return insets };
    for i in 0..covers.length() {
        let Some(c) = covers.item(i).and_then(|n| n.dyn_into::<Element>().ok()) else { continue };
        let r = c.get_bounding_client_rect();
        let overlaps = r.width() > 0.0 && r.height() > 0.0 && r.right() > m.left() && r.left() < m.right() && r.bottom() > m.top() && r.top() < m.bottom();
        if !overlaps {
            continue;
        }
        let said = c.get_attribute("data-covers").unwrap_or_default();
        let hugs_left = said == "left" || (said.is_empty() && r.left() - m.left() < GAP * 2.0);
        let hugs_right = said == "right" || (said.is_empty() && m.right() - r.right() < GAP * 2.0);
        if (said == "left" || said == "right") || (r.width() < m.width() * 0.5 && (hugs_left || hugs_right)) {
            // A panel down one side.
            if hugs_left {
                insets.left = insets.left.max(r.right() - m.left() + GAP);
            } else {
                insets.right = insets.right.max(m.right() - r.left() + GAP);
            }
        } else if r.top() + r.height() / 2.0 < m.top() + m.height() / 2.0 {
            insets.top = insets.top.max(r.bottom() - m.top() + GAP / 2.0);
        } else {
            insets.bottom = insets.bottom.max(m.bottom() - r.top() + GAP / 2.0);
        }
    }
    insets
}

/// What binds the page to the map: the keyboard on the map, what floats over it, and keeping the map in step
/// with the city. The map itself is drawn by MapLibre into `#basemap`, which the page holds; this shows only
/// why there is none when there is none.
#[component]
pub fn MapView(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::closure::Closure;
        // The places follow the city and whether the map is ready; the highlight follows the pointer.
        Effect::new(move |_| {
            vm.with(|v| {
                v.view();
                v.basemap_state();
            });
            vm.with(|v| v.sync_places());
        });
        Effect::new(move |_| {
            vm.with(|v| v.hot());
            vm.with(|v| v.sync_highlight());
        });

        Effect::new(move |_| {
            let Some(el) = leptos::prelude::document().get_element_by_id("basemap") else { return };
            let key = Closure::<dyn FnMut(KeyboardEvent)>::new(move |e: KeyboardEvent| {
                if e.meta_key() || e.ctrl_key() || e.alt_key() {
                    return;
                }
                if vm.with(|v| v.press_key(&e.key())) {
                    e.prevent_default();
                }
            });
            let _ = el.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref());
            key.forget();
            // What floats over the map is kept clear of when the places are fitted.
            let fit = {
                let el = el.clone();
                move || {
                    let r = el.get_bounding_client_rect();
                    if r.width() > 0.0 && r.height() > 0.0 {
                        vm.with(|v| v.set_insets(insets_of(&el)));
                    }
                }
            };
            fit();
            let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| fit());
            if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
                o.observe(&el);
                observe_covers(&o);
            }
            observer.forget();
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

    view! {
        <p
            class="basemap-note"
            role="status"
            hidden=move || !matches!(vm.with(|v| v.basemap_state()), BasemapState::Unavailable(_))
        >
            {move || match vm.with(|v| v.basemap_state()) {
                BasemapState::Unavailable(key) => vm.with(|v| v.word(key)),
                _ => String::new(),
            }}
        </p>
    }
}

/// A key to what the colours of the places on the map say.
#[component]
pub fn Legend(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    move || {
        vm.with(|v| v.status_key())
            .into_iter()
            .map(|(id, name)| {
                view! {
                    <li>
                        <span class=format!("dot dot-{id}") aria-hidden="true"></span>
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
    let works = Signal::derive(move || !vm.with(|v| v.status().bad));
    view! {
        <p id="fit" class=move || if works.get() { "fit" } else { "fit bad" } role="status">
            <Tick works=works/>
            {move || vm.with(|v| v.status().text)}
        </p>
    }
}

/// Opens a page of the site.
fn go(href: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = leptos::prelude::window().location().set_href(href);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = href;
}

/// The search box in the header. What is typed finds junctions and streets by name
/// and small print; the arrow keys move down the results, Enter opens the one they
/// are on (or the first), Escape clears, and `/` goes to the box from anywhere.
#[component]
pub fn SearchBox(vm: Rc<MapVm>) -> impl IntoView {
    let vm = Bound::new(vm);
    let input = NodeRef::<leptos::html::Input>::new();
    #[cfg(target_arch = "wasm32")]
    {
        let handle = window_event_listener(leptos::ev::keydown, move |e: KeyboardEvent| {
            let in_a_field =
                e.target().and_then(|t| t.dyn_into::<Element>().ok()).is_some_and(|el| el.closest("input, select, textarea").ok().flatten().is_some());
            if e.key() == "/" && !e.meta_key() && !e.ctrl_key() && !e.alt_key() && !in_a_field {
                e.prevent_default();
                if let Some(i) = input.get_untracked() {
                    let _ = i.focus();
                }
            }
        });
        on_cleanup(move || handle.remove());
    }
    let open = move || vm.with(|v| v.searching());
    let active = move || vm.with(|v| v.active_result());
    let keydown = move |e: KeyboardEvent| match e.key().as_str() {
        "ArrowDown" => {
            e.prevent_default();
            vm.with(|v| v.move_active(1));
        }
        "ArrowUp" => {
            e.prevent_default();
            vm.with(|v| v.move_active(-1));
        }
        "Escape" => {
            if vm.with(|v| v.search_text()).is_empty() {
                if let Some(i) = input.get_untracked() {
                    let _ = i.blur();
                }
            } else {
                e.prevent_default();
                vm.with(|v| v.set_search(""));
            }
        }
        _ => {}
    };
    view! {
        <form
            class="search"
            role="search"
            on:submit=move |e| {
                e.prevent_default();
                if let Some(href) = vm.with(|v| v.chosen_href()) {
                    go(&href);
                }
            }
        >
            <svg class="search-ico" viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" focusable="false"><circle cx="8.5" cy="8.5" r="5.5"/><path d="m12.8 12.8 4 4"/></svg>
            <input
                id="search"
                type="search"
                name="q"
                node_ref=input
                placeholder=word(vm, "map-search-label")
                aria-label=word(vm, "map-search-label")
                role="combobox"
                aria-autocomplete="list"
                aria-controls="search-results"
                aria-expanded=move || open().to_string()
                aria-activedescendant=move || active().map(|i| format!("sr-{i}"))
                autocomplete="off"
                spellcheck="false"
                enterkeyhint="go"
                prop:value=move || vm.with(|v| v.search_text())
                on:input=move |e| vm.with(|v| v.set_search(&event_target_value(&e)))
                on:keydown=keydown
            />
            <kbd class="search-key" aria-hidden="true">"/"</kbd>
            <button type="submit" class="search-go">{word(vm, "map-search-go")}</button>
            <div class="search-pop" hidden=move || !open()>
                <ul id="search-results" role="listbox" aria-label=word(vm, "map-search-results")>
                    {move || vm.with(|v| v.results()).into_iter().take(MapVm::SHOWN).enumerate().map(|(i, r)| result_row(vm, r, i)).collect_view()}
                </ul>
                <p class="search-note" role="status">{move || vm.with(|v| v.search_note())}</p>
            </div>
        </form>
    }
}

/// How a place stands, as a tag in its row, in the language of the page.
fn state_tag(vm: Vm, tag: Option<StateTag>) -> Option<impl IntoView> {
    tag.map(|t| view! { <span class=if t.bad { "st bad" } else { "st" }>{word(vm, t.key)}</span> })
}

/// A place's name and small print, in the language of the page.
fn row_words(vm: Vm, name: Said, sub: Said) -> impl IntoView {
    view! {
        <b>{move || vm.with(|v| v.say(&name))}</b>
        <small>{move || vm.with(|v| v.say(&sub))}</small>
    }
}

fn result_row(vm: Vm, row: PlaceRow, i: usize) -> impl IntoView {
    let on = move || vm.with(|v| v.active_result()) == Some(i);
    let (enter, focus) = (row.hot.clone(), row.hot.clone());
    let words = row_words(vm, row.name, row.sub);
    let tag = state_tag(vm, row.tag);
    view! {
        <li role="option" id=format!("sr-{i}") aria-selected=move || on().to_string()>
            <a
                class=move || if on() { "place-row on" } else { "place-row" }
                href=row.href
                tabindex="-1"
                on:pointerenter=move |_| vm.with(|v| v.set_hot(Some(enter.clone())))
                on:pointerleave=move |_| vm.with(|v| v.set_hot(None))
                on:focus=move |_| vm.with(|v| v.set_hot(Some(focus.clone())))
                on:blur=move |_| vm.with(|v| v.set_hot(None))
            >
                {words}
                {tag}
            </a>
        </li>
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
        <div class="tb-cell tb-wide"><span>{word(vm, "map-tb-city")}</span><b id="tb-street">{move || vm.with(|v| v.title())}</b></div>
        <div class="tb-cell"><span>{word(vm, "map-places")}</span><b id="tb-places" class="fig">{move || vm.with(|v| v.places().to_string())}</b></div>
        <div class="tb-cell"><span>{word(vm, "map-tb-changed")}</span><b id="tb-changes" class="fig">{move || vm.with(|v| v.changes().to_string())}</b></div>
    }
}
