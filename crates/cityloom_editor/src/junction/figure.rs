//! The junction's 2.5D as a flat figure, for the design-review report (`drawing` is the page's Leptos view, so the
//! figure lives here). No DOM and no signals.

use cityloom_report::Figure;

use crate::junction::iso::{build_diagram, iso_label};
use crate::junction::read_model::JView;
use crate::shared::flat::flatten;
use crate::shared::i18n::I18n;
use crate::shared::units::Units;

/// The drawing's own size in points, before `flatten` scales it to the box.
const POINT_SIZE: f64 = 252.0;

/// The junction's 2.5D fitted into `box_w` by `box_h`, metric, with no selection.
pub fn figure(v: &JView, box_w: f32, box_h: f32) -> Figure {
    let (frame, diagram) = build_diagram(v, POINT_SIZE, POINT_SIZE, Units::default());
    flatten(&diagram, &frame, box_w, box_h)
}

/// What a screen reader is told the drawing is.
pub fn alt(v: &JView, i18n: &I18n) -> String {
    iso_label(v, i18n)
}
