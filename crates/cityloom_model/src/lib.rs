//! The plain street network that the importer of `citymodel_builder` (`citymodel_builder::importer`) produces and
//! the editor reads.
//! It stands between them so that the editor does not depend on osm2streets.
//!
//! It also holds the street as the city keeps it (`street`) and the catalogue its pieces name (`catalogue`): what
//! OpenStreetMap can say of a street, without the editor's rules for changing it.
//!
//! Positions in the network are metres from the south-west corner of the data: `x` east, `y` north.

pub mod catalogue;
mod corridor;
mod cut;
mod frame;
mod graph;
mod hours;
mod importer;
mod junction;
mod lane;
mod network;
mod osm;
mod point;
mod provenance;
mod road;
mod road_type;
pub mod street;
mod zone;

pub use corridor::{Corridor, UserClass};
pub use cut::NetworkTile;
pub use frame::Frame;
pub use graph::{Edge, Node, NodeId, connected};
pub use hours::{Hours, Window};
pub use importer::{IMPORTER_VERSION, importer_of, with_importer};
pub use junction::{Control, Junction};
pub use lane::{BufferType, Direction, Lane, LaneKind};
pub use network::Network;
pub use osm::{NodeRef, OsmRef, SourceRef, WayRef};
pub use point::{Coordinate, Local, Point, Street};
pub use provenance::Provenance;
pub use road::{Road, RoadId};
pub use road_type::RoadType;
pub use zone::Zone;
