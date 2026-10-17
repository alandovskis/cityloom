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
    for (piece, way) in doc.ways.values_mut().chain(doc.clipped_copied_ways.iter_mut().map(|(_, way)| way)).enumerate() {
        for key in two_way.get(&piece).into_iter().flatten() {
            way.tags.insert(key.clone(), "no");
        }
        explicit_sidewalks(&mut way.tags);
        explicit_cycleways(&mut way.tags);
    }
    for (id, way) in doc.ways.iter().chain(doc.clipped_copied_ways.iter().map(|(id, way)| (id, way))) {
        if is_off_road(&way.tags) {
            continue;
        }
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
/// the way that is the track itself is not a road (`is_off_road` leaves it out of what osm2streets is given). Whether the track
/// runs one way (with the traffic, as osm2streets reads `cycleway=track`) or both is told by the road's own
/// `cycleway:<side>:oneway`, or, for a track mapped as ways of its own, by `two_way_tracks`.
fn explicit_cycleways(tags: &mut Tags) {
    for key in ["cycleway", "cycleway:left", "cycleway:right", "cycleway:both"] {
        if tags.is(key, "separate") {
            tags.insert(key, "track");
        }
    }
}

/// How far from the road's centre line a separately mapped track lies, in metres: nearer is a lane of the road,
/// further is another street's. 16 m reaches past the track of a wide boulevard (four to six lanes, a buffer and the
/// track lie 13 m or more from the centre) and stops short of a parallel street's, 20 m and more away.
const TRACK_OFFSET_M: std::ops::RangeInclusive<f64> = 1.5..=16.0;

/// The most a track may turn from the road beside it, in degrees.
const TRACK_TURN_DEG: f64 = 30.0;

/// The share of a track's sample points that must lie beside the road, so that a track covering only part of a long
/// road, or crossing it, is judged by where it runs.
const TRACK_SHARE: f64 = 0.6;

/// How many points are taken along a track, evenly by length.
const TRACK_SAMPLES: usize = 5;

/// The `cycleway:<side>:oneway` tags (`=no`) to give the roads whose bike track is mapped as ways of its own and runs
/// both ways, by the index of the piece of way (the ways of the document in order, then the pieces the clip cut, each
/// its own). osm2streets does not keep the ways of a track (and its lane has one direction only), so the road's side
/// is tied to them here, before it runs. A track is a `highway=cycleway` way, or a `path` or `footway` designated for
/// bicycles (osm2streets reads those as bike lanes as well), that is not a crossing.
///
/// A track belongs to one road, chosen among the carriageways it runs along (`runs_along`: at least 60% of five points
/// along it lie within the road's extent, 1.5 to 16 m from its centre line on a side, and within 30 degrees of it).
/// Of those, a road that says `separate` on that side comes first, then the nearest (the mean distance of the points),
/// then the lower way id, so that a service road or a frontage lane nearer to the track does not take it from the
/// street it is mapped for. The side is two-way when it has such tracks and none of them is one-way (OSM's default for
/// a cycleway is both ways: `is_one_way`). A side where the road's own tags say `oneway` is left as they say.
///
/// A road that says `separate` on a side but also `cycleway:<side>:oneway` (or `cycleway:both:oneway`) does not use
/// a track, so it ranks as one that does not say `separate`. A tagged road that is not a carriageway (an alley, a
/// pedestrian street, a `road` or a `track`) is never a candidate and stays as its tags say: letting any road
/// eligible would also make the cycleways themselves roads, so the list of carriageways stays.
///
/// Known limit: a standalone cycleway 5 to 16 m from a street tagged `separate` can be claimed by it when it really
/// follows a railway or a river, and a road that says `separate` beside a track that is another street's own takes it
/// when both lie within that band.
fn two_way_tracks(doc: &Document) -> BTreeMap<usize, Vec<String>> {
    let ways: Vec<(WayID, &OsmWay)> = doc.ways.iter().map(|(id, w)| (*id, w)).chain(doc.clipped_copied_ways.iter().map(|(id, w)| (*id, w))).collect();
    let separate = |road: &OsmWay, side: &str| {
        ["cycleway", "cycleway:both", &format!("cycleway:{side}")].iter().rev().find_map(|k| road.tags.get(k)).is_some_and(|v| v == "separate")
    };
    let has_oneway = |road: &OsmWay, side: &str| road.tags.contains_key(&format!("cycleway:{side}:oneway")) || road.tags.contains_key("cycleway:both:oneway");
    let wanted = |road: &OsmWay, side: &str| separate(road, side) && !has_oneway(road, side);
    let roads: Vec<usize> = (0..ways.len()).filter(|&i| is_carriageway(&ways[i].1.tags)).collect();
    if !roads.iter().any(|&r| ["left", "right"].iter().any(|side| wanted(ways[r].1, side))) {
        return BTreeMap::new();
    }
    let boxes: Vec<(f64, f64, f64, f64)> = roads.iter().map(|&r| bounding_box(&ways[r].1.pts)).collect();
    // each track's road and side
    let mut owned: Vec<(usize, &str, &OsmWay)> = Vec::new();
    for (_, track) in ways.iter().filter(|(_, w)| is_track(&w.tags)) {
        let points = samples(track);
        let Some(&(mid, _)) = points.get(points.len() / 2) else { continue };
        let margin = TRACK_OFFSET_M.end() * 2.5;
        let best = roads
            .iter()
            .zip(&boxes)
            .filter(|(_, b)| mid.x() >= b.0 - margin && mid.x() <= b.1 + margin && mid.y() >= b.2 - margin && mid.y() <= b.3 + margin)
            .filter_map(|(&r, _)| {
                let side = ["left", "right"].into_iter().find(|side| runs_along(ways[r].1, track, side))?;
                let mean = points.iter().map(|(p, _)| distance_to_line(*p, &ways[r].1.pts)).sum::<f64>() / points.len() as f64;
                Some((!wanted(ways[r].1, side), mean, ways[r].0, r, side))
            })
            .min_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
        if let Some((_, _, _, r, side)) = best {
            owned.push((r, side, track));
        }
    }
    let mut found: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for &r in &roads {
        for side in ["left", "right"] {
            if !wanted(ways[r].1, side) {
                continue;
            }
            let mut mine = owned.iter().filter(|(owner, s, _)| *owner == r && *s == side).peekable();
            if mine.peek().is_some() && mine.all(|(_, _, t)| !is_one_way(&t.tags)) {
                found.entry(r).or_default().push(format!("cycleway:{side}:oneway"));
            }
        }
    }
    found
}

fn bounding_box(line: &[geom::Pt2D]) -> (f64, f64, f64, f64) {
    let (x0, x1) = line.iter().fold((f64::MAX, f64::MIN), |(lo, hi), q| (lo.min(q.x()), hi.max(q.x())));
    let (y0, y1) = line.iter().fold((f64::MAX, f64::MIN), |(lo, hi), q| (lo.min(q.y()), hi.max(q.y())));
    (x0, x1, y0, y1)
}

/// A way cars drive on, the kind a bike track lies beside: the classes from motorway to unclassified, residential,
/// living_street, busway, and a `service` road that is not a driveway, a parking aisle, a drive-through or an alley
/// (a back lane is not a street a track is mapped for).
fn is_carriageway(tags: &Tags) -> bool {
    if tags.is("highway", "service") && tags.is_any("service", vec!["driveway", "parking_aisle", "drive-through", "alley"]) {
        return false;
    }
    tags.get("highway").is_some_and(|h| {
        matches!(
            h.as_str(),
            "motorway"
                | "motorway_link"
                | "trunk"
                | "trunk_link"
                | "primary"
                | "primary_link"
                | "secondary"
                | "secondary_link"
                | "tertiary"
                | "tertiary_link"
                | "unclassified"
                | "residential"
                | "living_street"
                | "service"
                | "busway"
        )
    })
}

/// A way mapped as a bike track of its own: a cycleway, or a path or footway designated for bicycles, not a crossing.
fn is_track(tags: &Tags) -> bool {
    !tags.is("cycleway", "crossing")
        && !tags.is("footway", "crossing")
        && (tags.is("highway", "cycleway") || (tags.is_any("highway", vec!["path", "footway"]) && tags.is("bicycle", "designated")))
}

/// A cycleway, path or footway: the data holds those for `two_way_tracks` (and a person may bring a file that has
/// them), but they are not streets. osm2streets would make roads of them, with a junction wherever one meets or
/// crosses a road, so they are not handed to it.
fn is_off_road(tags: &Tags) -> bool {
    tags.is_any("highway", vec!["cycleway", "path", "footway"])
}

/// Whether a way is one-way for bicycles: what `oneway:bicycle` or `bicycle:oneway` says, or else `oneway`.
fn is_one_way(tags: &Tags) -> bool {
    let value = ["oneway:bicycle", "bicycle:oneway", "oneway"].iter().find_map(|k| tags.get(k));
    value.is_some_and(|v| matches!(v.as_str(), "yes" | "true" | "1" | "-1"))
}

/// Points along a way, evenly by length (at a tenth, three tenths ... of it), each with the segment it lies on.
fn samples(way: &OsmWay) -> Vec<(geom::Pt2D, (geom::Pt2D, geom::Pt2D))> {
    let lengths: Vec<f64> = way.pts.windows(2).map(|w| (w[1].x() - w[0].x()).hypot(w[1].y() - w[0].y())).collect();
    let total: f64 = lengths.iter().sum();
    if way.pts.len() < 2 || total == 0.0 {
        return Vec::new();
    }
    (0..TRACK_SAMPLES)
        .map(|k| {
            let mut left = total * (k as f64 + 0.5) / TRACK_SAMPLES as f64;
            let mut i = 0;
            while i + 1 < lengths.len() && left > lengths[i] {
                left -= lengths[i];
                i += 1;
            }
            let (a, b) = (way.pts[i], way.pts[i + 1]);
            let t = if lengths[i] == 0.0 { 0.0 } else { (left / lengths[i]).min(1.0) };
            (geom::Pt2D::new(a.x() + t * (b.x() - a.x()), a.y() + t * (b.y() - a.y())), (a, b))
        })
        .collect()
}

fn distance_to_line(p: geom::Pt2D, line: &[geom::Pt2D]) -> f64 {
    line.windows(2).map(|w| distance_to_segment(p, w[0], w[1])).fold(f64::MAX, f64::min)
}

/// Whether `track` runs along `road` on its `side`: at least 60% of the points along the track lie, beside a segment
/// of the road (the point falls on the segment, not past its end), 1.5 to 16 m from it on that side, and the
/// track there points the way the segment does or the opposite, within 30 degrees.
fn runs_along(road: &OsmWay, track: &OsmWay, side: &str) -> bool {
    let points = samples(track);
    if points.is_empty() || road.pts.len() < 2 {
        return false;
    }
    let beside = points
        .iter()
        .filter(|(p, (a, b))| {
            // the track's own direction where the point lies
            let (tx, ty) = (b.x() - a.x(), b.y() - a.y());
            let nearest = road
                .pts
                .windows(2)
                .filter_map(|w| {
                    let (rx, ry) = (w[1].x() - w[0].x(), w[1].y() - w[0].y());
                    let len2 = rx * rx + ry * ry;
                    let t = ((p.x() - w[0].x()) * rx + (p.y() - w[0].y()) * ry) / len2;
                    // Pt2D's y runs south, so a point to the left of the road's direction has a negative cross product
                    let cross = rx * (p.y() - w[0].y()) - ry * (p.x() - w[0].x());
                    (len2 > 0.0 && (0.0..=1.0).contains(&t)).then(|| (cross.abs() / len2.sqrt(), cross < 0.0, rx, ry))
                })
                .min_by(|x, y| x.0.total_cmp(&y.0));
            let Some((offset, left, rx, ry)) = nearest else { return false };
            let cos = (rx * tx + ry * ty).abs() / (rx.hypot(ry) * tx.hypot(ty));
            TRACK_OFFSET_M.contains(&offset) && left == (side == "left") && cos >= TRACK_TURN_DEG.to_radians().cos()
        })
        .count();
    beside as f64 >= TRACK_SHARE * points.len() as f64
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

    fn way(pts: &[(f64, f64)]) -> OsmWay {
        OsmWay { nodes: Vec::new(), pts: pts.iter().map(|&(x, y)| geom::Pt2D::new(x, y)).collect(), tags: Tags::empty(), version: None }
    }

    fn tags(pairs: &[(&str, &str)]) -> Tags {
        Tags::new(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    /// A road along the x axis, drawn west to east, 100 m long. y runs south, so the road's left is y < 0.
    fn road() -> OsmWay {
        way(&[(0.0, 0.0), (100.0, 0.0)])
    }

    #[test]
    fn a_track_runs_along_the_road_on_the_side_it_lies() {
        let north = way(&[(10.0, -5.0), (90.0, -5.0)]);
        assert!(runs_along(&road(), &north, "left"));
        assert!(!runs_along(&road(), &north, "right"));
        let south_back = way(&[(90.0, 5.0), (10.0, 5.0)]);
        assert!(runs_along(&road(), &south_back, "right"), "the track's own direction does not matter");
        assert!(!runs_along(&road(), &south_back, "left"));
    }

    #[test]
    fn a_track_too_near_or_too_far_to_be_beside_the_road_does_not_run_along_it() {
        assert!(!runs_along(&road(), &way(&[(10.0, -0.5), (90.0, -0.5)]), "left"));
        assert!(!runs_along(&road(), &way(&[(10.0, -20.0), (90.0, -20.0)]), "left"));
        assert!(runs_along(&road(), &way(&[(10.0, -1.5), (90.0, -1.5)]), "left"));
        assert!(runs_along(&road(), &way(&[(10.0, -16.0), (90.0, -16.0)]), "left"), "the track of a wide boulevard");
        assert!(!runs_along(&road(), &way(&[(10.0, -17.0), (90.0, -17.0)]), "left"));
    }

    #[test]
    fn a_track_that_turns_from_the_road_by_more_than_30_degrees_or_lies_past_its_end_does_not_run_along_it() {
        assert!(!runs_along(&road(), &way(&[(10.0, -5.0), (50.0, -45.0)]), "left"), "a diagonal");
        assert!(!runs_along(&road(), &way(&[(50.0, -2.0), (50.0, -10.0)]), "left"), "across");
        assert!(!runs_along(&road(), &way(&[(110.0, -5.0), (190.0, -5.0)]), "left"), "beyond its end");
        // a bend of the track that stays beside the road for most of its length does
        assert!(runs_along(&road(), &way(&[(10.0, -5.0), (80.0, -5.0), (90.0, -9.0)]), "left"));
    }

    #[test]
    fn a_track_beside_part_of_a_long_road_runs_along_it_and_one_that_overlaps_a_little_does_not() {
        let long = way(&[(0.0, 0.0), (500.0, 0.0)]);
        assert!(runs_along(&long, &way(&[(200.0, -5.0), (260.0, -5.0)]), "left"));
        assert!(!runs_along(&long, &way(&[(450.0, -5.0), (800.0, -5.0)]), "left"), "most of it lies past the road's end");
    }

    #[test]
    fn a_road_of_several_segments_is_judged_by_the_segment_nearest_each_point() {
        let bent = way(&[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0)]);
        assert!(runs_along(&bent, &way(&[(55.0, 5.0), (55.0, 45.0)]), "left"), "beside the second segment, which runs south: its left is east");
    }

    #[test]
    fn a_way_is_one_way_for_bicycles_by_the_tag_that_names_them_before_oneway() {
        assert!(is_one_way(&tags(&[("oneway", "yes")])));
        assert!(is_one_way(&tags(&[("oneway", "-1")])));
        assert!(is_one_way(&tags(&[("oneway:bicycle", "yes")])));
        assert!(is_one_way(&tags(&[("bicycle:oneway", "true")])));
        assert!(!is_one_way(&tags(&[("oneway", "no")])));
        assert!(!is_one_way(&tags(&[("highway", "cycleway")])));
        assert!(!is_one_way(&tags(&[("oneway", "yes"), ("oneway:bicycle", "no")])), "the tag for bicycles wins");
        assert!(is_one_way(&tags(&[("oneway", "no"), ("oneway:bicycle", "yes")])));
        assert!(!is_one_way(&tags(&[("oneway", "yes"), ("bicycle:oneway", "no")])));
    }

    #[test]
    fn a_driveway_a_parking_aisle_and_an_alley_are_no_carriageway_and_a_service_road_is() {
        assert!(is_carriageway(&tags(&[("highway", "service")])));
        assert!(is_carriageway(&tags(&[("highway", "residential")])));
        for kind in ["driveway", "parking_aisle", "drive-through", "alley"] {
            assert!(!is_carriageway(&tags(&[("highway", "service"), ("service", kind)])), "{kind}");
        }
        assert!(!is_carriageway(&tags(&[("highway", "cycleway")])));
    }

    #[test]
    fn a_cycleway_or_a_path_designated_for_bicycles_is_a_track_and_a_crossing_is_not() {
        assert!(is_track(&tags(&[("highway", "cycleway")])));
        assert!(is_track(&tags(&[("highway", "path"), ("bicycle", "designated")])));
        assert!(is_track(&tags(&[("highway", "footway"), ("bicycle", "designated")])));
        assert!(!is_track(&tags(&[("highway", "footway")])));
        assert!(!is_track(&tags(&[("highway", "cycleway"), ("cycleway", "crossing")])));
        assert!(!is_track(&tags(&[("highway", "path"), ("bicycle", "yes")])));
    }

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
