//! The city's places as GeoJSON for the map: each street along its real centreline, each junction a point.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::city::model::CityView;
use crate::city::projection::Projection;
use crate::restriction::model::{LonLat, Restriction};

/// How a place stands: needing attention beats having been changed.
fn status(ok: bool, edited: bool) -> &'static str {
    if !ok {
        "bad"
    } else if edited {
        "changed"
    } else {
        "ok"
    }
}

/// A coordinate to six decimals of a degree: about a tenth of a metre.
fn r6(v: f64) -> f64 {
    (v * 1e6).round() / 1e6
}

fn position(p: &Projection, x_mm: i32, y_mm: i32) -> [f64; 2] {
    let [lon, lat] = p.lon_lat(x_mm, y_mm);
    [r6(lon), r6(lat)]
}

/// The streets and junctions of the city as a GeoJSON feature collection. A street in `conflicts` (by uid) has what
/// the rule of an area it is inside does not allow.
pub fn places(v: &CityView, p: &Projection, conflicts: &BTreeSet<u32>) -> String {
    let mut features: Vec<Value> = Vec::with_capacity(v.edges.len() + v.nodes.len());
    for e in &v.edges {
        features.push(json!({
            "type": "Feature",
            "properties": { "hot": format!("s-{}", e.uid), "status": status(e.ok, e.edited), "width_m": e.row_mm as f64 / 1000.0, "class": e.class.id(), "conflict": conflicts.contains(&e.uid) },
            "geometry": { "type": "LineString", "coordinates": e.shape_mm.iter().map(|q| position(p, q[0], q[1])).collect::<Vec<_>>() },
        }));
    }
    for n in v.nodes.iter().filter(|n| n.junction) {
        features.push(json!({
            "type": "Feature",
            "properties": { "hot": format!("j-{}", n.uid), "status": status(n.ok, n.edited), "radius_m": n.radius_mm as f64 / 1000.0 },
            "geometry": { "type": "Point", "coordinates": position(p, n.x_mm, n.y_mm) },
        }));
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// How far a zone tick reaches past each edge of the street, in millimetres.
pub const TICK_OVERHANG_MM: i32 = 2_000;

/// The point `d_mm` along a polyline, in layout millimetres, and the unit tangent of the segment it lies on. `None`
/// at the start or at or past the end, or on a segment of no length.
fn point_at(shape: &[[i32; 2]], d_mm: f64) -> Option<([f64; 2], [f64; 2])> {
    if d_mm <= 0.0 {
        return None;
    }
    let mut left = d_mm;
    for w in shape.windows(2) {
        let (dx, dy) = ((w[1][0] - w[0][0]) as f64, (w[1][1] - w[0][1]) as f64);
        let len = dx.hypot(dy);
        if left < len {
            let t = [dx / len, dy / len];
            return Some(([w[0][0] as f64 + t[0] * left, w[0][1] as f64 + t[1] * left], t));
        }
        left -= len;
    }
    None
}

/// A short bar across each street at each of its zone boundaries, as GeoJSON: centred on the centreline, along the
/// normal, `TICK_OVERHANG_MM` past each edge. Each says its street (`s-<uid>`) and which boundary it is (`n`, from 1).
pub fn zone_ticks(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::new();
    for e in &v.edges {
        let half = e.row_mm as f64 / 2.0 + TICK_OVERHANG_MM as f64;
        for (i, &d) in e.zone_breaks_mm.iter().enumerate() {
            let Some(([x, y], [tx, ty])) = point_at(&e.shape_mm, d as f64) else { continue };
            let end = |side: f64| position(p, (x - ty * half * side).round() as i32, (y + tx * half * side).round() as i32);
            features.push(json!({
                "type": "Feature",
                "properties": { "street": format!("s-{}", e.uid), "n": i + 1 },
                "geometry": { "type": "LineString", "coordinates": [end(1.0), end(-1.0)] },
            }));
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// A ring of corners as GeoJSON wants a polygon's: closed, the first corner again at the end.
fn closed(ring: &[LonLat]) -> Vec<[f64; 2]> {
    let mut c: Vec<[f64; 2]> = ring.iter().map(|p| [r6(p[0]), r6(p[1])]).collect();
    if let Some(&first) = c.first() {
        c.push(first);
    }
    c
}

/// The restrictions as GeoJSON: each a polygon with its id and rule, the selected one marked and its corners as
/// points to drag. `moving` is the selected one's outline while a corner of it is dragged.
pub fn areas(list: &[Restriction], selected: Option<&str>, moving: Option<&[LonLat]>) -> String {
    let mut features: Vec<Value> = Vec::new();
    for r in list {
        let on = selected == Some(r.id.as_str());
        let ring = if on { moving.unwrap_or(&r.ring) } else { &r.ring };
        features.push(json!({
            "type": "Feature",
            "properties": { "area": r.id, "rule": r.rule.id(), "selected": on },
            "geometry": { "type": "Polygon", "coordinates": [closed(ring)] },
        }));
        if on {
            for (i, c) in ring.iter().enumerate() {
                features.push(json!({
                    "type": "Feature",
                    "properties": { "area": r.id, "i": i },
                    "geometry": { "type": "Point", "coordinates": [r6(c[0]), r6(c[1])] },
                }));
            }
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// The outline being drawn: the area it would close once it has three corners, the path so far, and each corner,
/// the first marked (pressing it closes the outline).
pub fn draft(corners: &[LonLat]) -> String {
    let at = |c: &LonLat| [r6(c[0]), r6(c[1])];
    let mut features: Vec<Value> = Vec::new();
    if corners.len() >= 3 {
        features.push(json!({ "type": "Feature", "properties": { "part": "fill" }, "geometry": { "type": "Polygon", "coordinates": [closed(corners)] } }));
    }
    if corners.len() >= 2 {
        features.push(json!({ "type": "Feature", "properties": { "part": "path" }, "geometry": { "type": "LineString", "coordinates": corners.iter().map(at).collect::<Vec<_>>() } }));
    }
    for (i, c) in corners.iter().enumerate() {
        features.push(json!({ "type": "Feature", "properties": { "i": i, "first": i == 0 }, "geometry": { "type": "Point", "coordinates": at(c) } }));
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// West, south, east and north of everything the places draw, in degrees.
pub fn bounds(v: &CityView, p: &Projection) -> Option<[f64; 4]> {
    let streets = v.edges.iter().flat_map(|e| e.shape_mm.iter().map(|q| (q[0], q[1])));
    let junctions = v.nodes.iter().filter(|n| n.junction).map(|n| (n.x_mm, n.y_mm));
    streets.chain(junctions).map(|(x, y)| p.lon_lat(x, y)).fold(None, |b, [lon, lat]| match b {
        None => Some([lon, lat, lon, lat]),
        Some([w, s, e, n]) => Some([w.min(lon), s.min(lat), e.max(lon), n.max(lat)]),
    })
}

/// The `hot` id of each place and the box of what the map draws of it (west, south, east, north, in degrees): a
/// street's drawn shape, a junction's point. The same order as `places`.
pub fn place_boxes(v: &CityView, p: &Projection) -> Vec<(String, [f64; 4])> {
    let mut boxes = Vec::with_capacity(v.edges.len() + v.nodes.len());
    for e in &v.edges {
        let mut shape = e.shape_mm.iter().map(|q| position(p, q[0], q[1]));
        let Some([lon, lat]) = shape.next() else { continue };
        let b = shape.fold([lon, lat, lon, lat], |[w, s, east, n], [lon, lat]| [w.min(lon), s.min(lat), east.max(lon), n.max(lat)]);
        boxes.push((format!("s-{}", e.uid), b));
    }
    for n in v.nodes.iter().filter(|n| n.junction) {
        let [lon, lat] = position(p, n.x_mm, n.y_mm);
        boxes.push((format!("j-{}", n.uid), [lon, lat, lon, lat]));
    }
    boxes
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use osm_network::{Control, Lane, LaneKind, Network, Node, Road, Way};
    use serde_json::Value;

    use super::*;
    use crate::city::model::City;
    use crate::city::projection::Projection;
    use crate::place::area::Area;
    use crate::restriction::model::{Restriction, Rule};

    /// One street that bends: east for 80 m, then north-east to a point 100 m east and 50 m north.
    fn bent_road() -> Network {
        let node = |id, x_m, y_m| Node { id, osm_nodes: vec![id as i64], osm_versions: Default::default(), x_m, y_m, junction: false, control: Control::None };
        let lane = |way| Lane { kind: LaneKind::Driving, way, width_m: 3.0, buffer_type: None, hours: None };
        Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
                osm_versions: Default::default(),
                name: Some("Bend Street".into()),
                highway: "residential".into(),
                from: 1,
                to: 2,
                lanes: vec![lane(Way::Forward), lane(Way::Backward)],
                points: vec![(0.0, 0.0), (80.0, 0.0), (100.0, 50.0)],
            }],
        }
    }

    fn projection(v: &crate::city::model::CityView) -> Projection {
        Projection::new(Area::new("Bendville", 45.5, -73.6).bounds(), v.origin_m.unwrap())
    }

    fn parse(json: &str) -> Value {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_street_is_a_line_along_its_centreline_with_the_place_it_opens_and_how_it_stands() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1, "one street and no junction");
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "LineString");
        assert_eq!(f["properties"]["hot"], "s-1");
        assert_eq!(f["properties"]["status"], "ok");
        assert_eq!(f["properties"]["width_m"].as_f64().unwrap(), v.edges[0].row_mm as f64 / 1000.0);
        let line = f["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(line.len(), 3, "the bend is kept");
        let round = |x: f64| (x * 1e6).round() / 1e6;
        let first = p.lon_lat(v.edges[0].shape_mm[0][0], v.edges[0].shape_mm[0][1]);
        assert_eq!(line[0], serde_json::json!([round(first[0]), round(first[1])]));
        // the middle point is off the chord between the ends: the street is not drawn straight
        let (a, m, b) = (&line[0], &line[1], &line[2]);
        let lerp = a[1].as_f64().unwrap()
            + (b[1].as_f64().unwrap() - a[1].as_f64().unwrap()) * (m[0].as_f64().unwrap() - a[0].as_f64().unwrap())
                / (b[0].as_f64().unwrap() - a[0].as_f64().unwrap());
        assert!((m[1].as_f64().unwrap() - lerp).abs() > 1e-5, "{m} on the chord from {a} to {b}");
    }

    #[test]
    fn a_street_says_its_class_so_that_the_map_can_tell_a_pedestrian_street() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let class =
            |v: &crate::city::model::CityView| parse(&places(v, &p, &BTreeSet::new()))["features"][0]["properties"]["class"].as_str().unwrap().to_string();
        assert_eq!(class(&v), "local");
        v.edges[0].class = crate::shared::catalogue::StreetClass::Pedestrian;
        assert_eq!(class(&v), "pedestrian");
    }

    #[test]
    fn a_street_that_fails_or_has_been_changed_says_so() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let status =
            |v: &crate::city::model::CityView| parse(&places(v, &p, &BTreeSet::new()))["features"][0]["properties"]["status"].as_str().unwrap().to_string();
        assert_eq!(status(&v), "ok");
        v.edges[0].edited = true;
        assert_eq!(status(&v), "changed");
        v.edges[0].ok = false;
        assert_eq!(status(&v), "bad", "failing wins over changed");
    }

    #[test]
    fn a_junction_is_a_point_with_its_radius_and_a_street_end_is_not_drawn() {
        let v = City::new().view(0); // the sample city has junctions and gates
        let p = Projection::new(Area::new("Sample", 45.5, -73.6).bounds(), [0.0, 0.0]);
        let fc = parse(&places(&v, &p, &BTreeSet::new()));
        let points: Vec<&Value> = fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "Point").collect();
        assert_eq!(points.len(), v.nodes.iter().filter(|n| n.junction).count());
        assert!(points.iter().all(|f| f["properties"]["hot"].as_str().unwrap().starts_with("j-") && f["properties"]["radius_m"].as_f64().unwrap() > 0.0));
        let streets = fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "LineString").count();
        assert_eq!(streets, v.edges.len());
    }

    #[test]
    fn each_place_has_the_box_of_its_drawn_shape_and_a_junction_a_point() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let boxes = place_boxes(&v, &p);
        assert_eq!(boxes.len(), 1, "one street and no junction");
        let line = parse(&places(&v, &p, &BTreeSet::new()))["features"][0]["geometry"]["coordinates"].clone();
        let (lons, lats): (Vec<f64>, Vec<f64>) = line.as_array().unwrap().iter().map(|c| (c[0].as_f64().unwrap(), c[1].as_f64().unwrap())).unzip();
        let min = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(boxes[0], ("s-1".to_string(), [min(&lons), min(&lats), max(&lons), max(&lats)]));

        let city = City::new().view(0); // the sample city has junctions
        let pc = Projection::new(Area::new("Sample", 45.5, -73.6).bounds(), [0.0, 0.0]);
        let boxes = place_boxes(&city, &pc);
        let junctions: Vec<_> = boxes.iter().filter(|(h, _)| h.starts_with("j-")).collect();
        assert_eq!(junctions.len(), city.nodes.iter().filter(|n| n.junction).count());
        assert!(junctions.iter().all(|(_, b)| b[0] == b[2] && b[1] == b[3]), "a junction is a point");
        assert_eq!(boxes.len(), city.edges.len() + junctions.len());
    }

    #[test]
    fn the_bounds_hold_every_point_of_every_street() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let [w, s, e, n] = bounds(&v, &p).unwrap();
        for c in parse(&places(&v, &p, &BTreeSet::new()))["features"][0]["geometry"]["coordinates"].as_array().unwrap() {
            let (lon, lat) = (c[0].as_f64().unwrap(), c[1].as_f64().unwrap());
            assert!(w - 1e-6 <= lon && lon <= e + 1e-6 && s - 1e-6 <= lat && lat <= n + 1e-6, "{c} outside {w} {s} {e} {n}");
        }
        assert!(w < e && s < n);
    }

    #[test]
    fn a_street_in_conflict_with_an_area_says_so() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let conflict = |set: BTreeSet<u32>| parse(&places(&v, &p, &set))["features"][0]["properties"]["conflict"].as_bool().unwrap();
        assert!(!conflict(BTreeSet::new()));
        assert!(conflict([1].into()));
    }

    fn area(id: &str, rule: Rule) -> Restriction {
        Restriction { id: id.into(), name: String::new(), rule, ring: vec![[-73.6, 45.5], [-73.59, 45.5], [-73.59, 45.51]] }
    }

    #[test]
    fn an_area_is_a_closed_polygon_with_its_id_and_rule_and_the_selected_one_has_its_corners() {
        let list = [area("r1", Rule::CarFree), area("r2", Rule::SpeedLimit { kmh: 30 })];
        let fc = parse(&areas(&list, Some("r2"), None));
        let f = fc["features"].as_array().unwrap();
        assert_eq!(f.len(), 2 + 3, "two outlines and the selected one's three corners");
        assert_eq!(f[0]["geometry"]["type"], "Polygon");
        assert_eq!(f[0]["properties"]["area"], "r1");
        assert_eq!(f[0]["properties"]["rule"], "car_free");
        assert_eq!(f[0]["properties"]["selected"], false);
        let ring = f[0]["geometry"]["coordinates"][0].as_array().unwrap();
        assert_eq!(ring.len(), 4, "closed: the first corner again");
        assert_eq!(ring[0], ring[3]);
        assert_eq!(f[1]["properties"]["selected"], true);
        assert_eq!(f[2]["geometry"]["type"], "Point");
        assert_eq!((f[2]["properties"]["area"].as_str(), f[2]["properties"]["i"].as_u64()), (Some("r2"), Some(0)));
    }

    #[test]
    fn the_selected_area_is_drawn_where_its_corner_is_being_dragged() {
        let list = [area("r1", Rule::CarFree)];
        let moved = [[-73.61, 45.5], [-73.59, 45.5], [-73.59, 45.51]];
        let fc = parse(&areas(&list, Some("r1"), Some(&moved)));
        assert_eq!(fc["features"][0]["geometry"]["coordinates"][0][0], serde_json::json!([-73.61, 45.5]));
        assert_eq!(fc["features"][1]["geometry"]["coordinates"], serde_json::json!([-73.61, 45.5]));
        let fc = parse(&areas(&list, None, Some(&moved)));
        assert_eq!(fc["features"][0]["geometry"]["coordinates"][0][0], serde_json::json!([-73.6, 45.5]), "only the selected one moves");
    }

    #[test]
    fn the_outline_being_drawn_is_its_corners_the_path_and_once_it_can_close_its_area() {
        assert!(parse(&draft(&[]))["features"].as_array().unwrap().is_empty());
        let one = parse(&draft(&[[-73.6, 45.5]]));
        assert_eq!(one["features"].as_array().unwrap().len(), 1);
        assert_eq!(one["features"][0]["properties"]["first"], true);
        let three = parse(&draft(&[[-73.6, 45.5], [-73.59, 45.5], [-73.59, 45.51]]));
        let parts: Vec<String> = three["features"].as_array().unwrap().iter().map(|f| f["geometry"]["type"].as_str().unwrap().to_string()).collect();
        assert_eq!(parts, ["Polygon", "LineString", "Point", "Point", "Point"]);
        assert_eq!(three["features"][0]["properties"]["part"], "fill");
        assert_eq!(three["features"][1]["properties"]["part"], "path");
        assert_eq!(three["features"][4]["properties"]["i"], 2);
    }

    /// One straight street, east for 100 m, 8 m wide.
    fn straight_road() -> Network {
        let mut n = bent_road();
        n.nodes[1].x_m = 100.0;
        n.nodes[1].y_m = 0.0;
        n.roads[0].points = vec![(0.0, 0.0), (100.0, 0.0)];
        n.roads[0].lanes.iter_mut().for_each(|l| l.width_m = 4.0);
        n
    }

    #[test]
    fn a_point_along_a_polyline_has_the_tangent_of_its_segment() {
        let shape = [[0, 0], [10_000, 0], [10_000, 10_000]];
        assert_eq!(point_at(&shape, 15_000.0), Some(([10_000.0, 5_000.0], [0.0, 1.0])));
        assert_eq!(point_at(&shape, 4_000.0), Some(([4_000.0, 0.0], [1.0, 0.0])));
        assert_eq!(point_at(&shape, 0.0), None);
        assert_eq!(point_at(&shape, 20_000.0), None);
        assert_eq!(point_at(&shape, 25_000.0), None);
        assert_eq!(point_at(&[[0, 0], [0, 0]], 1.0), None);
    }

    #[test]
    fn a_boundary_is_a_tick_across_the_street_two_metres_past_each_edge() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].zone_breaks_mm = vec![30_000];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1);
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "LineString");
        assert_eq!(f["properties"]["street"], format!("s-{}", v.edges[0].uid));
        assert_eq!(f["properties"]["n"], 1);
        let line = f["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(line.len(), 2);
        let c = |i: usize, j: usize| line[i][j].as_f64().unwrap();
        assert_eq!(c(0, 0), c(1, 0), "the tick runs north-south across an east-west street");
        let across_mm = v.edges[0].row_mm + 2 * TICK_OVERHANG_MM;
        let metres = (c(0, 1) - c(1, 1)).abs() * 111_195.0;
        assert!((metres - across_mm as f64 / 1000.0).abs() < 0.2, "{metres} m for {across_mm} mm");
        // centred on the centreline at 30 m
        let [x0, y0] = v.edges[0].shape_mm[0];
        let mid = position(&p, x0 + 30_000, y0);
        assert!((c(0, 0) - mid[0]).abs() < 2e-6 && ((c(0, 1) + c(1, 1)) / 2.0 - mid[1]).abs() < 2e-6);
    }

    #[test]
    fn a_street_without_boundaries_adds_nothing_and_places_is_unchanged() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let fc = parse(&zone_ticks(&v, &p));
        assert_eq!(fc["type"], "FeatureCollection");
        assert!(fc["features"].as_array().unwrap().is_empty());
        assert!(!places(&v, &p, &BTreeSet::new()).contains("zone"));
    }
}
