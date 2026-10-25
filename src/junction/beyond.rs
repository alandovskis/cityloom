//! An arm's end zone cut at its boundary, and what lies beyond: the taper (a quadrilateral per roadway piece that
//! moves) and the next zone's pieces, to the end of the drawn arm. Pure functions on millimetres; the zone arithmetic
//! (where the boundary stands, how long the taper is, which pieces differ) is `street::model::beyond`'s, and this
//! only draws it, at true scale, cut where the arm ends.

use crate::junction::geometry::{P, at, poly, strip};
use crate::junction::model::{Piece, Profile};
use crate::junction::read_model::PieceView;
use crate::street::model::{Beyond, TaperPiece};

/// What an arm draws for its boundary.
#[derive(Debug, Default)]
pub struct Drawn {
    /// Where each end-zone piece's strip ends, by index into the profile's pieces: the whole arm (`len`) where
    /// nothing is drawn beyond, else the start of the taper for a piece that moves and the boundary for the rest.
    pub cuts: Vec<f64>,
    /// The next zone's strips, from the end of the taper (a piece that moves) or the boundary to `len`.
    pub extra: Vec<PieceView>,
    /// One quadrilateral per piece that moves.
    pub taper: Vec<PieceView>,
    /// The lines across the arm's property width at the taper's two ends (at the boundary alone when it is a jog).
    pub marks: Vec<[P; 2]>,
}

fn span(p: &Piece) -> (i32, i32) {
    (p.x_mm, p.x_mm + p.width_mm)
}

/// The arm's pieces up to their boundary and what follows it, for an arm of the given `bearing` whose pieces lie
/// `lat(x_mm)` to the right of its axis, drawn to `len` from the junction's centre. Nothing is drawn beyond when the
/// boundary, or the taper's start, is at or past `len`.
pub fn draw(bearing: f64, lat: &dyn Fn(i32) -> f64, base: &Profile, next: &Profile, beyond: &Beyond, len: f64) -> Drawn {
    let boundary = f64::from(beyond.boundary_mm);
    let nothing = || Drawn { cuts: vec![len; base.pieces.len()], ..Drawn::default() };
    if boundary >= len || beyond.taper.as_ref().is_some_and(|t| f64::from(t.from_mm) >= len) {
        return nothing();
    }
    let moving = beyond.taper.as_ref().map_or(&[][..], |t| &t.pieces[..]);
    // The taper's ends; a jog is a taper of no length at the boundary.
    let (ta, tb) = beyond.taper.as_ref().map_or((boundary, boundary), |t| (f64::from(t.from_mm), f64::from(t.to_mm)));
    let moves_here = |p: &Piece| moving.iter().any(|m| m.was == Some(span(p)));
    let moves_there = |p: &Piece| moving.iter().any(|m| m.becomes == Some(span(p)));
    let cuts = base.pieces.iter().map(|p| if moves_here(p) { ta } else { boundary }).collect();
    let extra = next
        .pieces
        .iter()
        .filter_map(|p| {
            let t0 = if moves_there(p) { tb } else { boundary };
            let (x0, x1) = (lat(p.x_mm), lat(p.x_mm + p.width_mm));
            (t0 < len).then(|| PieceView {
                kind: p.kind,
                material: p.material,
                direction: p.direction,
                poly: poly(&strip(bearing, x0, x1, t0, len)),
                taper: false,
            })
        })
        .collect();
    let taper = if tb > ta { moving.iter().filter_map(|m| quad(bearing, lat, base, next, m, ta, tb, len)).collect() } else { Vec::new() };
    let across = |t: f64| [at(bearing, lat(0), t), at(bearing, lat(base.row_mm), t)];
    let marks = if tb > ta { vec![ta, tb] } else { vec![boundary] }.into_iter().filter(|&t| t < len).map(across).collect();
    Drawn { cuts, extra, taper, marks }
}

/// One roadway piece's taper over `ta..tb`, cut at `len`: its edges move straight from where the piece is at `ta` to
/// where it is at `tb`, so a taper cut short is the part of the same slope, not a steeper one. In the end-zone piece's
/// material, or the zone across's where the end zone has none.
#[allow(clippy::too_many_arguments)]
fn quad(bearing: f64, lat: &dyn Fn(i32) -> f64, base: &Profile, next: &Profile, m: &TaperPiece, ta: f64, tb: f64, len: f64) -> Option<PieceView> {
    let own = m.was.and_then(|w| base.pieces.iter().find(|p| span(p) == w)).or_else(|| m.becomes.and_then(|w| next.pieces.iter().find(|p| span(p) == w)))?;
    let te = tb.min(len);
    let f = (te - ta) / (tb - ta);
    let (a0, a1) = (lat(m.from.0), lat(m.from.1));
    let (b0, b1) = (a0 + (lat(m.to.0) - a0) * f, a1 + (lat(m.to.1) - a1) * f);
    let corners = [at(bearing, a0, ta), at(bearing, a1, ta), at(bearing, b1, te), at(bearing, b0, te)];
    Some(PieceView { kind: m.kind, material: own.material, direction: own.direction, poly: poly(&corners), taper: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::unlabelled;
    use crate::shared::catalogue::kind_index;
    use crate::street::model::BeyondTaper;
    use serde_json::Value;

    const LEN: f64 = 36_000.0;
    const ROW: i32 = 13_000;

    fn kind(id: &str) -> usize {
        kind_index(id).unwrap()
    }

    /// A profile of pieces `(kind, x0, x1)` across a 13 m property width.
    fn profile(pieces: &[(&str, i32, i32)]) -> Profile {
        let pieces = pieces
            .iter()
            .map(|&(k, x0, x1)| Piece { kind: kind(k), material: "asphalt", x_mm: x0, width_mm: x1 - x0, direction: (k == "travel").then_some("away") })
            .collect();
        Profile {
            name: unlabelled(),
            row_mm: ROW,
            pieces,
            road_l: 3_000,
            road_r: 10_000,
            enter_x: Vec::new(),
            leave_x: Vec::new(),
            enter_span: (0, 0),
            leave_span: (0, 0),
            park: [0, 0],
            edge_kind: [None, None],
        }
    }

    /// North-bound arm: x on the plan is the lateral position and y is minus the distance out.
    fn lat(x: i32) -> f64 {
        x as f64 - ROW as f64 / 2.0
    }

    /// The distances out of a polygon's lowest and highest points.
    fn along(poly: &[Value]) -> (f64, f64) {
        let ys: Vec<f64> = poly.iter().filter(|c| c[0] != "Z").map(|c| -c[2].as_f64().unwrap()).collect();
        (ys.iter().cloned().fold(f64::MAX, f64::min), ys.iter().cloned().fold(f64::MIN, f64::max))
    }

    /// A polygon's corners on the plan.
    fn corners(poly: &[Value]) -> Vec<(f64, f64)> {
        poly.iter().filter(|c| c[0] != "Z").map(|c| (c[1].as_f64().unwrap(), c[2].as_f64().unwrap())).collect()
    }

    fn piece(p: &Profile, i: usize) -> (i32, i32) {
        span(&p.pieces[i])
    }

    /// The end zone: a sidewalk, a pocket (3.5 m), a through lane (3.5 m), a sidewalk.
    fn pocket_zone() -> Profile {
        profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)])
    }

    /// The zone across, without the pocket: its place is planting.
    fn plain_zone() -> Profile {
        profile(&[("sidewalk", 0, 3_000), ("planting", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)])
    }

    fn pocket_taper(from_mm: i32, to_mm: i32) -> Beyond {
        let was = (3_000, 6_500);
        Beyond {
            boundary_mm: 20_000,
            next: Vec::new(),
            taper: Some(BeyondTaper {
                from_mm,
                to_mm,
                need_mm: to_mm - from_mm,
                room_mm: 80_000,
                merges: true,
                pieces: vec![TaperPiece { kind: kind("travel"), from: was, to: (3_000, 3_000), was: Some(was), becomes: None }],
            }),
        }
    }

    fn drawn(base: &Profile, next: &Profile, b: &Beyond) -> Drawn {
        draw(0.0, &lat, base, next, b, LEN)
    }

    #[test]
    fn a_boundary_past_the_arm_draws_nothing_new() {
        let mut b = pocket_taper(20_000, 30_000);
        b.boundary_mm = 36_000;
        let d = drawn(&pocket_zone(), &plain_zone(), &b);
        assert_eq!(d.cuts, vec![LEN; 4]);
        assert!(d.extra.is_empty() && d.taper.is_empty() && d.marks.is_empty());
        b.boundary_mm = 40_000;
        let d = drawn(&pocket_zone(), &plain_zone(), &b);
        assert_eq!(d.cuts, vec![LEN; 4]);
        assert!(d.extra.is_empty() && d.taper.is_empty() && d.marks.is_empty());
        // a taper that starts at the end of the arm is no taper there
        let mut b = pocket_taper(36_000, 50_000);
        b.boundary_mm = 20_000;
        let d = drawn(&pocket_zone(), &plain_zone(), &b);
        assert_eq!(d.cuts, vec![LEN; 4]);
        assert!(d.extra.is_empty() && d.taper.is_empty() && d.marks.is_empty());
    }

    #[test]
    fn a_pocket_ends_at_the_boundary_and_tapers_out_beyond_it() {
        let (base, next) = (pocket_zone(), plain_zone());
        let d = drawn(&base, &next, &pocket_taper(20_000, 30_000));
        // the end zone ends at B = Ta = 20 000 whether or not its piece tapers
        assert_eq!(d.cuts, vec![20_000.0; 4]);
        let [quad] = &d.taper[..] else { panic!("one quad: {}", d.taper.len()) };
        assert!(quad.taper);
        assert_eq!((quad.kind, quad.material, quad.direction), (kind("travel"), "asphalt", Some("away")), "the end-zone piece's");
        // near edge the lane's width, far edge none; the plan has the lateral position across and minus the distance out
        assert_eq!(corners(&quad.poly), vec![(-3_500.0, -20_000.0), (0.0, -20_000.0), (-3_500.0, -30_000.0), (-3_500.0, -30_000.0)]);
        // the zone across: four strips from B to the end of the arm; none of them is a taper
        assert_eq!(d.extra.len(), 4);
        assert!(d.extra.iter().all(|p| !p.taper));
        assert!(d.extra.iter().all(|p| along(&p.poly) == (20_000.0, LEN)));
        assert_eq!(d.extra[1].kind, kind("planting"));
    }

    #[test]
    fn a_lane_added_by_the_next_zone_opens_inside_the_end_zone() {
        // the end zone's piece (6.5 to 10 m across) is narrower once the lane opens; the taper is hosted by the end zone
        let base = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)]);
        let next = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 5_000), ("travel", 5_000, 10_000), ("sidewalk", 10_000, 13_000)]);
        let moves = TaperPiece { kind: kind("travel"), from: (6_500, 10_000), to: (5_000, 10_000), was: Some((6_500, 10_000)), becomes: Some((5_000, 10_000)) };
        let b = Beyond {
            boundary_mm: 20_000,
            next: Vec::new(),
            taper: Some(BeyondTaper { from_mm: 12_000, to_mm: 20_000, need_mm: 8_000, room_mm: 20_000, merges: true, pieces: vec![moves] }),
        };
        let d = drawn(&base, &next, &b);
        // the piece that moves ends at B - need; every other end-zone piece runs to B
        assert_eq!(d.cuts, vec![20_000.0, 20_000.0, 12_000.0, 20_000.0]);
        let [quad] = &d.taper[..] else { panic!("one quad") };
        assert_eq!(corners(&quad.poly), vec![(0.0, -12_000.0), (3_500.0, -12_000.0), (3_500.0, -20_000.0), (-1_500.0, -20_000.0)]);
        assert_eq!(along(&quad.poly), (12_000.0, 20_000.0));
        // the piece across that moves begins where the taper ends, here at B
        let starts: Vec<f64> = d.extra.iter().map(|p| along(&p.poly).0).collect();
        assert_eq!(starts, vec![20_000.0; 4]);
    }

    #[test]
    fn a_taper_running_past_the_arm_is_cut_not_squeezed() {
        // 20 m to 52.5 m, the arm ends at 36 m: 16 of the 32.5 m, so 16/32.5 of the lane's 3.5 m is gone
        let d = drawn(&pocket_zone(), &plain_zone(), &pocket_taper(20_000, 52_500));
        let [quad] = &d.taper[..] else { panic!("one quad") };
        let c = corners(&quad.poly);
        assert_eq!(c.len(), 4);
        assert_eq!((c[0], c[1]), ((-3_500.0, -20_000.0), (0.0, -20_000.0)));
        let kept = 3_500.0 * (1.0 - 16_000.0 / 32_500.0);
        assert_eq!(c[2].1, -LEN);
        assert!((c[2].0 - (-3_500.0 + kept)).abs() <= 1.0, "right edge at {} wanted {}", c[2].0, -3_500.0 + kept);
        assert_eq!(c[3], (-3_500.0, -LEN));
        assert_eq!(along(&quad.poly).1, LEN, "no point past the arm");
        // the zone across is not reached: its pieces that jog begin at B, those that move would begin at Tb
        assert!(d.extra.iter().all(|p| along(&p.poly).1 <= LEN));
    }

    #[test]
    fn non_roadway_pieces_jog_at_the_boundary() {
        let d = drawn(&pocket_zone(), &plain_zone(), &pocket_taper(20_000, 30_000));
        assert_eq!(d.cuts[0], 20_000.0, "the end zone's sidewalk");
        assert_eq!(d.cuts[3], 20_000.0);
        assert_eq!(along(&d.extra[0].poly).0, 20_000.0, "the sidewalk across");
        assert_eq!(along(&d.extra[1].poly).0, 20_000.0, "the planting across");
    }

    #[test]
    fn a_piece_across_that_moves_begins_where_the_taper_ends() {
        // the end zone loses a lane beside its through lane: the through lane widens across the boundary
        let base = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)]);
        let next = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 5_000), ("travel", 5_000, 10_000), ("sidewalk", 10_000, 13_000)]);
        let moves = TaperPiece { kind: kind("travel"), from: (6_500, 10_000), to: (5_000, 10_000), was: Some((6_500, 10_000)), becomes: Some((5_000, 10_000)) };
        let b = Beyond {
            boundary_mm: 20_000,
            next: Vec::new(),
            taper: Some(BeyondTaper { from_mm: 20_000, to_mm: 30_000, need_mm: 10_000, room_mm: 80_000, merges: true, pieces: vec![moves] }),
        };
        let d = drawn(&base, &next, &b);
        assert_eq!(d.cuts[2], 20_000.0);
        let starts: Vec<f64> = d.extra.iter().map(|p| along(&p.poly).0).collect();
        assert_eq!(starts, vec![20_000.0, 20_000.0, 30_000.0, 20_000.0], "only the piece that moves waits for the taper's end");
        assert_eq!(piece(&next, 2), (5_000, 10_000));
    }

    #[test]
    fn the_marks_stand_at_the_taper_ends() {
        let d = drawn(&pocket_zone(), &plain_zone(), &pocket_taper(20_000, 30_000));
        // across the property width, 13 m, at Ta and at Tb
        assert_eq!(d.marks.len(), 2);
        let round = |m: &[P; 2]| [(m[0].0.round(), m[0].1.round()), (m[1].0.round(), m[1].1.round())];
        assert_eq!(round(&d.marks[0]), [(-6_500.0, -20_000.0), (6_500.0, -20_000.0)]);
        assert_eq!(round(&d.marks[1]), [(-6_500.0, -30_000.0), (6_500.0, -30_000.0)]);
        // a jog has one, at the boundary
        let mut jog = pocket_taper(20_000, 30_000);
        jog.taper = None;
        let d = drawn(&pocket_zone(), &plain_zone(), &jog);
        assert_eq!(d.marks.len(), 1);
        assert_eq!(round(&d.marks[0]), [(-6_500.0, -20_000.0), (6_500.0, -20_000.0)]);
        // and Tb past the arm is dropped
        let d = drawn(&pocket_zone(), &plain_zone(), &pocket_taper(20_000, 52_500));
        assert_eq!(d.marks.len(), 1);
    }

    #[test]
    fn a_jog_cuts_every_end_zone_piece_at_the_boundary() {
        let mut jog = pocket_taper(20_000, 30_000);
        jog.taper = None;
        let d = drawn(&pocket_zone(), &plain_zone(), &jog);
        assert_eq!(d.cuts, vec![20_000.0; 4]);
        assert!(d.taper.is_empty());
        assert_eq!(d.extra.len(), 4);
    }
}
