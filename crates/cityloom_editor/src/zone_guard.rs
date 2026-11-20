//! No message says "zone" of a street: a part of a street along its length is a stretch (a `tronçon`), and the
//! places that are zones by name keep the word.
//!
//! Every message of every slice, in both languages, is read in every variant of its selectors, then formatted with
//! sample values; the ids that still say "zone" are listed.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};

use crate::shared::i18n::{Args, Locale};

/// The places that are zones by name: "school zone" and "loading zone", and "zone scolaire". French says "zone de
/// chargement" for the loading zone (the glossary's term), "zone tampon" for a buffer (a piece across the street, not
/// a part along it), and names two kinds of area "zone sans voitures" (an area rule) and "zones à circulation
/// restreinte".
const ALLOWED: [&str; 8] = [
    "school zone",
    "loading zone",
    "zone scolaire",
    "zone de chargement",
    "zones de chargement",
    "zone tampon",
    "zone sans voitures",
    "zones à circulation restreinte",
];

/// French messages that say "zone" for an area of the map though their English one neither says "area" nor names
/// one, and why.
const AREA_MESSAGES: [(&str, &str); 1] = [("map-report-col-inside", "the area rules' table: the streets inside the area (\"Rues dans la zone\")")];

fn says(text: &str, word: &str) -> bool {
    let mut text = text.to_lowercase();
    for phrase in ALLOWED {
        text = text.replace(phrase, "");
    }
    text.contains(word)
}

fn words(p: &Pattern<&str>, text: &mut String, vars: &mut BTreeSet<&'static str>) {
    for el in &p.elements {
        match el {
            PatternElement::TextElement { value } => text.push_str(value),
            PatternElement::Placeable { expression } => expression_words(expression, text, vars),
        }
    }
}

fn inline_words(e: &InlineExpression<&str>, text: &mut String, vars: &mut BTreeSet<&'static str>) {
    match e {
        InlineExpression::StringLiteral { value } => text.push_str(value),
        InlineExpression::VariableReference { id } => {
            // `Args` takes a variable's name as `&'static str`.
            vars.insert(String::leak(id.name.to_string()));
        }
        InlineExpression::Placeable { expression } => expression_words(expression, text, vars),
        _ => {}
    }
}

fn expression_words(e: &Expression<&str>, text: &mut String, vars: &mut BTreeSet<&'static str>) {
    match e {
        Expression::Inline(i) => inline_words(i, text, vars),
        Expression::Select { selector, variants } => {
            inline_words(selector, text, vars);
            for v in variants {
                text.push(' ');
                words(&v.value, text, vars);
            }
        }
    }
}

/// A message as the guard reads it: its words in every variant, then formatted with sample values, and the names of
/// its variables.
struct Message {
    said: Vec<String>,
    vars: BTreeSet<&'static str>,
}

/// Every message of every slice in `locale`, by id.
fn messages(locale: Locale) -> BTreeMap<String, Message> {
    let i18n = crate::i18n_for(locale);
    let mut all = BTreeMap::new();
    for resources in crate::RESOURCES {
        let source = match locale {
            Locale::En => resources.en,
            Locale::FrCa => resources.fr,
        };
        let resource = fluent_syntax::parser::parse(source).expect("the messages parse");
        for entry in &resource.body {
            let Entry::Message(m) = entry else { continue };
            let Some(pattern) = &m.value else { continue };
            let (mut text, mut vars) = (String::new(), BTreeSet::new());
            words(pattern, &mut text, &mut vars);
            let mut said = vec![text];
            said.extend([1, 2, 5].map(|n| i18n.tr_now(m.id.name, &vars.iter().fold(Args::new(), |args, v| args.num(v, n)))));
            all.insert(m.id.name.to_string(), Message { said, vars });
        }
    }
    all
}

/// French says "zone" for an area of the map (a `règle de zone`, the glossary's term), so a French message may say it
/// where its English one speaks of an area: says "area", takes an `area`, or is one of `AREA_MESSAGES`.
#[test]
fn no_message_anywhere_says_zone_except_school_and_loading_zones() {
    let en = messages(Locale::En);
    let fr = messages(Locale::FrCa);
    let of_an_area = |id: &str| {
        AREA_MESSAGES.iter().any(|(area, _)| *area == id) || en.get(id).is_some_and(|m| m.vars.contains("area") || m.said.iter().any(|s| says(s, "area")))
    };
    let mut offending = Vec::new();
    for (id, m) in &en {
        if m.said.iter().any(|s| says(s, "zone")) {
            offending.push(format!("en: {id}"));
        }
    }
    for (id, m) in &fr {
        if !of_an_area(id) && m.said.iter().any(|s| says(s, "zone")) {
            offending.push(format!("fr-CA: {id}"));
        }
    }
    assert_eq!(offending, Vec::<String>::new());
}

#[test]
fn every_area_message_is_a_message() {
    let en = messages(Locale::En);
    for (id, _) in AREA_MESSAGES {
        assert!(en.contains_key(id), "{id} is no message");
    }
}
