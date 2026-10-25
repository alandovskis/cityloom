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
use super::{Editor, Use, ZoneEnd};
/// One use of the curbside as the 2.5D shows it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ShownInstance {
    /// The curbside piece, an index into `View.segments`.
    pub piece: usize,
    /// Where it lies along the stretch, in millimetres from the stretch's near end: only the part inside the stretch
    /// and the zone.
    pub along: (f64, f64),
    #[serde(rename = "use")]
    pub use_: Use,
}

impl Editor {
    /// The instances that overlap the stretch the 2.5D shows, on the selected zone's curbside pieces. None across a
    /// boundary: the neighbour's half shows no uses.
    pub(super) fn curb_shown(&self) -> Vec<ShownInstance> {
        let content = &self.states[self.cursor].content;
        let length_mm = content.length_mm.or(self.frame.map(|f| f.length_mm)).unwrap_or(0);
        if content.curb.is_empty() || length_mm <= 0 {
            return Vec::new();
        }
        let (from, to) = content.span(self.zone.min(content.zone_count().saturating_sub(1)), length_mm);
        // The reach of the selected zone's run along the stretch, and the anchored position of its low end.
        let half = f64::from(SHOWN_MM) / 2.0;
        let (near, far, start) = match self.joint() {
            None => (0.0, f64::from(SHOWN_MM), from),
            Some(j) => {
                let (near, far) = if j.far { (0.0, half) } else { (half, f64::from(SHOWN_MM)) };
                (near, far, if j.zone > self.zone { to - SHOWN_MM / 2 } else { from })
            }
        };
        let (lo, hi) = (start.max(from), (start + (far - near) as i32).min(to));
        let segs = self.current();
        let place = |pos: i32| match content.from_end {
            ZoneEnd::First => near + f64::from(pos - start),
            ZoneEnd::Last => far - f64::from(pos - start),
        };
        content
            .curb
            .iter()
            .filter_map(|i| {
                let (s, e) = (i.from_mm.max(lo), (i.from_mm + i.length_mm).min(hi));
                if s >= e {
                    return None;
                }
                let piece = curbside(segs, i.side)?;
                let (a, b) = (place(s), place(e));
                Some(ShownInstance { piece, along: (a.min(b), a.max(b)), use_: i.use_ })
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
}
