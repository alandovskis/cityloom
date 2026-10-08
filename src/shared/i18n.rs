//! Words in two languages: the locale a person chose, the messages of each slice
//! in English and Canadian French, and how a message is looked up and typeset.
//! A slice keeps its own `.ftl` files and hands them over as `Resources`; this
//! module knows no slice.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use leptos::prelude::*;
use unic_langid::langid;

use crate::shared::ports::Ports;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    En,
    FrCa,
}

impl Locale {
    pub const ALL: [Locale; 2] = [Locale::En, Locale::FrCa];

    /// What is kept in storage and set as the document's `lang`.
    pub fn tag(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::FrCa => "fr-CA",
        }
    }

    /// Any French is Canadian French here; anything else is English.
    pub fn parse(tag: &str) -> Locale {
        let primary = tag.split(['-', '_']).next().unwrap_or("");
        if primary.eq_ignore_ascii_case("fr") { Locale::FrCa } else { Locale::En }
    }
}

/// One slice's messages, as the text of its two `.ftl` files.
#[derive(Clone, Copy)]
pub struct Resources {
    pub en: &'static str,
    pub fr: &'static str,
}

/// The named values a message is given.
#[derive(Default)]
pub struct Args(FluentArgs<'static>);

impl Args {
    pub fn new() -> Args {
        Args(FluentArgs::new())
    }

    pub fn num(mut self, name: &'static str, n: i64) -> Args {
        self.0.set(name, n);
        self
    }

    pub fn str(mut self, name: &'static str, s: impl Into<String>) -> Args {
        self.0.set(name, s.into());
        self
    }
}

type Bundle = FluentBundle<FluentResource>;

pub struct I18n {
    locale: ArcRwSignal<Locale>,
    en: Bundle,
    fr: Bundle,
    missing: RefCell<BTreeSet<String>>,
}

fn bundle(lang: unic_langid::LanguageIdentifier, texts: impl Iterator<Item = &'static str>) -> Bundle {
    let mut b = FluentBundle::new(vec![lang]);
    // The texts are compiled in and each slice's parity test (`parity_problems`) reports one that does not parse, so a parse failure here is a bug in the build.
    b.set_use_isolating(false);
    for text in texts {
        let res = FluentResource::try_new(text.to_string()).expect("a .ftl file does not parse");
        b.add_resource(res).expect("two .ftl files define the same message id");
    }
    b
}

impl I18n {
    pub fn new(locale: Locale, resources: &[Resources]) -> Rc<I18n> {
        Rc::new(I18n {
            locale: ArcRwSignal::new(locale),
            en: bundle(langid!("en"), resources.iter().map(|r| r.en)),
            fr: bundle(langid!("fr-CA"), resources.iter().map(|r| r.fr)),
            missing: RefCell::default(),
        })
    }

    /// The locale; a view that reads it is drawn again when it changes.
    pub fn locale(&self) -> Locale {
        self.locale.get()
    }

    /// The same, for a command: nothing is watched.
    pub fn locale_now(&self) -> Locale {
        self.locale.get_untracked()
    }

    pub fn set(&self, locale: Locale) {
        self.locale.set(locale);
    }

    /// The message `key` in the active locale, tracked.
    pub fn tr(&self, key: &str, args: &Args) -> String {
        self.say(self.locale(), key, args)
    }

    /// The same, for a command: nothing is watched.
    pub fn tr_now(&self, key: &str, args: &Args) -> String {
        self.say(self.locale_now(), key, args)
    }

    /// The keys that were asked for and not found in the active language, in order.
    pub fn missing(&self) -> Vec<String> {
        self.missing.borrow().iter().cloned().collect()
    }

    fn say(&self, locale: Locale, key: &str, args: &Args) -> String {
        let found = self.format(locale, key, args).or_else(|| {
            self.missing.borrow_mut().insert(key.to_string());
            (locale != Locale::En).then(|| self.format(Locale::En, key, args)).flatten()
        });
        typography(locale, &found.unwrap_or_else(|| key.to_string()))
    }

    fn format(&self, locale: Locale, key: &str, args: &Args) -> Option<String> {
        let b = if locale == Locale::En { &self.en } else { &self.fr };
        let pattern = b.get_message(key)?.value()?;
        let mut errors = Vec::new();
        let text = b.format_pattern(pattern, Some(&args.0), &mut errors).into_owned();
        if !errors.is_empty() {
            self.missing.borrow_mut().insert(key.to_string());
        }
        Some(text)
    }
}

/// Where the person's choice of language is kept.
pub const LANG_KEY: &str = "cityloom-lang";

/// A stored choice: `en` or any French; anything else is not one.
fn stored(tag: &str) -> Option<Locale> {
    let primary = tag.split(['-', '_']).next().unwrap_or("");
    (primary.eq_ignore_ascii_case("en") || primary.eq_ignore_ascii_case("fr")).then(|| Locale::parse(tag))
}

/// The language to start in: the one a person chose, else the browser's, else English.
pub fn detect(ports: &Ports) -> Locale {
    ports.storage.recall(LANG_KEY).and_then(|tag| stored(&tag)).or_else(|| ports.page.language().map(|tag| Locale::parse(&tag))).unwrap_or_default()
}

const NBSP: char = '\u{a0}';

/// Canadian French typography (the OQLF's): a non-breaking space before a colon
/// and inside « », none before ; ! ?. Translators write ordinary spaces.
pub fn typography(locale: Locale, text: &str) -> String {
    if locale == Locale::En {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        let after_open = i > 0 && chars[i - 1] == '\u{ab}';
        let nbsp = c == ' ' && (next == Some(':') || next == Some('\u{bb}') || after_open);
        out.push(if nbsp { NBSP } else { c });
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const R: Resources = Resources {
        en: "hello = Hello, { $name }.\nchecks = { $n ->\n    [one] One check\n   *[other] { $n } checks\n}\nonly-en = English only\nland = Langue : français\n",
        fr: "hello = Bonjour, { $name }.\nchecks = { $n ->\n    [one] { $n } vérification\n   *[other] { $n } vérifications\n}\nland = Langue : français\n",
    };

    use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};
    use fluent_syntax::parser::parse;
    use std::collections::BTreeMap;

    fn variables_in_inline(e: &InlineExpression<&str>, out: &mut BTreeSet<String>) {
        match e {
            InlineExpression::VariableReference { id } => {
                out.insert(id.name.to_string());
            }
            InlineExpression::Placeable { expression } => variables_in_expression(expression, out),
            InlineExpression::FunctionReference { arguments, .. } | InlineExpression::TermReference { arguments: Some(arguments), .. } => {
                for p in &arguments.positional {
                    variables_in_inline(p, out);
                }
                for n in &arguments.named {
                    variables_in_inline(&n.value, out);
                }
            }
            _ => {}
        }
    }

    fn variables_in_expression(e: &Expression<&str>, out: &mut BTreeSet<String>) {
        match e {
            Expression::Inline(i) => variables_in_inline(i, out),
            Expression::Select { selector, variants } => {
                variables_in_inline(selector, out);
                for v in variants {
                    variables_in_pattern(&v.value, out);
                }
            }
        }
    }

    fn variables_in_pattern(p: &Pattern<&str>, out: &mut BTreeSet<String>) {
        for el in &p.elements {
            if let PatternElement::Placeable { expression } = el {
                variables_in_expression(expression, out);
            }
        }
    }

    fn messages(text: &str, lang: &str, problems: &mut Vec<String>) -> BTreeMap<String, BTreeSet<String>> {
        let resource = match parse(text) {
            Ok(r) => r,
            Err((r, errors)) => {
                problems.push(format!("{lang} does not parse: {errors:?}"));
                r
            }
        };
        let mut map = BTreeMap::new();
        for entry in &resource.body {
            if let Entry::Message(m) = entry {
                let mut vars = BTreeSet::new();
                if let Some(p) = &m.value {
                    variables_in_pattern(p, &mut vars);
                }
                map.insert(m.id.name.to_string(), vars);
            }
        }
        map
    }

    /// What differs between the two languages of one slice: ids in one only, variables that differ,
    /// and a file that does not parse.
    pub(crate) fn parity_problems(en: &str, fr: &str) -> Vec<String> {
        let mut problems = Vec::new();
        let (e, f) = (messages(en, "en", &mut problems), messages(fr, "fr", &mut problems));
        for id in e.keys().filter(|k| !f.contains_key(*k)) {
            problems.push(format!("{id}: missing in fr"));
        }
        for id in f.keys().filter(|k| !e.contains_key(*k)) {
            problems.push(format!("{id}: missing in en"));
        }
        for (id, ev) in &e {
            if let Some(fv) = f.get(id).filter(|fv| *fv != ev) {
                let join = |s: &BTreeSet<String>| s.iter().cloned().collect::<Vec<_>>().join(", ");
                problems.push(format!("{id}: variables differ (en: {}; fr: {})", join(ev), join(fv)));
            }
        }
        problems
    }

    #[test]
    fn locale_parses_any_french_as_fr_ca_and_anything_else_as_english() {
        assert_eq!(Locale::parse("fr-CA"), Locale::FrCa);
        assert_eq!(Locale::parse("fr"), Locale::FrCa);
        assert_eq!(Locale::parse("fr-FR"), Locale::FrCa);
        assert_eq!(Locale::parse("en-US"), Locale::En);
        assert_eq!(Locale::parse("de"), Locale::En);
        assert_eq!(Locale::parse(""), Locale::En);
        assert_eq!(Locale::FrCa.tag(), "fr-CA");
        assert_eq!(Locale::En.tag(), "en");
    }

    #[test]
    fn a_message_is_looked_up_in_the_active_locale_with_named_arguments() {
        let i = I18n::new(Locale::En, &[R]);
        assert_eq!(i.tr_now("hello", &Args::new().str("name", "Ada")), "Hello, Ada.");
        i.set(Locale::FrCa);
        assert_eq!(i.tr_now("hello", &Args::new().str("name", "Ada")), "Bonjour, Ada.");
    }

    #[test]
    fn zero_and_one_are_singular_in_french_and_zero_is_plural_in_english() {
        let i = I18n::new(Locale::FrCa, &[R]);
        let n = |n| i.tr_now("checks", &Args::new().num("n", n));
        assert_eq!(n(0), "0 vérification");
        assert_eq!(n(1), "1 vérification");
        assert_eq!(n(2), "2 vérifications");
        i.set(Locale::En);
        assert_eq!(n(0), "0 checks");
        assert_eq!(n(1), "One check");
    }

    #[test]
    fn a_message_missing_in_french_falls_back_to_english_and_is_recorded() {
        let i = I18n::new(Locale::FrCa, &[R]);
        assert_eq!(i.tr_now("only-en", &Args::new()), "English only");
        assert_eq!(i.missing(), vec!["only-en".to_string()]);
    }

    #[test]
    fn a_key_in_no_language_returns_the_key_and_is_recorded() {
        let i = I18n::new(Locale::En, &[R]);
        assert_eq!(i.tr_now("nope", &Args::new()), "nope");
        assert_eq!(i.missing(), vec!["nope".to_string()]);
    }

    #[test]
    fn placeables_carry_no_isolation_marks() {
        let i = I18n::new(Locale::En, &[R]);
        let s = i.tr_now("hello", &Args::new().str("name", "Ada"));
        assert!(!s.contains('\u{2068}') && !s.contains('\u{2069}'));
    }

    #[test]
    fn french_gets_a_non_breaking_space_before_a_colon_and_none_before_other_marks() {
        assert_eq!(typography(Locale::FrCa, "Langue : français"), "Langue\u{a0}: français");
        assert_eq!(typography(Locale::FrCa, "Voilà ! Oui ? Non ; peut-être"), "Voilà ! Oui ? Non ; peut-être");
        assert_eq!(typography(Locale::FrCa, "Il dit « oui » ici"), "Il dit «\u{a0}oui\u{a0}» ici");
        assert_eq!(typography(Locale::FrCa, "à 06:30"), "à 06:30");
        assert_eq!(typography(Locale::En, "Language : English"), "Language : English");
    }

    #[test]
    fn two_resource_sets_with_the_same_id_are_refused() {
        let dup = Resources { en: "hello = x\n", fr: "hello = y\n" };
        let r = std::panic::catch_unwind(|| I18n::new(Locale::En, &[R, dup]));
        assert!(r.is_err());
    }

    #[test]
    fn the_locale_signal_notifies_a_watcher() {
        use leptos::prelude::*;
        let owner = Owner::new();
        owner.set();
        let i = I18n::new(Locale::En, &[R]);
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let (s, j) = (seen.clone(), StoredValue::new_local(i.clone()));
        // Effects run on a reactive executor; reading the tracked value inside a Memo is enough to prove tracking.
        let memo = Memo::new(move |_| j.with_value(|i| i.locale()));
        s.borrow_mut().push(memo.get());
        i.set(Locale::FrCa);
        s.borrow_mut().push(memo.get());
        assert_eq!(*seen.borrow(), vec![Locale::En, Locale::FrCa]);
    }

    #[test]
    fn parity_reports_a_message_in_one_language_only_and_a_variable_that_differs() {
        let en = "a = One\nb = Hi { $name }\nc = { $n ->\n    [one] x\n   *[other] { $n } y\n}\n";
        let fr = "a = Un\nb = Salut { $nom }\nd = Extra\n";
        let p = parity_problems(en, fr);
        assert!(p.contains(&"c: missing in fr".to_string()), "{p:?}");
        assert!(p.contains(&"d: missing in en".to_string()), "{p:?}");
        assert!(p.contains(&"b: variables differ (en: name; fr: nom)".to_string()), "{p:?}");
        assert!(parity_problems("a = x\n", "a = y\n").is_empty());
    }

    #[test]
    fn parity_reports_a_file_that_does_not_parse() {
        assert!(parity_problems("a = { \n", "a = x\n")[0].starts_with("en does not parse"));
    }

    use crate::shared::ports::{Storage, test_ports_with_page};

    #[test]
    fn the_stored_language_wins_over_the_browser_s() {
        let (ports, page, storage) = test_ports_with_page();
        storage.remember(LANG_KEY, "fr-CA");
        *page.language.borrow_mut() = Some("en-US".into());
        assert_eq!(detect(&ports), Locale::FrCa);
    }

    #[test]
    fn without_a_stored_language_the_browser_s_is_used_and_then_english() {
        let (ports, page, _) = test_ports_with_page();
        assert_eq!(detect(&ports), Locale::En);
        *page.language.borrow_mut() = Some("fr-CA".into());
        assert_eq!(detect(&ports), Locale::FrCa);
    }

    #[test]
    fn a_stored_value_that_is_not_a_language_is_ignored() {
        let (ports, page, storage) = test_ports_with_page();
        storage.remember(LANG_KEY, "de");
        *page.language.borrow_mut() = Some("fr".into());
        assert_eq!(detect(&ports), Locale::FrCa);
        storage.remember(LANG_KEY, "");
        assert_eq!(detect(&ports), Locale::FrCa);
    }
}
