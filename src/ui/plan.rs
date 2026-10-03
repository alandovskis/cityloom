//! The plan of the junction, drawn by `plan_svg`, and the pointer input on it:
//! pressing something selects it, dragging a grip turns a street or moves a
//! corner or crossing, and dropping a street from the palette adds it.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{Element, HtmlElement, KeyboardEvent, PointerEvent};

use crate::junction::Target;
use crate::ui::keys;
use crate::ui::plan_svg::{plan_label, plan_svg};
use crate::vm::plan_frame::Frame;
use crate::vm::junction::JunctionVm;
use crate::ui::watch::Watch;

/// What a grip is dragging.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grip {
    Arm,
    Corner,
    Crossing,
}

impl Grip {
    pub fn named(role: &str) -> Option<Grip> {
        match role {
            "grip-arm" => Some(Grip::Arm),
            "grip-corner" => Some(Grip::Corner),
            "grip-crossing" => Some(Grip::Crossing),
            _ => None,
        }
    }
}

/// What pressing something on the plan selects. A press on nothing clears the
/// selection; a press on a part that cannot be selected does nothing to it.
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

/// Where the window's size and the drawing's width are now.
#[cfg(target_arch = "wasm32")]
fn viewport_of(wrap: &Element) -> (f64, f64) {
    let height = leptos::web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|h| h.as_f64()).unwrap_or(800.0);
    (wrap.client_width() as f64, height)
}

#[derive(Clone, Copy)]
struct Drag {
    grip: Grip,
    uid: u32,
}

#[component]
pub fn PlanDrawing(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    let viewport = RwSignal::new((1000.0, 800.0));
    let drag = StoredValue::new_local(None::<Drag>);

    let wrap = NodeRef::<leptos::html::Div>::new();
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if let Some(el) = wrap.get() {
                watch_size(el.into(), viewport);
            }
        });
        // Control or Command with Z or Y undoes and redoes, wherever the keyboard is.
        use crate::ui::keys::Shortcut;
        window_event_listener(leptos::ev::keydown, move |e: KeyboardEvent| {
            if !(e.meta_key() || e.ctrl_key()) || typing(&e) {
                return;
            }
            match keys::shortcut(&e.key(), e.shift_key()) {
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
        if let Some((action, prevent)) = keys::on_plan(&e.key(), e.shift_key(), w.view_now().selected.kind) {
            if prevent {
                e.prevent_default();
            }
            w.apply(action);
        }
    };

    let markup = move || {
        let (v, units) = w.now();
        let (width, window) = viewport.get();
        let out = plan_svg(&v, width, window, units);
        w.set_frame(out.frame);
        out.markup
    };
    let size = move || {
        let (v, _) = w.now();
        let (width, window) = viewport.get();
        Frame::fit(v.bounds, width, window)
    };

    // A point in the drawing, from where the pointer is on the page.
    let plan_point = move |e: &PointerEvent| {
        let svg: Element = event_target(e);
        let r = svg.get_bounding_client_rect();
        w.frame().map_or((0.0, 0.0), |f| f.plan_point((e.client_x() as f64 - r.left(), e.client_y() as f64 - r.top())))
    };

    let press = move |e: PointerEvent| {
        if e.button() != 0 {
            return;
        }
        let svg: Element = event_target(&e);
        let hit = e.target().and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()).and_then(|t| t.closest("[data-role]").ok().flatten());
        let role = hit.as_ref().and_then(|h| h.get_attribute("data-role")).unwrap_or_default();
        let number = |name: &str| hit.as_ref().and_then(|h| h.get_attribute(name)).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
        let uid = number("data-uid") as u32;
        if let Some(grip) = Grip::named(&role) {
            if grip == Grip::Arm {
                w.quiet(|j| {
                    j.select(Target::Arm(uid));
                    true
                });
            }
            drag.set_value(Some(Drag { grip, uid }));
            w.quiet(|j| {
                j.begin_gesture();
                true
            });
            let _ = svg.set_pointer_capture(e.pointer_id());
            e.prevent_default();
            return;
        }
        let target = selected_by(&role, uid, number("data-lane"));
        if target != Target::None || w.view_now().selected.kind.is_some() {
            w.select(target);
        }
        focus_wrap();
    };

    let moved = move |e: PointerEvent| {
        let Some(d) = drag.get_value() else { return };
        let (x, y) = plan_point(&e);
        w.quiet(|j| match d.grip {
            Grip::Arm => j.drag_arm_to(d.uid, x, y),
            Grip::Corner => j.drag_corner_to(d.uid, x, y),
            Grip::Crossing => j.drag_crossing_to(d.uid, x, y),
        });
    };

    let finish = move |commit: bool| {
        if drag.get_value().is_none() {
            return;
        }
        drag.set_value(None);
        w.finish_gesture(commit);
    };

    view! {
      <div class="wrap" id="wrap" tabindex="0" role="group" aria-label="Junction plan editor" aria-describedby="keys" node_ref=wrap on:keydown=keydown>
        <svg
            id="drawing"
            xmlns="http://www.w3.org/2000/svg"
            role="img"
            aria-label=move || plan_label(&w.view())
            width=move || size().width.to_string()
            height=move || size().height.to_string()
            viewBox=move || {
                let f = size();
                format!("0 0 {} {}", f.width, f.height)
            }
            on:pointerdown=press
            on:pointermove=moved
            on:pointerup=move |_| finish(true)
            on:pointercancel=move |_| finish(false)
            inner_html=markup
        ></svg>
      </div>
    }
}

/// Puts the keyboard on the plan's group, as pressing the plan does.
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
    e.target().and_then(|t| leptos::wasm_bindgen::JsCast::dyn_into::<Element>(t).ok()).is_some_and(|t| t.closest("input, select, textarea").ok().flatten().is_some())
}

/// Keeps `viewport` as wide as the plan's group and as high as the window.
#[cfg(target_arch = "wasm32")]
fn watch_size(wrap: Element, viewport: RwSignal<(f64, f64)>) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};

    let update = {
        let wrap = wrap.clone();
        move || viewport.set(viewport_of(&wrap))
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
    window_event_listener(leptos::ev::resize, move |_| update());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grip_is_named_by_the_part_it_moves() {
        assert_eq!(Grip::named("grip-arm"), Some(Grip::Arm));
        assert_eq!(Grip::named("grip-corner"), Some(Grip::Corner));
        assert_eq!(Grip::named("grip-crossing"), Some(Grip::Crossing));
        assert_eq!(Grip::named("arm"), None);
    }

    #[test]
    fn pressing_a_part_of_the_plan_selects_it_and_pressing_nothing_clears() {
        assert_eq!(selected_by("arm", 3, 0), Target::Arm(3));
        assert_eq!(selected_by("corner", 3, 0), Target::Corner(3));
        assert_eq!(selected_by("crossing", 3, 0), Target::Crossing(3));
        assert_eq!(selected_by("lane", 3, 1), Target::Lane(3, 1));
        assert_eq!(selected_by("bus", 0, 0), Target::Bus);
        assert_eq!(selected_by("cycle", 0, 0), Target::Cycle);
        assert_eq!(selected_by("", 0, 0), Target::None);
        assert_eq!(selected_by("none", 3, 0), Target::None);
    }
}
