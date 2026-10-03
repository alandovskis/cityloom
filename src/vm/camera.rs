//! Where the map is looked at from, and how far in: a camera on the city drawn in
//! metres. It knows nothing of the page except the size of the window it fills.

use crate::units::Units;

/// How far in a person may zoom, as multiples of the zoom that fits the city.
const ZOOM_OUT: f64 = 0.8;
const ZOOM_IN: f64 = 10.0;
/// Room round the city, in metres, and extra at the left to keep the scale bar clear.
const MARGIN_M: f64 = 70.0;
const LEFT_EXTRA_M: f64 = 190.0;

/// The box the camera looks over, in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct World {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl World {
    /// The box round a city whose bounds are `[x0, y0, x1, y1]` in millimetres.
    pub fn round(bounds_mm: [i32; 4]) -> World {
        let [x0, y0, x1, y1] = bounds_mm.map(|v| v as f64 / 1000.0);
        World { x0: x0 - MARGIN_M - LEFT_EXTRA_M, y0: y0 - MARGIN_M, x1: x1 + MARGIN_M, y1: y1 + MARGIN_M }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Pixels to the metre.
    pub k: f64,
    /// The point at the middle of the window, in metres.
    pub cx: f64,
    pub cy: f64,
    /// The zoom that fits the whole city in the window.
    pub fit_k: f64,
    pub width: f64,
    pub height: f64,
    /// The person has zoomed or panned, so a resize keeps their view.
    pub moved: bool,
    pub world: World,
}

impl Camera {
    /// A camera on the whole city, in a window of this size.
    pub fn new(world: World, width: f64, height: f64) -> Camera {
        let mut c = Camera { k: 1.0, cx: 0.0, cy: 0.0, fit_k: 1.0, width, height, moved: false, world };
        c.fit();
        c
    }

    /// Shows the whole city.
    pub fn fit(&mut self) {
        let w = self.world;
        self.fit_k = (self.width / (w.x1 - w.x0)).min(self.height / (w.y1 - w.y0));
        self.k = self.fit_k;
        self.cx = (w.x0 + w.x1) / 2.0;
        self.cy = (w.y0 + w.y1) / 2.0;
        self.moved = false;
    }

    /// Keeps the camera over the city and within the zoom a person is given.
    pub fn clamp(&mut self) {
        self.k = self.k.clamp(self.fit_k * ZOOM_OUT, self.fit_k * ZOOM_IN);
        self.cx = self.cx.clamp(self.world.x0, self.world.x1);
        self.cy = self.cy.clamp(self.world.y0, self.world.y1);
    }

    /// The window changed size. A camera the person has moved keeps its view, at
    /// the same zoom relative to the whole city.
    pub fn resize(&mut self, width: f64, height: f64) {
        let keep = self.moved.then_some((self.k / self.fit_k, self.cx, self.cy));
        self.width = width;
        self.height = height;
        self.fit();
        if let Some((rel, cx, cy)) = keep {
            self.k = rel * self.fit_k;
            self.cx = cx;
            self.cy = cy;
            self.moved = true;
        }
        self.clamp();
    }

    /// Zooms to `k`, keeping the point under (`sx`, `sy`) pixels from the window's
    /// middle where it is.
    pub fn zoom_at(&mut self, k: f64, sx: f64, sy: f64) {
        let (wx, wy) = (self.cx + sx / self.k, self.cy + sy / self.k);
        self.k = k.clamp(self.fit_k * ZOOM_OUT, self.fit_k * ZOOM_IN);
        self.cx = wx - sx / self.k;
        self.cy = wy - sy / self.k;
        self.moved = true;
        self.clamp();
    }

    /// Zooms by a factor about the middle of the window.
    pub fn zoom_by(&mut self, factor: f64) {
        self.zoom_at(self.k * factor, 0.0, 0.0);
    }

    /// Moves the view by pixels.
    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        self.cx += dx / self.k;
        self.cy += dy / self.k;
        self.moved = true;
        self.clamp();
    }

    /// The `viewBox` of the drawing: what the window shows, in metres.
    pub fn view_box(&self) -> String {
        let (w, h) = (self.width / self.k, self.height / self.k);
        let r2 = |n: f64| (n * 100.0).round() / 100.0;
        format!("{} {} {} {}", r2(self.cx - w / 2.0), r2(self.cy - h / 2.0), r2(w), r2(h))
    }

    /// A scale bar that fits in 170 px, of a round length in the units shown.
    pub fn scale_bar(&self, units: Units) -> ScaleBar {
        let (per_unit, options): (f64, &[i32]) = match units {
            Units::Metres => (self.k, &[10, 20, 50, 100, 200, 500, 1000]),
            Units::Feet => (self.k * 0.3048, &[50, 100, 200, 500, 1000, 2000, 5000]),
        };
        let length = options.iter().copied().filter(|n| *n as f64 * per_unit <= 170.0).last().unwrap_or(options[0]);
        ScaleBar { length, width_px: length as f64 * per_unit, unit: units.word() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScaleBar {
    pub length: i32,
    pub width_px: f64,
    pub unit: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn city() -> World {
        World::round([0, 0, 1_000_000, 500_000])
    }

    #[test]
    fn the_world_is_the_city_with_a_margin_and_extra_room_at_the_left() {
        let w = city();
        assert_eq!((w.x0, w.y0, w.x1, w.y1), (-260.0, -70.0, 1070.0, 570.0));
    }

    #[test]
    fn a_new_camera_fits_the_whole_city_in_the_window_and_centres_on_it() {
        let c = Camera::new(city(), 800.0, 520.0);
        assert!((c.k - (800.0_f64 / 1330.0).min(520.0 / 640.0)).abs() < 1e-12);
        assert_eq!((c.cx, c.cy), (405.0, 250.0));
        assert!(!c.moved);
        assert_eq!(c.k, c.fit_k);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer_where_it_is_and_stays_within_the_zoom_allowed() {
        let mut c = Camera::new(city(), 800.0, 520.0);
        let (sx, sy) = (100.0, -50.0);
        let under = (c.cx + sx / c.k, c.cy + sy / c.k);
        c.zoom_at(c.k * 2.0, sx, sy);
        assert!(c.moved);
        let after = (c.cx + sx / c.k, c.cy + sy / c.k);
        assert!((under.0 - after.0).abs() < 1e-9 && (under.1 - after.1).abs() < 1e-9);
        c.zoom_at(1e9, 0.0, 0.0);
        assert!((c.k - c.fit_k * 10.0).abs() < 1e-9);
        c.zoom_at(1e-9, 0.0, 0.0);
        assert!((c.k - c.fit_k * 0.8).abs() < 1e-12);
    }

    #[test]
    fn panning_moves_the_view_in_metres_and_cannot_leave_the_city() {
        let mut c = Camera::new(city(), 800.0, 520.0);
        let before = (c.cx, c.cy);
        c.pan_by(40.0, -20.0);
        assert!((c.cx - (before.0 + 40.0 / c.k)).abs() < 1e-9 && (c.cy - (before.1 - 20.0 / c.k)).abs() < 1e-9);
        c.pan_by(1e9, -1e9);
        assert_eq!((c.cx, c.cy), (c.world.x1, c.world.y0));
    }

    #[test]
    fn fitting_again_puts_the_whole_city_back_and_forgets_the_person_moved_it() {
        let mut c = Camera::new(city(), 800.0, 520.0);
        c.zoom_by(3.0);
        c.pan_by(30.0, 30.0);
        c.fit();
        assert_eq!(c, Camera::new(city(), 800.0, 520.0));
    }

    #[test]
    fn a_window_that_changes_size_refits_a_camera_nobody_moved_and_keeps_the_view_of_one_that_was() {
        let mut c = Camera::new(city(), 800.0, 520.0);
        c.resize(1200.0, 600.0);
        assert_eq!((c.width, c.height), (1200.0, 600.0));
        assert_eq!(c.k, c.fit_k);
        let mut m = Camera::new(city(), 800.0, 520.0);
        m.zoom_by(2.0);
        m.pan_by(50.0, 0.0);
        let (rel, cx, cy) = (m.k / m.fit_k, m.cx, m.cy);
        m.resize(1200.0, 600.0);
        assert!(m.moved);
        assert!((m.k / m.fit_k - rel).abs() < 1e-9);
        assert_eq!((m.cx, m.cy), (cx, cy));
    }

    #[test]
    fn the_view_box_is_what_the_window_shows_in_metres() {
        let c = Camera::new(city(), 800.0, 520.0);
        let vb: Vec<f64> = c.view_box().split(' ').map(|n| n.parse().unwrap()).collect();
        assert!((vb[2] - 800.0 / c.k).abs() < 0.01 && (vb[3] - 520.0 / c.k).abs() < 0.01);
        assert!((vb[0] + vb[2] / 2.0 - c.cx).abs() < 0.01);
    }

    #[test]
    fn the_scale_bar_is_the_longest_round_length_that_fits_in_the_window_s_corner() {
        let mut c = Camera::new(city(), 800.0, 520.0);
        c.k = 1.0;
        assert_eq!(c.scale_bar(Units::Metres), ScaleBar { length: 100, width_px: 100.0, unit: "m" });
        c.k = 10.0;
        assert_eq!(c.scale_bar(Units::Metres).length, 10);
        c.k = 0.01;
        assert_eq!(c.scale_bar(Units::Metres).length, 1000);
        c.k = 1.0;
        let ft = c.scale_bar(Units::Feet);
        assert_eq!((ft.length, ft.unit), (500, "ft"));
        assert!((ft.width_px - 500.0 * 0.3048).abs() < 1e-9);
    }

    #[test]
    fn a_scale_that_fits_nothing_still_shows_the_shortest_bar() {
        let mut c = Camera::new(city(), 800.0, 520.0);
        c.k = 100.0;
        assert_eq!(c.scale_bar(Units::Metres).length, 10);
    }
}
