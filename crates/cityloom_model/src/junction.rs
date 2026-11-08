use serde::{Deserialize, Serialize};

use crate::{NodeId, Point, Provenance, RoadId};

/// How traffic is run where roads meet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    /// Nothing is recorded: no signs, no signals.
    None,
    Signs,
    Signals,
    /// The roads that have priority; traffic on the others gives way.
    Priority(Vec<RoadId>),
}

/// A junction or street end of the network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeRef;

    fn node(provenance: Provenance) -> Junction {
        Junction { id: NodeId(1), provenance, position: Point::new(1.0, 2.0), junction: false, control: Control::None }
    }

    #[test]
    fn a_junction_is_kept_with_its_provenance_and_position() {
        let n = node(Provenance::of_nodes([NodeRef::with_version(3, 9), NodeRef::new(5)]));
        let json = serde_json::to_value(&n).unwrap();
        assert_eq!(json["provenance"], serde_json::json!([{"node": {"id": 3, "version": 9}}, {"node": {"id": 5}}]));
        assert_eq!(json["position"], serde_json::json!([1.0, 2.0]));
        assert_eq!(serde_json::from_value::<Junction>(json).unwrap(), n);
    }

    #[test]
    fn a_priority_control_names_the_roads_that_have_priority_and_the_old_controls_still_read() {
        assert_eq!(serde_json::to_string(&Control::Priority(vec![RoadId(2), RoadId(5)])).unwrap(), r#"{"priority":[2,5]}"#);
        assert_eq!(serde_json::from_str::<Control>(r#"{"priority":[2,5]}"#).unwrap(), Control::Priority(vec![RoadId(2), RoadId(5)]));
        assert_eq!(serde_json::from_str::<Control>(r#""signs""#).unwrap(), Control::Signs);
        assert_eq!(serde_json::from_str::<Control>(r#""none""#).unwrap(), Control::None);
    }
}
