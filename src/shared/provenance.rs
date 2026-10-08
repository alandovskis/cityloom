//! Where a street or a junction came from.

use serde::{Deserialize, Serialize};

/// An OpenStreetMap way (of a street) or node (of a junction) that a place was made from, and the version it had when
/// the data was read.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OsmRef {
    pub id: i64,
    pub version: Option<i32>,
}
