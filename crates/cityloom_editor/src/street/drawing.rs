//! The street's 2.5D as a flat figure, for the design-review report: the same diagram the page draws, with no selection
//! outline and no press targets, scaled to a box. No DOM and no signals.

use cityloom_report::Figure;

use crate::shared::diagram::Diagram;
use crate::shared::flat::{flatten, flatten_pair};
use crate::shared::i18n::I18n;
use crate::shared::units::Units;
use crate::street::iso::{Projection, diagram_and_projection, iso_label};
use crate::street::model::View;

/// The drawing's own width in points, before `flatten` scales it to the box.
const POINT_WIDTH: f64 = 252.0;

/// The diagram and projection of the shown zone's section alone: across a boundary the neighbour's (`iso-across`) is left out.
fn drawn(v: &View, i18n: &I18n) -> (Diagram, Projection) {
    let (mut diagram, projection) = diagram_and_projection(v, i18n, POINT_WIDTH);
    diagram.sections.retain(|s| !s.class.as_deref().is_some_and(|c| c.split_whitespace().any(|w| w == "iso-across")));
    (diagram, projection)
}

/// The street's 2.5D fitted into `box_w` by `box_h`, its words in `i18n`'s language. A selected piece is not outlined,
/// because `flatten` skips the outline and press-target layers.
pub fn figure(v: &View, i18n: &I18n, box_w: f32, box_h: f32) -> Figure {
    let (diagram, projection) = drawn(v, i18n);
    flatten(&diagram, &projection, box_w, box_h)
}

/// Before and After drawn at one scale (the smaller of the two that fit the box), so the same width looks the same in both.
pub fn figures(before: &View, after: &View, i18n: &I18n, box_w: f32, box_h: f32) -> (Figure, Figure) {
    let (db, pb) = drawn(before, i18n);
    let (da, pa) = drawn(after, i18n);
    flatten_pair((&db, &pb), (&da, &pa), box_w, box_h)
}

/// What a screen reader is told the drawing is, in metres.
pub fn alt(v: &View, i18n: &I18n) -> String {
    iso_label(v, i18n, Units::default())
}
