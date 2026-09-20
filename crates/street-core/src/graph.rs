//! `StreetNetworkGraph` — the network of imported streets.

use std::collections::HashMap;

use crate::street::{Street, StreetIdentity};

/// An intersection's identity in the network graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntersectionId(String);

impl IntersectionId {
    /// Builds an `IntersectionId` from its underlying identifier.
    pub fn new(id: impl Into<String>) -> IntersectionId {
        IntersectionId(id.into())
    }
}

/// The network of imported streets and how they connect — a core type,
/// not an adapter type, so the outer ring never depends on osm2streets'
/// own graph representation. Read-only from this Unit's perspective: the
/// adjacency between intersections and streets is supplied at
/// construction by whatever built the graph (`u4-street-import`'s
/// adapter, via the `StreetSource` port) and this Unit never recomputes
/// or mutates it.
#[derive(Clone, Debug, PartialEq)]
pub struct StreetNetworkGraph {
    graph_id: String,
    streets: Vec<Street>,
    intersections: Vec<IntersectionId>,
    adjacency: HashMap<IntersectionId, Vec<StreetIdentity>>,
}

impl StreetNetworkGraph {
    /// Builds a `StreetNetworkGraph` from its streets, intersections and
    /// the adjacency between them.
    pub fn new(
        graph_id: impl Into<String>,
        streets: Vec<Street>,
        intersections: Vec<IntersectionId>,
        adjacency: HashMap<IntersectionId, Vec<StreetIdentity>>,
    ) -> StreetNetworkGraph {
        StreetNetworkGraph {
            graph_id: graph_id.into(),
            streets,
            intersections,
            adjacency,
        }
    }

    /// This graph's own identifier.
    pub fn graph_id(&self) -> &str {
        &self.graph_id
    }

    /// Iterates this graph's streets via a shared reference — the graph
    /// is immutable once constructed (BR5.1), so nothing in this Unit's
    /// API can mutate a `Street` reached through it.
    pub fn streets(&self) -> std::slice::Iter<'_, Street> {
        self.streets.iter()
    }

    /// This graph's intersections.
    pub fn intersections(&self) -> &[IntersectionId] {
        &self.intersections
    }

    /// Which streets meet at a given intersection, if any are recorded.
    pub fn streets_at(&self, intersection: &IntersectionId) -> Option<&[StreetIdentity]> {
        self.adjacency.get(intersection).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lane::{Direction, Lane};
    use crate::provenance::{Dimension, Provenance};
    use crate::street::BoundingNodeIds;

    fn a_street(osm_way_id: &str) -> Street {
        Street::new(
            osm_way_id,
            BoundingNodeIds::new("100", "200"),
            None,
            vec![Lane::new(
                "Travel",
                Direction::Forward,
                Dimension {
                    metres: 3.0,
                    provenance: Provenance::Mapped,
                },
                0,
            )],
        )
    }

    #[test]
    fn a_graph_reports_the_streets_it_was_built_with() {
        let graph = StreetNetworkGraph::new(
            "graph-1",
            vec![a_street("way-1"), a_street("way-2")],
            vec![],
            HashMap::new(),
        );

        let ids: Vec<String> = graph
            .streets()
            .map(|street| street.identity().osm_way_id)
            .collect();

        assert_eq!(ids, vec!["way-1".to_string(), "way-2".to_string()]);
    }

    #[test]
    fn an_empty_graph_reports_no_streets() {
        let graph = StreetNetworkGraph::new("graph-1", vec![], vec![], HashMap::new());

        assert_eq!(graph.streets().count(), 0);
    }

    #[test]
    fn a_graph_reports_its_own_id() {
        let graph = StreetNetworkGraph::new("graph-1", vec![], vec![], HashMap::new());

        assert_eq!(graph.graph_id(), "graph-1");
    }
}
