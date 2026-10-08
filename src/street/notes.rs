//! The notes beside the street's section: how its width is shared out, how many
//! people it moves, whether it works, what has changed, and the transit
//! measures it forms or could.

use crate::shared::i18n::Locale;
use crate::shared::units::{Units, group_thousands};
use crate::street::model::{Check, View};

/// Which way a change goes, as the class of the cell that shows it.
pub fn change_class(delta: i32) -> &'static str {
    match delta {
        d if d > 0 => "up",
        d if d < 0 => "down",
        _ => "zero",
    }
}

/// How capacity has changed, as a percentage of what it was: `+12%`, `\u{2212}8%`, `0`.
pub fn capacity_change(existing: i32, now: i32) -> String {
    let d = now - existing;
    // Rounded as `Math.round` does, half toward the larger.
    let pct = if existing != 0 { ((d as f64 * 100.0 / existing as f64) + 0.5).floor() as i32 } else { 0 };
    match pct {
        0 => "0".to_string(),
        p if p > 0 => format!("+{p}%"),
        p => format!("\u{2212}{}%", -p),
    }
}

/// What a check says, with the length it speaks of in the units shown.
pub fn check_detail(c: &Check, v: &View, units: Units) -> String {
    match c.id {
        "fits" if c.ok => {
            if v.delta_mm == 0 {
                "Every metre is used".to_string()
            } else {
                format!("{} left to use", units.length_fine(c.amount_mm))
            }
        }
        "fits" => format!("{} too wide. Narrow or remove a piece.", units.length_fine(c.amount_mm)),
        "access" if c.ok => format!("A lane of {} or more", units.length_fine(c.amount_mm)),
        "access" => format!("No lane of {} or more", units.length_fine(c.amount_mm)),
        _ => c.detail.clone(),
    }
}

/// How many checks fail, in words for the status line.
pub fn failing_text(failing: usize) -> String {
    if failing == 1 { "1 check fails".to_string() } else { format!("{failing} checks fail") }
}

// ---- the components ----------------------------------------------------------------------

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::atlas::{MEASURES, Where};
use crate::street::vm::StreetVm;
use crate::street::watch::SheetWatch;

fn mode_class(mode: crate::shared::catalogue::Mode) -> String {
    serde_json::to_value(mode).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn change_cell(delta: i32, text: String) -> impl IntoView {
    view! { <td class=change_class(delta)>{text}</td> }
}

/// How the street's width is shared out among the uses, today and in the design.
#[component]
pub fn Space(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    move || {
        let (v, units) = w.now();
        let o = &v.outcomes;
        let rows = o
            .share
            .iter()
            .zip(&o.existing_share)
            .map(|(s, ex)| {
                view! {
                    <tr>
                        <td><span class=format!("mc mc-{}", mode_class(s.mode)) aria-hidden="true"></span>{s.label}</td>
                        <td>{units.fine(ex.mm)}</td>
                        <td>{units.fine(s.mm)}</td>
                        {change_cell(s.mm - ex.mm, units.signed(s.mm - ex.mm))}
                    </tr>
                }
            })
            .collect_view();
        view! {
            <caption class="sr-only">{format!("Width by use, in {}", units.name())}</caption>
            <thead>
                <tr><th scope="col">"Use"</th><th scope="col">"Today"</th><th scope="col">"Your design"</th><th scope="col">"Change"</th></tr>
            </thead>
            <tbody>{rows}</tbody>
        }
        .into_any()
    }
}

/// How many people the street moves in an hour, today and in the design.
#[component]
pub fn Capacity(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    move || {
        let v = w.view();
        let o = &v.outcomes;
        view! {
            <caption class="sr-only">"People per hour, placeholder rates"</caption>
            <thead>
                <tr><th scope="col"></th><th scope="col">"Today"</th><th scope="col">"Your design"</th><th scope="col">"Change"</th></tr>
            </thead>
            <tbody>
                <tr>
                    <td>"People per hour"</td>
                    <td>{group_thousands(o.existing_capacity_pph.into(), Locale::En)}</td>
                    <td>{group_thousands(o.capacity_pph.into(), Locale::En)}</td>
                    {change_cell(o.capacity_pph - o.existing_capacity_pph, capacity_change(o.existing_capacity_pph, o.capacity_pph))}
                </tr>
            </tbody>
        }
        .into_any()
    }
}

fn icon(ok: bool) -> impl IntoView {
    let d = if ok { "M2.5 8.5 6.5 12.5 13.5 3.5" } else { "M3 3l10 10M13 3 3 13" };
    view! { <svg viewBox="0 0 16 16" aria-hidden="true"><path d=d/></svg> }
}

/// Whether the street works, one line to a check.
#[component]
pub fn Checks(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    move || {
        let (v, units) = w.now();
        v.checks
            .iter()
            .map(|c| {
                view! {
                    <li class=if c.ok { "ok" } else { "bad" }>
                        {icon(c.ok)}
                        <div>
                            <b>{c.label}<span class="sr-only">{if c.ok { ": passes" } else { ": fails" }}</span></b>
                            <span>{check_detail(c, &v, units)}</span>
                        </div>
                    </li>
                }
            })
            .collect_view()
    }
}

fn failing(w: &SheetWatch) -> usize {
    w.view().checks.iter().filter(|c| !c.ok).count()
}

/// The button under the drawing that says how many checks fail and goes to them.
#[component]
pub fn FitChecks(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let go = move |_| {
        let doc = document();
        let click = |id: &str| {
            if let Some(el) = doc.get_element_by_id(id) {
                leptos::wasm_bindgen::JsCast::unchecked_into::<leptos::web_sys::HtmlElement>(el).click();
            }
        };
        if doc.document_element().is_some_and(|r| r.get_attribute("data-notes").as_deref() == Some("closed")) {
            click("notes-toggle");
        }
        click("t-checks");
        if let Some(tab) = doc.get_element_by_id("t-checks") {
            let _ = leptos::wasm_bindgen::JsCast::unchecked_into::<leptos::web_sys::HtmlElement>(tab).focus();
        }
    };
    view! {
        <button type="button" id="fit-checks" class="fit-checks" hidden=move || failing(&w) == 0 on:click=go>
            {move || failing_text(failing(&w))}
        </button>
    }
}

/// The count of failing checks on the Checks tab.
#[component]
pub fn ChecksBadge(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    view! {
        <span class="tab-n" id="checks-n" hidden=move || failing(&w) == 0>
            {move || (failing(&w) > 0).then(|| view! { {failing(&w)}<span class="sr-only">" fail"</span> })}
        </span>
    }
}

/// What has been changed, from the street as it is today.
#[component]
pub fn Revisions(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    #[cfg(target_arch = "wasm32")]
    {
        let version = w.version();
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
        let v = w.view();
        let last = v.revisions.len().checked_sub(1);
        let rows = v
            .revisions
            .iter()
            .enumerate()
            .map(|(i, r)| view! { <tr class=if Some(i) == last { "now" } else { "" }><td>{r.step}</td><td>{r.label.clone()}</td></tr> })
            .collect_view();
        view! {
            <thead><tr><th scope="col">"Step"</th><th scope="col">"What changed"</th></tr></thead>
            <tbody>
                <tr class=if v.revisions.is_empty() { "base now" } else { "base" }><td>"\u{2014}"</td><td>"Street today"</td></tr>
                {rows}
            </tbody>
        }
        .into_any()
    }
}

/// What each group of the Atlas toolbox means here, in one line.
fn group_note(group: &str) -> &'static str {
    match group {
        "Linear continuous measures" => "Lane arrangements along the street. Arrange lays the roadway out as that measure.",
        "Localized measures" => "Features at one junction. Set them on the intersection page.",
        "Area-wide measures" => "They cover many streets, so they are not modelled here.",
        _ => "",
    }
}

/// Every measure of the Transit Priority Atlas toolbox: the lane arrangements can
/// be recognised in this street and laid out from its width; the rest say where
/// they are set, or why they are not modelled.
#[component]
pub fn Measures(vm: Rc<StreetVm>) -> impl IntoView {
    let w = SheetWatch::new(vm);
    let mut groups: Vec<&'static str> = Vec::new();
    for m in &MEASURES {
        if !groups.contains(&m.group) {
            groups.push(m.group);
        }
    }
    move || {
        let v = w.view();
        groups
            .iter()
            .map(|group| {
                let rows = MEASURES
                    .iter()
                    .filter(|m| m.group == *group)
                    .map(|m| {
                        let found = (m.place == Where::Street).then(|| v.measures.iter().find(|f| f.code == m.code)).flatten();
                        let state = match (m.place, found) {
                            (Where::Street, Some(f)) if f.present => view! { <b class="m-on">"This street"</b> }.into_any(),
                            (Where::Street, Some(f)) if f.can_apply => {
                                let code = m.code;
                                view! {
                                    <button type="button" class="btn m-apply" aria-label=format!("Arrange the street as {} {}", m.code, m.name) on:click=move |_| { w.apply_measure(code); }>
                                        "Arrange"
                                    </button>
                                }
                                .into_any()
                            }
                            (Where::Street, Some(f)) => view! { <span class="m-not">{f.unavailable.unwrap_or("Will not fit")}</span> }.into_any(),
                            (Where::Junction, _) => "Set at a junction".into_any(),
                            _ => view! { <span class="m-not">"Not modelled"</span> }.into_any(),
                        };
                        let note = (m.place != Where::Street && !m.note.is_empty()).then(|| view! { <small>{m.note}</small> });
                        let problems = found.filter(|f| f.present).map(|f| f.problems.iter().map(|p| view! { <small class="m-problem">{p.clone()}</small> }).collect_view());
                        view! {
                            <tr>
                                <th scope="row"><span class="dirtag">{m.code}</span>{m.name}{note}{problems}</th>
                                <td>{state}</td>
                            </tr>
                        }
                    })
                    .collect_view();
                let note = group_note(group);
                view! {
                    <tbody>
                        <tr class="m-group"><th colspan="2" scope="colgroup">{*group}{(!note.is_empty()).then(|| view! { <small class="m-group-note">{note}</small> })}</th></tr>
                        {rows}
                    </tbody>
                }
            })
            .collect_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::model::Editor;

    #[test]
    fn a_change_goes_up_down_or_nowhere() {
        assert_eq!((change_class(5), change_class(-5), change_class(0)), ("up", "down", "zero"));
    }

    #[test]
    fn capacity_changes_as_a_percentage_of_what_it_was() {
        assert_eq!(capacity_change(1_000, 1_120), "+12%");
        assert_eq!(capacity_change(1_000, 920), "\u{2212}8%");
        assert_eq!(capacity_change(1_000, 1_000), "0");
        assert_eq!(capacity_change(1_000, 1_004), "0");
        assert_eq!(capacity_change(0, 500), "0");
        // A half rounds toward the larger, as it always has on this page.
        assert_eq!(capacity_change(1_000, 1_005), "+1%");
        assert_eq!(capacity_change(1_000, 995), "0");
    }

    fn check(v: &View, id: &str) -> Check {
        let c = v.checks.iter().find(|c| c.id == id).unwrap();
        Check { id: c.id, ok: c.ok, amount_mm: c.amount_mm, label: c.label, detail: c.detail.clone() }
    }

    #[test]
    fn the_fit_check_says_whether_the_street_is_full_has_room_or_is_too_wide() {
        let mut e = Editor::new(0);
        let v = e.view();
        assert_eq!(check_detail(&check(&v, "fits"), &v, Units::Metres), "Every metre is used");
        let first = v.segments[0].uid;
        e.remove(first);
        let v = e.view();
        assert_eq!(check_detail(&check(&v, "fits"), &v, Units::Metres), "3.3 m left to use");
        let mut e = Editor::new(0);
        e.set_width(first, 3_800);
        let v = e.view();
        assert_eq!(check_detail(&check(&v, "fits"), &v, Units::Metres), "0.5 m too wide. Narrow or remove a piece.");
    }

    #[test]
    fn the_access_check_names_the_lane_width_it_asks_for() {
        let v = Editor::new(0).view();
        let c = check(&v, "access");
        let said = check_detail(&c, &v, Units::Metres);
        assert!(said.starts_with(if c.ok { "A lane of " } else { "No lane of " }) && said.ends_with(" or more"), "{said}");
    }

    #[test]
    fn other_checks_say_only_their_detail() {
        let v = Editor::new(0).view();
        let other = v.checks.iter().find(|c| c.id != "fits" && c.id != "access").unwrap();
        let c = Check { id: other.id, ok: other.ok, amount_mm: other.amount_mm, label: other.label, detail: other.detail.clone() };
        assert_eq!(check_detail(&c, &v, Units::Metres), other.detail);
    }

    #[test]
    fn the_status_line_counts_failing_checks() {
        assert_eq!(failing_text(1), "1 check fails");
        assert_eq!(failing_text(3), "3 checks fail");
    }
}
