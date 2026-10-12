//! Drawing a restriction's outline: idle, then placing corners one at a time, then closed and waiting for its rule
//! and name. Pure: the map's view-model feeds it what the pointer and the keys do, and says what came of it.

use crate::restriction::model::{LonLat, same_spot, valid};
use crate::shared::said::Said;

/// Where the drawing of an outline is.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Draw {
    #[default]
    Idle,
    /// Corners placed so far.
    Drawing(Vec<LonLat>),
    /// The outline closed: waiting for its rule and name.
    Naming(Vec<LonLat>),
}

/// What the person did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Start,
    /// A corner at this spot: a press on the map, or Enter at its centre.
    Add(LonLat),
    /// A press on a corner already placed, by its index: the first closes the outline.
    PressVertex(usize),
    /// Enter with Shift, or the Finish button.
    Close,
    /// Backspace: the last corner goes.
    Back,
    /// Escape, or Cancel.
    Cancel,
}

/// What came of a step, for the view-model to say.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Ignored,
    Started,
    /// A corner placed: how many there are now.
    Added(usize),
    /// The last corner removed: how many are left.
    Removed(usize),
    Closed,
    Cancelled,
    /// The outline cannot be closed, for this reason; drawing goes on.
    Refused(Said),
}

impl Draw {
    /// The corners placed, or of the closed outline; none when idle.
    pub fn corners(&self) -> &[LonLat] {
        match self {
            Draw::Idle => &[],
            Draw::Drawing(c) | Draw::Naming(c) => c,
        }
    }

    pub fn step(&mut self, step: Step) -> Outcome {
        let (next, outcome) = match (std::mem::take(self), step) {
            (Draw::Idle, Step::Start) => (Draw::Drawing(Vec::new()), Outcome::Started),
            (Draw::Drawing(mut c), Step::Add(p)) => {
                // The last corner again is a double-click, and the first again is a closing repeat: neither is a
                // corner, and `valid` must never see either (it would say the outline crosses itself).
                if c.last().is_some_and(|&l| same_spot(l, p)) || c.first().is_some_and(|&f| same_spot(f, p)) {
                    (Draw::Drawing(c), Outcome::Ignored)
                } else {
                    c.push(p);
                    let n = c.len();
                    (Draw::Drawing(c), Outcome::Added(n))
                }
            }
            (Draw::Drawing(c), Step::PressVertex(0) | Step::Close) => match valid(&c) {
                Ok(()) => (Draw::Naming(c), Outcome::Closed),
                Err(why) => (Draw::Drawing(c), Outcome::Refused(why)),
            },
            (Draw::Drawing(mut c), Step::Back) => match c.pop() {
                Some(_) => {
                    let n = c.len();
                    (Draw::Drawing(c), Outcome::Removed(n))
                }
                None => (Draw::Drawing(c), Outcome::Ignored),
            },
            (Draw::Drawing(_) | Draw::Naming(_), Step::Cancel) => (Draw::Idle, Outcome::Cancelled),
            (state, _) => (state, Outcome::Ignored),
        };
        *self = next;
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: LonLat = [-73.60, 45.52];
    const B: LonLat = [-73.59, 45.52];
    const C: LonLat = [-73.59, 45.53];
    const D: LonLat = [-73.60, 45.53];

    fn drawing(corners: &[LonLat]) -> Draw {
        let mut d = Draw::Idle;
        d.step(Step::Start);
        for &p in corners {
            d.step(Step::Add(p));
        }
        d
    }

    #[test]
    fn drawing_starts_from_idle_only() {
        let mut d = Draw::Idle;
        assert_eq!(d.step(Step::Start), Outcome::Started);
        assert_eq!(d, Draw::Drawing(vec![]));
        assert_eq!(d.step(Step::Start), Outcome::Ignored, "already drawing");
        for step in [Step::Add(A), Step::PressVertex(0), Step::Close, Step::Back, Step::Cancel] {
            let mut idle = Draw::Idle;
            assert_eq!(idle.step(step), Outcome::Ignored, "{step:?}");
            assert_eq!(idle, Draw::Idle);
        }
    }

    #[test]
    fn each_press_places_a_corner_and_the_same_spot_twice_is_one_corner() {
        let mut d = drawing(&[]);
        assert_eq!(d.step(Step::Add(A)), Outcome::Added(1));
        assert_eq!(d.step(Step::Add(B)), Outcome::Added(2));
        assert_eq!(d.step(Step::Add([B[0] + 1e-8, B[1]])), Outcome::Ignored, "the same micro-degree");
        assert_eq!(d.corners(), &[A, B]);
    }

    #[test]
    fn pressing_the_first_corner_or_closing_ends_an_outline_of_three_or_more() {
        let mut d = drawing(&[A, B, C]);
        assert_eq!(d.step(Step::PressVertex(2)), Outcome::Ignored, "only the first corner closes");
        assert_eq!(d.step(Step::PressVertex(0)), Outcome::Closed);
        assert_eq!(d, Draw::Naming(vec![A, B, C]));
        let mut d = drawing(&[A, B, C, D]);
        assert_eq!(d.step(Step::Close), Outcome::Closed);
        assert_eq!(d.corners(), &[A, B, C, D]);
    }

    #[test]
    fn closing_too_soon_or_across_itself_is_refused_with_the_reason_and_drawing_goes_on() {
        let mut d = drawing(&[A, B]);
        assert_eq!(d.step(Step::Close), Outcome::Refused(Said::new("restriction-refused-too-few")));
        assert_eq!(d, Draw::Drawing(vec![A, B]));
        let mut bow = drawing(&[A, C, B, D]);
        assert_eq!(bow.step(Step::PressVertex(0)), Outcome::Refused(Said::new("restriction-refused-crossing")));
        assert_eq!(bow.corners().len(), 4);
    }

    #[test]
    fn backspace_removes_the_last_corner_and_does_nothing_with_none() {
        let mut d = drawing(&[A, B]);
        assert_eq!(d.step(Step::Back), Outcome::Removed(1));
        assert_eq!(d.step(Step::Back), Outcome::Removed(0));
        assert_eq!(d.step(Step::Back), Outcome::Ignored);
        assert_eq!(d, Draw::Drawing(vec![]));
    }

    #[test]
    fn escape_cancels_drawing_and_naming() {
        let mut d = drawing(&[A, B]);
        assert_eq!(d.step(Step::Cancel), Outcome::Cancelled);
        assert_eq!(d, Draw::Idle);
        let mut n = drawing(&[A, B, C]);
        n.step(Step::Close);
        assert_eq!(n.step(Step::Cancel), Outcome::Cancelled);
        assert_eq!(n, Draw::Idle);
    }

    #[test]
    fn a_closed_outline_takes_no_more_corners() {
        let mut n = drawing(&[A, B, C]);
        n.step(Step::Close);
        for step in [Step::Add(D), Step::PressVertex(0), Step::Close, Step::Back, Step::Start] {
            assert_eq!(n.step(step), Outcome::Ignored, "{step:?}");
        }
        assert_eq!(n, Draw::Naming(vec![A, B, C]));
    }

    #[test]
    fn a_double_click_is_one_corner_and_closes_cleanly() {
        let mut d = drawing(&[A, B]);
        assert_eq!(d.step(Step::Add(C)), Outcome::Added(3));
        assert_eq!(d.step(Step::Add(C)), Outcome::Ignored, "a double-click");
        assert_eq!(d.corners(), &[A, B, C]);
        assert_eq!(d.step(Step::Close), Outcome::Closed, "not 'crosses itself'");
    }

    #[test]
    fn placing_the_first_corner_again_is_not_a_corner_and_the_ring_is_never_closed_by_repeating_it() {
        let mut d = drawing(&[A, B, C]);
        assert_eq!(d.step(Step::Add(A)), Outcome::Ignored);
        assert_eq!(d.corners(), &[A, B, C]);
        assert_eq!(d.step(Step::Close), Outcome::Closed);
        assert_eq!(d, Draw::Naming(vec![A, B, C]));
    }
}
