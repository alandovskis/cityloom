//! Where a point of the street lands in the drawing, and the sizes the 2.5D is laid out with.
//!
//! The street is seen in perspective from its centre line: the eye is `EYE_HEIGHT_MM` up and `EYE_BACK_MM` behind the
//! near end, over the middle of the street, looking along it. A point `along` mm from the near end is `EYE_BACK_MM +
//! along` from the eye, and lands `across` mm from the centre line at `across * f / depth` pixels from the drawing's
//! middle and `(eye height - up) * f / depth` pixels under the horizon. So the centre line is vertical, parallel lines
//! along the street meet at one vanishing point on the horizon, and the street narrows with distance.

/// How long a stretch of the street is shown, in millimetres.
pub const STRETCH_MM: f64 = 40_000.0;
/// How high the eye is above the ground, in millimetres: above the tallest crown, so that every top is seen from above.
pub const EYE_HEIGHT_MM: f64 = 9_000.0;
/// How far behind the near end of the stretch the eye is, in millimetres.
pub const EYE_BACK_MM: f64 = 14_000.0;
/// The tallest the drawing grows, so that it stays about as tall as the section.
pub const MAX_HEIGHT: f64 = 560.0;
/// Room round the scene, in pixels.
pub(super) const PAD: f64 = 30.0;
/// The narrowest the drawing is laid out for: the group around it is no narrower (`.wrap`'s `min-width` in
/// `web/editor.css`). On a narrower screen, 400 px say, it does not fit: it scrolls sideways, as the section
/// does (laid out at least 680 px wide, `Geometry::fit`).
const MIN_WIDTH: f64 = 600.0;
/// The highest anything is drawn, in millimetres above the ground. The tallest is a tree: 5 m on a planting strip drawn
/// `RISE` times its height (5.45 m); the rest is a margin for its strokes. The drawing is as tall as the far end of
/// the ground and this need, so a street with no trees leaves a little room above it, and no crown is cut off.
pub const TALLEST_MM: f64 = 5_500.0;

/// How many times higher than they stand the slabs and curbs are drawn, so that a rise reads.
pub const RISE: f64 = 3.0;

/// How high a rise of `mm` is drawn, in millimetres of the drawing.
pub fn drawn_mm(mm: i32) -> f64 {
    mm as f64 * RISE
}

/// How many bands a textured top is cut into along the street (`band_edges`).
pub const BANDS: usize = 6;

/// How a point of the street, in millimetres across it, up from the ground and along it from the
/// near end, lands in the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    /// Pixels to the millimetre across the street at its near end (a thing `along` mm further is drawn smaller
    /// by `EYE_BACK_MM / (EYE_BACK_MM + along)`).
    pub scale: f64,
    /// Where the eye is across the street, the middle of the scene, in millimetres.
    pub centre_mm: f64,
    /// Where the middle of the street is on the drawing, in pixels from the left.
    pub centre_px: f64,
    /// Where the horizon is, in pixels from the top (above the drawing's top when the street is low).
    pub horizon: f64,
    pub width: f64,
    pub height: f64,
}

impl Projection {
    /// The projection for a drawing `width` wide, or `MIN_WIDTH` when that is narrower, showing a street
    /// `span_mm` wide: the near end of the street fills that width, unless that would make the drawing taller
    /// than `MAX_HEIGHT`; then it is centred. The top of the drawing is the top of the tallest crown at the far end.
    pub fn fit(width: f64, span_mm: i32) -> Projection {
        let width = width.max(MIN_WIDTH);
        let across = span_mm.max(1) as f64;
        // How far the tallest thing at the far end stands above the horizon, in near-end millimetres.
        let headroom = (EYE_HEIGHT_MM - TALLEST_MM) * EYE_BACK_MM / (EYE_BACK_MM + STRETCH_MM);
        let scale = ((width - 2.0 * PAD) / across).min((MAX_HEIGHT - 2.0 * PAD) / (EYE_HEIGHT_MM - headroom));
        let horizon = PAD - headroom * scale;
        Projection { scale, centre_mm: across / 2.0, centre_px: width / 2.0, horizon, width, height: horizon + EYE_HEIGHT_MM * scale + PAD }
    }

    /// Pixels to the millimetre at `along_mm` from the near end.
    pub fn scale_at(&self, along_mm: f64) -> f64 {
        self.scale * EYE_BACK_MM / (EYE_BACK_MM + along_mm)
    }

    /// Where a point is in the drawing, in pixels.
    pub fn at(&self, x_mm: f64, up_mm: f64, along_mm: f64) -> (f64, f64) {
        let k = self.scale_at(along_mm);
        (self.centre_px + (x_mm - self.centre_mm) * k, self.horizon + (EYE_HEIGHT_MM - up_mm) * k)
    }

    /// The matrix (as `a b c d e f`, `x_px = a*u + c*v + e`, `y_px = b*u + d*v + f`) that is nearest to the projection
    /// of the plane `up_mm` high, from `(u, v)` = (across, along) in millimetres, about the point `(u, v)`. It is the
    /// projection's slope there, so a texture laid by it has the right size and lean at that depth and drifts from
    /// the true perspective away from it.
    pub fn ground_at(&self, up_mm: f64, u: f64, v: f64) -> [f64; 6] {
        let depth = EYE_BACK_MM + v;
        let k = self.scale_at(v);
        let (a, c, d) = (k, -(u - self.centre_mm) * k / depth, -(EYE_HEIGHT_MM - up_mm) * k / depth);
        let (x, y) = self.at(u, up_mm, v);
        [a, 0.0, c, d, x - a * u - c * v, y - d * v]
    }
}

/// How far along the street each of the `BANDS` bands of a textured top begins, and the end of the last: equal steps
/// in depth from the eye's point of view, so that a band is as much farther than the one before as that one is
/// than the one before it and the texture's scale changes by the same step at every seam.
pub fn band_edges() -> [f64; BANDS + 1] {
    let ratio = ((EYE_BACK_MM + STRETCH_MM) / EYE_BACK_MM).powf(1.0 / BANDS as f64);
    let mut edges = [0.0; BANDS + 1];
    for (i, e) in edges.iter_mut().enumerate() {
        *e = (EYE_BACK_MM * ratio.powi(i as i32) - EYE_BACK_MM).min(STRETCH_MM);
    }
    edges[BANDS] = STRETCH_MM;
    edges
}

/// A piece narrower than this on screen, at the depth where a symbol is painted on it, draws without that symbol. It
/// is below the section's 26 px.
pub const MIN_SOLID_PX: f64 = 20.0;
/// A piece narrower than this on screen, at the depth where something stands on it, draws without that thing. It is
/// lower than `MIN_SOLID_PX`: a tree or a post is a box standing up off its piece, so it reads on a narrow strip
/// where a flat symbol does not, and a tree shrinks with distance as it should, far trees being narrow boxes and
/// not missing.
pub const MIN_STANDING_PX: f64 = 10.0;
/// How far along the street a lane's symbol (all within 2.5 m nearer and 2 m farther of it, `symbols.rs`) is laid, in
/// millimetres: near the near end, where the street is widest on the screen, so that the symbol reads.
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
    fn a_point_on_the_centre_line_at_the_ground_is_at_the_middle_of_the_drawing_and_the_line_is_vertical() {
        let p = Projection::fit(1000.0, 18_000);
        for along in [0.0, 5_000.0, STRETCH_MM] {
            assert!((p.at(9_000.0, 0.0, along).0 - 500.0).abs() < 1e-9, "{along}");
        }
        // Whatever the height: the centre line and the plane over it are one vertical line.
        assert!((p.at(9_000.0, 3_000.0, 12_000.0).0 - 500.0).abs() < 1e-9);
        // The drawing's middle is the street's, not the box's: an odd span is centred as well.
        assert!((Projection::fit(1000.0, 7_001).at(3_500.5, 0.0, 0.0).0 - 500.0).abs() < 1e-9);
    }

    #[test]
    fn lines_along_the_street_meet_at_one_vanishing_point_on_the_horizon() {
        let p = Projection::fit(1000.0, 18_000);
        for (x, up) in [(0.0, 0.0), (18_000.0, 450.0), (4_000.0, 3_000.0)] {
            let (vx, vy) = p.at(x, up, 1.0e13);
            assert!((vx - 500.0).abs() < 1e-3 && (vy - p.horizon).abs() < 1e-3, "({x}, {up}): ({vx}, {vy})");
        }
        // The horizon is where the eye's height is: a point at the eye's height is on it at every depth.
        assert!((p.at(0.0, EYE_HEIGHT_MM, 7_000.0).1 - p.horizon).abs() < 1e-9);
        // And a line along the street is straight on the screen: three of its points are collinear.
        let (a, b, c) = (p.at(2_000.0, 0.0, 0.0), p.at(2_000.0, 0.0, 15_000.0), p.at(2_000.0, 0.0, STRETCH_MM));
        let cross = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        assert!(cross.abs() < 1e-6, "{cross}");
    }

    #[test]
    fn the_near_end_is_wider_than_the_far_end_by_the_ratio_of_their_depths() {
        let p = Projection::fit(1000.0, 18_000);
        let width = |along: f64| p.at(18_000.0, 0.0, along).0 - p.at(0.0, 0.0, along).0;
        assert!(width(0.0) > width(STRETCH_MM));
        assert!((width(STRETCH_MM) / width(0.0) - EYE_BACK_MM / (EYE_BACK_MM + STRETCH_MM)).abs() < 1e-12);
        // The near end fills the drawing less its margins; farther is higher on the screen.
        assert!((width(0.0) - (1000.0 - 2.0 * PAD)).abs() < 1e-9);
        assert!(p.at(0.0, 0.0, STRETCH_MM).1 < p.at(0.0, 0.0, 0.0).1);
        // What is higher is higher on the screen, at one depth.
        assert!(p.at(0.0, 450.0, 0.0).1 < p.at(0.0, 0.0, 0.0).1);
    }

    #[test]
    fn the_scene_fits_the_width_and_a_narrow_street_is_held_to_the_height_and_centred() {
        // A wide street: the width decides.
        let wide = Projection::fit(1000.0, 31_000);
        assert!((wide.at(0.0, 0.0, 0.0).0 - PAD).abs() < 1e-9 && (wide.at(31_000.0, 0.0, 0.0).0 - (1000.0 - PAD)).abs() < 1e-9);
        assert!(wide.height <= MAX_HEIGHT);
        // A narrow street: the height decides, so the drawing is no taller than MAX_HEIGHT, and the street is centred.
        let narrow = Projection::fit(1000.0, 4_000);
        assert!((narrow.height - MAX_HEIGHT).abs() < 1e-9, "{narrow:?}");
        assert!(narrow.at(0.0, 0.0, 0.0).0 > PAD);
        assert!((narrow.at(0.0, 0.0, 0.0).0 - (1000.0 - narrow.at(4_000.0, 0.0, 0.0).0)).abs() < 1e-9, "centred: {narrow:?}");
        // The drawing is laid out no narrower than the section's group.
        assert_eq!(Projection::fit(100.0, 18_000).width, 600.0);
        // A street of no width still gives a scale.
        assert!(Projection::fit(1000.0, 0).scale.is_finite());
    }

    #[test]
    fn the_tallest_crown_at_the_far_end_is_at_the_top_margin_and_nothing_stands_higher() {
        for (width, span) in [(600.0, 18_000), (1050.0, 18_000), (1440.0, 31_000), (1050.0, 4_000)] {
            let p = Projection::fit(width, span);
            let top = p.at(0.0, TALLEST_MM, STRETCH_MM).1;
            assert!((top - PAD).abs() < 1e-9, "{width}: {top}");
            // A crown nearer is lower on the screen (the eye is above it).
            assert!(p.at(0.0, TALLEST_MM, 0.0).1 > top);
            // The tree of the tallest strip: 5 m on a strip drawn RISE times its 150 mm.
            assert!(TALLEST_MM >= 5_000.0 + drawn_mm(150));
        }
    }

    #[test]
    fn the_ground_matrix_is_the_projection_s_slope_at_its_point() {
        let p = Projection::fit(1000.0, 18_000);
        let m = p.ground_at(450.0, 3_000.0, 10_000.0);
        // It maps its own point exactly where the projection puts it.
        let (x, y) = p.at(3_000.0, 450.0, 10_000.0);
        assert!(close((m[0] * 3_000.0 + m[2] * 10_000.0 + m[4], m[1] * 3_000.0 + m[3] * 10_000.0 + m[5]), (x, y)));
        // And a step of a metre along or across lands where the projection puts it, to within the curvature.
        let step = |du: f64, dv: f64| {
            let (px, py) = p.at(3_000.0 + du, 450.0, 10_000.0 + dv);
            let (ax, ay) = (m[0] * (3_000.0 + du) + m[2] * (10_000.0 + dv) + m[4], m[1] * (3_000.0 + du) + m[3] * (10_000.0 + dv) + m[5]);
            ((px - ax).abs(), (py - ay).abs())
        };
        assert!(step(1_000.0, 0.0).0 < 1e-9);
        assert!(step(0.0, 500.0).0 < 0.1 && step(0.0, 500.0).1 < 0.15, "{:?}", step(0.0, 500.0));
    }

    #[test]
    fn the_bands_cover_the_stretch_and_grow_with_depth() {
        let e = band_edges();
        assert_eq!((e[0], e[BANDS]), (0.0, STRETCH_MM));
        assert!(e.windows(2).all(|w| w[0] < w[1]));
        // Equal steps of depth: the ratio of each band's far depth to its near depth is the same.
        let ratios: Vec<f64> = e.windows(2).map(|w| (EYE_BACK_MM + w[1]) / (EYE_BACK_MM + w[0])).collect();
        assert!(ratios.iter().all(|r| (r - ratios[0]).abs() < 1e-9), "{ratios:?}");
        assert!(e.windows(3).all(|w| w[1] - w[0] < w[2] - w[1]));
    }
}
