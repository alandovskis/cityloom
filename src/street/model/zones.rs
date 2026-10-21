//! Zones: a street divided along its length into ordered stretches, each with the street's base section or
//! a section of its own. A street with no zones is the one-section street the city has always kept.

use serde::{Deserialize, Serialize};

use super::{Segment, Street, flip};

/// The shortest a zone may be, in millimetres.
pub const MIN_ZONE_MM: i32 = 5_000;

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

    fn other(self) -> ZoneEnd {
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
pub(super) fn same_pieces(a: &[Segment], b: &[Segment]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| Segment { uid: y.uid, ..x.clone() } == *y)
}

/// Brings the zoning to its canonical form: zones sorted by start with the first at 0; a section equal to the
/// base (ignoring uids) kept as `None`; one zone left is no zoning (its own section becomes the base); no zones
/// left leaves no end and no length.
pub(super) fn settle_zones(segments: &mut Vec<Segment>, zones: &mut Vec<Zone>, from_end: &mut ZoneEnd, length_mm: &mut Option<i32>) {
    zones.sort_by_key(|z| z.from_mm);
    if let Some(first) = zones.first_mut() {
        first.from_mm = 0;
    }
    for z in zones.iter_mut() {
        if z.segments.as_deref().is_some_and(|s| same_pieces(s, segments)) {
            z.segments = None;
        }
    }
    if zones.len() == 1 {
        if let Some(own) = zones.remove(0).segments {
            *segments = own;
        }
    }
    if zones.is_empty() {
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
        settle_zones(&mut self.segments, &mut self.zones, &mut self.from_end, &mut self.length_mm);
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
        s.zones = Vec::new();
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
        if !self.zones.is_empty() {
            self.from_end = self.from_end.other();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::zoned;
    use super::*;
    use crate::shared::catalogue::{KINDS, Side, StreetClass, kind_index};

    fn sample() -> Street {
        Street::sample(0, Side::Right)
    }

    fn with_uids_shifted(segs: &[Segment]) -> Vec<Segment> {
        segs.iter().map(|s| Segment { uid: s.uid + 100, ..s.clone() }).collect()
    }

    /// A section that differs from the sample's base: its first piece is wider.
    fn other(base: &[Segment]) -> Vec<Segment> {
        let mut v = base.to_vec();
        v[0].width_mm += 100;
        v
    }

    #[test]
    fn a_street_with_no_zones_is_written_exactly_as_before_and_one_written_before_zones_reads_as_one_zone() {
        let s = sample();
        let v = serde_json::to_value(&s).unwrap();
        for key in ["zones", "from_end", "length_mm"] {
            assert!(v.get(key).is_none(), "{key} is written");
        }
        let back: Street = serde_json::from_value(v).unwrap();
        assert!(back.zones.is_empty());
        assert_eq!(back.from_end, ZoneEnd::First);
        assert_eq!(back.length_mm, None);
        assert_eq!(back, s);
    }

    #[test]
    fn a_zoned_street_round_trips_and_writes_only_what_is_set() {
        let mut s = zoned(sample(), &[0, 20_000], 60_000);
        s.zones[1].name = Some("Terrace".into());
        s.from_end = ZoneEnd::Last;
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["from_end"], "Last");
        assert_eq!(v["length_mm"], 60_000);
        let zones = v["zones"].as_array().unwrap();
        assert!(zones[0].get("name").is_none() && zones[0].get("segments").is_none());
        assert_eq!(zones[1]["name"], "Terrace");
        let back: Street = serde_json::from_value(v).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn a_zone_whose_section_equals_the_base_ignoring_uids_is_stored_as_none() {
        let mut s = zoned(sample(), &[0, 20_000], 60_000);
        s.zones[0].segments = Some(with_uids_shifted(&s.segments));
        s.zones[1].segments = Some(other(&s.segments));
        s.settle_zones();
        assert_eq!(s.zones[0].segments, None);
        assert_eq!(s.zones[1].segments, Some(other(&s.segments)));
    }

    #[test]
    fn settling_down_to_one_zone_moves_the_surviving_section_to_the_base_and_clears_the_zoning() {
        let mut s = zoned(sample(), &[0], 60_000);
        let next_uid = s.next_uid;
        let own = other(&s.segments);
        s.zones = vec![Zone { from_mm: 7_000, segments: Some(own.clone()), name: Some("Lone".into()) }];
        s.from_end = ZoneEnd::Last;
        s.length_mm = Some(60_000);
        s.settle_zones();
        assert_eq!(s.segments, own);
        assert!(s.zones.is_empty());
        assert_eq!(s.from_end, ZoneEnd::First);
        assert_eq!(s.length_mm, None);
        assert_eq!(s.next_uid, next_uid);

        // a one-zone result of an equal section keeps the base and drops the name
        let mut t = sample();
        let base = t.segments.clone();
        t.zones = vec![Zone { from_mm: 0, segments: Some(with_uids_shifted(&base)), name: Some("Lone".into()) }];
        t.length_mm = Some(10_000);
        t.settle_zones();
        assert_eq!(t.segments, base);
        assert!(t.zones.is_empty());

        // sorted, first forced to 0
        let mut u = sample();
        u.zones = vec![Zone { from_mm: 30_000, segments: None, name: Some("b".into()) }, Zone { from_mm: 4_000, segments: None, name: Some("a".into()) }];
        u.length_mm = Some(60_000);
        u.settle_zones();
        assert_eq!(u.zones.iter().map(|z| z.from_mm).collect::<Vec<_>>(), [0, 30_000]);
        assert_eq!(u.zones[0].name.as_deref(), Some("a"));
    }

    #[test]
    fn zones_must_start_at_0_rise_strictly_and_end_inside_the_length_or_the_street_is_not_sound() {
        let good = zoned(sample(), &[0, 20_000, 40_000], 60_000);
        assert!(good.is_sound());

        let mut first = good.clone();
        first.zones[0].from_mm = 100;
        assert!(!first.is_sound());

        let mut equal = good.clone();
        equal.zones[2].from_mm = 20_000;
        assert!(!equal.is_sound());

        let mut past = good.clone();
        past.zones[2].from_mm = 60_000;
        assert!(!past.is_sound());

        let mut no_length = good.clone();
        no_length.length_mm = None;
        assert!(!no_length.is_sound());

        let mut bad_index = good.clone();
        let mut own = good.segments.clone();
        own[0].kind = KINDS.len();
        bad_index.zones[1].segments = Some(own);
        assert!(!bad_index.is_sound());
    }

    #[test]
    fn the_class_and_has_motor_look_at_every_zone() {
        let sidewalk = kind_index("sidewalk").unwrap();
        let travel = kind_index("travel").unwrap();
        let walk = |segs: &[Segment]| -> Vec<Segment> {
            segs.iter().map(|g| Segment { variants: Vec::new(), direction: None, ..Segment::new(g.uid, sidewalk, g.width_mm) }).collect()
        };
        let mut s = sample();
        s.segments = walk(&s.segments);
        s = zoned(s, &[0, 20_000], 60_000);
        s.zones[0].segments = Some(s.segments.clone());
        let mut motor = walk(&s.segments);
        motor[1] = Segment::new(motor[1].uid, travel, motor[1].width_mm);
        s.zones[1].segments = Some(motor);
        s.class = StreetClass::Local;
        let s = s.settled();
        assert!(s.has_motor());
        assert_ne!(s.class, StreetClass::Pedestrian);

        let mut none = s.clone();
        none.zones[1].segments = Some(walk(&none.zones[1].segments.clone().unwrap()));
        let none = none.settled();
        assert!(!none.has_motor());
        assert_eq!(none.class, StreetClass::Pedestrian);
        assert_eq!(none.class_before, Some(StreetClass::Local));
    }

    #[test]
    fn for_side_flips_the_lanes_of_every_zone_and_reversed_flips_the_end_the_positions_are_measured_from() {
        let mut s = zoned(sample(), &[0, 20_000], 60_000);
        s.zones[1].segments = Some(other(&s.segments));
        let r = s.reversed();
        assert_eq!(r.from_end, ZoneEnd::Last);
        assert_eq!(r.zones.iter().map(|z| z.from_mm).collect::<Vec<_>>(), [0, 20_000]);
        let flipped = |segs: &[Segment]| -> Vec<Segment> {
            let mut v = segs.to_vec();
            v.reverse();
            for g in &mut v {
                flip(&mut g.direction);
                for w in &mut g.variants {
                    flip(&mut w.direction);
                }
            }
            v
        };
        assert_eq!(r.segments, flipped(&s.segments));
        assert_eq!(r.zones[1].segments, Some(flipped(s.zones[1].segments.as_ref().unwrap())));
        assert_eq!(r.reversed(), s);

        let l = s.for_side(Side::Left);
        let mut base = s.segments.clone();
        for g in &mut base {
            flip(&mut g.direction);
        }
        assert_eq!(l.segments, base);
        let mut own = s.zones[1].segments.clone().unwrap();
        for g in &mut own {
            flip(&mut g.direction);
        }
        assert_eq!(l.zones[1].segments, Some(own));
        assert_eq!(l.for_side(Side::Right), s);
    }

    #[test]
    fn at_end_gives_the_zone_nearest_that_end_as_a_street_of_one_zone() {
        let mut s = zoned(sample(), &[0, 20_000, 40_000], 60_000);
        let first = other(&s.segments);
        let mut last = other(&s.segments);
        last[1].width_mm += 100;
        s.zones[0].segments = Some(first.clone());
        s.zones[2].segments = Some(last.clone());
        let base = s.segments.clone();

        let a = s.at_end(true);
        assert_eq!(a.segments, first);
        assert!(a.zones.is_empty() && a.length_mm.is_none() && a.from_end == ZoneEnd::First);
        assert!(a.is_sound());
        assert_eq!(s.at_end(false).segments, last);

        s.from_end = ZoneEnd::Last;
        assert_eq!(s.at_end(true).segments, last);
        assert_eq!(s.at_end(false).segments, first);

        // a middle-only difference leaves the ends on the base
        let mut m = zoned(sample(), &[0, 20_000, 40_000], 60_000);
        m.zones[1].segments = Some(other(&base));
        assert_eq!(m.at_end(true).segments, base);

        let plain = sample();
        assert_eq!(plain.at_end(true), plain);
        assert_eq!(plain.at_end(false), plain);
    }
}
