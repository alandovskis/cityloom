//! The zone strip's geometry, as pure decisions the component carries out.

use super::model::{ZoneView, snap};

/// A press moves a boundary only after it has moved this far.
const DRAG_START_PX: f64 = 4.0;
/// No zone's cell is narrower than this share of the strip, so a short zone of a long street is still a target.
const MIN_SHARE: f64 = 0.06;

/// A position along the street from a pointer position on the strip, snapped to `SNAP_MM` and kept within the street.
pub fn boundary_mm(x_px: f64, strip_px: f64, length_mm: i32) -> i32 {
    if strip_px <= 0.0 || length_mm <= 0 {
        return 0;
    }
    let mm = (x_px / strip_px * length_mm as f64).round().clamp(0.0, length_mm as f64) as i32;
    snap(mm).min(length_mm)
}

/// The flex-grow of each zone's cell: its length, but never under `MIN_SHARE` of the whole.
pub fn cell_basis(zones: &[ZoneView]) -> Vec<f64> {
    let total: f64 = zones.iter().map(|z| f64::from((z.to_mm - z.from_mm).max(0))).sum();
    let floor = total * MIN_SHARE;
    zones.iter().map(|z| f64::from((z.to_mm - z.from_mm).max(0)).max(floor)).collect()
}

/// Whether a press on a boundary has moved far enough to be a drag.
pub fn is_boundary_drag(dx_px: f64) -> bool {
    dx_px.abs() >= DRAG_START_PX
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::said::Said;

    fn zone(index: usize, from_mm: i32, to_mm: i32) -> ZoneView {
        ZoneView { index, from_mm, to_mm, label: Said::new("zone-unnamed"), own: false, cut_off: false, ok: true }
    }

    #[test]
    fn a_pointer_position_is_a_position_along_the_street_snapped_and_clamped() {
        assert_eq!(boundary_mm(0.0, 600.0, 60_000), 0);
        assert_eq!(boundary_mm(300.0, 600.0, 60_000), 30_000);
        assert_eq!(boundary_mm(640.0, 600.0, 60_000), 60_000);
        assert_eq!(boundary_mm(-20.0, 600.0, 60_000), 0);
        assert_eq!(boundary_mm(301.2, 600.0, 60_000), 30_100);
        assert_eq!(boundary_mm(10.0, 0.0, 60_000), 0, "a strip with no width");
    }

    #[test]
    fn a_short_zone_of_a_long_street_keeps_a_target_wide_enough() {
        let zones = [zone(0, 0, 5_000), zone(1, 5_000, 2_000_000)];
        let b = cell_basis(&zones);
        assert_eq!(b.len(), 2);
        // Of a 600 px strip the short zone gets at least 24 px, and the long one stays the longer.
        let share = b[0] / (b[0] + b[1]);
        assert!(share * 600.0 >= 24.0, "share {share}");
        assert!(b[1] > b[0]);
        // Zones in proportion where none is short.
        let even = cell_basis(&[zone(0, 0, 30_000), zone(1, 30_000, 60_000)]);
        assert_eq!(even[0], even[1]);
        assert!(cell_basis(&[]).is_empty());
    }

    #[test]
    fn a_press_is_a_drag_only_after_a_few_pixels() {
        assert!(!is_boundary_drag(3.9) && !is_boundary_drag(-3.9));
        assert!(is_boundary_drag(4.0) && is_boundary_drag(-4.0) && is_boundary_drag(40.0));
    }
}
