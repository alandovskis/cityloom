//! A street as the city keeps it and the street editor edits it: the pieces of its cross-section, its zones along its
//! length, its speed limit and what stands along it (curbside uses, transit stops, raised elements, trees and green
//! infrastructure). This is what OpenStreetMap can say of a street; the rules for editing it, the checks and the
//! wording are the editor's. All lengths are integer millimetres.

use serde::{Deserialize, Serialize};

/// An OpenStreetMap way (of a street) or node (of a junction) that a place was made from, and the version it had when
/// the data was read. Unlike the network's `crate::OsmRef`, it does not say which of the two it is: the place does.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OsmRef {
    pub id: i64,
    pub version: Option<i32>,
}
