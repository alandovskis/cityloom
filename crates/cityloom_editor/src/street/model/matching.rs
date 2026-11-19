//! How the pieces of two sections of a street pair up: by uid, then by kind, left to right.

use super::Segment;

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

/// For each piece of `a`, its index in `b`, paired by `match_pieces`; `None` where `b` has no such piece.
pub(crate) fn counterpart(a: &[Segment], b: &[Segment]) -> Vec<Option<usize>> {
    let mut out = vec![None; a.len()];
    for (i, j) in match_pieces(a, b) {
        if let (Some(i), Some(j)) = (i, j) {
            out[i] = Some(j);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::catalogue::kind_index;

    fn seg(uid: u32, id: &str, w: i32) -> Segment {
        Segment::new(uid, kind_index(id).unwrap(), w)
    }

    #[test]
    fn counterpart_matches_by_uid_first() {
        let a = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500), seg(3, "sidewalk", 2000)];
        let b = vec![seg(1, "sidewalk", 2500), seg(2, "travel", 3000), seg(3, "sidewalk", 1500)];
        assert_eq!(counterpart(&a, &b), vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn counterpart_falls_back_to_kind_left_to_right() {
        let a = vec![seg(1, "sidewalk", 2000), seg(2, "travel", 3500), seg(3, "travel", 3500), seg(4, "sidewalk", 2000)];
        let b = vec![seg(11, "sidewalk", 2000), seg(12, "travel", 3500), seg(13, "travel", 3500), seg(14, "sidewalk", 2000)];
        assert_eq!(counterpart(&a, &b), vec![Some(0), Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn counterpart_leaves_a_piece_with_no_kind_in_b_unmatched() {
        let a = vec![seg(1, "sidewalk", 2000), seg(2, "bike", 1500), seg(3, "travel", 3500)];
        let b = vec![seg(1, "sidewalk", 2000), seg(3, "travel", 3500)];
        assert_eq!(counterpart(&a, &b), vec![Some(0), None, Some(1)]);
    }
}
