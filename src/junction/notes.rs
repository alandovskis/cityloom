//! The notes beside the plan, drawn from the junction's view: how far it is to
//! cross each street, where paths meet, whether it works, and what has changed.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::read_model::{CrossingView, JView};
use crate::junction::turns::compass;
use crate::junction::vm::JunctionVm;
use crate::junction::watch::Watch;
use crate::shared::atlas::{MEASURES, Where};
use crate::shared::units::Units;
use crate::street::model::Check;

/// How far it is to cross: the length, or the stages and their length.
fn crossing_text(c: &CrossingView, units: Units) -> String {
    if c.stages > 1 { format!("{} × {}", c.stages, units.number(c.stage_mm)) } else { units.number(c.distance_mm) }
}

/// What a check says, with the length it speaks of in the units shown.
fn check_detail(c: &Check, units: Units) -> String {
    if c.id == "crossing" && c.amount_mm != 0 { format!("{}: {}", c.detail, units.length(c.amount_mm)) } else { c.detail.clone() }
}

/// Why the conflict counts are what they are, where that is not obvious.
fn conflict_note(v: &JView) -> &'static str {
    if v.conflicts.by_phase {
        "A signal takes turns, so paths that cross do not meet at the same time."
    } else if v.ring.is_some() {
        "Traffic in a roundabout only merges and splits; it never crosses."
    } else {
        ""
    }
}

/// The compass points of the arms a measure is in use on, in arm order.
fn used_on(v: &JView, code: &str) -> Vec<&'static str> {
    v.arms.iter().filter(|a| a.transit.codes.contains(&code)).map(|a| compass(a.bearing)).collect()
}

/// The Atlas's groups of measures, in the order they are listed.
fn atlas_groups() -> Vec<&'static str> {
    let mut groups: Vec<&'static str> = Vec::new();
    for m in &MEASURES {
        if !groups.contains(&m.group) {
            groups.push(m.group);
        }
    }
    groups
}

fn icon(ok: bool) -> impl IntoView {
    let d = if ok { "M2.5 8.5 6.5 12.5 13.5 3.5" } else { "M3 3l10 10M13 3 3 13" };
    view! { <svg viewBox="0 0 16 16" aria-hidden="true"><path d=d/></svg> }
}

#[component]
pub fn Across(vm: Rc<JunctionVm>) -> impl IntoView {
    let watch = Watch::new(vm);
    move || {
        let (v, units) = watch.now();
        let rows = v
            .arms
            .iter()
            .map(|a| {
                let across = match &a.crossing {
                    Some(c) => view! { <td class=if c.too_far { "bad" } else { "" }>{crossing_text(c, units)}</td> }.into_any(),
                    None => view! { <td class="zero">"none"</td> }.into_any(),
                };
                let lanes = if a.enters { a.lanes.len() } else { 0 };
                view! {
                    <tr>
                        <th scope="row"><span class="dirtag">{compass(a.bearing)}</span>{a.street}</th>
                        {across}
                        <td>{lanes}</td>
                    </tr>
                }
            })
            .collect_view();
        view! {
            <caption class="sr-only">"How far it is to cross each street, and how many lanes come in"</caption>
            <thead>
                <tr>
                    <th scope="col">"Street"</th>
                    <th scope="col">"To cross"</th>
                    <th scope="col">"Lanes in"</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        }
        .into_any()
    }
}

#[component]
pub fn Conflicts(vm: Rc<JunctionVm>) -> impl IntoView {
    let watch = Watch::new(vm);
    move || {
        let (v, _) = watch.now();
        let c = &v.conflicts;
        view! {
            <caption class="sr-only">"Points where the paths of allowed turns meet"</caption>
            <thead>
                <tr>
                    <th scope="col">"Kind"</th>
                    <th scope="col">"Points"</th>
                </tr>
            </thead>
            <tbody>
                <tr><th scope="row">"Crossing"</th><td>{c.crossing}</td></tr>
                <tr><th scope="row">"Merging"</th><td>{c.merging}</td></tr>
                <tr><th scope="row">"Splitting"</th><td>{c.diverging}</td></tr>
                <tr class="total"><th scope="row">"All"</th><td>{c.crossing + c.merging + c.diverging}</td></tr>
            </tbody>
        }
        .into_any()
    }
}

#[component]
pub fn ConflictNote(vm: Rc<JunctionVm>) -> impl IntoView {
    let watch = Watch::new(vm);
    move || conflict_note(&watch.now().0)
}

#[component]
pub fn Checks(vm: Rc<JunctionVm>) -> impl IntoView {
    let watch = Watch::new(vm);
    move || {
        let (v, units) = watch.now();
        v.checks
            .iter()
            .map(|k| {
                view! {
                    <li class=if k.ok { "ok" } else { "bad" }>
                        {icon(k.ok)}
                        <div>
                            <b>{k.label}<span class="sr-only">{if k.ok { ": passes" } else { ": fails" }}</span></b>
                            <span>{check_detail(k, units)}</span>
                        </div>
                    </li>
                }
            })
            .collect_view()
    }
}

/// Every measure of the Transit Priority Atlas toolbox, with where it is
/// modelled and where this junction uses it.
#[component]
pub fn Measures(vm: Rc<JunctionVm>) -> impl IntoView {
    let watch = Watch::new(vm);
    move || {
        let (v, _) = watch.now();
        atlas_groups()
            .into_iter()
            .map(|group| {
                let rows = MEASURES
                    .iter()
                    .filter(|m| m.group == group)
                    .map(|m| {
                        let on = used_on(&v, m.code);
                        let state = if !on.is_empty() {
                            view! { <b class="m-on">{format!("In use on {}", on.join(", "))}</b> }.into_any()
                        } else {
                            match m.place {
                                Where::Junction => "Set on a street".into_any(),
                                Where::Street => "Street editor".into_any(),
                                Where::Not => view! { <span class="m-not">"Not modelled"</span> }.into_any(),
                            }
                        };
                        let note = (!m.note.is_empty()).then(|| view! { <small>{m.note}</small> });
                        view! {
                            <tr>
                                <th scope="row"><span class="dirtag">{m.code}</span>{m.name}{note}</th>
                                <td>{state}</td>
                            </tr>
                        }
                    })
                    .collect_view();
                view! {
                    <tbody>
                        <tr class="m-group"><th colspan="2" scope="colgroup">{group}</th></tr>
                        {rows}
                    </tbody>
                }
            })
            .collect_view()
    }
}

#[component]
pub fn Revisions(vm: Rc<JunctionVm>) -> impl IntoView {
    let watch = Watch::new(vm);
    // The newest change is the one to see.
    #[cfg(target_arch = "wasm32")]
    {
        let version = watch.version();
        Effect::new(move |_| {
            version.track();
            request_animation_frame(|| {
                if let Some(el) = document().get_element_by_id("revs") {
                    el.set_scroll_top(el.scroll_height());
                }
            });
        });
    }
    move || {
        let (v, _) = watch.now();
        let last = v.revisions.len().checked_sub(1);
        let rows = v
            .revisions
            .iter()
            .enumerate()
            .map(|(i, r)| view! { <tr class=if Some(i) == last { "now" } else { "" }><td>{r.step}</td><td>{r.label.clone()}</td></tr> })
            .collect_view();
        view! {
            <thead>
                <tr>
                    <th scope="col">"Step"</th>
                    <th scope="col">"What changed"</th>
                </tr>
            </thead>
            <tbody>
                <tr class=if v.revisions.is_empty() { "base now" } else { "base" }>
                    <td>"—"</td>
                    <td>"Junction today"</td>
                </tr>
                {rows}
            </tbody>
        }
        .into_any()
    }
}

#[cfg(test)]
mod tests {
    use crate::junction::model::*;
    use crate::shared::units::Units;
    use crate::street::model::Check;

    use super::*;

    fn arm_view(j: &Junction, bearing: i32) -> crate::junction::read_model::ArmView {
        j.view().arms.into_iter().find(|a| a.bearing == bearing).unwrap()
    }

    #[test]
    fn a_crossing_in_stages_says_how_many_and_how_long() {
        let j = Junction::new(0);
        let north = arm_view(&j, 0); // the avenue
        let c = north.crossing.as_ref().unwrap();
        assert_eq!(c.stages, 2);
        assert_eq!(crossing_text(c, Units::Metres), "2 × 9.0");
        assert_eq!(crossing_text(c, Units::Feet), "2 × 29.5");
    }

    #[test]
    fn a_crossing_in_one_go_says_how_far() {
        let j = Junction::new(0);
        let east = arm_view(&j, 90);
        let c = east.crossing.as_ref().unwrap();
        assert_eq!(c.stages, 1);
        assert_eq!(crossing_text(c, Units::Metres), Units::Metres.number(c.distance_mm));
    }

    fn check(id: &'static str, amount_mm: i32) -> Check {
        Check { id, ok: false, amount_mm, label: "A check", detail: "Too much".into() }
    }

    #[test]
    fn a_crossing_check_names_its_length_in_the_units_shown() {
        assert_eq!(check_detail(&check("crossing", 9_000), Units::Metres), "Too much: 9.0 m");
        assert_eq!(check_detail(&check("crossing", 9_000), Units::Feet), "Too much: 29.5 ft");
    }

    #[test]
    fn other_checks_and_a_crossing_check_without_a_length_say_only_their_detail() {
        assert_eq!(check_detail(&check("turning-speed", 9_000), Units::Metres), "Too much");
        assert_eq!(check_detail(&check("crossing", 0), Units::Metres), "Too much");
    }

    #[test]
    fn the_note_on_conflicts_depends_on_how_the_junction_is_run() {
        let mut j = Junction::new(0); // a signal
        assert_eq!(conflict_note(&j.view()), "A signal takes turns, so paths that cross do not meet at the same time.");
        assert!(j.set_control(ALL_WAY_STOP));
        assert_eq!(conflict_note(&j.view()), "");
        assert!(j.set_control(ROUNDABOUT));
        assert_eq!(conflict_note(&j.view()), "Traffic in a roundabout only merges and splits; it never crosses.");
    }

    #[test]
    fn a_measure_is_in_use_on_the_arms_that_have_it_in_arm_order() {
        let mut j = Junction::new(0);
        let (n, e) = (arm_view(&j, 0).uid, arm_view(&j, 90).uid);
        assert!(used_on(&j.view(), "N1").is_empty());
        j.set_bus_lane(e, true);
        j.set_filter(e, true);
        j.set_approach(e, Q_CURB);
        j.set_filter(n, true);
        j.set_bus_lane(n, true);
        let v = j.view();
        assert_eq!(used_on(&v, "N1"), vec!["N", "E"]);
        assert_eq!(used_on(&v, "G2"), vec!["E"]);
        assert!(used_on(&v, "L1").is_empty());
    }

    #[test]
    fn the_atlas_groups_come_in_the_order_they_are_listed() {
        assert_eq!(atlas_groups(), vec!["Linear continuous measures", "Localized measures", "Area-wide measures"]);
    }
}
