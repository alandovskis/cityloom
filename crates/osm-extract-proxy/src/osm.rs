//! `osm` — the in-memory OpenStreetMap elements (`Node`, `Way`) and a `Clip`
//! of them, shared by the build tool and the cut. Coordinates are the raw
//! integers the OSM PBF format encodes (`decode.rs`/`encode.rs` never widen
//! them): at the default granularity of 100, one raw unit is 100
//! nanodegrees, so a raw value is converted to true nanodegrees by
//! multiplying by [`RAW_GRANULARITY`] before any [`crate::geo`] conversion
//! runs. Pure.

use std::collections::HashMap;

use crate::geo::Extent;

pub use crate::geo::{nano_to_units_ceil, nano_to_units_floor};

/// The OSM PBF default granularity: nanodegrees per raw coordinate unit.
pub const RAW_GRANULARITY: i64 = 100;

/// An OSM node: an id and a position in integer nanodegrees, plus any tags
/// this Unit keeps (BR8.2 keeps only the tags the filter profile needs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: i64,
    pub lat: i64,
    pub lon: i64,
    pub tags: Vec<(String, String)>,
}

/// An OSM way: an id, its ordered node references, and its tags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Way {
    pub id: i64,
    pub refs: Vec<i64>,
    pub tags: Vec<(String, String)>,
}

/// A clip of elements: the unit the cut, the encoder and the store all
/// exchange. Nodes then ways, each sorted and de-duplicated by id (BR5.3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clip {
    pub nodes: Vec<Node>,
    pub ways: Vec<Way>,
}

impl Clip {
    /// Sort nodes and ways by id and drop duplicates by id, keeping the
    /// first occurrence's content (BR5.3). Idempotent.
    pub fn normalise(&mut self) {
        self.nodes.sort_by_key(|n| n.id);
        self.nodes.dedup_by_key(|n| n.id);
        self.ways.sort_by_key(|w| w.id);
        self.ways.dedup_by_key(|w| w.id);
    }

    pub fn has_ways(&self) -> bool {
        !self.ways.is_empty()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn way_count(&self) -> usize {
        self.ways.len()
    }

    /// The extent of a way from the nodes it references; `None` if any
    /// referenced node is missing from this clip.
    pub fn way_extent(&self, way: &Way) -> Option<Extent> {
        let by_id: HashMap<i64, &Node> = self.nodes.iter().map(|n| (n.id, n)).collect();
        let mut points = Vec::with_capacity(way.refs.len());
        for r in &way.refs {
            let node = by_id.get(r)?;
            points.push((node.lon * RAW_GRANULARITY, node.lat * RAW_GRANULARITY));
        }
        Extent::of_points(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: i64) -> Node {
        Node {
            id,
            lat: id * 1_000,
            lon: -id * 1_000,
            tags: Vec::new(),
        }
    }

    fn way(id: i64, refs: &[i64]) -> Way {
        Way {
            id,
            refs: refs.to_vec(),
            tags: vec![("highway".to_string(), "residential".to_string())],
        }
    }

    // BR5.3 — elements are ordered by type (nodes then ways) and by id, and
    // each appears at most once however many cells contributed it.
    #[test]
    fn normalise_sorts_by_id_and_drops_duplicates() {
        let mut clip = Clip {
            nodes: vec![node(30), node(10), node(20), node(10)],
            ways: vec![way(2, &[10, 20]), way(1, &[20, 30]), way(2, &[10, 20])],
        };
        clip.normalise();
        assert_eq!(
            clip.nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
        assert_eq!(
            clip.ways.iter().map(|w| w.id).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn normalise_is_idempotent_and_keeps_element_content() {
        let mut clip = Clip {
            nodes: vec![node(2), node(1)],
            ways: vec![way(5, &[1, 2])],
        };
        clip.normalise();
        let once = clip.clone();
        clip.normalise();
        assert_eq!(clip, once);
        assert_eq!(
            clip.ways.first().map(|w| w.refs.as_slice()),
            Some(&[1, 2][..])
        );
    }

    #[test]
    fn element_counts_and_emptiness() {
        let empty = Clip::default();
        assert!(!empty.has_ways());
        assert_eq!(empty.node_count(), 0);
        let clip = Clip {
            nodes: vec![node(1), node(2)],
            ways: vec![way(1, &[1, 2])],
        };
        assert!(clip.has_ways());
        assert_eq!(clip.node_count(), 2);
        assert_eq!(clip.way_count(), 1);
    }

    #[test]
    fn way_extent_comes_from_its_nodes() {
        let clip = Clip {
            nodes: vec![
                Node {
                    id: 1,
                    lat: 436_500_000,
                    lon: -796_300_000,
                    tags: Vec::new(),
                },
                Node {
                    id: 2,
                    lat: 436_600_000,
                    lon: -796_200_000,
                    tags: Vec::new(),
                },
            ],
            ways: vec![way(1, &[1, 2]), way(2, &[1, 99])],
        };
        let extent = clip
            .way_extent(clip.ways.first().unwrap())
            .expect("both nodes present");
        assert_eq!(extent.min_lon, crate::geo::Units(-7_963_000));
        assert_eq!(extent.max_lat, crate::geo::Units(4_366_000));
        assert!(
            clip.way_extent(clip.ways.get(1).unwrap()).is_none(),
            "a missing node has no extent"
        );
    }

    #[test]
    fn nanodegrees_to_units_floors_toward_negative_infinity() {
        assert_eq!(nano_to_units_floor(15_000), crate::geo::Units(1));
        assert_eq!(nano_to_units_floor(-15_000), crate::geo::Units(-2));
        assert_eq!(nano_to_units_ceil(15_000), crate::geo::Units(2));
        assert_eq!(nano_to_units_ceil(-15_000), crate::geo::Units(-1));
        assert_eq!(nano_to_units_ceil(20_000), crate::geo::Units(2));
    }
}
