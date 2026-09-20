//! `StreetImportAdapter` — the only component permitted to depend on
//! `osm2streets` directly; converts its native output into `street-core`'s
//! provenance-carrying types (`components.md` `StreetImportAdapter`; BR1.1,
//! BR3.1, BR4.1, BR5.1, BR6.1, BR7.1, SD-1, SD-3).

use std::any::Any;
use std::collections::HashMap;

use street_core::{
    BoundingNodeIds, Dimension, Direction, IntersectionId, KERB_BUFFER_LANE_TYPE, Lane, Provenance,
    Street, StreetIdentity, StreetNetworkGraph,
};

use crate::failure::ImportFailure;
use crate::log::log_locally;

/// The imported baseline plus this Unit's correction-reconciliation
/// bookkeeping (`entities.md` `ImportedStreet`).
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedStreet {
    pub street: Street,
    /// Present for corridor/intersection imports; `None` for a single
    /// isolated street.
    pub network_graph: Option<StreetNetworkGraph>,
    /// One outcome per stored `Correction` whose `target_street` matches —
    /// empty on a first-ever import.
    pub correction_reconciliation: Vec<crate::correction::CorrectionReconciliationOutcome>,
}

/// Converts raw OSM extract bytes into this project's own street/lane
/// model via the pinned `osm2streets` crate (SD-3), wrapping every call
/// into it in `catch_unwind` (SD-1) so a panic inside the dependency never
/// escapes this boundary.
#[derive(Clone, Copy, Debug)]
pub struct StreetImportAdapter {
    inferred_sidewalks: bool,
    inferred_kerbs: bool,
}

impl Default for StreetImportAdapter {
    fn default() -> StreetImportAdapter {
        StreetImportAdapter {
            inferred_sidewalks: false,
            inferred_kerbs: true,
        }
    }
}

impl StreetImportAdapter {
    pub fn new() -> StreetImportAdapter {
        StreetImportAdapter::default()
    }

    /// Converts `extract_bytes` into a [`StreetNetworkGraph`] (BR1.1,
    /// BR4.1). `street_name` is the name shown at selection (BR3.1),
    /// carried into any [`ImportFailure`] this method returns.
    ///
    /// Every call into `osm2streets`/`streets_reader` happens inside
    /// `std::panic::catch_unwind` (SD-1): a panic there — verified during
    /// this Unit's build to be a real, reachable outcome for some
    /// malformed extracts, not a theoretical one — is caught, logged
    /// locally with `street_name` (SD-2), and classified as
    /// `malformed_extract`, never allowed to unwind past this function or
    /// to carry its message into the returned value.
    pub fn import_network(
        &self,
        street_name: &str,
        extract_bytes: &[u8],
    ) -> Result<StreetNetworkGraph, ImportFailure> {
        let inferred_sidewalks = self.inferred_sidewalks;
        let inferred_kerbs = self.inferred_kerbs;

        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            convert_bytes(extract_bytes, inferred_sidewalks, inferred_kerbs)
        }));

        match outcome {
            Ok(Ok(graph)) => Ok(graph),
            Ok(Err(detail)) => {
                log_locally(street_name, &detail);
                Err(ImportFailure::malformed_extract(street_name))
            }
            Err(panic_payload) => {
                log_locally(street_name, &panic_message(&panic_payload));
                Err(ImportFailure::malformed_extract(street_name))
            }
        }
    }
}

fn panic_message(payload: &Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "osm2streets panicked with a non-string payload".to_string()
    }
}

/// The one place this crate calls into `osm2streets`/`streets_reader`
/// directly (SD-3), and the one place it converts their native output into
/// `street-core` types (BR1.1). Returns `Err(detail)` — a plain `String`,
/// never an osm2streets-native error type — for `streets_reader`'s own
/// graceful parse failures; a panic is handled by the caller's
/// `catch_unwind`, not here.
fn convert_bytes(
    extract_bytes: &[u8],
    inferred_sidewalks: bool,
    inferred_kerbs: bool,
) -> Result<StreetNetworkGraph, String> {
    let mut timer = abstutil::Timer::new("cityloom-street-import");
    let mut cfg = osm2streets::MapConfig::default();
    cfg.inferred_sidewalks = inferred_sidewalks;
    cfg.inferred_kerbs = inferred_kerbs;

    let (streets, _document) =
        streets_reader::osm_to_street_network(extract_bytes, None, cfg, &mut timer)
            .map_err(|error| error.to_string())?;

    Ok(build_graph(&streets))
}

fn build_graph(streets: &osm2streets::StreetNetwork) -> StreetNetworkGraph {
    let mut identities = HashMap::new();
    let mut converted_streets = Vec::with_capacity(streets.roads.len());
    for (road_id, road) in &streets.roads {
        let street = convert_road(streets, road);
        identities.insert(*road_id, street.identity());
        converted_streets.push(street);
    }

    let mut intersection_ids = Vec::with_capacity(streets.intersections.len());
    let mut adjacency = HashMap::with_capacity(streets.intersections.len());
    for (intersection_id, intersection) in &streets.intersections {
        let id = IntersectionId::new(intersection_key(*intersection_id, intersection));
        let connected = intersection
            .roads
            .iter()
            .filter_map(|road_id| identities.get(road_id).cloned())
            .collect::<Vec<StreetIdentity>>();
        intersection_ids.push(id.clone());
        adjacency.insert(id, connected);
    }

    StreetNetworkGraph::new("import", converted_streets, intersection_ids, adjacency)
}

/// A stable-enough key for an intersection: its own OSM node ids, joined,
/// or (only if osm2streets produced an intersection with none — not
/// observed against any of this Unit's fixtures, but not assumed
/// impossible) a fallback naming its internal, non-OSM `IntersectionID` so
/// this never panics.
fn intersection_key(
    intersection_id: osm2streets::IntersectionID,
    intersection: &osm2streets::Intersection,
) -> String {
    if intersection.osm_ids.is_empty() {
        format!("synthetic-intersection-{intersection_id:?}")
    } else {
        intersection
            .osm_ids
            .iter()
            .map(|node_id| node_id.0.to_string())
            .collect::<Vec<_>>()
            .join("+")
    }
}

fn convert_road(streets: &osm2streets::StreetNetwork, road: &osm2streets::Road) -> Street {
    let osm_way_id = road
        .osm_ids
        .iter()
        .map(|way_id| way_id.0.to_string())
        .collect::<Vec<_>>()
        .join("+");
    let bounding_node_ids = BoundingNodeIds::new(
        first_osm_node_id(streets, road.src_i),
        first_osm_node_id(streets, road.dst_i),
    );
    let lanes = convert_lanes(&road.lane_specs_ltr);

    Street::new(osm_way_id, bounding_node_ids, road.name.clone(), lanes)
}

/// The intersection's own OSM node id, or (see `intersection_key`) a
/// non-panicking fallback naming its internal id.
fn first_osm_node_id(
    streets: &osm2streets::StreetNetwork,
    id: osm2streets::IntersectionID,
) -> String {
    streets.intersections[&id]
        .osm_ids
        .first()
        .map(|node_id| node_id.0.to_string())
        .unwrap_or_else(|| format!("synthetic-intersection-{id:?}"))
}

fn convert_lanes(lane_specs: &[osm2streets::LaneSpec]) -> Vec<Lane> {
    let mut ordinal_from_kerb: HashMap<(String, Direction), u32> = HashMap::new();

    lane_specs
        .iter()
        .map(|spec| {
            let lane_type = lane_type_name(spec.lt);
            let direction = convert_direction(spec.dir);
            let counter = ordinal_from_kerb
                .entry((lane_type.clone(), direction))
                .or_insert(0);
            let ordinal = *counter;
            *counter += 1;

            Lane::new(lane_type, direction, convert_dimension(spec), ordinal)
        })
        .collect()
}

/// This project's lane-type name for an osm2streets `LaneType`. Naming
/// authority is osm2streets' own schema (`team.md`, Code Style) — every
/// variant maps to osm2streets' own `short_name()` — with one required
/// override: `street-core`'s carriageway-width computation (BR4.1) matches
/// the kerb buffer by the exact constant `street_core::
/// KERB_BUFFER_LANE_TYPE` ("kerb_buffer"), not osm2lanes' own
/// `short_name()` spelling ("curb"), so this is the one place that
/// difference is bridged.
fn lane_type_name(lane_type: osm2streets::LaneType) -> String {
    if lane_type == osm2streets::LaneType::Buffer(osm2streets::BufferType::Curb) {
        KERB_BUFFER_LANE_TYPE.to_string()
    } else {
        lane_type.short_name().to_string()
    }
}

fn convert_direction(direction: osm2streets::Direction) -> Direction {
    match direction {
        osm2streets::Direction::Forward => Direction::Forward,
        osm2streets::Direction::Backward => Direction::Backward,
    }
}

/// BR4.1: `Mapped` when osm2streets/muv-osm reports the lane's width as
/// tagged/explicit (`LaneSpec.lane`'s own width is `Some`, whether from an
/// explicit per-lane width tag or the overall way `width` tag divided
/// across lanes — muv-osm's own documented behaviour for that field);
/// `Inferred` otherwise, including every lane osm2streets synthesises with
/// no underlying `muv_osm::lanes::Lane` at all (`LaneSpec.lane` is `None`
/// — e.g. an inferred kerb buffer). There is no third outcome at this
/// boundary (BR4.1) — `UserSet` is only ever assigned by a `Correction`.
fn convert_dimension(spec: &osm2streets::LaneSpec) -> Dimension {
    let provenance = if spec.lane.as_ref().is_some_and(|lane| lane.width.is_some()) {
        Provenance::Mapped
    } else {
        Provenance::Inferred
    };
    Dimension {
        metres: spec.width.inner_meters(),
        provenance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use street_core::Provenance;

    fn fixture_bytes(name: &str) -> Vec<u8> {
        let path = format!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/osm2streets/{}"),
            name
        );
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading fixture {path}: {error}"))
    }

    // Test 8 / AC3.1.1: `well-tagged-street.osm.xml` converts to a
    // `Street` with type, direction, and width populated for each lane,
    // in left-to-right order (BR4.1). Real, recorded osm2streets output
    // against this fixture (`code-summary.md` records the full spike
    // output this Unit's Red step captured): the four driving lanes'
    // widths are `Mapped` (`width=16` divides across them exactly, and
    // muv-osm reports that division as the lane's own explicit width);
    // the cycleway (`Biking`) lane and the inferred kerb buffer are
    // `Inferred` — the fixture tags `cycleway:left=track` (presence and
    // type) but never a *width* for that lane, and the kerb buffer is an
    // osm2streets-synthesised lane with no underlying OSM tag at all. This
    // Unit therefore records AC3.1.1's "entirely Mapped" expectation as
    // not holding against the real pinned dependency for this fixture —
    // see `code-summary.md`'s discrepancy note — and asserts the true,
    // verified behaviour instead of a false "all Mapped" claim.
    #[test]
    fn well_tagged_street_has_mapped_driving_lane_widths_in_ltr_order() {
        let adapter = StreetImportAdapter::new();

        let graph = adapter
            .import_network(
                "Northeast Pacific Street",
                &fixture_bytes("well-tagged-street.osm.xml"),
            )
            .expect("well-tagged-street.osm.xml must import");

        let streets: Vec<_> = graph.streets().collect();
        assert_eq!(streets.len(), 1);
        let street = streets[0];
        assert_eq!(street.name(), Some("Northeast Pacific Street"));
        assert_eq!(street.lane_count(), 6);

        let expected_types_and_directions = [
            ("bike lane", Direction::Backward),
            (KERB_BUFFER_LANE_TYPE, Direction::Backward),
            ("driving lane", Direction::Backward),
            ("driving lane", Direction::Forward),
            ("driving lane", Direction::Forward),
            ("driving lane", Direction::Forward),
        ];
        for (index, (lane_type, direction)) in expected_types_and_directions.iter().enumerate() {
            let lane = street.lane_at(index).unwrap();
            assert_eq!(lane.lane_type(), *lane_type, "lane {index}");
            assert_eq!(lane.direction(), *direction, "lane {index}");
        }

        // The four driving lanes: `width=16` divided across them, Mapped.
        for index in 2..6 {
            let lane = street.lane_at(index).unwrap();
            assert_eq!(lane.width().metres, 4.0, "lane {index}");
            assert_eq!(lane.width().provenance, Provenance::Mapped, "lane {index}");
        }
        // The cycleway lane and the synthesised kerb buffer: no width tag
        // of their own anywhere in this fixture, so Inferred.
        assert_eq!(
            street.lane_at(0).unwrap().width().provenance,
            Provenance::Inferred
        );
        assert_eq!(
            street.lane_at(1).unwrap().width().provenance,
            Provenance::Inferred
        );
    }

    // Test 9: `thinly-tagged-street.osm.xml` converts with at least one
    // lane carrying `Inferred` provenance (BR4.1, `team.md`'s
    // provenance-test floor).
    #[test]
    fn thinly_tagged_street_yields_at_least_one_inferred_lane() {
        let adapter = StreetImportAdapter::new();

        let graph = adapter
            .import_network(
                "Northeast 48th Street",
                &fixture_bytes("thinly-tagged-street.osm.xml"),
            )
            .expect("thinly-tagged-street.osm.xml must import");

        let streets: Vec<_> = graph.streets().collect();
        assert_eq!(streets.len(), 1);
        let street = streets[0];
        assert_eq!(street.lane_count(), 2);
        assert!(
            (0..street.lane_count())
                .filter_map(|index| street.lane_at(index))
                .any(|lane| lane.width().provenance == Provenance::Inferred),
            "no width tag exists anywhere in this fixture; every lane width must be Inferred"
        );
    }

    // Test 10: `one-way-street.osm.xml` converts with every lane
    // attributed to a single travel direction.
    #[test]
    fn one_way_street_attributes_every_lane_to_a_single_direction() {
        let adapter = StreetImportAdapter::new();

        let graph = adapter
            .import_network("Madison Street", &fixture_bytes("one-way-street.osm.xml"))
            .expect("one-way-street.osm.xml must import");

        let streets: Vec<_> = graph.streets().collect();
        assert_eq!(streets.len(), 1);
        let street = streets[0];
        assert!(street.lane_count() >= 1);
        let directions: Vec<Direction> = (0..street.lane_count())
            .filter_map(|index| street.lane_at(index))
            .map(|lane| lane.direction())
            .collect();
        assert!(
            directions
                .iter()
                .all(|direction| *direction == directions[0]),
            "a one-way street's lanes must all share one direction, got {directions:?}"
        );
    }

    // Test 11: `street-with-cycleway.osm.xml` converts without conflating
    // the road way and the separately-mapped cycleway way.
    #[test]
    fn street_with_cycleway_does_not_conflate_the_two_ways() {
        let adapter = StreetImportAdapter::new();

        let graph = adapter
            .import_network(
                "North Northlake Way",
                &fixture_bytes("street-with-cycleway.osm.xml"),
            )
            .expect("street-with-cycleway.osm.xml must import");

        let streets: Vec<_> = graph.streets().collect();
        assert_eq!(
            streets.len(),
            2,
            "the road and the separately-mapped cycleway must remain two distinct streets"
        );
        let way_ids: Vec<String> = streets
            .iter()
            .map(|street| street.identity().osm_way_id)
            .collect();
        assert!(way_ids.iter().any(|id| id == "8111822"));
        assert!(way_ids.iter().any(|id| id == "1081893497"));
    }

    // Test 12: `one-intersection.osm.xml` converts to a `StreetNetworkGraph`
    // connecting the split segments at the shared node.
    #[test]
    fn one_intersection_connects_the_split_segments() {
        let adapter = StreetImportAdapter::new();

        let graph = adapter
            .import_network(
                "15th Avenue Northeast & Northeast 50th Street",
                &fixture_bytes("one-intersection.osm.xml"),
            )
            .expect("one-intersection.osm.xml must import");

        assert_eq!(graph.streets().count(), 5);
        let four_way_junction = graph
            .intersections()
            .iter()
            .find(|intersection| graph.streets_at(intersection).map(<[_]>::len) == Some(4));
        assert!(
            four_way_junction.is_some(),
            "the real four-way signalized junction must connect four streets"
        );
    }

    // Test 13: a synthetic malformed byte sequence classifies as
    // `ImportFailure::malformed_extract` without an uncaught panic
    // escaping the adapter (SD-1's `catch_unwind` boundary). This exact
    // byte sequence (a way with a single node) is verified, during this
    // Unit's build, to genuinely panic inside `osm2streets`'s geometry
    // code (`Bad Pt2D NaN, NaN` in the pinned `geom` crate) rather than
    // return a graceful `Err` — so this test exercises the real
    // `catch_unwind` path, not just the ordinary error path Step 14 below
    // covers implicitly.
    #[test]
    fn a_single_node_way_panics_inside_osm2streets_and_is_caught() {
        let malformed = br#"<?xml version="1.0" encoding="UTF-8"?>
<osm version="0.6">
  <node id="1" lat="47.66" lon="-122.31" version="1"/>
  <way id="1" version="1">
    <nd ref="1"/>
    <tag k="highway" v="residential"/>
  </way>
</osm>"#;
        let adapter = StreetImportAdapter::new();

        let result = adapter.import_network("Malformed Street", malformed);

        let failure = result.expect_err("a single-node way must classify as a failure");
        assert_eq!(
            failure.reason(),
            cityloom_api_types::FailureReason::MalformedExtract
        );
        assert_eq!(failure.street_name(), "Malformed Street");
    }

    // A second, non-panicking malformed case (empty bytes) — exercises the
    // adapter's graceful `Err` path (`streets_reader`'s own parse failure)
    // as distinct from the panic path above.
    #[test]
    fn empty_bytes_classify_as_malformed_extract_without_panicking() {
        let adapter = StreetImportAdapter::new();

        let result = adapter.import_network("Malformed Street", b"");

        let failure = result.expect_err("empty bytes must classify as a failure");
        assert_eq!(
            failure.reason(),
            cityloom_api_types::FailureReason::MalformedExtract
        );
    }

    // Test 14 / AC3.1.4: `well-tagged-street.osm.xml`'s lane count and
    // order match the fixture exactly, asserted against literal expected
    // values recorded from the fixture's own tags (`lanes=4` split
    // `lanes:forward=3`/`lanes:backward=1`, plus the separately-tagged
    // `cycleway:left=track` and the kerb buffer osm2streets infers
    // alongside it — recorded from this Unit's Red-step spike output, not
    // re-derived from the fixture at test time).
    #[test]
    fn well_tagged_street_lane_count_and_order_match_the_fixture_exactly() {
        let adapter = StreetImportAdapter::new();

        let graph = adapter
            .import_network(
                "Northeast Pacific Street",
                &fixture_bytes("well-tagged-street.osm.xml"),
            )
            .expect("well-tagged-street.osm.xml must import");

        let streets: Vec<_> = graph.streets().collect();
        let street = streets[0];
        assert_eq!(street.lane_count(), 6);
        let forward_count = (0..street.lane_count())
            .filter_map(|index| street.lane_at(index))
            .filter(|lane| lane.direction() == Direction::Forward)
            .count();
        let backward_count = (0..street.lane_count())
            .filter_map(|index| street.lane_at(index))
            .filter(|lane| lane.direction() == Direction::Backward)
            .count();
        // 3 forward driving lanes (lanes:forward=3, none backward-side);
        // 1 backward driving lane (lanes:backward=1) plus the
        // backward-side bike lane (`cycleway:left=track`) and its kerb
        // buffer = 3 backward.
        assert_eq!(forward_count, 3);
        assert_eq!(backward_count, 3);
    }
}
