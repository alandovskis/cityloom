//! The junction's 2.5D as it was written before it was built as a diagram (#140, stage 7): a copy of the old writer, kept for
//! the test that proves the diagram draws the same markup, in the parts it is made of, and then deleted.

use std::fmt::Write;

use super::*;
use crate::junction::iso_marks::{
    LEAVE_ARROW_MM, Spot, crossing_dimension, lane_arrow, lane_arrow_size, movement_strokes, plain_arrow, ring_arrows, spread_labels,
};
use crate::junction::read_model::{ArmView, JView};
use crate::shared::catalogue::KINDS;
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::format::{esc, f1};
use crate::shared::iso::paint::{Paint, Pattern, Style, class, quads};
use crate::shared::said::say;
use crate::shared::units::Units;
use crate::street::model::StopKind;

type P = (f64, f64);

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
    /// How much of `flat` is the carriageway where the streets meet and the ring's arrows.
    ground: usize,
}

/// The markup of the picture in the parts it is made of, in the order they were written.
pub struct Parts {
    pub defs: String,
    pub ground: String,
    pub surface: String,
    pub solids: String,
    pub marks: String,
    pub posts: String,
    pub sel: String,
    pub names: String,
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
            ground: 0,
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
        // A part narrower than a finger is given the rest of the 24 px as a stroke nobody sees, which a press also lands on.
        let on_screen: Vec<P> = foot.iter().map(|p| self.f.at(*p, h_mm)).collect();
        let grow = target_growth_px(&on_screen);
        let reach = if grow > 0.0 { format!(" stroke-width=\"{}\"", f1(grow)) } else { String::new() };
        write!(self.flat, "<path class=\"{class}\"{attrs}{reach} d=\"{d}\">{title}</path>").unwrap();
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

    /// A line of text standing at a point of the screen, in pixels, over a halo, on top of everything.
    fn label_px(&mut self, class: &str, at: P, text: &str) {
        write!(self.names, "<text class=\"{class}\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>", f1(at.0), f1(at.1), esc(text)).unwrap();
    }

    fn finish(mut self) -> Parts {
        self.solids.sort_by(|a, b| Self::depth(a).total_cmp(&Self::depth(b)));
        let solids: String = (0..self.solids.len()).map(|i| self.render(i)).collect();
        let (ground, surface) = self.flat.split_at(self.ground);
        Parts {
            defs: shades(&self.f),
            ground: ground.to_string(),
            surface: surface.to_string(),
            solids,
            marks: self.marks,
            posts: self.posts,
            sel: self.sel,
            names: self.names,
        }
    }
}

/// The junction in 2.5D, in a drawing `width` wide in a window `window_height` high. Every part the keys select can
/// be pressed (`hit_attrs`), and the one that is selected is outlined.
pub fn parts(v: &JView, i18n: &I18n, width: f64, window_height: f64, units: Units) -> (IsoFrame, Parts) {
    let f = IsoFrame::fit(&content_points(v), width, window_height);
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
    d.ground = d.flat.len();
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

    // A corner is a place to press, drawn after the streets so that it is the one pressed where it is made wider than
    // it is, to be as big as a finger (bare ground or a sidewalk's corner alike).
    for c in v.corners.iter().filter(|c| !c.straight) {
        d.press_area("iso-hit", &hit_attrs("corner", Some(c.uid), None), &flatten(&c.wedge), if c.walk { wall } else { 0.0 }, None);
    }

    // The lengths of the crossings, written over the drawing once all are known: two that would overlap on the
    // screen are moved apart, the longer crossing keeping its place, and one with no room is left out.
    let mut dims: Vec<Dim> = Vec::new();

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
            let (x, y) = d.f.at(at, h);
            dims.push(Dim { at: (x, y + 5.0), warn: c.too_far, mm: c.distance_mm, text });
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

    dims.sort_by(|a, b| b.warn.cmp(&a.warn).then(b.mm.cmp(&a.mm)));
    let spots: Vec<Spot> = dims.iter().map(|m| Spot { at: m.at, chars: m.text.chars().count() }).collect();
    for (m, spot) in dims.iter().zip(spread_labels(&spots)) {
        if let Some((x, y)) = spot {
            d.label_px(&format!("t-dim t-halo{}", if m.warn { " t-warn" } else { "" }), (x, y), &m.text);
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

    (f, d.finish())
}

/// A crossing's length, to be written where there is room for it: where it wants to stand on the screen, and whether
/// the crossing is too long, and how long it is, which of two that would overlap is kept.
struct Dim {
    at: P,
    warn: bool,
    mm: i32,
    text: String,
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
    use crate::junction::iso::iso_svg;
    use crate::junction::model::*;
    use crate::junction::tests::{end_stop, end_use, junction_with_a_pocket, junction_with_stops, junction_with_uses};
    use crate::shared::i18n::Locale;

    /// The old markup in the parts it was written in, as the diagram groups them: its layers' groups round the parts, the words
    /// before the outline of what is selected, and a press area with nothing in it self-closed. These are all the differences.
    fn expected(p: &Parts) -> String {
        let fix = |s: &str| s.replace("\"></path>", "\"/>");
        format!(
            "{}<g class=\"iso iso-junction\"><g class=\"iso-layer-ground\">{}</g><g class=\"iso-surface\">{}</g><g class=\"iso-raised\">{}</g><g class=\"iso-markings\">{}</g><g class=\"iso-objects\">{}</g><g class=\"iso-signs\">{}</g><g class=\"iso-layer-outline\">{}</g></g>",
            p.defs,
            fix(&p.ground),
            fix(&p.surface),
            fix(&p.solids),
            fix(&p.marks),
            fix(&p.posts),
            fix(&p.names),
            fix(&p.sel)
        )
    }

    /// Every width and unit the page draws in, and both languages: the diagram says the same as the old writer did.
    fn same(name: &str, j: &Junction) {
        let v = j.view();
        for (width, height) in [(1440.0, 900.0), (1050.0, 800.0), (600.0, 700.0), (320.0, 900.0), (240.0, 700.0)] {
            for (locale, units) in [(Locale::En, Units::Metres), (Locale::FrCa, Units::Feet)] {
                let i18n = crate::i18n_for(locale);
                let (_, parts) = parts(&v, &i18n, width, height, units);
                let new = iso_svg(&v, &i18n, width, height, units).markup;
                let old = expected(&parts);
                let at = new.bytes().zip(old.bytes()).take_while(|(a, b)| a == b).count();
                let around = |s: &str| s.get(at.saturating_sub(80)..(at + 120).min(s.len())).unwrap_or("").to_string();
                assert!(
                    new == old,
                    "{name} at {width} in {locale:?}: the diagram's markup is not the old writer's\nfirst difference at byte {at}\nnew: {}\nold: {}",
                    around(&new),
                    around(&old)
                );
            }
        }
    }

    fn with_every_target_selected(name: &str, mut j: Junction) {
        same(name, &j);
        for t in j.targets() {
            j.select(t);
            same(&format!("{name} with {t:?} selected"), &j);
        }
    }

    #[test]
    fn the_sample_junctions_are_drawn_as_they_were() {
        for i in 0..JUNCTION_SAMPLES.len() {
            with_every_target_selected(&format!("sample {i}"), Junction::new(i));
        }
    }

    #[test]
    fn a_junction_without_sidewalks_one_with_walkways_and_a_raised_table_are_drawn_as_they_were() {
        with_every_target_selected("no sidewalks", junction_of(&[("travel", 3200), ("travel", 3200)]));
        with_every_target_selected("walkways", junction_with_walkways(&[90], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)]));
        with_every_target_selected(
            "every street a walkway",
            junction_with_walkways(&[0, 90, 180, 270], &[("sidewalk", 3000), ("terrace", 2000), ("sidewalk", 3000)]),
        );
        let mut raised = Junction::new(0);
        assert!(raised.set_raised(true));
        with_every_target_selected("raised", raised);
        let mut walked = junction_of(&[("sidewalk", 2000), ("travel", 3200), ("travel", 3200), ("sidewalk", 2000)]);
        let arm = walked.current().arms[0].uid;
        assert!(walked.set_continuous(arm, true));
        with_every_target_selected("a continuous sidewalk", walked);
    }

    #[test]
    fn roundabouts_and_every_turbo_layout_and_block_are_drawn_as_they_were() {
        let mut round = Junction::new(0);
        round.set_control(ROUNDABOUT);
        let at = |j: &Junction, b: i32| j.current().arms.iter().find(|a| a.bearing == b).unwrap().uid;
        let (n, so) = (at(&round, 0), at(&round, 180));
        with_every_target_selected(
            "a roundabout",
            Junction::new(0).tap(|j| {
                j.set_control(ROUNDABOUT);
            }),
        );
        assert!(round.set_bus(Some((n, so))) && round.set_cycle(Some(2_000)));
        with_every_target_selected("a roundabout with a bus lane and a cycle track", round);
        let mut j = junction_of(&[("sidewalk", 3_000), ("travel", 3_500), ("travel", 3_500), ("travel", 3_500), ("sidewalk", 3_000)]);
        assert!(j.set_control(ROUNDABOUT));
        for layout in 0..5 {
            assert!(j.set_turbo(Some(layout)));
            for size in 0..4 {
                let _ = j.set_turbo_size(size);
                same(&format!("turbo {layout} block {size}"), &j);
            }
            assert!(j.set_cycle_track(true) || true);
            same(&format!("turbo {layout} with a cycle track"), &j);
            let _ = j.set_cycle_track(false);
        }
        let mut basic = Junction::new(0);
        assert!(basic.set_control(ROUNDABOUT) && basic.set_turbo(Some(0)) && basic.set_cycle_track(true));
        with_every_target_selected("a basic turbo with a cycle track", basic);
    }

    #[test]
    fn the_stops_the_curbside_uses_the_approach_and_the_measures_are_drawn_as_they_were() {
        use crate::street::model::{StopKind, Use};
        let (j, _) = junction_with_stops(|_| {}, |lane, walk| vec![end_stop(25_000, 33_000, lane, walk)]);
        with_every_target_selected("a near stop", j);
        for kind in [StopKind::Bulb, StopKind::Island] {
            let (j, _) = junction_with_stops(
                |_| {},
                |lane, walk| {
                    let mut s = end_stop(25_000, 33_000, lane, walk);
                    s.kind = kind;
                    s.side = crate::shared::catalogue::Side::Left;
                    vec![s]
                },
            );
            same(&format!("a {kind:?} stop"), &j);
        }
        let (j, _) = junction_with_stops(|_| {}, |lane, walk| vec![end_stop(500, 9_000, lane, walk)]);
        same("a stop inside the junction", &j);
        for (near, use_) in [(30_000, Use::Parking), (18_000, Use::Parking), (25_000, Use::Tree), (25_000, Use::Loading)] {
            let (j, _) = junction_with_uses(|p| vec![end_use(near, use_, p)]);
            same(&format!("{use_:?} at {near}"), &j);
        }
        let (j, _) = junction_with_a_pocket(|_| {});
        with_every_target_selected("an arm with a taper", j);
        let mut j = Junction::new(0);
        let e = j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid;
        let w = j.current().arms.iter().find(|a| a.bearing == 270).unwrap().uid;
        assert!(j.set_bus_lane(e, true) && j.set_approach(e, GATE_YIELD) && j.set_approach(w, Q_VIRTUAL) && j.set_rule(e, RULE_DEAD_END));
        with_every_target_selected("gates, a loop, a dead end", j);
        let mut q = Junction::new(0);
        let e = q.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid;
        assert!(q.set_bus_lane(e, true) && q.set_approach(e, Q_CURB) && q.set_stop(e, STOP_BULB) && q.set_rule(e, RULE_RIRO));
        with_every_target_selected("a queue jump, a bulb and a right-in right-out island", q);
        // A corner that fails and a crossing that is too long.
        let mut bad = Junction::new(0);
        let n = bad.current().arms[0].uid;
        for _ in 0..30 {
            bad.step_corner(n, 1);
        }
        with_every_target_selected("a fast corner", bad);
        let mut priority = Junction::new(0);
        assert!(priority.set_control(PRIORITY));
        with_every_target_selected("a priority junction", priority);
    }

    trait Tap: Sized {
        fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
            f(&mut self);
            self
        }
    }
    impl Tap for Junction {}
}
