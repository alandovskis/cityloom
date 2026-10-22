//! The zone strip: a bar for the street's length with one cell per zone, and the pure decisions it carries out.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{Element, HtmlElement};

use super::keys::{ZoneAction, on_strip};
use super::model::{ZoneEnd, ZoneView, snap};
use super::text::zone_text;
use super::vm::StreetVm;
use super::watch::SheetWatch;
use crate::city::model::EndView;
use crate::shared::i18n::Args;
use crate::shared::said::say;

/// A press moves a boundary only after it has moved this far.
const DRAG_START_PX: f64 = 4.0;
/// No zone's cell is narrower than this share of the strip, so a short zone of a long street is still a target.
/// A handle's width on the strip: the pointer target size (WCAG 2.5.8), as `.zone-handle` in `editor.css`.
const HANDLE_PX: f64 = 24.0;
const MIN_SHARE: f64 = 0.06;

/// A position along the street from a pointer position on the strip, snapped to `SNAP_MM` and kept within the street.
pub fn boundary_mm(x_px: f64, strip_px: f64, length_mm: i32) -> i32 {
    if strip_px <= 0.0 || length_mm <= 0 {
        return 0;
    }
    let mm = (x_px / strip_px * length_mm as f64).round().clamp(0.0, length_mm as f64) as i32;
    snap(mm).min(length_mm)
}

/// The flex-grow of each zone's cell, as shares that add up to 1: in proportion to its length, but never under
/// `MIN_SHARE` (the cells that would be are pinned at it and the rest share what is left by length). When the
/// zones are too many for the floor to hold, every cell gets an equal share. The target size in pixels is not
/// decided here: it is the view's CSS `min-width` on each cell (the strip scrolls when they do not fit).
pub fn cell_basis(zones: &[ZoneView]) -> Vec<f64> {
    let n = zones.len();
    if n == 0 {
        return Vec::new();
    }
    let equal = 1.0 / n as f64;
    if n as f64 * MIN_SHARE >= 1.0 {
        return vec![equal; n];
    }
    let len: Vec<f64> = zones.iter().map(|z| f64::from((z.to_mm - z.from_mm).max(0))).collect();
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
    /// The zone to the right of the handle.
    zone: usize,
    /// Moved far enough to be a drag (`begin_boundary_drag` has been called).
    active: bool,
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

/// The zones of the street along a bar, between the title card and the drawing: one WAI-ARIA listbox and one tab
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
    let boundary_at = move |x: f64, zone: usize| -> Option<i32> {
        let doc = document();
        let left = doc.get_element_by_id(&cell_id(0))?.get_bounding_client_rect().left();
        let last = doc.get_element_by_id(&cell_id(w.view_now().zones.zones.len().checked_sub(1)?))?;
        // The handles between the cells are not street: leave out the ones before the pointer (and, at its own, half).
        let handles = (w.view_now().zones.zones.len() - 1) as f64 * HANDLE_PX;
        let width = last.get_bounding_client_rect().right() - left - handles;
        let length = w.view_now().zones.zones.last()?.to_mm;
        Some(boundary_mm(x - left - (zone as f64 - 0.5) * HANDLE_PX, width, length))
    };
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
        press.set_value(Some(Press { start_x: f64::from(ev.client_x()), zone: i, active: false }));
        if let Some(handle) = ev.current_target().and_then(|t| t.dyn_into::<Element>().ok()) {
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
        if let Some(mm) = boundary_at(x, p.zone) {
            w.drag_boundary_to(mm);
        }
    };
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
            <p class="zone-caption">{from}</p>
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
            {move || notice().map(|n| view! { <p class="zone-notice" role="status">{n}</p> })}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::said::Said;

    fn zone(index: usize, from_mm: i32, to_mm: i32) -> ZoneView {
        ZoneView { index, from_mm, to_mm, label: Said::new("zone-unnamed"), own: false, cut_off: false, ok: true }
    }

    #[test]
    fn a_pointer_position_is_a_position_along_the_street_snapped_and_clamped() {
        assert_eq!(boundary_mm(0.0, 600.0, 60_000), 0);
        assert_eq!(boundary_mm(300.0, 600.0, 60_000), 30_000);
        assert_eq!(boundary_mm(640.0, 600.0, 60_000), 60_000);
        assert_eq!(boundary_mm(-20.0, 600.0, 60_000), 0);
        assert_eq!(boundary_mm(301.2, 600.0, 60_000), 30_100);
        assert_eq!(boundary_mm(10.0, 0.0, 60_000), 0, "a strip with no width");
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

    #[test]
    fn a_press_is_a_drag_only_after_a_few_pixels() {
        assert!(!is_boundary_drag(3.9) && !is_boundary_drag(-3.9));
        assert!(is_boundary_drag(4.0) && is_boundary_drag(-4.0) && is_boundary_drag(40.0));
    }
}
