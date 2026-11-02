//! The objects layer as shapes: what stands on the pieces (trees, shelters, posts, racks), a group per box with the faces
//! the eye sees, in the order the eye needs among themselves. Equal to what `run_layers` wrote for `iso-objects`.

use crate::shared::diagram::{Face, Layer, LayerKind, Pt, Shape};
use crate::street::model::{SegView, ShownInstance, ShownStop};

use crate::street::iso::curbside;
use crate::street::iso::order::{self, Bx};
use crate::street::iso::projection::Projection;
use crate::street::iso::raised::inner_face;
use crate::street::iso::solids::{Block, blocks};
use crate::street::iso::stops;

fn face(corners: [(f64, f64, f64); 4], class: &str, face: Face) -> Shape {
    Shape::Plane { pts: corners.map(|(x, y, z)| Pt::new(x, y, z)).to_vec(), class: class.into(), face, labelled: false, hatch: None }
}

/// A box's faces the eye sees: its front, its side on the middle of the street's side (none when it is over the eye)
/// and its top.
pub(in crate::street::iso) fn block(p: &Projection, b: &Block) -> Shape {
    let ((x0, x1), (y0, y1), (z0, z1)) = (b.x, b.y, b.z);
    let mut shapes = vec![face([(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0)], "o-front", Face::Front)];
    if let Some(right) = inner_face(p, x0, x1) {
        let x = if right { x1 } else { x0 };
        shapes.push(face([(x, y0, z0), (x, y0, z1), (x, y1, z1), (x, y1, z0)], "o-side", Face::Side));
    }
    shapes.push(face([(x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)], "o-top", Face::Top));
    Shape::Group { class: format!("iso-solid s-{}", b.solid.name()), uid: None, data: None, hidden: false, title: None, shapes }
}

/// The boxes that stand on the pieces, on the shown curbside uses and on the shown stops, in the order they are painted
/// in among themselves (`order::paint_order`): a crown nearer the middle of the street covers a shelter beyond it.
pub(in crate::street::iso) fn objects(p: &Projection, segs: &[SegView], uses: &[ShownInstance], stops: &[ShownStop]) -> Layer {
    let mut solids: Vec<Block> =
        (0..segs.len()).flat_map(|i| blocks(segs, i, p)).chain(curbside::blocks(segs, uses, p)).chain(stops::blocks(segs, stops, p)).collect();
    solids.sort_by(|a, b| b.from_eye(p).total_cmp(&a.from_eye(p)).then(a.y.0.total_cmp(&b.y.0)));
    let boxes: Vec<(Bx, bool)> = solids.iter().map(|b| (b.bounds(), false)).collect();
    let shapes = order::paint_order(&boxes, p.centre_mm).into_iter().map(|k| block(p, &solids[k])).collect();
    Layer { kind: LayerKind::Objects, shapes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::diagram::render::layer_markup;
    use crate::street::iso::layers::Layer as Old;
    use crate::street::iso::scene::testing::{each_object_run, parity};
    use crate::street::iso::solids::{Solid, block as old_block};

    /// The old writer's objects layer for a run, as `run_layers` wrote it.
    fn old(p: &Projection, segs: &[SegView], uses: &[ShownInstance], stops: &[ShownStop]) -> String {
        let mut solids: Vec<Block> =
            (0..segs.len()).flat_map(|i| blocks(segs, i, p)).chain(curbside::blocks(segs, uses, p)).chain(stops::blocks(segs, stops, p)).collect();
        solids.sort_by(|a, b| b.from_eye(p).total_cmp(&a.from_eye(p)).then(a.y.0.total_cmp(&b.y.0)));
        let boxes: Vec<(Bx, bool)> = solids.iter().map(|b| (b.bounds(), false)).collect();
        let mut out = Old::Objects.open();
        for k in order::paint_order(&boxes, p.centre_mm) {
            old_block(&mut out, p, &solids[k]);
        }
        out.push_str("</g>");
        out
    }

    #[test]
    fn the_objects_layer_equals_the_old_writer_on_every_sample() {
        let (mut runs, mut with) = (0, std::collections::BTreeSet::new());
        each_object_run(|name, p, segs, uses, stops| {
            let layer = objects(p, segs, uses, stops);
            assert_eq!(layer.kind, LayerKind::Objects, "{name}");
            let expected = old(p, segs, uses, stops);
            for s in ["trunk", "crown", "shelter", "post"] {
                if expected.contains(&format!("iso-solid s-{s}\"")) {
                    with.insert(s);
                }
            }
            parity(&layer, p, &expected);
            runs += 1;
        });
        assert!(runs >= 40, "{runs} runs");
        assert_eq!(with.len(), 4, "every kind of solid is exercised: {with:?}");
    }

    #[test]
    fn a_far_lower_object_is_painted_over_a_nearer_taller_strip() {
        // The spec's accepted exception: objects are a layer over every raised strip, so a post beyond a median that
        // stands nearer the middle of the street and higher is still painted over it. Checked on the real assembly.
        use crate::shared::catalogue::kind_index;
        use crate::street::iso::iso_svg;
        use crate::street::iso::projection::Reach;
        use crate::street::iso::projection::drawn_mm;
        use crate::street::iso::raised::height_mm;
        use crate::street::iso::testing::en;
        use crate::street::model::Editor;
        let mut e = Editor::new(0);
        let bike = e.add(kind_index("bike").unwrap(), 1);
        let median = e.add(kind_index("median").unwrap(), 3);
        let v = e.view();
        let p = Projection::fit(1050.0, v.row_mm.max(v.total_mm)).over(Reach::STRETCH);
        let seg = |uid| v.segments.iter().position(|s| s.uid == uid).unwrap();
        let (bi, mi) = (seg(bike), seg(median));
        let m = &v.segments[mi];
        let (m_x0, m_top) = (m.x_mm as f64, drawn_mm(height_mm("median")));
        assert!(m_x0 < p.centre_mm, "the median is left of the eye");
        let post = blocks(&v.segments, bi, &p).into_iter().find(|b| b.solid == Solid::Post).expect("the bike lane has posts");
        assert!(post.x.1 <= m_x0, "the post is farther from the middle of the street than the median");
        assert!(post.y.0 < m_top, "the post stands lower than the median's top");
        let svg = iso_svg(&v, &en(), 1050.0).markup;
        let median_last = svg.rfind(&format!("data-uid=\"{median}\"")).unwrap();
        let post_first = svg.find("iso-solid s-post").unwrap();
        assert!(post_first > median_last, "every post is painted after every raised strip");
    }
}
