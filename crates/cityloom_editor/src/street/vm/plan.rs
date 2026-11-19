//! A press on the street's plan, and what it becomes: a click, a drag that picks a stretch, a drag of one end of the
//! pick, of a boundary between runs or of a thing along the street. Pure decisions over what the view measures (the
//! pointer's pixels for the drag threshold, its position along the street from the plan's left end); `StreetVm`'s
//! plan commands carry them out.

use crate::street::model::PlanThing;

/// A press becomes a drag only after it has moved this far.
const DRAG_START_PX: f64 = 4.0;

/// Whether a press has moved far enough to be a drag.
pub fn is_boundary_drag(dx_px: f64) -> bool {
    dx_px.abs() >= DRAG_START_PX
}

/// What a press on the plan landed on. Positions count from the plan's left end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanPress {
    /// The street itself (an empty part of a band), at `at_mm`.
    Street { at_mm: i32 },
    /// A handle of the pick: its left one, or its right one.
    Handle { left: bool },
    /// The marker of the boundary that starts zone `zone` (`PlanBoundary.zone`).
    Boundary { zone: usize },
    /// A thing along the street, pressed at `at_mm`.
    Thing { thing: PlanThing, at_mm: i32 },
}

/// A press under way. Positions count from the plan's left end, but a thing's are anchored, as its row keeps them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlanGesture {
    /// Pressed at `x_px` on the screen and not yet moved past the threshold: a click if it is let go now.
    Pending(PlanPress, f64),
    /// A drag picking from `anchor_mm`, where it started.
    Picking { anchor_mm: i32 },
    /// A drag of one end of the pick; the other end stays at `fixed_mm`.
    Handle { fixed_mm: i32 },
    /// A drag of the boundary that starts zone `zone`, from `at_mm`, begun with `pick` (`None` for the whole street),
    /// which follows the boundary where it ended there, and the piece `selected`, which stays selected.
    Boundary { zone: usize, at_mm: i32, pick: Option<(i32, i32)>, selected: Option<u32> },
    /// A drag of a thing that started at `from_mm`, pressed at `grab_mm` (both anchored).
    Thing { thing: PlanThing, from_mm: i32, grab_mm: i32 },
    /// A drag that was refused when it would have begun (another drag under way, or nothing may move): nothing until
    /// it is let go.
    Refused,
}

/// The press, before it has moved.
pub fn press(p: PlanPress, x_px: f64) -> PlanGesture {
    PlanGesture::Pending(p, x_px)
}

/// Whether the gesture is a drag with the pointer at `x_px`: a press that has moved past the threshold, or a drag
/// already begun.
pub fn moved(g: &PlanGesture, x_px: f64) -> bool {
    match g {
        PlanGesture::Pending(_, from_px) => is_boundary_drag(x_px - from_px),
        _ => true,
    }
}

/// The end of the pick a handle drag leaves where it is: the right end for the left handle, else the left end. `pick`
/// is the picked stretch, `None` for the whole street, `length_mm` the street's.
pub fn fixed_end(left: bool, pick: Option<(i32, i32)>, length_mm: i32) -> i32 {
    let (from, to) = pick.unwrap_or((0, length_mm));
    if left { to } else { from }
}

/// The pick after the boundary at `from_mm` has moved to `to_mm`: an end of `pick` that was on the boundary follows
/// it; the whole street stays the whole street.
pub fn pick_after_move(pick: Option<(i32, i32)>, from_mm: i32, to_mm: i32) -> Option<(i32, i32)> {
    pick.map(|(a, b)| (if a == from_mm { to_mm } else { a }, if b == from_mm { to_mm } else { b }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::Side;

    #[test]
    fn a_press_is_a_drag_only_after_a_few_pixels() {
        assert!(!is_boundary_drag(3.9) && !is_boundary_drag(-3.9));
        assert!(is_boundary_drag(4.0) && is_boundary_drag(-4.0) && is_boundary_drag(40.0));
        let g = press(PlanPress::Street { at_mm: 10_000 }, 100.0);
        assert_eq!(g, PlanGesture::Pending(PlanPress::Street { at_mm: 10_000 }, 100.0));
        assert!(!moved(&g, 103.9) && !moved(&g, 96.1));
        assert!(moved(&g, 104.0) && moved(&g, 96.0));
        assert!(moved(&PlanGesture::Picking { anchor_mm: 0 }, 100.0), "a drag begun stays one");
        let thing = press(PlanPress::Thing { thing: PlanThing::Curb { side: Side::Left, cell: 1 }, at_mm: 5_000 }, 10.0);
        assert!(!moved(&thing, 12.0));
    }

    #[test]
    fn a_handle_drag_keeps_the_other_end_and_a_boundary_drag_takes_the_pick_s_end_along() {
        assert_eq!(fixed_end(true, Some((10_000, 30_000)), 60_000), 30_000);
        assert_eq!(fixed_end(false, Some((10_000, 30_000)), 60_000), 10_000);
        assert_eq!(fixed_end(true, None, 60_000), 60_000, "the whole street's handles are at its ends");
        assert_eq!(fixed_end(false, None, 60_000), 0);
        assert_eq!(pick_after_move(Some((10_000, 30_000)), 10_000, 12_000), Some((12_000, 30_000)));
        assert_eq!(pick_after_move(Some((10_000, 30_000)), 30_000, 28_000), Some((10_000, 28_000)));
        assert_eq!(pick_after_move(Some((10_000, 30_000)), 40_000, 45_000), Some((10_000, 30_000)));
        assert_eq!(pick_after_move(None, 40_000, 45_000), None);
    }
}
