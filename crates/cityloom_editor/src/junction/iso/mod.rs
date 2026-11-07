//! The junction in 2.5D, from the model's view, as a diagram (`shared::diagram`): `build.rs` makes its shapes, `scene.rs`
//! gathers them into layers, and the shared renderer writes them through the oblique frame (`IsoFrame`, a `Projection`).
//! A plan point is rotated by a fixed angle, its depth squashed, and its height lifted up the screen; there is one
//! viewpoint, so what stands is painted far to near with no depth buffer.

mod build;
mod scene;

use std::fmt::Write;

use serde_json::Value;

use crate::junction::frame::Frame;
use crate::junction::geometry::{sample_arc, sample_quad};
use crate::junction::read_model::JView;
use crate::junction::text::arms_text;
use crate::shared::catalogue::{KINDS, MATERIALS};
use crate::shared::diagram::render::render;
use crate::shared::diagram::{Projection, Pt};
use crate::shared::i18n::{Args, I18n};
use crate::shared::iso::ground::{patterns, patterns_laid};
use crate::shared::iso::shading::shade_filters;
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
    /// the points of `content` (on the ground) and the tallest thing standing above them.
    pub fn fit(content: &[P], width: f64, window_height: f64) -> IsoFrame {
        let plan = Frame::fit(width, window_height);
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

/// The oblique frame as the diagram's projection: a point of the diagram is across, up and along, and here those are the
/// plan's x, the height and the plan's y. The map is affine, so one pattern fills a whole top, and the matrix that lays
/// it is read off three points of the ground.
impl Projection for IsoFrame {
    fn at(&self, p: Pt) -> (f64, f64) {
        IsoFrame::at(self, (p.x, p.z), p.y)
    }

    fn size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    fn ground_at(&self, _up: f64, _u: f64, _v: f64) -> [f64; 6] {
        let (o, x, y) = (self.at((0.0, 0.0), 0.0), self.at((1.0, 0.0), 0.0), self.at((0.0, 1.0), 0.0));
        [x.0 - o.0, x.1 - o.1, y.0 - o.0, y.1 - o.1, o.0, o.1]
    }
}

/// The diagram and frame of a drawing `width` wide in a window `window_height` high, for a flat figure
/// (`junction::figure`): no selection is made, and the words (titles) are English, which a flat figure does not draw.
pub(crate) fn build_diagram(v: &JView, width: f64, window_height: f64, units: Units) -> (IsoFrame, crate::shared::diagram::Diagram) {
    let i18n = crate::i18n_for(crate::shared::i18n::Locale::En);
    build::build(v, &i18n, width, window_height, units)
}

/// What the page needs to put the 2.5D picture in its `<svg>`.
pub struct IsoSvg {
    pub frame: IsoFrame,
    pub markup: String,
}

/// The filters that darken the faces turned from the eye: a ramp a little, a side more. SVG filters, since CSS
/// `filter: brightness()` on SVG shapes may not be applied in Safari.
pub(super) fn shades(f: &IsoFrame, laid: bool) -> String {
    format!("<defs>{}{}</defs>", shade_filters(&[("iso-shade-ramp", "0.9"), ("iso-shade-side", "0.78")]), materials(f, laid))
}

/// The surface materials' patterns laid on the ground as this frame projects it: the one matrix of the projection, which
/// is affine, lays them all.
/// `laid`: the drawing has a surface in a material no piece can be of (a curbside rain garden), so its pattern too.
fn materials(f: &IsoFrame, laid: bool) -> String {
    let ground = f.ground_at(0.0, 0.0, 0.0);
    if laid { patterns_laid("iso-m", ground) } else { patterns("iso-m", ground) }
}

/// The material a surface of this kind has when nothing says otherwise: the junction's own surfaces (the core, the
/// ring, a corner) are not a street's pieces.
fn default_material(kind_id: &str) -> &'static str {
    KINDS.iter().find(|k| k.id == kind_id).map_or("concrete", |k| MATERIALS[k.materials[0]].id)
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

/// "Basic turbo roundabout, regular size": what the drawing is, when the junction has a turbo.
pub(crate) fn turbo_name(v: &JView, i18n: &I18n) -> Option<String> {
    let t = v.turbo.as_ref()?;
    Some(i18n.tr(t.label_key, &Args::new().str("size", i18n.tr(t.size_key, &Args::new()).to_lowercase())))
}

/// What a screen reader is told the picture is: the junction in 2.5D, its streets, and a roundabout, a turbo layout
/// or a raised table where it is one. Drawn again when the language changes.
pub fn iso_label(v: &JView, i18n: &I18n) -> String {
    let arms = arms_text(v, i18n);
    if let Some(name) = turbo_name(v, i18n) {
        return i18n.tr("jn-iso-label-turbo", &Args::new().str("arms", arms).str("name", name));
    }
    let key = if v.control == "roundabout" {
        "jn-iso-label-roundabout"
    } else if v.raised {
        "jn-iso-label-raised"
    } else {
        "jn-iso-label"
    };
    i18n.tr(key, &Args::new().str("arms", arms))
}

/// The least a part to press is asked to be on the screen, in pixels (WCAG 2.2, 2.5.8 Target Size, Minimum).
pub const MIN_TARGET_PX: f64 = 24.0;

/// How much wider a polygon on the screen must be made all round, as the width of a stroke, to be as thick as
/// `MIN_TARGET_PX`: its thickness is taken as twice its area over its perimeter (a strip's width, near enough), and
/// 0 when it is thick enough already.
pub fn target_growth_px(points: &[P]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let (mut twice_area, mut perimeter) = (0.0, 0.0);
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        twice_area += a.0 * b.1 - b.0 * a.1;
        perimeter += (b.0 - a.0).hypot(b.1 - a.1);
    }
    if perimeter < 1e-9 {
        return 0.0;
    }
    let thickness = twice_area.abs() / perimeter;
    (MIN_TARGET_PX - thickness).max(0.0)
}

/// The attributes that make a shape of the picture pressable: what a press on it selects, as the plan's shapes say it
/// (`drawing::selected_by` reads them back). The cycle track and the bus lane are the only one of their kind, and have
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
pub(super) fn circle(r: f64) -> Vec<P> {
    (0..48)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / 48.0;
            (r * t.cos(), r * t.sin())
        })
        .collect()
}

/// The junction in 2.5D, in a drawing `width` wide in a window `window_height` high: every part the keys select can be
/// pressed (`hit_attrs`), and the one that is selected is outlined. The diagram's layers are in one group, `iso iso-junction`.
pub fn iso_svg(v: &JView, i18n: &I18n, width: f64, window_height: f64, units: Units) -> IsoSvg {
    let (frame, mut diagram) = build::build(v, i18n, width, window_height, units);
    // The defs come before the group, as an element of the picture's own.
    let defs = std::mem::take(&mut diagram.defs);
    IsoSvg { frame, markup: format!("{defs}<g class=\"iso iso-junction\">{}</g>", render(&diagram, &frame)) }
}

#[cfg(test)]
mod tests {
    use super::scene::{NAME_CHAR_PX, Scene, lift_to};
    use super::*;
    use crate::shared::diagram::render::path_d;
    use crate::shared::iso::format::f1;
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
        IsoFrame::fit(&[(-5000.0, -5000.0), (5000.0, 5000.0)], 1000.0, 1000.0)
    }

    #[test]
    fn the_oblique_frame_is_an_affine_projection_whose_one_matrix_lays_every_texture() {
        let f = frame();
        assert!(Projection::affine(&f));
        assert_eq!(Projection::size(&f), (f.width, f.height));
        let m = f.ground_at(0.0, 0.0, 0.0);
        for (x, y) in [(0.0, 0.0), (1234.0, -567.0), (-9000.0, 4000.0)] {
            let (px, py) = Projection::at(&f, Pt::new(x, 0.0, y));
            assert!((px - (m[0] * x + m[2] * y + m[4])).abs() < 1e-6 && (py - (m[1] * x + m[3] * y + m[5])).abs() < 1e-6, "({x}, {y})");
            // A height only lifts a point up the screen, by what `project` says, whatever the point.
            let (hx, hy) = Projection::at(&f, Pt::new(x, 150.0, y));
            assert!((hx - px).abs() < 1e-9 && ((py - hy) - 150.0 * LIFT * f.scale).abs() < 1e-6);
        }
        // The patterns of the picture are laid by that matrix.
        assert_eq!(materials(&f, false), patterns("iso-m", m));
    }

    #[test]
    fn the_picture_is_a_diagram_of_layers_bottom_to_top_with_what_stands_far_to_near() {
        use crate::shared::diagram::{LayerKind, Shape};
        let v = Junction::new(0).view();
        let (_, d) = build::build(&v, &crate::i18n_for(crate::shared::i18n::Locale::En), 1000.0, 800.0, Units::Metres);
        assert_eq!(d.ground.kind, LayerKind::Ground);
        assert_eq!(d.sections.len(), 1);
        let kinds: Vec<LayerKind> = d.sections[0].layers.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, [LayerKind::Surface, LayerKind::Raised, LayerKind::Markings, LayerKind::Objects, LayerKind::Signs]);
        assert_eq!(d.outline.kind, LayerKind::Outline);
        assert!(d.outline.shapes.is_empty(), "nothing is selected, nothing is outlined");
        assert!(d.hits.shapes.is_empty(), "the places to press are laid with what they are on");
        // The carriageway is the ground; the sidewalks stand, painted far to near; the words are captions.
        assert!(!d.ground.shapes.is_empty());
        let depth = |s: &Shape| match s {
            Shape::Elem { attrs, .. } => attrs.split("data-depth=\"").nth(1).and_then(|r| r.split('"').next()).and_then(|n| n.parse::<f64>().ok()),
            _ => None,
        };
        let depths: Vec<f64> = d.sections[0].layers[1].shapes.iter().filter_map(depth).collect();
        assert!(depths.len() >= 8 && depths.windows(2).all(|w| w[0] <= w[1]), "{depths:?}");
        assert!(d.sections[0].layers[4].shapes.iter().all(|s| matches!(s, Shape::Caption { .. })) && !d.sections[0].layers[4].shapes.is_empty());
    }

    fn square(x0: f64, x1: f64) -> Vec<(f64, f64)> {
        vec![(x0, 0.0), (x1, 0.0), (x1, 1000.0), (x0, 1000.0)]
    }

    /// The scene as the markup of its diagram.
    fn finished(d: Scene) -> String {
        let f = d.f;
        crate::shared::diagram::render::render(&d.into_diagram(String::new()), &f)
    }

    /// The path data of a polygon `h` above the ground in a frame, as the picture writes it.
    fn path_of(frame: IsoFrame, pts: &[P], h: f64) -> String {
        path_d(&frame, &[pts.iter().map(|&p| Pt::new(p.0, h, p.1)).collect()], true)
    }

    fn walls(d: Scene) -> usize {
        count(&finished(d), "class=\"iso-side ")
    }

    #[test]
    fn no_wall_is_drawn_where_two_surfaces_at_the_same_height_meet_and_one_is_where_they_differ() {
        let mut one = Scene::new(frame());
        one.stand("sidewalk", square(0.0, 1000.0), 150.0, true);
        let alone = walls(one);
        assert!(alone > 0);
        let pair = |h: f64| {
            let mut d = Scene::new(frame());
            d.stand("sidewalk", square(0.0, 1000.0), 150.0, true);
            d.stand("median", square(1000.0, 2000.0), h, true);
            walls(d)
        };
        assert_eq!(pair(150.0), 2 * alone - 1, "the wall where they touch is not drawn");
        assert_eq!(pair(300.0), 2 * alone, "a step is a wall");
        let mut table = Scene::new(frame());
        table.stand("sidewalk", square(0.0, 1000.0), 150.0, true);
        table.table("sidewalk", &square(1000.0, 2000.0), 150.0);
        assert_eq!(walls(table), alone - 1, "nor is one where a surface level with it begins");
    }

    #[test]
    fn a_divider_is_drawn_as_the_open_line_it_is_and_never_joined_to_its_start() {
        let mut d = Scene::new(frame());
        d.divider(&[(-3_000.0, 0.0), (0.0, 3_000.0), (3_000.0, 0.0), (0.0, -3_000.0)]);
        assert_eq!(count(&finished(d), "<path class=\"iso-paint white"), 2 * 3, "a double line of three segments: two lines of three quads, not four");
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
        // Repinned for #140 stages 1, 2, 4, 6 and 7 (the diagram's layer groups), which gave the shapes the attributes a press reads and drew the parts that only
        // have a place to press (a lane, a crossing, a bare corner); no shape moved.
        let (got, pinned) = (iso_fingerprint(), 402_851_788_980_089_418);
        assert_eq!(
            got, pinned,
            "the 2.5D markup of the sample junctions no longer draws byte for byte as it did before the end-zone beyond was added (this pins that an arm without a beyond is untouched). If you changed how these junctions draw on purpose, look at the 2.5D of each sample junction first; only when the picture is what you meant, replace the pinned number with {got}."
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
        let path = path_of(with.frame, &pts, 0.0);
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
        let css = include_str!("../../../../../web/editor.css");
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
                crate::junction::drawing::selected_by(role, number("data-uid").unwrap_or(0) as u32, number("data-lane").unwrap_or(0))
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
    fn a_strip_narrower_than_a_finger_grows_to_one_and_a_wide_shape_does_not() {
        let strip = |w: f64| [(0.0, 0.0), (400.0, 0.0), (400.0, w), (0.0, w)];
        // A strip 10 px wide is made 14 px wider (its thickness is a little under its width, 2A/P).
        let grow = target_growth_px(&strip(10.0));
        assert!((grow - (MIN_TARGET_PX - 2.0 * 400.0 * 10.0 / (2.0 * 410.0))).abs() < 1e-9 && grow > 13.0 && grow < 15.0, "{grow}");
        assert_eq!(target_growth_px(&strip(60.0)), 0.0);
        assert_eq!(target_growth_px(&[(0.0, 0.0), (1.0, 1.0)]), 0.0, "a line is no shape");
        assert_eq!(target_growth_px(&[(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)]), 0.0, "nor is a point");
    }

    #[test]
    fn on_a_narrow_drawing_the_places_to_press_that_are_thinner_than_a_finger_say_how_much_to_grow() {
        let growing = |width: f64| {
            let j = Junction::new(0);
            let m = iso_svg(&j.view(), &crate::i18n_for(crate::shared::i18n::Locale::En), width, 800.0, Units::Metres).markup;
            let tags: Vec<&str> = m.split("<path class=\"lane-hit\"").skip(1).map(|t| &t[..t.find('>').unwrap()]).collect();
            (tags.len(), tags.iter().filter(|t| t.contains("stroke-width=")).count())
        };
        let (lanes, wide) = growing(1440.0);
        let (narrow_lanes, narrow) = growing(320.0);
        assert_eq!(lanes, narrow_lanes);
        assert!(narrow >= wide && narrow > 0, "at 320 px the lanes are thinner than a finger: {narrow} of {narrow_lanes} grow, {wide} of {lanes} at 1440");
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
        let d = path_of(s.frame, &wedge, height_of("sidewalk"));
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
        for width in [240.0, 320.0, 400.0, 1000.0] {
            let b = [-20_000.0, -20_000.0, 20_000.0, 20_000.0];
            let corners = [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])];
            let f = IsoFrame::fit(&corners, width, 1000.0);
            for corner in corners {
                for h in [0.0, TALLEST_MM] {
                    let (x, y) = f.at(corner, h);
                    assert!(x >= 0.0 && x <= f.width && y >= 0.0 && y <= f.height, "{width}: {corner:?} {h} -> {x},{y}");
                }
            }
        }
    }

    #[test]
    fn the_drawing_is_as_wide_and_as_tall_as_the_page_leaves_room_for() {
        let f = IsoFrame::fit(&[(0.0, 0.0), (1000.0, 1000.0)], 1000.0, 1000.0);
        let size = crate::junction::frame::Frame::fit(1000.0, 1000.0);
        assert_eq!((f.width, f.height), (size.width, size.height));
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
