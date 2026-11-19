//! The pick: the stretch of the street the edits act on, or the whole street. Like the zone selection it replaces,
//! it is outside the history: undo, redo and Start over only clamp it.
//!
//! Positions here are millimetres from the street's anchored end, as the zones keep them; `left_mm` and `anchored_mm`
//! convert to and from the plan's left end (the street's first end), which is what a person sees and types.

use std::ops::Range;

use super::zones::MIN_ZONE_MM;
use super::{Arg, Editor, Said, ZoneEnd, snap};
use cityloom_model::street::{same_pieces, speed_in};

/// What the edits act on: the whole street, or a stretch of it (anchored millimetres, `from_mm < to_mm`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Whole,
    Stretch { from_mm: i32, to_mm: i32 },
}

/// A maximal part of the street that looks the same: neighbouring zones with the same pieces (ignoring uids) and the
/// same speed limit. `from_mm` and `to_mm` are anchored; the last run ends at the street's end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub zones: Range<usize>,
    pub from_mm: i32,
    pub to_mm: i32,
}

/// Why a pick was refused. The pick stays as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickRefusal {
    /// Both ends of the street are cut by the map's box: there is no end to measure from, so only the whole street.
    NoAnchor,
    /// The stretch would be shorter than `MIN_ZONE_MM`.
    TooShort,
    OutsideStreet,
    /// A drag is under way: its edits belong to the stretch it started on.
    Dragging,
}

impl PickRefusal {
    pub fn said(self) -> Said {
        let key = match self {
            PickRefusal::NoAnchor => "pick-refused-no-anchor",
            PickRefusal::TooShort => "pick-refused-too-short",
            PickRefusal::OutsideStreet => "pick-refused-outside",
            PickRefusal::Dragging => "pick-refused-dragging",
        };
        let said = Said::new(key);
        if self == PickRefusal::TooShort { said.with("min", Arg::Length(MIN_ZONE_MM)) } else { said }
    }
}

impl Editor {
    pub fn pick(&self) -> Pick {
        self.pick
    }

    /// Picks the whole street. Nothing changes while a drag is under way.
    pub fn pick_whole(&mut self) {
        if self.gesture.is_none() {
            self.choose(Pick::Whole);
        }
    }

    /// Picks the stretch between `a_mm` and `b_mm` (anchored, either order). Each end snaps to `SNAP_MM`, then to a
    /// street end or a boundary between runs closer than `MIN_ZONE_MM`; a stretch as long as the street is the whole
    /// street. On a street whose zones were drawn elsewhere, the stretch grows to the whole runs it touches.
    pub fn pick_stretch(&mut self, a_mm: i32, b_mm: i32) -> Result<Pick, PickRefusal> {
        let length = self.pickable()?;
        let (lo, hi) = (snap(a_mm.min(b_mm)), snap(a_mm.max(b_mm)));
        if lo < 0 || hi > length {
            return Err(PickRefusal::OutsideStreet);
        }
        let runs = self.runs();
        let (lo, hi) = if self.elsewhere().is_some() {
            // Whole runs only: the run `lo` lies in, to the run the stretch ends in.
            let first = runs.iter().find(|r| lo < r.to_mm).or(runs.last());
            let last = runs.iter().rfind(|r| r.from_mm < hi.max(lo + 1)).or(runs.first());
            (first.map_or(0, |r| r.from_mm), last.map_or(length, |r| r.to_mm))
        } else {
            let marks: Vec<i32> = [0, length].into_iter().chain(runs.iter().skip(1).map(|r| r.from_mm)).collect();
            let near = |x: i32| marks.iter().copied().filter(|m| (m - x).abs() < MIN_ZONE_MM).min_by_key(|m| (m - x).abs()).unwrap_or(x);
            let (lo, hi) = (near(lo), near(hi));
            if hi - lo < MIN_ZONE_MM {
                return Err(PickRefusal::TooShort);
            }
            (lo, hi)
        };
        let pick = Self::stretch_or_whole(lo, hi, length);
        self.choose(pick);
        Ok(pick)
    }

    /// Picks the run at `at_mm` (anchored): the whole part of the street around it that looks the same.
    pub fn pick_run_at(&mut self, at_mm: i32) -> Result<Pick, PickRefusal> {
        let length = self.pickable()?;
        if at_mm < 0 || at_mm > length {
            return Err(PickRefusal::OutsideStreet);
        }
        let runs = self.runs();
        let run = runs.iter().find(|r| at_mm < r.to_mm).or(runs.last()).ok_or(PickRefusal::OutsideStreet)?;
        let pick = Self::stretch_or_whole(run.from_mm, run.to_mm, length);
        self.choose(pick);
        Ok(pick)
    }

    /// The runs of the street in anchored order. Zones beyond the loaded end are in none.
    pub(super) fn runs(&self) -> Vec<Run> {
        let content = &self.states[self.cursor].content;
        let length = self.pick_length_mm();
        let looks_alike = |a: usize, b: usize| {
            same_pieces(content.section(a), content.section(b)) && speed_in(content.speed, &content.zones, a) == speed_in(content.speed, &content.zones, b)
        };
        let mut runs: Vec<Run> = Vec::new();
        for z in 0..self.pickable_zones() {
            let (from_mm, to_mm) = content.span(z, length);
            match runs.last_mut() {
                Some(run) if looks_alike(run.zones.start, z) => {
                    run.zones.end = z + 1;
                    run.to_mm = to_mm.min(length);
                }
                _ => runs.push(Run { zones: z..z + 1, from_mm, to_mm: to_mm.min(length) }),
            }
        }
        runs
    }

    /// The zones the pick covers, in anchored order. The whole street leaves out the zones beyond the loaded end.
    #[cfg_attr(not(test), expect(dead_code, reason = "the edits over a pick act on these zones from a later change"))]
    pub(super) fn zones_in_pick(&self) -> Range<usize> {
        match self.pick {
            Pick::Whole => 0..self.pickable_zones(),
            Pick::Stretch { from_mm, to_mm } => self.zone_at(from_mm, false)..self.zone_at(to_mm, true) + 1,
        }
    }

    /// Where an anchored position lies from the plan's left end, the street's first end.
    pub fn left_mm(&self, anchored_mm: i32) -> i32 {
        match self.anchor_end() {
            ZoneEnd::First => anchored_mm,
            ZoneEnd::Last => self.pick_length_mm() - anchored_mm,
        }
    }

    /// Where a position counted from the plan's left end lies from the anchored end: `left_mm`'s inverse.
    pub fn anchored_mm(&self, left_mm: i32) -> i32 {
        self.left_mm(left_mm)
    }

    /// Picks the span of zone `z` and shows it, keeping the chosen piece: what the zone strip's commands do.
    pub(super) fn follow_zone(&mut self, z: usize) {
        let length = self.pick_length_mm();
        let (from_mm, to_mm) = self.states[self.cursor].content.span(z, length);
        self.pick = Self::stretch_or_whole(from_mm, to_mm.min(length), length);
        self.zone = z;
    }

    /// After an edit, the history or the frame moved: a stretch that starts past the street's end becomes the whole
    /// street, one that runs past it ends there, and the shown zone is the one at the pick's left end again. (A street
    /// with no frame keeps the span of a zone the zone strip chose, along the length its zones were drawn on.)
    pub(super) fn clamp_pick(&mut self) {
        if let Pick::Stretch { from_mm, to_mm } = self.pick {
            let length = self.pick_length_mm();
            self.pick = if from_mm >= length { Pick::Whole } else { Self::stretch_or_whole(from_mm.max(0), to_mm.min(length), length) };
        }
        self.zone = self.shown_zone();
    }

    /// Takes `pick` and shows the zone at its left end; the chosen piece is let go when that zone changes.
    fn choose(&mut self, pick: Pick) {
        self.pick = pick;
        let zone = self.shown_zone();
        if zone != self.zone {
            self.zone = zone;
            self.selected = None;
        }
    }

    /// The street's length when a stretch of it may be picked.
    fn pickable(&self) -> Result<i32, PickRefusal> {
        if self.gesture.is_some() {
            return Err(PickRefusal::Dragging);
        }
        self.frame.map(|f| f.length_mm).ok_or(PickRefusal::NoAnchor)
    }

    fn stretch_or_whole(from_mm: i32, to_mm: i32, length_mm: i32) -> Pick {
        if from_mm <= 0 && to_mm >= length_mm { Pick::Whole } else { Pick::Stretch { from_mm, to_mm } }
    }

    /// The length the pick lies along: the road as loaded (the plan is drawn at it), else the one the zones were
    /// drawn on.
    fn pick_length_mm(&self) -> i32 {
        self.frame.map(|f| f.length_mm).or(self.states[self.cursor].content.length_mm).unwrap_or(0)
    }

    /// The end positions are measured from: the street's own once it keeps one, else its road's.
    fn anchor_end(&self) -> ZoneEnd {
        let content = &self.states[self.cursor].content;
        if !content.zones.is_empty() || content.length_mm.is_some() { content.from_end } else { self.frame.map_or(ZoneEnd::First, |f| f.from_end) }
    }

    /// How many zones, from the first, lie on the loaded street: the rest are cut off.
    fn pickable_zones(&self) -> usize {
        let count = self.states[self.cursor].content.zone_count();
        (0..count).take_while(|&z| !self.is_cut_off(z)).count().max(1)
    }

    /// The zone holding anchored position `at_mm`: the one it starts, or with `ending`, the one it ends.
    fn zone_at(&self, at_mm: i32, ending: bool) -> usize {
        let zones = &self.states[self.cursor].content.zones;
        (0..self.pickable_zones()).rfind(|&z| zones.get(z).is_none_or(|g| if ending { g.from_mm < at_mm } else { g.from_mm <= at_mm })).unwrap_or(0)
    }

    /// The zone at the pick's left end on the plan: its start when the street is anchored at its first end, else its
    /// end. The whole street shows the leftmost run.
    fn shown_zone(&self) -> usize {
        let (from_mm, to_mm) = match self.pick {
            Pick::Whole => (0, self.pick_length_mm()),
            Pick::Stretch { from_mm, to_mm } => (from_mm, to_mm),
        };
        match self.anchor_end() {
            ZoneEnd::First => self.zone_at(from_mm, false),
            ZoneEnd::Last => self.zone_at(to_mm, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::zoned;
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::street::model::{Speed, Street, StreetExt, ZoneFrame};

    const LEN: i32 = 60_000;

    fn sample() -> Street {
        Street::sample(0, Side::Right)
    }

    /// The sample street divided at `starts` (measured from its anchored end, `from_end`), drawn on a road `LEN`
    /// long; zone `i` has its first piece widened by `widen[i]` mm (0 leaves it on the base section).
    fn street(starts: &[i32], widen: &[i32], from_end: ZoneEnd) -> Street {
        let base = sample();
        let mut s = zoned(base.clone(), starts, LEN);
        for (zone, by) in s.zones.iter_mut().zip(widen) {
            if *by != 0 {
                let mut own = base.segments.clone();
                own[0].width_mm += by;
                zone.segments = Some(own);
            }
        }
        s.from_end = from_end;
        s.settled()
    }

    fn frame(length_mm: i32, from_end: ZoneEnd) -> Option<ZoneFrame> {
        Some(ZoneFrame { length_mm, from_end })
    }

    /// An editor on `now` (an earlier revision of the plain sample), on a road as `frame` says.
    fn editor(now: &Street, frame: Option<ZoneFrame>) -> Editor {
        Editor::from_street(&sample(), now, 0).with_frame(frame)
    }

    /// Three runs: 0 to 20 m, 20 m to 40 m, 40 m to the end.
    fn three_runs() -> Editor {
        editor(&street(&[0, 20_000, 40_000], &[0, 100, 0], ZoneEnd::First), frame(LEN, ZoneEnd::First))
    }

    fn stretch(from_mm: i32, to_mm: i32) -> Pick {
        Pick::Stretch { from_mm, to_mm }
    }

    #[test]
    fn a_new_editor_picks_the_whole_street() {
        let plain = editor(&sample(), frame(LEN, ZoneEnd::First));
        assert_eq!(plain.pick(), Pick::Whole);
        assert_eq!(plain.zone(), 0);
        assert_eq!(plain.zones_in_pick(), 0..1);
        let e = three_runs();
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.zone(), 0, "the leftmost run is shown");
        assert_eq!(e.zones_in_pick(), 0..3);
    }

    #[test]
    fn a_stretch_snaps_to_100_mm_and_to_a_boundary_or_end_within_5_m() {
        let mut e = three_runs();
        assert_eq!(e.pick_stretch(10_040, 14_960), Ok(stretch(10_000, 15_000)), "5 m from a boundary is not within it");
        assert_eq!(e.pick(), stretch(10_000, 15_000));
        assert_eq!(e.pick_stretch(31_000, 3_000), Ok(stretch(0, 31_000)), "either order; the street's end is near");
        assert_eq!(e.pick_stretch(22_000, 36_050), Ok(stretch(20_000, 40_000)), "both boundaries are near");
        assert_eq!(e.pick_stretch(50_000, 57_500), Ok(stretch(50_000, LEN)));
        assert_eq!(e.zone(), 2);
        assert_eq!(e.zones_in_pick(), 2..3);
        assert_eq!(e.pick_stretch(-100, 10_000), Err(PickRefusal::OutsideStreet));
        assert_eq!(e.pick_stretch(10_000, LEN + 100), Err(PickRefusal::OutsideStreet));
        assert_eq!(e.pick(), stretch(50_000, LEN), "a refusal leaves the pick");
    }

    #[test]
    fn a_stretch_shorter_than_5_m_is_refused_and_the_pick_stays() {
        let mut e = three_runs();
        e.pick_stretch(25_000, 35_000).unwrap();
        assert_eq!(e.pick_stretch(10_000, 14_900), Err(PickRefusal::TooShort));
        assert_eq!(e.pick_stretch(18_000, 22_000), Err(PickRefusal::TooShort), "both ends snap to the boundary");
        assert_eq!(e.pick(), stretch(25_000, 35_000));
        assert_eq!(e.zone(), 1);
        assert_eq!(PickRefusal::TooShort.said(), Said::new("pick-refused-too-short").with("min", Arg::Length(MIN_ZONE_MM)));
    }

    #[test]
    fn a_stretch_as_long_as_the_street_is_the_whole_street() {
        let mut e = three_runs();
        e.pick_stretch(25_000, 35_000).unwrap();
        assert_eq!(e.pick_stretch(LEN, 0), Ok(Pick::Whole));
        e.pick_stretch(25_000, 35_000).unwrap();
        assert_eq!(e.pick_stretch(2_000, 58_000), Ok(Pick::Whole), "both ends snap to the street's ends");
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.zone(), 0);
        e.pick_stretch(25_000, 35_000).unwrap();
        e.pick_whole();
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.zone(), 0);
    }

    #[test]
    fn picking_a_run_takes_identical_neighbouring_zones_together() {
        // Zones 0 and 1 have the same section and speed: one run, with no boundary between them to snap to.
        let mut e = editor(&street(&[0, 20_000, 40_000], &[0, 0, 100], ZoneEnd::First), frame(LEN, ZoneEnd::First));
        assert_eq!(e.runs(), [Run { zones: 0..2, from_mm: 0, to_mm: 40_000 }, Run { zones: 2..3, from_mm: 40_000, to_mm: LEN }]);
        assert_eq!(e.pick_run_at(25_000), Ok(stretch(0, 40_000)));
        assert_eq!(e.zones_in_pick(), 0..2);
        assert_eq!(e.zone(), 0);
        assert_eq!(e.pick_run_at(LEN), Ok(stretch(40_000, LEN)), "the far end belongs to the last run");
        assert_eq!(e.zone(), 2);
        assert_eq!(e.pick_stretch(18_000, 30_000), Ok(stretch(18_000, 30_000)), "no boundary is drawn inside a run");
        assert_eq!(e.zones_in_pick(), 0..2);

        // A speed limit sets a zone apart as much as its pieces do.
        let mut s = street(&[0, 20_000, 40_000], &[0, 0, 0], ZoneEnd::First);
        s.zones[1].speed = Speed::Kmh(30);
        let e = editor(&s, frame(LEN, ZoneEnd::First));
        assert_eq!(e.runs().iter().map(|r| r.zones.clone()).collect::<Vec<_>>(), [0..1, 1..2, 2..3]);

        // A street of one section is one run.
        let mut plain = editor(&sample(), frame(LEN, ZoneEnd::First));
        assert_eq!(plain.runs(), [Run { zones: 0..1, from_mm: 0, to_mm: LEN }]);
        assert_eq!(plain.pick_run_at(30_000), Ok(Pick::Whole));
    }

    #[test]
    fn cut_off_zones_are_in_no_run_and_cannot_be_picked() {
        // Drawn on a road 60 m long, loaded on one 55 m long: the zone from 58 m starts beyond the loaded end.
        let mut e = editor(&street(&[0, 30_000, 58_000], &[0, 100, 200], ZoneEnd::First), frame(55_000, ZoneEnd::First));
        assert_eq!(e.runs(), [Run { zones: 0..1, from_mm: 0, to_mm: 30_000 }, Run { zones: 1..2, from_mm: 30_000, to_mm: 55_000 }]);
        assert_eq!(e.zones_in_pick(), 0..2, "the whole street leaves the cut-off zone out");
        assert_eq!(e.pick_run_at(56_000), Err(PickRefusal::OutsideStreet));
        assert_eq!(e.pick_stretch(40_000, 59_000), Err(PickRefusal::OutsideStreet));
        assert!(!e.select_zone(2));
        assert_eq!(e.pick_run_at(55_000), Ok(stretch(30_000, 55_000)));
        assert_eq!(e.zone(), 1);
        assert_eq!(PickRefusal::OutsideStreet.said(), Said::new("pick-refused-outside"));
    }

    #[test]
    fn a_street_with_no_frame_can_only_be_picked_whole() {
        let mut e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 0], ZoneEnd::First), None);
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.pick_stretch(0, 10_000), Err(PickRefusal::NoAnchor));
        assert_eq!(e.pick_run_at(5_000), Err(PickRefusal::NoAnchor));
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.zones_in_pick(), 0..3, "an edit with no anchor goes to every run");
        assert_eq!(PickRefusal::NoAnchor.said(), Said::new("pick-refused-no-anchor"));
    }

    #[test]
    fn an_elsewhere_street_picks_only_whole_runs() {
        // Drawn on a road 60 m long, loaded on one 58 m long: the runs are kept, the last ending at the loaded end.
        let mut e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 200], ZoneEnd::First), frame(58_000, ZoneEnd::First));
        assert!(e.elsewhere().is_some());
        assert_eq!(e.pick_stretch(25_000, 30_000), Ok(stretch(20_000, 40_000)), "a drag inside a run picks that run");
        assert_eq!(e.zone(), 1);
        assert_eq!(e.pick_stretch(25_000, 41_000), Ok(stretch(20_000, 58_000)), "a drag across runs picks them all");
        assert_eq!(e.pick_stretch(20_000, 40_000), Ok(stretch(20_000, 40_000)), "an end on a boundary stays");
        assert_eq!(e.pick_stretch(5_000, 45_000), Ok(Pick::Whole));
        assert_eq!(e.pick_run_at(10_000), Ok(stretch(0, 20_000)));
        assert_eq!(e.zones_in_pick(), 0..1);
    }

    #[test]
    fn the_shown_zone_is_the_one_at_the_picks_left_end_on_a_street_anchored_at_its_last_end() {
        // Anchored at the last end: zone 0 lies at the right of the plan, zone 2 at its left.
        let mut e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 200], ZoneEnd::Last), frame(LEN, ZoneEnd::Last));
        assert!(e.elsewhere().is_none());
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.zone(), 2, "the whole street shows the leftmost run");
        assert_eq!(e.pick_stretch(20_000, 40_000), Ok(stretch(20_000, 40_000)));
        assert_eq!(e.zone(), 1);
        assert_eq!(e.pick_stretch(30_000, 50_000), Ok(stretch(30_000, 50_000)));
        assert_eq!(e.zone(), 2, "its left end, 10 m from the plan's left, is in zone 2");
        assert_eq!(e.zones_in_pick(), 1..3);
        assert_eq!(e.left_mm(50_000), 10_000);
        assert!(e.select_zone(0));
        assert_eq!(e.pick(), stretch(0, 20_000));
        assert_eq!(e.zone(), 0);
        assert!(e.select_zone(2));
        assert_eq!(e.pick(), stretch(40_000, LEN));
        assert_eq!(e.zone(), 2);
    }

    #[test]
    fn left_and_anchored_positions_convert_both_ways_for_either_end() {
        let first = editor(&street(&[0, 20_000], &[0, 100], ZoneEnd::First), frame(LEN, ZoneEnd::First));
        let last = editor(&street(&[0, 20_000], &[0, 100], ZoneEnd::Last), frame(LEN, ZoneEnd::Last));
        // A street with no zones yet takes the end its road is anchored at.
        let plain_last = editor(&sample(), frame(LEN, ZoneEnd::Last));
        for x in [0, 12_300, 30_000, LEN] {
            assert_eq!(first.left_mm(x), x);
            assert_eq!(first.anchored_mm(x), x);
            assert_eq!(last.left_mm(x), LEN - x);
            assert_eq!(last.anchored_mm(LEN - x), x);
            assert_eq!(last.anchored_mm(last.left_mm(x)), x);
            assert_eq!(plain_last.left_mm(x), LEN - x);
        }
    }

    #[test]
    fn the_pick_survives_undo_clamped_to_the_street() {
        // The zones are an earlier revision of the plain street.
        let mut e = three_runs();
        assert_eq!(e.pick_stretch(40_000, LEN), Ok(stretch(40_000, LEN)));
        assert_eq!(e.zone(), 2);
        assert!(e.undo());
        assert_eq!(e.pick(), stretch(40_000, LEN), "the pick is outside the history");
        assert_eq!(e.zone(), 0, "the street has one zone now");
        assert_eq!(e.zones_in_pick(), 0..1);
        assert!(e.redo());
        assert_eq!(e.zone(), 2);
        assert!(e.reset());
        assert_eq!(e.pick(), stretch(40_000, LEN));
        assert_eq!(e.zone(), 0);
        assert!(e.undo());
        assert_eq!(e.zone(), 2);
        // Opened on a shorter road, a stretch running past the street's end ends there, and one lying past it is
        // the whole street.
        let e = e.with_frame(frame(50_000, ZoneEnd::First));
        assert_eq!(e.pick(), stretch(40_000, 50_000));
        assert_eq!(e.zone(), 2);
        let e = e.with_frame(frame(40_000, ZoneEnd::First));
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.zone(), 0);
        // With no frame at all, a street with no zones has no length: only the whole street.
        let mut e = editor(&sample(), frame(LEN, ZoneEnd::First));
        e.pick_stretch(20_000, 40_000).unwrap();
        let e = e.with_frame(None);
        assert_eq!(e.pick(), Pick::Whole);
    }

    #[test]
    fn nothing_is_picked_while_a_drag_is_under_way() {
        let mut e = three_runs();
        e.pick_stretch(20_000, 40_000).unwrap();
        e.begin_gesture();
        assert_eq!(e.pick_stretch(0, 10_000), Err(PickRefusal::Dragging));
        assert_eq!(e.pick_run_at(50_000), Err(PickRefusal::Dragging));
        e.pick_whole();
        assert_eq!(e.pick(), stretch(20_000, 40_000), "the drag's section stays the one it started on");
        e.cancel_gesture();
        assert_eq!(PickRefusal::Dragging.said(), Said::new("pick-refused-dragging"));
    }

    #[test]
    fn moving_a_boundary_keeps_the_zone_and_its_pick_together() {
        // The zone strip moves the start of the selected zone; the pick follows it, and a cancelled drag puts both back.
        let mut e = three_runs();
        assert!(e.select_zone(1));
        assert_eq!(e.move_boundary(25_000), Ok(()));
        assert_eq!((e.zone(), e.pick()), (1, stretch(25_000, 40_000)));
        e.begin_gesture();
        assert_eq!(e.move_boundary(15_000), Ok(()));
        assert_eq!((e.zone(), e.pick()), (1, stretch(15_000, 40_000)));
        e.cancel_gesture();
        assert_eq!((e.zone(), e.pick()), (1, stretch(25_000, 40_000)));
        e.begin_gesture();
        assert_eq!(e.move_boundary(30_000), Ok(()));
        assert!(e.end_gesture());
        assert_eq!((e.zone(), e.pick()), (1, stretch(30_000, 40_000)));
    }
}
