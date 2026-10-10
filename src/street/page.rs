//! The rest of the street page around the section: its heading, the status
//! line, undo and redo, the menu to add a piece, the time of day, and the cue
//! that says how to begin.

use std::rc::Rc;

use leptos::prelude::*;

use crate::city::model::EndView;
use crate::shared::catalogue::{KINDS, group_key, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::provenance::Sources;
use crate::shared::said::{self, hhmm};
use crate::shared::symbols::icon;
use crate::shared::tick::Tick;
use crate::street::model::{SegView, View};
use crate::street::text::status_text;
use crate::street::vm::StreetVm;
use crate::street::watch::SheetWatch;

// ---- what the page is made of --------------------------------------------------------

/// The pieces by what they are for, so seventeen rows read as seven lists: each group by its id
/// (its name is the message `group-<id>`) and the kinds in it.
pub const ADD_GROUPS: [(&str, &[&str]); 7] = [
    ("walking", &["sidewalk"]),
    ("greenery", &["planting", "median"]),
    ("cycling", &["bike", "bikerack", "bikeshare"]),
    ("transit", &["bus", "busshelter", "busstation"]),
    ("roadway", &["travel", "parking", "loading", "shoulder"]),
    ("furniture", &["bench", "terrace"]),
    ("utilities", &["pole", "streetlamp"]),
];

/// The indices into the catalogue of the kinds in one group, those the catalogue has.
pub fn group_kinds(ids: &[&str]) -> Vec<usize> {
    ids.iter().filter_map(|id| KINDS.iter().position(|k| k.id == *id)).collect()
}

/// Where a new piece goes: after the selected one, or at the end.
pub fn insert_index(v: &View) -> usize {
    v.selected.and_then(|u| v.segments.iter().position(|s| s.uid == u)).map_or(v.segments.len(), |i| i + 1)
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

/// The days of the week an OSM opening-hours rule names (`Mo-Fr,Su`), in the words of the language.
fn weekdays(days: &str, i18n: &I18n) -> String {
    const IDS: [&str; 7] = ["mo", "tu", "we", "th", "fr", "sa", "su"];
    let mut out = String::new();
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        let id = run.to_ascii_lowercase();
        if IDS.contains(&id.as_str()) {
            out.push_str(&i18n.tr(&format!("day-{id}"), &Args::new()));
        } else {
            out.push_str(run);
        }
        run.clear();
    };
    for c in days.chars() {
        if c.is_ascii_alphabetic() {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// What the clock says about the selected piece: its usual type, and the others and when.
pub fn clock_note(s: &SegView, i18n: &I18n) -> String {
    if s.variants.is_empty() {
        return String::new();
    }
    let kind = |k: usize| i18n.tr(&kind_key(KINDS[k].id), &Args::new());
    let others: Vec<String> = s
        .variants
        .iter()
        .map(|v| {
            let args = Args::new().str("kind", kind(v.kind).to_lowercase()).str("from", hhmm(v.from_min)).str("to", hhmm(v.to_min));
            match &v.days {
                Some(d) => i18n.tr("clock-window-days", &args.str("days", weekdays(d, i18n))),
                None => i18n.tr("clock-window", &args),
            }
        })
        .collect();
    i18n.tr("clock-except", &Args::new().str("kind", kind(s.base_kind)).str("others", others.join(", ")))
}

/// The ends of a street named in a sentence: `A`, or `A and B`, or `A, B and C`.
fn ends_list(names: &[String], i18n: &I18n) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => i18n.tr("ends-and", &Args::new().str("a", rest.join(", ")).str("b", last.clone())),
    }
}

/// The title of the window: the street and where it runs between, or the page's own name when
/// it is not a street of a city.
pub fn page_title(i18n: &I18n, street: &str, ends: &[String]) -> String {
    if ends.is_empty() {
        return i18n.tr("title-street-editor", &Args::new());
    }
    i18n.tr("title-between", &Args::new().str("street", street).str("ends", ends_list(ends, i18n)))
}

// ---- the components ----------------------------------------------------------------------

fn back_link(w: SheetWatch) -> impl IntoView {
    view! {
        <a class="back" href="map.html">
            <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden="true" focusable="false"><path d="M12 7H2M6 3 2 7l4 4"/></svg>
            {say(w, "header-city-map")}
        </a>
    }
}

/// A word of the page, asked again when the language is switched.
fn say(w: SheetWatch, key: &'static str) -> impl Fn() -> String + 'static {
    move || w.i18n().tr(key, &Args::new())
}

/// What the street is called, in the language of the page.
fn street_title(w: SheetWatch) -> String {
    said::say(&w.i18n(), w.units(), &w.view().name)
}

/// One end of the street, named in the language of the page: a link where it is a junction.
fn end_link(w: SheetWatch, e: &EndView) -> impl IntoView {
    let name = e.name.clone();
    let words = move || said::say(&w.i18n(), w.units(), &name);
    if e.junction { view! { <a href=format!("intersection.html?junction={}", e.uid)>{words}</a> }.into_any() } else { words.into_any() }
}

/// The street's name and how wide it is; for a street of a city, the way back to
/// the map and the two places it runs between.
#[component]
pub fn StreetHeader(vm: Rc<StreetVm>, ends: Option<Vec<EndView>>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    // This page owns the window's title; it is written again when the street or the language changes.
    #[cfg(target_arch = "wasm32")]
    {
        let ends: Vec<said::Said> = ends.iter().flatten().map(|e| e.name.clone()).collect();
        Effect::new(move |_| {
            let names: Vec<String> = ends.iter().map(|e| said::say(&w.i18n(), w.units(), e)).collect();
            document().set_title(&page_title(&w.i18n(), &street_title(w), &names));
        });
    }
    let linked = ends.is_some();
    let between = ends.map(|ends| match ends.as_slice() {
        [a, b] => view! {
            " "
            <span aria-hidden="true">"\u{b7}"</span>
            " "
            <span>
                {move || format!("{} ", w.i18n().tr("header-between", &Args::new()))}
                {end_link(w, a)}
                {move || format!(" {} ", w.i18n().tr("header-and", &Args::new()))}
                {end_link(w, b)}
            </span>
        }
        .into_any(),
        _ => ().into_any(),
    });
    view! {
        <h1 id="street-name">{move || street_title(w)}</h1>
        <p class="street-sub" id="street-sub">
            {linked.then(|| view! { {back_link(w)} " " <span aria-hidden="true">"\u{b7}"</span> " " })}
            <span>{say(w, "header-section")}</span>
            " "
            <span aria-hidden="true">"\u{b7}"</span>
            " "
            <span><b id="row-dim" class="fig">{move || w.units().length_fine_in(w.view().row_mm, w.i18n().locale())}</b>{move || format!(" {}", w.i18n().tr("header-wide", &Args::new()))}</span>
            {between}
        </p>
    }
}

#[component]
pub fn TitleBlock(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    view! {
        <div class="tb-cell tb-wide"><span>{say(w, "block-street")}</span><b id="tb-street">{move || street_title(w)}</b></div>
        <div class="tb-cell"><span>{say(w, "block-width")}</span><b id="tb-row" class="fig">{move || w.units().length_fine_in(w.view().row_mm, w.i18n().locale())}</b></div>
        <div class="tb-cell"><span>{say(w, "block-changes")}</span><b id="tb-changes" class="fig">{move || w.view().revisions.len().to_string()}</b></div>
        {move || {
            let source = w.view().source.clone();
            (!source.is_empty())
                .then(|| {
                    view! {
                        <div class="tb-cell tb-full"><span>{say(w, "block-osm")}</span><b class="tb-src"><Sources kind="way" refs=source/></b></div>
                        <div class="tb-cell tb-full">
                            <span>{say(w, "block-data")}</span>
                            <b id="tb-state">{move || w.i18n().tr(if w.view().changed { "block-edited" } else { "block-imported" }, &Args::new())}</b>
                        </div>
                    }
                })
        }}
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
                let i18n = w.i18n();
                // The sentence is built from the locale as it is now; reading it here redraws the line on a switch.
                i18n.locale();
                status_text(&v, &i18n, units)
            }}
        </p>
    }
}

#[component]
pub fn History(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    view! {
        <div class="btns" role="toolbar" aria-label=say(w, "history-tools")>
            <button type="button" id="undo" class="btn" disabled=move || !w.view().can_undo on:click=move |_| { w.undo(); }>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M5.5 3 2.5 6l3 3M2.5 6H10a3.5 3.5 0 0 1 0 7H6"/></svg>
                <span class="lbl">{say(w, "history-undo")}</span>
            </button>
            <button type="button" id="redo" class="btn" disabled=move || !w.view().can_redo on:click=move |_| { w.redo(); }>
                <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M10.5 3l3 3-3 3M13.5 6H6a3.5 3.5 0 0 0 0 7h4"/></svg>
                <span class="lbl">{say(w, "history-redo")}</span>
            </button>
        </div>
        <button type="button" id="reset" class="btn" title=say(w, "history-reset-title") disabled=move || !w.view().changed on:click=move |_| { w.reset(); }>
            <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d="M2.5 8a5.5 5.5 0 1 0 1.8-4.1M2.5 2.5v3h3"/></svg>
            <span class="lbl">{say(w, "history-reset")}</span>
        </button>
    }
}

/// Where the list of pieces is drawn, in window pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuBox {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub max_height: f64,
}

/// The list opens under its button, as wide as it likes up to 720 px, and is kept inside the
/// window: moved left if the button is near the right edge, and given the height that is left
/// below it (at least 160 px, moving up for that), so what does not fit scrolls.
pub fn menu_box(button_left: f64, button_bottom: f64, window_w: f64, window_h: f64) -> MenuBox {
    const MARGIN: f64 = 8.0;
    const MIN_HEIGHT: f64 = 160.0;
    let width = (window_w - 2.0 * MARGIN).min(720.0);
    let left = button_left.clamp(MARGIN, (window_w - width - MARGIN).max(MARGIN));
    let top = (button_bottom + MARGIN).min(window_h - MIN_HEIGHT - MARGIN).max(MARGIN);
    MenuBox { left, top, width, max_height: (window_h - top - MARGIN).max(MIN_HEIGHT) }
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
        // The list is fixed to the window, so the stage cannot clip it: put it under its button.
        let place = move || {
            let (Some(b), Some(m), Some(win)) = (button.get_untracked(), menu.get_untracked(), leptos::web_sys::window()) else { return };
            let size = |v: Result<leptos::wasm_bindgen::JsValue, _>| v.ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let r = b.get_bounding_client_rect();
            let at = menu_box(r.left(), r.bottom(), size(win.inner_width()), size(win.inner_height()));
            let _ = m.set_attribute("style", &format!("left:{}px;top:{}px;width:{}px;max-height:{}px", at.left, at.top, at.width, at.max_height));
        };
        window_event_listener(leptos::ev::resize, move |_| {
            if open.get_untracked() {
                place();
            }
        });
        Effect::new(move |_| {
            if open.get() {
                place();
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
        .map(|(g, (group, ids))| {
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
                                <span class="add-icon" inner_html=icon(kind.id)></span>
                                <b>{move || w.i18n().tr(&kind_key(kind.id), &Args::new())}</b>
                                <span class="dw">{move || w.units().length_fine_in(kind.default_mm, w.i18n().locale())}</span>
                            </button>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <li role="presentation" class="add-group">
                    <p class="add-group-h" id=format!("add-g-{g}")>{move || w.i18n().tr(&group_key(group), &Args::new())}</p>
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
            {say(w, "add-button")}
        </button>
        <div id="add-menu" class="menu add-menu" hidden=move || !open.get() node_ref=menu on:keydown=menu_keys on:focusout=focus_out>
            <p class="menu-h">{say(w, "add-button")}</p>
            <p class="hint">{say(w, "add-hint")}</p>
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
        v.selected.and_then(|u| v.segments.iter().find(|s| s.uid == u).map(|s| (clock_bands(s), clock_note(s, &w.i18n()))))
    };
    view! {
        <div class="clock" id="clock" hidden=move || !timed()>
            <label for="time" class="clock-l">{say(w, "clock-time-of-day")}</label>
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
        v.segments.iter().any(|s| !s.variants.is_empty()).then(|| w.i18n().tr("clock-numbers-for", &Args::new().str("time", hhmm(v.time_min))))
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
            <p>{say(w, "welcome-text")}</p>
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
                {say(w, "welcome-dismiss")}
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::Editor;

    #[test]
    fn the_menu_opens_under_its_button_and_never_leaves_the_window() {
        // Room: under the button, as wide as it likes, to the bottom of the window.
        let m = menu_box(340.0, 190.0, 1440.0, 900.0);
        assert_eq!((m.left, m.top, m.width, m.max_height), (340.0, 198.0, 720.0, 694.0));
        // A button near the right edge: the menu is drawn leftwards to fit.
        let m = menu_box(792.0, 150.0, 1280.0, 720.0);
        assert_eq!((m.left, m.width), (552.0, 720.0));
        // A phone: the menu is the width of the window less its margins.
        let m = menu_box(14.0, 270.0, 390.0, 844.0);
        assert_eq!((m.left, m.width), (8.0, 374.0));
        // A short window: the menu keeps some height and moves up to have it.
        let m = menu_box(10.0, 250.0, 1000.0, 300.0);
        assert_eq!((m.top, m.max_height), (132.0, 160.0));
        for (l, b, w, h) in [(340.0, 190.0, 1024.0, 768.0), (792.0, 150.0, 1280.0, 720.0), (14.0, 270.0, 390.0, 844.0), (10.0, 250.0, 1000.0, 300.0)] {
            let m = menu_box(l, b, w, h);
            assert!(m.left >= 0.0 && m.left + m.width <= w && m.top >= 0.0 && m.top + m.max_height <= h, "{w}x{h}");
        }
    }

    #[test]
    fn the_kinds_to_add_come_in_four_lists_that_cover_every_kind_but_the_one_with_no_place() {
        let all: Vec<usize> = ADD_GROUPS.iter().flat_map(|(_, ids)| group_kinds(ids)).collect();
        assert_eq!(all.len(), 17);
        let mut seen = all.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 17, "each kind is in one list");
        assert_eq!(KINDS.len(), 17);
        let ids = |group: &str| -> Vec<&str> {
            let (_, ids) = ADD_GROUPS.iter().find(|(name, _)| *name == group).unwrap();
            group_kinds(ids).into_iter().map(|k| KINDS[k].id).collect()
        };
        assert_eq!(ids("walking"), ["sidewalk"]);
        assert_eq!(ids("greenery"), ["planting", "median"]);
        assert_eq!(ids("transit"), ["bus", "busshelter", "busstation"]);
        assert_eq!(ids("furniture"), ["bench", "terrace"]);
        assert_eq!(ids("utilities"), ["pole", "streetlamp"]);
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
        let i18n = crate::i18n_for(crate::shared::i18n::Locale::En);
        let note = clock_note(&s, &i18n);
        let base = i18n.tr_now(&crate::shared::catalogue::kind_key(KINDS[s.base_kind].id), &crate::shared::i18n::Args::new());
        assert!(note.starts_with(&base) && note.contains(" except "), "{note}");
        assert!(note.contains("07:00\u{2013}10:00"), "{note}");
        s.variants[0].days = Some("Mo-Fr".into());
        assert!(clock_note(&s, &i18n).contains("Mo-Fr 07:00\u{2013}10:00"), "{}", clock_note(&s, &i18n));
        s.variants.clear();
        assert_eq!(clock_note(&s, &i18n), "");
    }

    #[test]
    fn the_ends_of_a_city_street_are_read_from_the_city() {
        let city = crate::city::model::City::new();
        let ends = city.street_ends(1);
        let i18n = crate::i18n_for(crate::shared::i18n::Locale::En);
        let read = |e: &EndView| (said::say_now(&i18n, crate::shared::units::Units::Metres, &e.name), e.junction, e.uid);
        assert_eq!(ends.iter().map(read).collect::<Vec<_>>(), [("the edge of the map".to_string(), false, 1), ("Junction 4".to_string(), true, 2)]);
        assert!(city.street_ends(99).is_empty());
    }
}
