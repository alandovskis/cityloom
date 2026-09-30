//! Plan geometry for a junction. Pure functions on millimetres, no state.
//!
//! The sheet's plan has north up and y growing downward, like SVG. A bearing
//! runs clockwise from north. An arm leaves the junction along its bearing;
//! its "right" is the side on your right looking outward, which is the side
//! toward the next arm clockwise.

use serde_json::{Value, json};

pub type P = (f64, f64);

pub fn dir(bearing: f64) -> P {
    let t = bearing.to_radians();
    (t.sin(), -t.cos())
}

pub fn right(bearing: f64) -> P {
    let t = bearing.to_radians();
    (t.cos(), t.sin())
}

/// The point `lat` to the right of an arm's axis and `t` out along it.
pub fn at(bearing: f64, lat: f64, t: f64) -> P {
    let (d, r) = (dir(bearing), right(bearing));
    (lat * r.0 + t * d.0, lat * r.1 + t * d.1)
}

pub fn add(a: P, b: P) -> P {
    (a.0 + b.0, a.1 + b.1)
}

pub fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}

pub fn scale(a: P, k: f64) -> P {
    (a.0 * k, a.1 * k)
}

pub fn cross(a: P, b: P) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

pub fn dist(a: P, b: P) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The clockwise gap from bearing `a` to bearing `b`, in 0..360.
pub fn gap(a: i32, b: i32) -> i32 {
    (b - a).rem_euclid(360)
}

/// Where the line `lat_a` off arm `a`'s axis meets the line `lat_b` off arm
/// `b`'s axis, as distances (t, s) out along each. None when parallel.
pub fn meet(a: f64, lat_a: f64, b: f64, lat_b: f64) -> Option<(f64, f64)> {
    let (da, db) = (dir(a), dir(b));
    let d = cross(da, db);
    if d.abs() < 1e-9 {
        return None;
    }
    let q = sub(scale(right(b), lat_b), scale(right(a), lat_a));
    Some((cross(q, db) / d, -cross(da, q) / d))
}

/// Distance out along an arm at which the line `lat` off its axis meets a
/// circle of `radius` about the centre. None if the line misses it.
pub fn on_circle(lat: f64, radius: f64) -> Option<f64> {
    (lat.abs() < radius).then(|| (radius * radius - lat * lat).sqrt())
}

// ---- path commands, as JSON the page turns into SVG ------------------------

pub fn m(p: P) -> Value {
    json!(["M", p.0.round(), p.1.round()])
}

pub fn l(p: P) -> Value {
    json!(["L", p.0.round(), p.1.round()])
}

pub fn arc(r: f64, large: bool, sweep: bool, p: P) -> Value {
    json!(["A", r.round(), u8::from(large), u8::from(sweep), p.0.round(), p.1.round()])
}

pub fn quad(c: P, p: P) -> Value {
    json!(["Q", c.0.round(), c.1.round(), p.0.round(), p.1.round()])
}

pub fn z() -> Value {
    json!(["Z"])
}

pub fn poly(points: &[P]) -> Vec<Value> {
    let mut v: Vec<Value> = points.iter().enumerate().map(|(i, &p)| if i == 0 { m(p) } else { l(p) }).collect();
    v.push(z());
    v
}

/// A rectangle `lat0..lat1` across and `t0..t1` along an arm, as four corners.
pub fn strip(bearing: f64, lat0: f64, lat1: f64, t0: f64, t1: f64) -> [P; 4] {
    [at(bearing, lat0, t0), at(bearing, lat1, t0), at(bearing, lat1, t1), at(bearing, lat0, t1)]
}

/// A curb fillet between two lines that meet at `corner` with an interior
/// angle of `delta` degrees: the two tangent points and the arc's radius. The
/// tangent points are `radius / tan(delta / 2)` from the corner along each
/// line, outward.
pub fn fillet_setback(radius: f64, delta_deg: f64) -> f64 {
    radius / (delta_deg.to_radians() / 2.0).tan()
}

/// The arc from `a` to `b` about `o`, as an SVG arc command; always the short way.
pub fn short_arc(o: P, r: f64, a: P, b: P) -> Value {
    arc(r, false, cross(sub(a, o), sub(b, o)) > 0.0, b)
}

/// The arc along a circle about the origin from `a` to `b`, clockwise.
pub fn ring_arc(r: f64, a: P, b: P) -> Value {
    let sweep = (b.1.atan2(b.0) - a.1.atan2(a.0)).to_degrees().rem_euclid(360.0);
    arc(r, sweep > 180.0, true, b)
}

/// Whether two open polylines cross.
pub fn polylines_cross(a: &[P], b: &[P]) -> bool {
    let seg = |p: P, q: P, r: P, s: P| {
        let (d1, d2) = (cross(sub(q, p), sub(r, p)), cross(sub(q, p), sub(s, p)));
        let (d3, d4) = (cross(sub(s, r), sub(p, r)), cross(sub(s, r), sub(q, r)));
        d1 * d2 < 0.0 && d3 * d4 < 0.0
    };
    a.windows(2).any(|x| b.windows(2).any(|y| seg(x[0], x[1], y[0], y[1])))
}

/// Samples a quadratic Bezier.
pub fn sample_quad(a: P, c: P, b: P, n: usize) -> Vec<P> {
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            (u * u * a.0 + 2.0 * u * t * c.0 + t * t * b.0, u * u * a.1 + 2.0 * u * t * c.1 + t * t * b.1)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn north_is_up_and_right_is_east() {
        assert!(near(dir(0.0).0, 0.0) && near(dir(0.0).1, -1.0));
        assert!(near(right(0.0).0, 1.0) && near(right(0.0).1, 0.0));
        assert!(near(dir(90.0).0, 1.0));
        assert!(near(right(90.0).1, 1.0));
    }

    #[test]
    fn curb_lines_of_a_square_crossing_meet_at_the_expected_corner() {
        // North arm's right curb is 5 m east of the axis; the east arm's left
        // curb is 5 m north of its axis. They meet 5 m out on each arm.
        let (t, s) = meet(0.0, 5000.0, 90.0, -5000.0).unwrap();
        assert!(near(t, 5000.0) && near(s, 5000.0), "{t} {s}");
        let p = at(0.0, 5000.0, t);
        assert!(near(p.0, 5000.0) && near(p.1, -5000.0));
    }

    #[test]
    fn opposite_arms_do_not_meet() {
        assert!(meet(0.0, 5000.0, 180.0, -5000.0).is_none());
    }

    #[test]
    fn a_right_angle_fillet_sets_back_by_its_radius() {
        assert!(near(fillet_setback(6000.0, 90.0), 6000.0));
        assert!(fillet_setback(6000.0, 60.0) > 6000.0);
    }

    #[test]
    fn circle_meets_line_at_pythagoras() {
        assert!(near(on_circle(3000.0, 5000.0).unwrap(), 4000.0));
        assert!(on_circle(6000.0, 5000.0).is_none());
    }

    #[test]
    fn crossing_polylines_are_found() {
        let a = [(0.0, 0.0), (10.0, 10.0)];
        let b = [(0.0, 10.0), (10.0, 0.0)];
        let c = [(20.0, 0.0), (30.0, 10.0)];
        assert!(polylines_cross(&a, &b));
        assert!(!polylines_cross(&a, &c));
    }

    #[test]
    fn gap_is_clockwise() {
        assert_eq!(gap(350, 10), 20);
        assert_eq!(gap(10, 350), 340);
    }
}
