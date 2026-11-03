//! How big the junction's drawing is: as wide as its group and as tall as the window leaves room for.

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
        let width = width.max(320.0);
        let narrow = width < 640.0;
        let height = if narrow { (width * 1.5).min(620.0) } else { (window_height - 555.0).clamp(340.0, 760.0) }.round();
        Frame { width, height, narrow }
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
        assert!(Frame::fit(100.0, 1000.0).width >= 320.0);
    }
}
