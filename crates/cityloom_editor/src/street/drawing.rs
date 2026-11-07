//! The street's 2.5D as a flat figure, for the design-review report: the same diagram the page draws, with no selection
//! outline and no press targets, scaled to a box. No DOM and no signals.

use cityloom_report::Figure;

use crate::shared::flat::flatten;
use crate::shared::i18n::I18n;
use crate::shared::units::Units;
use crate::street::iso::{diagram_and_projection, iso_label};
use crate::street::model::View;

/// The drawing's own width in points, before `flatten` scales it to the box.
const POINT_WIDTH: f64 = 252.0;

/// The street's 2.5D fitted into `box_w` by `box_h` (the view as the editor gives it for the zone shown; a
/// selected piece is not outlined, since the outline and the press targets are not drawn in a figure).
pub fn figure(v: &View, box_w: f32, box_h: f32) -> Figure {
    let (diagram, projection) = diagram_and_projection(v, POINT_WIDTH);
    flatten(&diagram, &projection, box_w, box_h)
}

/// What a screen reader is told the drawing is, in metres.
pub fn alt(v: &View, i18n: &I18n) -> String {
    iso_label(v, i18n, Units::default())
}
