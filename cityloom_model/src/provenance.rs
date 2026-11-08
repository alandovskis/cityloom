use crate::{NodeRef, OsmRef, WayRef};

/// What a node or a road was made from in OpenStreetMap, each thing with the version it had when the data was
/// read. A road's are ways and a node's are nodes; the key an edit is kept under comes from them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Provenance {
    pub refs: Vec<OsmRef>,
}

impl Provenance {
    pub fn of_ways(ways: impl IntoIterator<Item = WayRef>) -> Provenance {
        Provenance { refs: ways.into_iter().map(OsmRef::Way).collect() }
    }

    pub fn of_nodes(nodes: impl IntoIterator<Item = NodeRef>) -> Provenance {
        Provenance { refs: nodes.into_iter().map(OsmRef::Node).collect() }
    }

    pub fn ways(&self) -> impl Iterator<Item = WayRef> + '_ {
        self.refs.iter().filter_map(|r| match r {
            OsmRef::Way(w) => Some(*w),
            OsmRef::Node(_) => None,
        })
    }

    pub fn nodes(&self) -> impl Iterator<Item = NodeRef> + '_ {
        self.refs.iter().filter_map(|r| match r {
            OsmRef::Node(n) => Some(*n),
            OsmRef::Way(_) => None,
        })
    }

    /// Whether it includes the way with this OSM id, whatever version of it.
    pub fn has_way(&self, id: i64) -> bool {
        self.ways().any(|w| w.id == id)
    }

    /// Whether it includes the node with this OSM id, whatever version of it.
    pub fn has_node(&self, id: i64) -> bool {
        self.nodes().any(|n| n.id == id)
    }

    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    pub fn clear(&mut self) {
        self.refs.clear();
    }

    /// Adds what `other` was made from.
    pub fn extend(&mut self, other: Provenance) {
        self.refs.extend(other.refs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_tells_the_ways_from_the_nodes() {
        let p = Provenance { refs: vec![WayRef::with_version(1, 3).into(), NodeRef::new(1).into(), WayRef::new(2).into()] };
        assert_eq!(p.ways().collect::<Vec<_>>(), [WayRef::with_version(1, 3), WayRef::new(2)]);
        assert_eq!(p.nodes().collect::<Vec<_>>(), [NodeRef::new(1)]);
    }

    #[test]
    fn it_asks_by_id_whatever_the_version() {
        let p = Provenance::of_ways([WayRef::with_version(7, 2)]);
        assert!(p.has_way(7));
        assert!(!p.has_way(8));
        assert!(!p.has_node(7), "a node of the same id is not the way");
    }

    #[test]
    fn extending_it_adds_what_the_other_was_made_from() {
        let mut p = Provenance::of_nodes([NodeRef::with_version(1, 101)]);
        p.extend(Provenance::of_nodes([NodeRef::with_version(5, 105)]));
        assert_eq!(p.nodes().collect::<Vec<_>>(), [NodeRef::with_version(1, 101), NodeRef::with_version(5, 105)]);
        p.clear();
        assert!(p.is_empty());
    }
}
