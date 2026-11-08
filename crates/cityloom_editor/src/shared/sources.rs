//! The sources behind the design rules (#78): one table, read by every view and by the report, so a citation is
//! written once. A rule is named by its check id (`transition-room`, `crossing`, ...); the street and the junction page
//! share one namespace, so an id used by both pages (`stop-lane` is) must not be given an entry. A rule with no entry is a
//! synthetic placeholder, and says so: the registry never answers "nothing", so a rule cannot be shown unmarked.
//!
//! Every figure in the catalogue is still UNVERIFIED (see CLAUDE.md). An entry that names a source records where a
//! formula was taken from, not that CityLoom has checked its figures against it; that is `Confidence::Unchecked`.
//! `Confidence` gains a `Verified` the day a figure is checked against its source.

use crate::shared::i18n::{Args, I18n};
use crate::shared::said::{Said, say, say_now};
use crate::shared::units::Units;

/// How far a rule can be trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    /// A figure of CityLoom's own, with no source: it stands in until an authoritative one replaces it.
    Placeholder,
    /// A source is named, and the figures have not been checked against it.
    Unchecked,
}

impl Confidence {
    /// The message that names it.
    pub fn key(self) -> &'static str {
        match self {
            Confidence::Placeholder => "src-confidence-placeholder",
            Confidence::Unchecked => "src-confidence-unchecked",
        }
    }
}

/// A document a rule is taken from. Titles and publishers are proper names and are not translated.
#[derive(Debug, PartialEq, Eq)]
pub struct Source {
    pub id: &'static str,
    pub title: &'static str,
    pub publisher: &'static str,
    /// The message that names where it applies (`src-jurisdiction-us`, `-ca` or `-qc`).
    pub jurisdiction: &'static str,
    /// A page that is known to hold it; `None` where no address has been confirmed.
    pub url: Option<&'static str>,
}

pub const SOURCES: &[Source] = &[
    Source {
        id: "mutcd-2003",
        title: "Manual on Uniform Traffic Control Devices, 2003 edition, Part 3, Figure 3B-12",
        publisher: "Federal Highway Administration",
        jurisdiction: "src-jurisdiction-us",
        url: Some("https://mutcd.fhwa.dot.gov/pdfs/2003r1/pdf_index.htm"),
    },
    Source {
        id: "york-ds-100",
        title: "Design Standard DS-100, 2023 (lane-addition tapers, based on the TAC guide)",
        publisher: "Regional Municipality of York",
        jurisdiction: "src-jurisdiction-ca",
        url: None,
    },
    Source {
        id: "nacto-bikeway",
        title: "Urban Bikeway Design Guide (bike lane tapers of at least 1:5)",
        publisher: "National Association of City Transportation Officials",
        jurisdiction: "src-jurisdiction-us",
        url: Some("https://nacto.org/publication/urban-bikeway-design-guide/"),
    },
    Source {
        id: "mtq-t1-c1",
        title: "Conception routière, Tome I, chapitre 1, Classification fonctionnelle (2021-06-15)",
        publisher: "Ministère des Transports du Québec",
        jurisdiction: "src-jurisdiction-qc",
        url: None,
    },
    Source {
        id: "mtq-t1-c7",
        title: "Conception routière, Tome I, chapitre 7, Distance de visibilité (2024-06-15)",
        publisher: "Ministère des Transports du Québec",
        jurisdiction: "src-jurisdiction-qc",
        url: None,
    },
    Source {
        id: "mtq-t1-c8",
        title: "Conception routière, Tome I, chapitre 8, Carrefours plans (2026-06-15)",
        publisher: "Ministère des Transports du Québec",
        jurisdiction: "src-jurisdiction-qc",
        url: None,
    },
    Source {
        id: "mtq-t1-c14",
        title: "Conception routière, Tome I, chapitre 14, Stationnement (2026-06-15)",
        publisher: "Ministère des Transports du Québec",
        jurisdiction: "src-jurisdiction-qc",
        url: None,
    },
    Source {
        id: "mtq-t1-d14",
        title: "Conception routière, Tome I, chapitre 14, dessins normalisés 001 et 002, Stationnement sur rue en parallèle (2006-06-15)",
        publisher: "Ministère des Transports du Québec",
        jurisdiction: "src-jurisdiction-qc",
        url: None,
    },
    Source {
        id: "mtq-t1-c15",
        title: "Conception routière, Tome I, chapitre 15, Voies cyclables (2026-06-15)",
        publisher: "Ministère des Transports du Québec",
        jurisdiction: "src-jurisdiction-qc",
        url: None,
    },
];

/// What is known of one rule.
struct Entry {
    rule: &'static str,
    /// The message that names the rule.
    label: &'static str,
    /// The message that says where the rule applies.
    applies: &'static str,
    sources: &'static [&'static str],
}

const ENTRIES: &[Entry] = &[
    Entry {
        rule: "transition-room",
        label: "check-label-transition-room",
        applies: "src-applies-transition",
        sources: &["mutcd-2003", "york-ds-100", "nacto-bikeway"],
    },
    Entry { rule: "lane-drop", label: "check-label-lane-drop", applies: "src-applies-transition", sources: &["mutcd-2003", "york-ds-100"] },
    Entry { rule: "curb-length", label: "check-label-curb-length", applies: "src-applies-stall-length", sources: &["mtq-t1-d14"] },
    Entry { rule: "curb-bike", label: "check-label-curb-bike", applies: "src-applies-door-buffer", sources: &["mtq-t1-c15"] },
    Entry { rule: "curb-crossing", label: "check-label-curb-crossing", applies: "src-applies-parking-setback", sources: &["mtq-t1-c14", "mtq-t1-d14"] },
    Entry { rule: "curb-near", label: "jn-check-curb-near", applies: "src-applies-parking-setback", sources: &["mtq-t1-c14", "mtq-t1-d14"] },
    Entry { rule: "crossing", label: "jn-check-crossing", applies: "src-applies-refuge", sources: &["mtq-t1-c8"] },
    Entry { rule: "green-sightline", label: "check-label-green-sightline", applies: "src-applies-sight-triangle", sources: &["mtq-t1-c7"] },
];

/// The rule's source as a view reads it.
#[derive(Debug, PartialEq)]
pub struct SourceView {
    pub title: &'static str,
    pub publisher: &'static str,
    pub jurisdiction: Said,
    pub url: Option<&'static str>,
}

/// Everything a view or the report says about a rule's sources, as data.
#[derive(Debug, PartialEq)]
pub struct Citation {
    pub rule: &'static str,
    pub confidence: Confidence,
    pub applies: Said,
    pub sources: Vec<SourceView>,
}

/// The rules that name a source, each with the message that names it: what the report's Sources page lists.
pub fn cited() -> impl Iterator<Item = (&'static str, &'static str)> {
    ENTRIES.iter().map(|e| (e.rule, e.label))
}

fn source(id: &str) -> &'static Source {
    SOURCES.iter().find(|s| s.id == id).unwrap_or_else(|| panic!("the source {id:?} is not in SOURCES"))
}

/// The citation of a rule: its sources and where it applies, or the placeholder notice when none is recorded.
pub fn citation(rule: &'static str) -> Citation {
    match ENTRIES.iter().find(|e| e.rule == rule) {
        Some(e) => Citation {
            rule,
            confidence: Confidence::Unchecked,
            applies: Said::new(e.applies),
            sources: e
                .sources
                .iter()
                .map(|id| {
                    let s = source(id);
                    SourceView { title: s.title, publisher: s.publisher, jurisdiction: Said::new(s.jurisdiction), url: s.url }
                })
                .collect(),
        },
        None => Citation { rule, confidence: Confidence::Placeholder, applies: Said::new("src-applies-placeholder"), sources: Vec::new() },
    }
}

/// One source in words.
#[derive(Debug, PartialEq, Eq)]
pub struct NoteSource {
    /// Publisher, title and jurisdiction.
    pub text: String,
    pub url: Option<&'static str>,
    /// What the link is called for a screen reader, which says that it opens another tab.
    pub open: String,
}

/// A rule's sources in the page's language, ready to draw. Whatever draws it (a view, the report) reads this.
#[derive(Debug, PartialEq, Eq)]
pub struct Note {
    pub confidence: Confidence,
    /// `Source: placeholder, no source`.
    pub summary: String,
    pub applies: String,
    pub sources: Vec<NoteSource>,
}

/// The words of a rule's citation; watched, so a view is drawn again when the language changes.
pub fn note(rule: &'static str, i18n: &I18n, units: Units) -> Note {
    words(rule, &|key, args| i18n.tr(key, args), &|said| say(i18n, units, said))
}

/// The same, for a command (the report): nothing is watched.
pub fn note_now(rule: &'static str, i18n: &I18n, units: Units) -> Note {
    words(rule, &|key, args| i18n.tr_now(key, args), &|said| say_now(i18n, units, said))
}

fn words(rule: &'static str, tr: &dyn Fn(&str, &Args) -> String, said: &dyn Fn(&Said) -> String) -> Note {
    let c = citation(rule);
    let status = tr(c.confidence.key(), &Args::new());
    Note {
        confidence: c.confidence,
        summary: tr("src-summary", &Args::new().str("status", status)),
        applies: said(&c.applies),
        sources: c
            .sources
            .iter()
            .map(|s| NoteSource {
                text: tr("src-source", &Args::new().str("publisher", s.publisher).str("title", s.title).str("jurisdiction", said(&s.jurisdiction))),
                url: s.url,
                open: tr("src-open-label", &Args::new().str("title", s.title)),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_without_an_entry_is_a_placeholder_with_no_source() {
        let c = citation("calming-length");
        assert_eq!(c.confidence, Confidence::Placeholder);
        assert!(c.sources.is_empty());
        assert_eq!(c.applies, Said::new("src-applies-placeholder"));
    }

    #[test]
    fn a_rule_with_an_entry_names_its_sources_and_says_they_are_not_checked() {
        let c = citation("transition-room");
        assert_eq!(c.confidence, Confidence::Unchecked);
        assert_eq!(c.sources.len(), 3);
        assert_eq!(c.sources[0].publisher, "Federal Highway Administration");
        assert_eq!(c.sources[0].jurisdiction, Said::new("src-jurisdiction-us"));
    }

    #[test]
    fn every_entry_cites_sources_that_exist_and_is_listed_once() {
        for (i, e) in ENTRIES.iter().enumerate() {
            assert!(!e.sources.is_empty(), "{} names no source: leave it out and it is a placeholder", e.rule);
            for id in e.sources {
                assert!(SOURCES.iter().any(|s| s.id == *id), "{} cites {id}, which is not in SOURCES", e.rule);
            }
            assert!(ENTRIES[..i].iter().all(|o| o.rule != e.rule), "{} is listed twice", e.rule);
        }
        for (i, s) in SOURCES.iter().enumerate() {
            assert!(SOURCES[..i].iter().all(|o| o.id != s.id), "{} is listed twice", s.id);
            assert!(
                ["src-jurisdiction-us", "src-jurisdiction-ca", "src-jurisdiction-qc"].contains(&s.jurisdiction),
                "{}: unknown jurisdiction {}",
                s.id,
                s.jurisdiction
            );
            assert!(s.url.is_none_or(|u| u.starts_with("https://")), "{}: a link must be https", s.id);
        }
    }

    #[test]
    fn a_note_is_worded_in_both_languages_and_a_placeholder_says_so() {
        for locale in [crate::shared::i18n::Locale::En, crate::shared::i18n::Locale::FrCa] {
            let i18n = crate::i18n_for(locale);
            let placeholder = note("calming-length", &i18n, Units::Metres);
            assert!(placeholder.sources.is_empty() && !placeholder.summary.is_empty() && !placeholder.applies.is_empty());
            let cited = note("lane-drop", &i18n, Units::Metres);
            assert_eq!(cited.sources.len(), 2);
            assert!(cited.sources[0].text.contains("Federal Highway Administration"));
            assert!(cited.sources[0].url.is_some() && cited.sources[0].open.contains("Manual on Uniform"));
            assert!(i18n.missing().is_empty(), "{:?}", i18n.missing());
        }
        let en = crate::i18n_for(crate::shared::i18n::Locale::En);
        assert_eq!(note("calming-length", &en, Units::Metres).summary, "Source: placeholder, no source");
    }

    #[test]
    fn the_checks_the_mtq_speaks_to_cite_the_chapter_they_were_read_in() {
        for (rule, chapter) in [
            ("curb-bike", "chapitre 15"),
            ("curb-crossing", "chapitre 14"),
            ("curb-near", "chapitre 14"),
            ("crossing", "chapitre 8"),
            ("green-sightline", "chapitre 7"),
        ] {
            let c = citation(rule);
            assert_eq!(c.confidence, Confidence::Unchecked, "{rule}");
            assert!(!c.sources.is_empty() && c.sources[0].title.contains(chapter) && c.sources[0].publisher.contains("Transports du Québec"), "{rule}");
        }
    }
}
