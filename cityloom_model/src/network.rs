use serde::{Deserialize, Serialize};

use crate::{Control, Corridor, Junction, Road};

/// The street network.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Network {
    /// Whether traffic keeps to the left.
    pub left_hand: bool,
    /// What stands at each node of the graph.
    pub junctions: Vec<Junction>,
    pub roads: Vec<Road>,
    /// The roads grouped by who they serve; see `Network::derive_corridors`. Kept only when there are some.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub corridors: Vec<Corridor>,
}

impl Network {
    /// Takes from every priority control the roads the network does not have, so it never names one that is gone.
    pub(crate) fn drop_unknown_priority(&mut self) {
        let roads = &self.roads;
        for j in &mut self.junctions {
            if let Control::Priority(ids) = &mut j.control {
                ids.retain(|id| roads.iter().any(|r| r.edge.id == *id));
            }
        }
    }
}
