//! The signs layer as shapes: the stops' signs, which stand in the selected run only.

use crate::shared::diagram::{Layer, LayerKind};
use crate::street::model::{SegView, View};

use crate::street::iso::projection::Projection;
use crate::street::iso::stops;

/// The signs of the run `segs`, over everything but the outline; none across the boundary, where the neighbour's run
/// is drawn without them.
pub(in crate::street::iso) fn signs(v: &View, segs: &[SegView], p: &Projection, across: bool) -> Layer {
    let mut shapes = Vec::new();
    if !across {
        shapes.extend(stops::signs(segs, &v.stops_shown, p));
    }
    Layer { kind: LayerKind::Signs, shapes }
}
