//! The junction's 2.5D as a flat figure, for the design-review report (`drawing` is the page's Leptos view, so the
//! figure lives here). No DOM and no signals.

use cityloom_report::Figure;

use crate::junction::iso::{build_diagram, iso_label};
use crate::junction::read_model::JView;
use crate::shared::flat::flatten_fit;
use crate::shared::i18n::I18n;
use crate::shared::units::Units;

/// The frame the drawing is laid out in, large so that the names are never clamped away from the street they name; `flatten_fit` fits it to the box.
const POINT_SIZE: f64 = 1000.0;

/// The junction's 2.5D fitted into `box_w` by `box_h`, its words in `i18n`'s language and its lengths in `units`; the junction's own selection
/// is not outlined (the outline layer is not drawn).
pub fn figure(v: &JView, i18n: &I18n, units: Units, box_w: f32, box_h: f32) -> Figure {
    let (frame, diagram) = build_diagram(v, i18n, POINT_SIZE, POINT_SIZE, units);
    flatten_fit(&diagram, &frame, box_w, box_h)
}

/// What a screen reader is told the drawing is.
pub fn alt(v: &JView, i18n: &I18n) -> String {
    iso_label(v, i18n)
}
