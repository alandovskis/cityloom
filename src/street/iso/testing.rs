//! Helpers the tests of the 2.5D share.

use std::rc::Rc;

use crate::shared::catalogue::kind_index;
use crate::shared::i18n::{I18n, Locale};
use crate::street::model::Editor;

use super::iso_svg;

pub fn en() -> Rc<I18n> {
    crate::i18n_for(Locale::En)
}

pub fn drawn(e: &Editor) -> String {
    iso_svg(&e.view(), &en(), 1050.0).markup
}

pub fn count(s: &str, needle: &str) -> usize {
    s.matches(needle).count()
}

/// The `points` of the top face of the piece `uid`.
pub fn top_of(markup: &str, uid: u32) -> String {
    let group = markup.split(&format!("<g class=\"iso-piece\" data-uid=\"{uid}\">")).nth(1).expect("the piece is drawn");
    let face = &group[group.find("class=\"iso-top ").expect("a top face")..];
    let at = face.find("points=\"").unwrap() + 8;
    face[at..at + face[at..].find('"').unwrap()].to_string()
}

/// Every x of every polygon's points.
pub fn xs(markup: &str) -> Vec<f64> {
    markup
        .split("points=\"")
        .skip(1)
        .flat_map(|rest| rest.split('"').next().unwrap().split(' ').map(|pt| pt.split(',').next().unwrap().parse::<f64>().unwrap()).collect::<Vec<_>>())
        .collect()
}

/// Every point of every polygon.
pub fn xs_ys(markup: &str) -> Vec<(f64, f64)> {
    markup
        .split("points=\"")
        .skip(1)
        .flat_map(|rest| {
            rest.split('"')
                .next()
                .unwrap()
                .split(' ')
                .map(|pt| {
                    let mut c = pt.split(',').map(|n| n.parse::<f64>().unwrap());
                    (c.next().unwrap(), c.next().unwrap())
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The convex hull of points on the drawing, counter-clockwise (monotone chain).
pub fn hull(mut pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut out: Vec<(f64, f64)> = Vec::new();
    for pass in 0..2 {
        let start = out.len();
        let iter: Box<dyn Iterator<Item = &(f64, f64)>> = if pass == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
        for &q in iter {
            while out.len() >= start + 2 && cross(out[out.len() - 2], out[out.len() - 1], q) <= 0.0 {
                out.pop();
            }
            out.push(q);
        }
        out.pop();
    }
    out
}

/// Two convex shapes on the drawing do not overlap: some edge of one separates them (separating axes).
pub fn apart(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    [a, b].iter().any(|shape| {
        (0..shape.len()).any(|k| {
            let (p, q) = (shape[k], shape[(k + 1) % shape.len()]);
            let n = (q.1 - p.1, p.0 - q.0);
            let span = |s: &[(f64, f64)]| s.iter().map(|v| v.0 * n.0 + v.1 * n.1).fold((f64::MAX, f64::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)));
            let ((a0, a1), (b0, b1)) = (span(a), span(b));
            a1 < b0 || b1 < a0
        })
    })
}

/// Sample Street 1 with two one-way bike lanes, each 1.8 m and curbed: one between the driving lanes (a curb on
/// each edge) and one between the right parking lane and the sidewalk (a curb on its left, the sidewalk raised on
/// its right).
pub fn one_way_bikes() -> (Editor, u32, u32) {
    let mut e = Editor::new(0);
    let bike = kind_index("bike").unwrap();
    let middle = e.add(bike, 3);
    let kerbside = e.add(bike, 6);
    assert!(e.set_direction(middle, Some(0)) && e.set_direction(kerbside, Some(0)));
    (e, middle, kerbside)
}

/// A street with things to stand on its pieces: Sample Avenue 2 (two planting strips of trees) with a
/// bike lane, whose curb gets posts, and a bus shelter, and a piece selected.
pub fn furnished() -> Editor {
    let mut e = Editor::new(1);
    e.add(kind_index("bike").unwrap(), 3);
    e.add(kind_index("busshelter").unwrap(), 1);
    let pick = e.view().segments[4].uid;
    e.select(Some(pick));
    e
}
