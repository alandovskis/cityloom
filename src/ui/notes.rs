//! The notes beside the plan, drawn from the junction's view: how far it is to
//! cross each street, where paths meet, whether it works, and what has changed.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction_view::{CrossingView, JView};
use crate::model::Check;
use crate::ui::shared::Shared;
use crate::ui::turns::compass;
use crate::units::Units;

/// How far it is to cross: the length, or the stages and their length.
fn crossing_text(c: &CrossingView, units: Units) -> String {
    if c.stages > 1 {
        format!("{} × {}", c.stages, units.number(c.stage_mm))
    } else {
        units.number(c.distance_mm)
    }
}

/// What a check says, with the length it speaks of in the units shown.
fn check_detail(c: &Check, units: Units) -> String {
    if c.id == "crossing" && c.amount_mm != 0 {
        format!("{}: {}", c.detail, units.length(c.amount_mm))
    } else {
        c.detail.clone()
    }
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

/// What a component needs to draw from the shared junction: the signals it
/// watches, and the junction itself, kept where only this thread can reach it.
struct Watch {
    version: RwSignal<u32>,
    units: RwSignal<Units>,
    shared: StoredValue<Rc<Shared>, LocalStorage>,
}

impl Watch {
    fn new(shared: Rc<Shared>) -> Watch {
        Watch { version: shared.version(), units: shared.units(), shared: StoredValue::new_local(shared) }
    }

    /// The view and the units, drawing again when either changes.
    fn now(&self) -> (Rc<JView>, Units) {
        self.version.track();
        (self.shared.with_value(|s| s.view()), self.units.get())
    }
}

fn icon(ok: bool) -> impl IntoView {
    let d = if ok { "M2.5 8.5 6.5 12.5 13.5 3.5" } else { "M3 3l10 10M13 3 3 13" };
    view! { <svg viewBox="0 0 16 16" aria-hidden="true"><path d=d/></svg> }
}

#[component]
pub fn Across(shared: Rc<Shared>) -> impl IntoView {
    let watch = Watch::new(shared);
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
pub fn Conflicts(shared: Rc<Shared>) -> impl IntoView {
    let watch = Watch::new(shared);
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
pub fn ConflictNote(shared: Rc<Shared>) -> impl IntoView {
    let watch = Watch::new(shared);
    move || conflict_note(&watch.now().0)
}

#[component]
pub fn Checks(shared: Rc<Shared>) -> impl IntoView {
    let watch = Watch::new(shared);
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

#[component]
pub fn Revisions(shared: Rc<Shared>) -> impl IntoView {
    let watch = Watch::new(shared);
    // The newest change is the one to see.
    let version = watch.version;
    Effect::new(move |_| {
        version.track();
        request_animation_frame(|| {
            if let Some(el) = document().get_element_by_id("revs") {
                el.set_scroll_top(el.scroll_height());
            }
        });
    });
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
    use crate::junction::*;
    use crate::model::Check;
    use crate::units::Units;

    use super::*;

    fn arm_view(j: &Junction, bearing: i32) -> crate::junction_view::ArmView {
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
}
