//! Where a point of the street lands in the drawing, and the sizes the 2.5D is laid out with.

/// How long a stretch of the street is shown, in millimetres.
pub const STRETCH_MM: f64 = 40_000.0;
/// How far a millimetre along the street moves the drawing right, and up: half scale, at the
/// angle of a 3-4-5 triangle (about 37 degrees), so the points it gives are exact.
pub const RECEDE_X: f64 = 0.4;
pub const RECEDE_Y: f64 = 0.3;
/// The tallest the drawing grows, so that it stays about as tall as the section.
pub const MAX_HEIGHT: f64 = 560.0;
/// Room round the scene, in pixels.
pub(super) const PAD: f64 = 30.0;
/// The narrowest the drawing is laid out for: the group around it is no narrower (`.wrap`'s `min-width` in
/// `web/editor.css`). On a narrower screen, 400 px say, it does not fit: it scrolls sideways, as the section
/// does (laid out at least 680 px wide, `Geometry::fit`).
const MIN_WIDTH: f64 = 600.0;
/// Room above the far end of the ground for what stands on the pieces. The tallest is a tree, 5 m on a
/// planting strip drawn `RISE` times its height (5.45 m); the farthest crown ends 2.8 m short of the far
/// end, so it needs 4.61 m of this; the rest (0.39 m) is a margin for its strokes.
const HEADROOM_MM: f64 = 5_000.0;

/// How many times higher than they stand the slabs and curbs are drawn, so that a rise reads.
pub const RISE: f64 = 3.0;

/// How high a rise of `mm` is drawn, in millimetres of the drawing.
pub fn drawn_mm(mm: i32) -> f64 {
    mm as f64 * RISE
}

/// How a point of the street, in millimetres across it, up from the ground and along it from the
/// near end, lands in the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    /// Pixels to the millimetre.
    pub scale: f64,
    /// Where the street's left edge is at its near end, in pixels.
    pub left: f64,
    /// Where the ground is at the near end, in pixels from the top.
    pub base: f64,
    pub width: f64,
    pub height: f64,
}

impl Projection {
    /// The projection for a drawing `width` wide, or `MIN_WIDTH` when that is narrower, showing a street
    /// `span_mm` wide: the scene fills that width, unless that would make the drawing taller than
    /// `MAX_HEIGHT`; then it is centred.
    pub fn fit(width: f64, span_mm: i32) -> Projection {
        let width = width.max(MIN_WIDTH);
        let across = span_mm.max(1) as f64 + STRETCH_MM * RECEDE_X;
        let up = STRETCH_MM * RECEDE_Y + HEADROOM_MM;
        let scale = ((width - 2.0 * PAD) / across).min((MAX_HEIGHT - 2.0 * PAD) / up);
        let left = (width - across * scale) / 2.0;
        let base = PAD + up * scale;
        Projection { scale, left, base, width, height: base + PAD }
    }

    /// Where a point is in the drawing, in pixels.
    pub fn at(&self, x_mm: f64, up_mm: f64, along_mm: f64) -> (f64, f64) {
        (self.left + (x_mm + along_mm * RECEDE_X) * self.scale, self.base - (up_mm + along_mm * RECEDE_Y) * self.scale)
    }
}

/// A piece narrower than this on screen draws without what stands on it and without its arrow. It is below the
/// section's 26 px: the 2.5D draws a street smaller (it fits the recession across too), and a tree is a box
/// standing up off its strip, so it reads on a 1.5 m strip at 760 px (about 21 px).
pub const MIN_SOLID_PX: f64 = 20.0;
/// How far along the street a one-way arrow is laid, in millimetres: near the near end, because what stands
/// on the pieces hides only the ground farther along than itself: a tree's crown nothing nearer than about 10 m,
/// and the nearest trunk, from 5.4 m on, only a band of ground within 2.5 m to the left of its strip.
pub const ARROW_AT_MM: f64 = 6_000.0;

/// A pixel value to two decimals, which is all a drawing needs.
pub(super) fn r(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The points of a polygon whose corners are in millimetres: across, up, along.
pub(super) fn points(p: &Projection, corners: &[(f64, f64, f64)]) -> String {
    corners
        .iter()
        .map(|&(x, y, z)| {
            let (sx, sy) = p.at(x, y, z);
            format!("{},{}", r(sx), r(sy))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn the_projection_puts_the_corners_of_a_unit_box_exactly_where_the_oblique_view_has_them() {
        // 31 m of street and 16 m of depth across 940 px: 0.02 px to the millimetre; 12 m of depth and 5 m of headroom up.
        let p = Projection::fit(1000.0, 31_000);
        assert!((p.scale - 0.02).abs() < 1e-12, "{}", p.scale);
        assert_eq!((p.left, p.width), (30.0, 1000.0));
        assert!((p.base - 370.0).abs() < 1e-9 && (p.height - 400.0).abs() < 1e-9, "{p:?}");
        // Width runs right at full scale, height runs up, depth runs up and right at half scale.
        assert!(close(p.at(0.0, 0.0, 0.0), (30.0, 370.0)));
        assert!(close(p.at(1_000.0, 0.0, 0.0), (50.0, 370.0)));
        assert!(close(p.at(0.0, 1_000.0, 0.0), (30.0, 350.0)));
        assert!(close(p.at(0.0, 0.0, 1_000.0), (38.0, 364.0)));
        assert!(close(p.at(1_000.0, 1_000.0, 1_000.0), (58.0, 344.0)));
    }

    #[test]
    fn the_scene_fits_the_width_and_a_narrow_street_is_held_to_the_height_and_centred() {
        // A wide street: the width decides, the scene spans the drawing less its margins.
        let wide = Projection::fit(1000.0, 31_000);
        let across = 31_000.0 + STRETCH_MM * RECEDE_X;
        assert!((wide.left + across * wide.scale - (1000.0 - 30.0)).abs() < 1e-9);
        // A narrow street: the height decides, so the drawing is no taller than MAX_HEIGHT, and the scene is centred.
        let narrow = Projection::fit(1000.0, 4_000);
        assert!((narrow.height - MAX_HEIGHT).abs() < 1e-9, "{narrow:?}");
        let across = 4_000.0 + STRETCH_MM * RECEDE_X;
        assert!((narrow.left - (1000.0 - (narrow.left + across * narrow.scale))).abs() < 1e-9, "centred: {narrow:?}");
        // The drawing is laid out no narrower than the section's group.
        assert_eq!(Projection::fit(100.0, 18_000).width, 600.0);
        // A street of no width still gives a scale.
        assert!(Projection::fit(1000.0, 0).scale.is_finite());
    }
}
