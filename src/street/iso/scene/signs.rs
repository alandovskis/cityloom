//! The signs layer as shapes: the speed limit sign and the stops' signs, which stand in the selected run only.

use crate::shared::diagram::{Layer, LayerKind};
use crate::shared::i18n::I18n;
use crate::street::model::{SegView, View};

use crate::street::iso::projection::Projection;
use crate::street::iso::{sign, stops};

/// The signs of the run `segs`, over everything but the outline; none across the boundary, where the neighbour's run
/// is drawn without them.
pub(in crate::street::iso) fn signs(v: &View, segs: &[SegView], p: &Projection, i18n: &I18n, across: bool) -> Layer {
    let mut shapes = Vec::new();
    if !across {
        shapes.extend(sign::sign(v, segs, p, i18n));
        shapes.extend(stops::signs(segs, &v.stops_shown, p));
    }
    Layer { kind: LayerKind::Signs, shapes }
}
