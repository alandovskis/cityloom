use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{NodeId, NodeRef, Point, Provenance};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    None,
    Signs,
    Signals,
}

/// A junction or street end of the network.
///
/// Kept as a node always was: the OSM nodes as bare ids in `osm_nodes` and their versions in `osm_versions`, only for
/// the nodes whose version is known, so a network kept by an earlier build still reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "JunctionWire", into = "JunctionWire")]
pub struct Junction {
    pub id: NodeId,
    /// The OSM nodes it stands for, each with its version when known; the key an edit is kept under.
    pub provenance: Provenance,
    /// Where it lies, in metres from the south-west corner of the data.
    pub position: Point,
    /// Where three or more roads meet and traffic gives way, as opposed to a street running off the data or ending.
    pub junction: bool,
    pub control: Control,
}

#[derive(Serialize, Deserialize)]
struct JunctionWire {
    id: NodeId,
    osm_nodes: Vec<i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    osm_versions: BTreeMap<i64, i32>,
    x_m: f64,
    y_m: f64,
    junction: bool,
    control: Control,
}

impl From<JunctionWire> for Junction {
    fn from(w: JunctionWire) -> Junction {
        let nodes = w.osm_nodes.iter().map(|&id| NodeRef { id, version: w.osm_versions.get(&id).copied() });
        Junction { id: w.id, provenance: Provenance::of_nodes(nodes), position: Point::new(w.x_m, w.y_m), junction: w.junction, control: w.control }
    }
}

impl From<Junction> for JunctionWire {
    fn from(n: Junction) -> JunctionWire {
        JunctionWire {
            id: n.id,
            osm_nodes: n.provenance.nodes().map(|r| r.id).collect(),
            osm_versions: n.provenance.nodes().filter_map(|r| Some((r.id, r.version?))).collect(),
            x_m: n.position.x_m(),
            y_m: n.position.y_m(),
            junction: n.junction,
            control: n.control,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(provenance: Provenance) -> Junction {
        Junction { id: NodeId(1), provenance, position: Point::new(1.0, 2.0), junction: false, control: Control::None }
    }

    #[test]
    fn a_node_is_kept_with_its_osm_nodes_as_bare_ids_and_their_versions_apart() {
        let n = node(Provenance::of_nodes([NodeRef::with_version(3, 9), NodeRef::new(5)]));
        let json: serde_json::Value = serde_json::to_value(&n).unwrap();
        assert_eq!(json["osm_nodes"], serde_json::json!([3, 5]));
        assert_eq!(json["osm_versions"], serde_json::json!({"3": 9}));
        assert_eq!(serde_json::from_value::<Junction>(json).unwrap(), n);
    }

    #[test]
    fn a_node_kept_before_versions_still_reads() {
        let n: Junction = serde_json::from_str(r#"{"id":1,"osm_nodes":[4],"x_m":0.0,"y_m":0.0,"junction":true,"control":"signs"}"#).unwrap();
        assert_eq!(n.provenance, Provenance::of_nodes([NodeRef::new(4)]));
        assert!(serde_json::to_value(&n).unwrap().get("osm_versions").is_none());
    }
}
