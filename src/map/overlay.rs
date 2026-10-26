//! The city's places as GeoJSON for the map: each street along its real centreline, each junction a point.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::city::model::CityView;
use crate::city::projection::Projection;
use crate::restriction::model::{LonLat, Restriction};
use crate::shared::catalogue::Side;

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

/// The part of a polyline between `from_mm` and `to_mm` along it, in layout millimetres: the point at each end and the
/// polyline's own vertices between. Empty when it has no length or the span is empty.
fn stretch(shape: &[[i32; 2]], from_mm: f64, to_mm: f64) -> Vec<[f64; 2]> {
    let mut points: Vec<[f64; 2]> = Vec::new();
    let mut start = 0.0;
    for w in shape.windows(2) {
        let (dx, dy) = ((w[1][0] - w[0][0]) as f64, (w[1][1] - w[0][1]) as f64);
        let len = dx.hypot(dy);
        let end = start + len;
        if len > 0.0 && end > from_mm && start < to_mm {
            let at = |d: f64| {
                let k = (d - start) / len;
                [w[0][0] as f64 + dx * k, w[0][1] as f64 + dy * k]
            };
            if points.is_empty() {
                points.push(at(from_mm.max(start)));
            }
            points.push(at(to_mm.min(end)));
        }
        start = end;
    }
    points
}

/// What marks each of a street's zone boundaries, as GeoJSON. A boundary with a transition (its number in
/// `zone_bands_mm`) is a band: a line along the centreline over the transition, `band: true`. Any other boundary is a
/// short bar across the street, centred on the centreline, along the normal, `TICK_OVERHANG_MM` past each edge. Each
/// says its street (`s-<uid>`) and which boundary it is (`n`, from 1).
pub fn zone_ticks(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::new();
    for e in &v.edges {
        let half = e.row_mm as f64 / 2.0 + TICK_OVERHANG_MM as f64;
        let street = format!("s-{}", e.uid);
        for (i, &d) in e.zone_breaks_mm.iter().enumerate() {
            if let Some(&(_, from, to)) = e.zone_bands_mm.iter().find(|b| b.0 == i + 1) {
                let line = stretch(&e.shape_mm, from as f64, to as f64);
                if line.len() >= 2 {
                    let coordinates: Vec<[f64; 2]> = line.iter().map(|q| position(p, q[0].round() as i32, q[1].round() as i32)).collect();
                    features.push(json!({
                        "type": "Feature",
                        "properties": { "street": street, "n": i + 1, "band": true },
                        "geometry": { "type": "LineString", "coordinates": coordinates },
                    }));
                    continue;
                }
            }
            let Some(([x, y], [tx, ty])) = point_at(&e.shape_mm, d as f64) else { continue };
            let end = |side: f64| position(p, (x - ty * half * side).round() as i32, (y + tx * half * side).round() as i32);
            features.push(json!({
                "type": "Feature",
                "properties": { "street": street, "n": i + 1 },
                "geometry": { "type": "LineString", "coordinates": [end(1.0), end(-1.0)] },
            }));
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// The offset of each vertex of `line` from itself, `across_mm` to the right of the direction of travel, in layout
/// millimetres. A vertex between two segments moves along the normalised sum of their right normals, scaled by
/// `1 / cos(half the turn)` to keep the offset line `across_mm` from both, but by no more than 2, so a sharp turn
/// does not throw a spike.
fn offset_line(line: &[[f64; 2]], across_mm: f64) -> Vec<[f64; 2]> {
    let normal = |a: [f64; 2], b: [f64; 2]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy);
        [-dy / len, dx / len]
    };
    line.iter()
        .enumerate()
        .map(|(i, &q)| {
            let before = (i > 0).then(|| normal(line[i - 1], q));
            let after = line.get(i + 1).map(|&next| normal(q, next));
            let (n, scale) = match (before, after) {
                (Some(a), Some(b)) => {
                    let sum = [a[0] + b[0], a[1] + b[1]];
                    let len = sum[0].hypot(sum[1]);
                    if len < 1e-9 {
                        (a, 1.0)
                    } else {
                        let m = [sum[0] / len, sum[1] / len];
                        (m, (1.0 / (m[0] * a[0] + m[1] * a[1])).min(2.0))
                    }
                }
                (Some(n), None) | (None, Some(n)) => (n, 1.0),
                (None, None) => ([0.0, 0.0], 1.0),
            };
            [q[0] + n[0] * across_mm * scale, q[1] + n[1] * across_mm * scale]
        })
        .collect()
}

/// The ring of the strip beside a polyline, between `from_mm` and `to_mm` along it (as `stretch` measures) and
/// `lo_mm` to `hi_mm` across it (positive to the right looking along the shape), in layout millimetres. Closed, and
/// wound anticlockwise on the map (clockwise in layout millimetres, where y runs south). Empty when the span has no
/// length.
fn offset_ring(shape: &[[i32; 2]], from_mm: f64, to_mm: f64, lo_mm: f64, hi_mm: f64) -> Vec<[f64; 2]> {
    let line = stretch(shape, from_mm, to_mm);
    if line.len() < 2 {
        return Vec::new();
    }
    let (lo, hi) = (offset_line(&line, lo_mm), offset_line(&line, hi_mm));
    let mut ring: Vec<[f64; 2]> = vec![lo[0]];
    ring.extend(hi.iter().copied());
    ring.extend(lo.iter().rev().take(lo.len() - 1).copied());
    ring.push(lo[0]);
    ring
}

fn ring_coordinates(p: &Projection, ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    ring.iter().map(|q| position(p, q[0].round() as i32, q[1].round() as i32)).collect()
}

fn side_id(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
    }
}

/// The curbside uses of every street as a FeatureCollection of polygons beside the street, each saying its street
/// (`hot`, `s-<uid>`), the use (`use`, its snake-case id) and the `side`.
pub fn curbside(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::new();
    for e in &v.edges {
        for u in &e.curb_mm {
            let ring = offset_ring(&e.shape_mm, u.from_mm as f64, u.to_mm as f64, u.lo_mm as f64, u.hi_mm as f64);
            if ring.is_empty() {
                continue;
            }
            features.push(json!({
                "type": "Feature",
                "properties": { "hot": format!("s-{}", e.uid), "use": u.use_, "side": side_id(u.side) },
                "geometry": { "type": "Polygon", "coordinates": [ring_coordinates(p, &ring)] },
            }));
        }
    }
    json!({ "type": "FeatureCollection", "features": features }).to_string()
}

/// The transit stops of every street: for each, its platform as a polygon (`part: "platform"`) and a point (`part:
/// "dot"`) in the middle of its length on the platform's outer edge, with its street (`hot`), `kind` and `side`, and
/// its `name` when it has one. Nothing here depends on the language.
pub fn stops(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::new();
    for e in &v.edges {
        for s in &e.stops_mm {
            let ring = offset_ring(&e.shape_mm, s.from_mm as f64, s.to_mm as f64, s.lo_mm as f64, s.hi_mm as f64);
            let outer = if s.side == Side::Right { s.hi_mm } else { s.lo_mm } as f64;
            let Some(([x, y], [tx, ty])) = point_at(&e.shape_mm, (s.from_mm as f64 + s.to_mm as f64) / 2.0) else { continue };
            if ring.is_empty() {
                continue;
            }
            let mut platform = json!({ "part": "platform", "hot": format!("s-{}", e.uid), "kind": s.kind, "side": side_id(s.side) });
            features.push(json!({
                "type": "Feature",
                "properties": platform.clone(),
                "geometry": { "type": "Polygon", "coordinates": [ring_coordinates(p, &ring)] },
            }));
            platform["part"] = json!("dot");
            let mut properties = platform;
            if let Some(name) = &s.name {
                properties["name"] = json!(name);
            }
            features.push(json!({
                "type": "Feature",
                "properties": properties,
                "geometry": { "type": "Point", "coordinates": position(p, (x - ty * outer).round() as i32, (y + tx * outer).round() as i32) },
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
                speed_kmh: None,
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
    fn a_band_runs_along_the_street_instead_of_across() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let shape = v.edges[0].shape_mm.clone();
        assert_eq!(shape.len(), 3);
        v.edges[0].zone_breaks_mm = vec![90_000];
        v.edges[0].zone_bands_mm = vec![(1, 70_000, 90_000)];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1, "the band replaces the tick");
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "LineString");
        assert_eq!(f["properties"]["band"], true);
        assert_eq!(f["properties"]["n"], 1);
        assert_eq!(f["properties"]["street"], format!("s-{}", v.edges[0].uid));
        let line = f["geometry"]["coordinates"].as_array().unwrap();
        let at = |x: i32, y: i32| position(&p, x, y);
        let (x0, y0) = (shape[0][0], shape[0][1]);
        assert_eq!(line.len(), 3, "its two ends and the bend between");
        assert_eq!(line[0], serde_json::json!(at(x0 + 70_000, y0)));
        assert_eq!(line[1], serde_json::json!(at(shape[1][0], shape[1][1])));
        let (dx, dy) = ((shape[2][0] - shape[1][0]) as f64, (shape[2][1] - shape[1][1]) as f64);
        let k = 10_000.0 / dx.hypot(dy);
        let end = at((shape[1][0] as f64 + dx * k).round() as i32, (shape[1][1] as f64 + dy * k).round() as i32);
        let got = &line[2];
        assert!((got[0].as_f64().unwrap() - end[0]).abs() < 2e-6 && (got[1].as_f64().unwrap() - end[1]).abs() < 2e-6);
    }

    #[test]
    fn a_boundary_without_a_transition_keeps_its_tick() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].zone_breaks_mm = vec![20_000, 60_000];
        v.edges[0].zone_bands_mm = vec![(2, 40_000, 60_000)];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert!(features[0]["properties"].get("band").is_none());
        assert_eq!(features[0]["properties"]["n"], 1);
        assert_eq!(features[0]["geometry"]["coordinates"].as_array().unwrap().len(), 2);
        assert_eq!((features[1]["properties"]["n"].clone(), features[1]["properties"]["band"].clone()), (2.into(), true.into()));
    }

    #[test]
    fn a_clipped_band_does_not_take_the_previous_boundary() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].zone_breaks_mm = vec![40_000, 60_000];
        v.edges[0].zone_bands_mm = vec![(2, 40_000, 60_000)];
        let fc = parse(&zone_ticks(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!((features[0]["properties"]["n"].clone(), features[0]["properties"].get("band").is_none()), (1.into(), true));
        assert_eq!((features[1]["properties"]["n"].clone(), features[1]["properties"]["band"].clone()), (2.into(), true.into()));
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

    use crate::street::model::{MapStop, MapUse, StopKind, Use};

    fn a_use(side: Side, from_mm: i32, to_mm: i32, lo_mm: i32, hi_mm: i32) -> MapUse {
        MapUse { side, use_: Use::Parking, from_mm, to_mm, lo_mm, hi_mm }
    }

    fn a_stop(name: Option<&str>) -> MapStop {
        MapStop { side: Side::Right, kind: StopKind::Bulb, name: name.map(String::from), from_mm: 20_000, to_mm: 32_000, lo_mm: 3_500, hi_mm: 6_000 }
    }

    fn ring_of(f: &Value) -> Vec<(f64, f64)> {
        f["geometry"]["coordinates"][0].as_array().unwrap().iter().map(|c| (c[0].as_f64().unwrap(), c[1].as_f64().unwrap())).collect()
    }

    fn signed_area(ring: &[(f64, f64)]) -> f64 {
        ring.windows(2).map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1).sum::<f64>() / 2.0
    }

    #[test]
    fn a_use_is_a_polygon_beside_the_street() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].curb_mm = vec![a_use(Side::Right, 10_000, 16_000, 3_500, 6_000)];
        let fc = parse(&curbside(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 1);
        let f = &features[0];
        assert_eq!(f["geometry"]["type"], "Polygon");
        assert_eq!(f["properties"]["hot"], format!("s-{}", v.edges[0].uid));
        assert_eq!(f["properties"]["use"], "parking");
        assert_eq!(f["properties"]["side"], "right");
        let ring = ring_of(f);
        assert_eq!(ring.len(), 5, "four corners, closed");
        assert_eq!(ring[0], ring[4]);
        let [x0, y0] = v.edges[0].shape_mm[0];
        let corner = |along: i32, across: i32| {
            let [lon, lat] = p.lon_lat(x0 + along, y0 + across);
            (lon, lat)
        };
        for (got, want) in ring[..4].iter().zip([corner(10_000, 3_500), corner(10_000, 6_000), corner(16_000, 6_000), corner(16_000, 3_500)]) {
            assert!((got.0 - want.0).abs() < 2e-6 && (got.1 - want.1).abs() < 2e-6, "{got:?} is not {want:?}");
        }
        assert!(ring[1].1 < ring[0].1 || (ring[1].1 - ring[0].1).abs() < 2e-6, "right is south of an eastward street");
    }

    #[test]
    fn left_is_north_of_an_eastward_street() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].curb_mm = vec![a_use(Side::Left, 10_000, 16_000, -6_000, -3_500)];
        let ring = ring_of(&parse(&curbside(&v, &p))["features"][0]);
        let [x0, y0] = v.edges[0].shape_mm[0];
        let line_lat = p.lon_lat(x0, y0)[1];
        assert!(ring.iter().all(|c| c.1 > line_lat), "every corner north of the line");
    }

    #[test]
    fn a_bend_does_not_spike() {
        let shape = [[0, 0], [20_000, 0], [20_000, 20_000]];
        let ring = offset_ring(&shape, 10_000.0, 30_000.0, 3_500.0, 6_000.0);
        let near = |q: [f64; 2]| {
            shape
                .windows(2)
                .map(|w| {
                    let (ax, ay, bx, by) = (w[0][0] as f64, w[0][1] as f64, w[1][0] as f64, w[1][1] as f64);
                    let t = (((q[0] - ax) * (bx - ax) + (q[1] - ay) * (by - ay)) / ((bx - ax).powi(2) + (by - ay).powi(2))).clamp(0.0, 1.0);
                    (q[0] - (ax + t * (bx - ax))).hypot(q[1] - (ay + t * (by - ay)))
                })
                .fold(f64::INFINITY, f64::min)
        };
        assert!(ring.iter().all(|&q| near(q) <= 2.0 * 6_000.0 + 1e-6), "a vertex spikes: {ring:?}");
        assert_eq!(ring.first(), ring.last(), "closed");
        // layout y is south; the ring is wound anticlockwise on the map (y north), so clockwise in layout millimetres
        let layout_area: f64 = ring.windows(2).map(|w| w[0][0] * w[1][1] - w[1][0] * w[0][1]).sum::<f64>() / 2.0;
        assert!(layout_area < 0.0, "{layout_area}");
    }

    #[test]
    fn a_use_ring_is_wound_anticlockwise_on_the_map() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].curb_mm = vec![a_use(Side::Right, 10_000, 100_000, 3_500, 6_000)];
        assert!(signed_area(&ring_of(&parse(&curbside(&v, &p))["features"][0])) > 0.0);
    }

    #[test]
    fn a_stop_is_a_platform_and_a_dot_with_its_street() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].stops_mm = vec![a_stop(Some("Hôtel-de-Ville"))];
        let fc = parse(&stops(&v, &p));
        let features = fc["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        let hot = format!("s-{}", v.edges[0].uid);
        let (platform, dot) = (&features[0], &features[1]);
        assert_eq!((platform["geometry"]["type"].as_str(), platform["properties"]["part"].as_str()), (Some("Polygon"), Some("platform")));
        assert_eq!((dot["geometry"]["type"].as_str(), dot["properties"]["part"].as_str()), (Some("Point"), Some("dot")));
        for f in features {
            assert_eq!(f["properties"]["hot"], hot);
            assert_eq!(f["properties"]["kind"], "bulb");
            assert_eq!(f["properties"]["side"], "right");
        }
        assert_eq!(dot["properties"]["name"], "Hôtel-de-Ville");
        let [x0, y0] = v.edges[0].shape_mm[0];
        let [lon, lat] = p.lon_lat(x0 + 26_000, y0 + 6_000);
        let c = dot["geometry"]["coordinates"].as_array().unwrap();
        assert!((c[0].as_f64().unwrap() - lon).abs() < 2e-6 && (c[1].as_f64().unwrap() - lat).abs() < 2e-6, "the middle, on the outer edge");
    }

    #[test]
    fn an_unnamed_stop_has_no_name_property() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].stops_mm = vec![a_stop(None)];
        let fc = parse(&stops(&v, &p));
        assert!(fc["features"][1]["properties"].get("name").is_none());
    }

    #[test]
    fn the_stops_data_does_not_depend_on_the_language() {
        // `stops` takes no `I18n`: the same city gives the same string every time, and holds no wording of ours
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        v.edges[0].stops_mm = vec![a_stop(None)];
        assert_eq!(stops(&v, &p), stops(&v, &p));
    }

    #[test]
    fn a_city_without_uses_has_empty_collections() {
        let v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        assert!(parse(&curbside(&v, &p))["features"].as_array().unwrap().is_empty());
        assert!(parse(&stops(&v, &p))["features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn places_are_unchanged_by_uses() {
        let mut v = City::from_network(&straight_road(), "Bendville").view(0);
        let p = projection(&v);
        let before = places(&v, &p, &BTreeSet::new());
        v.edges[0].curb_mm = vec![a_use(Side::Right, 10_000, 16_000, 3_500, 6_000)];
        v.edges[0].stops_mm = vec![a_stop(None)];
        assert_eq!(places(&v, &p, &BTreeSet::new()), before);
    }
}
