//! OpenStreetMap to a plain street network.
//!
//! osm2streets does the hard part (splitting ways at junctions, collapsing the
//! tangle around a big crossing, reading lane tags). This crate runs it and
//! hands the result on as a small, stable shape of its own, so the editor
//! depends on that shape and not on osm2streets. It is built as a separate
//! WebAssembly module, loaded when a place is opened.

use abstutil::Timer;
pub use osm_network::*;
use osm2streets::{Direction, DrivingSide, IntersectionControl, IntersectionKind, LaneType, MapConfig, StreetNetwork, Transformation};
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
    let (mut streets, _doc) = streets_reader::osm_to_street_network(osm, clip, config, &mut timer).map_err(|e| e.to_string())?;
    streets.apply_transformations(Transformation::standard_for_clipped_areas(), &mut timer);
    let mut network = convert(&streets);
    osm_network::merge::merge_dual_carriageways(&mut network);
    Ok(network)
}

fn convert(streets: &StreetNetwork) -> Network {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shoulder_is_a_buffer_and_not_a_sidewalk() {
        assert_eq!(lane_kind(LaneType::Shoulder), LaneKind::Buffer);
        assert_eq!(lane_kind(LaneType::Sidewalk), LaneKind::Sidewalk);
        assert_eq!(lane_kind(LaneType::Footway), LaneKind::Sidewalk);
    }
}

/// For the browser: OSM XML or PBF in, the network as JSON out (or the error text, as a thrown string).
/// `bounds` is empty, or the box to keep: south, west, north, east.
#[wasm_bindgen]
pub fn osm_to_network(osm: &[u8], bounds: &[f64]) -> Result<String, String> {
    let bounds = <[f64; 4]>::try_from(bounds).ok();
    serde_json::to_string(&import_in(osm, bounds)?).map_err(|e| e.to_string())
}
