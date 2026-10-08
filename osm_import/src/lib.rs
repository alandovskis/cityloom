//! OpenStreetMap to a plain street network.
//!
//! osm2streets does the hard part (splitting ways at junctions, collapsing the
//! tangle around a big crossing, reading lane tags). This crate runs it and
//! hands the result on as a small, stable shape of its own, so the editor
//! depends on that shape and not on osm2streets. It is built as a separate
//! WebAssembly module, loaded when a place is opened.

use abstutil::{Tags, Timer};
pub use osm_network::*;
use osm2streets::{Direction, DrivingSide, IntersectionControl, IntersectionKind, LaneType, MapConfig, StreetNetwork, Transformation};
use streets_reader::osm_reader::Document;
use wasm_bindgen::prelude::*;

/// Reads OSM XML (or PBF) into a network.
pub fn import(osm: &[u8]) -> Result<Network, String> {
    import_in(osm, None)
}

/// The same, keeping only what lies in a box (south, west, north, east, in degrees): a road that
/// crosses its edge is cut there and ends in a street running off the map.
pub fn import_in(osm: &[u8], bounds: Option<[f64; 4]>) -> Result<Network, String> {
    let mut timer = Timer::throwaway();
    let clip = bounds.map(|[south, west, north, east]| {
        [(west, south), (east, south), (east, north), (west, north), (west, south)].into_iter().map(|(lon, lat)| geom::LonLat::new(lon, lat)).collect()
    });
    // Most streets carry no sidewalk tags: without inference they would have none.
    let config = MapConfig { inferred_sidewalks: true, ..MapConfig::default() };
    let (mut streets, doc) = streets_reader::osm_to_street_network(osm, clip, config, &mut timer).map_err(|e| e.to_string())?;
    streets.apply_transformations(Transformation::standard_for_clipped_areas(), &mut timer);
    let mut network = convert(&streets, &doc);
    osm_network::merge::merge_dual_carriageways(&mut network);
    Ok(network)
}

fn convert(streets: &StreetNetwork, doc: &Document) -> Network {
    let bounds = streets.gps_bounds.to_bounds();
    let height = bounds.max_y;
    let mut net = Network { left_hand: streets.config.driving_side == DrivingSide::Left, ..Network::default() };
    for (id, i) in &streets.intersections {
        let c = i.polygon.center();
        net.nodes.push(Node {
            id: id.0 as u32,
            osm_nodes: i.osm_ids.iter().map(|n| n.0).collect(),
            x_m: c.x(),
            y_m: height - c.y(),
            junction: i.kind == IntersectionKind::Intersection,
            control: match i.control {
                IntersectionControl::Signalled => Control::Signals,
                IntersectionControl::Signed => Control::Signs,
                _ => Control::None,
            },
        });
    }
    for (id, r) in &streets.roads {
        // osm2streets leaves a way's conditional tags behind, so they are read from the way itself
        let hours = r.osm_ids.iter().find_map(|w| doc.ways.get(w).and_then(|w| bus_hours(&w.tags)));
        net.roads.push(Road {
            id: id.0 as u32,
            osm_ways: r.osm_ids.iter().map(|w| w.0).collect(),
            name: r.name.clone(),
            highway: r.highway_type.clone(),
            from: r.src_i.0 as u32,
            to: r.dst_i.0 as u32,
            lanes: r
                .lane_specs_ltr
                .iter()
                .map(|l| Lane {
                    kind: lane_kind(l.lt),
                    way: if l.dir == Direction::Forward { Way::Forward } else { Way::Backward },
                    width_m: l.width.inner_meters(),
                    hours: if l.lt == LaneType::Bus { hours.clone() } else { None },
                })
                .collect(),
            points: r.center_line.points().iter().map(|p| (p.x(), height - p.y())).collect(),
        });
    }
    net
}

/// A shoulder is not somewhere to walk: it is a buffer between the road and what lies beside it.
fn lane_kind(lt: LaneType) -> LaneKind {
    match lt {
        LaneType::Driving => LaneKind::Driving,
        LaneType::Parking(_) => LaneKind::Parking,
        LaneType::Sidewalk | LaneType::Footway => LaneKind::Sidewalk,
        LaneType::Biking => LaneKind::Bike,
        LaneType::Bus => LaneKind::Bus,
        LaneType::Buffer(_) | LaneType::Shoulder => LaneKind::Buffer,
        _ => LaneKind::Other,
    }
}

/// The hours a way's bus lane is one, from `lanes:bus:conditional` or `lanes:psv:conditional`
/// (`1 @ (Mo-Fr 06:00-10:00,14:30-19:00)`): what follows the `@`, without its parentheses.
fn bus_hours(tags: &Tags) -> Option<String> {
    let value = ["lanes:bus:conditional", "lanes:psv:conditional"].iter().find_map(|k| tags.get(*k))?;
    let (_, hours) = value.split_once('@')?;
    let hours = hours.trim();
    let hours = hours.strip_prefix('(').and_then(|h| h.strip_suffix(')')).unwrap_or(hours).trim();
    (!hours.is_empty()).then(|| hours.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shoulder_is_a_buffer_and_not_a_sidewalk() {
        assert_eq!(lane_kind(LaneType::Shoulder), LaneKind::Buffer);
        assert_eq!(lane_kind(LaneType::Sidewalk), LaneKind::Sidewalk);
        assert_eq!(lane_kind(LaneType::Footway), LaneKind::Sidewalk);
    }

    fn hours(key: &str, value: &str) -> Option<String> {
        bus_hours(&Tags::new([(key.to_string(), value.to_string())].into()))
    }

    #[test]
    fn bus_hours_are_what_follows_the_at_sign_with_or_without_parentheses() {
        assert_eq!(hours("lanes:psv:conditional", "1 @ (Mo-Fr 06:00-10:00,14:30-19:00)").as_deref(), Some("Mo-Fr 06:00-10:00,14:30-19:00"));
        assert_eq!(hours("lanes:bus:conditional", "1 @ Mo-Fr 07:00-09:00").as_deref(), Some("Mo-Fr 07:00-09:00"));
    }

    #[test]
    fn a_conditional_without_hours_or_for_something_else_gives_none() {
        assert_eq!(hours("lanes:psv:conditional", "1"), None);
        assert_eq!(hours("lanes:psv:conditional", "1 @ ()"), None);
        assert_eq!(hours("parking:lane:conditional", "no_parking @ (Mo-Fr 07:00-09:00)"), None);
    }
}

/// For the browser: OSM XML or PBF in, the network as JSON out (or the error text, as a thrown string).
/// `bounds` is empty, or the box to keep: south, west, north, east.
#[wasm_bindgen]
pub fn osm_to_network(osm: &[u8], bounds: &[f64]) -> Result<String, String> {
    let bounds = <[f64; 4]>::try_from(bounds).ok();
    serde_json::to_string(&import_in(osm, bounds)?).map_err(|e| e.to_string())
}
