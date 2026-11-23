//! The junction's drawing, the 2.5D (`iso/`), and the input on it: pressing a part selects it as the keys do, and
//! the keys on the group step through the parts and change the selected one. There is nothing to drag: what a
//! street's direction, a corner's radius and a crossing's setback need is a typed field with steppers in the
//! inspector, and a key.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{Element, HtmlElement, KeyboardEvent, PointerEvent};

use crate::junction::model::Target;
use crate::junction::view::frame::Frame;
use crate::junction::view::iso::{iso_label, iso_svg_framed};
use crate::junction::vm::JunctionVm;
use crate::junction::vm::keys;
use crate::junction::vm::watch::Watch;
use crate::shared::i18n::Args;

/// What pressing something in the drawing selects. A press on nothing clears the selection; a press on a part that
/// cannot be selected does nothing to it.
pub fn selected_by(role: &str, uid: u32, lane: usize) -> Target {
    match role {
        "cycle" => Target::Cycle,
        "bus" => Target::Bus,
        "lane" => Target::Lane(uid, lane),
        "arm" => Target::Arm(uid),
        "crossing" => Target::Crossing(uid),
        "corner" => Target::Corner(uid),
        _ => Target::None,
    }
}

/// What a press on a shape selects, from the attributes it carries (`data-role`, `data-uid`, `data-lane`, written by
/// `iso::hit_attrs`). A number that is missing or is not one is 0.
pub fn pressed(attribute: impl Fn(&str) -> Option<String>) -> Target {
    let number = |name: &str| attribute(name).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    selected_by(&attribute("data-role").unwrap_or_default(), number("data-uid") as u32, number("data-lane"))
}

/// The frame the drawing is laid out in now (`Frame::in_room`): as wide as its group, and as tall as the room its
/// column under the title leaves it (`shared::column_room`): the scroller's height less what else the drawing's card
/// holds around the picture (the part buttons, the plan key, its padding). The card does not grow to fill the column
/// (`web/editor.css`, `.tray-page .plan-drawing`), so this holds no slack and the same room gives the same drawing,
/// whatever came before. Where that column does not scroll (a phone: the page scrolls), the window's height decides.
#[cfg(target_arch = "wasm32")]
fn frame_of(wrap: &Element, column: Option<&Element>, card: Option<&Element>) -> Frame {
    use crate::shared::column_room::{height, room_in_column};
    let picture = wrap.query_selector("svg").ok().flatten().map_or_else(|| height(wrap), |svg| svg.get_bounding_client_rect().height());
    let room = room_in_column(column, card, picture);
    let window = leptos::web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|h| h.as_f64()).unwrap_or(800.0);
    Frame::in_room(wrap.client_width() as f64, room, window)
}

#[component]
pub fn Drawing(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    let frame = RwSignal::new(Frame::fit(1000.0, 800.0));

    let wrap = NodeRef::<leptos::html::Div>::new();
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if let Some(el) = wrap.get() {
                watch_size(el.into(), frame);
            }
        });
        // Control or Command with Z or Y undoes and redoes, wherever the keyboard is.
        use crate::shared::shortcut::{Shortcut, shortcut};
        window_event_listener(leptos::ev::keydown, move |e: KeyboardEvent| {
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

    let keydown = move |e: KeyboardEvent| {
        if e.meta_key() || e.ctrl_key() || e.alt_key() {
            return;
        }
        // A key pressed on a button under the drawing (the step buttons) is the button's own: it bubbles up to the group, which
        // must not turn, resize or remove what is selected for it.
        if e.target().and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()).is_some_and(|t| t.id() != "wrap") {
            return;
        }
        if let Some((action, prevent)) = keys::on_drawing(&e.key(), e.shift_key(), w.view_now().selected.kind) {
            if prevent {
                e.prevent_default();
            }
            w.apply(action);
        }
    };

    let markup = move || {
        let (v, units) = w.now();
        iso_svg_framed(&v, &w.i18n(), frame.get(), units).markup
    };
    let label = move || iso_label(&w.view(), &w.i18n());
    let size = move || frame.get();

    // A press on a part of the drawing selects it, as the keys do.
    let press = move |e: PointerEvent| {
        if e.button() != 0 {
            return;
        }
        let hit = e.target().and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()).and_then(|t| t.closest("[data-role]").ok().flatten());
        let target = pressed(|name| hit.as_ref().and_then(|h| h.get_attribute(name)));
        if target != Target::None || w.view_now().selected.kind.is_some() {
            w.select(target);
        }
        focus_wrap();
    };

    // Under the drawing are the keys' way through the parts, for a pointer: a press can miss a part that is small on the
    // screen, and these two buttons are as large as a finger and reach every part the keys do.
    view! {
      <div class="wrap" id="wrap" tabindex="0" role="group" aria-label=move || w.i18n().tr("jn-editor", &Args::new()) aria-describedby="keys" node_ref=wrap on:keydown=keydown>
        <svg
            id="drawing"
            xmlns="http://www.w3.org/2000/svg"
            role="img"
            aria-label=label
            width=move || size().width.to_string()
            height=move || size().height.to_string()
            viewBox=move || {
                let f = size();
                format!("0 0 {} {}", f.width, f.height)
            }
            on:pointerdown=press
            inner_html=markup
        ></svg>
        <div class="part-steps">
            <button type="button" class="btn" id="part-previous" on:click=move |_| w.apply(keys::Action::Select(-1))>{move || w.i18n().tr("jn-part-previous", &Args::new())}</button>
            <button type="button" class="btn" id="part-next" on:click=move |_| w.apply(keys::Action::Select(1))>{move || w.i18n().tr("jn-part-next", &Args::new())}</button>
        </div>
      </div>
    }
}

/// Puts the keyboard on the drawing's group, as pressing the drawing does.
fn focus_wrap() {
    if let Some(el) = document().get_element_by_id("wrap").and_then(|e| leptos::wasm_bindgen::JsCast::dyn_into::<HtmlElement>(e).ok()) {
        let options = leptos::web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = el.focus_with_options(&options);
    }
}

/// Whether a key press came from a field being typed in.
#[cfg(target_arch = "wasm32")]
fn typing(e: &KeyboardEvent) -> bool {
    e.target()
        .and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok())
        .is_some_and(|t| t.closest("input, select, textarea").ok().flatten().is_some())
}

/// Keeps `frame` as the drawing is laid out now (`frame_of`), as its group, its column and the window change size.
#[cfg(target_arch = "wasm32")]
fn watch_size(wrap: Element, frame: RwSignal<Frame>) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};

    let column = wrap.closest(".stage-scroll").ok().flatten();
    let card = wrap.closest(".drawing").ok().flatten();
    let update = {
        let (wrap, column, card) = (wrap.clone(), column.clone(), card.clone());
        move || {
            let now = frame_of(&wrap, column.as_ref(), card.as_ref());
            if frame.get_untracked() != now {
                frame.set(now);
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
        // The column changes height as the notes' tray opens and closes, and as the head wraps; the card as the key
        // under the drawing gains or loses a row; the group as the part buttons wrap.
        for el in [column.as_ref(), card.as_ref(), Some(&wrap)].into_iter().flatten() {
            o.observe(el);
        }
    }
    observer.forget();
    window_event_listener(leptos::ev::resize, move |_| update());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressing_a_part_of_the_drawing_selects_it_and_pressing_nothing_clears() {
        assert_eq!(selected_by("arm", 3, 0), Target::Arm(3));
        assert_eq!(selected_by("corner", 3, 0), Target::Corner(3));
        assert_eq!(selected_by("crossing", 3, 0), Target::Crossing(3));
        assert_eq!(selected_by("lane", 3, 1), Target::Lane(3, 1));
        assert_eq!(selected_by("bus", 0, 0), Target::Bus);
        assert_eq!(selected_by("cycle", 0, 0), Target::Cycle);
        assert_eq!(selected_by("", 0, 0), Target::None);
        assert_eq!(selected_by("none", 3, 0), Target::None);
    }

    #[test]
    fn a_press_reads_the_attributes_of_what_it_landed_on_and_a_part_with_none_selects_nothing() {
        let on = |role: &'static str, uid: &'static str, lane: &'static str| {
            move |name: &str| match name {
                "data-role" => Some(role.to_string()),
                "data-uid" if !uid.is_empty() => Some(uid.to_string()),
                "data-lane" if !lane.is_empty() => Some(lane.to_string()),
                _ => None,
            }
        };
        assert_eq!(pressed(on("arm", "4", "")), Target::Arm(4));
        assert_eq!(pressed(on("lane", "4", "2")), Target::Lane(4, 2));
        assert_eq!(pressed(on("crossing", "4", "")), Target::Crossing(4));
        assert_eq!(pressed(on("cycle", "", "")), Target::Cycle);
        assert_eq!(pressed(on("bus", "", "")), Target::Bus);
        // A number that is not one is zero; a part with no role selects nothing.
        assert_eq!(pressed(on("arm", "x", "")), Target::Arm(0));
        assert_eq!(pressed(on("", "4", "")), Target::None);
        assert_eq!(pressed(|_| None), Target::None);
    }

    #[test]
    fn the_picture_gives_the_shapes_the_attributes_a_press_reads() {
        // `hit_attrs` is the writing of what `pressed` reads.
        let attrs = crate::junction::view::iso::hit_attrs("lane", Some(4), Some(2));
        let read = |name: &str| attrs.split(&format!(" {name}=\"")).nth(1).and_then(|v| v.split('"').next()).map(str::to_string);
        assert_eq!(pressed(read), Target::Lane(4, 2));
        let read = |name: &str| {
            crate::junction::view::iso::hit_attrs("cycle", None, None)
                .split(&format!(" {name}=\""))
                .nth(1)
                .and_then(|v| v.split('"').next())
                .map(str::to_string)
        };
        assert_eq!(pressed(read), Target::Cycle);
    }
}
