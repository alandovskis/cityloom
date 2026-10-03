//! The rest of the junction page around the plan: its heading, the status
//! line, undo and redo, the key to the strips, and the streets and samples to
//! start from.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{Element, HtmlElement, PointerEvent};

use crate::shared::catalogue::{KINDS, SAMPLES};
use crate::junction::model::JUNCTION_SAMPLES;
use crate::junction::read_model::JView;
use crate::junction::text::{arms_text, fit_text};
use crate::junction::vm::JunctionVm;
use crate::junction::watch::Watch;

/// The kinds of piece that appear in the plan, in the order the catalogue lists them.
pub fn key_kinds(v: &JView) -> Vec<usize> {
    let mut kinds: Vec<usize> = v.arms.iter().flat_map(|a| a.pieces.iter().map(|p| p.kind)).collect();
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

/// The tint and hatch of a kind of piece, in a box of `width` by 22.
fn swatch_rects(id: &str, x: f64, width: f64) -> impl IntoView {
    view! {
        <rect class=format!("k-{id}") x=x width=width height="22" stroke="none"/>
        <rect x=x width=width height="22" fill=format!("url(#h-{id})") stroke="none"/>
    }
}

#[component]
pub fn Header(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    let name = move || w.view().name.clone();
    let linked = move || w.view().linked;
    let arms = move || arms_text(&w.view());
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        if w.view().linked {
            document().set_title(&format!("{} · CityLoom", w.view().name));
        }
    });
    view! {
        <h1 id="street-name">{name}</h1>
        <p class="street-sub" id="street-sub">
            {move || linked().then(|| view! {
                <a class="back" href="map.html">
                    <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden="true" focusable="false"><path d="M12 7H2M6 3 2 7l4 4"/></svg>
                    "City map"
                </a>
                " "
                <span aria-hidden="true">"·"</span>
                " "
            })}
            <span>"Junction plan"</span>
            " "
            <span aria-hidden="true">"·"</span>
            " "
            <span><b id="arm-count" class="fig">{arms}</b></span>
        </p>
    }
}

#[component]
pub fn TitleBlock(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    view! {
        <div class="tb-cell tb-wide"><span>"Junction"</span><b id="tb-street">{move || w.view().name.clone()}</b></div>
        <div class="tb-cell"><span>"Streets"</span><b id="tb-row" class="fig">{move || w.view().arms.len().to_string()}</b></div>
        <div class="tb-cell"><span>"Changes made"</span><b id="tb-changes" class="fig">{move || w.view().revisions.len().to_string()}</b></div>
    }
}

/// The status line: whether the junction works.
#[component]
pub fn Fit(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    view! {
        <p id="fit" class=move || if w.view().checks.iter().any(|c| !c.ok) { "fit bad" } else { "fit" } role="status">
            {move || fit_text(&w.view())}
        </p>
    }
}

#[component]
pub fn History(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    view! {
        <button type="button" id="undo" class="btn" disabled=move || !w.view().can_undo on:click=move |_| { w.undo(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M5.5 3 2.5 6l3 3M2.5 6H10a3.5 3.5 0 0 1 0 7H6"/></svg>
            "Undo"
        </button>
        <button type="button" id="redo" class="btn" disabled=move || !w.view().can_redo on:click=move |_| { w.redo(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M10.5 3l3 3-3 3M13.5 6H6a3.5 3.5 0 0 0 0 7h4"/></svg>
            "Redo"
        </button>
        <button type="button" id="reset" class="btn" disabled=move || !w.view().changed on:click=move |_| { w.reset(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M2.5 8a5.5 5.5 0 1 0 1.8-4.1M2.5 2.5v3h3"/></svg>
            "Start over"
        </button>
    }
}

/// A key to the strips: the tint and hatch of each piece that appears in the plan.
#[component]
pub fn Key(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    move || {
        key_kinds(&w.view())
            .into_iter()
            .map(|k| {
                view! {
                    <li>
                        <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">{swatch_rects(KINDS[k].id, 0.0, 44.0)}</svg>
                        {KINDS[k].name}
                    </li>
                }
            })
            .collect_view()
    }
}

/// Where a press on a street chip is, until it is let go.
struct Chip {
    street: usize,
    x: i32,
    y: i32,
    active: bool,
    ghost: Option<HtmlElement>,
}

fn street_chip(w: Watch, street: usize, chip: StoredValue<Option<Chip>, LocalStorage>, dragged: StoredValue<bool, LocalStorage>) -> impl IntoView {
    let s = &SAMPLES[street];
    let mut x = 0.0;
    let rects = s
        .segments
        .iter()
        .map(|(id, mm)| {
            let width = *mm as f64 * 44.0 / s.row_mm as f64;
            let r = swatch_rects(id, x, width);
            x += width;
            r
        })
        .collect_view();

    let press = move |e: PointerEvent| {
        if e.button() != 0 {
            return;
        }
        dragged.set_value(false);
        chip.set_value(Some(Chip { street, x: e.client_x(), y: e.client_y(), active: false, ghost: None }));
        if let Some(t) = e.current_target().and_then(|t| t.dyn_into::<Element>().ok()) {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
    };
    let moved = move |e: PointerEvent| {
        chip.update_value(|c| {
            let Some(c) = c else { return };
            if !c.active && f64::from((e.client_x() - c.x).pow(2) + (e.client_y() - c.y).pow(2)).sqrt() > 6.0 {
                c.active = true;
                dragged.set_value(true);
                if let Ok(g) = document().create_element("div") {
                    let g: HtmlElement = g.unchecked_into();
                    g.set_class_name("drag-chip");
                    g.set_text_content(Some(SAMPLES[c.street].name));
                    if let Some(body) = document().body() {
                        let _ = body.append_child(&g);
                    }
                    c.ghost = Some(g);
                }
            }
            if c.active {
                if let Some(g) = &c.ghost {
                    let _ = g.style().set_property("left", &format!("{}px", e.client_x()));
                    let _ = g.style().set_property("top", &format!("{}px", e.client_y()));
                }
            }
        });
    };
    let end = move |commit: bool, e: &PointerEvent| {
        let Some(c) = chip.try_update_value(|c| c.take()).flatten() else { return };
        if let Some(g) = &c.ghost {
            g.remove();
        }
        if !c.active || !commit {
            return;
        }
        let over = document().get_element_by_id("drawing").is_some_and(|svg| {
            let r = svg.get_bounding_client_rect();
            let (x, y) = (e.client_x() as f64, e.client_y() as f64);
            x >= r.left() && x <= r.right() && y >= r.top() && y <= r.bottom()
        });
        if over {
            let Some(frame) = w.frame() else { return };
            let Some(svg) = document().get_element_by_id("drawing") else { return };
            let r = svg.get_bounding_client_rect();
            let (px, py) = frame.plan_point((e.client_x() as f64 - r.left(), e.client_y() as f64 - r.top()));
            w.edit(|j| j.add_arm_toward(c.street, px, py) != 0);
        }
    };
    view! {
        <li>
            <button
                type="button"
                class="chip"
                on:pointerdown=press
                on:pointermove=moved
                on:pointerup=move |e| end(true, &e)
                on:pointercancel=move |e| end(false, &e)
                on:click=move |e| {
                    if e.detail() > 1 || dragged.get_value() {
                        return;
                    }
                    w.edit(|j| j.add_arm(street, -1) != 0);
                }
            >
                <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">{rects}</svg>
                <span><b>{s.name}</b><small>{move || format!("{} wide", w.units().length(s.row_mm))}</small></span>
                <svg class="grip-ico" viewBox="0 0 10 14" aria-hidden="true"><path d="M2 2h.01M8 2h.01M2 7h.01M8 7h.01M2 12h.01M8 12h.01"/></svg>
            </button>
        </li>
    }
}

/// The streets to add: drag one onto the plan, or press it to add in the widest gap.
#[component]
pub fn Palette(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    let chip = StoredValue::new_local(None::<Chip>);
    let dragged = StoredValue::new_local(false);
    SAMPLES.iter().enumerate().filter(|(_, s)| !s.freeway).map(|(i, _)| street_chip(w, i, chip, dragged)).collect_view()
}

/// The sample junctions to start from.
#[component]
pub fn Samples(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    JUNCTION_SAMPLES
        .iter()
        .enumerate()
        .map(|(i, s)| {
            view! {
                <li>
                    <button type="button" class="chip plain" aria-pressed=move || (w.view().sample == i).to_string() on:click=move |_| w.load_sample(i)>
                        <span></span>
                        <span><b>{s.name}</b><small>{format!("{} streets", s.arms.len())}</small></span>
                    </button>
                </li>
            }
        })
        .collect_view()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::*;

    #[test]
    fn the_key_lists_each_piece_in_the_plan_once_in_catalogue_order() {
        let kinds = key_kinds(&Junction::new(0).view());
        assert!(kinds.windows(2).all(|w| w[0] < w[1]));
        assert!(!kinds.is_empty());
        let names: Vec<&str> = kinds.iter().map(|k| KINDS[*k].id).collect();
        assert!(names.contains(&"sidewalk") && names.contains(&"travel"));
    }

    #[test]
    fn a_junction_without_a_bike_lane_does_not_key_one() {
        let names: Vec<&str> = key_kinds(&Junction::new(0).view()).iter().map(|k| KINDS[*k].id).collect();
        assert!(!names.contains(&"bikeshare"));
    }
}
