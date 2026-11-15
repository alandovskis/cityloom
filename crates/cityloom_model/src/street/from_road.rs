//! The street a road of the network is read as: its class, its pieces from the road's lanes, and the OSM ways it was
//! made from. The street editor brings each width inside what its kind allows when it builds the street from these
//! pieces (`StreetExt::imported` in the editor).

use crate::catalogue::{KINDS, StreetClass, kind_index};
use crate::{BufferType, Direction, Lane, LaneKind, Provenance, RoadType};

use super::{OsmRef, Piece, Window};

/// The class of a road, by its type.
pub fn class_of(road_type: RoadType) -> StreetClass {
    match road_type {
        RoadType::Motorway => StreetClass::Motorway,
        RoadType::Highway | RoadType::Arterial => StreetClass::Arterial,
        RoadType::Collector => StreetClass::Collector,
        RoadType::Local => StreetClass::Local,
    }
}

/// The catalogue kind that stands for a lane. A buffer is planting only when its type says it is planted or grass
/// (`Planters`, `Verge`); a curb, paint, posts, a barrier, a shoulder or a buffer of no known type is the neutral
/// `buffer`, a strip of concrete that is not green (a curb between a bike lane or sidewalk and a road lane is that
/// piece's own: see `pieces`).
/// Anything else (a turn lane, a rail track, the median between two carriageways) is a median: what is left of the
/// road when no lane uses it. A shoulder is not the catalogue's `shoulder`: that kind is a motor kind, and osm2streets
/// gives a lane anything may use that name, a shared path's included, which would make a footpath a street for cars.
fn kind_of(lane: &Lane) -> &'static str {
    match lane.kind {
        LaneKind::Buffer if matches!(lane.buffer_type, Some(BufferType::Planters | BufferType::Verge)) => "planting",
        kind => kind_id(kind),
    }
}

/// The catalogue kind a kind of lane is, a buffer being the neutral `buffer` (see `kind_of` for when it is planted).
fn kind_id(kind: LaneKind) -> &'static str {
    match kind {
        LaneKind::Driving => "travel",
        LaneKind::Parking => "parking",
        LaneKind::Sidewalk => "sidewalk",
        LaneKind::Bike => "bike",
        LaneKind::Bus => "bus",
        LaneKind::Buffer => "buffer",
        LaneKind::Other => "median",
    }
}

/// Whether a lane is a piece of the roadway a curb stands against.
fn is_road(lane: &Lane) -> bool {
    matches!(lane.kind, LaneKind::Driving | LaneKind::Parking | LaneKind::Bus)
}

/// Whether a lane has a curb of its own the editor draws against a road piece: a bike lane or a sidewalk.
fn has_own_curb(lane: &Lane) -> bool {
    matches!(lane.kind, LaneKind::Bike | LaneKind::Sidewalk)
}

/// The pieces importer version 4 made of the lanes: every buffer that is not planted a median of its own (so at least
/// 600 mm), and nothing merged. Only their width is kept, as `EdgeDef::legacy_rows_mm`.
pub fn legacy_pieces(lanes: &[Lane]) -> Vec<Piece> {
    let median = kind_index("median").unwrap_or_default();
    lanes.iter().map(|l| Piece { kind: if kind_of(l) == "buffer" { median } else { piece(l).kind }, ..piece(l) }).collect()
}

/// The lanes as they were read before the ways of a bike track were fetched: a lane that runs both ways, which that
/// made of a track beside a one-way road, is a one-way lane of half the width (the join made it of two). Known limits:
/// a side whose two lanes were not equal (a `cycleway:<side>:width`) gives no width that was, so its record stays an
/// orphan; and a lane that was two-way before (`cycleway:<side>:oneway=no`) is offered a width it never had.
pub fn one_way_bike_lanes(lanes: &[Lane]) -> Vec<Lane> {
    lanes
        .iter()
        .map(|l| {
            if l.kind == LaneKind::Bike && l.direction == Direction::Both {
                Lane { direction: Direction::Forward, width_m: l.width_m / 2.0, ..l.clone() }
            } else {
                l.clone()
            }
        })
        .collect()
}

/// The pieces of a road's lanes, left to right. A curb buffer between a bike lane or a sidewalk and a road lane is
/// that piece's own curb, as the editor has it (`curb`, which the section and the 2.5D stand against a road piece,
/// inside the piece's width): the buffer's width goes to the piece, up to its maximum. Any other curb buffer is a
/// `buffer`.
pub fn pieces(lanes: &[Lane]) -> Vec<Piece> {
    let is_curb = |l: &Lane| l.kind == LaneKind::Buffer && l.buffer_type == Some(BufferType::Curb);
    // the curb at `i` belongs to the bike lane or sidewalk on one side when a road lane is on the other
    let owner = |i: usize| -> Option<usize> {
        let (before, after) = (i.checked_sub(1).map(|j| &lanes[j])?, lanes.get(i + 1)?);
        match (has_own_curb(before), has_own_curb(after)) {
            (true, false) if is_road(after) => Some(i - 1),
            (false, true) if is_road(before) => Some(i + 1),
            _ => None,
        }
    };
    // what each owner can still take: its kind's maximum less its own width
    let mut room: Vec<i32> = lanes
        .iter()
        .map(|l| has_own_curb(l).then(|| kind_index(kind_of(l)).map_or(0, |k| KINDS[k].max_mm) - (l.width_m * 1000.0).round() as i32).unwrap_or(0).max(0))
        .collect();
    let buffer_min_mm = kind_index("buffer").map_or(0, |k| KINDS[k].min_mm);
    let mut merged: Vec<Lane> = lanes.to_vec();
    let mut gone = vec![false; lanes.len()];
    for i in (0..lanes.len()).filter(|&i| is_curb(&lanes[i])) {
        if let Some(owner) = owner(i) {
            let width_mm = (lanes[i].width_m * 1000.0).round() as i32;
            let taken = width_mm.min(room[owner]);
            room[owner] -= taken;
            merged[owner].width_m += f64::from(taken) / 1000.0;
            let rest = width_mm - taken;
            if rest == 0 {
                gone[i] = true;
            } else if rest < buffer_min_mm {
                // too little for a buffer: the road lane beside the curb takes it, so the street keeps its width
                merged[2 * i - owner].width_m += f64::from(rest) / 1000.0;
                gone[i] = true;
            } else {
                // what is past the maximum stays beside the piece as a buffer
                merged[i].width_m = f64::from(rest) / 1000.0;
            }
        }
    }
    merged.iter().zip(&gone).filter(|(_, gone)| !**gone).map(|(l, _)| piece(l)).collect()
}

fn piece(lane: &Lane) -> Piece {
    let kind = |id| kind_index(id).expect("the catalogue has every kind of lane");
    // what the lane is at certain hours, the rest of the day being its own kind
    let variants = lane
        .windows
        .iter()
        .map(|w| Window { kind: kind(kind_id(w.kind)), from_min: w.hours.from_min, to_min: w.hours.to_min, days: w.hours.days.clone() })
        .collect();
    Piece {
        kind: kind(kind_of(lane)),
        width_mm: (lane.width_m * 1000.0).round() as i32,
        direction: (lane.direction != Direction::Both || lane.kind != LaneKind::Bike).then_some(usize::from(lane.direction == Direction::Backward)),
        variants,
    }
}

/// The OSM ways or nodes with the version each had.
pub fn refs(p: &Provenance) -> Vec<OsmRef> {
    p.refs
        .iter()
        .map(|r| match r {
            crate::OsmRef::Way(w) => OsmRef { id: w.id, version: w.version },
            crate::OsmRef::Node(n) => OsmRef { id: n.id, version: n.version },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lane(kind: LaneKind, way: Direction, width_m: f64) -> Lane {
        Lane { kind, direction: way, width_m, buffer_type: None, windows: vec![], shape: vec![] }
    }

    fn bus_lane(hours: Option<&str>) -> Lane {
        let mut lane = lane(LaneKind::Bus, Direction::Forward, 3.2);
        if let Some(hours) = hours {
            lane.only_during(hours, LaneKind::Parking);
        }
        lane
    }

    fn windows(p: &Piece) -> Vec<(&'static str, i32, i32)> {
        p.variants.iter().map(|w| (KINDS[w.kind].id, w.from_min, w.to_min)).collect()
    }

    fn buffer(buffer_type: Option<BufferType>) -> Lane {
        Lane { buffer_type, ..lane(LaneKind::Buffer, Direction::Forward, 0.6) }
    }

    fn kind_id(lane: &Lane) -> &'static str {
        KINDS[piece(lane).kind].id
    }

    fn ids(lanes: &[Lane]) -> Vec<&'static str> {
        pieces(lanes).iter().map(|p| KINDS[p.kind].id).collect()
    }

    fn widths(lanes: &[Lane]) -> Vec<i32> {
        pieces(lanes).iter().map(|p| p.width_mm).collect()
    }

    fn curb() -> Lane {
        buffer(Some(BufferType::Curb))
    }

    fn bike() -> Lane {
        lane(LaneKind::Bike, Direction::Forward, 1.8)
    }

    fn driving() -> Lane {
        lane(LaneKind::Driving, Direction::Forward, 3.3)
    }

    #[test]
    fn a_two_way_bike_lane_has_no_direction_and_a_one_way_one_keeps_it() {
        assert_eq!(piece(&lane(LaneKind::Bike, Direction::Both, 3.0)).direction, None);
        assert_eq!(piece(&lane(LaneKind::Bike, Direction::Forward, 1.5)).direction, Some(0));
        assert_eq!(piece(&lane(LaneKind::Bike, Direction::Backward, 1.5)).direction, Some(1));
        // only a bike lane can run both ways: a driving lane always has a direction
        assert_eq!(piece(&lane(LaneKind::Driving, Direction::Both, 3.0)).direction, Some(0));
    }

    #[test]
    fn a_buffer_is_planting_only_when_it_is_planted_or_grass() {
        for t in [BufferType::Planters, BufferType::Verge] {
            assert_eq!(kind_id(&buffer(Some(t))), "planting", "{t:?}");
        }
    }

    #[test]
    fn any_other_buffer_and_one_of_no_known_type_is_the_neutral_buffer_and_not_a_median() {
        let others = [None, Some(BufferType::Curb), Some(BufferType::Stripes), Some(BufferType::FlexPosts), Some(BufferType::JerseyBarrier)];
        for t in others {
            assert_eq!(kind_id(&buffer(t)), "buffer", "{t:?}");
        }
    }

    #[test]
    fn a_curb_buffer_with_no_bike_lane_or_sidewalk_to_belong_to_is_the_neutral_buffer() {
        // beside a driving lane only, at the edge, between two sidewalks, two bike lanes, or a bike lane and a sidewalk
        let sidewalk = || lane(LaneKind::Sidewalk, Direction::Forward, 2.0);
        assert_eq!(ids(&[driving(), curb(), driving()]), ["travel", "buffer", "travel"]);
        assert_eq!(ids(&[curb(), driving()]), ["buffer", "travel"]);
        assert_eq!(ids(&[sidewalk(), curb(), sidewalk()]), ["sidewalk", "buffer", "sidewalk"]);
        assert_eq!(ids(&[bike(), curb(), bike()]), ["bike", "buffer", "bike"]);
        assert_eq!(ids(&[bike(), curb(), sidewalk()]), ["bike", "buffer", "sidewalk"], "no road for the curb to stand against");
        assert_eq!(widths(&[bike(), curb(), sidewalk()]), [1800, 600, 2000]);
        assert_eq!(ids(&[sidewalk(), curb(), bike()]), ["sidewalk", "buffer", "bike"]);
    }

    #[test]
    fn paint_posts_a_barrier_and_a_shoulder_beside_a_bike_lane_stay_buffers_of_their_own() {
        for t in [Some(BufferType::Stripes), Some(BufferType::FlexPosts), Some(BufferType::JerseyBarrier), None] {
            let lanes = [bike(), buffer(t), driving()];
            assert_eq!(ids(&lanes), ["bike", "buffer", "travel"], "{t:?}");
            assert_eq!(widths(&lanes), [1800, 600, 3300], "{t:?}");
        }
        let shoulder = lane(LaneKind::Buffer, Direction::Forward, 1.0);
        assert_eq!(ids(&[shoulder, driving()]), ["buffer", "travel"]);
    }

    #[test]
    fn the_other_lanes_keep_their_kinds() {
        let kinds = [
            (LaneKind::Driving, "travel"),
            (LaneKind::Parking, "parking"),
            (LaneKind::Sidewalk, "sidewalk"),
            (LaneKind::Bike, "bike"),
            (LaneKind::Bus, "bus"),
            (LaneKind::Other, "median"),
        ];
        for (kind, id) in kinds {
            assert_eq!(kind_id(&lane(kind, Direction::Forward, 3.0)), id, "{kind:?}");
        }
    }

    #[test]
    fn the_days_of_the_hours_go_with_each_window() {
        let days = |hours| piece(&bus_lane(Some(hours))).variants.iter().map(|w| w.days.clone()).collect::<Vec<_>>();
        assert_eq!(days("Mo-Fr 06:00-10:00,14:30-19:00"), [Some("Mo-Fr".to_string()), Some("Mo-Fr".to_string())]);
        assert_eq!(days("Sa,Su 08:00-12:00"), [Some("Sa,Su".to_string())]);
        assert_eq!(days("06:00-10:00"), [None]);
    }

    #[test]
    fn a_bus_lane_with_hours_is_parking_outside_them() {
        let p = piece(&bus_lane(Some("Mo-Fr 06:00-10:00,14:30-19:00")));
        assert_eq!(KINDS[p.kind].id, "parking");
        assert_eq!(windows(&p), [("bus", 360, 600), ("bus", 870, 1140)]);
    }

    #[test]
    fn a_bus_lane_without_hours_or_with_hours_that_cannot_be_read_stays_a_bus_lane() {
        for hours in [None, Some("Sa,Su 07:00-09:00; Mo-Fr 06:00-10:00"), Some("sunrise-sunset"), Some("Mo-Fr")] {
            let p = piece(&bus_lane(hours));
            assert_eq!((KINDS[p.kind].id, p.variants.len()), ("bus", 0), "{hours:?}");
        }
    }

    #[test]
    fn a_road_is_classed_by_its_type() {
        use crate::catalogue::StreetClass::*;
        for (road_type, class) in [
            (RoadType::Motorway, Motorway),
            (RoadType::Highway, Arterial),
            (RoadType::Arterial, Arterial),
            (RoadType::Collector, Collector),
            (RoadType::Local, Local),
        ] {
            assert_eq!(class_of(road_type), class, "{road_type:?}");
        }
    }
}
