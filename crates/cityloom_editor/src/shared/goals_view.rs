//! The goal picker (a menu) and its guidance, drawn in the Checks tab of the street, junction and map pages. It offers and
//! explains; no control here edits a design (`goals.rs`). On the map a change to the selected streets can be picked
//! as the one to review, which is as far as this goes: making it stays with the review and its apply button.

use std::rc::Rc;

use leptos::prelude::*;

use crate::shared::atlas::name_key;
use crate::shared::goals::{Goal, GoalChoice, Place};
use crate::shared::i18n::{Args, I18n};

/// The picker: a menu of the goals, "No goal" first, with the note that a goal changes nothing. Call it inside a
/// closure that tracks only the page's words, so a choice does not draw the menu again under the focus; the chosen
/// goal is set as the menu's value.
pub fn goal_picker(choice: GoalChoice, i18n: &I18n) -> impl IntoView + use<> {
    let plain = |key: &str| i18n.tr(key, &Args::new());
    let current = choice.get_untracked();
    let option = |goal: Option<Goal>, label: String| {
        let value = goal.map_or("", Goal::id);
        view! { <option value=value selected=goal == current>{label}</option> }
    };
    let mut options = vec![option(None, plain("goal-none")).into_any()];
    options.extend(Goal::ALL.into_iter().map(|g| option(Some(g), plain(g.name())).into_any()));
    let chosen = StoredValue::new_local(choice);
    view! {
        <div class="goal-guide">
            <label for="goal-pick">{plain("goal-legend")}</label>
            <select
                id="goal-pick"
                prop:value=move || chosen.with_value(|c| c.get()).map_or("", Goal::id)
                on:change=move |ev| chosen.with_value(|c| c.set(Goal::from_id(&event_target_value(&ev))))
            >
                {options}
            </select>
            <p class="goal-note">{plain("goal-changes-nothing")}</p>
        </div>
    }
}

/// For the chosen goal, why each measure fits and what it is likely to cost; nothing without one. Call it inside a
/// closure that tracks the page's words and the goal. `pick_change` is the map's way of choosing the change to review,
/// given the change's id; it is `None` where there is nothing to pick it for (no editor offers it, or no street is
/// selected).
pub fn goal_guidance(goal: Option<Goal>, place: Place, i18n: &I18n, pick_change: Option<Rc<dyn Fn(&'static str)>>) -> impl IntoView + use<> {
    let plain = |key: &str| i18n.tr(key, &Args::new());
    let guidance = goal.map(|goal| {
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
                    <details class="goal-measures" open>
                        <summary>{i18n.tr("goal-picks-summary", &Args::new().num("n", count as i64))}</summary>
                        <ul class="goal-picks">{picks}</ul>
                    </details>
                }
                .into_any()
            }}
        }
    });
    view! { <div class="goal-guidance" aria-live="polite">{guidance}</div> }
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
        let i18n = crate::i18n_for(locale);
        html(move || view! { {goal_picker(choice.clone(), &i18n)}{goal_guidance(goal, place, &i18n, None)} }.into_any())
    }

    #[test]
    fn the_picker_is_a_labelled_menu_of_every_goal_with_the_chosen_one_selected() {
        let h = drawn(None, Place::Street, Locale::En);
        assert!(h.contains(r#"<label for="goal-pick">Planning goal</label>"#) && h.contains(r#"<select id="goal-pick""#), "{h}");
        assert!(!h.contains(r#"type="radio""#), "a menu, not a radio button per goal: {h}");
        assert!(h.contains(r#"<option value="" selected>No goal</option>"#), "no goal is the choice: {h}");
        for g in Goal::ALL {
            assert!(h.contains(&format!(r#"<option value="{}">"#, g.id())), "{}: {h}", g.id());
        }
        assert!(h.contains("never changes the design") && !h.contains("goal-picks"), "no guidance without a goal: {h}");
        let chosen = drawn(Some(Goal::Buses), Place::Street, Locale::En);
        assert!(chosen.contains(r#"<option value="buses" selected>"#), "{chosen}");
    }

    #[test]
    fn the_measures_are_listed_open_and_counted() {
        let h = drawn(Some(Goal::Buses), Place::Junction, Locale::En);
        assert!(h.contains(r#"<details open class="goal-measures">"#), "open, in the goal's own column: {h}");
        assert!(h.contains("Measures to consider (8)"), "{h}");
        let none = drawn(Some(Goal::SchoolRoute), Place::Street, Locale::En);
        assert!(none.contains(r#"<details open class="goal-measures">"#));
    }

    #[test]
    fn on_the_map_a_change_is_named_and_offered_only_where_streets_are_selected() {
        let choice = GoalChoice::new(Rc::new(MemoryStorage::default()));
        choice.set(Some(Goal::SchoolRoute));
        let without = html(move || goal_guidance(choice.get(), Place::Map, &crate::i18n_for(Locale::En), None).into_any());
        assert!(without.contains("School street") && without.contains("Select streets on the map"), "{without}");
        assert!(!without.contains("goal-use"));
        let with = html(move || goal_guidance(Some(Goal::SchoolRoute), Place::Map, &crate::i18n_for(Locale::En), Some(Rc::new(|_| {}))).into_any());
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
