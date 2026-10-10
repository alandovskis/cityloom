//! The panel beside the plan: what is selected, and the controls to change it.
//! Every control reads the model's view and edits through the model; a control
//! that stays on the panel keeps its element, and so its focus, across edits.
//! Its words are asked for in the views' closures, so they follow a switch of language.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use crate::junction::model::*;
use crate::junction::read_model::{CrossingView, JView};
use crate::junction::turns::{compass_key, turn_glyph};
use crate::junction::vm::JunctionVm;
use crate::junction::watch::Watch;
use crate::shared::i18n::{Args, I18n};
use crate::shared::units::{Units, parse_number};

// ---- text the panel is made of ---------------------------------------------

/// The message of a lane's turn button, for a screen reader: "Lane 1 to Main Street (south), straight on".
fn dest_key(class: u8) -> &'static str {
    match class {
        LEFT => "jn-insp-dest-left",
        THROUGH => "jn-insp-dest-through",
        _ => "jn-insp-dest-right",
    }
}

/// The message of where a lane goes, at the start of a sentence: "Straight on to Main Street (south)".
fn goes_key(class: u8) -> &'static str {
    match class {
        LEFT => "jn-insp-goes-left",
        THROUGH => "jn-insp-goes-through",
        _ => "jn-insp-goes-right",
    }
}

/// Where a lane goes, as its option says it: "Straight on to Main Street (south)", and why it cannot when
/// the street's traffic only comes in.
pub fn goes_text(i18n: &I18n, class: u8, street: String, open: bool) -> String {
    let goes = i18n.tr(goes_key(class), &Args::new().str("street", street));
    if open { goes } else { i18n.tr("jn-insp-one-way-in", &Args::new().str("goes", goes)) }
}

/// The message of where a lane sits among its street's lanes, when that is worth saying.
fn lane_note(lanes: usize, i: usize) -> Option<&'static str> {
    if lanes == 1 {
        Some("jn-insp-lane-only")
    } else if i == 0 {
        Some("jn-insp-lane-middle")
    } else if i + 1 == lanes {
        Some("jn-insp-lane-curb")
    } else {
        None
    }
}

/// What the Crossing section says about the crossing as it stands, in the language of the page.
pub fn crossing_range(c: &CrossingView, i18n: &I18n, units: Units) -> String {
    let locale = i18n.locale();
    let how = if c.stages > 1 {
        i18n.tr("jn-insp-crossing-stages", &Args::new().num("n", c.stages as i64).str("length", units.length_in(c.stage_mm, locale)))
    } else {
        i18n.tr("jn-insp-crossing-one-go", &Args::new().str("length", units.length_in(c.distance_mm, locale)))
    };
    if c.too_far { i18n.tr("jn-insp-too-far", &Args::new().str("crossing", how)) } else { how }
}

/// The lengths a field takes, as the arguments of a message.
fn range_args(i18n: &I18n, units: Units, lo: i32, hi: i32) -> Args {
    let locale = i18n.locale();
    Args::new().str("min", units.length_in(lo, locale)).str("max", units.length_in(hi, locale))
}

fn range_text(i18n: &I18n, units: Units, lo: i32, hi: i32) -> String {
    i18n.tr("jn-insp-range", &range_args(i18n, units, lo, hi))
}

// ---- words, asked for where the view draws them ----------------------------------

/// A message of the panel, as a view draws it, drawn again when the language changes. Every word of the
/// panel is this one closure type, so the views are not made again for each.
fn words(w: Watch, key: &'static str) -> impl Fn() -> String + Copy + Send + Sync + 'static {
    move || w.tr(key)
}

/// A message of the panel with its arguments, drawn again when the language changes.
fn tr(w: Watch, key: &str, args: Args) -> String {
    w.i18n().tr(key, &args)
}

/// A length in the units shown and the language of the page.
fn length(w: Watch, mm: i32) -> String {
    w.units().length_in(mm, w.i18n().locale())
}

/// A message about one length.
fn with_length(w: Watch, key: &str, name: &'static str, mm: i32) -> String {
    tr(w, key, Args::new().str(name, length(w, mm)))
}

/// A stepper's name, for a screen reader: what it does, by how much.
fn stepper(w: Watch, key: &'static str, mm: i32) -> Signal<String> {
    Signal::derive(move || with_length(w, key, "step", mm))
}

// ---- small pieces -------------------------------------------------------------

fn tick() -> impl IntoView {
    view! { <svg class="tick" viewBox="0 0 16 16" aria-hidden="true"><path d="M2.5 8.5 6.5 12.5 13.5 3.5"/></svg> }
}

fn btn_icon(plus: bool) -> impl IntoView {
    let d = if plus { "M3 8h10M8 3v10" } else { "M3.5 3.5l9 9M12.5 3.5l-9 9" };
    view! { <svg class="btn-ico" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false"><path d=d/></svg> }
}

/// A checkbox-like button in a list of options.
fn option_btn(
    checked: impl Fn() -> bool + Send + Sync + 'static,
    disabled: impl Fn() -> bool + Send + Sync + 'static,
    glyph: AnyView,
    name: impl Fn() -> String + Send + Sync + 'static,
    on_click: impl Fn() + 'static,
) -> AnyView {
    option_view(Signal::derive(checked), Signal::derive(disabled), glyph, Signal::derive(name), Box::new(on_click))
}

/// The markup of an option, made once for every option of the panel.
fn option_view(checked: Signal<bool>, disabled: Signal<bool>, glyph: AnyView, name: Signal<String>, on_click: Box<dyn Fn()>) -> AnyView {
    view! {
        <li>
            <button
                type="button"
                class="opt"
                role="checkbox"
                aria-checked=move || checked.get().to_string()
                tabindex="0"
                disabled=move || disabled.get()
                on:click=move |_| on_click()
            >
                {glyph}<span>{move || name.get()}</span>{tick()}
            </button>
        </li>
    }
    .into_any()
}

/// A section with a heading (a message) and its contents.
fn section(w: Watch, id: &'static str, heading: &'static str, body: impl IntoView) -> impl IntoView {
    view! { <section class="insp-sec"><h3 class="note-h" id=id>{words(w, heading)}</h3>{body}</section> }
}

fn hint(text: Signal<String>) -> impl IntoView {
    move || {
        let t = text.get();
        (!t.is_empty()).then(|| view! { <p class="insp-range">{t}</p> })
    }
}

/// What a number field with steppers is made of. The values are read as the
/// panel is drawn; the steppers and the typed value are told what to do.
struct Num {
    id: &'static str,
    /// The message of the field's heading.
    label: &'static str,
    minus: Signal<String>,
    plus: Signal<String>,
    value: Signal<String>,
    tag: Signal<String>,
    hint: Signal<String>,
}

fn num_field(w: Watch, n: Num, on_step: impl Fn(i32) + 'static, on_set: impl Fn(f64) + 'static) -> AnyView {
    num_view(w, n, Rc::new(on_step), Box::new(on_set))
}

/// The markup of a number field, made once for every field of the panel.
fn num_view(w: Watch, n: Num, on_step: Rc<dyn Fn(i32)>, on_set: Box<dyn Fn(f64)>) -> AnyView {
    let (down, up, keyed) = (on_step.clone(), on_step.clone(), on_step);
    let Num { id, label, minus, plus, value, tag, hint: note } = n;
    let hint_id = format!("{id}-hint");
    let described_by = hint_id.clone();
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id=id>{words(w, label)}</h3>
            <div class="stepper">
                <button type="button" class="ico" aria-label=move || minus.get() on:click=move |_| down(-1)>
                    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8"/></svg>
                </button>
                <span class="wfield">
                    <input
                        type="text"
                        inputmode="decimal"
                        autocomplete="off"
                        value=move || value.get()
                        prop:value=move || value.get()
                        aria-labelledby=id
                        aria-describedby=move || (!note.get().is_empty()).then(|| described_by.clone())
                        on:keydown=move |ev| {
                            if let Some(dir) = step_key(&ev.key()) {
                                ev.prevent_default();
                                keyed(dir);
                            }
                        }
                        on:change=move |ev| {
                            let input = event_target::<HtmlInputElement>(&ev);
                            commit_typed(&input.value(), &*on_set);
                            // What the field holds is the model's value, whether it took the edit or refused it.
                            input.set_value(&value.get_untracked());
                        }
                    />
                    <span class="unit-tag" aria-hidden="true">{move || tag.get()}</span>
                </span>
                <button type="button" class="ico" aria-label=move || plus.get() on:click=move |_| up(1)>
                    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8M7 3v8"/></svg>
                </button>
            </div>
            {hint_with_id(note, hint_id)}
        </section>
    }
    .into_any()
}

/// A hint that a field is described by.
fn hint_with_id(text: Signal<String>, id: String) -> impl IntoView {
    move || {
        let t = text.get();
        (!t.is_empty()).then(|| view! { <p class="insp-range" id=id.clone()>{t}</p> })
    }
}

/// The step an arrow key makes in a number field, as the browser's spinner did.
pub fn step_key(key: &str) -> Option<i32> {
    match key {
        "ArrowUp" => Some(1),
        "ArrowDown" => Some(-1),
        _ => None,
    }
}

/// Hands what was typed (a comma or a point for the decimal mark) to `set`;
/// false when it is not a number.
pub fn commit_typed(typed: &str, set: &dyn Fn(f64)) -> bool {
    let Some(v) = parse_number(typed) else { return false };
    set(v);
    true
}

/// Sets the cycle track's width from what was typed, in the units shown; false when it is not a number.
pub fn commit_cycle(w: Watch, typed: &str) -> bool {
    commit_typed(typed, &|v| set_cycle_width(w, v))
}

/// Turns a street to the degrees typed: a fraction is rounded, and the model snaps it to its step.
pub fn commit_bearing(w: Watch, uid: u32, typed: &str) -> bool {
    commit_typed(typed, &|v| set_bearing_degrees(w, uid, v))
}

fn set_bearing_degrees(w: Watch, uid: u32, degrees: f64) {
    w.edit(|j| j.set_bearing(uid, degrees.round() as i32));
}

fn set_cycle_width(w: Watch, typed: f64) {
    w.edit(|j| j.set_cycle(Some(w.units_now().mm(typed))));
}

fn derived(w: Watch, f: impl Fn(&JView, Units) -> String + Send + Sync + 'static) -> Signal<String> {
    Signal::derive(move || f(&w.view(), w.units()))
}

/// A hint of the lengths a field takes.
fn range_hint(w: Watch, lo: i32, hi: i32) -> Signal<String> {
    Signal::derive(move || range_text(&w.i18n(), w.units(), lo, hi))
}

/// A length in the field's units, to `places`, of whatever `mm` reads.
fn length_signal(w: Watch, places: usize, mm: impl Fn(&JView) -> i32 + Send + Sync + 'static) -> Signal<String> {
    derived(w, move |v, u| u.fixed_in(mm(v), places, w.i18n().locale()))
}

fn unit_tag(w: Watch) -> Signal<String> {
    Signal::derive(move || w.units().word().to_string())
}

// ---- the junction's own controls ------------------------------------------------

fn ring_size(w: Watch) -> impl IntoView {
    let spec = Num {
        id: "i-h-ring",
        label: "jn-insp-ring-size",
        minus: stepper(w, "jn-insp-smaller", RING_STEP_MM),
        plus: stepper(w, "jn-insp-larger", RING_STEP_MM),
        value: length_signal(w, 1, |v| v.ring.as_ref().map_or(0, |r| r.radius_mm * 2)),
        tag: Signal::derive(move || tr(w, "jn-insp-unit-across", Args::new().str("unit", w.units().word()))),
        hint: derived(w, move |v, _| with_length(w, "jn-insp-ring-hint", "length", v.ring.as_ref().map_or(0, |r| r.floor_mm * 2))),
    };
    num_field(
        w,
        spec,
        move |dir| {
            w.edit(|j| j.step_ring(dir));
        },
        move |v| {
            w.edit(|j| j.set_ring_radius(w.units_now().mm(v / 2.0)));
        },
    )
}

/// The cycle track's width; `note` is the message of its hint, which says what the width costs.
fn cycle_width(w: Watch, note: &'static str) -> impl IntoView {
    let spec = Num {
        id: "i-h-cycle-width",
        label: "jn-insp-track-width",
        minus: stepper(w, "jn-insp-narrower", RING_STEP_MM),
        plus: stepper(w, "jn-insp-wider", RING_STEP_MM),
        value: length_signal(w, 1, |v| v.ring.as_ref().and_then(|r| r.cycle_mm).unwrap_or(0)),
        tag: unit_tag(w),
        hint: Signal::derive(move || tr(w, note, range_args(&w.i18n(), w.units(), CYCLE_MIN_MM, CYCLE_MAX_MM))),
    };
    num_field(
        w,
        spec,
        move |dir| {
            w.edit(|j| j.step_cycle(dir));
        },
        move |v| set_cycle_width(w, v),
    )
}

/// What the bus lane across a roundabout's island is.
fn bus_note(w: Watch) -> Option<String> {
    w.view().bus.as_ref().map(|b| with_length(w, "jn-insp-bus-note", "width", b.width_mm))
}

fn bus_choice(w: Watch) -> impl IntoView {
    let options = move || {
        let v = w.view();
        let cur = v.bus.as_ref().map(|b| (b.from.min(b.to), b.from.max(b.to)));
        v.bus_options
            .iter()
            .map(|o| {
                let (a, b) = (o.a, o.b);
                view! { <option value=format!("{a}-{b}") prop:selected=cur == Some((a, b))>{w.say(&o.label)}</option> }
            })
            .collect_view()
    };
    let none_selected = move || w.view().bus.is_none();
    let note = Signal::derive(move || bus_note(w).unwrap_or_else(|| w.tr("jn-insp-bus-offer")));
    section(
        w,
        "i-h-bus",
        "jn-insp-bus-across",
        view! {
            <select
                aria-labelledby="i-h-bus"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    let pair = value.split_once('-').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)));
                    w.edit(|j| j.set_bus(pair));
                }
            >
                <option value="" prop:selected=none_selected>{words(w, "jn-insp-bus-none")}</option>
                {options}
            </select>
            {hint(note)}
        },
    )
}

fn cycle_track(w: Watch, cycle: Memo<bool>) -> impl IntoView {
    section(
        w,
        "i-h-cycle",
        "jn-insp-cycle",
        view! {
            <ul class="opts">
                {option_btn(
                    move || cycle.get(),
                    || false,
                    ().into_any(),
                    words(w, "jn-insp-cycle-option"),
                    move || {
                        w.edit(|j| j.set_cycle_track(!cycle.get_untracked()));
                    },
                )}
            </ul>
        },
    )
}

/// The junction's control, and what only a roundabout has.
fn control_section(w: Watch) -> impl IntoView {
    let ring = Memo::new(move |_| w.view().ring.is_some());
    let cycle = Memo::new(move |_| w.view().ring.as_ref().is_some_and(|r| r.cycle_mm.is_some()));
    let control = section(
        w,
        "i-h-control",
        "jn-insp-control",
        view! {
            <select
                aria-labelledby="i-h-control"
                on:change=move |ev| {
                    if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                        w.edit(|j| j.set_control(i));
                    }
                }
            >
                {CONTROLS
                    .iter()
                    .enumerate()
                    .map(|(i, c)| view! { <option value=i.to_string() prop:selected=move || w.view().control_index == i>{words(w, c.key)}</option> })
                    .collect_view()}
            </select>
        },
    );
    view! {
        {control}
        {move || ring.get().then(|| ring_size(w))}
        {move || ring.get().then(|| cycle_track(w, cycle))}
        {move || (ring.get() && cycle.get()).then(|| cycle_width(w, "jn-insp-track-hint-inside"))}
        {move || ring.get().then(|| bus_choice(w))}
    }
}

// ---- what is selected ----------------------------------------------------------

fn head(name: impl Fn() -> String + Send + Sync + 'static, sub: impl Fn() -> String + Send + Sync + 'static) -> AnyView {
    head_view(Signal::derive(name), Signal::derive(sub))
}

/// The markup of the panel's heading, made once for every heading.
fn head_view(name: Signal<String>, sub: Signal<String>) -> AnyView {
    view! { <div class="insp-head"><div><h2 class="insp-name">{move || name.get()}</h2><p class="insp-sub">{move || sub.get()}</p></div></div> }.into_any()
}

fn danger_button(w: Watch, label: &'static str, on_click: impl Fn() + 'static) -> impl IntoView {
    view! { <section class="insp-sec"><button type="button" class="btn danger" on:click=move |_| on_click()>{btn_icon(false)}{words(w, label)}</button></section> }
}

fn bus_panel(w: Watch) -> AnyView {
    let label = move || {
        let v = w.view();
        let Some(b) = v.bus.as_ref() else { return String::new() };
        let pair = (b.from.min(b.to), b.from.max(b.to));
        v.bus_options.iter().find(|o| (o.a, o.b) == pair).map(|o| w.say(&o.label)).unwrap_or_default()
    };
    view! {
        {head(words(w, "jn-insp-bus"), label)}
        <section class="insp-sec">
            {hint(Signal::derive(move || bus_note(w).unwrap_or_default()))}
        </section>
        {control_section(w)}
        {danger_button(w, "jn-insp-bus-remove", move || {
            w.edit(|j| j.set_bus(None));
        })}
    }
    .into_any()
}

fn cycle_panel(w: Watch) -> AnyView {
    view! {
        {head(words(w, "jn-insp-cycle"), words(w, "jn-insp-cycle-sub"))}
        {cycle_width(w, "jn-insp-track-hint-crossing")}
        {control_section(w)}
        {danger_button(w, "jn-insp-cycle-remove", move || {
            w.edit(|j| j.set_cycle_track(false));
        })}
    }
    .into_any()
}

fn junction_panel(w: Watch) -> AnyView {
    view! {
        <p class="insp-empty">{words(w, "jn-insp-empty")}</p>
        {control_section(w)}
    }
    .into_any()
}

/// The short tag of the way an arm leaves, or nothing when there is no such arm.
fn compass_tag(w: Watch, arm: Option<&crate::junction::read_model::ArmView>) -> String {
    arm.map(|a| w.tr(compass_key(a.bearing))).unwrap_or_default()
}

fn corner_panel(w: Watch, uid: u32) -> AnyView {
    let sub = move || {
        let v = w.view();
        let from = compass_tag(w, v.arms.iter().find(|a| a.uid == uid));
        let to = compass_tag(w, v.corners.iter().find(|c| c.uid == uid).and_then(|c| v.arms.iter().find(|a| a.uid == c.next_uid)));
        tr(w, "jn-arm-to-arm", Args::new().str("from", from).str("to", to))
    };
    let spec = Num {
        id: "i-h-corner",
        label: "jn-insp-curb-radius",
        minus: stepper(w, "jn-insp-tighter", RING_STEP_MM),
        plus: stepper(w, "jn-insp-wider", RING_STEP_MM),
        value: length_signal(w, 1, move |v| v.corners.iter().find(|c| c.uid == uid).map_or(0, |c| c.radius_mm)),
        tag: unit_tag(w),
        hint: derived(w, move |v, u| {
            let speed = v.corners.iter().find(|c| c.uid == uid).map_or(0.0, |c| c.speed_kmh);
            tr(w, "jn-insp-corner-hint", range_args(&w.i18n(), u, MIN_CORNER_MM, MAX_CORNER_MM).num("speed", speed.round() as i64))
        }),
    };
    view! {
        {head(words(w, "jn-insp-corner"), sub)}
        {num_field(
            w,
            spec,
            move |dir| {
                w.edit(|j| j.step_corner(uid, dir));
            },
            move |v| {
                w.edit(|j| j.set_corner(uid, w.units_now().mm(v)));
            },
        )}
        <p class="insp-empty">{words(w, "jn-insp-corner-note")}</p>
        {control_section(w)}
    }
    .into_any()
}

/// One street a lane may go to, as a turn button on its row.
fn dest_button(w: Watch, uid: u32, lane: usize, to: u32) -> impl IntoView {
    let v = w.view_now();
    let class = v.arms.iter().find(|a| a.uid == uid).and_then(|a| a.lanes.get(lane)).and_then(|l| l.dests.iter().find(|d| d.uid == to)).map_or(0, |d| d.class);
    let label =
        move || w.arm(uid, |a| a.lanes.get(lane).and_then(|l| l.dests.iter().find(|d| d.uid == to)).map(|d| w.say(&d.label))).flatten().unwrap_or_default();
    let tag = move || compass_tag(w, w.view().arms.iter().find(|x| x.uid == to));
    let state = move || {
        w.arm(uid, |a| a.lanes.get(lane).and_then(|l| l.dests.iter().find(|d| d.uid == to).map(|d| (d.on, d.open, l.bad))))
            .flatten()
            .unwrap_or((false, false, false))
    };
    let name = move || tr(w, dest_key(class), Args::new().num("n", lane as i64 + 1).str("street", label()));
    view! {
        <button
            type="button"
            class=move || {
                let (on, _, bad) = state();
                format!("turn dest{}{}", if on { " on" } else { "" }, if on && bad { " bad" } else { "" })
            }
            aria-pressed=move || state().0.to_string()
            disabled=move || !state().1
            title=label
            aria-label=name
            on:click=move |_| {
                let on = state().0;
                w.edit(|j| j.set_lane_dest(uid, lane, to, !on));
            }
        >
            {turn_glyph(class, 16)}
            <span aria-hidden="true">{tag}</span>
        </button>
    }
}

/// The streets a lane can go to, in the order the model lists them.
fn dest_uids(w: Watch, uid: u32, lane: usize) -> Memo<Vec<u32>> {
    Memo::new(move |_| w.arm(uid, |a| a.lanes.get(lane).map(|l| l.dests.iter().map(|d| d.uid).collect::<Vec<_>>())).flatten().unwrap_or_default())
}

fn lane_row(w: Watch, uid: u32, i: usize) -> impl IntoView {
    let dests = dest_uids(w, uid, i);
    let bad = move || w.arm(uid, |a| a.lanes.get(i).is_some_and(|l| l.bad)).unwrap_or(false);
    let note = move || w.arm(uid, |a| lane_note(a.lanes.len(), i)).flatten().map(|key| w.tr(key)).unwrap_or_default();
    view! {
        <li class=move || if bad() { "lane-row bad" } else { "lane-row" }>
            <button type="button" class="lane-pick" on:click=move |_| w.select(Target::Lane(uid, i))>
                <span>{move || tr(w, "jn-insp-lane", Args::new().num("n", i as i64 + 1))}<small>{note}</small></span>
            </button>
            <span class="turns-btns">
                <For each=move || dests.get() key=|d| *d children=move |d| dest_button(w, uid, i, d)/>
            </span>
        </li>
    }
}

fn lane_panel(w: Watch, uid: u32, lane: usize) -> AnyView {
    let dests = dest_uids(w, uid, lane);
    let sub = move || {
        w.arm(uid, |a| match lane_note(a.lanes.len(), lane) {
            Some(key) => tr(w, "jn-insp-lane-sub", Args::new().str("arm", w.say(&a.label)).str("place", w.tr(key))),
            None => w.say(&a.label),
        })
        .unwrap_or_default()
    };
    let title =
        move || w.arm(uid, |a| tr(w, "jn-insp-lane-title", Args::new().num("n", lane as i64 + 1).num("count", a.lanes.len() as i64))).unwrap_or_default();
    let range = Signal::derive(move || {
        w.arm(uid, |a| match a.lanes.get(lane) {
            Some(l) if l.bad => w.tr("jn-insp-lane-banned"),
            Some(l) => with_length(w, "jn-insp-lane-width", "width", l.width_mm),
            None => String::new(),
        })
        .unwrap_or_default()
    });
    let option = move |to: u32| {
        let dest = move || {
            w.arm(uid, |a| a.lanes.get(lane).and_then(|l| l.dests.iter().find(|d| d.uid == to)).map(|d| (d.class, w.say(&d.label), d.on, d.open))).flatten()
        };
        let class = w
            .view_now()
            .arms
            .iter()
            .find(|a| a.uid == uid)
            .and_then(|a| a.lanes.get(lane))
            .and_then(|l| l.dests.iter().find(|d| d.uid == to))
            .map_or(0, |d| d.class);
        let state = move || dest().map_or((false, false), |(_, _, on, open)| (on, open));
        option_btn(
            move || state().0,
            move || !state().1,
            turn_glyph(class, 22).into_any(),
            move || {
                let (class, label, _, open) = dest().unwrap_or((class, String::new(), false, false));
                goes_text(&w.i18n(), class, label, open)
            },
            move || {
                let on = state().0;
                w.edit(|j| j.set_lane_dest(uid, lane, to, !on));
            },
        )
    };
    view! {
        {head(title, sub)}
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-serves">{words(w, "jn-insp-lane-goes")}</h3>
            <ul class="opts"><For each=move || dests.get() key=|d| *d children=option/></ul>
            {hint(range)}
        </section>
        <section class="insp-sec">
            <button type="button" class="btn" on:click=move |_| w.select(Target::Arm(uid))>{words(w, "jn-insp-whole-street")}</button>
        </section>
        {control_section(w)}
    }
    .into_any()
}

// ---- a street ------------------------------------------------------------------

fn crossing_fields(w: Watch, uid: u32) -> impl IntoView {
    let range = Signal::derive(move || w.arm(uid, |a| a.crossing.as_ref().map(|c| crossing_range(c, &w.i18n(), w.units()))).flatten().unwrap_or_default());
    let island = move || w.arm(uid, |a| a.crossing.as_ref().is_some_and(|c| c.island)).unwrap_or(false);
    let can_island = move || w.arm(uid, |a| a.can_island).unwrap_or(false);
    let setback = Num {
        id: "i-h-setback",
        label: "jn-insp-setback",
        minus: stepper(w, "jn-insp-closer", RING_STEP_MM),
        plus: stepper(w, "jn-insp-farther", RING_STEP_MM),
        value: length_signal(w, 1, move |v| v.arms.iter().find(|a| a.uid == uid).and_then(|a| a.crossing.as_ref()).map_or(0, |c| c.setback_mm)),
        tag: unit_tag(w),
        hint: range_hint(w, MIN_SETBACK_MM, MAX_SETBACK_MM),
    };
    let width = Num {
        id: "i-h-cwidth",
        label: "jn-insp-crossing-width",
        minus: stepper(w, "jn-insp-narrower", RING_STEP_MM),
        plus: stepper(w, "jn-insp-wider", RING_STEP_MM),
        value: length_signal(w, 1, move |v| v.arms.iter().find(|a| a.uid == uid).and_then(|a| a.crossing.as_ref()).map_or(0, |c| c.width_mm)),
        tag: unit_tag(w),
        hint: range_hint(w, MIN_CROSSING_MM, MAX_CROSSING_MM),
    };
    let bulb = move |side: usize| {
        let has = move || w.arm(uid, |a| a.can_bulb[side]).unwrap_or(false);
        option_btn(
            move || w.arm(uid, |a| a.bulbs[side].is_some()).unwrap_or(false),
            move || !has(),
            ().into_any(),
            move || {
                let bulb = w.tr(if side == 0 { "jn-insp-bulb-left" } else { "jn-insp-bulb-right" });
                if has() { bulb } else { tr(w, "jn-insp-no-parking", Args::new().str("bulb", bulb)) }
            },
            move || {
                let on = w.arm(uid, |a| a.bulbs[side].is_some()).unwrap_or(false);
                w.edit(|j| j.set_bulb(uid, side, !on));
            },
        )
    };
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-cross">{words(w, "jn-insp-crossing")}</h3>
            {hint(range)}
            <button type="button" class="btn" on:click=move |_| { w.edit(|j| j.set_crossing(uid, false)); }>{btn_icon(false)}{words(w, "jn-insp-crossing-remove")}</button>
        </section>
        {num_field(
            w,
            setback,
            move |dir| {
                w.edit(|j| j.step_setback(uid, dir));
            },
            move |v| {
                w.edit(|j| j.set_setback(uid, w.units_now().mm(v)));
            },
        )}
        {num_field(
            w,
            width,
            move |dir| {
                w.edit(|j| j.step_crossing_width(uid, dir));
            },
            move |v| {
                w.edit(|j| j.set_crossing_width(uid, w.units_now().mm(v)));
            },
        )}
        {section(
            w,
            "i-h-refuge",
            "jn-insp-halfway",
            view! {
                <ul class="opts">
                    {option_btn(
                        island,
                        move || !can_island(),
                        ().into_any(),
                        move || w.tr(if can_island() { "jn-insp-island" } else { "jn-insp-island-narrow" }),
                        move || { w.edit(|j| j.set_island(uid, !island())); },
                    )}
                </ul>
            },
        )}
        {section(w, "i-h-bulb", "jn-insp-shorten", view! { <ul class="opts">{bulb(0)}{bulb(1)}</ul> })}
    }
}

fn crossing_section(w: Watch, uid: u32) -> impl IntoView {
    let has = Memo::new(move |_| w.arm(uid, |a| a.crossing.is_some()).unwrap_or(false));
    move || {
        if has.get() {
            crossing_fields(w, uid).into_any()
        } else {
            view! {
                <section class="insp-sec">
                    <h3 class="note-h" id="i-h-cross">{words(w, "jn-insp-crossing")}</h3>
                    <button type="button" class="btn" on:click=move |_| { w.edit(|j| j.set_crossing(uid, true)); }>{btn_icon(true)}{words(w, "jn-insp-crossing-mark")}</button>
                </section>
            }
            .into_any()
        }
    }
}

/// A measure's name in a list, after its Atlas code when it has one.
fn coded(w: Watch, code: &str, key: &str) -> String {
    if code.is_empty() { w.tr(key) } else { tr(w, "jn-insp-coded", Args::new().str("code", code.to_string()).str("name", w.tr(key))) }
}

/// A list of items to choose one from, by position in a catalogue; `label` is the message of its name.
fn choice(
    w: Watch,
    key: &'static str,
    label: &'static str,
    items: &'static [Item],
    current: impl Fn(&crate::junction::read_model::TransitView) -> usize + Send + Sync + Copy + 'static,
    set: impl Fn(&mut Junction, usize) -> bool + Copy + 'static,
    uid: u32,
) -> impl IntoView {
    let id = format!("i-h-{key}");
    let labelled = id.clone();
    view! {
        <label class="fld">
            <span id=id>{words(w, label)}</span>
            <select
                aria-labelledby=labelled
                on:change=move |ev| {
                    if let Ok(i) = event_target::<HtmlSelectElement>(&ev).value().parse::<usize>() {
                        w.edit(|j| set(j, i));
                    }
                }
            >
                {items
                    .iter()
                    .enumerate()
                    .map(|(i, o)| {
                        view! { <option value=i.to_string() prop:selected=move || w.arm(uid, |a| current(&a.transit) == i).unwrap_or(false)>{move || coded(w, o.code, o.key)}</option> }
                    })
                    .collect_view()}
            </select>
        </label>
    }
}

fn transit_section(w: Watch, uid: u32) -> impl IntoView {
    let bus_lane = move || w.arm(uid, |a| a.transit.bus_lane).unwrap_or(false);
    let filter = move || w.arm(uid, |a| a.transit.filter).unwrap_or(false);
    let kind = Memo::new(move |_| w.arm(uid, |a| a.transit.approach_kind).unwrap_or("none"));
    let problems = move || {
        let list = w.arm(uid, |a| a.transit.problems.clone()).unwrap_or_default();
        (!list.is_empty()).then(|| view! { <ul class="problems">{list.into_iter().map(|p| view! { <li>{w.say(&p)}</li> }).collect_view()}</ul> })
    };
    let length_field = move || {
        let queue = kind.get() == "queue";
        matches!(kind.get(), "queue" | "gate").then(|| {
            let spec = Num {
                id: "i-h-alen",
                label: if queue { "jn-insp-queue-length" } else { "jn-insp-gate-distance" },
                minus: stepper(w, "jn-insp-shorter", APPROACH_STEP_MM),
                plus: stepper(w, "jn-insp-longer", APPROACH_STEP_MM),
                value: length_signal(w, 1, move |v| v.arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.transit.approach_mm)),
                tag: unit_tag(w),
                hint: range_hint(w, APPROACH_MIN_MM, APPROACH_MAX_MM),
            };
            num_field(
                w,
                spec,
                move |dir| {
                    w.edit(|j| j.step_approach_len(uid, dir));
                },
                move |v| {
                    w.edit(|j| j.set_approach_len(uid, w.units_now().mm(v)));
                },
            )
        })
    };
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-transit">{words(w, "jn-insp-transit")}</h3>
            <ul class="opts">
                {option_btn(
                    bus_lane,
                    || false,
                    ().into_any(),
                    words(w, "jn-insp-bus-lane-in"),
                    move || { w.edit(|j| j.set_bus_lane(uid, !bus_lane())); },
                )}
            </ul>
            {choice(w, "approach", "jn-insp-approach", &APPROACHES, |t| t.approach, move |j, i| j.set_approach(uid, i), uid)}
            {choice(w, "stop", "jn-insp-stop", &STOPS, |t| t.stop, move |j, i| j.set_stop(uid, i), uid)}
            {choice(w, "rule", "jn-insp-rule", &RULES, |t| t.rule, move |j, i| j.set_rule(uid, i), uid)}
            <ul class="opts">
                {option_btn(
                    filter,
                    || false,
                    ().into_any(),
                    move || coded(w, FILTER_CODE, "jn-insp-filter"),
                    move || { w.edit(|j| j.set_filter(uid, !filter())); },
                )}
            </ul>
            {problems}
        </section>
        {length_field}
    }
}

fn arm_panel(w: Watch, uid: u32, crossing_only: bool) -> AnyView {
    let name = move || w.arm(uid, |a| w.say(&a.street)).unwrap_or_default();
    let sub = move || {
        w.arm(uid, |a| {
            let args = Args::new().str("compass", w.tr(compass_key(a.bearing))).num("degrees", a.bearing as i64).str("width", length(w, a.road_mm));
            tr(w, "jn-insp-arm-sub", args)
        })
        .unwrap_or_default()
    };
    if crossing_only {
        return view! { {head(name, sub)} {crossing_section(w, uid)} {control_section(w)} }.into_any();
    }
    let enters = Memo::new(move |_| w.arm(uid, |a| a.enters).unwrap_or(false));
    let lanes = Memo::new(move |_| w.arm(uid, |a| a.lanes.len()).unwrap_or(0));
    let linked = w.view_now().linked;
    let direction = Num {
        id: "i-h-bearing",
        label: "jn-insp-direction",
        minus: Signal::derive(move || tr(w, "jn-insp-anticlockwise", Args::new().num("degrees", BEARING_STEP as i64))),
        plus: Signal::derive(move || tr(w, "jn-insp-clockwise", Args::new().num("degrees", BEARING_STEP as i64))),
        value: Signal::derive(move || w.arm(uid, |a| a.bearing.to_string()).unwrap_or_default()),
        tag: Signal::derive(|| "°".to_string()),
        hint: Signal::derive(move || tr(w, "jn-insp-direction-hint", Args::new().num("degrees", MIN_SEPARATION as i64))),
    };
    let offset = Num {
        id: "i-h-offset",
        label: "jn-insp-shift",
        minus: stepper(w, "jn-insp-shift-left", OFFSET_STEP_MM),
        plus: stepper(w, "jn-insp-shift-right", OFFSET_STEP_MM),
        value: length_signal(w, 2, move |v| v.arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.offset_mm)),
        tag: unit_tag(w),
        hint: derived(w, move |v, _| with_length(w, "jn-insp-shift-hint", "length", v.arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.max_offset_mm))),
    };
    let street = {
        let edge = w.view_now().arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.edge);
        section(
            w,
            "i-h-street",
            "jn-insp-street",
            view! {
                <a class="btn" href=format!("street.html?street={edge}")>{words(w, "jn-insp-cross-section")}</a>
                <p class="insp-range">{words(w, "jn-insp-street-note")}</p>
            },
        )
        .into_any()
    };
    let remove = (!linked).then(|| {
        let can = move || w.view().can_remove;
        view! {
            <section class="insp-sec">
                <button type="button" class="btn danger" disabled=move || !can() on:click=move |_| { w.edit(|j| j.remove_arm(uid)); }>
                    {btn_icon(false)}{words(w, "jn-insp-remove-street")}
                </button>
                {move || (!can()).then(|| view! { <p class="insp-range">{move || tr(w, "jn-insp-min-arms", Args::new().num("n", MIN_ARMS as i64))}</p> })}
            </section>
        }
    });
    view! {
        {head(name, sub)}
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-lanes">{words(w, "jn-insp-lanes-in")}</h3>
            {move || if enters.get() {
                view! { <ul class="lanes"><For each=move || 0..lanes.get() key=|i| *i children=move |i| lane_row(w, uid, i)/></ul> }.into_any()
            } else {
                view! { <p class="insp-range">{words(w, "jn-insp-one-way-out")}</p> }.into_any()
            }}
        </section>
        {crossing_section(w, uid)}
        {transit_section(w, uid)}
        {num_field(
            w,
            direction,
            move |dir| { w.edit(|j| j.step_bearing(uid, dir)); },
            move |v| set_bearing_degrees(w, uid, v),
        )}
        {num_field(
            w,
            offset,
            move |dir| { w.edit(|j| j.step_offset(uid, dir)); },
            move |v| { w.edit(|j| j.set_offset(uid, w.units_now().mm(v))); },
        )}
        {street}
        {control_section(w)}
        {remove}
    }
    .into_any()
}

// ---- the panel -----------------------------------------------------------------------

#[component]
pub fn Inspector(vm: Rc<JunctionVm>) -> impl IntoView {
    let w = Watch::new(vm);
    // The panel is drawn again only when what is selected changes; edits change
    // what is in it, not what it is made of.
    let selected = Memo::new(move |_| {
        let s = &w.view().selected;
        (s.kind, s.uid, s.lane)
    });
    move || match selected.get() {
        (Some("bus"), ..) => bus_panel(w),
        (Some("cycle"), ..) => cycle_panel(w),
        (Some("lane"), uid, lane) => lane_panel(w, uid, lane),
        (Some("corner"), uid, _) => corner_panel(w, uid),
        (Some("crossing"), uid, _) => arm_panel(w, uid, true),
        (Some("arm"), uid, _) => arm_panel(w, uid, false),
        _ => junction_panel(w),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;

    fn crossing(j: &Junction, bearing: i32) -> CrossingView {
        j.view().arms.into_iter().find(|a| a.bearing == bearing).unwrap().crossing.unwrap()
    }

    #[test]
    fn a_number_field_with_an_empty_hint_is_not_described_by_one() {
        let vm = crate::junction::vm::JunctionVm::new(crate::shared::platform::browser_ports(), crate::i18n_for(Locale::En), Junction::new(0), None);
        let w = Watch::new(vm);
        let field = |hint: &'static str| Num {
            id: "f",
            label: "jn-insp-ring-hint",
            minus: Signal::derive(String::new),
            plus: Signal::derive(String::new),
            value: Signal::derive(|| "1".to_string()),
            tag: Signal::derive(String::new),
            hint: Signal::derive(move || hint.to_string()),
        };
        let html = |hint| crate::shared::testing::html(move || num_view(w, field(hint), Rc::new(|_| {}), Box::new(|_| {})));
        assert!(!html("").contains("aria-describedby"));
        assert!(html("1 m to 2 m").contains("aria-describedby=\"f-hint\""));
    }

    #[test]
    fn a_lane_is_placed_among_its_street_s_lanes() {
        assert_eq!(lane_note(1, 0), Some("jn-insp-lane-only"));
        assert_eq!(lane_note(2, 0), Some("jn-insp-lane-middle"));
        assert_eq!(lane_note(2, 1), Some("jn-insp-lane-curb"));
        assert_eq!(lane_note(3, 1), None);
    }

    #[test]
    fn a_crossing_in_stages_is_described_by_its_stages() {
        let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
        let j = Junction::new(0);
        assert_eq!(crossing_range(&crossing(&j, 0), &en, Units::Metres), "2 stages of 9.0 m");
        assert_eq!(crossing_range(&crossing(&j, 0), &en, Units::Feet), "2 stages of 29.5 ft");
        assert_eq!(crossing_range(&crossing(&j, 0), &fr, Units::Metres), "2 traversées de 9,0\u{a0}m");
    }

    #[test]
    fn a_crossing_in_one_go_is_described_by_its_length() {
        let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
        let j = Junction::new(0);
        assert_eq!(crossing_range(&crossing(&j, 90), &en, Units::Metres), "11.4 m to cross");
        assert_eq!(crossing_range(&crossing(&j, 90), &fr, Units::Metres), "11,4\u{a0}m à traverser");
    }

    #[test]
    fn a_crossing_that_is_too_far_says_so() {
        let mut c = crossing(&Junction::new(0), 90);
        c.too_far = true;
        assert_eq!(crossing_range(&c, &crate::i18n_for(Locale::En), Units::Metres), "11.4 m to cross. Too far in one go.");
    }

    #[test]
    fn a_range_is_written_in_the_units_shown() {
        let (en, fr) = (crate::i18n_for(Locale::En), crate::i18n_for(Locale::FrCa));
        assert_eq!(range_text(&en, Units::Metres, 1_000, 15_000), "1.0 m to 15.0 m");
        assert_eq!(range_text(&en, Units::Feet, 3_048, 6_096), "10.0 ft to 20.0 ft");
        assert_eq!(range_text(&fr, Units::Metres, 1_000, 15_000), "De 1,0\u{a0}m à 15,0\u{a0}m");
    }

    #[test]
    fn each_class_of_turn_has_its_own_words() {
        let en = crate::i18n_for(Locale::En);
        let words = |key: &str| en.tr_now(key, &Args::new().num("n", 1).str("street", "X"));
        assert_eq!([LEFT, THROUGH, RIGHT].map(|c| words(dest_key(c))), ["Lane 1 to X, left", "Lane 1 to X, straight on", "Lane 1 to X, right"]);
        assert_eq!([LEFT, THROUGH, RIGHT].map(|c| words(goes_key(c))), ["Left to X", "Straight on to X", "Right to X"]);
    }
}
