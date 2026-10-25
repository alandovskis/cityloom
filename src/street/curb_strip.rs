//! The strips along the street under the zone strip: for each side that can have one, a bar for the street's length
//! with one cell per curbside use and one per gap between them, and under it the same for the transit stops. Both are
//! a `Row`; the pure decisions a drag carries out are written once. It follows the zone strip (`strip.rs`): one
//! listbox and one tab stop, the arrow keys choosing a cell and Alt with them moving the thing in it.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlElement;

use super::keys::{ZoneAction, on_strip};
use super::model::{SNAP_MM, StripCell, snap};
use super::row::{CurbRow, Row, StopRow, cell_id, strip_id};
use super::strip::{basis_by_length, is_boundary_drag};
use super::text::cell_text;
use super::watch::SheetWatch;
use crate::shared::catalogue::Side;
use crate::shared::i18n::Args;

/// What a drag of one thing works from, as measured when the press began.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragSpan {
    /// Where the thing starts.
    pub from_mm: i32,
    /// How many millimetres of street a pixel of the whole strip stands for.
    pub mm_per_px: f64,
    /// The lowest and the highest start it may take without meeting another thing or an end of the street.
    pub low_mm: i32,
    pub high_mm: i32,
}

/// The span of the thing in cell `cell` when the whole strip is `strip_px` wide. The scale is the strip's (the street's
/// length over its width), not its own cell's, which the minimum width of a short cell would skew. `None` for a
/// gap, a cell that is not there or a strip with no width. It may move into the gap before it and the gap after it, never over a neighbouring thing.
pub fn drag_span<C: StripCell>(cells: &[C], cell: usize, strip_px: f64) -> Option<DragSpan> {
    let c = cells.get(cell).filter(|c| c.instance().is_some())?;
    let length = c.to_mm() - c.from_mm();
    let street_mm = cells.last().map_or(0, |l| l.to_mm()) - cells.first().map_or(0, |f| f.from_mm());
    if strip_px <= 0.0 || length <= 0 || street_mm <= 0 {
        return None;
    }
    let low_mm = match cell.checked_sub(1).and_then(|p| cells.get(p)) {
        Some(p) if p.instance().is_none() => p.from_mm(),
        _ => c.from_mm(),
    };
    let reach = match cells.get(cell + 1) {
        Some(n) if n.instance().is_none() => n.to_mm(),
        _ => c.to_mm(),
    };
    // The highest start is the last multiple of the grid at which it still fits.
    let high_mm = (reach - length).div_euclid(SNAP_MM) * SNAP_MM;
    Some(DragSpan { from_mm: c.from_mm(), mm_per_px: f64::from(street_mm) / strip_px, low_mm, high_mm: high_mm.max(c.from_mm()) })
}

/// Where the thing starts when the pointer is `dx_px` from where it was pressed: snapped, and kept within the span.
pub fn dragged_from_mm(dx_px: f64, s: &DragSpan) -> i32 {
    let mm = f64::from(s.from_mm) + dx_px * s.mm_per_px;
    let (low, high) = (s.low_mm.min(s.high_mm), s.high_mm);
    snap(mm.round().clamp(f64::from(low), f64::from(high)) as i32).clamp(low, high)
}

/// A press on a thing, held until it ends.
#[derive(Clone, Copy)]
struct Press {
    start_x: f64,
    span: DragSpan,
    /// Moved far enough to be a drag (`begin_cell_drag` has been called and was taken).
    active: bool,
}

/// The chosen cell of `side`'s strip in the row, if the row has the side and one is chosen.
fn selected_of<R: Row>(w: SheetWatch, side: Side) -> Option<usize> {
    let v = w.view();
    R::strips(&v).sides.iter().find(|s| s.side == side)?.selected
}

/// The cell `i` of `side`'s strip. Every attribute follows the view, so the elements stay while the cells change.
fn strip_cell<R: Row>(w: SheetWatch, side: Side, i: usize, on_down: impl Fn(leptos::ev::PointerEvent, usize) + Copy + 'static) -> impl IntoView {
    let cell = move || {
        let v = w.view();
        R::strips(&v).sides.iter().find(|s| s.side == side).and_then(|s| s.cells.get(i).map(|c| (c.instance().is_some(), c.ok())))
    };
    let name = move || {
        let (i18n, units) = (w.i18n(), w.units());
        let v = w.view();
        R::strips(&v)
            .sides
            .iter()
            .find(|s| s.side == side)
            .and_then(|s| s.cells.get(i))
            .map(|c| crate::shared::said::say(&i18n, units, c.label()))
            .unwrap_or_default()
    };
    let id = cell_id::<R>(side, i);
    let choose = move |_| w.select_cell::<R>(side, i);
    view! {
        <div
            class=format!("{}-cell", R::ID)
            class:gap=move || cell().is_some_and(|c| !c.0)
            class:failing=move || cell().is_some_and(|c| !c.1)
            id=id
            role="option"
            aria-selected=move || R::strips(&w.view()).sides.iter().find(|s| s.side == side).is_some_and(|s| s.selected == Some(i)).to_string()
            aria-label=move || {
                let _ = w.i18n().locale();
                let v = w.view();
                R::strips(&v).sides.iter().find(|s| s.side == side).map(|s| cell_text(R::CELL_KEY, s, i, &w.i18n(), w.units())).unwrap_or_default()
            }
            aria-description=move || cell().is_some_and(|c| !c.1).then(|| w.i18n().tr("curb-cell-failing", &Args::new()))
            title=name
            style=move || {
                let v = w.view();
                let lengths: Vec<i32> =
                    R::strips(&v).sides.iter().find(|s| s.side == side).map(|s| s.cells.iter().map(|c| c.to_mm() - c.from_mm()).collect()).unwrap_or_default();
                format!("flex-grow: {:.4}", basis_by_length(&lengths).get(i).copied().unwrap_or(0.0))
            }
            on:pointerdown=move |e| on_down(e, i)
            on:click=choose
        >
            <span class=format!("{}-name", R::ID)>{name}</span>
        </div>
    }
}

/// One side's strip of the row, under the zone strip (or the curbside strip of the same side).
fn strip<R: Row>(w: SheetWatch, side: Side) -> impl IntoView {
    let id = strip_id::<R>(side);
    let side_text = move || w.i18n().tr(if side == Side::Left { "side-word-left" } else { "side-word-right" }, &Args::new());
    let caption = move || w.i18n().tr(R::CAPTION_KEY, &Args::new().str("side", side_text()));
    let strip_label = move || w.i18n().tr(R::STRIP_LABEL_KEY, &Args::new().str("side", side_text()));
    let active = move || selected_of::<R>(w, side).map(|i| cell_id::<R>(side, i));
    let count = move || {
        let v = w.view();
        R::strips(&v).sides.iter().find(|s| s.side == side).map_or(0, |s| s.cells.len())
    };

    // A press on a thing selects it; once it has moved a few pixels it drags the thing. The strip holds the pointer,
    // not the cell, because the cells come and go as the gaps around the thing change during the drag.
    let press = StoredValue::new_local(None::<Press>);
    let finish = move |keep: bool| {
        if let Some(p) = press.get_value() {
            press.set_value(None);
            if p.active {
                w.end_cell_drag(keep);
            }
        }
    };
    let on_down = move |ev: leptos::ev::PointerEvent, i: usize| {
        if ev.button() != 0 || !ev.is_primary() {
            return;
        }
        finish(false);
        w.select_cell::<R>(side, i);
        let Some(strip) = document().get_element_by_id(&strip_id::<R>(side)) else { return };
        let width = f64::from(strip.scroll_width());
        let span = {
            let v = w.view_now();
            R::strips(&v).sides.iter().find(|s| s.side == side).and_then(|s| drag_span(&s.cells, i, width))
        };
        let Some(span) = span else { return };
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
            if !w.begin_cell_drag::<R>() {
                press.set_value(None);
                return;
            }
            p.active = true;
            press.set_value(Some(p));
        }
        w.drag_cell_to::<R>(dragged_from_mm(dx, &p.span));
    };
    // The chosen cell is brought into view inside the strip (its own scroll, never the page's).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let Some(cell_id) = active() else { return };
        let doc = document();
        if let (Some(cell), Some(strip)) = (doc.get_element_by_id(&cell_id), doc.get_element_by_id(&strip_id::<R>(side))) {
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
            ZoneAction::Select(d) => w.select_cell_relative::<R>(side, d),
            ZoneAction::SelectEdge { last } => w.select_cell_edge::<R>(side, last),
            ZoneAction::Nudge(mm) => {
                w.nudge_cell::<R>(mm);
            }
        }
    };
    // Reads nothing itself (the list is read inside `each`), so it runs once and the cells are kept.
    let cells = move || {
        view! {
            <For each=move || 0..count() key=|i| *i let:i>
                {strip_cell::<R>(w, side, i, on_down)}
            </For>
        }
    };
    view! {
        <div class="curb-side">
            <p class="zone-caption" title=caption>{caption}</p>
            <div
                id=id
                class=format!("{}-strip", R::ID)
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

/// The strips, for each side that has one: the curbside strip, then the stops' strip under it. Nothing for a street
/// with none.
#[component]
pub fn CurbStrips(vm: Rc<super::vm::StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    // The heading names the curbside strips; a street with only stops rows is headed by them.
    let heading = move || w.i18n().tr(if w.view().curb.sides.is_empty() { "st-stop-heading" } else { "st-curb-heading" }, &Args::new());
    let has = |side: Side, sides: Vec<Side>| sides.contains(&side);
    let curb_sides = move || w.view().curb.sides.iter().map(|s| s.side).collect::<Vec<_>>();
    let stop_sides = move || w.view().stops.sides.iter().map(|s| s.side).collect::<Vec<_>>();
    let sides = move || [Side::Left, Side::Right].into_iter().filter(|s| has(*s, curb_sides()) || has(*s, stop_sides())).collect::<Vec<_>>();
    view! {
        <Show when=move || !sides().is_empty()>
            <section class="curb-strip-section" aria-labelledby="h-curb">
                <h2 id="h-curb" class="sr-only">{heading}</h2>
                <For each=sides key=|s| *s let:side>
                    <Show when=move || has(side, curb_sides())>{strip::<CurbRow>(w, side)}</Show>
                    <Show when=move || has(side, stop_sides())>{strip::<StopRow>(w, side)}</Show>
                </For>
            </section>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::said::Said;
    use crate::street::model::CurbCell;

    fn cell(instance: Option<usize>, from_mm: i32, to_mm: i32) -> CurbCell {
        CurbCell { instance, from_mm, to_mm, use_: None, label: Said::new("curb-gap"), ok: true }
    }

    /// A gap, a use of 6 m and a gap, over 60 m, then a use against the end.
    fn cells() -> Vec<CurbCell> {
        vec![cell(None, 0, 10_000), cell(Some(0), 10_000, 16_000), cell(None, 16_000, 50_000), cell(Some(1), 50_000, 60_000)]
    }

    #[test]
    fn a_use_can_move_into_the_gaps_beside_it_and_no_further() {
        let s = drag_span(&cells(), 1, 600.0).unwrap();
        assert_eq!((s.low_mm, s.high_mm), (0, 44_000), "from the start of the gap before to where it still fits in the gap after");
        assert!((s.mm_per_px - 100.0).abs() < 1e-9);
        let end = drag_span(&cells(), 3, 600.0).unwrap();
        assert_eq!((end.low_mm, end.high_mm), (16_000, 50_000), "it cannot move past the end");
        assert_eq!(drag_span(&cells(), 0, 600.0), None, "a gap is not dragged");
        assert_eq!(drag_span(&cells(), 9, 600.0), None);
        assert_eq!(drag_span(&cells(), 1, 0.0), None, "a strip with no width");
    }

    #[test]
    fn the_scale_is_the_strips_whatever_the_width_of_a_short_cell() {
        // A 2 m use on a 60 m street, in a strip of 600 px: 100 mm a pixel, though its own cell is held at 24 px.
        let c = vec![cell(None, 0, 10_000), cell(Some(0), 10_000, 12_000), cell(None, 12_000, 60_000)];
        let s = drag_span(&c, 1, 600.0).unwrap();
        assert!((s.mm_per_px - 100.0).abs() < 1e-9, "{s:?}");
        assert_eq!(dragged_from_mm(10.0, &s), 11_000);
    }

    #[test]
    fn a_use_beside_another_cannot_move_toward_it() {
        let c = vec![cell(Some(0), 0, 6_000), cell(Some(1), 6_000, 12_000), cell(None, 12_000, 20_000)];
        let s = drag_span(&c, 1, 200.0).unwrap();
        assert_eq!((s.low_mm, s.high_mm), (6_000, 14_000));
    }

    #[test]
    fn the_high_end_is_on_the_grid_when_the_street_is_not() {
        let c = vec![cell(Some(0), 0, 6_000), cell(None, 6_000, 20_050)];
        assert_eq!(drag_span(&c, 0, 200.0).unwrap().high_mm, 14_000);
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

    fn stop_cell(instance: Option<usize>, from_mm: i32, to_mm: i32) -> crate::street::model::StopCell {
        crate::street::model::StopCell {
            instance,
            from_mm,
            to_mm,
            kind: instance.map(|_| crate::street::model::StopKind::Curb),
            shelter: false,
            name: None,
            label: Said::new("curb-gap"),
            ok: true,
        }
    }

    #[test]
    fn a_stop_is_dragged_within_the_free_stretch_between_its_neighbours() {
        // Stops of 9 m at 10 m and at 40 m, with a gap on each side and between them, over 60 m.
        let cells = vec![
            stop_cell(None, 0, 10_000),
            stop_cell(Some(0), 10_000, 19_000),
            stop_cell(None, 19_000, 40_000),
            stop_cell(Some(1), 40_000, 49_000),
            stop_cell(None, 49_000, 60_000),
        ];
        let first = drag_span(&cells, 1, 600.0).unwrap();
        assert_eq!((first.low_mm, first.high_mm), (0, 31_000), "from the start of the street to where it still fits before the next stop");
        let second = drag_span(&cells, 3, 600.0).unwrap();
        assert_eq!((second.low_mm, second.high_mm), (19_000, 51_000));
        assert_eq!(dragged_from_mm(10_000.0, &first), 31_000, "the pointer far past the next stop stops at it");
        assert_eq!(dragged_from_mm(-10_000.0, &second), 19_000);
        assert_eq!(drag_span(&cells, 2, 600.0), None, "a gap is not dragged");
        let touching = vec![stop_cell(Some(0), 0, 9_000), stop_cell(Some(1), 9_000, 18_000), stop_cell(None, 18_000, 30_000)];
        let s = drag_span(&touching, 1, 300.0).unwrap();
        assert_eq!((s.low_mm, s.high_mm), (9_000, 21_000), "it cannot move toward a stop it touches");
    }

    #[test]
    fn the_basis_by_length_is_the_zone_strips_rule() {
        let b = basis_by_length(&[5_000, 2_000_000]);
        assert!(b[0] >= 0.06 - 1e-9 && b[1] > b[0]);
        assert!(basis_by_length(&[]).is_empty());
    }
}
