//! The plain street network that `osm_import` produces and the editor reads.
//! It stands between them so that the editor does not depend on osm2streets.
//!
//! Positions are metres from the south-west corner of the data: `x` east, `y` north.

mod lane;
pub mod merge;
mod network;
mod node;
mod osm;
mod point;
mod road;
mod road_type;

pub use lane::{BufferType, Lane, LaneKind, Way};
pub use network::Network;
pub use node::{Control, Node};
pub use osm::{NodeRef, OsmRef, WayRef};
pub use point::{Coordinate, Point};
pub use road::Road;
pub use road_type::RoadType;
