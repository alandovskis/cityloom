//! What a press on the plan lands on, from where it is on the drawing: the plan's hit layer. Every target is at least
//! a given size each way, however small the mark drawn for it: `TARGET_PX` (WCAG 2.5.8), `COARSE_TARGET_PX` on a coarse
//! pointer. Where targets overlap the one drawn on top wins: a handle of the pick, then the things along the street (the
//! last drawn first), then a boundary between runs that may move; anywhere else is the street itself, or, beside the
//! bands, the empty lane of that side.

use super::{Along, BANDS_H, HANDLE_OVER, LANE_H, across_of, stack};
use crate::shared::catalogue::Side;
use crate::street::model::PlanView;
use crate::street::vm::plan::PlanPress;

/// The smallest a target is, each way, in pixels.
pub const TARGET_PX: f64 = 24.0;
/// The same on a coarse pointer (a finger).
pub const COARSE_TARGET_PX: f64 = 44.0;

/// A press on the plan: what it landed on, and how far the pointer was from the line it took (a handle or a boundary),
/// so that the line follows the pointer from where it was taken instead of jumping under it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub press: PlanPress,
    pub grab_px: f64,
}

/// What a press at `(x, y)` px from the top left of the plan drawn `width_px` wide (`plan_svg`) lands on, every target
/// at least `target_px` each way.
pub fn hit(p: &PlanView, width_px: f64, x: f64, y: f64, target_px: f64) -> Hit {
    let at = Along::of(p, width_px);
    let on = |press| Hit { press, grab_px: 0.0 };
    let bands = (LANE_H, LANE_H + BANDS_H);
    if let Some((from, to)) = p.pick.and_then(|(a, b)| at.cut_span(a, b)) {
        let reach = (bands.0 - HANDLE_OVER, bands.1 + HANDLE_OVER);
        let nearest = [(true, from), (false, to)]
            .into_iter()
            .filter(|&(_, hx)| within(x, y, (hx - target_px / 2.0, hx + target_px / 2.0), reach))
            .min_by(|a, b| (x - a.1).abs().total_cmp(&(x - b.1).abs()));
        if let Some((left, hx)) = nearest {
            return Hit { press: PlanPress::Handle { left }, grab_px: x - hx };
        }
    }
    let across = BANDS_H / p.runs.iter().map(super::run_total).max().unwrap_or(0).max(1) as f64;
    let stacks: Vec<Vec<(f64, f64)>> = p.runs.iter().map(|r| stack(r, across)).collect();
    for o in p.objects.iter().rev() {
        let Some(span) = at.cut_span(o.from_mm, o.to_mm) else { continue };
        let (top, h) = across_of(o, p, &stacks);
        if within(x, y, widened(span, target_px), widened((top, top + h), target_px)) {
            return on(PlanPress::Thing { thing: o.thing, at_mm: at.mm(x) });
        }
    }
    let nearest = p
        .boundaries
        .iter()
        .filter(|b| b.movable)
        .map(|b| (b.zone, at.x(b.at_mm)))
        .filter(|&(_, bx)| at.length_mm.is_some() && within(x, y, (bx - target_px / 2.0, bx + target_px / 2.0), bands))
        .min_by(|a, b| (x - a.1).abs().total_cmp(&(x - b.1).abs()));
    if let Some((zone, bx)) = nearest {
        return Hit { press: PlanPress::Boundary { zone }, grab_px: x - bx };
    }
    let at_mm = at.mm(x);
    on(if y < bands.0 {
        PlanPress::Lane { side: Side::Left, at_mm }
    } else if y > bands.1 {
        PlanPress::Lane { side: Side::Right, at_mm }
    } else {
        PlanPress::Street { at_mm }
    })
}

/// Where `x_px` from the plan's left edge lies along the street, from its left end, kept on the street (0 on a plan with
/// no length).
pub fn plan_mm(p: &PlanView, width_px: f64, x_px: f64) -> i32 {
    Along::of(p, width_px).mm(x_px)
}

/// The span `(from, to)`, widened about its middle to `target_px` when it is narrower.
fn widened((from, to): (f64, f64), target_px: f64) -> (f64, f64) {
    let short = target_px - (to - from);
    if short > 0.0 { (from - short / 2.0, to + short / 2.0) } else { (from, to) }
}

fn within(x: f64, y: f64, (left, right): (f64, f64), (top, bottom): (f64, f64)) -> bool {
    (left..=right).contains(&x) && (top..=bottom).contains(&y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{MATERIALS, kind_index};
    use crate::shared::said::Said;
    use crate::street::model::{PlanBand, PlanBoundary, PlanObject, PlanRun, PlanThing, ZoneEnd};

    const LEN: i32 = 60_000;
    /// Drawn 600 px wide, a 60 m street is 100 mm to the pixel.
    const WIDTH: f64 = 600.0;
    /// The middle of the bands, across.
    const MID: f64 = LANE_H + BANDS_H / 2.0;

    fn band(uid: u32, kind: &str, width_mm: i32) -> PlanBand {
        let material = MATERIALS.iter().position(|m| m.id == "asphalt").expect("a material");
        PlanBand { uid, kind: kind_index(kind).expect("a kind"), material, width_mm }
    }

    fn run(from_mm: i32, to_mm: i32, zone: usize) -> PlanRun {
        PlanRun { from_mm, to_mm, zone, bands: vec![band(1, "sidewalk", 2_000), band(2, "travel", 3_300), band(3, "sidewalk", 2_000)], ok: true }
    }

    fn thing(thing: PlanThing, from_mm: i32, to_mm: i32, side: Option<Side>) -> PlanObject {
        PlanObject { thing, from_mm, to_mm, side, label: Said::new("plan-cut"), chosen: false, host: None }
    }

    /// Two runs meeting at 20 m.
    fn plan() -> PlanView {
        PlanView {
            length_mm: Some(LEN),
            runs: vec![run(0, 20_000, 0), run(20_000, LEN, 1)],
            boundaries: vec![PlanBoundary { at_mm: 20_000, zone: 1, taper: None, movable: true }],
            objects: Vec::new(),
            pick: None,
            cut: [false, false],
            elsewhere: false,
            from_end: Some(ZoneEnd::First),
        }
    }

    fn press(p: &PlanView, x: f64, y: f64) -> PlanPress {
        hit(p, WIDTH, x, y, TARGET_PX).press
    }

    fn press_at(p: &PlanView, x: f64, y: f64, target: f64) -> PlanPress {
        hit(p, WIDTH, x, y, target).press
    }

    #[test]
    fn an_empty_part_of_the_street_is_the_street_at_that_place() {
        assert_eq!(press(&plan(), 450.0, MID), PlanPress::Street { at_mm: 45_000 });
        assert_eq!(press(&plan(), 450.0, 2.0), PlanPress::Lane { side: Side::Left, at_mm: 45_000 }, "the empty lane of the side above");
        assert_eq!(press(&plan(), 450.0, LANE_H + BANDS_H + 2.0), PlanPress::Lane { side: Side::Right, at_mm: 45_000 }, "and below");
        assert_eq!(plan_mm(&plan(), WIDTH, -30.0), 0, "kept on the street");
        assert_eq!(plan_mm(&plan(), WIDTH, 900.0), LEN);
        let schematic = PlanView { length_mm: None, boundaries: Vec::new(), ..plan() };
        assert_eq!(press(&schematic, 300.0, MID), PlanPress::Street { at_mm: 0 }, "a plan with no length has no positions");
        assert_eq!(press(&schematic, 200.0, MID), PlanPress::Street { at_mm: 0 }, "nor boundaries to take");
    }

    #[test]
    fn a_boundary_is_a_target_wide_about_its_line_and_is_taken_where_it_was_pressed() {
        // The line at 200 px.
        for x in [188.0, 200.0, 212.0] {
            assert_eq!(press(&plan(), x, MID), PlanPress::Boundary { zone: 1 }, "{x}");
        }
        assert_eq!(press(&plan(), 187.0, MID), PlanPress::Street { at_mm: 18_700 });
        assert_eq!(press(&plan(), 213.0, MID), PlanPress::Street { at_mm: 21_300 });
        assert_eq!(hit(&plan(), WIDTH, 207.0, MID, TARGET_PX).grab_px, 7.0);
    }

    #[test]
    fn a_handle_is_a_target_wide_and_wins_over_what_lies_under_it() {
        let p = PlanView { pick: Some((20_000, 40_000)), objects: vec![thing(PlanThing::Calming { cell: 0 }, 39_000, 41_000, None)], ..plan() };
        // The left handle sits on the boundary, the right one on the raised element.
        assert_eq!(hit(&p, WIDTH, 195.0, MID, TARGET_PX), Hit { press: PlanPress::Handle { left: true }, grab_px: -5.0 });
        assert_eq!(press(&p, 411.0, MID), PlanPress::Handle { left: false });
        assert_eq!(press(&p, 389.0, MID), PlanPress::Handle { left: false });
        assert_eq!(press(&p, 300.0, MID), PlanPress::Street { at_mm: 30_000 }, "inside the pick is the street");
        // The handles reach a little past the bands, no further.
        assert_eq!(press(&p, 400.0, LANE_H - HANDLE_OVER), PlanPress::Handle { left: false });
        assert_eq!(press(&p, 200.0, LANE_H - HANDLE_OVER - 1.0), PlanPress::Lane { side: Side::Left, at_mm: 20_000 });
        // A pick 5 m long at the minimum scale: each handle takes the half nearer it.
        let long = 2_000_000;
        let p = PlanView { length_mm: Some(long), runs: vec![run(0, long, 0)], boundaries: Vec::new(), pick: Some((100_000, 105_000)), ..plan() };
        let (left, right) = (100.0 * super::super::MIN_PX_PER_M, 105.0 * super::super::MIN_PX_PER_M);
        assert!((right - left - TARGET_PX).abs() < 1e-9, "{left} {right}");
        assert_eq!(press(&p, left + 11.0, MID), PlanPress::Handle { left: true });
        assert_eq!(press(&p, right - 11.0, MID), PlanPress::Handle { left: false });
    }

    #[test]
    fn a_thing_shorter_than_a_target_is_still_a_target_wide_and_high() {
        // A tree 1 m long (10 px), on the right sidewalk.
        let tree = PlanObject { host: Some(3), ..thing(PlanThing::Green { i: 0 }, 50_000, 51_000, None) };
        let p = PlanView { objects: vec![tree], ..plan() };
        let at = |x: f64, y: f64| press(&p, x, y);
        let tree = |x: f64| PlanPress::Thing { thing: PlanThing::Green { i: 0 }, at_mm: (x * 100.0) as i32 };
        // The sidewalk is drawn about 26 px high, from about 94 px to the bottom of the bands.
        let (top, bottom) = (LANE_H + BANDS_H - 2_000.0 * BANDS_H / 7_300.0, LANE_H + BANDS_H);
        assert!(bottom - top >= TARGET_PX);
        assert_eq!(at(493.0, top + 1.0), tree(493.0));
        assert_eq!(at(517.0, bottom - 1.0), tree(517.0));
        assert_eq!(at(492.0, top + 1.0), PlanPress::Street { at_mm: 49_200 });
        assert_eq!(at(505.0, top - 1.0), PlanPress::Street { at_mm: 50_500 }, "not the travel lane beside it");
        // A thing in the lane of its side is the lane high.
        let stop = thing(PlanThing::Stop { side: Side::Left, cell: 0 }, 10_000, 10_500, Some(Side::Left));
        let p = PlanView { objects: vec![stop], ..plan() };
        for y in [0.0, LANE_H - 0.5] {
            assert!(matches!(press(&p, 102.0, y), PlanPress::Thing { .. }), "{y}");
        }
    }

    #[test]
    fn on_a_coarse_pointer_every_target_is_44_px_each_way() {
        const COARSE: f64 = 44.0;
        // The boundary's line at 200 px.
        for x in [178.0, 222.0] {
            assert_eq!(press_at(&plan(), x, MID, COARSE), PlanPress::Boundary { zone: 1 }, "{x}");
        }
        assert_eq!(press_at(&plan(), 177.0, MID, COARSE), PlanPress::Street { at_mm: 17_700 });
        // A tree 1 m long (10 px) on the right sidewalk: 44 px about its middle, along and across.
        let tree = PlanObject { host: Some(3), ..thing(PlanThing::Green { i: 0 }, 50_000, 51_000, None) };
        let p = PlanView { objects: vec![tree], ..plan() };
        let is_tree = |x: f64, y: f64| matches!(press_at(&p, x, y, COARSE), PlanPress::Thing { .. });
        assert!(is_tree(483.0, MID + 30.0) && is_tree(527.0, MID + 30.0));
        assert!(!is_tree(482.0, MID + 30.0) && !is_tree(528.0, MID + 30.0));
        // A stop in the lane above the bands reaches 44 px down from its middle's 22 px past the lane.
        let stop = thing(PlanThing::Stop { side: Side::Left, cell: 0 }, 10_000, 10_500, Some(Side::Left));
        let p = PlanView { objects: vec![stop], ..plan() };
        assert!(matches!(press_at(&p, 102.0, 33.0, COARSE), PlanPress::Thing { .. }));
        assert!(matches!(press_at(&p, 81.0, 12.0, COARSE), PlanPress::Thing { .. }));
        assert!(!matches!(press_at(&p, 79.0, 12.0, COARSE), PlanPress::Thing { .. }));
        // The handles too.
        let p = PlanView { pick: Some((20_000, 40_000)), ..plan() };
        assert_eq!(press_at(&p, 421.0, MID, COARSE), PlanPress::Handle { left: false });
        assert_eq!(press_at(&p, 423.0, MID, COARSE), PlanPress::Street { at_mm: 42_300 });
    }

    #[test]
    fn a_boundary_that_cannot_move_is_no_target() {
        let p = PlanView { boundaries: vec![PlanBoundary { movable: false, ..plan().boundaries[0] }], ..plan() };
        assert_eq!(press(&p, 205.0, MID), PlanPress::Street { at_mm: 20_500 });
    }

    #[test]
    fn where_things_overlap_the_one_drawn_on_top_takes_the_press() {
        // A curbside use and a stop on the left side, the stop drawn after the use and over part of it.
        let curb = thing(PlanThing::Curb { side: Side::Left, cell: 1 }, 10_000, 30_000, Some(Side::Left));
        let stop = thing(PlanThing::Stop { side: Side::Left, cell: 0 }, 25_000, 40_000, Some(Side::Left));
        let p = PlanView { objects: vec![curb.clone(), stop.clone()], ..plan() };
        let on = |x: f64| match press(&p, x, LANE_H / 2.0) {
            PlanPress::Thing { thing, .. } => Some(thing),
            _ => None,
        };
        assert_eq!(on(150.0), Some(curb.thing));
        assert_eq!(on(270.0), Some(stop.thing), "where both lie, the stop drawn on top");
        assert_eq!(on(350.0), Some(stop.thing));
        // Drawn the other way round, the use is on top.
        let p = PlanView { objects: vec![stop.clone(), curb.clone()], ..plan() };
        assert!(matches!(press(&p, 270.0, LANE_H / 2.0), PlanPress::Thing { thing, .. } if thing == curb.thing));
        // A thing over a boundary takes the press there.
        let table = thing(PlanThing::Calming { cell: 0 }, 19_000, 22_000, None);
        let p = PlanView { objects: vec![table], ..plan() };
        assert!(matches!(press(&p, 200.0, MID), PlanPress::Thing { .. }));
    }
}
