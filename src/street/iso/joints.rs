//! The regular joints of a textured top (a concrete panel's, a paver's, a brick's, a tree-pit grate's) as geometry.
//!
//! A pattern laid on a perspective by affine bands steps at every seam, and a joint is a line the eye follows, so these
//! are drawn as lines in ground millimetres, each point projected through the perspective (`Projection::at`): exact at
//! every depth, with no seam. The fine grain (asphalt, gravel, grass, a bed's mulch, a concrete's broom finish) stays
//! a band pattern, where a step is not seen.
//!
//! **Thinning.** Joints that would be closer than `MIN_JOINT_GAP_PX` on the screen are not drawn, so the markup is
//! bounded by the drawing's pixels and the far end does not turn to mush. A set of parallel joints is thinned by halves:
//! where its neighbours would be under 2 px apart, every second line is dropped, and every second again, until the
//! lines left are at least 2 px apart. Joints across the street (a course's edge) are judged at their own depth, by
//! how far apart they are down the screen (`course_level`); joints along it, whose gap is the same at every point of
//! them and shrinks toward the vanishing point, are drawn as far as that gap lasts (`along_reach`). A running bond's
//! ticks (the short joints between the units of a course) are drawn only for the courses that are not thinned.

use crate::shared::diagram::render::joints_d;
use crate::shared::diagram::{JointRun, Pt};
use crate::shared::iso::ground::Joints;

use super::projection::{EYE_BACK_MM, Projection, STRETCH_MM};

/// The nearest two joints are drawn on the screen, in pixels.
pub const MIN_JOINT_GAP_PX: f64 = 2.0;

/// How many times a set of joints across the street is halved at depth `z` (from the near end) before its neighbours,
/// `course_mm` apart, are `MIN_JOINT_GAP_PX` apart down the screen at a top `h` high: 0 when they already are.
pub(super) fn course_level(p: &Projection, h: f64, course_mm: f64, z: f64) -> u32 {
    let down = |q: f64| (p.at(0.0, h, z).1 - p.at(0.0, h, z + q).1).abs();
    let mut level = 0;
    while level < 24 && down(course_mm * f64::from(1u32 << level)) < MIN_JOINT_GAP_PX {
        level += 1;
    }
    level
}

/// How far along the street a joint along it, the `i`th of those `unit_mm` apart, is drawn, in millimetres from the
/// near end: as far as the lines left after halving its set as often as its index is divisible by 2 (a line at index 0
/// is never dropped) are `MIN_JOINT_GAP_PX` apart across the screen. Nothing (0) when even the near end is too close.
pub(super) fn along_reach(p: &Projection, unit_mm: f64, i: u64) -> f64 {
    if i == 0 {
        return STRETCH_MM;
    }
    let apart = unit_mm * f64::from(1u32 << i.trailing_zeros().min(24));
    (EYE_BACK_MM * (p.scale * apart / MIN_JOINT_GAP_PX - 1.0)).clamp(0.0, STRETCH_MM)
}

fn run(x0: f64, z0: f64, x1: f64, z1: f64, h: f64, flat: bool) -> JointRun {
    JointRun { from: Pt::new(x0, h, z0), to: Pt::new(x1, h, z1), flat }
}

/// The path data of a top's joints (`runs`, written by the renderer).
pub(super) fn path(p: &Projection, j: &Joints, x0: f64, x1: f64, h: f64) -> String {
    joints_d(p, &runs(p, j, x0, x1, h))
}

/// The joints of a top, `x0` to `x1` across and the projection's reach along, `h` (drawn mm) high. Every
/// point is inside the top: the joints are cut to its edges here, and a joint on an edge is left to the edge. The
/// courses stay at multiples of their size from the near end of the stretch, so a reach continues the texture.
pub(super) fn runs(p: &Projection, j: &Joints, x0: f64, x1: f64, h: f64) -> Vec<JointRun> {
    const EDGE: f64 = 0.5;
    let (near, far) = (p.reach.near, p.reach.far);
    let mut d = Vec::new();
    // Across the street, one every course, at their own depth.
    for k in 1.. {
        let z = k as f64 * j.course_mm;
        if z >= far - EDGE {
            break;
        }
        if z <= near + EDGE {
            continue;
        }
        if (k as u64).trailing_zeros() >= course_level(p, h, j.course_mm, z) {
            d.push(run(x0, z, x1, z, h, true));
        }
    }
    let first = (x0 / j.unit_mm).floor() as u64;
    if j.bond {
        // A running bond: the short joints between the units of each course, a half unit out in every other one.
        for k in 0.. {
            let z = k as f64 * j.course_mm;
            if z >= far - EDGE || course_level(p, h, j.course_mm, z) > 0 || j.unit_mm * p.scale_at(z) < MIN_JOINT_GAP_PX {
                break;
            }
            let out = if k % 2 == 1 { 0.5 } else { 0.0 };
            for i in first.. {
                let u = (i as f64 + out) * j.unit_mm;
                if u >= x1 - EDGE {
                    break;
                }
                if u > x0 + EDGE {
                    // A course that ends before the reach begins is not drawn; one that crosses its edge is cut there.
                    let (from, to) = (z.max(near), (z + j.course_mm).min(far));
                    if to > from {
                        d.push(run(u, from, u, to, h, false));
                    }
                }
            }
        }
    } else {
        // A grid: a line along the street, as far as it keeps its neighbours 2 px away.
        for i in first + 1.. {
            let u = i as f64 * j.unit_mm;
            if u >= x1 - EDGE {
                break;
            }
            let reach = along_reach(p, j.unit_mm, i);
            let to = reach.min(far);
            if u > x0 + EDGE && to - near >= 1.0 {
                d.push(run(u, near, u, to, h, false));
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::iso::ground::joints;
    use crate::street::iso::projection::{Reach, band_edges};

    /// One drawn joint, as its two ends on the screen.
    type Seg = ((f64, f64), (f64, f64));

    /// The segments of a path as `path` writes it: `Mx,yHx2` across and `Mx,yLx2,y2` along or a tick.
    fn segments(d: &str) -> Vec<Seg> {
        let mut out = Vec::new();
        for part in d.split('M').skip(1) {
            let (start, rest) = part.split_at(part.find(['H', 'L']).unwrap());
            let mut s = start.split(',').map(|n| n.parse::<f64>().unwrap());
            let a = (s.next().unwrap(), s.next().unwrap());
            let b = if let Some(x) = rest.strip_prefix('H') {
                (x.parse::<f64>().unwrap(), a.1)
            } else {
                let mut e = rest[1..].split(',').map(|n| n.parse::<f64>().unwrap());
                (e.next().unwrap(), e.next().unwrap())
            };
            out.push((a, b));
        }
        out
    }

    fn sidewalk() -> (Projection, f64) {
        (Projection::fit(1050.0, 18_000), crate::street::iso::drawn_mm(150))
    }

    #[test]
    fn the_renderer_writes_a_tops_joints_as_the_street_does() {
        use crate::shared::diagram::{Diagram, Layer, LayerKind, render::render};
        let (p, h) = sidewalk();
        for m in ["concrete", "brick", "permeable", "trees"] {
            let j = joints(m).unwrap();
            let runs = runs(&p, &j, 0.0, 3_300.0, h);
            let d = Diagram {
                defs: String::new(),
                ground: Layer { kind: LayerKind::Ground, shapes: vec![crate::shared::diagram::Shape::Joints { runs, class: format!("joints j-{m}") }] },
                sections: vec![],
                outline: Layer { kind: LayerKind::Outline, shapes: vec![] },
            };
            let want = format!(r#"<path class="joints j-{m}" d="{}"/>"#, path(&p, &j, 0.0, 3_300.0, h));
            assert!(path(&p, &j, 0.0, 3_300.0, h).len() > 10, "{m}");
            assert_eq!(render(&d, &p), format!(r#"<g class="iso-layer-ground">{want}</g><g class="iso-layer-outline"></g>"#), "{m}");
        }
    }

    #[test]
    fn a_concrete_panel_s_joints_are_lines_every_1_8_m_each_way_projected_at_their_own_depth() {
        let (p, h) = sidewalk();
        let d = path(&p, &joints("concrete").unwrap(), 0.0, 3_300.0, h);
        let segs = segments(&d);
        // Across the street: one every 1.8 m from 1.8 m to 39.6 m, each from the strip's left edge to its right.
        let across: Vec<&Seg> = segs.iter().filter(|s| s.0.1 == s.1.1).collect();
        assert_eq!(across.len(), 22);
        for (k, s) in across.iter().enumerate() {
            let z = 1_800.0 * (k + 1) as f64;
            let (l, r) = (p.at(0.0, h, z), p.at(3_300.0, h, z));
            assert!((s.0.0 - l.0).abs() < 0.06 && (s.0.1 - l.1).abs() < 0.06 && (s.1.0 - r.0).abs() < 0.06, "{k}: {s:?} {l:?} {r:?}");
        }
        // Along it: one, at 1.8 m (3.6 m is past the strip's 3.3 m), from the near end to the far.
        let along: Vec<&Seg> = segs.iter().filter(|s| s.0.1 != s.1.1).collect();
        assert_eq!(along.len(), 1);
        let (near, far) = (p.at(1_800.0, h, 0.0), p.at(1_800.0, h, STRETCH_MM));
        assert!((along[0].0.0 - near.0).abs() < 0.06 && (along[0].0.1 - near.1).abs() < 0.06);
        assert!((along[0].1.0 - far.0).abs() < 0.06 && (along[0].1.1 - far.1).abs() < 0.06);
    }

    #[test]
    fn a_joint_is_exact_across_what_used_to_be_a_band_seam() {
        // The bands the texture was laid in stepped by pixels at each of their seams; a joint is one straight line, and
        // the projection's own point at every seam's depth lies on it.
        let (p, h) = sidewalk();
        let segs = segments(&path(&p, &joints("concrete").unwrap(), 0.0, 3_300.0, h));
        let along = segs.iter().find(|s| s.0.1 != s.1.1).unwrap();
        for z in band_edges(Reach::STRETCH) {
            let (x, y) = p.at(1_800.0, h, z);
            // The distance from the point to the line through the joint's ends.
            let (dx, dy) = (along.1.0 - along.0.0, along.1.1 - along.0.1);
            let dist = ((x - along.0.0) * dy - (y - along.0.1) * dx).abs() / dx.hypot(dy);
            assert!(dist < 0.1, "{z}: {dist}");
        }
        // And the across joints near a seam are at their own depth, a seam or none.
        let seam = band_edges(Reach::STRETCH)[3];
        let k = (seam / 1_800.0).floor();
        let want = p.at(0.0, h, 1_800.0 * k).1;
        assert!(segs.iter().any(|s| s.0.1 == s.1.1 && (s.0.1 - want).abs() < 0.06));
    }

    #[test]
    fn joint_lines_stay_inside_the_reach() {
        for reach in [Reach { near: 0.0, far: 20_000.0 }, Reach { near: 20_000.0, far: 40_000.0 }] {
            let (p, h) = sidewalk();
            let p = p.over(reach);
            let (top, bottom) = (p.at(0.0, h, reach.far).1, p.at(0.0, h, reach.near).1);
            for m in ["concrete", "permeable", "brick"] {
                let segs = segments(&path(&p, &joints(m).unwrap(), 0.0, 3_300.0, h));
                // Fine bond joints are dropped as they close up with distance, so only the grid and the near half need some.
                assert!(m != "concrete" && reach.near > 0.0 || !segs.is_empty(), "{m}");
                for (a, b) in segs {
                    for y in [a.1, b.1] {
                        assert!(y >= top - 0.06 && y <= bottom + 0.06, "{m} {reach:?}: {y} not in {top}..{bottom}");
                    }
                }
            }
        }
    }

    /// Whether `pt` is inside the convex polygon `poly` (counter-clockwise or clockwise), within `tol` px.
    fn inside(poly: &[(f64, f64)], pt: (f64, f64), tol: f64) -> bool {
        let mut sign = 0.0;
        for k in 0..poly.len() {
            let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
            let cross = (b.0 - a.0) * (pt.1 - a.1) - (b.1 - a.1) * (pt.0 - a.0);
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            if cross.abs() / len <= tol {
                continue;
            }
            if sign == 0.0 {
                sign = cross.signum();
            } else if cross.signum() != sign {
                return false;
            }
        }
        true
    }

    #[test]
    fn every_joint_of_every_material_stays_inside_the_top_it_belongs_to() {
        for (width, span) in [(1050.0, 18_000), (600.0, 30_000), (358.0, 18_000)] {
            let p = Projection::fit(width, span);
            for (id, x0, x1, h) in
                [("concrete", 600.0, 3_900.0, 450.0), ("brick", 1_050.0, 2_550.0, 450.0), ("permeable", 0.0, 2_400.0, 0.0), ("trees", 5_000.0, 6_500.0, 450.0)]
            {
                let top = [p.at(x0, h, 0.0), p.at(x1, h, 0.0), p.at(x1, h, STRETCH_MM), p.at(x0, h, STRETCH_MM)];
                let segs = segments(&path(&p, &joints(id).unwrap(), x0, x1, h));
                assert!(!segs.is_empty(), "{id} at {width}");
                for s in segs {
                    assert!(inside(&top, s.0, 0.1) && inside(&top, s.1, 0.1), "{id} at {width}: {s:?}");
                }
            }
        }
    }

    #[test]
    fn joints_closer_than_2_px_are_thinned_by_halves_so_the_lines_left_are_2_px_apart_at_every_depth() {
        let (p, h) = sidewalk();
        for id in ["brick", "permeable", "trees", "concrete"] {
            let j = joints(id).unwrap();
            let segs = segments(&path(&p, &j, 0.0, 3_300.0, h));
            let mut ys: Vec<f64> = segs.iter().filter(|s| s.0.1 == s.1.1).map(|s| s.0.1).collect();
            ys.sort_by(|a, b| b.total_cmp(a));
            assert!(ys.windows(2).all(|w| w[0] - w[1] >= MIN_JOINT_GAP_PX - 0.15), "{id}: {ys:?}");
            // Thinned: far fewer lines than the 40 m holds, and bounded by the pixels the street covers down the screen.
            let all = (STRETCH_MM / j.course_mm) as usize;
            let span = p.at(0.0, h, 0.0).1 - p.at(0.0, h, STRETCH_MM).1;
            assert!(ys.len() as f64 <= span / MIN_JOINT_GAP_PX + 2.0, "{id}: {} lines in {span} px", ys.len());
            if j.course_mm < 400.0 {
                assert!(ys.len() <= all / 2, "{id}: {} of {all}", ys.len());
            }
        }
        // The nearest are all drawn: a brick's courses are 3.5 px apart at the near end.
        let brick = segments(&path(&p, &joints("brick").unwrap(), 0.0, 3_300.0, h));
        let first = p.at(0.0, h, 100.0).1;
        assert!(brick.iter().any(|s| s.0.1 == s.1.1 && (s.0.1 - first).abs() < 0.06));
        // The level counts the halvings: none where the lines are far enough apart, more farther.
        assert_eq!(course_level(&p, h, 1_800.0, 1_800.0), 0);
        let levels: Vec<u32> = [100.0, 3_000.0, 12_000.0, 36_000.0].iter().map(|&z| course_level(&p, h, 100.0, z)).collect();
        assert_eq!(levels, [0, 0, 2, 4]);
    }

    #[test]
    fn a_joint_along_the_street_goes_as_far_as_its_neighbours_stay_2_px_apart() {
        // At 1050 px a joint 200 mm from the next is 11 px from it at the near end and 2.9 px at 40 m: all go to the end.
        let p = Projection::fit(1050.0, 18_000);
        for i in [1, 2, 3, 7] {
            assert_eq!(along_reach(&p, 200.0, i), STRETCH_MM, "{i}");
        }
        // On a scene 60 m wide at 600 px they are 1.8 px apart at the near end: the odd ones are not drawn, and the even
        // ones, 3.6 px apart, only as far as they stay 2 px apart.
        let p = Projection::fit(600.0, 60_000);
        assert_eq!(along_reach(&p, 200.0, 1), 0.0);
        assert!(along_reach(&p, 200.0, 2) > 0.0 && along_reach(&p, 200.0, 2) < STRETCH_MM);
        // A line on a multiple of four lasts at least as far, and the line at the edge of the grid always goes to the end.
        assert!(along_reach(&p, 200.0, 4) >= along_reach(&p, 200.0, 2));
        assert_eq!(along_reach(&p, 200.0, 0), STRETCH_MM);
        // Where it is cut the neighbours are 2 px apart.
        let z = along_reach(&p, 200.0, 2);
        assert!((400.0 * p.scale_at(z) - MIN_JOINT_GAP_PX).abs() < 1e-6);
    }

    #[test]
    fn a_running_bond_has_ticks_between_the_units_of_each_course_only_where_the_courses_are_not_thinned() {
        let (p, h) = sidewalk();
        let j = joints("brick").unwrap();
        let segs = segments(&path(&p, &j, 0.0, 3_300.0, h));
        let ticks: Vec<&Seg> = segs.iter().filter(|s| s.0.1 != s.1.1).collect();
        assert!(!ticks.is_empty());
        // Each tick is one course long on the screen (its two ends at the course's two edges), at least 2 px: the courses
        // it is drawn between are the ones nothing thinned.
        for t in &ticks {
            assert!((t.0.1 - t.1.1).abs() >= MIN_JOINT_GAP_PX - 0.15, "{t:?}");
        }
        // The first course has them 200 mm apart, from 200 to 3 200 mm, in the unit's own positions: a half unit out in
        // the second.
        let first: Vec<f64> = ticks.iter().filter(|t| (t.0.1 - p.at(0.0, h, 0.0).1).abs() < 0.06).map(|t| t.0.0).collect();
        let want: Vec<f64> = (1..=16).map(|k| p.at(200.0 * k as f64, h, 0.0).0).collect();
        assert_eq!(first.len(), want.len());
        assert!(first.iter().zip(&want).all(|(a, b)| (a - b).abs() < 0.06), "{first:?}");
        let second: Vec<f64> = ticks.iter().filter(|t| (t.0.1 - p.at(0.0, h, 100.0).1).abs() < 0.06).map(|t| t.0.0).collect();
        assert_eq!(second.len(), 16);
        assert!(second.iter().zip((0..16).map(|k| p.at(100.0 + 200.0 * k as f64, h, 100.0).0)).all(|(a, b)| (a - b).abs() < 0.06), "{second:?}");
        // A grid (concrete, the grate) has none: its joints along are lines.
        assert!(segments(&path(&p, &joints("trees").unwrap(), 0.0, 1_500.0, h)).iter().any(|s| s.0.1 != s.1.1));
    }
}
