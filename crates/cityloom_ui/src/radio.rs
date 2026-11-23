//! A group of radio buttons: exactly one option is chosen, or none yet.

use leptos::prelude::*;

/// One option of a group, as it is drawn.
pub struct RadioOption {
    pub id: Option<String>,
    /// Drawn before the name.
    pub glyph: Option<AnyView>,
    pub name: Signal<String>,
    /// Inside the button, under the name.
    pub help: Option<Signal<String>>,
    /// The id and the words of a description after the button, which the button is described by.
    pub description: Option<(String, Signal<String>)>,
    /// Whether it can be chosen.
    pub enabled: bool,
}

impl RadioOption {
    /// An option that is its name alone and can be chosen.
    pub fn new(name: Signal<String>) -> Self {
        todo!()
    }
}

/// A list of radio buttons, one tab stop: the chosen option, or the first when none is. The arrow keys move the focus and
/// the choice and wrap, Home and End go to the ends.
#[component]
pub fn RadioGroup<K, D>(
    options: Signal<Vec<K>>,
    chosen: Signal<Option<K>>,
    draw: D,
    choose: Callback<K>,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] labelled_by: Option<String>,
    #[prop(optional)] label: Option<Signal<String>>,
    #[prop(optional, into)] described_by: Option<String>,
    #[prop(default = "opts")] class: &'static str,
) -> impl IntoView
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    D: Fn(K) -> RadioOption + Send + Sync + 'static,
{
    let view: AnyView = todo!();
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{attr, has_class, html, said, tags, word};

    fn keys() -> Signal<Vec<&'static str>> {
        Signal::derive(|| vec!["a", "b", "c"])
    }

    fn name(key: &str) -> Signal<String> {
        word(match key {
            "a" => "Alpha",
            "b" => "Beta",
            _ => "Gamma",
        })
    }

    /// The group labelled by `h`, with `chosen` chosen, each option as `draw` makes it.
    fn drawn(chosen: Option<&'static str>, draw: impl Fn(&'static str) -> RadioOption + Send + Sync + 'static) -> String {
        html(move || {
            view! { <RadioGroup options=keys() chosen=Signal::derive(move || chosen) draw=draw choose=Callback::new(|_| {}) labelled_by="h"/> }.into_any()
        })
    }

    fn group(chosen: Option<&'static str>) -> String {
        drawn(chosen, |k| RadioOption::new(name(k)))
    }

    /// Attribute `name` of each button, in order.
    fn of_buttons<'a>(h: &'a str, name: &str) -> Vec<Option<&'a str>> {
        tags(h, "button").into_iter().map(|b| attr(b, name)).collect()
    }

    #[test]
    fn a_group_is_a_labelled_list_of_radio_buttons_with_the_chosen_one_checked() {
        let h = group(Some("b"));
        let lists = tags(&h, "ul");
        assert_eq!(lists.len(), 1, "{h}");
        assert!(h.starts_with("<ul"), "{h}");
        assert!(has_class(lists[0], "opts"), "{h}");
        assert_eq!((attr(lists[0], "role"), attr(lists[0], "aria-labelledby")), (Some("radiogroup"), Some("h")), "{h}");
        assert_eq!(h.matches(r#"<li role="none">"#).count(), 3, "{h}");
        let buttons = tags(&h, "button");
        assert_eq!(buttons.len(), 3, "{h}");
        for button in buttons {
            assert_eq!((attr(button, "type"), attr(button, "role")), (Some("button"), Some("radio")), "{button}");
            assert!(has_class(button, "opt"), "{button}");
        }
        for name in ["Alpha", "Beta", "Gamma"] {
            assert_eq!(h.matches(&format!("<span>{name}</span>")).count(), 1, "{name}: {h}");
        }
        assert_eq!(h.matches(r#"class="tick""#).count(), 3, "{h}");
        assert_eq!(of_buttons(&h, "aria-checked"), [Some("false"), Some("true"), Some("false")], "{h}");
    }

    #[test]
    fn only_the_chosen_option_is_in_the_tab_order() {
        let h = group(Some("b"));
        assert_eq!(of_buttons(&h, "tabindex"), [Some("-1"), Some("0"), Some("-1")], "{h}");
    }

    #[test]
    fn with_nothing_chosen_only_the_first_option_is_in_the_tab_order() {
        for chosen in [None, Some("not among the options")] {
            let h = group(chosen);
            assert_eq!(of_buttons(&h, "tabindex"), [Some("0"), Some("-1"), Some("-1")], "{chosen:?}: {h}");
            assert_eq!(of_buttons(&h, "aria-checked"), [Some("false"); 3], "{chosen:?}: {h}");
        }
    }

    #[test]
    fn a_group_takes_its_id_its_label_its_description_and_its_class() {
        let h = html(|| {
            view! {
                <RadioGroup
                    options=keys()
                    chosen=Signal::derive(|| Some("a"))
                    draw=|k| RadioOption { id: Some(format!("opt-{k}")), ..RadioOption::new(name(k)) }
                    choose=Callback::new(|_| {})
                    id="g"
                    label=word("Pattern for phase 1")
                    described_by="d"
                    class="opts phase-patterns"
                />
            }
            .into_any()
        });
        let list = tags(&h, "ul")[0];
        assert_eq!((attr(list, "id"), attr(list, "aria-describedby"), attr(list, "class")), (Some("g"), Some("d"), Some("opts phase-patterns")), "{h}");
        assert!(list.contains(r#"role="radiogroup" aria-label="Pattern for phase 1""#), "{list}");
        assert_eq!(attr(list, "aria-labelledby"), None, "{list}");
        assert_eq!(of_buttons(&h, "id"), [Some("opt-a"), Some("opt-b"), Some("opt-c")], "{h}");
        // Given none of them, it has none.
        let h = group(Some("a"));
        let list = tags(&h, "ul")[0];
        assert_eq!((attr(list, "id"), attr(list, "aria-label"), attr(list, "aria-describedby")), (None, None, None), "{list}");
        assert_eq!(of_buttons(&h, "id"), [None; 3], "{h}");
    }

    #[test]
    fn an_option_s_help_is_in_its_button_and_its_description_after_it() {
        let h = drawn(Some("a"), |k| match k {
            "a" => RadioOption { help: Some(word("Nothing is sent")), ..RadioOption::new(name(k)) },
            "b" => RadioOption { description: Some(("desc-b".into(), word("Every street at once"))), ..RadioOption::new(name(k)) },
            _ => RadioOption::new(name(k)),
        });
        let items: Vec<(&str, &str)> = h.split("<li").skip(1).map(|li| li.split_once("</button>").expect("a button")).collect();
        assert_eq!(items.len(), 3, "{h}");
        let helped = r#"<span class="choice-body"><span class="choice-label">Alpha</span><span class="choice-help">Nothing is sent</span></span>"#;
        assert!(items[0].0.contains(helped), "{h}");
        assert_eq!(h.matches("choice-help").count(), 1, "only the first option has help: {h}");
        let after = items[1].1;
        let description = tags(after, "span").first().copied().unwrap_or_else(|| panic!("a description after the button: {h}"));
        assert_eq!((attr(description, "id"), attr(description, "class")), (Some("desc-b"), Some("insp-range")), "{description}");
        assert!(after.contains(&format!("{description}Every street at once</span>")), "{after}");
        assert_eq!(of_buttons(&h, "aria-describedby"), [None, Some("desc-b"), None], "{h}");
    }

    #[test]
    fn an_option_that_cannot_be_chosen_is_said_to_be_and_keeps_the_tab_stop_while_checked() {
        let h = drawn(Some("a"), |k| RadioOption { enabled: k != "a", ..RadioOption::new(name(k)) });
        assert_eq!(of_buttons(&h, "aria-disabled"), [Some("true"), None, None], "{h}");
        assert_eq!(of_buttons(&h, "aria-checked"), [Some("true"), Some("false"), Some("false")], "{h}");
        assert_eq!(of_buttons(&h, "tabindex"), [Some("0"), Some("-1"), Some("-1")], "{h}");
    }

    #[test]
    fn a_radio_group_has_no_word_of_its_own() {
        let h = drawn(Some("a"), |k| RadioOption {
            help: (k == "a").then(|| word("")),
            description: (k == "b").then(|| ("desc-b".into(), word(""))),
            enabled: k != "c",
            ..RadioOption::new(word(""))
        });
        assert_eq!(said(&h), "", "{h}");
    }
}
