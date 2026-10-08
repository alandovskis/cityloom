//! The street's section, drawn by `street_svg`, and the pointer and keyboard
//! input on it: pressing selects a piece, dragging a boundary or the far edge
//! resizes, dragging a piece moves it, and the arrows, plus and minus, Enter and
//! Delete do the same by key.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{Element, HtmlElement, KeyboardEvent, PointerEvent};

use crate::shared::bind::current_element;
use crate::street::keys::{self, Action};
use crate::street::svg::{Geometry, Interaction, Moving, street_label, street_svg};
use crate::street::vm::StreetVm;
use crate::street::watch::SheetWatch;

/// How far a pointer must move before a pressed piece is being moved.
const DRAG_START_PX: f64 = 4.0;

/// What a press on the drawing started.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag {
    /// A boundary between pieces, by the index of the piece to its left.
    Resize { i: usize, start_x: f64 },
    /// The far edge of the last piece.
    Edge { uid: u32, start_x: f64 },
    /// A piece that may be dragged to a new place once the pointer has moved.
    Move { uid: u32, start_x: f64, active: bool },
}

/// What pressing something on the drawing starts, from its role and numbers.
pub fn drag_for(role: &str, uid: u32, i: usize, start_x: f64) -> Option<Drag> {
    match role {
        "handle" => Some(Drag::Resize { i, start_x }),
        "edge" => Some(Drag::Edge { uid, start_x }),
        "seg" => Some(Drag::Move { uid, start_x, active: false }),
        _ => None,
    }
}

/// How far a pointer has dragged, in millimetres of street.
pub fn delta_mm(dx_px: f64, scale: f64) -> i32 {
    (dx_px / scale).round() as i32
}

/// Whether a press has moved far enough to be a drag.
pub fn is_drag(dx_px: f64) -> bool {
    dx_px.abs() >= DRAG_START_PX
}

#[cfg(target_arch = "wasm32")]
fn coarse() -> bool {
    leptos::web_sys::window().and_then(|w| w.match_media("(pointer: coarse)").ok().flatten()).is_some_and(|m| m.matches())
}

#[cfg(not(target_arch = "wasm32"))]
fn coarse() -> bool {
    false
}

#[component]
pub fn StreetDrawing(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let width = RwSignal::new(1050.0_f64);
    let interaction = RwSignal::new(Interaction::default());
    let geometry = StoredValue::new_local(Geometry::fit(1050.0, 18_000));
    let drag = StoredValue::new_local(None::<Drag>);
    let tapped = StoredValue::new_local(false);
    let was_over = StoredValue::new_local(false);
    let fresh = StoredValue::new_local(false);
    let row_right = StoredValue::new_local(0.0_f64);
    let wrap = NodeRef::<leptos::html::Div>::new();

    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if let Some(el) = wrap.get() {
                watch_size(el.into(), width);
            }
        });
        // Bring a street that has just run over into view, and say whether it scrolls.
        let version = w.version();
        Effect::new(move |_| {
            version.track();
            width.track();
            interaction.track();
            request_animation_frame(move || after_draw(fresh.get_value(), row_right.get_value()));
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

    let markup = move || {
        let (v, units) = w.now();
        let mut ui = interaction.get();
        ui.coarse = coarse();
        let over = v.delta_mm > 0;
        ui.fresh = over && !was_over.get_value();
        was_over.set_value(over);
        fresh.set_value(ui.fresh);
        let out = street_svg(&v, &w.i18n(), width.get(), units, &ui);
        geometry.set_value(out.geometry);
        row_right.set_value(out.row_right);
        out.markup
    };
    let size = move || Geometry::fit(width.get(), w.view().row_mm);
    let height = move || size().rows.height;

    let press = move |e: PointerEvent| {
        if e.pointer_type() == "mouse" && e.button() != 0 {
            return;
        }
        let svg: Element = current_element(&e);
        let hit = e.target().and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()).and_then(|t| t.closest("[data-role]").ok().flatten());
        focus_wrap(wrap);
        let Some(hit) = hit else { return };
        let number = |name: &str| hit.get_attribute(name).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let role = hit.get_attribute("data-role").unwrap_or_default();
        let uid = number("data-uid") as u32;
        let start_x = e.client_x() as f64;
        let Some(d) = drag_for(&role, uid, number("data-i"), start_x) else { return };
        match d {
            Drag::Resize { i, .. } => {
                w.quiet(|ed| ed.begin_gesture());
                interaction.update(|ui| ui.resizing = Some(i));
            }
            Drag::Edge { .. } => {
                w.quiet(|ed| ed.begin_gesture());
                interaction.update(|ui| ui.edge = true);
            }
            Drag::Move { uid, .. } => {
                w.select(Some(uid));
                tapped.set_value(true);
            }
        }
        drag.set_value(Some(d));
        let _ = svg.set_pointer_capture(e.pointer_id());
        e.prevent_default();
    };

    let moved = move |e: PointerEvent| {
        let Some(d) = drag.get_value() else { return };
        let scale = geometry.with_value(|g| g.scale);
        match d {
            Drag::Resize { i, start_x } => {
                w.quiet(|ed| ed.resize_boundary(i, delta_mm(e.client_x() as f64 - start_x, scale)));
            }
            Drag::Edge { uid, start_x } => {
                w.quiet(|ed| ed.resize_edge(uid, delta_mm(e.client_x() as f64 - start_x, scale)));
            }
            Drag::Move { uid, start_x, active } => {
                if !active && !is_drag(e.client_x() as f64 - start_x) {
                    return;
                }
                tapped.set_value(false);
                drag.set_value(Some(Drag::Move { uid, start_x, active: true }));
                let svg: Element = current_element(&e);
                let left = svg.get_bounding_client_rect().left();
                let px = e.client_x() as f64 - left;
                let mm = geometry.with_value(|g| g.mm_at(px));
                let index = w.quiet(|ed| ed.drop_index(mm, uid));
                interaction.update(|ui| ui.moving = Some(Moving { uid, px, index: Some(index) }));
            }
        }
    };

    let finish = move |commit: bool| {
        let Some(d) = drag.get_value() else { return };
        drag.set_value(None);
        let moving = interaction.get_untracked().moving;
        interaction.update(|ui| {
            ui.resizing = None;
            ui.edge = false;
            ui.moving = None;
        });
        match d {
            Drag::Resize { .. } | Drag::Edge { .. } => w.finish_gesture(commit),
            Drag::Move { uid, active, .. } => {
                if let (true, true, Some(Moving { index: Some(index), .. })) = (commit, active, moving) {
                    w.edit(|ed| ed.move_to(uid, index));
                }
            }
        }
    };

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
            Action::Escape if drag.get_value().is_some() => finish(false),
            other => w.apply(other),
        }
    };

    view! {
        <div class="wrap" id="wrap" tabindex="0" role="group" aria-label="Street cross-section editor" aria-describedby="keys" node_ref=wrap on:keydown=keydown>
            <svg
                id="drawing"
                xmlns="http://www.w3.org/2000/svg"
                role="img"
                aria-label=move || {
                    let (v, units) = w.now();
                    let i18n = w.i18n();
                    // Read so the label is drawn again when the language is switched.
                    i18n.locale();
                    street_label(&v, &i18n, units)
                }
                width=move || size().width.to_string()
                height=move || height().to_string()
                viewBox=move || format!("0 0 {} {}", size().width, height())
                on:pointerdown=press
                on:pointermove=moved
                on:pointerup=move |_| {
                    finish(true);
                    if tapped.get_value() {
                        tapped.set_value(false);
                        show_inspector();
                    }
                }
                on:pointercancel=move |_| finish(false)
                inner_html=markup
            ></svg>
        </div>
    }
}

/// Puts the keyboard on the section, as pressing it does.
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
    if doc.document_element().is_some_and(|r| r.get_attribute("data-inspector").as_deref() == Some("closed")) {
        if let Some(toggle) = doc.get_element_by_id("inspector-toggle") {
            leptos::wasm_bindgen::JsCast::unchecked_into::<HtmlElement>(toggle).click();
        }
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
/// that does not turn into a drag brings it into view.
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

#[cfg(target_arch = "wasm32")]
fn typing(e: &KeyboardEvent) -> bool {
    e.target()
        .and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok())
        .is_some_and(|t| t.closest("input, select, textarea").ok().flatten().is_some())
}

/// After the drawing is redrawn: bring a street that has just run over into
/// view, and say whether the section scrolls.
#[cfg(target_arch = "wasm32")]
fn after_draw(fresh: bool, row_right: f64) {
    let doc = document();
    let Some(scroll) = doc.get_element_by_id("scroll") else { return };
    if fresh {
        let reach = row_right + 140.0 - scroll.client_width() as f64;
        if reach > scroll.scroll_left() as f64 {
            scroll.set_scroll_left(reach as i32);
        }
    }
    if let Ok(Some(cue)) = doc.query_selector(".cue") {
        let _ = cue.class_list().toggle_with_force("on", scroll.scroll_width() > scroll.client_width() + 1);
    }
}

/// Keeps `width` as wide as the section's group.
#[cfg(target_arch = "wasm32")]
fn watch_size(wrap: Element, width: RwSignal<f64>) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};

    let update = {
        let wrap = wrap.clone();
        move || width.set(wrap.client_width() as f64)
    };
    update();
    let on_resize = update.clone();
    let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| on_resize());
    if let Some(scroll) = wrap.parent_element() {
        if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
            o.observe(&scroll);
        }
    }
    observer.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressing_a_boundary_an_edge_or_a_piece_starts_the_drag_that_suits_it() {
        assert_eq!(drag_for("handle", 0, 3, 50.0), Some(Drag::Resize { i: 3, start_x: 50.0 }));
        assert_eq!(drag_for("edge", 7, 0, 50.0), Some(Drag::Edge { uid: 7, start_x: 50.0 }));
        assert_eq!(drag_for("seg", 7, 0, 50.0), Some(Drag::Move { uid: 7, start_x: 50.0, active: false }));
        assert_eq!(drag_for("", 0, 0, 0.0), None);
        assert_eq!(drag_for("grip", 0, 0, 0.0), None);
    }

    #[test]
    fn a_drag_is_measured_in_millimetres_of_street() {
        assert_eq!(delta_mm(10.0, 0.05), 200);
        assert_eq!(delta_mm(-10.0, 0.05), -200);
        assert_eq!(delta_mm(0.4, 0.05), 8);
    }

    #[test]
    fn a_press_is_a_drag_only_after_it_has_moved_a_few_pixels() {
        assert!(!is_drag(3.9) && !is_drag(-3.9));
        assert!(is_drag(4.0) && is_drag(-4.0) && is_drag(40.0));
    }
}
