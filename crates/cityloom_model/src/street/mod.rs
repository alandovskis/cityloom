//! A street as the city keeps it and the street editor edits it: the pieces of its cross-section, its zones along its
//! length, its speed limit and what stands along it (curbside uses, transit stops, raised elements, trees and green
//! infrastructure). This is what OpenStreetMap can say of a street; the rules for editing it, the checks and the
//! wording are the editor's. All lengths are integer millimetres.

use serde::{Deserialize, Serialize};

use crate::SourceRef;
use crate::catalogue::{CURBS, DIRECTIONS, KINDS, MATERIALS, Side, StreetClass};

mod calming;
mod curb;
mod from_road;
mod green;
mod segment;
mod speed;
mod stop;
mod zone;

pub use calming::{Calming, Kind as CalmingKind};
pub use curb::{Instance, Use};
pub use from_road::{class_of, legacy_pieces, one_way_bike_lanes, pieces, refs};
pub use green::{Green, GreenKind, TreeSize, sort_green};
pub use segment::{SLOT_MIN, SLOTS, Segment, Variant, valid_window, window_mask};
pub use speed::{Speed, max_kmh, speed_in};
pub use stop::{Stop, StopKind};
pub use zone::{Zone, ZoneEnd, ZoneFrame, same_pieces, settle_zones};

/// One street as the city keeps it: the pieces of its section as the street
/// editor shows them, looking from the street's first end toward its second.
/// Directions are written for the side of the road in `side`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Street {
    /// What sort of street it is.
    pub class: StreetClass,
    /// The class the street had before it became pedestrian: giving it a lane again restores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_before: Option<StreetClass>,
    /// What the street is called.
    #[serde(default)]
    pub name: Option<String>,
    pub row_mm: i32,
    pub side: Side,
    pub segments: Vec<Segment>,
    pub next_uid: u32,
    /// The OSM ways it was made from. The sample streets are made from none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source: Vec<SourceRef>,
    /// The street's zones along its length, in order of `from_mm`. Empty for a street of one section, the base.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<Zone>,
    /// The end the zones' positions are measured from: the end that is a real OSM node. Written only with zones.
    #[serde(default, skip_serializing_if = "ZoneEnd::is_first")]
    pub from_end: ZoneEnd,
    /// The length along the centreline the zones were drawn against. Written only with zones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_mm: Option<i32>,
    /// The speed limit, kept as `speed_kmh`: absent when not yet decided, `null` when the road has none.
    #[serde(default, rename = "speed_kmh", skip_serializing_if = "Speed::is_undecided")]
    pub speed: Speed,
    /// The uses of the curbside along the street, left side first, then by `from_mm`. Empty for a street whose curbside
    /// is whatever its sections hold; with any, `from_end` and `length_mm` are written as they are with zones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curb: Vec<Instance>,
    /// The transit stops along the street, left side first, then by `from_mm`. Empty for a street with none; with
    /// any, `from_end` and `length_mm` are written as they are with zones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stops: Vec<Stop>,
    /// The raised tables and crossings along the street, by `from_mm`. Empty for a street with none; with any,
    /// `from_end` and `length_mm` are written as they are with zones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calming: Vec<Calming>,
    /// The trees and green infrastructure along the street, by `from_mm`. Empty for a street with none; with any,
    /// `from_end` and `length_mm` are written as they are with zones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub green: Vec<Green>,
}

/// The class a street has, and the one it had before it became pedestrian, given whether it has a motor piece.
pub fn settled_class(motor: bool, class: StreetClass, before: Option<StreetClass>) -> (StreetClass, Option<StreetClass>) {
    match (motor, class) {
        (false, class) if class != StreetClass::Pedestrian => (StreetClass::Pedestrian, Some(class)),
        (true, StreetClass::Pedestrian) => (before.unwrap_or(StreetClass::Local), None),
        _ => (class, before),
    }
}

/// Turns a direction of travel the other way.
pub fn flip(d: &mut Option<usize>) {
    if let Some(d) = d {
        *d = 1 - *d;
    }
}

/// One piece of a street, as read from a road of the network.
#[derive(Clone, Debug)]
pub struct Piece {
    pub kind: usize,
    pub width_mm: i32,
    /// Index into `DIRECTIONS`, for a kind that has one.
    pub direction: Option<usize>,
    /// The times of day it is another kind; they must not overlap.
    pub variants: Vec<Window>,
}

/// A time of day, in minutes after midnight and on quarter hours as `Variant`'s are, that a piece is of another kind.
#[derive(Clone, Debug)]
pub struct Window {
    pub kind: usize,
    pub from_min: i32,
    pub to_min: i32,
    /// The days of the week, as `Variant::days`.
    pub days: Option<String>,
}

impl Street {
    /// Whether any piece of any section is a motor piece.
    pub fn has_motor(&self) -> bool {
        self.sections().iter().any(|s| s.iter().any(Segment::has_motor))
    }

    /// Keeps the class true to the pieces. No motor piece: pedestrian, remembering the class it had. A motor
    /// piece (travel, parking, loading, bus, shoulder) on a pedestrian street: the class it had, or else local. Run after every edit and when a street is built.
    pub fn settle_class(&mut self) {
        (self.class, self.class_before) = settled_class(self.has_motor(), self.class, self.class_before);
    }

    /// The same street, settled.
    pub fn settled(mut self) -> Street {
        self.settle_zones();
        self.settle_class();
        self
    }

    /// The same street, called `name`.
    pub fn named(mut self, name: &str) -> Street {
        self.name = Some(name.to_string());
        self
    }

    /// The same street with its lanes written for `side`: running the other
    /// way when the side differs.
    pub fn for_side(&self, side: Side) -> Street {
        let mut s = self.clone();
        if s.side != side {
            s.side = side;
            for seg in &mut s.segments {
                flip(&mut seg.direction);
                for v in &mut seg.variants {
                    flip(&mut v.direction);
                }
            }
            s.flip_zone_lanes();
        }
        s
    }

    /// The street as seen from its other end: the pieces in the opposite order
    /// and every lane running the other way.
    pub fn reversed(&self) -> Street {
        let mut s = self.clone();
        s.segments.reverse();
        for seg in &mut s.segments {
            flip(&mut seg.direction);
            for v in &mut seg.variants {
                flip(&mut v.direction);
            }
        }
        s.reverse_zones();
        s.reverse_curb();
        s.reverse_stops();
        // a raised element spans the roadway: no side to swap, and its place stays measured from the same end
        s.reverse_green();
        s
    }

    /// Whether every index in it points into the catalogue, so that it can be
    /// drawn. A street that came from storage is checked before it is used.
    pub fn is_sound(&self) -> bool {
        self.row_mm > 0
            && self.section_is_sound(&self.segments)
            && self.zones_are_sound()
            && self.curb_is_sound()
            && self.stops_are_sound()
            && self.green_is_sound()
            && self.calming_is_sound()
    }

    /// Whether every piece of a section points into the catalogue and has a uid the street has issued.
    fn section_is_sound(&self, section: &[Segment]) -> bool {
        let seg_ok = |kind: usize, material: usize, curb: Option<usize>, direction: Option<usize>| {
            kind < KINDS.len()
                && KINDS[kind].materials.contains(&material)
                && material < MATERIALS.len()
                && curb.is_none_or(|c| c < CURBS.len() && KINDS[kind].has_curb)
                && direction.is_none_or(|d| d < DIRECTIONS.len())
        };
        !section.is_empty()
            && section.iter().all(|s| {
                // the variants are checked before the tram rule reads their kinds
                seg_ok(s.kind, s.material, s.curb, s.direction)
                    && s.variants.iter().all(|v| seg_ok(v.kind, v.material, None, v.direction) && valid_window(v.from_min, v.to_min))
                    && (!s.tram || KINDS[s.kind].id == "bus" || s.variants.iter().any(|v| KINDS[v.kind].id == "bus"))
                    && s.width_mm > 0
            })
            && section.iter().map(|s| s.uid).all(|u| u < self.next_uid)
    }

    /// The zones start at 0, rise strictly and end inside the length they were drawn against; each own section is sound.
    fn zones_are_sound(&self) -> bool {
        let Some(first) = self.zones.first() else { return true };
        let Some(length) = self.length_mm else { return false };
        first.from_mm == 0
            && self.zones.windows(2).all(|w| w[0].from_mm < w[1].from_mm)
            && self.zones.last().is_some_and(|z| z.from_mm < length)
            && self.zones.iter().filter_map(|z| z.segments.as_deref()).all(|s| self.section_is_sound(s))
    }
}

/// The width of a section: its pieces side by side.
pub fn total(segments: &[Segment]) -> i32 {
    segments.iter().map(|s| s.width_mm).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::kind_index;

    /// A local street of a sidewalk, a lane each way and a sidewalk, with nothing along it.
    fn plain() -> Street {
        let kind = |id| kind_index(id).unwrap();
        let mut away = Segment::new(2, kind("travel"), 3_300);
        away.direction = Some(0);
        let mut toward = Segment::new(3, kind("travel"), 3_300);
        toward.direction = Some(1);
        let segments = vec![Segment::new(1, kind("sidewalk"), 3_000), away, toward, Segment::new(4, kind("sidewalk"), 3_000)];
        Street {
            class: StreetClass::Local,
            class_before: None,
            name: Some("Main".into()),
            row_mm: total(&segments),
            side: Side::Right,
            segments,
            next_uid: 5,
            source: vec![SourceRef { id: 7, version: Some(3) }],
            zones: Vec::new(),
            from_end: ZoneEnd::First,
            length_mm: None,
            speed: Speed::Kmh(40),
            curb: Vec::new(),
            stops: Vec::new(),
            calming: Vec::new(),
            green: Vec::new(),
        }
    }

    #[test]
    fn a_street_with_nothing_along_it_is_kept_without_the_fields_it_does_not_use() {
        let json = serde_json::to_value(plain()).unwrap();
        let keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["class", "name", "next_uid", "row_mm", "segments", "side", "source", "speed_kmh"]);
        assert_eq!(json["source"], serde_json::json!([{"id": 7, "version": 3}]));
        assert_eq!(
            json["segments"][1],
            serde_json::json!({"uid": 2, "kind": 3, "width_mm": 3300, "material": 0, "curb": null, "direction": 0, "tram": false, "variants": []})
        );
        assert_eq!(serde_json::from_value::<Street>(json).unwrap(), plain());
    }

    #[test]
    fn a_speed_is_kept_as_speed_kmh_absent_when_undecided_null_when_there_is_none_or_a_number() {
        for (speed, written) in [(Speed::Undecided, None), (Speed::NotSet, Some("\"speed_kmh\":null")), (Speed::Kmh(30), Some("\"speed_kmh\":30"))] {
            let street = Street { speed, ..plain() };
            let json = serde_json::to_string(&street).unwrap();
            match written {
                Some(text) => assert!(json.contains(text), "{speed:?}: {json}"),
                None => assert!(!json.contains("speed_kmh"), "{speed:?}: {json}"),
            }
            assert_eq!(serde_json::from_str::<Street>(&json).unwrap(), street);
        }
    }

    #[test]
    fn a_street_with_zones_and_things_along_it_keeps_them_and_where_they_are_measured_from() {
        let mut s = plain();
        s.zones = vec![
            Zone { from_mm: 0, segments: None, name: None, speed: Speed::Undecided },
            Zone { from_mm: 30_000, segments: None, name: None, speed: Speed::Kmh(30) },
        ];
        s.from_end = ZoneEnd::Last;
        s.length_mm = Some(80_000);
        s.curb = vec![Instance { side: Side::Left, from_mm: 1_000, length_mm: 6_000, use_: Use::Parking }];
        s.stops = vec![Stop { side: Side::Right, from_mm: 2_000, length_mm: 12_000, kind: StopKind::Curb, shelter: true, name: None }];
        s.calming = vec![Calming { from_mm: 40_000, length_mm: 7_000, kind: CalmingKind::Table, first: None }];
        s.green = vec![Green { kind: GreenKind::Tree, from_mm: 5_000, length_mm: 1_500, offset_mm: 1_500, width_mm: 1_500, size: Some(TreeSize::M) }];
        assert!(s.is_sound());
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["from_end"], "Last");
        assert_eq!(json["zones"][1], serde_json::json!({"from_mm": 30000, "speed_kmh": 30}));
        assert_eq!(json["curb"][0]["use"], "parking");
        assert_eq!(json["calming"][0], serde_json::json!({"from_mm": 40000, "length_mm": 7000, "kind": "table"}));
        assert_eq!(json["green"][0]["size"], "M");
        assert_eq!(serde_json::from_value::<Street>(json).unwrap(), s);
    }

    #[test]
    fn seen_from_its_other_end_a_street_has_its_pieces_in_the_other_order_running_the_other_way() {
        let mut s = plain();
        s.segments[0].width_mm = 2_000;
        s.length_mm = Some(80_000);
        s.curb = vec![Instance { side: Side::Left, from_mm: 1_000, length_mm: 6_000, use_: Use::Loading }];
        let r = s.reversed();
        assert_eq!(r.segments.iter().map(|g| (g.uid, g.direction)).collect::<Vec<_>>(), [(4, None), (3, Some(0)), (2, Some(1)), (1, None)]);
        assert_eq!(r.curb[0].side, Side::Right, "a use keeps its place, on the other side");
        assert_eq!(r.from_end, ZoneEnd::Last, "positions stay measured from the same physical end");
        assert_eq!(r.reversed(), s);
    }

    #[test]
    fn a_street_whose_indices_leave_the_catalogue_or_whose_things_have_no_length_to_lie_along_is_not_sound() {
        assert!(plain().is_sound());
        let mut bad_kind = plain();
        bad_kind.segments[0].kind = crate::catalogue::KINDS.len();
        let mut bad_uid = plain();
        bad_uid.next_uid = 4;
        let mut no_length = plain();
        no_length.stops = vec![Stop { side: Side::Left, from_mm: 0, length_mm: 9_000, kind: StopKind::Bulb, shelter: false, name: None }];
        for s in [bad_kind, bad_uid, no_length] {
            assert!(!s.is_sound(), "{s:?}");
        }
    }

    #[test]
    fn a_street_with_no_motor_piece_is_pedestrian_and_gets_its_class_back_with_one() {
        let mut s = plain();
        s.segments.retain(|g| !g.has_motor());
        s.settle_class();
        assert_eq!((s.class, s.class_before), (StreetClass::Pedestrian, Some(StreetClass::Local)));
        s.segments.push(Segment::new(5, kind_index("travel").unwrap(), 3_300));
        s.next_uid = 6;
        s.settle_class();
        assert_eq!((s.class, s.class_before), (StreetClass::Local, None));
    }
}
