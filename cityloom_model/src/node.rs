use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::NodeRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    None,
    Signs,
    Signals,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: u32,
    /// The OSM nodes it stands for; the key an edit is kept under.
    pub osm_nodes: Vec<NodeRef>,
    /// The version OpenStreetMap had of each of them when the data was read.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub osm_versions: BTreeMap<NodeRef, i32>,
    pub x_m: f64,
    pub y_m: f64,
    /// Where three or more roads meet and traffic gives way, as opposed to a street running off the data or ending.
    pub junction: bool,
    pub control: Control,
}
