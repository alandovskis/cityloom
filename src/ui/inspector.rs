//! The panel beside the plan: what is selected, and the controls to change it.
//! Every control reads the model's view and edits through the model; a control
//! that stays on the panel keeps its element, and so its focus, across edits.

use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use crate::catalogue::SAMPLES;
use crate::junction::*;
use crate::junction_view::{CrossingView, JView};
use crate::ui::shared::Shared;
use crate::ui::turns::{compass, turn_glyph, turn_name, turn_word};
use crate::ui::watch::Watch;
use crate::units::Units;

// ---- text the panel is made of ---------------------------------------------

/// Where a lane sits among its street's lanes, when that is worth saying.
fn lane_note(lanes: usize, i: usize) -> &'static str {
    if lanes == 1 {
        "the only lane"
    } else if i == 0 {
        "nearest the middle"
    } else if i + 1 == lanes {
        "nearest the curb"
    } else {
        ""
    }
}

/// What the Crossing section says about the crossing as it stands.
fn crossing_range(c: &CrossingView, units: Units) -> String {
    let how = if c.stages > 1 { format!("{} stages of {}", c.stages, units.length(c.stage_mm)) } else { format!("{} to cross", units.length(c.distance_mm)) };
    if c.too_far { format!("{how}. Too far in one go.") } else { how }
}

/// How much a stepper moves a typed length, as the field's `step` says it.
fn step_attr(units: Units) -> &'static str {
    match units {
        Units::Metres => "0.1",
        Units::Feet => "0.5",
    }
}

fn range_text(units: Units, lo: i32, hi: i32) -> String {
    format!("{} to {}", units.length(lo), units.length(hi))
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
) -> impl IntoView {
    view! {
        <li>
            <button
                type="button"
                class="opt"
                role="checkbox"
                aria-checked=move || checked().to_string()
                tabindex="0"
                disabled=disabled
                on:click=move |_| on_click()
            >
                {glyph}<span>{name}</span>{tick()}
            </button>
        </li>
    }
}

/// A section with a heading, a note under it, and its contents.
fn section(id: &'static str, heading: &'static str, body: impl IntoView) -> impl IntoView {
    view! { <section class="insp-sec"><h3 class="note-h" id=id>{heading}</h3>{body}</section> }
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
    label: &'static str,
    minus: &'static str,
    plus: &'static str,
    min: Option<&'static str>,
    max: Option<&'static str>,
    value: Signal<String>,
    step: Signal<String>,
    tag: Signal<String>,
    hint: Signal<String>,
}

fn num_field(n: Num, on_step: impl Fn(i32) + Clone + 'static, on_set: impl Fn(f64) + 'static) -> impl IntoView {
    let (down, up) = (on_step.clone(), on_step);
    let value = n.value;
    let step = n.step;
    let tag = n.tag;
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id=n.id>{n.label}</h3>
            <div class="stepper">
                <button type="button" class="ico" aria-label=n.minus on:click=move |_| down(-1)>
                    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8"/></svg>
                </button>
                <span class="wfield">
                    <input
                        type="number"
                        inputmode="decimal"
                        step=move || step.get()
                        min=n.min
                        max=n.max
                        value=move || value.get()
                        prop:value=move || value.get()
                        aria-labelledby=n.id
                        on:change=move |ev| {
                            let input = event_target::<HtmlInputElement>(&ev);
                            match input.value().parse::<f64>() {
                                Ok(v) if v.is_finite() => on_set(v),
                                _ => input.set_value(&value.get_untracked()),
                            }
                        }
                    />
                    <span class="unit-tag" aria-hidden="true">{move || tag.get()}</span>
                </span>
                <button type="button" class="ico" aria-label=n.plus on:click=move |_| up(1)>
                    <svg viewBox="0 0 14 14" aria-hidden="true"><path d="M3 7h8M7 3v8"/></svg>
                </button>
            </div>
            {hint(n.hint)}
        </section>
    }
}

fn derived(w: Watch, f: impl Fn(&JView, Units) -> String + Send + Sync + 'static) -> Signal<String> {
    Signal::derive(move || f(&w.view(), w.units()))
}

fn constant(text: &'static str) -> Signal<String> {
    Signal::derive(move || text.to_string())
}

/// A length in the field's units, to `places`, of whatever `mm` reads.
fn length_signal(w: Watch, places: usize, mm: impl Fn(&JView) -> i32 + Send + Sync + 'static) -> Signal<String> {
    derived(w, move |v, u| u.fixed(mm(v), places))
}

fn step_signal(w: Watch) -> Signal<String> {
    Signal::derive(move || step_attr(w.units()).to_string())
}

fn unit_tag(w: Watch) -> Signal<String> {
    Signal::derive(move || w.units().word().to_string())
}

// ---- the junction's own controls ------------------------------------------------

fn ring_size(w: Watch) -> impl IntoView {
    let spec = Num {
        id: "i-h-ring",
        label: "Size of the roundabout",
        minus: "Smaller by 0.5 m",
        plus: "Larger by 0.5 m",
        min: None,
        max: None,
        value: length_signal(w, 1, |v| v.ring.as_ref().map_or(0, |r| r.radius_mm * 2)),
        step: step_signal(w),
        tag: Signal::derive(move || format!("{} across", w.units().word())),
        hint: derived(w, |v, u| {
            format!("Across the outside. No smaller than {} with these streets.", u.length(v.ring.as_ref().map_or(0, |r| r.floor_mm * 2)))
        }),
    };
    num_field(
        spec,
        move |dir| {
            w.edit(|j| j.step_ring(dir));
        },
        move |v| {
            w.edit(|j| j.set_ring_radius(w.units_now().mm(v / 2.0)));
        },
    )
}

fn cycle_width(w: Watch, tail: &'static str) -> impl IntoView {
    let spec = Num {
        id: "i-h-cycle-width",
        label: "Track width",
        minus: "Narrower by 0.5 m",
        plus: "Wider by 0.5 m",
        min: None,
        max: None,
        value: length_signal(w, 1, |v| v.ring.as_ref().and_then(|r| r.cycle_mm).unwrap_or(0)),
        step: step_signal(w),
        tag: unit_tag(w),
        hint: derived(w, move |_, u| format!("{}. {tail}", range_text(u, CYCLE_MIN_MM, CYCLE_MAX_MM))),
    };
    num_field(
        spec,
        move |dir| {
            w.edit(|j| j.step_cycle(dir));
        },
        move |v| {
            w.edit(|j| j.set_cycle(Some(w.units_now().mm(v))));
        },
    )
}

fn bus_choice(w: Watch) -> impl IntoView {
    let options = move || {
        let v = w.view();
        let cur = v.bus.as_ref().map(|b| (b.from.min(b.to), b.from.max(b.to)));
        v.bus_options
            .iter()
            .map(|o| {
                let (a, b) = (o.a, o.b);
                view! { <option value=format!("{a}-{b}") prop:selected=cur == Some((a, b))>{o.label.clone()}</option> }
            })
            .collect_view()
    };
    let none_selected = move || w.view().bus.is_none();
    let note = Signal::derive(move || match w.view().bus.as_ref() {
        Some(b) => format!(
            "A {} bus-only lane straight across the island. It crosses the ring where it enters and leaves.",
            w.units().length(b.width_mm)
        ),
        None => "Lets buses cut across the island between two streets.".to_string(),
    });
    section(
        "i-h-bus",
        "Bus lane through the middle",
        view! {
            <select
                aria-labelledby="i-h-bus"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    let pair = value.split_once('-').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)));
                    w.edit(|j| j.set_bus(pair));
                }
            >
                <option value="" prop:selected=none_selected>"No bus lane"</option>
                {options}
            </select>
            {hint(note)}
        },
    )
}

fn cycle_track(w: Watch, cycle: Memo<bool>) -> impl IntoView {
    section(
        "i-h-cycle",
        "Cycle track",
        view! {
            <ul class="opts">
                {option_btn(
                    move || cycle.get(),
                    || false,
                    ().into_any(),
                    || "Track around the outside".to_string(),
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
        "i-h-control",
        "Junction control",
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
                    .map(|(i, c)| view! { <option value=i.to_string() prop:selected=move || w.view().control_index == i>{c.name}</option> })
                    .collect_view()}
            </select>
        },
    );
    view! {
        {control}
        {move || ring.get().then(|| ring_size(w))}
        {move || ring.get().then(|| cycle_track(w, cycle))}
        {move || (ring.get() && cycle.get()).then(|| cycle_width(w, "It takes space from the carriageway inside the same circle."))}
        {move || ring.get().then(|| bus_choice(w))}
    }
}

// ---- what is selected ----------------------------------------------------------

fn head(name: impl Fn() -> String + Send + Sync + 'static, sub: impl Fn() -> String + Send + Sync + 'static) -> impl IntoView {
    view! { <div class="insp-head"><div><h2 class="insp-name">{name}</h2><p class="insp-sub">{sub}</p></div></div> }
}

fn danger_button(label: &'static str, on_click: impl Fn() + 'static) -> impl IntoView {
    view! { <section class="insp-sec"><button type="button" class="btn danger" on:click=move |_| on_click()>{btn_icon(false)}{label}</button></section> }
}

fn bus_panel(w: Watch) -> AnyView {
    let label = move || {
        let v = w.view();
        let Some(b) = v.bus.as_ref() else { return String::new() };
        let pair = (b.from.min(b.to), b.from.max(b.to));
        v.bus_options.iter().find(|o| (o.a, o.b) == pair).map(|o| o.label.clone()).unwrap_or_default()
    };
    view! {
        {head(|| "Bus lane".to_string(), label)}
        <section class="insp-sec">
            {hint(Signal::derive(move || match w.view().bus.as_ref() {
                Some(b) => format!(
                    "A {} bus-only lane straight across the island. It crosses the ring where it enters and leaves.",
                    w.units().length(b.width_mm)
                ),
                None => String::new(),
            }))}
        </section>
        {control_section(w)}
        {danger_button("Remove the bus lane", move || {
            w.edit(|j| j.set_bus(None));
        })}
    }
    .into_any()
}

fn cycle_panel(w: Watch) -> AnyView {
    view! {
        {head(|| "Cycle track".to_string(), || "Round the outside of the roundabout".to_string())}
        {cycle_width(w, "Cyclists cross every street where it meets the ring.")}
        {control_section(w)}
        {danger_button("Remove the cycle track", move || {
            w.edit(|j| j.set_cycle_track(false));
        })}
    }
    .into_any()
}

fn junction_panel(w: Watch) -> AnyView {
    view! {
        <p class="insp-empty">"Select a street, corner or crossing to change it."</p>
        {control_section(w)}
    }
    .into_any()
}

fn corner_panel(w: Watch, uid: u32) -> AnyView {
    let sub = move || {
        let v = w.view();
        let from = v.arms.iter().find(|a| a.uid == uid).map_or("", |a| compass(a.bearing));
        let to = v.corners.iter().find(|c| c.uid == uid).and_then(|c| v.arms.iter().find(|a| a.uid == c.next_uid)).map_or("", |a| compass(a.bearing));
        format!("{from} to {to}")
    };
    let spec = Num {
        id: "i-h-corner",
        label: "Curb radius",
        minus: "Tighter by 0.5 m",
        plus: "Wider by 0.5 m",
        min: None,
        max: None,
        value: length_signal(w, 1, move |v| v.corners.iter().find(|c| c.uid == uid).map_or(0, |c| c.radius_mm)),
        step: step_signal(w),
        tag: unit_tag(w),
        hint: derived(w, move |v, u| {
            let speed = v.corners.iter().find(|c| c.uid == uid).map_or(0.0, |c| c.speed_kmh);
            format!("{}. Cars turn here at about {} km/h.", range_text(u, MIN_CORNER_MM, MAX_CORNER_MM), speed.round())
        }),
    };
    view! {
        {head(|| "Corner".to_string(), sub)}
        {num_field(
            spec,
            move |dir| {
                w.edit(|j| j.step_corner(uid, dir));
            },
            move |v| {
                w.edit(|j| j.set_corner(uid, w.units_now().mm(v)));
            },
        )}
        <p class="insp-empty">"A tight corner slows turning cars and shortens the walk across. A wide one lets them swing through faster."</p>
        {control_section(w)}
    }
    .into_any()
}

/// One street a lane may go to, as a turn button on its row.
fn dest_button(w: Watch, uid: u32, lane: usize, to: u32) -> impl IntoView {
    let v = w.view_now();
    let a = v.arms.iter().find(|a| a.uid == uid);
    let d = a.and_then(|a| a.lanes.get(lane)).and_then(|l| l.dests.iter().find(|d| d.uid == to));
    let (class, label) = d.map_or((0, String::new()), |d| (d.class, d.label.clone()));
    let bearing = v.arms.iter().find(|x| x.uid == to).map_or(0, |x| x.bearing);
    let state = move || w.arm(uid, |a| a.lanes.get(lane).and_then(|l| l.dests.iter().find(|d| d.uid == to).map(|d| (d.on, d.open, l.bad)))).flatten().unwrap_or((false, false, false));
    let name = format!("Lane {} to {}, {}", lane + 1, label, turn_word(class));
    view! {
        <button
            type="button"
            class=move || {
                let (on, _, bad) = state();
                format!("turn dest{}{}", if on { " on" } else { "" }, if on && bad { " bad" } else { "" })
            }
            aria-pressed=move || state().0.to_string()
            disabled=move || !state().1
            title=label.clone()
            aria-label=name
            on:click=move |_| {
                let on = state().0;
                w.edit(|j| j.set_lane_dest(uid, lane, to, !on));
            }
        >
            {turn_glyph(class, 16)}
            <span aria-hidden="true">{compass(bearing)}</span>
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
    let note = move || w.arm(uid, |a| lane_note(a.lanes.len(), i)).unwrap_or("");
    view! {
        <li class=move || if bad() { "lane-row bad" } else { "lane-row" }>
            <button type="button" class="lane-pick" on:click=move |_| w.select(Target::Lane(uid, i))>
                <span>{format!("Lane {}", i + 1)}<small>{note}</small></span>
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
        w.arm(uid, |a| {
            let note = lane_note(a.lanes.len(), lane);
            if note.is_empty() { a.label.clone() } else { format!("{} · {note}", a.label) }
        })
        .unwrap_or_default()
    };
    let title = move || w.arm(uid, |a| format!("Lane {} of {}", lane + 1, a.lanes.len())).unwrap_or_default();
    let range = Signal::derive(move || {
        w.arm(uid, |a| match a.lanes.get(lane) {
            Some(l) if l.bad => "Every street it goes to is banned. Add a street, or allow a turn.".to_string(),
            Some(l) => format!("{} wide. A lane has to go to at least one street.", w.units().length(l.width_mm)),
            None => String::new(),
        })
        .unwrap_or_default()
    });
    let option = move |to: u32| {
        let v = w.view_now();
        let d = v.arms.iter().find(|a| a.uid == uid).and_then(|a| a.lanes.get(lane)).and_then(|l| l.dests.iter().find(|d| d.uid == to));
        let (class, label) = d.map_or((0, String::new()), |d| (d.class, d.label.clone()));
        let state = move || w.arm(uid, |a| a.lanes.get(lane).and_then(|l| l.dests.iter().find(|d| d.uid == to).map(|d| (d.on, d.open)))).flatten().unwrap_or((false, false));
        option_btn(
            move || state().0,
            move || !state().1,
            turn_glyph(class, 22).into_any(),
            move || format!("{} to {}{}", turn_name(class), label, if state().1 { "" } else { " (one way in)" }),
            move || {
                let on = state().0;
                w.edit(|j| j.set_lane_dest(uid, lane, to, !on));
            },
        )
    };
    view! {
        {head(title, sub)}
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-serves">"Where this lane goes"</h3>
            <ul class="opts"><For each=move || dests.get() key=|d| *d children=option/></ul>
            {hint(range)}
        </section>
        <section class="insp-sec">
            <button type="button" class="btn" on:click=move |_| w.select(Target::Arm(uid))>"Select the whole street"</button>
        </section>
        {control_section(w)}
    }
    .into_any()
}

// ---- a street ------------------------------------------------------------------

fn crossing_fields(w: Watch, uid: u32) -> impl IntoView {
    let range = Signal::derive(move || w.arm(uid, |a| a.crossing.as_ref().map(|c| crossing_range(c, w.units()))).flatten().unwrap_or_default());
    let island = move || w.arm(uid, |a| a.crossing.as_ref().is_some_and(|c| c.island)).unwrap_or(false);
    let can_island = move || w.arm(uid, |a| a.can_island).unwrap_or(false);
    let setback = Num {
        id: "i-h-setback",
        label: "Set back from the junction",
        minus: "Closer by 0.5 m",
        plus: "Farther by 0.5 m",
        min: None,
        max: None,
        value: length_signal(w, 1, move |v| v.arms.iter().find(|a| a.uid == uid).and_then(|a| a.crossing.as_ref()).map_or(0, |c| c.setback_mm)),
        step: step_signal(w),
        tag: unit_tag(w),
        hint: derived(w, |_, u| range_text(u, MIN_SETBACK_MM, MAX_SETBACK_MM)),
    };
    let width = Num {
        id: "i-h-cwidth",
        label: "Crossing width",
        minus: "Narrower by 0.5 m",
        plus: "Wider by 0.5 m",
        min: None,
        max: None,
        value: length_signal(w, 1, move |v| v.arms.iter().find(|a| a.uid == uid).and_then(|a| a.crossing.as_ref()).map_or(0, |c| c.width_mm)),
        step: step_signal(w),
        tag: unit_tag(w),
        hint: derived(w, |_, u| range_text(u, MIN_CROSSING_MM, MAX_CROSSING_MM)),
    };
    let bulb = move |side: usize| {
        let has = move || w.arm(uid, |a| a.can_bulb[side]).unwrap_or(false);
        option_btn(
            move || w.arm(uid, |a| a.bulbs[side].is_some()).unwrap_or(false),
            move || !has(),
            ().into_any(),
            move || format!("{} curb bulge{}", if side == 0 { "Left" } else { "Right" }, if has() { "" } else { " (no parking there)" }),
            move || {
                let on = w.arm(uid, |a| a.bulbs[side].is_some()).unwrap_or(false);
                w.edit(|j| j.set_bulb(uid, side, !on));
            },
        )
    };
    view! {
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-cross">"Crossing"</h3>
            {hint(range)}
            <button type="button" class="btn" on:click=move |_| { w.edit(|j| j.set_crossing(uid, false)); }>{btn_icon(false)}"Remove the crossing"</button>
        </section>
        {num_field(
            setback,
            move |dir| {
                w.edit(|j| j.step_setback(uid, dir));
            },
            move |v| {
                w.edit(|j| j.set_setback(uid, w.units_now().mm(v)));
            },
        )}
        {num_field(
            width,
            move |dir| {
                w.edit(|j| j.step_crossing_width(uid, dir));
            },
            move |v| {
                w.edit(|j| j.set_crossing_width(uid, w.units_now().mm(v)));
            },
        )}
        {section(
            "i-h-refuge",
            "Halfway island",
            view! {
                <ul class="opts">
                    {option_btn(
                        island,
                        move || !can_island(),
                        ().into_any(),
                        move || if can_island() { "Refuge island in the middle".to_string() } else { "Refuge island (road too narrow)".to_string() },
                        move || { w.edit(|j| j.set_island(uid, !island())); },
                    )}
                </ul>
            },
        )}
        {section("i-h-bulb", "Shorten the crossing", view! { <ul class="opts">{bulb(0)}{bulb(1)}</ul> })}
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
                    <h3 class="note-h" id="i-h-cross">"Crossing"</h3>
                    <button type="button" class="btn" on:click=move |_| { w.edit(|j| j.set_crossing(uid, true)); }>{btn_icon(true)}"Mark a crossing"</button>
                </section>
            }
            .into_any()
        }
    }
}

/// A list of items to choose one from, by position in a catalogue.
fn choice(
    w: Watch,
    key: &'static str,
    label: &'static str,
    items: &'static [Item],
    current: impl Fn(&crate::junction_view::TransitView) -> usize + Send + Sync + Copy + 'static,
    set: impl Fn(&mut Junction, usize) -> bool + Copy + 'static,
    uid: u32,
) -> impl IntoView {
    let id = format!("i-h-{key}");
    let labelled = id.clone();
    view! {
        <label class="fld">
            <span id=id>{label}</span>
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
                        let text = if o.code.is_empty() { o.name.to_string() } else { format!("{} {}", o.code, o.name) };
                        view! { <option value=i.to_string() prop:selected=move || w.arm(uid, |a| current(&a.transit) == i).unwrap_or(false)>{text}</option> }
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
        (!list.is_empty()).then(|| view! { <ul class="problems">{list.into_iter().map(|p| view! { <li>{p}</li> }).collect_view()}</ul> })
    };
    let length_field = move || {
        let queue = kind.get() == "queue";
        matches!(kind.get(), "queue" | "gate").then(|| {
            let spec = Num {
                id: "i-h-alen",
                label: if queue { "Length of the queue jump" } else { "Gate distance upstream" },
                minus: "Shorter by 5 m",
                plus: "Longer by 5 m",
                min: None,
                max: None,
                value: length_signal(w, 1, move |v| v.arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.transit.approach_mm)),
                step: step_signal(w),
                tag: unit_tag(w),
                hint: derived(w, |_, u| range_text(u, APPROACH_MIN_MM, APPROACH_MAX_MM)),
            };
            num_field(
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
            <h3 class="note-h" id="i-h-transit">"Transit priority"</h3>
            <ul class="opts">
                {option_btn(
                    bus_lane,
                    || false,
                    ().into_any(),
                    || "Bus lane along the way in".to_string(),
                    move || { w.edit(|j| j.set_bus_lane(uid, !bus_lane())); },
                )}
            </ul>
            {choice(w, "approach", "At the approach", &APPROACHES, |t| t.approach, move |j, i| j.set_approach(uid, i), uid)}
            {choice(w, "stop", "Bus stop", &STOPS, |t| t.stop, move |j, i| j.set_stop(uid, i), uid)}
            {choice(w, "rule", "Turns", &RULES, |t| t.rule, move |j, i| j.set_rule(uid, i), uid)}
            <ul class="opts">
                {option_btn(
                    filter,
                    || false,
                    ().into_any(),
                    || format!("{FILTER_CODE} Transit modal filter"),
                    move || { w.edit(|j| j.set_filter(uid, !filter())); },
                )}
            </ul>
            {problems}
        </section>
        {length_field}
    }
}

fn arm_panel(w: Watch, uid: u32, crossing_only: bool) -> AnyView {
    let name = move || w.arm(uid, |a| a.street.to_string()).unwrap_or_default();
    let sub = move || w.arm(uid, |a| format!("{}, {}° · {} road", compass(a.bearing), a.bearing, w.units().length(a.road_mm))).unwrap_or_default();
    if crossing_only {
        return view! { {head(name, sub)} {crossing_section(w, uid)} {control_section(w)} }.into_any();
    }
    let enters = Memo::new(move |_| w.arm(uid, |a| a.enters).unwrap_or(false));
    let lanes = Memo::new(move |_| w.arm(uid, |a| a.lanes.len()).unwrap_or(0));
    let linked = w.view_now().linked;
    let direction = Num {
        id: "i-h-bearing",
        label: "Direction",
        minus: "Turn anticlockwise by 5°",
        plus: "Turn clockwise by 5°",
        min: Some("0"),
        max: Some("355"),
        value: Signal::derive(move || w.arm(uid, |a| a.bearing.to_string()).unwrap_or_default()),
        step: Signal::derive(|| BEARING_STEP.to_string()),
        tag: constant("°"),
        hint: constant("Clockwise from north. At least 30° from its neighbours."),
    };
    let offset = Num {
        id: "i-h-offset",
        label: "Shift sideways",
        minus: "Shift left by 0.1 m",
        plus: "Shift right by 0.1 m",
        min: None,
        max: None,
        value: length_signal(w, 2, move |v| v.arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.offset_mm)),
        step: step_signal(w),
        tag: unit_tag(w),
        hint: derived(w, move |v, u| {
            let max = v.arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.max_offset_mm);
            format!("Up to {} either way. Looking out from the junction.", u.length(max))
        }),
    };
    let street = if linked {
        let edge = w.view_now().arms.iter().find(|a| a.uid == uid).map_or(0, |a| a.edge);
        section(
            "i-h-street",
            "Street",
            view! {
                <a class="btn" href=format!("index.html?street={edge}")>"Open the cross-section"</a>
                <p class="insp-range">"This street belongs to the city. Its layout is edited in the street editor, and changes there show here."</p>
            },
        )
        .into_any()
    } else {
        section(
            "i-h-street",
            "Street",
            view! {
                <select
                    aria-labelledby="i-h-street"
                    on:change=move |ev| {
                        if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                            w.edit(|j| j.set_street(uid, i));
                        }
                    }
                >
                    {SAMPLES
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| !s.freeway)
                        .map(|(i, s)| view! { <option value=i.to_string() prop:selected=move || w.arm(uid, |a| a.street_index == i).unwrap_or(false)>{s.name}</option> })
                        .collect_view()}
                </select>
                <p class="insp-range">"The street's own layout is edited in the street editor."</p>
            },
        )
        .into_any()
    };
    let remove = (!linked).then(|| {
        let can = move || w.view().can_remove;
        view! {
            <section class="insp-sec">
                <button type="button" class="btn danger" disabled=move || !can() on:click=move |_| { w.edit(|j| j.remove_arm(uid)); }>
                    {btn_icon(false)}"Remove this street"
                </button>
                {move || (!can()).then(|| view! { <p class="insp-range">{format!("A junction needs at least {MIN_ARMS} streets.")}</p> })}
            </section>
        }
    });
    view! {
        {head(name, sub)}
        <section class="insp-sec">
            <h3 class="note-h" id="i-h-lanes">"Lanes coming in"</h3>
            {move || if enters.get() {
                view! { <ul class="lanes"><For each=move || 0..lanes.get() key=|i| *i children=move |i| lane_row(w, uid, i)/></ul> }.into_any()
            } else {
                view! { <p class="insp-range">"One way out. No lanes come in."</p> }.into_any()
            }}
        </section>
        {crossing_section(w, uid)}
        {transit_section(w, uid)}
        {num_field(
            direction,
            move |dir| { w.edit(|j| j.step_bearing(uid, dir)); },
            move |v| { w.edit(|j| j.set_bearing(uid, v.round() as i32)); },
        )}
        {num_field(
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
pub fn Inspector(shared: Rc<Shared>) -> impl IntoView {
    let w = Watch::new(shared);
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

    fn crossing(j: &Junction, bearing: i32) -> CrossingView {
        j.view().arms.into_iter().find(|a| a.bearing == bearing).unwrap().crossing.unwrap()
    }

    #[test]
    fn a_lane_is_placed_among_its_street_s_lanes() {
        assert_eq!(lane_note(1, 0), "the only lane");
        assert_eq!(lane_note(2, 0), "nearest the middle");
        assert_eq!(lane_note(2, 1), "nearest the curb");
        assert_eq!(lane_note(3, 1), "");
    }

    #[test]
    fn a_crossing_in_stages_is_described_by_its_stages() {
        let j = Junction::new(0);
        assert_eq!(crossing_range(&crossing(&j, 0), Units::Metres), "2 stages of 9.0 m");
        assert_eq!(crossing_range(&crossing(&j, 0), Units::Feet), "2 stages of 29.5 ft");
    }

    #[test]
    fn a_crossing_in_one_go_is_described_by_its_length() {
        let j = Junction::new(0);
        assert_eq!(crossing_range(&crossing(&j, 90), Units::Metres), "11.4 m to cross");
    }

    #[test]
    fn a_crossing_that_is_too_far_says_so() {
        let mut c = crossing(&Junction::new(0), 90);
        c.too_far = true;
        assert_eq!(crossing_range(&c, Units::Metres), "11.4 m to cross. Too far in one go.");
    }

    #[test]
    fn a_range_is_written_in_the_units_shown() {
        assert_eq!(range_text(Units::Metres, 1_000, 15_000), "1.0 m to 15.0 m");
        assert_eq!(range_text(Units::Feet, 3_048, 6_096), "10.0 ft to 20.0 ft");
        assert_eq!(step_attr(Units::Metres), "0.1");
        assert_eq!(step_attr(Units::Feet), "0.5");
    }
}
