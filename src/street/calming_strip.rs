//! The strip of the raised tables and crossings under the curbside rows: one listbox for the whole street (an element
//! spans the roadway, so there is no side) with a cell per element and per free stretch. It follows the curbside strip
//! (`curb_strip.rs`) and shares its drag arithmetic: one listbox and one tab stop, the arrow keys choosing a cell and Alt
//! with them moving the element in it.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlElement;

use super::curb_strip::{DragSpan, drag_span, dragged_from_mm};
use super::keys::{ZoneAction, on_strip};
use super::model::StripCell;
use super::row::CalmingRow;
use super::strip::{basis_by_length, is_boundary_drag};
use super::text::calming_text;
use super::watch::SheetWatch;
use crate::shared::i18n::Args;

/// A press on an element, held until it ends.
#[derive(Clone, Copy)]
struct Press {
    start_x: f64,
    span: DragSpan,
    active: bool,
}

fn cell(w: SheetWatch, i: usize, on_down: impl Fn(leptos::ev::PointerEvent, usize) + Copy + 'static) -> impl IntoView {
    let state = move || w.view().calming_strip.cells.get(i).map(|c| (c.element.is_some(), c.ok));
    let name = move || {
        let (i18n, units) = (w.i18n(), w.units());
        w.view().calming_strip.cells.get(i).map(|c| crate::shared::said::say(&i18n, units, c.label())).unwrap_or_default()
    };
    view! {
        <div
            class="calming-cell"
            class:gap=move || state().is_some_and(|c| !c.0)
            class:failing=move || state().is_some_and(|c| !c.1)
            id=CalmingRow::cell_id(i)
            role="option"
            aria-selected=move || (w.view().calming_strip.selected == Some(i)).to_string()
            aria-label=move || {
                let _ = w.i18n().locale();
                calming_text(&w.view().calming_strip, i, &w.i18n(), w.units())
            }
            aria-description=move || state().is_some_and(|c| !c.1).then(|| w.i18n().tr("calming-cell-failing", &Args::new()))
            title=name
            style=move || {
                let lengths: Vec<i32> = w.view().calming_strip.cells.iter().map(|c| c.to_mm - c.from_mm).collect();
                format!("flex-grow: {:.4}", basis_by_length(&lengths).get(i).copied().unwrap_or(0.0))
            }
            on:pointerdown=move |e| on_down(e, i)
            on:click=move |_| w.select_calming(i)
        >
            <span class="calming-name">{name}</span>
        </div>
    }
}

/// The strip, under the curbside rows. Nothing for a street with no length to lay elements along.
#[component]
pub fn CalmingStrip(vm: Rc<super::vm::StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let count = move || w.view().calming_strip.cells.len();
    let any = move || count() > 0;
    let active = move || w.view().calming_strip.selected.map(CalmingRow::cell_id);
    let press = StoredValue::new_local(None::<Press>);
    let finish = move |keep: bool| {
        if let Some(p) = press.get_value() {
            press.set_value(None);
            if p.active {
                w.end_calming_drag(keep);
            }
        }
    };
    let on_down = move |ev: leptos::ev::PointerEvent, i: usize| {
        if ev.button() != 0 || !ev.is_primary() {
            return;
        }
        finish(false);
        w.select_calming(i);
        let Some(strip) = document().get_element_by_id(CalmingRow::STRIP_ID) else { return };
        let width = f64::from(strip.scroll_width());
        let span = drag_span(&w.view_now().calming_strip.cells, i, width);
        let Some(span) = span else { return };
        press.set_value(Some(Press { start_x: f64::from(ev.client_x()), span, active: false }));
        // The strip, not the cell, holds the pointer: the cells come and go as the stretches around the element change.
        let _ = strip.set_pointer_capture(ev.pointer_id());
        ev.prevent_default();
        if let Ok(strip) = strip.dyn_into::<HtmlElement>() {
            let _ = strip.focus();
        }
    };
    let on_move = move |ev: leptos::ev::PointerEvent| {
        let Some(mut p) = press.get_value() else { return };
        let dx = f64::from(ev.client_x()) - p.start_x;
        if !p.active {
            if !is_boundary_drag(dx) {
                return;
            }
            if !w.begin_calming_drag() {
                press.set_value(None);
                return;
            }
            p.active = true;
            press.set_value(Some(p));
        }
        w.drag_calming_to(dragged_from_mm(dx, &p.span));
    };
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let Some(cell_id) = active() else { return };
        let doc = document();
        if let (Some(cell), Some(strip)) = (doc.get_element_by_id(&cell_id), doc.get_element_by_id(CalmingRow::STRIP_ID)) {
            let (c, s) = (cell.get_bounding_client_rect(), strip.get_bounding_client_rect());
            let by = super::strip::scroll_into_view_px(c.left(), c.right(), s.left(), s.right());
            if by != 0.0 {
                strip.set_scroll_left(strip.scroll_left() + by.round() as i32);
            }
        }
    });
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" && press.get_value().is_some() {
            finish(false);
            return;
        }
        let Some((action, stop)) = on_strip(&ev.key(), ev.shift_key(), ev.alt_key(), ev.ctrl_key(), ev.meta_key()) else { return };
        if stop {
            ev.prevent_default();
        }
        match action {
            ZoneAction::Select(d) => w.select_calming_relative(d),
            ZoneAction::SelectEdge { last } => w.select_calming_edge(last),
            ZoneAction::Nudge(mm) => {
                w.nudge_calming(mm);
            }
        }
    };
    let heading = move || w.i18n().tr("st-calming-heading", &Args::new());
    let caption = move || w.i18n().tr("st-calming-caption", &Args::new());
    let label = move || w.i18n().tr("st-calming-strip-label", &Args::new());
    let cells = move || {
        view! {
            <For each=move || 0..count() key=|i| *i let:i>
                {cell(w, i, on_down)}
            </For>
        }
    };
    view! {
        <Show when=any>
            <section class="curb-strip-section" aria-labelledby="h-calming">
                <h2 id="h-calming" class="sr-only">{heading}</h2>
                <div class="curb-side">
                    <p class="zone-caption" title=caption>{caption}</p>
                    <div
                        id=CalmingRow::STRIP_ID
                        class="calming-strip"
                        role="listbox"
                        aria-orientation="horizontal"
                        tabindex="0"
                        aria-label=label
                        aria-activedescendant=active
                        on:keydown=on_key
                        on:pointermove=on_move
                        on:pointerup=move |_| finish(true)
                        on:pointercancel=move |_| finish(false)
                        on:lostpointercapture=move |_| finish(false)
                    >
                        {cells}
                    </div>
                </div>
            </section>
        </Show>
    }
}
