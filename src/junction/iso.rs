//! The junction in 2.5D, as SVG markup, from the model's view: a read-only picture of what the plan draws flat.
//! A plan point is rotated by a fixed angle, its depth squashed, and its height lifted up the screen; there is one
//! viewpoint, so the shapes are painted far to near with no depth buffer.

use serde_json::Value;

use crate::junction::geometry::{sample_arc, sample_quad};

type P = (f64, f64);

/// How the plan is turned, in degrees, before it is squashed.
pub const ROTATION_DEG: f64 = 30.0;
/// How much of its depth a point keeps.
pub const SQUASH: f64 = 0.55;
/// How much taller than true a height is drawn, so that a curb reads.
pub const LIFT: f64 = 3.0;
/// The tallest thing drawn, for the room left above the junction.
pub const TALLEST_MM: f64 = 3000.0;

/// The steps a curve is sampled in.
const CURVE_STEPS: usize = 12;

/// A plan point as the viewer sees the ground: turned, not yet squashed. Its `.1` is how near it is to the viewer.
pub fn view_point(p: P) -> P {
    let t = ROTATION_DEG.to_radians();
    (p.0 * t.cos() - p.1 * t.sin(), p.0 * t.sin() + p.1 * t.cos())
}

/// A plan point `h_mm` above the ground, in millimetres on the screen before it is scaled.
pub fn project(p: P, h_mm: f64) -> P {
    let (x, y) = view_point(p);
    (x, y * SQUASH - h_mm * LIFT)
}

/// The points of a path of the model's drawing commands, curves sampled.
pub fn flatten(cmds: &[Value]) -> Vec<P> {
    let num = |c: &Value, i: usize| c[i].as_f64().unwrap_or(0.0);
    let mut pts: Vec<P> = Vec::new();
    for c in cmds {
        match c[0].as_str().unwrap_or("Z") {
            "M" | "L" => pts.push((num(c, 1), num(c, 2))),
            "Q" => {
                if let Some(&from) = pts.last() {
                    pts.extend(sample_quad(from, (num(c, 1), num(c, 2)), (num(c, 3), num(c, 4)), CURVE_STEPS).into_iter().skip(1));
                }
            }
            "A" => {
                if let Some(&from) = pts.last() {
                    let to = (num(c, 4), num(c, 5));
                    pts.extend(sample_arc(from, num(c, 1), num(c, 2) != 0.0, num(c, 3) != 0.0, to, CURVE_STEPS).into_iter().skip(1));
                }
            }
            _ => {}
        }
    }
    pts
}

/// How high a piece of this kind stands above the roadway, in millimetres. Presentation, not catalogue data.
pub fn height_of(kind_id: &str) -> f64 {
    match kind_id {
        "sidewalk" | "median" | "planting" => 150.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_origin_stays_put_and_height_lifts_it_by_the_exaggeration() {
        assert_eq!(project((0.0, 0.0), 0.0), (0.0, 0.0));
        assert_eq!(project((0.0, 0.0), 150.0), (0.0, -450.0));
    }

    #[test]
    fn a_point_is_rotated_then_its_depth_squashed() {
        let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
        let p = project((1000.0, 0.0), 0.0);
        assert!((p.0 - 1000.0 * c).abs() < 1e-9);
        assert!((p.1 - 1000.0 * s * SQUASH).abs() < 1e-9);
        assert!((view_point((0.0, 1000.0)).1 - 1000.0 * c).abs() < 1e-9);
    }

    #[test]
    fn path_commands_flatten_to_a_polyline_that_ends_where_the_path_ends() {
        let cmds = vec![
            json!(["M", 0.0, 0.0]),
            json!(["L", 1000.0, 0.0]),
            json!(["Q", 1000.0, 1000.0, 0.0, 1000.0]),
            json!(["A", 500.0, 0, 1, 0.0, 0.0]),
            json!(["Z"]),
        ];
        let pts = flatten(&cmds);
        assert_eq!(pts[0], (0.0, 0.0));
        assert_eq!(pts[1], (1000.0, 0.0));
        assert!(pts.len() > 10);
        let last = *pts.last().unwrap();
        assert!(last.0.abs() < 1e-6 && last.1.abs() < 1e-6);
    }

    #[test]
    fn every_kind_of_piece_has_a_height() {
        for k in crate::shared::catalogue::KINDS.iter() {
            let h = height_of(k.id);
            assert!((0.0..=TALLEST_MM).contains(&h), "{}", k.id);
        }
        assert_eq!(height_of("travel"), 0.0);
        assert_eq!(height_of("sidewalk"), 150.0);
        assert_eq!(height_of("median"), 150.0);
        assert_eq!(height_of("planting"), 150.0);
    }
}
