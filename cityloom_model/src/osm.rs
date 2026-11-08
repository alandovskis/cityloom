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

/// An OpenStreetMap node, by its id, with the version OpenStreetMap had of it when the data was read, when that is known.
/// Two refs are equal only if their versions are too: to ask whether a node was made from an OSM node, compare `id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeRef {
    pub id: i64,
    pub version: Option<i32>,
}

impl NodeRef {
    /// A node whose version is not known.
    pub fn new(id: i64) -> NodeRef {
        NodeRef { id, version: None }
    }

    pub fn with_version(id: i64, version: i32) -> NodeRef {
        NodeRef { id, version: Some(version) }
    }
}

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
            OsmRef::Node(n) => n.id,
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
        n.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_way_and_a_node_of_one_id_are_not_the_same_thing() {
        assert_ne!(OsmRef::from(WayRef::new(1)), OsmRef::from(NodeRef::new(1)));
        assert_eq!(OsmRef::from(NodeRef::new(1)).id(), 1);
    }
}
