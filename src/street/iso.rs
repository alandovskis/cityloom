//! The street in 2.5D: a straight stretch of it seen from above and to one side, each piece an
//! extruded strip with its height, the few things that stand on the pieces as plain solids, and the
//! one-way arrows on the roadway. It is drawn from the same view the section is and is read-only:
//! the section is where the street is edited.
//!
//! The rises are exaggerated: a curb or a sidewalk stands 150 mm, a few pixels at the scale a whole
//! street is drawn at, so the slabs, and the curbs with them, are drawn `RISE` times as high as they stand
//! (the height tables keep the true heights). What a curb stands above its own piece (a bike lane's divider)
//! is drawn true, so that it does not bury a narrow lane (`curb_top_mm`). The things that stand on the
//! pieces keep their own size and sit on the drawn tops.

use std::fmt::Write;

use crate::shared::catalogue::{KINDS, kind_key};
use crate::shared::i18n::{Args, I18n};
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::{SegView, View};
use crate::street::svg::{curb_edges, curb_width_mm, dir_glyph, esc};

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
/// Room above the far end of the ground for what stands on the pieces. The tallest is a tree, 5 m on a
/// planting strip drawn `RISE` times its height (5.45 m); the farthest crown ends 2.8 m short of the far
/// end, so it needs 4.61 m of this; the rest (0.39 m) is a margin for its strokes.
const HEADROOM_MM: f64 = 5_000.0;

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

/// How high the top of a curb on a piece of this kind is drawn, in millimetres of the drawing: as high as the
/// piece, exaggerated with it, and what the curb stands above the piece at its true height. A bike lane is at the
/// roadway's level, so its curb is a divider drawn true: exaggerated, its wall would cover half a narrow lane.
pub fn curb_top_mm(curb: &str, kind_id: &str) -> f64 {
    let h = height_mm(kind_id);
    drawn_mm(h) + (curb_height_mm(curb) - h).max(0) as f64
}

/// A piece narrower than this on screen draws without what stands on it and without its arrow. It is below the
/// section's 26 px: the 2.5D draws a street smaller (it fits the recession across too), and a tree is a box
/// standing up off its strip, so it reads on a 1.5 m strip at 760 px (about 21 px).
pub const MIN_SOLID_PX: f64 = 20.0;
/// How far along the street a one-way arrow is laid, in millimetres: near the near end, because what stands
/// on the pieces hides only the ground farther along than itself: a tree's crown nothing nearer than about 10 m,
/// and the nearest trunk, from 5.4 m on, only a band of ground within 2.5 m to the left of its strip.
pub const ARROW_AT_MM: f64 = 6_000.0;

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

/// Which face of a strip, or of a curb, a polygon is.
#[derive(Clone, Copy)]
enum Face {
    Top,
    Front,
    Side,
}

/// One face: a polygon in the tint of the piece of kind `kind`, or of a curb when there is none
/// (marked with the face it is, so that the faces turned from the eye are shaded), then the hatch
/// `hatch` over it.
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
    write!(out, " points=\"{pts}\"/><polygon class=\"hatch\" points=\"{pts}\" fill=\"url(#{hatch})\"/>").unwrap();
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
fn curb(out: &mut String, p: &Projection, v: &View, i: usize, right: bool) {
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

/// Something standing on a piece, drawn as a plain box.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Solid {
    Trunk,
    Crown,
    Shelter,
    Post,
}

impl Solid {
    fn name(self) -> &'static str {
        match self {
            Solid::Trunk => "trunk",
            Solid::Crown => "crown",
            Solid::Shelter => "shelter",
            Solid::Post => "post",
        }
    }
}

/// A box standing in the scene, in millimetres of the drawing: across, up, along.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Block {
    solid: Solid,
    x: (f64, f64),
    y: (f64, f64),
    z: (f64, f64),
}

/// What stands on the piece at `i`: a tree every 8 m on street trees, a shelter on a bus shelter's
/// piece, a post every 4 m along a bike lane's curb (not an island's). Each keeps its own size and
/// stands on the top as drawn, `RISE` times as high as the piece stands. Nothing on a piece too
/// narrow on screen to show it.
fn blocks(v: &View, i: usize, p: &Projection) -> Vec<Block> {
    let s = &v.segments[i];
    let (x0, w) = (s.x_mm as f64, s.width_mm as f64);
    if w * p.scale < MIN_SOLID_PX {
        return Vec::new();
    }
    let id = KINDS[s.kind].id;
    let (cx, h) = (x0 + w / 2.0, drawn_mm(height_mm(id)));
    let along = |first: f64, every: f64| (0..).map(move |n| first + n as f64 * every).take_while(|z| *z < STRETCH_MM);
    let mut out = Vec::new();
    if s.material == "trees" {
        let crown = (w * 0.9).min(2_400.0);
        for z in along(4_000.0, 8_000.0) {
            out.push(Block { solid: Solid::Trunk, x: (cx - 125.0, cx + 125.0), y: (h, h + 2_000.0), z: (z - 125.0, z + 125.0) });
            out.push(Block { solid: Solid::Crown, x: (cx - crown / 2.0, cx + crown / 2.0), y: (h + 2_000.0, h + 5_000.0), z: (z - 1_200.0, z + 1_200.0) });
        }
    }
    if id == "busshelter" {
        let d = (w * 0.8).min(1_500.0);
        out.push(Block { solid: Solid::Shelter, x: (cx - d / 2.0, cx + d / 2.0), y: (h, h + 2_500.0), z: (18_000.0, 22_000.0) });
    }
    if let Some(c) = s.curb.filter(|c| id == "bike" && *c != "island") {
        let (top, cw) = (curb_top_mm(c, id), curb_width_mm(c).min(w / 2.0));
        for right in curb_edges(v, i) {
            let px = if right { x0 + w - cw / 2.0 } else { x0 + cw / 2.0 };
            for z in along(2_000.0, 4_000.0) {
                out.push(Block { solid: Solid::Post, x: (px - 60.0, px + 60.0), y: (top, top + 900.0), z: (z - 60.0, z + 60.0) });
            }
        }
    }
    out
}

/// A box's three faces the eye sees: its front, its right side and its top.
fn block(out: &mut String, p: &Projection, b: &Block) {
    let ((x0, x1), (y0, y1), (z0, z1)) = (b.x, b.y, b.z);
    write!(
        out,
        "<g class=\"iso-solid s-{}\"><polygon class=\"o-front\" points=\"{}\"/><polygon class=\"o-side\" points=\"{}\"/><polygon class=\"o-top\" points=\"{}\"/></g>",
        b.solid.name(),
        points(p, &[(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0)]),
        points(p, &[(x1, y0, z0), (x1, y0, z1), (x1, y1, z1), (x1, y1, z0)]),
        points(p, &[(x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)])
    )
    .unwrap();
}

/// The section's one-way arrow, laid on the piece's top `ARROW_AT_MM` along the stretch: drawn in
/// millimetres and mapped onto the street's plane, so "away" points into the distance.
fn arrow(out: &mut String, p: &Projection, s: &SegView) {
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

/// The outline of the selected piece at `i`, as one shape round everything drawn for it (its strip
/// and its curbs): its near face, its right side where that is seen, and its top, at the height of
/// its highest part.
fn outline(out: &mut String, p: &Projection, v: &View, i: usize) {
    let s = &v.segments[i];
    let id = KINDS[s.kind].id;
    let top = s.curb.filter(|_| !curb_edges(v, i).is_empty()).map_or(drawn_mm(height_mm(id)), |c| curb_top_mm(c, id));
    let (x0, x1, l) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64, STRETCH_MM);
    let low = beside(v, i).min(top);
    write!(
        out,
        "<polygon class=\"iso-sel\" points=\"{}\"/>",
        points(p, &[(x0, 0.0, 0.0), (x1, 0.0, 0.0), (x1, low, 0.0), (x1, low, l), (x1, top, l), (x0, top, l), (x0, top, 0.0)])
    )
    .unwrap();
}

/// How wide the scene is across: the street, or the pieces where they run over it.
fn span_mm(v: &View) -> i32 {
    v.row_mm.max(v.total_mm)
}

/// How tall the 2.5D of this street is drawn `width` wide, without drawing it.
pub fn iso_height(v: &View, width: f64) -> f64 {
    Projection::fit(width, span_mm(v)).height
}

/// The street in 2.5D, drawn `width` wide: a stretch of `STRETCH_MM` seen obliquely, from the same
/// view the section is drawn from. Hovering a piece names it.
///
/// What is drawn is ordered so that nothing needs sorting by depth: the ground, then the strips from
/// the lowest to the highest (a raised piece stands in front of the roadway beside it on both sides),
/// each piece's slab, then its one-way arrow, before its curbs. Then what stands on the pieces from far to
/// near, and last the selected piece's outline.
pub fn iso_svg(v: &View, i18n: &I18n, width: f64) -> IsoSvg {
    let span = span_mm(v);
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
        // A lane's arrow lies on its own strip, so its curbs and any higher piece, drawn later, stand over it.
        if edge.is_none() {
            arrow(&mut out, &p, s);
        }
    }

    // What stands on the pieces, so that a nearer box covers the ones behind it: piece by piece from left
    // to right (each box stays inside its piece, and the eye sees the right sides, so a box on a piece
    // further right is the nearer wherever two overlap), and on each piece from the far end to the near and
    // from left to right. A tree's crown starts nearer than its trunk, so it covers it.
    let mut solids: Vec<(usize, Block)> = (0..v.segments.len()).flat_map(|i| blocks(v, i, &p).into_iter().map(move |b| (i, b))).collect();
    solids.sort_by(|(i, a), (j, b)| i.cmp(j).then(b.z.0.total_cmp(&a.z.0)).then(a.x.0.total_cmp(&b.x.0)));
    for (_, b) in &solids {
        block(&mut out, &p, b);
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
        // 31 m of street and 16 m of depth across 940 px: 0.02 px to the millimetre; 12 m of depth and 5 m of headroom up.
        let p = Projection::fit(1000.0, 31_000);
        assert!((p.scale - 0.02).abs() < 1e-12, "{}", p.scale);
        assert_eq!((p.left, p.width), (30.0, 1000.0));
        assert!((p.base - 370.0).abs() < 1e-9 && (p.height - 400.0).abs() < 1e-9, "{p:?}");
        // Width runs right at full scale, height runs up, depth runs up and right at half scale.
        assert!(close(p.at(0.0, 0.0, 0.0), (30.0, 370.0)));
        assert!(close(p.at(1_000.0, 0.0, 0.0), (50.0, 370.0)));
        assert!(close(p.at(0.0, 1_000.0, 0.0), (30.0, 350.0)));
        assert!(close(p.at(0.0, 0.0, 1_000.0), (38.0, 364.0)));
        assert!(close(p.at(1_000.0, 1_000.0, 1_000.0), (58.0, 344.0)));
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

    #[test]
    fn a_planting_strip_of_trees_has_a_tree_every_8_m_each_a_trunk_and_a_crown() {
        // Sample Avenue 2 has two 1.5 m planting strips of street trees: five trees each along 40 m.
        let m = drawn(&Editor::new(1));
        assert_eq!(count(&m, "class=\"iso-solid s-crown\""), 10);
        assert_eq!(count(&m, "class=\"iso-solid s-trunk\""), 10);
        // A tree is a trunk with its crown over it: a crown follows its trunk.
        assert!(m.find("s-trunk").unwrap() < m.find("s-crown").unwrap());
        // Every solid comes after every strip.
        assert!(m.rfind("class=\"iso-piece\"").unwrap() < m.find("class=\"iso-solid").unwrap());
    }

    #[test]
    fn a_tree_stands_on_its_strip_s_drawn_top_at_its_own_true_height() {
        // The nearest tree of Sample Avenue 2's first (left) planting strip, 4 m along: a 250 mm trunk 2 m tall whose
        // foot is on the strip's top, drawn RISE times as high as the strip stands, and a crown 3 m tall over it. The
        // faces are matched whole, across as well as up, so a tree on another piece would not pass.
        let v = Editor::new(1).view();
        let strip = v.segments.iter().find(|s| s.material == "trees").unwrap();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let (cx, top) = (strip.x_mm as f64 + strip.width_mm as f64 / 2.0, drawn_mm(150));
        let trunk_front =
            points(&p, &[(cx - 125.0, top, 3_875.0), (cx + 125.0, top, 3_875.0), (cx + 125.0, top + 2_000.0, 3_875.0), (cx - 125.0, top + 2_000.0, 3_875.0)]);
        assert!(m.contains(&format!("<g class=\"iso-solid s-trunk\"><polygon class=\"o-front\" points=\"{trunk_front}\"/>")), "{m}");
        let half = strip.width_mm as f64 * 0.9 / 2.0;
        let crown_top = points(
            &p,
            &[
                (cx - half, top + 5_000.0, 2_800.0),
                (cx + half, top + 5_000.0, 2_800.0),
                (cx + half, top + 5_000.0, 5_200.0),
                (cx - half, top + 5_000.0, 5_200.0),
            ],
        );
        assert!(m.contains(&format!("<polygon class=\"o-top\" points=\"{crown_top}\"/>")), "{m}");
    }

    #[test]
    fn the_farthest_crown_stays_inside_the_drawing_at_every_width() {
        for width in [600.0, 760.0, 1050.0, 1800.0] {
            let s = iso_svg(&Editor::new(1).view(), &en(), width);
            let top = xs_ys(&s.markup).into_iter().map(|(_, y)| y).fold(f64::MAX, f64::min);
            assert!(top >= PAD - 1e-9, "{width}: {top}");
        }
    }

    /// Every point of every polygon.
    fn xs_ys(markup: &str) -> Vec<(f64, f64)> {
        markup
            .split("points=\"")
            .skip(1)
            .flat_map(|rest| {
                rest.split('"')
                    .next()
                    .unwrap()
                    .split(' ')
                    .map(|pt| {
                        let mut c = pt.split(',').map(|n| n.parse::<f64>().unwrap());
                        (c.next().unwrap(), c.next().unwrap())
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn what_stands_on_a_piece_is_drawn_after_what_stands_on_the_pieces_to_its_left() {
        // The eye sees the right sides, so a solid on a piece further right is nearer and covers the ones to its left
        // even when it starts farther along: the shelter (18 m to 22 m) comes before the crown of the tree at 20 m
        // on the planting strip to its right.
        let mut e = Editor::new(1);
        e.add(kind_index("busshelter").unwrap(), 1);
        let m = drawn(&e);
        assert!(m.find("s-shelter").unwrap() < m.find("s-crown").unwrap());
    }

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
        assert!(m.contains(&format!("<g transform=\"scale({} 1)\">", head / (0.64 * len))), "{m}");
    }

    /// The convex hull of points on the drawing, counter-clockwise (monotone chain).
    fn hull(mut pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
        let mut out: Vec<(f64, f64)> = Vec::new();
        for pass in 0..2 {
            let start = out.len();
            let iter: Box<dyn Iterator<Item = &(f64, f64)>> = if pass == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
            for &q in iter {
                while out.len() >= start + 2 && cross(out[out.len() - 2], out[out.len() - 1], q) <= 0.0 {
                    out.pop();
                }
                out.push(q);
            }
            out.pop();
        }
        out
    }

    /// Two convex shapes on the drawing do not overlap: some edge of one separates them (separating axes).
    fn apart(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
        [a, b].iter().any(|shape| {
            (0..shape.len()).any(|k| {
                let (p, q) = (shape[k], shape[(k + 1) % shape.len()]);
                let n = (q.1 - p.1, p.0 - q.0);
                let span = |s: &[(f64, f64)]| s.iter().map(|v| v.0 * n.0 + v.1 * n.1).fold((f64::MAX, f64::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)));
                let ((a0, a1), (b0, b1)) = (span(a), span(b));
                a1 < b0 || b1 < a0
            })
        })
    }

    #[test]
    fn an_arrow_lies_near_the_near_end_where_no_tree_can_hide_it() {
        // The arrow is anchored ARROW_AT_MM along its lane.
        let v = Editor::new(1).view();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm));
        let m = iso_svg(&v, &en(), 1050.0).markup;
        let lane = v.segments.iter().find(|s| s.direction.is_some()).unwrap();
        let (e, f) = p.at(lane.x_mm as f64 + lane.width_mm as f64 / 2.0, 0.0, ARROW_AT_MM);
        assert!(m.contains(&format!(" {} {})\"><g transform=\"scale(", r(e), r(f))), "{m}");
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
                for s in v.segments.iter().filter(|s| s.direction.is_some() && s.width_mm as f64 * p.scale >= MIN_SOLID_PX) {
                    let (len, head) = arrow_shape(s.width_mm as f64);
                    let (cx, h) = (s.x_mm as f64 + s.width_mm as f64 / 2.0, drawn_mm(height_mm(KINDS[s.kind].id)));
                    let (z0, z1) = (ARROW_AT_MM - len / 2.0, ARROW_AT_MM + len / 2.0);
                    let ground =
                        [(cx - head / 2.0, h, z0), (cx + head / 2.0, h, z0), (cx + head / 2.0, h, z1), (cx - head / 2.0, h, z1)].map(|(x, y, z)| p.at(x, y, z));
                    assert!(trees.iter().all(|t| apart(t, &ground)), "{width}: the arrow of {} is hidden", s.uid);
                }
            }
        }
    }

    /// Sample Street 1 with two one-way bike lanes, each 1.8 m and curbed: one between the driving lanes (a curb on
    /// each edge) and one between the right parking lane and the sidewalk (a curb on its left, the sidewalk raised on
    /// its right).
    fn one_way_bikes() -> (Editor, u32, u32) {
        let mut e = Editor::new(0);
        let bike = kind_index("bike").unwrap();
        let middle = e.add(bike, 3);
        let kerbside = e.add(bike, 6);
        assert!(e.set_direction(middle, Some(0)) && e.set_direction(kerbside, Some(0)));
        (e, middle, kerbside)
    }

    #[test]
    fn a_lane_s_arrow_is_drawn_with_its_strip_so_its_curbs_and_a_higher_neighbour_stand_over_it() {
        let (e, middle, kerbside) = one_way_bikes();
        let v = e.view();
        let m = iso_svg(&v, &en(), 1050.0).markup;
        assert_eq!(count(&m, "class=\"iso-dir\""), 4);
        for uid in [middle, kerbside] {
            let open = format!("<g class=\"iso-piece\" data-uid=\"{uid}\">");
            let slab = m.find(&open).expect("the slab is drawn");
            // The arrow follows the lane's own strip straight away...
            let after = slab + m[slab..].find("</g>").unwrap() + 4;
            assert!(m[after..].starts_with("<g class=\"iso-dir\""), "{uid}: {}", &m[after..after + 60]);
            // ...and comes before the lane's curb blocks.
            let curbs: Vec<usize> = m.match_indices(&open).skip(1).map(|(at, _)| at).collect();
            assert!(!curbs.is_empty() && curbs.iter().all(|c| *c > after), "{uid}");
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
    fn a_bus_shelter_stands_on_its_piece_and_a_bike_lane_s_curb_carries_posts() {
        let mut e = Editor::new(0);
        e.add(kind_index("busshelter").unwrap(), 1);
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-shelter\""), 1);
        // A bike lane with a curb on both edges: a post every 4 m on each, ten each along 40 m.
        let mut e = Editor::new(0);
        let u = e.add(kind_index("bike").unwrap(), 2);
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-post\""), 20);
        // A bus boarding island is a platform: no posts.
        assert!(e.set_curb(u, Some(6)));
        assert_eq!(count(&drawn(&e), "class=\"iso-solid s-post\""), 0);
    }

    #[test]
    fn a_piece_too_narrow_on_screen_draws_without_its_solids_but_keeps_its_faces() {
        // At 600 px Sample Avenue 2's 1.5 m planting strips are under 20 px (17.6): no trees, all eleven tops.
        let narrow = iso_svg(&Editor::new(1).view(), &en(), 600.0).markup;
        assert_eq!(count(&narrow, "class=\"iso-solid"), 0);
        assert_eq!(count(&narrow, "class=\"iso-top "), 11);
        // The freeway at 600 px still draws every piece, with no missing number.
        let freeway = iso_svg(&Editor::new(3).view(), &en(), 600.0);
        assert_eq!(count(&freeway.markup, "class=\"iso-top "), 9);
        assert!(!freeway.markup.contains("NaN"));
    }

    #[test]
    fn at_760_px_the_planting_strips_keep_their_trees_the_shelter_and_the_bike_lane_s_posts() {
        // A 1.5 m strip is about 21 px there: narrow, but a tree is a box standing up and reads.
        let avenue = iso_svg(&Editor::new(1).view(), &en(), 760.0).markup;
        assert_eq!(count(&avenue, "class=\"iso-solid s-crown\""), 10);
        let m = iso_svg(&furnished().view(), &en(), 760.0).markup;
        assert_eq!(count(&m, "class=\"iso-solid s-crown\""), 10);
        assert_eq!(count(&m, "class=\"iso-solid s-shelter\""), 1);
        assert_eq!(count(&m, "class=\"iso-solid s-post\""), 20);
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
        assert!(m.contains(&format!("transform=\"matrix({} 0 {} {} ", p.scale, -RECEDE_X * p.scale, RECEDE_Y * p.scale)), "{m}");
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
        assert!(m.contains(&format!("<polygon class=\"iso-sel\" points=\"{},{} ", r(x), r(y))), "{m}");
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
        assert!(!m.contains("NaN") && !m.contains("inf") && count(&m, "class=\"iso-sel\"") == 1, "{m}");
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

    #[test]
    fn the_2_5d_is_labelled_with_the_street_the_stretch_and_its_pieces_in_each_language() {
        let v = Editor::new(0).view();
        assert_eq!(iso_label(&v, &en(), Units::Metres), "Sample Street 1 in 2.5D, along a 40 m stretch. 6 segments.");
        assert_eq!(iso_label(&v, &en(), Units::Feet), "Sample Street 1 in 2.5D, along a 131 ft stretch. 6 segments.");
        let fr = crate::i18n_for(Locale::FrCa);
        assert_eq!(iso_label(&v, &fr, Units::Metres), "Sample Street 1 en 2,5D, sur un tronçon de 40\u{a0}m. 6 éléments.");
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
            .chain([("furnished".to_string(), furnished()), ("one-way bikes".to_string(), one_way_bikes().0)])
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
