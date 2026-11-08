//! The sources of a rule, drawn under it as a disclosure: the summary says whether the rule is a placeholder or has
//! named sources, and opening it shows where the rule applies and the references. A reference opens in a new tab, so
//! the person keeps the editing context they were in.

use leptos::prelude::*;

use crate::shared::i18n::{Args, I18n};
use crate::shared::sources::{Confidence, Note, note};
use crate::shared::units::Units;

/// The disclosure of the rule `rule`, worded in the page's language.
/// `check` is the words of the check the note sits under; it names the summary for a screen reader, since every
/// summary would otherwise read alike.
pub fn source_note(rule: &'static str, check: &str, i18n: &I18n, units: Units) -> impl IntoView + use<> {
    let n = note(rule, i18n, units);
    let name = i18n.tr("src-summary-for", &Args::new().str("summary", n.summary.clone()).str("check", check));
    disclosure(n, name, i18n.tr("src-open", &Args::new()))
}

/// The disclosure of a note already worded: `name` is the summary's name for a screen reader, `open` the words of a link.
fn disclosure(n: Note, name: String, open: String) -> impl IntoView + use<> {
    let placeholder = n.confidence == Confidence::Placeholder;
    let sources = n
        .sources
        .into_iter()
        .map(|s| {
            let link = s.url.map(|href| view! { " " <a href=href target="_blank" rel="noopener noreferrer" aria-label=s.open.clone()>{open.clone()}</a> });
            view! { <li>{s.text}{link}</li> }
        })
        .collect_view();
    view! {
        <details class="src" class:placeholder=placeholder>
            <summary aria-label=name>{n.summary}</summary>
            <p>{n.applies}</p>
            <ul>{sources}</ul>
        </details>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;
    use crate::shared::testing::html;

    #[test]
    fn a_placeholder_rule_is_marked_and_lists_no_reference() {
        let h = html(|| source_note("calming-length", "Check", &crate::i18n_for(Locale::En), Units::Metres).into_any());
        assert!(
            h.contains("class=\"src placeholder\"")
                && h.contains("Source: placeholder, no source")
                && h.contains("aria-label=\"Source: placeholder, no source (Check)\"")
        );
        assert!(!h.contains("<a "));
    }

    #[test]
    fn a_cited_rule_says_its_status_and_names_its_references() {
        let h = html(|| source_note("transition-room", "Check", &crate::i18n_for(Locale::En), Units::Metres).into_any());
        assert!(h.contains("class=\"src\"") && h.contains("Source: named, figures not checked"));
        assert!(h.contains("Regional Municipality of York"));
    }

    #[test]
    fn a_reference_with_a_link_opens_in_a_new_tab_and_the_name_says_so() {
        use crate::shared::sources::NoteSource;
        let note = Note {
            confidence: Confidence::Unchecked,
            summary: "Source: named".into(),
            applies: "Applies here.".into(),
            sources: vec![NoteSource {
                text: "Publisher, Title".into(),
                url: Some("https://example.org/doc"),
                open: "Open reference: Title (opens in a new tab)".into(),
            }],
        };
        let h = html(|| disclosure(note, "Source: named (Check)".into(), "Open reference".into()).into_any());
        assert!(h.contains("href=\"https://example.org/doc\"") && h.contains("target=\"_blank\"") && h.contains("rel=\"noopener noreferrer\""));
        assert!(h.contains(">Open reference<") && h.contains("aria-label=\"Open reference: Title (opens in a new tab)\""));
    }
}
