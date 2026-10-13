//! What stands above the roadway: the heights of the pieces and of their curbs, the slabs and curb blocks drawn
//! with their front and side faces, and the order they are drawn in.

use std::fmt::Write;

use crate::shared::catalogue::KINDS;
use crate::shared::iso::shading::shade_filters;
use crate::street::model::View;
use crate::street::svg::{curb_edges, curb_width_mm};

use super::projection::{Projection, STRETCH_MM, drawn_mm, points};

/// How high each kind of piece stands above the roadway, in millimetres. They are plausible, not
/// surveyed, like the catalogue's rates. The furniture kinds stand on the sidewalk, so at its height.
pub const HEIGHTS: [(&str, i32); 17] = [
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

/// Which face of a strip, or of a curb, a polygon is.
#[derive(Clone, Copy)]
pub(super) enum Face {
    Top,
    Front,
    Side,
}

/// How much darker the faces turned from the eye are drawn, so that the depth reads: the front a little,
/// the side more. They are SVG filters the drawing carries, not CSS `filter: brightness()`, which Safari
/// may not apply to SVG shapes (an SVG filter is SVG 1.1); in sRGB, so they darken as `brightness()` does.
const SHADES: [(&str, &str); 2] = [("iso-shade-front", "0.9"), ("iso-shade-side", "0.78")];

/// The shading filters, for the drawing's `<defs>`.
pub(super) fn shades() -> String {
    shade_filters(&SHADES)
}

/// The shading a face of this kind gets, as an attribute: none for a top.
pub(super) fn shade(what: Face) -> &'static str {
    match what {
        Face::Top => "",
        Face::Front => " filter=\"url(#iso-shade-front)\"",
        Face::Side => " filter=\"url(#iso-shade-side)\"",
    }
}

/// One face: a polygon in the tint of the piece of kind `kind`, or of a curb when there is none
/// (marked with the face it is), shaded when it is turned from the eye, then the hatch `hatch` over it.
fn face(out: &mut String, what: Face, kind: Option<&str>, hatch: &str, pts: &str) {
    let name = match what {
        Face::Top => "top",
        Face::Front => "front",
        Face::Side => "side",
    };
    match kind {
        Some(k) => write!(out, "<polygon class=\"iso-{name} k-{k}\""),
        None => write!(out, "<polygon class=\"iso-curb\" data-face=\"{name}\""),
    }
    .unwrap();
    write!(out, " points=\"{pts}\"{}/><polygon class=\"hatch\" points=\"{pts}\" fill=\"url(#{hatch})\"/>", shade(what)).unwrap();
}

/// How high the piece after `i` is drawn, or the ground when `i` is the last.
pub(super) fn beside(v: &View, i: usize) -> f64 {
    v.segments.get(i + 1).map_or(0.0, |n| drawn_mm(height_mm(KINDS[n.kind].id)))
}

/// The piece at `i` as a strip along the stretch: its top, and where it stands above the ground its
/// front (the cut at the near end) and its right side where the next piece is lower. Its left side
/// faces away from the eye, so it is never seen.
pub(super) fn slab(out: &mut String, p: &Projection, v: &View, i: usize) {
    let s = &v.segments[i];
    let id = KINDS[s.kind].id;
    let (x0, x1, h, l) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64, drawn_mm(height_mm(id)), STRETCH_MM);
    let hatch = format!("h-{id}");
    if h > 0.0 {
        face(out, Face::Front, Some(id), &hatch, &points(p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, h, 0.0), (x0, h, 0.0)]));
        let low = beside(v, i);
        if low < h {
            face(out, Face::Side, Some(id), &hatch, &points(p, &[(x1, low, 0.0), (x1, low, l), (x1, h, l), (x1, h, 0.0)]));
        }
    }
    face(out, Face::Top, Some(id), &hatch, &points(p, &[(x0, h, 0.0), (x1, h, 0.0), (x1, h, l), (x0, h, l)]));
}

/// The curb of the piece at `i` along one edge (`right`). As high as the piece, it is a stripe on its
/// top; higher (a bike lane's, a boarding island), it is a block with its own front, side and top.
pub(super) fn curb(out: &mut String, p: &Projection, v: &View, i: usize, right: bool) {
    let s = &v.segments[i];
    let Some(c) = s.curb else { return };
    let id = KINDS[s.kind].id;
    let (h, l) = (drawn_mm(height_mm(id)), STRETCH_MM);
    let top = curb_top_mm(c, id);
    let cw = curb_width_mm(c).min(s.width_mm as f64 / 2.0);
    let edge = (s.x_mm + s.width_mm) as f64;
    let (x0, x1) = if right { (edge - cw, edge) } else { (s.x_mm as f64, s.x_mm as f64 + cw) };
    let hatch = format!("c-{c}");
    if top > h {
        face(out, Face::Front, None, &hatch, &points(p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, top, 0.0), (x0, top, 0.0)]));
        let low = if right { beside(v, i) } else { h };
        face(out, Face::Side, None, &hatch, &points(p, &[(x1, low, 0.0), (x1, low, l), (x1, top, l), (x1, top, 0.0)]));
    }
    face(out, Face::Top, None, &hatch, &points(p, &[(x0, top, 0.0), (x1, top, 0.0), (x1, top, l), (x0, top, l)]));
}

/// The strips to draw, each as (how high its top is drawn, which piece, which edge's curb if it is one), from the
/// lowest drawn top to the highest, so that a higher strip stands in front of a lower one beside it.
pub(super) fn strips(v: &View) -> Vec<(f64, usize, Option<bool>)> {
    let mut strips: Vec<(f64, usize, Option<bool>)> = Vec::new();
    for (i, s) in v.segments.iter().enumerate() {
        let id = KINDS[s.kind].id;
        strips.push((drawn_mm(height_mm(id)), i, None));
        if let Some(c) = s.curb {
            for right in curb_edges(v, i) {
                strips.push((curb_top_mm(c, id), i, Some(right)));
            }
        }
    }
    strips.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.is_some().cmp(&b.2.is_some())));
    strips
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{KINDS, kind_index};
    use crate::street::iso::testing::{furnished, one_way_bikes};
    use crate::street::model::Editor;

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
    fn the_strips_are_drawn_from_the_lowest_to_the_highest_as_they_are_drawn() {
        // A curbed bike lane beside a sidewalk: the curb stands 150 mm and the sidewalk 150 mm, but the sidewalk is
        // drawn three times as high, so it must come after the curb.
        let mut e = Editor::new(0);
        e.add(kind_index("bike").unwrap(), 1);
        for e in [e, Editor::new(0), Editor::new(1), furnished(), one_way_bikes().0] {
            let v = e.view();
            let tops: Vec<f64> = strips(&v)
                .iter()
                .map(|&(_, i, edge)| {
                    let (s, id) = (&v.segments[i], KINDS[v.segments[i].kind].id);
                    match edge {
                        None => drawn_mm(height_mm(id)),
                        Some(_) => curb_top_mm(s.curb.unwrap(), id),
                    }
                })
                .collect();
            assert!(tops.windows(2).all(|w| w[0] <= w[1]), "{tops:?}");
        }
    }
}
