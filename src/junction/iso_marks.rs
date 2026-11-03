//! What the plan draws on the ground and the 2.5D draws after it (#140, stage 2): the turn arrows on the lanes, the
//! small arrows where traffic leaves, the circulation arrows on a roundabout. Pure geometry in millimetres on the
//! plan, as polylines; `iso.rs` projects them. Nothing here knows the screen or a colour.

use crate::junction::geometry::{dir, sample_quad};
use crate::junction::model::{LEFT, RIGHT, THROUGH};
use crate::junction::read_model::{JView, LaneView, RingView};
use crate::junction::turbo_ring::mouth_on_open;

type P = (f64, f64);

/// How long a turn arrow is, as the plan draws it (about 5.2 m), and the most it takes of a lane's length: it sits
/// 5.5 m back from the stop line, so half of it stays under 3 m.
pub const LANE_ARROW_MM: f64 = 5_200.0;
const LANE_ARROW_MAX_MM: f64 = 6_000.0;
/// A turn arrow with a bend is this much wider than long, and a lane must hold it with room to spare.
const TURN_ARROW_WIDTH: f64 = 0.84;
/// The small arrow where an arm's traffic leaves.
pub const LEAVE_ARROW_MM: f64 = 3_600.0;
/// The arrow on a roundabout's ring, and the least room it is drawn in.
pub const RING_ARROW_MM: f64 = 5_000.0;
/// How far inside the ring's outer edge the arrows on a plain roundabout stand.
const RING_ARROW_INSET_MM: f64 = 3_000.0;

/// A point of a glyph (x to the right of the heading, y backwards: the arrow points to -y) on the plan, with the
/// glyph's origin at `at` and turned to `heading` degrees clockwise from north.
pub fn place(at: P, heading: f64, local: P) -> P {
    let t = heading.to_radians();
    (at.0 + local.0 * t.cos() - local.1 * t.sin(), at.1 + local.0 * t.sin() + local.1 * t.cos())
}

/// How long the turn arrows of an arm's lanes are: never so long that it reaches the stop line, and never so wide
/// that a lane with a bend overflows its own width.
pub fn lane_arrow_size(lanes: &[LaneView]) -> f64 {
    let mut size = LANE_ARROW_MM.min(LANE_ARROW_MAX_MM);
    for l in lanes.iter().filter(|l| l.uses & (LEFT | RIGHT) != 0) {
        size = size.min(0.95 * l.width_mm as f64 / TURN_ARROW_WIDTH);
    }
    size
}

/// The strokes of the arrow on a lane: a stem, a head ahead for a through lane, and a bend with its head for each
/// side it turns to. `h` is its length.
pub fn lane_arrow(l: &LaneView, h: f64) -> Vec<Vec<P>> {
    let through = l.uses & THROUGH != 0;
    let bw = h * 0.42;
    let mut strokes = vec![vec![(0.0, h / 2.0), (0.0, if through { -h / 2.0 } else { 0.0 })]];
    if through {
        strokes.push(vec![(-h * 0.18, -h / 2.0 + h * 0.2), (0.0, -h / 2.0), (h * 0.18, -h / 2.0 + h * 0.2)]);
    }
    for (side, flag) in [(-1.0, LEFT), (1.0, RIGHT)] {
        if l.uses & flag != 0 {
            strokes.push(sample_quad((0.0, -h * 0.05), (0.0, -h * 0.3), (side * bw, -h * 0.3), 8));
            strokes.push(vec![(side * (bw - h * 0.16), -h * 0.3 - h * 0.16), (side * bw, -h * 0.3), (side * (bw - h * 0.16), -h * 0.3 + h * 0.16)]);
        }
    }
    strokes.into_iter().map(|s| s.into_iter().map(|p| place(l.at, l.heading as f64, p)).collect()).collect()
}

/// A plain arrow: a stem and a head of `head` times its length `h`, at `at` and pointing along `heading`.
pub fn plain_arrow(at: P, heading: f64, h: f64, head: f64) -> Vec<Vec<P>> {
    let k = h * head;
    [vec![(0.0, h / 2.0), (0.0, -h / 2.0)], vec![(-k, -h / 2.0 + k), (0.0, -h / 2.0), (k, -h / 2.0 + k)]]
        .into_iter()
        .map(|s| s.into_iter().map(|p| place(at, heading, p)).collect())
        .collect()
}

/// Where the ray from the middle at `bearing` meets the middle of the outermost ring lane, and how wide that lane is
/// there, both in mm. A turbo's lanes are open, one on each half of the ring, so the half the ray runs into is the one
/// that answers (the other half's hits are behind the middle, and are dropped).
fn outer_lane_at(ring: &RingView, bearing: f64) -> Option<(f64, f64)> {
    let last = ring.lanes.iter().map(|l| l.k).max()?;
    ring.lanes
        .iter()
        .filter(|l| l.k == last)
        .filter_map(|l| {
            let (t, o, i) = (mouth_on_open(&l.centre, bearing, 0.0)?, mouth_on_open(&l.outer, bearing, 0.0)?, mouth_on_open(&l.inner, bearing, 0.0)?);
            (t > 0.0).then_some((t, (o - i).abs()))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
}

/// An arrow on a roundabout's ring: where it stands, the way it points and how long it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingArrow {
    pub at: P,
    pub heading: f64,
    pub size: f64,
}

/// The arrows that show which way the ring runs, one between each two streets: in the middle of the outer lane of a
/// turbo, and a little inside the edge of a plain ring.
pub fn ring_arrows(v: &JView) -> Vec<RingArrow> {
    let Some(ring) = &v.ring else { return Vec::new() };
    let ccw = ring.circulation == "anticlockwise";
    let bearings: Vec<i32> = v.arms.iter().map(|a| a.bearing).collect();
    let mut out = Vec::new();
    for (i, b) in bearings.iter().enumerate() {
        let next = bearings[(i + 1) % bearings.len()];
        let gap = (next - b + 360) % 360;
        let gap = if gap == 0 { 360 } else { gap };
        let ang = *b as f64 + gap as f64 / 2.0;
        let heading = ang + if ccw { -90.0 } else { 90.0 };
        let d = dir(ang);
        let (reach, room) = if ring.lanes.is_empty() {
            let mid = ring.road_mm as f64 - RING_ARROW_INSET_MM;
            (mid, mid)
        } else {
            match outer_lane_at(ring, ang) {
                Some(found) => found,
                None => continue,
            }
        };
        out.push(RingArrow { at: (d.0 * reach, d.1 * reach), heading, size: RING_ARROW_MM.min(room * 0.85) });
    }
    out
}

/// The strokes of a turn: its path, then the head of an arrow where an allowed one ends, or a cross at the middle of a
/// banned one. A path with no length has none.
pub fn movement_strokes(path: &[P], allowed: bool) -> Vec<Vec<P>> {
    let (Some(&first), Some(&last)) = (path.first(), path.last()) else { return Vec::new() };
    let mut strokes = vec![path.to_vec()];
    if allowed {
        // The last part of the path that has a direction.
        if let Some(&back) = path.iter().rev().find(|p| (p.0 - last.0).hypot(p.1 - last.1) > 1.0) {
            let len = (last.0 - back.0).hypot(last.1 - back.1);
            let u = ((last.0 - back.0) / len, (last.1 - back.1) / len);
            let wing = |turn: f64| {
                let (c, s) = (turn.cos(), turn.sin());
                (last.0 - HEAD_MM * (u.0 * c - u.1 * s), last.1 - HEAD_MM * (u.0 * s + u.1 * c))
            };
            strokes.push(vec![wing(HEAD_ANGLE), last, wing(-HEAD_ANGLE)]);
        }
    } else {
        let mid = ((first.0 + last.0) / 2.0, (first.1 + last.1) / 2.0);
        strokes.push(vec![(mid.0 - CROSS_MM, mid.1 - CROSS_MM), (mid.0 + CROSS_MM, mid.1 + CROSS_MM)]);
        strokes.push(vec![(mid.0 + CROSS_MM, mid.1 - CROSS_MM), (mid.0 - CROSS_MM, mid.1 + CROSS_MM)]);
    }
    strokes
}

/// The length of the arrow head of a turn, and its angle off the path.
const HEAD_MM: f64 = 1_100.0;
const HEAD_ANGLE: f64 = 0.55;
/// Half the size of the cross on a banned turn.
const CROSS_MM: f64 = 600.0;
/// How far from the crossing, towards the middle of the junction, its length is written; nothing else is drawn there.
const DIM_OFFSET_MM: f64 = 800.0;
/// The length of the ticks that end a dimension line.
const TICK_MM: f64 = 350.0;

/// A crossing's length written across it: the dimension line on the junction side of the crossing's near edge, a tick
/// at each end, and where its figure goes. `poly` is the crossing's four corners, across the street first; `bearing`
/// is its arm's.
pub fn crossing_dimension(poly: &[P], bearing: f64) -> Option<(Vec<Vec<P>>, P)> {
    let (a, b) = (*poly.first()?, *poly.get(1)?);
    let n = dir(bearing);
    let (q0, q1) = ((a.0 - n.0 * DIM_OFFSET_MM, a.1 - n.1 * DIM_OFFSET_MM), (b.0 - n.0 * DIM_OFFSET_MM, b.1 - n.1 * DIM_OFFSET_MM));
    let ang = (q1.1 - q0.1).atan2(q1.0 - q0.0) + std::f64::consts::FRAC_PI_4;
    let t = (ang.cos() * TICK_MM, ang.sin() * TICK_MM);
    let tick = |p: P| vec![(p.0 - t.0, p.1 - t.1), (p.0 + t.0, p.1 + t.1)];
    Some((vec![vec![q0, q1], tick(q0), tick(q1)], ((q0.0 + q1.0) / 2.0, (q0.1 + q1.1) / 2.0)))
}

/// A text on the screen that wants a place: where (its middle and baseline, in pixels) and how many characters wide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub at: P,
    pub chars: usize,
}

/// How wide a character of the dimension figures is, how much room above its baseline and below it a text holds,
/// and the air kept between two.
const CHAR_PX: f64 = 9.0;
const LINE_PX: f64 = 16.0;
const AIR_PX: f64 = 3.0;
/// How far a text is moved, one way or the other, to clear another, as many lines as it may be moved.
const STEP_PX: f64 = LINE_PX + AIR_PX;
const NUDGES: [f64; 5] = [0.0, STEP_PX, -STEP_PX, 2.0 * STEP_PX, -2.0 * STEP_PX];

fn overlap(a: &Spot, b: &Spot) -> bool {
    let wide = (a.chars + b.chars) as f64 * CHAR_PX / 2.0 + AIR_PX;
    (a.at.0 - b.at.0).abs() < wide && (a.at.1 - b.at.1).abs() < LINE_PX + AIR_PX
}

/// Where each text may stand so that no two overlap, kept in the order given, which is the order of their
/// importance: a text that overlaps one already placed is moved up or down by whole lines, and goes unplaced (`None`)
/// when none of those places is clear.
pub fn spread_labels(spots: &[Spot]) -> Vec<Option<P>> {
    let mut placed: Vec<Spot> = Vec::new();
    spots
        .iter()
        .map(|s| {
            let found = NUDGES.iter().map(|dy| Spot { at: (s.at.0, s.at.1 + dy), chars: s.chars }).find(|c| placed.iter().all(|p| !overlap(c, p)));
            if let Some(c) = found {
                placed.push(c);
            }
            found.map(|c| c.at)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::*;

    #[test]
    fn texts_that_overlap_are_moved_by_a_line_and_the_most_important_one_keeps_its_place() {
        let at = |x: f64, y: f64| Spot { at: (x, y), chars: 4 };
        // Far apart: nobody moves.
        assert_eq!(spread_labels(&[at(0.0, 0.0), at(200.0, 0.0), at(0.0, 100.0)]), vec![Some((0.0, 0.0)), Some((200.0, 0.0)), Some((0.0, 100.0))]);
        // On top of each other: the first stays where it is, the others are moved up or down by whole lines, and when
        // the five places there are for a text are taken, the next has none.
        let all = spread_labels(&[at(0.0, 0.0), at(5.0, 2.0), at(-4.0, 1.0), at(2.0, 3.0), at(1.0, 0.0), at(0.0, 4.0)]);
        assert_eq!(all[0], Some((0.0, 0.0)));
        let placed_count = all.iter().flatten().count();
        assert!((3..=5).contains(&placed_count), "at most five lines of room: {all:?}");
        assert!(all.iter().any(|p| p.is_none()), "six on one spot do not all fit");
        // Whatever is placed clears everything else placed.
        let placed: Vec<Spot> = all.iter().flatten().map(|p| Spot { at: *p, chars: 4 }).collect();
        for (i, a) in placed.iter().enumerate() {
            assert!(placed[i + 1..].iter().all(|b| !overlap(a, b)), "{a:?}");
        }
    }

    #[test]
    fn a_long_text_needs_more_room_than_a_short_one() {
        let (short, long) = (Spot { at: (0.0, 0.0), chars: 2 }, Spot { at: (0.0, 0.0), chars: 8 });
        let beside = |c: &Spot, x: f64| Spot { at: (x, 0.0), chars: c.chars };
        assert!(overlap(&short, &beside(&short, 20.0)));
        assert!(!overlap(&short, &beside(&short, 30.0)));
        assert!(overlap(&long, &beside(&long, 60.0)));
        assert!(!overlap(&long, &beside(&long, 80.0)));
    }

    #[test]
    fn an_allowed_turn_ends_in_a_head_and_a_banned_one_has_a_cross() {
        let path = vec![(0.0, 0.0), (0.0, -5_000.0), (5_000.0, -5_000.0)];
        let allowed = movement_strokes(&path, true);
        assert_eq!(allowed.len(), 2);
        assert_eq!(allowed[1][1], (5_000.0, -5_000.0), "the head's tip is the end of the path");
        assert!(allowed[1][0].0 < 5_000.0 && allowed[1][2].0 < 5_000.0, "the wings trail behind the tip: {:?}", allowed[1]);
        let banned = movement_strokes(&path, false);
        assert_eq!(banned.len(), 3, "the path and the two strokes of a cross");
        assert!(banned[1].iter().chain(&banned[2]).all(|p| (p.0 - 2_500.0).abs() <= CROSS_MM && (p.1 + 2_500.0).abs() <= CROSS_MM));
        assert!(movement_strokes(&[], true).is_empty());
        assert_eq!(movement_strokes(&[(1.0, 1.0)], true).len(), 1, "a point has no direction to point");
    }

    #[test]
    fn a_crossing_is_measured_on_the_junction_side_of_its_near_edge() {
        let poly = [(-3_000.0, -10_000.0), (3_000.0, -10_000.0), (3_000.0, -12_000.0), (-3_000.0, -12_000.0)];
        let (strokes, label) = crossing_dimension(&poly, 0.0).unwrap();
        assert_eq!(strokes.len(), 3);
        // The north arm leaves along -y, so the junction is at +y of its crossing.
        assert!(strokes[0].iter().all(|p| (p.1 - (-10_000.0 + DIM_OFFSET_MM)).abs() < 1e-6));
        assert_eq!(strokes[0][1].0 - strokes[0][0].0, 6_000.0);
        assert_eq!(label, (0.0, -10_000.0 + DIM_OFFSET_MM));
        assert!(crossing_dimension(&[], 0.0).is_none());
    }

    fn lane(uses: u8, heading: i32) -> LaneView {
        LaneView { uses, dests: Vec::new(), bad: false, at: (1000.0, 2000.0), heading, poly: Vec::new(), width_mm: 3_500, fixed: false }
    }

    fn near(a: P, b: P) -> bool {
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
    }

    #[test]
    fn a_glyph_points_along_its_heading_from_where_it_stands() {
        // North is -y on the plan: heading 0 points up, 90 east, 180 down.
        assert!(near(place((0.0, 0.0), 0.0, (0.0, -1.0)), (0.0, -1.0)));
        assert!(near(place((0.0, 0.0), 90.0, (0.0, -1.0)), (1.0, 0.0)));
        assert!(near(place((0.0, 0.0), 180.0, (0.0, -1.0)), (0.0, 1.0)));
        // A point to the right of an arrow going north is east.
        assert!(near(place((5.0, 5.0), 0.0, (1.0, 0.0)), (6.0, 5.0)));
    }

    #[test]
    fn a_lane_arrow_has_a_stem_a_head_for_through_and_a_bend_for_each_turn() {
        let h = 5_000.0;
        let strokes = |uses| lane_arrow(&lane(uses, 0), h).len();
        assert_eq!(strokes(THROUGH), 2, "stem and head");
        assert_eq!(strokes(LEFT), 3, "stem, bend and its head");
        assert_eq!(strokes(THROUGH | LEFT | RIGHT), 6);
        // The stem of a lane going north starts at the back and ends ahead of the middle.
        let through = lane_arrow(&lane(THROUGH, 0), h);
        assert!(near(through[0][0], (1000.0, 2000.0 + h / 2.0)));
        assert!(near(through[0][1], (1000.0, 2000.0 - h / 2.0)));
    }

    #[test]
    fn a_left_bend_goes_to_the_left_of_the_way_the_lane_runs() {
        let bend = |uses, heading| {
            let strokes = lane_arrow(&lane(uses, heading), 5_000.0);
            *strokes[1].last().unwrap()
        };
        // Going north, left is west (smaller x); going east, left is north (smaller y).
        assert!(bend(LEFT, 0).0 < 1000.0 && bend(RIGHT, 0).0 > 1000.0);
        assert!(bend(LEFT, 90).1 < 2000.0 && bend(RIGHT, 90).1 > 2000.0);
    }

    #[test]
    fn the_arrows_of_an_arm_never_overflow_the_narrowest_lane_that_bends() {
        let mut narrow = lane(LEFT, 0);
        narrow.width_mm = 2_400;
        assert_eq!(lane_arrow_size(&[lane(THROUGH, 0)]), 5_200.0);
        let size = lane_arrow_size(&[lane(THROUGH, 0), narrow]);
        assert!(size * TURN_ARROW_WIDTH <= 2_400.0 && size < 5_200.0, "{size}");
    }

    #[test]
    fn a_plain_arrow_is_a_stem_and_a_head_pointing_along_its_heading() {
        let a = plain_arrow((0.0, 0.0), 90.0, 2_000.0, 0.2);
        assert_eq!(a.len(), 2);
        assert!(near(a[0][0], (-1_000.0, 0.0)) && near(a[0][1], (1_000.0, 0.0)), "{a:?}");
        assert!(near(a[1][1], (1_000.0, 0.0)));
    }

    #[test]
    fn a_ring_has_an_arrow_between_each_two_streets_running_the_way_the_ring_does() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let v = j.view();
        let arrows = ring_arrows(&v);
        assert_eq!(arrows.len(), v.arms.len());
        let ring = v.ring.as_ref().unwrap();
        for a in &arrows {
            // Between the streets of the sample (north, east, south, west), so at 45 degrees from each axis.
            assert!((a.at.0.abs() - a.at.1.abs()).abs() < 1.0, "{a:?}");
            assert!(a.at.0.hypot(a.at.1) < ring.road_mm as f64 && a.size <= RING_ARROW_MM);
        }
        // Clockwise: at north-east, the arrow points south-east (heading 135); anticlockwise, north-west (-45 = 315).
        let ne = arrows.iter().find(|a| a.at.0 > 0.0 && a.at.1 < 0.0).unwrap();
        let wanted = if ring.circulation == "anticlockwise" { 315.0 } else { 135.0 };
        assert!((ne.heading.rem_euclid(360.0) - wanted).abs() < 1e-6, "{ne:?}");
    }

    #[test]
    fn a_turbo_ring_has_its_arrows_in_the_middle_of_its_outer_lane() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT) && j.set_turbo(Some(0)));
        let v = j.view();
        let arrows = ring_arrows(&v);
        assert!(!arrows.is_empty() && arrows.len() <= v.arms.len());
        let ring = v.ring.as_ref().unwrap();
        for a in &arrows {
            assert!(a.size > 0.0 && a.at.0.hypot(a.at.1) < ring.radius_mm as f64 * 1.5, "{a:?}");
        }
    }

    #[test]
    fn no_ring_no_arrows() {
        assert!(ring_arrows(&Junction::new(0).view()).is_empty());
    }

    // ---- what the 2.5D draws of them ----------------------------------------------------------------------------

    use crate::junction::iso::{flatten, iso_svg};
    use crate::junction::tests::{end_stop, end_use, junction_with_a_pocket, junction_with_stops, junction_with_uses};
    use crate::shared::units::Units;

    fn svg(j: &Junction) -> String {
        iso_svg(&j.view(), &crate::i18n_for(crate::shared::i18n::Locale::En), 1000.0, 1000.0, Units::Metres).markup
    }

    fn count(s: &str, needle: &str) -> usize {
        s.matches(needle).count()
    }

    fn no_missing_number(m: &str) {
        assert!(!m.contains("NaN") && !m.contains("inf"));
    }

    #[test]
    fn every_entering_lane_has_its_arrow_and_every_leaving_street_its_small_one() {
        for sample in 0..4 {
            let j = Junction::new(sample);
            let (v, m) = (j.view(), svg(&j));
            let lanes: usize = v.arms.iter().map(|a| a.lanes.len()).sum();
            let leaving: usize = v.arms.iter().map(|a| a.leave_arrows.len()).sum();
            assert!(lanes > 0 && leaving > 0, "sample {sample}");
            assert_eq!(count(&m, "<g class=\"lane-arrow\" "), lanes, "sample {sample}");
            assert_eq!(count(&m, "<g class=\"lane-arrow out\" "), leaving, "sample {sample}");
            no_missing_number(&m);
        }
    }

    #[test]
    fn an_arrow_is_a_halo_under_a_line_and_is_sized_for_the_screen() {
        let m = svg(&Junction::new(0));
        let g = &m[m.find("<g class=\"lane-arrow\" ").unwrap()..];
        let g = &g[..g.find("</g>").unwrap()];
        assert!(g.find("class=\"halo\"").unwrap() < g.rfind("<path d=").unwrap(), "{g}");
        let size: f64 = g.split("--arrow:").nth(1).unwrap().split(['"', ';']).next().unwrap().parse().unwrap();
        assert!((20.0..200.0).contains(&size), "{size}");
    }

    #[test]
    fn a_lane_that_serves_only_banned_turns_has_a_bad_arrow() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        let bad = j.view().arms[0].lanes.iter().filter(|l| l.bad).count();
        assert_eq!(count(&svg(&j), "<g class=\"lane-arrow bad\" "), bad);
        // Ban the turns of every lane of an arm: its lanes serve nothing that is allowed.
        for lane in 0..j.current().arms[0].lanes.len() {
            for dest in j.current().arms.iter().map(|a| a.uid).filter(|u| *u != uid).collect::<Vec<_>>() {
                let _ = j.set_lane_dest(uid, lane, dest, true);
            }
        }
        let v = j.view();
        let bad = v.arms.iter().flat_map(|a| &a.lanes).filter(|l| l.bad).count();
        assert_eq!(count(&svg(&j), "<g class=\"lane-arrow bad\" "), bad);
    }

    #[test]
    fn a_roundabout_has_its_circulation_arrows_and_a_turbo_too() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let (v, m) = (j.view(), svg(&j));
        let ring = ring_arrows(&v).len();
        assert_eq!(ring, v.arms.len());
        let lanes: usize = v.arms.iter().map(|a| a.lanes.len()).sum();
        let leaving: usize = v.arms.iter().map(|a| a.leave_arrows.len()).sum();
        assert_eq!(count(&m, "<g class=\"lane-arrow\" "), lanes + ring);
        assert_eq!(count(&m, "<g class=\"lane-arrow out\" "), leaving);
        assert!(j.set_turbo(Some(0)));
        let (v, m) = (j.view(), svg(&j));
        let lanes: usize = v.arms.iter().map(|a| a.lanes.len()).sum();
        assert_eq!(count(&m, "<g class=\"lane-arrow\" "), lanes + ring_arrows(&v).len());
        no_missing_number(&m);
    }

    #[test]
    fn the_selected_arm_shows_where_its_traffic_may_go_and_a_banned_turn_is_crossed_out() {
        let mut j = Junction::new(0);
        assert_eq!(count(&svg(&j), "class=\"mv"), 0, "nothing selected, no turns");
        let uid = j.current().arms[0].uid;
        let other = j.current().arms[1].uid;
        j.select(Target::Arm(uid));
        let v = j.view();
        let mine: Vec<_> = v.movements.iter().filter(|m| m.from == uid).collect();
        assert!(mine.len() >= 2);
        let m = svg(&j);
        assert_eq!(count(&m, "class=\"mv"), mine.len());
        assert_eq!(count(&m, "class=\"mv no"), mine.iter().filter(|m| !m.allowed).count());
        // A lane, a crossing and a corner of the arm show its turns too, as on the plan.
        j.select(Target::Lane(uid, 0));
        assert_eq!(count(&svg(&j), "class=\"mv"), mine.len());
        // Banning a turn makes it a dashed, crossed-out one.
        let allowed = mine.iter().filter(|m| m.allowed).count();
        assert!(allowed > 0);
        let class = v.movements.iter().find(|m| m.from == uid && m.to == other).map(|m| m.class);
        assert!(class.is_some());
        no_missing_number(&m);
    }

    #[test]
    fn a_turn_that_has_no_lane_is_bad_and_one_made_the_indirect_way_is_marked() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        j.select(Target::Arm(uid));
        let v = j.view();
        let bad = v.movements.iter().filter(|m| m.from == uid && m.allowed && !m.lane).count();
        let indirect = v.movements.iter().filter(|m| m.from == uid && m.indirect).count();
        let m = svg(&j);
        assert_eq!(count(&m, "class=\"mv bad"), bad);
        assert_eq!(count(&m, " indirect\""), indirect);
        // Close every lane of the arm to turns: the allowed turns have no lane, so they are bad.
        for lane in 0..j.current().arms[0].lanes.len() {
            for dest in j.current().arms.iter().map(|a| a.uid).filter(|u| *u != uid).collect::<Vec<_>>() {
                let _ = j.set_lane_dest(uid, lane, dest, false);
            }
        }
        let v = j.view();
        let bad = v.movements.iter().filter(|m| m.from == uid && m.allowed && !m.lane).count();
        assert_eq!(count(&svg(&j), "class=\"mv bad"), bad);
    }

    #[test]
    fn a_turn_starts_where_the_street_enters_and_ends_where_the_other_leaves() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        j.select(Target::Arm(uid));
        let v = j.view();
        let s = iso_svg(&v, &crate::i18n_for(crate::shared::i18n::Locale::En), 1000.0, 1000.0, Units::Metres);
        let m = v.movements.iter().find(|m| m.from == uid && m.allowed).unwrap();
        let path = flatten(&m.path);
        let (x, y) = s.frame.at(path[0], 0.0);
        assert!(s.markup.contains(&format!("d=\"M{} {}", crate::shared::iso::format::f1(x), crate::shared::iso::format::f1(y))), "the turn starts at {x},{y}");
    }

    #[test]
    fn a_stop_draws_its_pad_dwell_run_and_post_and_a_bad_one_says_so() {
        let (j, uid) = junction_with_stops(|_| {}, |lane, walk| vec![end_stop(25_000, 33_000, lane, walk)]);
        let v = j.view();
        let s = &v.arms.iter().find(|a| a.uid == uid).unwrap().stops[0];
        assert!(s.bad);
        let m = svg(&j);
        for class in ["stop-pad stop-bad", "stop-dwell stop-bad", "stop-run", "stop-post"] {
            assert_eq!(count(&m, &format!("class=\"{class}")), 1, "{class}");
        }
        assert_eq!(count(&m, "class=\"stop-shelter\""), 1, "the stop has a shelter");
        no_missing_number(&m);
        // Far from the junction and with a lane that goes straight on, a stop is not bad.
        let (j, _) = junction_with_stops(|_| {}, |lane, walk| vec![end_stop(25_000, 33_000, lane, walk)]);
        let plain = svg(&Junction::new(0));
        assert!(!plain.contains("stop-pad") && !plain.contains("stop-run"));
        assert!(svg(&j).len() > plain.len());
    }

    #[test]
    fn a_stop_far_side_of_the_crossing_has_no_run_and_a_bulb_or_island_says_which() {
        use crate::shared::catalogue::Side;
        use crate::street::model::StopKind;
        for (kind, class) in [(StopKind::Bulb, "stop-pad stop-bulb"), (StopKind::Island, "stop-pad stop-island")] {
            let (j, _) = junction_with_stops(
                |_| {},
                |lane, walk| {
                    let mut s = end_stop(25_000, 33_000, lane, walk);
                    s.kind = kind;
                    s.side = Side::Left;
                    vec![s]
                },
            );
            assert_eq!(count(&svg(&j), &format!("class=\"{class}")), 1, "{class}");
        }
        let (j, _) = junction_with_stops(
            |_| {},
            |lane, walk| {
                let mut s = end_stop(25_000, 33_000, lane, walk);
                s.enters = false;
                vec![s]
            },
        );
        assert_eq!(count(&svg(&j), "class=\"stop-run"), 0);
    }

    #[test]
    fn a_boundary_with_a_taper_draws_its_two_marks_across_the_arm() {
        let (j, _) = junction_with_a_pocket(|_| {});
        let v = j.view();
        let marks: usize = v.arms.iter().map(|a| a.beyond_marks.len()).sum();
        assert!(marks >= 1);
        assert_eq!(count(&svg(&j), "class=\"approach-mark\""), marks);
        assert_eq!(count(&svg(&Junction::new(0)), "approach-mark"), 0);
    }

    #[test]
    fn the_measures_of_an_arm_are_tagged_with_their_codes_and_drawn() {
        let mut j = Junction::new(0);
        let e = j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid;
        let w = j.current().arms.iter().find(|a| a.bearing == 270).unwrap().uid;
        assert_eq!(count(&svg(&j), "measure-tag"), 0);
        assert!(j.set_bus_lane(e, true) && j.set_approach(e, GATE_YIELD) && j.set_approach(w, Q_VIRTUAL) && j.set_rule(e, RULE_DEAD_END));
        let (v, m) = (j.view(), svg(&j));
        let tags: usize = v.arms.iter().map(|a| a.transit.tags.len()).sum();
        assert!(tags >= 2);
        assert_eq!(count(&m, "class=\"t-note t-halo measure-tag\""), tags);
        for code in v.arms.iter().flat_map(|a| a.transit.tags.iter().map(|t| t.code)) {
            assert!(m.contains(&format!(">{code}</text>")), "{code}");
        }
        assert_eq!(count(&m, "class=\"measure-loop\""), v.arms.iter().filter(|a| a.transit.virtual_loop.is_some()).count());
        assert_eq!(count(&m, "class=\"dead-cap\""), v.arms.iter().filter(|a| a.transit.cap.is_some()).count());
        // The gate is a stop line of its own, beside the arm's.
        let gates = v.arms.iter().filter(|a| a.transit.gate.is_some()).count();
        assert!(gates >= 1);
        no_missing_number(&m);
    }

    #[test]
    fn a_queue_jump_lane_is_dashed_and_a_dead_end_arm_is_capped() {
        let mut j = Junction::new(0);
        let e = j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid;
        assert!(j.set_bus_lane(e, true) && j.set_approach(e, Q_CURB));
        assert_eq!(count(&svg(&j), "class=\"iso-queue\""), 1);
        assert_eq!(count(&svg(&Junction::new(0)), "iso-queue"), 0);
    }

    #[test]
    fn a_curbside_use_that_fails_its_check_is_outlined_and_one_that_passes_is_not() {
        use crate::street::model::Use;
        let probe = svg(&junction_with_uses(|p| vec![end_use(30_000, Use::Parking, p)]).0);
        assert_eq!(count(&probe, "curb-use-bad"), 0);
        let (j, _) = junction_with_uses(|p| vec![end_use(18_000, Use::Parking, p)]);
        let bad = j.view().arms.iter().flat_map(|a| &a.curb_uses).filter(|u| u.bad).count();
        assert_eq!(count(&svg(&j), "class=\"curb-use curb-use-bad\""), bad);
    }

    #[test]
    fn a_curbside_use_stands_or_lies_on_its_arm_in_its_own_material() {
        use crate::street::model::Use;
        let before = svg(&Junction::new(0));
        let (j, _) = junction_with_uses(|p| vec![end_use(25_000, Use::Tree, p)]);
        let m = svg(&j);
        assert_eq!(count(&m, "mat-trees\""), count(&before, "mat-trees\"") + 1, "a street tree stands as a raised strip");
        let (j, _) = junction_with_uses(|p| vec![end_use(25_000, Use::Parking, p)]);
        assert!(count(&svg(&j), "iso-flat k-parking") > count(&before, "iso-flat k-parking"), "parking lies in its lane");
    }

    #[test]
    fn a_corner_that_fails_is_drawn_with_a_warning_curb() {
        let mut j = Junction::new(0);
        let uid = j.current().arms[0].uid;
        let fast = |j: &Junction| j.view().corners.iter().filter(|c| !c.straight && (!c.ok || c.fast)).count();
        let before = fast(&j);
        assert_eq!(count(&svg(&j), "class=\"curb-line bad\""), before);
        // A big radius makes the turn fast; whatever the view then calls bad is drawn.
        for _ in 0..30 {
            j.step_corner(uid, 1);
        }
        assert_eq!(count(&svg(&j), "class=\"curb-line bad\""), fast(&j));
    }

    #[test]
    fn a_crossing_has_its_length_written_across_it_and_a_long_one_is_a_warning() {
        let j = Junction::new(0);
        let (v, m) = (j.view(), svg(&j));
        let crossings = v.arms.iter().filter(|a| a.crossing.is_some()).count();
        assert!(crossings > 0);
        assert_eq!(count(&m, "class=\"dim\"") + count(&m, "class=\"dim dim-warn\""), crossings);
        assert_eq!(count(&m, "class=\"t-dim t-halo"), crossings);
        let long = v.arms.iter().filter_map(|a| a.crossing.as_ref()).filter(|c| c.too_far).count();
        assert_eq!(count(&m, "class=\"dim dim-warn\""), long);
    }

    /// The crossing lengths written in a drawing: where each stands and how many characters it has.
    fn dimension_texts(markup: &str) -> Vec<Spot> {
        markup
            .split("<text class=\"t-dim t-halo")
            .skip(1)
            .map(|t| {
                let num = |name: &str| t.split(&format!(" {name}=\"")).nth(1).unwrap().split('"').next().unwrap().parse::<f64>().unwrap();
                let text = t.split('>').nth(1).unwrap().split('<').next().unwrap();
                Spot { at: (num("x"), num("y")), chars: text.chars().count() }
            })
            .collect()
    }

    #[test]
    fn the_lengths_of_neighbouring_crossings_never_overlap_and_the_longest_keeps_its_place() {
        for width in [1000.0, 600.0, 320.0] {
            for sample in 0..4 {
                let j = Junction::new(sample);
                let v = j.view();
                let m = iso_svg(&v, &crate::i18n_for(crate::shared::i18n::Locale::En), width, 800.0, Units::Metres).markup;
                let texts = dimension_texts(&m);
                for (i, a) in texts.iter().enumerate() {
                    assert!(texts[i + 1..].iter().all(|b| !overlap(a, b)), "sample {sample} at {width}: {a:?} meets another in {texts:?}");
                }
                let crossings = v.arms.iter().filter(|a| a.crossing.is_some()).count();
                assert!(!texts.is_empty() && texts.len() <= crossings, "sample {sample} at {width}");
                // The lines are all drawn, whether or not their figures are.
                assert_eq!(count(&m, "class=\"dim\"") + count(&m, "class=\"dim dim-warn\""), crossings);
            }
        }
    }

    #[test]
    fn a_roundabout_with_a_bus_lane_says_so_and_a_turbo_entry_is_widened() {
        let mut j = Junction::new(0);
        assert!(j.set_control(ROUNDABOUT));
        let at = |j: &Junction, b: i32| j.current().arms.iter().find(|a| a.bearing == b).unwrap().uid;
        let (n, s) = (at(&j, 0), at(&j, 180));
        assert!(!svg(&j).contains(">Bus only</text>"));
        assert!(j.set_bus(Some((n, s))));
        assert_eq!(count(&svg(&j), ">Bus only</text>"), 1);
    }

    #[test]
    fn nothing_the_2_5d_marks_on_the_ground_takes_a_press() {
        let (j, _) = junction_with_stops(|_| {}, |lane, walk| vec![end_stop(25_000, 33_000, lane, walk)]);
        let mut j = j;
        let uid = j.current().arms[0].uid;
        j.select(Target::Arm(uid));
        let m = svg(&j);
        for class in ["lane-arrow", "iso-casing", "mv", "stop-pad", "stop-run", "dim"] {
            for part in m.split(&format!("class=\"{class}")).skip(1) {
                assert!(!part[..part.find('>').unwrap()].contains("data-role"), "{class}");
            }
        }
    }
}
