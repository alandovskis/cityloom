//! How big the junction's drawing is: as wide as its group and as tall as the room it is left: on the junction page
//! the room its column leaves it (`in_room`), on the compare page the window's height less what is around it (`fit`).

/// The least wide the drawing is made: it scales to the group it has down to this, so that the page does not scroll
/// sideways in a window of 280 px (the street's 2.5D is held to the same).
pub const MIN_WIDTH: f64 = 240.0;

/// How much of a window's height `Frame::fit` leaves to what is around the drawing: the compare page's columns.
const AROUND: f64 = 555.0;

/// The least tall a drawing sized to its column is made (`Frame::in_room`): about a third of the street's tallest, the
/// least the street page scales its drawing down to (`street::view::iso::projection::MIN_FIT_SCALE`). Shorter than
/// this, the column scrolls.
pub const MIN_ROOM_HEIGHT: f64 = 160.0;

/// The tallest a wide drawing is made, whatever room it has.
pub const MAX_HEIGHT: f64 = 760.0;

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
        let height = if narrow { (width * 1.5).min(620.0) } else { (window_height - AROUND).clamp(340.0, MAX_HEIGHT) }.round();
        Frame { width, height, narrow }
    }

    /// The frame for a drawing `width` wide in a column that leaves it `room` pixels of height (the junction page,
    /// `view::drawing`): as tall as the room, between `MIN_ROOM_HEIGHT` and the tallest `fit` makes, so a short laptop
    /// with the notes' tray open draws the junction smaller and whole. A narrow drawing keeps `fit`'s height, and so
    /// does one whose column does not scroll (`room` is `INFINITY`): the window decides.
    pub fn in_room(width: f64, room: f64, window_height: f64) -> Frame {
        let fitted = Frame::fit(width, window_height);
        if fitted.narrow || !room.is_finite() {
            return fitted;
        }
        Frame { height: room.clamp(MIN_ROOM_HEIGHT, MAX_HEIGHT).round(), ..fitted }
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
    fn a_drawing_in_a_column_is_as_tall_as_its_room_within_the_bounds() {
        assert_eq!(Frame::in_room(1000.0, 500.0, 900.0).height, 500.0);
        // a short laptop with the notes' tray open: drawn smaller, whole, down to a floor
        assert_eq!(Frame::in_room(1000.0, 250.0, 800.0).height, 250.0);
        assert_eq!(Frame::in_room(1000.0, 50.0, 800.0).height, MIN_ROOM_HEIGHT);
        assert_eq!(Frame::in_room(1000.0, 2000.0, 900.0).height, 760.0);
        // a column that does not scroll gives no room: the window decides, as on the compare page
        assert_eq!(Frame::in_room(1000.0, f64::INFINITY, 1000.0), Frame::fit(1000.0, 1000.0));
        // a narrow drawing keeps its own height
        assert_eq!(Frame::in_room(400.0, 250.0, 800.0), Frame::fit(400.0, 800.0));
    }
}
