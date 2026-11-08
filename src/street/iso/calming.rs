//! Raised tables and crossings in the 2.5D (#147): the roadway's pieces lifted over the plateau by `TABLE_HEIGHT_MM`
//! (drawn `RISE` times as high, as the slabs are), a ramp at each end, the lane markings stopped at the ramp feet, and
//! the bars of a zebra on a crossing's plateau. The model says where (`ShownCalming`); this module says what of it is
//! drawn and what it covers, and `scene::tables` builds the shapes.
//!
//! An element is drawn on the selected zone's run only, whole or not at all: it is judged on its plateau
//! (`ShownCalming::drawn`), and a ramp that crosses the stretch's edge is cut there at the height it has reached.
//! A transition's taper is not followed: the table lies across the pieces as the section has them.

use crate::shared::catalogue::Side;
use crate::street::model::{CHICANE_HEIGHT_MM, CHICANE_WIDTH_MM, CalmingKind, SegView, ShownCalming, TABLE_HEIGHT_MM};

use super::order::Bx;
use super::projection::drawn_mm;
use super::raised::Strip;

/// Width of a zebra bar and of the gap between two, in millimetres. An estimate pending MTQ Tome V's drawing.
pub(super) const ZEBRA_MM: f64 = 400.0;

/// One element to draw.
#[derive(Clone, Copy, Debug)]
pub(super) struct Table {
    /// The whole element, ramps included, cut to the stretch and the zone.
    pub along: (f64, f64),
    /// The whole element, not cut.
    pub full: (f64, f64),
    /// The plateau between the ramps, not cut.
    pub plateau: (f64, f64),
    pub kind: CalmingKind,
}

impl Table {
    /// How high the plateau is drawn.
    pub fn height() -> f64 {
        drawn_mm(TABLE_HEIGHT_MM)
    }

    /// How high the element stands at `z`, drawn: nothing at its feet, the plateau's height between the ramps.
    pub fn height_at(&self, z: f64) -> f64 {
        let h = Self::height();
        if z <= self.plateau.0 {
            let run = self.plateau.0 - self.full.0;
            if run <= 0.0 { h } else { h * ((z - self.full.0) / run).clamp(0.0, 1.0) }
        } else if z >= self.plateau.1 {
            let run = self.full.1 - self.plateau.1;
            if run <= 0.0 { h } else { h * ((self.full.1 - z) / run).clamp(0.0, 1.0) }
        } else {
            h
        }
    }
}

/// One build-out of a chicane to draw: a block that reaches into the roadway from the curb on `side`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Buildout {
    pub along: (f64, f64),
    pub side: Side,
}

impl Buildout {
    /// How high it is drawn.
    pub fn height() -> f64 {
        drawn_mm(CHICANE_HEIGHT_MM)
    }
}

/// What the run lays the elements over: the elements drawn and the roadway pieces they lift.
#[derive(Clone, Debug, Default)]
pub(super) struct Laid {
    pub tables: Vec<Table>,
    pub buildouts: Vec<Buildout>,
    /// The pieces lifted, indices into the section's pieces: those drawn flat on the roadway, left to right.
    pub pieces: Vec<usize>,
}

impl Laid {
    /// The elements of `shown` that are drawn, over the `flat` pieces of the section. Nothing without a roadway.
    pub fn of(shown: &[ShownCalming], flat: &[Strip]) -> Laid {
        let mut pieces: Vec<usize> = flat.iter().map(|&(_, i, _)| i).collect();
        pieces.sort_unstable();
        if pieces.is_empty() {
            return Laid::default();
        }
        let tables = shown
            .iter()
            .filter(|s| s.kind != CalmingKind::Chicane && s.drawn())
            .map(|s| Table { along: s.along, full: s.full, plateau: s.plateau, kind: s.kind })
            .collect();
        let buildouts = shown
            .iter()
            .filter(|s| s.kind == CalmingKind::Chicane)
            .flat_map(|s| s.buildouts.iter().map(|&(n, l, side)| Buildout { along: (n, l), side }))
            .collect();
        Laid { tables, buildouts, pieces }
    }

    /// The stretches of the street the markings are kept off: each element, ramp to ramp.
    pub fn cuts(&self) -> Vec<(f64, f64)> {
        self.tables.iter().map(|t| t.along).collect()
    }

    /// The box an element fills, for ordering: across the lifted pieces, up to the plateau, along its length.
    pub fn bx(&self, segs: &[SegView], t: &Table) -> Bx {
        let (lo, hi) = self.across(segs);
        Bx { x: (lo, hi), y: (0.0, Table::height()), z: t.along }
    }

    /// Where a build-out stands across: `CHICANE_WIDTH_MM` in from the roadway's edge on its side, no more than half the
    /// roadway.
    pub fn buildout_across(&self, segs: &[SegView], b: &Buildout) -> (f64, f64) {
        let (lo, hi) = self.across(segs);
        let w = f64::from(CHICANE_WIDTH_MM).min((hi - lo) / 2.0);
        if b.side == Side::Left { (lo, lo + w) } else { (hi - w, hi) }
    }

    /// The box a build-out fills, for ordering.
    pub fn buildout_bx(&self, segs: &[SegView], b: &Buildout) -> Bx {
        Bx { x: self.buildout_across(segs, b), y: (0.0, Buildout::height()), z: b.along }
    }

    /// The span across of the lifted pieces.
    pub fn across(&self, segs: &[SegView]) -> (f64, f64) {
        self.pieces.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &i| {
            let s = &segs[i];
            (lo.min(s.x_mm as f64), hi.max((s.x_mm + s.width_mm) as f64))
        })
    }
}

/// Millimetres of `span` left when each of the `cuts` is taken out, in order.
pub(super) fn outside(span: (f64, f64), cuts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut left = vec![span];
    for &(a, b) in cuts {
        left = left
            .into_iter()
            .flat_map(|(s, e)| {
                if b <= s || a >= e {
                    return vec![(s, e)];
                }
                let mut parts = Vec::new();
                if a > s {
                    parts.push((s, a));
                }
                if b < e {
                    parts.push((b, e));
                }
                parts
            })
            .collect();
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::iso::iso_svg;
    use crate::street::iso::scene::testing::{curbed, racked};
    use crate::street::iso::testing::{drawn, en, furnished, joined, joined_from_last, marked, seen};
    use crate::street::model::Editor;

    fn fnv(s: &str) -> u64 {
        s.bytes().fold(0xcbf29ce484222325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
    }

    #[test]
    fn a_street_without_calming_draws_byte_identically() {
        // The markup of these streets before raised tables existed, as a hash: any element, class or filter added to
        // a street with none changes one of them.
        let mut got = Vec::new();
        for i in 0..4 {
            got.push(fnv(&drawn(&Editor::new(i))));
        }
        got.extend([fnv(&drawn(&furnished())), fnv(&drawn(&marked())), fnv(&drawn(&curbed())), fnv(&drawn(&racked()))]);
        for v in [joined(true, 0), joined(true, 1), joined_from_last(true, 0), joined_from_last(false, 1)] {
            got.push(fnv(&seen(&iso_svg(&v, &en(), 1050.0).markup)));
        }
        let want: [u64; 12] = [
            2280381342206955045,
            17738161488425106810,
            16921369358436838195,
            11998779749613471576,
            11670796961289239781,
            15060454089154254298,
            6187210646751281260,
            8120316114476581152,
            12735677471064399099,
            14102934797627728503,
            2840175765913559113,
            12727850407270563526,
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn what_is_left_of_a_span_outside_the_cuts() {
        assert_eq!(outside((0.0, 40.0), &[]), vec![(0.0, 40.0)]);
        assert_eq!(outside((0.0, 40.0), &[(10.0, 17.0)]), vec![(0.0, 10.0), (17.0, 40.0)]);
        assert_eq!(outside((0.0, 40.0), &[(10.0, 17.0), (30.0, 45.0)]), vec![(0.0, 10.0), (17.0, 30.0)]);
        assert_eq!(outside((0.0, 5.0), &[(10.0, 17.0)]), vec![(0.0, 5.0)]);
        assert!(outside((10.0, 17.0), &[(8.0, 20.0)]).is_empty());
    }

    #[test]
    fn the_height_climbs_the_ramps_and_stays_on_the_plateau() {
        let t = Table { along: (10_000.0, 17_000.0), full: (10_000.0, 17_000.0), plateau: (11_500.0, 15_500.0), kind: CalmingKind::Table };
        let h = Table::height();
        assert_eq!(h, drawn_mm(TABLE_HEIGHT_MM));
        assert_eq!((t.height_at(10_000.0), t.height_at(10_750.0), t.height_at(11_500.0), t.height_at(13_000.0)), (0.0, h / 2.0, h, h));
        assert_eq!((t.height_at(15_500.0), t.height_at(16_250.0), t.height_at(17_000.0), t.height_at(5_000.0)), (h, h / 2.0, 0.0, 0.0));
    }
}

#[cfg(test)]
mod drawing {
    use super::*;
    use crate::shared::catalogue::Side;
    use crate::shared::units::Units;
    use crate::street::iso::iso_label;
    use crate::street::iso::testing::{drawn, en};
    use crate::street::model::{Editor, Street, ZoneEnd, ZoneFrame};

    fn editor(kind: CalmingKind, from_mm: i32, length_mm: i32) -> Editor {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
        e.add_calming(kind, from_mm, length_mm).unwrap();
        e
    }

    fn count(markup: &str, what: &str) -> usize {
        markup.matches(what).count()
    }

    #[test]
    fn a_table_is_one_titled_group_of_ramps_and_a_raised_top_with_no_uid() {
        let m = drawn(&editor(CalmingKind::Table, 10_000, 7_000));
        assert_eq!(count(&m, "<g class=\"iso-table\""), 1);
        let g = &m[m.find("<g class=\"iso-table\"").unwrap()..];
        let g = &g[..g.find("</g>").unwrap()];
        assert!(g.contains("iso-ramp"), "a ramp at each end");
        assert!(g.contains("<title>"), "named for a screen reader");
        assert!(!g.contains("data-uid"), "the strip selects it, the drawing does not");
        assert!(!g.contains("iso-zebra"));
    }

    #[test]
    fn a_crossing_paints_zebra_bars_and_a_table_does_not() {
        assert!(count(&drawn(&editor(CalmingKind::Crossing, 10_000, 7_000)), "iso-zebra") > 0);
        assert_eq!(count(&drawn(&editor(CalmingKind::Table, 10_000, 7_000)), "iso-zebra"), 0);
    }

    #[test]
    fn the_defs_carry_the_ramps_shade_only_with_a_table() {
        assert!(drawn(&editor(CalmingKind::Table, 10_000, 7_000)).contains("iso-shade-ramp"));
        assert!(!drawn(&Editor::new(0)).contains("iso-shade-ramp"));
    }

    #[test]
    fn the_lane_markings_stop_at_the_ramp_feet() {
        let flat = drawn(&Editor::new(0));
        let with = drawn(&editor(CalmingKind::Table, 10_000, 7_000));
        assert_ne!(flat.matches("iso-paint").count(), with.matches("iso-paint").count(), "paint is cut where the table stands");
    }

    #[test]
    fn an_element_whose_plateau_leaves_the_stretch_is_not_drawn() {
        let m = drawn(&editor(CalmingKind::Table, 37_000, 7_000));
        assert_eq!(count(&m, "<g class=\"iso-table\""), 0);
        let m = drawn(&editor(CalmingKind::Table, 50_000, 7_000));
        assert_eq!(count(&m, "<g class=\"iso-table\""), 0);
    }

    #[test]
    fn the_label_counts_the_tables_drawn_in_both_languages() {
        let e = editor(CalmingKind::Table, 10_000, 7_000);
        assert!(iso_label(&e.view(), &en(), Units::Metres).contains("One raised table is shown."));
        assert!(!iso_label(&Editor::new(0).view(), &en(), Units::Metres).contains("raised table"));
        let e = editor(CalmingKind::Table, 37_000, 7_000);
        assert!(!iso_label(&e.view(), &en(), Units::Metres).contains("raised table"));
    }

    fn chicane_of(from_mm: i32, length_mm: i32, first: Option<Side>) -> Editor {
        let s = Street::sample(0, Side::Right);
        let mut e = Editor::from_street(&s, &s, 0).with_frame(Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
        e.add_calming_first(CalmingKind::Chicane, from_mm, length_mm, first).unwrap();
        e
    }

    fn chicane(from_mm: i32, first: Option<Side>) -> Editor {
        chicane_of(from_mm, 24_000, first)
    }

    #[test]
    fn a_chicane_is_a_titled_group_per_build_out_with_no_ramp_and_no_uid() {
        let m = drawn(&chicane(5_000, None));
        assert_eq!(count(&m, "<g class=\"iso-chicane\""), 3, "three build-outs stand in the stretch");
        let g = &m[m.find("<g class=\"iso-chicane\"").unwrap()..];
        let g = &g[..g.find("</g>").unwrap()];
        assert!(g.contains("<title>Chicane</title>"), "named for a screen reader: {g}");
        assert!(!g.contains("data-uid"), "the strip selects it, the drawing does not");
        assert_eq!(count(&m, "iso-ramp"), 0, "a chicane has no ramps");
        assert_eq!(count(&m, "<g class=\"iso-table\""), 0);
        assert!(!m.contains("iso-shade-ramp"), "the ramps' shade is for tables");
    }

    #[test]
    fn the_build_outs_alternate_sides_from_the_first_and_a_build_out_past_the_stretch_is_not_drawn() {
        let m = drawn(&chicane(25_000, None));
        assert_eq!(count(&m, "<g class=\"iso-chicane\""), 2, "the third lies past the 40 m");
        assert_eq!(count(&drawn(&chicane_of(37_000, 14_000, None)), "<g class=\"iso-chicane\""), 0);
        // The side moves the first build-out across the street: the drawing differs and has as many.
        let right = drawn(&chicane(5_000, Some(Side::Right)));
        let left = drawn(&chicane(5_000, Some(Side::Left)));
        assert_ne!(right, left);
        assert_eq!(count(&left, "<g class=\"iso-chicane\""), 3);
    }

    #[test]
    fn the_label_counts_the_chicanes_apart_from_the_tables() {
        let e = chicane(5_000, None);
        let label = iso_label(&e.view(), &en(), Units::Metres);
        assert!(label.contains("One chicane is shown."), "{label}");
        assert!(!label.contains("raised table"), "{label}");
        let none = iso_label(&chicane_of(37_000, 14_000, None).view(), &en(), Units::Metres);
        assert!(!none.contains("chicane is shown") && !none.contains("chicanes are shown"), "{none}");
    }

    #[test]
    fn a_table_and_a_chicane_together_draw_both() {
        let mut e = chicane(25_000, None);
        e.add_calming(CalmingKind::Table, 5_000, 7_000).unwrap();
        let m = drawn(&e);
        assert_eq!((count(&m, "<g class=\"iso-table\""), count(&m, "<g class=\"iso-chicane\"")), (1, 2));
    }
}
