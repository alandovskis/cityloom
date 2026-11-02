//! What stands above the roadway: the heights of the pieces and of their curbs, and the slabs and curb blocks drawn
//! with their front and side faces.

use crate::shared::catalogue::KINDS;
use crate::shared::iso::ground::fine_patterns;
use crate::shared::iso::shading::shade_filters;
use crate::street::model::SegView;
use crate::street::pieces::{curb_edges, curb_width_mm};

use super::order::Bx;
use super::projection::{Projection, Reach, drawn_mm};

/// How high each kind of piece stands above the roadway, in millimetres. They are plausible, not
/// surveyed, like the catalogue's rates. The furniture kinds stand on the sidewalk, so at its height.
pub const HEIGHTS: [(&str, i32); 18] = [
    ("sidewalk", 150),
    ("planting", 150),
    ("bike", 0),
    ("travel", 0),
    ("bus", 0),
    ("parking", 0),
    ("median", 150),
    ("loading", 0),
    ("shoulder", 0),
    ("bikerack", 150),
    ("bikeshare", 150),
    ("pole", 150),
    ("busshelter", 150),
    ("busstation", 150),
    ("bench", 150),
    ("terrace", 150),
    ("streetlamp", 150),
    ("buffer", 0),
];

/// How high a piece of this kind stands, in millimetres.
pub fn height_mm(kind_id: &str) -> i32 {
    HEIGHTS.iter().find(|(id, _)| *id == kind_id).map_or(0, |(_, h)| *h)
}

/// How high a curb stands above the roadway, in millimetres: a bike-friendly curb is a low ramp, a
/// Kassel kerb and a bus boarding island are raised for boarding.
pub fn curb_height_mm(curb: &str) -> i32 {
    match curb {
        "bikefriendly" => 60,
        "kassel" => 180,
        "island" => 200,
        _ => 150,
    }
}

/// How high the top of a curb on a piece of this kind is drawn, in millimetres of the drawing: as high as the
/// piece, exaggerated with it, and what the curb stands above the piece at its true height. A bike lane is at the
/// roadway's level, so its curb is a divider drawn true: exaggerated, its wall would cover half a narrow lane.
pub fn curb_top_mm(curb: &str, kind_id: &str) -> f64 {
    let h = height_mm(kind_id);
    drawn_mm(h) + (curb_height_mm(curb) - h).max(0) as f64
}

/// How much darker the faces turned from the eye are drawn, so that the depth reads: the front a little,
/// the side more. They are SVG filters the drawing carries, not CSS `filter: brightness()`, which Safari
/// may not apply to SVG shapes (an SVG filter is SVG 1.1); in sRGB, so they darken as `brightness()` does.
const SHADES: [(&str, &str); 2] = [("iso-shade-front", "0.9"), ("iso-shade-side", "0.78")];

/// The shading filters, for the drawing's `<defs>`.
pub(super) fn shades() -> String {
    shade_filters(&SHADES)
}

/// The surface materials' fine patterns for the drawing's `<defs>`: the grain, and not the regular joints of the
/// materials that have them, which are geometry (`joints`). They are drawn in millimetres of ground with no transform
/// of their own: a textured top is laid by the matrix of the polygon that fills with one (`top`), band by band.
pub(super) fn materials() -> String {
    fine_patterns("iso-m", [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
}

/// How high the piece next to `i` on its right (`right`) or its left is drawn, or the ground when there is none.
pub(super) fn beside(segs: &[SegView], i: usize, right: bool) -> f64 {
    let n = if right { segs.get(i + 1) } else { i.checked_sub(1).and_then(|j| segs.get(j)) };
    n.map_or(0.0, |n| drawn_mm(height_mm(KINDS[n.kind].id)))
}

/// Which side face of something spanning `x0` to `x1` across the street the eye sees: its right face (`Some(true)`)
/// when it is wholly left of the eye, its left face (`Some(false)`) when it is wholly right of it, and none when it
/// is over the eye (both faces are turned away) or its face is edge-on, in the eye's own plane.
pub(super) fn inner_face(p: &Projection, x0: f64, x1: f64) -> Option<bool> {
    if x1 < p.centre_mm {
        Some(true)
    } else if x0 > p.centre_mm {
        Some(false)
    } else {
        None
    }
}

/// Where the curb of the piece at `i` along one edge (`right`) lies across the street.
pub(super) fn curb_span(s: &crate::street::model::SegView, c: &str, right: bool) -> (f64, f64) {
    let cw = curb_width_mm(c).min(s.width_mm as f64 / 2.0);
    let edge = (s.x_mm + s.width_mm) as f64;
    if right { (edge - cw, edge) } else { (s.x_mm as f64, s.x_mm as f64 + cw) }
}

/// One strip to draw: how high its top is drawn, which piece, and which edge's curb if it is a curb.
pub(super) type Strip = (f64, usize, Option<bool>);

/// The box a strip fills, for ordering: its across range (a curb's, for a curb), from the ground to its top, along the
/// reach (the part of the stretch its section covers).
pub(super) fn strip_box(segs: &[SegView], strip: Strip, reach: Reach) -> Bx {
    let (top, i, edge) = strip;
    let s = &segs[i];
    let x = match (edge, s.curb) {
        (Some(right), Some(c)) => curb_span(s, c, right),
        _ => (s.x_mm as f64, (s.x_mm + s.width_mm) as f64),
    };
    Bx { x, y: (0.0, top), z: (reach.near, reach.far) }
}

/// How far a strip is from the eye across the street, in millimetres: from its nearest edge, nothing for one the eye is over.
fn across_eye(segs: &[SegView], p: &Projection, strip: Strip) -> f64 {
    let (_, i, edge) = strip;
    let s = &segs[i];
    let (x0, x1) = match (edge, s.curb) {
        (Some(right), Some(c)) => curb_span(s, c, right),
        _ => (s.x_mm as f64, (s.x_mm + s.width_mm) as f64),
    };
    (x0 - p.centre_mm).max(p.centre_mm - x1).max(0.0)
}

/// The strips to draw, each as (how high its top is drawn, which piece, which edge's curb if it is one), in the order they
/// are to be painted among themselves: from the farthest from the eye across the street to the nearest, and the lowest
/// first among those as far. The eye is over the middle of the street and every strip runs the whole stretch, so a strip
/// nearer the middle stands in front of the ones beyond it. This is the order the roadway's flat pieces are painted in
/// and the order `order::paint_order` keeps among the raised strips, which it interleaves with the solids.
pub(super) fn strips(segs: &[SegView], p: &Projection) -> Vec<Strip> {
    let mut strips: Vec<Strip> = Vec::new();
    for (i, s) in segs.iter().enumerate() {
        let id = KINDS[s.kind].id;
        strips.push((drawn_mm(height_mm(id)), i, None));
        if let Some(c) = s.curb {
            for right in curb_edges(segs, i) {
                strips.push((curb_top_mm(c, id), i, Some(right)));
            }
        }
    }
    strips.sort_by(|a, b| {
        across_eye(segs, p, *b).total_cmp(&across_eye(segs, p, *a)).then(a.0.total_cmp(&b.0)).then(a.1.cmp(&b.1)).then(a.2.is_some().cmp(&b.2.is_some()))
    });
    strips
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{KINDS, kind_index};
    use crate::shared::diagram::render::layer_markup;
    use crate::street::iso::projection::{Projection, Reach, points, r};
    use crate::street::iso::testing::{furnished, one_way_bikes};
    use crate::street::model::Editor;

    #[test]
    fn a_slab_over_a_reach_has_its_cut_at_the_near_end_of_the_reach() {
        use crate::street::iso::testing::xs_ys;
        let v = Editor::new(0).view();
        let i = v.segments.iter().position(|s| KINDS[s.kind].id == "sidewalk").unwrap();
        let s = &v.segments[i];
        let (x0, h) = (s.x_mm as f64, drawn_mm(height_mm("sidewalk")));
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm)).over(Reach { near: 20_000.0, far: 40_000.0 });
        let out = layer_markup(&crate::street::iso::scene::raised(&p, &v.segments, &crate::street::iso::testing::en(), &[(h, i, None)], &[], false, None), &p);
        let cut = r(p.at(x0, 0.0, 20_000.0).1);
        let ys: Vec<f64> = xs_ys(&out).into_iter().map(|(_, y)| y).collect();
        let front =
            points(&p, &[(x0, 0.0, 20_000.0), ((s.x_mm + s.width_mm) as f64, 0.0, 20_000.0), ((s.x_mm + s.width_mm) as f64, h, 20_000.0), (x0, h, 20_000.0)]);
        assert!(out.contains(&format!("points=\"{front}\"")), "the front is the cut at the reach's near end");
        assert!(ys.iter().all(|&y| y <= cut + 1e-9), "nothing nearer than the cut");
        assert!((ys.iter().cloned().fold(f64::MIN, f64::max) - cut).abs() < 1e-9);
        let far = r(p.at(x0, h, 40_000.0).1);
        assert!(ys.iter().all(|&y| y >= far - 1e-9));
    }

    #[test]
    fn every_kind_has_one_height_and_the_table_names_no_other() {
        for k in &KINDS {
            assert_eq!(HEIGHTS.iter().filter(|(id, _)| *id == k.id).count(), 1, "{}", k.id);
        }
        for (id, _) in &HEIGHTS {
            assert!(KINDS.iter().any(|k| k.id == *id), "{id} is not a kind");
        }
    }

    #[test]
    fn the_roadway_is_at_the_ground_and_the_sidewalk_median_and_planting_stand_150_mm_above_it() {
        for id in ["travel", "bus", "parking", "loading", "shoulder", "bike"] {
            assert_eq!(height_mm(id), 0, "{id}");
        }
        for id in ["sidewalk", "median", "planting"] {
            assert_eq!(height_mm(id), 150, "{id}");
        }
        assert_eq!(height_mm("nonsense"), 0);
    }

    #[test]
    fn a_curb_stands_150_mm_and_a_ramp_an_island_and_a_kassel_kerb_have_their_own_step() {
        assert_eq!(curb_height_mm("concrete"), 150);
        assert_eq!(curb_height_mm("granite"), 150);
        assert_eq!(curb_height_mm("bikefriendly"), 60);
        assert_eq!(curb_height_mm("kassel"), 180);
        assert_eq!(curb_height_mm("island"), 200);
    }

    #[test]
    fn the_furniture_stands_on_the_sidewalk_at_its_height() {
        for id in ["bikerack", "bikeshare", "pole", "busshelter", "busstation", "bench", "terrace", "streetlamp"] {
            assert_eq!(height_mm(id), 150, "{id}");
        }
    }

    #[test]
    fn every_curb_of_the_catalogue_has_its_height_decided() {
        // The match in curb_height_mm ends in a wildcard: a curb added to the catalogue must be added here too,
        // so that its height is a decision and not the wildcard's.
        let decided = [("granite", 150), ("concrete", 150), ("asphalt", 150), ("planted", 150), ("kassel", 180), ("bikefriendly", 60), ("island", 200)];
        assert_eq!(decided.len(), crate::shared::catalogue::CURBS.len());
        for c in &crate::shared::catalogue::CURBS {
            let (_, h) = decided.iter().find(|(id, _)| *id == c.id).unwrap_or_else(|| panic!("{} has no decided height", c.id));
            assert_eq!(curb_height_mm(c.id), *h, "{}", c.id);
        }
    }

    #[test]
    fn a_curb_rises_with_its_piece_and_what_it_stands_above_the_piece_is_drawn_true() {
        // A sidewalk's curb is as high as the sidewalk, exaggerated with it; an island's 50 mm more is drawn true.
        assert_eq!(curb_top_mm("concrete", "sidewalk"), drawn_mm(150));
        assert_eq!(curb_top_mm("island", "sidewalk"), drawn_mm(150) + 50.0);
        // A bike lane is at the roadway's level: its curb is a divider standing its true height, so that it does not
        // cover half the lane.
        assert_eq!(curb_top_mm("concrete", "bike"), 150.0);
        assert_eq!(curb_top_mm("bikefriendly", "bike"), 60.0);
    }

    #[test]
    fn the_strips_are_drawn_from_the_farthest_from_the_middle_of_the_street_to_the_nearest() {
        let mut e = Editor::new(0);
        e.add(kind_index("bike").unwrap(), 1);
        for e in [e, Editor::new(0), Editor::new(1), Editor::new(3), furnished(), one_way_bikes().0] {
            let v = e.view();
            let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
            let s = strips(&v.segments, &p);
            let far: Vec<f64> = s.iter().map(|&st| across_eye(&v.segments, &p, st)).collect();
            assert!(far.windows(2).all(|w| w[0] >= w[1]), "{far:?}");
            // Among strips as far, the lower is first.
            for w in s.windows(2) {
                if across_eye(&v.segments, &p, w[0]) == across_eye(&v.segments, &p, w[1]) {
                    assert!(w[0].0 <= w[1].0);
                }
            }
        }
    }

    #[test]
    fn from_over_the_middle_a_piece_left_of_it_shows_its_right_face_and_one_right_of_it_its_left() {
        let p = Projection::fit(1000.0, 18_000);
        assert_eq!(inner_face(&p, 0.0, 2_400.0), Some(true));
        assert_eq!(inner_face(&p, 15_600.0, 18_000.0), Some(false));
        // Over the eye, or edge-on in its plane: no side is seen.
        assert_eq!(inner_face(&p, 8_000.0, 10_000.0), None);
        assert_eq!(inner_face(&p, 0.0, 9_000.0), None);
        assert_eq!(inner_face(&p, 9_000.0, 12_000.0), None);
    }
}
