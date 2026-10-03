//! How the junction's plan sits in the drawing: millimetres on the plan to pixels,
//! and back for a pointer.

type P = (f64, f64);

/// How a plan on the junction's own millimetres sits in the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub scale: f64,
    pub ox: f64,
    pub oy: f64,
    pub width: f64,
    pub height: f64,
    /// Too little room beside the plan for a street's name to sit beside it.
    pub narrow: bool,
}

impl Frame {
    /// The frame for a drawing `width` wide, in a window `window_height` high.
    pub fn fit(bounds: [f64; 4], width: f64, window_height: f64) -> Frame {
        let width = width.max(320.0);
        let narrow = width < 640.0;
        let height = if narrow { (width * 1.5).min(620.0) } else { (window_height - 240.0).clamp(520.0, 820.0) }.round();
        let [bx0, by0, bx1, by1] = bounds;
        let pad_x = if narrow { 24.0 } else { (width * 0.2).min(150.0) };
        let pad_y = 64.0;
        let scale = ((width - 2.0 * pad_x) / (bx1 - bx0)).min((height - 2.0 * pad_y) / (by1 - by0));
        Frame { scale, ox: width / 2.0 - (bx0 + bx1) / 2.0 * scale, oy: height / 2.0 - (by0 + by1) / 2.0 * scale, width, height, narrow }
    }

    /// A point on the plan, in pixels.
    pub fn at(&self, p: P) -> P {
        (self.ox + p.0 * self.scale, self.oy + p.1 * self.scale)
    }

    /// A pixel position in the drawing, as a point on the plan.
    pub fn plan_point(&self, px: P) -> P {
        ((px.0 - self.ox) / self.scale, (px.1 - self.oy) / self.scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_fits_the_plan_in_the_drawing_and_maps_points_both_ways() {
        let f = Frame::fit([-20_000.0, -20_000.0, 20_000.0, 20_000.0], 1000.0, 1000.0);
        assert!(!f.narrow);
        assert_eq!((f.width, f.height), (1000.0, 760.0));
        // Centred, and nothing is outside the drawing.
        let (cx, cy) = f.at((0.0, 0.0));
        assert!((cx - 500.0).abs() < 1e-9 && (cy - 380.0).abs() < 1e-9);
        let (x0, y0) = f.at((-20_000.0, -20_000.0));
        let (x1, y1) = f.at((20_000.0, 20_000.0));
        assert!(x0 >= 0.0 && y0 >= 0.0 && x1 <= f.width && y1 <= f.height);
        let back = f.plan_point(f.at((1234.0, -5678.0)));
        assert!((back.0 - 1234.0).abs() < 1e-6 && (back.1 + 5678.0).abs() < 1e-6);
    }

    #[test]
    fn a_narrow_drawing_is_taller_than_it_is_wide_and_has_room_for_no_labels_beside() {
        let f = Frame::fit([-20_000.0, -20_000.0, 20_000.0, 20_000.0], 400.0, 1000.0);
        assert!(f.narrow);
        assert_eq!((f.width, f.height), (400.0, 600.0));
        assert!(Frame::fit([-1.0, -1.0, 1.0, 1.0], 100.0, 1000.0).width >= 320.0);
    }
}
