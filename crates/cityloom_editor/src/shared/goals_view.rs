//! The goal picker and its guidance, drawn in the Checks tab of the street, junction and map pages. It offers and
//! explains; no control here edits a design (`goals.rs`). On the map a change to the selected streets can be picked
//! as the one to review, which is as far as this goes: making it stays with the review and its apply button.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::atlas::name_key;
use crate::shared::goals::{Goal, GoalChoice, Place};
use crate::shared::i18n::{Args, I18n};

/// The picker and, for the chosen goal, why each measure fits and what it is likely to cost. Call it inside a
/// closure that tracks the page's words. `pick_change` is the map's way of choosing the change to review, given the
/// change's id; it is `None` where there is nothing to pick it for (no editor offers it, or no street is selected).
pub fn goal_guide(choice: GoalChoice, place: Place, i18n: &I18n, pick_change: Option<Rc<dyn Fn(&'static str)>>) -> impl IntoView + use<> {
    let plain = |key: &str| i18n.tr(key, &Args::new());
    let current = choice.get();
    let radio = |goal: Option<Goal>, label: String| {
        let choice = choice.clone();
        let id = format!("goal-{}", goal.map_or("none", Goal::id));
        view! {
            <label class="goal-choice">
                <input type="radio" name="goal" id=id checked=goal == current on:change=move |_| choice.set(goal)/>
                {label}
            </label>
        }
    };
    let mut radios = vec![radio(None, plain("goal-none")).into_any()];
    radios.extend(Goal::ALL.into_iter().map(|g| radio(Some(g), plain(g.name())).into_any()));
    let guidance = current.map(|goal| {
        let picks = goal
            .picks_at(place)
            .map(|p| {
                let conflict = p.conflict.map(|(other, why)| {
                    let text = i18n.tr("goal-conflict", &Args::new().str("goal", i18n.tr(other.name(), &Args::new())).str("why", plain(why)));
                    view! { <span class="goal-conflict">{text}</span> }
                });
                let title = match p.name {
                    Some(name) => plain(name),
                    None => i18n.tr("goal-measure", &Args::new().str("code", p.code).str("name", plain(&name_key(p.code)))),
                };
                let choose = (place == Place::Map).then(|| match &pick_change {
                    Some(pick) => {
                        let (pick, op) = (pick.clone(), p.code);
                        view! {
                            <button type="button" class="btn quiet goal-use" data-op=op on:click=move |_| pick(op)>
                                {i18n.tr("goal-use", &Args::new().str("change", plain(p.name.unwrap_or(""))))}
                            </button>
                        }
                        .into_any()
                    }
                    None => view! { <span class="goal-select">{plain("goal-select-first")}</span> }.into_any(),
                });
                view! {
                    <li>
                        <b>{title}</b>
                        <span>{plain(p.fit)}</span>
                        {conflict}
                        {choose}
                    </li>
                }
            })
            .collect_view();
        let count = goal.picks_at(place).count();
        view! {
            <p>{plain(goal.intro())}</p>
            <p>{plain("goal-focus-note")}</p>
            {if count == 0 {
                view! { <p>{plain("goal-no-picks")}</p> }.into_any()
            } else {
                view! {
                    <details class="goal-measures">
                        <summary>{i18n.tr("goal-picks-summary", &Args::new().num("n", count as i64))}</summary>
                        <ul class="goal-picks">{picks}</ul>
                    </details>
                }
                .into_any()
            }}
        }
    });
    view! {
        <fieldset class="goal-guide">
            <legend>{plain("goal-legend")}</legend>
            {radios}
            <p class="goal-note">{plain("goal-changes-nothing")}</p>
            <div aria-live="polite">{guidance}</div>
        </fieldset>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::MemoryStorage;
    use crate::shared::testing::html;
    use std::rc::Rc;

    fn drawn(goal: Option<Goal>, place: Place, locale: Locale) -> String {
        let choice = GoalChoice::new(Rc::new(MemoryStorage::default()));
        choice.set(goal);
        html(move || goal_guide(choice, place, &crate::i18n_for(locale), None).into_any())
    }

    #[test]
    fn with_no_goal_the_picker_offers_every_goal_and_no_guidance() {
        let h = drawn(None, Place::Street, Locale::En);
        for g in Goal::ALL {
            assert!(h.contains(&format!("id=\"goal-{}\"", g.id())), "{}", g.id());
        }
        assert!(h.contains("id=\"goal-none\"") && !h.contains("goal-picks"));
        assert!(h.contains("never changes the design"));
    }

    #[test]
    fn the_measures_are_in_a_closed_disclosure_that_counts_them() {
        let h = drawn(Some(Goal::Buses), Place::Junction, Locale::En);
        assert!(h.contains("<details class=\"goal-measures\""), "{h}");
        assert!(!h.contains("goal-measures\" open"), "closed until opened");
        assert!(h.contains("Measures to consider (8)"), "{h}");
        let none = drawn(Some(Goal::SchoolRoute), Place::Street, Locale::En);
        assert!(none.contains("<details class=\"goal-measures\""));
    }

    #[test]
    fn on_the_map_a_change_is_named_and_offered_only_where_streets_are_selected() {
        let choice = GoalChoice::new(Rc::new(MemoryStorage::default()));
        choice.set(Some(Goal::SchoolRoute));
        let without = html({
            let choice = choice.clone();
            move || goal_guide(choice, Place::Map, &crate::i18n_for(Locale::En), None).into_any()
        });
        assert!(without.contains("School street") && without.contains("Select streets on the map"), "{without}");
        assert!(!without.contains("goal-use"));
        let with = html(move || goal_guide(choice, Place::Map, &crate::i18n_for(Locale::En), Some(Rc::new(|_| {}))).into_any());
        assert!(with.contains("class=\"btn quiet goal-use\"") && with.contains("data-op=\"school\"") && with.contains("Review School street"), "{with}");
    }

    #[test]
    fn a_chosen_goal_explains_each_measure_and_names_the_likely_conflict() {
        let h = drawn(Some(Goal::Buses), Place::Junction, Locale::En);
        assert!(h.contains("M1") && h.contains("Bus Bulbs"));
        assert!(h.contains("goal-conflict"), "bus bulbs work against the school route");
        assert!(h.contains("Improve a school route"));
    }

    #[test]
    fn both_languages_word_every_goal_in_both_editors() {
        for locale in [Locale::En, Locale::FrCa] {
            for g in Goal::ALL {
                for place in [Place::Street, Place::Junction, Place::Map] {
                    let h = drawn(Some(g), place, locale);
                    // An unknown message renders as its own id.
                    assert!(!h.contains("goal-fit") && !h.contains("goal-conflict-") && !h.contains("goal-intro"), "{} {place:?} {locale:?}", g.id());
                }
            }
        }
    }
}
