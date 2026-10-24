//! The transition between two adjacent zones' sections: how long the stretch is over which the pieces that differ
//! taper from one section to the other, and which zone carries it. Pure, no browser types.
//!
//! Every taper constant below is unverified (TAC GDG ch. 8–9, MTQ Tome I ch. 8 not checked).

use super::Segment;
use crate::shared::catalogue::{KINDS, is_roadway};

/// The zone that holds the taper: the one without the added or dropped piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    /// The zone nearer the anchored end (`Street::from_end`).
    Near,
    /// The zone farther from it.
    Far,
}

/// One piece across the boundary: its `[x0, x1)` in millimetres from the street's left edge on each side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub kind: usize,
    /// On the near zone's side; `None` where the piece is absent there.
    pub before: Option<(i32, i32)>,
    /// On the far zone's side; `None` where the piece is absent there.
    pub after: Option<(i32, i32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub length_mm: i32,
    pub host: Host,
    /// Every piece of both sides, in left-to-right order of the near section (`before`), a piece only the far section
    /// has among the matched pieces where the far section has it; unmatched pieces included.
    pub changes: Vec<Change>,
    /// False when a travel or bus lane that only one side has has no lane of its direction beside it, on its own
    /// side, to open from or merge into.
    pub merges: bool,
}

/// The speed taken when the street has none, in km/h.
const DEFAULT_SPEED_KMH: i64 = 50;

/// How a kind tapers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Taper {
    /// Travel and bus lanes: a lane added opens at `R·W`; a shift or drop is a merge taper.
    Lane,
    /// Parking, loading, shoulder: a merge taper whichever way.
    Merge,
    /// Bike lane: at least 1:5.
    Bike,
    None,
}

fn taper(kind: usize) -> Taper {
    match KINDS[kind].id {
        "travel" | "bus" => Taper::Lane,
        "parking" | "loading" | "shoulder" => Taper::Merge,
        "bike" => Taper::Bike,
        _ => Taper::None,
    }
}

/// Rounds a length in millimetres up to the next 100 mm.
fn up_100(mm: i64) -> i64 {
    (mm + 99) / 100 * 100
}

/// Shift, width change or dropped lane (travel, bus, parking, loading, shoulder): `L = W·S²/155` below 70 km/h,
/// `L = 0.62·W·S` from 70 (US MUTCD 2003, Part 3, Fig. 3B-12, metric; work-zone merge taper). L and W in metres,
/// S in km/h. Unverified (TAC GDG ch. 8–9, MTQ Tome I ch. 8 not checked). Here in millimetres, rounded up.
fn merge_taper_mm(w_mm: i64, s: i64) -> i64 {
    if s < 70 { (w_mm * s * s + 154) / 155 } else { (w_mm * 62 * s + 99) / 100 }
}

/// A travel or bus lane added: `L = R·W`, R = 15 at up to 50 km/h, 18 up to 60, 24 above; a speed between listed
/// ones takes the next higher's ratio (York Region DS-100, 2023, "based on TAC"). Unverified (TAC GDG ch. 8–9, MTQ
/// Tome I ch. 8 not checked).
fn lane_added_mm(w_mm: i64, s: i64) -> i64 {
    let r = if s <= 50 {
        15
    } else if s <= 60 {
        18
    } else {
        24
    };
    r * w_mm
}

/// A bike lane shifted, added or dropped: `L = 5·W` (NACTO, at least 1:5). Unverified (TAC GDG ch. 8–9, MTQ Tome I
/// ch. 8 not checked).
fn bike_mm(w_mm: i64) -> i64 {
    5 * w_mm
}

fn spans(section: &[Segment]) -> Vec<(i32, i32)> {
    let mut x = 0;
    section
        .iter()
        .map(|s| {
            let r = (x, x + s.width_mm);
            x = r.1;
            r
        })
        .collect()
}

/// The length this one change needs, in millimetres (not rounded).
fn needed_mm(c: &Change, s: i64) -> i64 {
    match (c.before, c.after) {
        (Some(b), Some(a)) => {
            let w = i64::from((b.0 - a.0).abs().max((b.1 - a.1).abs()));
            if w == 0 {
                return 0;
            }
            match taper(c.kind) {
                Taper::Lane | Taper::Merge => merge_taper_mm(w, s),
                Taper::Bike => bike_mm(w),
                Taper::None => 0,
            }
        }
        (None, Some(a)) => {
            let w = i64::from(a.1 - a.0);
            match taper(c.kind) {
                Taper::Lane => lane_added_mm(w, s),
                Taper::Merge => merge_taper_mm(w, s),
                Taper::Bike => bike_mm(w),
                Taper::None => 0,
            }
        }
        (Some(b), None) => {
            let w = i64::from(b.1 - b.0);
            match taper(c.kind) {
                Taper::Lane | Taper::Merge => merge_taper_mm(w, s),
                Taper::Bike => bike_mm(w),
                Taper::None => 0,
            }
        }
        (None, None) => 0,
    }
}

/// The transition from the near zone's section to the far one's, or `None` when nothing needs a taper.
pub fn transition(near: &[Segment], far: &[Segment], speed_kmh: Option<u16>) -> Option<Transition> {
    let s = speed_kmh.map_or(DEFAULT_SPEED_KMH, i64::from);
    let pairs = match_pieces(near, far);
    let (nx, fx) = (spans(near), spans(far));
    let at = |i: Option<usize>, x: &[(i32, i32)]| i.map(|i| x[i]);
    let changes: Vec<Change> =
        pairs.iter().map(|&(n, f)| Change { kind: n.map_or_else(|| far[f.unwrap()].kind, |n| near[n].kind), before: at(n, &nx), after: at(f, &fx) }).collect();
    let length = up_100(changes.iter().map(|c| needed_mm(c, s)).max().unwrap_or(0));
    if length == 0 {
        return None;
    }

    let dropped = pairs.iter().any(|p| p.1.is_none());
    let added = pairs.iter().any(|p| p.0.is_none());
    let roadway = |section: &[Segment]| section.iter().filter(|p| is_roadway(p.kind)).count();
    let host = match (dropped, added) {
        (false, true) => Host::Near,
        (true, false) => Host::Far,
        (true, true) if roadway(near) < roadway(far) => Host::Near,
        _ => Host::Far,
    };

    // A travel or bus lane that only one side has must have, beside it in its own section, a lane of the same
    // direction that the other side has too: it opens from, or closes into, that lane. The same on both sides, so the
    // answer does not depend on which end the zoning is anchored at.
    let lane = |section: &[Segment], i: usize| taper(section[i].kind) == Taper::Lane;
    let beside_matched = |section: &[Segment], i: usize, matched: &dyn Fn(usize) -> bool| {
        [i.checked_sub(1), Some(i + 1)]
            .into_iter()
            .flatten()
            .filter(|&j| j < section.len())
            .any(|j| lane(section, j) && section[j].direction == section[i].direction && matched(j))
    };
    let merges = pairs.iter().all(|&(n, f)| match (n, f) {
        (Some(i), None) if lane(near, i) => beside_matched(near, i, &|j| pairs.iter().any(|p| p.0 == Some(j) && p.1.is_some())),
        (None, Some(i)) if lane(far, i) => beside_matched(far, i, &|j| pairs.iter().any(|p| p.1 == Some(j) && p.0.is_some())),
        _ => true,
    });

    Some(Transition { length_mm: length as i32, host, changes, merges })
}

/// Pair the pieces of two sections: by uid first, then the rest by kind in left-to-right order. A uid that
/// names another kind on the other side does not match. Pairs whose order differs between the sides (the strips
/// would cross) are split into a dropped and an added entry, as few of them as it takes. The entries run in section order.
pub(crate) fn match_pieces(near: &[Segment], far: &[Segment]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut to_far: Vec<Option<usize>> = vec![None; near.len()];
    let mut taken = vec![false; far.len()];
    for (i, n) in near.iter().enumerate() {
        if let Some(j) = far.iter().position(|f| f.uid == n.uid && f.kind == n.kind) {
            to_far[i] = Some(j);
            taken[j] = true;
        }
    }
    for (i, n) in near.iter().enumerate() {
        if to_far[i].is_none()
            && let Some(j) = (0..far.len()).find(|&j| !taken[j] && far[j].kind == n.kind)
        {
            to_far[i] = Some(j);
            taken[j] = true;
        }
    }
    // The strips of matched pieces must not cross: keep the longest run whose far places rise with the near ones (the
    // longest increasing subsequence), so a piece moved across several others is the only one split. The first of
    // equal runs is kept.
    let matched: Vec<(usize, usize)> = to_far.iter().enumerate().filter_map(|(i, j)| j.map(|j| (i, j))).collect();
    let mut run = vec![1usize; matched.len()];
    let mut back: Vec<Option<usize>> = vec![None; matched.len()];
    for k in 0..matched.len() {
        for p in 0..k {
            if matched[p].1 < matched[k].1 && run[p] + 1 > run[k] {
                run[k] = run[p] + 1;
                back[k] = Some(p);
            }
        }
    }
    let mut keep = vec![false; matched.len()];
    let mut at = (0..matched.len()).rev().max_by_key(|&k| run[k]);
    while let Some(k) = at {
        keep[k] = true;
        at = back[k];
    }
    for (&(i, j), &kept) in matched.iter().zip(&keep) {
        if !kept {
            to_far[i] = None;
            taken[j] = false;
        }
    }
    let mut out = Vec::new();
    let mut j = 0;
    for (i, m) in to_far.iter().enumerate() {
        match m {
            Some(m) => {
                while j < *m {
                    if !taken[j] {
                        out.push((None, Some(j)));
                    }
                    j += 1;
                }
                out.push((Some(i), Some(*m)));
                j = m + 1;
            }
            None => out.push((Some(i), None)),
        }
    }
    out.extend((j..far.len()).filter(|&j| !taken[j]).map(|j| (None, Some(j))));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::kind_index;

    fn k(id: &str) -> usize {
        kind_index(id).unwrap()
    }

    fn seg(uid: u32, id: &str, w: i32) -> Segment {
        Segment::new(uid, k(id), w)
    }

    fn dir(mut s: Segment, d: usize) -> Segment {
        s.direction = Some(d);
        s
    }

    fn base() -> Vec<Segment> {
        vec![seg(1, "sidewalk", 2000), dir(seg(2, "travel", 3500), 0), dir(seg(3, "travel", 3500), 1), seg(4, "sidewalk", 2000)]
    }

    #[test]
    fn identical_sections_have_no_transition() {
        let far: Vec<Segment> = base()
            .into_iter()
            .map(|mut s| {
                s.uid += 10;
                s
            })
            .collect();
        assert_eq!(transition(&base(), &far, Some(50)), None);
    }

    fn with_extra() -> Vec<Segment> {
        let mut v = base();
        // at the right end, so no other piece shifts
        v.push(dir(seg(9, "travel", 3500), 0));
        v
    }

    #[test]
    fn a_lane_added_at_50_tapers_15_to_1() {
        let t = transition(&base(), &with_extra(), Some(50)).unwrap();
        assert_eq!(t.length_mm, 52_500);
        assert_eq!(t.host, Host::Near);
    }

    #[test]
    fn ratios_step_up_with_speed() {
        let l = |s| transition(&base(), &with_extra(), Some(s)).unwrap().length_mm;
        assert_eq!(l(60), 63_000);
        assert_eq!(l(70), 84_000);
        assert_eq!(l(55), 63_000);
    }

    #[test]
    fn a_lane_shift_below_70_uses_ws2_over_155() {
        // the sidewalk grows 1 m, so the lane shifts 1 m
        let near = vec![seg(1, "sidewalk", 2000), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 2000)];
        let far = vec![seg(1, "sidewalk", 3000), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 1000)];
        let t = transition(&near, &far, Some(50)).unwrap();
        assert_eq!(t.length_mm, 16_200);
        assert_eq!(t.host, Host::Far);
    }

    #[test]
    fn a_lane_shift_at_70_uses_0_62_ws() {
        let near = vec![seg(1, "sidewalk", 2000), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 2000)];
        let far = vec![seg(1, "sidewalk", 3000), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 1000)];
        assert_eq!(transition(&near, &far, Some(70)).unwrap().length_mm, 43_400);
    }

    #[test]
    fn a_dropped_lane_merges_into_its_neighbour() {
        let near = vec![dir(seg(1, "travel", 3500), 0), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 2000)];
        let far = vec![dir(seg(1, "travel", 3500), 0), seg(3, "sidewalk", 2000)];
        let t = transition(&near, &far, Some(50)).unwrap();
        assert!(t.merges);
        assert_eq!(t.host, Host::Far);
        assert_eq!(t.length_mm, 56_500);
    }

    #[test]
    fn a_dropped_lane_beside_parking_cannot_merge() {
        let near = vec![seg(1, "parking", 2500), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 2000)];
        let far = vec![seg(1, "parking", 2500), seg(3, "sidewalk", 2000)];
        let t = transition(&near, &far, Some(50)).unwrap();
        assert!(!t.merges);
    }

    #[test]
    fn a_bike_lane_shift_is_one_to_five() {
        let near = vec![seg(1, "sidewalk", 2000), dir(seg(2, "bike", 1500), 0), seg(3, "sidewalk", 2000)];
        let far = vec![seg(1, "sidewalk", 3500), dir(seg(2, "bike", 1500), 0), seg(3, "sidewalk", 500)];
        assert_eq!(transition(&near, &far, Some(50)).unwrap().length_mm, 7_500);
    }

    #[test]
    fn sidewalks_and_medians_do_not_taper() {
        // only the outer edge moves, so the lane does not shift
        let near = vec![dir(seg(2, "travel", 3500), 0), seg(1, "sidewalk", 2000)];
        let far = vec![dir(seg(2, "travel", 3500), 0), seg(1, "sidewalk", 2500)];
        assert_eq!(transition(&near, &far, Some(50)), None);
        let near = vec![dir(seg(2, "travel", 3500), 0), seg(1, "median", 2000)];
        let far = vec![dir(seg(2, "travel", 3500), 0), seg(1, "median", 2500)];
        assert_eq!(transition(&near, &far, Some(50)), None);
    }

    #[test]
    fn no_speed_uses_fifty() {
        assert_eq!(transition(&base(), &with_extra(), None), transition(&base(), &with_extra(), Some(50)));
    }

    #[test]
    fn fresh_uids_match_by_kind_in_order() {
        let near = base();
        let mut far: Vec<Segment> = base()
            .into_iter()
            .map(|mut s| {
                s.uid += 100;
                s
            })
            .collect();
        far[2].width_mm = 4000;
        far[3].width_mm = 1500;
        let m = match_pieces(&near, &far);
        assert_eq!(m, vec![(Some(0), Some(0)), (Some(1), Some(1)), (Some(2), Some(2)), (Some(3), Some(3))]);
        let t = transition(&near, &far, Some(50)).unwrap();
        assert!(t.changes.iter().all(|c| c.before.is_some() && c.after.is_some()));
        assert_eq!(t.host, Host::Far);
    }

    #[test]
    fn crossing_pieces_are_unmatched() {
        let near = vec![dir(seg(1, "travel", 3500), 0), seg(2, "bike", 1500)];
        let far = vec![seg(2, "bike", 1500), dir(seg(1, "travel", 3500), 0)];
        let m = match_pieces(&near, &far);
        assert_eq!(m.len(), 3, "one of the two stays matched, the other is split: {m:?}");
        assert_eq!(m.iter().filter(|(a, b)| a.is_some() != b.is_some()).count(), 2);
        let t = transition(&near, &far, Some(50)).unwrap();
        assert_eq!(t.changes.len(), 3);
    }

    #[test]
    fn the_longest_piece_sets_the_length() {
        let buffer = |w| seg(1, "buffer", w);
        // The buffer widens 1.5 m and pushes the bike lane (1:5, 7 500) but not the driving lane before it.
        let near = vec![dir(seg(2, "travel", 3500), 0), buffer(500), seg(3, "bike", 1500), seg(4, "sidewalk", 2000)];
        let far = vec![dir(seg(2, "travel", 3500), 0), buffer(2000), seg(3, "bike", 1500), seg(4, "sidewalk", 500)];
        assert_eq!(transition(&near, &far, Some(50)).unwrap().length_mm, 7_500);
        // Put the driving lane behind the buffer too: it shifts 1.5 m (24 200) and the longer rule sets the length.
        let near = vec![buffer(500), dir(seg(2, "travel", 3500), 0), seg(3, "bike", 1500), seg(4, "sidewalk", 2000)];
        let far = vec![buffer(2000), dir(seg(2, "travel", 3500), 0), seg(3, "bike", 1500), seg(4, "sidewalk", 500)];
        assert_eq!(transition(&near, &far, Some(50)).unwrap().length_mm, 24_200);
    }

    /// Near `[sidewalk, bike, travel, travel, sidewalk]`, far with the bike lane moved across both travel lanes.
    fn bike_moved_across_two_lanes() -> (Vec<Segment>, Vec<Segment>) {
        let near =
            vec![seg(1, "sidewalk", 2000), seg(2, "bike", 1500), dir(seg(3, "travel", 3500), 0), dir(seg(4, "travel", 3500), 1), seg(5, "sidewalk", 2000)];
        let far =
            vec![seg(1, "sidewalk", 2000), dir(seg(3, "travel", 3500), 0), dir(seg(4, "travel", 3500), 1), seg(2, "bike", 1500), seg(5, "sidewalk", 2000)];
        (near, far)
    }

    #[test]
    fn a_piece_moved_across_two_others_is_the_only_one_unmatched() {
        let (near, far) = bike_moved_across_two_lanes();
        let m = match_pieces(&near, &far);
        let unmatched: Vec<_> = m.iter().filter(|p| p.0.is_none() || p.1.is_none()).collect();
        assert_eq!(unmatched.len(), 2, "the bike lane dropped on one side and added on the other: {m:?}");
        assert!(unmatched.iter().all(|p| p.0 == Some(1) || p.1 == Some(3)), "{m:?}");
        for lane in [2, 3] {
            assert!(m.iter().any(|p| p.0 == Some(lane) && p.1 == Some(lane - 1)), "travel lane {lane} stays matched: {m:?}");
        }
    }

    #[test]
    fn a_piece_moved_across_two_travel_lanes_does_not_drop_a_lane() {
        let (near, far) = bike_moved_across_two_lanes();
        let t = transition(&near, &far, Some(50)).unwrap();
        assert!(t.merges, "no travel lane ends");
        assert_eq!(t.length_mm, 24_200, "the lanes shift by the bike lane's 1.5 m, not the 56.5 m of a dropped lane");
    }

    #[test]
    fn lane_drop_does_not_depend_on_which_end_the_zoning_is_anchored_at() {
        // Anchoring at the other end swaps near and far; the same design must pass or fail either way.
        let beside_parking_dropped =
            (vec![seg(1, "parking", 2500), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 2000)], vec![seg(1, "parking", 2500), seg(3, "sidewalk", 2000)]);
        let merging = (
            vec![dir(seg(1, "travel", 3500), 0), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 2000)],
            vec![dir(seg(1, "travel", 3500), 0), seg(3, "sidewalk", 2000)],
        );
        let (a, b) = (base(), with_extra());
        for (name, (x, y), merges) in
            [("beside parking", beside_parking_dropped, false), ("merging", merging, true), ("added beside a sidewalk", (a, b), false)]
        {
            let (first, last) = (transition(&x, &y, Some(50)).unwrap().merges, transition(&y, &x, Some(50)).unwrap().merges);
            assert_eq!((first, last), (merges, merges), "{name}");
        }
    }

    #[test]
    fn an_added_lane_needs_a_matched_lane_of_its_direction_beside_it() {
        let near = vec![dir(seg(1, "travel", 3500), 0), seg(3, "sidewalk", 2000)];
        let beside = vec![dir(seg(1, "travel", 3500), 0), dir(seg(2, "travel", 3500), 0), seg(3, "sidewalk", 2000)];
        let opposite = vec![dir(seg(1, "travel", 3500), 0), dir(seg(2, "travel", 3500), 1), seg(3, "sidewalk", 2000)];
        assert!(transition(&near, &beside, Some(50)).unwrap().merges);
        assert!(!transition(&near, &opposite, Some(50)).unwrap().merges, "a lane that opens beside traffic of the other direction");
    }
}
