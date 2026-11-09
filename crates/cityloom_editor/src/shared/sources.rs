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
    /// The message that names where it applies (`src-jurisdiction-ca`, `-qc` or `-mtl`).
    pub jurisdiction: &'static str,
    /// A page that is known to hold it; `None` where no address has been confirmed.
    pub url: Option<&'static str>,
}

const fn doc(id: &'static str, title: &'static str, publisher: &'static str, jurisdiction: &'static str, url: Option<&'static str>) -> Source {
    Source { id, title, publisher, jurisdiction, url }
}

const MTQ: &str = "Ministère des Transports du Québec";
const QC: &str = "src-jurisdiction-qc";
const MTL: &str = "src-jurisdiction-mtl";
const VILLE: &str = "Ville de Montréal";

/// A sheet of the Ville de Montréal's Répertoire des pratiques d'aménagement de la rue, by its name and its path on the portal.
macro_rules! mtl {
    ($id:literal, $sheet:literal, $path:literal) => {
        doc(
            $id,
            concat!("Répertoire des pratiques d'aménagement de la rue, ", $sheet),
            VILLE,
            MTL,
            Some(concat!("https://montreal-amenagement-rues.powerappsportals.com", $path)),
        )
    };
}

pub const SOURCES: &[Source] = &[
    doc(
        "york-ds-100",
        "Design Standard DS-100, 2023 (lane-addition tapers, based on the TAC guide)",
        "Regional Municipality of York",
        "src-jurisdiction-ca",
        None,
    ),
    doc("mtq-t1-c1", "Conception routière, Tome I, chapitre 1, Classification fonctionnelle (2021-06-15)", MTQ, QC, None),
    doc("mtq-t1-c7", "Conception routière, Tome I, chapitre 7, Distance de visibilité (2024-06-15)", MTQ, QC, None),
    doc("mtq-t1-c8", "Conception routière, Tome I, chapitre 8, Carrefours plans (2026-06-15)", MTQ, QC, None),
    doc("mtq-t1-c14", "Conception routière, Tome I, chapitre 14, Stationnement (2026-06-15)", MTQ, QC, None),
    doc(
        "mtq-t1-d14",
        "Conception routière, Tome I, chapitre 14, dessins normalisés 001 et 002, Stationnement sur rue en parallèle (2006-06-15)",
        MTQ,
        QC,
        None,
    ),
    doc("mtl-repertoire", "Répertoire des pratiques d'aménagement de la rue", VILLE, MTL, Some("https://montreal-amenagement-rues.powerappsportals.com/")),
    mtl!("mtl-velorue", "fiche Vélorue", "/Types/Velorue/"),
    mtl!("mtl-arret-bus", "fiche Arrêt d'autobus en bordure d'une voie cyclable", "/Composantes/Arr%C3%AAt-bus-voie-cyclable/"),
    mtl!("mtl-avancee", "fiche Avancées de trottoir", "/Composantes/Avanc%C3%A9es-de-trottoir/"),
    mtl!("mtl-cafe", "fiche Café-terrasse et placottoir", "/Composantes/Caf%C3%A9-terrasse-et-placottoir/"),
    mtl!("mtl-ilot", "fiche Îlot refuge", "/Composantes/ilot-refuge/"),
    mtl!("mtl-traversant", "fiche Trottoir et piste cyclable traversants", "/Composantes/Trottoir-traversant/"),
    mtl!("mtl-corridor", "outil de conception Largeur des corridors piétons", "/Outils/Corridor-pi%C3%A9ton/"),
    mtl!("mtl-degagement", "outil de conception Dégagement visuel minimal aux intersections", "/Outils/D%C3%A9gagement-visuel-minimal-aux-intersections/"),
    mtl!("mtl-plantations", "outil de conception Hauteur des plantations", "/Outils/Hauteur-des-plantations/"),
    mtl!("mtl-voies", "outil de conception Largeur de voies", "/Outils/Conception---Largeur-de-voies/"),
    doc("mtq-t1-c15", "Conception routière, Tome I, chapitre 15, Voies cyclables (2026-06-15)", MTQ, QC, None),
    doc("tpa-toolbox", "Transit Priority Atlas, toolbox", "Marron Institute", "src-jurisdiction-none", Some("https://tpa.transitcosts.com/atlas/toolbox")),
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

const fn entry(rule: &'static str, label: &'static str, applies: &'static str, sources: &'static [&'static str]) -> Entry {
    Entry { rule, label, applies, sources }
}

const ENTRIES: &[Entry] = &[
    entry("transition-room", "check-label-transition-room", "src-applies-transition", &["york-ds-100"]),
    entry("lane-drop", "check-label-lane-drop", "src-applies-transition", &["york-ds-100"]),
    entry("curb-length", "check-label-curb-length", "src-applies-stall-length", &["mtq-t1-d14"]),
    entry("calming-passage", "check-label-calming-passage", "src-applies-diverter", &["mtl-velorue"]),
    entry("curb-bike", "check-label-curb-bike", "src-applies-door-buffer", &["mtq-t1-c15", "mtl-velorue", "mtl-voies"]),
    entry("curb-crossing", "check-label-curb-crossing", "src-applies-parking-setback", &["mtq-t1-c14", "mtq-t1-d14", "mtl-degagement"]),
    entry("curb-near", "jn-check-curb-near", "src-applies-parking-setback", &["mtq-t1-c14", "mtq-t1-d14", "mtl-degagement"]),
    entry("crossing", "jn-check-crossing", "src-applies-refuge", &["mtq-t1-c8", "mtl-ilot"]),
    entry("green-sightline", "check-label-green-sightline", "src-applies-sight-triangle", &["mtq-t1-c7", "mtl-degagement", "mtl-plantations", "mtl-avancee"]),
    entry("curb-sidewalk", "check-label-curb-sidewalk", "src-applies-patio-sidewalk", &["mtl-cafe"]),
    entry("stop-length", "check-label-stop-length", "src-applies-stop-length", &["mtl-arret-bus"]),
    entry("stop-width", "check-label-stop-width", "src-applies-stop-width", &["mtl-arret-bus"]),
    entry("stop-bike", "check-label-stop-bike", "src-applies-stop-bike", &["mtl-arret-bus"]),
    entry("green-clear-path", "check-label-green-clear-path", "src-applies-clear-path", &["mtl-corridor"]),
    entry("access", "check-label-access", "src-applies-pedestrian-access", &["mtl-corridor"]),
    entry("calming-length", "check-label-calming-length", "src-applies-raised-crossing", &["mtl-traversant"]),
    entry("atlas-measure", "st-transit-heading", "src-applies-atlas", &["tpa-toolbox"]),
    entry("atlas-junction", "src-atlas-junction-label", "src-applies-atlas-junction", &["tpa-toolbox"]),
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
        let c = citation("green-spacing");
        assert_eq!(c.confidence, Confidence::Placeholder);
        assert!(c.sources.is_empty());
        assert_eq!(c.applies, Said::new("src-applies-placeholder"));
    }

    #[test]
    fn a_rule_with_an_entry_names_its_sources_and_says_they_are_not_checked() {
        let c = citation("transition-room");
        assert_eq!(c.confidence, Confidence::Unchecked);
        assert_eq!(c.sources.len(), 1);
        assert_eq!(c.sources[0].publisher, "Regional Municipality of York");
        assert_eq!(c.sources[0].jurisdiction, Said::new("src-jurisdiction-ca"));
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
                ["src-jurisdiction-ca", "src-jurisdiction-qc", "src-jurisdiction-mtl", "src-jurisdiction-none"].contains(&s.jurisdiction),
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
            let placeholder = note("green-spacing", &i18n, Units::Metres);
            assert!(placeholder.sources.is_empty() && !placeholder.summary.is_empty() && !placeholder.applies.is_empty());
            let cited = note("lane-drop", &i18n, Units::Metres);
            assert_eq!(cited.sources.len(), 1);
            assert!(cited.sources[0].text.contains("Regional Municipality of York"));
            assert!(cited.sources[0].open.contains("Design Standard DS-100"));
            assert!(i18n.missing().is_empty(), "{:?}", i18n.missing());
        }
        let en = crate::i18n_for(crate::shared::i18n::Locale::En);
        assert_eq!(note("green-spacing", &en, Units::Metres).summary, "Source: placeholder, no source");
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

    #[test]
    fn the_atlas_measures_cite_the_toolbox_as_the_source_of_their_names_only() {
        let c = citation("atlas-measure");
        assert_eq!(c.confidence, Confidence::Unchecked);
        assert_eq!(c.applies, Said::new("src-applies-atlas"));
        assert_eq!(c.sources.len(), 1);
        assert_eq!((c.sources[0].publisher, c.sources[0].url), ("Marron Institute", Some("https://tpa.transitcosts.com/atlas/toolbox")));
    }

    #[test]
    fn a_rule_the_ville_de_montreal_speaks_to_links_its_sheet() {
        let c = citation("calming-passage");
        assert_eq!(c.confidence, Confidence::Unchecked);
        assert_eq!(c.sources.len(), 1);
        assert_eq!(c.sources[0].publisher, "Ville de Montréal");
        assert_eq!(c.sources[0].url, Some("https://montreal-amenagement-rues.powerappsportals.com/Types/Velorue/"));
        assert_eq!(c.sources[0].jurisdiction, Said::new("src-jurisdiction-mtl"));
        assert!(citation("curb-bike").sources.iter().any(|s| s.publisher == "Ville de Montréal"));
    }

    #[test]
    fn the_checks_the_ville_de_montreal_speaks_to_cite_its_sheet_with_a_link() {
        for (rule, sheet) in [
            ("curb-crossing", "Dégagement visuel"),
            ("curb-near", "Dégagement visuel"),
            ("curb-bike", "Largeur de voies"),
            ("curb-sidewalk", "Café-terrasse"),
            ("green-sightline", "Hauteur des plantations"),
            ("green-clear-path", "corridors piétons"),
            ("access", "corridors piétons"),
            ("crossing", "Îlot refuge"),
            ("stop-length", "Arrêt d'autobus"),
            ("stop-width", "Arrêt d'autobus"),
            ("stop-bike", "Arrêt d'autobus"),
            ("calming-length", "Trottoir et piste cyclable traversants"),
        ] {
            let c = citation(rule);
            assert_eq!(c.confidence, Confidence::Unchecked, "{rule}");
            let s = c.sources.iter().find(|s| s.title.contains(sheet)).unwrap_or_else(|| panic!("{rule} does not cite {sheet}"));
            assert!(s.url.is_some_and(|u| u.starts_with("https://montreal-amenagement-rues.powerappsportals.com/")), "{rule}");
        }
    }
}
