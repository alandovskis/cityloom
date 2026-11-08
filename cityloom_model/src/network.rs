use serde::{Deserialize, Serialize};

use crate::{Control, Corridor, Junction, Road};

/// The street network.
///
/// A road without a name is left out when a network is read, and so is a road a corridor names that the network does
/// not have (a corridor left with none is dropped).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "NetworkWire")]
pub struct Network {
    /// Whether traffic keeps to the left.
    pub left_hand: bool,
    /// What stands at each node; the key stays `nodes` where a network is kept.
    #[serde(rename = "nodes")]
    pub junctions: Vec<Junction>,
    pub roads: Vec<Road>,
    /// The roads grouped by who they serve; see `Network::derive_corridors`. Kept only when there are some.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corridors: Vec<Corridor>,
}

#[derive(Deserialize)]
struct NetworkWire {
    left_hand: bool,
    nodes: Vec<Junction>,
    #[serde(deserialize_with = "crate::road::named_roads")]
    roads: Vec<Road>,
    #[serde(default)]
    corridors: Vec<Corridor>,
}

impl From<NetworkWire> for Network {
    fn from(w: NetworkWire) -> Network {
        let corridors = w
            .corridors
            .into_iter()
            .map(|mut c| {
                c.roads.retain(|id| w.roads.iter().any(|r| r.id == *id));
                c
            })
            .filter(|c| !c.roads.is_empty())
            .collect();
        let mut net = Network { left_hand: w.left_hand, junctions: w.nodes, roads: w.roads, corridors };
        net.drop_unknown_priority();
        net
    }
}

impl Network {
    /// Takes from every priority control the roads the network does not have, so it never names one that is gone.
    pub(crate) fn drop_unknown_priority(&mut self) {
        let roads = &self.roads;
        for j in &mut self.junctions {
            if let Control::Priority(ids) = &mut j.control {
                ids.retain(|id| roads.iter().any(|r| r.id == *id));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RoadId;

    #[test]
    fn reading_a_network_drops_priority_roads_it_does_not_have() {
        let json = r#"{"left_hand":false,"nodes":[{"id":1,"osm_nodes":[],"x_m":0.0,"y_m":0.0,"junction":true,"control":{"priority":[7,99]}},
            {"id":2,"osm_nodes":[],"x_m":9.0,"y_m":0.0,"junction":false,"control":"none"}],
            "roads":[{"id":7,"osm_ways":[],"name":"Main","highway":"x","from":1,"to":2,"lanes":[],"points":[]}]}"#;
        let net: Network = serde_json::from_str(json).unwrap();
        assert_eq!(net.junctions[0].control, Control::Priority(vec![RoadId(7)]));
    }
}
