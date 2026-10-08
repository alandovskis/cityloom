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
use streets_reader::{OsmExtract, osm_reader::Document};
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
    let (mut streets, doc) = read_streets(osm, clip, config, &mut timer)?;
    streets.apply_transformations(Transformation::standard_for_clipped_areas(), &mut timer);
    let mut network = convert(&streets, &doc);
    osm_network::merge::merge_dual_carriageways(&mut network);
    Ok(network)
}

/// `streets_reader::osm_to_street_network`, with the sidewalk tags made explicit before the lanes are worked out
/// from them. That function does not let anything in between, and its reading step is private, so the steps are
/// repeated here as it takes them.
fn read_streets(osm: &[u8], clip: Option<Vec<geom::LonLat>>, config: MapConfig, timer: &mut Timer) -> Result<(StreetNetwork, Document), String> {
    let err = |e: &dyn std::fmt::Display| e.to_string();
    let mut streets = StreetNetwork::blank();
    streets.config = config;
    let mut doc = Document::read(osm, clip.as_ref().map(|pts| geom::GPSBounds::from(pts.clone())), timer).map_err(|e| err(&e))?;
    streets.gps_bounds = doc.gps_bounds.clone().ok_or("the OSM input has no GPS bounds")?;
    if let Some(pts) = clip {
        streets.boundary_polygon = geom::Ring::deduping_new(streets.gps_bounds.convert(&pts)).map_err(|e| err(&e))?.into_polygon();
        doc.clip(&streets.boundary_polygon, timer);
    } else {
        streets.boundary_polygon = streets.gps_bounds.to_bounds().get_rectangle();
    }
    streets_reader::detect_country_code(&mut streets);

    let mut extract = OsmExtract::new();
    for (id, node) in &doc.nodes {
        extract.handle_node(*id, node);
    }
    for way in doc.ways.values_mut().chain(doc.clipped_copied_ways.iter_mut().map(|(_, way)| way)) {
        explicit_sidewalks(&mut way.tags);
    }
    for (id, way) in doc.ways.iter().chain(doc.clipped_copied_ways.iter().map(|(id, way)| (id, way))) {
        extract.handle_way(*id, way, &streets.config);
    }
    for (id, rel) in &doc.relations {
        extract.handle_relation(*id, rel);
    }
    streets_reader::split_ways::split_up_roads(&mut streets, extract, timer);
    // Cul-de-sacs aren't supported yet.
    streets.retain_roads(|r| r.src_i != r.dst_i);
    Ok((streets, doc))
}

/// A way tagged `sidewalk:left` and/or `sidewalk:right` (and not `sidewalk`) is given the `sidewalk` they say.
/// osm2streets takes a side not marked `no` to have one but then leaves out one mapped as `separate`, and takes a
/// side that is not tagged at all to have one as well. Here a side has a sidewalk whatever it is tagged, except
/// `no`, and a side not tagged has none. The sidewalk beside the road is shown whether it is part of the road's
/// own way or a separate way of its own.
fn explicit_sidewalks(tags: &mut Tags) {
    if tags.contains_key("sidewalk") || !(tags.contains_key("sidewalk:left") || tags.contains_key("sidewalk:right")) {
        return;
    }
    let has = |side: &str| tags.get(side).is_some_and(|v| v != "no" && v != "none");
    let value = match (has("sidewalk:left"), has("sidewalk:right")) {
        (true, true) => "both",
        (true, false) => "left",
        (false, true) => "right",
        (false, false) => "none",
    };
    tags.remove("sidewalk:left");
    tags.remove("sidewalk:right");
    tags.insert("sidewalk", value);
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
            osm_versions: i.osm_ids.iter().filter_map(|n| Some((n.0, doc.nodes.get(n)?.version?))).collect(),
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
            osm_versions: r.osm_ids.iter().filter_map(|w| Some((w.0, doc.ways.get(w)?.version?))).collect(),
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

    fn sidewalks(tags: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut tags = Tags::new(tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        explicit_sidewalks(&mut tags);
        tags.into_inner().into_iter().collect()
    }

    #[test]
    fn the_sidewalk_of_each_side_becomes_one_explicit_sidewalk_tag() {
        let pair = |k: &str, v: &str| (k.to_string(), v.to_string());
        assert_eq!(sidewalks(&[("sidewalk:left", "yes"), ("sidewalk:right", "separate")]), [pair("sidewalk", "both")]);
        assert_eq!(sidewalks(&[("sidewalk:right", "explicit")]), [pair("sidewalk", "right")]);
        assert_eq!(sidewalks(&[("sidewalk:left", "no"), ("sidewalk:right", "none")]), [pair("sidewalk", "none")]);
    }

    #[test]
    fn a_way_with_a_sidewalk_tag_or_with_no_side_tagged_is_left_as_it_is() {
        let tags = [("highway", "residential"), ("sidewalk", "left"), ("sidewalk:right", "yes")];
        assert_eq!(sidewalks(&tags).len(), 3);
        assert_eq!(sidewalks(&[("highway", "residential")]), [("highway".to_string(), "residential".to_string())]);
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
