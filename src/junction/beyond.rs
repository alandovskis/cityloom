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

/// One place across the arm, in the end zone and in the zone across: the piece that is there on each side (none where it
/// appears or disappears) and its span across at the taper's start (`a`) and end (`b`). A side with no piece is a
/// zero-width span at the running edge, so the `a`s tile the width, and so do the `b`s.
struct Place {
    base: Option<usize>,
    next: Option<usize>,
    a: (i32, i32),
    b: (i32, i32),
}

impl Place {
    /// Whether the place changes across the taper. One that does not is a strip that jogs at the boundary.
    fn moves(&self) -> bool {
        self.a != self.b
    }
}

/// Pairs the end zone's pieces with the zone across's, left to right. The taper's pieces say which roadway piece
/// becomes which; the rest are the same kind, or a kind changing in place (a jog), or else appear or disappear.
fn places(base: &Profile, next: &Profile, moving: &[TaperPiece]) -> Vec<Place> {
    let was = |x: (i32, i32)| moving.iter().any(|m| m.was == Some(x));
    let becomes = |x: (i32, i32)| moving.iter().any(|m| m.becomes == Some(x));
    let (mut i, mut j, mut ra, mut rb) = (0, 0, 0, 0);
    let mut out = Vec::new();
    while i < base.pieces.len() || j < next.pieces.len() {
        let pair = (base.pieces.get(i), next.pieces.get(j));
        let (take_base, take_next) = match pair {
            (Some(x), Some(y)) => {
                let (sx, sy) = (span(x), span(y));
                let told = moving.iter().any(|m| m.was == Some(sx) && m.becomes == Some(sy));
                let free = !was(sx) && !becomes(sy);
                if told || (free && (x.kind == y.kind || sx == sy)) {
                    (true, true)
                } else if becomes(sy) {
                    (false, true)
                } else if free && sy.0 < sx.0 {
                    // nothing accounts for it and it lies before the end zone's piece: it appears
                    (false, true)
                } else {
                    // the end zone's piece has no counterpart here: it disappears
                    (true, false)
                }
            }
            (Some(_), None) => (true, false),
            _ => (false, true),
        };
        let a = if take_base { span(&base.pieces[i]) } else { (ra, ra) };
        let b = if take_next { span(&next.pieces[j]) } else { (rb, rb) };
        (ra, rb) = (a.1, b.1);
        out.push(Place { base: take_base.then_some(i), next: take_next.then_some(j), a, b });
        i += usize::from(take_base);
        j += usize::from(take_next);
    }
    out
}

/// The arm's pieces up to their boundary and what follows it, for an arm of the given `bearing` whose pieces lie
/// `lat(x_mm)` to the right of its axis, drawn from `floor` (where its roadway strips begin, so nothing reaches into
/// the junction's core) to `len`, both from the junction's centre. Nothing is drawn beyond when the boundary, or the
/// taper's start, is at or past `len`.
///
/// Over the taper every lateral position is in exactly one quad (or one strip of a piece that does not move); before
/// it, in one end-zone strip, and after it, in one strip of the zone across.
pub fn draw(bearing: f64, lat: &dyn Fn(i32) -> f64, base: &Profile, next: &Profile, beyond: &Beyond, floor: f64, len: f64) -> Drawn {
    let boundary = f64::from(beyond.boundary_mm).max(floor);
    let nothing = || Drawn { cuts: vec![len; base.pieces.len()], ..Drawn::default() };
    if boundary >= len || beyond.taper.as_ref().is_some_and(|t| f64::from(t.from_mm).max(floor) >= len) {
        return nothing();
    }
    let moving = beyond.taper.as_ref().map_or(&[][..], |t| &t.pieces[..]);
    // The taper's ends; a jog is a taper of no length at the boundary.
    let (ta, tb) = beyond.taper.as_ref().map_or((boundary, boundary), |t| (f64::from(t.from_mm).max(floor), f64::from(t.to_mm).max(floor)));
    let places = places(base, next, moving);
    let mut cuts = vec![boundary; base.pieces.len()];
    let (mut extra, mut taper) = (Vec::new(), Vec::new());
    for p in &places {
        if let Some(i) = p.base.filter(|_| p.moves()) {
            cuts[i] = ta;
        }
        if let Some(j) = p.next {
            let t0 = if p.moves() { tb } else { boundary };
            let n = &next.pieces[j];
            if t0 < len {
                let (x0, x1) = (lat(n.x_mm), lat(n.x_mm + n.width_mm));
                extra.push(PieceView {
                    kind: n.kind,
                    material: n.material,
                    direction: n.direction,
                    poly: poly(&strip(bearing, x0, x1, t0, len)),
                    taper: false,
                });
            }
        }
        if p.moves() && tb > ta {
            // The end zone's piece, else the zone across's, gives the quad its kind and material (a place always has
            // one: `places` makes a place of a piece of one of them).
            let own = p.base.map(|i| &base.pieces[i]).or_else(|| p.next.map(|j| &next.pieces[j]));
            if let Some(own) = own {
                let te = tb.min(len);
                let f = (te - ta) / (tb - ta);
                // Edges move straight from where they are at `ta` to where they are at `tb`: a taper cut short is
                // the same slope, cut.
                let edge = |from: i32, to: i32| lat(from) + (lat(to) - lat(from)) * f;
                let corners =
                    [at(bearing, lat(p.a.0), ta), at(bearing, lat(p.a.1), ta), at(bearing, edge(p.a.1, p.b.1), te), at(bearing, edge(p.a.0, p.b.0), te)];
                taper.push(PieceView { kind: own.kind, material: own.material, direction: own.direction, poly: poly(&corners), taper: true });
            }
        }
    }
    // The marks run across the property width, which the end zone and the zone across share (one `row_mm`).
    let across = |t: f64| [at(bearing, lat(0), t), at(bearing, lat(base.row_mm), t)];
    let marks = if tb > ta { vec![ta, tb] } else { vec![boundary] }.into_iter().filter(|&t| t > floor && t < len).map(across).collect();
    Drawn { cuts, extra, taper, marks }
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
        draw(0.0, &lat, base, next, b, 0.0, LEN)
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
        // the pocket narrows to nothing and the planting that takes its place widens from nothing
        let [quad, filler] = &d.taper[..] else { panic!("two quads: {}", d.taper.len()) };
        assert!(quad.taper && filler.taper);
        assert_eq!(corners(&filler.poly), vec![(0.0, -20_000.0), (0.0, -20_000.0), (0.0, -30_000.0), (-3_500.0, -30_000.0)]);
        assert_eq!((filler.kind, filler.material), (kind("planting"), "asphalt"), "the zone across's piece has no end-zone one");
        assert_eq!((quad.kind, quad.material, quad.direction), (kind("travel"), "asphalt", Some("away")), "the end-zone piece's");
        // near edge the lane's width, far edge none; the plan has the lateral position across and minus the distance out
        assert_eq!(corners(&quad.poly), vec![(-3_500.0, -20_000.0), (0.0, -20_000.0), (-3_500.0, -30_000.0), (-3_500.0, -30_000.0)]);
        // the zone across: four strips to the end of the arm, none a taper; the planting waits for the taper's end
        assert_eq!(d.extra.len(), 4);
        assert!(d.extra.iter().all(|p| !p.taper));
        let starts: Vec<f64> = d.extra.iter().map(|p| along(&p.poly).0).collect();
        assert_eq!(starts, vec![20_000.0, 30_000.0, 20_000.0, 20_000.0]);
        assert!(d.extra.iter().all(|p| along(&p.poly).1 == LEN));
        assert_eq!(d.extra[1].kind, kind("planting"));
    }

    /// A lane opens inside the end zone, taking its width from the planting beside it.
    fn lane_opens() -> (Profile, Profile, Beyond) {
        let base = profile(&[("sidewalk", 0, 3_000), ("planting", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)]);
        let next = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)]);
        let opens = TaperPiece { kind: kind("travel"), from: (3_000, 3_000), to: (3_000, 6_500), was: None, becomes: Some((3_000, 6_500)) };
        let b = Beyond {
            boundary_mm: 20_000,
            next: Vec::new(),
            taper: Some(BeyondTaper { from_mm: 12_000, to_mm: 20_000, need_mm: 8_000, room_mm: 20_000, merges: true, pieces: vec![opens] }),
        };
        (base, next, b)
    }

    #[test]
    fn a_lane_added_by_the_next_zone_opens_inside_the_end_zone() {
        let (base, next, b) = lane_opens();
        let d = drawn(&base, &next, &b);
        // the planting that gives the lane its width ends at Ta and narrows under the taper; the rest run to B
        assert_eq!(d.cuts, vec![20_000.0, 12_000.0, 20_000.0, 20_000.0]);
        let [lane, planting] = &d.taper[..] else { panic!("two quads") };
        assert_eq!((lane.kind, planting.kind), (kind("travel"), kind("planting")));
        assert_eq!(corners(&lane.poly), vec![(-3_500.0, -12_000.0), (-3_500.0, -12_000.0), (0.0, -20_000.0), (-3_500.0, -20_000.0)]);
        assert_eq!(corners(&planting.poly), vec![(-3_500.0, -12_000.0), (0.0, -12_000.0), (0.0, -20_000.0), (0.0, -20_000.0)]);
        // the lane across begins where the taper ends, here at B
        let starts: Vec<f64> = d.extra.iter().map(|p| along(&p.poly).0).collect();
        assert_eq!(starts, vec![20_000.0; 4]);
    }

    #[test]
    fn a_taper_running_past_the_arm_is_cut_not_squeezed() {
        // 20 m to 52.5 m, the arm ends at 36 m: 16 of the 32.5 m, so 16/32.5 of the lane's 3.5 m is gone
        let d = drawn(&pocket_zone(), &plain_zone(), &pocket_taper(20_000, 52_500));
        let [quad, filler] = &d.taper[..] else { panic!("the pocket and the planting that fills it") };
        assert_eq!(along(&filler.poly).1, LEN);
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
        assert_eq!(along(&d.extra[3].poly).0, 20_000.0);
    }

    #[test]
    fn a_piece_across_that_moves_begins_where_the_taper_ends() {
        // the end zone loses a lane beside its through lane: the through lane widens across the boundary
        let base = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)]);
        let next = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 5_000), ("travel", 5_000, 10_000), ("sidewalk", 10_000, 13_000)]);
        let shift = |from: (i32, i32), to: (i32, i32)| TaperPiece { kind: kind("travel"), from, to, was: Some(from), becomes: Some(to) };
        let b = Beyond {
            boundary_mm: 20_000,
            next: Vec::new(),
            taper: Some(BeyondTaper {
                from_mm: 20_000,
                to_mm: 30_000,
                need_mm: 10_000,
                room_mm: 80_000,
                merges: true,
                pieces: vec![shift((3_000, 6_500), (3_000, 5_000)), shift((6_500, 10_000), (5_000, 10_000))],
            }),
        };
        let d = drawn(&base, &next, &b);
        assert_eq!(d.cuts, vec![20_000.0; 4]);
        let starts: Vec<f64> = d.extra.iter().map(|p| along(&p.poly).0).collect();
        assert_eq!(starts, vec![20_000.0, 30_000.0, 30_000.0, 20_000.0], "only the pieces that move wait for the taper's end");
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

    /// The lateral extents `(x0, x1)` of every drawn polygon that crosses distance `t`, the end zone's strips (from
    /// `start`) included, sorted by where they begin.
    fn widths_at(d: &Drawn, base: &Profile, t: f64) -> Vec<(f64, f64)> {
        let strips: Vec<PieceView> = base
            .pieces
            .iter()
            .zip(&d.cuts)
            .map(|(p, &cut)| PieceView {
                kind: p.kind,
                material: p.material,
                direction: p.direction,
                poly: poly(&strip(0.0, lat(p.x_mm), lat(p.x_mm + p.width_mm), 0.0, cut)),
                taper: false,
            })
            .collect();
        let mut out = Vec::new();
        for p in strips.iter().chain(&d.taper).chain(&d.extra) {
            let c = corners(&p.poly);
            let (lo, hi) = (c.iter().map(|q| -q.1).fold(f64::MAX, f64::min), c.iter().map(|q| -q.1).fold(f64::MIN, f64::max));
            if !(lo < t && t < hi) {
                continue;
            }
            // where the polygon's edges cross the line, the extreme two
            let mut xs = Vec::new();
            for k in 0..c.len() {
                let (p, q) = (c[k], c[(k + 1) % c.len()]);
                let (y0, y1) = (-p.1, -q.1);
                if (y0 <= t && t <= y1 || y1 <= t && t <= y0) && y0 != y1 {
                    xs.push(p.0 + (q.0 - p.0) * (t - y0) / (y1 - y0));
                }
            }
            let (x0, x1) = (xs.iter().cloned().fold(f64::MAX, f64::min), xs.iter().cloned().fold(f64::MIN, f64::max));
            if x1 - x0 > 0.5 {
                out.push((x0, x1));
            }
        }
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out
    }

    /// Every lateral position at distance `t` is in exactly one polygon: the pieces abut and tile the width.
    fn assert_tiles(d: &Drawn, base: &Profile, t: f64) {
        let w = widths_at(d, base, t);
        let mut at = lat(0);
        for (x0, x1) in &w {
            assert!((x0 - at).abs() <= 1.0, "at {t}: a gap or overlap at {at}, next piece begins at {x0}: {w:?}");
            at = *x1;
        }
        assert!((at - lat(ROW)).abs() <= 1.0, "at {t}: covered to {at}: {w:?}");
    }

    #[test]
    fn nothing_drawn_overlaps_and_nothing_is_left_out() {
        let shift = |from: (i32, i32), to: (i32, i32)| TaperPiece { kind: kind("travel"), from, to, was: Some(from), becomes: Some(to) };
        let moving = |pieces: Vec<TaperPiece>, from_mm, to_mm| Beyond {
            boundary_mm: 20_000,
            next: Vec::new(),
            taper: Some(BeyondTaper { from_mm, to_mm, need_mm: to_mm - from_mm, room_mm: 80_000, merges: true, pieces }),
        };
        let (opens_base, opens_next, opens) = lane_opens();
        let shifted_base = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 6_500), ("travel", 6_500, 10_000), ("sidewalk", 10_000, 13_000)]);
        let shifted_next = profile(&[("sidewalk", 0, 3_000), ("travel", 3_000, 5_000), ("travel", 5_000, 10_000), ("sidewalk", 10_000, 13_000)]);
        let shifts = moving(vec![shift((3_000, 6_500), (3_000, 5_000)), shift((6_500, 10_000), (5_000, 10_000))], 20_000, 30_000);
        let cases = [
            ("a pocket", pocket_zone(), plain_zone(), pocket_taper(20_000, 30_000)),
            ("a pocket cut at the arm's end", pocket_zone(), plain_zone(), pocket_taper(20_000, 52_500)),
            ("a lane opening in the end zone", opens_base, opens_next, opens),
            ("a shift", shifted_base, shifted_next, shifts),
        ];
        for (what, base, next, b) in &cases {
            let d = drawn(base, next, b);
            for t in [10_000.0, 19_000.0, 21_000.0, 25_000.0, 29_000.0, 31_000.0, 35_000.0] {
                // the lane opening runs 12 000 to 20 000: its sample points
                let t = if *what == "a lane opening in the end zone" { t - 6_000.0 } else { t };
                assert_tiles(&d, base, t);
            }
        }
    }

    #[test]
    fn a_boundary_nearer_than_the_mouth_draws_nothing_inside_it() {
        let mut b = pocket_taper(2_000, 12_000);
        b.boundary_mm = 2_000;
        let d = draw(0.0, &lat, &pocket_zone(), &plain_zone(), &b, 6_000.0, LEN);
        assert!(d.cuts.iter().all(|&c| c >= 6_000.0), "{:?}", d.cuts);
        for p in d.extra.iter().chain(&d.taper) {
            assert!(along(&p.poly).0 >= 6_000.0, "reaches into the core: {:?}", along(&p.poly));
            assert!(along(&p.poly).0 <= along(&p.poly).1, "inverted");
        }
        assert!(d.marks.iter().all(|m| -m[0].1 > 6_000.0), "a mark at or before the mouth is dropped");
        // the taper that still runs beyond the mouth is drawn from it, on the same slope
        assert_eq!(d.taper.len(), 2);
        assert_eq!(along(&d.taper[0].poly), (6_000.0, 12_000.0));
        // wholly inside the mouth: a jog at the mouth, no quad, no mark
        let mut inside = pocket_taper(2_000, 4_000);
        inside.boundary_mm = 2_000;
        let d = draw(0.0, &lat, &pocket_zone(), &plain_zone(), &inside, 6_000.0, LEN);
        assert!(d.taper.is_empty() && d.marks.is_empty());
        assert!(d.cuts.iter().all(|&c| c == 6_000.0));
        assert!(d.extra.iter().all(|p| along(&p.poly).0 == 6_000.0));
    }
}
