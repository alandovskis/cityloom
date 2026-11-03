//! What the plan draws on the ground and the 2.5D draws after it (#140, stage 2): the turn arrows on the lanes, the
//! small arrows where traffic leaves, the circulation arrows on a roundabout. Pure geometry in millimetres on the
//! plan, as polylines; `iso.rs` projects them. Nothing here knows the screen or a colour.

use crate::junction::geometry::{dir, sample_quad};
use crate::junction::model::{LEFT, RIGHT, THROUGH};
use crate::junction::plan_svg::outer_lane_at;
use crate::junction::read_model::{JView, LaneView};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::*;

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
}
