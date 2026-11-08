//! Pavement paint on the ground plane, in millimetres, with no projection: the street's markings and, later,
//! the turbo roundabout's painted dividers use the same definition, and each slice projects the quads with its
//! own projection.

/// What the paint is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    White,
    Yellow,
}

/// Painted all along, or in dashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Solid,
    /// `DASH_MM` of paint, then `GAP_MM` without, from the start of the path.
    Broken,
}

/// How a line is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub paint: Paint,
    pub pattern: Pattern,
    /// Two lines side by side, `DOUBLE_GAP_MM` apart.
    pub double: bool,
}

/// Four corners on the ground plane, in millimetres: across, along.
pub type Quad = [(f64, f64); 4];

/// How wide a painted line is, in millimetres. Tome V allows 100 to 150; the MTQ network uses 120 to 125.
pub const LINE_MM: f64 = 120.0;
/// The space between the two lines of a double line.
pub const DOUBLE_GAP_MM: f64 = 120.0;
/// The length of a dash of a broken line between lanes of one direction.
pub const DASH_MM: f64 = 3_000.0;
/// The length of the gap between two dashes.
pub const GAP_MM: f64 = 6_000.0;

/// The CSS classes of a paint: the colour is a token of the stylesheet, so that it follows the theme.
pub fn class(paint: Paint) -> &'static str {
    match paint {
        Paint::White => "iso-paint white",
        Paint::Yellow => "iso-paint yellow",
    }
}

/// The quads that paint `path` (a ground-plane polyline, mm) in `style`: dashes cut along its length from its
/// start, a double line as two lines offset ±(LINE_MM + DOUBLE_GAP_MM) / 2 to either side. Each quad lies on
/// its own segment (there is no mitre at a bend), and a segment of no length is skipped.
pub fn quads(path: &[(f64, f64)], style: Style) -> Vec<(Paint, Quad)> {
    let half = (LINE_MM + DOUBLE_GAP_MM) / 2.0;
    let offsets = if style.double { vec![-half, half] } else { vec![0.0] };
    let mut out = Vec::new();
    for off in offsets {
        let mut run = 0.0;
        for w in path.windows(2) {
            let (a, b) = (w[0], w[1]);
            let len = (b.0 - a.0).hypot(b.1 - a.1);
            if len < 1e-9 {
                continue;
            }
            let (ux, uz) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
            // The right-hand normal: for a path running along the street it points across, to the right.
            let (nx, nz) = (uz, -ux);
            for (s0, s1) in spans(style.pattern, run, run + len) {
                let (t0, t1) = (s0 - run, s1 - run);
                let at = |t: f64, side: f64| {
                    let across = off + side * LINE_MM / 2.0;
                    (a.0 + ux * t + nx * across, a.1 + uz * t + nz * across)
                };
                out.push((style.paint, [at(t0, -1.0), at(t1, -1.0), at(t1, 1.0), at(t0, 1.0)]));
            }
            run += len;
        }
    }
    out
}

/// The stretches of the path between the distances `from` and `to` along it that carry paint.
fn spans(pattern: Pattern, from: f64, to: f64) -> Vec<(f64, f64)> {
    match pattern {
        Pattern::Solid => vec![(from, to)],
        Pattern::Broken => {
            let period = DASH_MM + GAP_MM;
            let (first, last) = ((from / period).floor() as i64, (to / period).floor() as i64);
            (first..=last)
                .filter_map(|k| {
                    let start = (k as f64 * period).max(from);
                    let end = (k as f64 * period + DASH_MM).min(to);
                    (end - start > 1e-9).then_some((start, end))
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLID: Style = Style { paint: Paint::White, pattern: Pattern::Solid, double: false };
    const BROKEN: Style = Style { paint: Paint::White, pattern: Pattern::Broken, double: false };

    fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    /// How long a quad is along its segment, and how wide.
    fn size(q: &Quad) -> (f64, f64) {
        (dist(q[0], q[1]), dist(q[1], q[2]))
    }

    fn centre_across(q: &Quad) -> f64 {
        q.iter().map(|c| c.0).sum::<f64>() / 4.0
    }

    #[test]
    fn a_solid_line_gives_one_quad_per_path_segment_as_wide_as_the_line() {
        let q = quads(&[(1_000.0, 0.0), (1_000.0, 10_000.0), (1_000.0, 25_000.0)], SOLID);
        assert_eq!(q.len(), 2);
        assert!(q.iter().all(|(p, _)| *p == Paint::White));
        let (a, b) = (size(&q[0].1), size(&q[1].1));
        assert!((a.0 - 10_000.0).abs() < 1e-6 && (b.0 - 15_000.0).abs() < 1e-6, "{a:?} {b:?}");
        assert!((a.1 - LINE_MM).abs() < 1e-6 && (b.1 - LINE_MM).abs() < 1e-6);
        // Corners: start left, end left, end right, start right.
        assert_eq!(q[0].1, [(940.0, 0.0), (940.0, 10_000.0), (1_060.0, 10_000.0), (1_060.0, 0.0)]);
    }

    #[test]
    fn a_broken_line_over_40_m_has_five_dashes_each_3_m_long_9_m_apart() {
        let q = quads(&[(0.0, 0.0), (0.0, 40_000.0)], BROKEN);
        assert_eq!(q.len(), 5);
        for (n, (_, quad)) in q.iter().enumerate() {
            assert!((size(quad).0 - DASH_MM).abs() < 1e-6, "dash {n}: {:?}", size(quad));
            assert!((quad[0].1 - n as f64 * (DASH_MM + GAP_MM)).abs() < 1e-6, "dash {n} starts at {}", quad[0].1);
        }
    }

    #[test]
    fn the_last_dash_is_cut_where_the_path_ends_and_a_dash_runs_on_round_a_vertex() {
        // 37 m: the fifth dash would run from 36 m to 39 m and is cut at 37 m.
        let q = quads(&[(0.0, 0.0), (0.0, 37_000.0)], BROKEN);
        assert_eq!(q.len(), 5);
        assert!((size(&q[4].1).0 - 1_000.0).abs() < 1e-6);
        // A dash from 0 to 3 m over a vertex at 2 m is two quads, 2 m and 1 m long.
        let bent = quads(&[(0.0, 0.0), (0.0, 2_000.0), (0.0, 6_000.0)], BROKEN);
        assert_eq!(bent.len(), 2);
        assert!((size(&bent[0].1).0 - 2_000.0).abs() < 1e-6 && (size(&bent[1].1).0 - 1_000.0).abs() < 1e-6);
    }

    #[test]
    fn a_double_line_is_two_lines_120_mm_either_side_of_the_centre() {
        let style = Style { paint: Paint::Yellow, pattern: Pattern::Solid, double: true };
        let q = quads(&[(1_000.0, 0.0), (1_000.0, 40_000.0)], style);
        assert_eq!(q.len(), 2);
        assert!(q.iter().all(|(p, _)| *p == Paint::Yellow));
        let mut xs: Vec<f64> = q.iter().map(|(_, quad)| centre_across(quad)).collect();
        xs.sort_by(f64::total_cmp);
        assert!((xs[0] - 880.0).abs() < 1e-6 && (xs[1] - 1_120.0).abs() < 1e-6, "{xs:?}");
        assert!(((LINE_MM + DOUBLE_GAP_MM) / 2.0 - 120.0).abs() < 1e-12);
        // Broken and double: both lines are cut the same way.
        let both = quads(&[(1_000.0, 0.0), (1_000.0, 40_000.0)], Style { pattern: Pattern::Broken, ..style });
        assert_eq!(both.len(), 10);
    }

    #[test]
    fn a_bent_path_keeps_each_quad_on_its_own_segment() {
        let q = quads(&[(0.0, 0.0), (0.0, 1_000.0), (1_000.0, 1_000.0)], SOLID);
        assert_eq!(q.len(), 2);
        let near = |p: (f64, f64), x: (f64, f64), z: (f64, f64)| {
            p.0 >= x.0 - 60.0 - 1e-9 && p.0 <= x.1 + 60.0 + 1e-9 && p.1 >= z.0 - 60.0 - 1e-9 && p.1 <= z.1 + 60.0 + 1e-9
        };
        assert!(q[0].1.iter().all(|&c| near(c, (0.0, 0.0), (0.0, 1_000.0))), "{:?}", q[0].1);
        assert!(q[1].1.iter().all(|&c| near(c, (0.0, 1_000.0), (1_000.0, 1_000.0))), "{:?}", q[1].1);
    }

    #[test]
    fn no_path_no_paint_and_a_zero_length_segment_is_skipped() {
        assert!(quads(&[], SOLID).is_empty());
        assert!(quads(&[(5.0, 5.0)], SOLID).is_empty());
        assert!(quads(&[(5.0, 5.0), (5.0, 5.0)], SOLID).is_empty());
        assert_eq!(quads(&[(0.0, 0.0), (0.0, 0.0), (0.0, 100.0)], SOLID).len(), 1);
    }

    #[test]
    fn each_paint_has_its_class() {
        assert_eq!(class(Paint::White), "iso-paint white");
        assert_eq!(class(Paint::Yellow), "iso-paint yellow");
    }
}
