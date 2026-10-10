//! What a model has to say, as data: the key of a message and the things it is about. A model has
//! no words and no language; `say` puts a `Said` into words in the language of the page, so a list
//! of what was changed follows a switch of language.
//!
//! The renderer is here, in the kernel, so any slice can use it. The message texts are in each
//! slice's `.ftl` files, all registered together by `crate::i18n_for`; the street slice owns the
//! keys a `Said` carries for now.

use serde::Serialize;

use crate::shared::atlas::name_key;
use crate::shared::catalogue::{curb_key, direction_key, kind_key, material_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::units::Units;

/// A time of day, `06:30`; the hour wraps at 24.
pub fn hhmm(min: i32) -> String {
    format!("{:02}:{:02}", (min / 60) % 24, min % 60)
}

/// One thing a message is about. The id arguments name entries of the catalogue; the view-model
/// turns each into the name it has in the language of the page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Arg {
    Num(i64),
    /// Words that are the person's or the data's own, said as they are.
    Text(String),
    /// A kind of piece, by catalogue id, named as it is at the start of a sentence.
    Kind(&'static str),
    /// A kind of piece, by catalogue id, named in the middle of a sentence.
    KindLower(&'static str),
    /// A surface material, by catalogue id, named in the middle of a sentence.
    MaterialLower(&'static str),
    /// A curb, by catalogue id, named in the middle of a sentence.
    CurbLower(&'static str),
    /// A direction of travel, by catalogue id, named in the middle of a sentence.
    DirectionLower(&'static str),
    /// An Atlas measure, by code, named as the Atlas names it.
    Measure(&'static str),
    /// A time of day, in minutes after midnight.
    Clock(i32),
    /// A length in millimetres, written in the units shown.
    Length(i32),
    /// Another message, by key, said in its place.
    Msg(&'static str),
    /// Another message, by key, said in the middle of a sentence (in lower case).
    MsgLower(&'static str),
    /// Another message with its own things, said in its place.
    Said(Box<Said>),
}

/// A message key with the things it is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Said {
    pub key: &'static str,
    pub args: Vec<(&'static str, Arg)>,
}

impl Said {
    /// A message that is about nothing in particular.
    pub fn new(key: &'static str) -> Said {
        Said { key, args: Vec::new() }
    }

    /// The same message, about one more thing.
    pub fn with(mut self, name: &'static str, arg: Arg) -> Said {
        self.args.push((name, arg));
        self
    }
}

/// What a model said, in words, for a view: it asks with `tr`, so it is drawn again when the
/// language is switched.
pub fn say(i18n: &I18n, units: Units, said: &Said) -> String {
    put_into_words(i18n, units, said, true)
}

/// The same, for a command: nothing is watched.
pub fn say_now(i18n: &I18n, units: Units, said: &Said) -> String {
    put_into_words(i18n, units, said, false)
}

fn put_into_words(i18n: &I18n, units: Units, said: &Said, watched: bool) -> String {
    let locale = if watched { i18n.locale() } else { i18n.locale_now() };
    let tr = |key: &str, args: &Args| if watched { i18n.tr(key, args) } else { i18n.tr_now(key, args) };
    let name = |key: String| tr(&key, &Args::new());
    let mut args = Args::new();
    for (arg_name, arg) in &said.args {
        args = match arg {
            Arg::Num(n) => args.num(arg_name, *n),
            Arg::Text(t) => args.str(arg_name, t.clone()),
            Arg::Kind(id) => args.str(arg_name, name(kind_key(id))),
            Arg::KindLower(id) => args.str(arg_name, name(kind_key(id)).to_lowercase()),
            Arg::MaterialLower(id) => args.str(arg_name, name(material_key(id)).to_lowercase()),
            Arg::CurbLower(id) => args.str(arg_name, name(curb_key(id)).to_lowercase()),
            Arg::DirectionLower(id) => args.str(arg_name, name(direction_key(id)).to_lowercase()),
            Arg::Measure(code) => args.str(arg_name, name(name_key(code))),
            Arg::Clock(min) => args.str(arg_name, hhmm(*min)),
            Arg::Length(mm) => args.str(arg_name, units.length_fine_in(*mm, locale)),
            Arg::Msg(key) => args.str(arg_name, tr(key, &Args::new())),
            Arg::MsgLower(key) => args.str(arg_name, tr(key, &Args::new()).to_lowercase()),
            Arg::Said(inner) => args.str(arg_name, put_into_words(i18n, units, inner, watched)),
        };
    }
    tr(said.key, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::i18n::Locale;

    use crate::shared::testing::{en, fr};

    #[test]
    fn a_time_of_day_is_written_hours_and_minutes_and_the_hour_wraps() {
        assert_eq!(hhmm(0), "00:00");
        assert_eq!(hhmm(390), "06:30");
        assert_eq!(hhmm(1_439), "23:59");
        assert_eq!(hhmm(1_440), "00:00");
    }

    #[test]
    fn each_kind_of_argument_is_put_into_words_in_both_languages() {
        let kind = Said::new("rev-add").with("kind", Arg::KindLower("bike"));
        assert_eq!((en(&kind), fr(&kind)), ("Add bike lane".into(), "Ajout\u{a0}: piste cyclable".into()));
        let material = Said::new("rev-surface").with("kind", Arg::Kind("parking")).with("material", Arg::MaterialLower("permeable"));
        assert_eq!((en(&material), fr(&material)), ("Parking surface: permeable paving".into(), "Stationnement, surface\u{a0}: revêtement perméable".into()));
        let curb = Said::new("rev-curb").with("kind", Arg::Kind("sidewalk")).with("curb", Arg::CurbLower("granite"));
        assert_eq!((en(&curb), fr(&curb)), ("Sidewalk curb: granite".into(), "Trottoir, bordure\u{a0}: granit".into()));
        let none = Said::new("rev-curb").with("kind", Arg::Kind("sidewalk")).with("curb", Arg::Msg("rev-word-none"));
        assert_eq!((en(&none), fr(&none)), ("Sidewalk curb: none".into(), "Trottoir, bordure\u{a0}: aucune".into()));
        let lower = Said::new("rev-curb").with("kind", Arg::Kind("sidewalk")).with("curb", Arg::MsgLower("kind-planting"));
        assert_eq!((en(&lower), fr(&lower)), ("Sidewalk curb: planting strip".into(), "Trottoir, bordure\u{a0}: bande plantée".into()));
        let way = Said::new("rev-direction").with("kind", Arg::Kind("travel")).with("direction", Arg::DirectionLower("away"));
        assert_eq!((en(&way), fr(&way)), ("Driving lane direction: away from you".into(), "Voie de circulation, sens\u{a0}: s’éloigne de vous".into()));
        let window = Said::new("rev-window")
            .with("base", Arg::Kind("parking"))
            .with("kind", Arg::KindLower("bus"))
            .with("from", Arg::Clock(420))
            .with("to", Arg::Clock(600));
        assert_eq!(en(&window), "Parking is transit lane 07:00-10:00");
        let measure = Said::new("rev-measure").with("code", Arg::Text("B1".into())).with("name", Arg::Measure("B1"));
        assert_eq!(en(&measure), "B1 Center-Running Transit Lanes");
    }

    #[test]
    fn a_length_is_written_with_the_decimal_mark_of_the_language_and_the_units_shown() {
        let over = Said::new("check-fits-over").with("amount", Arg::Length(1_800));
        assert_eq!(en(&over), "1.8 m too wide. Narrow or remove a piece.");
        assert_eq!(fr(&over), "1,8\u{a0}m de trop. Rétrécissez ou retirez un élément.");
        assert_eq!(say_now(&crate::i18n_for(Locale::En), Units::Feet, &over), "5.9 ft too wide. Narrow or remove a piece.");
    }

    #[test]
    fn a_said_can_be_an_argument_of_another() {
        // `edit-undone` takes `$fit`; the inner message is worded in the same language.
        let inner = Said::new("street-today");
        let outer = Said::new("edit-undone").with("fit", Arg::Said(Box::new(inner.clone())));
        assert_eq!(en(&outer), format!("Undone. {}.", en(&inner)));
        assert_eq!(fr(&outer), format!("Annulé. {}.", fr(&inner)));
        assert_ne!(en(&inner), fr(&inner));
    }

    #[test]
    fn a_number_and_the_person_s_own_words_are_passed_as_they_are() {
        let svg = Said::new("svg-too-wide").with("length", Arg::Text("2 m".into()));
        assert_eq!(en(&svg), "2 m too wide");
        let said = Said::new("failing").with("n", Arg::Num(2));
        assert_eq!((en(&said), fr(&said)), ("2 checks fail".into(), "2 vérifications échouent".into()));
    }
}
