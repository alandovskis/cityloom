//! The identity of a place: a key built from the OpenStreetMap ids it was made from, the same whichever tile it is read from.

use std::collections::BTreeSet;

use crate::shared::provenance::OsmRef;

/// A junction's key: `n` and the smallest OpenStreetMap node of the set it was made from.
pub fn junction_key(source: &[OsmRef]) -> Option<String> {
    source.iter().map(|r| r.id).min().map(|id| format!("n{id}"))
}

/// What stands for the end of a street where the road leaves the area's box: that end is a node the importer made,
/// with no OpenStreetMap node.
const OPEN_END: &str = "x";

/// The way a street leaves an end, in four words: each a quarter of the compass centred on north, east, south or
/// west, so that the two pieces of a way through a junction (they leave it more than a quarter apart) differ. The
/// bearing is not fixed by OpenStreetMap alone (`Layout::leaving_deg`): it is measured from the junction's point
/// and along the trimmed centreline, which osm2streets moves when the junction's cluster changes with the box, by up
/// to 60 degrees measured on the Plateau. So a street's word can flip far from 45 degrees off a compass point. Where
/// the key then differs, matching by the ways and nodes and the bearing finds the street again (`edits::assign`);
/// the residual risk is an exact key, which outranks the bearing check, naming another piece after a flip (none
/// seen over 11 probed areas). Anchoring the key on the next OSM node along the way is the proper fix, deferred.
const HEADINGS: [&str; 4] = ["n", "e", "s", "w"];

/// The word for leaving at `deg` degrees clockwise from north.
pub fn heading(deg: f64) -> &'static str {
    HEADINGS[((deg.rem_euclid(360.0) + 45.0) / 90.0) as usize % 4]
}

/// One end of a street as its key sees it: the OpenStreetMap nodes there (none where the road is cut at the area's
/// box) and the way the street leaves it, in degrees clockwise from north.
#[derive(Clone, Copy)]
pub struct End<'a> {
    pub source: &'a [OsmRef],
    pub leaving_deg: f64,
}

/// A street's key: `w` and its smallest way, then the keys of its two ends in sorted order (so the pieces of one way
/// differ, and which end is `a` does not matter). An end with no OpenStreetMap node, where a road leaves the area's
/// box, is `x`, and the key then ends with the way the street leaves its other end (`w7~n2~x~e`): the two pieces of
/// a way through junction n2, both cut at the box, are told apart (the word itself can differ between boxes: see
/// `HEADINGS`). `twinned` says that another
/// street of the same way joins the same two ends (the halves of a closed way): the key then ends with the way the
/// street leaves its first end. Any other street with two real ends is keyed by its way and ends alone, as before.
pub fn street_key(ways: &[OsmRef], a: End, b: End, twinned: bool) -> Option<String> {
    let way = ways.iter().map(|r| r.id).min()?;
    let mut ends = [(junction_key(a.source), a.leaving_deg), (junction_key(b.source), b.leaving_deg)];
    // a real end (`n…`) sorts before an open one (`x`)
    ends.sort_by(|p, q| (p.0.is_none(), &p.0).cmp(&(q.0.is_none(), &q.0)));
    let word = |end: &(Option<String>, f64)| end.0.clone().unwrap_or_else(|| OPEN_END.to_string());
    let key = format!("w{way}~{}~{}", word(&ends[0]), word(&ends[1]));
    let leaves = match (&ends[0].0, &ends[1].0) {
        (Some(_), None) => true,
        (Some(_), Some(_)) => twinned,
        _ => false,
    };
    Some(if leaves { format!("{key}~{}", heading(ends[0].1)) } else { key })
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

    /// A street's key from its ways and its ends' nodes, leaving `a` northward and `b` southward.
    fn key(ways: &[i64], a: &[i64], b: &[i64]) -> Option<String> {
        let (a, b) = (refs(a), refs(b));
        street_key(&refs(ways), End { source: &a, leaving_deg: 0.0 }, End { source: &b, leaving_deg: 180.0 }, false)
    }

    #[test]
    fn a_junction_is_named_by_the_smallest_osm_node_of_its_set() {
        assert_eq!(junction_key(&refs(&[32661316, 212641295, 1429454590])).as_deref(), Some("n32661316"));
        assert_eq!(junction_key(&refs(&[1429454590, 32661316])), junction_key(&refs(&[32661316, 1429454590])), "the order of the set does not matter");
        assert_eq!(junction_key(&[]), None);
    }

    #[test]
    fn a_street_is_named_by_its_way_and_its_two_ends_in_sorted_order() {
        let k = key(&[4911698], &[215729261], &[32124773]);
        assert_eq!(k.as_deref(), Some("w4911698~n215729261~n32124773"));
        assert_eq!(k, key(&[4911698], &[32124773], &[215729261]), "which end is `a` does not matter");
    }

    #[test]
    fn the_pieces_of_one_way_are_told_apart_by_their_ends() {
        assert_ne!(key(&[7], &[1], &[2]), key(&[7], &[2], &[3]));
    }

    #[test]
    fn a_road_made_of_several_ways_is_named_by_the_smallest() {
        assert_eq!(key(&[90, 12, 40], &[1], &[2]).as_deref(), Some("w12~n1~n2"));
    }

    #[test]
    fn a_street_whose_way_has_no_osm_id_has_no_key() {
        assert_eq!(key(&[], &[1], &[2]), None);
    }

    fn leaving(ways: &[i64], a: (&[i64], f64), b: (&[i64], f64), twinned: bool) -> Option<String> {
        let (ra, rb) = (refs(a.0), refs(b.0));
        street_key(&refs(ways), End { source: &ra, leaving_deg: a.1 }, End { source: &rb, leaving_deg: b.1 }, twinned)
    }

    #[test]
    fn a_street_leaving_the_area_is_keyed_by_its_other_end_and_the_way_it_leaves_it() {
        assert_eq!(leaving(&[7], (&[], 0.0), (&[2], 90.0), false).as_deref(), Some("w7~n2~x~e"));
        assert_eq!(leaving(&[7], (&[2], 90.0), (&[], 0.0), false).as_deref(), Some("w7~n2~x~e"), "which end is `a` does not matter");
        assert_eq!(leaving(&[7], (&[], 0.0), (&[], 90.0), false).as_deref(), Some("w7~x~x"), "no real end: nothing to leave");
    }

    #[test]
    fn the_two_pieces_of_a_way_through_a_junction_both_cut_at_the_box_have_different_keys() {
        // the way runs through n2 from west to east; the box cuts it on both sides
        let east = leaving(&[7], (&[2], 92.0), (&[], 270.0), false);
        let west = leaving(&[7], (&[], 90.0), (&[2], 268.0), false);
        assert_eq!((east.as_deref(), west.as_deref()), (Some("w7~n2~x~e"), Some("w7~n2~x~w")));
    }

    #[test]
    fn the_halves_of_a_closed_way_are_told_apart_by_the_way_they_leave_their_first_end() {
        let north = leaving(&[7], (&[1], 10.0), (&[2], 170.0), true);
        let south = leaving(&[7], (&[2], 10.0), (&[1], 170.0), true);
        assert_eq!((north.as_deref(), south.as_deref()), (Some("w7~n1~n2~n"), Some("w7~n1~n2~s")));
        assert_eq!(leaving(&[7], (&[1], 10.0), (&[2], 170.0), false).as_deref(), Some("w7~n1~n2"), "a street with two real ends and no twin keeps its key");
    }

    #[test]
    fn a_heading_is_a_quarter_of_the_compass_centred_on_its_point() {
        let words: Vec<&str> = [0.0, 44.9, 45.0, 134.9, 135.0, 224.9, 225.0, 314.9, 315.0, 359.9, -10.0, 370.0].into_iter().map(heading).collect();
        assert_eq!(words, ["n", "n", "e", "e", "s", "s", "w", "w", "n", "n", "n", "n"]);
    }
}
