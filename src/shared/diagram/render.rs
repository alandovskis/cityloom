//! Writes a diagram as SVG markup through a projection.

use super::{Diagram, Face, JointRun, Layer, Projection, Pt, Section, Shape, Texture};
use std::fmt::Write;

use crate::shared::iso::format::{esc, f1};

/// A pixel value to two decimals, which is all a drawing needs.
fn r(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn points(p: &dyn Projection, pts: &[Pt]) -> String {
    pts.iter()
        .map(|&pt| {
            let (sx, sy) = p.at(pt);
            format!("{},{}", r(sx), r(sy))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A textured top over `near` to `far` along the street, `h` high, its edges across given along the street (`edge`: across
/// `(left, right)` at an `along`; straight between the bands, so each band is a quadrilateral of its own). It is the
/// polygon of class `class`, then the material's texture over it. A projection that is not an affine map cannot lay
/// one pattern on the top: it is cut into bands along the street (`Projection::bands`), each carrying the affine map
/// that is the projection's slope at its middle (`ground_at`); the band's polygon is written in the ground's own
/// millimetres, as the true quadrilateral the band is on the screen taken back through that map, so it fits the top
/// exactly and its `transform` lays the texture. An affine projection fills the one polygon with the pattern.
pub fn textured(out: &mut String, p: &dyn Projection, class: &str, material: &str, h: f64, (near, far): (f64, f64), edge: &dyn Fn(f64) -> (f64, f64)) {
    let ((a0, a1), (b0, b1)) = (edge(near), edge(far));
    let pts = points(p, &[Pt::new(a0, h, near), Pt::new(a1, h, near), Pt::new(b1, h, far), Pt::new(b0, h, far)]);
    if p.affine() {
        write!(out, "<polygon class=\"{class}\" points=\"{pts}\" fill=\"url(#iso-m-{material})\"/>").unwrap();
        return;
    }
    write!(out, "<polygon class=\"{class}\" points=\"{pts}\"/>").unwrap();
    // The bands share their fill, and so the group carries it: a texture's polygons are bare.
    write!(out, "<g class=\"hatch\" fill=\"url(#iso-m-{material})\">").unwrap();
    for w in p.bands(near, far).windows(2) {
        let (z0, z1) = (w[0], w[1]);
        let mid = p.band_mid(z0, z1);
        let (m0, m1) = edge(mid);
        let g = p.ground_at(h, (m0 + m1) / 2.0, mid);
        // The numbers as written, so that the polygon is taken back through exactly the map that lays the texture.
        let [a, _, c, d, e, f] = [g[0], g[1], g[2], g[3], g[4], g[5]].map(|n| (n * 1e6).round() / 1e6);
        let (e, f) = ((e * 100.0).round() / 100.0, (f * 100.0).round() / 100.0);
        let (e0, e1) = (edge(z0), edge(z1));
        let local: Vec<String> = [(e0.0, z0), (e0.1, z0), (e1.1, z1), (e1.0, z1)]
            .iter()
            .map(|&(x, z)| {
                let (sx, sy) = p.at(Pt::new(x, h, z));
                let v = (sy - f) / d;
                let u = (sx - e - c * v) / a;
                format!("{:.0},{:.0}", u, v)
            })
            .collect();
        write!(out, "<polygon transform=\"matrix({a} 0 {c} {d} {e} {f})\" points=\"{}\"/>", local.join(" ")).unwrap();
    }
    out.push_str("</g>");
}

/// The edges of a textured top across the street along it: the left edge runs from the near-left corner `l.0` to the
/// far-left `l.1`, the right from the near-right `r.0` to the far-right `r.1`, each straight between them.
fn lerp_edge(l: (Pt, Pt), r: (Pt, Pt)) -> impl Fn(f64) -> (f64, f64) {
    let at = move |(a, b): (Pt, Pt), z: f64| if b.z == a.z { a.x } else { a.x + (b.x - a.x) * (z - a.z) / (b.z - a.z) };
    move |z| (at(l, z), at(r, z))
}

/// A point as `x,y` to a tenth of a pixel.
fn pt1(p: &dyn Projection, pt: Pt) -> String {
    let (x, y) = p.at(pt);
    format!("{},{}", f1(x), f1(y))
}

/// The path data of a top's joints, to a tenth of a pixel: `Mx,yHx2` for a run level across the drawing and
/// `Mx,yLx2,y2` for any other.
pub fn joints_d(p: &dyn Projection, runs: &[JointRun]) -> String {
    let mut d = String::new();
    for run in runs {
        if run.flat {
            write!(d, "M{}H{}", pt1(p, run.from), f1(p.at(run.to).0)).unwrap();
        } else {
            write!(d, "M{}L{}", pt1(p, run.from), pt1(p, run.to)).unwrap();
        }
    }
    d
}

/// A face's name and the filter that darkens it, as an attribute: none for a top.
fn shade(face: Face) -> (&'static str, &'static str) {
    match face {
        Face::Front => ("front", " filter=\"url(#iso-shade-front)\""),
        Face::Side => ("side", " filter=\"url(#iso-shade-side)\""),
        Face::Ramp => ("ramp", " filter=\"url(#iso-shade-ramp)\""),
        Face::Top => ("top", ""),
    }
}

fn plane(out: &mut String, p: &dyn Projection, pts: &[Pt], class: &str, face: Face, labelled: bool, hatch: Option<&str>) {
    let (name, filter) = shade(face);
    let pts = points(p, pts);
    write!(out, "<polygon class=\"{class}\"").unwrap();
    if labelled {
        write!(out, " data-face=\"{name}\"").unwrap();
    }
    write!(out, " points=\"{pts}\"{filter}/>").unwrap();
    if let Some(h) = hatch {
        write!(out, "<polygon class=\"hatch\" points=\"{pts}\" fill=\"url(#{h})\"/>").unwrap();
    }
}

fn polygon(out: &mut String, p: &dyn Projection, pts: &[Pt], class: &str, face: Option<Face>, texture: Option<&Texture>) {
    if let Some(t) = texture {
        // Four corners, the near edge first (left, right), then the far edge (right, left).
        debug_assert_eq!(pts.len(), 4, "a textured polygon is the four corners of a top");
        if let [a, b, c, d] = pts[..] {
            textured(out, p, class, &t.material, a.y, (a.z, c.z), &lerp_edge((a, d), (b, c)));
            return;
        }
    }
    match face.map(shade) {
        Some((name, filter)) => write!(out, "<polygon class=\"{class}\" data-face=\"{name}\" points=\"{}\"{filter}/>", points(p, pts)).unwrap(),
        None => write!(out, "<polygon class=\"{class}\" points=\"{}\"/>", points(p, pts)).unwrap(),
    }
}

fn shape(out: &mut String, s: &Shape, p: &dyn Projection) {
    match s {
        Shape::Polygon { pts, class, face, texture } => polygon(out, p, pts, class, *face, texture.as_ref()),
        Shape::Plane { pts, class, face, labelled, hatch } => plane(out, p, pts, class, *face, *labelled, hatch.as_deref()),
        Shape::Joints { runs, class } => {
            let d = joints_d(p, runs);
            if !d.is_empty() {
                write!(out, "<path class=\"{class}\" d=\"{d}\"/>").unwrap();
            }
        }
        Shape::Polyline { pts, class } => out.push_str(&format!(r#"<polyline class="{class}" points="{}"/>"#, points(p, pts))),
        Shape::Group { class, uid, data, title, shapes } => {
            out.push_str(&format!(r#"<g class="{class}""#));
            if let Some(uid) = uid {
                out.push_str(&format!(r#" data-uid="{uid}""#));
            }
            if let Some((name, n)) = data {
                out.push_str(&format!(r#" {name}="{n}""#));
            }
            out.push('>');
            if let Some(title) = title {
                out.push_str(&format!("<title>{}</title>", esc(title)));
            }
            for s in shapes {
                shape(out, s, p);
            }
            out.push_str("</g>");
        }
    }
}

fn layer(out: &mut String, l: &Layer, p: &dyn Projection) {
    out.push_str(&format!(r#"<g class="{}">"#, l.kind.class()));
    for s in &l.shapes {
        shape(out, s, p);
    }
    out.push_str("</g>");
}

/// One layer as markup, on its own.
pub fn layer_markup(l: &Layer, p: &dyn Projection) -> String {
    let mut out = String::new();
    layer(&mut out, l, p);
    out
}

fn section(out: &mut String, s: &Section, p: &dyn Projection) {
    if let Some(class) = &s.class {
        out.push_str(&format!(r#"<g class="{class}">"#));
    }
    for l in &s.layers {
        layer(out, l, p);
    }
    if s.class.is_some() {
        out.push_str("</g>");
    }
}

/// The diagram as markup: its defs, the ground, each section, then the outline.
pub fn render(d: &Diagram, p: &dyn Projection) -> String {
    let mut out = d.defs.clone();
    layer(&mut out, &d.ground, p);
    for s in &d.sections {
        section(&mut out, s, p);
    }
    layer(&mut out, &d.outline, p);
    out
}

#[cfg(test)]
mod tests {
    use super::super::LayerKind;
    use super::*;

    struct Scale(f64);

    impl Projection for Scale {
        fn at(&self, p: Pt) -> (f64, f64) {
            (p.x * self.0, p.y * self.0 + p.z * self.0)
        }
        fn size(&self) -> (f64, f64) {
            (100.0, 100.0)
        }
    }

    fn poly(class: &str) -> Shape {
        Shape::Polygon { pts: vec![Pt::new(1.0, 2.0, 3.0), Pt::new(4.0, 5.0, 6.0)], class: class.into(), face: None, texture: None }
    }

    fn layer_of(kind: LayerKind, shapes: Vec<Shape>) -> Layer {
        Layer { kind, shapes }
    }

    #[test]
    fn render_writes_the_layers_in_order_each_in_a_group_of_its_class() {
        let d = Diagram {
            defs: "<defs/>".into(),
            ground: layer_of(LayerKind::Ground, vec![]),
            sections: vec![Section { class: None, layers: vec![layer_of(LayerKind::Surface, vec![]), layer_of(LayerKind::Raised, vec![])] }],
            outline: layer_of(LayerKind::Outline, vec![]),
        };
        assert_eq!(
            render(&d, &Scale(1.0)),
            r#"<defs/><g class="iso-layer-ground"></g><g class="iso-surface"></g><g class="iso-raised"></g><g class="iso-layer-outline"></g>"#
        );
    }

    #[test]
    fn a_section_with_a_class_is_a_group_around_its_layers_and_one_without_is_not() {
        let mk = |class: Option<&str>| Diagram {
            defs: String::new(),
            ground: layer_of(LayerKind::Ground, vec![]),
            sections: vec![Section { class: class.map(String::from), layers: vec![layer_of(LayerKind::Surface, vec![])] }],
            outline: layer_of(LayerKind::Outline, vec![]),
        };
        let with = render(&mk(Some("iso-section iso-across")), &Scale(1.0));
        assert!(with.contains(r#"<g class="iso-section iso-across"><g class="iso-surface"></g></g>"#), "{with}");
        let without = render(&mk(None), &Scale(1.0));
        assert!(!without.contains("iso-section"), "{without}");
    }

    #[test]
    fn points_are_rounded_to_two_decimals_like_the_street_does() {
        let d = Diagram { defs: String::new(), ground: layer_of(LayerKind::Ground, vec![]), sections: vec![], outline: layer_of(LayerKind::Outline, vec![]) };
        // A hand-made projection that lands between two hundredths.
        let s = render(
            &Diagram { ground: layer_of(LayerKind::Ground, vec![Shape::Polyline { pts: vec![Pt::new(1.0, 0.0, 0.0)], class: "c".into() }]), ..d },
            &Scale(1.006),
        );
        assert!(s.contains(r#"points="1.01,0""#), "{s}");
    }

    #[test]
    fn a_group_carries_its_uid_and_an_escaped_title() {
        let g = Shape::Group { class: "iso-piece".into(), uid: Some(7), data: None, title: Some("A & <B>".into()), shapes: vec![poly("iso-top")] };
        let bare = Shape::Group { class: "g".into(), uid: None, data: None, title: None, shapes: vec![] };
        let d = Diagram {
            defs: String::new(),
            ground: layer_of(LayerKind::Ground, vec![g, bare]),
            sections: vec![],
            outline: layer_of(LayerKind::Outline, vec![]),
        };
        let s = render(&d, &Scale(1.0));
        assert!(s.contains(r#"<g class="iso-piece" data-uid="7"><title>A &amp; &lt;B&gt;</title><polygon class="iso-top" points="1,5 4,11"/></g>"#), "{s}");
        assert!(s.contains(r#"<g class="g"></g>"#), "{s}");
    }

    /// A perspective stand-in: not affine, bands `[0, 10, 20]`, the identity matrix everywhere.
    struct Persp;

    impl Projection for Persp {
        fn at(&self, p: Pt) -> (f64, f64) {
            (p.x, p.y + p.z)
        }
        fn size(&self) -> (f64, f64) {
            (100.0, 100.0)
        }
        fn affine(&self) -> bool {
            false
        }
        fn ground_at(&self, _up: f64, _u: f64, _v: f64) -> [f64; 6] {
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        }
        fn bands(&self, _near: f64, _far: f64) -> Vec<f64> {
            vec![0.0, 10.0, 20.0]
        }
        fn band_mid(&self, z0: f64, z1: f64) -> f64 {
            (z0 + z1) / 2.0
        }
    }

    /// The same with an affine map: one polygon is enough.
    struct Flat;

    impl Projection for Flat {
        fn at(&self, p: Pt) -> (f64, f64) {
            (p.x, p.y + p.z)
        }
        fn size(&self) -> (f64, f64) {
            (100.0, 100.0)
        }
        fn affine(&self) -> bool {
            true
        }
        fn ground_at(&self, _up: f64, _u: f64, _v: f64) -> [f64; 6] {
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        }
        fn bands(&self, near: f64, far: f64) -> Vec<f64> {
            vec![near, far]
        }
        fn band_mid(&self, z0: f64, z1: f64) -> f64 {
            (z0 + z1) / 2.0
        }
    }

    fn top() -> Shape {
        Shape::Polygon {
            pts: vec![Pt::new(0.0, 0.0, 0.0), Pt::new(10.0, 0.0, 0.0), Pt::new(10.0, 0.0, 20.0), Pt::new(0.0, 0.0, 20.0)],
            class: "iso-top".into(),
            face: None,
            texture: Some(Texture { material: "concrete".into(), tile_mm: (1800.0, 1800.0) }),
        }
    }

    fn drawn(s: Shape, p: &dyn Projection) -> String {
        let d = Diagram { defs: String::new(), ground: layer_of(LayerKind::Ground, vec![s]), sections: vec![], outline: layer_of(LayerKind::Outline, vec![]) };
        render(&d, p)
    }

    #[test]
    fn a_textured_top_on_a_perspective_is_cut_into_bands_each_with_its_transform() {
        let s = drawn(top(), &Persp);
        assert!(
            s.contains(concat!(
                r#"<polygon class="iso-top" points="0,0 10,0 10,20 0,20"/>"#,
                r#"<g class="hatch" fill="url(#iso-m-concrete)">"#,
                r#"<polygon transform="matrix(1 0 0 1 0 0)" points="0,0 10,0 10,10 0,10"/>"#,
                r#"<polygon transform="matrix(1 0 0 1 0 0)" points="0,10 10,10 10,20 0,20"/></g>"#
            )),
            "{s}"
        );
        assert_eq!(s.matches("transform=").count(), 2);
    }

    #[test]
    fn a_textured_top_on_an_affine_projection_is_one_polygon() {
        let s = drawn(top(), &Flat);
        assert_eq!(s.matches("<polygon").count(), 1, "{s}");
        assert_eq!(s.matches("transform=").count(), 0, "{s}");
        assert!(s.contains(r##"fill="url(#iso-m-concrete)""##), "{s}");
    }

    #[test]
    fn a_face_carries_its_shading_filter() {
        let face = |f| Shape::Polygon { pts: vec![Pt::new(1.0, 2.0, 3.0)], class: "iso-curb".into(), face: Some(f), texture: None };
        assert!(drawn(face(Face::Front), &Flat).contains(r##"<polygon class="iso-curb" data-face="front" points="1,5" filter="url(#iso-shade-front)"/>"##));
        assert!(drawn(face(Face::Side), &Flat).contains(r##"data-face="side" points="1,5" filter="url(#iso-shade-side)"/>"##));
        assert!(drawn(face(Face::Ramp), &Flat).contains(r##"data-face="ramp" points="1,5" filter="url(#iso-shade-ramp)"/>"##));
        let top = drawn(face(Face::Top), &Flat);
        assert!(top.contains(r#"data-face="top" points="1,5"/>"#) && !top.contains("filter"), "{top}");
    }

    #[test]
    fn all_the_joints_of_a_top_are_one_path_after_the_bands() {
        let run = |a: (f64, f64, f64), b: (f64, f64, f64), flat| JointRun { from: Pt::new(a.0, a.1, a.2), to: Pt::new(b.0, b.1, b.2), flat };
        let joints = Shape::Joints {
            runs: vec![run((0.0, 0.0, 5.0), (10.0, 0.0, 5.0), true), run((4.0, 0.0, 0.0), (4.0, 0.0, 20.0), false)],
            class: "joints j-concrete".into(),
        };
        let s = drawn(Shape::Group { class: "g".into(), uid: None, data: None, title: None, shapes: vec![top(), joints] }, &Persp);
        assert!(s.contains(r#"</g><path class="joints j-concrete" d="M0,5H10M4,0L4,20"/></g>"#), "{s}");
        assert_eq!(s.matches("<path").count(), 1, "{s}");
    }

    /// A projection that shears and scales, so the band polygons are inverted through a real matrix.
    struct Sheared;

    impl Projection for Sheared {
        fn at(&self, p: Pt) -> (f64, f64) {
            // x_px = 2u + 0.5v + 3, y_px = 4v + 7 (the plane's up is ignored).
            (2.0 * p.x + 0.5 * p.z + 3.0, 4.0 * p.z + 7.0)
        }
        fn size(&self) -> (f64, f64) {
            (100.0, 100.0)
        }
        fn affine(&self) -> bool {
            false
        }
        fn ground_at(&self, _up: f64, _u: f64, _v: f64) -> [f64; 6] {
            [2.0, 0.0, 0.5, 4.0, 3.0, 7.0]
        }
        fn bands(&self, near: f64, far: f64) -> Vec<f64> {
            vec![near, far]
        }
        fn band_mid(&self, z0: f64, z1: f64) -> f64 {
            (z0 + z1) / 2.0
        }
    }

    #[test]
    fn a_band_polygon_is_the_top_taken_back_through_the_matrix_that_lays_its_texture() {
        let s = drawn(top(), &Sheared);
        // The matrix is the projection's own, so each corner comes back to its ground millimetres.
        assert!(s.contains(r#"<polygon transform="matrix(2 0 0.5 4 3 7)" points="0,0 10,0 10,20 0,20"/>"#), "{s}");
    }

    #[test]
    fn a_plane_is_shaded_by_its_face_labelled_on_request_and_hatched_over() {
        let plane = |face, labelled, hatch: Option<&str>| Shape::Plane {
            pts: vec![Pt::new(1.0, 2.0, 3.0)],
            class: "iso-front k-x".into(),
            face,
            labelled,
            hatch: hatch.map(String::from),
        };
        let at = |s| layer_markup(&layer_of(LayerKind::Raised, vec![s]), &Scale(1.0));
        assert_eq!(
            at(plane(Face::Front, false, Some("h-x"))),
            r#"<g class="iso-raised"><polygon class="iso-front k-x" points="1,5" filter="url(#iso-shade-front)"/><polygon class="hatch" points="1,5" fill="url(#h-x)"/></g>"#
        );
        assert_eq!(at(plane(Face::Top, true, None)), r#"<g class="iso-raised"><polygon class="iso-front k-x" data-face="top" points="1,5"/></g>"#);
        assert!(at(plane(Face::Side, true, None)).contains(r#"data-face="side" points="1,5" filter="url(#iso-shade-side)""#));
    }
}
