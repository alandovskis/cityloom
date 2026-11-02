//! The outline of the selected piece as shapes: a dark casing under a light line, round everything drawn for it.

use crate::shared::catalogue::KINDS;
use crate::shared::diagram::{Layer, LayerKind, Pt, Shape};
use crate::street::model::SegView;
use crate::street::pieces::curb_edges;

use crate::street::iso::projection::{Projection, drawn_mm};
use crate::street::iso::raised::{beside, curb_top_mm, height_mm, inner_face};

/// The outline of the selected piece at `i`, as one shape round everything drawn for it (its strip and its curbs): its
/// near face, the side the eye sees where there is one, and its top, at the height of its highest part. `p` is the
/// projection over the selected run's reach.
pub(in crate::street::iso) fn outline(p: &Projection, segs: &[SegView], i: usize) -> Layer {
    let s = &segs[i];
    let id = KINDS[s.kind].id;
    let top = s.curb.filter(|_| !curb_edges(segs, i).is_empty()).map_or(drawn_mm(height_mm(id)), |c| curb_top_mm(c, id));
    let (x0, x1, l) = (s.x_mm as f64, (s.x_mm + s.width_mm) as f64, p.reach.far);
    let n = p.reach.near;
    let corners = match inner_face(p, x0, x1) {
        // Left of the eye: its right face is seen, so the outline climbs it and comes back along the top.
        Some(true) => {
            let low = beside(segs, i, true).min(top);
            vec![(x0, 0.0, n), (x1, 0.0, n), (x1, low, n), (x1, low, l), (x1, top, l), (x0, top, l), (x0, top, n)]
        }
        Some(false) => {
            let low = beside(segs, i, false).min(top);
            vec![(x1, 0.0, n), (x0, 0.0, n), (x0, low, n), (x0, low, l), (x0, top, l), (x1, top, l), (x1, top, n)]
        }
        None => vec![(x0, 0.0, n), (x1, 0.0, n), (x1, top, n), (x1, top, l), (x0, top, l), (x0, top, n)],
    };
    let pts: Vec<Pt> = corners.into_iter().map(|(x, y, z)| Pt::new(x, y, z)).collect();
    // A dark casing under the light line: one of the two is seen on any material, in either theme.
    let line = |class: &str| Shape::Polygon { pts: pts.clone(), class: class.into(), face: None, texture: None };
    Layer { kind: LayerKind::Outline, shapes: vec![line("iso-sel-casing"), line("iso-sel")] }
}
