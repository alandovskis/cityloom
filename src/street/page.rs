//! The rest of the street page around the section: its heading, the status
//! line, undo and redo, the menu to add a piece, the time of day, and the cue
//! that says how to begin.

use std::rc::Rc;

use leptos::prelude::*;
use serde::Deserialize;

use crate::shared::catalogue::KINDS;
use crate::shared::tick::Tick;
use crate::street::model::{SegView, View};
use crate::street::text::status_text;
use crate::street::vm::StreetVm;
use crate::street::watch::SheetWatch;

// ---- what the page is made of --------------------------------------------------------

/// The pieces by what they are for, so seventeen rows read as six lists.
pub const ADD_GROUPS: [(&str, &[&str]); 6] = [
    ("Walk and plant", &["sidewalk", "planting", "median"]),
    ("Cycling", &["bike", "bikerack", "bikeshare"]),
    ("Transit", &["bus", "busshelter", "busstation"]),
    ("Roadway", &["travel", "parking", "loading", "shoulder"]),
    ("Furniture", &["bench", "terrace"]),
    ("Utilities", &["pole", "streetlamp"]),
];

/// The indices into the catalogue of the kinds in one group, those the catalogue has.
pub fn group_kinds(ids: &[&str]) -> Vec<usize> {
    ids.iter().filter_map(|id| KINDS.iter().position(|k| k.id == *id)).collect()
}

/// Where a new piece goes: after the selected one, or at the end.
pub fn insert_index(v: &View) -> usize {
    v.selected.and_then(|u| v.segments.iter().position(|s| s.uid == u)).map_or(v.segments.len(), |i| i + 1)
}

/// A time of day, `06:30`; the hour wraps at 24.
pub fn hhmm(min: i32) -> String {
    format!("{:02}:{:02}", (min / 60) % 24, min % 60)
}

/// Minutes after midnight from `hh:mm`, or None when it is not a time.
pub fn to_min(text: &str) -> Option<i32> {
    let (h, m) = text.split_once(':')?;
    Some(h.trim().parse::<i32>().ok()? * 60 + m.trim().parse::<i32>().ok()?)
}

/// Where a piece is other than its usual type through the day, as stretches of
/// a bar from 0 to 100, with the kind they are: a window past midnight is two.
pub fn clock_bands(s: &SegView) -> Vec<(f64, f64, &'static str)> {
    let at = |m: i32| m as f64 / 1440.0 * 100.0;
    let mut bands = Vec::new();
    for v in &s.variants {
        let id = KINDS[v.kind].id;
        if v.from_min < v.to_min {
            bands.push((at(v.from_min), at(v.to_min), id));
        } else {
            bands.push((at(v.from_min), 100.0, id));
            bands.push((0.0, at(v.to_min), id));
        }
    }
    bands
}

/// What the clock says about the selected piece: its usual type, and the others and when.
pub fn clock_note(s: &SegView) -> String {
    if s.variants.is_empty() {
        return String::new();
    }
    let others: Vec<String> =
        s.variants.iter().map(|v| format!("{} {}\u{2013}{}", KINDS[v.kind].name.to_lowercase(), hhmm(v.from_min), hhmm(v.to_min))).collect();
    format!("{} except {}", KINDS[s.base_kind].name, others.join(", "))
}

/// The ends of a street that belongs to a city: a junction, or where it leaves the map.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct StreetEnd {
    pub name: String,
    pub junction: bool,
    pub uid: u32,
}

// ---- the components ----------------------------------------------------------------------

fn back_link() -> impl IntoView {
    view! {
        <a class="back" href="map.html">
            <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden="true" focusable="false"><path d="M12 7H2M6 3 2 7l4 4"/></svg>
            "City map"
        </a>
    }
}

fn end_link(e: &StreetEnd) -> impl IntoView {
    let name = e.name.clone();
    e.junction
        .then(|| format!("intersection.html?junction={}", e.uid))
        .map_or_else(|| name.clone().into_any(), |href| view! { <a href=href>{name.clone()}</a> }.into_any())
}

/// The street's name and how wide it is; for a street of a city, the way back to
/// the map and the two places it runs between.
#[component]
pub fn StreetHeader(vm: Rc<StreetVm>, ends: Option<Vec<StreetEnd>>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    #[cfg(target_arch = "wasm32")]
    if let Some(ends) = &ends {
        let names: Vec<String> = ends.iter().map(|e| e.name.clone()).collect();
        Effect::new(move |_| {
            document().set_title(&format!("{} between {} \u{b7} CityLoom", w.view().name, names.join(" and ")));
        });
    }
    let linked = ends.is_some();
    let between = ends.map(|ends| match ends.as_slice() {
        [a, b] => view! { " " <span aria-hidden="true">"\u{b7}"</span> " " <span>"between " {end_link(a)} " and " {end_link(b)}</span> }.into_any(),
        _ => ().into_any(),
    });
    view! {
        <h1 id="street-name">{move || w.view().name.clone()}</h1>
        <p class="street-sub" id="street-sub">
            {linked.then(|| view! { {back_link()} " " <span aria-hidden="true">"\u{b7}"</span> " " })}
            <span>"Street cross-section"</span>
            " "
            <span aria-hidden="true">"\u{b7}"</span>
            " "
            <span><b id="row-dim" class="fig">{move || w.units().length_fine(w.view().row_mm)}</b>" wide"</span>
            {between}
        </p>
    }
}

#[component]
pub fn TitleBlock(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    view! {
        <div class="tb-cell tb-wide"><span>"Street"</span><b id="tb-street">{move || w.view().name.clone()}</b></div>
        <div class="tb-cell"><span>"Width"</span><b id="tb-row" class="fig">{move || w.units().length_fine(w.view().row_mm)}</b></div>
        <div class="tb-cell"><span>"Changes made"</span><b id="tb-changes" class="fig">{move || w.view().revisions.len().to_string()}</b></div>
    }
}

/// The status line: whether the pieces fit the street.
#[component]
pub fn Fit(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let too_wide = move || w.view().delta_mm > 0;
    // The width is all used: neither over nor under.
    let fits = Signal::derive(move || w.view().delta_mm == 0);
    view! {
        <p id="fit" class=move || if too_wide() { "fit bad" } else { "fit" } role="status">
            <Tick works=fits/>
            {move || {
                let (v, units) = w.now();
                status_text(&v, units)
            }}
        </p>
    }
}

#[component]
pub fn History(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    view! {
        <div class="btns" role="toolbar" aria-label="Sheet tools">
            <button type="button" id="undo" class="btn" disabled=move || !w.view().can_undo on:click=move |_| { w.undo(); }>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M5.5 3 2.5 6l3 3M2.5 6H10a3.5 3.5 0 0 1 0 7H6"/></svg>
                <span class="lbl">"Undo"</span>
            </button>
            <button type="button" id="redo" class="btn" disabled=move || !w.view().can_redo on:click=move |_| { w.redo(); }>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M10.5 3l3 3-3 3M13.5 6H6a3.5 3.5 0 0 0 0 7h4"/></svg>
                <span class="lbl">"Redo"</span>
            </button>
        </div>
        <button type="button" id="reset" class="btn" title="Back to the street as it is today. Undo brings your changes back." disabled=move || !w.view().changed on:click=move |_| { w.reset(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M2.5 8a5.5 5.5 0 1 0 1.8-4.1M2.5 2.5v3h3"/></svg>
            <span class="lbl">"Start over"</span>
        </button>
    }
}

/// A swatch of a kind of piece: its tint and hatch, with the parking mark.
fn kind_swatch(id: &'static str) -> impl IntoView {
    view! {
        <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">
            <rect class=format!("k-{id}") width="44" height="22" stroke="none"/>
            <rect width="44" height="22" fill=format!("url(#h-{id})") stroke="none"/>
            {(id == "parking").then(|| view! { <text class="slab-p" x="22" y="16" text-anchor="middle" font-size="14">"P"</text> })}
        </svg>
    }
}

/// The button that opens the list of pieces, and the list: arrows move through
/// it, Escape closes it, and choosing a piece adds it after the selected one.
#[component]
pub fn AddMenu(vm: Rc<StreetVm>) -> impl IntoView {
    use leptos::wasm_bindgen::JsCast;
    use leptos::web_sys::{Element, FocusEvent, HtmlElement, KeyboardEvent};

    let w = SheetWatch::new(vm);
    let open = RwSignal::new(false);
    let button = NodeRef::<leptos::html::Button>::new();
    let menu = NodeRef::<leptos::html::Div>::new();
    let items = move || -> Vec<HtmlElement> {
        let Some(menu) = menu.get_untracked() else { return Vec::new() };
        let list = menu.query_selector_all(".add-item").ok();
        (0..list.as_ref().map_or(0, |l| l.length())).filter_map(|i| list.as_ref()?.item(i)?.dyn_into::<HtmlElement>().ok()).collect()
    };
    let close = move |refocus: bool| {
        open.set(false);
        if refocus {
            if let Some(b) = button.get_untracked() {
                let _ = HtmlElement::from(b).focus();
            }
        }
    };
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if open.get() {
                // Once the list is shown, so that it can take the focus.
                leptos::task::spawn_local(async move {
                    leptos::task::tick().await;
                    if let Some(first) = items().first() {
                        let _ = first.focus();
                    }
                });
            }
        });
        window_event_listener(leptos::ev::pointerdown, move |e: leptos::web_sys::PointerEvent| {
            let inside = |n: Option<HtmlElement>| n.zip(e.target().and_then(|t| t.dyn_into::<Element>().ok())).is_some_and(|(n, t)| n.contains(Some(&t)));
            if open.get_untracked() && !inside(menu.get_untracked().map(HtmlElement::from)) && !inside(button.get_untracked().map(HtmlElement::from)) {
                open.set(false);
            }
        });
        window_event_listener(leptos::ev::keydown, move |e: KeyboardEvent| {
            if e.key() == "Escape" && open.get_untracked() {
                close(true);
            }
        });
    }
    let menu_keys = move |e: KeyboardEvent| {
        let list = items();
        let active = document().active_element();
        let at = list.iter().position(|i| active.as_ref() == Some(i.unchecked_ref::<Element>())).map_or(-1, |p| p as i32);
        let to = match e.key().as_str() {
            "ArrowDown" => at + 1,
            "ArrowUp" => at - 1,
            "Home" => 0,
            "End" => list.len() as i32 - 1,
            _ => return,
        };
        e.prevent_default();
        if !list.is_empty() {
            let _ = list[to.rem_euclid(list.len() as i32) as usize].focus();
        }
    };
    let focus_out = move |e: FocusEvent| {
        let outside = e.related_target().and_then(|t| t.dyn_into::<Element>().ok()).is_some_and(|t| {
            let in_menu = menu.get_untracked().is_some_and(|m| m.contains(Some(&t)));
            let is_button = button.get_untracked().is_some_and(|b| b.unchecked_ref::<Element>() == &t);
            !in_menu && !is_button
        });
        if outside {
            open.set(false);
        }
    };
    let groups = ADD_GROUPS
        .iter()
        .enumerate()
        .map(|(g, (name, ids))| {
            let rows = group_kinds(ids)
                .into_iter()
                .map(|k| {
                    let kind = &KINDS[k];
                    view! {
                        <li>
                            <button
                                type="button"
                                class="add-item"
                                on:click=move |_| {
                                    close(true);
                                    let at = insert_index(&w.view_now());
                                    w.edit(|e| e.add(k, at) != 0);
                                }
                            >
                                {kind_swatch(kind.id)}
                                <b>{kind.name}</b>
                                <span class="dw">{move || w.units().length_fine(kind.default_mm)}</span>
                            </button>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <li role="presentation" class="add-group">
                    <p class="add-group-h" id=format!("add-g-{g}")>{*name}</p>
                    <ul class="add-sub" role="group" aria-labelledby=format!("add-g-{g}")>{rows}</ul>
                </li>
            }
        })
        .collect_view();
    view! {
        <button
            type="button"
            id="add-btn"
            class="btn"
            aria-haspopup="true"
            aria-expanded=move || open.get().to_string()
            aria-controls="add-menu"
            node_ref=button
            on:click=move |_| open.update(|o| *o = !*o)
            on:keydown=move |e: KeyboardEvent| {
                if e.key() == "ArrowDown" && !open.get_untracked() {
                    e.prevent_default();
                    open.set(true);
                }
            }
        >
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M3 8h10M8 3v10"/></svg>
            "Add a piece"
        </button>
        <div id="add-menu" class="menu add-menu" hidden=move || !open.get() node_ref=menu on:keydown=menu_keys on:focusout=focus_out>
            <p class="menu-h">"Add a piece"</p>
            <p class="hint">"Goes after the selected piece."</p>
            <ul id="palette" class="add-list">{groups}</ul>
        </div>
    }
}

/// The slider that sets the time the street is shown at, and under it a bar that
/// marks when the selected piece is something other than its usual type.
#[component]
pub fn Clock(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    // Time only matters once some piece changes type through the day.
    let timed = move || w.view().segments.iter().any(|s| !s.variants.is_empty());
    let selected = move || {
        let v = w.view();
        v.selected.and_then(|u| v.segments.iter().find(|s| s.uid == u).map(|s| (clock_bands(s), clock_note(s))))
    };
    view! {
        <div class="clock" id="clock" hidden=move || !timed()>
            <label for="time" class="clock-l">"Time of day"</label>
            <input
                type="range"
                id="time"
                min="0"
                max="95"
                step="1"
                value=move || (w.view().time_min / 15).to_string()
                prop:value=move || (w.view().time_min / 15).to_string()
                aria-valuetext=move || hhmm(w.view().time_min)
                on:input=move |e| {
                    if let Ok(v) = event_target_value(&e).parse::<i32>() {
                        w.quiet(|ed| ed.set_time(v * 15));
                    }
                }
            />
            <output id="time-out" for="time" class="fig">{move || hhmm(w.view().time_min)}</output>
            <div class="clock-scale" aria-hidden="true"><span>"00"</span><span>"06"</span><span>"12"</span><span>"18"</span><span>"24"</span></div>
            <div class=move || if selected().is_some_and(|(b, _)| !b.is_empty()) { "clock-bar on" } else { "clock-bar" } id="clock-bar" aria-hidden="true">
                {move || {
                    selected()
                        .map(|(bands, _)| {
                            bands
                                .into_iter()
                                .map(|(a, b, id)| view! { <i style=format!("left:{a}%;width:{}%;background:var(--k-{id})", b - a)></i> })
                                .collect_view()
                        })
                }}
            </div>
            <p class="clock-note" id="clock-note">{move || selected().map(|(_, note)| note)}</p>
        </div>
    }
}

/// Says which time the numbers are for, once there is a time to speak of.
#[component]
pub fn TimeNote(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    move || {
        let v = w.view();
        v.segments.iter().any(|s| !s.variants.is_empty()).then(|| format!("Numbers are for {}.", hhmm(v.time_min)))
    }
}

/// The cue that says how to begin. It goes when it is dismissed or the street
/// has been changed, and is remembered so it does not return; "How this works"
/// in the settings menu brings it back and keeps it until it is dismissed.
#[component]
pub fn Welcome(vm: Rc<StreetVm>) -> impl IntoView {
    use crate::shared::store::remember;

    // Only the browser watches the street, for the first change.
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let w = SheetWatch::new(vm);
    let pinned = StoredValue::new_local(false);
    let root = || document().document_element();
    let dismiss = move || {
        pinned.set_value(false);
        if let Some(r) = root() {
            let _ = r.set_attribute("data-welcome", "seen");
        }
        remember("cityloom-welcome", "seen");
    };
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::wasm_bindgen::JsCast;
        Effect::new(move |_| {
            if !w.view().revisions.is_empty() && !pinned.get_value() {
                dismiss();
            }
        });
        window_event_listener(leptos::ev::click, move |e: leptos::web_sys::MouseEvent| {
            let hit = e.target().and_then(|t| t.dyn_into::<leptos::web_sys::Element>().ok()).is_some_and(|t| t.closest("#how-btn").ok().flatten().is_some());
            if !hit {
                return;
            }
            pinned.set_value(true);
            if let Some(r) = root() {
                let _ = r.remove_attribute("data-welcome");
            }
            remember("cityloom-welcome", "show");
            let doc = document();
            if let Some(el) = doc.get_element_by_id("welcome") {
                let o = leptos::web_sys::ScrollIntoViewOptions::new();
                o.set_block(leptos::web_sys::ScrollLogicalPosition::Nearest);
                el.scroll_into_view_with_scroll_into_view_options(&o);
            }
            if let Some(b) = doc.get_element_by_id("welcome-dismiss") {
                let _ = b.unchecked_into::<leptos::web_sys::HtmlElement>().focus();
            }
        });
    }
    view! {
        <div class="welcome" id="welcome">
            <p>"Add a piece, then drag to arrange. The width is fixed; a piece that does not fit turns orange."</p>
            <button
                type="button"
                id="welcome-dismiss"
                class="btn"
                on:click=move |_| {
                    dismiss();
                    if let Some(wrap) = document().get_element_by_id("wrap") {
                        let _ = leptos::wasm_bindgen::JsCast::unchecked_into::<leptos::web_sys::HtmlElement>(wrap).focus();
                    }
                }
            >
                "Got it"
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::Editor;

    #[test]
    fn the_kinds_to_add_come_in_four_lists_that_cover_every_kind_but_the_one_with_no_place() {
        let all: Vec<usize> = ADD_GROUPS.iter().flat_map(|(_, ids)| group_kinds(ids)).collect();
        assert_eq!(all.len(), 17);
        let mut seen = all.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 17, "each kind is in one list");
        assert_eq!(KINDS.len(), 17);
        let furniture: Vec<&str> = group_kinds(ADD_GROUPS[4].1).into_iter().map(|k| KINDS[k].id).collect();
        assert_eq!(furniture, ["bench", "terrace"]);
        let utilities: Vec<&str> = group_kinds(ADD_GROUPS[5].1).into_iter().map(|k| KINDS[k].id).collect();
        assert_eq!(utilities, ["pole", "streetlamp"]);
        let transit: Vec<&str> = group_kinds(ADD_GROUPS[2].1).into_iter().map(|k| KINDS[k].id).collect();
        assert_eq!(transit, ["bus", "busshelter", "busstation"]);
        assert_eq!(group_kinds(&["travel", "nonsense"]).len(), 1);
    }

    #[test]
    fn a_new_piece_goes_after_the_selected_one_or_at_the_end() {
        let mut e = Editor::new(0);
        assert_eq!(insert_index(&e.view()), 6);
        let u = e.view().segments[2].uid;
        e.select(Some(u));
        assert_eq!(insert_index(&e.view()), 3);
    }

    #[test]
    fn times_are_written_and_read_as_hours_and_minutes() {
        assert_eq!(hhmm(0), "00:00");
        assert_eq!(hhmm(390), "06:30");
        assert_eq!(hhmm(1_439), "23:59");
        assert_eq!(hhmm(1_440), "00:00");
        assert_eq!(to_min("06:30"), Some(390));
        assert_eq!(to_min("00:00"), Some(0));
        assert_eq!(to_min(""), None);
        assert_eq!(to_min("6"), None);
        assert_eq!(to_min("aa:bb"), None);
    }

    fn timed() -> SegView {
        let mut e = Editor::new(1);
        assert!(e.apply_measure("B3") || e.apply_measure("E2"));
        let v = e.view();
        v.segments.into_iter().find(|s| !s.variants.is_empty()).expect("the arrangement has a piece that changes through the day")
    }

    #[test]
    fn a_piece_that_changes_through_the_day_has_a_band_for_each_stretch() {
        let s = timed();
        let bands = clock_bands(&s);
        assert!(!bands.is_empty());
        for (a, b, id) in &bands {
            assert!(a < b && *a >= 0.0 && *b <= 100.0, "{a} {b}");
            assert!(KINDS.iter().any(|k| k.id == *id));
        }
        // A window that crosses midnight is two stretches.
        let mut s = s;
        s.variants[0].from_min = 22 * 60;
        s.variants[0].to_min = 2 * 60;
        let b = clock_bands(&s);
        assert_eq!(b[0].1, 100.0);
        assert_eq!(b[1].0, 0.0);
        assert!((b[0].0 - 22.0 / 24.0 * 100.0).abs() < 1e-9 && (b[1].1 - 2.0 / 24.0 * 100.0).abs() < 1e-9);
    }

    #[test]
    fn the_clock_says_what_a_piece_usually_is_and_when_it_is_something_else() {
        let mut s = timed();
        s.variants[0].from_min = 420;
        s.variants[0].to_min = 600;
        let note = clock_note(&s);
        assert!(note.starts_with(KINDS[s.base_kind].name) && note.contains(" except "), "{note}");
        assert!(note.contains("07:00\u{2013}10:00"), "{note}");
        s.variants.clear();
        assert_eq!(clock_note(&s), "");
    }

    #[test]
    fn the_ends_of_a_city_street_are_read_from_the_json_the_city_gives() {
        let ends: Vec<StreetEnd> =
            serde_json::from_str(r#"[{"name":"Junction 1","junction":true,"uid":3},{"name":"Edge of the map","junction":false,"uid":0}]"#).unwrap();
        assert_eq!(ends[0], StreetEnd { name: "Junction 1".into(), junction: true, uid: 3 });
        assert!(!ends[1].junction);
    }
}
