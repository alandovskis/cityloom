//! The curbside strips: for each side of the street that has a curbside lane, a bar for the street's length with one
//! cell per use and one per gap between them, and the pure decisions a drag of a use carries out. It follows the zone
//! strip (`strip.rs`): one listbox and one tab stop, the arrow keys choosing a cell and Alt with them moving the use.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlElement;

use super::keys::{ZoneAction, on_strip};
use super::model::{CurbCell, SNAP_MM, snap};
use super::strip::{basis_by_length, is_boundary_drag};
use super::text::curb_cell_text;
use super::watch::SheetWatch;
use crate::shared::catalogue::Side;
use crate::shared::i18n::Args;

/// The word a side is spoken and named by, and the part of the strip's and cells' ids.
fn side_word(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
    }
}

fn strip_id(side: Side) -> String {
    format!("curb-strip-{}", side_word(side))
}

fn cell_id(side: Side, index: usize) -> String {
    format!("curb-{}-{index}", side_word(side))
}

/// What a drag of one use works from, as measured when the press began.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragSpan {
    /// Where the use starts.
    pub from_mm: i32,
    /// How many millimetres of street a pixel of the use's own cell stands for.
    pub mm_per_px: f64,
    /// The lowest and the highest start it may take without meeting another use or an end of the street.
    pub low_mm: i32,
    pub high_mm: i32,
}

/// The span of the use in cell `cell` when its cell is `width_px` wide. `None` for a gap, a cell that is not there
/// or one with no width. It may move into the gap before it and the gap after it, never over a neighbouring use.
pub fn drag_span(cells: &[CurbCell], cell: usize, width_px: f64) -> Option<DragSpan> {
    let c = cells.get(cell).filter(|c| c.instance.is_some())?;
    let length = c.to_mm - c.from_mm;
    if width_px <= 0.0 || length <= 0 {
        return None;
    }
    let low_mm = match cell.checked_sub(1).and_then(|p| cells.get(p)) {
        Some(p) if p.instance.is_none() => p.from_mm,
        _ => c.from_mm,
    };
    let reach = match cells.get(cell + 1) {
        Some(n) if n.instance.is_none() => n.to_mm,
        _ => c.to_mm,
    };
    // The highest start is the last multiple of the grid at which it still fits.
    let high_mm = (reach - length).div_euclid(SNAP_MM) * SNAP_MM;
    Some(DragSpan { from_mm: c.from_mm, mm_per_px: f64::from(length) / width_px, low_mm, high_mm: high_mm.max(c.from_mm) })
}

/// Where the use starts when the pointer is `dx_px` from where it was pressed: snapped, and kept within the span.
pub fn dragged_from_mm(dx_px: f64, s: &DragSpan) -> i32 {
    let mm = f64::from(s.from_mm) + dx_px * s.mm_per_px;
    let (low, high) = (s.low_mm.min(s.high_mm), s.high_mm);
    snap(mm.round().clamp(f64::from(low), f64::from(high)) as i32).clamp(low, high)
}

/// A press on a use, held until it ends.
#[derive(Clone, Copy)]
struct Press {
    start_x: f64,
    span: DragSpan,
    /// Moved far enough to be a drag (`begin_curb_drag` has been called and was taken).
    active: bool,
}

/// The cell `i` of `side`'s strip. Every attribute follows the view, so the elements stay while the cells change.
fn curb_cell(w: SheetWatch, side: Side, i: usize, on_down: impl Fn(leptos::ev::PointerEvent, usize) + Copy + 'static) -> impl IntoView {
    let cell = move || w.view().curb.sides.iter().find(|s| s.side == side).and_then(|s| s.cells.get(i).map(|c| (c.instance.is_some(), c.ok)));
    let name = move || {
        let (i18n, units) = (w.i18n(), w.units());
        w.view()
            .curb
            .sides
            .iter()
            .find(|s| s.side == side)
            .and_then(|s| s.cells.get(i))
            .map(|c| crate::shared::said::say(&i18n, units, &c.label))
            .unwrap_or_default()
    };
    view! {
        <div
            class="curb-cell"
            class:gap=move || cell().is_some_and(|c| !c.0)
            class:failing=move || cell().is_some_and(|c| !c.1)
            id=cell_id(side, i)
            role="option"
            aria-selected=move || w.view().curb.sides.iter().find(|s| s.side == side).is_some_and(|s| s.selected == Some(i)).to_string()
            aria-label=move || {
                let _ = w.i18n().locale();
                w.view().curb.sides.iter().find(|s| s.side == side).map(|s| curb_cell_text(s, i, &w.i18n(), w.units())).unwrap_or_default()
            }
            aria-description=move || cell().is_some_and(|c| !c.1).then(|| w.i18n().tr("curb-cell-failing", &Args::new()))
            title=name
            style=move || {
                let lengths: Vec<i32> =
                    w.view().curb.sides.iter().find(|s| s.side == side).map(|s| s.cells.iter().map(|c| c.to_mm - c.from_mm).collect()).unwrap_or_default();
                format!("flex-grow: {:.4}", basis_by_length(&lengths).get(i).copied().unwrap_or(0.0))
            }
            on:pointerdown=move |e| on_down(e, i)
            on:click=move |_| w.select_curb(side, i)
        >
            <span class="curb-name">{name}</span>
        </div>
    }
}

/// One side's strip, under the zone strip.
#[component]
fn CurbStrip(w: SheetWatch, side: Side) -> impl IntoView {
    let id = strip_id(side);
    let side_text = move || w.i18n().tr(if side == Side::Left { "side-word-left" } else { "side-word-right" }, &Args::new());
    let caption = move || w.i18n().tr("st-curb-caption", &Args::new().str("side", side_text()));
    let strip_label = move || w.i18n().tr("st-curb-strip-label", &Args::new().str("side", side_text()));
    let selected = move || w.view().curb.sides.iter().find(|s| s.side == side).and_then(|s| s.selected);
    let active = move || selected().map(|i| cell_id(side, i));
    let count = move || w.view().curb.sides.iter().find(|s| s.side == side).map_or(0, |s| s.cells.len());

    // A press on a use selects it; once it has moved a few pixels it drags the use. The strip holds the pointer, not
    // the cell, because the cells come and go as the gaps around the use change during the drag.
    let press = StoredValue::new_local(None::<Press>);
    let finish = move |keep: bool| {
        if let Some(p) = press.get_value() {
            press.set_value(None);
            if p.active {
                w.end_curb_drag(keep);
            }
        }
    };
    let on_down = move |ev: leptos::ev::PointerEvent, i: usize| {
        if ev.button() != 0 || !ev.is_primary() {
            return;
        }
        finish(false);
        w.select_curb(side, i);
        let (cell, strip) = (ev.current_target().and_then(|t| t.dyn_into::<HtmlElement>().ok()), document().get_element_by_id(&strip_id(side)));
        let (Some(cell), Some(strip)) = (cell, strip) else { return };
        let width = cell.get_bounding_client_rect().width();
        let Some(span) = w.view_now().curb.sides.iter().find(|s| s.side == side).and_then(|s| drag_span(&s.cells, i, width)) else { return };
        press.set_value(Some(Press { start_x: f64::from(ev.client_x()), span, active: false }));
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
            if !w.begin_curb_drag() {
                press.set_value(None);
                return;
            }
            p.active = true;
            press.set_value(Some(p));
        }
        w.drag_curb_to(dragged_from_mm(dx, &p.span));
    };
    // The chosen cell is brought into view inside the strip (its own scroll, never the page's).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let Some(cell_id) = active() else { return };
        let doc = document();
        if let (Some(cell), Some(strip)) = (doc.get_element_by_id(&cell_id), doc.get_element_by_id(&strip_id(side))) {
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
            ZoneAction::Select(d) => w.select_curb_relative(side, d),
            ZoneAction::SelectEdge { last } => w.select_curb_edge(side, last),
            ZoneAction::Nudge(mm) => {
                w.nudge_curb(mm);
            }
        }
    };
    // Reads nothing itself (the list is read inside `each`), so it runs once and the cells are kept.
    let cells = move || {
        view! {
            <For each=move || 0..count() key=|i| *i let:i>
                {curb_cell(w, side, i, on_down)}
            </For>
        }
    };
    view! {
        <div class="curb-side">
            <p class="zone-caption" title=caption>{caption}</p>
            <div
                id=id
                class="curb-strip"
                role="listbox"
                aria-orientation="horizontal"
                tabindex="0"
                aria-label=strip_label
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
    }
}

/// The curbside strips, one for each side that has a curbside lane: nothing for a street with none.
#[component]
pub fn CurbStrips(vm: Rc<super::vm::StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let heading = move || w.i18n().tr("st-curb-heading", &Args::new());
    let sides = move || w.view().curb.sides.iter().map(|s| s.side).collect::<Vec<_>>();
    view! {
        <Show when=move || !sides().is_empty()>
            <section class="curb-strip-section" aria-labelledby="h-curb">
                <h2 id="h-curb" class="sr-only">{heading}</h2>
                <For each=sides key=|s| *s let:side>
                    <CurbStrip w=w side=side/>
                </For>
            </section>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::said::Said;

    fn cell(instance: Option<usize>, from_mm: i32, to_mm: i32) -> CurbCell {
        CurbCell { instance, from_mm, to_mm, use_: None, label: Said::new("curb-gap"), ok: true }
    }

    /// A gap, a use of 6 m and a gap, over 60 m, then a use against the end.
    fn cells() -> Vec<CurbCell> {
        vec![cell(None, 0, 10_000), cell(Some(0), 10_000, 16_000), cell(None, 16_000, 50_000), cell(Some(1), 50_000, 60_000)]
    }

    #[test]
    fn a_use_can_move_into_the_gaps_beside_it_and_no_further() {
        let s = drag_span(&cells(), 1, 60.0).unwrap();
        assert_eq!((s.low_mm, s.high_mm), (0, 44_000), "from the start of the gap before to where it still fits in the gap after");
        assert!((s.mm_per_px - 100.0).abs() < 1e-9);
        let end = drag_span(&cells(), 3, 40.0).unwrap();
        assert_eq!((end.low_mm, end.high_mm), (16_000, 50_000), "it cannot move past the end");
        assert_eq!(drag_span(&cells(), 0, 60.0), None, "a gap is not dragged");
        assert_eq!(drag_span(&cells(), 9, 60.0), None);
        assert_eq!(drag_span(&cells(), 1, 0.0), None, "a cell with no width");
    }

    #[test]
    fn a_use_beside_another_cannot_move_toward_it() {
        let c = vec![cell(Some(0), 0, 6_000), cell(Some(1), 6_000, 12_000), cell(None, 12_000, 20_000)];
        let s = drag_span(&c, 1, 60.0).unwrap();
        assert_eq!((s.low_mm, s.high_mm), (6_000, 14_000));
    }

    #[test]
    fn the_high_end_is_on_the_grid_when_the_street_is_not() {
        let c = vec![cell(Some(0), 0, 6_000), cell(None, 6_000, 20_050)];
        assert_eq!(drag_span(&c, 0, 60.0).unwrap().high_mm, 14_000);
    }

    #[test]
    fn the_pointer_places_the_use_snapped_and_within_the_span() {
        let s = DragSpan { from_mm: 10_000, mm_per_px: 100.0, low_mm: 0, high_mm: 44_000 };
        assert_eq!(dragged_from_mm(0.0, &s), 10_000);
        assert_eq!(dragged_from_mm(10.0, &s), 11_000);
        assert_eq!(dragged_from_mm(10.4, &s), 11_000, "to the grid");
        assert_eq!(dragged_from_mm(-500.0, &s), 0);
        assert_eq!(dragged_from_mm(500.0, &s), 44_000);
        let fixed = DragSpan { low_mm: 10_000, high_mm: 10_000, ..s };
        assert_eq!(dragged_from_mm(30.0, &fixed), 10_000, "no room: it stays");
    }

    #[test]
    fn the_basis_by_length_is_the_zone_strips_rule() {
        let b = basis_by_length(&[5_000, 2_000_000]);
        assert!(b[0] >= 0.06 - 1e-9 && b[1] > b[0]);
        assert!(basis_by_length(&[]).is_empty());
    }
}
