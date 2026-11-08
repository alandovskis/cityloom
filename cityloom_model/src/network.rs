use serde::{Deserialize, Serialize};

use crate::{Corridor, Junction, Road};

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
