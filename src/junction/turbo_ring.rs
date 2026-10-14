//! Helpers on the outlines of a turbo ring, in millimetres on the plan: where an arm's line meets an outline, the vertex
//! nearest a point, and the walk along an outline. The ring itself, a spiral, is `turbo_spiral.rs`. No rules here.

use crate::junction::geometry::{P, dir, dist, right};

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

    #[test]
    fn an_open_line_has_no_closing_side() {
        let line = [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0)];
        assert_eq!(mouth_on_open(&line, 0.0, 5.0), Some(-10.0));
        assert_eq!(mouth_on(&line, 0.0, 5.0), Some(-5.0), "closed, the side from the last point back to the first is met as well, and it is farther along");
    }

    #[test]
    fn walking_an_outline_goes_from_the_vertex_nearest_one_point_to_the_one_nearest_another() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert_eq!(nearest(&square, (9.0, 1.0)), 1);
        assert_eq!(follow(&square, (0.5, 0.0), (9.0, 9.0), true), [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        assert_eq!(follow(&square, (0.5, 0.0), (9.0, 9.0), false), [(0.0, 0.0), (0.0, 10.0), (10.0, 10.0)]);
        assert_eq!(follow(&square, (0.0, 0.0), (0.0, 0.0), true), [(0.0, 0.0)]);
    }
}
