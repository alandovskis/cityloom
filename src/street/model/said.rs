//! What the model has to say, as data: the key of a message and the things it is about. The
//! model has no words and no language; `street::text::say` puts a `Said` into words in the
//! language of the page, so a list of what was changed follows a switch of language.

use serde::Serialize;

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
