//! The raised tables and crossings the 2.5D shows: the elements that overlap the stretch it draws, mapped by the same
//! `Stretch` as the curbside uses (see `curb_shown`), so from the anchored end whatever it is and mirrored when it is
//! the last end. A table spans the roadway, so it has no side and no piece.

use serde::Serialize;

use super::Editor;
use super::calming::{Kind, RAMP_MM};
use crate::shared::catalogue::Side;

/// One raised element as the 2.5D shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShownCalming {
    /// Where it lies along the stretch, in millimetres from the stretch's near end: only the part inside the stretch
    /// and the zone.
    pub along: (f64, f64),
    /// The plateau, between the two ramps, in the same coordinates and not clipped: it may lie partly outside the
    /// stretch.
    pub plateau: (f64, f64),
    /// The whole element, ramps included, in the same coordinates and not clipped.
    pub full: (f64, f64),
    pub kind: Kind,
    /// The whole element lies inside the stretch and the zone.
    pub whole: bool,
    /// For a chicane, the build-outs that lie wholly inside the stretch and the zone, in the same coordinates, each with
    /// the side of the section it is on; none for anything else.
    pub buildouts: Vec<(f64, f64, Side)>,
}

impl ShownCalming {
    /// Whether the drawing shows it: the plateau lies wholly inside the stretch and the zone, so an element cut by
    /// the stretch's edge is drawn whole or not at all, as a tree is, judged on its plateau.
    pub fn drawn(&self) -> bool {
        if self.kind == Kind::Chicane {
            return !self.buildouts.is_empty();
        }
        // A modal filter is a block: whole or not at all.
        if self.kind == Kind::ModalFilter {
            return self.whole;
        }
        self.plateau.0 >= self.along.0 && self.plateau.1 <= self.along.1
    }
}

impl Editor {
    /// The raised elements that overlap the stretch the 2.5D shows. None without a stretch.
    pub(super) fn calming_shown(&self) -> Vec<ShownCalming> {
        let content = &self.states[self.cursor].content;
        if content.calming.is_empty() {
            return Vec::new();
        }
        let Some(stretch) = self.stretch() else { return Vec::new() };
        content
            .calming
            .iter()
            .filter_map(|c| {
                let (along, full) = stretch.map(c.from_mm, c.to_mm())?;
                // Each ramp is `RAMP_MM` long, or half the element when that is shorter.
                let ramp = f64::from(RAMP_MM).min((full.1 - full.0) / 2.0);
                let plateau = (full.0 + ramp, full.1 - ramp);
                // A chicane is its build-outs, each drawn whole or not at all.
                let buildouts = c
                    .buildouts()
                    .into_iter()
                    .filter_map(|(from, to, side)| stretch.map(from, to).filter(|(a, f)| a == f).map(|(a, _)| (a.0, a.1, side)))
                    .collect();
                Some(ShownCalming { along, plateau, full, kind: c.kind, whole: along == full, buildouts })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::street::model::{Street, ZoneEnd, ZoneFrame};

    const LEN: i32 = 60_000;

    fn editor(from_end: ZoneEnd) -> Editor {
        let s = Street::sample(0, Side::Right);
        Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: LEN, from_end }))
    }

    #[test]
    fn a_street_with_no_elements_shows_none() {
        assert!(Editor::new(0).view().calming_shown.is_empty());
    }

    #[test]
    fn an_element_is_mapped_into_the_stretch_with_its_plateau_between_the_ramps() {
        let mut e = editor(ZoneEnd::First);
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        e.add_calming(Kind::Crossing, 38_000, 7_000).unwrap();
        e.add_calming(Kind::Table, 50_000, 7_000).unwrap();
        let s = e.view().calming_shown;
        assert_eq!(s.len(), 2, "the third lies wholly past the stretch");
        assert_eq!(
            (s[0].along, s[0].full, s[0].plateau, s[0].kind, s[0].whole),
            ((10_000.0, 17_000.0), (10_000.0, 17_000.0), (11_200.0, 15_800.0), Kind::Table, true)
        );
        // The second runs past the 40 m: the part inside is clipped, the plateau is not, and it is not drawn.
        assert_eq!((s[1].along, s[1].full, s[1].plateau, s[1].whole), ((38_000.0, 40_000.0), (38_000.0, 45_000.0), (39_200.0, 43_800.0), false));
        assert!(s[0].drawn() && !s[1].drawn());
    }

    #[test]
    fn a_ramp_that_crosses_the_edge_does_not_hide_a_plateau_inside_it() {
        let mut e = editor(ZoneEnd::First);
        e.add_calming(Kind::Table, 33_000, 7_000).unwrap();
        let s = e.view().calming_shown;
        assert_eq!((s[0].along, s[0].plateau, s[0].whole, s[0].drawn()), ((33_000.0, 40_000.0), (34_200.0, 38_800.0), true, true));
        let mut e = editor(ZoneEnd::First);
        e.add_calming(Kind::Table, 37_000, 7_000).unwrap();
        let s = e.view().calming_shown;
        assert_eq!((s[0].whole, s[0].drawn()), (false, false), "the plateau reaches 42.8 m");
    }

    #[test]
    fn anchored_at_the_last_end_the_stretch_is_mirrored() {
        let mut e = editor(ZoneEnd::Last);
        e.add_calming(Kind::Table, 10_000, 7_000).unwrap();
        let s = e.view().calming_shown;
        // 10 m to 17 m from the anchored end is 30 m to 23 m from the stretch's near end.
        assert_eq!((s[0].along, s[0].plateau), ((23_000.0, 30_000.0), (24_200.0, 28_800.0)));
    }

    #[test]
    fn a_chicane_shows_the_build_outs_wholly_inside_the_stretch_with_their_sides() {
        let mut e = editor(ZoneEnd::First);
        e.add_calming(Kind::Chicane, 5_000, 24_000).unwrap();
        let s = e.view().calming_shown;
        assert_eq!(s[0].buildouts, vec![(5_000.0, 9_000.0, Side::Right), (15_000.0, 19_000.0, Side::Left), (25_000.0, 29_000.0, Side::Right)]);
        assert!(s[0].drawn());
        // The third build-out of one starting at 25 m lies past the 40 m: only the whole ones are shown.
        let mut e = editor(ZoneEnd::First);
        e.add_calming(Kind::Chicane, 25_000, 24_000).unwrap();
        let s = e.view().calming_shown;
        assert_eq!(s[0].buildouts, vec![(25_000.0, 29_000.0, Side::Right), (35_000.0, 39_000.0, Side::Left)]);
        // One that begins inside and has no build-out whole in the stretch is not drawn.
        let mut e = editor(ZoneEnd::First);
        e.add_calming(Kind::Chicane, 37_000, 14_000).unwrap();
        let s = e.view().calming_shown;
        assert!(s[0].buildouts.is_empty() && !s[0].drawn());
    }

    #[test]
    fn a_chicane_anchored_at_the_last_end_is_mirrored_along_the_stretch_but_keeps_its_sides() {
        let mut e = editor(ZoneEnd::Last);
        e.add_calming_first(Kind::Chicane, 5_000, 14_000, Some(Side::Left)).unwrap();
        let s = e.view().calming_shown;
        // 5 to 9 m and 15 to 19 m from the anchored end are 31 to 35 m and 21 to 25 m from the stretch's near end.
        assert_eq!(s[0].buildouts, vec![(31_000.0, 35_000.0, Side::Left), (21_000.0, 25_000.0, Side::Right)]);
    }
}
