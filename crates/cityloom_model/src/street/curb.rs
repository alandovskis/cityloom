//! Curbside programmes: the parking or loading lane at a street's edge, divided along the street into an ordered
//! list of uses (a loading bay, a tree, a few parking spaces, a patio), each with an extent.
//!
//! They belong to the street, not to a section: a position is measured along the whole street from its anchored end
//! (`Street::from_end`), as a zone boundary is, so splitting, merging or moving zones leaves them where they are.

use serde::{Deserialize, Serialize};

use super::Street;
use crate::catalogue::Side;

/// What a stretch of curbside is used for. Each is drawn as an existing catalogue kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Use {
    Parking,
    Loading,
    Tree,
    Planting,
    RainGarden,
    Patio,
    BikeParking,
    Platform,
}

impl Use {
    /// Every use, in the order a choice lists them.
    pub const ALL: [Use; 8] = [Use::Parking, Use::Loading, Use::Tree, Use::Planting, Use::RainGarden, Use::Patio, Use::BikeParking, Use::Platform];

    /// The catalogue kind it is drawn as.
    pub fn kind_id(self) -> &'static str {
        match self {
            Use::Parking => "parking",
            Use::Loading => "loading",
            Use::Tree | Use::Planting | Use::RainGarden => "planting",
            Use::Patio => "terrace",
            Use::BikeParking => "bikerack",
            Use::Platform => "sidewalk",
        }
    }

    /// The catalogue material it is drawn in, where it is not the kind's own.
    pub fn material(self) -> Option<&'static str> {
        match self {
            Use::Tree => Some("trees"),
            Use::RainGarden => Some("rain_garden"),
            _ => None,
        }
    }

    /// Whether it stands at sidewalk height (a curb extension) rather than lying on the roadway.
    pub fn raised(self) -> bool {
        !matches!(self, Use::Parking | Use::Loading)
    }
}

/// One use of the curbside, along `length_mm` of the street from `from_mm`.
///
/// `side` is the side of the section, not the region's driving side: `Left` is the first piece of the section's
/// list (the street's left looking from its first end), `Right` the last. It does not change with the end the
/// positions are measured from; `Street::reversed` swaps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Instance {
    pub side: Side,
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of the editor's snap (100 mm).
    pub from_mm: i32,
    pub length_mm: i32,
    #[serde(rename = "use")]
    pub use_: Use,
}

impl Instance {
    /// Where it ends, in millimetres from the anchored end.
    pub fn to_mm(&self) -> i32 {
        self.from_mm + self.length_mm
    }
}

impl Street {
    /// Swaps the sides of the instances for a street seen from its other end. Their positions stay measured from
    /// the same physical end (`reverse_zones` flips the label of that end).
    pub(super) fn reverse_curb(&mut self) {
        for i in &mut self.curb {
            i.side = match i.side {
                Side::Left => Side::Right,
                Side::Right => Side::Left,
            };
        }
        self.curb.sort_by_key(|i| (i.side == Side::Right, i.from_mm));
    }

    /// Whether the instances can be placed: they need the length they were drawn against, and lie in front of it.
    pub(super) fn curb_is_sound(&self) -> bool {
        self.curb.is_empty() || (self.length_mm.is_some() && self.curb.iter().all(|i| i.from_mm >= 0 && i.length_mm > 0))
    }
}
