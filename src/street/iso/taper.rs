//! The transition at a joint as geometry: how each roadway piece of the host's run goes from the host's own section,
//! at the end of the run away from the seam, to the other zone's, at the seam. Pure; it only decides across
//! positions, and `raised` and `paint` draw them.
//!
//! The taper fills the whole run, whatever the transition's true length (schematic; the label gives the length).

use crate::shared::catalogue::is_roadway;
use crate::street::model::{Change, SegView, Taper};

use super::projection::Reach;

/// A piece's `[x0, x1)` across the street, in millimetres, at the end of the run away from the seam and at the seam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Shape {
    pub away: (f64, f64),
    pub seam: (f64, f64),
}

/// One roadway piece that changes: its kind, where the host has it (`None` where only the other zone has it), and its
/// shape.
#[derive(Clone, Debug, PartialEq)]
struct Entry {
    kind: usize,
    host: Option<(i32, i32)>,
    shape: Shape,
}

/// The tapering pieces of one run.
pub(super) struct Draw {
    /// The seam is the run's far end (otherwise its near end).
    seam_far: bool,
    entries: Vec<Entry>,
}

/// The edge of the nearest matched piece, on one side of the boundary, to put a piece that has no place there: the
/// right edge of the matched piece before it, else the left edge of the one after it.
fn anchor(changes: &[Change], at: usize, side: fn(&Change) -> Option<(i32, i32)>) -> Option<f64> {
    let matched = |c: &Change| c.before.is_some() && c.after.is_some();
    let before = changes[..at].iter().rev().find(|c| matched(c)).and_then(side).map(|r| r.1);
    before.or_else(|| changes[at + 1..].iter().find(|c| matched(c)).and_then(side).map(|r| r.0)).map(f64::from)
}

fn f((a, b): (i32, i32)) -> (f64, f64) {
    (f64::from(a), f64::from(b))
}

impl Draw {
    /// The taper of a run whose seam is at its far end (`seam_far`) or near end. Only roadway pieces whose place
    /// differs on the two sides taper: a piece the other zone lacks closes to a point at the seam, a piece the host
    /// lacks opens from a point at the far end.
    pub fn new(taper: &Taper, seam_far: bool) -> Draw {
        let mut entries = Vec::new();
        for (i, c) in taper.changes.iter().enumerate() {
            if !is_roadway(c.kind) || c.before == c.after {
                continue;
            }
            let (Some(away), Some(seam)) = (
                c.before.map(f).or_else(|| anchor(&taper.changes, i, |c| c.before).map(|x| (x, x))),
                c.after.map(f).or_else(|| anchor(&taper.changes, i, |c| c.after).map(|x| (x, x))),
            ) else {
                continue;
            };
            entries.push(Entry { kind: c.kind, host: c.before, shape: Shape { away, seam } });
        }
        Draw { seam_far, entries }
    }

    /// The shape of a piece of the host's run, when it tapers.
    pub fn shape_of(&self, s: &SegView) -> Option<Shape> {
        let range = (s.x_mm, s.x_mm + s.width_mm);
        self.entries.iter().find(|e| e.host == Some(range) && e.kind == s.base_kind).map(|e| e.shape)
    }

    /// The pieces the host does not have, which open in its run: their kind and shape.
    pub fn slivers(&self) -> impl Iterator<Item = (usize, Shape)> + '_ {
        self.entries.iter().filter(|e| e.host.is_none()).map(|e| (e.kind, e.shape))
    }

    /// How far along the taper `along_mm` is: 0 at the end away from the seam, 1 at the seam.
    fn t(&self, reach: Reach, along_mm: f64) -> f64 {
        let t = if self.seam_far { along_mm - reach.near } else { reach.far - along_mm } / reach.len();
        t.clamp(0.0, 1.0)
    }

    /// A shape's edges `along_mm` along the street.
    pub fn edges(&self, shape: Shape, reach: Reach, along_mm: f64) -> (f64, f64) {
        let t = self.t(reach, along_mm);
        let lerp = |a: f64, b: f64| a + (b - a) * t;
        (lerp(shape.away.0, shape.seam.0), lerp(shape.away.1, shape.seam.1))
    }

    /// How a paint line at `x_mm` (host side) is moved at `along_mm`: it follows the roadway piece whose edge it lies
    /// on; a line on no tapering piece's edge stays where it is.
    pub fn line_at(&self, x_mm: i32, reach: Reach, along_mm: f64) -> f64 {
        let seam = self.entries.iter().find_map(|e| match e.host {
            Some((a, _)) if a == x_mm => Some(e.shape.seam.0),
            Some((_, b)) if b == x_mm => Some(e.shape.seam.1),
            _ => None,
        });
        let x = f64::from(x_mm);
        seam.map_or(x, |s| x + (s - x) * self.t(reach, along_mm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::kind_index;

    fn change(id: &str, before: Option<(i32, i32)>, after: Option<(i32, i32)>) -> Change {
        Change { kind: kind_index(id).unwrap(), before, after }
    }

    fn taper(changes: Vec<Change>) -> Taper {
        Taper { host_is_selected: true, length_mm: 50_000, changes }
    }

    const REACH: Reach = Reach { near: 0.0, far: 20_000.0 };

    #[test]
    fn a_piece_the_host_lacks_opens_from_its_neighbours_edge_to_the_other_zones_place() {
        let t = taper(vec![
            change("sidewalk", Some((0, 2000)), Some((0, 2000))),
            change("travel", Some((2000, 5500)), Some((2000, 5500))),
            change("travel", None, Some((5500, 9000))),
            change("sidewalk", Some((5500, 7500)), Some((9000, 11_000))),
        ]);
        let d = Draw::new(&t, true);
        assert_eq!(d.entries.len(), 1, "the unchanged lane does not taper, and the shifted sidewalk is not roadway");
        let (kind, shape) = d.slivers().next().unwrap();
        assert_eq!(kind, kind_index("travel").unwrap());
        assert_eq!(shape, Shape { away: (5500.0, 5500.0), seam: (5500.0, 9000.0) });
        // Seam at the far end: nothing at the near end, everything at the far one.
        assert_eq!(d.edges(shape, REACH, 0.0), (5500.0, 5500.0));
        assert_eq!(d.edges(shape, REACH, 10_000.0), (5500.0, 7250.0));
        assert_eq!(d.edges(shape, REACH, 20_000.0), (5500.0, 9000.0));
    }

    #[test]
    fn a_piece_the_other_zone_lacks_closes_to_a_point_at_the_seam_and_the_seam_may_be_the_near_end() {
        let t = taper(vec![
            change("travel", Some((0, 3500)), Some((0, 3500))),
            change("bike", Some((3500, 5000)), None),
            change("sidewalk", Some((5000, 7000)), Some((3500, 5500))),
        ]);
        let d = Draw::new(&t, false);
        let mut s = crate::street::model::Editor::new(0).view().segments.into_iter().next().unwrap();
        (s.kind, s.base_kind, s.x_mm, s.width_mm) = (kind_index("bike").unwrap(), kind_index("bike").unwrap(), 3500, 1500);
        let shape = d.shape_of(&s).expect("the host's bike lane tapers");
        assert_eq!(shape, Shape { away: (3500.0, 5000.0), seam: (3500.0, 3500.0) });
        // Seam at the near end: full at the far end, a point at the near end.
        assert_eq!(d.edges(shape, REACH, 20_000.0), (3500.0, 5000.0));
        assert_eq!(d.edges(shape, REACH, 0.0), (3500.0, 3500.0));
        // The line on its right edge goes with it; a line elsewhere stays.
        assert_eq!(d.line_at(5000, REACH, 0.0), 3500.0);
        assert_eq!(d.line_at(5000, REACH, 20_000.0), 5000.0);
        assert_eq!(d.line_at(1234, REACH, 0.0), 1234.0);
    }

    #[test]
    fn a_piece_is_found_by_its_own_kind_whatever_the_clock_shows() {
        let t = taper(vec![change("parking", Some((0, 2400)), Some((0, 3000))), change("travel", Some((2400, 5700)), Some((3000, 6300)))]);
        let d = Draw::new(&t, true);
        let mut s = crate::street::model::Editor::new(0).view().segments.into_iter().next().unwrap();
        // a parking lane shown as a bus lane in its morning window
        (s.kind, s.base_kind, s.x_mm, s.width_mm) = (kind_index("bus").unwrap(), kind_index("parking").unwrap(), 0, 2400);
        assert!(d.shape_of(&s).is_some(), "the taper follows the own kind, not the shown one");
    }
}
