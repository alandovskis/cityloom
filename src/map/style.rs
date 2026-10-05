//! The layers the places are drawn with on the map. Widths are in metres on the ground, so the layers
//! depend on the latitude of the place; the colours are the page's own.

use serde_json::{Value, json};

/// The name of the GeoJSON source the layers draw.
pub const SOURCE: &str = "places";

/// What the colours say: a place that works, one that has been changed, one that needs attention.
pub const OK: &str = "#4b6b8a";
pub const CHANGED: &str = "#2563eb";
pub const BAD: &str = "#dc2626";
/// The place the pointer or the focus is on.
const HOT: &str = "#f59e0b";

/// MapLibre's zoom 0 shows the earth on one 512-pixel tile: this many pixels to a metre there, at this
/// latitude. A width in metres is that times two to the zoom.
fn pixels_per_metre_at_zoom_0(lat: f64) -> f64 {
    512.0 / (40_075_016.686 * lat.to_radians().cos())
}

/// A property in metres, drawn at least `min_px` wide (plus `extra_px`), growing with the zoom as the
/// ground does. MapLibre allows `zoom` only as the input of a top-level `interpolate`, so the metres to
/// pixels scale is taken at the two ends of the zoom range and the base-2 interpolation in between is exact
/// (apart from the floor, which the interpolation blends: at most `min_px` too wide at low zooms).
fn metres(property: &str, lat: f64, min_px: f64, extra_px: f64) -> Value {
    let k = pixels_per_metre_at_zoom_0(lat);
    let at = |scale: f64| json!(["+", ["max", min_px, ["*", ["get", property], k * scale]], extra_px]);
    json!(["interpolate", ["exponential", 2], ["zoom"], 0, at(1.0), 24, at(2f64.powi(24))])
}

/// The colour of a place: the highlight where the pointer is, else the one its status says.
fn colour() -> Value {
    json!(["case", ["boolean", ["feature-state", "hot"], false], HOT, ["match", ["get", "status"], "bad", BAD, "changed", CHANGED, OK]])
}

/// The layers, as JSON, for a place at this latitude.
pub fn layers(lat: f64) -> String {
    json!([
        {
            "id": "places-casing", "type": "line", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "LineString"],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": "#ffffff", "line-opacity": 0.9, "line-width": metres("width_m", lat, 3.0, 3.0) }
        },
        {
            "id": "places-street", "type": "line", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "LineString"],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": colour(), "line-width": metres("width_m", lat, 2.5, 0.0) }
        },
        {
            "id": "places-junction", "type": "circle", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "Point"],
            "paint": { "circle-color": colour(), "circle-radius": metres("radius_m", lat, 5.0, 0.0), "circle-stroke-color": "#ffffff", "circle-stroke-width": 2 }
        }
    ])
    .to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn layers(lat: f64) -> Vec<Value> {
        serde_json::from_str(&super::layers(lat)).unwrap()
    }

    #[test]
    fn the_places_are_drawn_in_three_layers_on_one_source() {
        let l = layers(45.5);
        let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["places-casing", "places-street", "places-junction"]);
        assert!(l.iter().all(|l| l["source"] == SOURCE));
        assert_eq!(SOURCE, "places");
    }

    #[test]
    fn the_colour_follows_the_status_and_the_highlight() {
        let street = &layers(45.5)[1];
        let colour = street["paint"]["line-color"].to_string();
        assert!(colour.contains("feature-state") && colour.contains("\"status\""));
        for c in [OK, CHANGED, BAD] {
            assert!(colour.contains(c), "{c} in {colour}");
        }
    }

    #[test]
    fn a_width_in_metres_doubles_with_each_zoom_and_is_a_pixel_scale_at_the_latitude() {
        let street = &layers(0.0)[1];
        let width = &street["paint"]["line-width"];
        let text = width.to_string();
        assert!(text.contains("\"width_m\"") && text.contains("\"exponential\""), "{text}");
        // pull the two stops of the zoom interpolation out of the expression
        fn stops(v: &Value, out: &mut Vec<(f64, f64)>) {
            if let Some(a) = v.as_array() {
                if a.first().is_some_and(|f| f == "interpolate") {
                    let rest = &a[3..];
                    for pair in rest.chunks(2) {
                        // the output is ["+", ["max", min_px, ["*", ["get", prop], k]], extra_px]
                        out.push((pair[0].as_f64().unwrap(), pair[1][1][2][2].as_f64().unwrap()));
                    }
                }
                a.iter().for_each(|x| stops(x, out));
            }
        }
        let mut s = Vec::new();
        stops(width, &mut s);
        assert_eq!(s.len(), 2, "{text}");
        assert_eq!(s[0].0, 0.0);
        assert_eq!(s[1].0, 24.0);
        assert_eq!(s[1].1 / s[0].1, 2f64.powi(24), "doubling with each zoom");
        // at the equator, at zoom 17, a metre is about 1.675 pixels on 512-pixel tiles
        let at_17 = s[0].1 * 2f64.powi(17);
        assert!((at_17 - 1.675).abs() < 0.01, "{at_17}");
        // and fewer pixels to the metre away from the equator... no: more, because a degree is shorter
        assert!(layers(60.0)[1]["paint"]["line-width"].to_string() != text);
    }

    #[test]
    fn a_junction_is_a_circle_with_a_radius_in_metres_and_a_minimum() {
        let junction = &layers(45.5)[2];
        assert_eq!(junction["type"], "circle");
        let r = junction["paint"]["circle-radius"].to_string();
        assert!(r.contains("\"radius_m\"") && r.contains("\"max\""), "{r}");
    }
}
