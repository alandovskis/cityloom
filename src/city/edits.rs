//! The document the resident's edits are kept in: one for the whole metro area, its records keyed by the keys of
//! `city::keys`, so an edit follows its place whichever area or tile the place is read from.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::junction::model::State;
use crate::street::model::Street;

/// The shape of the document. Raised when the shape changes.
pub const VERSION: u32 = 3;
/// Where the document is kept.
pub const EDITS_KEY: &str = "cityloom-edits";
/// Where a document that could not be read is copied before the first write replaces it.
pub const DAMAGED_KEY: &str = "cityloom-edits-damaged";

/// An edited street: what it was made from (for matching it again) and the street itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreetRecord {
    /// The OpenStreetMap ways the road was made from.
    pub ways: BTreeSet<i64>,
    /// The OpenStreetMap nodes of both its end junctions.
    pub nodes: BTreeSet<i64>,
    pub street: Street,
}

/// An edited junction: the nodes it was made from, its state (each arm's `edge` stored as 0) and the key of the
/// street at each arm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JunctionRecord {
    pub nodes: BTreeSet<i64>,
    pub state: State,
    /// Arm uid to the key of the street it carries.
    pub arm_streets: BTreeMap<u32, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Edits {
    pub version: u32,
    pub streets: BTreeMap<String, StreetRecord>,
    pub junctions: BTreeMap<String, JunctionRecord>,
}

impl Edits {
    /// An empty document of the current version.
    pub fn new() -> Self {
        Edits { version: VERSION, ..Edits::default() }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("the edits serialise")
    }
}

/// What the store held.
#[derive(Debug)]
pub enum Document {
    /// Nothing was kept.
    Empty,
    Current(Edits),
    /// Written by a newer build: not read, and never written over.
    Future,
    /// Not a document of this version; the text is kept so it can be copied aside.
    Damaged(String),
}

pub fn parse(stored: Option<&str>) -> Document {
    let Some(raw) = stored.filter(|s| !s.trim().is_empty()) else { return Document::Empty };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else { return Document::Damaged(raw.to_string()) };
    if value["version"].as_u64().is_some_and(|v| v > u64::from(VERSION)) {
        return Document::Future;
    }
    match serde_json::from_value::<Edits>(value) {
        Ok(edits) if edits.version == VERSION => Document::Current(edits),
        _ => Document::Damaged(raw.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_stored_is_an_empty_document() {
        assert!(matches!(parse(None), Document::Empty));
        assert!(matches!(parse(Some("")), Document::Empty));
    }

    #[test]
    fn a_document_comes_back_as_it_was_kept() {
        let mut e = Edits::new();
        e.streets.insert(
            "w1~n2~n3".into(),
            StreetRecord { ways: [1].into(), nodes: [2, 3].into(), street: crate::street::model::Street::sample(0, crate::shared::catalogue::Side::Right) },
        );
        let Document::Current(back) = parse(Some(&e.to_json())) else { panic!("not read back") };
        assert_eq!(back, e);
    }

    #[test]
    fn a_newer_version_is_told_apart_and_not_read() {
        assert!(matches!(parse(Some(r#"{"version":4,"streets":{},"junctions":{}}"#)), Document::Future));
    }

    #[test]
    fn what_is_not_a_document_is_damaged_and_keeps_its_text() {
        let Document::Damaged(raw) = parse(Some("{not json")) else { panic!("not damaged") };
        assert_eq!(raw, "{not json");
        assert!(matches!(parse(Some(r#"{"version":3,"streets":5}"#)), Document::Damaged(_)));
        assert!(matches!(parse(Some(r#"{"version":2,"streets":{},"junctions":{}}"#)), Document::Damaged(_)), "an old shape is not this document");
    }
}
