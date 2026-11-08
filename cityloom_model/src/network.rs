use serde::{Deserialize, Serialize};

use crate::{Node, Road};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Network {
    /// Whether traffic keeps to the left.
    pub left_hand: bool,
    pub nodes: Vec<Node>,
    /// A road without a name is left out when a network is read.
    #[serde(deserialize_with = "crate::road::named_roads")]
    pub roads: Vec<Road>,
}
