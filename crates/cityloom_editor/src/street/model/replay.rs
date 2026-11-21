//! Replaying an edit made on one section of a street onto another section.
//!
//! The edit is read as the difference between the section before and after it (the same section, so its pieces keep
//! their uids except the ones added or removed) and made again on the target section, on each piece's counterpart there
//! (`counterpart`: by uid, then by kind left to right). Replay never refuses: refusals come from the edit on the shown
//! section alone. It reports whether a changed, removed or moved piece had no counterpart in the target.

use super::Segment;
use super::matching::{counterpart, rising};

/// The target section after the replay, and whether some of the edit found nowhere to land in it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Replayed {
    pub section: Vec<Segment>,
    pub missed: bool,
}

/// Makes the edit that turned `before` into `after` on `target`.
///
/// - A piece of `before` still in `after` (same uid) passes every field the edit changed (kind, width, material, curb,
///   direction, tram, variants) to its counterpart; the fields the edit did not change keep the target's values.
/// - A piece removed by the edit has its counterpart removed.
/// - A piece added by the edit is inserted, with its uid, right after the counterpart of its left neighbour in `after`,
///   else right before the counterpart of its right neighbour, else at the same end (failing those, beside the nearest
///   piece that has a counterpart, else at the nearer end).
/// - A piece the edit moved (its order among the pieces kept changed) has its counterpart placed in the same way.
/// - A piece whose kind the edit changed is then brought inside what the new kind allows (`fit_to_kind`).
/// - A piece with no counterpart leaves the target as it was for that piece; when the edit changed, removed or moved
///   it, the replay `missed`.
pub(super) fn replay(before: &[Segment], after: &[Segment], target: &[Segment]) -> Replayed {
    let to_target = counterpart(before, target);
    let to_after: Vec<Option<usize>> = before.iter().map(|b| after.iter().position(|a| a.uid == b.uid)).collect();
    let moved = moved(&to_after);
    let mut missed = false;
    let mut section = target.to_vec();
    // For each piece of the target: the piece of `after` it now stands for, and whether it leaves its place.
    let mut stands_for: Vec<Option<usize>> = vec![None; target.len()];
    let mut lifted = vec![false; target.len()];
    // The pieces to put in place, by their index in `after`.
    let mut to_place: Vec<Option<Segment>> = vec![None; after.len()];
    for (i, b) in before.iter().enumerate() {
        let edited = moved[i] || to_after[i].is_none_or(|a| after[a] != *b);
        match (to_target[i], to_after[i]) {
            (None, _) => missed |= edited,
            (Some(t), None) => lifted[t] = true,
            (Some(t), Some(a)) => {
                carry(&mut section[t], b, &after[a]);
                if moved[i] {
                    lifted[t] = true;
                    to_place[a] = Some(section[t].clone());
                } else {
                    stands_for[t] = Some(a);
                }
            }
        }
    }
    for (a, piece) in after.iter().enumerate() {
        if !before.iter().any(|b| b.uid == piece.uid) {
            to_place[a] = Some(piece.clone());
        }
    }
    let mut out: Vec<(Option<usize>, Segment)> =
        section.into_iter().zip(stands_for).zip(lifted).filter(|(_, lifted)| !lifted).map(|((piece, a), _)| (a, piece)).collect();
    for (a, piece) in to_place.into_iter().enumerate() {
        if let Some(piece) = piece {
            let at = place(&out, a, after.len());
            out.insert(at, (Some(a), piece));
        }
    }
    Replayed { section: out.into_iter().map(|(_, piece)| piece).collect(), missed }
}

/// Sets on `target` each field that differs between `before` and `after`, to `after`'s value. A piece whose kind
/// changed is then brought inside what its new kind allows (`fit_to_kind`): the fields the edit left may not suit it.
fn carry(target: &mut Segment, before: &Segment, after: &Segment) {
    let kind_changed = before.kind != after.kind;
    if kind_changed {
        target.kind = after.kind;
    }
    if before.width_mm != after.width_mm {
        target.width_mm = after.width_mm;
    }
    if before.material != after.material {
        target.material = after.material;
    }
    if before.curb != after.curb {
        target.curb = after.curb;
    }
    if before.direction != after.direction {
        target.direction = after.direction;
    }
    if before.tram != after.tram {
        target.tram = after.tram;
    }
    if before.variants != after.variants {
        target.variants.clone_from(&after.variants);
    }
    if kind_changed {
        super::fit_to_kind(target);
    }
}

/// Which pieces of `before` the edit moved, given each one's place in `after`: of the pieces kept, those outside the
/// longest run whose places in `after` rise with their places in `before` (`rising`), so a piece moved across several
/// others is the only one moved.
fn moved(to_after: &[Option<usize>]) -> Vec<bool> {
    let kept: Vec<(usize, usize)> = to_after.iter().enumerate().filter_map(|(i, a)| a.map(|a| (i, a))).collect();
    let mut moved = vec![false; to_after.len()];
    for (&(i, _), rises) in kept.iter().zip(rising(&kept.iter().map(|&(_, a)| a).collect::<Vec<_>>())) {
        moved[i] = !rises;
    }
    moved
}

/// Where in `out` (target pieces, each with the index in `after` it stands for) the piece at `a` of the `n` pieces of
/// `after` goes.
fn place(out: &[(Option<usize>, Segment)], a: usize, n: usize) -> usize {
    let found = |x: usize| out.iter().position(|(stands_for, _)| *stands_for == Some(x));
    if a > 0
        && let Some(p) = found(a - 1)
    {
        return p + 1;
    }
    if let Some(p) = found(a + 1) {
        return p;
    }
    if a == 0 {
        return 0;
    }
    if a + 1 == n {
        return out.len();
    }
    if let Some(p) = (0..a - 1).rev().find_map(found) {
        return p + 1;
    }
    if let Some(p) = (a + 2..n).find_map(found) {
        return p;
    }
    if 2 * a < n { 0 } else { out.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::kind_index;

    fn seg(uid: u32, id: &str, w: i32) -> Segment {
        Segment::new(uid, kind_index(id).unwrap(), w)
    }

    fn widths(s: &[Segment]) -> Vec<i32> {
        s.iter().map(|p| p.width_mm).collect()
    }

    fn uids(s: &[Segment]) -> Vec<u32> {
        s.iter().map(|p| p.uid).collect()
    }

    #[test]
    fn a_width_change_reaches_the_same_piece_and_nothing_else() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500), seg(3, "sidewalk", 2000)];
        let mut after = before.clone();
        after[0].width_mm = 3000;
        let target = vec![seg(1, "sidewalk", 1800), seg(2, "travel", 3200), seg(3, "sidewalk", 2200)];
        let out = replay(&before, &after, &target);
        assert_eq!(widths(&out.section), vec![3000, 3200, 2200]);
        assert_eq!(uids(&out.section), vec![1, 2, 3]);
        assert!(!out.missed);
    }

    #[test]
    fn a_change_reaches_a_piece_matched_by_kind_when_uids_differ() {
        // The School template gives the school stretch fresh uids: the sidewalk is still found by its kind.
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500), seg(3, "sidewalk", 2000)];
        let mut after = before.clone();
        after[0].width_mm = 3000;
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "travel", 3200), seg(13, "sidewalk", 2200)];
        let out = replay(&before, &after, &target);
        assert_eq!(widths(&out.section), vec![3000, 3200, 2200]);
        assert_eq!(uids(&out.section), vec![11, 12, 13]);
        assert!(!out.missed);
    }

    #[test]
    fn a_change_keeps_the_fields_the_edit_did_not_touch() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500)];
        let mut after = before.clone();
        after[1].direction = Some(1);
        let mut target = before.clone();
        target[1].width_mm = 3000;
        target[1].material = 1;
        let out = replay(&before, &after, &target);
        assert_eq!(out.section[1].direction, Some(1));
        assert_eq!(out.section[1].width_mm, 3000);
        assert_eq!(out.section[1].material, 1);
        assert_eq!(out.section[0], target[0]);
    }

    #[test]
    fn an_added_piece_lands_beside_the_same_neighbour_with_the_same_uid() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500), seg(3, "sidewalk", 2000)];
        let after = vec![seg(1, "sidewalk", 2000), seg(9, "bike", 1500), seg(2, "travel", 3500), seg(3, "sidewalk", 2000)];
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "parking", 2400), seg(13, "travel", 3200), seg(14, "sidewalk", 2200)];
        let out = replay(&before, &after, &target);
        assert_eq!(uids(&out.section), vec![11, 9, 12, 13, 14]);
        assert_eq!(out.section[1], after[1]);
        assert!(!out.missed);
    }

    #[test]
    fn an_added_piece_with_no_left_neighbour_goes_before_its_right_one() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500)];
        let after = vec![seg(9, "planting", 1500), seg(1, "sidewalk", 2000), seg(2, "travel", 3500)];
        let target = vec![seg(11, "parking", 2400), seg(12, "sidewalk", 1800), seg(13, "travel", 3200)];
        let out = replay(&before, &after, &target);
        assert_eq!(uids(&out.section), vec![11, 9, 12, 13]);
        assert!(!out.missed);
    }

    #[test]
    fn pieces_added_side_by_side_keep_their_order() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500)];
        let after = vec![seg(1, "sidewalk", 2000), seg(8, "bike", 1500), seg(9, "parking", 2400), seg(2, "travel", 3500)];
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "travel", 3200)];
        let out = replay(&before, &after, &target);
        assert_eq!(uids(&out.section), vec![11, 8, 9, 12]);
    }

    #[test]
    fn a_removed_piece_is_removed_where_it_has_a_counterpart() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "parking", 2400), seg(3, "travel", 3500)];
        let after = vec![seg(1, "sidewalk", 2000), seg(3, "travel", 3500)];
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "parking", 2000), seg(13, "travel", 3200)];
        let out = replay(&before, &after, &target);
        assert_eq!(uids(&out.section), vec![11, 13]);
        assert!(!out.missed);

        let lacking = vec![seg(11, "sidewalk", 1800), seg(13, "travel", 3200)];
        let out = replay(&before, &after, &lacking);
        assert_eq!(out.section, lacking);
        assert!(out.missed);
    }

    #[test]
    fn a_moved_piece_moves_beside_the_same_neighbour() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "bike", 1500), seg(3, "parking", 2400), seg(4, "travel", 3500)];
        let after = vec![seg(1, "sidewalk", 2000), seg(3, "parking", 2400), seg(4, "travel", 3500), seg(2, "bike", 1500)];
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "bike", 1800), seg(13, "parking", 2000), seg(14, "travel", 3200)];
        let out = replay(&before, &after, &target);
        assert_eq!(uids(&out.section), vec![11, 13, 14, 12]);
        assert_eq!(widths(&out.section), vec![1800, 2000, 3200, 1800]);
        assert!(!out.missed);
    }

    #[test]
    fn a_change_to_a_piece_the_target_lacks_leaves_the_target_as_it_was() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "bike", 1500), seg(3, "travel", 3500)];
        let mut after = before.clone();
        after[1].width_mm = 2000;
        let target = vec![seg(11, "sidewalk", 1800), seg(13, "travel", 3200)];
        let out = replay(&before, &after, &target);
        assert_eq!(out.section, target);
        assert!(out.missed);
    }

    #[test]
    fn an_unchanged_piece_the_target_lacks_is_not_a_miss() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "bike", 1500), seg(3, "travel", 3500)];
        let mut after = before.clone();
        after[2].width_mm = 3200;
        let target = vec![seg(11, "sidewalk", 1800), seg(13, "travel", 3500)];
        let out = replay(&before, &after, &target);
        assert_eq!(widths(&out.section), vec![1800, 3200]);
        assert!(!out.missed);
    }

    #[test]
    fn a_moved_piece_the_target_lacks_is_a_miss() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "bike", 1500), seg(3, "parking", 2400), seg(4, "travel", 3500)];
        let after = vec![seg(1, "sidewalk", 2000), seg(3, "parking", 2400), seg(4, "travel", 3500), seg(2, "bike", 1500)];
        let target = vec![seg(11, "sidewalk", 1800), seg(13, "parking", 2000), seg(14, "travel", 3200)];
        let out = replay(&before, &after, &target);
        assert_eq!(out.section, target);
        assert!(out.missed);
    }

    #[test]
    fn a_kind_change_reaches_a_piece_matched_by_kind() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "parking", 2400), seg(3, "travel", 3500)];
        let mut after = before.clone();
        after[1] = seg(2, "bike", 2400);
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "parking", 2000), seg(13, "travel", 3200)];
        let out = replay(&before, &after, &target);
        let bike = &out.section[1];
        assert_eq!((bike.uid, bike.kind, bike.width_mm), (12, kind_index("bike").unwrap(), 2000));
        assert_eq!((bike.material, bike.curb, bike.direction), (after[1].material, after[1].curb, after[1].direction));
        assert_eq!(out.section[0], target[0]);
        assert_eq!(out.section[2], target[2]);
        assert!(!out.missed);
    }

    #[test]
    fn an_added_piece_whose_neighbours_the_target_lacks_goes_beside_the_nearest_piece_it_has() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "bike", 1500), seg(3, "bus", 3300), seg(4, "travel", 3500), seg(5, "sidewalk", 2000)];
        let after = vec![
            seg(1, "sidewalk", 2000),
            seg(2, "bike", 1500),
            seg(9, "planting", 1500),
            seg(3, "bus", 3300),
            seg(4, "travel", 3500),
            seg(5, "sidewalk", 2000),
        ];
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "parking", 2400), seg(14, "travel", 3200), seg(15, "sidewalk", 2200)];
        let out = replay(&before, &after, &target);
        assert_eq!(uids(&out.section), vec![11, 9, 12, 14, 15]);
        assert!(!out.missed);
    }

    #[test]
    fn an_added_last_piece_whose_left_neighbour_the_target_lacks_goes_at_the_right_end() {
        let before = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500), seg(3, "bike", 1500)];
        let after = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500), seg(3, "bike", 1500), seg(9, "sidewalk", 2000)];
        let target = vec![seg(11, "sidewalk", 1800), seg(12, "travel", 3200), seg(13, "parking", 2400)];
        let out = replay(&before, &after, &target);
        assert_eq!(uids(&out.section), vec![11, 12, 13, 9]);
        assert!(!out.missed);
    }

    #[test]
    fn a_kind_change_leaves_no_surface_or_curb_the_new_kind_does_not_take() {
        // The shown sidewalk becomes a travel lane keeping its concrete; the target's brick and curb do not suit one.
        let before = vec![seg(1, "sidewalk", 2000)];
        let travel = kind_index("travel").unwrap();
        let after = vec![Segment { kind: travel, direction: Some(1), curb: None, ..before[0].clone() }];
        let mut target = vec![seg(11, "sidewalk", 1800)];
        target[0].material = 3;
        target[0].curb = Some(4);
        let out = replay(&before, &after, &target).section;
        let k = &crate::shared::catalogue::KINDS[travel];
        assert_eq!(out[0].kind, travel);
        assert_eq!(out[0].material, k.materials[0], "brick is no travel surface: the kind's own");
        assert_eq!((out[0].curb, out[0].direction), (None, Some(1)));
        let (lo, hi) = crate::standards::model::kind_bounds(travel);
        assert!((lo..=hi).contains(&out[0].width_mm), "the width is within the kind's");
    }
}
