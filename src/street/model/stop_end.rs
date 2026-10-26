//! The transit stops at a street's end, as an arm of a junction reads them: in the arm's orientation, measured
//! from the junction's centre, with the lane the bus uses and the pieces the stop stands on. Derived, never saved.
//! Pure, no browser types.

use super::stop::{StopKind, bus_lane, stand};
use super::{LENGTH_TOLERANCE_MM, Street, ZoneEnd, ZoneFrame};
use crate::shared::catalogue::Side;

/// How far from the junction's centre an arm is drawn. The junction's `ARM_LENGTH_MM` (36 000), repeated: the street
/// slice must not depend on the junction's.
const END_REACH_MM: i32 = 36_000;

/// How far past the end of a stop's reach the junction wants it, so a queue or a turn does not block it.
/// UNVERIFIED: the same value as the crossing clearance (`CROSSING_CLEARANCE_MM`), not taken from a standard.
pub const STOP_CLEAR_MM: i32 = 5_000;

/// The distance between a stop and what reaches `reach_mm` from the junction's centre: how far past the reach the
/// stop's near end lies (negative when the stop is inside it).
pub fn stop_gap_mm(near_mm: i32, reach_mm: i32) -> i32 {
    near_mm - reach_mm
}

/// One stop in a street's end zone, as the arm that ends there reads it. Everything is in the arm's orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndStop {
    /// The stop's index in the street's `stops`.
    pub index: usize,
    /// The arm's side: swapped when the arm is at the street's last end.
    pub side: Side,
    /// Millimetres from the street's end (the junction's centre) to the stop's near and far ends.
    pub near_mm: i32,
    pub far_mm: i32,
    pub kind: StopKind,
    pub shelter: bool,
    pub name: Option<String>,
    /// Whether the bus's half runs toward the junction.
    pub enters: bool,
    /// The piece the bus uses, an index into the end zone's section as the arm reads it (reversed at the last end).
    pub lane: Option<usize>,
    /// The pieces the stop stands on, in the same section.
    pub pad: Vec<usize>,
}

impl Street {
    /// The stops wholly inside the end zone at the street's first end (`first_end`) or its last, within the arm's
    /// reach, nearest first. Empty with no frame, a frame of another road (another length or anchored at the other
    /// end) or no stops.
    pub fn end_stops(&self, first_end: bool, frame: Option<ZoneFrame>) -> Vec<EndStop> {
        let Some(frame) = frame else { return Vec::new() };
        let same_length = self.length_mm.is_some_and(|drawn| (drawn - frame.length_mm).abs() <= LENGTH_TOLERANCE_MM);
        if self.stops.is_empty() || frame.from_end != self.from_end || !same_length {
            return Vec::new();
        }
        // The end zone in millimetres from the anchored end, as `at_end` picks it.
        let anchored = (self.from_end == ZoneEnd::First) == first_end;
        let range = match self.zones.last() {
            Some(_) if anchored => (0, self.zones.get(1).map_or(frame.length_mm, |z| z.from_mm)),
            Some(last) => (last.from_mm, frame.length_mm),
            None => (0, frame.length_mm),
        };
        let mut section = self.at_end(first_end);
        if !first_end {
            section = section.reversed();
        }
        let mut found: Vec<EndStop> = self
            .stops
            .iter()
            .enumerate()
            .filter(|(_, s)| s.from_mm >= range.0 && s.to_mm() <= range.1)
            .map(|(index, s)| {
                let near_mm = if anchored { s.from_mm } else { frame.length_mm - s.to_mm() };
                let side = if first_end {
                    s.side
                } else {
                    match s.side {
                        Side::Left => Side::Right,
                        Side::Right => Side::Left,
                    }
                };
                let lane = bus_lane(&section.segments, side, section.side);
                let pad = lane
                    .and_then(|l| stand(&section.segments, l, side, s.kind))
                    .map(|p| if p.raised.is_empty() { p.stand.into_iter().collect() } else { p.raised })
                    .unwrap_or_default();
                EndStop {
                    index,
                    side,
                    near_mm,
                    far_mm: near_mm + s.length_mm,
                    kind: s.kind,
                    shelter: s.shelter,
                    name: s.name.clone(),
                    enters: side != section.side,
                    lane,
                    pad,
                }
            })
            .filter(|s| s.near_mm < END_REACH_MM)
            .collect();
        found.sort_by_key(|s| s.near_mm);
        found
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::zoned;
    use super::super::{Segment, Stop};
    use super::*;
    use crate::shared::catalogue::{KINDS, kind_index};

    const LEN: i32 = 60_000;

    fn frame(from_end: ZoneEnd) -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LEN, from_end })
    }

    fn stop(side: Side, from_mm: i32, length_mm: i32, kind: StopKind) -> Stop {
        Stop { side, from_mm, length_mm, kind, shelter: false, name: None }
    }

    /// The right-hand sample (sidewalk, parking, travel toward, travel away, parking, sidewalk) with `stops`,
    /// zoned at `starts`, anchored at `from_end`, 60 m long.
    fn street(stops: Vec<Stop>, starts: &[i32], from_end: ZoneEnd) -> Street {
        let mut s = zoned(Street::sample(0, Side::Right), starts, LEN);
        // Settling clears the length of a street with nothing placed along it; stops are set after it.
        s.length_mm = Some(LEN);
        s.from_end = from_end;
        s.stops = stops;
        s
    }

    fn one(stops: Vec<Stop>) -> Street {
        street(stops, &[], ZoneEnd::First)
    }

    fn kind_of(s: &Street, i: Option<usize>) -> &'static str {
        KINDS[s.segments[i.expect("a lane")].kind].id
    }

    #[test]
    fn a_street_without_stops_has_none() {
        assert!(one(Vec::new()).end_stops(true, frame(ZoneEnd::First)).is_empty());
    }

    #[test]
    fn a_stop_at_the_first_end_is_on_the_arm() {
        let s = one(vec![stop(Side::Right, 12_000, 10_000, StopKind::Curb)]);
        let e = s.end_stops(true, frame(ZoneEnd::First));
        assert_eq!(e.len(), 1);
        assert_eq!((e[0].side, e[0].near_mm, e[0].far_mm, e[0].enters, e[0].index), (Side::Right, 12_000, 22_000, false, 0));
        assert_eq!(e[0].lane, Some(3));
        assert_eq!(s.segments[3].direction, Some(0), "the piece going away");
    }

    #[test]
    fn the_left_side_enters_with_traffic_on_the_right() {
        let s = one(vec![stop(Side::Left, 12_000, 10_000, StopKind::Curb)]);
        let e = s.end_stops(true, frame(ZoneEnd::First));
        assert!(e[0].enters);
        assert_eq!(e[0].lane, Some(2));
        assert_eq!(s.segments[2].direction, Some(1), "the piece toward the junction");
    }

    #[test]
    fn an_arm_at_the_last_end_swaps_the_side_and_measures_from_it() {
        let s = one(vec![stop(Side::Right, 30_000, 10_000, StopKind::Curb)]);
        let e = s.end_stops(false, frame(ZoneEnd::First));
        assert_eq!((e[0].side, e[0].near_mm, e[0].far_mm), (Side::Left, 20_000, 30_000));
        assert!(e[0].enters, "the lane that runs away from the first end runs toward the last");
        // The section the arm reads is reversed: its left travel piece is the sample's right one.
        let arm = s.at_end(false).reversed();
        assert_eq!(e[0].lane, Some(2));
        assert_eq!(arm.segments[2].direction, Some(1));
        assert_eq!(arm.segments[2].uid, s.segments[3].uid);
    }

    #[test]
    fn a_street_anchored_at_its_last_end_measures_from_the_other_end() {
        let s = street(vec![stop(Side::Right, 30_000, 10_000, StopKind::Curb)], &[], ZoneEnd::Last);
        let at_first = s.end_stops(true, frame(ZoneEnd::Last));
        assert_eq!((at_first[0].near_mm, at_first[0].far_mm), (LEN - 40_000, LEN - 30_000));
        let at_last = s.end_stops(false, frame(ZoneEnd::Last));
        assert_eq!((at_last[0].near_mm, at_last[0].side), (30_000, Side::Left));
    }

    #[test]
    fn only_a_stop_inside_the_end_zone_is_on_the_arm() {
        let stops = vec![
            stop(Side::Right, 5_000, 10_000, StopKind::Curb),
            stop(Side::Right, 15_000, 10_000, StopKind::Curb),
            stop(Side::Right, 40_000, 10_000, StopKind::Curb),
        ];
        let zoned_street = street(stops.clone(), &[0, 20_000], ZoneEnd::First);
        let near: Vec<usize> = zoned_street.end_stops(true, frame(ZoneEnd::First)).iter().map(|e| e.index).collect();
        assert_eq!(near, vec![0], "the stop across the boundary and the one past it are not on the arm");
        let far: Vec<usize> = zoned_street.end_stops(false, frame(ZoneEnd::First)).iter().map(|e| e.index).collect();
        assert_eq!(far, vec![2], "at the last end the end zone is the last one");
        let unzoned: Vec<usize> = one(stops).end_stops(true, frame(ZoneEnd::First)).iter().map(|e| e.index).collect();
        assert_eq!(unzoned, vec![0, 1], "no zones: the whole road, within the arm's reach");
    }

    #[test]
    fn a_stop_past_the_drawn_arm_is_left_off() {
        let far = |from_mm| one(vec![stop(Side::Right, from_mm, 9_000, StopKind::Curb)]).end_stops(true, frame(ZoneEnd::First)).len();
        assert_eq!(far(36_000), 0);
        assert_eq!(far(35_900), 1);
    }

    #[test]
    fn a_frame_that_does_not_match_gives_no_stops() {
        let s = one(vec![stop(Side::Right, 12_000, 10_000, StopKind::Curb)]);
        assert_eq!(s.end_stops(true, frame(ZoneEnd::First)).len(), 1);
        let other_length = Some(ZoneFrame { length_mm: LEN + 2_000, from_end: ZoneEnd::First });
        assert!(s.end_stops(true, other_length).is_empty());
        assert!(s.end_stops(true, frame(ZoneEnd::Last)).is_empty(), "anchored at the other end");
        assert!(s.end_stops(true, None).is_empty());
    }

    /// The sample with the sample's right travel piece a bus lane.
    fn bus_street(stops: Vec<Stop>) -> Street {
        let mut s = one(stops);
        s.segments[3] = Segment { kind: kind_index("bus").unwrap(), ..s.segments[3].clone() };
        s
    }

    #[test]
    fn a_bus_lane_stop_names_the_bus_piece() {
        let s = bus_street(vec![stop(Side::Right, 12_000, 10_000, StopKind::Curb)]);
        let e = s.end_stops(true, frame(ZoneEnd::First));
        assert_eq!(kind_of(&s, e[0].lane), "bus");
    }

    #[test]
    fn a_stop_with_no_lane_has_none() {
        let mut s = one(vec![stop(Side::Right, 12_000, 10_000, StopKind::Curb)]);
        for g in &mut s.segments {
            g.kind = kind_index("sidewalk").unwrap();
            g.direction = None;
        }
        let e = s.end_stops(true, frame(ZoneEnd::First));
        assert_eq!(e.len(), 1, "still listed");
        assert_eq!((e[0].lane, e[0].pad.is_empty()), (None, true));
    }

    #[test]
    fn pad_is_the_platform_pieces() {
        let mut s = one(vec![stop(Side::Right, 12_000, 10_000, StopKind::Bulb), stop(Side::Left, 12_000, 10_000, StopKind::Curb)]);
        // No parking on the left, so the bus meets the curb there.
        s.segments[1].kind = kind_index("sidewalk").unwrap();
        let e = s.end_stops(true, frame(ZoneEnd::First));
        let by = |side| e.iter().find(|x| x.side == side).unwrap();
        assert_eq!(by(Side::Right).pad, vec![4], "a bulb stands on the parking piece");
        assert_eq!(kind_of(&s, Some(4)), "parking");
        assert_eq!(by(Side::Left).pad, vec![1], "a curb stop stands on the sidewalk beside the lane");
    }

    #[test]
    fn stop_gap_is_distance_past_the_reach() {
        assert_eq!(stop_gap_mm(14_000, 9_000), 5_000);
        assert!(stop_gap_mm(4_000, 9_000) < 0);
    }
}
