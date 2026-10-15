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
use std::collections::BTreeMap;

use osm2streets::osm::WayID;
use streets_reader::osm_reader::Way as OsmWay;
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
    let two_way = two_way_tracks(&doc);
    for (id, way) in doc.ways.iter_mut().map(|(id, way)| (*id, way)).chain(doc.clipped_copied_ways.iter_mut().map(|(id, way)| (*id, way))) {
        for key in two_way.get(&id).into_iter().flatten() {
            way.tags.insert(key.clone(), "no");
        }
        explicit_sidewalks(&mut way.tags);
        explicit_cycleways(&mut way.tags);
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

/// A way tagged `sidewalk=separate` is given `sidewalk=both`, which says the same. A way tagged `sidewalk:left`,
/// `sidewalk:right` and/or `sidewalk:both` (and not `sidewalk`) is given the `sidewalk` they say. osm2streets takes a side not marked `no` to have one but then leaves out one mapped as `separate`, and
/// takes a side that is not tagged at all to have one as well. Here a side has a sidewalk whatever it is tagged,
/// except `no`, and a side not tagged has none. The sidewalk beside the road is shown whether it is part of the
/// road's own way or a separate way of its own. `both` is the shorthand for the two sides, and a side tagged on its
/// own says more than it does.
fn explicit_sidewalks(tags: &mut Tags) {
    // `sidewalk=separate` is the shorthand `sidewalk:both=separate`, which osm2streets leaves out
    if tags.is("sidewalk", "separate") {
        tags.remove("sidewalk");
        if !tags.contains_key("sidewalk:both") {
            tags.insert("sidewalk:both", "separate");
        }
    }
    if tags.contains_key("sidewalk") || !["sidewalk:left", "sidewalk:right", "sidewalk:both"].iter().any(|k| tags.contains_key(k)) {
        return;
    }
    let has = |side: &str| tags.get(side).or_else(|| tags.get("sidewalk:both")).is_some_and(|v| v != "no" && v != "none");
    let value = match (has("sidewalk:left"), has("sidewalk:right")) {
        (true, true) => "both",
        (true, false) => "left",
        (false, true) => "right",
        (false, false) => "none",
    };
    tags.remove("sidewalk:left");
    tags.remove("sidewalk:right");
    tags.remove("sidewalk:both");
    tags.insert("sidewalk", value);
}

/// A side whose bike track is mapped as a way of its own (`cycleway:left=separate`, `cycleway:right=separate`,
/// `cycleway:both=separate` or `cycleway=separate`) has the bike track beside the road that it has, as a sidewalk
/// mapped separately has its sidewalk: osm2streets leaves a track mapped as `separate` out of the road's lanes, and
/// the way that is the track itself is not kept (it is not among the highways the data holds). Whether the track
/// runs one way or both is what the road's own tags say; a track mapped as a way of its own with `oneway=no` on that
/// way is shown as running with the traffic.
fn explicit_cycleways(tags: &mut Tags) {
    for key in ["cycleway", "cycleway:left", "cycleway:right", "cycleway:both"] {
        if tags.is(key, "separate") {
            tags.insert(key, "track");
        }
    }
}

/// How near a separately mapped cycleway lies to the road it runs beside, to its middle, in metres.
const TRACK_NEAR_M: f64 = 30.0;

/// The `cycleway:<side>:oneway` tags (`=no`) to give the roads whose bike track is mapped as ways of its own and runs
/// both ways. osm2streets does not keep the ways of a track (and its lane has one direction only), so the road's side
/// is tied to them here, before it runs: a `highway=cycleway` way that runs along the road (within 30 degrees of it,
/// its middle within 30 m, on the side tagged `separate`), is not a crossing and is not one-way (OSM's default for a
/// cycleway is both ways; `oneway`, `oneway:bicycle` or `bicycle:oneway` `yes`, `true`, `1` or `-1` say otherwise).
/// A side where such ways are both one-way and two-way is left as the road's tags say.
fn two_way_tracks(doc: &Document) -> BTreeMap<WayID, Vec<String>> {
    let tracks: Vec<_> = doc.ways.values().filter(|w| w.tags.is("highway", "cycleway") && !w.tags.is("cycleway", "crossing")).collect();
    let mut found = BTreeMap::new();
    for (id, road) in &doc.ways {
        if !road.tags.has_any(vec!["cycleway", "cycleway:left", "cycleway:right", "cycleway:both"]) || road.tags.is("highway", "cycleway") {
            continue;
        }
        let separate = |side: &str| {
            ["cycleway", "cycleway:both", &format!("cycleway:{side}")].iter().rev().find_map(|k| road.tags.get(k)).is_some_and(|v| v == "separate")
        };
        for side in ["left", "right"] {
            if !separate(side) || road.tags.contains_key(&format!("cycleway:{side}:oneway")) || road.tags.contains_key("cycleway:both:oneway") {
                continue;
            }
            let along: Vec<_> = tracks.iter().filter(|t| runs_along(road, t, side)).collect();
            if !along.is_empty() && along.iter().all(|t| !is_one_way(&t.tags)) {
                found.entry(*id).or_insert_with(Vec::new).push(format!("cycleway:{side}:oneway"));
            }
        }
    }
    found
}

fn is_one_way(tags: &Tags) -> bool {
    ["oneway", "oneway:bicycle", "bicycle:oneway"].iter().any(|k| tags.get(k).is_some_and(|v| matches!(v.as_str(), "yes" | "true" | "1" | "-1")))
}

/// Whether `track` runs along `road` on its `side`: its middle lies near the road, on that side, and it points the
/// way the road does or the opposite.
fn runs_along(road: &OsmWay, track: &OsmWay, side: &str) -> bool {
    let (Some(mid), true) = (track.pts.get(track.pts.len() / 2), road.pts.len() > 1) else { return false };
    let (Some(first), Some(last)) = (track.pts.first(), track.pts.last()) else { return false };
    let Some((a, b, dist)) = road.pts.windows(2).map(|w| (w[0], w[1], distance_to_segment(*mid, w[0], w[1]))).min_by(|x, y| x.2.total_cmp(&y.2)) else {
        return false;
    };
    if dist > TRACK_NEAR_M {
        return false;
    }
    let (rx, ry) = (b.x() - a.x(), b.y() - a.y());
    let (tx, ty) = (last.x() - first.x(), last.y() - first.y());
    let (rl, tl) = (rx.hypot(ry), tx.hypot(ty));
    if rl == 0.0 || tl == 0.0 || (rx * tx + ry * ty).abs() / (rl * tl) < 30f64.to_radians().cos() {
        return false;
    }
    // Pt2D's y runs south, so a point to the left of the road's direction has a negative cross product
    let cross = rx * (mid.y() - a.y()) - ry * (mid.x() - a.x());
    (cross < 0.0) == (side == "left")
}

fn distance_to_segment(p: geom::Pt2D, a: geom::Pt2D, b: geom::Pt2D) -> f64 {
    let (dx, dy) = (b.x() - a.x(), b.y() - a.y());
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 { 0.0 } else { (((p.x() - a.x()) * dx + (p.y() - a.y()) * dy) / len2).clamp(0.0, 1.0) };
    (p.x() - (a.x() + t * dx)).hypot(p.y() - (a.y() + t * dy))
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
            lanes: join_two_way_bike_lanes(
                r.lane_specs_ltr
                    .iter()
                    .map(|l| Lane {
                        kind: lane_kind(l.lt),
                        buffer_type: buffer_type(l.lt),
                        way: if l.dir == Direction::Forward { Way::Forward } else { Way::Backward },
                        width_m: l.width.inner_meters(),
                        hours: if l.lt == LaneType::Bus { hours.clone() } else { None },
                    })
                    .collect(),
            ),
            points: r.center_line.points().iter().map(|p| (p.x(), height - p.y())).collect(),
        });
    }
    net
}

/// A bike track that runs both ways is two lanes to osm2streets, one for each way, side by side (a `oneway=no` on the
/// side of the road). It is one lane here, as wide as the two, that runs both ways.
fn join_two_way_bike_lanes(lanes: Vec<Lane>) -> Vec<Lane> {
    let mut joined: Vec<Lane> = Vec::with_capacity(lanes.len());
    for lane in lanes {
        match joined.last_mut() {
            Some(last)
                if last.kind == LaneKind::Bike
                    && lane.kind == LaneKind::Bike
                    && matches!((last.way, lane.way), (Way::Forward, Way::Backward) | (Way::Backward, Way::Forward)) =>
            {
                last.way = Way::Both;
                last.width_m += lane.width_m;
            }
            _ => joined.push(lane),
        }
    }
    joined
}

/// What a buffer lane is made of, osm2streets' own types. A shoulder has none: see `lane_kind`.
fn buffer_type(lt: LaneType) -> Option<BufferType> {
    let LaneType::Buffer(t) = lt else { return None };
    Some(match t {
        osm2streets::BufferType::Stripes => BufferType::Stripes,
        osm2streets::BufferType::FlexPosts => BufferType::FlexPosts,
        osm2streets::BufferType::Planters => BufferType::Planters,
        osm2streets::BufferType::JerseyBarrier => BufferType::JerseyBarrier,
        osm2streets::BufferType::Curb => BufferType::Curb,
        osm2streets::BufferType::Verge => BufferType::Verge,
    })
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
    let value = ["lanes:bus:conditional", "lanes:psv:conditional"].iter().find_map(|k| tags.get(k))?;
    let (_, hours) = value.split_once('@')?;
    let hours = hours.trim();
    let hours = hours.strip_prefix('(').and_then(|h| h.strip_suffix(')')).unwrap_or(hours).trim();
    (!hours.is_empty()).then(|| hours.to_string())
}

/// For the browser: OSM XML or PBF in, the network as JSON out (or the error text, as a thrown string).
/// `bounds` is empty, or the box to keep: south, west, north, east.
#[wasm_bindgen]
pub fn osm_to_network(osm: &[u8], bounds: &[f64]) -> Result<String, String> {
    let bounds = <[f64; 4]>::try_from(bounds).ok();
    serde_json::to_string(&import_in(osm, bounds)?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bike(way: Way, width_m: f64) -> Lane {
        Lane { kind: LaneKind::Bike, way, width_m, buffer_type: None, hours: None }
    }

    #[test]
    fn two_bike_lanes_side_by_side_that_run_against_each_other_are_one_two_way_lane() {
        let lanes = join_two_way_bike_lanes(vec![bike(Way::Backward, 1.5), bike(Way::Forward, 1.5)]);
        assert_eq!(lanes, [bike(Way::Both, 3.0)]);
        let lanes = join_two_way_bike_lanes(vec![bike(Way::Forward, 1.5), bike(Way::Backward, 1.2)]);
        assert_eq!(lanes, [bike(Way::Both, 2.7)]);
    }

    #[test]
    fn bike_lanes_that_run_the_same_way_or_are_apart_stay_as_they_are() {
        let same = vec![bike(Way::Forward, 1.5), bike(Way::Forward, 1.5)];
        assert_eq!(join_two_way_bike_lanes(same.clone()), same);
        let driving = Lane { kind: LaneKind::Driving, ..bike(Way::Forward, 3.0) };
        let apart = vec![bike(Way::Backward, 1.5), driving, bike(Way::Forward, 1.5)];
        assert_eq!(join_two_way_bike_lanes(apart.clone()), apart);
    }

    #[test]
    fn a_buffer_keeps_the_type_osm2streets_gave_it() {
        use osm2streets::BufferType as B;
        let types = [
            (B::Stripes, BufferType::Stripes),
            (B::FlexPosts, BufferType::FlexPosts),
            (B::Planters, BufferType::Planters),
            (B::JerseyBarrier, BufferType::JerseyBarrier),
            (B::Curb, BufferType::Curb),
            (B::Verge, BufferType::Verge),
        ];
        for (from, to) in types {
            assert_eq!(lane_kind(LaneType::Buffer(from)), LaneKind::Buffer);
            assert_eq!(buffer_type(LaneType::Buffer(from)), Some(to));
        }
        assert_eq!(buffer_type(LaneType::Shoulder), None);
        assert_eq!(buffer_type(LaneType::Driving), None);
    }

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

    fn cycleways(tags: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut tags = Tags::new(tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        explicit_cycleways(&mut tags);
        tags.into_inner().into_iter().collect()
    }

    #[test]
    fn a_bike_track_mapped_separately_becomes_a_bike_track_on_that_side() {
        let pair = |k: &str, v: &str| (k.to_string(), v.to_string());
        assert_eq!(cycleways(&[("cycleway:both", "separate")]), [pair("cycleway:both", "track")]);
        assert_eq!(cycleways(&[("cycleway:left", "separate"), ("cycleway:right", "no")]), [pair("cycleway:left", "track"), pair("cycleway:right", "no")]);
        assert_eq!(cycleways(&[("cycleway", "separate")]), [pair("cycleway", "track")]);
    }

    #[test]
    fn a_bike_lane_tagged_any_other_way_is_left_as_it_is() {
        let tags = [("cycleway:left", "lane"), ("cycleway:right", "shared_lane"), ("cycleway", "no"), ("cycleway:both", "track")];
        assert_eq!(cycleways(&tags).len(), 4);
        assert!(cycleways(&tags).contains(&("cycleway:left".to_string(), "lane".to_string())));
        assert!(cycleways(&tags).contains(&("cycleway:both".to_string(), "track".to_string())));
    }

    #[test]
    fn a_plain_separate_sidewalk_tag_is_a_sidewalk_on_each_side_and_the_both_shorthand_is_left_beside_a_sidewalk_tag() {
        let pair = |k: &str, v: &str| (k.to_string(), v.to_string());
        assert_eq!(sidewalks(&[("sidewalk", "separate")]), [pair("sidewalk", "both")]);
        assert_eq!(sidewalks(&[("sidewalk", "left"), ("sidewalk:both", "yes")]).len(), 2, "a way that says `sidewalk` is left as it is");
        // a side tagged on its own overrides the shorthand, `separate` as much as `both`
        assert_eq!(sidewalks(&[("sidewalk", "separate"), ("sidewalk:left", "no")]), [pair("sidewalk", "right")]);
        assert_eq!(sidewalks(&[("sidewalk", "separate"), ("sidewalk:right", "no"), ("sidewalk:left", "yes")]), [pair("sidewalk", "left")]);
    }

    #[test]
    fn the_both_shorthand_gives_a_sidewalk_on_each_side_and_a_side_tagged_on_its_own_overrides_it() {
        let pair = |k: &str, v: &str| (k.to_string(), v.to_string());
        assert_eq!(sidewalks(&[("sidewalk:both", "separate")]), [pair("sidewalk", "both")]);
        assert_eq!(sidewalks(&[("sidewalk:both", "no")]), [pair("sidewalk", "none")]);
        assert_eq!(sidewalks(&[("sidewalk:both", "yes"), ("sidewalk:left", "no")]), [pair("sidewalk", "right")]);
        assert_eq!(sidewalks(&[("sidewalk:both", "no"), ("sidewalk:right", "yes")]), [pair("sidewalk", "right")]);
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
