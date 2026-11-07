//! Traffic calming: raised tables and raised crossings along a street, each an extent of the roadway lifted to a
//! plateau with a ramp at either end.
//!
//! They belong to the street, not to a section: a position is measured along the whole street from its anchored end
//! (`Street::from_end`), as a zone boundary and a curbside use are, and a table spans the whole width of the roadway,
//! so it has no side. A street with none is written, drawn and checked exactly as before.

use serde::{Deserialize, Serialize};

use super::{Content, Said};

/// How high the plateau of a raised table stands above the roadway, in millimetres. A placeholder, UNVERIFIED
/// (a speed table is commonly 75 to 100 mm; TAC and the MTQ guides are not checked).
pub const TABLE_HEIGHT_MM: i32 = 100;
/// How long each ramp is, in millimetres. A placeholder, UNVERIFIED.
pub const RAMP_MM: i32 = 1_500;
/// The shortest a raised table should be, ramps included, in millimetres. A placeholder, UNVERIFIED; a shorter one
/// is allowed and fails its check.
pub const MIN_TABLE_MM: i32 = 6_000;
/// The shortest plateau between the two ramps, in millimetres. A placeholder, UNVERIFIED.
pub const MIN_PLATEAU_MM: i32 = 3_000;
/// The highest speed limit a raised table is meant for, in km/h. A placeholder, UNVERIFIED.
pub const MAX_TABLE_SPEED_KMH: u8 = 50;
/// How far from a transit stop's boarding area a table must stand, in millimetres. A placeholder, UNVERIFIED.
pub const CALMING_STOP_CLEAR_MM: i32 = 5_000;
/// How far from a junction's reach a table must stand, in millimetres. A placeholder, UNVERIFIED.
pub const CALMING_JUNCTION_CLEAR_MM: i32 = 5_000;
/// The length a new element is given when none is typed, in millimetres. A placeholder, UNVERIFIED.
pub const DEFAULT_TABLE_MM: i32 = 7_000;

/// What a raised element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A raised table: a speed table across the roadway.
    Table,
    /// A raised pedestrian crossing: the plateau is the crossing.
    Crossing,
}

impl Kind {
    /// Every kind, in the order a choice lists them.
    pub const ALL: [Kind; 2] = [Kind::Table, Kind::Crossing];

    /// The kind's stable name, as saved and as the choice's value.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Table => "table",
            Kind::Crossing => "crossing",
        }
    }

    /// The message that names it.
    pub fn name_key(self) -> &'static str {
        match self {
            Kind::Table => "calming-kind-table",
            Kind::Crossing => "calming-kind-crossing",
        }
    }
}

/// One raised element, along `length_mm` of the street from `from_mm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Calming {
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of `SNAP_MM`.
    pub from_mm: i32,
    pub length_mm: i32,
    pub kind: Kind,
}

impl Calming {
    pub fn to_mm(&self) -> i32 {
        self.from_mm + self.length_mm
    }
}

/// Why an edit of a raised element was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalmingRefusal {
    /// Both ends of the street are cut by the map's box, so there is no end to measure from.
    NoAnchor,
    /// The elements were drawn on a road of another length or anchored at its other end.
    Elsewhere,
    /// A drag is under way.
    Dragging,
    /// It would start before the street or run past its end.
    Outside,
    /// It would overlap another element.
    Overlap,
    /// It would be shorter than the kind's minimum.
    TooShort { min_mm: i32 },
    /// The street is a freeway.
    Freeway,
    /// The street has no roadway to raise.
    NoRoadway,
    /// The roadway changes with the time of day there.
    TimeWindows,
}

impl CalmingRefusal {
    /// The refusal, in words.
    pub fn said(self) -> Said {
        let key = match self {
            CalmingRefusal::NoAnchor => "calming-refused-no-anchor",
            CalmingRefusal::Elsewhere => "calming-refused-elsewhere",
            CalmingRefusal::Dragging => "calming-refused-dragging",
            CalmingRefusal::Outside => "calming-refused-outside",
            CalmingRefusal::Overlap => "calming-refused-overlap",
            CalmingRefusal::TooShort { .. } => "calming-refused-too-short",
            CalmingRefusal::Freeway => "calming-refused-freeway",
            CalmingRefusal::NoRoadway => "calming-refused-no-roadway",
            CalmingRefusal::TimeWindows => "calming-refused-time-windows",
        };
        let said = Said::new(key);
        match self {
            CalmingRefusal::TooShort { min_mm } => said.with("min", super::Arg::Length(min_mm)),
            _ => said,
        }
    }
}

impl Content {
    /// The elements in their canonical order: by position.
    pub(super) fn sort_calming(&mut self) {
        self.calming.sort_by_key(|c| c.from_mm);
    }
}

impl super::Street {
    /// Whether the elements can be placed: they need the length they were drawn against, and lie in front of it.
    pub(super) fn calming_is_sound(&self) -> bool {
        self.calming.is_empty() || (self.length_mm.is_some() && self.calming.iter().all(|c| c.from_mm >= 0 && c.length_mm > 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::street::model::Street;

    fn table(from_mm: i32) -> Calming {
        Calming { from_mm, length_mm: 7_000, kind: Kind::Table }
    }

    #[test]
    fn kind_ids_are_stable() {
        assert_eq!(Kind::ALL, [Kind::Table, Kind::Crossing]);
        assert_eq!(Kind::Table.id(), "table");
        assert_eq!(Kind::Crossing.id(), "crossing");
        assert_eq!(serde_json::to_string(&Kind::Table).unwrap(), r#""table""#);
        assert_eq!(serde_json::to_string(&Kind::Crossing).unwrap(), r#""crossing""#);
    }

    #[test]
    fn a_street_without_calming_serialises_as_before() {
        let s = Street::sample(0, Side::Right);
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("calming"), "{text}");
        let back: Street = serde_json::from_str(&text).unwrap();
        assert!(back.calming.is_empty());
    }

    #[test]
    fn a_street_with_calming_round_trips_in_a_pinned_shape() {
        let mut s = Street::sample(0, Side::Right);
        s.calming = vec![table(10_000), Calming { from_mm: 30_000, length_mm: 6_000, kind: Kind::Crossing }];
        assert_eq!(
            serde_json::to_string(&s.calming).unwrap(),
            r#"[{"from_mm":10000,"length_mm":7000,"kind":"table"},{"from_mm":30000,"length_mm":6000,"kind":"crossing"}]"#
        );
        let back: Street = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.calming, s.calming);
    }

    #[test]
    fn a_refusal_says_its_own_message_and_a_short_one_names_the_minimum() {
        assert_eq!(CalmingRefusal::Overlap.said().key, "calming-refused-overlap");
        assert_eq!(CalmingRefusal::TimeWindows.said().key, "calming-refused-time-windows");
        let short = CalmingRefusal::TooShort { min_mm: MIN_TABLE_MM }.said();
        assert_eq!(short.key, "calming-refused-too-short");
        assert_eq!(short, Said::new("calming-refused-too-short").with("min", super::super::Arg::Length(MIN_TABLE_MM)));
    }

    #[test]
    fn a_street_with_elements_is_sound_only_with_the_length_they_were_drawn_on() {
        let mut s = Street::sample(0, Side::Right);
        s.calming = vec![table(0)];
        assert!(!s.is_sound());
        s.length_mm = Some(60_000);
        assert!(s.is_sound());
    }
}
