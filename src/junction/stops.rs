//! The street's transit stops on an arm, as polygons: the platform a stop stands on, the lane where the bus dwells,
//! the run of a near-side stop's bus to the stop line and the post. Pure functions on millimetres; which stops an arm
//! has, and on which pieces, is `street::model::stop_end`'s, and this only draws them, at true scale, cut to the part
//! of the arm that is drawn.

use serde::Serialize;
use serde_json::Value;

use crate::junction::geometry::{P, at, l as line, m, poly, strip};
use crate::junction::model::{Piece, Profile};
use crate::junction::read_model::ArmLayout;
use crate::shared::catalogue::{KINDS, is_roadway};
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

/// The stop `stop` on the arm `l` (shifted `off` sideways), whose bus stops at `stop_line` when it goes on (none
/// where no line is drawn: the run then ends at the mouth), drawn as far as `len` from the centre; `cut(i)` is where
/// the strip of the section's piece `i` ends. A part inside the mouth or past where its piece ends is cut. `gap_mm` and `bad` are the
/// caller's to fill: they need the crossing.
pub fn draw(l: &ArmLayout, off: f64, stop: &EndStop, stop_line: Option<f64>, cut: &dyn Fn(usize) -> f64, len: f64) -> StopView {
    let lat = |x: i32| off + f64::from(x) - f64::from(l.prof.row_mm) / 2.0;
    let (near, far) = (f64::from(stop.near_mm), f64::from(stop.far_mm));
    let span = |ix: &[usize]| {
        let ps: Vec<&Piece> = ix.iter().filter_map(|&i| l.prof.pieces.get(i)).collect();
        let lo = ps.iter().map(|p| p.x_mm).min()?;
        let hi = ps.iter().map(|p| p.x_mm + p.width_mm).max()?;
        Some((lat(lo), lat(hi)))
    };
    // Where the pieces' strips end: the whole arm, or sooner for a roadway piece that moves across the boundary.
    let ends = |ix: &[usize]| ix.iter().filter(|&&i| l.prof.pieces.get(i).is_some_and(|p| is_roadway(p.kind))).fold(len, |e, &i| e.min(cut(i)));
    let pad_span = span(&stop.pad);
    let lane_span = stop.lane.and_then(|i| span(&[i]));
    // The platform begins at the mouth; the lane, like every roadway strip, where the strips begin.
    let rect = |s: Option<(f64, f64)>, from: f64, to: f64| {
        s.filter(|_| from.max(near) < to.min(far)).map(|(a, b)| poly(&strip(l.bearing, a, b, from.max(near), to.min(far))))
    };
    // A near-side bus goes on from the stop to the stop line, down the middle of its lane; with no stop line, to the mouth.
    let run = lane_span.filter(|_| stop.enters).and_then(|(a, b)| {
        let from = stop_line.unwrap_or(l.mouth).max(l.mouth);
        let c = (a + b) / 2.0;
        (near > from && near <= len).then(|| vec![m(at(l.bearing, c, near)), line(at(l.bearing, c, from))])
    });
    let near_drawn = near >= l.mouth && near <= len;
    let post = pad_span.or(lane_span).filter(|_| near_drawn).map(|(a, b)| at(l.bearing, (a + b) / 2.0, near));
    let lane_end = stop.lane.map_or(len, |i| ends(&[i]));
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
        pad: rect(pad_span, l.mouth, ends(&stop.pad)),
        dwell: rect(lane_span, l.strip0, lane_end),
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
        let v = draw(&l, 0.0, &stop(14_000, 26_000, true, Some(1), vec![0]), Some(7_000.0), &|_| LEN, LEN);
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
        let v = draw(&l, 0.0, &stop(14_000, 26_000, false, Some(3), vec![5]), Some(7_000.0), &|_| LEN, LEN);
        assert!(v.run.is_none());
        assert!(v.pad.is_some() && v.dwell.is_some() && v.post.is_some());
    }

    #[test]
    fn a_bus_lane_stop_is_in_the_bus_lane_and_has_no_entering_lane() {
        let l = layout();
        let v = draw(&l, 0.0, &stop(14_000, 26_000, true, Some(4), vec![5]), Some(7_000.0), &|_| LEN, LEN);
        assert_eq!((v.lane, v.in_bus_lane), (None, true));
        assert!(v.run.is_some(), "the bus still runs on to the stop line");
    }

    #[test]
    fn a_stop_with_no_lane_draws_a_pad_only() {
        let l = layout();
        let v = draw(&l, 0.0, &stop(14_000, 26_000, true, None, vec![0]), Some(7_000.0), &|_| LEN, LEN);
        assert!(v.pad.is_some() && v.dwell.is_none() && v.run.is_none());
    }

    #[test]
    fn a_stop_is_clipped_to_the_arm() {
        let l = layout();
        // from inside the mouth to past the end of the arm
        let v = draw(&l, 0.0, &stop(1_000, 40_000, true, Some(1), vec![0]), Some(3_000.0), &|_| LEN, LEN);
        for poly in [v.pad.as_ref().unwrap(), v.dwell.as_ref().unwrap()] {
            assert_eq!(along(poly), (MOUTH, LEN));
        }
        assert!(v.run.is_none(), "the run would lie inside the mouth");
        assert!(v.post.is_none(), "the post would stand inside the mouth");
        // wholly inside the mouth, or wholly past the end: nothing is drawn
        for (near, far) in [(500, 3_500), (37_000, 45_000)] {
            let v = draw(&l, 0.0, &stop(near, far, true, Some(1), vec![0]), Some(7_000.0), &|_| LEN, LEN);
            assert!(v.pad.is_none() && v.dwell.is_none() && v.run.is_none() && v.post.is_none(), "{near}..{far}");
        }
    }

    #[test]
    fn the_arm_s_offset_moves_the_stop_sideways() {
        let l = layout();
        let v = draw(&l, 1_000.0, &stop(14_000, 26_000, false, Some(1), vec![0]), Some(7_000.0), &|_| LEN, LEN);
        assert_eq!(across(v.dwell.as_ref().unwrap()), (-5_000.0, -2_000.0));
    }

    #[test]
    fn the_dwell_and_a_roadway_pad_stop_where_their_piece_is_cut() {
        let l = layout();
        // the entering piece 1 moves across the boundary: its strip ends at the taper's start, 12 m out
        let cut = |pi: usize| if pi == 1 { 12_000.0 } else { LEN };
        let v = draw(&l, 0.0, &stop(10_000, 20_000, true, Some(1), vec![0]), Some(7_000.0), &cut, LEN);
        assert_eq!(along(v.dwell.as_ref().unwrap()), (10_000.0, 12_000.0));
        assert_eq!(along(v.pad.as_ref().unwrap()), (10_000.0, 20_000.0), "a sidewalk goes on");
        let v = draw(&l, 0.0, &stop(10_000, 20_000, true, Some(1), vec![1]), Some(7_000.0), &cut, LEN);
        assert_eq!(along(v.pad.as_ref().unwrap()), (10_000.0, 12_000.0), "a pad that is roadway ends with it");
        let v = draw(&l, 0.0, &stop(13_000, 20_000, true, Some(1), vec![0]), Some(7_000.0), &cut, LEN);
        assert!(v.dwell.is_none(), "wholly past where its piece ends");
    }

    #[test]
    fn the_dwell_begins_where_the_roadway_strips_do() {
        let mut l = layout();
        l.strip0 = 5_000.0;
        let v = draw(&l, 0.0, &stop(4_500, 20_000, true, Some(1), vec![0]), Some(7_000.0), &|_| LEN, LEN);
        assert_eq!(along(v.dwell.as_ref().unwrap()), (5_000.0, 20_000.0));
        assert_eq!(along(v.pad.as_ref().unwrap()), (4_500.0, 20_000.0), "a platform begins at the stop or the mouth, whichever is further out");
    }

    #[test]
    fn without_a_stop_line_the_run_ends_at_the_mouth() {
        let l = layout();
        let v = draw(&l, 0.0, &stop(14_000, 26_000, true, Some(1), vec![0]), None, &|_| LEN, LEN);
        assert_eq!(along(v.run.as_ref().unwrap()), (MOUTH, 14_000.0));
    }

    /// The stops of a street at one end, drawn on the arm that ends there: the dwell is on its lane's piece and a bus
    /// that enters stands on a piece that enters, whichever end and whichever side of the road traffic keeps.
    #[test]
    fn a_stop_stands_on_its_lane_at_either_end_and_in_either_hand() {
        use crate::junction::model::profile_of;
        use crate::street::model::{Stop, Street, ZoneEnd, ZoneFrame};
        for traffic in [Side::Right, Side::Left] {
            for first_end in [true, false] {
                let mut st = Street::sample(0, traffic);
                st.length_mm = Some(60_000);
                st.from_end = ZoneEnd::First;
                for side in [Side::Left, Side::Right] {
                    st.stops = vec![Stop { side, from_mm: 25_000, length_mm: 12_000, kind: StopKind::Curb, shelter: false, name: None }];
                    let found = st.end_stops(first_end, Some(ZoneFrame { length_mm: 60_000, from_end: ZoneEnd::First }));
                    let e = &found[0];
                    let section = if first_end { st.at_end(true) } else { st.at_end(false).reversed() };
                    let mut l = layout();
                    l.prof = profile_of(&section, if traffic == Side::Right { 0 } else { 3 });
                    let lane = e.lane.expect("a lane");
                    let piece = &l.prof.pieces[lane];
                    assert_eq!(piece.direction == Some("toward"), e.enters, "{traffic:?} first_end {first_end} {side:?}");
                    let v = draw(&l, 0.0, e, Some(7_000.0), &|_| LEN, LEN);
                    let half = f64::from(l.prof.row_mm) / 2.0;
                    let want = (f64::from(piece.x_mm) - half, f64::from(piece.x_mm + piece.width_mm) - half);
                    assert!(v.dwell.is_some(), "no dwell: {e:?} {:?}", l.prof.pieces);
                    assert_eq!(across(v.dwell.as_ref().unwrap()), want, "{traffic:?} first_end {first_end} {side:?}");
                    assert_eq!(v.enters, e.enters);
                }
            }
        }
    }
}
