//! The street's transit stops on an arm, as polygons: the platform a stop stands on, the lane where the bus dwells,
//! the run of a near-side stop's bus to the stop line and the post. Pure functions on millimetres; which stops an arm
//! has, and on which pieces, is `street::model::stop_end`'s, and this only draws them, at true scale, cut to the part
//! of the arm that is drawn.

use serde::Serialize;
use serde_json::Value;

use crate::junction::geometry::{P, at, l as line, m, poly, strip};
use crate::junction::model::{Piece, Profile};
use crate::junction::read_model::ArmLayout;
use crate::shared::catalogue::KINDS;
use crate::street::model::{EndStop, StopKind};

/// One stop on an arm, as the plan draws and the checks read it.
#[derive(Debug, Serialize)]
pub struct StopView {
    /// The stop's index in its street's `stops`.
    pub index: usize,
    pub kind: StopKind,
    pub shelter: bool,
    pub name: Option<String>,
    /// Whether the bus's half runs toward the junction (a near-side stop).
    pub enters: bool,
    /// The entering lane the bus uses (driver's left to right), `None` for a bus lane or a lane the junction does not
    /// enter.
    pub lane: Option<usize>,
    pub in_bus_lane: bool,
    /// Millimetres from the junction's centre to the stop's near edge.
    pub near_mm: i32,
    /// How far past the crossing's far edge (the mouth with no crossing) the near edge stands; negative inside it.
    pub gap_mm: i32,
    /// The platform, the bus's dwell, the bus's run to the stop line and the post; each only where it lies on the
    /// drawn arm.
    pub pad: Option<Vec<Value>>,
    pub dwell: Option<Vec<Value>>,
    pub run: Option<Vec<Value>>,
    pub post: Option<P>,
    /// The stop stands too near the junction: its near edge is less than `STOP_CLEAR_MM` past what reaches out from
    /// the centre.
    pub bad: bool,
}

/// The entering lane (driver's left to right) that the section's piece `piece` is: the number of entering `travel`
/// pieces after it. `None` unless it is a `travel` piece with direction `toward`.
pub fn lane_of(profile: &Profile, piece: usize) -> Option<usize> {
    let entering = |p: &Piece| KINDS[p.kind].id == "travel" && p.direction == Some("toward");
    let p = profile.pieces.get(piece)?;
    entering(p).then(|| profile.pieces[piece + 1..].iter().filter(|q| entering(q)).count())
}

/// The stop `stop` on the arm `l` (shifted `off` sideways), whose bus stops at `stop_line_t` when it goes on, drawn
/// as far as `len` from the centre. A part inside the mouth or past `len` is cut. `gap_mm` and `bad` are the
/// caller's to fill: they need the crossing.
pub fn draw(l: &ArmLayout, off: f64, stop: &EndStop, stop_line_t: f64, len: f64) -> StopView {
    let lat = |x: i32| off + f64::from(x) - f64::from(l.prof.row_mm) / 2.0;
    let (t0, t1) = (f64::from(stop.near_mm).max(l.mouth), f64::from(stop.far_mm).min(len));
    let drawn = t0 < t1;
    let span = |ix: &[usize]| {
        let ps: Vec<&Piece> = ix.iter().filter_map(|&i| l.prof.pieces.get(i)).collect();
        let lo = ps.iter().map(|p| p.x_mm).min()?;
        let hi = ps.iter().map(|p| p.x_mm + p.width_mm).max()?;
        Some((lat(lo), lat(hi)))
    };
    let pad_span = span(&stop.pad);
    let lane_span = stop.lane.and_then(|i| span(&[i]));
    let rect = |s: Option<(f64, f64)>| s.filter(|_| drawn).map(|(a, b)| poly(&strip(l.bearing, a, b, t0, t1)));
    // A near-side bus goes on from the stop to the stop line, down the middle of its lane.
    let run = lane_span.filter(|_| stop.enters).and_then(|(a, b)| {
        let (from, to) = (stop_line_t.max(l.mouth), f64::from(stop.near_mm));
        let c = (a + b) / 2.0;
        (to > from && to <= len).then(|| vec![m(at(l.bearing, c, to)), line(at(l.bearing, c, from))])
    });
    let near_drawn = f64::from(stop.near_mm) >= l.mouth && f64::from(stop.near_mm) <= len;
    let post = pad_span.or(lane_span).filter(|_| near_drawn).map(|(a, b)| at(l.bearing, (a + b) / 2.0, f64::from(stop.near_mm)));
    StopView {
        index: stop.index,
        kind: stop.kind,
        shelter: stop.shelter,
        name: stop.name.clone(),
        enters: stop.enters,
        lane: stop.lane.and_then(|i| lane_of(&l.prof, i)),
        in_bus_lane: stop.lane.and_then(|i| l.prof.pieces.get(i)).is_some_and(|p| KINDS[p.kind].id == "bus"),
        near_mm: stop.near_mm,
        gap_mm: 0,
        pad: rect(pad_span),
        dwell: rect(lane_span),
        run,
        post,
        bad: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{Piece, unlabelled};
    use crate::shared::catalogue::{Side, kind_index};
    use serde_json::Value;

    const ROW: i32 = 18_000;
    const MOUTH: f64 = 4_000.0;
    const LEN: f64 = 36_000.0;

    fn kind(id: &str) -> usize {
        kind_index(id).unwrap()
    }

    /// An arm looking outward, traffic on the right: its left half enters. A sidewalk, two entering lanes, a leaving
    /// lane, a bus lane that leaves and a sidewalk; the pieces are numbered in that order.
    fn profile() -> Profile {
        let rows: [(&str, i32, Option<&'static str>); 6] = [
            ("sidewalk", 3_000, None),
            ("travel", 3_000, Some("toward")),
            ("travel", 3_000, Some("toward")),
            ("travel", 3_000, Some("away")),
            ("bus", 3_000, Some("toward")),
            ("sidewalk", 3_000, None),
        ];
        let mut x = 0;
        let pieces: Vec<Piece> = rows
            .iter()
            .map(|&(k, w, d)| {
                let p = Piece { kind: kind(k), material: "asphalt", x_mm: x, width_mm: w, direction: d };
                x += w;
                p
            })
            .collect();
        Profile {
            name: unlabelled(),
            row_mm: ROW,
            pieces,
            road_l: 3_000,
            road_r: 15_000,
            enter_x: vec![7_500, 4_500],
            leave_x: vec![10_500],
            enter_span: (3_000, 9_000),
            leave_span: (9_000, 12_000),
            park: [0, 0],
            edge_kind: [None, None],
        }
    }

    fn layout() -> ArmLayout {
        ArmLayout {
            bearing: 0.0,
            prof: profile(),
            cl: -6_000.0,
            cr: 6_000.0,
            pl: -9_000.0,
            pr: 9_000.0,
            mouth: MOUTH,
            strip0: MOUTH,
            entry: None,
            lane_mm: None,
        }
    }

    fn stop(near: i32, far: i32, enters: bool, lane: Option<usize>, pad: Vec<usize>) -> EndStop {
        EndStop { index: 0, side: Side::Left, near_mm: near, far_mm: far, kind: StopKind::Curb, shelter: false, name: None, enters, lane, pad }
    }

    /// The distances out of a polygon's lowest and highest points (the bearing is north, so y is minus the distance).
    fn along(poly: &[Value]) -> (f64, f64) {
        let ys: Vec<f64> = poly.iter().filter(|c| c[0] != "Z").map(|c| -c[2].as_f64().unwrap()).collect();
        (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max))
    }

    /// The lateral positions of a polygon's leftmost and rightmost points.
    fn across(poly: &[Value]) -> (f64, f64) {
        let xs: Vec<f64> = poly.iter().filter(|c| c[0] != "Z").map(|c| c[1].as_f64().unwrap()).collect();
        (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max))
    }

    #[test]
    fn lane_of_counts_entering_travel_pieces_to_its_right() {
        let mut p = profile();
        // a third entering lane, to the right of the other two
        p.pieces[3].direction = Some("toward");
        assert_eq!([lane_of(&p, 3), lane_of(&p, 2), lane_of(&p, 1)], [Some(0), Some(1), Some(2)], "the piece furthest right is the driver's first");
    }

    #[test]
    fn a_bus_piece_and_a_leaving_lane_have_no_lane() {
        let p = profile();
        assert_eq!(lane_of(&p, 4), None, "a bus piece enters the junction but has no lane");
        assert_eq!(lane_of(&p, 3), None, "a leaving lane");
        assert_eq!(lane_of(&p, 0), None, "a sidewalk");
        assert_eq!(lane_of(&p, 1), Some(1));
        assert_eq!(lane_of(&p, 2), Some(0));
    }

    #[test]
    fn a_near_side_stop_draws_pad_dwell_post_and_a_run_to_the_stop_line() {
        let l = layout();
        // on the first entering piece (index 1), standing on the sidewalk (index 0), 14 m to 26 m out
        let v = draw(&l, 0.0, &stop(14_000, 26_000, true, Some(1), vec![0]), 7_000.0, LEN);
        let pad = v.pad.as_ref().expect("a pad");
        assert_eq!(along(pad), (14_000.0, 26_000.0));
        assert_eq!(across(pad), (-9_000.0, -6_000.0), "the sidewalk's span");
        let dwell = v.dwell.as_ref().expect("a dwell");
        assert_eq!(along(dwell), (14_000.0, 26_000.0));
        assert_eq!(across(dwell), (-6_000.0, -3_000.0), "the lane's span");
        let run = v.run.as_ref().expect("a run");
        assert_eq!(run.len(), 2, "an open line");
        assert_eq!(along(run), (7_000.0, 14_000.0), "from the stop's near edge to the stop line");
        assert_eq!(across(run), (-4_500.0, -4_500.0), "down the lane's centre");
        assert_eq!(v.post, Some((-7_500.0, -14_000.0)), "on the platform, at its near end");
        assert_eq!((v.lane, v.in_bus_lane, v.enters, v.near_mm), (Some(1), false, true, 14_000));
    }

    #[test]
    fn a_far_side_stop_has_no_run() {
        let l = layout();
        let v = draw(&l, 0.0, &stop(14_000, 26_000, false, Some(3), vec![5]), 7_000.0, LEN);
        assert!(v.run.is_none());
        assert!(v.pad.is_some() && v.dwell.is_some() && v.post.is_some());
    }

    #[test]
    fn a_bus_lane_stop_is_in_the_bus_lane_and_has_no_entering_lane() {
        let l = layout();
        let v = draw(&l, 0.0, &stop(14_000, 26_000, true, Some(4), vec![5]), 7_000.0, LEN);
        assert_eq!((v.lane, v.in_bus_lane), (None, true));
        assert!(v.run.is_some(), "the bus still runs on to the stop line");
    }

    #[test]
    fn a_stop_with_no_lane_draws_a_pad_only() {
        let l = layout();
        let v = draw(&l, 0.0, &stop(14_000, 26_000, true, None, vec![0]), 7_000.0, LEN);
        assert!(v.pad.is_some() && v.dwell.is_none() && v.run.is_none());
    }

    #[test]
    fn a_stop_is_clipped_to_the_arm() {
        let l = layout();
        // from inside the mouth to past the end of the arm
        let v = draw(&l, 0.0, &stop(1_000, 40_000, true, Some(1), vec![0]), 3_000.0, LEN);
        for poly in [v.pad.as_ref().unwrap(), v.dwell.as_ref().unwrap()] {
            assert_eq!(along(poly), (MOUTH, LEN));
        }
        assert!(v.run.is_none(), "the run would lie inside the mouth");
        assert!(v.post.is_none(), "the post would stand inside the mouth");
        // wholly inside the mouth, or wholly past the end: nothing is drawn
        for (near, far) in [(500, 3_500), (37_000, 45_000)] {
            let v = draw(&l, 0.0, &stop(near, far, true, Some(1), vec![0]), 7_000.0, LEN);
            assert!(v.pad.is_none() && v.dwell.is_none() && v.run.is_none() && v.post.is_none(), "{near}..{far}");
        }
    }

    #[test]
    fn the_arm_s_offset_moves_the_stop_sideways() {
        let l = layout();
        let v = draw(&l, 1_000.0, &stop(14_000, 26_000, false, Some(1), vec![0]), 7_000.0, LEN);
        assert_eq!(across(v.dwell.as_ref().unwrap()), (-5_000.0, -2_000.0));
    }
}
