//! The identity of a place: a key built from the OpenStreetMap ids it was made from, the same whichever tile it is read from.

use std::collections::BTreeSet;

use crate::shared::provenance::OsmRef;

/// A junction's key: `n` and the smallest OpenStreetMap node of the set it was made from.
pub fn junction_key(source: &[OsmRef]) -> Option<String> {
    source.iter().map(|r| r.id).min().map(|id| format!("n{id}"))
}

/// A street's key: `w` and its smallest way, then the keys of its two ends in sorted order (so the pieces of one way
/// differ, and which end is `a` does not matter).
pub fn street_key(ways: &[OsmRef], end_a: &[OsmRef], end_b: &[OsmRef]) -> Option<String> {
    let way = ways.iter().map(|r| r.id).min()?;
    let mut ends = [junction_key(end_a)?, junction_key(end_b)?];
    ends.sort();
    Some(format!("w{way}~{}~{}", ends[0], ends[1]))
}

/// The OpenStreetMap ids of a source, as a set.
pub fn ids(source: &[OsmRef]) -> BTreeSet<i64> {
    source.iter().map(|r| r.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refs(ids: &[i64]) -> Vec<OsmRef> {
        ids.iter().map(|&id| OsmRef { id, version: None }).collect()
    }

    #[test]
    fn a_junction_is_named_by_the_smallest_osm_node_of_its_set() {
        assert_eq!(junction_key(&refs(&[32661316, 212641295, 1429454590])).as_deref(), Some("n32661316"));
        assert_eq!(junction_key(&refs(&[1429454590, 32661316])), junction_key(&refs(&[32661316, 1429454590])), "the order of the set does not matter");
        assert_eq!(junction_key(&[]), None);
    }

    #[test]
    fn a_street_is_named_by_its_way_and_its_two_ends_in_sorted_order() {
        let key = street_key(&refs(&[4911698]), &refs(&[215729261]), &refs(&[32124773]));
        assert_eq!(key.as_deref(), Some("w4911698~n215729261~n32124773"));
        assert_eq!(key, street_key(&refs(&[4911698]), &refs(&[32124773]), &refs(&[215729261])), "which end is `a` does not matter");
    }

    #[test]
    fn the_pieces_of_one_way_are_told_apart_by_their_ends() {
        let a = street_key(&refs(&[7]), &refs(&[1]), &refs(&[2]));
        let b = street_key(&refs(&[7]), &refs(&[2]), &refs(&[3]));
        assert_ne!(a, b);
    }

    #[test]
    fn a_road_made_of_several_ways_is_named_by_the_smallest() {
        assert_eq!(street_key(&refs(&[90, 12, 40]), &refs(&[1]), &refs(&[2])).as_deref(), Some("w12~n1~n2"));
    }

    #[test]
    fn a_street_whose_way_or_either_end_has_no_osm_id_has_no_key() {
        assert_eq!(street_key(&[], &refs(&[1]), &refs(&[2])), None);
        assert_eq!(street_key(&refs(&[7]), &[], &refs(&[2])), None);
        assert_eq!(street_key(&refs(&[7]), &refs(&[1]), &[]), None);
    }
}
