//! Writes a diagram as SVG markup through a projection.

use super::{Diagram, Layer, Projection, Pt, Section, Shape};
use crate::shared::iso::format::esc;

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

fn shape(out: &mut String, s: &Shape, p: &dyn Projection) {
    match s {
        Shape::Polygon { pts, class, .. } => out.push_str(&format!(r#"<polygon class="{class}" points="{}"/>"#, points(p, pts))),
        Shape::Polyline { pts, class } => out.push_str(&format!(r#"<polyline class="{class}" points="{}"/>"#, points(p, pts))),
        Shape::Group { class, uid, title, shapes } => {
            out.push_str(&format!(r#"<g class="{class}""#));
            if let Some(uid) = uid {
                out.push_str(&format!(r#" data-uid="{uid}""#));
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
        let g = Shape::Group { class: "iso-piece".into(), uid: Some(7), title: Some("A & <B>".into()), shapes: vec![poly("iso-top")] };
        let bare = Shape::Group { class: "g".into(), uid: None, title: None, shapes: vec![] };
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
}
