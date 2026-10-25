//! The zone strip: a bar for the street's length with one cell per zone, and the pure decisions it carries out.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{Element, HtmlElement};

use super::keys::{ZoneAction, on_strip};
use super::model::{MIN_ZONE_MM, ZoneEnd, ZoneView, snap};
use super::text::zone_text;
use super::vm::StreetVm;
use super::watch::SheetWatch;
use crate::city::model::EndView;
use crate::shared::i18n::Args;
use crate::shared::said::say;

/// A press moves a boundary only after it has moved this far.
const DRAG_START_PX: f64 = 4.0;
/// No zone's cell is narrower than this share of the strip, so a short zone of a long street is still a target.
const MIN_SHARE: f64 = 0.06;

/// The two cells a handle separates, as measured on the screen when a press began.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairSpan {
    /// Where the left cell begins.
    pub left_px: f64,
    /// The two cells' widths together, without the handle between them.
    pub cells_px: f64,
    /// The handle's width.
    pub handle_px: f64,
    /// Where the left cell begins and the right cell ends along the street.
    pub from_mm: i32,
    pub to_mm: i32,
}

/// Where the boundary between the two cells goes when the handle's centre is at `centre_px`: the handle's left edge
/// divides the cells' pixels in the proportion of their mm, whatever other cells or minimum widths do to the rest of
/// the strip. Snapped, and kept so that both zones stay at least `MIN_ZONE_MM`.
pub fn pair_boundary_mm(centre_px: f64, s: &PairSpan) -> i32 {
    if s.cells_px <= 0.0 || s.to_mm <= s.from_mm {
        return s.from_mm;
    }
    let left_cell = centre_px - s.handle_px / 2.0 - s.left_px;
    let mm = s.from_mm as f64 + left_cell / s.cells_px * f64::from(s.to_mm - s.from_mm);
    let (low, high) = (s.from_mm + MIN_ZONE_MM, s.to_mm - MIN_ZONE_MM);
    if low > high {
        return s.from_mm;
    }
    snap(mm.round().clamp(f64::from(low), f64::from(high)) as i32).clamp(low, high)
}

/// The flex-grow of each zone's cell, as shares that add up to 1: in proportion to its length, but never under
/// `MIN_SHARE` (the cells that would be are pinned at it and the rest share what is left by length). When the
/// zones are too many for the floor to hold, every cell gets an equal share. The target size in pixels is not
/// decided here: it is the view's CSS `min-width` on each cell (the strip scrolls when they do not fit).
pub fn cell_basis(zones: &[ZoneView]) -> Vec<f64> {
    basis_by_length(&zones.iter().map(|z| z.to_mm - z.from_mm).collect::<Vec<_>>())
}

/// The same for any strip of cells, from their lengths (the curbside strips use it).
pub fn basis_by_length(lengths: &[i32]) -> Vec<f64> {
    let n = lengths.len();
    if n == 0 {
        return Vec::new();
    }
    let equal = 1.0 / n as f64;
    if n as f64 * MIN_SHARE >= 1.0 {
        return vec![equal; n];
    }
    let len: Vec<f64> = lengths.iter().map(|l| f64::from((*l).max(0))).collect();
    let mut pinned = vec![false; n];
    loop {
        let free: Vec<usize> = (0..n).filter(|&i| !pinned[i]).collect();
        let left = 1.0 - (n - free.len()) as f64 * MIN_SHARE;
        let sum: f64 = free.iter().map(|&i| len[i]).sum();
        let share = |i: usize| if sum > 0.0 { left * len[i] / sum } else { left / free.len() as f64 };
        let short: Vec<usize> = free.iter().copied().filter(|&i| share(i) < MIN_SHARE).collect();
        if short.is_empty() {
            return (0..n).map(|i| if pinned[i] { MIN_SHARE } else { share(i) }).collect();
        }
        for i in short {
            pinned[i] = true;
        }
    }
}

/// Whether a press on a boundary has moved far enough to be a drag.
pub fn is_boundary_drag(dx_px: f64) -> bool {
    dx_px.abs() >= DRAG_START_PX
}

/// The id of the cell of zone `index`.
fn cell_id(index: usize) -> String {
    format!("zone-{index}")
}

/// A drag of a boundary, from a press on its handle.
#[derive(Clone, Copy)]
struct Press {
    start_x: f64,
    /// The pointer's distance from the handle's centre at the press: the boundary follows the handle, not the pointer.
    grab_px: f64,
    /// The two cells the handle separates.
    span: PairSpan,
    /// Moved far enough to be a drag (`begin_boundary_drag` has been called).
    active: bool,
}

/// How far the strip must scroll to bring a cell wholly into view: negative to the left, positive to the right, zero
/// when it already shows. A cell wider than the strip is lined up on its left edge.
pub fn scroll_into_view_px(cell_left: f64, cell_right: f64, strip_left: f64, strip_right: f64) -> f64 {
    if cell_left < strip_left || cell_right - cell_left > strip_right - strip_left {
        cell_left - strip_left
    } else if cell_right > strip_right {
        cell_right - strip_right
    } else {
        0.0
    }
}

/// Measures the handle before zone `i` and the two cells it separates.
fn measure(w: SheetWatch, handle: &Element, i: usize) -> Option<PairSpan> {
    let doc = document();
    let rect = |index: usize| doc.get_element_by_id(&cell_id(index)).map(|e| e.get_bounding_client_rect());
    let (a, b) = (rect(i.checked_sub(1)?)?, rect(i)?);
    let v = w.view_now();
    Some(PairSpan {
        left_px: a.left(),
        cells_px: a.width() + b.width(),
        handle_px: handle.get_bounding_client_rect().width(),
        from_mm: v.zones.zones.get(i - 1)?.from_mm,
        to_mm: v.zones.zones.get(i)?.to_mm,
    })
}

/// The handle before zone `i` (none for the first) and the cell of zone `i`. Every attribute follows the view, so
/// the elements stay while the zones change.
fn zone_cell(
    w: SheetWatch,
    i: usize,
    on_down: impl Fn(leptos::ev::PointerEvent, usize) + Copy + 'static,
    on_move: impl Fn(leptos::ev::PointerEvent) + Copy + 'static,
    finish: impl Fn(bool) + Copy + 'static,
) -> impl IntoView {
    let name = move || {
        let (i18n, units) = (w.i18n(), w.units());
        w.view().zones.zones.get(i).map(|z| say(&i18n, units, &z.label)).unwrap_or_default()
    };
    let handle = (i > 0).then(|| {
        view! {
            <span
                class="zone-handle"
                aria-hidden="true"
                on:pointerdown=move |e| on_down(e, i)
                on:pointermove=on_move
                on:pointerup=move |_| finish(true)
                on:pointercancel=move |_| finish(false)
                on:lostpointercapture=move |_| finish(false)
            ></span>
        }
    });
    view! {
        {handle}
        <div
            class="zone-cell"
            class:own=move || w.view().zones.zones.get(i).is_some_and(|z| z.own)
            class:failing=move || w.view().zones.zones.get(i).is_some_and(|z| !z.ok)
            id=cell_id(i)
            role="option"
            aria-selected=move || (w.view().zones.selected == i).to_string()
            aria-disabled=move || w.view().zones.zones.get(i).is_some_and(|z| z.cut_off).then_some("true")
            aria-label=move || {
                let _ = w.i18n().locale();
                zone_text(&w.view(), i, &w.i18n(), w.units())
            }
            aria-description=move || {
                w.view().zones.zones.get(i).is_some_and(|z| z.cut_off).then(|| w.i18n().tr("zone-cut-off", &Args::new()))
            }
            title=name
            style=move || format!("flex-grow: {:.4}", cell_basis(&w.view().zones.zones).get(i).copied().unwrap_or(0.0))
            on:click=move |_| w.select_zone(i)
        >
            <span class="zone-name">{name}</span>
        </div>
    }
}

/// The zones of the street along a bar, under the drawing: one WAI-ARIA listbox and one tab
/// stop, the arrow keys choosing a zone and Alt with them moving its start. Each cell is at least 24 px wide
/// (WCAG 2.5.8); when they do not fit the strip scrolls inside itself, never the page. The handles between the
/// cells are for the pointer alone (`aria-hidden`); the keys do the same.
#[component]
pub fn ZoneStrip(vm: Rc<StreetVm>, ends: Option<Vec<EndView>>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let ends = StoredValue::new_local(ends);
    let from = move || {
        let i18n = w.i18n();
        let (first, name) = match w.view().zones.from_end {
            ZoneEnd::First => (true, 0),
            ZoneEnd::Last => (false, 1),
        };
        let end = ends.with_value(|e| e.as_ref().and_then(|e| e.get(name)).map(|e| say(&i18n, w.units(), &e.name)));
        let end = end.unwrap_or_else(|| i18n.tr(if first { "st-zones-end-first" } else { "st-zones-end-second" }, &Args::new()));
        i18n.tr("st-zones-from", &Args::new().str("end", end))
    };
    let heading = move || w.i18n().tr("st-zones-heading", &Args::new());
    let strip_label = move || w.i18n().tr("st-zones-strip-label", &Args::new());
    let active = move || cell_id(w.view().zones.selected);
    // A press on a handle selects the zone to its right; once it has moved a few pixels it drags that zone's start.
    // The pointer is captured by the handle, so the handles and cells are keyed by index and updated in place: a
    // view change during a drag must not replace the element that holds the capture.
    let press = StoredValue::new_local(None::<Press>);
    let finish = move |commit: bool| {
        if let Some(p) = press.get_value() {
            press.set_value(None);
            if p.active {
                w.end_boundary_drag(commit);
            }
        }
    };
    let on_down = move |ev: leptos::ev::PointerEvent, i: usize| {
        if ev.button() != 0 || !ev.is_primary() {
            return;
        }
        finish(false);
        w.select_zone(i);
        let handle = ev.current_target().and_then(|t| t.dyn_into::<Element>().ok());
        let Some(span) = handle.as_ref().and_then(|h| measure(w, h, i)) else { return };
        let handle_rect = handle.as_ref().map(|h| h.get_bounding_client_rect());
        let centre = handle_rect.map_or(0.0, |r| r.left() + r.width() / 2.0);
        let x = f64::from(ev.client_x());
        press.set_value(Some(Press { start_x: x, grab_px: x - centre, span, active: false }));
        if let Some(handle) = handle {
            let _ = handle.set_pointer_capture(ev.pointer_id());
        }
        ev.prevent_default();
        if let Some(strip) = document().get_element_by_id("zone-strip").and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
            let _ = strip.focus();
        }
    };
    let on_move = move |ev: leptos::ev::PointerEvent| {
        let Some(mut p) = press.get_value() else { return };
        let x = f64::from(ev.client_x());
        if !p.active {
            if !is_boundary_drag(x - p.start_x) {
                return;
            }
            w.begin_boundary_drag();
            p.active = true;
            press.set_value(Some(p));
        }
        w.drag_boundary_to(pair_boundary_mm(x - p.grab_px, &p.span));
    };
    // The chosen cell is brought into view inside the strip (its own scroll, never the page's).
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let id = active();
        let doc = document();
        if let (Some(cell), Some(strip)) = (doc.get_element_by_id(&id), doc.get_element_by_id("zone-strip")) {
            let (c, s) = (cell.get_bounding_client_rect(), strip.get_bounding_client_rect());
            let by = scroll_into_view_px(c.left(), c.right(), s.left(), s.right());
            if by != 0.0 {
                strip.set_scroll_left(strip.scroll_left() + by.round() as i32);
            }
        }
    });
    // Reads nothing itself (the list is read inside `each`), so it runs once and the cells are kept.
    let cells = move || {
        view! {
            <For each=move || 0..w.view().zones.zones.len() key=|i| *i let:i>
                {zone_cell(w, i, on_down, on_move, finish)}
            </For>
        }
    };
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
            ZoneAction::Select(d) => w.select_zone_relative(d),
            ZoneAction::SelectEdge { last } => w.select_zone_edge(last),
            ZoneAction::Nudge(mm) => {
                w.nudge_boundary(mm);
            }
        }
    };
    let notice = move || {
        let e = w.view().zones.elsewhere.as_ref().map(|e| (e.drawn_mm, e.loaded_mm))?;
        let (i18n, units) = (w.i18n(), w.units());
        let locale = i18n.locale();
        Some(i18n.tr("zones-elsewhere", &Args::new().str("drawn", units.length_in(e.0, locale)).str("loaded", units.length_in(e.1, locale))))
    };
    view! {
        <section class="zone-strip-section" aria-labelledby="h-zones">
            <h2 id="h-zones" class="sr-only">{heading}</h2>
            <p class="zone-caption" title=from>{from}</p>
            <div
                id="zone-strip"
                class="zone-strip"
                role="listbox"
                aria-orientation="horizontal"
                tabindex="0"
                aria-label=strip_label
                aria-activedescendant=active
                on:keydown=on_key
            >
                {cells}
            </div>
            <p class="zone-notice" role="status">{notice}</p>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::said::Said;

    fn zone(index: usize, from_mm: i32, to_mm: i32) -> ZoneView {
        ZoneView {
            index,
            from_mm,
            to_mm,
            label: Said::new("zone-unnamed"),
            label_mid: Said::new("zone-unnamed-mid"),
            named: false,
            own: false,
            cut_off: false,
            ok: true,
        }
    }

    #[test]
    fn a_cell_out_of_view_scrolls_the_strip_just_enough_to_show_it() {
        assert_eq!(scroll_into_view_px(10.0, 40.0, 0.0, 100.0), 0.0, "already in view");
        assert_eq!(scroll_into_view_px(-30.0, 0.0, 0.0, 100.0), -30.0, "off the left");
        assert_eq!(scroll_into_view_px(90.0, 130.0, 0.0, 100.0), 30.0, "off the right");
        assert_eq!(scroll_into_view_px(-10.0, 20.0, 0.0, 100.0), -10.0, "part off the left");
        assert_eq!(scroll_into_view_px(-10.0, 200.0, 0.0, 100.0), -10.0, "wider than the strip: its left edge");
    }

    fn shares(lengths: &[i32]) -> Vec<f64> {
        let mut at = 0;
        let zones: Vec<ZoneView> = lengths
            .iter()
            .enumerate()
            .map(|(i, l)| {
                at += l;
                zone(i, at - l, at)
            })
            .collect();
        let b = cell_basis(&zones);
        let sum: f64 = b.iter().sum();
        b.iter().map(|x| x / sum).collect()
    }

    #[test]
    fn a_short_zone_beside_a_long_one_keeps_at_least_the_floor_share() {
        let s = shares(&[5_000, 2_000_000]);
        assert!(s[0] >= MIN_SHARE - 1e-9, "{s:?}");
        assert!(s[1] > s[0]);
        let even = shares(&[30_000, 30_000]);
        assert!((even[0] - even[1]).abs() < 1e-12);
        assert!(cell_basis(&[]).is_empty());
    }

    #[test]
    fn ten_short_zones_and_a_long_one_each_keep_the_floor_share() {
        let mut l = vec![5_000; 10];
        l.push(2_000_000);
        let s = shares(&l);
        assert!(s.iter().all(|x| *x >= MIN_SHARE - 1e-9), "{s:?}");
        assert!((s.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(s[10] > s[0]);
    }

    #[test]
    fn zones_too_many_for_the_floor_get_equal_shares() {
        let s = shares(&[5_000; 40]);
        assert!(s.iter().all(|x| (x - 1.0 / 40.0).abs() < 1e-12), "{s:?}");
        let s = shares(&[5_000, 2_000_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000]);
        assert!(s.iter().all(|x| (x - 1.0 / 17.0).abs() < 1e-12), "{s:?}");
    }

    fn span() -> PairSpan {
        // two cells of 100 px each, a 24 px handle, over 40 m
        PairSpan { left_px: 10.0, cells_px: 200.0, handle_px: 24.0, from_mm: 20_000, to_mm: 60_000 }
    }

    #[test]
    fn the_handle_divides_the_pair_in_the_proportion_of_their_pixels_and_mm() {
        // the handle's left edge at 110 px: half of the pair, 40 m
        assert_eq!(pair_boundary_mm(110.0 + 12.0, &span()), 40_000);
        // a quarter of the way
        assert_eq!(pair_boundary_mm(60.0 + 12.0, &span()), 30_000);
    }

    #[test]
    fn the_boundary_keeps_both_zones_at_least_the_minimum_and_snaps() {
        assert_eq!(pair_boundary_mm(-500.0, &span()), 20_000 + MIN_ZONE_MM);
        assert_eq!(pair_boundary_mm(900.0, &span()), 60_000 - MIN_ZONE_MM);
        assert_eq!(pair_boundary_mm(61.2 + 12.0, &span()) % 100, 0);
        let tight = PairSpan { to_mm: 20_000 + 2 * MIN_ZONE_MM - 1, ..span() };
        assert_eq!(pair_boundary_mm(100.0, &tight), 20_000, "no room: the boundary stays");
        let none = PairSpan { cells_px: 0.0, ..span() };
        assert_eq!(pair_boundary_mm(100.0, &none), 20_000);
    }

    #[test]
    fn a_press_is_a_drag_only_after_a_few_pixels() {
        assert!(!is_boundary_drag(3.9) && !is_boundary_drag(-3.9));
        assert!(is_boundary_drag(4.0) && is_boundary_drag(-4.0) && is_boundary_drag(40.0));
    }
}
