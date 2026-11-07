//! The junction's drawing being built: shapes of the shared diagram (`shared::diagram`), in plan millimetres and heights,
//! gathered by what kind of thing they are, then made a `Diagram` of layers. Nothing here writes markup: the renderer
//! does, through the oblique frame (`IsoFrame` as a `Projection`).
//!
//! The layers, bottom to top (the junction's own use of the kernel's kinds):
//! - `Ground`: the carriageway where the streets meet, a roundabout's ring and its arrows.
//! - `Surface`: what lies on the ground in the order it is laid (the streets' pieces, a table's top, the places to press, the
//!   paint and the marks on the road): the order is what a press and an overlap are decided by.
//! - `Raised`: what stands, painted far to near (slabs, islands, a bulb), each with the walls turned to the viewer.
//! - `Markings`: outlines and lines that belong on what stands (a stop's platform, a curb, a turn), over it.
//! - `Objects`: the posts of a signal, a sign and a bollard, with their heads.
//! - `Signs`: the words: the streets' names, the crossings' lengths, the measures' codes.
//! - `Outline` (the diagram's own): the outline of what is selected.

use super::{IsoFrame, P, default_material, target_growth_px, view_point};
use crate::shared::diagram::{Diagram, Layer, LayerKind, Pt, Section, Shape};
use crate::shared::iso::format::f1;
use crate::shared::iso::paint::{Paint, Pattern, Style, class, quads};

/// How wide a zebra stripe is, and the gap between two, as the plan drew them.
const ZEBRA_MM: f64 = 600.0;
/// How thick a stop line is, and the dashes and gaps of one where traffic gives way.
const STOP_LINE_MM: f64 = 300.0;
const YIELD_DASH_MM: f64 = 700.0;
const YIELD_GAP_MM: f64 = 500.0;
/// How long on the screen an arrow's length on the ground is drawn, near enough, for the weight of its line.
const ARROW_SCREEN: f64 = 0.8;
/// What a street's name takes of the drawing, a character at a time, to keep the whole name inside it.
pub(super) const NAME_CHAR_PX: f64 = 9.0;
/// How far a name stands from the end of its arm, and from the edge of the drawing, in pixels.
const NAME_GAP_PX: f64 = 12.0;
const NAME_EDGE_PX: f64 = 4.0;
/// How far a vertex may be from an edge and still be on it: where a ramp meets its top, where one surface meets another.
const EDGE_TOLERANCE_MM: f64 = 50.0;

/// A point of the plan `h_mm` above the ground, as the diagram's point: across, up, along are the plan's x, the height, the plan's y.
pub(super) fn pt(p: P, h_mm: f64) -> Pt {
    Pt::new(p.0, h_mm, p.1)
}

fn ring(foot: &[P], h_mm: f64) -> Vec<Vec<Pt>> {
    vec![foot.iter().map(|&p| pt(p, h_mm)).collect()]
}

/// A closed path of `foot` `h_mm` above the ground, with its class and the attributes after it.
fn closed(class: String, attrs: String, foot: &[P], h_mm: f64) -> Shape {
    Shape::Path { runs: ring(foot, h_mm), closed: true, class, attrs, title: None }
}

/// The texture laid over a surface: the pattern of the material, as a path of its own (the surface's CSS fill must not
/// cover it).
fn hatch(material: &str, foot: &[P], h_mm: f64) -> Shape {
    closed("hatch".into(), format!(" fill=\"url(#iso-m-{material})\""), foot, h_mm)
}

/// A shape that stands on the ground: its footprint, its height, and whether its sides are drawn.
pub(super) struct Solid {
    kind_id: &'static str,
    /// The material of its top.
    material: &'static str,
    foot: Vec<P>,
    h_mm: f64,
    sides: bool,
    /// The attributes that make it pressable (`hit_attrs`), or none.
    hit: String,
}

fn distance_to_segment(p: P, a: P, b: P) -> f64 {
    let ab = (b.0 - a.0, b.1 - a.1);
    let len2 = ab.0 * ab.0 + ab.1 * ab.1;
    let t = if len2 < 1e-9 { 0.0 } else { (((p.0 - a.0) * ab.0 + (p.1 - a.1) * ab.1) / len2).clamp(0.0, 1.0) };
    (p.0 - a.0 - t * ab.0).hypot(p.1 - a.1 - t * ab.1)
}

/// Whether `p` is within `EDGE_TOLERANCE_MM` of an edge of `poly`.
pub(super) fn near_boundary(p: P, poly: &[P]) -> bool {
    (0..poly.len()).any(|i| distance_to_segment(p, poly[i], poly[(i + 1) % poly.len()]) < EDGE_TOLERANCE_MM)
}

/// Each vertex of `foot` with `h_mm` where it lies on the boundary of `top` and 0 where it does not: the slope of a
/// ramp rises from the road to the edge it meets.
pub(super) fn lift_to(foot: &[P], top: &[P], h_mm: f64) -> Vec<(P, f64)> {
    foot.iter().map(|&p| (p, if near_boundary(p, top) { h_mm } else { 0.0 })).collect()
}

pub(super) struct Scene {
    pub(super) f: IsoFrame,
    flat: Vec<Shape>,
    solids: Vec<Solid>,
    /// The footprints and heights of the surfaces with no wall round them, which a solid level with one does not
    /// show a wall against.
    tables: Vec<(Vec<P>, f64)>,
    /// Outlines and lines that belong on what stands (a stop's platform, a curb), painted over the solids.
    marks: Vec<Shape>,
    /// Posts with their heads, painted over the solids.
    posts: Vec<Shape>,
    /// The outline of what is selected.
    sel: Vec<Shape>,
    /// The attributes the shapes drawn now carry so that a press on them selects something (`hit_attrs`); empty
    /// for a shape that is only looked at.
    pub(super) hit: String,
    /// The words, painted last so nothing standing hides one.
    names: Vec<Shape>,
    /// How much of `flat` is the carriageway where the streets meet and the ring's arrows.
    ground: usize,
}

impl Scene {
    pub(super) fn new(f: IsoFrame) -> Scene {
        Scene {
            f,
            flat: Vec::new(),
            solids: Vec::new(),
            tables: Vec::new(),
            marks: Vec::new(),
            posts: Vec::new(),
            sel: Vec::new(),
            hit: String::new(),
            names: Vec::new(),
            ground: 0,
        }
    }

    /// What has been laid so far is the ground: the carriageway where the streets meet.
    pub(super) fn ground_laid(&mut self) {
        self.ground = self.flat.len();
    }

    /// A shape lying on the ground, in the default material of its kind.
    pub(super) fn lay(&mut self, kind_id: &str, foot: &[P]) {
        self.lay_in(kind_id, default_material(kind_id), foot);
    }

    /// A shape lying on the ground, in `material`.
    pub(super) fn lay_in(&mut self, kind_id: &str, material: &str, foot: &[P]) {
        if foot.len() < 3 {
            return;
        }
        self.flat.push(closed(format!("iso-flat k-{kind_id} mat-{material}"), self.hit.clone(), foot, 0.0));
        self.flat.push(hatch(material, foot, 0.0));
    }

    /// The stripes of a zebra crossing: `foot` is its four corners, across the street first. They run the length
    /// of the crossing, as wide as the gaps between them.
    pub(super) fn zebra(&mut self, foot: &[P], h_mm: f64) {
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
            self.flat.push(closed("iso-zebra".into(), String::new(), &[a, b, (b.0 + along.0, b.1 + along.1), (a.0 + along.0, a.1 + along.1)], h_mm));
        }
    }

    /// A solid double white line along `path`, an open polyline of points, lying on the road. The quads come from the
    /// shared paint and are projected like the road they lie on. The line is not joined to its start: a turbo's
    /// dividers are pieces between two streets.
    pub(super) fn divider(&mut self, path: &[P]) {
        for (paint, quad) in quads(path, Style { paint: Paint::White, pattern: Pattern::Solid, double: true }) {
            self.flat.push(closed(class(paint).to_string(), String::new(), &quad, 0.0));
        }
    }

    /// A stop line from `a` to `b`: a row of dashes where traffic gives way.
    pub(super) fn stop_line(&mut self, a: P, b: P, h_mm: f64, dashed: bool) {
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
            self.flat.push(closed(
                "iso-stopline".into(),
                String::new(),
                &[(p.0 + n.0, p.1 + n.1), (q.0 + n.0, q.1 + n.1), (q.0 - n.0, q.1 - n.1), (p.0 - n.0, p.1 - n.1)],
                h_mm,
            ));
            t += dash + gap;
        }
    }

    /// A surface `h_mm` high with no wall round it, a table's top or a continuous sidewalk, drawn over what lies
    /// on the ground.
    pub(super) fn table(&mut self, kind_id: &str, foot: &[P], h_mm: f64) {
        if foot.len() < 3 {
            return;
        }
        self.tables.push((foot.to_vec(), h_mm));
        let material = default_material(kind_id);
        self.flat.push(closed(format!("iso-table iso-flat k-{kind_id} mat-{material}"), self.hit.clone(), foot, h_mm));
        self.flat.push(hatch(material, foot, h_mm));
    }

    /// A ramp from the ground up to `top`: its vertices on the top's edge at `h_mm`, the rest on the ground.
    pub(super) fn ramp(&mut self, kind_id: &str, foot: &[P], top: &[P], h_mm: f64) {
        if foot.len() < 3 {
            return;
        }
        let run: Vec<Pt> = lift_to(foot, top, h_mm).into_iter().map(|(p, h)| pt(p, h)).collect();
        self.flat.push(Shape::Path {
            runs: vec![run],
            closed: true,
            class: format!("iso-ramp k-{kind_id}"),
            attrs: " filter=\"url(#iso-shade-ramp)\"".into(),
            title: None,
        });
    }

    pub(super) fn stand(&mut self, kind_id: &'static str, foot: Vec<P>, h_mm: f64, sides: bool) {
        self.stand_in(kind_id, default_material(kind_id), foot, h_mm, sides);
    }

    pub(super) fn stand_in(&mut self, kind_id: &'static str, material: &'static str, foot: Vec<P>, h_mm: f64, sides: bool) {
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

    /// A solid as its group: the walls turned to the viewer, then its top and the texture on it.
    fn solid(&self, i: usize) -> Shape {
        let s = &self.solids[i];
        let id = s.kind_id;
        let mut shapes = Vec::new();
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
                    shapes.push(Shape::Path {
                        runs: vec![vec![pt(pa, 0.0), pt(pb, 0.0), pt(pb, s.h_mm), pt(pa, s.h_mm)]],
                        closed: true,
                        class: format!("iso-side k-{id}"),
                        attrs: " filter=\"url(#iso-shade-side)\"".into(),
                        title: None,
                    });
                }
            }
        }
        shapes.push(closed(format!("iso-top k-{id} mat-{}", s.material), String::new(), &s.foot, s.h_mm));
        shapes.push(hatch(s.material, &s.foot, s.h_mm));
        Shape::Elem { tag: "g", class: "iso-solid".into(), attrs: format!(" data-depth=\"{}\"{}", f1(Self::depth(s)), s.hit), shapes }
    }

    /// A post from the ground up to `h_mm`, with a head on it.
    pub(super) fn post(&mut self, at: P, h_mm: f64, head: &str) {
        let (x0, y0) = self.f.at(at, 0.0);
        let (x1, y1) = self.f.at(at, h_mm);
        let r = (250.0 * self.f.scale).clamp(2.5, 7.0);
        self.posts.push(Shape::Elem {
            tag: "line",
            class: "iso-post".into(),
            attrs: format!(" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"", f1(x0), f1(y0), f1(x1), f1(y1)),
            shapes: Vec::new(),
        });
        self.posts.push(Shape::Elem {
            tag: "circle",
            class: format!("iso-head {head}"),
            attrs: format!(" cx=\"{}\" cy=\"{}\" r=\"{}\"", f1(x1), f1(y1), f1(r)),
            shapes: Vec::new(),
        });
    }

    /// A stop's post with its shelter's box, at a point of the ground `h_mm` up, both of a size in pixels.
    pub(super) fn stop_post(&mut self, at: P, h_mm: f64, shelter: bool) {
        let (x, y) = self.f.at(at, h_mm);
        let mut shapes = Vec::new();
        if shelter {
            shapes.push(Shape::Elem {
                tag: "rect",
                class: "stop-shelter".into(),
                attrs: format!(" x=\"{}\" y=\"{}\" width=\"10\" height=\"10\"", f1(x - 5.0), f1(y - 5.0)),
                shapes: Vec::new(),
            });
        }
        shapes.push(Shape::Elem { tag: "circle", class: String::new(), attrs: format!(" cx=\"{}\" cy=\"{}\" r=\"3.5\"", f1(x), f1(y)), shapes: Vec::new() });
        self.posts.push(Shape::Elem { tag: "g", class: "stop-post".into(), attrs: String::new(), shapes });
    }

    /// The outline of a shape `h_mm` above the ground, to show what is selected: two tones, the light line over a dark
    /// casing, so that one of them stands out from any surface (the street page's selection outline).
    pub(super) fn outline(&mut self, foot: &[P], h_mm: f64) {
        if foot.len() >= 3 {
            self.sel.push(closed("iso-sel-casing".into(), String::new(), foot, h_mm));
            self.sel.push(closed("sel-box iso-sel".into(), String::new(), foot, h_mm));
        }
    }

    /// Makes the shapes `draw` draws pressable: a press on one selects what `attrs` (`hit_attrs`) says.
    pub(super) fn pressing(&mut self, attrs: String, draw: impl FnOnce(&mut Scene)) {
        self.hit = attrs;
        draw(self);
        self.hit.clear();
    }

    /// A shape that is only a place to press, with nothing painted in it. `class` is its CSS class, `title` what the
    /// pointer is told it is.
    pub(super) fn press_area(&mut self, class: &str, attrs: &str, foot: &[P], h_mm: f64, title: Option<&str>) {
        if foot.len() < 3 {
            return;
        }
        // A part narrower than a finger is given the rest of the 24 px as a stroke nobody sees, which a press also lands on.
        let on_screen: Vec<P> = foot.iter().map(|p| self.f.at(*p, h_mm)).collect();
        let grow = target_growth_px(&on_screen);
        let reach = if grow > 0.0 { format!(" stroke-width=\"{}\"", f1(grow)) } else { String::new() };
        self.flat.push(Shape::Path {
            runs: ring(foot, h_mm),
            closed: true,
            class: class.into(),
            attrs: format!("{attrs}{reach}"),
            title: title.map(String::from),
        });
    }

    /// A street's name beyond the end of its arm, away from the middle of the junction and kept inside the drawing.
    pub(super) fn name(&mut self, end: P, text: &str, selected: bool) {
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
        self.names.push(Shape::Caption {
            px: (left + shift, y),
            class: format!("iso-name t-mark t-halo{}", if selected { " t-blue" } else { "" }),
            anchor,
            text: text.into(),
        });
    }

    /// A mark of `class` along `runs`, with a casing under it in the panel's colour (a dark one for a warning), so that
    /// it is seen on any surface, light or dark. On the ground it lies under what stands; over, it is drawn over it.
    fn mark(&mut self, over: bool, class: &str, warn: bool, runs: Vec<Vec<Pt>>, closed: bool) {
        let drawn = if closed { runs.iter().any(|r| r.len() >= 3) } else { runs.iter().any(|r| r.len() >= 2) };
        if !drawn {
            return;
        }
        let casing = if warn { "iso-casing warn" } else { "iso-casing" };
        let layer = if over { &mut self.marks } else { &mut self.flat };
        layer.push(Shape::Path { runs: runs.clone(), closed, class: casing.into(), attrs: String::new(), title: None });
        layer.push(Shape::Path { runs, closed, class: class.into(), attrs: String::new(), title: None });
    }

    /// A closed outline of `foot` `h_mm` above the ground, as a mark.
    pub(super) fn mark_ring(&mut self, over: bool, class: &str, warn: bool, foot: &[P], h_mm: f64) {
        if foot.len() >= 3 {
            self.mark(over, class, warn, ring(foot, h_mm), true);
        }
    }

    /// Open lines `h_mm` above the ground, as a mark.
    pub(super) fn mark_lines(&mut self, over: bool, class: &str, warn: bool, strokes: &[Vec<P>], h_mm: f64) {
        let runs = strokes.iter().filter(|l| l.len() >= 2).map(|l| l.iter().map(|&p| pt(p, h_mm)).collect()).collect();
        self.mark(over, class, warn, runs, false);
    }

    /// A glyph painted on the road: `strokes` on the plan, `extra` the classes added to a lane's arrow (`bad`, `out`),
    /// `size_mm` how long it is on the ground, which sets how heavy its line is. A halo of the panel's colour is under it.
    pub(super) fn glyph(&mut self, extra: &str, strokes: &[Vec<P>], size_mm: f64, h_mm: f64) {
        let runs: Vec<Vec<Pt>> = strokes.iter().filter(|l| l.len() >= 2).map(|l| l.iter().map(|&p| pt(p, h_mm)).collect()).collect();
        if runs.is_empty() {
            return;
        }
        let px = f1(size_mm * self.f.scale * ARROW_SCREEN);
        let stroke = |class: &str| Shape::Path { runs: runs.clone(), closed: false, class: class.into(), attrs: String::new(), title: None };
        self.flat.push(Shape::Elem {
            tag: "g",
            class: format!("lane-arrow{extra}"),
            attrs: format!(" style=\"--arrow:{px}\""),
            shapes: vec![stroke("halo"), stroke("")],
        });
    }

    /// A line of text standing at a point of the ground, over a halo, on top of everything. `class` is its classes.
    pub(super) fn label(&mut self, class: &str, at: P, h_mm: f64, dy: f64, text: &str) {
        let (x, y) = self.f.at(at, h_mm);
        self.names.push(Shape::Caption { px: (x, y + dy), class: class.into(), anchor: "middle", text: text.into() });
    }

    /// A line of text standing at a point of the screen, in pixels, over a halo, on top of everything.
    pub(super) fn label_px(&mut self, class: &str, at: P, text: &str) {
        self.names.push(Shape::Caption { px: at, class: class.into(), anchor: "middle", text: text.into() });
    }

    /// The scene as the diagram it is: layers, bottom to top.
    pub(super) fn into_diagram(mut self, defs: String) -> Diagram {
        self.solids.sort_by(|a, b| Self::depth(a).total_cmp(&Self::depth(b)));
        let raised: Vec<Shape> = (0..self.solids.len()).map(|i| self.solid(i)).collect();
        let surface = self.flat.split_off(self.ground);
        let layer = |kind, shapes| Layer { kind, shapes };
        Diagram {
            defs,
            ground: layer(LayerKind::Ground, self.flat),
            sections: vec![Section {
                class: None,
                layers: vec![
                    layer(LayerKind::Surface, surface),
                    layer(LayerKind::Raised, raised),
                    layer(LayerKind::Markings, self.marks),
                    layer(LayerKind::Objects, self.posts),
                    layer(LayerKind::Signs, self.names),
                ],
            }],
            outline: layer(LayerKind::Outline, self.sel),
            hits: layer(LayerKind::Hit, Vec::new()),
        }
    }
}
