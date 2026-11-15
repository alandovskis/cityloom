//! Traffic calming: raised tables and raised crossings along a street, chicanes and modal filters.
//!
//! They belong to the street, not to a section: a position is measured along the whole street from its anchored end
//! (`Street::from_end`), and a table spans the whole width of the roadway, so it has no side; a chicane's build-outs
//! alternate from side to side, starting on the side it says.

use serde::{Deserialize, Serialize};

use crate::catalogue::Side;

/// What a raised element is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A raised table: a speed table across the roadway.
    Table,
    /// A raised pedestrian crossing: the plateau is the crossing.
    Crossing,
    /// A chicane: build-outs from alternate curbs, which make the road bend. Appended last so the values a choice
    /// saved for the others keep their place.
    Chicane,
    /// A modal filter: a barrier or planter across the roadway that stops motor traffic and leaves a way through for
    /// people on foot and on bikes. Appended last, as the chicane was.
    ModalFilter,
}

impl Kind {
    /// Every kind, in the order a choice lists them.
    pub const ALL: [Kind; 4] = [Kind::Table, Kind::Crossing, Kind::Chicane, Kind::ModalFilter];

    /// The kind's stable name, as saved and as the choice's value.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Table => "table",
            Kind::Crossing => "crossing",
            Kind::Chicane => "chicane",
            Kind::ModalFilter => "modal_filter",
        }
    }
}

/// One raised element, along `length_mm` of the street from `from_mm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Calming {
    /// Millimetres from the anchored end (`Street::from_end`) to where it starts, a multiple of the editor's snap (100 mm).
    pub from_mm: i32,
    pub length_mm: i32,
    pub kind: Kind,
    /// For a chicane, the side its first build-out is on (the section's side, Left being the first piece); `None`
    /// means the right. Saved only when set, and only a chicane has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<Side>,
}

impl Calming {
    /// Where it ends, in millimetres from the anchored end.
    pub fn to_mm(&self) -> i32 {
        self.from_mm + self.length_mm
    }

    /// The side a chicane's first build-out is on.
    pub fn first_side(&self) -> Side {
        self.first.unwrap_or(Side::Right)
    }
}

impl super::Street {
    /// Whether the elements can be placed: they need the length they were drawn against, and lie in front of it.
    pub(super) fn calming_is_sound(&self) -> bool {
        self.calming.is_empty() || (self.length_mm.is_some() && self.calming.iter().all(|c| c.from_mm >= 0 && c.length_mm > 0))
    }
}
