//! The street's drawing: the street in 2.5D, drawn by `iso_svg`. A press on a piece selects it and brings up its
//! details, where its width and kind are edited (a perspective has no handles to drag), and the arrows, plus and
//! minus, Enter and Delete edit the street by key.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{Element, HtmlElement, KeyboardEvent, PointerEvent};

use crate::shared::i18n::Args;
use crate::street::iso::{iso_label_at, iso_svg, max_height};
use crate::street::keys::{self, Action};
use crate::street::vm::StreetVm;
use crate::street::watch::SheetWatch;

#[component]
pub fn StreetDrawing(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let width = RwSignal::new(1050.0_f64);
    let wrap = NodeRef::<leptos::html::Div>::new();

    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if let Some(el) = wrap.get() {
                watch_size(el.into(), width);
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
        iso_svg(&v, &w.i18n(), width.get())
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

    // A press on a piece selects it and brings up its details.
    let pick = move |e: PointerEvent| {
        if e.pointer_type() == "mouse" && e.button() != 0 {
            return;
        }
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
            // The group keeps the height the drawing is held to (`iso::max_height`), and the drawing, which is as
            // wide as the group, is centred in it.
            style:min-height=move || format!("{}px", max_height(width.get()))
        >
            <svg
                id="drawing"
                class="iso"
                xmlns="http://www.w3.org/2000/svg"
                on:pointerdown=pick
                role="img"
                aria-label=move || {
                    let (v, units) = w.now();
                    let i18n = w.i18n();
                    i18n.locale();
                    iso_label_at(&v, &i18n, units, width.get())
                }
                width=move || iso.with(|s| s.width.to_string())
                height=move || iso.with(|s| s.height.to_string())
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

/// Keeps `width` as wide as the drawing's group.
#[cfg(target_arch = "wasm32")]
fn watch_size(wrap: Element, width: RwSignal<f64>) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};

    // The group is watched as well as its scroller: its own width can change with no change to the scroller's.
    let update = {
        let wrap = wrap.clone();
        move || {
            let now = wrap.client_width() as f64;
            if (width.get_untracked() - now).abs() >= 0.5 {
                width.set(now);
            }
        }
    };
    update();
    let on_resize = update.clone();
    let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| on_resize());
    if let Some(scroll) = wrap.parent_element() {
        if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
            o.observe(&scroll);
            o.observe(&wrap);
        }
    }
    observer.forget();
}
