//! The Green row under the zone strip and the curbside rows: one bar for the street's length with a cell for each
//! tree and planting, on a line of its own when it lies beside another across the street. It follows the curbside
//! rows (`curb_strip.rs`): one listbox and one tab stop, the arrow keys choosing a cell and Alt with them moving the
//! element in it, and a drag of the cell moving it along the street.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlElement;

use super::keys::{ZoneAction, on_strip};
use super::model::{GreenCell, snap};
use super::strip::is_boundary_drag;
use super::text::green_cell_text;
use super::watch::SheetWatch;
use crate::shared::i18n::Args;

/// The height of a line of the row, in pixels: a cell is a target of at least 24 pixels (WCAG 2.5.8).
pub const TRACK_PX: u32 = 24;

/// How many lines the row shows before it scrolls.
pub const TRACKS_SHOWN: usize = 2;

/// Where a cell lies in the row: its place along the strip, at least 24 pixels wide and kept inside the strip, and
/// its line.
pub fn cell_style(c: &GreenCell, length_mm: i32) -> String {
    let length = f64::from(length_mm.max(1));
    let left = f64::from(c.from_mm) / length * 100.0;
    let width = f64::from(c.to_mm - c.from_mm) / length * 100.0;
    format!("left: min({left:.4}%, calc(100% - {TRACK_PX}px)); width: max({width:.4}%, {TRACK_PX}px); top: {}px", c.track as u32 * TRACK_PX)
}

/// Where an element starts when the pointer is `dx_px` from where it was pressed: snapped, and kept inside the
/// street. `mm_per_px` is the street's length over the width of the strip. What it meets on the way is the model's to
/// refuse.
pub fn dragged_green_from_mm(dx_px: f64, from_mm: i32, span_mm: i32, mm_per_px: f64, length_mm: i32) -> i32 {
    let high = (length_mm - span_mm).max(0);
    let mm = (f64::from(from_mm) + dx_px * mm_per_px).round().clamp(0.0, f64::from(high));
    snap(mm as i32).clamp(0, high)
}

/// A press on an element, held until it ends.
#[derive(Clone, Copy)]
struct Press {
    start_x: f64,
    from_mm: i32,
    span_mm: i32,
    mm_per_px: f64,
    length_mm: i32,
    /// Moved far enough to be a drag (`begin_green_drag` has been called and was taken).
    active: bool,
}

fn cell_of(w: SheetWatch, i: usize) -> Option<GreenCell> {
    w.view().green.cells.get(i).cloned()
}

fn green_cell(w: SheetWatch, i: usize, on_down: impl Fn(leptos::ev::PointerEvent, usize) + Copy + 'static) -> impl IntoView {
    let name = move || {
        let (i18n, units) = (w.i18n(), w.units());
        cell_of(w, i).map(|c| crate::shared::said::say(&i18n, units, &c.label)).unwrap_or_default()
    };
    let failing = move || cell_of(w, i).is_some_and(|c| c.failing);
    view! {
        <div
            class="green-cell"
            class:failing=failing
            id=format!("green-{i}")
            role="option"
            aria-selected=move || (w.view().green.selected == Some(i)).to_string()
            aria-label=move || {
                let _ = w.i18n().locale();
                green_cell_text(&w.view().green, i, &w.i18n(), w.units())
            }
            aria-description=move || failing().then(|| w.i18n().tr("curb-cell-failing", &Args::new()))
            title=name
            style=move || {
                let v = w.view();
                v.green.cells.get(i).map(|c| cell_style(c, v.green.length_mm)).unwrap_or_default()
            }
            on:pointerdown=move |e| on_down(e, i)
            on:click=move |_| w.select_green(i)
        >
            <span class="green-name">{name}</span>
        </div>
    }
}

/// The Green row. Nothing for a street with no trees or planting.
#[component]
pub fn GreenStrip(vm: Rc<super::vm::StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let text = move |key: &'static str| w.i18n().tr(key, &Args::new());
    let count = move || w.view().green.cells.len();
    let has = move || count() > 0;
    let active = move || w.view().green.selected.map(|i| format!("green-{i}"));

    // A press on an element selects it; once it has moved a few pixels it drags the element. The strip holds the
    // pointer, not the cell, because the cells come and go as the elements do.
    let press = StoredValue::new_local(None::<Press>);
    let finish = move |keep: bool| {
        if let Some(p) = press.get_value() {
            press.set_value(None);
            if p.active {
                w.end_green_drag(keep);
            }
        }
    };
    let on_down = move |ev: leptos::ev::PointerEvent, i: usize| {
        if ev.button() != 0 || !ev.is_primary() {
            return;
        }
        finish(false);
        w.select_green(i);
        let Some(strip) = document().get_element_by_id("green-strip") else { return };
        let width = f64::from(strip.client_width());
        let (cell, length_mm) = {
            let v = w.view_now();
            (v.green.cells.get(i).cloned(), v.green.length_mm)
        };
        let Some(cell) = cell else { return };
        if width <= 0.0 || length_mm <= 0 {
            return;
        }
        let mm_per_px = f64::from(length_mm) / width;
        press.set_value(Some(Press {
            start_x: f64::from(ev.client_x()),
            from_mm: cell.from_mm,
            span_mm: cell.to_mm - cell.from_mm,
            mm_per_px,
            length_mm,
            active: false,
        }));
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
            if !w.begin_green_drag() {
                press.set_value(None);
                return;
            }
            p.active = true;
            press.set_value(Some(p));
        }
        w.drag_green_to(dragged_green_from_mm(dx, p.from_mm, p.span_mm, p.mm_per_px, p.length_mm));
    };
    // The chosen cell is brought into view inside the strip (its own scroll, never the page's).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let Some(id) = active() else { return };
        let doc = document();
        if let (Some(cell), Some(strip)) = (doc.get_element_by_id(&id), doc.get_element_by_id("green-strip"))
            && let (Ok(cell), Ok(strip)) = (cell.dyn_into::<HtmlElement>(), strip.dyn_into::<HtmlElement>())
        {
            let (top, height) = (cell.offset_top(), strip.client_height());
            if top < strip.scroll_top() {
                strip.set_scroll_top(top);
            } else if top + TRACK_PX as i32 > strip.scroll_top() + height {
                strip.set_scroll_top(top + TRACK_PX as i32 - height);
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
            ZoneAction::Select(d) => w.select_green_relative(d),
            ZoneAction::SelectEdge { last } => w.select_green_edge(last),
            ZoneAction::Nudge(mm) => {
                w.nudge_green(mm);
            }
        }
    };
    // Reads nothing itself (the count is read inside `each`), so it runs once and the cells are kept.
    let cells = move || {
        view! {
            <For each=move || 0..count() key=|i| *i let:i>
                {green_cell(w, i, on_down)}
            </For>
        }
    };
    view! {
        <Show when=has>
            <section class="curb-strip-section" aria-labelledby="h-green">
                <h2 id="h-green" class="sr-only">{move || text("st-green-heading")}</h2>
                <div class="curb-side">
                    <p class="zone-caption" title=move || text("st-green-caption")>{move || text("st-green-caption")}</p>
                    <div
                        id="green-strip"
                        class="green-strip"
                        role="listbox"
                        aria-orientation="horizontal"
                        tabindex="0"
                        aria-label=move || text("st-green-strip-label")
                        aria-activedescendant=active
                        style=move || format!("--tracks: {}", w.view().green.tracks.clamp(1, TRACKS_SHOWN))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::said::Said;
    use crate::street::model::GreenKind;

    fn cell(from_mm: i32, to_mm: i32, track: usize) -> GreenCell {
        GreenCell {
            index: 0,
            from_mm,
            to_mm,
            track,
            kind: GreenKind::Tree,
            size: None,
            width_mm: 1_900,
            host: Some(0),
            fine_mm: 0,
            label: Said::new("green-instance-off"),
            failing: false,
        }
    }

    #[test]
    fn a_cell_lies_by_its_place_along_the_street_and_on_its_line() {
        let s = cell_style(&cell(15_000, 30_000, 1), 60_000);
        assert_eq!(s, "left: min(25.0000%, calc(100% - 24px)); width: max(25.0000%, 24px); top: 24px");
    }

    #[test]
    fn a_short_cell_keeps_a_target_and_one_at_the_end_stays_inside() {
        let s = cell_style(&cell(58_500, 60_000, 0), 60_000);
        assert!(s.contains("width: max(2.5000%, 24px)") && s.contains("left: min(97.5000%, calc(100% - 24px))"), "{s}");
    }

    #[test]
    fn the_pointer_places_the_element_snapped_and_inside_the_street() {
        // 60 m over 600 px: 100 mm a pixel.
        assert_eq!(dragged_green_from_mm(0.0, 10_000, 1_900, 100.0, 60_000), 10_000);
        assert_eq!(dragged_green_from_mm(10.0, 10_000, 1_900, 100.0, 60_000), 11_000);
        assert_eq!(dragged_green_from_mm(10.4, 10_000, 1_900, 100.0, 60_000), 11_000, "to the grid");
        assert_eq!(dragged_green_from_mm(-500.0, 10_000, 1_900, 100.0, 60_000), 0);
        assert_eq!(dragged_green_from_mm(5_000.0, 10_000, 1_900, 100.0, 60_000), 58_100, "its far end stops at the street's");
    }
}
