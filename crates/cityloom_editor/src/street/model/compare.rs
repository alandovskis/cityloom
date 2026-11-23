//! What differs between two designs of one street, beyond the impact summary: the lanes each way, the pieces added,
//! removed or changed, and the other things that set the two apart (speed limits, stretches, curbside uses, stops,
//! raised elements, greenery).
//!
//! Pure: no wording but `Said`. The designs are the street's own data; nothing is read from the page.

use super::matching::rising;
use super::{Segment, Street};
use crate::shared::catalogue::{DIRECTIONS, DirectionRule, KINDS};
use crate::shared::said::{Arg, Said};
use std::collections::BTreeSet;

/// How many driving lanes (the pieces that must run one way) of the base section run in one direction, in each of two
/// designs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneCount {
    /// An index into `DIRECTIONS`.
    pub direction: usize,
    pub a: usize,
    pub b: usize,
}

impl LaneCount {
    /// `Away from you: 2 → 1`, as a message.
    pub fn said(&self) -> Said {
        Said::new("cmp-lanes")
            .with("direction", Arg::Msg(direction_message(self.direction)))
            .with("a", Arg::Num(self.a as i64))
            .with("b", Arg::Num(self.b as i64))
    }
}

/// The message that names a direction of travel, as `shared::catalogue::direction_key` builds it.
fn direction_message(direction: usize) -> &'static str {
    match DIRECTIONS[direction].id {
        "away" => "direction-away",
        _ => "direction-toward",
    }
}

/// One difference between the base sections of the two designs. A piece is the same piece in both by its `uid`, else by
/// a longest common subsequence of the kinds, left to right, between the pieces paired by `uid`; where the uids of the
/// sections are not the same set and that pairs fewer pieces of one kind, by a longest common subsequence of the kinds
/// over both whole sections alone (see `pair`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceChange {
    /// A piece only B has.
    Added { kind: usize, width_mm: i32 },
    /// A piece only A has.
    Removed { kind: usize, width_mm: i32 },
    /// A piece that is of another kind in B; only a pair by `uid` can be, so none when the pairing is by kinds alone.
    Kind { from: usize, to: usize },
    /// A piece of one kind that is wider or narrower in B.
    Width { kind: usize, a_mm: i32, b_mm: i32 },
}

impl PieceChange {
    pub fn said(&self) -> Said {
        let kind = |k: usize| Arg::Kind(KINDS[k].id);
        let lower = |k: usize| Arg::KindLower(KINDS[k].id);
        match *self {
            PieceChange::Added { kind: k, width_mm } => Said::new("cmp-piece-added").with("kind", lower(k)).with("width", Arg::Length(width_mm)),
            PieceChange::Removed { kind: k, width_mm } => Said::new("cmp-piece-removed").with("kind", lower(k)).with("width", Arg::Length(width_mm)),
            PieceChange::Kind { from, to } => Said::new("cmp-piece-kind").with("from", Arg::Kind(KINDS[from].id)).with("to", lower(to)),
            PieceChange::Width { kind: k, a_mm, b_mm } => {
                Said::new("cmp-piece-width").with("kind", kind(k)).with("a", Arg::Length(a_mm)).with("b", Arg::Length(b_mm))
            }
        }
    }
}

/// What else can set two designs apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Other {
    Speed,
    Stretches,
    /// Something other than their kinds and widths of the base pieces paired as `pieces` pairs them (surface, curb,
    /// direction of a piece that need not have one, times of day), or, where they are paired by `uid` first, the order of
    /// those paired by `uid`.
    PieceDetails,
    Curbside,
    Stops,
    Calming,
    Green,
}

impl Other {
    /// The message that names it, which the comparison's frame owns.
    pub fn key(self) -> &'static str {
        match self {
            Other::Speed => "cmp-other-speed",
            Other::Stretches => "cmp-other-stretches",
            Other::PieceDetails => "cmp-other-piece-details",
            Other::Curbside => "cmp-other-curbside",
            Other::Stops => "cmp-other-stops",
            Other::Calming => "cmp-other-calming",
            Other::Green => "cmp-other-green",
        }
    }
}

/// How many driving lanes each way the base section of each design has, for the directions where the counts differ.
pub fn lanes(a: &Street, b: &Street) -> Vec<LaneCount> {
    let count = |s: &Street, direction: usize| {
        s.segments.iter().filter(|p| KINDS[p.kind].direction == DirectionRule::Required && p.direction == Some(direction)).count()
    };
    (0..DIRECTIONS.len()).map(|direction| LaneCount { direction, a: count(a, direction), b: count(b, direction) }).filter(|c| c.a != c.b).collect()
}

/// A piece that is the same in both designs: its index among A's base pieces and among B's.
struct Paired {
    a: usize,
    b: usize,
}

/// The base pieces of designs A and B that are the same piece.
struct Pairing {
    pairs: Vec<Paired>,
    /// Paired by `uid` first (`pair_by_uid`), not by the kinds alone.
    by_uid: bool,
}

/// The base pieces of designs A and B that are the same piece: by `uid` first (`pair_by_uid`), or by the kinds alone, a
/// longest common subsequence of them over both whole sections.
///
/// A scenario numbers its new pieces from the baseline's next uid, so two designs can give one uid to pieces that are
/// not the same. When both sections hold the same set of uids, the pairing is by `uid`; otherwise it is by `uid` only
/// when that gives at least as many pairs of one kind, by `uid` and by alignment, as the kinds alone do.
fn pair(a: &[Segment], b: &[Segment]) -> Pairing {
    let uids = |section: &[Segment]| section.iter().map(|p| p.uid).collect::<BTreeSet<_>>();
    let by_uid = pair_by_uid(a, b);
    let by_kind = align(a, &Vec::from_iter(0..a.len()), b, &Vec::from_iter(0..b.len()));
    let of_one_kind = by_uid.iter().filter(|p| a[p.a].kind == b[p.b].kind).count();
    if uids(a) == uids(b) || of_one_kind >= by_kind.len() { Pairing { pairs: by_uid, by_uid: true } } else { Pairing { pairs: by_kind, by_uid: false } }
}

/// The pieces with the same `uid`, whatever their kinds, and, since a Transform measure or a template gives its pieces
/// new uids, those a longest common subsequence of the kinds, left to right, pairs among the rest. The pairs by `uid`
/// that keep their order (the longest run of them whose places in B rise with those in A) part the street in gaps, and
/// the alignment stays within a gap, so it crosses none of them.
///
/// Not `matching::match_pieces`, which pairs the pieces of a taper: it refuses a pair by `uid` of another kind; it gives
/// a piece with none the first free piece of its kind, wherever that lies, so what is left without crossings need not be
/// a longest common subsequence of the kinds; and it keeps only the longest rising run of its pairs and splits the
/// others in a dropped and an added piece, where here a pair by `uid` that crosses others stays a pair, a piece detail.
fn pair_by_uid(a: &[Segment], b: &[Segment]) -> Vec<Paired> {
    let mut pairs: Vec<Paired> =
        b.iter().enumerate().filter_map(|(j, now)| a.iter().position(|was| was.uid == now.uid).map(|i| Paired { a: i, b: j })).collect();
    let free_a: Vec<usize> = (0..a.len()).filter(|&i| !pairs.iter().any(|p| p.a == i)).collect();
    let free_b: Vec<usize> = (0..b.len()).filter(|&j| !pairs.iter().any(|p| p.b == j)).collect();
    pairs.sort_by_key(|p| p.a);
    // the gaps, each up to (and without) its end in A and in B: one before each pair that keeps its order, and the last
    let keep = rising(&pairs.iter().map(|p| p.b).collect::<Vec<_>>());
    let mut ends: Vec<(usize, usize)> = pairs.iter().zip(&keep).filter(|&(_, &kept)| kept).map(|(p, _)| (p.a, p.b)).collect();
    ends.push((a.len(), b.len()));
    let (mut from_a, mut from_b) = (0, 0);
    for (end_a, end_b) in ends {
        let gap_a: Vec<usize> = free_a.iter().copied().filter(|i| (from_a..end_a).contains(i)).collect();
        let gap_b: Vec<usize> = free_b.iter().copied().filter(|j| (from_b..end_b).contains(j)).collect();
        pairs.extend(align(a, &gap_a, b, &gap_b));
        (from_a, from_b) = (end_a, end_b);
    }
    pairs
}

/// A longest common subsequence of the kinds of some of A's pieces and some of B's (their indices, left to right), the
/// one that pairs the pieces furthest to the left.
fn align(a: &[Segment], of_a: &[usize], b: &[Segment], of_b: &[usize]) -> Vec<Paired> {
    let same_kind = |i: usize, j: usize| a[of_a[i]].kind == b[of_b[j]].kind;
    // `longest[i][j]`: how many pairs the pieces from `i` on and from `j` on can make
    let mut longest = vec![vec![0usize; of_b.len() + 1]; of_a.len() + 1];
    for i in (0..of_a.len()).rev() {
        for j in (0..of_b.len()).rev() {
            longest[i][j] = if same_kind(i, j) { longest[i + 1][j + 1] + 1 } else { longest[i + 1][j].max(longest[i][j + 1]) };
        }
    }
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < of_a.len() && j < of_b.len() {
        if same_kind(i, j) {
            pairs.push(Paired { a: of_a[i], b: of_b[j] });
            (i, j) = (i + 1, j + 1);
        } else if longest[i + 1][j] == longest[i][j] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

/// The pieces added, removed or changed from design A to design B in the base section, in B's order and then A's
/// removed ones, and whether the stretches along the street (where each starts, and how many) differ. A piece is the
/// same in both as the pairing `pair` chooses (by `uid` first, or by kinds alone) says; one it leaves unpaired is added
/// or removed.
pub fn pieces(a: &Street, b: &Street) -> (bool, Vec<PieceChange>) {
    let pairs = pair(&a.segments, &b.segments).pairs;
    let mut changes = Vec::new();
    for (j, now) in b.segments.iter().enumerate() {
        match pairs.iter().find(|p| p.b == j).map(|p| &a.segments[p.a]) {
            None => changes.push(PieceChange::Added { kind: now.kind, width_mm: now.width_mm }),
            Some(was) => {
                if was.kind != now.kind {
                    changes.push(PieceChange::Kind { from: was.kind, to: now.kind });
                }
                if was.width_mm != now.width_mm {
                    changes.push(PieceChange::Width { kind: now.kind, a_mm: was.width_mm, b_mm: now.width_mm });
                }
            }
        }
    }
    for (_, was) in a.segments.iter().enumerate().filter(|(i, _)| !pairs.iter().any(|p| p.a == *i)) {
        changes.push(PieceChange::Removed { kind: was.kind, width_mm: was.width_mm });
    }
    (starts(a) != starts(b), changes)
}

/// The sections of a street with their uids cleared, which say nothing of what a design holds.
fn without_uids(section: &[Segment]) -> Vec<Segment> {
    section.iter().map(|p| Segment { uid: 0, ..p.clone() }).collect()
}

/// Whether the base sections differ in more than what `pieces` lists: a field other than uid, kind and width of a piece
/// both have, as `pair` pairs them (a driving lane's direction is `lanes`'), or, when it pairs them by `uid` first, the
/// order of the pieces paired by `uid`: by kinds alone, a shared uid says nothing.
fn piece_details_differ(a: &Street, b: &Street) -> bool {
    let shared =
        |from: &Street, other: &Street| from.segments.iter().filter(|p| other.segments.iter().any(|q| q.uid == p.uid)).map(|p| p.uid).collect::<Vec<_>>();
    let pairing = pair(&a.segments, &b.segments);
    pairing.pairs.iter().any(|p| {
        let (was, now) = (&a.segments[p.a], &b.segments[p.b]);
        let driving = |p: &Segment| KINDS[p.kind].direction == DirectionRule::Required;
        let direction = if driving(was) || driving(now) { now.direction } else { was.direction };
        Segment { uid: now.uid, kind: now.kind, width_mm: now.width_mm, direction, ..was.clone() } != *now
    }) || (pairing.by_uid && shared(a, b) != shared(b, a))
}

/// A list whose order carries no meaning, in an order that is the same for the same contents. It sorts by the
/// derived `Debug` text, so it relies on every field being in the derived `Debug`.
fn unordered<T: Clone + std::fmt::Debug>(list: &[T]) -> Vec<T> {
    let mut sorted = list.to_vec();
    sorted.sort_by_cached_key(|x| format!("{x:?}"));
    sorted
}

/// Where each stretch along the street starts.
fn starts(s: &Street) -> Vec<i32> {
    s.zones.iter().map(|z| z.from_mm).collect()
}

/// The things beyond the base section that differ between the two designs, in the order of `Other`. Stretches that
/// start elsewhere are `pieces`' to say, so `Stretches` is only for stretches that start alike and hold other sections.
pub fn others(a: &Street, b: &Street) -> Vec<Other> {
    let speeds = |s: &Street| (s.speed, s.zones.iter().map(|z| z.speed).collect::<Vec<_>>());
    // A zone's saved `name` is left out: the page never shows it (a stretch is named by where it lies).
    let sections = |s: &Street| s.zones.iter().map(|z| z.segments.as_deref().map(without_uids)).collect::<Vec<_>>();
    [
        (Other::Speed, speeds(a) != speeds(b)),
        (Other::Stretches, starts(a) == starts(b) && sections(a) != sections(b)),
        (Other::PieceDetails, piece_details_differ(a, b)),
        (Other::Curbside, unordered(&a.curb) != unordered(&b.curb)),
        (Other::Stops, unordered(&a.stops) != unordered(&b.stops)),
        (Other::Calming, unordered(&a.calming) != unordered(&b.calming)),
        (Other::Green, unordered(&a.green) != unordered(&b.green)),
    ]
    .into_iter()
    .filter_map(|(other, differs)| differs.then_some(other))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::{Side, kind_index};
    use crate::shared::testing::en;
    use crate::street::model::{Green, GreenKind, Speed, Stop, StopKind, StreetExt, TreeSize, Zone};
    use crate::street::model::{Instance, Use};

    fn sample() -> Street {
        Street::sample(0, Side::Right)
    }

    fn driving(s: &Street, direction: usize) -> usize {
        s.segments.iter().filter(|p| KINDS[p.kind].direction == DirectionRule::Required && p.direction == Some(direction)).count()
    }

    #[test]
    fn identical_streets_differ_in_nothing() {
        let (a, b) = (sample(), sample());
        assert!(lanes(&a, &b).is_empty());
        assert_eq!(pieces(&a, &b), (false, vec![]));
        assert!(others(&a, &b).is_empty());
    }

    #[test]
    fn a_widened_and_a_removed_piece_are_listed() {
        let a = sample();
        let mut b = a.clone();
        b.segments[0].width_mm += 500;
        let gone = b.segments.pop().unwrap();
        let first = &a.segments[0];
        assert_eq!(
            pieces(&a, &b).1,
            vec![
                PieceChange::Width { kind: first.kind, a_mm: first.width_mm, b_mm: first.width_mm + 500 },
                PieceChange::Removed { kind: gone.kind, width_mm: gone.width_mm },
            ]
        );
    }

    #[test]
    fn an_added_piece_and_another_kind_are_listed() {
        let a = sample();
        let mut b = a.clone();
        let bike = kind_index("bike").unwrap();
        let from = b.segments[1].kind;
        b.segments[1].kind = bike;
        b.segments.insert(0, Segment { uid: 99, kind: bike, width_mm: 1500, ..a.segments[1].clone() });
        assert_eq!(pieces(&a, &b).1, vec![PieceChange::Added { kind: bike, width_mm: 1500 }, PieceChange::Kind { from, to: bike }]);
    }

    #[test]
    fn a_lane_turned_around_changes_the_counts_each_way() {
        let a = sample();
        let mut b = a.clone();
        let lane = b.segments.iter_mut().find(|p| KINDS[p.kind].direction == DirectionRule::Required && p.direction == Some(0)).unwrap();
        lane.direction = Some(1);
        assert_eq!(
            lanes(&a, &b),
            vec![LaneCount { direction: 0, a: driving(&a, 0), b: driving(&a, 0) - 1 }, LaneCount { direction: 1, a: driving(&a, 1), b: driving(&a, 1) + 1 },]
        );
    }

    #[test]
    fn stretches_that_start_elsewhere_are_reported() {
        let a = sample();
        let mut b = a.clone();
        let zone = |from_mm| Zone { from_mm, segments: None, name: None, speed: Speed::Undecided };
        b.zones = vec![zone(0), zone(5_000)];
        assert!(pieces(&a, &b).0);
        // said once, by the pieces' line, not again among the other things
        assert!(!others(&a, &b).contains(&Other::Stretches), "{:?}", others(&a, &b));
    }

    #[test]
    fn stretches_that_start_alike_but_hold_other_sections_are_an_other() {
        let mut a = sample();
        let zone = |segments| Zone { from_mm: 5_000, segments, name: None, speed: Speed::Undecided };
        a.zones = vec![zone(None)];
        let mut b = a.clone();
        b.zones = vec![zone(Some(a.segments[..1].to_vec()))];
        assert!(!pieces(&a, &b).0);
        assert_eq!(others(&a, &b), vec![Other::Stretches]);
    }

    fn with_curb(s: &mut Street) {
        s.curb = vec![
            Instance { side: Side::Left, from_mm: 0, length_mm: 5_000, use_: Use::Parking },
            Instance { side: Side::Right, from_mm: 1_000, length_mm: 2_000, use_: Use::Loading },
            Instance { side: Side::Left, from_mm: 9_000, length_mm: 3_000, use_: Use::Patio },
        ];
    }

    #[test]
    fn curbside_uses_in_another_order_with_other_uids_are_the_same() {
        let mut a = sample();
        with_curb(&mut a);
        let mut b = a.clone();
        b.curb.reverse();
        b.curb.rotate_left(1);
        for (i, p) in b.segments.iter_mut().enumerate() {
            p.uid = 100 + i as u32;
        }
        assert!(others(&a, &b).is_empty());
    }

    #[test]
    fn a_moved_stop_and_a_new_tree_are_named() {
        let mut a = sample();
        a.stops = vec![Stop { side: Side::Right, from_mm: 4_000, length_mm: 12_000, kind: StopKind::Curb, shelter: false, name: None }];
        let mut b = a.clone();
        b.stops[0].from_mm = 8_000;
        b.green = vec![Green { kind: GreenKind::Tree, from_mm: 2_000, length_mm: 500, offset_mm: 1_000, width_mm: 800, size: Some(TreeSize::M) }];
        assert_eq!(others(&a, &b), vec![Other::Stops, Other::Green]);
    }

    #[test]
    fn a_curb_or_surface_change_alone_is_a_piece_detail() {
        let a = sample();
        let mut b = a.clone();
        let p = b.segments.iter_mut().find(|p| KINDS[p.kind].materials.len() > 1).unwrap();
        p.material = KINDS[p.kind].materials[1];
        assert_eq!(others(&a, &b), vec![Other::PieceDetails]);
        assert_eq!(pieces(&a, &b), (false, vec![]));
    }

    #[test]
    fn a_direction_on_a_piece_that_need_not_have_one_is_a_piece_detail() {
        let mut a = sample();
        let bike = kind_index("bike").unwrap();
        a.segments[1].kind = bike;
        a.segments[1].direction = None;
        let mut b = a.clone();
        b.segments[1].direction = Some(1);
        assert_eq!(others(&a, &b), vec![Other::PieceDetails]);
    }

    #[test]
    fn swapped_pieces_are_a_piece_detail() {
        let a = sample();
        let mut b = a.clone();
        b.segments.swap(0, 1);
        assert_eq!(others(&a, &b), vec![Other::PieceDetails]);
    }

    #[test]
    fn a_new_speed_is_named() {
        let a = sample();
        let mut b = a.clone();
        b.speed = Speed::Kmh(30);
        assert_eq!(others(&a, &b), vec![Other::Speed]);
    }

    #[test]
    fn the_changes_are_said_with_the_kind_and_the_widths() {
        let kind = kind_index("travel").unwrap();
        let said = PieceChange::Width { kind, a_mm: 3_300, b_mm: 3_000 }.said();
        assert_eq!(said.key, "cmp-piece-width");
        assert!(en(&said).contains("3.3"), "{}", en(&said));
        assert_eq!(en(&LaneCount { direction: 0, a: 2, b: 1 }.said()), "Away from you: 2 → 1");
        assert_eq!(Other::Calming.key(), "cmp-other-calming");
    }

    // A Transform measure or a template rebuilds the pieces with new uids (CLM-21): the pieces are then paired by a
    // longest common subsequence over their kinds, left to right, and only what that leaves unpaired is added or removed.

    fn kind(id: &str) -> usize {
        kind_index(id).unwrap()
    }

    /// `s` with every piece given a uid the sample does not use (100 and on), in its order.
    fn renumbered(s: &Street) -> Street {
        let mut s = s.clone();
        for (i, p) in s.segments.iter_mut().enumerate() {
            p.uid = 100 + i as u32;
        }
        s
    }

    /// The first uid none of `s`'s pieces has.
    fn past_uids(s: &Street) -> u32 {
        s.segments.iter().map(|p| p.uid).max().unwrap_or(0) + 1
    }

    fn removed(changes: &[PieceChange]) -> Vec<PieceChange> {
        changes.iter().copied().filter(|c| matches!(c, PieceChange::Removed { .. })).collect()
    }

    fn added_kinds(changes: &[PieceChange]) -> std::collections::BTreeSet<usize> {
        changes
            .iter()
            .filter_map(|c| match *c {
                PieceChange::Added { kind, .. } => Some(kind),
                _ => None,
            })
            .collect()
    }

    fn added_or_removed_of(changes: &[PieceChange], k: usize) -> Vec<PieceChange> {
        changes.iter().copied().filter(|c| matches!(*c, PieceChange::Added { kind, .. } | PieceChange::Removed { kind, .. } if kind == k)).collect()
    }

    fn has_kind_change(changes: &[PieceChange]) -> bool {
        changes.iter().any(|c| matches!(c, PieceChange::Kind { .. }))
    }

    #[test]
    fn fresh_uids_with_a_widened_piece_say_only_the_width() {
        let a = sample();
        let mut b = renumbered(&a);
        b.segments[2].width_mm += 500;
        let third = &a.segments[2];
        assert_eq!(pieces(&a, &b).1, vec![PieceChange::Width { kind: third.kind, a_mm: third.width_mm, b_mm: third.width_mm + 500 }]);
    }

    #[test]
    fn fresh_uids_with_an_inserted_and_a_dropped_piece_say_only_those() {
        let a = sample();
        let mut b = a.clone();
        let bike = kind("bike");
        b.segments.insert(1, Segment::new(0, bike, 1500));
        let last_travel = b.segments.iter().rposition(|p| p.kind == kind("travel")).unwrap();
        let gone = b.segments.remove(last_travel);
        let b = renumbered(&b);
        assert_eq!(
            pieces(&a, &b).1,
            vec![PieceChange::Added { kind: bike, width_mm: 1500 }, PieceChange::Removed { kind: gone.kind, width_mm: gone.width_mm }]
        );
    }

    #[test]
    fn a_measure_like_rebuild_keeps_edges_and_aligns_the_middle() {
        let a = sample();
        let (sidewalk, parking, travel, bike) = (kind("sidewalk"), kind("parking"), kind("travel"), kind("bike"));
        assert_eq!(a.segments.iter().map(|p| p.kind).collect::<Vec<_>>(), [sidewalk, parking, travel, travel, parking, sidewalk]);
        // the edge sidewalks keep their uids; the middle is laid out afresh: the first parking widened, the first
        // travel lane now a bike lane of another width, the rest as they were
        let middle = [
            Segment { uid: 101, width_mm: a.segments[1].width_mm + 500, ..a.segments[1].clone() },
            Segment::new(102, bike, 1500),
            Segment { uid: 103, ..a.segments[3].clone() },
            Segment { uid: 104, ..a.segments[4].clone() },
        ];
        let segments = std::iter::once(a.segments[0].clone()).chain(middle).chain(std::iter::once(a.segments[5].clone())).collect();
        let b = Street { segments, ..a.clone() };
        let changes = pieces(&a, &b).1;
        assert_eq!(
            changes,
            vec![
                PieceChange::Width { kind: parking, a_mm: a.segments[1].width_mm, b_mm: a.segments[1].width_mm + 500 },
                PieceChange::Added { kind: bike, width_mm: 1500 },
                PieceChange::Removed { kind: travel, width_mm: a.segments[2].width_mm },
            ]
        );
        assert!(!has_kind_change(&changes), "{changes:?}");
    }

    #[test]
    fn a_real_measure_reports_no_removed_piece_whose_kind_survives() {
        let a = sample();
        let mut next_uid = past_uids(&a);
        let segments = crate::street::model::measures::arrange("B4", &a.segments, 18_000, Side::Right, false, &mut next_uid)
            .expect("the sample street takes a centre bus lane");
        let b = Street { segments, ..a.clone() };
        let changes = pieces(&a, &b).1;
        let parking = a.segments.iter().filter(|p| p.kind == kind("parking")).map(|p| PieceChange::Removed { kind: p.kind, width_mm: p.width_mm });
        assert_eq!(removed(&changes), parking.collect::<Vec<_>>(), "{changes:?}");
        assert_eq!(added_kinds(&changes), [kind("bus"), kind("travel")].into_iter().collect(), "{changes:?}");
        assert_eq!(added_or_removed_of(&changes, kind("sidewalk")), vec![], "{changes:?}");
        assert!(!has_kind_change(&changes), "{changes:?}");
    }

    #[test]
    fn a_real_template_reports_no_removed_piece_whose_kind_survives() {
        let a = sample();
        let mut next_uid = past_uids(&a);
        let segments =
            crate::street::model::Template::Greenway.arrange(a.class, 18_000, Side::Right, &mut next_uid).expect("the sample street fits the greenway");
        assert!(segments.iter().all(|p| p.uid >= past_uids(&a)), "every uid is fresh");
        let b = Street { segments, ..a.clone() };
        let changes = pieces(&a, &b).1;
        let parking = a.segments.iter().filter(|p| p.kind == kind("parking")).map(|p| PieceChange::Removed { kind: p.kind, width_mm: p.width_mm });
        assert_eq!(removed(&changes), parking.collect::<Vec<_>>(), "{changes:?}");
        assert_eq!(added_or_removed_of(&changes, kind("sidewalk")), vec![], "{changes:?}");
        assert_eq!(added_or_removed_of(&changes, kind("travel")), vec![], "{changes:?}");
        assert!(!has_kind_change(&changes), "{changes:?}");
    }

    /// `pieces` as it was before CLM-21, pairing the pieces by uid only: what `pieces` must still give when the uids
    /// match.
    fn by_uid(a: &Street, b: &Street) -> (bool, Vec<PieceChange>) {
        let mut changes = Vec::new();
        for now in &b.segments {
            match a.segments.iter().find(|p| p.uid == now.uid) {
                None => changes.push(PieceChange::Added { kind: now.kind, width_mm: now.width_mm }),
                Some(was) => {
                    if was.kind != now.kind {
                        changes.push(PieceChange::Kind { from: was.kind, to: now.kind });
                    }
                    if was.width_mm != now.width_mm {
                        changes.push(PieceChange::Width { kind: now.kind, a_mm: was.width_mm, b_mm: now.width_mm });
                    }
                }
            }
        }
        for was in a.segments.iter().filter(|p| !b.segments.iter().any(|n| n.uid == p.uid)) {
            changes.push(PieceChange::Removed { kind: was.kind, width_mm: was.width_mm });
        }
        (starts(a) != starts(b), changes)
    }

    #[test]
    fn unchanged_ids_give_the_same_result_as_uid_matching() {
        let a = sample();
        let bike = kind("bike");
        let edit = |f: &dyn Fn(&mut Vec<Segment>)| {
            let mut b = a.clone();
            f(&mut b.segments);
            b
        };
        // (edit, the street it makes, whether it is a piece detail, whether every piece keeps its kind and place)
        let cases: Vec<(&str, Street, bool, bool)> = vec![
            ("widen", edit(&|s| s[0].width_mm += 500), false, true),
            ("change kind", edit(&|s| s[2].kind = bike), false, false),
            ("insert with a new uid", edit(&|s| s.insert(2, Segment::new(99, bike, 1500))), false, true),
            ("remove", edit(&|s| drop(s.remove(4))), false, true),
            ("swap two pieces", edit(&|s| s.swap(0, 1)), true, false),
            (
                "kind change plus insert",
                edit(&|s| {
                    s[1].kind = bike;
                    s.insert(0, Segment::new(99, bike, 1500));
                }),
                false,
                false,
            ),
        ];
        for (name, b, detail, in_place) in cases {
            assert_eq!(pieces(&a, &b), by_uid(&a, &b), "{name}");
            assert_eq!(others(&a, &b).contains(&Other::PieceDetails), detail, "{name}: {:?}", others(&a, &b));
            // where every piece keeps its kind and its place, the same edit with fresh uids reads the same: the pairing
            // by uid and the alignment of the kinds agree
            if in_place {
                assert_eq!(pieces(&a, &renumbered(&b)), by_uid(&a, &b), "{name}, renumbered");
            }
        }
    }

    #[test]
    fn fresh_uids_with_the_same_details_are_no_piece_detail() {
        let a = sample();
        let mut b = renumbered(&a);
        assert!(others(&a, &b).is_empty(), "{:?}", others(&a, &b));
        let p = b.segments.iter_mut().find(|p| KINDS[p.kind].materials.len() > 1).unwrap();
        p.material = KINDS[p.kind].materials[1];
        assert_eq!(others(&a, &b), vec![Other::PieceDetails]);
        assert_eq!(pieces(&a, &b), (false, vec![]));
    }

    #[test]
    fn fresh_uids_with_another_curb_is_a_piece_detail() {
        let a = sample();
        let mut b = renumbered(&a);
        let sidewalk = b.segments.iter_mut().find(|p| p.kind == kind("sidewalk")).unwrap();
        let curb = sidewalk.curb.expect("a sidewalk has a curb");
        sidewalk.curb = Some((curb + 1) % crate::shared::catalogue::CURBS.len());
        assert_eq!(others(&a, &b), vec![Other::PieceDetails]);
    }

    // Two designs can give one uid to two pieces that are not the same: each scenario numbers its new pieces from the
    // baseline's next uid. Where the uids of the two sections are not the same set, the pairing is by uid (with the
    // alignment in its gaps, "U" below) when it has at least as many pairs of one kind as a longest common subsequence of
    // the kinds over both whole sections ("L" below) has pairs; else it is L. A pair by uid of two kinds does not count.

    /// `s` laid out by a template in 18 m, its pieces numbered from the street's next uid.
    fn arranged(s: &Street, template: crate::street::model::Template) -> Street {
        let mut next_uid = s.next_uid;
        let segments = template.arrange(s.class, 18_000, Side::Right, &mut next_uid).expect("the sample street fits the template");
        Street { segments, next_uid, ..s.clone() }
    }

    /// The uid and the kind's id of each piece, left to right.
    fn uids_and_kinds(s: &Street) -> Vec<(u32, &'static str)> {
        s.segments.iter().map(|p| (p.uid, KINDS[p.kind].id)).collect()
    }

    /// The sample street with these pieces, each 3 m wide.
    fn made_of(section: &[(u32, &str)]) -> Street {
        Street { segments: section.iter().map(|&(uid, id)| Segment::new(uid, kind(id), 3_000)).collect(), ..sample() }
    }

    /// Two scenarios of one baseline, each made by a template, whose new pieces have the same uids: Greenway is A and
    /// Industrial is B. U has 2 pairs of one kind (uids 7 and 10; 8, 9, 11 and 12 each name two kinds) and L has 4 (the
    /// sidewalks and the travel lanes), so the pairing is L.
    fn two_templates() -> (Street, Street) {
        use crate::street::model::Template;
        let baseline = sample();
        assert_eq!(baseline.next_uid, 7);
        let (a, b) = (arranged(&baseline, Template::Greenway), arranged(&baseline, Template::Industrial));
        assert_eq!(
            uids_and_kinds(&a),
            [(7, "sidewalk"), (8, "planting"), (9, "bike"), (10, "travel"), (11, "travel"), (12, "bike"), (13, "planting"), (14, "sidewalk")]
        );
        assert_eq!(uids_and_kinds(&b), [(7, "sidewalk"), (8, "loading"), (9, "travel"), (10, "travel"), (11, "loading"), (12, "sidewalk")]);
        (a, b)
    }

    #[test]
    fn a_baseline_piece_with_the_uid_of_a_template_piece_of_another_kind_is_not_that_piece() {
        // the scenario is made first, so it numbers its pieces from 7; the baseline then gains a bike lane, also 7
        let b = arranged(&sample(), crate::street::model::Template::Greenway);
        let mut a = sample();
        a.segments.insert(1, Segment::new(7, kind("bike"), 1_800));
        assert_eq!(uids_and_kinds(&a), [(1, "sidewalk"), (7, "bike"), (2, "parking"), (3, "travel"), (4, "travel"), (5, "parking"), (6, "sidewalk")]);
        assert_eq!(
            uids_and_kinds(&b),
            [(7, "sidewalk"), (8, "planting"), (9, "bike"), (10, "travel"), (11, "travel"), (12, "bike"), (13, "planting"), (14, "sidewalk")]
        );
        // U pairs uid 7, of two kinds, which parts the street before B's first piece: 3 pairs of one kind (the travel
        // lanes and the right sidewalk). L has 5 (the left sidewalk and the bike lane too), so the pairing is L.
        let (sidewalk, planting, bike, travel, parking) = (kind("sidewalk"), kind("planting"), kind("bike"), kind("travel"), kind("parking"));
        assert_eq!(
            pieces(&a, &b),
            (
                false,
                vec![
                    PieceChange::Width { kind: sidewalk, a_mm: 3_300, b_mm: 3_000 },
                    PieceChange::Added { kind: planting, width_mm: 1_800 },
                    PieceChange::Width { kind: bike, a_mm: 1_800, b_mm: 1_500 },
                    PieceChange::Width { kind: travel, a_mm: 3_300, b_mm: 2_700 },
                    PieceChange::Width { kind: travel, a_mm: 3_300, b_mm: 2_700 },
                    PieceChange::Added { kind: bike, width_mm: 1_500 },
                    PieceChange::Added { kind: planting, width_mm: 1_800 },
                    PieceChange::Width { kind: sidewalk, a_mm: 3_300, b_mm: 3_000 },
                    PieceChange::Removed { kind: parking, width_mm: 2_400 },
                    PieceChange::Removed { kind: parking, width_mm: 2_400 },
                ]
            )
        );
    }

    #[test]
    fn two_template_scenarios_numbered_alike_pair_their_sidewalks_and_travel_lanes() {
        let (a, b) = two_templates();
        let (planting, bike, travel, loading) = (kind("planting"), kind("bike"), kind("travel"), kind("loading"));
        // the sidewalks are 3 m in both, so nothing is said of them
        assert_eq!(
            pieces(&a, &b),
            (
                false,
                vec![
                    PieceChange::Added { kind: loading, width_mm: 2_400 },
                    PieceChange::Width { kind: travel, a_mm: 2_700, b_mm: 3_600 },
                    PieceChange::Width { kind: travel, a_mm: 2_700, b_mm: 3_600 },
                    PieceChange::Added { kind: loading, width_mm: 2_400 },
                    PieceChange::Removed { kind: planting, width_mm: 1_800 },
                    PieceChange::Removed { kind: bike, width_mm: 1_500 },
                    PieceChange::Removed { kind: bike, width_mm: 1_500 },
                    PieceChange::Removed { kind: planting, width_mm: 1_800 },
                ]
            )
        );
    }

    #[test]
    fn two_template_scenarios_numbered_alike_differ_in_no_piece_detail() {
        // by uid a bike lane, which has a curb, would be a travel lane, which has none; the sidewalks and travel lanes
        // that L pairs have the same surfaces and curbs
        let (a, b) = two_templates();
        assert_eq!(others(&a, &b), vec![]);
    }

    #[test]
    fn pieces_with_colliding_uids_pair_by_uid_only_when_both_sections_hold_the_same_uids() {
        let a = made_of(&[(1, "sidewalk"), (2, "planting"), (3, "bike"), (4, "travel"), (5, "sidewalk")]);
        // the same uids: by uid, though U has 3 pairs of one kind (1, 3 and 5) and L has 5
        let same = made_of(&[(5, "sidewalk"), (4, "planting"), (3, "bike"), (2, "travel"), (1, "sidewalk")]);
        let (planting, travel) = (kind("planting"), kind("travel"));
        assert_eq!(pieces(&a, &same), (false, vec![PieceChange::Kind { from: travel, to: planting }, PieceChange::Kind { from: planting, to: travel }]));
        assert_eq!(others(&a, &same), vec![Other::PieceDetails]);
        // one uid only B has: U has 2 pairs of one kind (1 and 3; the pieces 5 and 9 are in no common gap) and L has 5, so
        // the pairing is L, which has no order by uid to differ
        let fresh = made_of(&[(9, "sidewalk"), (4, "planting"), (3, "bike"), (2, "travel"), (1, "sidewalk")]);
        assert_eq!(pieces(&a, &fresh), (false, vec![]));
        assert_eq!(others(&a, &fresh), vec![]);
    }

    #[test]
    fn a_kept_piece_moved_to_the_far_side_does_not_unpair_a_rebuilt_roadway() {
        let a = made_of(&[(1, "sidewalk"), (2, "planting"), (3, "travel"), (4, "travel"), (5, "planting"), (6, "sidewalk")]);
        let mut b = made_of(&[(2, "planting"), (7, "travel"), (8, "travel"), (5, "planting"), (6, "sidewalk"), (1, "sidewalk")]);
        b.segments[2].width_mm = 3_500;
        // the uid pairs in A's order are at 5, 0, 3 and 4 in B: the longest rising run, 0, 3 and 4, is the anchors, so the
        // roadway pieces share the gap between the plantings. U has 6 pairs of one kind (1, 2, 5 and 6 by uid, the two
        // lanes by alignment) and L has 5 (all but a sidewalk), so the pairing is U.
        assert_eq!(pieces(&a, &b), (false, vec![PieceChange::Width { kind: kind("travel"), a_mm: 3_000, b_mm: 3_500 }]));
        // the moved sidewalk changes the order of the pieces paired by uid
        assert_eq!(others(&a, &b), vec![Other::PieceDetails]);
    }
}
