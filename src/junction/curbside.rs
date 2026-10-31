//! The street's curbside uses on an arm, as polygons: each use near the junction is a strip on the arm's curbside piece,
//! drawn in the use's own kind and material. Pure functions on millimetres; which uses an arm has, and on which piece,
//! is `street::model::curb_end`'s, and this only draws them, at true scale, cut to the part of the arm that is drawn.

use serde::Serialize;
use serde_json::Value;

use crate::junction::geometry::{poly, strip};
use crate::junction::read_model::ArmLayout;
use crate::shared::catalogue::{KINDS, Side, kind_index};
use crate::street::model::{EndUse, Use};

/// How near a pedestrian crossing parking or loading may stand: the street page's rule
/// (`street::model::CROSSING_CLEARANCE_MM`), read by the junction against the crossing it draws.
/// UNVERIFIED: the Code de la sécurité routière article is not checked.
pub use crate::street::model::CROSSING_CLEARANCE_MM;

/// One curbside use on an arm, as the plan and the 2.5D draw it and the check reads it.
#[derive(Debug, Serialize)]
pub struct CurbUseView {
    /// The use's index in its street's `curb`.
    pub index: usize,
    pub use_: Use,
    /// The side of the arm it stands on, looking out from the centre.
    pub side: Side,
    /// The catalogue kind and material it is drawn in.
    pub kind: usize,
    pub material: &'static str,
    /// Millimetres from the junction's centre to the use's near edge.
    pub near_mm: i32,
    /// How far past the crossing's far edge (the mouth with no crossing) the near edge stands; negative inside it. Set
    /// by the caller, which knows the crossing.
    pub gap_mm: i32,
    /// The strip, only where it lies on the drawn arm and on its piece; `None` draws nothing.
    pub poly: Option<Vec<Value>>,
    /// A check names the use: parking or loading standing less than `CROSSING_CLEARANCE_MM` past the crossing.
    pub bad: bool,
}

/// The kind a use is drawn as, and its material: the use's own, else the kind's first.
pub fn look(u: Use) -> Option<(usize, &'static str)> {
    let kind = kind_index(u.kind_id())?;
    let material = match u.material() {
        Some(id) => crate::shared::catalogue::MATERIALS.iter().find(|m| m.id == id)?.id,
        None => crate::shared::catalogue::MATERIALS[*KINDS[kind].materials.first()?].id,
    };
    Some((kind, material))
}

/// The use `u` on the arm `l` (shifted `off` sideways), drawn from `floor` (where its piece's strip begins: nothing is
/// drawn nearer, so a use never lies on the crossing or the road) to `len` from the centre; `cut(i)` is where the
/// strip of the section's piece `i` ends. `None` for a use with no kind to draw (never happens for the catalogue's
/// uses). A use whose side has no curbside piece, or that lies wholly outside `floor..end`, has no polygon.
pub fn draw(l: &ArmLayout, off: f64, u: &EndUse, floor: f64, cut: &dyn Fn(usize) -> f64, len: f64) -> Option<CurbUseView> {
    let (kind, material) = look(u.use_)?;
    let poly = u.piece.and_then(|i| l.prof.pieces.get(i).map(|p| (i, p))).and_then(|(i, p)| {
        let (from, to) = (f64::from(u.near_mm).max(floor), f64::from(u.far_mm).min(len).min(cut(i)));
        (from < to).then(|| poly(&strip(l.bearing, l.lat(off, p.x_mm), l.lat(off, p.x_mm + p.width_mm), from, to)))
    });
    Some(CurbUseView { index: u.index, use_: u.use_, side: u.side, kind, material, near_mm: u.near_mm, gap_mm: 0, poly, bad: false })
}

/// Whether a use stands where parking or loading must keep clear of a crossing.
pub fn keeps_clear(u: Use) -> bool {
    matches!(u, Use::Parking | Use::Loading)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{Piece, Profile, unlabelled};
    use crate::shared::catalogue::kind_index;

    const MOUTH: f64 = 4_000.0;
    const LEN: f64 = 36_000.0;

    fn profile() -> Profile {
        let rows: [(&str, i32); 4] = [("sidewalk", 3_000), ("parking", 2_000), ("travel", 3_000), ("sidewalk", 3_000)];
        let mut x = 0;
        let pieces: Vec<Piece> = rows
            .iter()
            .map(|&(k, w)| {
                let p = Piece { kind: kind_index(k).unwrap(), material: "asphalt", x_mm: x, width_mm: w, direction: None };
                x += w;
                p
            })
            .collect();
        Profile {
            name: unlabelled(),
            row_mm: 11_000,
            pieces,
            road_l: 3_000,
            road_r: 8_000,
            enter_x: vec![],
            leave_x: vec![],
            enter_span: (0, 0),
            leave_span: (0, 0),
            park: [0, 0],
            edge_kind: [None, None],
        }
    }

    fn layout() -> ArmLayout {
        ArmLayout {
            bearing: 0.0,
            prof: profile(),
            cl: -2_500.0,
            cr: 2_500.0,
            pl: -5_500.0,
            pr: 5_500.0,
            mouth: MOUTH,
            strip0: MOUTH,
            entry: None,
            lane_mm: None,
        }
    }

    fn end_use(near: i32, far: i32, use_: Use, piece: Option<usize>) -> EndUse {
        EndUse { index: 0, side: Side::Left, use_, near_mm: near, far_mm: far, piece }
    }

    fn along(poly: &[Value]) -> (f64, f64) {
        let ys: Vec<f64> = poly.iter().filter(|c| c[0] != "Z").map(|c| -c[2].as_f64().unwrap()).collect();
        (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max))
    }

    fn across(poly: &[Value]) -> (f64, f64) {
        let xs: Vec<f64> = poly.iter().filter(|c| c[0] != "Z").map(|c| c[1].as_f64().unwrap()).collect();
        (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max))
    }

    #[test]
    fn a_use_is_a_strip_on_its_piece() {
        let v = draw(&layout(), 0.0, &end_use(14_000, 20_000, Use::Tree, Some(1)), MOUTH, &|_| LEN, LEN).unwrap();
        let p = v.poly.as_ref().expect("a strip");
        assert_eq!(along(p), (14_000.0, 20_000.0));
        assert_eq!(across(p), (-2_500.0, -500.0), "the parking piece's span");
        assert_eq!(KINDS[v.kind].id, "planting");
        assert_eq!(v.material, "trees");
        assert_eq!((v.near_mm, v.index), (14_000, 0));
    }

    #[test]
    fn a_use_takes_its_kinds_first_material_unless_it_has_its_own() {
        let (k, m) = look(Use::Patio).unwrap();
        assert_eq!(KINDS[k].id, "terrace");
        assert_eq!(m, crate::shared::catalogue::MATERIALS[KINDS[k].materials[0]].id);
        assert_eq!(look(Use::RainGarden).unwrap().1, "planted");
    }

    #[test]
    fn nothing_is_drawn_before_the_floor_or_past_the_arm_or_its_cut() {
        let l = layout();
        let v = draw(&l, 0.0, &end_use(2_000, 20_000, Use::Parking, Some(1)), 9_000.0, &|_| LEN, LEN).unwrap();
        assert_eq!(along(v.poly.as_ref().unwrap()), (9_000.0, 20_000.0), "cut at the floor");
        let v = draw(&l, 0.0, &end_use(30_000, 44_000, Use::Parking, Some(1)), MOUTH, &|_| LEN, LEN).unwrap();
        assert_eq!(along(v.poly.as_ref().unwrap()), (30_000.0, 36_000.0), "cut at the arm's end");
        let v = draw(&l, 0.0, &end_use(14_000, 30_000, Use::Parking, Some(1)), MOUTH, &|_| 20_000.0, LEN).unwrap();
        assert_eq!(along(v.poly.as_ref().unwrap()), (14_000.0, 20_000.0), "cut where the piece's strip ends");
        let v = draw(&l, 0.0, &end_use(1_000, 8_000, Use::Parking, Some(1)), 9_000.0, &|_| LEN, LEN).unwrap();
        assert!(v.poly.is_none(), "wholly before the floor");
    }

    #[test]
    fn a_use_with_no_piece_draws_nothing() {
        let v = draw(&layout(), 0.0, &end_use(14_000, 20_000, Use::Tree, None), MOUTH, &|_| LEN, LEN).unwrap();
        assert!(v.poly.is_none());
    }

    #[test]
    fn only_parking_and_loading_keep_clear_of_a_crossing() {
        assert!(keeps_clear(Use::Parking) && keeps_clear(Use::Loading));
        assert!(!keeps_clear(Use::Tree) && !keeps_clear(Use::Platform) && !keeps_clear(Use::BikeParking));
    }
}
