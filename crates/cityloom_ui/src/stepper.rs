//! A value between two buttons that step it, and the number field that usually is that value.

use leptos::prelude::*;

/// A button that steps down, the value (`children`), a button that steps up.
#[component]
pub fn Stepper(
    /// What the buttons say they do.
    less: Signal<String>,
    more: Signal<String>,
    /// Told -1 or 1.
    on_step: Callback<i32>,
    #[prop(optional, into)] less_id: Option<String>,
    #[prop(optional, into)] more_id: Option<String>,
    /// Whether the value is at its end: the button is said to be disabled and steps nothing.
    #[prop(optional)]
    less_off: Option<Signal<bool>>,
    #[prop(optional)] more_off: Option<Signal<bool>>,
    /// The id of the heading that names it: it is then a group.
    #[prop(optional, into)]
    group: Option<String>,
    children: Children,
) -> impl IntoView {
    let view: AnyView = todo!();
    view
}

/// A number and its unit. An arrow key steps it; a change is handed to `commit`, which says whether it was taken, and the
/// field then shows `value`. With `on_end`, Enter commits and Escape puts the value back, and either ends the typing,
/// unless what was typed was refused.
#[component]
pub fn NumberField(
    value: Signal<String>,
    unit: Signal<String>,
    commit: Callback<String, bool>,
    on_step: Callback<i32>,
    #[prop(default = "decimal")] inputmode: &'static str,
    #[prop(optional, into)] id: Option<String>,
    #[prop(optional, into)] labelled_by: Option<String>,
    #[prop(optional)] described_by: Option<Signal<Option<String>>>,
    /// Whether a unit of more than two characters is marked as long.
    #[prop(optional)]
    long_unit: bool,
    #[prop(optional)] on_end: Option<Callback<()>>,
) -> impl IntoView {
    let view: AnyView = todo!();
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{attr, has_class, html, said, tags, word};

    fn nothing() -> Callback<i32> {
        Callback::new(|_| {})
    }

    fn refused() -> Callback<String, bool> {
        Callback::new(|_| false)
    }

    /// A width field that `described_by` describes.
    fn width(described_by: Option<&'static str>) -> String {
        html(move || {
            view! {
                <Stepper less=word("Narrower") more=word("Wider") on_step=nothing()>
                    <NumberField
                        value=word("3.00")
                        unit=word("m")
                        commit=refused()
                        on_step=nothing()
                        id="width"
                        labelled_by="i-h-width"
                        described_by=Signal::derive(move || described_by.map(str::to_string))
                    />
                </Stepper>
            }
            .into_any()
        })
    }

    #[test]
    fn a_number_is_a_field_between_two_buttons_with_its_unit() {
        let h = width(Some("i-width-range"));
        let div = tags(&h, "div");
        assert!(h.starts_with("<div") && div.len() == 1 && has_class(div[0], "stepper"), "{h}");
        let buttons = tags(&h, "button");
        assert_eq!(buttons.iter().map(|b| attr(b, "aria-label")).collect::<Vec<_>>(), [Some("Narrower"), Some("Wider")], "{h}");
        for button in &buttons {
            assert!(has_class(button, "ico") && attr(button, "type") == Some("button"), "{button}");
        }
        // Between the buttons: the field, and its unit after it.
        let between = &h[h.find("</button>").unwrap()..h.rfind("<button").unwrap()];
        let field = tags(between, "span").into_iter().find(|s| has_class(s, "wfield")).unwrap_or_else(|| panic!("the field: {h}"));
        let inside = &between[between.find(field).unwrap()..];
        let input = tags(inside, "input").first().copied().unwrap_or_else(|| panic!("the input: {h}"));
        for (name, value) in [
            ("type", "text"),
            ("value", "3.00"),
            ("inputmode", "decimal"),
            ("autocomplete", "off"),
            ("id", "width"),
            ("aria-labelledby", "i-h-width"),
            ("aria-describedby", "i-width-range"),
        ] {
            assert_eq!(attr(input, name), Some(value), "{name}: {input}");
        }
        let after = &inside[inside.find(input).unwrap()..];
        let unit = tags(after, "span").into_iter().find(|s| has_class(s, "unit-tag")).unwrap_or_else(|| panic!("the unit: {h}"));
        assert_eq!(attr(unit, "aria-hidden"), Some("true"), "{unit}");
        assert!(after.contains(&format!("{unit}m</span>")), "{after}");
        assert!(!h.contains("role="), "with no group, no role: {h}");
    }

    #[test]
    fn a_field_described_by_nothing_has_no_description() {
        let h = width(None);
        assert_eq!(tags(&h, "input").len(), 1, "{h}");
        assert!(!h.contains("aria-describedby"), "{h}");
    }

    #[test]
    fn a_named_size_is_a_labelled_group_with_its_ends_aria_disabled() {
        let h = html(|| {
            view! {
                <Stepper
                    less=word("Smaller")
                    more=word("Larger")
                    on_step=nothing()
                    less_off=Signal::derive(|| true)
                    more_off=Signal::derive(|| false)
                    group="h"
                >
                    <span class="wfield turbo-size">"Mini"</span>
                </Stepper>
            }
            .into_any()
        });
        let div = tags(&h, "div")[0];
        assert!(has_class(div, "stepper"), "{div}");
        assert_eq!((attr(div, "role"), attr(div, "aria-labelledby")), (Some("group"), Some("h")), "{div}");
        assert_eq!(tags(&h, "button").iter().map(|b| attr(b, "aria-disabled")).collect::<Vec<_>>(), [Some("true"), None], "{h}");
        assert!(!h.contains(" disabled"), "a button at its end keeps the focus: {h}");
        let between = &h[h.find("</button>").unwrap()..h.rfind("<button").unwrap()];
        assert!(between.contains(r#"<span class="wfield turbo-size">Mini</span>"#), "{h}");
    }

    #[test]
    fn a_long_unit_is_marked_only_where_asked() {
        for (unit, long_unit, marked) in [("m across", true, true), ("m across", false, false), ("m", true, false), ("m", false, false)] {
            let h = html(move || {
                view! {
                    <Stepper less=word("Less") more=word("More") on_step=nothing() less_id="less" more_id="more">
                        <NumberField value=word("3") unit=word(unit) commit=refused() on_step=nothing() inputmode="numeric" long_unit=long_unit/>
                    </Stepper>
                }
                .into_any()
            });
            let span = tags(&h, "span").into_iter().find(|s| has_class(s, "unit-tag")).unwrap_or_else(|| panic!("the unit: {h}"));
            assert_eq!(has_class(span, "unit-long"), marked, "{unit}, {long_unit}: {span}");
            assert_eq!(tags(&h, "button").iter().map(|b| attr(b, "id")).collect::<Vec<_>>(), [Some("less"), Some("more")], "{h}");
            assert_eq!(attr(tags(&h, "input")[0], "inputmode"), Some("numeric"), "{h}");
        }
    }

    #[test]
    fn a_stepper_has_no_word_of_its_own() {
        let h = html(|| {
            view! {
                <Stepper less=word("") more=word("") on_step=nothing()>
                    <NumberField value=word("") unit=word("") commit=refused() on_step=nothing()/>
                </Stepper>
            }
            .into_any()
        });
        assert_eq!(said(&h), "", "{h}");
    }
}
