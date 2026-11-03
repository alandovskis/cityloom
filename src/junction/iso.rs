//! The junction in 2.5D, as SVG markup, from the model's view: a read-only picture of what the plan draws flat.
//! A plan point is rotated by a fixed angle, its depth squashed, and its height lifted up the screen; there is one
//! viewpoint, so the shapes are painted far to near with no depth buffer.

use std::fmt::Write;

use serde_json::Value;

use crate::junction::frame::Frame;
use crate::junction::geometry::{sample_arc, sample_quad};
use crate::junction::iso_marks::{LEAVE_ARROW_MM, crossing_dimension, lane_arrow, lane_arrow_size, movement_strokes, plain_arrow, ring_arrows};
use crate::junction::plan_svg::turbo_name;
use crate::junction::read_model::{ArmView, JView};
use crate::junction::text::arms_text;
use crate::shared::catalogue::{KINDS, MATERIALS};
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::{esc, f1};
use crate::shared::iso::ground::patterns;
use crate::shared::iso::paint::{Paint, Pattern, Style, class, quads};
use crate::shared::iso::shading::shade_filters;
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::StopKind;

type P = (f64, f64);

/// How the plan is turned, in degrees, before it is squashed.
pub const ROTATION_DEG: f64 = 30.0;
/// How much of its depth a point keeps.
pub const SQUASH: f64 = 0.55;
/// How much taller than true a height is drawn, so that a curb reads.
pub const LIFT: f64 = 3.0;
/// The tallest thing drawn, for the room left above the junction.
pub const TALLEST_MM: f64 = 3000.0;

/// How tall the things that stand on the ground are.
const SIGNAL_POST_MM: f64 = 2500.0;
const SIGN_POST_MM: f64 = 2000.0;
const BOLLARD_MM: f64 = 900.0;

/// How wide a zebra stripe is, and the gap between two, as the plan draws them.
const ZEBRA_MM: f64 = 600.0;
/// How thick a stop line is, and the dashes and gaps of one where traffic gives way.
const STOP_LINE_MM: f64 = 300.0;
const YIELD_DASH_MM: f64 = 700.0;
const YIELD_GAP_MM: f64 = 500.0;

/// How long on the screen an arrow's length on the ground is drawn, near enough, for the weight of its line.
const ARROW_SCREEN: f64 = 0.8;
/// The steps a curve is sampled in.
const CURVE_STEPS: usize = 12;

/// A plan point as the viewer sees the ground: turned, not yet squashed. Its `.1` is how near it is to the viewer.
pub fn view_point(p: P) -> P {
    let t = ROTATION_DEG.to_radians();
    (p.0 * t.cos() - p.1 * t.sin(), p.0 * t.sin() + p.1 * t.cos())
}

/// A plan point `h_mm` above the ground, in millimetres on the screen before it is scaled.
pub fn project(p: P, h_mm: f64) -> P {
    let (x, y) = view_point(p);
    (x, y * SQUASH - h_mm * LIFT)
}

/// The points of a path of the model's drawing commands, curves sampled.
pub fn flatten(cmds: &[Value]) -> Vec<P> {
    let num = |c: &Value, i: usize| c[i].as_f64().unwrap_or(0.0);
    let mut pts: Vec<P> = Vec::new();
    for c in cmds {
        match c[0].as_str().unwrap_or("Z") {
            "M" | "L" => pts.push((num(c, 1), num(c, 2))),
            "Q" => {
                if let Some(&from) = pts.last() {
                    pts.extend(sample_quad(from, (num(c, 1), num(c, 2)), (num(c, 3), num(c, 4)), CURVE_STEPS).into_iter().skip(1));
                }
            }
            "A" => {
                if let Some(&from) = pts.last() {
                    let to = (num(c, 4), num(c, 5));
                    pts.extend(sample_arc(from, num(c, 1), num(c, 2) != 0.0, num(c, 3) != 0.0, to, CURVE_STEPS).into_iter().skip(1));
                }
            }
            _ => {}
        }
    }
    pts
}

/// How high a piece of this kind stands above the roadway, in millimetres. Presentation, not catalogue data.
pub fn height_of(kind_id: &str) -> f64 {
    match kind_id {
        "sidewalk" | "median" | "planting" => 150.0,
        _ => 0.0,
    }
}

/// How the 2.5D picture sits in the drawing: the same size as the plan's, so the page does not jump on a switch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsoFrame {
    pub scale: f64,
    pub ox: f64,
    pub oy: f64,
    pub width: f64,
    pub height: f64,
}

impl IsoFrame {
    /// The frame for a drawing `width` wide in a window `window_height` high, as big as the plan's, that holds
    /// the points of `content` (on the ground) and the tallest thing standing above them. `bounds` is the plan's,
    /// which only sizes the drawing: it has room round the junction for the street names the picture has none of.
    pub fn fit(bounds: [f64; 4], content: &[P], width: f64, window_height: f64) -> IsoFrame {
        let plan = Frame::fit(bounds, width, window_height);
        let ground: Vec<P> = content.iter().map(|&c| project(c, 0.0)).collect();
        let (min_x, max_x) = ground.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.0), hi.max(p.0)));
        let (min_y, max_y) = ground.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
        let min_y = min_y - TALLEST_MM * LIFT;
        let (pad_x, pad_y) = if plan.narrow { (24.0, 40.0) } else { ((plan.width * 0.1).min(80.0), 40.0) };
        let scale = ((plan.width - 2.0 * pad_x) / (max_x - min_x)).min((plan.height - 2.0 * pad_y) / (max_y - min_y));
        IsoFrame {
            scale,
            ox: plan.width / 2.0 - (min_x + max_x) / 2.0 * scale,
            oy: plan.height / 2.0 - (min_y + max_y) / 2.0 * scale,
            width: plan.width,
            height: plan.height,
        }
    }

    /// A plan point `h_mm` above the ground, in pixels.
    pub fn at(&self, p: P, h_mm: f64) -> P {
        let (x, y) = project(p, h_mm);
        (self.ox + x * self.scale, self.oy + y * self.scale)
    }
}

/// What the page needs to put the 2.5D picture in its `<svg>`.
pub struct IsoSvg {
    pub frame: IsoFrame,
    pub markup: String,
}

/// A shape that stands on the ground: its footprint, its height, and whether its sides are drawn.
struct Solid {
    kind_id: &'static str,
    /// The material of its top.
    material: &'static str,
    foot: Vec<P>,
    h_mm: f64,
    sides: bool,
    /// The attributes that make it pressable (`hit_attrs`), or none.
    hit: String,
}

/// The filters that darken the faces turned from the eye: a ramp a little, a side more. SVG filters, since CSS
/// `filter: brightness()` on SVG shapes may not be applied in Safari.
fn shades(f: &IsoFrame) -> String {
    format!("<defs>{}{}</defs>", shade_filters(&[("iso-shade-ramp", "0.9"), ("iso-shade-side", "0.78")]), materials(f))
}

/// The surface materials' patterns laid on the ground as this frame projects it: the plan's x and y (in millimetres)
/// land where `at` puts them, so the matrix is read off three of its points.
fn materials(f: &IsoFrame) -> String {
    let (o, x, y) = (f.at((0.0, 0.0), 0.0), f.at((1.0, 0.0), 0.0), f.at((0.0, 1.0), 0.0));
    patterns("iso-m", [x.0 - o.0, x.1 - o.1, y.0 - o.0, y.1 - o.1, o.0, o.1])
}

/// The material a surface of this kind has when nothing says otherwise: the junction's own surfaces (the core, the
/// ring, a corner) are not a street's pieces.
fn default_material(kind_id: &str) -> &'static str {
    KINDS.iter().find(|k| k.id == kind_id).map_or("concrete", |k| MATERIALS[k.materials[0]].id)
}

/// How far a vertex may be from an edge and still be on it: where a ramp meets its top, where one surface meets
/// another.
const EDGE_TOLERANCE_MM: f64 = 50.0;

fn distance_to_segment(p: P, a: P, b: P) -> f64 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let len2 = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if len2 < 1e-9 { 0.0 } else { (((p.0 - a.0) * ab.0 + (p.1 - a.1) * ab.1) / len2).clamp(0.0, 1.0) };
    (p.0 - a.0 - t * ab.0).hypot(p.1 - a.1 - t * ab.1)
}

/// Whether `p` is within `EDGE_TOLERANCE_MM` of an edge of `poly`.
fn near_boundary(p: P, poly: &[P]) -> bool {
    (0..poly.len()).any(|i| distance_to_segment(p, poly[i], poly[(i + 1) % poly.len()]) < EDGE_TOLERANCE_MM)
}

/// Each vertex of `foot` with `h_mm` where it lies on the boundary of `top` and 0 where it does not: the slope of a
/// ramp rises from the road to the edge it meets.
fn lift_to(foot: &[P], top: &[P], h_mm: f64) -> Vec<(P, f64)> {
    foot.iter().map(|&p| (p, if near_boundary(p, top) { h_mm } else { 0.0 })).collect()
}

struct Draw {
    f: IsoFrame,
    flat: String,
    solids: Vec<Solid>,
    /// The footprints and heights of the surfaces with no wall round them, which a solid level with one does not
    /// show a wall against.
    tables: Vec<(Vec<P>, f64)>,
    /// Outlines and lines that belong on what stands (a stop's platform, a curb), painted over the solids.
    marks: String,
    /// Posts with their heads, painted over the solids.
    posts: String,
    /// The outline of what is selected.
    sel: String,
    /// The attributes the shapes drawn now carry so that a press on them selects something (`hit_attrs`); empty
    /// for a shape that is only looked at.
    hit: String,
    /// The streets' names, painted last so nothing standing hides one.
    names: String,
}

/// What a street's name takes of the drawing, a character at a time, to keep the whole name inside it.
const NAME_CHAR_PX: f64 = 9.0;
/// How far a name stands from the end of its arm, and from the edge of the drawing, in pixels.
const NAME_GAP_PX: f64 = 12.0;
const NAME_EDGE_PX: f64 = 4.0;

impl Draw {
    fn new(f: IsoFrame) -> Draw {
        Draw {
            f,
            flat: String::new(),
            solids: Vec::new(),
            tables: Vec::new(),
            marks: String::new(),
            posts: String::new(),
            sel: String::new(),
            hit: String::new(),
            names: String::new(),
        }
    }

    fn path(&self, pts: &[P], h_mm: f64) -> String {
        let mut d = String::new();
        for (i, p) in pts.iter().enumerate() {
            let (x, y) = self.f.at(*p, h_mm);
            write!(d, "{}{} {}", if i == 0 { "M" } else { "L" }, f1(x), f1(y)).unwrap();
        }
        d.push('Z');
        d
    }

    /// A shape lying on the ground, in the default material of its kind.
    fn lay(&mut self, kind_id: &str, foot: &[P]) {
        self.lay_in(kind_id, default_material(kind_id), foot);
    }

    /// A shape lying on the ground, in `material`.
    fn lay_in(&mut self, kind_id: &str, material: &str, foot: &[P]) {
        if foot.len() < 3 {
            return;
        }
        let d = self.path(foot, 0.0);
        let hit = &self.hit;
        write!(
            self.flat,
            "<path class=\"iso-flat k-{kind_id} mat-{material}\"{hit} d=\"{d}\"/><path class=\"hatch\" fill=\"url(#iso-m-{material})\" d=\"{d}\"/>"
        )
        .unwrap();
    }

    /// The stripes of a zebra crossing: `foot` is its four corners, across the street first. They run the length
    /// of the crossing, as wide as the gaps between them.
    fn zebra(&mut self, foot: &[P], h_mm: f64) {
        if foot.len() != 4 {
            return;
        }
        let (p0, p1, p3) = (foot[0], foot[1], foot[3]);
        let across = (p1.0 - p0.0, p1.1 - p0.1);
        let len = across.0.hypot(across.1);
        if len < 1e-9 {
            return;
        }
        let along = (p3.0 - p0.0, p3.1 - p0.1);
        for k in 0..(len / (2.0 * ZEBRA_MM)).ceil() as usize {
            let (t0, t1) = (2.0 * k as f64 * ZEBRA_MM / len, ((2.0 * k as f64 + 1.0) * ZEBRA_MM / len).min(1.0));
            let (a, b) = ((p0.0 + across.0 * t0, p0.1 + across.1 * t0), (p0.0 + across.0 * t1, p0.1 + across.1 * t1));
            let d = self.path(&[a, b, (b.0 + along.0, b.1 + along.1), (a.0 + along.0, a.1 + along.1)], h_mm);
            write!(self.flat, "<path class=\"iso-zebra\" d=\"{d}\"/>").unwrap();
        }
    }

    /// A solid double white line along `path`, an open polyline of points, lying on the road. The quads come from the
    /// shared paint and are projected here, like the road they lie on. The line is not joined to its start: a turbo's
    /// dividers are pieces between two streets.
    fn divider(&mut self, path: &[P]) {
        for (paint, quad) in quads(path, Style { paint: Paint::White, pattern: Pattern::Solid, double: true }) {
            let d = self.path(&quad, 0.0);
            write!(self.flat, "<path class=\"{}\" d=\"{d}\"/>", class(paint)).unwrap();
        }
    }

    /// A stop line from `a` to `b`: a row of dashes where traffic gives way, as the plan draws it.
    fn stop_line(&mut self, a: P, b: P, h_mm: f64, dashed: bool) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len < 1e-9 {
            return;
        }
        let n = (-dy / len * STOP_LINE_MM / 2.0, dx / len * STOP_LINE_MM / 2.0);
        let at = |t: f64| (a.0 + dx * t / len, a.1 + dy * t / len);
        let (dash, gap) = if dashed { (YIELD_DASH_MM, YIELD_GAP_MM) } else { (len, 0.0) };
        let mut t = 0.0;
        while t < len {
            let (p, q) = (at(t), at((t + dash).min(len)));
            let d = self.path(&[(p.0 + n.0, p.1 + n.1), (q.0 + n.0, q.1 + n.1), (q.0 - n.0, q.1 - n.1), (p.0 - n.0, p.1 - n.1)], h_mm);
            write!(self.flat, "<path class=\"iso-stopline\" d=\"{d}\"/>").unwrap();
            t += dash + gap;
        }
    }

    /// A surface `h_mm` high with no wall round it, a table's top or a continuous sidewalk, drawn over what lies
    /// on the ground.
    fn table(&mut self, kind_id: &str, foot: &[P], h_mm: f64) {
        if foot.len() < 3 {
            return;
        }
        self.tables.push((foot.to_vec(), h_mm));
        let d = self.path(foot, h_mm);
        let material = default_material(kind_id);
        let hit = &self.hit;
        write!(
            self.flat,
            "<path class=\"iso-table iso-flat k-{kind_id} mat-{material}\"{hit} d=\"{d}\"/><path class=\"hatch\" fill=\"url(#iso-m-{material})\" d=\"{d}\"/>"
        )
        .unwrap();
    }

    /// A ramp from the ground up to `top`: its vertices on the top's edge at `h_mm`, the rest on the ground.
    fn ramp(&mut self, kind_id: &str, foot: &[P], top: &[P], h_mm: f64) {
        if foot.len() < 3 {
            return;
        }
        let mut d = String::new();
        for (i, (p, h)) in lift_to(foot, top, h_mm).into_iter().enumerate() {
            let (x, y) = self.f.at(p, h);
            write!(d, "{}{} {}", if i == 0 { "M" } else { "L" }, f1(x), f1(y)).unwrap();
        }
        d.push('Z');
        write!(self.flat, "<path class=\"iso-ramp k-{kind_id}\" filter=\"url(#iso-shade-ramp)\" d=\"{d}\"/>").unwrap();
    }

    fn stand(&mut self, kind_id: &'static str, foot: Vec<P>, h_mm: f64, sides: bool) {
        self.stand_in(kind_id, default_material(kind_id), foot, h_mm, sides);
    }

    fn stand_in(&mut self, kind_id: &'static str, material: &'static str, foot: Vec<P>, h_mm: f64, sides: bool) {
        if foot.len() >= 3 {
            self.solids.push(Solid { kind_id, material, foot, h_mm, sides, hit: self.hit.clone() });
        }
    }

    /// The depth of a solid: how near its footprint's centre is to the viewer.
    fn depth(s: &Solid) -> f64 {
        let n = s.foot.len() as f64;
        let c = s.foot.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0 / n, a.1 + p.1 / n));
        view_point(c).1
    }

    /// Whether the edge `a`-`b` of solid `i` is shared with a surface as high as it is, which a curb would be
    /// drawn across for nothing: the solids and the tops that lie against it.
    fn meets_level(&self, i: usize, a: P, b: P) -> bool {
        let h = self.solids[i].h_mm;
        let on = |foot: &[P]| near_boundary(a, foot) && near_boundary(b, foot);
        self.solids.iter().enumerate().any(|(j, o)| j != i && o.h_mm == h && on(&o.foot)) || self.tables.iter().any(|(t, th)| *th == h && on(t))
    }

    fn render(&self, i: usize) -> String {
        let s = &self.solids[i];
        let id = s.kind_id;
        let mut g = format!("<g class=\"iso-solid\" data-depth=\"{}\"{}>", f1(Self::depth(s)), s.hit);
        if s.sides {
            let seen: Vec<P> = s.foot.iter().map(|p| view_point(*p)).collect();
            let n = seen.len();
            let area: f64 = (0..n).map(|i| seen[i].0 * seen[(i + 1) % n].1 - seen[(i + 1) % n].0 * seen[i].1).sum();
            for k in 0..n {
                let (a, b) = (seen[k], seen[(k + 1) % n]);
                // The edge's outward normal points towards the viewer when its depth component is positive.
                let e = (b.0 - a.0, b.1 - a.1);
                let toward_viewer = if area > 0.0 { -e.0 } else { e.0 } > 0.0;
                let (pa, pb) = (s.foot[k], s.foot[(k + 1) % n]);
                if toward_viewer && !self.meets_level(i, pa, pb) {
                    let ((x0, y0), (x1, y1)) = (self.f.at(pa, 0.0), self.f.at(pb, 0.0));
                    let ((x2, y2), (x3, y3)) = (self.f.at(pb, s.h_mm), self.f.at(pa, s.h_mm));
                    write!(
                        g,
                        "<path class=\"iso-side k-{id}\" filter=\"url(#iso-shade-side)\" d=\"M{} {}L{} {}L{} {}L{} {}Z\"/>",
                        f1(x0),
                        f1(y0),
                        f1(x1),
                        f1(y1),
                        f1(x2),
                        f1(y2),
                        f1(x3),
                        f1(y3)
                    )
                    .unwrap();
                }
            }
        }
        let d = self.path(&s.foot, s.h_mm);
        let m = s.material;
        write!(g, "<path class=\"iso-top k-{id} mat-{m}\" d=\"{d}\"/><path class=\"hatch\" fill=\"url(#iso-m-{m})\" d=\"{d}\"/></g>").unwrap();
        g
    }

    /// A post from the ground up to `h_mm`, with a head on it.
    fn post(&mut self, at: P, h_mm: f64, head: &str) {
        let (x0, y0) = self.f.at(at, 0.0);
        let (x1, y1) = self.f.at(at, h_mm);
        let r = (250.0 * self.f.scale).clamp(2.5, 7.0);
        write!(
            self.posts,
            "<line class=\"iso-post\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/><circle class=\"iso-head {head}\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
            f1(x0),
            f1(y0),
            f1(x1),
            f1(y1),
            f1(x1),
            f1(y1),
            f1(r)
        )
        .unwrap();
    }

    /// The outline of a shape `h_mm` above the ground, to show what is selected: two tones, the light line over a dark
    /// casing, so that one of them stands out from any surface (the street page's selection outline).
    fn outline(&mut self, foot: &[P], h_mm: f64) {
        if foot.len() >= 3 {
            let d = self.path(foot, h_mm);
            write!(self.sel, "<path class=\"iso-sel-casing\" d=\"{d}\"/><path class=\"sel-box iso-sel\" d=\"{d}\"/>").unwrap();
        }
    }

    /// Makes the shapes `draw` draws pressable: a press on one selects what `attrs` (`hit_attrs`) says.
    fn pressing(&mut self, attrs: String, draw: impl FnOnce(&mut Draw)) {
        self.hit = attrs;
        draw(self);
        self.hit.clear();
    }

    /// A shape that is only a place to press, with nothing painted in it. `class` is its CSS class, `title` what the
    /// pointer is told it is.
    fn press_area(&mut self, class: &str, attrs: &str, foot: &[P], h_mm: f64, title: Option<&str>) {
        if foot.len() < 3 {
            return;
        }
        let d = self.path(foot, h_mm);
        let title = title.map(|t| format!("<title>{}</title>", esc(t))).unwrap_or_default();
        write!(self.flat, "<path class=\"{class}\"{attrs} d=\"{d}\">{title}</path>").unwrap();
    }

    /// A street's name beyond the end of its arm, away from the middle of the junction and kept inside the drawing.
    fn name(&mut self, end: P, text: &str, selected: bool) {
        let (ex, ey) = self.f.at(end, 0.0);
        let (cx, cy) = self.f.at((0.0, 0.0), 0.0);
        let len = ((ex - cx).powi(2) + (ey - cy).powi(2)).sqrt().max(1.0);
        let (dx, dy) = ((ex - cx) / len, (ey - cy) / len);
        let w = text.chars().count() as f64 * NAME_CHAR_PX;
        let (anchor, shift) = if dx > 0.3 {
            ("start", 0.0)
        } else if dx < -0.3 {
            ("end", w)
        } else {
            ("middle", w / 2.0)
        };
        // the left edge of the text, kept between the drawing's edges
        let left = (ex + dx * NAME_GAP_PX - shift).min(self.f.width - NAME_EDGE_PX - w).max(NAME_EDGE_PX);
        let y = (ey
            + dy * NAME_GAP_PX
            + if dy < -0.3 {
                -4.0
            } else if dy > 0.3 {
                14.0
            } else {
                5.0
            })
        .min(self.f.height - NAME_EDGE_PX)
        .max(14.0);
        write!(
            self.names,
            "<text class=\"iso-name t-mark t-halo{}\" x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\">{}</text>",
            if selected { " t-blue" } else { "" },
            f1(left + shift),
            f1(y),
            esc(text)
        )
        .unwrap();
    }

    /// The path commands of open lines `h_mm` above the ground, one `M` for each.
    fn strokes_d(&self, strokes: &[Vec<P>], h_mm: f64) -> String {
        let mut d = String::new();
        for line in strokes.iter().filter(|l| l.len() >= 2) {
            for (i, p) in line.iter().enumerate() {
                let (x, y) = self.f.at(*p, h_mm);
                write!(d, "{}{} {}", if i == 0 { "M" } else { "L" }, f1(x), f1(y)).unwrap();
            }
        }
        d
    }

    /// A mark of `class` along `d`, with a casing under it in the panel's colour (a dark one for a warning), so that it
    /// is seen on any surface, light or dark. On the ground it lies under what stands; over, it is drawn over it.
    fn mark(&mut self, over: bool, class: &str, warn: bool, d: &str) {
        if d.is_empty() {
            return;
        }
        let casing = if warn { "iso-casing warn" } else { "iso-casing" };
        let layer = if over { &mut self.marks } else { &mut self.flat };
        write!(layer, "<path class=\"{casing}\" d=\"{d}\"/><path class=\"{class}\" d=\"{d}\"/>").unwrap();
    }

    /// A closed outline of `foot` `h_mm` above the ground, as a mark.
    fn mark_ring(&mut self, over: bool, class: &str, warn: bool, foot: &[P], h_mm: f64) {
        if foot.len() >= 3 {
            let d = self.path(foot, h_mm);
            self.mark(over, class, warn, &d);
        }
    }

    /// Open lines `h_mm` above the ground, as a mark.
    fn mark_lines(&mut self, over: bool, class: &str, warn: bool, strokes: &[Vec<P>], h_mm: f64) {
        let d = self.strokes_d(strokes, h_mm);
        self.mark(over, class, warn, &d);
    }

    /// A glyph painted on the road: `strokes` on the plan, `extra` the classes added to a lane's arrow (`bad`, `out`),
    /// `size_mm` how long it is on the ground, which sets how heavy its line is. A halo of the panel's colour is under it.
    fn glyph(&mut self, extra: &str, strokes: &[Vec<P>], size_mm: f64, h_mm: f64) {
        let d = self.strokes_d(strokes, h_mm);
        if d.is_empty() {
            return;
        }
        let px = f1(size_mm * self.f.scale * ARROW_SCREEN);
        write!(self.flat, "<g class=\"lane-arrow{extra}\" style=\"--arrow:{px}\"><path class=\"halo\" d=\"{d}\"/><path d=\"{d}\"/></g>").unwrap();
    }

    /// A line of text standing at a point of the ground, over a halo, on top of everything. `class` is its classes.
    fn label(&mut self, class: &str, at: P, h_mm: f64, dy: f64, text: &str) {
        let (x, y) = self.f.at(at, h_mm);
        write!(self.names, "<text class=\"{class}\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>", f1(x), f1(y + dy), esc(text)).unwrap();
    }

    fn finish(mut self) -> String {
        self.solids.sort_by(|a, b| Self::depth(a).total_cmp(&Self::depth(b)));
        let solids: String = (0..self.solids.len()).map(|i| self.render(i)).collect();
        format!("{}<g class=\"iso iso-junction\">{}{solids}{}{}{}{}</g>", shades(&self.f), self.flat, self.marks, self.posts, self.sel, self.names)
    }
}

/// The points the picture reaches on the ground: each arm from its mouth to its end, the core, and a
/// roundabout's ring, and what lies beyond an end zone. Everything drawn is inside them.
fn content_points(v: &JView) -> Vec<P> {
    let mut pts: Vec<P> = v
        .arms
        .iter()
        .flat_map(|a| {
            let uses = a.curb_uses.iter().filter_map(|u| u.poly.as_ref()).flat_map(|p| flatten(p));
            flatten(&a.outline).into_iter().chain(a.beyond.iter().flat_map(|p| flatten(&p.poly))).chain(uses)
        })
        .chain(flatten(&v.core))
        .collect();
    if let Some(ring) = &v.ring {
        if ring.outer_edge.is_empty() {
            let r = ring.radius_mm as f64;
            pts.extend([(r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)]);
        } else {
            pts.extend(ring.outer_edge.iter().copied());
        }
    }
    pts
}

/// What a screen reader is told the picture is, drawn again when the language changes.
pub fn iso_label(v: &JView, i18n: &I18n) -> String {
    match turbo_name(v, i18n) {
        Some(name) => i18n.tr("jn-iso-label-turbo", &Args::new().str("arms", arms_text(v, i18n)).str("name", name)),
        None => i18n.tr("jn-iso-label", &Args::new().str("arms", arms_text(v, i18n))),
    }
}

/// The attributes that make a shape of the picture pressable: what a press on it selects, as the plan's shapes say it
/// (`plan::selected_by` reads them back). The cycle track and the bus lane are the only one of their kind, and have
/// no uid.
pub fn hit_attrs(role: &str, uid: Option<u32>, lane: Option<usize>) -> String {
    let mut a = format!(" data-role=\"{role}\"");
    if let Some(uid) = uid {
        write!(a, " data-uid=\"{uid}\"").unwrap();
    }
    if let Some(lane) = lane {
        write!(a, " data-lane=\"{lane}\"").unwrap();
    }
    a
}

/// A circle of 48 points, round the middle of the junction.
fn circle(r: f64) -> Vec<P> {
    (0..48)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / 48.0;
            (r * t.cos(), r * t.sin())
        })
        .collect()
}

/// The junction in 2.5D, in a drawing `width` wide in a window `window_height` high. Every part the keys select can
/// be pressed (`hit_attrs`), and the one that is selected is outlined.
pub fn iso_svg(v: &JView, i18n: &I18n, width: f64, window_height: f64, units: Units) -> IsoSvg {
    let f = IsoFrame::fit(v.bounds, &content_points(v), width, window_height);
    let mut d = Draw::new(f);
    let wall = height_of("sidewalk");
    let selected = |kind: &str, uid: u32| v.selected.kind == Some(kind) && v.selected.uid == uid;

    // the carriageway where the streets meet, and a roundabout's ring and island
    if let Some(ring) = &v.ring {
        let track = ring.cycle_mm.is_some_and(|c| c != 0);
        if ring.lanes.is_empty() {
            if track {
                // The cycle track is the band outside the carriageway: the whole ring in its colour, the carriageway over it.
                d.pressing(hit_attrs("cycle", None, None), |d| d.lay("bike", &circle(ring.radius_mm as f64)));
                d.lay("travel", &circle(ring.road_mm as f64));
            } else {
                d.lay("travel", &circle(ring.radius_mm as f64));
            }
            if let Some(bus) = &v.bus {
                let poly = flatten(&bus.poly);
                d.pressing(hit_attrs("bus", None, None), |d| d.lay("bus", &poly));
                if v.selected.kind == Some("bus") {
                    d.outline(&poly, 0.0);
                }
            }
            d.stand("median", circle(ring.island_mm as f64), height_of("median"), true);
            if track && v.selected.kind == Some("cycle") {
                d.outline(&circle(ring.radius_mm as f64), 0.0);
                d.outline(&circle(ring.road_mm as f64), 0.0);
            }
        } else {
            // A turbo: the carriageway (the lanes and the 300 mm between them) is one travel surface, the dividers are paint on it.
            if track {
                d.pressing(hit_attrs("cycle", None, None), |d| d.lay("bike", &ring.outer_edge));
            }
            d.lay("travel", &ring.road_edge);
            for divider in &ring.dividers {
                d.divider(divider);
            }
            d.stand("median", ring.island_edge.clone(), height_of("median"), true);
            if track && v.selected.kind == Some("cycle") {
                d.outline(&ring.outer_edge, 0.0);
                d.outline(&ring.road_edge, 0.0);
            }
        }
    } else {
        d.lay("travel", &flatten(&v.core));
    }
    // Which way the ring runs, and what the bus lane across the middle of a plain one is.
    for a in ring_arrows(v) {
        d.glyph("", &plain_arrow(a.at, a.heading, a.size, RING_HEAD), a.size, 0.0);
    }
    if v.ring.as_ref().is_some_and(|r| r.lanes.is_empty()) && v.bus.is_some() {
        d.label("t-mark t-halo", (0.0, 0.0), 0.0, 5.0, &i18n.tr("jn-svg-bus-only", &Args::new()));
    }

    for c in &v.corners {
        // A corner that fails its check has its curb drawn in the warning's colour.
        if !c.ok || c.fast {
            d.mark_lines(true, "curb-line bad", true, &[flatten(&c.curb)], if c.walk { wall } else { 0.0 });
        }
        // A straight curb is no corner: nothing of it is pressed.
        let hit = if c.straight { String::new() } else { hit_attrs("corner", Some(c.uid), None) };
        d.pressing(hit.clone(), |d| {
            if c.walk {
                d.stand("sidewalk", flatten(&c.wedge), wall, true);
            }
            if let Some(p) = &c.protected {
                d.lay("bike", &flatten(&p.track));
                d.stand("sidewalk", flatten(&p.island), wall, true);
            }
        });
        if !c.straight {
            // Bare ground between two streets is a corner too: it is only a place to press.
            if !c.walk {
                d.press_area("iso-hit", &hit, &flatten(&c.wedge), 0.0, None);
            }
            if selected("corner", c.uid) {
                d.outline(&flatten(&c.wedge), if c.walk { wall } else { 0.0 });
            }
        }
    }

    for a in &v.arms {
        d.pressing(hit_attrs("arm", Some(a.uid), None), |d| {
            // The arm's end zone, then the taper and the zone across: a taper's quad stands or lies like any piece.
            for piece in a.pieces.iter().chain(&a.beyond) {
                let id = KINDS[piece.kind].id;
                let foot = flatten(&piece.poly);
                // A walkway is walking surface wall to wall, whatever stands on it.
                let h = if a.pedestrian { wall } else { height_of(id) };
                match h {
                    h if h > 0.0 => d.stand_in(id, piece.material, foot, h, true),
                    _ => d.lay_in(id, piece.material, &foot),
                }
            }
            // The street's curbside uses stand or lie on their piece, over it.
            for u in &a.curb_uses {
                let Some(poly) = &u.poly else { continue };
                let id = KINDS[u.kind].id;
                let foot = flatten(poly);
                let h = if a.pedestrian { wall } else { height_of(id) };
                // One that fails its check is outlined over what it stands on.
                if u.bad {
                    d.mark_ring(true, "curb-use curb-use-bad", false, &foot, h);
                }
                match h {
                    h if h > 0.0 => d.stand_in(id, u.material, foot, h, true),
                    _ => d.lay_in(id, u.material, &foot),
                }
            }
            // A turbo's entry is wider than its street, where the layout wants more lanes than the street has.
            if let Some(entry) = &a.turbo_entry {
                d.lay("travel", &flatten(entry));
            }
            // Where there is no parking beside the curb the road runs on, over the parking piece.
            for gap in &a.gaps {
                d.lay("travel", &flatten(gap));
            }
            // The bus lane and the queue jump run over the arm's pieces; a stop's bulb and an island stand like a sidewalk.
            let t = &a.transit;
            for lane in [&t.bus, &t.queue].into_iter().flatten() {
                d.lay("bus", &flatten(lane));
            }
            // The queue jump is a dashed lane.
            if let Some(queue) = &t.queue {
                d.mark_ring(false, "iso-queue", false, &flatten(queue), 0.0);
            }
            for poly in [&t.stop_poly, &t.island].into_iter().flatten() {
                d.stand("sidewalk", flatten(poly), wall, true);
            }
            for lane in &a.core_bike {
                d.lay("bike", &flatten(lane));
            }
            if let Some(b) = &a.bike_box {
                d.lay("bike", &flatten(b));
            }
            for bulb in a.bulbs.iter().flatten() {
                d.stand("sidewalk", flatten(bulb), wall, true);
            }
            if let Some(i) = a.crossing.as_ref().and_then(|c| c.island_poly.as_ref()) {
                // The island of a crossing is a part of the crossing.
                d.hit = hit_attrs("crossing", Some(a.uid), None);
                d.stand("sidewalk", flatten(i), wall, true);
            }
        });
        // An entering lane is pressed over the arm's pieces, as it is on the plan.
        let name = say(i18n, units, &a.label);
        for (i, lane) in a.lanes.iter().enumerate() {
            let on = selected("lane", a.uid) && v.selected.lane == i;
            let title = i18n.tr("jn-sel-lane", &Args::new().num("n", i as i64 + 1).num("count", a.lanes.len() as i64).str("arm", name.clone()));
            let class = if on { "lane-hit on" } else { "lane-hit" };
            d.press_area(class, &hit_attrs("lane", Some(a.uid), Some(i)), &flatten(&lane.poly), 0.0, Some(&title));
            if on {
                d.outline(&flatten(&lane.poly), 0.0);
            }
        }
    }

    // Surfaces raised to the sidewalk's level go over the road they are on, so they come after everything flat.
    if v.raised && v.ring.is_none() {
        d.table("sidewalk", &flatten(&v.core), wall);
    }
    for a in &v.arms {
        if let Some(r) = &a.raised {
            let top = flatten(&r.top);
            d.table("sidewalk", &top, wall);
            d.ramp("travel", &flatten(&r.ramp), &top, wall);
        }
        // The markings lie on the road, or on the table where it is raised.
        let level = if a.raised.is_some() { wall } else { 0.0 };
        // Only where traffic is controlled: the street that has the right of way has no line.
        if let Some([from, to]) = a.stop_line.filter(|_| a.role != "free") {
            d.stop_line(from, to, level, a.role == "yield");
        }
        if let Some(c) = a.crossing.as_ref().filter(|c| !c.continuous) {
            d.zebra(&flatten(&c.poly), level);
        }
        if let Some([end, _]) = a.stop_line {
            match a.role {
                "signal" => d.post(end, SIGNAL_POST_MM, "signal"),
                "stop" => d.post(end, SIGN_POST_MM, "stop"),
                _ => {}
            }
        }
        for b in &a.transit.bollards {
            d.post(*b, BOLLARD_MM, "bollard");
        }
        // The turn arrows on the lanes, and the small ones where the traffic leaves.
        let size = lane_arrow_size(&a.lanes);
        for lane in &a.lanes {
            d.glyph(if lane.bad { " bad" } else { "" }, &lane_arrow(lane, size), size, level);
        }
        for out in &a.leave_arrows {
            d.glyph(" out", &plain_arrow(out.at, out.heading as f64, LEAVE_ARROW_MM, LEAVE_HEAD), LEAVE_ARROW_MM, level);
        }
        draw_stops(&mut d, a, wall, level);
        draw_measures(&mut d, a, level);
        let on = selected("arm", a.uid);
        d.name(a.end, &say(i18n, units, &a.street), on);
        if on {
            d.outline(&flatten(&a.outline), 0.0);
        }
        let Some(c) = a.crossing.as_ref() else { continue };
        let top = flatten(&c.poly);
        let h = if c.continuous { wall } else { level };
        // How far across, on the junction side of the crossing.
        if let Some((strokes, at)) = crossing_dimension(&top, a.bearing as f64) {
            d.mark_lines(false, if c.too_far { "dim dim-warn" } else { "dim" }, c.too_far, &strokes, h);
            let number = |mm: i32| units.number_in(mm, i18n.locale());
            let text = if c.stages > 1 { format!("{} × {}", c.stages, number(c.stage_mm)) } else { number(c.distance_mm) };
            d.label(&format!("t-dim t-halo{}", if c.too_far { " t-warn" } else { "" }), at, h, 5.0, &text);
        }
        if c.continuous {
            d.table("sidewalk", &top, wall);
            for ramp in &c.ramps {
                d.ramp("sidewalk", &flatten(ramp), &top, wall);
            }
        }
        d.press_area("iso-hit", &hit_attrs("crossing", Some(a.uid), None), &top, h, None);
        if selected("crossing", a.uid) {
            d.outline(&top, h);
        }
    }

    // The turns of the street that is selected (or of a part of it), where they may go and where they may not.
    if v.selected.kind.is_some()
        && let Some(sa) = v.arms.iter().find(|a| a.uid == v.selected.uid)
    {
        let level = if v.raised && v.ring.is_none() { wall } else { 0.0 };
        for m in v.movements.iter().filter(|m| m.from == sa.uid) {
            let class = format!(
                "mv{}{}{}",
                if m.allowed { "" } else { " no" },
                if m.allowed && !m.lane { " bad" } else { "" },
                if m.indirect { " indirect" } else { "" }
            );
            let strokes = movement_strokes(&flatten(&m.path), m.allowed);
            d.mark_lines(true, &class, m.allowed && !m.lane, &strokes, level);
        }
    }

    IsoSvg { frame: f, markup: d.finish() }
}

/// An arrow's head, as a part of its length: a ring's, and where the traffic leaves.
const RING_HEAD: f64 = 0.2;
const LEAVE_HEAD: f64 = 0.19;

/// The street's transit stops on an arm: the platform over what it stands on, the bus's dwell and its run to the stop
/// line on the road, and the post.
fn draw_stops(d: &mut Draw, a: &ArmView, wall: f64, level: f64) {
    for st in &a.stops {
        let bad = if st.bad { " stop-bad" } else { "" };
        if let Some(pad) = &st.pad {
            let kind = match st.kind {
                StopKind::Curb => "",
                StopKind::Bulb => " stop-bulb",
                StopKind::Island => " stop-island",
            };
            d.mark_ring(true, &format!("stop-pad{kind}{bad}"), false, &flatten(pad), wall);
        }
        if let Some(dwell) = &st.dwell {
            d.mark_ring(false, &format!("stop-dwell{bad}"), false, &flatten(dwell), level);
        }
        if let Some(run) = &st.run {
            d.mark_lines(false, "stop-run", false, &[flatten(run)], level);
        }
        if let Some(post) = st.post {
            let (x, y) = d.f.at(post, wall);
            let shelter = if st.shelter {
                format!("<rect class=\"stop-shelter\" x=\"{}\" y=\"{}\" width=\"10\" height=\"10\"/>", f1(x - 5.0), f1(y - 5.0))
            } else {
                String::new()
            };
            write!(d.posts, "<g class=\"stop-post\">{shelter}<circle cx=\"{}\" cy=\"{}\" r=\"3.5\"/></g>", f1(x), f1(y)).unwrap();
        }
    }
}

/// The measures of an arm that are marks on the road: where a boundary's taper begins and ends, a virtual queue
/// jump's loop, a bus gate's line, a dead end's cap, and each measure's Atlas code.
fn draw_measures(d: &mut Draw, a: &ArmView, level: f64) {
    for [from, to] in &a.beyond_marks {
        d.mark_lines(false, "approach-mark", false, &[vec![*from, *to]], 0.0);
    }
    let t = &a.transit;
    if let Some(l) = &t.virtual_loop {
        d.mark_ring(false, "measure-loop", false, &flatten(l), level);
    }
    if let Some([from, to]) = t.gate {
        d.stop_line(from, to, level, t.gate_yields);
    }
    if let Some(c) = t.cap {
        d.mark_lines(false, "dead-cap", false, &[vec![c[0], c[1]]], level);
    }
    for tag in &t.tags {
        d.label("t-note t-halo measure-tag", tag.at, level, 4.0, tag.code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::junction::model::*;

    fn render(v: &JView) -> IsoSvg {
        iso_svg(v, &crate::i18n_for(crate::shared::i18n::Locale::En), 1000.0, 1000.0, Units::Metres)
    }

    fn svg(j: &Junction) -> IsoSvg {
        render(&j.view())
    }

    fn count(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    fn turbo(layout: usize) -> Junction {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT) && j.set_turbo(Some(layout)));
        j
    }

    #[test]
    fn a_turbo_roundabout_lays_its_road_and_paints_its_dividers_white() {
        let m = svg(&turbo(0)).markup;
        let paint = count(&m, "class=\"iso-paint white\"");
        assert!(paint > 0, "the divider is painted");
        assert_eq!(paint % 2, 0, "a double line is two lines of quads");
        assert!(m.find("class=\"iso-flat k-travel ").unwrap() < m.find("class=\"iso-paint white\"").unwrap(), "the paint lies on the road");
        assert!(!m.contains("iso-paint white\" data-face") && !m.contains("iso-paint white iso-top"), "paint is not a face");
        assert!(count(&svg(&turbo(3)).markup, "class=\"iso-paint white\"") > paint, "a spiral has two dividers");
        let mut plain = Junction::new(0);
        plain.set_control(ROUNDABOUT);
        assert_eq!(count(&svg(&plain).markup, "iso-paint"), 0);
    }

    #[test]
    fn a_turbo_roundabout_keeps_its_island_a_solid_and_its_cycle_track_round_the_road() {
        let mut j = turbo(0);
        assert!(j.set_cycle_track(true));
        let m = svg(&j).markup;
        assert!(count(&m, "iso-top k-median") >= 1);
        assert!(m.find("iso-flat k-bike").unwrap() < m.find("iso-flat k-travel").unwrap(), "the track is laid first, the road over it");
    }

    #[test]
    fn every_layout_and_block_draws_in_2_5d_without_a_missing_number() {
        // Four streets of three entering lanes of 3.5 m: a spiral's or rotor's three lanes need no more room than the street has.
        let mut j = junction_of(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        assert!(j.set_control(ROUNDABOUT));
        for layout in 0..5 {
            assert!(j.set_turbo(Some(layout)));
            for size in 0..4 {
                let _ = j.set_turbo_size(size); // the block it already has is no change
                assert_eq!(j.current().turbo.as_ref().unwrap().size, size, "layout {layout}");
                let m = svg(&j).markup;
                assert!(!m.contains("NaN") && !m.contains("inf"), "layout {layout} block {size}");
            }
        }
    }

    #[test]
    fn the_2_5d_picture_names_the_turbo_layout_and_size() {
        let j = turbo(0);
        let (en, fr) = (crate::i18n_for(crate::shared::i18n::Locale::En), crate::i18n_for(crate::shared::i18n::Locale::FrCa));
        assert_eq!(iso_label(&j.view(), &en), "2.5D view of the junction. 4 streets. Basic turbo roundabout, mini size.");
        assert_eq!(iso_label(&j.view(), &fr), "Vue en 2,5D de la jonction. 4 rues. Giratoire turbo de base, taille mini.");
    }

    fn depths(markup: &str) -> Vec<f64> {
        markup.split("data-depth=\"").skip(1).map(|rest| rest.split('"').next().unwrap().parse().unwrap()).collect()
    }

    #[test]
    fn a_walkway_arm_stands_at_sidewalk_height_whatever_its_pieces_are() {
        let j = junction_with_walkways(&[90], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)]);
        let s = svg(&j);
        assert!(count(&s.markup, "iso-top k-terrace") >= 1, "the terraces stand with the sidewalks");
        assert_eq!(count(&s.markup, "iso-flat k-terrace"), 0);
        assert!(count(&s.markup, "iso-flat k-travel") > 0, "the streets of traffic still lie flat");
        // the same terrace on a street of traffic lies flat
        let street = svg(&junction_of(&[("sidewalk", 2000), ("terrace", 1000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]));
        assert!(count(&street.markup, "iso-flat k-terrace") > 0);
    }

    fn walked() -> Junction {
        junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)])
    }

    #[test]
    fn a_sidewalk_stands_up_with_sides_and_a_road_lies_flat() {
        let s = svg(&walked());
        assert!(count(&s.markup, "iso-solid") >= 8, "two sidewalks on each of the arms");
        assert!(count(&s.markup, "class=\"iso-side k-sidewalk\"") > 0);
        assert!(count(&s.markup, "iso-top k-sidewalk") > 0);
        // A travel lane is drawn, but never with a side.
        assert!(count(&s.markup, "iso-flat k-travel") > 0);
        assert_eq!(count(&s.markup, "iso-side k-travel"), 0);
    }

    #[test]
    fn a_stretch_with_no_parking_beside_the_curb_is_drawn_as_road_over_the_parking() {
        let v = Junction::new(0).view();
        let s = render(&v);
        let arm = v.arms.iter().find(|a| !a.gaps.is_empty()).expect("the first sample has no-parking stretches beside its crossings");
        let d: String = flatten(&arm.gaps[0])
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (x, y) = s.frame.at(*p, 0.0);
                format!("{}{} {}", if i == 0 { "M" } else { "L" }, f1(x), f1(y))
            })
            .collect::<String>()
            + "Z";
        let hit = hit_attrs("arm", Some(arm.uid), None);
        let at = s.markup.find(&format!("class=\"iso-flat k-travel mat-asphalt\"{hit} d=\"{d}\"")).expect("the gap is drawn as road, and pressed as the arm");
        // It is painted after the arm's parking, which it covers.
        assert!(at > s.markup.find("iso-flat k-parking").unwrap());
    }

    fn frame() -> IsoFrame {
        IsoFrame::fit([-5000.0, -5000.0, 5000.0, 5000.0], &[(-5000.0, -5000.0), (5000.0, 5000.0)], 1000.0, 1000.0)
    }

    fn square(x0: f64, x1: f64) -> Vec<(f64, f64)> {
        vec![(x0, 0.0), (x1, 0.0), (x1, 1000.0), (x0, 1000.0)]
    }

    fn walls(d: Draw) -> usize {
        count(&d.finish(), "class=\"iso-side ")
    }

    #[test]
    fn no_wall_is_drawn_where_two_surfaces_at_the_same_height_meet_and_one_is_where_they_differ() {
        let mut one = Draw::new(frame());
        one.stand("sidewalk", square(0.0, 1000.0), 150.0, true);
        let alone = walls(one);
        assert!(alone > 0);
        let pair = |h: f64| {
            let mut d = Draw::new(frame());
            d.stand("sidewalk", square(0.0, 1000.0), 150.0, true);
            d.stand("median", square(1000.0, 2000.0), h, true);
            walls(d)
        };
        assert_eq!(pair(150.0), 2 * alone - 1, "the wall where they touch is not drawn");
        assert_eq!(pair(300.0), 2 * alone, "a step is a wall");
        let mut table = Draw::new(frame());
        table.stand("sidewalk", square(0.0, 1000.0), 150.0, true);
        table.table("sidewalk", &square(1000.0, 2000.0), 150.0);
        assert_eq!(walls(table), alone - 1, "nor is one where a surface level with it begins");
    }

    #[test]
    fn a_divider_is_drawn_as_the_open_line_it_is_and_never_joined_to_its_start() {
        let mut d = Draw::new(frame());
        d.divider(&[(-3_000.0, 0.0), (0.0, 3_000.0), (3_000.0, 0.0), (0.0, -3_000.0)]);
        assert_eq!(count(&d.finish(), "<path class=\"iso-paint white"), 2 * 3, "a double line of three segments: two lines of three quads, not four");
    }

    #[test]
    fn a_raised_junction_has_fewer_walls_than_its_sidewalks_have_edges_facing_the_viewer() {
        let flat = Junction::new(0).view();
        let mut j = Junction::new(0);
        assert!(j.set_raised(true));
        let raised = j.view();
        let (a, b) = (count(&render(&flat).markup, "class=\"iso-side "), count(&render(&raised).markup, "class=\"iso-side "));
        assert!(b < a, "raising the junction to the sidewalk's level takes walls away: {a} -> {b}");
    }

    #[test]
    fn a_zebra_crossing_has_stripes_and_a_stop_line_is_drawn_flat() {
        let mut j = Junction::new(0);
        let v = j.view();
        let m = render(&v).markup;
        let stops = v.arms.iter().filter(|a| a.stop_line.is_some()).count();
        assert!(stops > 0);
        assert_eq!(count(&m, "class=\"iso-stopline\""), stops);
        let zebras = v.arms.iter().filter_map(|a| a.crossing.as_ref()).filter(|c| !c.continuous).count();
        assert!(zebras > 0);
        let bars = count(&m, "class=\"iso-zebra\"");
        assert!(bars >= 3 * zebras, "{bars} stripes for {zebras} crossings");
        let arm = j.current().arms[0].uid;
        assert!(j.set_continuous(arm, true));
        let after = count(&render(&j.view()).markup, "class=\"iso-zebra\"");
        assert!(after < bars, "a continuous sidewalk has no stripes: {bars} -> {after}");
    }

    #[test]
    fn a_stop_line_is_drawn_only_where_traffic_is_controlled_and_dashed_where_it_yields() {
        let mut j = Junction::new(0);
        assert!(j.set_control(PRIORITY));
        let v = j.view();
        assert!(v.arms.iter().any(|a| a.role == "free" && a.stop_line.is_some()), "the priority street has a stop line to leave bare");
        let controlled = v.arms.iter().filter(|a| a.role != "free" && a.stop_line.is_some()).count();
        assert!(controlled > 0);
        assert_eq!(count(&render(&v).markup, "class=\"iso-stopline\""), controlled, "no line on the street that has the right of way");
        // Entering a roundabout gives way: the line is a row of dashes.
        assert!(j.set_control(ROUNDABOUT));
        let v = j.view();
        let yielding = v.arms.iter().filter(|a| a.role == "yield" && a.stop_line.is_some()).count();
        assert!(yielding > 0);
        let dashes = count(&render(&v).markup, "class=\"iso-stopline\"");
        assert!(dashes >= 2 * yielding, "{dashes} dashes for {yielding} yielding arms");
    }

    #[test]
    fn transit_measures_are_drawn_a_bus_queue_lane_a_stop_bulb_and_an_island() {
        let mut j = Junction::new(0);
        let e = j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid;
        let base = svg(&j).markup;
        assert!(j.set_bus_lane(e, true));
        assert!(j.set_approach(e, Q_CURB));
        let m = svg(&j).markup;
        assert!(count(&m, "iso-flat k-bus") > count(&base, "iso-flat k-bus"), "the bus lane and the queue jump");
        assert!(j.set_stop(e, STOP_BULB));
        let with_stop = count(&svg(&j).markup, "iso-top k-sidewalk");
        assert_eq!(with_stop, count(&m, "iso-top k-sidewalk") + 1, "the stop's bulb stands like a sidewalk");
        assert!(j.set_rule(e, RULE_RIRO));
        assert_eq!(count(&svg(&j).markup, "iso-top k-sidewalk"), with_stop + 1, "the right-in right-out island");
    }

    #[test]
    fn a_roundabout_draws_its_cycle_track_and_its_bus_lane() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let plain = svg(&j).markup;
        let (n, so) = (j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid, j.current().arms.iter().find(|a| a.bearing == 180).unwrap().uid);
        assert!(j.set_bus(Some((n, so))));
        assert!(j.set_cycle(Some(2_000)));
        let m = svg(&j).markup;
        assert!(count(&m, "iso-flat k-bike") > count(&plain, "iso-flat k-bike"), "the cycle track round the ring");
        assert!(count(&m, "iso-flat k-bus") > count(&plain, "iso-flat k-bus"), "the bus lane across it");
        assert!(!m.contains("NaN"));
    }

    #[test]
    fn a_junction_with_no_sidewalk_has_no_solid_sidewalks_or_sides() {
        let s = svg(&junction_of(&[("travel", 3200), ("travel", 3200)]));
        assert_eq!(count(&s.markup, "iso-side"), 0);
        assert_eq!(count(&s.markup, "k-sidewalk"), 0);
    }

    #[test]
    fn a_roundabout_draws_its_island_as_a_solid_and_no_number_is_missing() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let s = svg(&j);
        assert!(count(&s.markup, "iso-top k-median") >= 1);
        assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"));
    }

    #[test]
    fn solids_are_painted_far_to_near() {
        let d = depths(&svg(&walked()).markup);
        assert!(d.len() >= 8);
        assert!(d.windows(2).all(|w| w[0] <= w[1]), "{d:?}");
    }

    #[test]
    fn the_solid_farthest_from_the_viewer_is_painted_first_and_the_nearest_last() {
        let s = svg(&walked());
        let d = depths(&s.markup);
        let (min, max) = d.iter().fold((f64::MAX, f64::MIN), |(lo, hi), x| (lo.min(*x), hi.max(*x)));
        assert!(max - min > 1000.0, "the arms are spread in depth: {d:?}");
        assert_eq!((d[0], d[d.len() - 1]), (min, max));
    }

    /// FNV-1a over the whole 2.5D markup of every sample junction.
    fn iso_fingerprint() -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for sample in 0..JUNCTION_SAMPLES.len() {
            for b in svg(&Junction::new(sample)).markup.bytes() {
                h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    }

    #[test]
    fn an_arm_without_a_beyond_draws_as_before() {
        // Repinned for #140 stages 1 and 2, which gave the shapes the attributes a press reads and drew the parts that only
        // have a place to press (a lane, a crossing, a bare corner); no shape moved.
        let (got, pinned) = (iso_fingerprint(), 6_762_659_799_963_991_834);
        assert_eq!(
            got, pinned,
            "the 2.5D markup of the sample junctions no longer draws byte for byte as it did before the end-zone beyond was added (this pins that an arm without a beyond is untouched). If you changed how these junctions draw on purpose, look at a plan and a 2.5D of each sample junction first; only when the picture is what you meant, replace the pinned number with {got}."
        );
    }

    fn without_beyond(j: &Junction) -> JView {
        let mut bare = j.view();
        bare.arms.iter_mut().for_each(|a| a.beyond.clear());
        bare
    }

    #[test]
    fn a_pocket_arm_draws_the_taper_flat_in_the_iso() {
        let (j, _) = crate::junction::tests::junction_with_a_pocket(|_| {});
        let v = j.view();
        let a = v.arms.iter().find(|a| !a.beyond.is_empty()).unwrap();
        let lane = a.beyond.iter().find(|p| p.taper && KINDS[p.kind].id == "travel").expect("the pocket's quad");
        let (with, without) = (render(&v), render(&without_beyond(&j)));
        let flat = a.beyond.iter().filter(|p| height_of(KINDS[p.kind].id) == 0.0).count();
        assert_eq!(count(&with.markup, "iso-flat k-"), count(&without.markup, "iso-flat k-") + flat);
        let pts = flatten(&lane.poly);
        let d = |p: P, q: P| ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt();
        assert_eq!(pts.len(), 4);
        assert!(((d(pts[0], pts[3]) - d(pts[1], pts[2])).abs() > 1.0) || ((d(pts[0], pts[1]) - d(pts[2], pts[3])).abs() > 1.0), "not a rectangle: {pts:?}");
        let path = Draw::new(with.frame).path(&pts, 0.0);
        let hit = hit_attrs("arm", Some(a.uid), None);
        assert!(with.markup.contains(&format!("<path class=\"iso-flat k-travel mat-asphalt\"{hit} d=\"{path}\"/>")), "{path}");
    }

    #[test]
    fn a_sidewalk_stands_on_both_sides_of_the_boundary() {
        let (j, _) = crate::junction::tests::junction_with_a_pocket(|_| {});
        let v = j.view();
        let a = v.arms.iter().find(|a| !a.beyond.is_empty()).unwrap();
        let more = a.beyond.iter().filter(|p| !p.taper && KINDS[p.kind].id == "sidewalk").count();
        assert_eq!(more, 2, "a sidewalk each side beyond the boundary");
        let (with, without) = (render(&v).markup, render(&without_beyond(&j)).markup);
        assert_eq!(count(&with, "iso-top k-sidewalk"), count(&without, "iso-top k-sidewalk") + more);
        let planting = a.beyond.iter().filter(|p| KINDS[p.kind].id == "planting").count();
        assert!(planting > 0);
        assert_eq!(count(&with, "iso-top k-planting"), count(&without, "iso-top k-planting") + planting);
        // Splitting at the boundary gives the wall facing the viewer along the arm one more piece, and the boundary's own
        // ends, which face the viewer on one of the two, are not walled against the strip that continues them.
        assert_eq!(count(&with, "iso-side k-sidewalk"), count(&without, "iso-side k-sidewalk") + 1, "a wall drawn at the boundary would add more");
    }

    #[test]
    fn the_frame_holds_what_lies_beyond() {
        let (j, _) = crate::junction::tests::junction_with_a_pocket(|_| {});
        let v = j.view();
        let pts = content_points(&v);
        let a = v.arms.iter().find(|a| !a.beyond.is_empty()).unwrap();
        for p in a.beyond.iter().flat_map(|p| flatten(&p.poly)) {
            assert!(pts.contains(&p), "{p:?} is not among the points the frame fits");
        }
    }

    #[test]
    fn what_lies_beyond_stays_inside_the_drawing() {
        let (j, _) = crate::junction::tests::junction_with_a_pocket(|_| {});
        let (v, s) = (j.view(), svg(&j));
        let a = v.arms.iter().find(|a| !a.beyond.is_empty()).unwrap();
        for p in a.beyond.iter().flat_map(|p| flatten(&p.poly)) {
            for h in [0.0, height_of("sidewalk")] {
                let (x, y) = s.frame.at(p, h);
                assert!((0.0..=1000.0).contains(&x) && (0.0..=1000.0).contains(&y), "({x}, {y})");
            }
        }
    }

    #[test]
    fn every_sample_junction_draws_without_a_missing_number() {
        for sample in 0..4 {
            let s = svg(&Junction::new(sample));
            assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "sample {sample}");
            assert!(count(&s.markup, "iso-flat") + count(&s.markup, "iso-solid") > 0, "sample {sample}");
        }
    }

    /// The class of every surface drawn on the ground or standing, with the fill of the pattern layer that follows it.
    fn tops(markup: &str) -> Vec<(String, String)> {
        let paths: Vec<&str> = markup.split("<path class=\"").skip(1).collect();
        paths
            .windows(2)
            .filter(|w| w[0].starts_with("iso-flat ") || w[0].starts_with("iso-table ") || w[0].starts_with("iso-top "))
            .map(|w| (w[0].split('"').next().unwrap().to_string(), w[1].split("fill=\"").nth(1).unwrap().split('"').next().unwrap().to_string()))
            .collect()
    }

    #[test]
    fn every_top_is_in_a_material_with_its_pattern_over_it_and_the_kinds_own_when_it_has_none() {
        let m = svg(&walked()).markup;
        let tops = tops(&m);
        assert!(tops.len() > 5);
        for (class, fill) in &tops {
            let mat = class.split(" mat-").nth(1).unwrap_or_else(|| panic!("{class} has no material"));
            assert_eq!(fill, &format!("url(#iso-m-{mat})"), "{class}");
        }
        // The core is the junction's own surface: the default material of a travel lane.
        assert!(m.contains("class=\"iso-flat k-travel mat-asphalt\""));
        assert!(m.contains("class=\"iso-top k-sidewalk mat-"));
    }

    #[test]
    fn a_piece_s_material_colours_its_top_in_the_junction_too() {
        let mut v = Junction::new(0).view();
        let before = render(&v).markup;
        let piece = v.arms[0].pieces.iter_mut().find(|p| KINDS[p.kind].id == "sidewalk").expect("an arm has a sidewalk");
        let was = piece.material;
        piece.material = if was == "brick" { "concrete" } else { "brick" };
        let want = piece.material;
        let after = render(&v).markup;
        assert_ne!(before, after);
        assert_eq!(count(&after, &format!("iso-top k-sidewalk mat-{want}")), count(&before, &format!("iso-top k-sidewalk mat-{want}")) + 1);
        assert!(after.contains(&format!("fill=\"url(#iso-m-{want})\"")));
    }

    #[test]
    fn the_patterns_lie_on_the_ground_as_the_frame_projects_it() {
        let s = svg(&walked());
        let f = s.frame;
        let at = s.markup.find("id=\"iso-m-asphalt\"").expect("the drawing carries the patterns");
        let start = s.markup[..at].rfind("<pattern").unwrap();
        let tag = &s.markup[start..start + s.markup[start..].find('>').unwrap()];
        let n: Vec<f64> = tag.split("matrix(").nth(1).unwrap().split(')').next().unwrap().split(' ').map(|x| x.parse().unwrap()).collect();
        // A unit step along the plan's x lands where `at` puts it, from the origin.
        let (o, x, y) = (f.at((0.0, 0.0), 0.0), f.at((1.0, 0.0), 0.0), f.at((0.0, 1.0), 0.0));
        for (got, want) in [(n[0], x.0 - o.0), (n[1], x.1 - o.1), (n[2], y.0 - o.0), (n[3], y.1 - o.1), (n[4], o.0), (n[5], o.1)] {
            assert!((got - want).abs() < 0.01, "{n:?}");
        }
    }

    #[test]
    fn the_sides_and_ramps_are_shaded_by_the_drawing_s_own_filters_not_by_css() {
        // CSS `filter: brightness()` on SVG shapes may not be applied in Safari; an SVG filter is SVG 1.1.
        let mut j = walked();
        assert!(j.set_raised(true));
        let m = svg(&j).markup;
        for (id, slope) in [("iso-shade-ramp", "0.9"), ("iso-shade-side", "0.78")] {
            let at = m.find(&format!("<filter id=\"{id}\"")).unwrap_or_else(|| panic!("the drawing has no {id} filter"));
            let f = &m[at..at + m[at..].find("</filter>").unwrap()];
            assert!(f.contains("color-interpolation-filters=\"sRGB\""), "the {id} filter");
            assert_eq!(count(f, &format!("type=\"linear\" slope=\"{slope}\"")), 3, "the {id} filter");
        }
        assert!(m.find("<defs>").unwrap() < m.find("class=\"iso-flat").unwrap());
        assert_eq!(count(&m, "filter=\"url(#iso-shade-side)\""), count(&m, "class=\"iso-side "));
        assert_eq!(count(&m, "filter=\"url(#iso-shade-ramp)\""), count(&m, "class=\"iso-ramp "));
        assert!(count(&m, "class=\"iso-side ") > 0 && count(&m, "class=\"iso-ramp ") > 0);
        let css = include_str!("../../web/editor.css");
        assert!(!css.contains("brightness(0.9)") && !css.contains("brightness(0.78)"));
    }

    /// The `(x, y, anchor, text)` of each street name in the picture.
    fn names(markup: &str) -> Vec<(f64, f64, String, String)> {
        let attr = |tag: &str, key: &str| tag.split(&format!(" {key}=\"")).nth(1).unwrap().split('"').next().unwrap().to_string();
        markup
            .split("<text ")
            .skip(1)
            .filter(|t| t.contains("class=\"iso-name"))
            .map(|t| {
                (
                    attr(t, "x").parse().unwrap(),
                    attr(t, "y").parse().unwrap(),
                    attr(t, "text-anchor"),
                    t.split('>').nth(1).unwrap().split('<').next().unwrap().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn each_street_is_named_beyond_the_end_of_its_arm_and_the_names_are_painted_last() {
        for sample in 0..4 {
            let v = Junction::new(sample).view();
            let s = render(&v);
            let got = names(&s.markup);
            assert_eq!(got.len(), v.arms.len(), "sample {sample}");
            for a in &v.arms {
                let name = crate::shared::testing::en(&a.street);
                assert!(got.iter().any(|g| g.3 == name), "sample {sample}: {name}");
            }
            let first_name = s.markup.find("class=\"iso-name").unwrap();
            for earlier in ["class=\"iso-side ", "class=\"iso-post\"", "sel-box"] {
                assert!(s.markup.rfind(earlier).is_none_or(|at| at < first_name), "sample {sample}: {earlier}");
            }
        }
    }

    #[test]
    fn a_name_stands_farther_from_the_middle_of_the_junction_than_the_end_of_its_arm() {
        let v = Junction::new(0).view();
        let s = render(&v);
        let centre = s.frame.at((0.0, 0.0), 0.0);
        let far = |p: (f64, f64)| ((p.0 - centre.0).powi(2) + (p.1 - centre.1).powi(2)).sqrt();
        let got = names(&s.markup);
        for a in &v.arms {
            let name = crate::shared::testing::en(&a.street);
            let (x, y, _, _) = got.iter().find(|g| g.3 == name).unwrap();
            assert!(far((*x, *y)) > far(s.frame.at(a.end, 0.0)), "{name}");
        }
    }

    #[test]
    fn a_name_is_drawn_whole_inside_the_drawing_on_a_wide_and_a_narrow_screen() {
        for (width, sample) in [(1000.0, 0), (1000.0, 1), (1000.0, 2), (1000.0, 3), (360.0, 0), (360.0, 3)] {
            let v = Junction::new(sample).view();
            let s = iso_svg(&v, &crate::i18n_for(crate::shared::i18n::Locale::En), width, 800.0, Units::Metres);
            for (x, y, anchor, text) in names(&s.markup) {
                let w = text.chars().count() as f64 * NAME_CHAR_PX;
                let (lo, hi) = match anchor.as_str() {
                    "start" => (x, x + w),
                    "end" => (x - w, x),
                    _ => (x - w / 2.0, x + w / 2.0),
                };
                assert!(lo >= 0.0 && hi <= s.frame.width && y >= 0.0 && y <= s.frame.height, "{text} at {x},{y} ({anchor}) in {width} wide, sample {sample}");
            }
        }
    }

    #[test]
    fn only_the_selected_arm_s_name_is_blue() {
        let mut j = Junction::new(0);
        assert_eq!(count(&svg(&j).markup, "iso-name t-mark t-halo t-blue"), 0);
        let uid = j.current().arms[0].uid;
        j.select(Target::Arm(uid));
        assert_eq!(count(&svg(&j).markup, "iso-name t-mark t-halo t-blue"), 1);
    }

    #[test]
    fn only_the_selected_arm_is_outlined() {
        let mut j = Junction::new(0);
        assert_eq!(count(&svg(&j).markup, "sel-box"), 0);
        let uid = j.current().arms[0].uid;
        j.select(Target::Arm(uid));
        assert_eq!(count(&svg(&j).markup, "sel-box"), 1);
    }

    /// What a press on each pressable shape of the picture would select, in the way the view reads it (`plan::selected_by`).
    fn pressable(markup: &str) -> Vec<Target> {
        markup
            .split(" data-role=\"")
            .skip(1)
            .map(|rest| {
                let role = rest.split('"').next().unwrap();
                let tag = &rest[..rest.find('>').unwrap()];
                let number = |name: &str| tag.split(&format!("{name}=\"")).nth(1).and_then(|v| v.split('"').next()).and_then(|v| v.parse::<usize>().ok());
                crate::junction::plan::selected_by(role, number("data-uid").unwrap_or(0) as u32, number("data-lane").unwrap_or(0))
            })
            .collect()
    }

    /// Junctions that between them have every kind of thing the keys select: arms, lanes, crossings and corners (with
    /// and without a sidewalk to stand on), a roundabout's cycle track and bus lane, a turbo's cycle track.
    fn every_kind() -> Vec<(String, Junction)> {
        let mut all: Vec<(String, Junction)> = (0..JUNCTION_SAMPLES.len()).map(|i| (format!("sample {i}"), Junction::new(i))).collect();
        all.push(("no sidewalks".into(), junction_of(&[("travel", 3200), ("travel", 3200)])));
        all.push(("walkways".into(), junction_with_walkways(&[90], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)])));
        let mut round = Junction::new(0);
        round.set_control(ROUNDABOUT);
        let at = |j: &Junction, bearing: i32| j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid;
        let (n, so) = (at(&round, 0), at(&round, 180));
        assert!(round.set_bus(Some((n, so))) && round.set_cycle(Some(2_000)));
        all.push(("roundabout".into(), round));
        let mut basic = turbo(0);
        assert!(basic.set_cycle_track(true));
        all.push(("turbo".into(), basic));
        let mut raised = Junction::new(0);
        assert!(raised.set_raised(true));
        all.push(("raised".into(), raised));
        all
    }

    #[test]
    fn every_part_the_keys_step_through_can_be_pressed_in_the_picture() {
        for (name, mut j) in every_kind() {
            let pressable = pressable(&svg(&j).markup);
            let targets = j.targets();
            assert!(!targets.is_empty(), "{name}");
            for t in targets {
                j.select(t);
                assert_eq!(j.selected, t, "{name}: {t:?} is not kept as the selection");
                assert!(pressable.contains(&t), "{name}: nothing in the picture selects {t:?}");
            }
        }
    }

    #[test]
    fn what_is_only_looked_at_selects_nothing_and_a_bare_corner_is_still_pressed() {
        // The core, the paint, the posts and the names say nothing about what a press selects.
        let m = svg(&Junction::new(0)).markup;
        for class in ["iso-flat k-travel mat-asphalt\" d=", "iso-zebra", "iso-stopline", "iso-post", "iso-head", "iso-name", "hatch"] {
            for part in m.split(&format!("class=\"{class}")).skip(1) {
                let tag = &part[..part.find('>').unwrap()];
                assert!(!tag.contains("data-role"), "{class}: {tag}");
            }
        }
        // A corner without a walkway beside it is bare ground: it is still pressed to be selected.
        let bare = svg(&junction_of(&[("travel", 3200), ("travel", 3200)])).markup;
        assert!(bare.contains("class=\"iso-hit\" data-role=\"corner\""), "{bare}");
    }

    #[test]
    fn a_shape_that_stands_carries_what_a_press_on_it_selects() {
        let m = svg(&walked()).markup;
        let solids: Vec<&str> = m.split("<g class=\"iso-solid\" ").skip(1).collect();
        assert!(!solids.is_empty());
        let tag = |g: &&str| g.split('>').next().unwrap().to_string();
        assert!(solids.iter().map(tag).any(|t| t.contains("data-role=\"arm\" data-uid=\"")), "a sidewalk of an arm");
        assert!(solids.iter().map(tag).any(|t| t.contains("data-role=\"corner\" data-uid=\"")), "a corner's sidewalk");
    }

    #[test]
    fn every_selected_part_is_outlined_in_two_tones_and_nothing_else_is() {
        for (name, mut j) in every_kind() {
            assert_eq!(count(&svg(&j).markup, "iso-sel"), 0, "{name}: nothing selected, nothing outlined");
            for t in j.targets() {
                j.select(t);
                let m = svg(&j).markup;
                let lines = if t == Target::Cycle { 2 } else { 1 };
                assert_eq!(count(&m, "class=\"sel-box iso-sel\""), lines, "{name}: {t:?}");
                assert_eq!(count(&m, "class=\"iso-sel-casing\""), lines, "{name}: {t:?}");
                assert!(!m.contains("NaN") && !m.contains("inf"), "{name}: {t:?}");
                // The casing is under the line, and both are over everything that stands.
                assert!(m.find("iso-sel-casing").unwrap() < m.find("sel-box iso-sel").unwrap(), "{name}: {t:?}");
                assert!(m.find("iso-sel-casing").unwrap() > m.rfind("class=\"iso-solid\"").unwrap_or(0), "{name}: {t:?}");
            }
        }
    }

    #[test]
    fn a_selected_lane_is_shaded_and_named_and_the_others_are_not() {
        let mut j = Junction::new(0);
        let arm = j.current().arms[0].uid;
        assert!(j.current().arms[0].lanes.len() >= 2);
        let m = svg(&j).markup;
        assert_eq!(count(&m, "class=\"lane-hit\""), j.current().arms.iter().map(|a| a.lanes.len()).sum::<usize>());
        assert_eq!(count(&m, "lane-hit on"), 0);
        assert!(m.contains("<title>Lane 1 of "), "a lane has its name for the pointer");
        j.select(Target::Lane(arm, 1));
        let m = svg(&j).markup;
        assert_eq!(count(&m, "class=\"lane-hit on\" data-role=\"lane\" data-uid=\""), 1);
        assert!(m.contains(&format!("data-uid=\"{arm}\" data-lane=\"1\"")));
    }

    #[test]
    fn a_corner_is_outlined_on_the_height_it_stands_at() {
        let mut j = walked();
        let c = j.view().corners.iter().find(|c| c.walk && !c.straight).map(|c| c.uid).expect("a corner with a sidewalk");
        j.select(Target::Corner(c));
        let (v, s) = (j.view(), svg(&j));
        let wedge = flatten(&v.corners.iter().find(|k| k.uid == c).unwrap().wedge);
        let d = Draw::new(s.frame).path(&wedge, height_of("sidewalk"));
        assert!(s.markup.contains(&format!("class=\"sel-box iso-sel\" d=\"{d}\"")), "{d}");
    }

    #[test]
    fn a_signal_has_a_head_on_a_post_on_each_entering_arm_and_no_control_has_none() {
        let mut j = Junction::new(0);
        assert_eq!(j.current().control, SIGNAL, "the first sample is signalled");
        let signal = svg(&j);
        assert!(count(&signal.markup, "iso-post") >= 1);
        assert!(count(&signal.markup, "iso-head signal") >= 1);
        assert!(j.set_control(UNCONTROLLED));
        assert_eq!(count(&svg(&j).markup, "iso-post"), 0);
    }

    #[test]
    fn an_all_way_stop_has_a_sign_on_each_entering_arm() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ALL_WAY_STOP));
        assert!(count(&svg(&j).markup, "iso-head stop") >= 1);
    }

    #[test]
    fn the_vertices_of_a_ramp_that_touch_the_top_are_lifted_and_the_others_stay_down() {
        let top = vec![(0.0, 0.0), (1000.0, 0.0), (1000.0, 1000.0), (0.0, 1000.0)];
        let ramp = vec![(0.0, 1000.0), (1000.0, 1000.0), (1000.0, 2000.0), (0.0, 2000.0)];
        let lifted = lift_to(&ramp, &top, 150.0);
        assert_eq!(lifted.iter().map(|(_, h)| *h).collect::<Vec<_>>(), vec![150.0, 150.0, 0.0, 0.0]);
    }

    #[test]
    fn a_raised_junction_has_a_top_at_sidewalk_height_and_sloped_ramps() {
        let mut j = Junction::new(0);
        assert_eq!(count(&svg(&j).markup, "iso-ramp"), 0);
        assert!(j.set_raised(true));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "iso-table"), 5, "the core and four arms");
        assert_eq!(count(&s.markup, "class=\"iso-ramp "), 4);
        // The top has no wall round it.
        assert_eq!(count(&s.markup, "iso-side k-travel"), 0);
    }

    #[test]
    fn a_continuous_crossing_is_sidewalk_height_with_ramps_and_a_zebra_crossing_is_flat() {
        let mut j = Junction::new(0);
        let arm = j.current().arms[0].uid;
        let flat = svg(&j);
        assert_eq!(count(&flat.markup, "iso-ramp"), 0);
        assert_eq!(count(&flat.markup, "iso-table"), 0);
        assert!(j.set_continuous(arm, true));
        let s = svg(&j);
        assert_eq!(count(&s.markup, "iso-table"), 1);
        assert_eq!(count(&s.markup, "class=\"iso-ramp "), 2, "the strip before the crossing and the one beyond it");
    }

    #[test]
    fn the_origin_stays_put_and_height_lifts_it_by_the_exaggeration() {
        assert_eq!(project((0.0, 0.0), 0.0), (0.0, 0.0));
        assert_eq!(project((0.0, 0.0), 150.0), (0.0, -450.0));
    }

    #[test]
    fn a_point_is_rotated_then_its_depth_squashed() {
        let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
        let p = project((1000.0, 0.0), 0.0);
        assert!((p.0 - 1000.0 * c).abs() < 1e-9);
        assert!((p.1 - 1000.0 * s * SQUASH).abs() < 1e-9);
        assert!((view_point((0.0, 1000.0)).1 - 1000.0 * c).abs() < 1e-9);
    }

    #[test]
    fn path_commands_flatten_to_a_polyline_that_ends_where_the_path_ends() {
        let cmds = vec![
            json!(["M", 0.0, 0.0]),
            json!(["L", 1000.0, 0.0]),
            json!(["Q", 1000.0, 1000.0, 0.0, 1000.0]),
            json!(["A", 500.0, 0, 1, 0.0, 0.0]),
            json!(["Z"]),
        ];
        let pts = flatten(&cmds);
        assert_eq!(pts[0], (0.0, 0.0));
        assert_eq!(pts[1], (1000.0, 0.0));
        assert!(pts.len() > 10);
        let last = *pts.last().unwrap();
        assert!(last.0.abs() < 1e-6 && last.1.abs() < 1e-6);
    }

    #[test]
    fn the_whole_junction_and_its_tallest_post_are_inside_the_drawing() {
        for width in [320.0, 400.0, 1000.0] {
            let b = [-20_000.0, -20_000.0, 20_000.0, 20_000.0];
            let corners = [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])];
            let f = IsoFrame::fit(b, &corners, width, 1000.0);
            for corner in corners {
                for h in [0.0, TALLEST_MM] {
                    let (x, y) = f.at(corner, h);
                    assert!(x >= 0.0 && x <= f.width && y >= 0.0 && y <= f.height, "{width}: {corner:?} {h} -> {x},{y}");
                }
            }
        }
    }

    #[test]
    fn the_drawing_is_as_wide_and_as_tall_as_the_plans() {
        let b = [-20_000.0, -20_000.0, 20_000.0, 20_000.0];
        let f = IsoFrame::fit(b, &[(0.0, 0.0), (1000.0, 1000.0)], 1000.0, 1000.0);
        let plan = crate::junction::frame::Frame::fit(b, 1000.0, 1000.0);
        assert_eq!((f.width, f.height), (plan.width, plan.height));
    }

    #[test]
    fn the_junction_fills_most_of_the_drawing_and_none_of_it_is_outside() {
        for sample in 0..4 {
            let j = Junction::new(sample);
            let v = j.view();
            let s = render(&v);
            let pts: Vec<(f64, f64)> = v.arms.iter().flat_map(|a| flatten(&a.outline)).map(|p| s.frame.at(p, 0.0)).collect();
            let (min_x, max_x) = pts.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.0), hi.max(p.0)));
            let (min_y, max_y) = pts.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.1), hi.max(p.1)));
            assert!(min_x >= 0.0 && max_x <= s.frame.width && min_y >= 0.0 && max_y <= s.frame.height, "sample {sample}");
            let (across, down) = ((max_x - min_x) / s.frame.width, (max_y - min_y) / s.frame.height);
            assert!(across > 0.6 || down > 0.6, "sample {sample}: {across:.2} across, {down:.2} down");
        }
    }

    #[test]
    fn every_kind_of_piece_has_a_height() {
        for k in crate::shared::catalogue::KINDS.iter() {
            let h = height_of(k.id);
            assert!((0.0..=TALLEST_MM).contains(&h), "{}", k.id);
        }
        assert_eq!(height_of("travel"), 0.0);
        assert_eq!(height_of("sidewalk"), 150.0);
        assert_eq!(height_of("median"), 150.0);
        assert_eq!(height_of("planting"), 150.0);
    }
}
