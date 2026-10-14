//! The document the resident's edits are kept in: one for the whole metro area, its records keyed by the keys of
//! `city::keys`, so an edit follows its place whichever area or tile the place is read from.

use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::junction::model::State;
use crate::street::model::Street;

/// The shape of the document. Raised when the shape changes.
pub const VERSION: u32 = 4;
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
    /// For each end that is an OpenStreetMap node, by that end's key node (its smallest), the way the street
    /// leaves it in whole degrees clockwise from north: which piece of a way the edit was made on.
    pub leaving: BTreeMap<i64, i32>,
    pub street: Street,
}

/// The street at an arm of a junction, as the record names it: its key where it has one, the ways it was made
/// from and the way it leaves the junction (whole degrees clockwise from north), so that the arm finds its street
/// again among those at the junction when the street's key changes (it is cut by another area's box, say).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArmStreet {
    pub key: Option<String>,
    pub ways: BTreeSet<i64>,
    pub leaving: i32,
}

/// An edited junction: the nodes it was made from, its state (each arm's `edge` stored as 0) and the street at
/// each arm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JunctionRecord {
    pub nodes: BTreeSet<i64>,
    pub state: State,
    /// Arm uid to the street it carries.
    pub arm_streets: BTreeMap<u32, ArmStreet>,
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

/// The kept text as a document of this version (its version, and `streets` and `junctions` objects where they are
/// there), or what it is instead.
fn object(stored: Option<&str>) -> Result<Map<String, Value>, Document> {
    let Some(raw) = stored.filter(|s| !s.trim().is_empty()) else { return Err(Document::Empty) };
    let damaged = || Document::Damaged(raw.to_string());
    let Ok(Value::Object(doc)) = serde_json::from_str::<Value>(raw) else { return Err(damaged()) };
    match doc.get("version").and_then(Value::as_u64) {
        Some(v) if v > u64::from(VERSION) => return Err(Document::Future),
        Some(v) if v == u64::from(VERSION) => {}
        _ => return Err(damaged()),
    }
    if [STREETS, JUNCTIONS].iter().any(|section| doc.get(*section).is_some_and(|v| !v.is_object())) {
        return Err(damaged());
    }
    Ok(doc)
}

const STREETS: &str = "streets";
const JUNCTIONS: &str = "junctions";

/// The records of one section that this build can read; one it cannot (written by a build whose streets or
/// junctions have another shape) is left out, and left as it is in storage (`Draft`).
fn records<T: DeserializeOwned>(doc: &Map<String, Value>, section: &str) -> BTreeMap<String, T> {
    let kept = doc.get(section).and_then(Value::as_object).into_iter().flatten();
    kept.filter_map(|(key, value)| Some((key.clone(), T::deserialize(value).ok()?))).collect()
}

pub fn parse(stored: Option<&str>) -> Document {
    match object(stored) {
        Ok(doc) => Document::Current(Edits { version: VERSION, streets: records(&doc, STREETS), junctions: records(&doc, JUNCTIONS) }),
        Err(other) => other,
    }
}

/// The document as kept, every record as it was written (one this build cannot read, a field it does not know),
/// for a write that changes some records and leaves the others exactly as they were.
pub struct Draft {
    doc: Map<String, Value>,
}

/// The kept text as a draft to change, or what it is when it is not a document of this version.
pub fn draft(stored: Option<&str>) -> Result<Draft, Document> {
    object(stored).map(|doc| Draft { doc })
}

impl Draft {
    /// An empty document of this version.
    pub fn new() -> Draft {
        let mut doc = Map::new();
        doc.insert("version".into(), VERSION.into());
        Draft { doc }
    }

    fn put(&mut self, section: &str, key: &str, record: Result<Value, serde_json::Error>) -> bool {
        let Ok(record) = record else { return false };
        match self.doc.entry(section).or_insert_with(|| Value::Object(Map::new())) {
            Value::Object(records) => {
                records.insert(key.to_string(), record);
                true
            }
            _ => false,
        }
    }

    fn remove(&mut self, section: &str, key: &str) {
        if let Some(Value::Object(records)) = self.doc.get_mut(section) {
            records.remove(key);
        }
    }

    /// Keeps `record` under `key`. Says whether it could be.
    pub fn put_street(&mut self, key: &str, record: &StreetRecord) -> bool {
        self.put(STREETS, key, serde_json::to_value(record))
    }

    pub fn put_junction(&mut self, key: &str, record: &JunctionRecord) -> bool {
        self.put(JUNCTIONS, key, serde_json::to_value(record))
    }

    pub fn remove_street(&mut self, key: &str) {
        self.remove(STREETS, key);
    }

    pub fn remove_junction(&mut self, key: &str) {
        self.remove(JUNCTIONS, key);
    }

    pub fn to_json(&self) -> String {
        Value::Object(self.doc.clone()).to_string()
    }
}

/// What identifies a place, or a stored record, when they are matched: its key, the OpenStreetMap ids it was
/// made from (a junction has no `ways`) and, for a street, the way it leaves each end that is an OpenStreetMap
/// node (as `StreetRecord::leaving`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ids {
    pub key: Option<String>,
    pub ways: BTreeSet<i64>,
    pub nodes: BTreeSet<i64>,
    pub leaving: BTreeMap<i64, i32>,
}

/// How far apart two bearings are, in degrees (0 to 180).
pub fn degrees_apart(a: i32, b: i32) -> i32 {
    let d = (a - b).rem_euclid(360);
    d.min(360 - d)
}

/// Two streets that leave a shared end less than this many degrees apart leave it the same way.
pub const SAME_WAY_DEG: i32 = 45;

impl Ids {
    pub fn from_street_record(key: &str, r: &StreetRecord) -> Ids {
        Ids { key: Some(key.to_string()), ways: r.ways.clone(), nodes: r.nodes.clone(), leaving: r.leaving.clone() }
    }

    pub fn from_junction_record(key: &str, r: &JunctionRecord) -> Ids {
        Ids { key: Some(key.to_string()), ways: BTreeSet::new(), nodes: r.nodes.clone(), leaving: BTreeMap::new() }
    }

    /// Whether a place and a record leave an end they share the same way: of the pieces of a way that meet at a
    /// junction, the one the edit was made on.
    fn same_way(&self, other: &Ids) -> bool {
        self.leaving.iter().any(|(node, &deg)| other.leaving.get(node).is_some_and(|&o| degrees_apart(deg, o) < SAME_WAY_DEG))
    }

    /// Whether a place and a record leave an end they share different ways: another piece of the way.
    fn other_way(&self, other: &Ids) -> bool {
        self.leaving.iter().any(|(node, &deg)| other.leaving.get(node).is_some_and(|&o| degrees_apart(deg, o) >= SAME_WAY_DEG))
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
/// overlap, and of two that overlap as much, the one that leaves a shared end the same way, if that is unique. A
/// place that leaves a shared end another way than the record (by `SAME_WAY_DEG` or more) does not overlap it. A
/// record wanted by several places goes to the one with the larger score; a tie gives it to nobody.
pub fn assign(places: &[Ids], records: &[Ids]) -> Vec<Option<usize>> {
    // An exact key outranks any overlap; places sharing a key are told apart by their overlap.
    let score = |p: &Ids, r: &Ids| {
        let exact = p.key.is_some() && p.key == r.key;
        // a piece that leaves a shared end another way than the record's is not the piece the edit was made on
        let overlap = if !exact && p.other_way(r) { 0 } else { p.overlap(r) };
        (exact, overlap, overlap > 0 && p.same_way(r))
    };
    let none = (false, 0, false);
    // only a record with the place's key or one of its nodes can score: the others are never looked at, so a load
    // is not places × records
    let mut by_key: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut by_node: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (ri, r) in records.iter().enumerate() {
        if let Some(k) = &r.key {
            by_key.entry(k).or_default().push(ri);
        }
        for &n in &r.nodes {
            by_node.entry(n).or_default().push(ri);
        }
    }
    let candidates = |p: &Ids| -> BTreeSet<usize> {
        let keyed = p.key.as_deref().and_then(|k| by_key.get(k)).into_iter().flatten();
        let shared = p.nodes.iter().filter_map(|n| by_node.get(n)).flatten();
        keyed.chain(shared).copied().collect()
    };
    let mut want: Vec<Option<usize>> = places
        .iter()
        .map(|p| {
            let scored: Vec<(usize, _)> = candidates(p).into_iter().map(|ri| (ri, score(p, &records[ri]))).collect();
            let best = scored.iter().map(|&(_, s)| s).max().unwrap_or(none);
            if best == none {
                return None;
            }
            let mut top = scored.iter().filter(|&&(_, s)| s == best).map(|&(ri, _)| ri);
            let first = top.next()?;
            top.next().is_none().then_some(first)
        })
        .collect();
    let mut claimants: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (pi, w) in want.iter().enumerate() {
        if let Some(ri) = *w {
            claimants.entry(ri).or_default().push(pi);
        }
    }
    for (ri, claimants) in claimants.into_iter().filter(|(_, c)| c.len() > 1) {
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
            StreetRecord {
                ways: [1].into(),
                nodes: [2, 3].into(),
                leaving: [(2, 90)].into(),
                street: crate::street::model::Street::sample(0, crate::shared::catalogue::Side::Right),
            },
        );
        let Document::Current(back) = parse(Some(&e.to_json())) else { panic!("not read back") };
        assert_eq!(back, e);
    }

    #[test]
    fn a_newer_version_is_told_apart_and_not_read() {
        assert!(matches!(parse(Some(r#"{"version":5,"streets":{},"junctions":{}}"#)), Document::Future));
    }

    #[test]
    fn what_is_not_a_document_is_damaged_and_keeps_its_text() {
        let Document::Damaged(raw) = parse(Some("{not json")) else { panic!("not damaged") };
        assert_eq!(raw, "{not json");
        assert!(matches!(parse(Some(r#"{"version":4,"streets":5}"#)), Document::Damaged(_)));
        let Document::Damaged(raw) = parse(Some(r#"{"version":3,"streets":{},"junctions":{}}"#)) else { panic!("the previous version is read") };
        assert!(raw.contains("\"version\":3"), "its text is kept so that it can be copied aside");
        assert!(matches!(parse(Some(r#"{"version":2,"streets":{},"junctions":{}}"#)), Document::Damaged(_)), "an old shape is not this document");
    }

    fn pedestrian_record() -> StreetRecord {
        use crate::shared::catalogue::{Side, StreetClass, kind_index};
        use crate::street::model::{Piece, Street};
        let walk = Piece { kind: kind_index("sidewalk").unwrap(), width_mm: 3000, direction: None, variants: Vec::new() };
        StreetRecord {
            ways: [8].into(),
            nodes: [1, 2].into(),
            leaving: [(1, 90)].into(),
            street: Street::imported(StreetClass::Local, Side::Right, &[walk]).named("Rue Piétonne"),
        }
    }

    #[test]
    fn a_pedestrian_street_comes_back_with_the_class_it_had() {
        let mut e = Edits::new();
        e.streets.insert("w8~n1~n2".into(), pedestrian_record());
        let Document::Current(back) = parse(Some(&e.to_json())) else { panic!("not read back") };
        let street = &back.streets["w8~n1~n2"].street;
        assert_eq!(
            (street.class, street.class_before),
            (crate::shared::catalogue::StreetClass::Pedestrian, Some(crate::shared::catalogue::StreetClass::Local))
        );
        assert_eq!(back, e);
    }

    fn street_record() -> StreetRecord {
        StreetRecord {
            ways: [7].into(),
            nodes: [1, 2].into(),
            leaving: [(1, 90)].into(),
            street: crate::street::model::Street::sample(0, crate::shared::catalogue::Side::Right),
        }
    }

    #[test]
    fn a_record_that_cannot_be_read_is_left_out_and_the_others_are_read() {
        let mut e = Edits::new();
        e.streets.insert("w7~n1~n2".into(), street_record());
        let mut value: serde_json::Value = serde_json::from_str(&e.to_json()).unwrap();
        // written by a build whose streets have a class this one does not know
        let mut odd = value["streets"]["w7~n1~n2"].clone();
        odd["street"]["class"] = "Hovercraft".into();
        value["streets"]["w9~n1~n2"] = odd;
        let Document::Current(read) = parse(Some(&value.to_string())) else { panic!("the document is not read") };
        assert_eq!(read.streets.keys().collect::<Vec<_>>(), ["w7~n1~n2"]);
    }

    #[test]
    fn a_change_keeps_every_record_it_does_not_touch_as_it_was_kept_readable_or_not() {
        let mut e = Edits::new();
        e.streets.insert("w7~n1~n2".into(), street_record());
        let mut value: serde_json::Value = serde_json::from_str(&e.to_json()).unwrap();
        value["streets"]["w9~n1~n2"] = serde_json::json!({ "from": "a newer build", "street": { "class": "Hovercraft" } });
        value["streets"]["w7~n1~n2"]["note"] = "a field this build does not know".into();
        let Ok(mut draft) = draft(Some(&value.to_string())) else { panic!("not a document of this version") };
        assert!(draft.put_street("w8~n1~n2", &street_record()));
        draft.remove_street("w5~n1~n2");
        let back: serde_json::Value = serde_json::from_str(&draft.to_json()).unwrap();
        assert_eq!(back["streets"]["w9~n1~n2"], value["streets"]["w9~n1~n2"], "an unreadable record is written back unchanged");
        assert_eq!(back["streets"]["w7~n1~n2"], value["streets"]["w7~n1~n2"], "an unknown field of another record too");
        assert!(back["streets"]["w8~n1~n2"].is_object());
        assert_eq!(back["version"], VERSION);
    }

    /// Every field of a value, as `path: type`, arrays as `[]`.
    fn shape(value: &serde_json::Value, at: &str, out: &mut BTreeSet<String>) {
        use serde_json::Value::*;
        match value {
            Object(map) => {
                out.insert(format!("{at}: object"));
                for (k, v) in map {
                    // map keys that are ids or keys are data, not fields
                    let id = |k: &str| k.trim_start_matches('n').chars().all(|c| c.is_ascii_digit()) && !k.is_empty();
                    let field = if id(k) || k.contains('~') { "<key>" } else { k.as_str() };
                    shape(v, &format!("{at}.{field}"), out);
                }
            }
            Array(items) => {
                out.insert(format!("{at}: array"));
                items.iter().for_each(|v| shape(v, &format!("{at}[]"), out));
            }
            String(_) => drop(out.insert(format!("{at}: string"))),
            Number(_) => drop(out.insert(format!("{at}: number"))),
            Bool(_) => drop(out.insert(format!("{at}: bool"))),
            Null => drop(out.insert(format!("{at}: null"))),
        }
    }

    #[test]
    fn a_junction_record_with_signal_timing_round_trips_and_one_written_before_it_reads_as_untimed() {
        use crate::junction::model::{SIGNAL, Timing};
        let city = crate::city::model::City::new();
        let junction = city.view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let (key, mut record) = city.junction_record(junction).unwrap();
        assert!(record.state.arms.iter().all(|a| a.timing.is_none()), "an arm never timed has none to write");
        record.state.control = SIGNAL;
        let timing = Timing { green_s: 40, yellow_s: 4, all_red_s: 3, transit_lead_s: 6 };
        record.state.arms[0].timing = Some(timing);
        let mut e = Edits::new();
        e.junctions.insert(key.clone(), record);
        let text = e.to_json();
        assert!(text.contains("\"timing\""), "{text}");
        let Document::Current(back) = parse(Some(&text)) else { panic!("not a document of this version") };
        assert_eq!(back.junctions[&key].state.arms[0].timing, Some(timing));
        assert!(back.junctions[&key].state.arms[1..].iter().all(|a| a.timing.is_none()));
        // A record written before timing existed reads as untimed, and the document is still this version's.
        let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
        for arm in value["junctions"][&key]["state"]["arms"].as_array_mut().unwrap() {
            arm.as_object_mut().unwrap().remove("timing");
        }
        let Document::Current(old) = parse(Some(&value.to_string())) else { panic!("not a document of this version") };
        assert!(old.junctions[&key].state.arms.iter().all(|a| a.timing.is_none()));
        assert_eq!(old.version, VERSION);
    }

    /// Pins what a record is written as. When this fails, the shape of `Street`, `State` or a record changed:
    /// decide whether an older build can still read it, and if not raise `VERSION`, before changing this list.
    #[test]
    fn what_a_record_is_written_as_is_pinned_so_that_a_change_of_shape_is_a_decision_about_the_version() {
        let city = crate::city::model::City::new();
        let junction = city.view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let mut e = Edits::new();
        e.streets.insert("w7~n1~n2".into(), street_record());
        e.streets.insert("w8~n1~n2".into(), pedestrian_record());
        let (key, record) = city.junction_record(junction).unwrap();
        e.junctions.insert(key, record);
        let mut out = BTreeSet::new();
        shape(&serde_json::from_str(&e.to_json()).unwrap(), "", &mut out);
        let got: Vec<&str> = out.iter().map(String::as_str).collect();
        assert_eq!(got, GOLDEN, "{got:#?}");
    }

    #[rustfmt::skip]
    const GOLDEN: &[&str] = &[
    ".junctions.<key>.arm_streets.<key>.key: string",
    ".junctions.<key>.arm_streets.<key>.leaving: number",
    ".junctions.<key>.arm_streets.<key>.ways: array",
    ".junctions.<key>.arm_streets.<key>.ways[]: number",
    ".junctions.<key>.arm_streets.<key>: object",
    ".junctions.<key>.arm_streets: object",
    ".junctions.<key>.nodes: array",
    ".junctions.<key>.nodes[]: number",
    ".junctions.<key>.state.arms: array",
    ".junctions.<key>.state.arms[].approach: number",
    ".junctions.<key>.state.arms[].approach_mm: number",
    ".junctions.<key>.state.arms[].banned: array",
    ".junctions.<key>.state.arms[].bearing: number",
    ".junctions.<key>.state.arms[].bike_box: bool",
    ".junctions.<key>.state.arms[].bulb: array",
    ".junctions.<key>.state.arms[].bulb[]: bool",
    ".junctions.<key>.state.arms[].bus_lane: bool",
    ".junctions.<key>.state.arms[].corner_mm: number",
    ".junctions.<key>.state.arms[].crossing.continuous: bool",
    ".junctions.<key>.state.arms[].crossing.island: bool",
    ".junctions.<key>.state.arms[].crossing.setback_mm: number",
    ".junctions.<key>.state.arms[].crossing.width_mm: number",
    ".junctions.<key>.state.arms[].crossing: object",
    ".junctions.<key>.state.arms[].edge: number",
    ".junctions.<key>.state.arms[].filter: bool",
    ".junctions.<key>.state.arms[].lanes: array",
    ".junctions.<key>.state.arms[].lanes[].to: array",
    ".junctions.<key>.state.arms[].lanes[].to[]: number",
    ".junctions.<key>.state.arms[].lanes[]: object",
    ".junctions.<key>.state.arms[].offset_mm: number",
    ".junctions.<key>.state.arms[].protected: bool",
    ".junctions.<key>.state.arms[].rule: number",
    ".junctions.<key>.state.arms[].stop: number",
    ".junctions.<key>.state.arms[].uid: number",
    ".junctions.<key>.state.arms[]: object",
    ".junctions.<key>.state.bus: null",
    ".junctions.<key>.state.control: number",
    ".junctions.<key>.state.cycle: null",
    ".junctions.<key>.state.raised: bool",
    ".junctions.<key>.state.ring_extra_mm: number",
    ".junctions.<key>.state.source: array",
    ".junctions.<key>.state.source[].id: number",
    ".junctions.<key>.state.source[].version: null",
    ".junctions.<key>.state.source[]: object",
    ".junctions.<key>.state: object",
    ".junctions.<key>: object",
    ".junctions: object",
    ".streets.<key>.leaving.<key>: number",
    ".streets.<key>.leaving: object",
    ".streets.<key>.nodes: array",
    ".streets.<key>.nodes[]: number",
    ".streets.<key>.street.class: string",
    ".streets.<key>.street.class_before: string",
    ".streets.<key>.street.name: string",
    ".streets.<key>.street.next_uid: number",
    ".streets.<key>.street.row_mm: number",
    ".streets.<key>.street.segments: array",
    ".streets.<key>.street.segments[].curb: null",
    ".streets.<key>.street.segments[].curb: number",
    ".streets.<key>.street.segments[].direction: null",
    ".streets.<key>.street.segments[].direction: number",
    ".streets.<key>.street.segments[].kind: number",
    ".streets.<key>.street.segments[].material: number",
    ".streets.<key>.street.segments[].tram: bool",
    ".streets.<key>.street.segments[].uid: number",
    ".streets.<key>.street.segments[].variants: array",
    ".streets.<key>.street.segments[].width_mm: number",
    ".streets.<key>.street.segments[]: object",
    ".streets.<key>.street.side: string",
    ".streets.<key>.street: object",
    ".streets.<key>.ways: array",
    ".streets.<key>.ways[]: number",
    ".streets.<key>: object",
    ".streets: object",
    ".version: number",
    ": object",
    ];

    fn ids(key: Option<&str>, ways: &[i64], nodes: &[i64]) -> Ids {
        Ids { key: key.map(str::to_string), ways: ways.iter().copied().collect(), nodes: nodes.iter().copied().collect(), leaving: BTreeMap::new() }
    }

    fn leaving(mut ids: Ids, node: i64, deg: i32) -> Ids {
        ids.leaving.insert(node, deg);
        ids
    }

    #[test]
    fn of_two_pieces_of_a_way_that_overlap_a_record_as_much_the_one_leaving_the_shared_end_the_same_way_takes_it() {
        // kept in an area where the street ran east from n2 to n3; here the box cuts the way on both sides of n2
        let record = [leaving(ids(Some("w7~n2~n3"), &[7], &[2, 3]), 2, 88)];
        let east = leaving(ids(Some("w7~n2~x~e"), &[7], &[2]), 2, 91);
        let west = leaving(ids(Some("w7~n2~x~w"), &[7], &[2]), 2, 271);
        assert_eq!(assign(&[west.clone(), east.clone()], &record), [None, Some(0)]);
        let unknown = [ids(Some("w7~n2~n3"), &[7], &[2, 3])];
        assert_eq!(assign(&[west, east], &unknown), [None, None], "nothing says which way it left: nobody's");
    }

    #[test]
    fn a_piece_of_the_way_that_leaves_the_shared_end_another_way_does_not_take_the_record_even_alone() {
        // kept on the piece running east from n2; here only the piece running west from n2 is in the area
        let record = [leaving(ids(Some("w7~n2~n3"), &[7], &[2, 3]), 2, 90)];
        let west = leaving(ids(Some("w7~n1~n2"), &[7], &[1, 2]), 2, 270);
        assert_eq!(assign(&[west], &record), [None]);
        let unknown = ids(Some("w7~n1~n2"), &[7], &[1, 2]);
        assert_eq!(assign(&[unknown], &record), [Some(0)], "nothing says which way the place leaves: the overlap decides");
        let exact = leaving(ids(Some("w7~n2~n3"), &[7], &[2, 3]), 2, 270);
        assert_eq!(assign(&[exact], &record), [Some(0)], "the exact key is not second-guessed");
    }

    #[test]
    fn the_halves_of_a_way_split_in_the_middle_both_leave_their_ends_the_same_way_and_are_nobodys() {
        let record = [leaving(leaving(ids(Some("w7~n1~n2"), &[7], &[1, 2]), 1, 90), 2, 270)];
        let first = leaving(ids(Some("w7~n1~n5"), &[7], &[1, 5]), 1, 90);
        let second = leaving(ids(Some("w7~n2~n5"), &[7], &[2, 5]), 2, 270);
        assert_eq!(assign(&[first, second], &record), [None, None]);
    }

    #[test]
    fn bearings_are_apart_by_the_shorter_way_round() {
        assert_eq!((degrees_apart(10, 350), degrees_apart(350, 10), degrees_apart(0, 180), degrees_apart(90, 90)), (20, 20, 180, 0));
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
        let street = StreetRecord {
            ways: [7].into(),
            nodes: [1, 2].into(),
            leaving: [(1, 90)].into(),
            street: crate::street::model::Street::sample(0, crate::shared::catalogue::Side::Right),
        };
        assert_eq!(Ids::from_street_record("w7~n1~n2", &street), leaving(ids(Some("w7~n1~n2"), &[7], &[1, 2]), 1, 90));
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
