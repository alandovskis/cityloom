//! Which places can be edited: the ones the map's open view holds. A place is editable when its box lies within
//! the view; a place too big for the view in either direction cannot ever lie within it, so it is editable while
//! it crosses the view. A junction's box is its point.

/// West, south, east, north, in degrees.
pub type Rect = [f64; 4];

fn within(b: Rect, view: Rect) -> bool {
    b[0] >= view[0] && b[2] <= view[2] && b[1] >= view[1] && b[3] <= view[3]
}

fn crosses(b: Rect, view: Rect) -> bool {
    b[0] <= view[2] && b[2] >= view[0] && b[1] <= view[3] && b[3] >= view[1]
}

/// The `hot` ids of the places `view` makes editable, in the order of `boxes`.
pub fn editable(boxes: &[(String, Rect)], view: Rect) -> Vec<String> {
    let (width, height) = (view[2] - view[0], view[3] - view[1]);
    boxes
        .iter()
        .filter(|(_, b)| {
            let too_big = b[2] - b[0] > width || b[3] - b[1] > height;
            if too_big { crosses(*b, view) } else { within(*b, view) }
        })
        .map(|(hot, _)| hot.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Rect = [0.0, 0.0, 10.0, 10.0];

    fn editable_in(view: Rect, boxes: &[(&str, Rect)]) -> Vec<String> {
        let boxes: Vec<(String, Rect)> = boxes.iter().map(|(h, b)| (h.to_string(), *b)).collect();
        editable(&boxes, view)
    }

    #[test]
    fn a_place_wholly_in_view_is_editable_and_one_clipped_by_the_edge_is_not() {
        let got = editable_in(VIEW, &[("s-1", [1.0, 1.0, 4.0, 4.0]), ("s-2", [8.0, 1.0, 12.0, 4.0]), ("s-3", [1.0, -1.0, 4.0, 4.0])]);
        assert_eq!(got, ["s-1"]);
    }

    #[test]
    fn a_street_too_big_for_the_view_is_editable_while_it_crosses_it_and_not_when_it_misses_it() {
        let got = editable_in(
            VIEW,
            &[
                ("wide", [-5.0, 4.0, 15.0, 5.0]),
                ("tall", [4.0, -5.0, 5.0, 15.0]),
                ("wide-away", [-5.0, 20.0, 15.0, 25.0]),
                ("tall-away", [20.0, -5.0, 21.0, 15.0]),
            ],
        );
        assert_eq!(got, ["wide", "tall"]);
    }

    #[test]
    fn a_junction_is_editable_when_its_point_is_in_view_the_edge_included() {
        let at = |x: f64, y: f64| [x, y, x, y];
        let got = editable_in(VIEW, &[("j-1", at(0.0, 0.0)), ("j-2", at(10.0, 10.0)), ("j-3", at(5.0, 5.0)), ("j-4", at(10.1, 5.0))]);
        assert_eq!(got, ["j-1", "j-2", "j-3"]);
    }

    #[test]
    fn the_order_of_the_places_is_kept_and_no_places_make_none() {
        assert_eq!(editable_in(VIEW, &[("b", [1.0, 1.0, 2.0, 2.0]), ("a", [3.0, 3.0, 4.0, 4.0])]), ["b", "a"]);
        assert!(editable_in(VIEW, &[]).is_empty());
    }

    #[test]
    fn a_view_with_no_area_holds_only_a_point_on_it() {
        let got = editable_in([5.0, 5.0, 5.0, 5.0], &[("s-1", [1.0, 1.0, 2.0, 2.0]), ("j-1", [5.0, 5.0, 5.0, 5.0])]);
        assert_eq!(got, ["j-1"]);
    }
}
