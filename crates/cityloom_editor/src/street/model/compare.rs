//! What differs between two designs of one street, beyond the impact summary: the lanes each way, the pieces added,
//! removed or changed, and the other things that set the two apart (speed limits, stretches, curbside uses, stops,
//! raised elements, greenery).
//!
//! Pure: no wording but `Said`. The designs are the street's own data; nothing is read from the page.

use super::{Segment, Street};
use crate::shared::catalogue::{DIRECTIONS, DirectionRule, KINDS};
use crate::shared::said::{Arg, Said};

/// How many pieces of the base section run one way and must (driving lanes) in one direction, in each of two designs.
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

/// One difference between the base sections of the two designs; a piece is the same piece in both by its `uid`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceChange {
    /// A piece only B has.
    Added { kind: usize, width_mm: i32 },
    /// A piece only A has.
    Removed { kind: usize, width_mm: i32 },
    /// A piece that is of another kind in B.
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
    /// Something of the base pieces other than their kinds and widths (surface, curb, direction of a piece that
    /// need not have one, times of day), or their order.
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

/// The pieces added, removed or changed from design A to design B in the base section, in B's order and then A's
/// removed ones, and whether the stretches along the street (where each starts, and how many) differ.
pub fn pieces(a: &Street, b: &Street) -> (bool, Vec<PieceChange>) {
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
    let starts = |s: &Street| s.zones.iter().map(|z| z.from_mm).collect::<Vec<_>>();
    (starts(a) != starts(b), changes)
}

/// The sections of a street with their uids cleared, which say nothing of what a design holds.
fn without_uids(section: &[Segment]) -> Vec<Segment> {
    section.iter().map(|p| Segment { uid: 0, ..p.clone() }).collect()
}

/// Whether the base sections differ in more than what `pieces` lists: a field other than kind and width of a piece
/// both have (a driving lane's direction is `lanes`'), or the order of the pieces both have.
fn piece_details_differ(a: &Street, b: &Street) -> bool {
    let shared =
        |from: &Street, other: &Street| from.segments.iter().filter(|p| other.segments.iter().any(|q| q.uid == p.uid)).map(|p| p.uid).collect::<Vec<_>>();
    b.segments.iter().any(|now| {
        a.segments.iter().find(|p| p.uid == now.uid).is_some_and(|was| {
            let driving = |p: &Segment| KINDS[p.kind].direction == DirectionRule::Required;
            let direction = if driving(was) || driving(now) { now.direction } else { was.direction };
            Segment { kind: now.kind, width_mm: now.width_mm, direction, ..was.clone() } != *now
        })
    }) || shared(a, b) != shared(b, a)
}

/// A list whose order carries no meaning, in an order that is the same for the same contents. It sorts by the
/// derived `Debug` text, so it relies on every field being in the derived `Debug`.
fn unordered<T: Clone + std::fmt::Debug>(list: &[T]) -> Vec<T> {
    let mut sorted = list.to_vec();
    sorted.sort_by_cached_key(|x| format!("{x:?}"));
    sorted
}

/// The things beyond the base section that differ between the two designs, in the order of `Other`.
pub fn others(a: &Street, b: &Street) -> Vec<Other> {
    let speeds = |s: &Street| (s.speed, s.zones.iter().map(|z| z.speed).collect::<Vec<_>>());
    // A zone's saved `name` is left out: the page never shows it (a stretch is named by where it lies).
    let stretches = |s: &Street| s.zones.iter().map(|z| (z.from_mm, z.segments.as_deref().map(without_uids))).collect::<Vec<_>>();
    [
        (Other::Speed, speeds(a) != speeds(b)),
        (Other::Stretches, stretches(a) != stretches(b)),
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
}
