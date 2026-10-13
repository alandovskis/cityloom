//! The roadway's surface: today the one-way arrow laid on a piece's top.

use std::fmt::Write;

use crate::shared::catalogue::KINDS;
use crate::street::model::SegView;
use crate::street::svg::dir_glyph;

use super::projection::{ARROW_AT_MM, MIN_SOLID_PX, Projection, RECEDE_X, RECEDE_Y, drawn_mm, r};
use super::raised::height_mm;

/// The section's one-way arrow, laid on the piece's top `ARROW_AT_MM` along the stretch: drawn in
/// millimetres and mapped onto the street's plane, so "away" points into the distance.
pub(super) fn arrow(out: &mut String, p: &Projection, s: &SegView) {
    let Some(d) = s.direction else { return };
    let w = s.width_mm as f64;
    if w * p.scale < MIN_SOLID_PX {
        return;
    }
    let (cx, h, zc) = (s.x_mm as f64 + w / 2.0, drawn_mm(height_mm(KINDS[s.kind].id)), ARROW_AT_MM);
    let (len, head) = arrow_shape(w);
    let (e, f) = p.at(cx, h, zc);
    let k = p.scale;
    // The outer transform is the street's plane; the inner one narrows the glyph, whose head is as wide as
    // 0.64 of its length, to the head wanted.
    write!(
        out,
        "<g class=\"iso-dir\" transform=\"matrix({k} 0 {} {} {} {})\"><g transform=\"scale({} 1)\">{}</g></g>",
        -RECEDE_X * k,
        RECEDE_Y * k,
        r(e),
        r(f),
        head / (0.64 * len),
        dir_glyph(d, 0.0, 0.0, len)
    )
    .unwrap();
}

/// How long an arrow laid on a piece `w` wide is, and how wide its head, in millimetres: as long as two widths
/// of the piece and no longer than 8 m, with a head half the piece wide. A millimetre along the street is drawn
/// half as long as one across it, so this is an arrow about twice as long as its head is wide, as a painted one
/// looks from the side.
fn arrow_shape(w: f64) -> (f64, f64) {
    ((2.0 * w).min(8_000.0), w / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::iso::iso_svg;
    use crate::street::iso::solids::{Solid, blocks};
    use crate::street::iso::testing::{apart, count, drawn, en, furnished, hull, one_way_bikes};
    use crate::street::model::Editor;

    #[test]
    fn an_arrow_is_longer_on_the_street_than_its_head_is_wide_and_its_head_stays_well_inside_the_lane() {
        for w in [1_500.0, 3_300.0, 5_000.0] {
            let (len, head) = arrow_shape(w);
            assert!(head <= 0.6 * w, "{w}: head {head}");
            // Along the street a millimetre is drawn half as long as across it.
            assert!(0.5 * len >= 1.2 * head, "{w}: {len} long, head {head}");
            assert!(len <= 8_000.0);
        }
        let m = drawn(&Editor::new(0));
        let (len, head) = arrow_shape(3_300.0);
        assert!(m.contains(&format!("<g transform=\"scale({} 1)\">", head / (0.64 * len))), "the drawing has not got it");
    }

    #[test]
    fn an_arrow_lies_near_the_near_end_where_no_tree_can_hide_it() {
        // The arrow is anchored ARROW_AT_MM along its lane.
        let v = Editor::new(1).view();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let lane = v.segments.iter().find(|s| s.direction.is_some()).unwrap();
        let (e, f) = p.at(lane.x_mm as f64 + lane.width_mm as f64 / 2.0, 0.0, ARROW_AT_MM);
        assert!(m.contains(&format!(" {} {})\"><g transform=\"scale(", r(e), r(f))), "the drawing has not got it");
        // On every street with trees, at every width that draws them, no trunk or crown covers any part of the ground
        // an arrow is laid on (its length along by its head across).
        for e in [Editor::new(1), furnished()] {
            let v = e.view();
            for width in [760.0, 1050.0, 1800.0] {
                let p = Projection::fit(width, v.row_mm.max(v.total_mm));
                let trees: Vec<Vec<(f64, f64)>> = (0..v.segments.len())
                    .flat_map(|i| blocks(&v, i, &p))
                    .filter(|b| matches!(b.solid, Solid::Trunk | Solid::Crown))
                    .map(|b| {
                        let corners = [b.x.0, b.x.1]
                            .into_iter()
                            .flat_map(|x| [b.y.0, b.y.1].into_iter().flat_map(move |y| [b.z.0, b.z.1].into_iter().map(move |z| (x, y, z))));
                        hull(corners.map(|(x, y, z)| p.at(x, y, z)).collect())
                    })
                    .collect();
                assert!(!trees.is_empty(), "{width}");
                for (i, s) in v.segments.iter().enumerate().filter(|(_, s)| s.direction.is_some() && s.width_mm as f64 * p.scale >= MIN_SOLID_PX) {
                    let (len, head) = arrow_shape(s.width_mm as f64);
                    let (cx, h) = (s.x_mm as f64 + s.width_mm as f64 / 2.0, drawn_mm(height_mm(KINDS[s.kind].id)));
                    let (z0, z1) = (ARROW_AT_MM - len / 2.0, ARROW_AT_MM + len / 2.0);
                    let ground =
                        [(cx - head / 2.0, h, z0), (cx + head / 2.0, h, z0), (cx + head / 2.0, h, z1), (cx - head / 2.0, h, z1)].map(|(x, y, z)| p.at(x, y, z));
                    assert!(trees.iter().all(|t| apart(t, &ground)), "{width}: the arrow of piece {i} is hidden");
                }
            }
        }
    }

    #[test]
    fn a_lane_s_arrow_is_drawn_with_its_strip_so_its_curbs_and_a_higher_neighbour_stand_over_it() {
        let (e, middle, kerbside) = one_way_bikes();
        let v = e.view();
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-dir\""), 4);
        for (lane, uid) in [("middle", middle), ("kerbside", kerbside)] {
            let open = format!("<g class=\"iso-piece\" data-uid=\"{uid}\">");
            let slab = m.find(&open).expect("the slab is drawn");
            // The arrow follows the lane's own strip straight away...
            let after = slab + m[slab..].find("</g>").unwrap() + 4;
            assert!(m[after..].starts_with("<g class=\"iso-dir\""), "the {lane} lane's arrow does not follow its strip");
            // ...and comes before the lane's curb blocks.
            let curbs: Vec<usize> = m.match_indices(&open).skip(1).map(|(at, _)| at).collect();
            assert!(!curbs.is_empty() && curbs.iter().all(|c| *c > after), "the {lane} lane's curbs come before its arrow");
        }
        // The kerbside lane's right neighbour, the raised sidewalk, is drawn over its arrow.
        let pos = v.segments.iter().position(|s| s.uid == kerbside).unwrap();
        let sidewalk = v.segments[pos + 1].uid;
        assert_eq!(KINDS[v.segments[pos + 1].kind].id, "sidewalk");
        let slab = m.find(&format!("<g class=\"iso-piece\" data-uid=\"{kerbside}\">")).unwrap();
        let arrow = slab + m[slab..].find("<g class=\"iso-dir\"").unwrap();
        assert!(m.find(&format!("<g class=\"iso-piece\" data-uid=\"{sidewalk}\">")).unwrap() > arrow);
    }

    #[test]
    fn a_one_way_piece_carries_the_section_s_arrow_laid_on_the_street() {
        let e = Editor::new(0);
        let one_way = e.view().segments.iter().filter(|s| s.direction.is_some()).count();
        assert_eq!(one_way, 2);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-dir\""), one_way);
        assert_eq!(count(&m, "class=\"dir\""), one_way);
        // Laid on the plane: across at full scale, along receding up and to the right.
        let p = Projection::fit(1050.0, 18_000);
        assert!(m.contains(&format!("transform=\"matrix({} 0 {} {} ", p.scale, -RECEDE_X * p.scale, RECEDE_Y * p.scale)), "the drawing has not got it");
    }
}
