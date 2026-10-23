//! Where a point of the street lands in the drawing, and the sizes the 2.5D is laid out with.
//!
//! The street is seen in perspective from its centre line: the eye is `EYE_HEIGHT_MM` up and `EYE_BACK_MM` behind the
//! near end, over the middle of the street, looking along it. A point `along` mm from the near end is `EYE_BACK_MM +
//! along` from the eye, and lands `across` mm from the centre line at `across * f / depth` pixels from the drawing's
//! middle and `(eye height - up) * f / depth` pixels under the horizon. So the centre line is vertical, parallel lines
//! along the street meet at one vanishing point on the horizon, and the street narrows with distance.

use crate::street::svg::Geometry;

/// How long a stretch of the street is shown, in millimetres.
pub const STRETCH_MM: f64 = 40_000.0;
/// How high the eye is above the ground, in millimetres: above the tallest crown, so that every top is seen from above.
pub const EYE_HEIGHT_MM: f64 = 9_000.0;
/// How far behind the near end of the stretch the eye is, in millimetres.
pub const EYE_BACK_MM: f64 = 14_000.0;
/// The tallest the drawing grows at a `width`: as tall as the section is (`Geometry::fit`), which is as tall as the
/// group around either view (`street/view.rs` keeps its `min-height` at least that). So the 2.5D never makes the card
/// taller than the section does, and the status line under it stays in view at 1440 by 900 as it does with the
/// section; a narrow street is drawn smaller, centred, rather than taller.
pub fn max_height(width: f64) -> f64 {
    Geometry::fit(width, 1).rows.height
}
/// Room round the scene, in pixels.
pub(super) const PAD: f64 = 30.0;
/// The narrowest the drawing is laid out for, in pixels: about the narrowest phone's group. The 2.5D is read-only, has no
/// handles and needs no sideways scroll, so it is scaled to the width it has down to this.
pub(super) const MIN_WIDTH: f64 = 280.0;
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

/// The part of the stretch a drawing covers along the street, in millimetres from its near end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reach {
    pub near: f64,
    pub far: f64,
}

impl Reach {
    /// The whole stretch.
    pub const STRETCH: Reach = Reach { near: 0.0, far: STRETCH_MM };

    /// How long it is, in millimetres.
    pub fn len(self) -> f64 {
        self.far - self.near
    }
}

/// How many bands a textured top is cut into along the street (`band_edges`).
pub const BANDS: usize = 8;

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
    /// The part of the stretch the layers draw: the whole of it, unless `over` says otherwise. The scene's own fit (the
    /// headroom, the ground) stays on the whole stretch.
    pub reach: Reach,
}

impl Projection {
    /// The projection for a drawing `width` wide, or `MIN_WIDTH` when that is narrower, showing a street
    /// `span_mm` wide: the near end of the street fills that width, unless that would make the drawing taller
    /// than `max_height`; then it is centred. The top of the drawing is the top of the tallest crown at the far end.
    pub fn fit(width: f64, span_mm: i32) -> Projection {
        let width = width.max(MIN_WIDTH);
        let across = span_mm.max(1) as f64;
        // How far the tallest thing at the far end stands above the horizon, in near-end millimetres.
        let headroom = (EYE_HEIGHT_MM - TALLEST_MM) * EYE_BACK_MM / (EYE_BACK_MM + STRETCH_MM);
        let scale = ((width - 2.0 * PAD) / across).min((max_height(width) - 2.0 * PAD) / (EYE_HEIGHT_MM - headroom));
        let horizon = PAD - headroom * scale;
        Projection {
            scale,
            centre_mm: across / 2.0,
            centre_px: width / 2.0,
            horizon,
            width,
            height: horizon + EYE_HEIGHT_MM * scale + PAD,
            reach: Reach::STRETCH,
        }
    }

    /// The same projection, drawing only `reach`.
    pub fn over(self, reach: Reach) -> Projection {
        Projection { reach, ..self }
    }

    /// Pixels to the millimetre at `along_mm` from the near end.
    pub fn scale_at(&self, along_mm: f64) -> f64 {
        debug_assert!(EYE_BACK_MM + along_mm > 0.0);
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
/// of `1 / depth`, which is equal steps of the drawing's own scale and so equal steps down the screen. A texture laid
/// band by band is out of true at a seam by the step in scale there, and the step shows in pixels as the scale at the
/// seam times it: equal steps of depth, or of its logarithm, leave the nearest seam, where the street is biggest on the
/// screen, with the biggest step; equal steps of the scale give every seam about the same.
pub fn band_edges(reach: Reach) -> [f64; BANDS + 1] {
    let (near, far) = (1.0 / (EYE_BACK_MM + reach.near), 1.0 / (EYE_BACK_MM + reach.far));
    let mut edges = [0.0; BANDS + 1];
    for (i, e) in edges.iter_mut().enumerate() {
        *e = 1.0 / (near + (far - near) * i as f64 / BANDS as f64) - EYE_BACK_MM;
    }
    edges[0] = reach.near;
    edges[BANDS] = reach.far;
    edges
}

/// How far along the street a band from `z0` to `z1` is laid: the middle in `1 / depth`, where its scale is the mean
/// of the scales at its two ends.
pub fn band_mid(z0: f64, z1: f64) -> f64 {
    2.0 / (1.0 / (EYE_BACK_MM + z0) + 1.0 / (EYE_BACK_MM + z1)) - EYE_BACK_MM
}

/// A piece narrower than this on screen, at the depth where a symbol is painted on it, draws without that symbol. It
/// is below the section's 26 px.
pub const MIN_SOLID_PX: f64 = 20.0;
/// A piece narrower than this on screen, at the depth where something stands on it, draws without that thing. It is
/// a few pixels, far below `MIN_SOLID_PX`: a tree or a post is a box standing up off its piece, so it reads on a narrow
/// strip where a flat symbol does not, and a tree shrinks with distance as it should: a planted strip looks planted to
/// the far end of the stretch (a 1.5 m strip at 600 px is 7 px wide there), and only a piece that is a sliver on
/// the screen loses what stands on it.
pub const MIN_STANDING_PX: f64 = 3.0;
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
        let p = Projection::fit(1000.0, 30_000);
        let width = |along: f64| p.at(30_000.0, 0.0, along).0 - p.at(0.0, 0.0, along).0;
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
        assert!(wide.height <= max_height(1000.0));
        // A narrow street: the height decides, so the drawing is no taller than `max_height`, and the street is centred.
        let narrow = Projection::fit(1000.0, 4_000);
        assert!((narrow.height - max_height(1000.0)).abs() < 1e-9, "{narrow:?}");
        assert!(narrow.at(0.0, 0.0, 0.0).0 > PAD);
        assert!((narrow.at(0.0, 0.0, 0.0).0 - (1000.0 - narrow.at(4_000.0, 0.0, 0.0).0)).abs() < 1e-9, "centred: {narrow:?}");
        // The drawing is laid out as wide as it has room for, down to the narrowest phone.
        assert_eq!(Projection::fit(358.0, 18_000).width, 358.0);
        assert_eq!(Projection::fit(100.0, 18_000).width, MIN_WIDTH);
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
    fn fit_reaches_the_whole_stretch() {
        assert_eq!(Projection::fit(1050.0, 12_000).reach, Reach { near: 0.0, far: STRETCH_MM });
        assert_eq!(Reach::STRETCH.len(), STRETCH_MM);
        // The model's stretch is the one the 2.5D draws.
        assert_eq!(crate::street::model::SHOWN_MM as f64, STRETCH_MM);
        let p = Projection::fit(1050.0, 12_000);
        let part = Reach { near: 20_000.0, far: 40_000.0 };
        assert_eq!(p.over(part).reach, part);
        assert_eq!((p.over(part).scale, p.over(part).horizon), (p.scale, p.horizon));
        let e = band_edges(Reach::STRETCH);
        assert_eq!((e[0], e[BANDS]), (0.0, STRETCH_MM));
    }

    #[test]
    fn the_bands_of_a_reach_begin_and_end_on_it_in_equal_steps_of_one_over_depth() {
        let e = band_edges(Reach { near: 20_000.0, far: 40_000.0 });
        assert_eq!((e[0], e[BANDS]), (20_000.0, 40_000.0));
        let step = |i: usize| 1.0 / (EYE_BACK_MM + e[i]) - 1.0 / (EYE_BACK_MM + e[i + 1]);
        assert!((1..BANDS).all(|i| (step(i) - step(0)).abs() < 1e-12));
    }

    #[test]
    fn the_bands_cover_the_stretch_in_equal_steps_of_the_scale_so_the_far_ones_are_deeper() {
        let e = band_edges(Reach::STRETCH);
        assert_eq!((e[0], e[BANDS]), (0.0, STRETCH_MM));
        assert!(e.windows(2).all(|w| w[0] < w[1]));
        // Equal steps of the scale, 1 / depth: every band is as much smaller on the screen than the one before.
        let steps: Vec<f64> = e.windows(2).map(|w| 1.0 / (EYE_BACK_MM + w[0]) - 1.0 / (EYE_BACK_MM + w[1])).collect();
        assert!(steps.iter().all(|s| (s - steps[0]).abs() < 1e-12), "{steps:?}");
        // So a band is deeper than the one before it.
        assert!(e.windows(3).all(|w| w[1] - w[0] < w[2] - w[1]));
        // A band is laid where its scale is the mean of the two at its ends, which is inside it.
        for w in e.windows(2) {
            let mid = band_mid(w[0], w[1]);
            assert!(mid > w[0] && mid < w[1]);
            let (k0, k1, km) = (1.0 / (EYE_BACK_MM + w[0]), 1.0 / (EYE_BACK_MM + w[1]), 1.0 / (EYE_BACK_MM + mid));
            assert!((km - (k0 + k1) / 2.0).abs() < 1e-15);
        }
    }

    /// How far the texture of a strip `x0` to `x1` across, at `h`, jumps on the screen at each seam between two bands, in
    /// pixels: at the seam's three points (the strip's edges and middle), the two bands' matrices put a point of the
    /// pattern at different places on the ground, and the jump is that difference taken back to the screen.
    fn seam_steps(p: &Projection, x0: f64, x1: f64, h: f64) -> Vec<f64> {
        let e = band_edges(Reach::STRETCH);
        let local = |k: usize, x: f64, z: f64| {
            let g = p.ground_at(h, (x0 + x1) / 2.0, band_mid(e[k], e[k + 1]));
            let (sx, sy) = p.at(x, h, z);
            let v = (sy - g[5]) / g[3];
            ((sx - g[4] - g[2] * v) / g[0], v, g[0], g[3])
        };
        (0..BANDS - 1)
            .map(|k| {
                [x0, (x0 + x1) / 2.0, x1]
                    .iter()
                    .map(|&x| {
                        let (u1, v1, ..) = local(k, x, e[k + 1]);
                        let (u2, v2, a, d) = local(k + 1, x, e[k + 1]);
                        ((u1 - u2) * a).hypot((v1 - v2) * d)
                    })
                    .fold(0.0, f64::max)
            })
            .collect()
    }

    #[test]
    fn a_fine_texture_jumps_the_same_few_pixels_at_every_seam_where_a_step_is_not_seen() {
        // What is laid in bands now is only the fine grain (the regular joints are geometry, `joints.rs`: no step at all).
        // A 3.3 m sidewalk on Sample Street 1 (18 m across), at 684 px (the usual group at 1440 by 900), 1050 px and
        // 1440 px: the nearest seam is not the worst, as it was with bands of equal depth ratio (12.5 px at 1050 px with
        // six). No affine map fits a trapezoid, so the step is a few px, in speckle, mulch, grass and the broom grain.
        for (width, budget) in [(684.0, 5.1), (1050.0, 7.7), (1440.0, 8.6)] {
            let steps = seam_steps(&Projection::fit(width, 18_000), 0.0, 3_300.0, drawn_mm(150));
            let (lo, hi) = steps.iter().fold((f64::MAX, 0.0_f64), |(lo, hi), &s| (lo.min(s), hi.max(s)));
            assert!(hi <= budget, "{width}: {steps:?}");
            assert!(hi < 1.3 * lo, "{width}: every seam is about as big as the others: {steps:?}");
        }
        assert_eq!(BANDS, 8);
    }
}
