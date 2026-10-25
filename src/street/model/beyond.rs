//! What lies beyond a street's end zone, for the junction: the first boundary met going out from the junction, the
//! zone across it, and the taper that boundary needs. Derived, never saved. Pure, no browser types.
//!
//! Distances are along the arm from the junction's centre, which is the street's end (the mouth is not subtracted:
//! `junction::read_model` draws an arm from its mouth to `ARM_LENGTH_MM`, both measured from the centre).

use super::zones::{same_pieces, zone_label_mid};
use super::{Host, LENGTH_TOLERANCE_MM, Segment, Street, ZoneEnd, ZoneFrame, total, transition};
use crate::shared::catalogue::is_roadway;
use crate::shared::said::Said;

/// The first boundary met going out from an arm's end, and what is across it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beyond {
    /// Millimetres from the junction's centre to the boundary.
    pub boundary_mm: i32,
    /// The zone across the boundary as the arm sees it: looking out from the junction.
    pub next: Vec<Segment>,
    /// What the zone across is called, as it stands in the middle of a sentence.
    pub zone: Said,
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
    /// the boundary is at or past the road's end, when the two sections have the same pieces, or when either is wider than the street (`row_mm`; a narrower one fits and is drawn).
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
        // A section wider than the street does not fit and is not drawn. One narrower does fit ("left to use"): its
        // pieces are laid out from the left edge, and the mirrored spans below flip by the section's own total.
        let (end_total, across_total) = (total(sections[end]), total(sections[neighbour]));
        if end_total > self.row_mm || across_total > self.row_mm {
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
                // Each side flips by its own section's total: the view lays a section's pieces out from 0.
                let flip = |r: Option<(i32, i32)>, total: i32| r.map(|(x0, x1)| (total - x1, total - x0));
                changes = changes.into_iter().rev().map(|(k, was, becomes)| (k, flip(was, end_total), flip(becomes, across_total))).collect();
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
        Some(Beyond { boundary_mm, next: across.segments, zone: zone_label_mid(&self.zones[neighbour], neighbour + 1), taper })
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
    use crate::shared::said::Arg;
    use crate::street::model::{Speed, named_said, zoned};
    use ZoneEnd::{First, Last};

    const LEN: i32 = 100_000;

    fn frame(end: ZoneEnd) -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm: LEN, from_end: end })
    }

    fn lane(uid: u32, direction: usize) -> Segment {
        let mut l = Segment::new(uid, kind_index("travel").unwrap(), 3_300);
        l.direction = Some(direction);
        l
    }

    /// The sample street at 50 km/h with zones at 0 and `at`; zone `own` (0 or 1) has a travel lane more than the base,
    /// running `direction` (0 from the first end to the last).
    fn with_extra_lane(at: i32, own: usize, direction: usize) -> Street {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, at], LEN);
        s.speed = Speed::Kmh(50);
        // The lane takes the place of the last sidewalk (3 300 mm), so the section still fills the street.
        let mut section = s.segments.clone();
        section.pop();
        section.push(lane(900, direction));
        s.zones[own].segments = Some(section);
        s
    }

    #[test]
    fn a_zone_wider_than_the_street_gives_no_beyond() {
        // both sections fill the street: a beyond
        let filling = zoned(Street::sample(0, Side::Right), &[0, 20_000], LEN);
        let mut both = filling.clone();
        let mut section = both.segments.clone();
        section.iter_mut().find(|g| g.kind == kind_index("travel").unwrap()).unwrap().width_mm += 100;
        section.iter_mut().rev().find(|g| g.kind == kind_index("travel").unwrap()).unwrap().width_mm -= 100;
        both.zones[1].segments = Some(section);
        assert!(both.beyond(true, frame(First)).is_some(), "the pocket's sections fill the street");
        // an over-wide neighbour, then an over-wide end zone
        for own in [1, 0] {
            let mut s = filling.clone();
            let mut section = s.segments.clone();
            section.push(lane(900, 0));
            s.zones[own].segments = Some(section);
            assert_eq!(s.beyond(true, frame(First)), None, "zone {own} is wider than the street");
            assert_eq!(s.beyond(false, frame(First)), None, "from the other end too");
        }
    }

    #[test]
    fn the_beyond_names_the_zone_across() {
        let mut s = with_extra_lane(20_000, 0, 1);
        let zone = |s: &Street, first_end| s.beyond(first_end, frame(First)).unwrap().zone;
        assert_eq!(zone(&s, true), Said::new("zone-unnamed-mid").with("n", Arg::Num(2)));
        assert_eq!(zone(&s, false), Said::new("zone-unnamed-mid").with("n", Arg::Num(1)));
        s.zones[1].name = Some("Market".into());
        assert_eq!(zone(&s, true), named_said("Market"));
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
        let s = with_extra_lane(20_000, 0, 1);
        let b = s.beyond(true, frame(First)).expect("the end zone differs from the next");
        assert_eq!(b.boundary_mm, 20_000, "the distance from the street's end, which is the junction's centre");
        assert_eq!(b.next, s.segments);
        let t = b.taper.expect("a lane ends");
        // the lane runs toward the junction, so it opens as it leaves the end zone: R·W = 15 · 3 300
        assert_eq!((t.from_mm, t.to_mm, t.need_mm), (20_000, 69_500, 49_500));
        assert_eq!(t.room_mm, 80_000, "the host is the zone across, 80 m long");
        assert!(t.merges, "a lane that opens needs no merge");
        let [p] = &t.pieces[..] else { panic!("one piece differs: {:?}", t.pieces) };
        let was = (14_700, 18_000);
        assert_eq!(p.was, Some(was));
        assert_eq!(p.becomes, None);
        assert_eq!(p.from, was);
        assert_eq!(p.to, (14_700, 14_700), "zero-width at the right edge of the piece before it");
        assert_eq!(p.kind, kind_index("travel").unwrap());
    }

    #[test]
    fn a_lane_added_in_the_end_zone_by_the_next_zone_tapers_inside_the_end_zone() {
        let s = with_extra_lane(60_000, 1, 0);
        let t = s.beyond(true, frame(First)).unwrap().taper.unwrap();
        // L = R·W = 15 · 3 300 on the end zone, ending at the boundary
        assert_eq!((t.from_mm, t.to_mm, t.need_mm, t.room_mm), (10_500, 60_000, 49_500, 60_000));
        let [p] = &t.pieces[..] else { panic!("one piece: {:?}", t.pieces) };
        assert_eq!((p.was, p.becomes), (None, Some((14_700, 18_000))));
        assert_eq!((p.from, p.to), ((14_700, 14_700), (14_700, 18_000)));
        let cut = with_extra_lane(30_000, 1, 0).beyond(true, frame(First)).unwrap().taper.unwrap();
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
        let mut s = with_extra_lane(20_000, 0, 0);
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
        let s = with_extra_lane(80_000, 1, 0);
        let b = s.beyond(false, frame(First)).unwrap();
        assert_eq!(b.boundary_mm, 20_000);
        assert_eq!(b.next, s.reversed().segments, "the base, reversed and its lanes flipped");
        assert_ne!(b.next, s.segments);
        let t = b.taper.unwrap();
        // `transition` runs from the zone nearer the anchored end (zone 0, the zone across) to the farther one, which
        // has the lane: added, L = R·W = 15 · 3 300, hosted by zone 0 ending at the boundary (80 000 mm)
        assert_eq!((t.from_mm, t.to_mm, t.need_mm, t.room_mm), (20_000, 69_500, 49_500, 80_000));
        let [p] = &t.pieces[..] else { panic!("one piece: {:?}", t.pieces) };
        let row = s.row_mm;
        assert_eq!(p.was, Some((row - 18_000, row - 14_700)), "mirrored across the row");
        assert_eq!(p.becomes, None);
        assert_eq!(p.to, (row - 14_700, row - 14_700), "zero-width at the left edge of the (mirrored) base sidewalk, the first matched piece after it");
    }

    #[test]
    fn a_taper_without_room_says_so() {
        let s = with_extra_lane(20_000, 0, 1);
        let mut short = zoned(Street::sample(0, Side::Right), &[0, 20_000, 30_000], LEN);
        short.speed = Speed::Kmh(50);
        short.zones[0].segments = s.zones[0].segments.clone();
        short.zones[2].segments = Some(short.segments.clone());
        short.row_mm = s.row_mm;
        let t = short.beyond(true, frame(First)).unwrap().taper.unwrap();
        assert_eq!((t.room_mm, t.need_mm, t.to_mm), (10_000, 49_500, 30_000), "drawn over the room there is");
        assert!(t.room_mm < t.need_mm);
    }

    #[test]
    fn merges_is_the_transitions() {
        // the end zone's lane running away from it has no lane of its direction left across the boundary to merge into
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 20_000], LEN);
        s.speed = Speed::Kmh(50);
        // the dropped lane is the only one running away from the far zone
        let mut only = s.segments.clone();
        // (planting takes its width, so the section still fills the street)
        for p in only.iter_mut().filter(|p| p.kind == kind_index("travel").unwrap() && p.direction != Some(1)) {
            p.kind = kind_index("planting").unwrap();
            p.direction = None;
        }
        s.zones[1].segments = Some(only);
        let t = s.beyond(true, frame(First)).unwrap().taper.unwrap();
        assert!(!t.merges, "{t:?}");
    }

    #[test]
    fn no_beyond_where_the_frame_does_not_match() {
        let s = with_extra_lane(20_000, 0, 1);
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

    /// The spans of a section laid out left to right from 0.
    fn span_of(section: &[Segment], uid: u32) -> (i32, i32) {
        let mut x = 0;
        for p in section {
            if p.uid == uid {
                return (x, x + p.width_mm);
            }
            x += p.width_mm;
        }
        panic!("no piece {uid}");
    }

    #[test]
    fn mirrored_spans_are_those_of_the_street_seen_from_its_last_end() {
        // the last zone swaps its right-hand parking for a travel lane of the same width: both sections fill the street
        let mut s = zoned(Street::sample(0, Side::Right), &[0, 80_000], LEN);
        s.speed = Speed::Kmh(50);
        let parking = s.segments.iter().rev().find(|p| p.kind == kind_index("parking").unwrap()).unwrap().clone();
        let mut own = s.segments.clone();
        let at = own.iter().position(|p| p.uid == parking.uid).unwrap();
        own[at] = Segment { width_mm: parking.width_mm, ..lane(900, 0) };
        assert_eq!(own.iter().map(|p| p.width_mm).sum::<i32>(), s.row_mm);
        s.zones[1].segments = Some(own);
        let seen = s.reversed();
        let own_seen = seen.zones[1].segments.clone().unwrap();
        let b = s.beyond(false, frame(First)).unwrap();
        assert_eq!(b.next, seen.zones[0].segments.clone().unwrap_or_else(|| seen.segments.clone()));
        let t = b.taper.unwrap();
        let [added, dropped] = &t.pieces[..] else { panic!("a lane and a parking lane differ: {:?}", t.pieces) };
        let (added, dropped) = if added.kind == kind_index("travel").unwrap() { (added, dropped) } else { (dropped, added) };
        assert_eq!(added.was, Some(span_of(&own_seen, 900)), "the lane where the last end sees it");
        assert_eq!(added.becomes, None);
        assert_eq!(dropped.was, None);
        assert_eq!(dropped.becomes, Some(span_of(&b.next, parking.uid)), "the parking where the arm sees it, in the next zone");
        // the list runs left to right as the arm sees it
        let xs: Vec<i32> = t.pieces.iter().map(|p| p.was.or(p.becomes).unwrap().0).collect();
        assert!(xs.windows(2).all(|w| w[0] <= w[1]), "{xs:?}");
    }

    /// The sample street with zones at 0 and `at`, where zone `own` is the base with its last sidewalk replaced by a
    /// travel lane (uid 900) of `lane_mm`: narrower than the sidewalk, so that the section is under-filled but fits.
    fn under_filled(at: i32, own: usize, lane_mm: i32) -> Street {
        let mut s = zoned(Street::sample(0, Side::Right), &[0, at], LEN);
        s.speed = Speed::Kmh(50);
        let mut section = s.segments.clone();
        section.pop();
        section.push(Segment { width_mm: lane_mm, ..lane(900, 0) });
        assert!(total(&section) < s.row_mm);
        s.zones[own].segments = Some(section);
        s
    }

    #[test]
    fn an_under_filled_zone_at_the_first_end_still_has_a_beyond() {
        let s = under_filled(60_000, 0, 2_800);
        let own = s.zones[0].segments.clone().unwrap();
        let b = s.beyond(true, frame(First)).expect("the section fits, with width left to use");
        let [p] = &b.taper.expect("a lane is dropped across the boundary").pieces[..] else { panic!("one piece") };
        assert_eq!(p.was, Some(span_of(&own, 900)), "laid out from the left edge, as the view does");
        assert_eq!(p.becomes, None);
    }

    #[test]
    fn an_under_filled_zone_at_the_last_end_is_mirrored_by_its_own_total() {
        let s = under_filled(40_000, 1, 2_800);
        let seen = s.reversed();
        let own_seen = seen.zones[1].segments.clone().unwrap();
        let b = s.beyond(false, frame(First)).expect("the section fits, with width left to use");
        assert_eq!(b.next, seen.segments, "the zone across fills the street");
        let t = b.taper.expect("a lane opens across the boundary");
        // the pieces after the lane shift by the 500 mm the end zone leaves unused, so more than the lane moves
        let p = t.pieces.iter().find(|p| p.becomes.is_none()).expect("the lane");
        assert_eq!(p.was, Some(span_of(&own_seen, 900)), "flipped by the end zone's total, not by the street's width");
        assert_eq!(p.was.map(|w| w.0), Some(0), "the lane lies at the left edge, where the arm lays its pieces from");
        for q in t.pieces.iter().filter(|q| q.becomes.is_some()) {
            let (was, becomes) = (q.was.unwrap(), q.becomes.unwrap());
            assert_eq!((becomes.0 - was.0, becomes.1 - was.1), (500, 500), "{q:?}");
        }
    }

    #[test]
    fn a_left_hand_street_has_its_beyond_on_the_other_sides() {
        for first_end in [true, false] {
            let mut s = zoned(Street::sample(0, Side::Left), &[0, 20_000], LEN);
            s.speed = Speed::Kmh(50);
            let mut section = s.segments.clone();
            section.pop();
            section.push(lane(900, 1));
            assert_eq!(total(&section), s.row_mm);
            let own = usize::from(!first_end);
            s.zones[own].segments = Some(section);
            let arm = if first_end { s.clone() } else { s.reversed() };
            let seen = arm.zones[own].segments.clone().unwrap();
            let b = s.beyond(first_end, frame(First)).expect("the end zone differs from the next");
            let [p] = &b.taper.expect("the lane ends").pieces[..] else { panic!("one piece") };
            assert_eq!(p.was, Some(span_of(&seen, 900)), "first end: {first_end}");
            assert_eq!(p.becomes, None);
        }
    }

    #[test]
    fn a_steady_part_that_just_holds_the_transition_is_room_enough() {
        // host zone exactly as long as the transition needs: no shortage
        let s = with_extra_lane(20_000, 0, 1);
        let need = s.beyond(true, frame(First)).unwrap().taper.unwrap().need_mm;
        let mut exact = zoned(Street::sample(0, Side::Right), &[0, 20_000, 20_000 + need], LEN);
        exact.speed = Speed::Kmh(50);
        exact.zones[0].segments = s.zones[0].segments.clone();
        exact.zones[2].segments = Some(exact.segments.clone());
        let t = exact.beyond(true, frame(First)).unwrap().taper.unwrap();
        assert_eq!((t.room_mm, t.need_mm), (need, need));
    }
}
