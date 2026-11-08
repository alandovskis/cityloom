use serde::{Deserialize, Serialize};

use crate::{Coordinate, Lane, Street};

/// A stretch of a road with one cross-section. A road is divided lengthways into zones, in order from its first
/// end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    /// Where the zone starts along the road from its `from` end; 0 for the first zone.
    pub from: Coordinate<Street>,
    /// OSM's `maxspeed` for the stretch in km/h; none when it gives none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_kmh: Option<u16>,
    /// Left to right when facing from the road's `from` end to its `to` end.
    pub lanes: Vec<Lane>,
}

impl Zone {
    /// A zone that is the whole road, which is what a road made from one OSM way is.
    pub fn whole(lanes: Vec<Lane>) -> Zone {
        Zone { from: Coordinate::new(0.0), speed_kmh: None, lanes }
    }
}
