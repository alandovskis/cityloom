use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Network, Road, RoadId};

/// A node of the network's graph, by its id in it. Not an OSM node: see `NodeRef`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u32);

/// A place in the network's graph, apart from what stands there: see `Junction` for that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Node {
    pub id: NodeId,
}

/// A road as a link between two nodes, apart from what it is made of: see `Road` for that. Its id is the road's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge {
    pub id: RoadId,
    pub from: NodeId,
    pub to: NodeId,
}

impl Road {
    pub fn edge(&self) -> Edge {
        Edge { id: self.id, from: self.from, to: self.to }
    }
}

impl Network {
    /// The graph's nodes, one for each junction, in order.
    pub fn nodes(&self) -> Vec<Node> {
        self.junctions.iter().map(|j| Node { id: j.id }).collect()
    }

    /// The graph's edges, one for each road, in order.
    pub fn edges(&self) -> Vec<Edge> {
        self.roads.iter().map(Road::edge).collect()
    }
}

/// The edges grouped by the nodes they meet at: two are in one group when a chain of edges, each meeting the next at a
/// node, joins them. Each group lists its edges' ids in order, none twice; the groups are ordered by their lowest id.
pub fn connected(edges: &[Edge]) -> Vec<Vec<RoadId>> {
    let mut group_of: BTreeMap<NodeId, usize> = BTreeMap::new();
    let mut groups: Vec<Vec<RoadId>> = Vec::new();
    for edge in edges {
        let (a, b) = (group_of.get(&edge.from).copied(), group_of.get(&edge.to).copied());
        let target = match (a, b) {
            (Some(a), Some(b)) if a != b => {
                let moved = std::mem::take(&mut groups[b]);
                for g in group_of.values_mut().filter(|g| **g == b) {
                    *g = a;
                }
                groups[a].extend(moved);
                a
            }
            (Some(g), _) | (_, Some(g)) => g,
            (None, None) => {
                groups.push(Vec::new());
                groups.len() - 1
            }
        };
        groups[target].push(edge.id);
        group_of.insert(edge.from, target);
        group_of.insert(edge.to, target);
    }
    let mut found: Vec<Vec<RoadId>> = groups
        .into_iter()
        .filter(|g| !g.is_empty())
        .map(|mut ids| {
            ids.sort_unstable();
            ids.dedup();
            ids
        })
        .collect();
    found.sort_by_key(|g| g[0]);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(id: u32, from: u32, to: u32) -> Edge {
        Edge { id: RoadId(id), from: NodeId(from), to: NodeId(to) }
    }

    fn ids(groups: Vec<Vec<RoadId>>) -> Vec<Vec<u32>> {
        groups.into_iter().map(|g| g.into_iter().map(|r| r.0).collect()).collect()
    }

    #[test]
    fn a_node_id_is_kept_as_the_bare_number() {
        assert_eq!(serde_json::to_string(&NodeId(3)).unwrap(), "3");
        assert_eq!(serde_json::from_str::<NodeId>("3").unwrap(), NodeId(3));
    }

    #[test]
    fn edges_that_share_a_node_are_one_group_and_the_rest_are_apart() {
        let groups = connected(&[edge(1, 1, 2), edge(2, 3, 4), edge(3, 2, 5)]);
        assert_eq!(ids(groups), [vec![1, 3], vec![2]]);
    }

    #[test]
    fn an_edge_that_joins_two_groups_makes_them_one() {
        let groups = connected(&[edge(1, 1, 2), edge(2, 3, 4), edge(3, 2, 3), edge(4, 8, 9)]);
        assert_eq!(ids(groups), [vec![1, 2, 3], vec![4]]);
    }

    #[test]
    fn there_are_no_groups_without_edges() {
        assert!(connected(&[]).is_empty());
    }

    #[test]
    fn a_network_has_a_node_for_each_junction_and_an_edge_for_each_road() {
        use crate::{Control, Junction, Point, Provenance, RoadType, Zone};
        let junction =
            |id: u32| Junction { id: NodeId(id), provenance: Provenance::default(), position: Point::new(0.0, 0.0), junction: false, control: Control::None };
        let road = Road {
            id: RoadId(7),
            provenance: Provenance::default(),
            name: "Main".into(),
            road_type: RoadType::Local,
            from: NodeId(1),
            to: NodeId(2),
            zones: vec![Zone::whole(vec![])],
            speed_kmh: None,
            points: vec![],
        };
        let net = Network { left_hand: false, junctions: vec![junction(1), junction(2)], roads: vec![road], corridors: vec![] };
        assert_eq!(net.nodes(), [Node { id: NodeId(1) }, Node { id: NodeId(2) }]);
        assert_eq!(net.edges(), [edge(7, 1, 2)]);
    }
}
