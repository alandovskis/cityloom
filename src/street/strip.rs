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

/// The flex-grow of each zone's cell, as shares that add up to 1: in proportion to its length, but never under
/// `MIN_SHARE` (the cells that would be are pinned at it and the rest share what is left by length). When the
/// zones are too many for the floor to hold, every cell gets an equal share. The target size in pixels is not
/// decided here: it is the view's CSS `min-width` on each cell (the strip scrolls when they do not fit).
pub fn cell_basis(zones: &[ZoneView]) -> Vec<f64> {
    let n = zones.len();
    if n == 0 {
        return Vec::new();
    }
    let equal = 1.0 / n as f64;
    if n as f64 * MIN_SHARE >= 1.0 {
        return vec![equal; n];
    }
    let len: Vec<f64> = zones.iter().map(|z| f64::from((z.to_mm - z.from_mm).max(0))).collect();
    let mut pinned = vec![false; n];
    loop {
        let free: Vec<usize> = (0..n).filter(|&i| !pinned[i]).collect();
        let left = 1.0 - (n - free.len()) as f64 * MIN_SHARE;
        let sum: f64 = free.iter().map(|&i| len[i]).sum();
        let share = |i: usize| if sum > 0.0 { left * len[i] / sum } else { left / free.len() as f64 };
        let short: Vec<usize> = free.iter().copied().filter(|&i| share(i) < MIN_SHARE).collect();
        if short.is_empty() {
            return (0..n).map(|i| if pinned[i] { MIN_SHARE } else { share(i) }).collect();
        }
        for i in short {
            pinned[i] = true;
        }
    }
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

    fn shares(lengths: &[i32]) -> Vec<f64> {
        let mut at = 0;
        let zones: Vec<ZoneView> = lengths
            .iter()
            .enumerate()
            .map(|(i, l)| {
                at += l;
                zone(i, at - l, at)
            })
            .collect();
        let b = cell_basis(&zones);
        let sum: f64 = b.iter().sum();
        b.iter().map(|x| x / sum).collect()
    }

    #[test]
    fn a_short_zone_beside_a_long_one_keeps_at_least_the_floor_share() {
        let s = shares(&[5_000, 2_000_000]);
        assert!(s[0] >= MIN_SHARE - 1e-9, "{s:?}");
        assert!(s[1] > s[0]);
        let even = shares(&[30_000, 30_000]);
        assert!((even[0] - even[1]).abs() < 1e-12);
        assert!(cell_basis(&[]).is_empty());
    }

    #[test]
    fn ten_short_zones_and_a_long_one_each_keep_the_floor_share() {
        let mut l = vec![5_000; 10];
        l.push(2_000_000);
        let s = shares(&l);
        assert!(s.iter().all(|x| *x >= MIN_SHARE - 1e-9), "{s:?}");
        assert!((s.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert!(s[10] > s[0]);
    }

    #[test]
    fn zones_too_many_for_the_floor_get_equal_shares() {
        let s = shares(&[5_000; 40]);
        assert!(s.iter().all(|x| (x - 1.0 / 40.0).abs() < 1e-12), "{s:?}");
        let s = shares(&[5_000, 2_000_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000, 5_000]);
        assert!(s.iter().all(|x| (x - 1.0 / 17.0).abs() < 1e-12), "{s:?}");
    }

    #[test]
    fn a_press_is_a_drag_only_after_a_few_pixels() {
        assert!(!is_boundary_drag(3.9) && !is_boundary_drag(-3.9));
        assert!(is_boundary_drag(4.0) && is_boundary_drag(-4.0) && is_boundary_drag(40.0));
    }
}
