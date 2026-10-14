//! The ring of a turbo roundabout as geometry, in millimetres on the plan: a spiral (replaces the stadium of `turbo_ring.rs`). Each lane edge is two half
//! circles, one on each side of the translation axis, whose centres are `shift` apart along it (CETRA 2016 Fig. 3 and
//! 4; the island's edge is shifted by Δv and every other edge by Δu). The *ahead* half has its centre `shift / 2`
//! toward the axis direction and the *behind* half `shift / 2` away from it, so the two halves meet the axis at
//! different points, the front tip and the back tip. There the inner edge of a lane on one half meets the outer edge
//! of the same lane on the other, to within 50 mm for all four Dutch blocks: a lane goes on as the next lane out, the
//! outermost lane leaves the ring, and the innermost lane of a half begins against the island. UNVERIFIED (spec
//! amendment 2026-10-13, J1 to J4): which half is ahead (so where the tips lie among the streets), where a divider
//! begins and ends along the ring, the third lane, and where a movement leaves the ring. No rules here, and no `State`.

use std::f64::consts::PI;

use serde::Serialize;

use crate::junction::geometry::{P, add, at, dir, dist, right, scale};
use crate::junction::turbo::{Edge, TurboBlock};
use crate::junction::turbo_ring::{mouth_on, mouth_on_open, nearest};

/// Points along each half circle (so an edge has `2 * (STEPS + 1)`). Fine enough that a chord sits within ~11 mm of the arc at 21 m.
pub const STEPS: usize = 48;

/// A stretch of divider shorter than this (UNVERIFIED, a placeholder) between two streets is not kept: it would be a stub.
pub const MIN_DIVIDER_MM: f64 = 3_000.0;

/// The translation axis as a unit vector, and the unit vector toward the ahead half. Traffic goes round the ahead half
/// from the back tip to the front tip and then round the other half from the front tip to the back tip: anticlockwise
/// for right-hand traffic, where the ahead half is on the clockwise side of the axis, and clockwise for left-hand
/// traffic, where it is on the other.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axes {
    pub axis: P,
    pub side: P,
}

impl Axes {
    pub fn new(axis_deg: f64, right_hand: bool) -> Axes {
        let n = right(axis_deg);
        Axes { axis: dir(axis_deg), side: if right_hand { n } else { scale(n, -1.0) } }
    }

    /// Whether `p` lies on the ahead side of the axis (a point on it counts as ahead).
    pub fn ahead_of(&self, p: P) -> bool {
        p.0 * self.side.0 + p.1 * self.side.1 >= 0.0
    }
}

/// One half of an edge, `STEPS + 1` points from one tip to the other in the direction of traffic: the ahead half from
/// the back tip to the front tip, the behind half from the front tip to the back tip.
pub fn half_arc(axes: &Axes, ahead: bool, e: Edge) -> Vec<P> {
    let (r, s) = (e.radius_mm as f64, e.shift_mm as f64);
    let centre = scale(axes.axis, if ahead { s / 2.0 } else { -s / 2.0 });
    let bulge = if ahead { axes.side } else { scale(axes.side, -1.0) };
    (0..=STEPS)
        .map(|i| {
            let f = i as f64 / STEPS as f64;
            // The angle from the axis direction: 0 at the front tip, PI at the back tip.
            let t = if ahead { PI * (1.0 - f) } else { PI * f };
            add(centre, add(scale(axes.axis, r * t.cos()), scale(bulge, r * t.sin())))
        })
        .collect()
}

/// A closed edge as points running clockwise on the plan (increasing bearing), the last joining the first: the ahead
/// half then the behind half, with a straight step along the axis at each tip where they join, `shift` long.
pub fn ring_outline(axes: &Axes, e: Edge, right_hand: bool) -> Vec<P> {
    let mut pts = half_arc(axes, true, e);
    pts.extend(half_arc(axes, false, e));
    // That is the way traffic goes: anticlockwise for right-hand traffic.
    if right_hand {
        pts.reverse();
    }
    pts
}

/// One ring lane on one half of the ring: lane `k` (0 is the innermost), as open paths in the direction of traffic.
#[derive(Clone, Debug, Serialize)]
pub struct RingLane {
    pub k: usize,
    pub ahead: bool,
    pub inner: Vec<P>,
    pub outer: Vec<P>,
    pub centre: Vec<P>,
}

impl RingLane {
    /// The lane as a closed band: the outer edge, then the inner edge back.
    pub fn polygon(&self) -> Vec<P> {
        self.outer.iter().chain(self.inner.iter().rev()).copied().collect()
    }
}

/// A street's carriageway where it meets the ring: its bearing, and the lateral position of its left and right curb
/// (what `ArmLayout::cl` and `cr` are).
#[derive(Clone, Copy, Debug)]
pub struct Mouth {
    pub bearing: f64,
    pub cl: f64,
    pub cr: f64,
}

/// Everything the ring is drawn from.
#[derive(Clone, Debug)]
pub struct RingShape {
    pub axes: Axes,
    /// Two for each ring lane (one on each half), lane 0 first, the ahead half before the behind.
    pub lanes: Vec<RingLane>,
    /// The centre line of each painted divider, in open pieces, each between two streets (or a street and a tip).
    pub dividers: Vec<Vec<P>>,
    /// The island's edge, as a closed outline.
    pub island: Vec<P>,
    /// The carriageway's outer edge.
    pub road: Vec<P>,
    /// The outer edge of the cycle track, which is the road's edge when there is none.
    pub outer: Vec<P>,
    /// The streets' carriageways the dividers were cut at (a walk leaves the ring at a tip only for the first one after it).
    pub mouths: Vec<Mouth>,
    /// Whether traffic goes anticlockwise (right-hand traffic).
    pub anticlockwise: bool,
}

impl RingShape {
    /// How many lanes the ring has (each is two `RingLane`s).
    pub fn ring_lanes(&self) -> usize {
        self.lanes.iter().map(|l| l.k + 1).max().unwrap_or(0)
    }
}

/// Whether `p` lies inside a street's carriageway where it meets the ring: on the street's side of the middle, between its curbs.
fn inside_a_mouth(p: P, mouths: &[Mouth]) -> bool {
    mouths.iter().any(|m| {
        let (d, r) = (dir(m.bearing), right(m.bearing));
        let (along, across) = (p.0 * d.0 + p.1 * d.1, p.0 * r.0 + p.1 * r.1);
        along > 0.0 && across >= m.cl && across <= m.cr
    })
}

/// `arc` with the stretches inside a mouth cut out: the open pieces between them, those shorter than `MIN_DIVIDER_MM` dropped.
fn pieces(arc: &[P], mouths: &[Mouth]) -> Vec<Vec<P>> {
    let mut out: Vec<Vec<P>> = Vec::new();
    let mut run: Vec<P> = Vec::new();
    for &p in arc {
        if inside_a_mouth(p, mouths) {
            out.push(std::mem::take(&mut run));
        } else {
            run.push(p);
        }
    }
    out.push(run);
    out.retain(|r| r.len() >= 2 && r.windows(2).map(|w| dist(w[0], w[1])).sum::<f64>() >= MIN_DIVIDER_MM);
    out
}

pub fn ring_shape(block: &TurboBlock, ring_lanes: usize, cycle_mm: i32, axes: &Axes, right_hand: bool, mouths: &[Mouth]) -> RingShape {
    let edges = block.lanes(ring_lanes);
    let mut lanes = Vec::new();
    for (k, [inner, outer]) in edges.iter().enumerate() {
        let middle = Edge { radius_mm: (inner.radius_mm + outer.radius_mm) / 2, shift_mm: (inner.shift_mm + outer.shift_mm) / 2 };
        for ahead in [true, false] {
            lanes.push(RingLane {
                k,
                ahead,
                inner: half_arc(axes, ahead, *inner),
                outer: half_arc(axes, ahead, *outer),
                centre: half_arc(axes, ahead, middle),
            });
        }
    }
    let mut dividers = Vec::new();
    for d in block.dividers(ring_lanes) {
        for ahead in [true, false] {
            dividers.extend(pieces(&half_arc(axes, ahead, d), mouths));
        }
    }
    // A block always has at least two lanes.
    let (island, road) = (edges[0][0], edges[edges.len() - 1][1]);
    RingShape {
        axes: *axes,
        mouths: mouths.to_vec(),
        anticlockwise: right_hand,
        lanes,
        dividers,
        island: ring_outline(axes, island, right_hand),
        road: ring_outline(axes, road, right_hand),
        outer: ring_outline(axes, Edge { radius_mm: block.road_outer_mm(ring_lanes) + cycle_mm, shift_mm: block.du_mm }, right_hand),
    }
}

/// A movement's line round the ring.
#[derive(Clone, Debug)]
pub struct Walk {
    /// Where it meets its first lane, the lane's points in the direction of traffic, and where it meets the street it leaves by.
    pub points: Vec<P>,
    /// It reached a tip in the outermost lane and left the ring there, for the first street after the tip (UNVERIFIED, J4): the last point is the tip, and the line goes on in a straight chord to the street.
    pub left_at_tip: bool,
}

/// The line of a movement that comes in along the street `enter` (its bearing and the lateral position it runs at),
/// meets ring lane `k`, keeps its lane round the ring and steps out one lane at each tip, and leaves by the street
/// `leave`. None when `k` is no lane of the ring, when the streets miss it, or when the street cannot be reached from
/// lane `k`: the outermost lane leaves the ring at a tip, and for the first street after the tip only. Where the street
/// is on the half the walk is on and ahead of it, the walk ends there; otherwise it goes on to the tip.
pub fn spiral_walk(shape: &RingShape, k: usize, enter: (f64, f64), leave: (f64, f64)) -> Option<Walk> {
    // The lane, among the halves' lane k, that the street meets from outside: the one with the farthest hit.
    let (start, t) = shape
        .lanes
        .iter()
        .enumerate()
        .filter(|(_, l)| l.k == k)
        .filter_map(|(i, l)| mouth_on_open(&l.centre, enter.0, enter.1).map(|t| (i, t)))
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    let first = at(enter.0, enter.1, t);
    // The half the street it leaves by meets the carriageway on.
    let exit_ahead = mouth_on(&shape.road, leave.0, leave.1).map(|t| shape.axes.ahead_of(at(leave.0, leave.1, t)))?;
    let mut points = vec![first];
    let (mut lane, mut i) = (start, nearest(&shape.lanes[start].centre, first));
    loop {
        let l = &shape.lanes[lane];
        if l.ahead == exit_ahead
            && let Some(t) = mouth_on_open(&l.centre, leave.0, leave.1)
        {
            let last = at(leave.0, leave.1, t);
            let j = nearest(&l.centre, last);
            if j >= i {
                points.extend_from_slice(&l.centre[i..=j]);
                points.push(last);
                return Some(Walk { points, left_at_tip: false });
            }
        }
        points.extend_from_slice(&l.centre[i..]);
        // At the tip the lane goes on as the next lane out of the other half; the outermost has none, and leaves.
        match shape.lanes.iter().position(|n| n.k == l.k + 1 && n.ahead != l.ahead) {
            Some(next) => {
                lane = next;
                i = 0;
            }
            None => {
                // The outermost lane leaves at the tip, and only for the first street after it.
                let tip = *l.centre.last()?;
                let at_tip = tip.0.atan2(-tip.1).to_degrees();
                let after = |b: f64| if shape.anticlockwise { (at_tip - b).rem_euclid(360.0) } else { (b - at_tip).rem_euclid(360.0) };
                let first = shape.mouths.iter().map(|m| m.bearing).min_by(|a, b| after(*a).total_cmp(&after(*b)))?;
                return ((first - leave.0).abs() < 1e-6).then_some(Walk { points, left_at_tip: true });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::geometry::{add, at, cross, dist, polylines_cross, right, scale, sub};
    use crate::junction::turbo::BLOCKS;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    /// How far `p` is along the axis, and how far to the ahead side of it.
    fn along(p: P, a: &Axes) -> f64 {
        p.0 * a.axis.0 + p.1 * a.axis.1
    }

    fn aside(p: P, a: &Axes) -> f64 {
        p.0 * a.side.0 + p.1 * a.side.1
    }

    fn right_hand() -> Axes {
        Axes::new(57.5, true)
    }

    /// The four streets of a cross at the compass points, each 7 m of carriageway (3.5 m either side of its axis).
    fn cross_mouths() -> Vec<Mouth> {
        [0.0, 90.0, 180.0, 270.0].iter().map(|&bearing| Mouth { bearing, cl: -3_500.0, cr: 3_500.0 }).collect()
    }

    /// Whether `p` lies on the open polyline, to within 1 mm.
    fn on_line(line: &[P], p: P) -> bool {
        line.windows(2).any(|w| {
            let (ab, ap) = (sub(w[1], w[0]), sub(p, w[0]));
            let t = ((ap.0 * ab.0 + ap.1 * ab.1) / (ab.0 * ab.0 + ab.1 * ab.1).max(1e-9)).clamp(0.0, 1.0);
            dist(p, add(w[0], scale(ab, t))) < 1.0
        })
    }

    /// The lanes `(k, ahead)` a walk's points lie on, in order, each once in a row.
    fn lanes_of(shape: &RingShape, w: &Walk) -> Vec<(usize, bool)> {
        let mut seen = Vec::new();
        for &p in &w.points {
            let lane = shape.lanes.iter().find(|l| on_line(&l.centre, p)).map(|l| (l.k, l.ahead)).expect("every point of a walk lies on a lane's centre line");
            if seen.last() != Some(&lane) {
                seen.push(lane);
            }
        }
        seen
    }

    /// A walk in along a street's entering side (1 750 mm to the left of its axis, seen from the middle) and out along the exit side.
    fn walk(shape: &RingShape, k: usize, from: f64, to: f64) -> Walk {
        spiral_walk(shape, k, (from, -1_750.0), (to, 1_750.0)).expect("a walk")
    }

    #[test]
    fn a_half_edge_is_a_half_circle_on_its_side_of_the_axis_with_its_centre_half_the_shift_along_it() {
        let axes = right_hand();
        let edge = Edge { radius_mm: 21_150, shift_mm: 5_050 };
        let ahead = half_arc(&axes, true, edge);
        assert_eq!(ahead.len(), STEPS + 1);
        let centre = scale(axes.axis, 2_525.0);
        assert!(ahead.iter().all(|&p| near(dist(p, centre), 21_150.0)), "a circle of R about a centre 2 525 mm toward the axis direction");
        assert!(ahead.iter().all(|&p| aside(p, &axes) > -1e-6), "all on the ahead side");
        assert!(near(along(ahead[0], &axes), 2_525.0 - 21_150.0) && near(aside(ahead[0], &axes), 0.0), "it starts at the back tip, on the axis");
        assert!(near(along(ahead[STEPS], &axes), 2_525.0 + 21_150.0) && near(aside(ahead[STEPS], &axes), 0.0), "and ends at the front tip");
        let behind = half_arc(&axes, false, edge);
        assert!(near(along(behind[0], &axes), -2_525.0 + 21_150.0), "the other half starts at the front tip, 5 050 mm short of where the first ended");
        assert!(near(along(behind[STEPS], &axes), -2_525.0 - 21_150.0));
        assert!(behind.iter().all(|&p| aside(p, &axes) < 1e-6), "all on the other side");
    }

    #[test]
    fn the_edges_meet_across_the_axis_within_the_5_cm_of_the_dutch_figure() {
        // CETRA 2016 Fig. 4: at one end of the axis R1 of one half continues as R2 of the other and R3 as R4; at the other end the same
        // with the halves swapped. For a lane it is the inner edge of one half meeting the outer edge of the other. R1 is shifted by
        // dv and every other edge by du: that is the only labelling under which this holds, for all four blocks.
        let axes = right_hand();
        let tip = |ahead: bool, e: Edge, front: bool| {
            let arc = half_arc(&axes, ahead, e);
            along(arc[if ahead == front { STEPS } else { 0 }], &axes)
        };
        for b in &BLOCKS {
            for ring_lanes in [2, 3] {
                for [inner, outer] in b.lanes(ring_lanes) {
                    let name = format!("{}, {ring_lanes} lanes, radii {} and {}", b.id, inner.radius_mm, outer.radius_mm);
                    assert!((tip(true, inner, true) - tip(false, outer, true)).abs() <= 50.0 + 1e-6, "front tip: {name}");
                    assert!((tip(false, inner, false) - tip(true, outer, false)).abs() <= 50.0 + 1e-6, "back tip: {name}");
                    assert!((tip(true, inner, true) - tip(false, inner, true)).abs() >= 4_000.0, "one edge steps by its shift at the axis: {name}");
                }
            }
        }
    }

    #[test]
    fn the_outline_is_one_clockwise_curve_that_steps_back_along_the_axis_at_each_tip() {
        for right_hand in [true, false] {
            let axes = Axes::new(57.5, right_hand);
            let pts = ring_outline(&axes, Edge { radius_mm: 21_150, shift_mm: 5_050 }, right_hand);
            let n = pts.len();
            assert_eq!(n, 2 * (STEPS + 1));
            // North is up and y grows down, so clockwise on the screen is a positive shoelace sum.
            let area: f64 = (0..n).map(|i| cross(pts[i], pts[(i + 1) % n])).sum::<f64>() / 2.0;
            assert!(area > 0.0, "right-hand {right_hand}: {area}");
            let steps: Vec<f64> = (0..n)
                .filter(|&i| aside(pts[i], &axes).abs() < 1e-3 && aside(pts[(i + 1) % n], &axes).abs() < 1e-3)
                .map(|i| dist(pts[i], pts[(i + 1) % n]))
                .collect();
            assert_eq!(steps.len(), 2, "two sides lie on the axis, right-hand {right_hand}");
            assert!(steps.iter().all(|&s| near(s, 5_050.0)), "each as long as the shift: {steps:?}");
            let (lo, hi) = pts.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &p| (lo.min(along(p, &axes)), hi.max(along(p, &axes))));
            assert!(near(hi, 21_150.0 + 2_525.0) && near(lo, -21_150.0 - 2_525.0), "no wider along the axis than the stadium was");
        }
    }

    #[test]
    fn left_hand_traffic_gets_the_mirror_image_of_the_ring_across_the_axis() {
        let reflect = |p: P, a: &Axes| {
            let k = 2.0 * along(p, a);
            (k * a.axis.0 - p.0, k * a.axis.1 - p.1)
        };
        let (r, l) = (Axes::new(302.5, true), Axes::new(302.5, false));
        let edge = Edge { radius_mm: 21_150, shift_mm: 5_050 };
        let (right_out, left_out) = (ring_outline(&r, edge, true), ring_outline(&l, edge, false));
        for (i, p) in left_out.iter().enumerate() {
            let q = reflect(right_out[right_out.len() - 1 - i], &r);
            assert!(dist(*p, q) < 1e-3, "point {i}: {p:?} against the mirror of {q:?}");
        }
    }

    #[test]
    fn each_ring_lane_is_an_open_band_on_each_half_running_the_way_traffic_goes() {
        for right_hand in [true, false] {
            let axes = Axes::new(57.5, right_hand);
            let shape = ring_shape(&BLOCKS[0], 2, 0, &axes, right_hand, &[]);
            assert_eq!(shape.lanes.iter().map(|l| (l.k, l.ahead)).collect::<Vec<_>>(), [(0, true), (0, false), (1, true), (1, false)]);
            assert_eq!(shape.ring_lanes(), 2);
            for l in &shape.lanes {
                assert!(l.centre.len() == STEPS + 1 && l.centre.first() != l.centre.last(), "a lane on one half is open");
                let sweep: f64 = l.centre.windows(2).map(|w| cross(w[0], w[1])).sum();
                // Anticlockwise on the screen is a negative sum in these axes.
                assert_eq!(sweep > 0.0, !right_hand, "lane {} ahead {} runs anticlockwise for right-hand traffic only", l.k, l.ahead);
                let mid = STEPS / 2;
                let (i, c, o) = (aside(l.inner[mid], &axes).abs(), aside(l.centre[mid], &axes).abs(), aside(l.outer[mid], &axes).abs());
                assert!(i < c && c < o, "the centre line lies between the edges: {i} {c} {o}");
                assert_eq!(l.polygon().len(), l.outer.len() + l.inner.len());
            }
        }
    }

    #[test]
    fn at_each_tip_the_lane_that_goes_on_passes_between_the_ends_of_the_two_dividers() {
        let axes = right_hand();
        for b in &BLOCKS {
            let shape = ring_shape(b, 2, 0, &axes, true, &[]);
            assert_eq!(shape.dividers.len(), 2, "no street, so the divider of each half is whole");
            let (ahead_d, behind_d) = (&shape.dividers[0], &shape.dividers[1]);
            let lane = |k: usize, ahead: bool| shape.lanes.iter().find(|l| l.k == k && l.ahead == ahead).unwrap();
            // Front tip: lane 0 of the ahead half goes on as lane 1 of the other.
            let (from, to) = (*lane(0, true).centre.last().unwrap(), lane(1, false).centre[0]);
            let (d_ahead, d_behind) = (along(*ahead_d.last().unwrap(), &axes), along(behind_d[0], &axes));
            for u in [along(from, &axes), along(to, &axes)] {
                assert!(d_behind < u && u < d_ahead, "{}: front tip {d_behind} < {u} < {d_ahead}", b.id);
            }
            // Back tip: lane 0 of the other half goes on as lane 1 of the ahead half.
            let (from, to) = (*lane(0, false).centre.last().unwrap(), lane(1, true).centre[0]);
            let (d_ahead, d_behind) = (along(ahead_d[0], &axes), along(*behind_d.last().unwrap(), &axes));
            for u in [along(from, &axes), along(to, &axes)] {
                assert!(d_behind < u && u < d_ahead, "{}: back tip {d_behind} < {u} < {d_ahead}", b.id);
            }
        }
    }

    #[test]
    fn a_divider_stops_where_a_street_meets_the_ring_and_starts_again_beyond_it() {
        let axes = right_hand();
        let east = Mouth { bearing: 90.0, cl: -3_500.0, cr: 3_500.0 };
        let shape = ring_shape(&BLOCKS[0], 2, 0, &axes, true, &[east]);
        // The ahead half (bearings 57.5 to 237.5) holds the east street, so its divider is cut in two; the other half's is whole.
        assert_eq!(shape.dividers.len(), 3);
        let r = right(90.0);
        let across = |p: P| p.0 * r.0 + p.1 * r.1;
        for piece in &shape.dividers {
            assert!(piece.iter().all(|&p| !(p.0 > 0.0 && across(p).abs() <= 3_500.0)), "no divider inside the street's carriageway");
        }
        // Traffic goes anticlockwise, so the first piece runs up to the street's south edge (y grows down) and the second begins past its north edge,
        // each within one step (about 1 050 mm on the mini block) of it.
        let first_end = across(*shape.dividers[0].last().unwrap());
        let second_start = across(shape.dividers[1][0]);
        assert!(first_end >= 3_500.0 && first_end < 3_500.0 + 1_100.0, "{first_end}");
        assert!(second_start <= -3_500.0 && second_start > -3_500.0 - 1_100.0, "{second_start}");
    }

    #[test]
    fn a_stretch_of_divider_shorter_than_3_m_between_two_streets_is_not_kept() {
        // A straight "arc" up the line x = 5 000, a point every metre from y = -10 000 to 10 000, and two streets due east: one whose carriageway
        // covers y from -2 500 to 2 500, one y from 4 500 to 5 500. Outside them: y <= -3 000 (kept), y = 3 000 and 4 000 (1 m: dropped), y >= 6 000 (kept).
        let arc: Vec<P> = (-10..=10).map(|i| (5_000.0, i as f64 * 1_000.0)).collect();
        let mouths = [Mouth { bearing: 90.0, cl: -2_500.0, cr: 2_500.0 }, Mouth { bearing: 90.0, cl: 4_500.0, cr: 5_500.0 }];
        let got = pieces(&arc, &mouths);
        assert_eq!(got.len(), 2);
        assert_eq!((got[0][0].1, got[0].last().unwrap().1), (-10_000.0, -3_000.0));
        assert_eq!((got[1][0].1, got[1].last().unwrap().1), (6_000.0, 10_000.0));
        assert_eq!(pieces(&arc, &[]).len(), 1, "with no street the arc is whole");
    }

    #[test]
    fn a_walk_keeps_its_lane_and_steps_out_one_lane_at_each_tip() {
        // North, east, south and west are bearings 0, 90, 180, 270. The axis points 57.5 degrees, so the ahead half holds east and south,
        // the behind half north and west, and traffic goes anticlockwise: north, west, south, east.
        let shape = ring_shape(&BLOCKS[0], 2, 0, &right_hand(), true, &cross_mouths());
        let w = walk(&shape, 0, 0.0, 90.0);
        assert_eq!(
            lanes_of(&shape, &w),
            [(0, false), (1, true)],
            "north to east: the inner lane round to the back tip, then the outer lane of the other half, out at east"
        );
        assert!(!w.left_at_tip);
        assert_eq!(lanes_of(&shape, &walk(&shape, 1, 0.0, 270.0)), [(1, false)], "north to west on the outer lane never reaches a tip");
        assert_eq!(lanes_of(&shape, &walk(&shape, 0, 0.0, 180.0)), [(0, false), (1, true)], "north to south on the inner lane steps out at the back tip");
        assert_eq!(lanes_of(&shape, &walk(&shape, 0, 90.0, 0.0)), [(0, true), (1, false)], "east to north on the inner lane steps out at the front tip");
        assert_eq!(lanes_of(&shape, &walk(&shape, 0, 90.0, 270.0)), [(0, true), (1, false)], "and so does east to west");
    }

    #[test]
    fn a_walk_that_runs_out_of_lanes_leaves_the_ring_at_the_tip() {
        let shape = ring_shape(&BLOCKS[0], 2, 0, &right_hand(), true, &cross_mouths());
        let w = walk(&shape, 1, 90.0, 0.0);
        assert!(w.left_at_tip, "east's outer lane reaches the front tip with north still ahead: it leaves there");
        assert_eq!(lanes_of(&shape, &w), [(1, true)]);
        let lane = shape.lanes.iter().find(|l| l.k == 1 && l.ahead).unwrap();
        assert_eq!(w.points.last(), lane.centre.last(), "the walk ends at the front tip");
    }

    #[test]
    fn the_outermost_lane_leaves_at_a_tip_only_for_the_first_street_after_it() {
        let shape = ring_shape(&BLOCKS[0], 2, 0, &right_hand(), true, &cross_mouths());
        // Traffic goes north, west, south, east, and the front tip lies between east and north, the back tip between west and south.
        let go = |from: f64, to: f64| spiral_walk(&shape, 1, (from, -1_750.0), (to, 1_750.0));
        assert!(go(0.0, 90.0).is_none(), "north's outer lane to east would have to cross the ring");
        assert!(go(90.0, 180.0).is_none(), "east's outer lane to south likewise");
        let w = go(0.0, 180.0).expect("north's outer lane leaves at the back tip for south, the first street after it");
        assert!(w.left_at_tip);
        assert!(go(90.0, 0.0).is_some_and(|w| w.left_at_tip), "and east's at the front tip for north");
    }

    #[test]
    fn three_lanes_step_out_twice() {
        let shape = ring_shape(&BLOCKS[0], 3, 0, &right_hand(), true, &cross_mouths());
        assert_eq!(
            lanes_of(&shape, &walk(&shape, 0, 90.0, 180.0)),
            [(0, true), (1, false), (2, true)],
            "east to south on the inner lane: round the front tip, the back tip, and out"
        );
    }

    #[test]
    fn no_walk_crosses_a_divider_for_any_block_lane_count_street_pair_or_side_of_the_road() {
        let mut walks = 0;
        for right_hand in [true, false] {
            // `Turbo::axis_deg` turns the axis the other way for left-hand traffic.
            let axes = Axes::new(if right_hand { 57.5 } else { 302.5 }, right_hand);
            // Traffic keeps right or left, so the entering side of a street is the left of its axis for right-hand traffic.
            let sign = if right_hand { 1.0 } else { -1.0 };
            for b in &BLOCKS {
                for ring_lanes in [2, 3] {
                    let shape = ring_shape(b, ring_lanes, 0, &axes, right_hand, &cross_mouths());
                    assert!(!shape.dividers.is_empty());
                    for from in [0.0, 90.0, 180.0, 270.0] {
                        for to in [0.0, 90.0, 180.0, 270.0] {
                            if from == to {
                                continue;
                            }
                            for k in 0..ring_lanes {
                                let (enter, leave) = ((from, -1_750.0 * sign), (to, 1_750.0 * sign));
                                // A walk that cannot reach its street without crossing the ring is None (see the test after this one).
                                let Some(w) = spiral_walk(&shape, k, enter, leave) else { continue };
                                let mut path = vec![at(enter.0, enter.1, 60_000.0)];
                                path.extend(&w.points);
                                path.push(at(leave.0, leave.1, 60_000.0));
                                for d in &shape.dividers {
                                    assert!(
                                        !polylines_cross(&path, d),
                                        "{} {ring_lanes} lanes, right-hand {right_hand}: lane {k} from {from} to {to} crosses a divider",
                                        b.id
                                    );
                                }
                                walks += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            walks, 384,
            "480 (street, street, lane) walks, 96 of them are an outermost lane that has to leave the ring for a street that is not the first after the tip"
        );
    }

    #[test]
    fn every_block_lane_count_cycle_track_axis_and_side_draws_finite_points() {
        for right_hand in [true, false] {
            for axis in [0.0, 57.5, 155.0, 302.5] {
                for b in &BLOCKS {
                    for lanes in [2, 3] {
                        for cycle in [0, 3_000] {
                            let s = ring_shape(b, lanes, cycle, &Axes::new(axis, right_hand), right_hand, &cross_mouths());
                            assert_eq!(s.lanes.len(), 2 * lanes);
                            let all = s
                                .lanes
                                .iter()
                                .flat_map(|l| l.inner.iter().chain(&l.outer).chain(&l.centre))
                                .chain(s.dividers.iter().flatten())
                                .chain(&s.island)
                                .chain(&s.road)
                                .chain(&s.outer);
                            assert!(all.into_iter().all(|p| p.0.is_finite() && p.1.is_finite()), "{} {lanes} {cycle} {axis} {right_hand}", b.id);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_island_the_road_and_the_cycle_track_are_stepped_outlines_of_the_blocks_radii() {
        let axes = right_hand();
        let s = ring_shape(&BLOCKS[0], 2, 2_000, &axes, true, &[]);
        let across = |pts: &[P]| pts.iter().fold(f64::MIN, |m, &p| m.max(aside(p, &axes).abs()));
        let reach = |pts: &[P]| pts.iter().fold(f64::MIN, |m, &p| m.max(along(p, &axes)));
        assert!(near(across(&s.island), 10_500.0) && near(reach(&s.island), 10_500.0 + 5_750.0 / 2.0), "the island is R1 shifted by dv");
        assert!(near(across(&s.road), 21_150.0) && near(reach(&s.road), 21_150.0 + 5_050.0 / 2.0), "the road is R4 shifted by du");
        assert!(near(across(&s.outer), 23_150.0), "the cycle track is 2 m outside the road");
        let third = ring_shape(&BLOCKS[0], 3, 0, &axes, true, &[]);
        assert!(near(across(&third.road), 26_450.0), "the third lane: R4 + 300 mm + the lane");
        assert_eq!(third.dividers.len(), 4, "two dividers on each half");
    }

    #[test]
    fn an_arm_meets_the_outline_at_its_farthest_hit() {
        // a ring 10 m across the axis and 12 m along it (shift 4 m), its axis pointing north, right-hand traffic: the ahead half is east
        let ring = ring_outline(&Axes::new(0.0, true), Edge { radius_mm: 10_000, shift_mm: 4_000 }, true);
        // (Just off the axis: a line exactly on it runs through the tip vertices, where the sign of the rounding decides which side it meets.)
        assert!((mouth_on(&ring, 0.0, 1.0).unwrap() - 12_000.0).abs() < 1.0, "an arm up the axis meets the front tip");
        assert!((mouth_on(&ring, 180.0, 1.0).unwrap() - 12_000.0).abs() < 1.0, "and one down it the back tip");
        let east = (10_000.0_f64.powi(2) - 2_000.0_f64.powi(2)).sqrt();
        // 48 chords of a 10 m half circle fall short of it by up to 5.3 mm.
        assert!(
            (mouth_on(&ring, 90.0, 0.0).unwrap() - east).abs() < 10.0,
            "an arm across it meets the half circle, whose centre is 2 m off the middle: {east}"
        );
        let t = mouth_on(&ring, 0.0, 6_000.0).unwrap();
        assert!(t > 8_000.0 && t < 12_000.0, "off the axis it meets the arc a little short of the tip: {t}");
        assert_eq!(mouth_on(&ring, 0.0, 10_500.0), None, "a line beside the ring misses it");
    }
}
