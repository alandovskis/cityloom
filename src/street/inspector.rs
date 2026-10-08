//! The panel beside the street's section: the selected piece, and the controls
//! to change it. A control that stays on the panel keeps its element, and so
//! its focus, across edits.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{Element, HtmlInputElement, HtmlSelectElement, KeyboardEvent};

use crate::shared::catalogue::{CURBS, DIRECTIONS, DirectionRule, KINDS, MATERIALS};
use crate::shared::units::Units;
use crate::street::model::{SegView, View};
use crate::street::page::{hhmm, to_min};
use crate::street::vm::StreetVm;
use crate::street::watch::SheetWatch;

/// The kinds a piece may take at other times, with the one it is: the choices
/// in a list, in catalogue order.
pub fn variant_kinds(alt_kinds: &[usize], current: usize) -> Vec<usize> {
    let mut kinds: Vec<usize> = alt_kinds.iter().copied().chain(std::iter::once(current)).collect();
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

/// What a piece's width is allowed to be, as the field holds it.
pub fn width_range(s: &SegView, units: Units) -> String {
    format!("Allowed {} to {} {}", units.fixed(s.min_mm, 2), units.fixed(s.max_mm, 2), units.word())
}

/// Where the piece is, and how wide, under its name.
pub fn piece_sub(v: &View, s: &SegView, units: Units) -> String {
    let i = v.segments.iter().position(|x| x.uid == s.uid).unwrap_or(0);
    let time = if s.variants.is_empty() { String::new() } else { format!(" \u{b7} {}", hhmm(v.time_min)) };
    format!("{} wide \u{b7} {} of {}{time}", units.length_fine(s.width_mm), i + 1, v.segments.len())
}

/// What the width steppers say they do.
pub fn step_label(more: bool, units: Units) -> String {
    format!("{} by {} {}", if more { "Wider" } else { "Narrower" }, units.fine(units.step_mm()), units.word())
}

/// How a direction is picked in the field's `<select>`: the index into the
/// directions, or None for two-way.
pub fn direction_choice(value: &str) -> Option<usize> {
    DIRECTIONS.iter().position(|d| d.id == value)
}

// ---- small pieces -------------------------------------------------------------------------

fn tick() -> impl IntoView {
    view! { <svg class="tick" viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 8.5 6.5 12.5 13.5 3.5"/></svg> }
}

fn icon(path: &'static str) -> impl IntoView {
    view! { <svg viewBox="0 0 14 14" aria-hidden="true"><path d=path/></svg> }
}

fn swatch(id: &'static str) -> impl IntoView {
    view! {
        <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">
            <rect class=format!("k-{id}") width="44" height="22" stroke="none"/>
            <rect width="44" height="22" fill=format!("url(#h-{id})") stroke="none"/>
            {(id == "parking").then(|| view! { <text class="slab-p" x="22" y="16" text-anchor="middle" font-size="14">"P"</text> })}
        </svg>
    }
}

fn surface_swatch(id: &'static str) -> AnyView {
    view! {
        <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">
            <rect width="44" height="22" fill="var(--sheet)" stroke="none"/>
            <rect width="44" height="22" fill=format!("url(#m-{id})") stroke="none"/>
        </svg>
    }
    .into_any()
}

fn curb_swatch(id: Option<&'static str>) -> AnyView {
    match id {
        None => {
            view! { <svg class="swatch none" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><line x1="4" x2="40" y1="11" y2="11"/></svg> }.into_any()
        }
        Some(id) => view! {
            <svg class="swatch" viewBox="0 0 44 22" aria-hidden="true" focusable="false">
                <rect width="44" height="22" fill="var(--sheet)" stroke="none"/>
                <rect width="44" height="22" fill=format!("url(#c-{id})") stroke="none"/>
            </svg>
        }
        .into_any(),
    }
}

fn direction_swatch(id: Option<&'static str>) -> AnyView {
    match id {
        None => view! { <svg class="swatch none" viewBox="0 0 44 22" aria-hidden="true" focusable="false"><path class="dir-x" d="M8,11 H36 M12,7 L8,11 L12,15 M32,7 L36,11 L32,15"/></svg> }.into_any(),
        Some(id) => view! { <svg class="swatch none" viewBox="0 0 44 22" aria-hidden="true" focusable="false" inner_html=crate::street::svg::dir_glyph(id, 22.0, 11.0)></svg> }.into_any(),
    }
}

/// One choice in a group where exactly one is chosen.
fn radio(checked: impl Fn() -> bool + Send + Sync + 'static, glyph: AnyView, name: String, on_click: impl Fn() + 'static) -> impl IntoView {
    let tab = {
        // Only the chosen one is in the tab order; the arrows move between them.
        let checked = std::sync::Arc::new(checked);
        let c = checked.clone();
        (move || c().to_string(), move || if checked() { "0" } else { "-1" })
    };
    view! {
        <li>
            <button type="button" class="opt" role="radio" aria-checked=tab.0 tabindex=tab.1 on:click=move |_| on_click()>
                {glyph}<span>{name}</span>{tick()}
            </button>
        </li>
    }
}

/// Arrows move the choice in a group, as native radios do.
fn radio_keys(e: KeyboardEvent) {
    let step = match e.key().as_str() {
        "ArrowDown" | "ArrowRight" => 1,
        "ArrowUp" | "ArrowLeft" => -1,
        _ => return,
    };
    if e.meta_key() || e.ctrl_key() || e.alt_key() {
        return;
    }
    let Some(cur) = e.target().and_then(|t| t.dyn_into::<Element>().ok()).and_then(|t| t.closest("[role=\"radio\"]").ok().flatten()) else { return };
    let Some(group) = cur.closest("[role=\"radiogroup\"]").ok().flatten() else { return };
    e.prevent_default();
    let Ok(all) = group.query_selector_all("[role=\"radio\"]") else { return };
    let items: Vec<Element> = (0..all.length()).filter_map(|i| all.item(i)?.dyn_into::<Element>().ok()).collect();
    let at = items.iter().position(|i| *i == cur).unwrap_or(0) as i32;
    let next = &items[(at + step).rem_euclid(items.len() as i32) as usize];
    if let Ok(next) = next.clone().dyn_into::<leptos::web_sys::HtmlElement>() {
        let _ = next.focus();
        next.click();
    }
}

fn section(id: &'static str, heading: &'static str, note: impl IntoView, body: impl IntoView) -> impl IntoView {
    view! { <section class="insp-sec"><h3 class="note-h" id=id>{heading}</h3>{note}{body}</section> }
}

// ---- the panel ------------------------------------------------------------------------------

/// The kind a piece is at the time shown, which decides what can be set on it.
fn kind_of(w: SheetWatch, uid: u32) -> usize {
    w.segment(uid, |s| s.kind).unwrap_or(0)
}

fn width_section(w: SheetWatch, uid: u32) -> impl IntoView {
    let value = move || w.segment(uid, |s| w.units().fixed(s.width_mm, 2)).unwrap_or_default();
    let range = move || w.segment(uid, |s| width_range(s, w.units())).unwrap_or_default();
    let (lo, hi) = (
        move || w.segment(uid, |s| w.units().fixed(s.min_mm, 2)).unwrap_or_default(),
        move || w.segment(uid, |s| w.units().fixed(s.max_mm, 2)).unwrap_or_default(),
    );
    let step = move || if w.units() == Units::Metres { "0.1" } else { "0.25" };
    let nudge = move |dir: i32| {
        w.edit(|e| e.nudge_width(uid, dir * w.units_now().step_mm()));
    };
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-width">"Width"</h3>
            <div class="stepper">
                <button type="button" class="ico" aria-label=move || step_label(false, w.units()) on:click=move |_| nudge(-1)>{icon("M3 7h8")}</button>
                <span class="wfield">
                    <input
                        type="number"
                        id="width"
                        inputmode="decimal"
                        step=step
                        min=lo
                        max=hi
                        value=value
                        prop:value=value
                        aria-labelledby="i-h-width"
                        on:change=move |e| {
                            let input = leptos::prelude::event_target::<HtmlInputElement>(&e);
                            match input.value().parse::<f64>() {
                                Ok(v) if v.is_finite() => {
                                    let mm = w.units_now().mm(v);
                                    w.edit(|ed| ed.set_width(uid, mm));
                                }
                                _ => input.set_value(&w.segment(uid, |s| w.units_now().fixed(s.width_mm, 2)).unwrap_or_default()),
                            }
                        }
                    />
                    <span class="unit-tag" aria-hidden="true">{move || w.units().word()}</span>
                </span>
                <button type="button" class="ico" aria-label=move || step_label(true, w.units()) on:click=move |_| nudge(1)>{icon("M3 7h8M7 3v8")}</button>
            </div>
            <p class="insp-range">{range}</p>
        </section>
    }
}

fn surface_section(w: SheetWatch, uid: u32) -> impl IntoView {
    let kind = Memo::new(move |_| kind_of(w, uid));
    move || {
        let k = &KINDS[kind.get()];
        let planting = k.id == "planting";
        let options = k
            .materials
            .iter()
            .map(|&m| {
                let mat = &MATERIALS[m];
                radio(
                    move || w.segment(uid, |s| s.material == mat.id).unwrap_or(false),
                    surface_swatch(mat.id),
                    mat.name.to_string(),
                    move || {
                        w.edit(|e| e.set_material(uid, m));
                    },
                )
            })
            .collect_view();
        view! {
            <section class="insp-sec">
                <h3 class="note-h" id="i-h-surface">{if planting { "Planting" } else { "Surface" }}</h3>
                <p class="insp-range">{if planting { "What is planted in it." } else { "What it is paved with." }}</p>
                <ul class="opts" role="radiogroup" aria-labelledby="i-h-surface" on:keydown=radio_keys>{options}</ul>
            </section>
        }
    }
}

fn position_section(w: SheetWatch, uid: u32) -> impl IntoView {
    let place = move || {
        let v = w.view();
        v.segments.iter().position(|s| s.uid == uid).map(|i| (i, v.segments.len()))
    };
    let go = move |dir: i32| {
        if let Some((i, n)) = place() {
            let j = i as i32 + dir;
            if j >= 0 && (j as usize) < n {
                w.edit(|e| e.move_to(uid, j as usize));
            }
        }
    };
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-pos">"Position"</h3>
            <div class="move-btns">
                <button type="button" class="btn" disabled=move || place().is_none_or(|(i, _)| i == 0) on:click=move |_| go(-1)>
                    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M12 7H2M6 3 2 7l4 4"/></svg>
                    "Move left"
                </button>
                <button type="button" class="btn" disabled=move || place().is_none_or(|(i, n)| i + 1 == n) on:click=move |_| go(1)>
                    "Move right"
                    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M2 7h10M8 3l4 4-4 4"/></svg>
                </button>
            </div>
        </section>
    }
}

fn vehicle_section(w: SheetWatch, uid: u32) -> impl IntoView {
    let tram = move || w.segment(uid, |s| s.tram).unwrap_or(false);
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-veh">"Vehicle"</h3>
            <div class="units" role="group" aria-labelledby="i-h-veh">
                <button type="button" class="unit" aria-pressed=move || (!tram()).to_string() on:click=move |_| { w.edit(|e| e.set_tram(uid, false)); }>"Bus"</button>
                <button type="button" class="unit" aria-pressed=move || tram().to_string() on:click=move |_| { w.edit(|e| e.set_tram(uid, true)); }>"Tram"</button>
            </div>
        </section>
    }
}

/// One row of "other times": the type, its direction, when it is, and a way to remove it.
fn variant_row(w: SheetWatch, uid: u32, vi: usize) -> impl IntoView {
    let get = move |f: &dyn Fn(&crate::street::model::VariantView) -> String| w.segment(uid, |s| s.variants.get(vi).map(f)).flatten().unwrap_or_default();
    let now = move || w.segment(uid, |s| s.active_variant == Some(vi)).unwrap_or(false);
    let kind = move || w.segment(uid, |s| s.variants.get(vi).map(|v| v.kind)).flatten().unwrap_or(0);
    let kinds = move || w.segment(uid, |s| variant_kinds(&s.alt_kinds, s.variants.get(vi).map_or(0, |v| v.kind))).unwrap_or_default();
    let direction_rule = move || KINDS[kind()].direction;
    let times = move |from: bool, text: String| {
        let Some(min) = to_min(&text) else { return };
        let (a, b) = w.segment(uid, |s| s.variants.get(vi).map(|v| (v.from_min, v.to_min))).flatten().unwrap_or((0, 0));
        w.edit(|e| if from { e.set_variant_time(uid, vi, min, b) } else { e.set_variant_time(uid, vi, a, min) });
    };
    view! {
        <li class=move || if now() { "var now" } else { "var" }>
            <select
                aria-label=format!("Type {}", vi + 1)
                on:change=move |e| {
                    if let Ok(k) = leptos::prelude::event_target::<HtmlSelectElement>(&e).value().parse::<usize>() {
                        w.edit(|ed| ed.set_variant_kind(uid, vi, k));
                    }
                }
            >
                {move || kinds().into_iter().map(|k| view! { <option value=k.to_string() prop:selected=move || kind() == k>{KINDS[k].name}</option> }).collect_view()}
            </select>
            {move || {
                (direction_rule() != DirectionRule::None).then(|| {
                    let optional = direction_rule() == DirectionRule::Optional;
                    view! {
                        <select
                            aria-label=format!("Direction {}", vi + 1)
                            on:change=move |e| {
                                let value = leptos::prelude::event_target::<HtmlSelectElement>(&e).value();
                                w.edit(|ed| ed.set_variant_direction(uid, vi, direction_choice(&value)));
                            }
                        >
                            {DIRECTIONS
                                .iter()
                                .map(|d| view! { <option value=d.id prop:selected=move || w.segment(uid, |s| s.variants.get(vi).and_then(|v| v.direction) == Some(d.id)).unwrap_or(false)>{d.name}</option> })
                                .collect_view()}
                            {optional.then(|| view! { <option value="both" prop:selected=move || w.segment(uid, |s| s.variants.get(vi).is_some_and(|v| v.direction.is_none())).unwrap_or(false)>"Two-way"</option> })}
                        </select>
                    }
                })
            }}
            <span class="var-times">
                <input type="time" step="900" aria-label="From" prop:value=move || get(&|v| hhmm(v.from_min)) value=move || get(&|v| hhmm(v.from_min)) on:change=move |e| times(true, event_target_value(&e))/>
                <span aria-hidden="true">"to"</span>
                <input type="time" step="900" aria-label="To" prop:value=move || get(&|v| hhmm(v.to_min)) value=move || get(&|v| hhmm(v.to_min)) on:change=move |e| times(false, event_target_value(&e))/>
            </span>
            {move || {
                let days = get(&|v| v.days.clone().unwrap_or_default());
                (!days.is_empty()).then(|| view! { <span class="var-days">{days}</span> })
            }}
            <button
                type="button"
                class="ico danger"
                aria-label=move || format!("Remove {} at {} to {}", KINDS[kind()].name.to_lowercase(), get(&|v| hhmm(v.from_min)), get(&|v| hhmm(v.to_min)))
                on:click=move |_| { w.edit(|e| e.remove_variant(uid, vi)); }
            >
                {icon("M3 3l8 8M11 3l-8 8")}
            </button>
        </li>
    }
}

fn times_section(w: SheetWatch, uid: u32) -> impl IntoView {
    let count = Memo::new(move |_| w.segment(uid, |s| s.variants.len()).unwrap_or(0));
    let base = move || w.segment(uid, |s| format!("{} the rest of the day.", KINDS[s.base_kind].name)).unwrap_or_default();
    let can_add = move || w.segment(uid, |s| !s.alt_kinds.is_empty()).unwrap_or(false);
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-times">"Other times"</h3>
            <p class="insp-range">{base}</p>
            <ul class="vars"><For each=move || 0..count.get() key=|i| *i children=move |vi| variant_row(w, uid, vi)/></ul>
            <button type="button" class="btn" disabled=move || !can_add() on:click=move |_| { w.edit(|e| e.add_variant(uid)); }>
                <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8M7 3v8"/></svg>
                "Add other times"
            </button>
        </section>
    }
}

fn direction_section(w: SheetWatch, uid: u32, optional: bool) -> impl IntoView {
    let chosen = move |id: Option<&'static str>| move || w.segment(uid, |s| s.direction == id).unwrap_or(false);
    let options = DIRECTIONS
        .iter()
        .enumerate()
        .map(|(i, d)| {
            radio(chosen(Some(d.id)), direction_swatch(Some(d.id)), d.name.to_string(), move || {
                w.edit(|e| e.set_direction(uid, Some(i)));
            })
        })
        .collect_view();
    let both = optional.then(|| {
        radio(chosen(None), direction_swatch(None), "Two-way".to_string(), move || {
            w.edit(|e| e.set_direction(uid, None));
        })
    });
    section(
        "i-h-dir",
        "Direction",
        view! { <p class="insp-range">"Which way traffic goes."</p> },
        view! { <ul class="opts" role="radiogroup" aria-labelledby="i-h-dir" on:keydown=radio_keys>{options}{both}</ul> },
    )
}

fn curb_section(w: SheetWatch, uid: u32, curbs: &'static [usize]) -> impl IntoView {
    let options = curbs
        .iter()
        .map(|&ci| {
            let c = &CURBS[ci];
            radio(
                move || w.segment(uid, |s| s.curb == Some(c.id)).unwrap_or(false),
                curb_swatch(Some(c.id)),
                c.name.to_string(),
                move || {
                    w.edit(|e| e.set_curb(uid, Some(ci)));
                },
            )
        })
        .collect_view();
    let none = radio(
        move || w.segment(uid, |s| s.curb.is_none()).unwrap_or(false),
        curb_swatch(None),
        "None (flush)".to_string(),
        move || {
            w.edit(|e| e.set_curb(uid, None));
        },
    );
    section(
        "i-h-curb",
        "Curb",
        view! { <p class="insp-range">"The raised edge, if it has one."</p> },
        view! { <ul class="opts" role="radiogroup" aria-labelledby="i-h-curb" on:keydown=radio_keys>{options}{none}</ul> },
    )
}

/// The rarer settings: other times, direction and curb, under one disclosure.
/// It stays open once opened, and is open for a piece that already has other
/// times, since those change what the drawing shows.
fn more_section(w: SheetWatch, uid: u32, open: RwSignal<bool>) -> impl IntoView {
    let kind = Memo::new(move |_| kind_of(w, uid));
    let has_times = Memo::new(move |_| w.segment(uid, |s| !s.alt_kinds.is_empty() || !s.variants.is_empty()).unwrap_or(false));
    let forced = move || w.segment(uid, |s| !s.variants.is_empty()).unwrap_or(false);
    move || {
        let k = &KINDS[kind.get()];
        let any = has_times.get() || k.direction != DirectionRule::None || k.has_curb;
        any.then(|| {
            view! {
                <details
                    class="insp-more"
                    prop:open=move || open.get() || forced()
                    on:toggle=move |e| {
                        if !forced() {
                            open.set(leptos::prelude::event_target::<leptos::web_sys::HtmlDetailsElement>(&e).open());
                        }
                    }
                >
                    <summary>"More about this piece"</summary>
                    {has_times.get().then(|| times_section(w, uid))}
                    {(k.direction != DirectionRule::None).then(|| direction_section(w, uid, k.direction == DirectionRule::Optional))}
                    {k.has_curb.then(|| curb_section(w, uid, k.curbs))}
                </details>
            }
        })
    }
}

fn piece_panel(w: SheetWatch, uid: u32, more_open: RwSignal<bool>) -> AnyView {
    let kind = Memo::new(move |_| kind_of(w, uid));
    let name = move || KINDS[kind.get()].name;
    let sub = move || {
        let (v, units) = w.now();
        v.segments.iter().find(|s| s.uid == uid).map(|s| piece_sub(&v, s, units)).unwrap_or_default()
    };
    view! {
        {move || {
            let k = &KINDS[kind.get()];
            view! { <div class="insp-head">{swatch(k.id)}<div><h2 class="insp-name">{name()}</h2><p class="insp-sub">{sub}</p></div></div> }
        }}
        {width_section(w, uid)}
        {surface_section(w, uid)}
        {position_section(w, uid)}
        {move || (KINDS[kind.get()].id == "bus").then(|| vehicle_section(w, uid))}
        {more_section(w, uid, more_open)}
    }
    .into_any()
}

#[component]
pub fn StreetInspector(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let more_open = RwSignal::new(false);
    // The panel is drawn again only when the selection changes; edits change
    // what is in it, not what it is made of.
    let selected = Memo::new(move |_| w.view().selected);
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        // The colour of the piece, for the panel's accents.
        let Some(panel) = document().get_element_by_id("inspector") else { return };
        let style = leptos::wasm_bindgen::JsCast::unchecked_into::<leptos::web_sys::HtmlElement>(panel).style();
        match selected.get().and_then(|u| w.segment(u, |s| KINDS[s.kind].id)) {
            Some(id) => {
                let _ = style.set_property("--kc", &format!("var(--k-{id})"));
            }
            None => {
                let _ = style.remove_property("--kc");
            }
        }
    });
    move || match selected.get() {
        Some(uid) => piece_panel(w, uid, more_open),
        None => view! { <p class="insp-empty">"Select a piece to change its width and surface."</p> }.into_any(),
    }
}

#[cfg(test)]
mod pure_tests {
    use super::*;
    use crate::street::model::Editor;

    #[test]
    fn the_kinds_a_list_offers_are_the_alternatives_and_the_current_one_once_each_in_order() {
        assert_eq!(variant_kinds(&[8, 2, 5], 5), vec![2, 5, 8]);
        assert_eq!(variant_kinds(&[], 3), vec![3]);
        assert_eq!(variant_kinds(&[1, 1], 0), vec![0, 1]);
    }

    #[test]
    fn the_allowed_width_is_written_to_a_hundredth_in_the_units_shown() {
        let v = Editor::new(0).view();
        let s = &v.segments[0];
        assert_eq!(width_range(s, Units::Metres), format!("Allowed {:.2} to {:.2} m", s.min_mm as f64 / 1000.0, s.max_mm as f64 / 1000.0));
        assert!(width_range(s, Units::Feet).ends_with(" ft"));
    }

    #[test]
    fn a_piece_is_placed_among_the_pieces_and_a_timed_one_says_the_time() {
        let mut e = Editor::new(0);
        let v = e.view();
        assert_eq!(piece_sub(&v, &v.segments[2], Units::Metres), "3.3 m wide \u{b7} 3 of 6");
        let u = v.segments[1].uid;
        assert!(e.add_variant(u));
        let v = e.view();
        let s = v.segments.iter().find(|s| s.uid == u).unwrap();
        assert!(piece_sub(&v, s, Units::Metres).ends_with(&format!(" \u{b7} {}", hhmm(v.time_min))));
    }

    #[test]
    fn the_steppers_say_how_far_they_move_in_the_units_shown() {
        assert_eq!(step_label(false, Units::Metres), "Narrower by 0.1 m");
        assert_eq!(step_label(true, Units::Feet), "Wider by 1.0 ft");
    }

    #[test]
    fn a_direction_is_picked_by_its_name_and_two_way_by_none() {
        assert_eq!(direction_choice("away"), Some(0));
        assert_eq!(direction_choice("toward"), Some(1));
        assert_eq!(direction_choice("both"), None);
    }
}
