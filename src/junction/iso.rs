//! The junction in 2.5D, as SVG markup, from the model's view: a read-only picture of what the plan draws flat.
//! A plan point is rotated by a fixed angle, its depth squashed, and its height lifted up the screen; there is one
//! viewpoint, so the shapes are painted far to near with no depth buffer.

use std::fmt::Write;

use serde_json::Value;

use crate::junction::frame::Frame;
use crate::junction::geometry::{sample_arc, sample_quad};
use crate::junction::read_model::JView;
use crate::junction::text::arms_text;
use crate::shared::catalogue::KINDS;
use crate::shared::i18n::{Args, I18n};

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

/// A number to a tenth, with no trailing zero.
fn f1(n: f64) -> String {
    let r = (n * 10.0).round() / 10.0;
    if r == 0.0 { "0".to_string() } else { format!("{r}") }
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
    let (ramp, side) = (0.9, 0.78);
    format!(
        "<defs><filter id=\"iso-shade-ramp\" color-interpolation-filters=\"sRGB\"><feComponentTransfer><feFuncR type=\"linear\" slope=\"{ramp}\"/><feFuncG type=\"linear\" slope=\"{ramp}\"/><feFuncB type=\"linear\" slope=\"{ramp}\"/></feComponentTransfer></filter><filter id=\"iso-shade-side\" color-interpolation-filters=\"sRGB\"><feComponentTransfer><feFuncR type=\"linear\" slope=\"{side}\"/><feFuncG type=\"linear\" slope=\"{side}\"/><feFuncB type=\"linear\" slope=\"{side}\"/></feComponentTransfer></filter></defs>"
    )
}

/// How far a vertex of a ramp may be from the edge it meets and still be on it.
const RAMP_TOLERANCE_MM: f64 = 50.0;

fn distance_to_segment(p: P, a: P, b: P) -> f64 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let len2 = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if len2 < 1e-9 { 0.0 } else { (((p.0 - a.0) * ab.0 + (p.1 - a.1) * ab.1) / len2).clamp(0.0, 1.0) };
    (p.0 - a.0 - t * ab.0).hypot(p.1 - a.1 - t * ab.1)
}

/// Each vertex of `foot` with `h_mm` where it lies on the boundary of `top` and 0 where it does not: the slope of a
/// ramp rises from the road to the edge it meets.
fn lift_to(foot: &[P], top: &[P], h_mm: f64) -> Vec<(P, f64)> {
    let near = |p: P| (0..top.len()).any(|i| distance_to_segment(p, top[i], top[(i + 1) % top.len()]) < RAMP_TOLERANCE_MM);
    foot.iter().map(|&p| (p, if near(p) { h_mm } else { 0.0 })).collect()
}

struct Draw {
    f: IsoFrame,
    flat: String,
    solids: Vec<Solid>,
    /// Posts with their heads, painted over the solids.
    posts: String,
    /// The outline of the selected arm, painted last.
    sel: String,
}

impl Draw {
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

    /// A surface `h_mm` high with no wall round it, a table's top or a continuous sidewalk, drawn over what lies
    /// on the ground.
    fn table(&mut self, kind_id: &str, foot: &[P], h_mm: f64) {
        if foot.len() < 3 {
            return;
        }
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

    fn render(&self, s: &Solid) -> String {
        let id = s.kind_id;
        let mut g = format!("<g class=\"iso-solid\" data-depth=\"{}\">", f1(Self::depth(s)));
        if s.sides {
            let seen: Vec<P> = s.foot.iter().map(|p| view_point(*p)).collect();
            let n = seen.len();
            let area: f64 = (0..n).map(|i| seen[i].0 * seen[(i + 1) % n].1 - seen[(i + 1) % n].0 * seen[i].1).sum();
            for i in 0..n {
                let (a, b) = (seen[i], seen[(i + 1) % n]);
                // The edge's outward normal points towards the viewer when its depth component is positive.
                let e = (b.0 - a.0, b.1 - a.1);
                let toward_viewer = if area > 0.0 { -e.0 } else { e.0 } > 0.0;
                if toward_viewer {
                    let (pa, pb) = (s.foot[i], s.foot[(i + 1) % n]);
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

    fn finish(mut self) -> String {
        self.solids.sort_by(|a, b| Self::depth(a).total_cmp(&Self::depth(b)));
        let solids: String = self.solids.iter().map(|s| self.render(s)).collect();
        format!("{}<g class=\"iso\">{}{solids}{}{}</g>", shades(), self.flat, self.posts, self.sel)
    }
}

/// The points the picture reaches on the ground: each arm from its mouth to its end, the core, and a
/// roundabout's ring. Everything drawn is inside them.
fn content_points(v: &JView) -> Vec<P> {
    let mut pts: Vec<P> = v.arms.iter().flat_map(|a| flatten(&a.outline)).chain(flatten(&v.core)).collect();
    if let Some(ring) = &v.ring {
        let r = ring.radius_mm as f64;
        pts.extend([(r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)]);
    }
    pts
}

/// What a screen reader is told the picture is, drawn again when the language changes.
pub fn iso_label(v: &JView, i18n: &I18n) -> String {
    i18n.tr("jn-iso-label", &Args::new().str("arms", arms_text(v, i18n)))
}

/// The junction in 2.5D, in a drawing `width` wide in a window `window_height` high.
pub fn iso_svg(v: &JView, width: f64, window_height: f64) -> IsoSvg {
    let f = IsoFrame::fit(v.bounds, &content_points(v), width, window_height);
    let mut d = Draw { f, flat: String::new(), solids: Vec::new(), posts: String::new(), sel: String::new() };
    let wall = height_of("sidewalk");

    // the carriageway where the streets meet, and a roundabout's ring and island
    if let Some(ring) = &v.ring {
        let circle = |r: f64| -> Vec<P> {
            (0..48)
                .map(|i| {
                    let t = i as f64 * std::f64::consts::TAU / 48.0;
                    (r * t.cos(), r * t.sin())
                })
                .collect()
        };
        d.lay("travel", &circle(ring.radius_mm as f64));
        d.stand("median", circle(ring.island_mm as f64), height_of("median"), true);
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
            match height_of(id) {
                h if h > 0.0 => d.stand(id, foot, h, true),
                _ => d.lay(id, &foot),
            }
        }
        // Where there is no parking beside the curb the road runs on, over the parking piece.
        for gap in &a.gaps {
            d.lay("travel", &flatten(gap));
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
        if v.selected.kind == Some("arm") && v.selected.uid == a.uid {
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

    fn svg(j: &Junction) -> IsoSvg {
        iso_svg(&j.view(), 1000.0, 1000.0)
    }

    fn count(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    fn depths(markup: &str) -> Vec<f64> {
        markup.split("data-depth=\"").skip(1).map(|rest| rest.split('"').next().unwrap().parse().unwrap()).collect()
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
        let s = iso_svg(&v, 1000.0, 1000.0);
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
            let s = iso_svg(&v, 1000.0, 1000.0);
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
