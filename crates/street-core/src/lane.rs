//! `Lane`, `LaneKey`, `LaneList` — BR2.1, BR2.2.

use crate::provenance::Dimension;

/// The direction a lane's traffic (or its intended use) runs relative to
/// the street's own digitised direction. Naming authority is osm2streets'
/// schema (`team.md` Code Style); this crate does not invent lane-type or
/// direction vocabulary beyond what a `StreetSource` implementor hands it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Runs the same way as the street's digitised direction.
    Forward,
    /// Runs against the street's digitised direction.
    Backward,
}

/// A `Lane`'s derived, stable identity: `(laneType, direction,
/// ordinalFromKerb)`. Never a positional index (BR2.1) — the same real
/// lane, re-imported independently, resolves to the same `LaneKey`, so a
/// correction keyed on it survives a re-import.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LaneKey {
    lane_type: String,
    direction: Direction,
    ordinal_from_kerb: u32,
}

/// One lane of a `Street`, in the order it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    key: LaneKey,
    lane_type: String,
    direction: Direction,
    width: Dimension,
    ordinal_from_kerb: u32,
}

impl Lane {
    /// Constructs a `Lane`, deriving its [`LaneKey`] from `lane_type`,
    /// `direction` and `ordinal_from_kerb` (BR2.1) — never from a
    /// position in a list.
    pub fn new(
        lane_type: impl Into<String>,
        direction: Direction,
        width: Dimension,
        ordinal_from_kerb: u32,
    ) -> Lane {
        let lane_type = lane_type.into();
        let key = LaneKey {
            lane_type: lane_type.clone(),
            direction,
            ordinal_from_kerb,
        };
        Lane {
            key,
            lane_type,
            direction,
            width,
            ordinal_from_kerb,
        }
    }

    /// This lane's derived, stable identity.
    pub fn key(&self) -> &LaneKey {
        &self.key
    }

    /// osm2streets' own name for this lane's type.
    pub fn lane_type(&self) -> &str {
        &self.lane_type
    }

    /// This lane's direction relative to the street's digitised direction.
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// This lane's width, carrying its own provenance.
    pub fn width(&self) -> Dimension {
        self.width
    }

    /// This lane's position counting outward from the kerb, starting at 0.
    pub fn ordinal_from_kerb(&self) -> u32 {
        self.ordinal_from_kerb
    }
}

/// An ordered list of a `Street`'s lanes. Positionally addressed for
/// display purposes only — access is bounds-checked (BR2.2): a
/// [`LaneList::get`] call never panics, returning `None` for an
/// out-of-range index rather than indexing directly (`lanes[i]`).
#[derive(Clone, Debug, PartialEq)]
pub struct LaneList(Vec<Lane>);

impl LaneList {
    /// Builds a `LaneList` from lanes already in display order.
    pub fn new(lanes: Vec<Lane>) -> LaneList {
        LaneList(lanes)
    }

    /// Returns the lane at `index`, or `None` if `index` is out of range.
    /// Never panics (BR2.2).
    pub fn get(&self, index: usize) -> Option<&Lane> {
        self.0.get(index)
    }

    /// The number of lanes in this list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this list holds no lanes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterates the lanes in display order.
    pub fn iter(&self) -> std::slice::Iter<'_, Lane> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Provenance;

    fn width(metres: f64) -> Dimension {
        Dimension {
            metres,
            provenance: Provenance::Mapped,
        }
    }

    #[test]
    fn identical_lane_fields_derive_the_same_key() {
        let a = Lane::new("Travel", Direction::Forward, width(3.0), 1);
        let b = Lane::new("Travel", Direction::Forward, width(3.5), 1);

        assert_eq!(a.key(), b.key());
    }

    #[test]
    fn a_different_lane_type_derives_a_different_key() {
        let travel = Lane::new("Travel", Direction::Forward, width(3.0), 1);
        let parking = Lane::new("Parking", Direction::Forward, width(3.0), 1);

        assert_ne!(travel.key(), parking.key());
    }

    #[test]
    fn a_different_direction_derives_a_different_key() {
        let forward = Lane::new("Travel", Direction::Forward, width(3.0), 1);
        let backward = Lane::new("Travel", Direction::Backward, width(3.0), 1);

        assert_ne!(forward.key(), backward.key());
    }

    #[test]
    fn a_different_ordinal_from_kerb_derives_a_different_key() {
        let first = Lane::new("Travel", Direction::Forward, width(3.0), 1);
        let second = Lane::new("Travel", Direction::Forward, width(3.0), 2);

        assert_ne!(first.key(), second.key());
    }

    #[test]
    fn lane_list_get_returns_some_for_a_valid_index() {
        let lanes = LaneList::new(vec![Lane::new("Travel", Direction::Forward, width(3.0), 0)]);

        assert!(lanes.get(0).is_some());
    }

    #[test]
    fn lane_list_get_returns_none_for_an_out_of_range_index() {
        let lanes = LaneList::new(vec![Lane::new("Travel", Direction::Forward, width(3.0), 0)]);

        assert!(lanes.get(1).is_none());
        assert!(lanes.get(usize::MAX).is_none());
    }

    #[test]
    fn lane_list_len_and_is_empty() {
        let empty = LaneList::new(vec![]);
        let one = LaneList::new(vec![Lane::new("Travel", Direction::Forward, width(3.0), 0)]);

        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(one.len(), 1);
        assert!(!one.is_empty());
    }
}
