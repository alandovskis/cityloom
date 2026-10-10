//! The notes beside the plan, drawn from the junction's view: how far it is to
//! cross each street, where paths meet, whether it works, and what has changed.

use std::rc::Rc;

use leptos::prelude::*;

use crate::junction::read_model::Check;
use crate::junction::read_model::{CrossingView, JView};
use crate::junction::turns::compass_key;
use crate::junction::vm::JunctionVm;
use crate::junction::watch::Watch;
use crate::shared::atlas::{MEASURES, Where, group_name_key, name_key, note_key};
use crate::shared::i18n::{Args, I18n, Locale};
use crate::shared::said::say;
use crate::shared::units::Units;

/// How far it is to cross: the length, or the stages and their length, written in `locale`.
fn crossing_text(c: &CrossingView, units: Units, locale: Locale) -> String {
    if c.stages > 1 { format!("{} × {}", c.stages, units.number_in(c.stage_mm, locale)) } else { units.number_in(c.distance_mm, locale) }
}

/// What a check says, with the length it speaks of in the units shown; drawn again when the language changes.
fn check_detail(c: &Check, i18n: &I18n, units: Units) -> String {
    let detail = say(i18n, units, &c.detail);
    if c.id == "crossing" && c.amount_mm != 0 {
        i18n.tr("jn-check-length", &Args::new().str("detail", detail).str("length", units.length_in(c.amount_mm, i18n.locale())))
    } else {
        detail
    }
}

/// The message that says why the conflict counts are what they are, where that is not obvious.
fn conflict_note(v: &JView) -> Option<&'static str> {
    if v.conflicts.by_phase {
        Some("jn-conflicts-signal")
    } else if v.ring.is_some() {
        Some("jn-conflicts-roundabout")
    } else {
        None
    }
}

/// The messages of the compass points of the arms a measure is in use on, in arm order.
fn used_on(v: &JView, code: &str) -> Vec<&'static str> {
    v.arms.iter().filter(|a| a.transit.codes.contains(&code)).map(|a| compass_key(a.bearing)).collect()
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
        let locale = watch.i18n().locale();
        let rows = v
            .arms
            .iter()
            .map(|a| {
                let across = match &a.crossing {
                    Some(c) => view! { <td class=if c.too_far { "bad" } else { "" }>{crossing_text(c, units, locale)}</td> }.into_any(),
                    None => view! { <td class="zero">{watch.tr("jn-across-none")}</td> }.into_any(),
                };
                let lanes = if a.enters { a.lanes.len() } else { 0 };
                view! {
                    <tr>
                        <th scope="row"><span class="dirtag">{watch.tr(compass_key(a.bearing))}</span>{watch.say(&a.street)}</th>
                        {across}
                        <td>{lanes}</td>
                    </tr>
                }
            })
            .collect_view();
        view! {
            <caption class="sr-only">{watch.tr("jn-across-caption")}</caption>
            <thead>
                <tr>
                    <th scope="col">{watch.tr("jn-across-street")}</th>
                    <th scope="col">{watch.tr("jn-across-to-cross")}</th>
                    <th scope="col">{watch.tr("jn-across-lanes-in")}</th>
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
            <caption class="sr-only">{watch.tr("jn-conflicts-caption")}</caption>
            <thead>
                <tr>
                    <th scope="col">{watch.tr("jn-conflicts-kind")}</th>
                    <th scope="col">{watch.tr("jn-conflicts-points")}</th>
                </tr>
            </thead>
            <tbody>
                <tr><th scope="row">{watch.tr("jn-conflicts-crossing")}</th><td>{c.crossing}</td></tr>
                <tr><th scope="row">{watch.tr("jn-conflicts-merging")}</th><td>{c.merging}</td></tr>
                <tr><th scope="row">{watch.tr("jn-conflicts-splitting")}</th><td>{c.diverging}</td></tr>
                <tr class="total"><th scope="row">{watch.tr("jn-conflicts-all")}</th><td>{c.crossing + c.merging + c.diverging}</td></tr>
            </tbody>
        }
        .into_any()
    }
}

#[component]
pub fn ConflictNote(vm: Rc<JunctionVm>) -> impl IntoView {
    let watch = Watch::new(vm);
    move || conflict_note(&watch.view()).map(|key| watch.tr(key)).unwrap_or_default()
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
                            <b>{watch.say(&k.label)}<span class="sr-only">{watch.tr(if k.ok { "jn-check-passes" } else { "jn-check-fails" })}</span></b>
                            <span>{check_detail(k, &watch.i18n(), units)}</span>
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
                            let arms: Vec<String> = on.iter().map(|key| watch.tr(key)).collect();
                            let in_use = watch.i18n().tr("jn-measure-in-use", &Args::new().str("arms", arms.join(", ")));
                            view! { <b class="m-on">{in_use}</b> }.into_any()
                        } else {
                            match m.place {
                                Where::Junction => watch.tr("jn-measure-on-street").into_any(),
                                Where::Street => watch.tr("jn-measure-street-editor").into_any(),
                                Where::Not => view! { <span class="m-not">{watch.tr("jn-measure-not-modelled")}</span> }.into_any(),
                            }
                        };
                        let note = (!m.note.is_empty()).then(|| view! { <small>{watch.tr(&note_key(m.code))}</small> });
                        view! {
                            <tr>
                                <th scope="row"><span class="dirtag">{m.code}</span>{watch.tr(&name_key(m.code))}{note}</th>
                                <td>{state}</td>
                            </tr>
                        }
                    })
                    .collect_view();
                view! {
                    <tbody>
                        <tr class="m-group"><th colspan="2" scope="colgroup">{group_name_key(group).map_or_else(|| group.to_string(), |key| watch.tr(key))}</th></tr>
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
            .map(|(i, r)| view! { <tr class=if Some(i) == last { "now" } else { "" }><td>{r.step}</td><td>{watch.say(&r.label)}</td></tr> })
            .collect_view();
        view! {
            <thead>
                <tr>
                    <th scope="col">{watch.tr("jn-col-step")}</th>
                    <th scope="col">{watch.tr("jn-col-what-changed")}</th>
                </tr>
            </thead>
            <tbody>
                <tr class=if v.revisions.is_empty() { "base now" } else { "base" }>
                    <td>"—"</td>
                    <td>{watch.tr("city-junction-today")}</td>
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
    use crate::junction::read_model::Check;
    use crate::shared::said::Said;
    use crate::shared::units::Units;

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
        assert_eq!(crossing_text(c, Units::Metres, Locale::En), "2 × 9.0");
        assert_eq!(crossing_text(c, Units::Feet, Locale::En), "2 × 29.5");
        assert_eq!(crossing_text(c, Units::Metres, Locale::FrCa), "2 × 9,0");
    }

    #[test]
    fn a_crossing_in_one_go_says_how_far() {
        let j = Junction::new(0);
        let east = arm_view(&j, 90);
        let c = east.crossing.as_ref().unwrap();
        assert_eq!(c.stages, 1);
        assert_eq!(crossing_text(c, Units::Metres, Locale::En), Units::Metres.number(c.distance_mm));
    }

    fn check(id: &'static str, amount_mm: i32) -> Check {
        Check { id, ok: false, amount_mm, label: Said::new("jn-check-crossing"), detail: Said::new("jn-check-crossing-ok") }
    }

    #[test]
    fn a_crossing_check_names_its_length_in_the_units_shown() {
        let en = crate::i18n_for(crate::shared::i18n::Locale::En);
        assert_eq!(check_detail(&check("crossing", 9_000), &en, Units::Metres), "Longest crossing in one go: 9.0 m");
        assert_eq!(check_detail(&check("crossing", 9_000), &en, Units::Feet), "Longest crossing in one go: 29.5 ft");
        let fr = crate::i18n_for(Locale::FrCa);
        assert_eq!(check_detail(&check("crossing", 9_000), &fr, Units::Metres), "Plus longue traversée d’un seul coup\u{a0}: 9,0\u{a0}m");
    }

    #[test]
    fn other_checks_and_a_crossing_check_without_a_length_say_only_their_detail() {
        let en = crate::i18n_for(crate::shared::i18n::Locale::En);
        assert_eq!(check_detail(&check("turning-speed", 9_000), &en, Units::Metres), "Longest crossing in one go");
        assert_eq!(check_detail(&check("crossing", 0), &en, Units::Metres), "Longest crossing in one go");
    }

    #[test]
    fn the_note_on_conflicts_depends_on_how_the_junction_is_run() {
        let mut j = Junction::new(0); // a signal
        assert_eq!(conflict_note(&j.view()), Some("jn-conflicts-signal"));
        assert!(j.set_control(ALL_WAY_STOP));
        assert_eq!(conflict_note(&j.view()), None);
        assert!(j.set_control(ROUNDABOUT));
        assert_eq!(conflict_note(&j.view()), Some("jn-conflicts-roundabout"));
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
        assert_eq!(used_on(&v, "N1"), vec!["jn-compass-short-n", "jn-compass-short-e"]);
        assert_eq!(used_on(&v, "G2"), vec!["jn-compass-short-e"]);
        assert!(used_on(&v, "L1").is_empty());
    }

    #[test]
    fn the_atlas_groups_come_in_the_order_they_are_listed() {
        assert_eq!(atlas_groups(), vec!["Linear continuous measures", "Localized measures", "Area-wide measures"]);
        let keys: Vec<&str> = atlas_groups().into_iter().filter_map(group_name_key).collect();
        assert_eq!(keys, vec!["group-name-linear", "group-name-local", "group-name-area"]);
    }
}
