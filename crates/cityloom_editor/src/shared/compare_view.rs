//! The frame of the page that compares two designs of a place (`compare.rs`): a menu for each side, the two columns
//! the street's or the junction's half draws, and the summary of what differs. Every control is a native one. A
//! refused pick is said under the menus; the view-model (`Compare::choose`) says it in the live region too, before the
//! headline.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::compare::{CompareHost, Pick};
use crate::shared::i18n::Args;
use crate::shared::said::{Arg, Said, say};

/// The option's value for the baseline.
const BASELINE: &str = "base";

/// What a menu's value picks: None for the baseline.
fn picked(value: &str) -> Option<u32> {
    if value == BASELINE { None } else { value.parse().ok() }
}

/// The menu value of a design (None: the baseline).
fn value_of(id: Option<u32>) -> String {
    id.map_or_else(|| BASELINE.to_string(), |id| id.to_string())
}

/// The comparison page: `column_a` and `column_b` are the two halves' columns.
#[component]
pub fn CompareFrame<H: CompareHost + 'static>(
    host: Rc<H>,
    column_a: impl Fn() -> AnyView + 'static,
    column_b: impl Fn() -> AnyView + 'static,
) -> impl IntoView {
    let words = StoredValue::new_local(host.i18n());
    let host = StoredValue::new_local(host);
    let tr = move |key: &'static str| move || words.with_value(|i18n| i18n.tr(key, &Args::new()));
    // In the units the page shows, watched: what is said is said again when the person chooses others.
    let said = move |what: &Said| words.with_value(|i18n| say(i18n, host.with_value(|h| h.units()), what));
    let title = move || {
        let place = said(&host.with_value(|h| h.place_name()));
        words.with_value(|i18n| i18n.tr("cmp-title", &Args::new().str("place", place)))
    };
    let state = move || host.with_value(|h| h.compare_state().clone());
    let name_of = move |side: Pick| {
        let baseline = words.with_value(|i18n| i18n.tr("cmp-baseline", &Args::new()));
        let state = state();
        state.name_of(state.pick(side), &baseline)
    };
    let heading = move |key: &'static str, side: Pick| move || words.with_value(|i18n| i18n.tr(key, &Args::new().str("name", name_of(side))));
    let menu = move |side: Pick, id: &'static str, label: &'static str| {
        let options = move || {
            state()
                .items()
                .into_iter()
                .map(|item| {
                    let this = item.id;
                    view! { <option value=this.to_string() prop:selected=move || state().pick(side) == Some(this)>{item.name}</option> }
                })
                .collect_view()
        };
        let select = NodeRef::<leptos::html::Select>::new();
        // A refused pick changes no signal, so the menu would go on showing it: put it back to the design kept.
        let change = move |ev: leptos::ev::Event| {
            let id = picked(&event_target_value(&ev));
            host.with_value(|h| h.choose(side, id));
            if let Some(select) = select.get_untracked() {
                select.set_value(&value_of(state().pick_now(side)));
            }
        };
        view! {
            <div class="compare-pick">
                <label for=id>{tr(label)}</label>
                <select node_ref=select id=id on:change=change>
                    <option value=BASELINE prop:selected=move || state().pick(side).is_none()>{tr("cmp-baseline")}</option>
                    {options}
                </select>
            </div>
        }
    };
    let refusal = move || state().refusal().map(|why| said(&why));
    let summary = move || {
        let summary = host.with_value(|h| h.summary());
        let headline = said(&summary.headline(&name_of(Pick::A), &name_of(Pick::B)));
        if summary.is_same() {
            return view! { <p class="compare-same">{headline}</p> }.into_any();
        }
        let groups = summary
            .groups
            .into_iter()
            .filter(|g| !g.lines.is_empty())
            .map(|g| {
                let lines = g.lines.iter().map(|line| view! { <li>{said(line)}</li> }).collect_view();
                view! {
                    <h3>{tr(g.heading)}</h3>
                    <ul>{lines}</ul>
                }
            })
            .collect_view();
        // the other things that differ, joined as the junction joins its lists
        let also = summary
            .also
            .into_iter()
            .reduce(|head, tail| Said::new("jn-list-comma").with("head", Arg::Said(Box::new(head))).with("tail", Arg::Said(Box::new(tail))))
            .map(|list| {
                let text = said(&Said::new("cmp-also").with("list", Arg::Said(Box::new(list))));
                view! { <p class="compare-also">{text}</p> }
            });
        view! {
            <p class="compare-headline">{headline}</p>
            {groups}
            {also}
        }
        .into_any()
    };
    view! {
        <h1 id="compare-h">{title}</h1>
        <a id="compare-back" href=move || host.with_value(|h| h.back_href())>
            {move || host.with_value(|h| tr(h.back_key())())}
        </a>
        <div class="compare-picks">
            {menu(Pick::A, "compare-a", "cmp-pick-a")} {menu(Pick::B, "compare-b", "cmp-pick-b")}
        </div>
        <p id="compare-refusal" class="compare-refusal">
            {refusal}
        </p>
        <div class="compare-cols">
            <section class="compare-col" aria-labelledby="compare-a-h">
                <h2 id="compare-a-h">{heading("cmp-col-a", Pick::A)}</h2>
                {column_a()}
            </section>
            <section class="compare-col" aria-labelledby="compare-b-h">
                <h2 id="compare-b-h">{heading("cmp-col-b", Pick::B)}</h2>
                {column_b()}
            </section>
        </div>
        <section id="compare-summary" aria-labelledby="compare-summary-h">
            <h2 id="compare-summary-h">{tr("cmp-summary-h")}</h2>
            {summary}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::shared::compare::{CompareHost, CompareState, DiffGroup, Pick, Summary};
    use crate::shared::i18n::Locale;
    use crate::shared::said::{Arg, Said};
    use crate::shared::testing::html;
    use crate::shared::units::Units;

    struct Fake {
        i18n: Rc<crate::shared::i18n::I18n>,
        state: CompareState,
        summary: Summary,
        units: leptos::prelude::ArcRwSignal<Units>,
    }

    impl CompareHost for Fake {
        fn i18n(&self) -> Rc<crate::shared::i18n::I18n> {
            self.i18n.clone()
        }
        fn compare_state(&self) -> &CompareState {
            &self.state
        }
        fn choose(&self, side: Pick, id: Option<u32>) {
            self.state.set(side, id);
        }
        fn summary(&self) -> Summary {
            self.summary.clone()
        }
        fn back_href(&self) -> String {
            "street.html?street=7".into()
        }
        fn back_key(&self) -> &'static str {
            "cmp-back-street"
        }
        fn place_name(&self) -> Said {
            Said::new("city-the-edge-of-the-map")
        }
        fn units(&self) -> Units {
            self.units.get()
        }
        fn set_units(&self, units: Units) {
            self.units.set(units);
        }
    }

    fn fake(locale: Locale, summary: Summary) -> Rc<Fake> {
        let state = CompareState::default();
        state.list(vec![(1, "Wider".into()), (2, "Greener".into())]);
        state.set(Pick::B, Some(1));
        Rc::new(Fake { i18n: crate::i18n_for(locale), state, summary, units: leptos::prelude::ArcRwSignal::new(Units::Metres) })
    }

    fn one_group() -> Summary {
        Summary { groups: vec![DiffGroup { heading: "cmp-h-space", lines: vec![Said::new("scenario-missing")] }], also: vec![Said::new("cmp-other-speed")] }
    }

    fn render(host: Rc<Fake>) -> String {
        html(move || {
            view! { <CompareFrame host=host column_a=|| view! { <p>"col a"</p> }.into_any() column_b=|| view! { <p>"col b"</p> }.into_any()/> }.into_any()
        })
    }

    #[test]
    fn two_pickers_list_the_baseline_first_then_each_design() {
        let out = render(fake(Locale::En, one_group()));
        for id in ["compare-a", "compare-b"] {
            let at = out.find(&format!("id=\"{id}\"")).unwrap();
            let rest = &out[at..out[at..].find("</select>").unwrap() + at];
            let base = rest.find("<option value=\"base\"").unwrap();
            let one = rest.find("<option value=\"1\"").unwrap();
            let two = rest.find("<option value=\"2\"").unwrap();
            assert!(base < one && one < two, "{id}: {rest}");
        }
        assert!(out.contains("<label for=\"compare-a\">Design A</label>"), "{out}");
        assert!(out.contains("<label for=\"compare-b\">Design B</label>"));
        assert!(out.contains("id=\"compare-h\""));
        assert!(out.contains("id=\"compare-back\""));
        assert!(out.contains("Back to the street"));
        assert!(out.contains("href=\"street.html?street=7\""));
        assert!(out.contains("id=\"compare-refusal\""));
    }

    #[test]
    fn the_columns_are_labelled_by_their_headings_with_the_design_names() {
        let out = render(fake(Locale::En, one_group()));
        assert!(out.contains("aria-labelledby=\"compare-a-h\""), "{out}");
        assert!(out.contains("aria-labelledby=\"compare-b-h\""));
        assert!(out.contains("A: Baseline"), "{out}");
        assert!(out.contains("B: Wider"));
        assert!(out.contains("col a") && out.contains("col b"));
    }

    #[test]
    fn the_summary_lists_groups_lines_and_what_else_differs() {
        let out = render(fake(Locale::En, one_group()));
        assert!(out.contains("What differs in B"), "{out}");
        assert!(out.contains("<h3>Space</h3>"), "{out}");
        assert!(out.contains("That scenario is no longer here."));
        assert!(out.contains("Also differs in: speed limits"), "{out}");
        assert!(out.contains("differs from Baseline in 2 ways"), "{out}");
    }

    #[test]
    fn several_other_things_are_joined_with_commas() {
        let mut summary = one_group();
        summary.also.push(Said::new("cmp-other-stops").with("x", Arg::Num(1)));
        let out = render(fake(Locale::En, summary));
        assert!(out.contains("Also differs in: speed limits, stops"), "{out}");
    }

    #[test]
    fn an_empty_summary_says_the_designs_are_the_same() {
        let out = render(fake(Locale::En, Summary { groups: vec![DiffGroup { heading: "cmp-h-space", lines: vec![] }], also: vec![] }));
        assert!(out.contains("Wider and Baseline are the same."), "{out}");
        assert!(!out.contains("compare-also"));
        assert!(!out.contains("<h3"));
    }

    #[test]
    fn it_is_drawn_in_french_with_a_no_break_space_before_the_colon() {
        let out = render(fake(Locale::FrCa, one_group()));
        assert!(out.contains(">Conception A</label>"), "{out}");
        assert!(out.contains("Diffère aussi par\u{a0}: les limites de vitesse"), "{out}");
    }

    #[test]
    fn a_refusal_is_said_under_the_pickers() {
        let host = fake(Locale::En, one_group());
        host.state.refuse(Said::new("scenario-missing"));
        let out = render(host);
        let at = out.find("id=\"compare-refusal\"").unwrap();
        let paragraph = &out[at..];
        let text = &paragraph[paragraph.find('>').unwrap() + 1..paragraph.find("</p>").unwrap()];
        assert_eq!(text, "That scenario is no longer here.", "{out}");
    }

    #[test]
    fn lengths_are_said_in_the_units_the_page_shows() {
        let width = Said::new("cmp-piece-added").with("kind", Arg::KindLower("travel")).with("width", Arg::Length(3_048));
        let summary = Summary { groups: vec![DiffGroup { heading: "cmp-h-pieces", lines: vec![width] }], also: vec![] };
        let host = fake(Locale::En, summary);
        host.set_units(Units::Feet);
        let out = render(host);
        assert!(out.contains(", 10.0 ft</li>"), "{out}");
    }

    #[test]
    fn a_menu_value_is_the_baseline_or_an_id() {
        assert_eq!(picked("base"), None);
        assert_eq!(picked("3"), Some(3));
        // not a design of the menu: taken as the baseline, which the menu always holds
        assert_eq!(picked("x"), None);
        assert_eq!(value_of(None), "base");
        assert_eq!(value_of(Some(3)), "3");
    }
}
