use serde::{Deserialize, Serialize};

use crate::{Corridor, Junction, Road};

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
        Network { left_hand: w.left_hand, junctions: w.nodes, roads: w.roads, corridors }
    }
}
