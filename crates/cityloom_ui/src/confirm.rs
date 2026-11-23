//! A question asked before something is done.

use leptos::prelude::*;

/// A question and its buttons (`children`): a group that `labelled_by` names, where Escape answers no.
#[component]
pub fn Confirm(
    #[prop(into)] labelled_by: String,
    #[prop(optional, into)] described_by: Option<String>,
    #[prop(optional, into)] id: Option<String>,
    on_escape: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            role="group"
            class="confirm"
            id=id
            aria-labelledby=labelled_by
            aria-describedby=described_by
            on:keydown=move |e| {
                if e.key() == "Escape" {
                    // Not a key of what the question is asked in: Escape only answers it.
                    e.stop_propagation();
                    on_escape.run(());
                }
            }
        >
            {children()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{attr, has_class, html, said, tags};

    fn no() -> Callback<()> {
        Callback::new(|_| {})
    }

    #[test]
    fn a_question_is_a_labelled_group_holding_its_contents() {
        let h = html(|| view! { <Confirm labelled_by="q" on_escape=no()><p id="q">"Delete it?"</p><button>"No"</button></Confirm> }.into_any());
        assert!(h.starts_with("<div"), "{h}");
        let div = tags(&h, "div")[0];
        assert!(has_class(div, "confirm"), "{div}");
        assert_eq!((attr(div, "role"), attr(div, "aria-labelledby")), (Some("group"), Some("q")), "{div}");
        assert_eq!((attr(div, "aria-describedby"), attr(div, "id")), (None, None), "{div}");
        let inside = h.strip_prefix(div).unwrap().strip_suffix("</div>").unwrap_or_else(|| panic!("{h}"));
        assert!(inside.contains(r#"<p id="q">Delete it?</p><button>No</button>"#), "{inside}");
        // Described, with an id, only when it is given them.
        let h = html(|| view! { <Confirm labelled_by="q" described_by="w" id="stop-confirm" on_escape=no()>"Delete it?"</Confirm> }.into_any());
        let div = tags(&h, "div")[0];
        assert_eq!((attr(div, "aria-describedby"), attr(div, "id")), (Some("w"), Some("stop-confirm")), "{div}");
    }

    #[test]
    fn a_question_has_no_word_of_its_own() {
        let h = html(|| view! { <Confirm labelled_by="q" described_by="w" id="c" on_escape=no()><p></p></Confirm> }.into_any());
        assert_eq!(said(&h), "", "{h}");
    }
}
