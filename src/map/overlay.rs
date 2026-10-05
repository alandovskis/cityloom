//! The city's places as GeoJSON for the map: each street along its real centreline, each junction a point.

use serde_json::{Value, json};

use crate::city::model::CityView;
use crate::map::projection::Projection;

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

/// The streets and junctions of the city as a GeoJSON feature collection.
pub fn places(v: &CityView, p: &Projection) -> String {
    let mut features: Vec<Value> = Vec::with_capacity(v.edges.len() + v.nodes.len());
    for e in &v.edges {
        features.push(json!({
            "type": "Feature",
            "properties": { "hot": format!("s-{}", e.uid), "status": status(e.ok, e.edited), "width_m": e.row_mm as f64 / 1000.0 },
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

/// West, south, east and north of everything the places draw, in degrees.
pub fn bounds(v: &CityView, p: &Projection) -> Option<[f64; 4]> {
    let streets = v.edges.iter().flat_map(|e| e.shape_mm.iter().map(|q| (q[0], q[1])));
    let junctions = v.nodes.iter().filter(|n| n.junction).map(|n| (n.x_mm, n.y_mm));
    streets.chain(junctions).map(|(x, y)| p.lon_lat(x, y)).fold(None, |b, [lon, lat]| match b {
        None => Some([lon, lat, lon, lat]),
        Some([w, s, e, n]) => Some([w.min(lon), s.min(lat), e.max(lon), n.max(lat)]),
    })
}

#[cfg(test)]
mod tests {
    use osm_network::{Control, Lane, LaneKind, Network, Node, Road, Way};
    use serde_json::Value;

    use super::*;
    use crate::city::model::City;
    use crate::map::projection::Projection;
    use crate::place::area::Area;

    /// One street that bends: east for 80 m, then north-east to a point 100 m east and 50 m north.
    fn bent_road() -> Network {
        let node = |id, x_m, y_m| Node { id, osm_nodes: vec![id as i64], x_m, y_m, junction: false, control: Control::None };
        let lane = |way| Lane { kind: LaneKind::Driving, way, width_m: 3.0 };
        Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 100.0, 50.0)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
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
        let fc = parse(&places(&v, &p));
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
    fn a_street_that_fails_or_has_been_changed_says_so() {
        let mut v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let status = |v: &crate::city::model::CityView| parse(&places(v, &p))["features"][0]["properties"]["status"].as_str().unwrap().to_string();
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
        let fc = parse(&places(&v, &p));
        let points: Vec<&Value> = fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "Point").collect();
        assert_eq!(points.len(), v.nodes.iter().filter(|n| n.junction).count());
        assert!(points.iter().all(|f| f["properties"]["hot"].as_str().unwrap().starts_with("j-") && f["properties"]["radius_m"].as_f64().unwrap() > 0.0));
        let streets = fc["features"].as_array().unwrap().iter().filter(|f| f["geometry"]["type"] == "LineString").count();
        assert_eq!(streets, v.edges.len());
    }

    #[test]
    fn the_bounds_hold_every_point_of_every_street() {
        let v = City::from_network(&bent_road(), "Bendville").view(0);
        let p = projection(&v);
        let [w, s, e, n] = bounds(&v, &p).unwrap();
        for c in parse(&places(&v, &p))["features"][0]["geometry"]["coordinates"].as_array().unwrap() {
            let (lon, lat) = (c[0].as_f64().unwrap(), c[1].as_f64().unwrap());
            assert!(w - 1e-6 <= lon && lon <= e + 1e-6 && s - 1e-6 <= lat && lat <= n + 1e-6, "{c} outside {w} {s} {e} {n}");
        }
        assert!(w < e && s < n);
    }
}
