//! The street's drawing: the street in 2.5D, drawn by `iso_svg`. A press on a piece selects it and brings up its
//! details, where its width and kind are edited (a perspective has no handles to drag), and the arrows, plus and
//! minus, Enter and Delete edit the street by key.

pub mod calming_strip;
pub mod calming_tools;
pub mod curb_strip;
pub mod curb_tools;
pub mod drawing;
pub mod green_strip;
pub mod green_tools;
pub mod inspector;
pub mod iso;
pub mod notes;
pub mod notes_tray;
pub mod page;
pub mod panel_path;
pub mod piece_list;
pub mod speed_view;
pub mod stop_tools;
pub mod strip;
#[cfg(test)]
mod tests;
pub mod zone_tools;

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{Element, HtmlElement, KeyboardEvent, MouseEvent};

use crate::shared::i18n::Args;
use crate::street::view::iso::{fit, iso_label_at, iso_svg, max_height};
use crate::street::vm::StreetVm;
use crate::street::vm::keys::{self, Action};
use crate::street::vm::watch::SheetWatch;

#[component]
pub fn StreetDrawing(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let width = RwSignal::new(1050.0_f64);
    // The height the column under the title leaves the drawing (none limits it on a phone, where the page scrolls):
    // the drawing is laid out no wider than lets it fit, so it is seen whole.
    let room = RwSignal::new(f64::INFINITY);
    // The width the drawing is laid out for, and the scale it is drawn at (`iso::fit`).
    let fitted = move || fit(width.get(), room.get());
    let drawn = move || fitted().0;
    let scale = move || fitted().1;
    let wrap = NodeRef::<leptos::html::Div>::new();

    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if let Some(el) = wrap.get() {
                watch_size(el.into(), width, room);
            }
        });
        // Control or Command with Z or Y undoes and redoes, wherever the keyboard is.
        window_event_listener(leptos::ev::keydown, move |e: KeyboardEvent| {
            use crate::shared::shortcut::{Shortcut, shortcut};
            if !(e.meta_key() || e.ctrl_key()) || typing(&e) {
                return;
            }
            match shortcut(&e.key(), e.shift_key()) {
                Some(Shortcut::Undo) => {
                    e.prevent_default();
                    w.undo();
                }
                Some(Shortcut::Redo) => {
                    e.prevent_default();
                    w.redo();
                }
                None => {}
            }
        });
    }

    // Made again when the street, the width or the language changes: its names are asked with the tracked `tr`,
    // so the memo follows a switch.
    let iso = Memo::new(move |_| {
        let (v, _) = w.now();
        iso_svg(&v, &w.i18n(), drawn())
    });

    let keydown = move |e: KeyboardEvent| {
        if e.meta_key() || e.ctrl_key() || e.alt_key() {
            return;
        }
        let selected = w.view_now().selected.is_some();
        let Some((action, prevent)) = keys::on_section(&e.key(), e.shift_key(), selected) else { return };
        if prevent {
            e.prevent_default();
        }
        match action {
            Action::EditWidth => edit_width(),
            other => w.apply(other),
        }
    };

    // A press on a piece selects it and brings up its details. It is the click, the end of a press that did not turn into
    // a scroll (`touch-action` leaves the page's panning to the browser), so a swipe over the drawing selects nothing.
    // The press targets (`iso-hit`, the last layer) carry the uids and take the pointer.
    let pick = move |e: MouseEvent| {
        focus_wrap(wrap);
        let hit = e.target().and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()).and_then(|t| t.closest("[data-uid]").ok().flatten());
        if let Some(uid) = hit.and_then(|h| h.get_attribute("data-uid")).and_then(|v| v.parse::<u32>().ok()) {
            w.select(Some(uid));
            show_inspector();
        }
    };

    view! {
        <div
            class="wrap views iso-view"
            id="wrap"
            tabindex="0"
            role="group"
            aria-label=move || w.i18n().tr("view-iso-label", &Args::new())
            aria-describedby="keys"
            node_ref=wrap
            on:keydown=keydown
            // The group keeps the height the drawing is held to (`iso::max_height`, at its scale), and the drawing,
            // which is as wide as the group unless it is fitted to the column's height, is centred in it.
            style:min-height=move || format!("{}px", (max_height(drawn()) * scale()).round())
        >
            <svg
                id="drawing"
                class="iso"
                xmlns="http://www.w3.org/2000/svg"
                on:click=pick
                role="img"
                aria-label=move || {
                    let (v, units) = w.now();
                    let i18n = w.i18n();
                    i18n.locale();
                    iso_label_at(&v, &i18n, units, drawn())
                }
                // Drawn at its scale: the view box keeps the size it was laid out at, so the picture is scaled whole.
                width=move || iso.with(|s| (s.width * scale()).round().to_string())
                height=move || iso.with(|s| (s.height * scale()).round().to_string())
                viewBox=move || iso.with(|s| format!("0 0 {} {}", s.width, s.height))
                inner_html=move || iso.with(|s| s.markup.clone())
            ></svg>
        </div>
    }
}

/// Puts the keyboard on the drawing, as pressing it does.
fn focus_wrap(wrap: NodeRef<leptos::html::Div>) {
    if let Some(el) = wrap.get_untracked() {
        let options = leptos::web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = HtmlElement::from(el).focus_with_options(&options);
    }
}

/// Goes to the width field, opening the details if they are hidden.
fn edit_width() {
    let doc = document();
    if doc.document_element().is_some_and(|r| r.get_attribute("data-inspector").as_deref() == Some("closed"))
        && let Some(toggle) = doc.get_element_by_id("inspector-toggle")
    {
        leptos::wasm_bindgen::JsCast::unchecked_into::<HtmlElement>(toggle).click();
    }
    if let Some(field) = doc.get_element_by_id("width") {
        let field: HtmlElement = leptos::wasm_bindgen::JsCast::unchecked_into(field);
        let _ = field.focus();
        if let Some(input) = leptos::wasm_bindgen::JsCast::dyn_ref::<leptos::web_sys::HtmlInputElement>(&field) {
            input.select();
        }
    }
}

/// Stacked under 1100px the inspector sits below the street. A tap on a piece
/// brings it into view.
fn show_inspector() {
    #[cfg(target_arch = "wasm32")]
    {
        let win = leptos::web_sys::window();
        let matches = |q: &str| win.as_ref().and_then(|w| w.match_media(q).ok().flatten()).is_some_and(|m| m.matches());
        let doc = document();
        let closed = doc.document_element().is_some_and(|r| r.get_attribute("data-inspector").as_deref() == Some("closed"));
        if matches("(max-width: 1100px)") && !closed {
            if let Some(el) = doc.get_element_by_id("inspector") {
                let o = leptos::web_sys::ScrollIntoViewOptions::new();
                o.set_block(leptos::web_sys::ScrollLogicalPosition::Start);
                o.set_behavior(if matches("(prefers-reduced-motion: reduce)") {
                    leptos::web_sys::ScrollBehavior::Auto
                } else {
                    leptos::web_sys::ScrollBehavior::Smooth
                });
                el.scroll_into_view_with_scroll_into_view_options(&o);
            }
        }
    }
}

/// Typing in a field, or in text that can be edited: the page's keys are not for it.
#[cfg(target_arch = "wasm32")]
fn typing(e: &KeyboardEvent) -> bool {
    let Some(t) = e.target().and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()) else { return false };
    t.closest("input, select, textarea").ok().flatten().is_some()
        || leptos::wasm_bindgen::JsCast::dyn_ref::<HtmlElement>(&t).is_some_and(|h| h.is_content_editable())
}

/// Keeps `width` as wide as the drawing's group, and `room` as high as the street page's column under the title leaves
/// the drawing: the scroller's height less what else the drawing's card holds (the welcome, the clock, its padding).
/// Where that column does not scroll (a phone: the page scrolls), nothing limits the drawing's height.
#[cfg(target_arch = "wasm32")]
fn watch_size(wrap: Element, width: RwSignal<f64>, room: RwSignal<f64>) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};
    use leptos::web_sys::HtmlElement;

    let column = wrap.closest(".stage-scroll").ok().flatten();
    let card = wrap.closest(".drawing").ok().flatten();
    // The group is watched as well as its scroller: its own width can change with no change to the scroller's.
    let update = {
        let (wrap, column, card) = (wrap.clone(), column.clone(), card.clone());
        move || {
            let now = wrap.client_width() as f64;
            if (width.get_untracked() - now).abs() >= 0.5 {
                width.set(now);
            }
            let scrolls = |el: &Element| {
                leptos::prelude::window()
                    .get_computed_style(el)
                    .ok()
                    .flatten()
                    .and_then(|s| s.get_property_value("overflow-y").ok())
                    .is_some_and(|o| o == "auto" || o == "scroll")
            };
            let high = |el: &Element| el.dyn_ref::<HtmlElement>().map_or(0.0, |h| h.offset_height() as f64);
            let left = match (&column, &card) {
                (Some(column), Some(card)) if scrolls(column) => {
                    // the column's own padding, top and bottom (`web/editor.css`, `.stage-scroll`)
                    column.client_height() as f64 - (high(card) - high(&wrap)) - 4.0
                }
                _ => f64::INFINITY,
            };
            if room.get_untracked() != left {
                room.set(left);
            }
        }
    };
    update();
    let on_resize = update.clone();
    let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| on_resize());
    if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
        if let Some(scroll) = wrap.parent_element() {
            o.observe(&scroll);
        }
        o.observe(&wrap);
        // The column changes height as the notes' tray opens and closes, and the card as the welcome goes.
        for el in column.iter().chain(card.iter()) {
            o.observe(el);
        }
    }
    observer.forget();
}
