//! The goal picker and its guidance, drawn in the Checks tab of the street and junction pages. It offers and
//! explains; no control here edits a design (`goals.rs`).

use leptos::prelude::*;

use crate::shared::atlas::name_key;
use crate::shared::goals::{Goal, GoalChoice, Place};
use crate::shared::i18n::{Args, I18n};

/// The picker and, for the chosen goal, why each measure fits and what it is likely to cost. Call it inside a
/// closure that tracks the page's words.
pub fn goal_guide(choice: GoalChoice, place: Place, i18n: &I18n) -> impl IntoView + use<> {
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
                view! {
                    <li>
                        <b>{i18n.tr("goal-measure", &Args::new().str("code", p.code).str("name", plain(&name_key(p.code))))}</b>
                        <span>{plain(p.fit)}</span>
                        {conflict}
                    </li>
                }
            })
            .collect_view();
        let none = goal.picks_at(place).next().is_none();
        view! {
            <p>{plain(goal.intro())}</p>
            <p>{plain("goal-focus-note")}</p>
            <h3>{plain("goal-picks-heading")}</h3>
            {none.then(|| view! { <p>{plain("goal-no-picks")}</p> })}
            <ul class="goal-picks">{picks}</ul>
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
        html(move || goal_guide(choice, place, &crate::i18n_for(locale)).into_any())
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
                for place in [Place::Street, Place::Junction] {
                    let h = drawn(Some(g), place, locale);
                    // An unknown message renders as its own id.
                    assert!(!h.contains("goal-fit") && !h.contains("goal-conflict-") && !h.contains("goal-intro"), "{} {place:?} {locale:?}", g.id());
                }
            }
        }
    }
}
