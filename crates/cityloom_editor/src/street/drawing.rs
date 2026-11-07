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

/// The street's 2.5D fitted into `box_w` by `box_h`, its words in `i18n`'s language. Only the shown zone's section is
/// drawn: across a boundary the neighbour's (`iso-across`) is left out. A selected piece is not outlined, because
/// `flatten` skips the outline and press-target layers.
pub fn figure(v: &View, i18n: &I18n, box_w: f32, box_h: f32) -> Figure {
    let (mut diagram, projection) = diagram_and_projection(v, i18n, POINT_WIDTH);
    diagram.sections.retain(|s| !s.class.as_deref().is_some_and(|c| c.split_whitespace().any(|w| w == "iso-across")));
    flatten(&diagram, &projection, box_w, box_h)
}

/// What a screen reader is told the drawing is, in metres.
pub fn alt(v: &View, i18n: &I18n) -> String {
    iso_label(v, i18n, Units::default())
}
