//! The plan on the street page, in place of the zone strip: the drawing (`plan_svg`) with its hit layer (`hit`), the
//! keyboard's point along it, the street's ends above it and the stretch picked under it. The view forwards: a press, a
//! move and a release go to the view-model's plan commands (`plan_press`, `plan_move`, `plan_release`, `plan_cancel`),
//! a key on the point to `plan_key`.

use std::rc::Rc;

use leptos::ev::{KeyboardEvent, PointerEvent};
use leptos::prelude::*;

use super::{hit, plan_layout, plan_mm, plan_svg};
use crate::shared::i18n::Args;
use crate::shared::said::say;
use crate::street::vm::StreetVm;
use crate::street::vm::keys::on_plan;
use crate::street::vm::plan::{PlanGesture, PlanPress};
use crate::street::vm::text::{picked_said, plan_label, stretch_label};
use crate::street::vm::watch::SheetWatch;

/// The keys the point answers to beyond a slider's own (`keys::on_plan`), as `aria-keyshortcuts` names them.
const SHORTCUTS: &str = "Shift+ArrowLeft Shift+ArrowRight Shift+PageUp Shift+PageDown Shift+Home Shift+End Enter Escape Alt+ArrowLeft Alt+ArrowRight Shift+Alt+ArrowLeft Shift+Alt+ArrowRight";

/// What the pointer is over when nothing is pressed, for its cursor.
fn hover_of(p: PlanPress) -> &'static str {
    match p {
        PlanPress::Street { .. } => "street",
        PlanPress::Handle { .. } | PlanPress::Boundary { .. } => "edge",
        PlanPress::Thing { .. } => "thing",
    }
}

/// A position in metres as an ARIA value.
fn metres(mm: i32) -> String {
    format!("{}", f64::from(mm) / 1000.0)
}

/// The whole street from above, first end on the left: one group holding the drawing and the point along it, the one tab
/// stop (a slider: the arrows move it, with Shift they pick from where they began). A press on the drawing picks the run
/// there; a drag picks a stretch, or moves a handle of the pick, a boundary between runs or a thing along the street.
#[component]
pub fn Plan(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm.clone());
    let vm = StoredValue::new_local(vm);
    // The width the plan is fitted to: the group's, measured in the browser.
    let width = RwSignal::new(600.0_f64);
    let boxed = NodeRef::<leptos::html::Div>::new();
    let canvas = NodeRef::<leptos::html::Div>::new();
    let point = NodeRef::<leptos::html::Div>::new();
    // The press under way, and how far the pointer was from the line it took.
    let gesture = StoredValue::new_local(None::<(PlanGesture, f64)>);
    let hover = RwSignal::new("street");

    let svg = Memo::new(move |_| {
        let (v, units) = w.now();
        plan_svg(&v.plan, width.get(), &w.i18n(), units)
    });
    let layout = move |at_mm: i32| plan_layout(&w.view().plan, width.get(), at_mm);
    let label = move || plan_label(&w.view().plan).map(|said| say(&w.i18n(), w.units(), &said));
    let length = move || w.view().plan.length_mm.unwrap_or(0);
    let length_text = move |mm: i32| {
        let i18n = w.i18n();
        w.units().length_in(mm, i18n.locale())
    };
    // The stretch picked, or the whole street.
    let picked = move || {
        let plan = &w.view().plan;
        stretch_label(plan).or_else(|| picked_said(plan, Vec::new())).map(|said| say(&w.i18n(), w.units(), &said))
    };
    let notice = move || {
        let e = w.view().zones.elsewhere.as_ref().map(|e| (e.drawn_mm, e.loaded_mm))?;
        let (i18n, units) = (w.i18n(), w.units());
        let locale = i18n.locale();
        Some(i18n.tr("zones-elsewhere", &Args::new().str("drawn", units.length_in(e.0, locale)).str("loaded", units.length_in(e.1, locale))))
    };

    // Where the pointer is on the drawing, from its top left.
    let local = move |e: &PointerEvent| {
        let r = canvas.get_untracked()?.get_bounding_client_rect();
        Some((f64::from(e.client_x()) - r.left(), f64::from(e.client_y()) - r.top()))
    };
    // Where along the street the pointer is, less how far it was from the line it took.
    let along = move |x: f64, grab: f64| plan_mm(&w.view_now().plan, width.get_untracked(), x - grab);
    let cancel = move || {
        if let Some((g, _)) = gesture.get_value() {
            gesture.set_value(None);
            vm.with_value(|vm| vm.plan_cancel(g));
        }
    };
    let on_down = move |e: PointerEvent| {
        if e.button() != 0 || !e.is_primary() {
            return;
        }
        cancel();
        let Some((x, y)) = local(&e) else { return };
        let h = hit(&w.view_now().plan, width.get_untracked(), x, y);
        let g = vm.with_value(|vm| vm.plan_press(h.press, f64::from(e.client_x())));
        gesture.set_value(Some((g, h.grab_px)));
        // The drawing holds the pointer: it is never replaced, only what it shows.
        if let Some(c) = canvas.get_untracked() {
            let _ = c.set_pointer_capture(e.pointer_id());
        }
        e.prevent_default();
        if let Some(p) = point.get_untracked() {
            let options = leptos::web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = p.focus_with_options(&options);
        }
    };
    let on_move = move |e: PointerEvent| {
        let Some((x, y)) = local(&e) else { return };
        let Some((g, grab)) = gesture.get_value() else {
            let over = hover_of(hit(&w.view_now().plan, width.get_untracked(), x, y).press);
            if hover.get_untracked() != over {
                hover.set(over);
            }
            return;
        };
        let g = vm.with_value(|vm| vm.plan_move(&g, f64::from(e.client_x()), along(x, grab)));
        if gesture.with_value(Option::is_some) {
            gesture.set_value(Some((g, grab)));
        }
    };
    let on_up = move |e: PointerEvent| {
        let Some((g, grab)) = gesture.get_value() else { return };
        gesture.set_value(None);
        let at = local(&e).map_or(0, |(x, _)| along(x, grab));
        vm.with_value(|vm| vm.plan_release(g, at));
    };
    let on_key = move |e: KeyboardEvent| {
        if e.key() == "Escape" && gesture.with_value(Option::is_some) {
            e.prevent_default();
            cancel();
            return;
        }
        if e.ctrl_key() || e.meta_key() {
            return;
        }
        let Some(k) = on_plan(&e.key(), e.shift_key(), e.alt_key()) else { return };
        e.prevent_default();
        vm.with_value(|vm| vm.plan_key(k));
    };

    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if let Some(el) = boxed.get() {
                watch_width(el.into(), width);
            }
        });
        // The point is kept in sight inside the plan's own scroll (never the page's).
        Effect::new(move |_| {
            let x = layout(vm.with_value(|vm| vm.plan_point())).1;
            if let Some(b) = boxed.get() {
                let (left, shown) = (f64::from(b.scroll_left()), f64::from(b.client_width()));
                let by = crate::street::view::strip::scroll_into_view_px(x - 12.0, x + 12.0, left, left + shown);
                if by != 0.0 {
                    b.set_scroll_left(b.scroll_left() + by.round() as i32);
                }
            }
        });
    }

    view! {
        <div class="plan-section">
            <p class="plan-ends" aria-hidden="true">
                <span>{move || length_text(0)}</span>
                <span>{move || length_text(length())}</span>
            </p>
            <div id="plan" class="plan-box" role="group" aria-label=label node_ref=boxed>
                <div
                    class="plan-canvas"
                    data-hover=move || hover.get()
                    style:width=move || format!("{}px", layout(0).0.round())
                    node_ref=canvas
                    on:pointerdown=on_down
                    on:pointermove=on_move
                    on:pointerup=on_up
                    on:pointercancel=move |_| cancel()
                    on:lostpointercapture=move |_| cancel()
                >
                    <div class="plan-svg" inner_html=move || svg.get()></div>
                    <div
                        id="plan-point"
                        node_ref=point
                        role="slider"
                        tabindex="0"
                        aria-labelledby="plan"
                        aria-orientation="horizontal"
                        aria-valuemin="0"
                        aria-valuemax=move || metres(length())
                        aria-valuenow=move || metres(vm.with_value(|vm| vm.plan_point()))
                        aria-valuetext=move || vm.with_value(|vm| vm.plan_point_text())
                        aria-keyshortcuts=SHORTCUTS
                        style:left=move || format!("{}px", layout(vm.with_value(|vm| vm.plan_point())).1)
                        on:keydown=on_key
                    ></div>
                </div>
            </div>
            <p id="plan-picked" class="plan-picked">{picked}</p>
            <p class="plan-notice" role="status">{notice}</p>
        </div>
    }
}

/// Keeps `width` as wide as the plan's box.
#[cfg(target_arch = "wasm32")]
fn watch_width(el: leptos::web_sys::Element, width: RwSignal<f64>) {
    use leptos::wasm_bindgen::{JsCast, closure::Closure};

    let update = {
        let el = el.clone();
        move || {
            let now = f64::from(el.client_width());
            if now > 0.0 && (width.get_untracked() - now).abs() >= 0.5 {
                width.set(now);
            }
        }
    };
    update();
    let observer = Closure::<dyn FnMut(leptos::web_sys::js_sys::Array)>::new(move |_| update());
    if let Ok(o) = leptos::web_sys::ResizeObserver::new(observer.as_ref().unchecked_ref()) {
        o.observe(&el);
    }
    observer.forget();
}
