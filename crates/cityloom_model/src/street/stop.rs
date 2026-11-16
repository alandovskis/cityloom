//! Transit stops: a stretch of a street's edge where buses stop, drawn as a curb stop, a bulb or a boarding island.
//!
//! Like the curbside uses (`curb.rs`) they belong to the street, not to a section: a position is measured along the
//! whole street from its anchored end (`Street::from_end`).

use serde::{Deserialize, Serialize};

use super::Street;
use crate::catalogue::Side;

/// How a stop is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopKind {
    /// The bus stops in its lane against the curb.
    Curb,
    /// The sidewalk reaches out over the parking lane to the bus lane.
    Bulb,
    /// The bus stops beside an island with a bike lane behind it.
    Island,
}

impl StopKind {
    /// Every kind, in the order a choice lists them.
    pub const ALL: [StopKind; 3] = [StopKind::Curb, StopKind::Bulb, StopKind::Island];
}

/// One transit stop, along `length_mm` of the street from `from_mm`.
///
/// `side` is the side of the section, as for a curbside `Instance`: `Left` is the first piece of the section's list,
/// `Right` the last, whatever the end the positions are measured from; `Street::reversed` swaps it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stop {
    pub side: Side,
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of the editor's snap (100 mm).
    pub from_mm: i32,
    pub length_mm: i32,
    pub kind: StopKind,
    /// Whether it has a shelter.
    pub shelter: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Stop {
    /// Where it ends, in millimetres from the anchored end.
    pub fn to_mm(&self) -> i32 {
        self.from_mm + self.length_mm
    }
}

impl Street {
    /// Swaps the sides of the stops for a street seen from its other end. Their positions stay measured from the
    /// same physical end (`reverse_zones` flips the label of that end).
    pub(super) fn reverse_stops(&mut self) {
        for s in &mut self.stops {
            s.side = match s.side {
                Side::Left => Side::Right,
                Side::Right => Side::Left,
            };
        }
        self.stops.sort_by_key(|s| (s.side == Side::Right, s.from_mm));
    }

    /// Whether the stops can be placed: they need the length they were drawn against, and lie in front of it.
    pub(super) fn stops_are_sound(&self) -> bool {
        self.stops.is_empty() || (self.length_mm.is_some() && self.stops.iter().all(|s| s.from_mm >= 0 && s.length_mm > 0))
    }
}
