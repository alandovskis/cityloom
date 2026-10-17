//! The street in 2.5D: a straight stretch of it seen in perspective from over its centre line, looking along it, each
//! piece an extruded strip with its height, the few things that stand on the pieces as plain solids, and the
//! lane markings on the roadway. It is drawn from the same view the section is and is read-only:
//! the section is where the street is edited.
//!
//! The rises are exaggerated: a curb or a sidewalk stands 150 mm, a few pixels at the scale a whole
//! street is drawn at, so the slabs, and the curbs with them, are drawn `RISE` times as high as they stand
//! (the height tables keep the true heights). What a curb stands above its own piece (a bike lane's divider)
//! is drawn true, so that it does not bury a narrow lane (`curb_top_mm`). The things that stand on the
//! pieces keep their own size and sit on the drawn tops.
//!
//! The code is split by what it draws: `projection` (where a point lands), `raised` (the strips standing
//! above the roadway and their order), `solids` (boxes on the pieces), `surface` (the roadway), `markings` (which
//! lines and symbols a street carries), `symbols` (their shapes) and `paint` (the markings layer).

use std::fmt::Write;

use crate::shared::catalogue::KINDS;
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::View;
use crate::street::svg::curb_edges;

mod markings;
mod order;
mod paint;
mod projection;
mod raised;
mod solids;
mod surface;
mod symbols;
#[cfg(test)]
mod testing;

pub use projection::{ARROW_AT_MM, EYE_BACK_MM, EYE_HEIGHT_MM, MAX_HEIGHT, MIN_SOLID_PX, Projection, RISE, STRETCH_MM, TALLEST_MM, drawn_mm};
pub use raised::{HEIGHTS, curb_height_mm, curb_top_mm, height_mm};

use order::Bx;
use projection::points;
use raised::{beside, inner_face, materials, shades, strips};
use solids::{Block, block, blocks};

/// The 2.5D drawn, and the size it is drawn at.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoSvg {
    pub markup: String,
    pub width: f64,
    pub height: f64,
}

/// The outline of the selected piece at `i`, as one shape round everything drawn for it (its strip
/// and its curbs): its near face, the side the eye sees where there is one, and its top, at the height of
/// its highest part.
fn outline(out: &mut String, p: &Projection, v: &View, i: usize) {
    let s = &v.segments[i];
    let id = KINDS[s.kind].id;
    let top = s.curb.filter(|_| !curb_edges(v, i).is_empty()).map_or(drawn_mm(height_mm(id)), |c| curb_top_mm(c, id));
    let (x0, x1, l) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64, STRETCH_MM);
    let corners = match inner_face(p, x0, x1) {
        // Left of the eye: its right face is seen, so the outline climbs it and comes back along the top.
        Some(true) => {
            let low = beside(v, i, true).min(top);
            vec![(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, low, 0.0), (x1, low, l), (x1, top, l), (x0, top, l), (x0, top, 0.0)]
        }
        Some(false) => {
            let low = beside(v, i, false).min(top);
            vec![(x1, 0.0, 0.0), (x0, 0.0, 0.0), (x0, low, 0.0), (x0, low, l), (x0, top, l), (x1, top, l), (x1, top, 0.0)]
        }
        None => vec![(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, top, 0.0), (x1, top, l), (x0, top, l), (x0, top, 0.0)],
    };
    let at = points(p, &corners);
    // A dark casing under the light line: one of the two is seen on any material, in either theme.
    write!(out, "<polygon class=\"iso-sel-casing\" points=\"{at}\"/><polygon class=\"iso-sel\" points=\"{at}\"/>").unwrap();
}

/// How wide the scene is across: the street, or the pieces where they run over it.
fn span_mm(v: &View) -> i32 {
    v.row_mm.max(v.total_mm)
}

/// How tall the 2.5D of this street is drawn `width` wide, without drawing it.
pub fn iso_height(v: &View, width: f64) -> f64 {
    Projection::fit(width, span_mm(v)).height
}

/// The street in 2.5D, drawn `width` wide: a stretch of `STRETCH_MM` seen in perspective from the centre line,
/// from the same view the section is drawn from. Hovering a piece names it.
///
/// It is built in layers, each its own group, in the order the eye needs: the ground; the surface (the top of every
/// roadway piece); the markings (every line and symbol, painted on the surface); the raised layer (every strip
/// standing above the roadway, with its faces, covering the paint's edges where they meet) together with what stands
/// on the pieces, in the order the eye needs (`order`: a box is painted after the ones it is in front of, nearer the
/// middle of the street, nearer the near end or higher); and last the selected piece's outline.
pub fn iso_svg(v: &View, i18n: &I18n, width: f64) -> IsoSvg {
    let span = span_mm(v);
    let p = Projection::fit(width, span);
    let (w, l) = (span as f64, STRETCH_MM);
    let mut out = format!("<defs>{}{}</defs>", shades(), materials());
    write!(out, "<polygon class=\"iso-ground\" points=\"{}\"/>", points(&p, &[(0.0, 0.0, 0.0), (w, 0.0, 0.0), (w, 0.0, l), (0.0, 0.0, l)])).unwrap();

    let (flat, standing) = surface::split(strips(v, &p));
    out.push_str(&surface::layer(&p, v, i18n, &flat));
    out.push_str(&paint::layer(&p, v));

    // The raised strips and what stands on the pieces, in the order the eye needs (`order`): a box covers the ones it
    // is in front of, so a crown nearer the middle of the street covers a shelter beyond it, and a strip nearer the
    // middle covers the foot of a post beyond it. The strips are in groups of their own between the solids.
    let mut solids: Vec<Block> = (0..v.segments.len()).flat_map(|i| blocks(v, i, &p)).collect();
    solids.sort_by(|a, b| b.from_eye(&p).total_cmp(&a.from_eye(&p)).then(a.y.0.total_cmp(&b.y.0)));
    let boxes: Vec<(Bx, bool)> = standing.iter().map(|&s| (raised::strip_box(v, s), true)).chain(solids.iter().map(|b| (b.bounds(), false))).collect();
    out.push_str("<g class=\"iso-raised\">");
    let mut open = true;
    for k in order::paint_order(&boxes, p.centre_mm) {
        if k < standing.len() {
            if !open {
                out.push_str("<g class=\"iso-raised\">");
                open = true;
            }
            raised::piece(&mut out, &p, v, i18n, standing[k]);
        } else {
            if open {
                out.push_str("</g>");
                open = false;
            }
            block(&mut out, &p, &solids[k - standing.len()]);
        }
    }
    if open {
        out.push_str("</g>");
    }
    if let Some(i) = v.selected.and_then(|u| v.segments.iter().position(|s| s.uid == u)) {
        outline(&mut out, &p, v, i);
    }

    IsoSvg { markup: out, width: p.width, height: p.height }
}

/// What a screen reader is told the 2.5D drawing is: the street, the stretch shown, and its pieces.
pub fn iso_label(v: &View, i18n: &I18n, units: Units) -> String {
    let args = Args::new()
        .str("name", say(i18n, units, &v.name))
        .str("length", units.length_whole_in(STRETCH_MM as i32, i18n.locale()))
        .num("count", v.segments.len() as i64);
    i18n.tr("svg-iso-label", &args)
}

#[cfg(test)]
mod tests {
    use super::projection::{BANDS, EYE_BACK_MM, band_edges};
    use super::projection::{PAD, r};
    use super::testing::{count, drawn, en, furnished, marked, one_way_bikes, top_of, xs, xs_ys};
    use super::*;
    use crate::shared::catalogue::{KINDS, MATERIALS, kind_index};
    use crate::shared::i18n::Locale;
    use crate::street::model::Editor;

    /// How far a face rises on the drawing, in pixels: the first polygon of `class` in the `nth` group drawn for `uid`.
    fn rise_of(markup: &str, uid: u32, nth: usize, class: &str) -> f64 {
        let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(nth).expect("the group is drawn");
        let face = &group[group.find(&format!("class=\"{class}")).expect("the face is drawn")..];
        let at = face.find("points=\"").unwrap() + 8;
        let ys: Vec<f64> = face[at..at + face[at..].find('"').unwrap()].split(' ').map(|pt| pt.split(',').nth(1).unwrap().parse().unwrap()).collect();
        ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min)
    }

    #[test]
    fn a_rise_is_drawn_three_times_as_high_as_it_stands_so_that_it_reads() {
        assert_eq!(drawn_mm(150), 450.0);
        assert_eq!(drawn_mm(0), 0.0);
        // The tables keep the true heights; only the drawing exaggerates them.
        assert_eq!((height_mm("sidewalk"), curb_height_mm("concrete")), (150, 150));
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 2);
        let v = e.view();
        let scale = Projection::fit(1050.0, v.row_mm.max(v.total_mm)).scale;
        let m = iso_svg(&v, &en(), 1050.0).markup;
        // The sidewalk's front face is exaggerated; the front face of the bike lane's first curb block (its slab is the
        // first group), a divider standing above a lane at the roadway's level, is drawn true (`curb_top_mm`).
        let sidewalk = v.segments[0].uid;
        assert!((rise_of(&m, sidewalk, 1, "iso-front ") - 450.0 * scale).abs() < 0.02, "{}", rise_of(&m, sidewalk, 1, "iso-front "));
        assert!((rise_of(&m, bike, 2, "iso-curb") - 150.0 * scale).abs() < 0.02, "{}", rise_of(&m, bike, 2, "iso-curb"));
    }

    #[test]
    fn a_known_street_has_one_top_face_per_piece_each_in_its_tint_and_hatch() {
        let e = Editor::new(0);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-top "), 6);
        assert_eq!(count(&m, "class=\"iso-piece\""), 6 + 2, "a group per piece and per curb");
        assert_eq!(count(&m, "class=\"iso-top k-sidewalk mat-"), 2);
        assert_eq!(count(&m, "fill=\"url(#iso-m-asphalt)\""), 4 * BANDS, "BANDS per travel lane and parking lane, in the material's pattern");
        assert_eq!(count(&m, "fill=\"url(#h-travel)\""), 0, "the kind's hatch stays on the faces");
        assert_eq!(count(&m, "class=\"iso-ground\""), 1);
        assert!(!m.contains("NaN"));
    }

    /// The class attribute of the top face of the piece `uid`.
    fn top_class(markup: &str, uid: u32) -> String {
        let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(1).expect("the piece is drawn");
        let at = group.find("class=\"iso-top ").expect("a top face") + 7;
        group[at..at + group[at..].find('"').unwrap()].to_string()
    }

    #[test]
    fn every_top_is_in_its_pieces_material_with_the_material_s_pattern_over_it() {
        let e = Editor::new(0);
        let v = e.view();
        let m = drawn(&e);
        for s in &v.segments {
            let class = top_class(&m, s.uid);
            assert!(class.contains(&format!("k-{} mat-{}", KINDS[s.kind].id, s.material)), "{class}");
        }
        // The pattern is the material's, and the kind's hatch is no longer on a top.
        let sidewalk = v.segments[0].uid;
        let group = m.split(&format!("<g class=\"iso-piece\" data-uid=\"{sidewalk}\">")).nth(1).unwrap();
        let top = &group[group.find("class=\"iso-top ").unwrap()..];
        assert!(top.contains(&format!("fill=\"url(#iso-m-{})\"", v.segments[0].material)));
        assert!(!top.split("</g>").next().unwrap().contains("url(#h-sidewalk)") || top.contains("iso-front"), "only the faces keep the kind's hatch");
    }

    #[test]
    fn a_piece_whose_material_changes_changes_its_top_and_nothing_else_of_its_class() {
        let mut e = Editor::new(0);
        let uid = e.view().segments[0].uid;
        let kind = e.view().segments[0].kind;
        let before = drawn(&e);
        let other =
            KINDS[kind].materials.iter().copied().find(|&i| MATERIALS[i].id != e.view().segments[0].material).expect("a sidewalk has a second material");
        assert!(e.set_material(uid, other));
        let after = drawn(&e);
        assert_ne!(top_class(&before, uid), top_class(&after, uid));
        assert!(top_class(&after, uid).ends_with(&format!("mat-{}", MATERIALS[other].id)));
        assert!(after.contains(&format!("fill=\"url(#iso-m-{})\"", MATERIALS[other].id)));
        // the faces keep the kind's tint
        assert_eq!(count(&before, "class=\"iso-front k-sidewalk\""), count(&after, "class=\"iso-front k-sidewalk\""));
    }

    /// The bands of the top of the piece `uid` (its `iso-m-*` polygons): each as its `matrix(..)` and its points.
    fn bands(markup: &str, uid: u32) -> Vec<([f64; 6], Vec<(f64, f64)>)> {
        let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(1).expect("the piece is drawn");
        let group = &group[..group.find("</g>").unwrap()];
        group
            .split("<polygon class=\"hatch\" transform=\"matrix(")
            .skip(1)
            .filter(|b| b.contains("url(#iso-m-"))
            .map(|b| {
                let m: Vec<f64> = b[..b.find(')').unwrap()].split(' ').map(|n| n.parse().unwrap()).collect();
                let pts = &b[b.find("points=\"").unwrap() + 8..];
                let pts = pts[..pts.find('"').unwrap()]
                    .split(' ')
                    .map(|pt| {
                        let mut c = pt.split(',').map(|n| n.parse::<f64>().unwrap());
                        (c.next().unwrap(), c.next().unwrap())
                    })
                    .collect();
                ([m[0], m[1], m[2], m[3], m[4], m[5]], pts)
            })
            .collect()
    }

    #[test]
    fn the_patterns_are_drawn_in_millimetres_of_ground_and_each_band_of_a_top_lays_one_by_its_own_matrix() {
        let e = Editor::new(0);
        let v = e.view();
        let p = Projection::fit(1050.0, span_mm(&v));
        let m = drawn(&e);
        // The patterns carry no projection: they are in ground millimetres, the identity.
        for mat in MATERIALS {
            let at = m.find(&format!("id=\"iso-m-{}\"", mat.id)).expect("every material has a pattern");
            let start = m[..at].rfind("<pattern").unwrap();
            let tag = &m[start..start + m[start..].find('>').unwrap()];
            assert!(tag.contains("patternTransform=\"matrix(1.000000 0.000000 0.000000 1.000000 0.00 0.00)\""), "{tag}");
        }
        // Each top is cut into BANDS bands, whose polygons, laid by their matrices, are exactly the perspective's
        // quadrilaterals of the band (so they fit the top and each other), and whose matrices are the slope at the band's depth.
        let edges = band_edges();
        for s in &v.segments {
            let h = drawn_mm(height_mm(KINDS[s.kind].id));
            let bands = bands(&m, s.uid);
            assert_eq!(bands.len(), BANDS, "{}", KINDS[s.kind].id);
            let (x0, x1) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64);
            for (k, (t, local)) in bands.iter().enumerate() {
                let want = [(x0, edges[k]), (x1, edges[k]), (x1, edges[k + 1]), (x0, edges[k + 1])];
                for ((u, vv), (x, z)) in local.iter().zip(want) {
                    let got = (t[0] * u + t[2] * vv + t[4], t[1] * u + t[3] * vv + t[5]);
                    let (sx, sy) = p.at(x, h, z);
                    assert!((got.0 - sx).abs() < 0.1 && (got.1 - sy).abs() < 0.1, "{k}: {got:?} {sx} {sy}");
                }
                // The size across is the projection's scale at the band's middle in depth, which lies in the band.
                let depth = ((t[0] / p.scale).recip() * EYE_BACK_MM) - EYE_BACK_MM;
                assert!(depth > edges[k] && depth < edges[k + 1], "{k}: {depth}");
                assert_eq!((t[1], t[3] < 0.0), (0.0, true), "across stays horizontal, along goes up the screen");
            }
        }
    }

    #[test]
    fn a_raised_piece_has_a_front_face_and_a_side_face_where_its_neighbour_is_lower_and_the_roadway_has_neither() {
        let m = drawn(&Editor::new(0));
        assert_eq!(count(&m, "class=\"iso-front "), 2);
        assert_eq!(count(&m, "class=\"iso-front k-sidewalk\""), 2);
        assert_eq!(count(&m, "class=\"iso-side "), 2);
        assert_eq!(count(&m, "class=\"iso-side k-sidewalk\""), 2);
        for road in ["travel", "parking"] {
            assert_eq!(count(&m, &format!("iso-front k-{road}")) + count(&m, &format!("iso-side k-{road}")), 0, "{road}");
        }
        // Sample Avenue 2: a sidewalk beside a planting strip of the same height has no side face; each planting strip,
        // beside parking, shows its inner one; the median is over the eye, so it shows none.
        let avenue = drawn(&Editor::new(1));
        assert_eq!(count(&avenue, "class=\"iso-front "), 5);
        assert_eq!(count(&avenue, "class=\"iso-side "), 2);
    }

    /// The `points` of the first face of `class` in the first group drawn for `uid` that has one.
    fn face_points(markup: &str, uid: u32, class: &str) -> Option<String> {
        markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).skip(1).find_map(|g| {
            let g = &g[..g.find("</g>")?];
            let face = &g[g.find(&format!("class=\"{class}"))?..];
            let at = face.find("points=\"")? + 8;
            Some(face[at..at + face[at..].find('"')?].to_string())
        })
    }

    #[test]
    fn from_the_middle_the_left_sidewalk_shows_its_right_face_and_the_right_one_its_left_and_both_their_front() {
        let e = Editor::new(0);
        let v = e.view();
        let p = Projection::fit(1050.0, span_mm(&v));
        let m = drawn(&e);
        let (left, right) = (&v.segments[0], &v.segments[5]);
        let h = drawn_mm(150);
        let (lx, rx) = ((left.x_mm + left.width_mm) as f64, right.x_mm as f64);
        assert_eq!(face_points(&m, left.uid, "iso-side").unwrap(), points(&p, &[(lx, 0.0, 0.0), (lx, 0.0, STRETCH_MM), (lx, h, STRETCH_MM), (lx, h, 0.0)]));
        assert_eq!(face_points(&m, right.uid, "iso-side").unwrap(), points(&p, &[(rx, 0.0, 0.0), (rx, 0.0, STRETCH_MM), (rx, h, STRETCH_MM), (rx, h, 0.0)]));
        // The near face shows for both.
        for s in [left, right] {
            let (x0, x1) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64);
            assert_eq!(face_points(&m, s.uid, "iso-front").unwrap(), points(&p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, h, 0.0), (x0, h, 0.0)]));
        }
        // The sidewalks' inner faces are seen from the middle of the street, so the left one is a mirror of the right.
        let (l, r) = (xs_ys(&face_points(&m, left.uid, "iso-side").unwrap()), xs_ys(&face_points(&m, right.uid, "iso-side").unwrap()));
        for ((lx, ly), (rx, ry)) in l.iter().zip(&r) {
            assert!((lx + rx - p.width).abs() < 0.02 && (ly - ry).abs() < 0.02, "{l:?} {r:?}");
        }
    }

    #[test]
    fn a_piece_right_of_the_middle_is_outlined_along_its_left_face() {
        let mut e = Editor::new(0);
        let (uid, x0) = (e.view().segments[5].uid, e.view().segments[5].x_mm);
        e.select(Some(uid));
        let m = drawn(&e);
        let v = e.view();
        let p = Projection::fit(1050.0, span_mm(&v));
        let (x1, h) = ((x0 + v.segments[5].width_mm) as f64, drawn_mm(150));
        let (sx, sy) = p.at(x1, 0.0, 0.0);
        assert!(m.contains(&format!("<polygon class=\"iso-sel\" points=\"{},{} ", r(sx), r(sy))), "starts at the near right corner");
        let (fx, fy) = p.at(x0 as f64, h, STRETCH_MM);
        assert!(m.contains(&format!("{},{} ", r(fx), r(fy))), "goes up the left face's far edge");
    }

    #[test]
    fn a_buffer_is_drawn_at_the_roadway_s_level_with_no_face_no_curb_and_no_symbol() {
        let mut e = Editor::new(0);
        e.add(kind_index("buffer").unwrap(), 3);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-top k-buffer mat-"), 1);
        assert_eq!(count(&m, "iso-front k-buffer") + count(&m, "iso-side k-buffer"), 0, "it stands no higher than the roadway");
        assert_eq!(count(&m, "class=\"iso-curb\""), 2, "only the two sidewalks' curbs");
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn the_roadway_is_drawn_before_the_raised_pieces_so_they_stand_in_front_of_it() {
        let m = drawn(&Editor::new(0));
        assert!(m.rfind("iso-top k-travel").unwrap() < m.find("iso-top k-sidewalk").unwrap());
        assert!(m.find("class=\"iso-ground\"").unwrap() < m.find("class=\"iso-top ").unwrap());
    }

    #[test]
    fn a_sidewalk_s_curb_is_a_stripe_on_its_edge_and_a_bike_lane_s_curb_is_a_block_standing_above_it() {
        let mut e = Editor::new(0);
        let m = drawn(&e);
        // The two sidewalks' concrete curbs are as high as the sidewalk: a top stripe each.
        assert_eq!(count(&m, "class=\"iso-curb\""), 2);
        assert_eq!(count(&m, "fill=\"url(#c-concrete)\""), 2);
        // A bike lane (its curb is concrete by default) between parking and a driving lane: a block on each edge,
        // each with a front, a side and a top.
        e.add(kind_index("bike").unwrap(), 2);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-curb\""), 2 + 2 * 3);
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn the_height_of_the_2_5d_is_known_without_drawing_it() {
        let mut e = Editor::new(0);
        e.add(kind_index("travel").unwrap(), 2);
        for e in [Editor::new(0), Editor::new(1), e] {
            for width in [400.0, 600.0, 760.0, 1050.0, 1440.0] {
                assert_eq!(iso_height(&e.view(), width), iso_svg(&e.view(), &en(), width).height, "{width}");
            }
        }
    }

    #[test]
    fn a_curb_s_faces_say_which_face_each_is_so_that_the_turned_ones_are_shaded() {
        let mut e = Editor::new(0);
        e.add(kind_index("bike").unwrap(), 2);
        let m = drawn(&e);
        // The sidewalks' two stripes are tops; the bike lane's two blocks have a top, a front and a side each.
        assert_eq!(count(&m, "class=\"iso-curb\" data-face=\"top\""), 2 + 2);
        assert_eq!(count(&m, "class=\"iso-curb\" data-face=\"front\""), 2);
        assert_eq!(count(&m, "class=\"iso-curb\" data-face=\"side\""), 2);
    }

    #[test]
    fn every_curb_a_piece_can_have_draws_in_2_5d() {
        let mut e = Editor::new(0);
        let u = e.add(kind_index("bike").unwrap(), 2);
        for c in 0..crate::shared::catalogue::CURBS.len() {
            if e.set_curb(u, Some(c)) {
                assert!(!drawn(&e).contains("NaN"), "curb {c}");
            }
        }
    }

    #[test]
    fn the_2_5d_follows_an_edit_a_wider_piece_moves_the_next_one() {
        let mut e = Editor::new(0);
        let (first, second) = (e.view().segments[0].uid, e.view().segments[1].uid);
        let before = top_of(&drawn(&e), second);
        assert!(e.nudge_width(first, -500));
        assert_ne!(top_of(&drawn(&e), second), before);
    }

    #[test]
    fn two_different_streets_give_different_drawings() {
        // Each drawing has a top face for each of its own pieces, in its kind's tint, so two streets made of
        // other pieces draw differently, and not only in some number.
        let tops = |e: &Editor| {
            let m = drawn(e);
            let mut kinds: Vec<String> =
                m.split("class=\"iso-top k-").skip(1).map(|rest| rest.split('"').next().unwrap().split(' ').next().unwrap().to_string()).collect();
            kinds.sort();
            kinds
        };
        let pieces = |e: &Editor| {
            let mut kinds: Vec<String> = e.view().segments.iter().map(|s| KINDS[s.kind].id.to_string()).collect();
            kinds.sort();
            kinds
        };
        let (a, b) = (Editor::new(0), Editor::new(1));
        assert_eq!(tops(&a), pieces(&a));
        assert_eq!(tops(&b), pieces(&b));
        assert_ne!(tops(&a), tops(&b));
        assert_ne!(drawn(&a), drawn(&b));
    }

    #[test]
    fn a_design_wider_than_the_street_is_drawn_whole_inside_the_drawing() {
        let mut e = Editor::new(0);
        let u = e.view().segments[0].uid;
        assert!(e.set_width(u, 8_000));
        assert!(e.view().delta_mm > 0, "the design runs over the street");
        let s = iso_svg(&e.view(), &en(), 1050.0);
        let all = xs(&s.markup);
        assert!(all.iter().all(|x| *x >= 0.0 && *x <= s.width), "{:?}", all.iter().cloned().fold(f64::MIN, f64::max));
        assert_eq!(count(&s.markup, "class=\"iso-top "), 6);
    }

    #[test]
    fn a_street_with_no_pieces_is_the_ground_alone() {
        let mut e = Editor::new(0);
        while let Some(u) = e.view().segments.first().map(|s| s.uid) {
            assert!(e.remove(u));
        }
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-ground\""), 1);
        assert_eq!(count(&m, "class=\"iso-top "), 0);
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn the_names_a_piece_shows_on_hover_are_in_the_language_of_the_page() {
        let e = Editor::new(0);
        let i18n = en();
        assert!(iso_svg(&e.view(), &i18n, 1050.0).markup.contains("<title>Sidewalk</title>"));
        i18n.set(Locale::FrCa);
        let fr = iso_svg(&e.view(), &i18n, 1050.0).markup;
        assert!(fr.contains("<title>Trottoir</title>") && !fr.contains(">Sidewalk<"), "the French drawing names the sidewalk in French");
    }

    #[test]
    fn every_sample_street_draws_in_2_5d_without_a_missing_number_at_every_width() {
        for sample in 0..4 {
            for width in [600.0, 760.0, 1050.0, 1800.0] {
                let s = iso_svg(&Editor::new(sample).view(), &en(), width);
                assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "sample {sample} at {width}");
                assert!(s.height <= MAX_HEIGHT + 1e-9);
            }
        }
    }

    #[test]
    fn the_farthest_crown_stays_inside_the_drawing_at_every_width() {
        for width in [600.0, 760.0, 1050.0, 1800.0] {
            let s = iso_svg(&Editor::new(1).view(), &en(), width);
            let top = xs_ys(&s.markup).into_iter().map(|(_, y)| y).fold(f64::MAX, f64::min);
            assert!(top >= PAD - 1e-9, "{width}: {top}");
        }
    }

    #[test]
    fn the_selected_piece_is_outlined_and_no_other() {
        let mut e = Editor::new(0);
        assert_eq!(count(&drawn(&e), "class=\"iso-sel\""), 0);
        let (uid, x_mm) = (e.view().segments[2].uid, e.view().segments[2].x_mm);
        e.select(Some(uid));
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-sel\""), 1);
        // The outline starts at the selected piece's near left corner on the ground.
        let p = Projection::fit(1050.0, 18_000);
        let (x, y) = p.at(x_mm as f64, 0.0, 0.0);
        assert!(m.contains(&format!("<polygon class=\"iso-sel\" points=\"{},{} ", r(x), r(y))), "the drawing has not got it");
        // Drawn last, over everything.
        assert!(m.rfind("class=\"iso-sel\"").unwrap() > m.rfind("class=\"iso-piece\"").unwrap());
    }

    #[test]
    fn a_selection_of_no_piece_draws_no_outline_and_a_one_piece_street_draws_whole() {
        let mut v = Editor::new(0).view();
        v.selected = Some(9_999);
        assert_eq!(count(&iso_svg(&v, &en(), 1050.0).markup, "class=\"iso-sel\""), 0);
        let mut e = Editor::new(0);
        while e.view().segments.len() > 1 {
            assert!(e.remove(e.view().segments[0].uid));
        }
        e.select(Some(e.view().segments[0].uid));
        let m = drawn(&e);
        assert!(!m.contains("NaN") && !m.contains("inf") && count(&m, "class=\"iso-sel\"") == 1, "the drawing has not got it");
    }

    #[test]
    fn the_outline_of_a_selected_bike_lane_takes_in_its_curb_blocks() {
        // A bike lane's slab and its two curb blocks are three groups with one uid: the outline goes round all of them.
        let mut e = Editor::new(0);
        let u = e.add(kind_index("bike").unwrap(), 2);
        e.select(Some(u));
        let m = drawn(&e);
        let groups: Vec<&str> = m.split(&format!("<g class=\"iso-piece\" data-uid=\"{u}\">")).skip(1).map(|g| &g[..g.find("</g>").unwrap()]).collect();
        assert_eq!(groups.len(), 3);
        let sel = &m[m.find("class=\"iso-sel\"").unwrap()..];
        let sel = xs_ys(&sel[..sel.find("/>").unwrap()]);
        let (lo_x, hi_x) = sel.iter().fold((f64::MAX, f64::MIN), |(a, b), (x, _)| (a.min(*x), b.max(*x)));
        let (lo_y, hi_y) = sel.iter().fold((f64::MAX, f64::MIN), |(a, b), (_, y)| (a.min(*y), b.max(*y)));
        for g in groups {
            for (x, y) in xs_ys(g) {
                assert!(x >= lo_x - 1e-9 && x <= hi_x + 1e-9 && y >= lo_y - 1e-9 && y <= hi_y + 1e-9, "({x},{y}) outside {sel:?}");
            }
        }
    }

    #[test]
    fn the_faces_turned_from_the_eye_are_shaded_by_the_drawing_s_own_filters_not_by_css() {
        // CSS `filter: brightness()` on SVG shapes may not be applied in Safari; an SVG filter is SVG 1.1.
        let m = drawn(&furnished());
        for (id, slope) in [("iso-shade-front", "0.9"), ("iso-shade-side", "0.78")] {
            let at = m.find(&format!("<filter id=\"{id}\"")).unwrap_or_else(|| panic!("the drawing has no {id} filter"));
            let f = &m[at..at + m[at..].find("</filter>").unwrap()];
            assert!(f.contains("color-interpolation-filters=\"sRGB\""), "the {id} filter");
            assert_eq!(count(f, &format!("type=\"linear\" slope=\"{slope}\"")), 3, "the {id} filter");
        }
        assert!(m.find("<defs>").unwrap() < m.find("class=\"iso-ground\"").unwrap());
        let front = count(&m, "class=\"iso-front ") + count(&m, "data-face=\"front\"") + count(&m, "class=\"o-front\"");
        let side = count(&m, "class=\"iso-side ") + count(&m, "data-face=\"side\"") + count(&m, "class=\"o-side\"");
        assert!(front > 0 && side > 0);
        assert_eq!(count(&m, "filter=\"url(#iso-shade-front)\""), front);
        assert_eq!(count(&m, "filter=\"url(#iso-shade-side)\""), side);
        let css = include_str!("../../../web/editor.css");
        assert!(!css.contains("brightness(0.9)") && !css.contains("brightness(0.78)"));
    }

    #[test]
    fn the_2_5d_is_labelled_with_the_street_the_stretch_and_its_pieces_in_each_language() {
        let v = Editor::new(0).view();
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, along a 40 m stretch. 6 segments.");
        assert_eq!(iso_label(&v, &en(), Units::Feet), "Sample Street 1 in 2.5D, along a 131 ft stretch. 6 segments.");
        let fr = crate::i18n_for(Locale::FrCa);
        assert_eq!(iso_label(&v, &fr, Units::Metres), "Sample Street 1 en 2,5D, sur un tronçon de 40\u{a0}m. 6 éléments.");
    }

    #[test]
    fn the_layers_are_drawn_in_order_ground_surface_markings_raised_solids_and_selection() {
        let mut e = furnished();
        e.select(Some(e.view().segments[0].uid));
        let m = drawn(&e);
        let at = |needle: &str| m.find(needle).unwrap_or_else(|| panic!("no {needle}"));
        let order = [
            at("class=\"iso-ground\""),
            at("<g class=\"iso-surface\">"),
            at("<g class=\"iso-markings\">"),
            at("<g class=\"iso-raised\">"),
            at("class=\"iso-solid"),
            at("class=\"iso-sel\""),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        assert!(m.find("<defs>").unwrap() < order[0]);
    }

    #[test]
    fn the_surface_layer_holds_the_top_of_every_roadway_piece_and_the_raised_layer_the_rest() {
        let e = furnished();
        let v = e.view();
        let m = drawn(&e);
        let (surface, markings, raised) =
            (m.find("<g class=\"iso-surface\">").unwrap(), m.find("<g class=\"iso-markings\">").unwrap(), m.find("<g class=\"iso-raised\">").unwrap());
        let roadway = v.segments.iter().filter(|s| height_mm(KINDS[s.kind].id) == 0).count();
        assert_eq!(count(&m[surface..markings], "class=\"iso-top "), roadway);
        assert_eq!(count(&m[raised..], "class=\"iso-top "), v.segments.len() - roadway);
        assert!(!m[surface..markings].contains("k-sidewalk") && !m[surface..markings].contains("iso-curb"));
        // Each keeps its group, so a selection and the e2e selectors still find it.
        for s in &v.segments {
            assert!(m.contains(&format!("<g class=\"iso-piece\" data-uid=\"{}\">", s.uid)));
        }
    }

    #[test]
    fn the_paint_is_between_the_surface_and_the_raised_pieces_so_a_higher_neighbour_covers_it() {
        let (e, _, kerbside) = one_way_bikes();
        let v = e.view();
        let m = drawn(&e);
        let (markings, raised) = (m.find("<g class=\"iso-markings\">").unwrap(), m.find("<g class=\"iso-raised\">").unwrap());
        assert_eq!(count(&m[markings..raised], "iso-paint"), count(&m, "iso-paint"), "every bit of paint is in the markings layer");
        // The kerbside lane's right neighbour, the raised sidewalk, and its curb blocks are drawn after the paint.
        let pos = v.segments.iter().position(|s| s.uid == kerbside).unwrap();
        let sidewalk = v.segments[pos + 1].uid;
        assert_eq!(KINDS[v.segments[pos + 1].kind].id, "sidewalk");
        assert!(m.find(&format!("<g class=\"iso-piece\" data-uid=\"{sidewalk}\">")).unwrap() > raised);
        let curbs: Vec<usize> = m.match_indices(&format!("<g class=\"iso-piece\" data-uid=\"{kerbside}\">")).skip(1).map(|(at, _)| at).collect();
        assert!(!curbs.is_empty() && curbs.iter().all(|c| *c > raised));
    }

    #[test]
    fn a_known_street_has_a_known_number_of_paint_quads() {
        // Sample Street 1: a solid white line beside each parking lane (1 quad each), a double yellow one between the
        // lanes (2 quads), and an arrow on each driving lane.
        let m = drawn(&Editor::new(0));
        assert_eq!(count(&m, "class=\"iso-paint yellow\""), 2);
        assert_eq!(count(&m, "class=\"iso-paint white\""), 4);
        assert_eq!(count(&m, "class=\"iso-mark\""), 2);
        // The paint never carries the classes the shading and the counting of faces go by.
        for tag in m.split("<polygon ").filter(|t| t.starts_with("class=\"iso-paint")) {
            let head = &tag[..tag.find('>').unwrap()];
            assert!(
                !head.contains("iso-top")
                    && !head.contains("iso-front")
                    && !head.contains("iso-side")
                    && !head.contains("data-face")
                    && !head.contains("filter="),
                "{head}"
            );
        }
    }

    #[test]
    fn symbols_replace_the_arrows_on_driving_bus_and_bike_lanes() {
        for e in [Editor::new(0), Editor::new(1), Editor::new(3), furnished(), one_way_bikes().0] {
            let m = drawn(&e);
            assert_eq!(count(&m, "class=\"iso-dir\""), 0);
            assert_eq!(count(&m, "class=\"dir\""), 0);
            assert_eq!(count(&m, "class=\"dir-halo\""), 0);
        }
        // One symbol per lane that has one: furnished() has four driving lanes and a two-way bike lane.
        assert_eq!(count(&drawn(&furnished()), "class=\"iso-mark\""), 5);
        assert_eq!(count(&drawn(&one_way_bikes().0), "class=\"iso-mark\""), 4);
        let mut e = Editor::new(0);
        let bus = e.add(kind_index("bus").unwrap(), 3);
        assert!(drawn(&e).contains(&format!("data-mark=\"{bus}\"")));
    }

    #[test]
    fn paint_stays_inside_the_drawing_when_the_design_is_wider_than_the_street() {
        let mut e = Editor::new(0);
        let u = e.view().segments[0].uid;
        assert!(e.set_width(u, 8_000));
        assert!(e.view().delta_mm > 0);
        for width in [600.0, 1050.0, 1800.0] {
            let s = iso_svg(&e.view(), &en(), width);
            assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "{width}");
            let m = &s.markup[s.markup.find("<g class=\"iso-markings\">").unwrap()..s.markup.find("<g class=\"iso-raised\">").unwrap()];
            assert!(count(m, "iso-paint") > 0);
            for (x, y) in xs_ys(m) {
                assert!(x >= 0.0 && x <= s.width && y >= 0.0 && y <= s.height, "{width}: ({x}, {y})");
            }
        }
    }

    #[test]
    fn the_marked_street_draws_every_kind_of_paint() {
        let m = drawn(&marked());
        // `marked()` is: sidewalk, parking, driving lane, bus lane (with the traffic), driving lane, parking, two-way bike
        // lane, sidewalk. By the markings rules the polygons are, yellow: the double yellow between the bus lane and
        // the lane running the other way (2) and the bike lane's broken middle line (5 dashes in 40 m) = 7; white: a
        // solid line between each parking lane and its driving lane (2), the double white between the first lane and
        // the bus lane (2), the two lane arrows (2), the bus diamond and "BUS" (15) and the bike symbol (28: two
        // 12-piece wheels, frame, handlebar, two small arrows) = 49. The bike lane's own curb leaves its edges unpainted.
        assert_eq!(count(&m, "class=\"iso-paint yellow\""), 7);
        assert_eq!(count(&m, "class=\"iso-paint white\""), 49);
        assert!(!m.contains("NaN") && !m.contains("inf"));
    }

    /// Not a check: prints how long the markup of some streets is, at 1050 and 600 px.
    /// `cargo test street::iso::tests::markup_size -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn markup_size() {
        for (name, e) in
            [("sample 0", Editor::new(0)), ("sample 1", Editor::new(1)), ("sample 3", Editor::new(3)), ("furnished", furnished()), ("marked", marked())]
        {
            for width in [1050.0, 600.0] {
                println!("{name} at {width}: {} bytes", iso_svg(&e.view(), &en(), width).markup.len());
            }
        }
    }

    /// Not a check: writes `target/iso-preview-light.html` and `target/iso-preview-dark.html`, the 2.5D of
    /// every sample street and of `furnished()` at three widths with the page's styles and hatches, to look at
    /// in a browser. `cargo test street::iso::tests::preview -- --ignored`.
    #[test]
    #[ignore]
    fn preview() {
        use crate::shared::symbols::{CURB_HATCH, HATCH, MATERIAL_HATCH};
        let root = env!("CARGO_MANIFEST_DIR");
        let defs: String = HATCH.iter().chain(MATERIAL_HATCH.iter()).chain(CURB_HATCH.iter()).map(|(_, s)| *s).collect();
        let streets: Vec<(String, Editor)> = (0..4)
            .map(|i| (format!("sample {i}"), Editor::new(i)))
            .chain([("furnished".to_string(), furnished()), ("one-way bikes".to_string(), one_way_bikes().0), ("marked".to_string(), marked())])
            .collect();
        let mut body = String::new();
        for (name, e) in &streets {
            for width in [1050.0, 760.0, 600.0] {
                let s = iso_svg(&e.view(), &en(), width);
                body += &format!(
                    "<figure><figcaption>{name}, {width} px</figcaption><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">{m}</svg></figure>",
                    w = s.width,
                    h = s.height,
                    m = s.markup
                );
            }
        }
        for theme in ["light", "dark"] {
            let html = format!(
                "<!doctype html><html data-theme=\"{theme}\"><head><meta charset=\"utf-8\"><link rel=\"stylesheet\" href=\"file://{root}/web/app.css\"><link rel=\"stylesheet\" href=\"file://{root}/web/editor.css\"><style>body{{background:var(--panel);color:var(--ink);font:14px sans-serif}}figure{{margin:16px}}</style></head><body><svg width=\"0\" height=\"0\" style=\"position:absolute\"><defs>{defs}</defs></svg>{body}</body></html>"
            );
            std::fs::write(format!("{root}/target/iso-preview-{theme}.html"), html).unwrap();
        }
    }
}
