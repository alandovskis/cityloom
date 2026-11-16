//! Zones: a street divided along its length into ordered stretches, each with the street's base section or a section
//! of its own. A street with no zones is a street of one section.

use serde::{Deserialize, Serialize};

use super::{Segment, Speed, Street, flip};

/// One stretch of a street. It ends where the next one starts; the last ends at the street's length.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Zone {
    /// Millimetres from the anchored end (`Street::from_end`) to where the zone starts.
    pub from_mm: i32,
    /// The zone's own section; `None` is the street's base section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<Segment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The zone's own speed limit, kept as `speed_kmh` like the street's: `Undecided` (left out) is the street's.
    #[serde(default, rename = "speed_kmh", skip_serializing_if = "Speed::is_undecided")]
    pub speed: Speed,
}

/// Which end of the street the zone positions are measured from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoneEnd {
    #[default]
    First,
    Last,
}

impl ZoneEnd {
    pub fn is_first(&self) -> bool {
        *self == ZoneEnd::First
    }

    /// The other end.
    pub fn other(self) -> ZoneEnd {
        match self {
            ZoneEnd::First => ZoneEnd::Last,
            ZoneEnd::Last => ZoneEnd::First,
        }
    }
}

/// The street as loaded: its length along the centreline and the end that is a real OSM node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZoneFrame {
    pub length_mm: i32,
    pub from_end: ZoneEnd,
}

/// Whether two sections have the same pieces, ignoring their `uid`s.
pub fn same_pieces(a: &[Segment], b: &[Segment]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| Segment { uid: y.uid, ..x.clone() } == *y)
}

/// Brings the zoning to its canonical form: zones sorted by start with the first at 0; a section equal to the
/// base (ignoring uids) kept as `None`, a speed equal to the street's as the street's; one zone left is no zoning
/// (its own section and speed become the base); no zones left, and no curbside use to measure, leaves no end and no length.
pub fn settle_zones(
    segments: &mut Vec<Segment>,
    zones: &mut Vec<Zone>,
    speed: &mut Speed,
    from_end: &mut ZoneEnd,
    length_mm: &mut Option<i32>,
    keep_anchor: bool,
) {
    zones.sort_by_key(|z| z.from_mm);
    if let Some(first) = zones.first_mut() {
        first.from_mm = 0;
    }
    for z in zones.iter_mut() {
        if z.segments.as_deref().is_some_and(|s| same_pieces(s, segments)) {
            z.segments = None;
        }
        if z.speed == *speed {
            z.speed = Speed::Undecided;
        }
    }
    if zones.len() == 1 {
        let only = zones.remove(0);
        if let Some(own) = only.segments {
            *segments = own;
        }
        if !only.speed.is_undecided() {
            *speed = only.speed;
        }
    }
    if zones.is_empty() && !keep_anchor {
        *from_end = ZoneEnd::First;
        *length_mm = None;
    }
}

impl Street {
    /// One section per zone: its own, or else the base. Just the base when the street has no zones.
    pub fn sections(&self) -> Vec<&[Segment]> {
        if self.zones.is_empty() {
            return vec![&self.segments];
        }
        self.zones.iter().map(|z| z.segments.as_deref().unwrap_or(&self.segments)).collect()
    }

    /// Brings the zoning to its canonical form (see `settle_zones`).
    pub fn settle_zones(&mut self) {
        settle_zones(
            &mut self.segments,
            &mut self.zones,
            &mut self.speed,
            &mut self.from_end,
            &mut self.length_mm,
            !self.curb.is_empty() || !self.stops.is_empty() || !self.green.is_empty() || !self.calming.is_empty(),
        );
    }

    /// The pieces of the zone nearest an end of the street, borrowed: what `at_end` puts in `segments`.
    pub fn end_segments(&self, first_end: bool) -> &[Segment] {
        let nearest_is_first_zone = (self.from_end == ZoneEnd::First) == first_end;
        let zone = if nearest_is_first_zone { self.zones.first() } else { self.zones.last() };
        zone.and_then(|z| z.segments.as_deref()).unwrap_or(&self.segments)
    }

    /// The zone nearest an end of the street, as a street of one zone: its section as `segments`.
    /// `first_end` says whether that is the street's first end. A street with no zones gives itself.
    pub fn at_end(&self, first_end: bool) -> Street {
        let mut s = self.clone();
        if self.zones.is_empty() {
            return s;
        }
        let nearest_is_first_zone = (self.from_end == ZoneEnd::First) == first_end;
        let zone = if nearest_is_first_zone { self.zones.first() } else { self.zones.last() };
        if let Some(own) = zone.and_then(|z| z.segments.clone()) {
            s.segments = own;
        }
        let end_zone = if nearest_is_first_zone { 0 } else { self.zones.len() - 1 };
        s.speed = self.zone_speed(end_zone);
        s.zones = Vec::new();
        s.curb = Vec::new();
        s.stops = Vec::new();
        s.calming = Vec::new();
        s.green = Vec::new();
        s.from_end = ZoneEnd::First;
        s.length_mm = None;
        s
    }

    /// Flips the lanes of every own section, as `for_side` does the base.
    pub(super) fn flip_zone_lanes(&mut self) {
        for seg in self.zones.iter_mut().filter_map(|z| z.segments.as_mut()).flatten() {
            flip(&mut seg.direction);
            for v in &mut seg.variants {
                flip(&mut v.direction);
            }
        }
    }

    /// Reverses every own section and flips the end the positions are measured from: they stay measured from
    /// the same physical end, which is now the other label.
    pub(super) fn reverse_zones(&mut self) {
        for own in self.zones.iter_mut().filter_map(|z| z.segments.as_mut()) {
            own.reverse();
        }
        self.flip_zone_lanes();
        if !self.zones.is_empty() || !self.curb.is_empty() || !self.stops.is_empty() || !self.green.is_empty() || !self.calming.is_empty() {
            self.from_end = self.from_end.other();
        }
    }
}
