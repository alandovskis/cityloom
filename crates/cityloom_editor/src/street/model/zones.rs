//! Zones: a street divided along its length into ordered stretches, each with the street's base section or
//! a section of its own. A street with no zones is the one-section street the city has always kept.

#[cfg(test)]
use super::Zone;
use super::{ZoneEnd, ZoneFrame};
use serde::Serialize;

#[cfg(test)]
use super::Speed;
use super::transition::{Host, Transition, transition};
use super::zone_speed::taper_kmh;
use super::{Arg, Editor, Said, snap};

/// Why a transition fails its check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failing {
    /// The host zone is shorter than the taper.
    Room { need_mm: i32, room_mm: i32 },
    /// A lane ends with no lane beside it to merge into.
    LaneDrop,
}

/// The shortest a zone may be, in millimetres.
pub const MIN_ZONE_MM: i32 = 5_000;

/// Why a zone operation was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneRefusal {
    /// Both ends of the street are cut by the map's box, so there is no end to measure from.
    NoAnchor,
    /// The zones were drawn on a road of another length or anchored at its other end.
    Elsewhere,
    /// A zone would be shorter than `MIN_ZONE_MM`.
    TooShort,
    OutsideStreet,
    /// The first zone has no start boundary.
    NoBoundary,
    /// A drag is under way: nothing but the drag may change the zones.
    Dragging,
}

impl ZoneRefusal {
    /// In the words of the page, which knows stretches and the boundaries between them, not zones.
    pub fn said(self) -> Said {
        let key = match self {
            ZoneRefusal::NoAnchor => "pick-refused-no-anchor",
            ZoneRefusal::Elsewhere => "boundary-refused-elsewhere",
            ZoneRefusal::TooShort => "pick-refused-too-short",
            ZoneRefusal::OutsideStreet => "pick-refused-outside",
            ZoneRefusal::NoBoundary => "pick-refused-no-boundary",
            ZoneRefusal::Dragging => "zone-refused-dragging",
        };
        let said = Said::new(key);
        if self == ZoneRefusal::TooShort { said.with("min", Arg::Length(MIN_ZONE_MM)) } else { said }
    }
}

/// The zones of a street as the view carries them.
#[derive(Serialize)]
pub struct ZonesView {
    pub zones: Vec<ZoneView>,
    pub selected: usize,
    /// The street's length the zones lie along: the length they were drawn on, else the loaded one, else 0.
    pub length_mm: i32,
    pub from_end: ZoneEnd,
    /// The street has more than one zone.
    pub several: bool,
    /// Set when the zones were drawn on a road of another length or anchored at its other end.
    pub elsewhere: Option<Elsewhere>,
    /// The zone across the nearest boundary, for the 2.5D; `None` when no boundary is near.
    pub joint: Option<super::Joint>,
}

#[derive(Serialize)]
pub struct ZoneView {
    pub index: usize,
    pub from_mm: i32,
    pub to_mm: i32,
    /// The stretch of the run it is part of, from the plan's left end: "from 49 m to 82 m".
    pub label: Said,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Elsewhere {
    pub drawn_mm: i32,
    pub loaded_mm: i32,
}

/// A stretch of the street as the page names it, by where it lies from the plan's left end (the street's first end):
/// "from 49 m to 82 m", in the middle of a sentence. A zone's saved name is never said.
pub fn stretch_between(from_mm: i32, to_mm: i32) -> Said {
    Said::new("stretch-mid").with("from", Arg::Length(from_mm)).with("to", Arg::Length(to_mm))
}

/// How far the length a zoning was drawn against may differ from the loaded one before it is another road.
pub const LENGTH_TOLERANCE_MM: i32 = 1_000;

impl super::Content {
    /// Where zone `z` starts and ends, the last ending at the street's `length_mm`.
    pub(super) fn span(&self, z: usize, length_mm: i32) -> (i32, i32) {
        (self.zones.get(z).map_or(0, |g| g.from_mm), self.zones.get(z + 1).map_or(length_mm, |g| g.from_mm))
    }
}

impl Editor {
    /// Whether the zones can be rearranged: the street has an end to measure from, and a zoning it already has
    /// was drawn on this road.
    pub(super) fn structure_frame(&self, allow_drag: bool) -> Result<ZoneFrame, ZoneRefusal> {
        if self.gesture.is_some() && !allow_drag {
            return Err(ZoneRefusal::Dragging);
        }
        let frame = self.frame.ok_or(ZoneRefusal::NoAnchor)?;
        if self.elsewhere().is_some() {
            return Err(ZoneRefusal::Elsewhere);
        }
        Ok(frame)
    }

    /// Set when the zones were drawn on a road of another length (by more than a metre) or anchored at its other end.
    pub(super) fn elsewhere(&self) -> Option<Elsewhere> {
        let frame = self.frame?;
        let content = &self.states[self.cursor].content;
        let drawn = content.length_mm?;
        ((drawn - frame.length_mm).abs() > LENGTH_TOLERANCE_MM || content.from_end != frame.from_end)
            .then_some(Elsewhere { drawn_mm: drawn, loaded_mm: frame.length_mm })
    }

    /// Whether zone `z` starts beyond the loaded end of the street. It is kept, but not edited from here.
    pub(super) fn is_cut_off(&self, z: usize) -> bool {
        let Some(frame) = self.frame else { return false };
        self.states[self.cursor].content.zones.get(z).is_some_and(|g| g.from_mm >= frame.length_mm)
    }

    /// The zones as the strip shows them.
    pub(super) fn zones_view(&self) -> ZonesView {
        let content = &self.states[self.cursor].content;
        let length_mm = self.street_length_mm().unwrap_or(0);
        let from_end = if content.zones.is_empty() { self.frame.map_or(ZoneEnd::First, |f| f.from_end) } else { content.from_end };
        let runs = self.runs();
        let one = |z: usize| {
            let (from_mm, to_mm) = content.span(z, length_mm);
            // Named by the run it is part of, which the plan shows: a boundary inside a run is not seen.
            let label = runs.iter().find(|r| r.zones.contains(&z)).map_or_else(|| self.stretch_said(z), |r| self.stretch_of(r.from_mm, r.to_mm));
            ZoneView { index: z, from_mm, to_mm, label }
        };
        ZonesView {
            zones: (0..content.zone_count()).map(one).collect(),
            selected: self.zone,
            length_mm,
            from_end,
            several: content.zones.len() > 1,
            elsewhere: self.elsewhere(),
            joint: self.joint(),
        }
    }

    /// One entry per boundary `b` (between zones `b` and `b + 1`, in anchored order) whose two sections differ enough
    /// to need a taper. None with fewer than two zones or when the zoning was drawn against another length.
    pub(super) fn transitions(&self) -> Vec<(usize, Transition)> {
        let content = &self.states[self.cursor].content;
        if content.zones.len() < 2 || self.elsewhere().is_some() {
            return Vec::new();
        }
        // The pieces' own kinds, never a time-of-day variant's: a transition does not change with the clock.
        (0..content.zones.len() - 1)
            .filter_map(|b| {
                let speed = taper_kmh(content.speed, &content.zones, b);
                transition(content.section(b), content.section(b + 1), speed, content.from_end).map(|t| (b, t))
            })
            .collect()
    }

    /// The transitions that fail a check: `(boundary, host zone, which)`.
    pub(super) fn failing_transitions(&self) -> Vec<(usize, usize, Failing)> {
        let content = &self.states[self.cursor].content;
        let length_mm = self.street_length_mm().unwrap_or(0);
        let mut out = Vec::new();
        for (b, t) in self.transitions() {
            let host = if t.host == Host::Near { b } else { b + 1 };
            let (from, to) = content.span(host, length_mm);
            if to - from < t.length_mm {
                out.push((b, host, Failing::Room { need_mm: t.length_mm, room_mm: to - from }));
            }
            if !t.merges {
                out.push((b, host, Failing::LaneDrop));
            }
        }
        out
    }

    /// Zone `z` of the current content as the stretch of the plan it covers (`stretch_between`), cut to the street's
    /// loaded length.
    pub(super) fn stretch_said(&self, z: usize) -> Said {
        let length = self.pick_length_mm();
        let (from_mm, to_mm) = self.states[self.cursor].content.span(z, length);
        self.stretch_of(from_mm.min(length), to_mm.min(length))
    }

    /// The stretch between two anchored positions, named from the plan's left end (`stretch_between`). A street with
    /// no end to measure from has no positions: it is named as the whole street, or as part of it.
    pub(super) fn stretch_of(&self, a_mm: i32, b_mm: i32) -> Said {
        if self.frame.is_none() {
            let whole = a_mm.min(b_mm) <= 0 && a_mm.max(b_mm) >= self.pick_length_mm();
            return Said::new(if whole { "stretch-mid-whole" } else { "stretch-mid-part" });
        }
        let (a, b) = self.left_span(a_mm, b_mm);
        stretch_between(a, b)
    }

    /// Splits the selected zone at `at_mm` (default its middle, snapped to `SNAP_MM`). Both halves keep the section;
    /// the first keeps the name; the new zone is selected. Tests build zoned streets with it; the page splits only
    /// through an edit of the pick.
    #[cfg(test)]
    pub fn split_zone(&mut self, at_mm: Option<i32>) -> Result<(), ZoneRefusal> {
        let frame = self.structure_frame(false)?;
        let z = self.zone;
        let (start, end) = self.states[self.cursor].content.span(z, frame.length_mm);
        let at = snap(at_mm.unwrap_or(start + (end - start) / 2));
        if at < 0 || at > frame.length_mm {
            return Err(ZoneRefusal::OutsideStreet);
        }
        if at - start < MIN_ZONE_MM || end - at < MIN_ZONE_MM {
            return Err(ZoneRefusal::TooShort);
        }
        let label = Said::new("rev-step").with("n", Arg::Num(self.cursor as i64 + 1));
        self.edit_content(label, |c| {
            if c.zones.is_empty() {
                c.zones.push(Zone { from_mm: 0, segments: None, name: None, speed: Speed::Undecided });
            }
            let section = c.zones[z].segments.clone();
            c.zones.insert(z + 1, Zone { from_mm: at, segments: section, name: None, speed: c.zones[z].speed });
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        });
        self.follow_zone(z + 1);
        self.selected = None;
        Ok(())
    }

    /// Why the zones cannot be rearranged now (no anchor, another road, a drag under way), if they cannot.
    pub fn structure_blocked(&self) -> Option<ZoneRefusal> {
        self.structure_frame(false).err()
    }

    /// Moves the start of the selected zone to `to_mm`, keeping both neighbours at least `MIN_ZONE_MM` long. Inside a
    /// gesture it is shown as it goes and recorded as one revision at the end. The pick is the zone's span after it.
    #[cfg(test)]
    pub fn move_boundary(&mut self, to_mm: i32) -> Result<(), ZoneRefusal> {
        let z = self.move_boundary_of(self.zone, to_mm)?;
        // The pick is the zone's span, which has moved with its start.
        self.follow_zone(z);
        Ok(())
    }

    /// Moves the start of zone `z` to `to_mm`, as `move_boundary` does, leaving the pick (clamped once recorded) and
    /// the selected piece alone: what the plan's boundary drag moves. Gives back the zone the boundary starts after the
    /// move.
    ///
    /// The boundary is kept `MIN_ZONE_MM` from the boundaries the plan shows (between runs) and the street's ends, as
    /// `plan::boundary_kept` keeps a drag. A saved street may hold boundaries between zones that look the same (loading
    /// never joins them), hidden inside a run: the move joins the two runs it lies between into one zone each, so such
    /// a boundary neither stops it nor is left within the shortest stretch of it.
    pub fn move_boundary_of(&mut self, z: usize, to_mm: i32) -> Result<usize, ZoneRefusal> {
        let frame = self.structure_frame(true)?;
        let content = &self.states[self.cursor].content;
        if z == 0 || z >= content.zones.len() {
            return Err(ZoneRefusal::NoBoundary);
        }
        let to_mm = snap(to_mm);
        if to_mm < 0 || to_mm > frame.length_mm {
            return Err(ZoneRefusal::OutsideStreet);
        }
        // The runs either side: zones `first..z` before the boundary, `z..end` after it.
        let mut first = z - 1;
        while first > 0 && content.alike(first - 1, first) {
            first -= 1;
        }
        let mut end = z + 1;
        while end < content.zones.len() && content.alike(end - 1, end) {
            end += 1;
        }
        let (before, _) = content.span(first, frame.length_mm);
        let (_, after) = content.span(end - 1, frame.length_mm);
        if to_mm - before < MIN_ZONE_MM || after - to_mm < MIN_ZONE_MM {
            return Err(ZoneRefusal::TooShort);
        }
        let moved = first + 1;
        if content.zones[z].from_mm == to_mm && (first, end) == (z - 1, z + 1) {
            return Ok(z);
        }
        let label = Said::new("rev-boundary-move").with("to", Arg::Length(self.left_mm(to_mm)));
        let set = |c: &mut super::Content| {
            // The hidden boundaries go: the earlier zone of each run reaches over the rest of it (as a join keeps it).
            c.zones.drain(z + 1..end);
            c.zones.drain(first + 1..z);
            c.zones[moved].from_mm = to_mm;
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        };
        if self.gesture.is_some() {
            set(&mut self.states[self.cursor].content);
            self.pending_label = Some(label);
        } else {
            self.edit_content(label, set);
        }
        Ok(moved)
    }

    /// Moves the selected zone's start boundary by `delta_mm`. Does nothing where `move_boundary` refuses.
    #[cfg(test)]
    pub fn nudge_boundary(&mut self, delta_mm: i32) {
        let Some(from) = self.states[self.cursor].content.zones.get(self.zone).map(|z| z.from_mm) else {
            return;
        };
        let _ = self.move_boundary(from + delta_mm);
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::zoned;
    use super::*;
    use crate::shared::catalogue::{KINDS, Side, StreetClass, kind_index};
    use crate::street::model::Segment;
    use crate::street::model::Street;
    use crate::street::model::StreetExt;
    use cityloom_model::street::flip;

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
        s.zones = vec![Zone { from_mm: 7_000, segments: Some(own.clone()), name: Some("Lone".into()), speed: Speed::Undecided }];
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
        t.zones = vec![Zone { from_mm: 0, segments: Some(with_uids_shifted(&base)), name: Some("Lone".into()), speed: Speed::Undecided }];
        t.length_mm = Some(10_000);
        t.settle_zones();
        assert_eq!(t.segments, base);
        assert!(t.zones.is_empty());

        // sorted, first forced to 0
        let mut u = sample();
        u.zones = vec![
            Zone { from_mm: 30_000, segments: None, name: Some("b".into()), speed: Speed::Undecided },
            Zone { from_mm: 4_000, segments: None, name: Some("a".into()), speed: Speed::Undecided },
        ];
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

    // ---- the operations ------------------------------------------------

    const LEN: i32 = 60_000;

    fn frame() -> ZoneFrame {
        ZoneFrame { length_mm: LEN, from_end: ZoneEnd::First }
    }

    /// An editor on the sample street, one zone, with a frame; `starts` divide it first (as an earlier revision).
    fn editor(starts: &[i32]) -> Editor {
        let today = sample();
        let now = if starts.is_empty() { today.clone() } else { zoned(today.clone(), starts, LEN) };
        Editor::from_street(&today, &now, 0).with_frame(Some(frame()))
    }

    /// The same divided street, with zone `z` named `name` as a saved street may have it.
    fn named_editor(starts: &[i32], z: usize, name: &str) -> Editor {
        let today = sample();
        let mut now = zoned(today.clone(), starts, LEN);
        now.zones[z].name = Some(name.into());
        Editor::from_street(&today, &now, 0).with_frame(Some(frame()))
    }

    fn revisions(e: &Editor) -> usize {
        e.view().revisions.len()
    }

    fn starts(e: &Editor) -> Vec<i32> {
        e.snapshot().zones.iter().map(|z| z.from_mm).collect()
    }

    /// Gives the selected zone a section of its own that differs from the base, as an edit would.
    fn make_own(e: &mut Editor) {
        let uid = e.snapshot().segments[0].uid;
        let w = e.snapshot().segments[0].width_mm;
        assert!(e.set_width(uid, w + 100));
    }

    #[test]
    fn splitting_a_one_zone_street_in_the_middle_makes_two_zones_that_share_the_section_and_selects_the_new_one() {
        let mut e = editor(&[]);
        let before = revisions(&e);
        assert_eq!(e.split_zone(None), Ok(()));
        let s = e.snapshot();
        assert_eq!(starts(&e), [0, 30_000]);
        assert_eq!(s.length_mm, Some(LEN));
        assert_eq!(s.from_end, ZoneEnd::First);
        assert!(s.zones.iter().all(|z| z.segments.is_none() && z.name.is_none()));
        assert_eq!(e.zone(), 1);
        assert_eq!(revisions(&e), before + 1);
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-step", "only the tests split a zone by itself");
        assert!(e.undo());
        assert!(e.snapshot().zones.is_empty());
    }

    #[test]
    fn splitting_a_zone_with_its_own_section_gives_both_halves_the_same_pieces() {
        let mut e = editor(&[0, 20_000]);
        assert!(e.select_zone(1));
        make_own(&mut e);
        let own = e.snapshot().zones[1].segments.clone().unwrap();
        e.split_zone(Some(40_000)).unwrap();
        let s = e.snapshot();
        assert_eq!(starts(&e), [0, 20_000, 40_000]);
        assert_eq!(s.zones[1].segments.as_ref(), Some(&own));
        assert_eq!(s.zones[2].segments.as_ref(), Some(&own), "same pieces and uids");
        assert_eq!(e.zone(), 2);
    }

    #[test]
    fn the_first_half_keeps_the_name_and_the_new_zone_is_unnamed() {
        let mut e = named_editor(&[0, 20_000], 0, "Terrace");
        e.split_zone(Some(10_000)).unwrap();
        let s = e.snapshot();
        assert_eq!(s.zones[0].name.as_deref(), Some("Terrace"));
        assert_eq!(s.zones[1].name, None);
    }

    #[test]
    fn a_split_is_snapped_to_a_tenth_of_a_metre_and_refused_under_5_m_either_side_changing_nothing() {
        let mut e = editor(&[]);
        let before = revisions(&e);
        assert_eq!(e.split_zone(Some(4_900)), Err(ZoneRefusal::TooShort));
        assert_eq!(e.split_zone(Some(55_100)), Err(ZoneRefusal::TooShort));
        assert_eq!(e.split_zone(Some(61_000)), Err(ZoneRefusal::OutsideStreet));
        assert_eq!(e.split_zone(Some(-1_000)), Err(ZoneRefusal::OutsideStreet));
        assert_eq!(revisions(&e), before);
        assert!(e.snapshot().zones.is_empty());
        assert_eq!(e.split_zone(Some(5_000)), Ok(()));
        assert_eq!(starts(&e), [0, 5_000]);
        assert_eq!(e.split_zone(Some(30_040)), Ok(()), "the zone chosen is the new one, 5 000 to 60 000");
        assert_eq!(starts(&e), [0, 5_000, 30_000]);
    }

    #[test]
    fn a_street_cut_by_the_box_at_both_ends_cannot_be_split_and_says_why() {
        let s = sample();
        let mut e = Editor::from_street(&s, &s, 0);
        assert_eq!(e.split_zone(None), Err(ZoneRefusal::NoAnchor));
        assert_eq!(ZoneRefusal::NoAnchor.said().key, "pick-refused-no-anchor");
        assert_eq!(e.view().revisions.len(), 0);
    }

    #[test]
    fn every_refusal_is_said_in_the_words_of_stretches_and_boundaries() {
        use ZoneRefusal::*;
        let keys: Vec<_> = [NoAnchor, Elsewhere, TooShort, OutsideStreet, NoBoundary, Dragging].map(|r| r.said().key).to_vec();
        let expected = [
            "pick-refused-no-anchor",
            "boundary-refused-elsewhere",
            "pick-refused-too-short",
            "pick-refused-outside",
            "pick-refused-no-boundary",
            "zone-refused-dragging",
        ];
        assert_eq!(keys, expected);
        assert_eq!(TooShort.said(), Said::new("pick-refused-too-short").with("min", Arg::Length(MIN_ZONE_MM)));
    }

    #[test]
    fn a_street_whose_zones_were_drawn_at_another_length_or_end_refuses_every_structure_edit() {
        let today = sample();
        let now = zoned(today.clone(), &[0, 20_000, 40_000], LEN);
        let at = |len: i32, end: ZoneEnd| Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: len, from_end: end }));

        let mut e = at(58_500, ZoneEnd::First);
        assert_eq!(e.split_zone(Some(10_000)), Err(ZoneRefusal::Elsewhere));
        assert!(e.select_zone(1));
        assert_eq!(e.move_boundary(25_000), Err(ZoneRefusal::Elsewhere));
        make_own(&mut e);
        assert!(e.snapshot().zones[1].segments.is_some(), "a piece edit in a zone still works");

        let mut near = at(59_500, ZoneEnd::First);
        assert_eq!(near.split_zone(Some(10_000)), Ok(()));
        assert_eq!(near.snapshot().length_mm, Some(59_500), "a structure edit rewrites the length to the loaded one");

        let mut flipped = at(LEN, ZoneEnd::Last);
        assert_eq!(flipped.split_zone(Some(10_000)), Err(ZoneRefusal::Elsewhere));
    }

    #[test]
    fn a_boundary_moves_by_typed_value_and_by_nudges_and_stops_5_m_from_its_neighbours() {
        // Each zone its own speed limit, so that every boundary is one the plan shows.
        let today = sample();
        let mut now = zoned(today.clone(), &[0, 20_000, 40_000], LEN);
        for (z, kmh) in now.zones.iter_mut().zip([20, 30, 40]) {
            z.speed = Speed::Kmh(kmh);
        }
        let mut e = Editor::from_street(&today, &now, 0).with_frame(Some(frame()));
        assert_eq!(e.move_boundary(10_000), Err(ZoneRefusal::NoBoundary), "the first zone has no start boundary");
        assert!(e.select_zone(1));
        let before = revisions(&e);
        e.nudge_boundary(1_000);
        assert_eq!(starts(&e), [0, 21_000, 40_000]);
        e.nudge_boundary(-10_000);
        assert_eq!(starts(&e), [0, 11_000, 40_000]);
        assert_eq!(revisions(&e), before + 2);
        assert_eq!(e.view().revisions.last().unwrap().label, Said::new("rev-boundary-move").with("to", Arg::Length(11_000)));
        assert_eq!(e.move_boundary(4_940), Err(ZoneRefusal::TooShort));
        assert_eq!(e.move_boundary(35_100), Err(ZoneRefusal::TooShort));
        assert_eq!(e.move_boundary(35_050), Err(ZoneRefusal::TooShort), "snaps to 35 100, 4 900 mm from the next boundary");
        assert_eq!(e.move_boundary(30_040), Ok(()));
        assert_eq!(starts(&e), [0, 30_000, 40_000], "a typed position is stored on the 100 mm grid");
        assert_eq!(e.move_boundary(11_000), Ok(()));
        assert_eq!(e.move_boundary(35_000), Ok(()));
        assert_eq!(e.move_boundary(61_000), Err(ZoneRefusal::OutsideStreet));
        assert_eq!(e.move_boundary(5_000), Ok(()));
        e.nudge_boundary(-1_000);
        assert_eq!(starts(&e), [0, 5_000, 40_000], "a refused nudge changes nothing");
        assert_eq!(e.zone(), 1);
        assert!(e.select_zone(2));
        assert_eq!(e.move_boundary(40_000), Ok(()), "to where it is: nothing to record");
        let n = revisions(&e);
        assert_eq!(e.move_boundary(40_000), Ok(()));
        assert_eq!(revisions(&e), n);
    }

    #[test]
    fn structure_edits_are_refused_while_a_drag_is_under_way_and_leave_the_history_intact() {
        let mut e = editor(&[0, 20_000, 40_000]);
        assert!(e.select_zone(1));
        make_own(&mut e);
        let before = (revisions(&e), e.snapshot());
        e.begin_gesture();
        assert_eq!(e.move_boundary(25_000), Ok(()));
        assert_eq!(e.split_zone(Some(30_000)), Err(ZoneRefusal::Dragging));
        assert!(e.end_gesture());
        assert_eq!(revisions(&e), before.0 + 1);
        let s = e.snapshot();
        assert_eq!(starts(&e), [0, 25_000, 40_000]);
        assert_eq!((s.zones[1].name.clone(), s.zones.len()), (None, 3));
        assert!(e.undo());
        assert_eq!(e.snapshot(), before.1);
    }

    #[test]
    fn a_boundary_drag_is_one_revision() {
        let mut e = editor(&[0, 20_000]);
        assert!(e.select_zone(1));
        let before = revisions(&e);
        e.begin_gesture();
        for to in [21_000, 24_000, 26_000] {
            assert_eq!(e.move_boundary(to), Ok(()));
        }
        assert_eq!(starts(&e), [0, 26_000], "the drag shows as it goes");
        assert!(e.end_gesture());
        assert_eq!(revisions(&e), before + 1);
        assert_eq!(starts(&e), [0, 26_000]);
        assert!(e.undo());
        assert_eq!(starts(&e), [0, 20_000]);

        e.begin_gesture();
        assert_eq!(e.move_boundary(30_000), Ok(()));
        e.cancel_gesture();
        assert_eq!(starts(&e), [0, 20_000]);
    }

    #[test]
    fn a_street_with_no_end_to_measure_from_names_no_positions() {
        // A saved record of three stretches on a street whose ends are both cut: no frame.
        let today = sample();
        let mut now = super::super::zoned_apart(today.clone(), &[0, 20_000, 40_000], LEN);
        let mut narrow = today.segments.clone();
        narrow[0].width_mm = 600;
        now.zones[1].segments = Some(narrow);
        let e = Editor::from_street(&today, &now, 0).with_frame(None);
        assert_eq!(e.stretch_said(1), Said::new("stretch-mid-part"));
        assert_eq!(e.stretch_of(0, LEN), Said::new("stretch-mid-whole"));
        // A failing check names no position: the boundary between the stretches is "where the street changes".
        let failing: Vec<Said> = e.view().checks.into_iter().filter(|c| !c.ok).filter_map(|c| c.zone).collect();
        assert!(!failing.is_empty(), "the narrow sidewalk needs a transition it has no room for");
        assert!(failing.iter().all(|z| ["stretch-mid-part", "stretch-mid-whole", "check-boundary-somewhere"].contains(&z.key)), "{failing:?}");
        // With a frame, positions again.
        let e = Editor::from_street(&today, &now, 0).with_frame(Some(frame()));
        assert_eq!(e.stretch_said(1), stretch_between(20_000, 40_000));
    }

    #[test]
    fn a_zone_is_called_by_the_stretch_it_covers_from_the_plan_s_left_end_and_never_by_a_saved_name() {
        let labels = |e: &Editor| e.view().zones.zones.into_iter().map(|z| z.label).collect::<Vec<_>>();
        let today = sample();
        let mut now = super::super::zoned_apart(today.clone(), &[0, 20_000, 40_000], LEN);
        now.zones[1].name = Some("Market".into());
        let e = Editor::from_street(&today, &now, 0).with_frame(Some(frame()));
        assert_eq!(labels(&e), [stretch_between(0, 20_000), stretch_between(20_000, 40_000), stretch_between(40_000, LEN)]);
        // Zones that look the same are one run, which names each of them.
        let e = named_editor(&[0, 20_000, 40_000], 1, "Market");
        assert_eq!(labels(&e), [stretch_between(0, LEN), stretch_between(0, LEN), stretch_between(0, LEN)]);
        // Anchored at its last end, the first zone is the plan's rightmost stretch.
        let mut now = super::super::zoned_apart(today.clone(), &[0, 20_000], LEN);
        now.from_end = ZoneEnd::Last;
        let e = Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: ZoneEnd::Last }));
        assert_eq!(labels(&e), [stretch_between(40_000, LEN), stretch_between(0, 40_000)]);
        assert_eq!(stretch_between(20_000, 40_000), Said::new("stretch-mid").with("from", Arg::Length(20_000)).with("to", Arg::Length(40_000)));
    }

    #[test]
    fn no_section_holds_two_pieces_with_the_same_uid_after_any_operation() {
        fn check(e: &Editor, step: &str) {
            let s = e.snapshot();
            for section in std::iter::once(&s.segments).chain(s.zones.iter().filter_map(|z| z.segments.as_ref())) {
                let mut uids: Vec<_> = section.iter().map(|g| g.uid).collect();
                uids.sort_unstable();
                uids.dedup();
                assert_eq!(uids.len(), section.len(), "{step}");
            }
            assert!(s.is_sound(), "{step}");
        }
        let mut e = editor(&[]);
        e.split_zone(None).unwrap();
        check(&e, "split");
        make_own(&mut e);
        let added = e.add(kind_index("planting").unwrap(), 0);
        assert_ne!(added, 0);
        check(&e, "edit");
        e.split_zone(Some(45_000)).unwrap();
        check(&e, "split own");
        e.nudge_boundary(1_000);
        check(&e, "move");
        while e.undo() {
            check(&e, "undo");
        }
        while e.redo() {
            check(&e, "redo");
        }
    }

    #[test]
    fn the_view_says_whether_there_are_several_zones() {
        assert!(!editor(&[]).view().zones.several);
        assert!(named_editor(&[0, 20_000, 40_000], 1, "Market").view().zones.several);
    }

    #[test]
    fn a_change_of_side_mirrors_every_zones_own_section_as_well_as_the_base_in_one_revision() {
        let dirs = |segs: &[Segment]| segs.iter().map(|s| s.direction).collect::<Vec<_>>();
        let mut e = editor(&[0, 20_000, 40_000]);
        assert!(e.select_zone(1));
        make_own(&mut e);
        assert!(e.select_zone(2));
        // Widened by another width than zone 1, so that the two are not alike and do not join.
        let (uid, w) = (e.snapshot().segments[0].uid, e.snapshot().segments[0].width_mm);
        assert!(e.set_width(uid, w + 200));
        let before = e.snapshot();
        assert!(before.segments.iter().any(|s| s.direction.is_some()));
        let revs = revisions(&e);
        let uk = crate::shared::catalogue::REGIONS.iter().position(|r| r.id == "united-kingdom").unwrap();
        assert!(e.set_region(uk));
        assert_eq!(revisions(&e), revs + 1);
        let after = e.snapshot();
        let flip = |d: &Option<usize>| d.map(|d| 1 - d);
        let mirrored = |a: &[Segment], b: &[Segment]| {
            assert_eq!(dirs(b), dirs(a).iter().map(flip).collect::<Vec<_>>());
        };
        mirrored(&before.segments, &after.segments);
        for z in [1, 2] {
            mirrored(before.zones[z].segments.as_ref().unwrap(), after.zones[z].segments.as_ref().unwrap());
        }
        assert!(after.zones[0].segments.is_none());
        assert!(e.undo());
        let undone = e.snapshot();
        assert_eq!((undone.segments, undone.zones), (before.segments, before.zones), "one undo puts every section back");
    }

    fn check(e: &Editor, id: &str) -> Vec<super::super::Check> {
        e.view().checks.into_iter().filter(|c| c.id == id).collect()
    }

    fn say_in(locale: crate::shared::i18n::Locale, said: &Said) -> String {
        crate::street::vm::text::say_now(&crate::i18n_for(locale), crate::shared::units::Units::Metres, said)
    }

    /// Two zones split at `first_zone_mm`, the far one given a travel lane at the street's edge.
    fn lane_added_after(first_zone_mm: i32) -> Editor {
        let mut e = editor(&[0, first_zone_mm]);
        assert!(e.select_zone(1));
        let end = e.snapshot().segments.len();
        assert_ne!(e.add(kind_index("travel").unwrap(), end), 0);
        e
    }

    #[test]
    fn a_short_zone_before_an_added_lane_fails_transition_room() {
        let e = lane_added_after(20_000);
        let room = check(&e, "transition-room");
        assert_eq!(room.len(), 1);
        assert!(!room[0].ok);
        assert_eq!(room[0].zone, Some(Said::new("check-boundary").with("at", Arg::Length(20_000))));
        let detail = say_in(crate::shared::i18n::Locale::En, &room[0].detail);
        assert_eq!(detail, "The transition needs 49.5 m but the stretch from 0.0 m to 20.0 m is 20.0 m long.");
        assert!(!e.view().plan.runs[0].ok, "the host stretch");
    }

    #[test]
    fn a_transition_does_not_change_with_the_clock() {
        let mut e = editor(&[0, 30_000]);
        assert!(e.select_zone(1));
        let parking = kind_index("parking").unwrap();
        let uid = e.snapshot().segments.iter().find(|s| s.kind == parking).unwrap().uid;
        assert!(e.add_variant(uid), "a bus window on zone 2's parking lane");
        e.set_time(3 * 60);
        let night = (e.transitions(), check(&e, "lane-drop").len());
        e.set_time(8 * 60);
        let rush = (e.transitions(), check(&e, "lane-drop").len());
        assert!(!night.0.is_empty(), "the window widened the lane, so the boundary has a taper");
        assert!(night.0[0].1.changes.iter().all(|c| c.kind != kind_index("bus").unwrap()), "own kinds, never the window's");
        assert_eq!(night, rush);
    }

    #[test]
    fn the_same_layout_gives_the_same_lane_drop_whichever_end_it_is_anchored_at() {
        // A travel lane running away from the street's first end (direction 0), beside a sidewalk, in one of two zones.
        // In the zone at the first end it ends at the boundary and fails; in the zone at the last end it opens there and
        // passes. Anchoring at the other end changes which zone is "near" and nothing about the answer.
        let lane_drop = |lane_at_first_end: bool, anchored: ZoneEnd| {
            let today = sample();
            let mut now = zoned(today.clone(), &[0, 5_000], LEN);
            now.from_end = anchored;
            let mut own = today.segments.clone();
            let mut lane = Segment::new(900, kind_index("travel").unwrap(), 3_500);
            lane.direction = Some(0);
            own.push(lane);
            let index = usize::from(lane_at_first_end == (anchored == ZoneEnd::Last));
            now.zones[index].segments = Some(own);
            let e = Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end: anchored }));
            check(&e, "lane-drop").iter().map(|c| c.ok).collect::<Vec<_>>()
        };
        for at_first in [true, false] {
            assert_eq!(lane_drop(at_first, ZoneEnd::First), [!at_first], "lane in the zone at the {} end", if at_first { "first" } else { "last" });
            assert_eq!(lane_drop(at_first, ZoneEnd::Last), lane_drop(at_first, ZoneEnd::First));
        }
    }

    #[test]
    fn the_french_detail_keeps_its_units_after_a_no_break_space() {
        let e = lane_added_after(20_000);
        let detail = say_in(crate::shared::i18n::Locale::FrCa, &check(&e, "transition-room")[0].detail);
        assert!(detail.contains("49,5\u{a0}m"), "{detail}");
    }

    #[test]
    fn a_long_enough_zone_passes() {
        let e = lane_added_after(50_000);
        let room = check(&e, "transition-room");
        assert_eq!((room.len(), room[0].ok, room[0].zone.clone()), (1, true, None));
        assert!(e.view().plan.runs[0].ok);
        assert!(check(&e, "lane-drop")[0].ok);
    }

    #[test]
    fn a_dropped_lane_beside_parking_fails_lane_drop() {
        let mut e = editor(&[0, 30_000]);
        assert!(e.select_zone(1));
        // the lane running away from the first end ends at the boundary; uid 3 runs toward it, so it would open there
        assert!(e.remove(4));
        let drop = check(&e, "lane-drop");
        assert_eq!(drop.len(), 1);
        assert!(!drop[0].ok);
        assert!(drop[0].zone.is_some());
        assert_eq!(drop[0].detail, Said::new("check-detail-lane-drop"));
    }

    #[test]
    fn no_boundary_checks_for_one_zone_or_elsewhere() {
        let e = editor(&[]);
        assert!(check(&e, "transition-room").is_empty() && check(&e, "lane-drop").is_empty());
        let today = sample();
        let drawn = lane_added_after(20_000);
        let elsewhere = Editor::from_street(&today, &drawn.snapshot(), 0).with_frame(Some(ZoneFrame { length_mm: 58_000, from_end: ZoneEnd::First }));
        assert!(elsewhere.view().zones.elsewhere.is_some());
        assert!(check(&elsewhere, "transition-room").is_empty() && check(&elsewhere, "lane-drop").is_empty());
    }

    #[test]
    fn undoing_the_split_removes_the_boundary_check() {
        let mut e = editor(&[]);
        e.split_zone(Some(20_000)).unwrap();
        assert!(e.select_zone(1));
        let end = e.snapshot().segments.len();
        e.add(kind_index("travel").unwrap(), end);
        assert_eq!(check(&e, "transition-room").len(), 1);
        assert!(e.undo()); // the lane
        assert!(check(&e, "transition-room").is_empty(), "same sections, no transition");
        assert!(e.undo()); // the split
        assert!(check(&e, "transition-room").is_empty());
    }
}
