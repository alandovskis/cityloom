//! The junction in 2.5D, as SVG markup, from the model's view: a read-only picture of what the plan draws flat.
//! A plan point is rotated by a fixed angle, its depth squashed, and its height lifted up the screen; there is one
//! viewpoint, so the shapes are painted far to near with no depth buffer.

use std::fmt::Write;

use serde_json::Value;

use crate::junction::frame::Frame;
use crate::junction::geometry::{sample_arc, sample_quad};
use crate::junction::plan_svg::turbo_name;
use crate::junction::read_model::JView;
use crate::junction::text::arms_text;
use crate::shared::catalogue::KINDS;
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::{esc, f1};
use crate::shared::iso::paint::{Paint, Pattern, Style, class, quads};
use crate::shared::iso::shading::shade_filters;
use crate::shared::said::say;
use crate::shared::units::Units;

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
    foot: Vec<P>,
    h_mm: f64,
    sides: bool,
}

/// The filters that darken the faces turned from the eye: a ramp a little, a side more. SVG filters, since CSS
/// `filter: brightness()` on SVG shapes may not be applied in Safari.
fn shades() -> String {
    format!("<defs>{}</defs>", shade_filters(&[("iso-shade-ramp", "0.9"), ("iso-shade-side", "0.78")]))
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
    /// Posts with their heads, painted over the solids.
    posts: String,
    /// The outline of the selected arm.
    sel: String,
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
        Draw { f, flat: String::new(), solids: Vec::new(), tables: Vec::new(), posts: String::new(), sel: String::new(), names: String::new() }
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

    /// A shape lying on the ground.
    fn lay(&mut self, kind_id: &str, foot: &[P]) {
        if foot.len() < 3 {
            return;
        }
        let d = self.path(foot, 0.0);
        write!(self.flat, "<path class=\"iso-flat k-{kind_id}\" d=\"{d}\"/><path class=\"hatch\" fill=\"url(#h-{kind_id})\" d=\"{d}\"/>").unwrap();
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

    /// A solid double white line along `path` (a closed ring of points, which this joins to its start), lying on the road.
    /// The quads come from the shared paint and are projected here, like the road they lie on.
    fn divider(&mut self, path: &[P]) {
        let mut line = path.to_vec();
        if let Some(&first) = path.first() {
            line.push(first);
        }
        for (paint, quad) in quads(&line, Style { paint: Paint::White, pattern: Pattern::Solid, double: true }) {
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
        write!(self.flat, "<path class=\"iso-table iso-flat k-{kind_id}\" d=\"{d}\"/><path class=\"hatch\" fill=\"url(#h-{kind_id})\" d=\"{d}\"/>").unwrap();
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
        if foot.len() >= 3 {
            self.solids.push(Solid { kind_id, foot, h_mm, sides });
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
        let mut g = format!("<g class=\"iso-solid\" data-depth=\"{}\">", f1(Self::depth(s)));
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
        write!(g, "<path class=\"iso-top k-{id}\" d=\"{d}\"/><path class=\"hatch\" fill=\"url(#h-{id})\" d=\"{d}\"/></g>").unwrap();
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

    /// The outline of a shape lying on the ground, to show what is selected.
    fn outline(&mut self, foot: &[P]) {
        if foot.len() >= 3 {
            write!(self.sel, "<path class=\"sel-box\" d=\"{}\"/>", self.path(foot, 0.0)).unwrap();
        }
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

    fn finish(mut self) -> String {
        self.solids.sort_by(|a, b| Self::depth(a).total_cmp(&Self::depth(b)));
        let solids: String = (0..self.solids.len()).map(|i| self.render(i)).collect();
        format!("{}<g class=\"iso\">{}{solids}{}{}{}</g>", shades(), self.flat, self.posts, self.sel, self.names)
    }
}

/// The points the picture reaches on the ground: each arm from its mouth to its end, the core, and a
/// roundabout's ring. Everything drawn is inside them.
fn content_points(v: &JView) -> Vec<P> {
    let mut pts: Vec<P> = v.arms.iter().flat_map(|a| flatten(&a.outline)).chain(flatten(&v.core)).collect();
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

/// The junction in 2.5D, in a drawing `width` wide in a window `window_height` high.
pub fn iso_svg(v: &JView, i18n: &I18n, width: f64, window_height: f64, units: Units) -> IsoSvg {
    let f = IsoFrame::fit(v.bounds, &content_points(v), width, window_height);
    let mut d = Draw::new(f);
    let wall = height_of("sidewalk");

    // the carriageway where the streets meet, and a roundabout's ring and island
    if let Some(ring) = &v.ring {
        if ring.lanes.is_empty() {
            let circle = |r: f64| -> Vec<P> {
                (0..48)
                    .map(|i| {
                        let t = i as f64 * std::f64::consts::TAU / 48.0;
                        (r * t.cos(), r * t.sin())
                    })
                    .collect()
            };
            if ring.cycle_mm.is_some_and(|c| c != 0) {
                // The cycle track is the band outside the carriageway: the whole ring in its colour, the carriageway over it.
                d.lay("bike", &circle(ring.radius_mm as f64));
                d.lay("travel", &circle(ring.road_mm as f64));
            } else {
                d.lay("travel", &circle(ring.radius_mm as f64));
            }
            if let Some(bus) = &v.bus {
                d.lay("bus", &flatten(&bus.poly));
            }
            d.stand("median", circle(ring.island_mm as f64), height_of("median"), true);
        } else {
            // A turbo: the carriageway (the lanes and the 300 mm between them) is one travel surface, the dividers are paint on it.
            if ring.cycle_mm.is_some_and(|c| c != 0) {
                d.lay("bike", &ring.outer_edge);
            }
            d.lay("travel", &ring.road_edge);
            for divider in &ring.dividers {
                d.divider(divider);
            }
            d.stand("median", ring.island_edge.clone(), height_of("median"), true);
        }
    } else {
        d.lay("travel", &flatten(&v.core));
    }

    for c in &v.corners {
        if c.walk {
            d.stand("sidewalk", flatten(&c.wedge), wall, true);
        }
        if let Some(p) = &c.protected {
            d.lay("bike", &flatten(&p.track));
            d.stand("sidewalk", flatten(&p.island), wall, true);
        }
    }

    for a in &v.arms {
        for piece in &a.pieces {
            let id = KINDS[piece.kind].id;
            let foot = flatten(&piece.poly);
            // A walkway is walking surface wall to wall, whatever stands on it.
            let h = if a.pedestrian { wall } else { height_of(id) };
            match h {
                h if h > 0.0 => d.stand(id, foot, h, true),
                _ => d.lay(id, &foot),
            }
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
            d.stand("sidewalk", flatten(i), wall, true);
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
        let selected = v.selected.kind == Some("arm") && v.selected.uid == a.uid;
        d.name(a.end, &say(i18n, units, &a.street), selected);
        if selected {
            d.outline(&flatten(&a.outline));
        }
        let Some(c) = a.crossing.as_ref().filter(|c| c.continuous) else { continue };
        let top = flatten(&c.poly);
        d.table("sidewalk", &top, wall);
        for ramp in &c.ramps {
            d.ramp("sidewalk", &flatten(ramp), &top, wall);
        }
    }

    IsoSvg { frame: f, markup: d.finish() }
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
        assert!(m.find("class=\"iso-flat k-travel\"").unwrap() < m.find("class=\"iso-paint white\"").unwrap(), "the paint lies on the road");
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
                let _ = j.set_turbo_size(size);
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
        let at = s.markup.find(&format!("class=\"iso-flat k-travel\" d=\"{d}\"")).expect("the gap is drawn as road");
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

    #[test]
    fn every_sample_junction_draws_without_a_missing_number() {
        for sample in 0..4 {
            let s = svg(&Junction::new(sample));
            assert!(!s.markup.contains("NaN") && !s.markup.contains("inf"), "sample {sample}");
            assert!(count(&s.markup, "iso-flat") + count(&s.markup, "iso-solid") > 0, "sample {sample}");
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
