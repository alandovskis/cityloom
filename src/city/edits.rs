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

/// What identifies a place, or a stored record, when they are matched: its key and the OpenStreetMap ids it was
/// made from (a junction has no `ways`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ids {
    pub key: Option<String>,
    pub ways: BTreeSet<i64>,
    pub nodes: BTreeSet<i64>,
}

impl Ids {
    pub fn from_street_record(key: &str, r: &StreetRecord) -> Ids {
        Ids { key: Some(key.to_string()), ways: r.ways.clone(), nodes: r.nodes.clone() }
    }

    pub fn from_junction_record(key: &str, r: &JunctionRecord) -> Ids {
        Ids { key: Some(key.to_string()), ways: BTreeSet::new(), nodes: r.nodes.clone() }
    }

    /// How much a place and a record have in common. A street (it has ways) needs a shared way and a shared end.
    fn overlap(&self, other: &Ids) -> usize {
        let nodes = self.nodes.intersection(&other.nodes).count();
        if self.ways.is_empty() && other.ways.is_empty() {
            return nodes;
        }
        let ways = self.ways.intersection(&other.ways).count();
        if ways == 0 || nodes == 0 { 0 } else { ways + nodes }
    }
}

/// For each place, the index of the record it takes. An exact key wins; otherwise the record with the largest
/// overlap, if that is unique. A record wanted by several places goes to the one with the larger score; a tie
/// gives it to nobody.
pub fn assign(places: &[Ids], records: &[Ids]) -> Vec<Option<usize>> {
    // An exact key outranks any overlap; places sharing a key are told apart by their overlap.
    let score = |p: &Ids, r: &Ids| (p.key.is_some() && p.key == r.key, p.overlap(r));
    let none = (false, 0);
    let mut want: Vec<Option<usize>> = places
        .iter()
        .map(|p| {
            let best = records.iter().map(|r| score(p, r)).max().unwrap_or(none);
            if best == none {
                return None;
            }
            let mut top = records.iter().enumerate().filter(|(_, r)| score(p, r) == best).map(|(i, _)| i);
            let first = top.next()?;
            top.next().is_none().then_some(first)
        })
        .collect();
    for ri in 0..records.len() {
        let claimants: Vec<usize> = (0..places.len()).filter(|&pi| want[pi] == Some(ri)).collect();
        if claimants.len() < 2 {
            continue;
        }
        let top = claimants.iter().map(|&pi| score(&places[pi], &records[ri])).max().unwrap_or(none);
        let winners: Vec<usize> = claimants.iter().copied().filter(|&pi| score(&places[pi], &records[ri]) == top).collect();
        let keep = (winners.len() == 1).then(|| winners[0]);
        for &pi in &claimants {
            if Some(pi) != keep {
                want[pi] = None;
            }
        }
    }
    want
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

    fn ids(key: Option<&str>, ways: &[i64], nodes: &[i64]) -> Ids {
        Ids { key: key.map(str::to_string), ways: ways.iter().copied().collect(), nodes: nodes.iter().copied().collect() }
    }

    #[test]
    fn a_record_goes_to_the_place_with_its_key() {
        let places = [ids(Some("n5"), &[], &[5]), ids(Some("n9"), &[], &[9])];
        let records = [ids(Some("n9"), &[], &[9]), ids(Some("n5"), &[], &[5])];
        assert_eq!(assign(&places, &records), [Some(1), Some(0)]);
    }

    #[test]
    fn a_junction_whose_key_changed_is_found_by_a_shared_osm_node() {
        // OSM merged node 5 into a cluster whose smallest id is now 3
        let places = [ids(Some("n3"), &[], &[3, 5])];
        let records = [ids(Some("n5"), &[], &[5])];
        assert_eq!(assign(&places, &records), [Some(0)]);
    }

    #[test]
    fn a_street_needs_a_shared_way_and_a_shared_end() {
        let record = [ids(Some("w7~n1~n2"), &[7], &[1, 2])];
        assert_eq!(
            assign(&[ids(Some("w7~n2~n9"), &[7], &[2, 9])], &record),
            [Some(0)],
            "the way was cut at a new junction: it still shares the way and an end"
        );
        assert_eq!(assign(&[ids(Some("w8~n1~n2"), &[8], &[1, 2])], &record), [None], "same ends, other way");
        assert_eq!(assign(&[ids(Some("w7~n8~n9"), &[7], &[8, 9])], &record), [None], "same way, other ends");
    }

    #[test]
    fn two_places_claiming_one_record_give_it_to_the_larger_overlap_or_to_nobody() {
        let record = [ids(Some("n1"), &[], &[1, 2, 3])];
        assert_eq!(assign(&[ids(Some("n7"), &[], &[1, 2]), ids(Some("n8"), &[], &[3])], &record), [Some(0), None]);
        assert_eq!(assign(&[ids(Some("n7"), &[], &[1]), ids(Some("n8"), &[], &[2])], &record), [None, None], "a tie is nobody's");
    }

    #[test]
    fn the_place_with_the_exact_key_beats_an_overlapping_one() {
        let record = [ids(Some("n1"), &[], &[1, 2, 3])];
        let places = [ids(Some("n9"), &[], &[1, 2, 3]), ids(Some("n1"), &[], &[1])];
        assert_eq!(assign(&places, &record), [None, Some(0)]);
    }

    #[test]
    fn a_place_overlapping_two_records_equally_takes_neither() {
        let records = [ids(Some("n1"), &[], &[1]), ids(Some("n2"), &[], &[2])];
        assert_eq!(assign(&[ids(Some("n9"), &[], &[1, 2])], &records), [None]);
    }

    #[test]
    fn a_place_without_a_key_matches_only_by_overlap_and_may_match_nothing() {
        assert_eq!(assign(&[ids(None, &[], &[])], &[ids(Some("n1"), &[], &[1])]), [None]);
        assert_eq!(assign(&[ids(None, &[], &[1])], &[ids(Some("n1"), &[], &[1])]), [Some(0)]);
    }

    #[test]
    fn a_record_read_from_the_store_carries_its_key_and_ids() {
        let street =
            StreetRecord { ways: [7].into(), nodes: [1, 2].into(), street: crate::street::model::Street::sample(0, crate::shared::catalogue::Side::Right) };
        assert_eq!(Ids::from_street_record("w7~n1~n2", &street), ids(Some("w7~n1~n2"), &[7], &[1, 2]));
        let junction = JunctionRecord {
            nodes: [4, 5].into(),
            state: State {
                label: crate::junction::model::unlabelled(),
                arms: vec![],
                control: 0,
                ring_extra_mm: 0,
                bus: None,
                cycle: None,
                raised: false,
                source: Default::default(),
            },
            arm_streets: BTreeMap::new(),
        };
        assert_eq!(Ids::from_junction_record("n4", &junction), ids(Some("n4"), &[], &[4, 5]));
    }

    #[test]
    fn places_with_the_same_key_are_told_apart_by_overlap_and_a_tie_is_nobodys() {
        let record = [ids(Some("n1"), &[], &[1, 2])];
        assert_eq!(assign(&[ids(Some("n1"), &[], &[1]), ids(Some("n1"), &[], &[1, 2])], &record), [None, Some(0)]);
        assert_eq!(assign(&[ids(Some("n1"), &[], &[1]), ids(Some("n1"), &[], &[2])], &record), [None, None]);
    }

    #[test]
    fn two_street_places_claiming_one_record_give_it_to_the_larger_overlap() {
        let record = [ids(Some("w7~n1~n2"), &[7], &[1, 2])];
        let places = [ids(Some("w7~n2~n9"), &[7], &[2, 9]), ids(Some("w7~n1~n2~x"), &[7], &[1, 2])];
        assert_eq!(assign(&places, &record), [None, Some(0)]);
        let tied = [ids(Some("w7~n2~n9"), &[7], &[2, 9]), ids(Some("w7~n1~n8"), &[7], &[1, 8])];
        assert_eq!(assign(&tied, &record), [None, None], "a tie is nobody's");
    }

    #[test]
    fn the_street_with_the_exact_key_beats_a_larger_overlap() {
        let record = [ids(Some("w7~n1~n2"), &[7], &[1, 2])];
        let places = [ids(Some("w7~n1~n2~cut"), &[7, 8], &[1, 2, 3]), ids(Some("w7~n1~n2"), &[7], &[1, 2])];
        assert_eq!(assign(&places, &record), [None, Some(0)]);
    }

    #[test]
    fn the_junction_with_the_exact_key_beats_a_larger_overlap_even_when_it_overlaps_little() {
        let record = [ids(Some("n1"), &[], &[1, 2, 3])];
        let places = [ids(Some("n9"), &[], &[1, 2, 3]), ids(Some("n1"), &[], &[7])];
        assert_eq!(assign(&places, &record), [None, Some(0)]);
    }
}
