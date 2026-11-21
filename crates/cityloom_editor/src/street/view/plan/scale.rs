//! How the plan's ground is laid along its width: millimetres of street to pixels, one linear scale for the whole
//! street, the street fitted to the width it is given unless that would draw it smaller than `MIN_PX_PER_M`.

use crate::street::model::MIN_ZONE_MM;

/// The smallest scale the plan is drawn at, in pixels per metre: the shortest stretch a pick can be (`MIN_ZONE_MM`)
/// is then 24 px wide, the size of a target (WCAG 2.5.8), as a cell of the zone strip was at least. A street longer
/// than its width allows at this scale is drawn wider than the column, and the plan's own box scrolls sideways.
pub const MIN_PX_PER_M: f64 = 24.0 / (MIN_ZONE_MM as f64 / 1000.0);

/// Millimetres of street per pixel for a street `length_mm` long fitted to `width_px`, held to `MIN_PX_PER_M`.
pub fn mm_per_px(length_mm: i32, width_px: f64) -> f64 {
    (f64::from(length_mm.max(1)) / width_px.max(1.0)).min(1000.0 / MIN_PX_PER_M)
}

/// Where `at_mm` from the plan's left end lies, in pixels from its left edge.
pub fn x_px(at_mm: i32, mm_per_px: f64) -> f64 {
    f64::from(at_mm) / mm_per_px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_street_is_fitted_to_its_width_and_positions_are_linear() {
        let mpp = mm_per_px(60_000, 600.0);
        assert_eq!(mpp, 100.0);
        assert_eq!(x_px(0, mpp), 0.0);
        assert_eq!(x_px(30_000, mpp), 300.0);
        assert_eq!(x_px(60_000, mpp), 600.0);
    }

    #[test]
    fn the_shortest_stretch_is_a_target_wide_at_the_minimum_scale() {
        assert_eq!(x_px(MIN_ZONE_MM, 1000.0 / MIN_PX_PER_M), 24.0);
    }
}
