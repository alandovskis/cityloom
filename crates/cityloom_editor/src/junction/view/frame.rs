//! How big the junction's drawing is: as wide as its group and as tall as the room it is left: on the junction page
//! the room its column leaves it (`window_leaving`), on the compare page the window's height less what is around it.

/// The least wide the drawing is made: it scales to the group it has down to this, so that the page does not scroll
/// sideways in a window of 280 px (the street's 2.5D is held to the same).
pub const MIN_WIDTH: f64 = 240.0;

/// How much of a window's height `Frame::fit` leaves to what is around the drawing: the compare page's columns.
pub const AROUND: f64 = 555.0;

/// The size of the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub width: f64,
    pub height: f64,
    /// Too little room beside the junction for a street's name to sit beside it.
    pub narrow: bool,
}

impl Frame {
    /// The frame for a drawing `width` wide, in a window `window_height` high.
    pub fn fit(width: f64, window_height: f64) -> Frame {
        let width = width.max(MIN_WIDTH);
        let narrow = width < 640.0;
        let height = if narrow { (width * 1.5).min(620.0) } else { (window_height - AROUND).clamp(340.0, 760.0) }.round();
        Frame { width, height, narrow }
    }

    /// The window height for which `fit` makes the drawing `room` high (within its bounds): the junction page measures
    /// the room its column leaves the drawing and gives this. No limit (`INFINITY`, a column that does not scroll)
    /// stays none, and the drawing is as tall as it may be.
    pub fn window_leaving(room: f64) -> f64 {
        room + AROUND
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_drawing_is_as_tall_as_the_window_leaves_room_for() {
        let f = Frame::fit(1000.0, 1000.0);
        assert!(!f.narrow);
        assert_eq!((f.width, f.height), (1000.0, 445.0));
        assert_eq!(Frame::fit(1000.0, 400.0).height, 340.0);
        assert_eq!(Frame::fit(1000.0, 2000.0).height, 760.0);
    }

    #[test]
    fn a_narrow_drawing_is_taller_than_it_is_wide_and_has_room_for_no_labels_beside() {
        let f = Frame::fit(400.0, 1000.0);
        assert!(f.narrow);
        assert_eq!((f.width, f.height), (400.0, 600.0));
        assert!(Frame::fit(100.0, 1000.0).width >= MIN_WIDTH);
    }

    #[test]
    fn the_window_leaving_a_room_gives_a_drawing_that_high_within_the_bounds() {
        assert_eq!(Frame::fit(1000.0, Frame::window_leaving(500.0)).height, 500.0);
        assert_eq!(Frame::fit(1000.0, Frame::window_leaving(200.0)).height, 340.0);
        assert_eq!(Frame::fit(1000.0, Frame::window_leaving(f64::INFINITY)).height, 760.0);
        // the compare page's drawing, from the window itself, is as before
        assert_eq!(Frame::fit(1000.0, 1000.0).height, 445.0);
    }
}
