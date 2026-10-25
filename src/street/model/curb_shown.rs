//! The curbside uses the 2.5D shows: the instances that overlap the stretch it draws, mapped from the positions they
//! are kept at (along the whole street from the anchored end) to millimetres along the stretch.
//!
//! The drawing's direction is the section's: its near end is the street's first end and it recedes toward the last
//! end, whatever end the positions are measured from (`Street::from_end`). The stretch is the selected zone's first
//! `SHOWN_MM` from the zone's start (anchored), and across a near boundary (`Joint`) the half of the stretch the zone
//! has, the 20 m beside the boundary. Anchored positions increase away from the anchored end, so with the street
//! anchored at its last end the stretch is mirrored.

use serde::Serialize;

use super::curb::curbside;
use super::joint::SHOWN_MM;
use super::stop::{Stand, StopKind, bus_lane, stand};
use super::{Editor, REGIONS, Use, ZoneEnd};

/// One transit stop as the 2.5D shows it, on the selected zone's section.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShownStop {
    /// The pieces the platform raises to sidewalk height, indices into `View.segments`: a bulb's parking piece, an
    /// island's pieces. Empty for a curb stop: its sidewalk is the platform.
    pub raised: Vec<usize>,
    /// The piece the stop's sign and shelter stand on, an index into `View.segments`: the sidewalk beside the bus's
    /// lane (curb stop), the platform (bulb, island). None when there is nothing to stand on.
    pub stand: Option<usize>,
    /// Where it lies along the stretch, in millimetres from the stretch's near end: only the part inside the stretch
    /// and the zone.
    pub along: (f64, f64),
    /// The whole stop in the same coordinates; it may lie partly outside the stretch. The shelter and the sign are
    /// placed from it, so they stay where they are as the stretch moves.
    pub full: (f64, f64),
    pub kind: StopKind,
    pub shelter: bool,
}

/// One use of the curbside as the 2.5D shows it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ShownInstance {
    /// The curbside piece, an index into `View.segments`.
    pub piece: usize,
    /// Where it lies along the stretch, in millimetres from the stretch's near end: only the part inside the stretch
    /// and the zone.
    pub along: (f64, f64),
    /// The whole use in the same coordinates; it may lie partly outside the stretch. Trees and racks are placed from
    /// it, so they stay where they are as the stretch moves.
    pub full: (f64, f64),
    #[serde(rename = "use")]
    pub use_: Use,
}

/// The stretch the 2.5D shows of the selected zone: where it lies along the street, and how a position along the
/// street (from the anchored end) maps into it.
struct Stretch {
    /// The reach of the selected zone's run along the stretch, in millimetres from the near end.
    near: f64,
    far: f64,
    /// The anchored position of the run's low end, and the part of the zone it covers.
    start: i32,
    lo: i32,
    hi: i32,
    from_end: ZoneEnd,
}

impl Stretch {
    /// The anchored position `pos` in stretch coordinates.
    fn place(&self, pos: i32) -> f64 {
        match self.from_end {
            ZoneEnd::First => self.near + f64::from(pos - self.start),
            ZoneEnd::Last => self.far - f64::from(pos - self.start),
        }
    }

    /// Where the span `from..to` lies along the stretch: the part inside the stretch and the zone, and the whole.
    /// None when no part of it is.
    fn map(&self, from: i32, to: i32) -> Option<((f64, f64), (f64, f64))> {
        let (s, e) = (from.max(self.lo), to.min(self.hi));
        if s >= e {
            return None;
        }
        let (a, b) = (self.place(s), self.place(e));
        let (c, d) = (self.place(from), self.place(to));
        Some(((a.min(b), a.max(b)), (c.min(d), c.max(d))))
    }
}

impl Editor {
    /// The stretch shown for the selected zone, and the length of the street it lies in. None without a length.
    fn stretch(&self) -> Option<Stretch> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        if length_mm <= 0 {
            return None;
        }
        let (from, to) = content.span(self.zone.min(content.zone_count().saturating_sub(1)), length_mm);
        let half = f64::from(SHOWN_MM) / 2.0;
        let (near, far, start) = match self.joint() {
            None => (0.0, f64::from(SHOWN_MM), from),
            Some(j) => {
                let (near, far) = if j.far { (0.0, half) } else { (half, f64::from(SHOWN_MM)) };
                (near, far, if j.zone > self.zone { to - SHOWN_MM / 2 } else { from })
            }
        };
        let (lo, hi) = (start.max(from), (start + (far - near) as i32).min(to));
        Some(Stretch { near, far, start, lo, hi, from_end: content.from_end })
    }

    /// The transit stops that overlap the stretch the 2.5D shows, on the selected zone's section. A stop the section
    /// has no bus lane or platform for is not drawn: there is nothing to draw it on (its check fails).
    pub(super) fn stops_shown(&self) -> Vec<ShownStop> {
        let content = &self.states[self.cursor].content;
        if content.stops.is_empty() {
            return Vec::new();
        }
        let Some(stretch) = self.stretch() else { return Vec::new() };
        let (segs, driving) = (self.current(), REGIONS[self.region].drive_side);
        content
            .stops
            .iter()
            .filter_map(|s| {
                let (along, full) = stretch.map(s.from_mm, s.to_mm())?;
                let lane = bus_lane(segs, s.side, driving)?;
                let Stand { raised, stand } = stand(segs, lane, s.side, s.kind)?;
                Some(ShownStop { raised, stand, along, full, kind: s.kind, shelter: s.shelter })
            })
            .collect()
    }

    /// The instances that overlap the stretch the 2.5D shows, on the selected zone's curbside pieces. None across a
    /// boundary: the neighbour's half shows no uses.
    pub(super) fn curb_shown(&self) -> Vec<ShownInstance> {
        let content = &self.states[self.cursor].content;
        if content.curb.is_empty() {
            return Vec::new();
        }
        let Some(stretch) = self.stretch() else { return Vec::new() };
        let segs = self.current();
        content
            .curb
            .iter()
            .filter_map(|i| {
                let (along, full) = stretch.map(i.from_mm, i.from_mm + i.length_mm)?;
                let piece = curbside(segs, i.side)?;
                Some(ShownInstance { piece, along, full, use_: i.use_ })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::street::model::{Street, ZoneFrame};

    const LEN: i32 = 60_000;

    fn editor(from_end: ZoneEnd) -> Editor {
        let s = Street::sample(0, Side::Right);
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end }))
    }

    fn shown(e: &Editor) -> Vec<(usize, (f64, f64), Use)> {
        e.view().curb_shown.iter().map(|s| (s.piece, s.along, s.use_)).collect()
    }

    #[test]
    fn a_street_with_no_instances_shows_none() {
        assert!(Editor::new(0).view().curb_shown.is_empty());
    }

    #[test]
    fn the_stretch_is_the_first_40_m_from_the_anchored_end_and_clips_what_crosses_its_edge() {
        let mut e = editor(ZoneEnd::First);
        e.add_curb(Side::Right, 12_000, 6_000, Use::Parking).unwrap();
        e.add_curb(Side::Right, 38_000, 6_000, Use::Loading).unwrap();
        e.add_curb(Side::Right, 50_000, 6_000, Use::Parking).unwrap();
        e.add_curb(Side::Left, 2_000, 2_000, Use::Tree).unwrap();
        // Right is the last piece of the section, Left the first, whichever end the positions are measured from.
        assert_eq!(shown(&e), vec![(1, (2_000.0, 4_000.0), Use::Tree), (4, (12_000.0, 18_000.0), Use::Parking), (4, (38_000.0, 40_000.0), Use::Loading)]);
    }

    #[test]
    fn anchored_at_the_last_end_the_stretch_is_mirrored_and_the_sides_stay_the_sections() {
        let mut e = editor(ZoneEnd::Last);
        e.add_curb(Side::Right, 12_000, 6_000, Use::Parking).unwrap();
        e.add_curb(Side::Right, 38_000, 6_000, Use::Loading).unwrap();
        // Position 12 m from the anchored end is 28 m to 22 m from the stretch's own near end.
        assert_eq!(shown(&e), vec![(4, (22_000.0, 28_000.0), Use::Parking), (4, (0.0, 2_000.0), Use::Loading)]);
    }

    fn zoned_editor(from_end: ZoneEnd) -> Editor {
        let mut e = editor(from_end);
        e.add_curb(Side::Right, 12_000, 6_000, Use::Parking).unwrap();
        e.add_curb(Side::Right, 24_000, 6_000, Use::Tree).unwrap();
        e.split_zone(Some(20_000)).unwrap();
        e
    }

    #[test]
    fn across_a_boundary_the_zone_shows_the_half_beside_it_and_only_its_own_span() {
        let mut e = zoned_editor(ZoneEnd::First);
        assert!(e.select_zone(0));
        assert!(e.view().zones.joint.is_some());
        assert_eq!(shown(&e), vec![(4, (12_000.0, 18_000.0), Use::Parking)], "zone 1 is the near half, the use beyond the boundary is not shown");
        assert!(e.select_zone(1));
        // The zone after the boundary is the far half, 20 m to 40 m, its start at the boundary.
        assert_eq!(shown(&e), vec![(4, (24_000.0, 30_000.0), Use::Tree)]);
    }

    #[test]
    fn across_a_boundary_from_the_last_end_the_halves_swap() {
        let mut e = zoned_editor(ZoneEnd::Last);
        assert!(e.select_zone(0));
        // Zone 1 is anchored at the last end, so it is the far half and its start is the stretch's 40 m.
        assert_eq!(shown(&e), vec![(4, (22_000.0, 28_000.0), Use::Parking)]);
        assert!(e.select_zone(1));
        assert_eq!(shown(&e), vec![(4, (10_000.0, 16_000.0), Use::Tree)]);
    }

    // ---- the stops, mapped by the same stretch ----

    use crate::shared::catalogue::kind_index;
    use crate::street::model::{Piece, StopKind};

    /// Right-hand traffic, the bus in the last lane; a parking lane between it and the right sidewalk.
    fn bus_street(extra: &[(&str, i32, Option<usize>)]) -> Street {
        let mut ids = vec![("sidewalk", 3000, None), ("travel", 3300, Some(1)), ("bus", 3300, Some(0))];
        ids.extend_from_slice(extra);
        ids.push(("sidewalk", 3000, None));
        let pieces: Vec<Piece> =
            ids.iter().map(|&(id, width_mm, direction)| Piece { kind: kind_index(id).unwrap(), width_mm, direction, variants: Vec::new() }).collect();
        Street::imported(crate::shared::catalogue::StreetClass::Local, Side::Right, &pieces)
    }

    fn stop_editor(s: &Street, from_end: ZoneEnd) -> Editor {
        let region = crate::shared::catalogue::REGIONS.iter().position(|r| r.drive_side == Side::Right).unwrap();
        Editor::from_street(s, s, region).with_frame(Some(ZoneFrame { length_mm: LEN, from_end }))
    }

    #[test]
    fn a_street_with_no_stops_shows_none() {
        assert!(Editor::new(0).view().stops_shown.is_empty());
        assert!(stop_editor(&bus_street(&[]), ZoneEnd::First).view().stops_shown.is_empty());
    }

    #[test]
    fn a_stop_is_mapped_like_a_curb_use_from_either_end_with_the_platform_found() {
        let street = bus_street(&[("parking", 2400, None)]);
        let mut e = stop_editor(&street, ZoneEnd::First);
        e.add_stop(Side::Right, 36_000, 12_000, StopKind::Bulb).unwrap();
        let s = &e.view().stops_shown;
        assert_eq!(s.len(), 1);
        // The part inside the 40 m stretch and the whole stop, which runs past its end; the parking lane is raised.
        assert_eq!(
            (s[0].along, s[0].full, s[0].raised.clone(), s[0].stand, s[0].kind),
            ((36_000.0, 40_000.0), (36_000.0, 48_000.0), vec![3], Some(3), StopKind::Bulb)
        );
        // From the last end, the stretch is mirrored: 10 m to 22 m is 30 m to 18 m from the near end.
        let mut e = stop_editor(&street, ZoneEnd::Last);
        e.add_stop(Side::Right, 10_000, 12_000, StopKind::Bulb).unwrap();
        let s = &e.view().stops_shown;
        assert_eq!((s[0].along, s[0].full), ((18_000.0, 30_000.0), (18_000.0, 30_000.0)));
        // A curb stop raises nothing and stands on the sidewalk beside the lane; the part inside is clipped.
        let street = bus_street(&[]);
        let mut e = stop_editor(&street, ZoneEnd::Last);
        e.add_stop(Side::Right, 34_000, 12_000, StopKind::Curb).unwrap();
        let s = &e.view().stops_shown;
        assert_eq!((s[0].along, s[0].full, s[0].raised.clone(), s[0].stand), ((0.0, 6_000.0), (-6_000.0, 6_000.0), Vec::<usize>::new(), Some(3)));
        // One wholly past the stretch is not shown.
        let mut e = stop_editor(&street, ZoneEnd::First);
        e.add_stop(Side::Right, 41_000, 12_000, StopKind::Curb).unwrap();
        assert!(e.view().stops_shown.is_empty());
    }
}
