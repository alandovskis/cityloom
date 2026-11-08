//! The curbside uses at a street's end, as an arm of a junction reads them: in the arm's orientation, measured from
//! the junction's centre, with the piece each stands on. Derived, never saved. Pure, no browser types.

use super::curb::{Use, curbside};
use super::{LENGTH_TOLERANCE_MM, Street, ZoneEnd, ZoneFrame};
use crate::shared::catalogue::Side;

/// One curbside use in a street's end zone, as the arm that ends there reads it. Everything is in the arm's orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndUse {
    /// The use's index in the street's `curb`.
    pub index: usize,
    /// The arm's side: swapped when the arm is at the street's last end.
    pub side: Side,
    pub use_: Use,
    /// Millimetres from the street's end (the junction's centre) to the part of the use inside the end zone: its near
    /// and far ends. A use that crosses the zone's boundary is cut there.
    pub near_mm: i32,
    pub far_mm: i32,
    /// The piece the use stands on, an index into the end zone's section as the arm reads it (reversed at the last
    /// end); `None` where that side has no curbside piece to stand on, so nothing is drawn.
    pub piece: Option<usize>,
}

impl Street {
    /// The curbside uses that have a part in the end zone at the street's first end (`first_end`) or its last, nearest
    /// first. How far the arm is drawn is the junction's to decide: a use past it is returned all the same. Empty
    /// with no frame, a frame of another road (another length or anchored at the other end) or no uses.
    pub fn end_uses(&self, first_end: bool, frame: Option<ZoneFrame>) -> Vec<EndUse> {
        let Some(frame) = frame else { return Vec::new() };
        // A street with uses has a recorded length, so `None` cannot match.
        let same_length = self.length_mm.is_some_and(|drawn| (drawn - frame.length_mm).abs() <= LENGTH_TOLERANCE_MM);
        if self.curb.is_empty() || frame.from_end != self.from_end || !same_length {
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
        let mut found: Vec<EndUse> = self
            .curb
            .iter()
            .enumerate()
            .filter_map(|(index, i)| {
                let (from, to) = (i.from_mm.max(range.0), (i.from_mm + i.length_mm).min(range.1));
                if from >= to {
                    return None;
                }
                let (near_mm, far_mm) = if anchored { (from, to) } else { (frame.length_mm - to, frame.length_mm - from) };
                let side = if first_end {
                    i.side
                } else {
                    match i.side {
                        Side::Left => Side::Right,
                        Side::Right => Side::Left,
                    }
                };
                Some(EndUse { index, side, use_: i.use_, near_mm, far_mm, piece: curbside(&section.segments, side) })
            })
            .collect();
        found.sort_by_key(|u| (u.near_mm, u.index));
        found
    }
}

#[cfg(test)]
mod tests {
    use super::super::Instance;
    use super::super::fixtures::zoned;
    use super::*;
    use crate::shared::catalogue::{KINDS, kind_index};

    const LEN: i32 = 60_000;

    fn frame(from_end: ZoneEnd) -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LEN, from_end })
    }

    fn inst(side: Side, from_mm: i32, length_mm: i32, use_: Use) -> Instance {
        Instance { side, from_mm, length_mm, use_ }
    }

    /// The right-hand sample (sidewalk, parking, travel toward, travel away, parking, sidewalk) with `curb`, zoned at
    /// `starts`, anchored at `from_end`, 60 m long.
    fn street(curb: Vec<Instance>, starts: &[i32], from_end: ZoneEnd) -> Street {
        let mut s = zoned(Street::sample(0, Side::Right), starts, LEN);
        s.length_mm = Some(LEN);
        s.from_end = from_end;
        s.curb = curb;
        s
    }

    #[test]
    fn a_street_without_uses_has_none() {
        assert!(street(Vec::new(), &[], ZoneEnd::First).end_uses(true, frame(ZoneEnd::First)).is_empty());
    }

    #[test]
    fn a_use_at_the_first_end_is_on_the_arm_with_its_piece() {
        let s = street(vec![inst(Side::Right, 12_000, 6_000, Use::Parking)], &[], ZoneEnd::First);
        let e = s.end_uses(true, frame(ZoneEnd::First));
        assert_eq!(e.len(), 1);
        assert_eq!((e[0].side, e[0].near_mm, e[0].far_mm, e[0].index), (Side::Right, 12_000, 18_000, 0));
        assert_eq!(e[0].piece, Some(4));
        assert_eq!(KINDS[s.segments[4].kind].id, "parking");
    }

    #[test]
    fn an_arm_at_the_last_end_swaps_the_side_and_measures_from_it() {
        let s = street(vec![inst(Side::Right, 30_000, 6_000, Use::Tree)], &[], ZoneEnd::First);
        let e = s.end_uses(false, frame(ZoneEnd::First));
        assert_eq!((e[0].side, e[0].near_mm, e[0].far_mm), (Side::Left, 24_000, 30_000));
        // The arm reads the section reversed: its left parking piece is the sample's right one.
        let arm = s.at_end(false).reversed();
        assert_eq!(e[0].piece, Some(1));
        assert_eq!(arm.segments[1].uid, s.segments[4].uid);
    }

    #[test]
    fn a_street_anchored_at_its_last_end_measures_from_the_other_end() {
        let s = street(vec![inst(Side::Right, 30_000, 6_000, Use::Loading)], &[], ZoneEnd::Last);
        let at_first = s.end_uses(true, frame(ZoneEnd::Last));
        assert_eq!((at_first[0].near_mm, at_first[0].far_mm), (LEN - 36_000, LEN - 30_000));
        let at_last = s.end_uses(false, frame(ZoneEnd::Last));
        assert_eq!((at_last[0].near_mm, at_last[0].far_mm, at_last[0].side), (30_000, 36_000, Side::Left));
    }

    #[test]
    fn only_the_part_inside_the_end_zone_is_on_the_arm() {
        let curb =
            vec![inst(Side::Right, 5_000, 6_000, Use::Parking), inst(Side::Right, 17_000, 6_000, Use::Parking), inst(Side::Right, 40_000, 6_000, Use::Parking)];
        let s = street(curb, &[0, 20_000], ZoneEnd::First);
        let near: Vec<_> = s.end_uses(true, frame(ZoneEnd::First)).iter().map(|e| (e.index, e.near_mm, e.far_mm)).collect();
        assert_eq!(near, vec![(0, 5_000, 11_000), (1, 17_000, 20_000)], "the use across the boundary is cut at it");
        let far: Vec<_> = s.end_uses(false, frame(ZoneEnd::First)).iter().map(|e| (e.index, e.near_mm, e.far_mm)).collect();
        assert_eq!(far, vec![(2, 14_000, 20_000), (1, 37_000, 40_000)], "at the last end the end zone is the last one, nearest first");
    }

    #[test]
    fn a_side_with_no_curbside_piece_has_no_piece() {
        let mut s = street(vec![inst(Side::Right, 12_000, 6_000, Use::Planting)], &[], ZoneEnd::First);
        s.segments[4].kind = kind_index("sidewalk").unwrap();
        let e = s.end_uses(true, frame(ZoneEnd::First));
        assert_eq!((e.len(), e[0].piece), (1, None), "still listed, nothing to draw it on");
    }

    #[test]
    fn a_frame_that_does_not_match_gives_no_uses() {
        let s = street(vec![inst(Side::Right, 12_000, 6_000, Use::Parking)], &[], ZoneEnd::First);
        assert_eq!(s.end_uses(true, frame(ZoneEnd::First)).len(), 1);
        let other_length = Some(ZoneFrame { length_mm: LEN + 2_000, from_end: ZoneEnd::First });
        assert!(s.end_uses(true, other_length).is_empty());
        assert!(s.end_uses(true, frame(ZoneEnd::Last)).is_empty(), "anchored at the other end");
        assert!(s.end_uses(true, None).is_empty());
    }
}
