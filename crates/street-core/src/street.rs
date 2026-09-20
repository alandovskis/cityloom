//! `Street`, `BoundingNodeIds`, `StreetIdentity` — BR3.1, BR4.1, BR5.1.

use crate::lane::{Lane, LaneList};
use crate::provenance::{Dimension, Provenance};

/// This project's canonical lane-type name for a kerb buffer — the
/// project-owned boundary a carriageway is measured between (BR4.1).
/// A `StreetSource` implementor (`u4-street-import`'s adapter) is
/// responsible for mapping osm2streets' own lane-type vocabulary onto
/// this and the constants below when it constructs `Lane`s; this crate
/// stays free of any osm2streets dependency (BR7.1) while still owning
/// the one definition of the kerb (`components.md` StreetModel).
pub const KERB_BUFFER_LANE_TYPE: &str = "kerb_buffer";

/// This project's canonical lane-type name for a verge buffer, excluded
/// from the carriageway width when no kerb buffers are present (BR4.1).
pub const VERGE_BUFFER_LANE_TYPE: &str = "verge";

/// This project's canonical lane-type names for walkable lanes, excluded
/// from the carriageway width when no kerb buffers are present (BR4.1).
pub const WALKABLE_LANE_TYPES: &[&str] = &["sidewalk", "footway"];

/// A direction-normalised pair of OSM node ids bounding a street segment
/// — what osm2streets' `split_ways` actually splits on, and stable OSM
/// identities (unlike a way id alone, which `split_ways` can produce
/// several `Street`s from). Normalised at construction so the pair
/// compares equal regardless of the order the two ids were supplied in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoundingNodeIds {
    lower: String,
    upper: String,
}

impl BoundingNodeIds {
    /// Builds a direction-normalised node-id pair from the two OSM node
    /// ids bounding a segment, in either order.
    pub fn new(node_a: impl Into<String>, node_b: impl Into<String>) -> BoundingNodeIds {
        let node_a = node_a.into();
        let node_b = node_b.into();
        if node_a <= node_b {
            BoundingNodeIds {
                lower: node_a,
                upper: node_b,
            }
        } else {
            BoundingNodeIds {
                lower: node_b,
                upper: node_a,
            }
        }
    }
}

/// A `Street`'s identity: its OSM way id plus the direction-normalised
/// bounding node-id pair of its segment (BR3.1) — never the way id
/// alone, since `split_ways` can produce multiple `Street`s sharing one
/// way id.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StreetIdentity {
    /// The OSM way id this street was split from.
    pub osm_way_id: String,
    /// The direction-normalised bounding node-id pair of this segment.
    pub bounding_node_ids: BoundingNodeIds,
}

/// One imported street: its identity, name, and ordered lane list. The
/// baseline is immutable once constructed (BR5.1) — every accessor takes
/// `&self`, and no method here returns a mutable reference into it.
///
/// This is a build-time invariant of this Unit's public API, not a
/// runtime check: `Street` declares no `&mut self` method and no public
/// constructor that accepts a mutable borrow of an already-constructed
/// instance, so `let mut street: Street = ...; street.some_mutator()`
/// fails to compile for any mutator, because none exists to call. A
/// `trybuild`-style compile-fail test would only re-assert this same
/// absence at the cost of a dev-dependency this crate's zero-dependency
/// budget (SD-2) does not carry for one invariant; this doc comment is
/// the chosen alternative (`code-generation-plan.md` Step 9.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Street {
    osm_way_id: String,
    bounding_node_ids: BoundingNodeIds,
    name: Option<String>,
    lanes: LaneList,
    carriageway_width: Dimension,
}

/// Computes carriageway width per BR4.1: the sum of lane widths strictly
/// between the two kerb-buffer lanes when both are present in the list;
/// otherwise the sum of all lane widths excluding walkable lane types and
/// verge buffers. This is the single, sole definition of carriageway
/// width in the system (`components.md` StreetModel) — no other
/// component computes or overrides it.
fn compute_carriageway_width(lanes: &LaneList) -> Dimension {
    let kerb_buffer_positions: Vec<usize> = lanes
        .iter()
        .enumerate()
        .filter(|(_, lane)| lane.lane_type() == KERB_BUFFER_LANE_TYPE)
        .map(|(index, _)| index)
        .collect();

    let contributing: Vec<Dimension> = if let (Some(&first), Some(&last)) =
        (kerb_buffer_positions.first(), kerb_buffer_positions.last())
        && first != last
    {
        lanes
            .iter()
            .enumerate()
            .filter(|(index, _)| *index > first && *index < last)
            .map(|(_, lane)| lane.width())
            .collect()
    } else {
        lanes
            .iter()
            .filter(|lane| {
                lane.lane_type() != VERGE_BUFFER_LANE_TYPE
                    && !WALKABLE_LANE_TYPES.contains(&lane.lane_type())
            })
            .map(|lane| lane.width())
            .collect()
    };

    let metres = contributing.iter().map(|dimension| dimension.metres).sum();

    // The computed total carries Mapped provenance only when every
    // contributing Dimension was itself Mapped; a wholly-inferred
    // carriageway width is representable, and never silently promoted
    // to Mapped, because every contributing Dimension carries its own
    // Provenance (BR1.1, AC6.2.3). Any contributor that is Inferred or
    // UserSet makes the computed total Inferred as well — it is a
    // derived value this Unit produces, not a value read directly from
    // a single OSM tag, so it is never presented as more certain than
    // its least-certain contributor.
    let provenance = if contributing
        .iter()
        .all(|dimension| dimension.provenance == Provenance::Mapped)
    {
        Provenance::Mapped
    } else {
        Provenance::Inferred
    };

    Dimension { metres, provenance }
}

impl Street {
    /// Constructs a `Street`, fixing its identity and computing its
    /// `carriageway_width` once from the given lane list (BR4.1). There
    /// is no `&mut self` method on this type (BR5.1) — construction is
    /// the only way to produce a `Street`.
    pub fn new(
        osm_way_id: impl Into<String>,
        bounding_node_ids: BoundingNodeIds,
        name: Option<String>,
        lanes: Vec<Lane>,
    ) -> Street {
        let lanes = LaneList::new(lanes);
        let carriageway_width = compute_carriageway_width(&lanes);
        Street {
            osm_way_id: osm_way_id.into(),
            bounding_node_ids,
            name,
            lanes,
            carriageway_width,
        }
    }

    /// This street's identity: its OSM way id plus its bounding node-id
    /// pair (BR3.1).
    pub fn identity(&self) -> StreetIdentity {
        StreetIdentity {
            osm_way_id: self.osm_way_id.clone(),
            bounding_node_ids: self.bounding_node_ids.clone(),
        }
    }

    /// This street's name, if osm2streets provided one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The lane at `index`, or `None` if `index` is out of range. Never
    /// panics (BR2.2).
    pub fn lane_at(&self, index: usize) -> Option<&Lane> {
        self.lanes.get(index)
    }

    /// The number of lanes in this street.
    pub fn lane_count(&self) -> usize {
        self.lanes.len()
    }

    /// This street's carriageway width, computed once at construction
    /// (BR4.1).
    pub fn carriageway_width(&self) -> Dimension {
        self.carriageway_width
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lane::Direction;

    fn mapped(metres: f64) -> Dimension {
        Dimension {
            metres,
            provenance: Provenance::Mapped,
        }
    }

    fn inferred(metres: f64) -> Dimension {
        Dimension {
            metres,
            provenance: Provenance::Inferred,
        }
    }

    fn bounding_node_ids() -> BoundingNodeIds {
        BoundingNodeIds::new("100", "200")
    }

    #[test]
    fn bounding_node_ids_are_direction_normalised() {
        let a_then_b = BoundingNodeIds::new("100", "200");
        let b_then_a = BoundingNodeIds::new("200", "100");

        assert_eq!(a_then_b, b_then_a);
    }

    #[test]
    fn two_streets_with_the_same_way_id_but_different_node_ids_are_distinct() {
        let first = Street::new(
            "way-1",
            BoundingNodeIds::new("100", "200"),
            None,
            vec![Lane::new("Travel", Direction::Forward, mapped(3.0), 0)],
        );
        let second = Street::new(
            "way-1",
            BoundingNodeIds::new("200", "300"),
            None,
            vec![Lane::new("Travel", Direction::Forward, mapped(3.0), 0)],
        );

        assert_ne!(first.identity(), second.identity());
    }

    #[test]
    fn a_street_built_from_the_same_way_and_node_ids_is_independently_keyed() {
        let street = Street::new(
            "way-1",
            bounding_node_ids(),
            Some("Main Street".to_string()),
            vec![Lane::new("Travel", Direction::Forward, mapped(3.0), 0)],
        );

        assert_eq!(
            street.identity(),
            StreetIdentity {
                osm_way_id: "way-1".to_string(),
                bounding_node_ids: bounding_node_ids(),
            }
        );
    }

    #[test]
    fn carriageway_width_sums_between_two_kerb_buffers_when_both_present() {
        let lanes = vec![
            Lane::new(KERB_BUFFER_LANE_TYPE, Direction::Forward, mapped(0.3), 0),
            Lane::new("Travel", Direction::Forward, mapped(3.0), 1),
            Lane::new("Travel", Direction::Backward, mapped(3.0), 2),
            Lane::new(KERB_BUFFER_LANE_TYPE, Direction::Backward, mapped(0.3), 3),
        ];

        let street = Street::new("way-1", bounding_node_ids(), None, lanes);

        assert_eq!(street.carriageway_width().metres, 6.0);
    }

    #[test]
    fn carriageway_width_excludes_walkable_and_verge_lanes_without_kerb_buffers() {
        let lanes = vec![
            Lane::new(VERGE_BUFFER_LANE_TYPE, Direction::Forward, mapped(1.0), 0),
            Lane::new(WALKABLE_LANE_TYPES[0], Direction::Forward, mapped(2.0), 1),
            Lane::new("Travel", Direction::Forward, mapped(3.0), 2),
            Lane::new("Travel", Direction::Backward, mapped(3.0), 3),
        ];

        let street = Street::new("way-1", bounding_node_ids(), None, lanes);

        assert_eq!(street.carriageway_width().metres, 6.0);
    }

    #[test]
    fn carriageway_width_is_computable_when_every_contributing_width_is_inferred() {
        let lanes = vec![
            Lane::new("Travel", Direction::Forward, inferred(3.2), 0),
            Lane::new("Travel", Direction::Backward, inferred(3.2), 1),
        ];

        let street = Street::new("way-1", bounding_node_ids(), None, lanes);

        let carriageway_width = street.carriageway_width();
        assert_eq!(carriageway_width.metres, 6.4);
        assert_eq!(carriageway_width.provenance, Provenance::Inferred);
    }

    #[test]
    fn carriageway_width_is_mapped_when_every_contributing_dimension_is_mapped() {
        let lanes = vec![
            Lane::new("Travel", Direction::Forward, mapped(3.0), 0),
            Lane::new("Travel", Direction::Backward, mapped(3.0), 1),
        ];

        let street = Street::new("way-1", bounding_node_ids(), None, lanes);

        assert_eq!(street.carriageway_width().provenance, Provenance::Mapped);
    }

    #[test]
    fn carriageway_width_is_inferred_when_any_contributing_dimension_is_inferred() {
        let lanes = vec![
            Lane::new("Travel", Direction::Forward, mapped(3.0), 0),
            Lane::new("Travel", Direction::Backward, inferred(3.0), 1),
        ];

        let street = Street::new("way-1", bounding_node_ids(), None, lanes);

        assert_eq!(street.carriageway_width().provenance, Provenance::Inferred);
    }

    #[test]
    fn street_lane_at_is_bounds_checked() {
        let street = Street::new(
            "way-1",
            bounding_node_ids(),
            None,
            vec![Lane::new("Travel", Direction::Forward, mapped(3.0), 0)],
        );

        assert!(street.lane_at(0).is_some());
        assert!(street.lane_at(1).is_none());
    }
}
