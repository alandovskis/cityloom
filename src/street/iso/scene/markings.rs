//! The markings layer as shapes: paint quads and symbols on the surface. Equal to `super::super::paint::layer_over`.

use crate::shared::diagram::{Layer, LayerKind, Pt, Shape};
use crate::shared::iso::paint::{Paint, class, quads};
use crate::street::model::SegView;

use crate::street::iso::markings::{Mark, markings as inferred};
use crate::street::iso::paint::symbol_fits;
use crate::street::iso::projection::{ARROW_AT_MM, Projection, STRETCH_MM};
use crate::street::iso::symbols::shapes;
use crate::street::iso::taper::Draw;

fn polygon(class: &str, corners: impl Iterator<Item = (f64, f64, f64)>) -> Shape {
    Shape::Polygon { pts: corners.map(|(x, y, z)| Pt::new(x, y, z)).collect(), class: class.to_string(), face: None, texture: None }
}

/// A symbol lies on its lane where the lane is at `ARROW_AT_MM`: where the transition (`draw`) has moved the piece
/// there. A lane too narrow on the screen has none.
fn symbol(p: &Projection, segs: &[SegView], mark: &Mark, draw: Option<&Draw>) -> Option<Shape> {
    let s = segs.iter().find(|s| s.uid == mark.uid)?;
    let (x0, x1) = match draw.and_then(|d| d.shape_of(s).map(|shape| d.edges(shape, p.reach, p.reach.near + ARROW_AT_MM))) {
        Some(edges) => edges,
        None => (s.x_mm as f64, (s.x_mm + s.width_mm) as f64),
    };
    let w = x1 - x0;
    if !symbol_fits(p, w) {
        return None;
    }
    let cx = x0 + w / 2.0;
    let white = class(Paint::White);
    let shapes = shapes(mark.symbol, w)
        .iter()
        .map(|shape| polygon(white, shape.iter().map(|&(across, along)| (cx + across, 0.0, p.reach.near + ARROW_AT_MM + along))))
        .collect();
    Some(Shape::Group { class: "iso-mark".into(), uid: None, data: Some(("data-mark", mark.uid)), hidden: false, title: None, shapes })
}

/// The lines and symbols of the street, drawn flat on the ground over the reach of `p`. A line is the shared paint
/// quads of the path `[(x, 0), (x, STRETCH_MM)]` (across, along) cut to the reach, following the transition (`draw`)
/// where it moves an edge; a symbol is its polygons laid on its lane at `ARROW_AT_MM` from the reach's near end.
pub(in crate::street::iso) fn markings(p: &Projection, segs: &[SegView], draw: Option<&Draw>) -> Layer {
    let m = inferred(segs);
    let mut shapes = Vec::new();
    for line in &m.lines {
        let x = line.x_mm as f64;
        for (paint, quad) in quads(&[(x, 0.0), (x, STRETCH_MM)], line.style) {
            let (lo, hi) = quad.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| (lo.min(c.1), hi.max(c.1)));
            if hi.min(p.reach.far) - lo.max(p.reach.near) < 1e-9 {
                continue;
            }
            let corners = quad.iter().map(|&(across, along)| {
                let along = along.clamp(p.reach.near, p.reach.far);
                let moved = draw.map_or(0.0, |d| d.line_at(line.x_mm, p.reach, along) - x);
                (across + moved, 0.0, along)
            });
            shapes.push(polygon(class(paint), corners));
        }
    }
    shapes.extend(m.marks.iter().filter_map(|mark| symbol(p, segs, mark, draw)));
    Layer { kind: LayerKind::Markings, shapes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::street::iso::paint::layer_over;
    use crate::street::iso::projection::Reach;
    use crate::street::iso::scene::testing::{each_run, parity};
    use crate::street::model::Editor;

    #[test]
    fn the_markings_layer_equals_the_old_writer_on_every_sample() {
        let mut runs = 0;
        each_run(|_, p, segs, _, draw, _| {
            parity(&markings(p, segs, draw), p, &layer_over(p, segs, draw));
            runs += 1;
        });
        assert!(runs >= 30, "{runs} runs");
    }

    #[test]
    fn paint_stays_inside_the_drawing_when_the_design_is_wider_than_the_street() {
        let v = crate::street::iso::scene::testing::wide();
        for width in [600.0, 1050.0, 1800.0] {
            let p = Projection::fit(width, v.row_mm.max(v.total_mm)).over(Reach::STRETCH);
            let l = markings(&p, &v.segments, None);
            parity(&l, &p, &layer_over(&p, &v.segments, None));
            let m = crate::shared::diagram::render::layer_markup(&l, &p);
            assert!(!m.contains("NaN") && !m.contains("inf"), "{width}");
            let all = crate::street::iso::testing::xs(&m);
            assert!(!all.is_empty());
            assert!(all.iter().all(|x| *x >= 0.0 && *x <= p.width), "{width}: {all:?}");
        }
    }

    #[test]
    fn an_empty_street_gives_an_empty_markings_layer() {
        let mut e = Editor::new(0);
        while let Some(uid) = e.view().segments.first().map(|s| s.uid) {
            assert!(e.remove(uid));
        }
        let v = e.view();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm)).over(Reach::STRETCH);
        let l = markings(&p, &v.segments, None);
        assert!(l.shapes.is_empty());
        parity(&l, &p, "<g class=\"iso-markings\"></g>");
    }
}
