//! What stands above the roadway: the heights of the pieces and of their curbs, and the slabs and curb blocks drawn
//! with their front and side faces.

use std::fmt::Write;

use super::curbside::Over;
use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::esc;
use crate::shared::iso::ground::{fine_patterns, joints};
use crate::shared::iso::shading::shade_filters;
use crate::street::model::SegView;
use crate::street::pieces::{curb_edges, curb_width_mm};

use super::curbside;
use super::joints::path as joint_path;
use super::order::Bx;
use super::projection::{Projection, Reach, band_edges, band_mid, drawn_mm, points};

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

/// The surface materials' fine patterns for the drawing's `<defs>`: the grain, and not the regular joints of the
/// materials that have them, which are geometry (`joints`). They are drawn in millimetres of ground with no transform
/// of their own: a textured top is laid by the matrix of the polygon that fills with one (`top`), band by band.
pub(super) fn materials() -> String {
    fine_patterns("iso-m", [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
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

/// The top of a piece at `h` (drawn mm) across `x0` to `x1`: a polygon in the tint of the kind `id` and the material
/// `m`, the material's texture over it, and its regular joints (`joints`) over that. The texture is the fine grain, a
/// pattern in millimetres of ground; perspective is not an affine map, so the top is cut into `BANDS` bands along the
/// street (`band_edges`), each carrying the affine map that is the perspective's slope at its middle
/// (`Projection::ground_at`); the band's polygon is written in the ground's own millimetres, as the true quadrilateral
/// the band is on the screen taken back through that map, so it fits the top exactly and its `transform` lays the
/// texture. The grain is not continuous across a seam, which does not show in a grain; the joints, which the eye
/// follows, are not part of the pattern but lines projected point by point, so they have no seam.
fn top(out: &mut String, p: &Projection, id: &str, m: &str, x0: f64, x1: f64, h: f64) {
    top_over(out, p, id, m, h, &|_| (x0, x1), true);
}

/// The same top with its edges across given along the street (`edge`: across `(left, right)` at an `along_mm`), as a
/// transition makes them. The edges are straight in between, so each band is a quadrilateral of its own. The joints
/// of a material are laid only on a top with straight edges (`joints`).
pub(super) fn top_over(out: &mut String, p: &Projection, id: &str, m: &str, h: f64, edge: &dyn Fn(f64) -> (f64, f64), joints_too: bool) {
    let (n, l) = (p.reach.near, p.reach.far);
    let ((a0, a1), (b0, b1)) = (edge(n), edge(l));
    let (x0, x1) = (a0, a1);
    let pts = points(p, &[(a0, h, n), (a1, h, n), (b1, h, l), (b0, h, l)]);
    write!(out, "<polygon class=\"iso-top k-{id} mat-{m}\" points=\"{pts}\"/>").unwrap();
    // The bands share their fill, and so the group carries it: a texture's polygons are bare.
    write!(out, "<g class=\"hatch\" fill=\"url(#iso-m-{m})\">").unwrap();
    for w in band_edges(p.reach).windows(2) {
        let (z0, z1) = (w[0], w[1]);
        let (m0, m1) = edge(band_mid(z0, z1));
        let g = p.ground_at(h, (m0 + m1) / 2.0, band_mid(z0, z1));
        // The numbers as written, so that the polygon is taken back through exactly the map that lays the texture.
        let [a, _, c, d, e, f] = [g[0], g[1], g[2], g[3], g[4], g[5]].map(|n| (n * 1e6).round() / 1e6);
        let (e, f) = ((e * 100.0).round() / 100.0, (f * 100.0).round() / 100.0);
        let (e0, e1) = (edge(z0), edge(z1));
        let local: Vec<String> = [(e0.0, z0), (e0.1, z0), (e1.1, z1), (e1.0, z1)]
            .iter()
            .map(|&(x, z)| {
                let (sx, sy) = p.at(x, h, z);
                let v = (sy - f) / d;
                let u = (sx - e - c * v) / a;
                format!("{:.0},{:.0}", u, v)
            })
            .collect();
        write!(out, "<polygon transform=\"matrix({a} 0 {c} {d} {e} {f})\" points=\"{}\"/>", local.join(" ")).unwrap();
    }
    out.push_str("</g>");
    // The joints of the materials that have them, as lines over the grain (`joints`).
    if let Some(j) = joints(m).filter(|_| joints_too) {
        let d = joint_path(p, &j, x0, x1, h);
        if !d.is_empty() {
            write!(out, "<path class=\"joints j-{m}\" d=\"{d}\"/>").unwrap();
        }
    }
}

/// The piece at `i` as a strip along the stretch: its top, and where it stands above the ground its
/// front (the cut at the near end) and the side face the eye sees, where the piece beside it is lower.
pub(super) fn slab(out: &mut String, p: &Projection, segs: &[SegView], i: usize) {
    let s = &segs[i];
    slab_of(out, p, segs, i, KINDS[s.kind].id, s.material, (p.reach.near, p.reach.far), None);
}

/// A strip across the piece at `i` as a thing of kind `id` and material `m` over `along` of the reach: the piece's
/// own (`slab`), or a curbside use standing where the piece lies.
/// `edge` gives the piece's edges across at an `along_mm` where a transition moves them (a use standing on a tapering
/// piece follows it).
pub(super) fn slab_of(
    out: &mut String,
    p: &Projection,
    segs: &[SegView],
    i: usize,
    id: &str,
    m: &str,
    along: (f64, f64),
    edge: Option<&dyn Fn(f64) -> (f64, f64)>,
) {
    let s = &segs[i];
    let (n, l) = along;
    let (x0, x1) = edge.map_or((s.x_mm as f64, (s.x_mm + s.width_mm) as f64), |e| e(n));
    let h = drawn_mm(height_mm(id));
    let hatch = format!("h-{id}");
    if h > 0.0 {
        face(out, Face::Front, Some(id), &hatch, &points(p, &[(x0, 0.0, n), (x1, 0.0, n), (x1, h, n), (x0, h, n)]));
        if let Some(right) = inner_face(p, x0, x1) {
            let (far_x0, far_x1) = edge.map_or((x0, x1), |e| e(l));
            let (x, xl, low) = if right { (x1, far_x1, beside(segs, i, right)) } else { (x0, far_x0, beside(segs, i, right)) };
            if low < h {
                face(out, Face::Side, Some(id), &hatch, &points(p, &[(x, low, n), (xl, low, l), (xl, h, l), (x, h, n)]));
            }
        }
    }
    let sub = p.over(Reach { near: n, far: l });
    match edge {
        Some(edge) => top_over(out, &sub, id, m, h, edge, false),
        None => top(out, &sub, id, m, x0, x1, h),
    }
}

/// Where the curb of the piece at `i` along one edge (`right`) lies across the street.
fn curb_span(s: &crate::street::model::SegView, c: &str, right: bool) -> (f64, f64) {
    let cw = curb_width_mm(c).min(s.width_mm as f64 / 2.0);
    let edge = (s.x_mm + s.width_mm) as f64;
    if right { (edge - cw, edge) } else { (s.x_mm as f64, s.x_mm as f64 + cw) }
}

/// The curb of the piece at `i` along one edge (`right`). As high as the piece, it is a stripe on its
/// top; higher (a bike lane's, a boarding island), it is a block with its own front, side and top.
pub(super) fn curb(out: &mut String, p: &Projection, segs: &[SegView], i: usize, right: bool) {
    let s = &segs[i];
    let Some(c) = s.curb else { return };
    let id = KINDS[s.kind].id;
    let (h, n, l) = (drawn_mm(height_mm(id)), p.reach.near, p.reach.far);
    let top = curb_top_mm(c, id);
    let (x0, x1) = curb_span(s, c, right);
    let hatch = format!("c-{c}");
    if top > h {
        face(out, Face::Front, None, &hatch, &points(p, &[(x0, 0.0, n), (x1, 0.0, n), (x1, top, n), (x0, top, n)]));
        if let Some(face_right) = inner_face(p, x0, x1) {
            // A block's face on the piece's own side stands on the piece; its other face stands on what is beside it.
            let (x, low) = (if face_right { x1 } else { x0 }, if face_right == right { beside(segs, i, right) } else { h });
            face(out, Face::Side, None, &hatch, &points(p, &[(x, low, n), (x, low, l), (x, top, l), (x, top, n)]));
        }
    }
    face(out, Face::Top, None, &hatch, &points(p, &[(x0, top, n), (x1, top, n), (x1, top, l), (x0, top, l)]));
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

/// A strip as its own group, with the piece's name for hovering: its slab, or one of its curbs. A piece of the
/// section across the boundary (`across`) carries no uid: uids repeat from one section to the other, and what finds
/// a piece by its uid is the selected zone's.
pub(super) fn piece(out: &mut String, p: &Projection, segs: &[SegView], i18n: &I18n, strip: Strip, across: bool) {
    piece_with(out, p, segs, i18n, strip, across, None, &[]);
}

/// `piece`, its top (a roadway slab at the ground) tapering when `taper` gives its edges along the street.
pub(super) fn piece_with(
    out: &mut String,
    p: &Projection,
    segs: &[SegView],
    i18n: &I18n,
    strip: Strip,
    across: bool,
    taper: Option<&dyn Fn(f64) -> (f64, f64)>,
    shown: &[Over],
) {
    let (_, i, edge) = strip;
    let s = &segs[i];
    let name = i18n.tr(&kind_key(KINDS[s.kind].id), &Args::new());
    if across {
        write!(out, "<g class=\"iso-piece iso-across\"><title>{}</title>", esc(&name)).unwrap();
    } else {
        write!(out, "<g class=\"iso-piece\" data-uid=\"{}\"><title>{}</title>", s.uid, esc(&name)).unwrap();
    }
    match edge {
        None => match (taper, curbside::flat_parts(segs, i, p.reach, shown)) {
            (taper, Some(parts)) => curbside::flat(out, p, segs, i, &parts, taper),
            (Some(edge), None) => top_over(out, p, KINDS[s.kind].id, s.material, drawn_mm(height_mm(KINDS[s.kind].id)), edge, false),
            (None, None) => slab(out, p, segs, i),
        },
        Some(right) => curb(out, p, segs, i, right),
    }
    out.push_str("</g>");
}

/// A roadway piece the host's section lacks, opening in its run: its top in the kind's default material, in a group
/// of its own (`iso-taper`; it carries no uid, no piece of the street is it).
pub(super) fn sliver(out: &mut String, p: &Projection, i18n: &I18n, kind: usize, edge: &dyn Fn(f64) -> (f64, f64)) {
    let k = &KINDS[kind];
    let name = i18n.tr(&kind_key(k.id), &Args::new());
    write!(out, "<g class=\"iso-piece iso-taper\"><title>{}</title>", esc(&name)).unwrap();
    top_over(out, p, k.id, crate::shared::catalogue::MATERIALS[k.materials[0]].id, drawn_mm(height_mm(k.id)), edge, false);
    out.push_str("</g>");
}

/// The strips in the order given, between the group tag `open` and its end.
pub(super) fn group(open: &str, p: &Projection, segs: &[SegView], i18n: &I18n, strips: &[Strip], across: bool, shown: &[Over]) -> String {
    let mut out = String::from(open);
    for &strip in strips {
        piece_with(&mut out, p, segs, i18n, strip, across, None, shown);
    }
    out.push_str("</g>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{KINDS, kind_index};
    use crate::street::iso::projection::{Projection, Reach, r};
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
        let mut out = String::new();
        slab(&mut out, &p, &v.segments, i);
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
