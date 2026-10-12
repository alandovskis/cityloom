//! The street in 2.5D: a straight stretch of it seen from above and to one side, each piece an
//! extruded strip with its height, the few things that stand on the pieces as plain solids, and the
//! one-way arrows on the roadway. It is drawn from the same view the section is and is read-only:
//! the section is where the street is edited.
//!
//! The rises are exaggerated: a curb or a sidewalk stands 150 mm, a few pixels at the scale a whole
//! street is drawn at, so the slabs and curbs are drawn `RISE` times as high as they stand (the height
//! tables keep the true heights). The things that stand on the pieces keep their own size and sit on the
//! exaggerated tops.

use std::fmt::Write;

use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::street::model::View;
use crate::street::svg::{curb_edges, curb_width_mm, esc};

/// How long a stretch of the street is shown, in millimetres.
pub const STRETCH_MM: f64 = 40_000.0;
/// How far a millimetre along the street moves the drawing right, and up: half scale, at the
/// angle of a 3-4-5 triangle (about 37 degrees), so the points it gives are exact.
pub const RECEDE_X: f64 = 0.4;
pub const RECEDE_Y: f64 = 0.3;
/// The tallest the drawing grows, so that it stays about as tall as the section.
pub const MAX_HEIGHT: f64 = 560.0;
/// Room round the scene, in pixels.
const PAD: f64 = 30.0;
/// The narrowest the drawing is laid out for: the section's group is no narrower.
const MIN_WIDTH: f64 = 600.0;
/// Room above the far end of the ground for the tallest solid, a tree's crown at 5 m, standing on a
/// planting strip drawn `RISE` times its height.
const HEADROOM_MM: f64 = 6_000.0;

/// How many times higher than they stand the slabs and curbs are drawn, so that a rise reads.
pub const RISE: f64 = 3.0;

/// How high a rise of `mm` is drawn, in millimetres of the drawing.
pub fn drawn_mm(mm: i32) -> f64 {
    mm as f64 * RISE
}

/// How a point of the street, in millimetres across it, up from the ground and along it from the
/// near end, lands in the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    /// Pixels to the millimetre.
    pub scale: f64,
    /// Where the street's left edge is at its near end, in pixels.
    pub left: f64,
    /// Where the ground is at the near end, in pixels from the top.
    pub base: f64,
    pub width: f64,
    pub height: f64,
}

impl Projection {
    /// The projection for a drawing `width` wide showing a street `span_mm` wide: the scene fills
    /// the width, unless that would make the drawing taller than `MAX_HEIGHT`; then it is centred.
    pub fn fit(width: f64, span_mm: i32) -> Projection {
        let width = width.max(MIN_WIDTH);
        let across = span_mm.max(1) as f64 + STRETCH_MM * RECEDE_X;
        let up = STRETCH_MM * RECEDE_Y + HEADROOM_MM;
        let scale = ((width - 2.0 * PAD) / across).min((MAX_HEIGHT - 2.0 * PAD) / up);
        let left = (width - across * scale) / 2.0;
        let base = PAD + up * scale;
        Projection { scale, left, base, width, height: base + PAD }
    }

    /// Where a point is in the drawing, in pixels.
    pub fn at(&self, x_mm: f64, up_mm: f64, along_mm: f64) -> (f64, f64) {
        (self.left + (x_mm + along_mm * RECEDE_X) * self.scale, self.base - (up_mm + along_mm * RECEDE_Y) * self.scale)
    }
}

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

/// The 2.5D drawn, and the size it is drawn at.
#[derive(Clone, Debug, PartialEq)]
pub struct IsoSvg {
    pub markup: String,
    pub width: f64,
    pub height: f64,
}

/// A pixel value to two decimals, which is all a drawing needs.
fn r(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The points of a polygon whose corners are in millimetres: across, up, along.
fn points(p: &Projection, corners: &[(f64, f64, f64)]) -> String {
    corners
        .iter()
        .map(|&(x, y, z)| {
            let (sx, sy) = p.at(x, y, z);
            format!("{},{}", r(sx), r(sy))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Which face of a strip a polygon is, with the kind whose tint a piece's face takes.
#[derive(Clone, Copy)]
enum Face<'a> {
    Top(&'a str),
    Front(&'a str),
    Side(&'a str),
    Curb,
}

/// One face: a polygon in its tint, then the hatch `hatch` over it.
fn face(out: &mut String, what: Face, hatch: &str, pts: &str) {
    let (name, kind) = match what {
        Face::Top(k) => ("top", Some(k)),
        Face::Front(k) => ("front", Some(k)),
        Face::Side(k) => ("side", Some(k)),
        Face::Curb => ("curb", None),
    };
    let tint = kind.map(|k| format!(" k-{k}")).unwrap_or_default();
    write!(out, "<polygon class=\"iso-{name}{tint}\" points=\"{pts}\"/><polygon class=\"hatch\" points=\"{pts}\" fill=\"url(#{hatch})\"/>").unwrap();
}

/// How high the piece after `i` is drawn, or the ground when `i` is the last.
fn beside(v: &View, i: usize) -> f64 {
    v.segments.get(i + 1).map_or(0.0, |n| drawn_mm(height_mm(KINDS[n.kind].id)))
}

/// The piece at `i` as a strip along the stretch: its top, and where it stands above the ground its
/// front (the cut at the near end) and its right side where the next piece is lower. Its left side
/// faces away from the eye, so it is never seen.
fn slab(out: &mut String, p: &Projection, v: &View, i: usize) {
    let s = &v.segments[i];
    let id = KINDS[s.kind].id;
    let (x0, x1, h, l) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64, drawn_mm(height_mm(id)), STRETCH_MM);
    let hatch = format!("h-{id}");
    if h > 0.0 {
        face(out, Face::Front(id), &hatch, &points(p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, h, 0.0), (x0, h, 0.0)]));
        let low = beside(v, i);
        if low < h {
            face(out, Face::Side(id), &hatch, &points(p, &[(x1, low, 0.0), (x1, low, l), (x1, h, l), (x1, h, 0.0)]));
        }
    }
    face(out, Face::Top(id), &hatch, &points(p, &[(x0, h, 0.0), (x1, h, 0.0), (x1, h, l), (x0, h, l)]));
}

/// The curb of the piece at `i` along one edge (`right`). As high as the piece, it is a stripe on its
/// top; higher (a bike lane's, a boarding island), it is a block with its own front, side and top.
fn curb(out: &mut String, p: &Projection, v: &View, i: usize, right: bool) {
    let s = &v.segments[i];
    let Some(c) = s.curb else { return };
    let id = KINDS[s.kind].id;
    let (h, l) = (drawn_mm(height_mm(id)), STRETCH_MM);
    let top = drawn_mm(curb_height_mm(c).max(height_mm(id)));
    let cw = curb_width_mm(c).min(s.width_mm as f64 / 2.0);
    let edge = (s.x_mm + s.width_mm) as f64;
    let (x0, x1) = if right { (edge - cw, edge) } else { (s.x_mm as f64, s.x_mm as f64 + cw) };
    let hatch = format!("c-{c}");
    if top > h {
        face(out, Face::Curb, &hatch, &points(p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, top, 0.0), (x0, top, 0.0)]));
        let low = if right { beside(v, i) } else { h };
        face(out, Face::Curb, &hatch, &points(p, &[(x1, low, 0.0), (x1, low, l), (x1, top, l), (x1, top, 0.0)]));
    }
    face(out, Face::Curb, &hatch, &points(p, &[(x0, top, 0.0), (x1, top, 0.0), (x1, top, l), (x0, top, l)]));
}

/// The street in 2.5D, drawn `width` wide: a stretch of `STRETCH_MM` seen obliquely, from the same
/// view the section is drawn from. Hovering a piece names it.
///
/// What is drawn is ordered so that nothing needs sorting by depth: the ground, then the strips from
/// the lowest to the highest (a raised piece stands in front of the roadway beside it on both sides),
/// each piece's slab before its curbs.
pub fn iso_svg(v: &View, i18n: &I18n, width: f64) -> IsoSvg {
    let span = v.row_mm.max(v.total_mm);
    let p = Projection::fit(width, span);
    let (w, l) = (span as f64, STRETCH_MM);
    let mut out = String::new();
    write!(out, "<polygon class=\"iso-ground\" points=\"{}\"/>", points(&p, &[(0.0, 0.0, 0.0), (w, 0.0, 0.0), (w, 0.0, l), (0.0, 0.0, l)])).unwrap();

    // (how high it stands, which piece, which edge's curb if it is one)
    let mut strips: Vec<(i32, usize, Option<bool>)> = Vec::new();
    for (i, s) in v.segments.iter().enumerate() {
        let h = height_mm(KINDS[s.kind].id);
        strips.push((h, i, None));
        if let Some(c) = s.curb {
            for right in curb_edges(v, i) {
                strips.push((curb_height_mm(c).max(h), i, Some(right)));
            }
        }
    }
    strips.sort_by_key(|&(top, i, edge)| (top, i, edge.is_some()));
    for (_, i, edge) in strips {
        let s = &v.segments[i];
        let name = i18n.tr(&kind_key(KINDS[s.kind].id), &Args::new());
        write!(out, "<g class=\"iso-piece\" data-uid=\"{}\"><title>{}</title>", s.uid, esc(&name)).unwrap();
        match edge {
            None => slab(&mut out, &p, v, i),
            Some(right) => curb(&mut out, &p, v, i, right),
        }
        out.push_str("</g>");
    }

    IsoSvg { markup: out, width: p.width, height: p.height }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::KINDS;
    use crate::shared::catalogue::kind_index;
    use crate::shared::i18n::{I18n, Locale};
    use crate::street::model::Editor;
    use std::rc::Rc;

    fn en() -> Rc<I18n> {
        crate::i18n_for(Locale::En)
    }

    fn drawn(e: &Editor) -> String {
        iso_svg(&e.view(), &en(), 1050.0).markup
    }

    fn count(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    /// The `points` of the top face of the piece `uid`.
    fn top_of(markup: &str, uid: u32) -> String {
        let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(1).expect("the piece is drawn");
        let face = &group[group.find("class=\"iso-top ").expect("a top face")..];
        let at = face.find("points=\"").unwrap() + 8;
        face[at..at + face[at..].find('"').unwrap()].to_string()
    }

    /// Every x of every polygon's points.
    fn xs(markup: &str) -> Vec<f64> {
        markup
            .split("points=\"")
            .skip(1)
            .flat_map(|rest| rest.split('"').next().unwrap().split(' ').map(|pt| pt.split(',').next().unwrap().parse::<f64>().unwrap()).collect::<Vec<_>>())
            .collect()
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn the_projection_puts_the_corners_of_a_unit_box_exactly_where_the_oblique_view_has_them() {
        // 31 m of street and 16 m of depth across 940 px: 0.02 px to the millimetre; 12 m of depth and 6 m of headroom up.
        let p = Projection::fit(1000.0, 31_000);
        assert!((p.scale - 0.02).abs() < 1e-12, "{}", p.scale);
        assert_eq!((p.left, p.width), (30.0, 1000.0));
        assert!((p.base - 390.0).abs() < 1e-9 && (p.height - 420.0).abs() < 1e-9, "{p:?}");
        // Width runs right at full scale, height runs up, depth runs up and right at half scale.
        assert!(close(p.at(0.0, 0.0, 0.0), (30.0, 390.0)));
        assert!(close(p.at(1_000.0, 0.0, 0.0), (50.0, 390.0)));
        assert!(close(p.at(0.0, 1_000.0, 0.0), (30.0, 370.0)));
        assert!(close(p.at(0.0, 0.0, 1_000.0), (38.0, 384.0)));
        assert!(close(p.at(1_000.0, 1_000.0, 1_000.0), (58.0, 364.0)));
    }

    #[test]
    fn the_scene_fits_the_width_and_a_narrow_street_is_held_to_the_height_and_centred() {
        // A wide street: the width decides, the scene spans the drawing less its margins.
        let wide = Projection::fit(1000.0, 31_000);
        let across = 31_000.0 + STRETCH_MM * RECEDE_X;
        assert!((wide.left + across * wide.scale - (1000.0 - 30.0)).abs() < 1e-9);
        // A narrow street: the height decides, so the drawing is no taller than MAX_HEIGHT, and the scene is centred.
        let narrow = Projection::fit(1000.0, 4_000);
        assert!((narrow.height - MAX_HEIGHT).abs() < 1e-9, "{narrow:?}");
        let across = 4_000.0 + STRETCH_MM * RECEDE_X;
        assert!((narrow.left - (1000.0 - (narrow.left + across * narrow.scale))).abs() < 1e-9, "centred: {narrow:?}");
        // The drawing is laid out no narrower than the section's group.
        assert_eq!(Projection::fit(100.0, 18_000).width, 600.0);
        // A street of no width still gives a scale.
        assert!(Projection::fit(1000.0, 0).scale.is_finite());
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
        // The sidewalk's front face, and the front face of the bike lane's first curb block (its slab is the first group).
        let sidewalk = v.segments[0].uid;
        assert!((rise_of(&m, sidewalk, 1, "iso-front ") - 450.0 * scale).abs() < 0.02, "{}", rise_of(&m, sidewalk, 1, "iso-front "));
        assert!((rise_of(&m, bike, 2, "iso-curb") - 450.0 * scale).abs() < 0.02, "{}", rise_of(&m, bike, 2, "iso-curb"));
    }

    #[test]
    fn a_known_street_has_one_top_face_per_piece_each_in_its_tint_and_hatch() {
        let e = Editor::new(0);
        let m = drawn(&e);
        assert_eq!(count(&m, "class=\"iso-top "), 6);
        assert_eq!(count(&m, "class=\"iso-piece\""), 6 + 2, "a group per piece and per curb");
        assert_eq!(count(&m, "class=\"iso-top k-sidewalk\""), 2);
        assert_eq!(count(&m, "fill=\"url(#h-travel)\""), 2);
        assert_eq!(count(&m, "class=\"iso-ground\""), 1);
        assert!(!m.contains("NaN"));
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
        // Sample Avenue 2: a sidewalk beside a planting strip of the same height has no side face; the planting strip,
        // beside parking, has one; the median has one; the right sidewalk, at the street's edge, has one.
        let avenue = drawn(&Editor::new(1));
        assert_eq!(count(&avenue, "class=\"iso-front "), 5);
        assert_eq!(count(&avenue, "class=\"iso-side "), 3);
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
        assert_ne!(drawn(&Editor::new(0)), drawn(&Editor::new(1)));
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
        assert!(fr.contains("<title>Trottoir</title>") && !fr.contains(">Sidewalk<"), "{fr}");
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

    /// A street with things to stand on its pieces: Sample Avenue 2 (two planting strips of trees) with a
    /// bike lane, whose curb gets posts, and a bus shelter, and a piece selected.
    fn furnished() -> Editor {
        let mut e = Editor::new(1);
        e.add(kind_index("bike").unwrap(), 3);
        e.add(kind_index("busshelter").unwrap(), 1);
        let pick = e.view().segments[4].uid;
        e.select(Some(pick));
        e
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
        let streets: Vec<(String, Editor)> =
            (0..4).map(|i| (format!("sample {i}"), Editor::new(i))).chain(std::iter::once(("furnished".to_string(), furnished()))).collect();
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
