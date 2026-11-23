//! A section of the details.

use leptos::prelude::*;

/// A heading and its contents.
#[component]
pub fn Section(
    /// The heading's id.
    #[prop(into)]
    id: String,
    heading: Signal<String>,
    /// Whether it spans both columns of the details.
    #[prop(optional)]
    span: bool,
    /// Whether the heading labels the section.
    #[prop(optional)]
    labelled: bool,
    children: Children,
) -> impl IntoView {
    let view: AnyView = todo!();
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{attr, has_class, html, said, tags, word};

    #[test]
    fn a_section_is_its_heading_then_its_contents() {
        let h = html(|| view! { <Section id="i" heading=word("Heading")><p>"Body"</p></Section> }.into_any());
        let start = r#"<section class="insp-sec"><h3 id="i" class="note-h">Heading</h3>"#;
        let contents = h.strip_prefix(start).unwrap_or_else(|| panic!("{h}"));
        assert!(contents.starts_with("<p>Body</p>"), "{h}");
        assert!(contents.ends_with("</section>"), "{h}");
    }

    #[test]
    fn a_section_that_spans_both_columns_or_is_labelled_says_so() {
        let of = |span: bool, labelled: bool| {
            let h = html(move || view! { <Section id="i" heading=word("Heading") span=span labelled=labelled>"Body"</Section> }.into_any());
            tags(&h, "section")[0].to_string()
        };
        let wide = of(true, false);
        assert!(has_class(&wide, "insp-sec") && has_class(&wide, "insp-span"), "{wide}");
        assert_eq!(attr(&wide, "aria-labelledby"), None, "{wide}");
        let labelled = of(false, true);
        assert_eq!((attr(&labelled, "aria-labelledby"), attr(&labelled, "class")), (Some("i"), Some("insp-sec")), "{labelled}");
        assert_eq!(of(false, false), r#"<section class="insp-sec">"#);
    }

    #[test]
    fn a_section_has_no_word_of_its_own() {
        let h = html(|| view! { <Section id="i" heading=word("") span=true labelled=true><p></p></Section> }.into_any());
        assert_eq!(said(&h), "", "{h}");
    }
}
