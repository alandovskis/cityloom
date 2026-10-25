//! What lies beyond a street's end zone, for the junction: the first boundary met going out from the junction, the
//! zone across it, and the taper that boundary needs. Derived, never saved. Pure, no browser types.
//!
//! Distances are along the arm from the junction's centre, which is the street's end (the mouth is not subtracted:
//! `junction::read_model` draws an arm from its mouth to `ARM_LENGTH_MM`, both measured from the centre).

use super::zones::same_pieces;
use super::{Host, LENGTH_TOLERANCE_MM, Segment, Street, ZoneEnd, ZoneFrame, transition};
use crate::shared::catalogue::is_roadway;

/// The first boundary met going out from an arm's end, and what is across it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beyond {
    /// Millimetres from the junction's centre to the boundary.
    pub boundary_mm: i32,
    /// The zone across the boundary as the arm sees it: looking out from the junction.
    pub next: Vec<Segment>,
    /// The taper the boundary needs, when the two sections need one (otherwise the boundary is a jog).
    pub taper: Option<BeyondTaper>,
}

/// The transition at the boundary, in distances from the junction's centre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BeyondTaper {
    /// The taper runs from `from_mm` to `to_mm`; `from_mm` is the nearer to the junction.
    pub from_mm: i32,
    pub to_mm: i32,
    /// The length the transition needs, and the length of the host zone that has to hold it.
    pub need_mm: i32,
    pub room_mm: i32,
    /// False when a lane ends with no lane beside it to merge into.
    pub merges: bool,
    pub pieces: Vec<TaperPiece>,
}

/// One roadway piece that differs across the boundary: its `[x0, x1)` across the arm, in millimetres from the arm's
/// left edge looking outward.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaperPiece {
    pub kind: usize,
    /// The span at `from_mm` and at `to_mm`; zero-width where the piece is absent on that side.
    pub from: (i32, i32),
    pub to: (i32, i32),
    /// Where the end zone has the piece (`None` where it lacks it), and where the zone across has it.
    pub was: Option<(i32, i32)>,
    pub becomes: Option<(i32, i32)>,
}

impl Street {
    /// What lies beyond the end of the street at its first end (`first_end`) or its last: the first zone boundary met
    /// going out from the end, with the zone across it and the taper it needs. `None` for a street of one zone, with
    /// no frame, with a zoning drawn on another road (the page's "elsewhere", as `Editor::elsewhere` decides it), when
    /// the boundary is at or past the road's end, or when the two sections have the same pieces.
    pub fn beyond(&self, first_end: bool, frame: Option<ZoneFrame>) -> Option<Beyond> {
        let frame = frame?;
        let n = self.zones.len();
        if n < 2 || frame.from_end != self.from_end || self.length_mm.is_some_and(|drawn| (drawn - frame.length_mm).abs() > LENGTH_TOLERANCE_MM) {
            return None;
        }
        // As `at_end` picks the end zone; the neighbour is the one beside it.
        let anchored = (self.from_end == ZoneEnd::First) == first_end;
        let (end, neighbour) = if anchored { (0, 1) } else { (n - 1, n - 2) };
        let b = end.min(neighbour);
        let at = self.zones[b + 1].from_mm;
        if at >= frame.length_mm {
            return None;
        }
        let sections = self.sections();
        if same_pieces(sections[end], sections[neighbour]) {
            return None;
        }
        // Measured from the anchored end; the arm measures from its own, the junction's centre.
        let out = |lo: i32, hi: i32| if anchored { (lo, hi) } else { (frame.length_mm - hi, frame.length_mm - lo) };
        let boundary_mm = out(at, at).0;
        let mirrored = !first_end;
        let taper = transition(sections[b], sections[b + 1], self.speed.kmh(), self.from_end).map(|t| {
            let (host, interval) = match t.host {
                Host::Near => (b, ((at - t.length_mm).max(self.zones[b].from_mm), at)),
                Host::Far => (b + 1, (at, (at + t.length_mm).min(self.zones.get(b + 2).map_or(frame.length_mm, |z| z.from_mm).min(frame.length_mm)))),
            };
            let host_end = self.zones.get(host + 1).map_or(frame.length_mm, |z| z.from_mm).min(frame.length_mm);
            let (from_mm, to_mm) = out(interval.0, interval.1);
            // `before` is the nearer-the-anchor zone's side: the end zone's when that is zone `b`.
            let end_is_near = end == b;
            let mut changes: Vec<(usize, Option<(i32, i32)>, Option<(i32, i32)>)> =
                t.changes.iter().map(|c| if end_is_near { (c.kind, c.before, c.after) } else { (c.kind, c.after, c.before) }).collect();
            if mirrored {
                let flip = |r: Option<(i32, i32)>| r.map(|(x0, x1)| (self.row_mm - x1, self.row_mm - x0));
                changes = changes.into_iter().rev().map(|(k, was, becomes)| (k, flip(was), flip(becomes))).collect();
            }
            let pieces = changes
                .iter()
                .enumerate()
                .filter(|(_, (kind, was, becomes))| is_roadway(*kind) && was != becomes)
                .filter_map(|(i, &(kind, was, becomes))| {
                    let from = was.or_else(|| anchor(&changes, i, |c| c.1))?;
                    let to = becomes.or_else(|| anchor(&changes, i, |c| c.2))?;
                    Some(TaperPiece { kind, from, to, was, becomes })
                })
                .collect();
            BeyondTaper { from_mm, to_mm, need_mm: t.length_mm, room_mm: host_end - self.zones[host].from_mm, merges: t.merges, pieces }
        });
        let mut across = Street { segments: sections[neighbour].to_vec(), zones: Vec::new(), ..self.clone() };
        if !first_end {
            across = across.reversed();
        }
        Some(Beyond { boundary_mm, next: across.segments, taper })
    }
}

type Row = (usize, Option<(i32, i32)>, Option<(i32, i32)>);

/// A zero-width span at the nearest matched piece's edge on one side of the boundary, for a piece that has no place
/// there: the right edge of the matched piece before it, else the left edge of the one after it (as `iso/taper.rs`).
fn anchor(changes: &[Row], at: usize, side: fn(&Row) -> Option<(i32, i32)>) -> Option<(i32, i32)> {
    let matched = |c: &Row| c.1.is_some() && c.2.is_some();
    let x = changes[..at]
        .iter()
        .rev()
        .find(|c| matched(c))
        .and_then(side)
        .map(|r| r.1)
        .or_else(|| changes[at + 1..].iter().find(|c| matched(c)).and_then(side).map(|r| r.0))?;
    Some((x, x))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{Side, kind_index};
    use crate::street::model::{Speed, zoned};
    use ZoneEnd::{First, Last};

    const LEN: i32 = 100_000;

    fn frame(end: ZoneEnd) -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LEN, from_end: end })
    }

    fn lane(uid: u32) -> Segment {
        let mut l = Segment::new(uid, kind_index("travel").unwrap(), 3_500);
        l.direction = Some(0);
        l
    }

    /// The sample street at 50 km/h with zones at 0 and `at`; zone `own` (0 or 1) has a travel lane more than the base.
    fn with_extra_lane(at: i32, own: usize) -> Street {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, at], LEN);
        s.speed = Speed::Kmh(50);
        let mut section = s.segments.clone();
        section.push(lane(900));
        s.zones[own].segments = Some(section);
        s.row_mm += 3_500;
        s
    }

    #[test]
    fn a_street_of_one_zone_has_no_beyond() {
        let s = Street::sample(0, Side::Right);
        assert_eq!(s.beyond(true, frame(First)), None);
    }

    #[test]
    fn identical_sections_have_no_beyond() {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 20_000], LEN);
        let mut same = s.segments.clone();
        for p in &mut same {
            p.uid += 100;
        }
        s.zones[1].segments = Some(same);
        assert_eq!(s.beyond(true, frame(First)), None);
        assert_eq!(s.beyond(false, frame(First)), None);
    }

    #[test]
    fn a_pocket_zone_gives_a_boundary_and_a_taper_in_the_next_zone() {
        let s = with_extra_lane(20_000, 0);
        let b = s.beyond(true, frame(First)).expect("the end zone differs from the next");
        assert_eq!(b.boundary_mm, 20_000, "the distance from the street's end, which is the junction's centre");
        assert_eq!(b.next, s.segments);
        let t = b.taper.expect("a lane ends");
        // a dropped lane is a merge taper: W·S²/155 = 3 500 · 2 500 / 155 = 56 452, up to 56 500
        assert_eq!((t.from_mm, t.to_mm, t.need_mm), (20_000, 76_500, 56_500));
        assert_eq!(t.room_mm, 80_000, "the host is the zone across, 80 m long");
        assert!(!t.merges, "the lane ends beside a sidewalk, with no lane of its direction to merge into");
        let [p] = &t.pieces[..] else { panic!("one piece differs: {:?}", t.pieces) };
        let was = (18_000, 21_500);
        assert_eq!(p.was, Some(was));
        assert_eq!(p.becomes, None);
        assert_eq!(p.from, was);
        assert_eq!(p.to, (18_000, 18_000), "zero-width at the right edge of the piece before it");
        assert_eq!(p.kind, kind_index("travel").unwrap());
    }

    #[test]
    fn a_lane_added_in_the_end_zone_by_the_next_zone_tapers_inside_the_end_zone() {
        let s = with_extra_lane(60_000, 1);
        let t = s.beyond(true, frame(First)).unwrap().taper.unwrap();
        // L = R·W = 15 · 3 500 on the end zone, ending at the boundary
        assert_eq!((t.from_mm, t.to_mm, t.need_mm, t.room_mm), (7_500, 60_000, 52_500, 60_000));
        let [p] = &t.pieces[..] else { panic!("one piece: {:?}", t.pieces) };
        assert_eq!((p.was, p.becomes), (None, Some((18_000, 21_500))));
        assert_eq!((p.from, p.to), ((18_000, 18_000), (18_000, 21_500)));
        let cut = with_extra_lane(30_000, 1).beyond(true, frame(First)).unwrap().taper.unwrap();
        assert_eq!((cut.from_mm, cut.to_mm, cut.room_mm), (0, 30_000, 30_000), "cut at the end of the street");
    }

    #[test]
    fn a_change_off_the_roadway_is_a_jog() {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 20_000], LEN);
        let mut own = s.segments.clone();
        own[0].kind = kind_index("planting").unwrap();
        s.zones[0].segments = Some(own);
        let b = s.beyond(true, frame(First)).expect("the sections differ");
        assert_eq!((b.boundary_mm, b.taper), (20_000, None));
    }

    #[test]
    fn a_street_anchored_at_its_last_end_measures_from_the_other_end() {
        let mut s = with_extra_lane(20_000, 0);
        s.from_end = Last;
        // zones are measured from the last end: zone 0 is the last end's. The first end's arm sees zone 1.
        let first = s.beyond(true, frame(Last)).unwrap();
        assert_eq!(first.boundary_mm, LEN - 20_000);
        let last = s.beyond(false, frame(Last)).unwrap();
        assert_eq!(last.boundary_mm, 20_000);
        assert_eq!(last.next, s.reversed().segments, "the other zone is the base, seen from the last end");
    }

    #[test]
    fn an_arm_at_the_last_end_is_mirrored() {
        // zone 1 is the last zone, so the last end's; it has the extra lane the base does not
        let s = with_extra_lane(80_000, 1);
        let b = s.beyond(false, frame(First)).unwrap();
        assert_eq!(b.boundary_mm, 20_000);
        assert_eq!(b.next, s.reversed().segments, "the base, reversed and its lanes flipped");
        assert_ne!(b.next, s.segments);
        let t = b.taper.unwrap();
        // `transition` runs from the zone nearer the anchored end (zone 0, the zone across) to the farther one, which
        // has the lane: added, L = R·W = 15 · 3 500, hosted by zone 0 ending at the boundary (80 000 mm)
        assert_eq!((t.from_mm, t.to_mm, t.need_mm, t.room_mm), (20_000, 72_500, 52_500, 80_000));
        let [p] = &t.pieces[..] else { panic!("one piece: {:?}", t.pieces) };
        let row = s.row_mm;
        assert_eq!(p.was, Some((row - 21_500, row - 18_000)), "mirrored across the row");
        assert_eq!(p.becomes, None);
        assert_eq!(p.to, (row - 18_000, row - 18_000), "zero-width at the left edge of the (mirrored) base sidewalk, the first matched piece after it");
    }

    #[test]
    fn a_taper_without_room_says_so() {
        let s = with_extra_lane(20_000, 0);
        let mut short = zoned(Street::sample(0, Side::Right), &[0, 20_000, 30_000], LEN);
        short.speed = Speed::Kmh(50);
        short.zones[0].segments = s.zones[0].segments.clone();
        short.zones[2].segments = Some(short.segments.clone());
        short.row_mm = s.row_mm;
        let t = short.beyond(true, frame(First)).unwrap().taper.unwrap();
        assert_eq!((t.room_mm, t.need_mm, t.to_mm), (10_000, 56_500, 30_000), "drawn over the room there is");
        assert!(t.room_mm < t.need_mm);
    }

    #[test]
    fn merges_is_the_transitions() {
        // the end zone's lane running away from it has no lane of its direction left across the boundary to merge into
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 20_000], LEN);
        s.speed = Speed::Kmh(50);
        // the dropped lane is the only one running away from the far zone
        let mut only = s.segments.clone();
        only.retain(|p| p.kind != kind_index("travel").unwrap() || p.direction == Some(1));
        s.zones[1].segments = Some(only);
        let t = s.beyond(true, frame(First)).unwrap().taper.unwrap();
        assert!(!t.merges, "{t:?}");
    }

    #[test]
    fn no_beyond_where_the_frame_does_not_match() {
        let s = with_extra_lane(20_000, 0);
        assert!(s.beyond(true, frame(First)).is_some());
        let longer = Some(ZoneFrame { length_mm: LEN + 2_000, from_end: First });
        assert_eq!(s.beyond(true, longer), None, "another length by more than a metre");
        let within = Some(ZoneFrame { length_mm: LEN + 900, from_end: First });
        assert!(s.beyond(true, within).is_some());
        assert_eq!(s.beyond(true, frame(Last)), None, "anchored at the other end");
        assert_eq!(s.beyond(true, None), None, "no frame");
        let short = Some(ZoneFrame { length_mm: 20_000, from_end: First });
        let mut cut = s.clone();
        cut.length_mm = None;
        assert_eq!(cut.beyond(true, short), None, "the boundary is at the road's end or past it");
    }
}
