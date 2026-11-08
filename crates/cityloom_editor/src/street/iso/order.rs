//! The order the raised strips and the things standing on them are painted in.
//!
//! The eye is over the centre line, higher than everything drawn and behind everything. For two boxes with sides along
//! the axes (across, up, along the street) that do not overlap, some plane parallel to a side separates them, and a
//! box on the eye's side of such a plane cannot be hidden by the one beyond it. So **a box is painted after another
//! when a separating plane has it on the eye's side**: it is nearer the centre line (their across ranges do not
//! overlap and the eye is not past the other), or nearer the near end (their along ranges do not overlap), or
//! higher (their up ranges do not overlap). When planes disagree neither box hides the other, and the order between
//! them is free. Boxes that interpenetrate (a curb on its slab) have no such plane: strips keep the order they are
//! given in among themselves.

use super::projection::{EYE_BACK_MM, EYE_HEIGHT_MM};

/// A box standing in the scene, in millimetres of the drawing: across, up, along.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Bx {
    pub(super) x: (f64, f64),
    pub(super) y: (f64, f64),
    pub(super) z: (f64, f64),
}

impl Bx {
    /// Whether some plane that separates this box from `other` has this box on the eye's side, the eye being over
    /// `eye_x` across the street (`EYE_HEIGHT_MM` up and `EYE_BACK_MM` behind the near end).
    pub(super) fn in_front_of(&self, other: &Bx, eye_x: f64) -> bool {
        let across = if self.x.1 <= other.x.0 {
            eye_x <= other.x.0
        } else if other.x.1 <= self.x.0 {
            eye_x >= other.x.1
        } else {
            false
        };
        let along = self.z.1 <= other.z.0 && -EYE_BACK_MM <= other.z.0;
        let up = other.y.1 <= self.y.0 && EYE_HEIGHT_MM >= other.y.1;
        across || along || up
    }
}

/// The order to paint `boxes` in, as indices, farthest first. `boxes` are given in the order to prefer, and the
/// ones that are `strip`s keep that order among themselves; every other pair is ordered by `Bx::in_front_of`, the one
/// in front later, and a pair neither is in front of keeps its given order. A cycle (none is known for boxes seen from
/// over the centre line) is broken by taking the first one left.
pub(super) fn paint_order(boxes: &[(Bx, bool)], eye_x: f64) -> Vec<usize> {
    let n = boxes.len();
    // `after[j]` holds the boxes that must be painted before `j`.
    let mut before: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in i + 1..n {
            let ((a, a_strip), (b, b_strip)) = (boxes[i], boxes[j]);
            if a_strip && b_strip {
                before[j].push(i);
                continue;
            }
            match (a.in_front_of(&b, eye_x), b.in_front_of(&a, eye_x)) {
                (true, false) => before[i].push(j),
                (false, true) => before[j].push(i),
                _ => {}
            }
        }
    }
    let mut done = vec![false; n];
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let next = (0..n).find(|&k| !done[k] && before[k].iter().all(|&b| done[b])).or_else(|| (0..n).find(|&k| !done[k])).unwrap();
        done[next] = true;
        out.push(next);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(x: (f64, f64), y: (f64, f64), z: (f64, f64)) -> Bx {
        Bx { x, y, z }
    }

    #[test]
    fn a_box_nearer_the_centre_line_nearer_the_near_end_or_higher_is_in_front_of_the_one_it_is_apart_from() {
        let eye = 10_000.0;
        let (left, mid) = (bx((0.0, 1_000.0), (0.0, 500.0), (0.0, 1_000.0)), bx((5_000.0, 6_000.0), (0.0, 500.0), (0.0, 1_000.0)));
        assert!(mid.in_front_of(&left, eye) && !left.in_front_of(&mid, eye));
        // Right of the eye the other way round.
        let (a, b) = (bx((12_000.0, 13_000.0), (0.0, 500.0), (0.0, 1.0)), bx((20_000.0, 21_000.0), (0.0, 500.0), (0.0, 1.0)));
        assert!(a.in_front_of(&b, eye) && !b.in_front_of(&a, eye));
        // Nearer the near end, and higher.
        let (near, far) = (bx((0.0, 1.0), (0.0, 1.0), (0.0, 1_000.0)), bx((0.0, 1.0), (0.0, 1.0), (5_000.0, 6_000.0)));
        assert!(near.in_front_of(&far, eye) && !far.in_front_of(&near, eye));
        let (low, high) = (bx((0.0, 1.0), (0.0, 1_000.0), (0.0, 1.0)), bx((0.0, 1.0), (2_000.0, 3_000.0), (0.0, 1.0)));
        assert!(high.in_front_of(&low, eye) && !low.in_front_of(&high, eye));
        // Boxes that touch are apart.
        assert!(bx((0.0, 1.0), (500.0, 900.0), (0.0, 1.0)).in_front_of(&bx((0.0, 1.0), (0.0, 500.0), (0.0, 1.0)), eye));
    }

    #[test]
    fn two_boxes_with_the_eye_between_them_hide_neither_and_one_it_is_not_past_is_in_front() {
        let (a, b) = (bx((0.0, 1_000.0), (0.0, 500.0), (0.0, 1.0)), bx((9_000.0, 10_000.0), (0.0, 500.0), (0.0, 1.0)));
        assert!(a.in_front_of(&b, 500.0) && !b.in_front_of(&a, 500.0));
        assert!(a.in_front_of(&b, 5_000.0) && b.in_front_of(&a, 5_000.0));
    }

    #[test]
    fn a_box_beyond_a_nearer_box_is_painted_before_it_whatever_the_order_it_is_given_in() {
        // A low box beyond a tall one nearer the centre, listed first and last.
        let (beyond, tall) = (bx((0.0, 100.0), (0.0, 150.0), (0.0, 40_000.0)), bx((1_000.0, 2_000.0), (0.0, 450.0), (0.0, 40_000.0)));
        let eye = 9_000.0;
        assert_eq!(paint_order(&[(beyond, false), (tall, true)], eye), vec![0, 1]);
        assert_eq!(paint_order(&[(tall, true), (beyond, false)], eye), vec![1, 0]);
    }

    #[test]
    fn strips_keep_the_order_they_are_given_in_among_themselves_and_free_boxes_the_given_order() {
        let slab = bx((0.0, 3_000.0), (0.0, 450.0), (0.0, 40_000.0));
        let curb = bx((0.0, 200.0), (0.0, 600.0), (0.0, 40_000.0));
        assert_eq!(paint_order(&[(curb, true), (slab, true)], 9_000.0), vec![0, 1]);
        assert_eq!(paint_order(&[(slab, true), (curb, true)], 9_000.0), vec![0, 1]);
        let (a, b) = (bx((0.0, 1_000.0), (0.0, 500.0), (0.0, 1.0)), bx((9_000.0, 10_000.0), (0.0, 500.0), (0.0, 1.0)));
        assert_eq!(paint_order(&[(b, false), (a, false)], 5_000.0), vec![0, 1]);
        assert!(paint_order(&[], 0.0).is_empty());
    }
}
