//! Zones: a street divided along its length into ordered stretches, each with the street's base section or
//! a section of its own. A street with no zones is the one-section street the city has always kept.

use serde::{Deserialize, Serialize};

use super::transition::{Host, Transition, transition};
use super::{Arg, Editor, Said, Segment, Street, flip, named_said, snap};

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
/// left, and no curbside use to measure, leaves no end and no length.
pub(super) fn settle_zones(segments: &mut Vec<Segment>, zones: &mut Vec<Zone>, from_end: &mut ZoneEnd, length_mm: &mut Option<i32>, keep_anchor: bool) {
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
    if zones.is_empty() && !keep_anchor {
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
        settle_zones(&mut self.segments, &mut self.zones, &mut self.from_end, &mut self.length_mm, !self.curb.is_empty());
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
        s.curb = Vec::new();
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
        if !self.zones.is_empty() || !self.curb.is_empty() {
            self.from_end = self.from_end.other();
        }
    }
}

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
    /// There is no zone on that side to merge with.
    NoNeighbour,
    /// The first zone has no start boundary.
    NoBoundary,
    /// A drag is under way: nothing but the drag may change the zones.
    Dragging,
}

impl ZoneRefusal {
    pub fn said(self) -> Said {
        let key = match self {
            ZoneRefusal::NoAnchor => "zone-refused-no-anchor",
            ZoneRefusal::Elsewhere => "zone-refused-elsewhere",
            ZoneRefusal::TooShort => "zone-refused-too-short",
            ZoneRefusal::OutsideStreet => "zone-refused-outside",
            ZoneRefusal::NoNeighbour => "zone-refused-no-neighbour",
            ZoneRefusal::NoBoundary => "zone-refused-no-boundary",
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
    /// Why Split is not available, when it is not.
    pub split: Option<Said>,
    /// The street has more than one zone: only then are there zones to merge, name or give a section of their own.
    pub several: bool,
    /// The selected zone has a zone before it / after it to merge with.
    pub can_merge_prev: bool,
    pub can_merge_next: bool,
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
    pub label: Said,
    /// The same as it reads in the middle of a sentence.
    pub label_mid: Said,
    /// The zone has a name a person gave it (else `label` is "Zone n").
    pub named: bool,
    /// The zone has a section of its own.
    pub own: bool,
    /// The zone starts beyond the loaded end of the street.
    pub cut_off: bool,
    /// Every check passes in this zone.
    pub ok: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Elsewhere {
    pub drawn_mm: i32,
    pub loaded_mm: i32,
}

/// Which zone a merge keeps the section and name of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    Selected,
    Neighbour,
}

/// What a zone is called: its name, else "Zone n". `n` is its number as shown, counting from 1.
pub fn zone_label(zone: &Zone, n: usize) -> Said {
    match &zone.name {
        Some(name) => named_said(name),
        None => Said::new("zone-unnamed").with("n", Arg::Num(n as i64)),
    }
}

/// The same where it stands in the middle of a sentence ("Zone 2" is "zone 2" in French); a name stays as it was typed.
pub fn zone_label_mid(zone: &Zone, n: usize) -> Said {
    match &zone.name {
        Some(name) => named_said(name),
        None => Said::new("zone-unnamed-mid").with("n", Arg::Num(n as i64)),
    }
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
    fn structure_frame(&self, allow_drag: bool) -> Result<ZoneFrame, ZoneRefusal> {
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
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        let from_end = if content.zones.is_empty() { self.frame.map_or(ZoneEnd::First, |f| f.from_end) } else { content.from_end };
        let failing = self.failing_transitions();
        let one = |z: usize| {
            let (from_mm, to_mm) = content.span(z, length_mm);
            let ok = self.section_passes(content.section(z)) && !failing.iter().any(|(_, host, _)| *host == z);
            match content.zones.get(z) {
                Some(g) => ZoneView {
                    index: z,
                    from_mm,
                    to_mm,
                    label: zone_label(g, z + 1),
                    label_mid: zone_label_mid(g, z + 1),
                    named: g.name.is_some(),
                    own: g.segments.is_some(),
                    cut_off: self.is_cut_off(z),
                    ok,
                },
                None => ZoneView {
                    index: z,
                    from_mm,
                    to_mm,
                    label: Said::new("zone-unnamed").with("n", Arg::Num(1)),
                    label_mid: Said::new("zone-unnamed-mid").with("n", Arg::Num(1)),
                    named: false,
                    own: false,
                    cut_off: false,
                    ok,
                },
            }
        };
        ZonesView {
            zones: (0..content.zone_count()).map(one).collect(),
            selected: self.zone,
            length_mm,
            from_end,
            split: self.structure_frame(true).err().map(ZoneRefusal::said),
            several: content.zones.len() > 1,
            can_merge_prev: self.merge_neighbour(false).is_some(),
            can_merge_next: self.merge_neighbour(true).is_some(),
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
        let speed = content.speed.kmh();
        // The pieces' own kinds, never a time-of-day variant's: a transition does not change with the clock.
        (0..content.zones.len() - 1).filter_map(|b| transition(content.section(b), content.section(b + 1), speed, content.from_end).map(|t| (b, t))).collect()
    }

    /// The transitions that fail a check: `(boundary, host zone, which)`.
    pub(super) fn failing_transitions(&self) -> Vec<(usize, usize, Failing)> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
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

    /// The label of zone `z` of the current content, numbered from 1, as it reads in the middle of a sentence.
    fn label_of(&self, z: usize) -> Said {
        match self.states[self.cursor].content.zones.get(z) {
            Some(zone) => zone_label_mid(zone, z + 1),
            None => Said::new("zone-unnamed-mid").with("n", Arg::Num(1)),
        }
    }

    /// Splits the selected zone at `at_mm` (default its middle, snapped to `SNAP_MM`). Both halves keep the section;
    /// the first keeps the name; the new zone is selected.
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
        self.edit_content(Said::new("rev-zone-split").with("at", Arg::Length(at)), |c| {
            if c.zones.is_empty() {
                c.zones.push(Zone { from_mm: 0, segments: None, name: None });
            }
            let section = c.zones[z].segments.clone();
            c.zones.insert(z + 1, Zone { from_mm: at, segments: section, name: None });
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        });
        self.zone = z + 1;
        self.selected = None;
        Ok(())
    }

    /// The neighbour a merge would join the selected zone to, if there is one.
    fn merge_neighbour(&self, next: bool) -> Option<usize> {
        let count = self.states[self.cursor].content.zones.len();
        let z = self.zone;
        if next { (z + 1 < count).then_some(z + 1) } else { (count > 0 && z > 0).then(|| z - 1) }
    }

    /// Whether merging with the next (or previous) zone must ask which section to keep: the two differ.
    pub fn merge_needs_choice(&self, next: bool) -> bool {
        let content = &self.states[self.cursor].content;
        self.merge_neighbour(next).is_some_and(|n| !same_pieces(content.section(self.zone), content.section(n)))
    }

    /// Why the zones cannot be rearranged now (no anchor, another road, a drag under way), if they cannot.
    pub fn structure_blocked(&self) -> Option<ZoneRefusal> {
        self.structure_frame(false).err()
    }

    /// Merges the selected zone with the next (or previous) one. The merged zone starts where the earlier did, takes
    /// the section and name of the zone `keep` names, and is selected.
    pub fn merge_zone(&mut self, next: bool, keep: Keep) -> Result<(), ZoneRefusal> {
        let frame = self.structure_frame(false)?;
        let n = self.merge_neighbour(next).ok_or(ZoneRefusal::NoNeighbour)?;
        let (lo, hi) = (self.zone.min(n), self.zone.max(n));
        let kept = if keep == Keep::Selected { self.zone } else { n };
        // The choice is said by the number the kept zone had before the merge, which is the one the person pressed.
        let label = Said::new("rev-zone-merge").with("kept", Arg::Said(Box::new(zone_label_mid(&self.states[self.cursor].content.zones[kept], kept + 1))));
        self.edit_content(label, |c| {
            let survivor = c.zones[kept].clone();
            c.zones[lo] = Zone { from_mm: c.zones[lo].from_mm, ..survivor };
            c.zones.remove(hi);
            c.from_end = frame.from_end;
            c.length_mm = Some(frame.length_mm);
            true
        });
        self.zone = lo;
        self.selected = None;
        Ok(())
    }

    /// Moves the start of the selected zone to `to_mm`, keeping both neighbours at least `MIN_ZONE_MM` long. Inside a
    /// gesture it is shown as it goes and recorded as one revision at the end.
    pub fn move_boundary(&mut self, to_mm: i32) -> Result<(), ZoneRefusal> {
        let frame = self.structure_frame(true)?;
        let z = self.zone;
        let content = &self.states[self.cursor].content;
        if z == 0 || z >= content.zones.len() {
            return Err(ZoneRefusal::NoBoundary);
        }
        let to_mm = snap(to_mm);
        if to_mm < 0 || to_mm > frame.length_mm {
            return Err(ZoneRefusal::OutsideStreet);
        }
        let (before, _) = content.span(z - 1, frame.length_mm);
        let (_, after) = content.span(z, frame.length_mm);
        if to_mm - before < MIN_ZONE_MM || after - to_mm < MIN_ZONE_MM {
            return Err(ZoneRefusal::TooShort);
        }
        if content.zones[z].from_mm == to_mm {
            return Ok(());
        }
        let label = Said::new("rev-zone-move").with("zone", Arg::Said(Box::new(self.label_of(z)))).with("to", Arg::Length(to_mm));
        let set = |c: &mut super::Content| {
            c.zones[z].from_mm = to_mm;
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
        Ok(())
    }

    /// Moves the selected zone's start boundary by `delta_mm`. Does nothing where `move_boundary` refuses.
    pub fn nudge_boundary(&mut self, delta_mm: i32) {
        let Some(from) = self.states[self.cursor].content.zones.get(self.zone).map(|z| z.from_mm) else {
            return;
        };
        let _ = self.move_boundary(from + delta_mm);
    }

    /// Names the selected zone; blank clears the name. False when the street has no zones or nothing changed.
    pub fn name_zone(&mut self, name: Option<&str>) -> bool {
        if self.gesture.is_some() {
            return false;
        }
        let name = name.map(str::trim).filter(|n| !n.is_empty()).map(str::to_string);
        let z = self.zone;
        if z >= self.states[self.cursor].content.zones.len() {
            return false;
        }
        let label = Said::new(if name.is_some() { "rev-zone-name" } else { "rev-zone-unname" })
            .with("zone", Arg::Said(Box::new(zone_label_mid(&Zone { from_mm: 0, segments: None, name: name.clone() }, z + 1))));
        self.edit_content(label, |c| {
            c.zones[z].name = name;
            true
        })
    }

    /// Puts the selected zone back on the street's base section.
    pub fn reset_zone(&mut self) -> bool {
        if self.gesture.is_some() {
            return false;
        }
        let z = self.zone;
        if self.states[self.cursor].content.zones.get(z).is_none_or(|g| g.segments.is_none()) {
            return false;
        }
        let label = Said::new("rev-zone-reset").with("zone", Arg::Said(Box::new(self.label_of(z))));
        self.edit_content(label, |c| {
            c.zones[z].segments = None;
            true
        })
    }

    /// Makes the selected zone's section the street's base. Zones with none of their own follow it; zones with one
    /// keep it.
    pub fn use_for_street(&mut self) -> bool {
        if self.gesture.is_some() {
            return false;
        }
        let z = self.zone;
        if z >= self.states[self.cursor].content.zones.len() {
            return false;
        }
        let label = Said::new("rev-zone-use").with("zone", Arg::Said(Box::new(self.label_of(z))));
        self.edit_content(label, |c| {
            if let Some(own) = c.zones[z].segments.take() {
                c.segments = own;
            }
            true
        })
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
        assert_eq!(e.view().revisions.last().unwrap().label, Said::new("rev-zone-split").with("at", Arg::Length(30_000)));
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
        let mut e = editor(&[0, 20_000]);
        assert!(e.name_zone(Some("Terrace")));
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
        assert_eq!(ZoneRefusal::NoAnchor.said().key, "zone-refused-no-anchor");
        assert_eq!(e.view().revisions.len(), 0);
    }

    #[test]
    fn every_refusal_has_its_own_message_key() {
        use ZoneRefusal::*;
        let keys: Vec<_> = [NoAnchor, Elsewhere, TooShort, OutsideStreet, NoNeighbour, NoBoundary, Dragging].map(|r| r.said().key).to_vec();
        let expected = ["no-anchor", "elsewhere", "too-short", "outside", "no-neighbour", "no-boundary", "dragging"].map(|k| format!("zone-refused-{k}"));
        assert_eq!(keys, expected);
        assert_eq!(TooShort.said(), Said::new("zone-refused-too-short").with("min", Arg::Length(MIN_ZONE_MM)));
    }

    #[test]
    fn a_street_whose_zones_were_drawn_at_another_length_or_end_refuses_every_structure_edit() {
        let today = sample();
        let now = zoned(today.clone(), &[0, 20_000, 40_000], LEN);
        let at = |len: i32, end: ZoneEnd| Editor::from_street(&today, &now, 0).with_frame(Some(ZoneFrame { length_mm: len, from_end: end }));

        let mut e = at(58_500, ZoneEnd::First);
        assert_eq!(e.split_zone(Some(10_000)), Err(ZoneRefusal::Elsewhere));
        assert_eq!(e.merge_zone(true, Keep::Selected), Err(ZoneRefusal::Elsewhere));
        assert!(e.select_zone(1));
        assert_eq!(e.move_boundary(25_000), Err(ZoneRefusal::Elsewhere));
        make_own(&mut e);
        assert!(e.snapshot().zones[1].segments.is_some(), "a piece edit in a zone still works");
        assert!(e.name_zone(Some("Own")));

        let mut near = at(59_500, ZoneEnd::First);
        assert_eq!(near.split_zone(Some(10_000)), Ok(()));
        assert_eq!(near.snapshot().length_mm, Some(59_500), "a structure edit rewrites the length to the loaded one");

        let mut flipped = at(LEN, ZoneEnd::Last);
        assert_eq!(flipped.split_zone(Some(10_000)), Err(ZoneRefusal::Elsewhere));
    }

    #[test]
    fn merging_zones_with_the_same_section_needs_no_choice_and_one_with_another_asks_and_keeps_the_chosen_one() {
        let plain = editor(&[0, 20_000, 40_000]);
        assert!(!plain.merge_needs_choice(true), "both on the base");
        // zone 1 has a section of its own and a name
        let build = || {
            let mut e = editor(&[0, 20_000, 40_000]);
            assert!(e.select_zone(1));
            make_own(&mut e);
            assert!(e.name_zone(Some("Mine")));
            e
        };
        let e = build();
        assert!(e.merge_needs_choice(true));
        assert!(e.merge_needs_choice(false));
        let own = e.snapshot().zones[1].segments.clone();
        let base = e.snapshot().segments;

        let mut keep_selected = build();
        let before = revisions(&keep_selected);
        keep_selected.merge_zone(true, Keep::Selected).unwrap();
        let s = keep_selected.snapshot();
        assert_eq!(starts(&keep_selected), [0, 20_000]);
        assert_eq!(s.zones[1].segments, own);
        assert_eq!(s.zones[1].name.as_deref(), Some("Mine"));
        assert_eq!(keep_selected.zone(), 1);
        assert_eq!(revisions(&keep_selected), before + 1);
        assert_eq!(keep_selected.view().revisions.last().unwrap().label.key, "rev-zone-merge");

        let mut keep_neighbour = build();
        keep_neighbour.merge_zone(true, Keep::Neighbour).unwrap();
        let s = keep_neighbour.snapshot();
        assert_eq!(starts(&keep_neighbour), [0, 20_000]);
        assert_eq!(s.zones[1].segments, None);
        assert_eq!(s.zones[1].name, None);
        assert_eq!(s.segments, base);
        assert_eq!(keep_neighbour.zone(), 1);

        // the earlier start wins whichever side is merged
        let mut back = build();
        back.merge_zone(false, Keep::Selected).unwrap();
        assert_eq!(starts(&back), [0, 40_000]);
        assert_eq!(back.zone(), 0);
        assert_eq!(back.snapshot().zones[0].segments, own);
    }

    #[test]
    fn merging_the_last_pair_down_to_one_zone_leaves_no_zoning_and_the_kept_section_as_the_base() {
        let mut e = editor(&[0, 30_000]);
        assert!(e.select_zone(1));
        make_own(&mut e);
        let own = e.snapshot().zones[1].segments.clone().unwrap();
        let next_uid = e.snapshot().next_uid;
        e.merge_zone(false, Keep::Selected).unwrap();
        let s = e.snapshot();
        assert!(s.zones.is_empty());
        assert_eq!(s.segments, own);
        assert_eq!(s.length_mm, None);
        assert_eq!(s.next_uid, next_uid);
        assert_eq!(e.zone(), 0);
    }

    #[test]
    fn merge_is_refused_at_the_ends() {
        let mut e = editor(&[0, 20_000]);
        assert_eq!(e.merge_zone(false, Keep::Selected), Err(ZoneRefusal::NoNeighbour));
        assert!(!e.merge_needs_choice(false));
        assert!(e.select_zone(1));
        assert_eq!(e.merge_zone(true, Keep::Selected), Err(ZoneRefusal::NoNeighbour));
        let mut one = editor(&[]);
        assert_eq!(one.merge_zone(true, Keep::Selected), Err(ZoneRefusal::NoNeighbour));
    }

    #[test]
    fn a_boundary_moves_by_typed_value_and_by_nudges_and_stops_5_m_from_its_neighbours() {
        let mut e = editor(&[0, 20_000, 40_000]);
        assert_eq!(e.move_boundary(10_000), Err(ZoneRefusal::NoBoundary), "the first zone has no start boundary");
        assert!(e.select_zone(1));
        let before = revisions(&e);
        e.nudge_boundary(1_000);
        assert_eq!(starts(&e), [0, 21_000, 40_000]);
        e.nudge_boundary(-10_000);
        assert_eq!(starts(&e), [0, 11_000, 40_000]);
        assert_eq!(revisions(&e), before + 2);
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-zone-move");
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
    fn structure_and_name_edits_are_refused_while_a_drag_is_under_way_and_leave_the_history_intact() {
        let mut e = editor(&[0, 20_000, 40_000]);
        assert!(e.select_zone(1));
        make_own(&mut e);
        let before = (revisions(&e), e.snapshot());
        e.begin_gesture();
        assert_eq!(e.move_boundary(25_000), Ok(()));
        assert_eq!(e.split_zone(Some(30_000)), Err(ZoneRefusal::Dragging));
        assert_eq!(e.merge_zone(true, Keep::Selected), Err(ZoneRefusal::Dragging));
        assert!(!e.name_zone(Some("x")));
        assert!(!e.reset_zone());
        assert!(!e.use_for_street());
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
    fn naming_resetting_and_using_a_zone_for_the_street_are_each_one_undoable_step() {
        let mut e = editor(&[0, 20_000, 40_000]);
        assert!(e.select_zone(1));
        let n = revisions(&e);
        assert!(e.name_zone(Some("  Market  ")));
        assert_eq!(e.snapshot().zones[1].name.as_deref(), Some("Market"));
        assert_eq!(revisions(&e), n + 1);
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-zone-name");
        assert!(!e.name_zone(Some("Market")), "no change, no step");
        assert!(e.name_zone(Some("   ")));
        assert_eq!(e.snapshot().zones[1].name, None);
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-zone-unname");
        assert!(e.undo());
        assert_eq!(e.snapshot().zones[1].name.as_deref(), Some("Market"));

        make_own(&mut e);
        let own = e.snapshot().zones[1].segments.clone().unwrap();
        let base = e.snapshot().segments;
        let n = revisions(&e);
        assert!(e.reset_zone());
        assert_eq!(e.snapshot().zones[1].segments, None);
        assert_eq!(revisions(&e), n + 1);
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-zone-reset");
        assert!(!e.reset_zone(), "already on the base");
        assert!(e.undo());
        assert_eq!(e.snapshot().zones[1].segments.as_ref(), Some(&own));

        // zone 2 has its own section too; zone 0 follows the base
        assert!(e.select_zone(2));
        let mut other_own = base.clone();
        other_own[1].width_mm += 100;
        let uid = other_own[1].uid;
        assert!(e.set_width(uid, other_own[1].width_mm));
        let kept = e.snapshot().zones[2].segments.clone().unwrap();
        assert!(e.select_zone(1));
        let n = revisions(&e);
        assert!(e.use_for_street());
        let s = e.snapshot();
        assert_eq!(s.segments, own, "the zone's section is the base");
        assert_eq!(s.zones[1].segments, None);
        assert_eq!(s.zones[0].segments, None, "a zone with no section follows the new base");
        assert_eq!(s.zones[2].segments.as_ref(), Some(&kept), "a zone with one keeps it");
        assert_eq!(revisions(&e), n + 1);
        assert_eq!(e.view().revisions.last().unwrap().label.key, "rev-zone-use");
        assert!(!e.use_for_street(), "it is the base already");
        assert!(e.undo());
        assert_eq!(e.snapshot().segments, base);

        // a street of one zone has none of these
        let mut one = editor(&[]);
        assert!(!one.name_zone(Some("x")) && !one.reset_zone() && !one.use_for_street());
    }

    #[test]
    fn an_unnamed_zone_is_called_by_its_number_and_a_named_one_by_its_name() {
        let named = Zone { from_mm: 0, segments: None, name: Some("Terrace".into()) };
        let unnamed = Zone { from_mm: 0, segments: None, name: None };
        assert_eq!(zone_label(&named, 1), Said::new("city-name").with("name", Arg::Text("Terrace".into())));
        assert_eq!(zone_label(&unnamed, 3), Said::new("zone-unnamed").with("n", Arg::Num(3)));
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
        e.merge_zone(false, Keep::Neighbour).unwrap();
        check(&e, "merge");
        assert!(e.use_for_street());
        check(&e, "use");
        let _ = e.reset_zone();
        check(&e, "reset");
        while e.undo() {
            check(&e, "undo");
        }
        while e.redo() {
            check(&e, "redo");
        }
    }

    #[test]
    fn a_merge_names_the_zone_it_keeps_by_the_number_it_had_before_the_merge() {
        let mut e = editor(&[0, 20_000]);
        assert!(e.select_zone(1));
        make_own(&mut e);
        assert!(e.select_zone(0));
        e.merge_zone(true, Keep::Neighbour).unwrap();
        let kept = Said::new("zone-unnamed-mid").with("n", Arg::Num(2));
        assert_eq!(e.view().revisions.last().unwrap().label, Said::new("rev-zone-merge").with("kept", Arg::Said(Box::new(kept))));

        let mut e = editor(&[0, 20_000]);
        e.merge_zone(true, Keep::Selected).unwrap();
        let kept = Said::new("zone-unnamed-mid").with("n", Arg::Num(1));
        assert_eq!(e.view().revisions.last().unwrap().label, Said::new("rev-zone-merge").with("kept", Arg::Said(Box::new(kept))));
    }

    #[test]
    fn the_view_says_which_neighbours_the_selected_zone_has_and_which_zones_are_named() {
        let mut e = editor(&[]);
        let z = e.view().zones;
        assert!(!z.several && !z.can_merge_prev && !z.can_merge_next && !z.zones[0].named);
        e.split_zone(None).unwrap();
        e.split_zone(None).unwrap();
        assert!(e.select_zone(0));
        let z = e.view().zones;
        assert!(z.several && !z.can_merge_prev && z.can_merge_next);
        assert!(e.select_zone(1));
        assert!(e.name_zone(Some("Market")));
        let z = e.view().zones;
        assert!(z.can_merge_prev && z.can_merge_next);
        assert_eq!(z.zones.iter().map(|g| g.named).collect::<Vec<_>>(), [false, true, false]);
        assert!(e.select_zone(2));
        let z = e.view().zones;
        assert!(z.can_merge_prev && !z.can_merge_next);
    }

    #[test]
    fn a_change_of_side_mirrors_every_zones_own_section_as_well_as_the_base_in_one_revision() {
        let dirs = |segs: &[Segment]| segs.iter().map(|s| s.direction).collect::<Vec<_>>();
        let mut e = editor(&[0, 20_000, 40_000]);
        assert!(e.select_zone(1));
        make_own(&mut e);
        assert!(e.select_zone(2));
        make_own(&mut e);
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
        crate::street::text::say_now(&crate::i18n_for(locale), crate::shared::units::Units::Metres, said)
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
        let between = Said::new("check-boundary")
            .with("a", Arg::Said(Box::new(Said::new("zone-unnamed-mid").with("n", Arg::Num(1)))))
            .with("b", Arg::Said(Box::new(Said::new("zone-unnamed-mid").with("n", Arg::Num(2)))));
        assert_eq!(room[0].zone, Some(between));
        let detail = say_in(crate::shared::i18n::Locale::En, &room[0].detail);
        assert!(detail.contains("49.5"), "{detail}");
        assert!(!e.view().zones.zones[0].ok, "the host zone");
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
        assert!(e.view().zones.zones[0].ok);
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
