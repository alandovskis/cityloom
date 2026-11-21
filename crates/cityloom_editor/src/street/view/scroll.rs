//! How far a box that scrolls sideways must scroll to show a part of what it holds: a cell of a row, a tab of the Add
//! menu, the point of the plan.

/// How far the box must scroll to bring a cell wholly into view: negative to the left, positive to the right, zero
/// when it already shows. A cell wider than the box is lined up on its left edge.
pub fn scroll_into_view_px(cell_left: f64, cell_right: f64, box_left: f64, box_right: f64) -> f64 {
    if cell_left < box_left || cell_right - cell_left > box_right - box_left {
        cell_left - box_left
    } else if cell_right > box_right {
        cell_right - box_right
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_out_of_view_scrolls_the_box_just_enough_to_show_it() {
        assert_eq!(scroll_into_view_px(10.0, 40.0, 0.0, 100.0), 0.0, "already in view");
        assert_eq!(scroll_into_view_px(-30.0, 0.0, 0.0, 100.0), -30.0, "off the left");
        assert_eq!(scroll_into_view_px(90.0, 130.0, 0.0, 100.0), 30.0, "off the right");
        assert_eq!(scroll_into_view_px(-10.0, 20.0, 0.0, 100.0), -10.0, "part off the left");
        assert_eq!(scroll_into_view_px(-10.0, 200.0, 0.0, 100.0), -10.0, "wider than the box: its left edge");
    }
}
