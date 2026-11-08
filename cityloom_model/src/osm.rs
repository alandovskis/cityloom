use serde::{Deserialize, Serialize};

/// An OpenStreetMap way, by its id, with the version OpenStreetMap had of it when the data was read, when that is known.
/// Two refs are equal only if their versions are too: to ask whether a road was made from a way, compare `id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WayRef {
    pub id: i64,
    pub version: Option<i32>,
}

impl WayRef {
    /// A way whose version is not known.
    pub fn new(id: i64) -> WayRef {
        WayRef { id, version: None }
    }

    pub fn with_version(id: i64, version: i32) -> WayRef {
        WayRef { id, version: Some(version) }
    }
}

/// An OpenStreetMap node, by its id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeRef(pub i64);

/// A thing in OpenStreetMap that a road or a node was made from: a way or a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OsmRef {
    Way(WayRef),
    Node(NodeRef),
}

impl OsmRef {
    /// The OSM id, which is only unique among the ways or among the nodes.
    pub fn id(self) -> i64 {
        match self {
            OsmRef::Way(w) => w.id,
            OsmRef::Node(n) => n.0,
        }
    }
}

impl From<WayRef> for OsmRef {
    fn from(w: WayRef) -> Self {
        OsmRef::Way(w)
    }
}

impl From<NodeRef> for OsmRef {
    fn from(n: NodeRef) -> Self {
        OsmRef::Node(n)
    }
}

impl From<WayRef> for i64 {
    fn from(w: WayRef) -> Self {
        w.id
    }
}

impl From<NodeRef> for i64 {
    fn from(n: NodeRef) -> Self {
        n.0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn a_node_ref_is_kept_as_the_bare_id_it_replaces() {
        assert_eq!(serde_json::to_string(&vec![NodeRef(5), NodeRef(7)]).unwrap(), "[5,7]");
        let versions: BTreeMap<NodeRef, i32> = [(NodeRef(3), 9)].into();
        let json = serde_json::to_string(&versions).unwrap();
        assert_eq!(json, r#"{"3":9}"#);
        assert_eq!(serde_json::from_str::<BTreeMap<NodeRef, i32>>(&json).unwrap(), versions);
    }

    #[test]
    fn a_way_and_a_node_of_one_id_are_not_the_same_thing() {
        assert_ne!(OsmRef::from(WayRef::new(1)), OsmRef::from(NodeRef(1)));
        assert_eq!(OsmRef::from(NodeRef(1)).id(), 1);
    }
}
