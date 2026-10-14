//! The ring of a turbo roundabout as geometry, in millimetres on the plan. Each edge of a ring lane is a "stadium":
//! two half circles of one radius whose centres lie `shift` apart along the translation axis, joined by two straight
//! sides parallel to it. (The spec's "two half circles shifted apart", read so that the sides meet the arcs
//! tangentially.) The island's edge is shifted by Δv, every other edge by Δu. No rules here, and no `State`.

use std::f64::consts::{FRAC_PI_2, PI};

use serde::Serialize;

use crate::junction::geometry::{P, add, dir, dist, right, scale};
use crate::junction::turbo::{Edge, TurboBlock};

/// Points along each half circle (so an edge has `2 * (STEPS + 1)`). Fine enough that a chord sits within ~11 mm of the arc at 21 m.
pub const STEPS: usize = 48;

/// The centres of the two halves of an edge: the one toward the axis direction and the one away from it.
pub fn half_centres(axis_deg: f64, shift_mm: f64) -> (P, P) {
    let forward = scale(dir(axis_deg), shift_mm / 2.0);
    (forward, scale(forward, -1.0))
}

/// A closed edge, as points running clockwise on the plan (increasing bearing); the last joins the first.
pub fn stadium(radius_mm: f64, shift_mm: f64, axis_deg: f64) -> Vec<P> {
    let (a, n) = (dir(axis_deg), right(axis_deg));
    let (forward, back) = half_centres(axis_deg, shift_mm);
    let mut pts = Vec::with_capacity(2 * (STEPS + 1));
    for (centre, from) in [(forward, -FRAC_PI_2), (back, FRAC_PI_2)] {
        for i in 0..=STEPS {
            let t = from + PI * i as f64 / STEPS as f64;
            pts.push(add(centre, add(scale(a, radius_mm * t.cos()), scale(n, radius_mm * t.sin()))));
        }
    }
    pts
}

pub fn edge(e: Edge, axis_deg: f64) -> Vec<P> {
    stadium(e.radius_mm as f64, e.shift_mm as f64, axis_deg)
}

/// One ring lane: its inner and outer edge and the line along its middle.
#[derive(Clone, Debug, Serialize)]
pub struct RingLane {
    pub inner: Vec<P>,
    pub outer: Vec<P>,
    pub centre: Vec<P>,
}

/// Everything the ring is drawn from.
#[derive(Clone, Debug)]
pub struct RingShape {
    /// Innermost first.
    pub lanes: Vec<RingLane>,
    /// The centre line of each painted divider, innermost first.
    pub dividers: Vec<Vec<P>>,
    /// The island's edge (the innermost lane's inner edge).
    pub island: Vec<P>,
    /// The carriageway's outer edge (the outermost lane's outer edge).
    pub road: Vec<P>,
    /// The outer edge of the cycle track, which is the road's edge when there is none.
    pub outer: Vec<P>,
}

pub fn ring_shape(block: &TurboBlock, ring_lanes: usize, cycle_mm: i32, axis_deg: f64) -> RingShape {
    let lanes: Vec<RingLane> = block
        .lanes(ring_lanes)
        .iter()
        .map(|[inner, outer]| {
            let middle = Edge { radius_mm: (inner.radius_mm + outer.radius_mm) / 2, shift_mm: (inner.shift_mm + outer.shift_mm) / 2 };
            RingLane { inner: edge(*inner, axis_deg), outer: edge(*outer, axis_deg), centre: edge(middle, axis_deg) }
        })
        .collect();
    RingShape {
        island: lanes.first().map(|l| l.inner.clone()).unwrap_or_default(),
        road: lanes.last().map(|l| l.outer.clone()).unwrap_or_default(),
        dividers: block.dividers(ring_lanes).into_iter().map(|e| edge(e, axis_deg)).collect(),
        outer: edge(Edge { radius_mm: block.road_outer_mm(ring_lanes) + cycle_mm, shift_mm: block.du_mm }, axis_deg),
        lanes,
    }
}

/// The hits of the line `lat` off an arm's axis (the arm at `bearing`) with the segments of `points`, as distances
/// out along the arm; the farthest. `closed` joins the last point to the first.
fn farthest_hit(points: &[P], closed: bool, bearing: f64, lat: f64) -> Option<f64> {
    let (d, r) = (dir(bearing), right(bearing));
    let along = |p: P| p.0 * d.0 + p.1 * d.1;
    let across = |p: P| p.0 * r.0 + p.1 * r.1;
    let n = points.len();
    let segments = if closed { n } else { n.saturating_sub(1) };
    (0..segments)
        .filter_map(|i| {
            let (p, q) = (points[i], points[(i + 1) % n]);
            let (lp, lq) = (across(p), across(q));
            if (lp - lat) * (lq - lat) > 0.0 || (lq - lp).abs() < 1e-9 {
                return None;
            }
            Some(along(p) + (lat - lp) / (lq - lp) * (along(q) - along(p)))
        })
        .max_by(f64::total_cmp)
}

/// How far out along an arm of `bearing` the line `lat` off its axis meets the closed `outline`: the farthest such
/// hit, which is where the arm meets the ring from outside. None when the line misses it.
pub fn mouth_on(outline: &[P], bearing: f64, lat: f64) -> Option<f64> {
    farthest_hit(outline, true, bearing, lat)
}

/// The same for an open line (a lane's edge or centre line on one half of the ring).
pub fn mouth_on_open(line: &[P], bearing: f64, lat: f64) -> Option<f64> {
    farthest_hit(line, false, bearing, lat)
}

/// The index of the point nearest `p`.
pub fn nearest(points: &[P], p: P) -> usize {
    (0..points.len()).min_by(|&a, &b| dist(points[a], p).total_cmp(&dist(points[b], p))).unwrap_or(0)
}

/// The outline's vertices from the one nearest `from` to the one nearest `to`, clockwise (`forward`) or the other way.
pub fn follow(outline: &[P], from: P, to: P, forward: bool) -> Vec<P> {
    let n = outline.len();
    if n == 0 {
        return Vec::new();
    }
    let (mut i, end) = (nearest(outline, from), nearest(outline, to));
    let mut out = vec![outline[i]];
    while i != end {
        i = if forward { (i + 1) % n } else { (i + n - 1) % n };
        out.push(outline[i]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::geometry::{dir, right};
    use crate::junction::turbo::BLOCKS;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    /// The farthest the points reach along `axis_deg`, and across it.
    fn extent(pts: &[P], axis_deg: f64) -> (f64, f64) {
        let (a, n) = (dir(axis_deg), right(axis_deg));
        pts.iter().fold((f64::MIN, f64::MIN), |(u, v), p| (u.max(p.0 * a.0 + p.1 * a.1), v.max(p.0 * n.0 + p.1 * n.1)))
    }

    #[test]
    fn the_halves_of_an_edge_are_centred_half_the_shift_along_the_axis() {
        let (fwd, back) = half_centres(57.5, 5_050.0);
        assert!(near(fwd.0, 2_129.5634) && near(fwd.1, -1_356.6815), "{fwd:?}: 2525 mm along 57.5 degrees from north");
        assert!(near(back.0, -2_129.5634) && near(back.1, 1_356.6815));
        let (fwd, back) = half_centres(65.0, 5_050.0);
        assert!(near(fwd.0, 2_288.4272) && near(fwd.1, -1_067.1112), "{fwd:?}");
        assert!(near(back.0, -2_288.4272) && near(back.1, 1_067.1112));
        assert_eq!(half_centres(0.0, 0.0), ((0.0, 0.0), (-0.0, 0.0)));
    }

    #[test]
    fn an_edge_is_two_half_circles_so_far_apart_joined_by_straight_sides() {
        let pts = stadium(21_150.0, 5_050.0, 57.5);
        assert_eq!(pts.len(), 2 * (STEPS + 1));
        let (along, across) = extent(&pts, 57.5);
        assert!(near(along, 21_150.0 + 2_525.0), "the tips are R + shift/2 out along the axis: {along}");
        assert!(near(across, 21_150.0), "and R either side of it: {across}");
        let (fwd, _) = half_centres(57.5, 5_050.0);
        let tip = (fwd.0 + 21_150.0 * dir(57.5).0, fwd.1 + 21_150.0 * dir(57.5).1);
        assert!(pts.iter().any(|p| near(p.0, tip.0) && near(p.1, tip.1)), "one point is the tip of the forward half");
    }

    #[test]
    fn an_edge_with_no_shift_is_a_circle() {
        let pts = stadium(10_000.0, 0.0, 33.0);
        assert!(pts.iter().all(|p| (p.0.hypot(p.1) - 10_000.0).abs() < 1e-6));
    }

    #[test]
    fn an_edge_runs_clockwise_on_the_plan() {
        // North is up and y grows down: clockwise on the screen is increasing bearing, so the area it encloses is positive by the shoelace sum in y-down axes.
        let pts = stadium(1_000.0, 400.0, 0.0);
        let area: f64 = (0..pts.len()).map(|i| (pts[i].0 * pts[(i + 1) % pts.len()].1) - (pts[(i + 1) % pts.len()].0 * pts[i].1)).sum::<f64>() / 2.0;
        assert!(area > 0.0, "{area}");
    }

    #[test]
    fn every_block_puts_its_lane_edges_at_its_radii_across_the_axis_and_shifts_them_along_it() {
        let radii = [
            [10_500.0, 15_850.0, 16_150.0, 21_150.0],
            [12_000.0, 17_150.0, 17_450.0, 22_450.0],
            [15_000.0, 20_000.0, 20_300.0, 25_200.0],
            [20_000.0, 24_900.0, 25_200.0, 29_900.0],
        ];
        for (b, want) in BLOCKS.iter().zip(radii) {
            let shape = ring_shape(b, 2, 0, 57.5);
            assert_eq!(shape.lanes.len(), 2);
            let across = |pts: &[P]| extent(pts, 57.5).1;
            assert!(near(across(&shape.island), want[0]), "{}: the island", b.id);
            assert!(near(across(&shape.lanes[0].outer), want[1]), "{}: R2", b.id);
            assert!(near(across(&shape.lanes[1].inner), want[2]), "{}: R3", b.id);
            assert!(near(across(&shape.lanes[1].outer), want[3]), "{}: R4", b.id);
            assert!(near(across(&shape.road), want[3]) && near(across(&shape.outer), want[3]), "{}: no cycle track, no more", b.id);
            let along = |pts: &[P]| extent(pts, 57.5).0;
            assert!(near(along(&shape.island), want[0] + b.dv_mm as f64 / 2.0), "{}: the island is shifted by dv", b.id);
            assert!(near(along(&shape.lanes[1].outer), want[3] + b.du_mm as f64 / 2.0), "{}: the lanes by du", b.id);
            assert_eq!(shape.dividers.len(), 1);
            assert!(near(across(&shape.dividers[0]), (want[1] + want[2]) / 2.0), "{}: the divider is centred in the 300 mm", b.id);
        }
    }

    #[test]
    fn a_third_ring_lane_lies_outside_a_further_divider_and_a_cycle_track_outside_that() {
        let mini = &BLOCKS[0];
        let shape = ring_shape(mini, 3, 2_000, 57.5);
        assert_eq!(shape.lanes.len(), 3);
        assert_eq!(shape.dividers.len(), 2);
        let across = |pts: &[P]| extent(pts, 57.5).1;
        assert!(near(across(&shape.lanes[2].inner), 21_450.0));
        assert!(near(across(&shape.lanes[2].outer), 26_450.0) && near(across(&shape.road), 26_450.0));
        assert!(near(across(&shape.outer), 28_450.0), "the cycle track is 2 m outside the road");
        assert!(near(across(&shape.dividers[1]), 21_300.0));
    }

    #[test]
    fn a_lane_s_centre_line_lies_between_its_edges() {
        let shape = ring_shape(&BLOCKS[1], 2, 0, 65.0);
        let lane = &shape.lanes[0];
        assert!(near(extent(&lane.centre, 65.0).1, (12_000.0 + 17_150.0) / 2.0));
        assert_eq!(lane.centre.len(), 2 * (STEPS + 1));
    }

    #[test]
    fn an_arm_meets_the_outline_at_its_farthest_hit() {
        // a ring 10 m across the axis and 12 m along it (shift 4 m), its axis pointing north
        let ring = stadium(10_000.0, 4_000.0, 0.0);
        assert!(near(mouth_on(&ring, 0.0, 0.0).unwrap(), 12_000.0), "an arm up the axis meets the tip");
        assert!(near(mouth_on(&ring, 90.0, 0.0).unwrap(), 10_000.0), "an arm across it meets the straight side");
        assert!(near(mouth_on(&ring, 180.0, 0.0).unwrap(), 12_000.0));
        let t = mouth_on(&ring, 0.0, 6_000.0).unwrap();
        assert!(t > 8_000.0 && t < 12_000.0, "off the axis it meets the arc a little short of the tip: {t}");
        assert_eq!(mouth_on(&ring, 0.0, 10_500.0), None, "a line beside the ring misses it");
    }

    #[test]
    fn walking_an_outline_goes_from_the_vertex_nearest_one_point_to_the_one_nearest_another() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert_eq!(nearest(&square, (9.0, 1.0)), 1);
        assert_eq!(follow(&square, (0.5, 0.0), (9.0, 9.0), true), [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        assert_eq!(follow(&square, (0.5, 0.0), (9.0, 9.0), false), [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0)]);
        assert_eq!(follow(&square, (0.0, 0.0), (0.0, 0.0), true), [(0.0, 0.0)]);
    }

    #[test]
    fn every_block_and_lane_count_draws_finite_points() {
        for b in &BLOCKS {
            for lanes in [2, 3] {
                for cycle in [0, 3_000] {
                    let s = ring_shape(b, lanes, cycle, 302.5);
                    let all = s
                        .lanes
                        .iter()
                        .flat_map(|l| l.inner.iter().chain(&l.outer).chain(&l.centre))
                        .chain(s.dividers.iter().flatten())
                        .chain(&s.island)
                        .chain(&s.road)
                        .chain(&s.outer);
                    assert!(all.into_iter().all(|p| p.0.is_finite() && p.1.is_finite()), "{} {lanes} {cycle}", b.id);
                }
            }
        }
    }

    #[test]
    fn an_open_line_has_no_closing_side() {
        let line = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0)];
        assert_eq!(mouth_on_open(&line, 0.0, 5.0), Some(-10.0));
        assert_eq!(mouth_on(&line, 0.0, 5.0), Some(-5.0), "closed, the side from the last point back to the first is met as well, and it is farther along");
    }
}
