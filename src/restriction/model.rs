//! An area restriction: a traffic rule over an outline the resident draws, and which streets it holds. Pure: no
//! DOM, no signals, no storage. The outline is in longitude and latitude, so it does not depend on which tile or
//! area is loaded; the geometry is done in whole micro-degrees, where it is exact.

use crate::city::model::EdgeView;
use crate::city::projection::Projection;
use crate::shared::said::Said;

/// A position on the earth: longitude, then latitude, in degrees (GeoJSON's order).
pub type LonLat = [f64; 2];

/// A position in whole micro-degrees, where the geometry is exact.
type Micro = (i64, i64);

fn micro([lon, lat]: LonLat) -> Micro {
    ((lon * 1e6).round() as i64, (lat * 1e6).round() as i64)
}

/// Whether two positions are the same to the micro-degree.
pub fn same_spot(a: LonLat, b: LonLat) -> bool {
    micro(a) == micro(b)
}

/// Twice the signed area of the triangle a, b, c: positive when they turn left.
fn cross(a: Micro, b: Micro, c: Micro) -> i128 {
    (b.0 - a.0) as i128 * (c.1 - a.1) as i128 - (b.1 - a.1) as i128 * (c.0 - a.0) as i128
}

/// Whether `p` lies on the segment a-b, its ends included.
fn on_segment(p: Micro, a: Micro, b: Micro) -> bool {
    cross(a, b, p) == 0 && (a.0.min(b.0)..=a.0.max(b.0)).contains(&p.0) && (a.1.min(b.1)..=a.1.max(b.1)).contains(&p.1)
}

/// The ring's edges, the last joining the first.
fn edges(ring: &[Micro]) -> impl Iterator<Item = (Micro, Micro)> + '_ {
    (0..ring.len()).map(move |i| (ring[i], ring[(i + 1) % ring.len()]))
}

/// Whether `p` is inside the ring, by the even-odd rule; a point on the border counts as inside. The ring is not
/// closed explicitly. A ring across the antimeridian is not supported: it is read the long way round.
pub fn contains(ring: &[LonLat], p: LonLat) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let ring: Vec<Micro> = ring.iter().copied().map(micro).collect();
    let p = micro(p);
    let mut inside = false;
    for (a, b) in edges(&ring) {
        if on_segment(p, a, b) {
            return true;
        }
        // An edge that spans p's latitude (half-open, so a corner on it is counted once) crosses the ray east of
        // p when p is on its left going up, or on its right going down.
        if (a.1 > p.1) != (b.1 > p.1) && (cross(a, b, p) > 0) == (b.1 > a.1) {
            inside = !inside;
        }
    }
    inside
}

/// Whether two segments meet at all: cross, touch or overlap.
fn meet(a: Micro, b: Micro, c: Micro, d: Micro) -> bool {
    let opposite = |x: i128, y: i128| (x > 0 && y < 0) || (x < 0 && y > 0);
    (opposite(cross(c, d, a), cross(c, d, b)) && opposite(cross(a, b, c), cross(a, b, d)))
        || on_segment(a, c, d)
        || on_segment(b, c, d)
        || on_segment(c, a, b)
        || on_segment(d, a, b)
}

/// Whether the ring can be an area: at least three corners not all in a line, no corner used twice, and no edge
/// meeting another except where neighbours share their corner. A refusal says why. O(n²), which outlines drawn
/// by hand can afford on every drop.
pub fn valid(ring: &[LonLat]) -> Result<(), Said> {
    let crossing = || Err(Said::new("restriction-refused-crossing"));
    let r: Vec<Micro> = ring.iter().copied().map(micro).collect();
    let mut distinct = r.clone();
    distinct.sort_unstable();
    distinct.dedup();
    // All in a line: every corner is on the line through the first two distinct ones. (The signed area is no
    // test of that: a bow tie's two lobes cancel to zero.)
    let in_a_line = distinct.len() < 3 || distinct.iter().all(|&p| cross(distinct[0], distinct[1], p) == 0);
    if in_a_line {
        return Err(Said::new("restriction-refused-too-few"));
    }
    if distinct.len() < r.len() {
        return crossing();
    }
    let n = r.len();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            let (c, d) = (r[j], r[(j + 1) % n]);
            if j == i + 1 || (i == 0 && j == n - 1) {
                // Neighbours share one corner; one folding back along the other overlaps it.
                let (shared, far_a, far_b) = if j == i + 1 { (b, a, d) } else { (a, b, c) };
                if cross(shared, far_a, far_b) == 0 && (on_segment(far_b, shared, far_a) || on_segment(far_a, shared, far_b)) {
                    return crossing();
                }
            } else if meet(a, b, c, d) {
                return crossing();
            }
        }
    }
    Ok(())
}

/// The point halfway along a line, by its length, in the line's own units.
pub fn midpoint(line: &[[i32; 2]]) -> Option<[f64; 2]> {
    let pts: Vec<[f64; 2]> = line.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    let last = *pts.last()?;
    let len = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    let mut left = pts.windows(2).map(|w| len(w[0], w[1])).sum::<f64>() / 2.0;
    for w in pts.windows(2) {
        let l = len(w[0], w[1]);
        if l > 0.0 && left <= l {
            let t = left / l;
            return Some([w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t]);
        }
        left -= l;
    }
    Some(last)
}

/// Whether the street along `shape_mm` (layout millimetres) is inside the ring: its midpoint is.
pub fn holds(ring: &[LonLat], shape_mm: &[[i32; 2]], projection: &Projection) -> bool {
    midpoint(shape_mm).is_some_and(|[x, y]| contains(ring, projection.lon_lat(x.round() as i32, y.round() as i32)))
}

/// The streets of a city view inside the ring, by uid.
pub fn inside(ring: &[LonLat], edges: &[EdgeView], projection: &Projection) -> Vec<u32> {
    edges.iter().filter(|e| holds(ring, &e.shape_mm, projection)).map(|e| e.uid).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restriction::fixtures::{bent_midpoint, bent_store, square};
    use crate::shared::testing::{en, fr};

    const SQUARE: [LonLat; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    /// A U: the notch between x 1 and 2 reaches down to y 1.
    const U: [LonLat; 8] = [[0.0, 0.0], [3.0, 0.0], [3.0, 3.0], [2.0, 3.0], [2.0, 1.0], [1.0, 1.0], [1.0, 3.0], [0.0, 3.0]];

    #[test]
    fn a_point_is_inside_outside_or_on_the_border_which_counts_as_inside() {
        assert!(contains(&SQUARE, [0.5, 0.5]));
        assert!(!contains(&SQUARE, [1.5, 0.5]));
        assert!(!contains(&SQUARE, [0.5, -0.5]));
        assert!(contains(&SQUARE, [1.0, 0.5]), "on an edge");
        assert!(contains(&SQUARE, [0.5, 1.0]), "on the top edge");
        assert!(contains(&SQUARE, [0.0, 0.0]), "on a corner");
    }

    #[test]
    fn a_concave_outline_leaves_its_notch_out() {
        assert!(!contains(&U, [1.5, 2.0]), "in the notch");
        assert!(contains(&U, [0.5, 2.0]) && contains(&U, [2.5, 2.0]) && contains(&U, [1.5, 0.5]));
    }

    #[test]
    fn a_ray_through_corners_is_counted_once_at_each_crossing() {
        let diamond = [[1.0, 0.0], [2.0, 1.0], [1.0, 2.0], [0.0, 1.0]];
        assert!(contains(&diamond, [0.5, 1.0]) && contains(&diamond, [1.0, 1.0]));
        assert!(!contains(&diamond, [-0.5, 1.0]) && !contains(&diamond, [2.5, 1.0]));
    }

    #[test]
    fn the_geometry_is_exact_to_the_micro_degree() {
        assert!(contains(&SQUARE, [1.000_000_4, 0.5]), "rounds onto the border");
        assert!(!contains(&SQUARE, [1.000_001, 0.5]), "a micro-degree out is out");
    }

    #[test]
    fn an_outline_across_the_antimeridian_is_not_supported_and_reads_as_the_long_way_round() {
        let across = [[179.0, 0.0], [-179.0, 0.0], [-179.0, 1.0], [179.0, 1.0]];
        assert!(!contains(&across, [179.5, 0.5]));
        assert!(contains(&across, [0.0, 0.5]));
    }

    #[test]
    fn fewer_than_three_corners_hold_nothing() {
        assert!(!contains(&[[0.0, 0.0], [1.0, 1.0]], [0.5, 0.5]));
        assert!(!contains(&[], [0.0, 0.0]));
    }

    #[test]
    fn an_outline_is_valid_with_three_corners_not_in_a_line_and_no_crossing() {
        assert_eq!(valid(&SQUARE), Ok(()));
        assert_eq!(valid(&U), Ok(()));
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]), Ok(()));
    }

    #[test]
    fn too_few_corners_or_all_in_a_line_are_refused_with_the_reason() {
        let too_few = Said::new("restriction-refused-too-few");
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0]]), Err(too_few.clone()));
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]), Err(too_few.clone()), "in a line");
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 0.0], [0.0, 0.0]]), Err(too_few), "two distinct");
        assert_eq!(en(&Said::new("restriction-refused-too-few")), "An area needs at least three corners, not all in a line.");
        assert_eq!(fr(&Said::new("restriction-refused-too-few")), "Une zone a besoin d’au moins trois coins, pas tous alignés.");
    }

    #[test]
    fn crossing_touching_folding_back_and_repeated_corners_are_refused_as_a_crossing() {
        let crossing = Err(Said::new("restriction-refused-crossing"));
        assert_eq!(valid(&[[0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0]]), crossing, "a bow tie");
        assert_eq!(valid(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [2.0, 0.0]]), crossing, "the last edge folds back along the first");
        assert_eq!(valid(&[[0.0, 0.0], [4.0, 0.0], [2.0, 2.0], [4.0, 4.0], [0.0, 4.0], [2.0, 2.0]]), crossing, "a corner used twice");
        assert_eq!(valid(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [2.0, 0.0], [0.0, 4.0]]), crossing, "a corner touching an edge");
        assert_eq!(en(&Said::new("restriction-refused-crossing")), "The outline crosses itself. Move or remove a corner.");
    }

    #[test]
    fn the_midpoint_is_halfway_along_the_line_by_its_length() {
        assert_eq!(midpoint(&[[0, 0], [10, 0]]), Some([5.0, 0.0]));
        let [x, y] = midpoint(&[[0, 0], [80_000, 0], [100_000, 50_000]]).unwrap();
        let half = (80_000.0 + 20_000f64.hypot(50_000.0)) / 2.0;
        assert!((x - half).abs() < 1e-6 && y == 0.0, "{x} {y}");
        assert_eq!(midpoint(&[[3, 4]]), Some([3.0, 4.0]));
        assert_eq!(midpoint(&[]), None);
    }

    #[test]
    fn the_streets_inside_are_those_whose_midpoint_is() {
        let (ports, ..) = crate::shared::ports::test_ports();
        let store = bent_store(ports.storage);
        let v = store.open().view(0);
        let (p, mid) = bent_midpoint(&store);
        assert_eq!(inside(&square(mid, 0.0005), &v.edges, &p), vec![1]);
        assert_eq!(inside(&square([mid[0] + 0.01, mid[1]], 0.0005), &v.edges, &p), Vec::<u32>::new());
        let start = p.lon_lat(v.edges[0].shape_mm[0][0], v.edges[0].shape_mm[0][1]);
        assert_eq!(inside(&square(start, 0.0002), &v.edges, &p), Vec::<u32>::new(), "an end inside is not the street inside");
        assert!(holds(&square(mid, 0.0005), &v.edges[0].shape_mm, &p));
    }
}
