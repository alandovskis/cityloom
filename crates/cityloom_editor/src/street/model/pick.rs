//! The pick: the stretch of the street the edits act on, or the whole street. Like the zone selection it replaces,
//! it is outside the history: undo, redo and Start over only clamp it.
//!
//! Positions here are millimetres from the street's anchored end, as the zones keep them; `left_mm` and `anchored_mm`
//! convert to and from the plan's left end (the street's first end), which is what a person sees and types.

use std::ops::Range;

use super::matching::match_pieces;
use super::zones::MIN_ZONE_MM;
use super::{Arg, Content, Editor, Said, Segment, Speed, Zone, ZoneEnd, ZoneFrame, snap};
use crate::shared::catalogue::KINDS;
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
            let (lo, hi) = self.snap_to_runs(lo, hi, length);
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
        let mut runs: Vec<Run> = Vec::new();
        for z in 0..self.pickable_zones() {
            let (from_mm, to_mm) = content.span(z, length);
            match runs.last_mut() {
                Some(run) if content.alike(run.zones.start, z) => {
                    run.zones.end = z + 1;
                    run.to_mm = to_mm.min(length);
                }
                _ => runs.push(Run { zones: z..z + 1, from_mm, to_mm: to_mm.min(length) }),
            }
        }
        runs
    }

    /// The zones the pick covers, in anchored order. The whole street leaves out the zones beyond the loaded end.
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

    /// What sets the picked run apart from the run beside it on the plan's left (else the one on its right): each
    /// piece one of them has and the other lacks, each piece whose width differs, in section order, and the speed
    /// limit. Empty unless the pick is exactly one run of several.
    pub fn pick_differs(&self) -> Vec<Said> {
        let Pick::Stretch { from_mm, to_mm } = self.pick else { return Vec::new() };
        let runs = self.runs();
        let Some(i) = runs.iter().position(|r| (r.from_mm, r.to_mm) == (from_mm, to_mm)) else { return Vec::new() };
        let (before, after) = (i.checked_sub(1), Some(i + 1).filter(|&n| n < runs.len()));
        let beside = match self.anchor_end() {
            ZoneEnd::First => before.or(after),
            ZoneEnd::Last => after.or(before),
        };
        let Some(n) = beside else { return Vec::new() };
        let content = &self.states[self.cursor].content;
        let (zone, other_zone) = (runs[i].zones.start, runs[n].zones.start);
        let (run, other) = (content.section(zone), content.section(other_zone));
        let piece = |key: &'static str, s: &[Segment], j: usize| {
            Said::new(key).with("kind", Arg::KindLower(KINDS[s[j].kind].id)).with("side", Arg::Msg(side_word(s, j)))
        };
        let mut what = Vec::new();
        for pair in match_pieces(other, run) {
            match pair {
                (None, Some(j)) => what.push(piece("stretch-what-added", run, j)),
                (Some(k), None) => what.push(piece("stretch-what-dropped", other, k)),
                (Some(k), Some(j)) if other[k].width_mm != run[j].width_mm => {
                    what.push(piece("stretch-what-width", run, j).with("width", Arg::Length(run[j].width_mm)));
                }
                _ => {}
            }
        }
        let speed = speed_in(content.speed, &content.zones, zone);
        if speed != speed_in(content.speed, &content.zones, other_zone) {
            what.push(match speed {
                Speed::Kmh(kmh) => Said::new("stretch-what-speed").with("kmh", Arg::Num(i64::from(kmh))),
                _ => Said::new("stretch-what-no-speed"),
            });
        }
        what
    }

    /// Picks the span of zone `z` and shows it, keeping the chosen piece: what the zone strip's commands do.
    pub(super) fn follow_zone(&mut self, z: usize) {
        let length = self.pick_length_mm();
        let (from_mm, to_mm) = self.states[self.cursor].content.span(z, length);
        self.pick = Self::stretch_or_whole(from_mm, to_mm.min(length), length);
        self.zone = z;
    }

    /// After an edit, the history or the frame moved: a stretch that runs past the street's end ends there, an end
    /// that a boundary between runs now lies within `MIN_ZONE_MM` of snaps to it (as picking would have), and a
    /// stretch left shorter than `MIN_ZONE_MM` (or lying past the end) becomes the whole street, so that no edit makes
    /// a sliver. The shown zone is the one at the pick's left end again. (A street with no frame keeps the span of a
    /// zone the zone strip chose, along the length its zones were drawn on.)
    pub(super) fn clamp_pick(&mut self) {
        if let Pick::Stretch { from_mm, to_mm } = self.pick {
            let length = self.pick_length_mm();
            let (lo, hi) = self.snap_to_runs(from_mm.max(0), to_mm.min(length), length);
            self.pick = if hi - lo < MIN_ZONE_MM { Pick::Whole } else { Self::stretch_or_whole(lo, hi, length) };
        }
        self.zone = self.shown_zone();
    }

    /// Each of `lo` and `hi` moved to a street end or a boundary between runs closer than `MIN_ZONE_MM`, if one is.
    fn snap_to_runs(&self, lo: i32, hi: i32, length: i32) -> (i32, i32) {
        let marks: Vec<i32> = [0, length].into_iter().chain(self.runs().iter().skip(1).map(|r| r.from_mm)).collect();
        let near = |x: i32| marks.iter().copied().filter(|m| (m - x).abs() < MIN_ZONE_MM).min_by_key(|m| (m - x).abs()).unwrap_or(x);
        (near(lo), near(hi))
    }

    /// The spans (anchored) where an edit over the pick missed: where the edit found nothing to change that the shown
    /// zone had, each with the kind of the piece it lacked. Cleared by the next edit and by a move through the history.
    pub fn missed(&self) -> Vec<(i32, i32, usize)> {
        self.missed.clone()
    }

    /// Makes an edit over the pick as one revision: the street is split at the pick's ends where they fall inside a
    /// zone (`split_at_pick`; not on a street whose zones were drawn elsewhere, nor with no frame), `change` makes the
    /// edit on the zones the pick covers (given their range, the shown zone and whether the pick is the whole street,
    /// and returning the zones where it missed), and identical neighbours join (`join_alike`; not on a street whose
    /// zones were drawn elsewhere). False, with nothing recorded, when the street is as it was.
    ///
    /// Inside a drag, the split is made on the drag's first changing step and every step is made again from what it
    /// left, so a step back undoes a step; the edit is shown at once, and the join and the revision wait for
    /// `end_gesture`. Cancelling the drag leaves no boundary.
    pub(super) fn over_pick(&mut self, label: Said, change: impl FnOnce(&mut Content, Range<usize>, usize, bool) -> Vec<(usize, usize)>) -> bool {
        self.missed.clear();
        let (base, range) = match (&self.gesture, &self.gesture_pick) {
            (Some(_), Some(picked)) => picked.clone(),
            _ => self.split_for_pick(),
        };
        let shown = match self.anchor_end() {
            ZoneEnd::First => range.start,
            ZoneEnd::Last => range.end - 1,
        };
        let mut next = base.clone();
        let missed = change(&mut next, range.clone(), shown, self.pick == Pick::Whole);
        let spans = Self::spans_of(&next, &missed, self.pick_length_mm());
        if self.gesture.is_some() {
            self.gesture_pick = Some((base, range));
            self.states[self.cursor].content = next;
            self.zone = shown;
            self.pending_label = Some(label);
            self.missed = spans;
            return true;
        }
        if self.elsewhere().is_none() {
            next.join_alike(range);
        }
        next.settle();
        if next == self.states[self.cursor].content {
            return false;
        }
        self.push(label, next);
        self.missed = spans;
        true
    }

    /// The content an edit over the pick starts from, split at the pick's ends when the zones can be rearranged, and
    /// the zones the pick covers in it.
    fn split_for_pick(&self) -> (Content, Range<usize>) {
        let mut base = self.states[self.cursor].content.clone();
        let range = match (self.pick, self.structure_frame(true)) {
            (Pick::Stretch { .. }, Ok(frame)) => base.split_at_pick(self.pick, frame),
            _ => self.zones_in_pick(),
        };
        (base, range)
    }

    /// The anchored spans of `zones` (each with a kind) in `content`, neighbouring ones of the same kind as one.
    fn spans_of(content: &Content, zones: &[(usize, usize)], length_mm: i32) -> Vec<(i32, i32, usize)> {
        let mut spans: Vec<(i32, i32, usize)> = Vec::new();
        for &(z, kind) in zones {
            let (from, to) = content.span(z, length_mm);
            match spans.last_mut() {
                Some(last) if last.1 == from && last.2 == kind => last.1 = to.min(length_mm),
                _ => spans.push((from, to.min(length_mm), kind)),
            }
        }
        spans
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
    pub fn anchor_end(&self) -> ZoneEnd {
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

/// The side of the section piece `j` lies on, by its middle: the word for it.
fn side_word(section: &[Segment], j: usize) -> &'static str {
    let before: i32 = section[..j].iter().map(|s| s.width_mm).sum();
    let total: i32 = section.iter().map(|s| s.width_mm).sum();
    if 2 * before + section[j].width_mm < total { "side-word-left" } else { "side-word-right" }
}

impl Content {
    /// Whether zones `a` and `b` look the same: the same pieces (ignoring uids) and the same speed limit.
    pub(super) fn alike(&self, a: usize, b: usize) -> bool {
        same_pieces(self.section(a), self.section(b)) && speed_in(self.speed, &self.zones, a) == speed_in(self.speed, &self.zones, b)
    }

    /// Puts a boundary at each end of a stretch that falls inside a zone, the new zone copying the section and speed
    /// of the zone it is cut from, with no name; the zoning is then measured along `frame`, as `split_zone` does.
    /// Returns the zones now covering the pick (every zone, for the whole street). A boundary is never put where it
    /// would leave the part outside the pick shorter than `MIN_ZONE_MM`: that only happens beside a boundary inside a
    /// run (the pick snaps to the boundaries between runs), which moves to the pick's end instead, the zones on either
    /// side of it being alike.
    pub(super) fn split_at_pick(&mut self, pick: Pick, frame: ZoneFrame) -> Range<usize> {
        let Pick::Stretch { from_mm, to_mm } = pick else {
            return 0..self.zone_count();
        };
        let length = frame.length_mm;
        let needs = |c: &Content, at: i32| 0 < at && at < length && !c.zones.iter().any(|g| g.from_mm == at);
        if needs(self, from_mm) || needs(self, to_mm) {
            if self.zones.is_empty() {
                self.zones.push(Zone { from_mm: 0, segments: None, name: None, speed: Speed::Undecided });
            }
            self.from_end = frame.from_end;
            self.length_mm = Some(length);
            if needs(self, from_mm) {
                self.split_at(from_mm, length, true);
            }
            if needs(self, to_mm) {
                self.split_at(to_mm, length, false);
            }
        }
        let start = self.zones.iter().rposition(|g| g.from_mm <= from_mm).unwrap_or(0);
        let end = self.zones.iter().filter(|g| g.from_mm < to_mm).count().max(start + 1);
        start..end
    }

    /// Puts a boundary at `at_mm`, inside a zone; the pick lies after it when `pick_after`, else before it.
    fn split_at(&mut self, at_mm: i32, length_mm: i32, pick_after: bool) {
        let z = self.zones.iter().rposition(|g| g.from_mm < at_mm).unwrap_or(0);
        let (start, end) = self.span(z, length_mm);
        if pick_after && at_mm - start < MIN_ZONE_MM && z > 0 && self.alike(z - 1, z) {
            self.zones[z].from_mm = at_mm;
        } else if !pick_after && end - at_mm < MIN_ZONE_MM && z + 1 < self.zones.len() && self.alike(z, z + 1) {
            self.zones[z + 1].from_mm = at_mm;
        } else {
            let zone = Zone { from_mm: at_mm, name: None, ..self.zones[z].clone() };
            self.zones.insert(z + 1, zone);
        }
    }

    /// Joins neighbouring zones that look the same (`alike`) among the zones `around` and one more on each side: the
    /// earlier zone stays, with its section (and so its uids), name and speed, and reaches over the later one.
    pub(super) fn join_alike(&mut self, around: Range<usize>) {
        let mut z = around.start.saturating_sub(1) + 1;
        let mut end = (around.end + 1).min(self.zones.len());
        while z < end {
            if self.alike(z - 1, z) {
                self.zones.remove(z);
                end -= 1;
            } else {
                z += 1;
            }
        }
    }

    /// Gives each zone in `range` the section `change` makes of its own (`change` is told whether it is the zone
    /// `shown`, and gives the kind of the piece it missed, if it did). For the whole street (`whole`) the base section is changed once, for
    /// the zones that follow it, and a zone past `range` that follows it (one cut off) is first given a copy of it,
    /// so that it is not changed. Returns the zones where the change missed, each with the kind it missed.
    pub(super) fn change_sections(
        &mut self,
        range: Range<usize>,
        shown: usize,
        whole: bool,
        change: impl Fn(bool, &[Segment]) -> (Vec<Segment>, Option<usize>),
    ) -> Vec<(usize, usize)> {
        let mut missed = Vec::new();
        if whole && !self.zones.is_empty() {
            let base = self.segments.clone();
            for zone in self.zones[range.end..].iter_mut().filter(|g| g.segments.is_none()) {
                zone.segments = Some(base.clone());
            }
            let shown_follows = self.zones.get(shown).is_some_and(|g| g.segments.is_none());
            let (base, base_missed) = change(shown_follows, &base);
            self.segments = base;
            for z in range {
                match self.zones[z].segments.take() {
                    None => {
                        if let Some(kind) = base_missed {
                            missed.push((z, kind));
                        }
                    }
                    Some(own) => {
                        let (own, m) = change(z == shown, &own);
                        self.zones[z].segments = Some(own);
                        if let Some(kind) = m {
                            missed.push((z, kind));
                        }
                    }
                }
            }
        } else {
            for z in range {
                let (section, m) = change(z == shown, self.section(z));
                *self.section_mut(z) = section;
                if let Some(kind) = m {
                    missed.push((z, kind));
                }
            }
        }
        missed
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::zoned;
    use super::*;
    use crate::shared::catalogue::{Side, kind_index};
    use crate::street::model::{Segment, Speed, Street, StreetExt, ZoneFrame, ZoneRefusal};

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

    #[test]
    fn moving_a_named_zone_s_start_leaves_the_pick_and_the_selected_piece_alone() {
        let mut e = three_runs();
        e.pick_stretch(10_000, 30_000).unwrap();
        let uid = e.current()[0].uid;
        e.select(Some(uid));
        let revisions = e.states.len();
        e.begin_gesture();
        assert_eq!(e.move_boundary_of(2, 38_000), Ok(()));
        assert_eq!(e.move_boundary_of(2, 37_000), Ok(()));
        assert_eq!((e.pick(), e.zone(), e.view().selected), (stretch(10_000, 30_000), 0, Some(uid)), "nothing follows while it lasts");
        assert!(e.end_gesture());
        assert_eq!(e.states.len(), revisions + 1);
        assert_eq!(e.snapshot().zones.iter().map(|z| z.from_mm).collect::<Vec<_>>(), [0, 20_000, 37_000]);
        assert_eq!((e.pick(), e.view().selected), (stretch(10_000, 30_000), Some(uid)));
        e.begin_gesture();
        assert_eq!(e.move_boundary_of(2, 45_000), Ok(()));
        e.cancel_gesture();
        assert_eq!(e.snapshot().zones.iter().map(|z| z.from_mm).collect::<Vec<_>>(), [0, 20_000, 37_000]);
        assert_eq!(e.move_boundary_of(0, 5_000), Err(ZoneRefusal::NoBoundary));
        assert_eq!(e.move_boundary_of(2, 22_000), Err(ZoneRefusal::TooShort));
    }

    #[test]
    fn what_sets_a_picked_run_apart_is_read_against_the_run_on_its_left_else_on_its_right() {
        let words = |e: &Editor| e.pick_differs().iter().map(|s| s.key).collect::<Vec<_>>();
        // The middle run lacks the left parking, its right sidewalk is wider, it has a bike lane and 30 km/h.
        let base = sample();
        let mut s = zoned(base.clone(), &[0, 20_000, 40_000], LEN);
        let mut middle = base.segments.clone();
        middle.remove(1);
        middle.last_mut().unwrap().width_mm += 200;
        let at = middle.len() - 1;
        middle.insert(at, Segment::new(900, kind_index("bike").unwrap(), 1_800));
        s.zones[1].segments = Some(middle);
        s.zones[1].speed = Speed::Kmh(30);
        let mut e = editor(&s, frame(LEN, ZoneEnd::First));
        assert!(e.pick_differs().is_empty(), "the whole street");
        e.pick_stretch(20_000, 40_000).unwrap();
        let said = e.pick_differs();
        assert_eq!(words(&e), ["stretch-what-dropped", "stretch-what-added", "stretch-what-width", "stretch-what-speed"]);
        assert_eq!(said[0], Said::new("stretch-what-dropped").with("kind", Arg::KindLower("parking")).with("side", Arg::Msg("side-word-left")));
        assert_eq!(said[1], Said::new("stretch-what-added").with("kind", Arg::KindLower("bike")).with("side", Arg::Msg("side-word-right")));
        let sidewalk = base.segments.last().unwrap().width_mm + 200;
        assert_eq!(
            said[2],
            Said::new("stretch-what-width")
                .with("kind", Arg::KindLower("sidewalk"))
                .with("side", Arg::Msg("side-word-right"))
                .with("width", Arg::Length(sidewalk))
        );
        assert_eq!(said[3], Said::new("stretch-what-speed").with("kmh", Arg::Num(30)));
        // Against the left neighbour: the first run, read against the middle one on its right (it has no left one).
        e.pick_stretch(0, 20_000).unwrap();
        assert_eq!(words(&e), ["stretch-what-added", "stretch-what-dropped", "stretch-what-width", "stretch-what-no-speed"]);
        // A stretch that is not one whole run says nothing of it.
        e.pick_stretch(25_000, 35_000).unwrap();
        assert!(e.pick_differs().is_empty());
        // Anchored at the last end, the run on the plan's left is the next zone.
        let mut s = s.clone();
        s.from_end = ZoneEnd::Last;
        s.zones[2].segments = s.zones[1].segments.clone();
        s.zones[2].speed = Speed::Kmh(30);
        s.zones[1].segments = None;
        s.zones[1].speed = Speed::Undecided;
        s.zones[0].speed = Speed::Kmh(40);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(frame(LEN, ZoneEnd::Last));
        e.pick_stretch(20_000, 40_000).unwrap();
        assert_eq!(words(&e), ["stretch-what-added", "stretch-what-dropped", "stretch-what-width", "stretch-what-no-speed"], "read against zone 2, not zone 0");
    }

    #[test]
    fn a_reload_keeps_the_pick_where_it_is_on_the_plan_even_from_the_other_end() {
        let mut was = editor(&sample(), frame(LEN, ZoneEnd::First));
        was.pick_stretch(10_000, 30_000).unwrap();
        let mut now = editor(&street(&[0, 20_000], &[0, 100], ZoneEnd::Last), frame(LEN, ZoneEnd::Last));
        now.keep_choices(&was);
        assert_eq!(now.view().plan.pick, Some((10_000, 30_000)));
        assert_eq!(now.pick(), stretch(30_000, 50_000));
    }

    // ---- edits over the pick ----

    /// Where each zone of the street as it stands starts.
    fn starts(e: &Editor) -> Vec<i32> {
        e.snapshot().zones.iter().map(|z| z.from_mm).collect()
    }

    /// The width of piece `i` in each zone of the street as it stands (the one section of a street with no zones).
    fn widths_of(e: &Editor, i: usize) -> Vec<i32> {
        e.snapshot().sections().iter().map(|s| s[i].width_mm).collect()
    }

    /// The uid of piece `i` of the shown section.
    fn uid_of(e: &Editor, i: usize) -> u32 {
        e.current()[i].uid
    }

    #[test]
    fn picking_alone_never_splits() {
        let mut e = editor(&sample(), frame(120_000, ZoneEnd::First));
        assert_eq!(e.pick_stretch(40_000, 80_000), Ok(stretch(40_000, 80_000)));
        assert!(e.snapshot().zones.is_empty());
        assert_eq!(e.zones_in_pick(), 0..1);
    }

    #[test]
    fn an_edit_in_a_stretch_splits_at_its_ends_and_changes_only_inside() {
        let mut e = editor(&sample(), frame(LEN, ZoneEnd::First));
        e.pick_stretch(20_000, 40_000).unwrap();
        assert!(e.set_width(uid_of(&e, 0), 4_000));
        assert_eq!(starts(&e), [0, 20_000, 40_000]);
        let s = e.snapshot();
        let base = sample().segments;
        assert!(same_pieces(s.sections()[0], &base), "before the stretch, as it was");
        assert!(same_pieces(s.sections()[2], &base), "after the stretch, as it was");
        assert_eq!(widths_of(&e, 0), [3_300, 4_000, 3_300]);
        assert_eq!(&s.sections()[1][1..], &base[1..], "only the piece edited changed inside");
        assert_eq!((e.pick(), e.zone()), (stretch(20_000, 40_000), 1), "the pick keeps its span and shows its zone");
        assert!(e.undo());
        assert!(e.snapshot().zones.is_empty(), "the split and the edit are one step");
    }

    #[test]
    fn a_whole_street_edit_reaches_the_same_piece_in_every_run_and_leaves_a_piece_only_one_run_has() {
        // The middle run has a bike lane where the others park, and fresh uids, as a template gives.
        let mut s = zoned(sample(), &[0, 20_000, 40_000], LEN);
        let mut middle = s.segments.clone();
        middle[1] = Segment::new(100, kind_index("bike").unwrap(), 2_400);
        middle[0].uid = 101;
        s.zones[1].segments = Some(middle);
        let mut e = editor(&s, frame(LEN, ZoneEnd::First));
        assert_eq!(e.pick(), Pick::Whole);
        assert!(e.set_width(uid_of(&e, 0), 3_000));
        assert_eq!(widths_of(&e, 0), [3_000, 3_000, 3_000], "the sidewalk is 3 m everywhere");
        let after = e.snapshot();
        assert_eq!(after.zones.len(), 3);
        assert!(after.zones[0].segments.is_none() && after.zones[2].segments.is_none(), "the zones on the base still follow it");
        assert_eq!(after.sections()[1][1], Segment::new(100, kind_index("bike").unwrap(), 2_400), "the bike lane is intact");
        assert_eq!(after.sections()[1][0].uid, 101, "the school stretch keeps its uids");
        assert!(e.missed().is_empty());
    }

    #[test]
    fn an_edit_back_to_the_neighbours_section_joins_the_stretch_back() {
        let mut e = editor(&sample(), frame(LEN, ZoneEnd::First));
        e.pick_stretch(20_000, 40_000).unwrap();
        let uid = uid_of(&e, 0);
        assert!(e.set_width(uid, 4_000));
        assert_eq!(starts(&e), [0, 20_000, 40_000]);
        assert!(e.set_width(uid, 3_300));
        assert!(e.snapshot().zones.is_empty(), "identical neighbours join");
        assert_eq!(e.snapshot().segments, sample().segments);
        assert_eq!((e.pick(), e.zone()), (stretch(20_000, 40_000), 0));
    }

    /// The sample with zones at 0, 20 m and 40 m, the first two on the base (identical) and the last its own.
    fn identical_neighbours() -> Street {
        street(&[0, 20_000, 40_000], &[0, 0, 100], ZoneEnd::First)
    }

    #[test]
    fn a_saved_street_with_identical_neighbouring_zones_opens_unchanged() {
        let s = identical_neighbours();
        assert_eq!(s.zones.len(), 3);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(frame(LEN, ZoneEnd::First));
        assert_eq!(e.snapshot(), s, "opened as it was saved");
        assert_eq!(editor(&s, frame(LEN, ZoneEnd::First)).snapshot(), s, "as an earlier revision too");
        assert_eq!(e.pick_run_at(30_000), Ok(stretch(0, 40_000)), "a click picks both as one run");
        assert_eq!(e.snapshot(), s);
    }

    #[test]
    fn an_edit_touching_identical_neighbours_joins_them() {
        let mut e = editor(&identical_neighbours(), frame(LEN, ZoneEnd::First));
        e.pick_run_at(10_000).unwrap();
        assert!(e.set_width(uid_of(&e, 0), 4_000));
        assert_eq!(starts(&e), [0, 40_000]);
        assert_eq!(widths_of(&e, 0), [4_000, 3_400]);
        assert_eq!((e.pick(), e.zone()), (stretch(0, 40_000), 0));
    }

    #[test]
    fn an_edit_never_leaves_a_stretch_shorter_than_5_m() {
        // The pick starts 2 m into the second of two identical zones: the 2 m left of it go with the first.
        let mut e = editor(&identical_neighbours(), frame(LEN, ZoneEnd::First));
        assert_eq!(e.pick_stretch(22_000, LEN), Ok(stretch(22_000, LEN)));
        assert!(e.set_width(uid_of(&e, 0), 4_000));
        assert_eq!(starts(&e), [0, 22_000], "the stretch picked is one zone, with no 2 m zone before it");
        assert_eq!(widths_of(&e, 0), [3_300, 4_000]);
        // A history move that brings a boundary within 5 m of the pick's end snaps the end to it, as picking would.
        let mut e = editor(&sample(), frame(LEN, ZoneEnd::First));
        e.pick_stretch(10_000, 30_000).unwrap();
        assert!(e.set_width(uid_of(&e, 0), 4_000));
        assert!(e.undo());
        assert_eq!(e.pick_stretch(12_000, 50_000), Ok(stretch(12_000, 50_000)));
        assert!(e.redo());
        assert_eq!(e.pick(), stretch(10_000, 50_000));
        assert!(e.set_width(uid_of(&e, 0), 3_000));
        assert_eq!(starts(&e), [0, 10_000, 50_000]);
    }

    #[test]
    fn a_drag_in_a_picked_stretch_splits_once_and_is_one_revision() {
        let mut e = editor(&sample(), frame(LEN, ZoneEnd::First));
        e.pick_stretch(20_000, 40_000).unwrap();
        let (uid, revisions) = (uid_of(&e, 0), e.states.len());
        e.begin_gesture();
        for (by, zones) in [(100, 3), (300, 3), (200, 3)] {
            assert!(e.resize_edge(uid, by));
            assert_eq!(e.states[e.cursor].content.zones.len(), zones, "split once, at the first step");
            assert_eq!(e.zone(), 1);
            assert_eq!(e.current()[0].width_mm, 3_300 + by, "each step from where the drag started");
        }
        assert!(e.end_gesture());
        assert_eq!(e.states.len(), revisions + 1, "one revision");
        assert_eq!(starts(&e), [0, 20_000, 40_000]);
        assert_eq!(widths_of(&e, 0), [3_300, 3_500, 3_300]);
        assert!(e.undo());
        assert!(e.snapshot().zones.is_empty());
    }

    #[test]
    fn cancelling_that_drag_leaves_no_boundary() {
        let mut e = editor(&sample(), frame(LEN, ZoneEnd::First));
        e.pick_stretch(20_000, 40_000).unwrap();
        let (uid, revisions) = (uid_of(&e, 0), e.states.len());
        e.begin_gesture();
        assert!(e.resize_edge(uid, 300));
        e.cancel_gesture();
        assert!(e.snapshot().zones.is_empty());
        assert_eq!(e.snapshot().segments, sample().segments);
        assert_eq!(e.states.len(), revisions);
        assert_eq!((e.pick(), e.zone()), (stretch(20_000, 40_000), 0));
        // A drag back to where it started records nothing and leaves no boundary either.
        e.begin_gesture();
        assert!(e.resize_edge(uid, 300));
        assert!(e.resize_edge(uid, 0));
        assert!(!e.end_gesture());
        assert!(e.snapshot().zones.is_empty());
        assert_eq!(e.states.len(), revisions);
    }

    #[test]
    fn undo_after_split_and_join_restores_the_zones_exactly_and_clamps_the_pick() {
        // The middle run's sidewalk is 100 mm wider: making it the others' again joins the street into one.
        let mut e = three_runs();
        let zoned = e.snapshot();
        e.pick_run_at(30_000).unwrap();
        assert!(e.set_width(uid_of(&e, 0), 3_300));
        assert!(e.snapshot().zones.is_empty());
        assert_eq!((e.pick(), e.zone()), (stretch(20_000, 40_000), 0));
        assert!(e.undo());
        assert_eq!(e.snapshot(), zoned, "the zones exactly as they were");
        assert_eq!((e.pick(), e.zone(), e.zones_in_pick()), (stretch(20_000, 40_000), 1, 1..2));
        assert!(e.redo());
        assert_eq!((e.pick(), e.zone(), e.zones_in_pick()), (stretch(20_000, 40_000), 0, 0..1));
    }

    #[test]
    fn a_pick_clamped_shorter_than_5_m_is_the_whole_street() {
        let mut e = three_runs();
        e.pick_stretch(40_000, LEN).unwrap();
        let e = e.with_frame(frame(43_000, ZoneEnd::First));
        assert_eq!(e.pick(), Pick::Whole);
        assert_eq!(e.zone(), 0);
    }

    #[test]
    fn choosing_a_zone_does_nothing_while_a_drag_is_under_way() {
        let mut e = three_runs();
        e.pick_stretch(20_000, 40_000).unwrap();
        e.begin_gesture();
        assert!(!e.select_zone(2));
        assert_eq!((e.pick(), e.zone()), (stretch(20_000, 40_000), 1));
        e.cancel_gesture();
        assert!(e.select_zone(2));
        assert_eq!((e.pick(), e.zone()), (stretch(40_000, LEN), 2));
    }

    #[test]
    fn the_speed_of_a_stretch_is_set_for_it_and_whole_street_speed_clears_the_zones_own() {
        let mut s = sample();
        s.speed = Speed::Kmh(50);
        let mut e = editor(&s, frame(LEN, ZoneEnd::First));
        e.pick_stretch(20_000, 40_000).unwrap();
        assert!(e.set_speed(Some(30)));
        let now = e.snapshot();
        assert_eq!(starts(&e), [0, 20_000, 40_000]);
        assert_eq!(now.zones.iter().map(|z| z.speed).collect::<Vec<_>>(), [Speed::Undecided, Speed::Kmh(30), Speed::Undecided]);
        assert_eq!(now.speed, Speed::Kmh(50));
        assert!(!e.set_speed(Some(30)), "the stretch has that speed already");
        assert!(e.follow_street_speed());
        assert!(e.snapshot().zones.is_empty(), "the stretch is the street's again and joins back");
        assert!(e.set_speed(Some(30)));
        e.pick_whole();
        assert!(e.set_speed(Some(40)));
        let now = e.snapshot();
        assert_eq!(now.speed, Speed::Kmh(40));
        assert!(now.zones.is_empty(), "no zone keeps a speed of its own, so they join");
    }

    #[test]
    fn an_elsewhere_street_edits_every_zone_of_the_picked_run_without_splitting_or_joining() {
        // Drawn on a road 60 m long, loaded on one 58 m long. Zones 0 and 1 are one run.
        let mut e = editor(&identical_neighbours(), frame(58_000, ZoneEnd::First));
        assert!(e.elsewhere().is_some());
        assert_eq!(e.pick_stretch(5_000, 10_000), Ok(stretch(0, 40_000)));
        assert!(e.set_width(uid_of(&e, 0), 4_000));
        assert_eq!(starts(&e), [0, 20_000, 40_000], "no boundary made or joined");
        assert_eq!(widths_of(&e, 0), [4_000, 4_000, 3_400]);
        e.pick_whole();
        assert!(e.set_width(uid_of(&e, 0), 4_200));
        assert_eq!(starts(&e), [0, 20_000, 40_000]);
        assert_eq!(widths_of(&e, 0), [4_200, 4_200, 4_200]);
    }

    #[test]
    fn cut_off_zones_are_never_edited() {
        // Drawn on a road 60 m long, loaded on one 55 m long: the zone from 58 m, on the base, is cut off.
        let mut s = street(&[0, 30_000, 58_000], &[0, 100, 0], ZoneEnd::First);
        s.speed = Speed::Kmh(50);
        let mut e = editor(&s, frame(55_000, ZoneEnd::First));
        assert!(e.set_width(uid_of(&e, 0), 4_000));
        assert_eq!(widths_of(&e, 0), [4_000, 4_000, 3_300], "the cut-off zone keeps the section it had");
        assert!(e.set_speed(Some(30)));
        let now = e.snapshot();
        assert_eq!((0..3).map(|z| speed_in(now.speed, &now.zones, z)).collect::<Vec<_>>(), [Speed::Kmh(30), Speed::Kmh(30), Speed::Kmh(50)]);
    }

    #[test]
    fn a_change_to_a_piece_one_run_lacks_reports_that_run_as_missed() {
        // The middle run has no parking on its first side.
        let mut s = zoned(sample(), &[0, 20_000, 40_000], LEN);
        let mut middle = s.segments.clone();
        middle.remove(1);
        s.zones[1].segments = Some(middle);
        let mut e = editor(&s, frame(LEN, ZoneEnd::First));
        assert!(e.set_width(uid_of(&e, 1), 2_600));
        assert_eq!(e.missed(), [(20_000, 40_000, kind_index("parking").unwrap())], "the stretch lacks the parking changed");
        assert_eq!(widths_of(&e, 1), [2_600, 3_300, 2_600], "the middle run's travel lane is left alone");
        assert!(e.set_width(uid_of(&e, 0), 3_000));
        assert!(e.missed().is_empty(), "the next edit clears it");
    }

    #[test]
    fn an_edit_on_a_street_anchored_at_its_last_end_is_made_on_the_zone_shown_at_the_left() {
        let mut e = editor(&street(&[0, 20_000, 40_000], &[0, 100, 200], ZoneEnd::Last), frame(LEN, ZoneEnd::Last));
        e.pick_stretch(30_000, 50_000).unwrap();
        assert_eq!(e.zone(), 2);
        assert!(e.set_width(uid_of(&e, 0), 4_000));
        // Split at 30 m and 50 m; the two zones inside, alike now, join.
        assert_eq!(starts(&e), [0, 20_000, 30_000, 50_000]);
        assert_eq!(widths_of(&e, 0), [3_300, 3_400, 4_000, 3_500]);
        assert_eq!((e.pick(), e.zone()), (stretch(30_000, 50_000), 2));
    }

    #[test]
    fn every_zone_in_the_pick_can_be_made_pedestrian_or_given_one_section() {
        let mut e = three_runs();
        let walk = kind_index("sidewalk").unwrap();
        assert!(e.make_pedestrian(walk));
        assert!(e.snapshot().sections().iter().all(|s| !s.iter().any(Segment::has_motor)), "every run is pedestrian");
        let mut e = three_runs();
        e.pick_stretch(10_000, 30_000).unwrap();
        let section = e.current().to_vec();
        assert!(e.edit_uniform(Said::new("rev-measure"), section));
        // The pick has the shown section throughout, and joins the zone before it, which has it too.
        assert_eq!(starts(&e), [0, 30_000, 40_000]);
        assert_eq!(widths_of(&e, 0), [3_300, 3_400, 3_300]);
    }

    #[test]
    fn a_drag_back_to_where_it_started_leaves_the_shown_zone_the_picked_one() {
        // The drag splits the middle run at 25 m and 35 m; back where it started, the split is undone.
        let mut e = three_runs();
        e.pick_stretch(25_000, 35_000).unwrap();
        let zoned = e.snapshot();
        let uid = uid_of(&e, 0);
        e.begin_gesture();
        assert!(e.resize_edge(uid, 300));
        assert!(e.resize_edge(uid, 0));
        assert!(!e.end_gesture());
        assert_eq!(e.snapshot(), zoned);
        assert_eq!((e.pick(), e.zone()), (stretch(25_000, 35_000), 1));
        // The next edit is made on the picked run's own section, not on another run's pieces.
        assert!(e.set_width(uid, 3_000));
        assert_eq!(starts(&e), [0, 20_000, 25_000, 35_000, 40_000]);
        assert_eq!(widths_of(&e, 0), [3_300, 3_400, 3_000, 3_400, 3_300]);
        assert_eq!(&e.snapshot().sections()[2][1..], &zoned.sections()[1][1..]);
    }
}
