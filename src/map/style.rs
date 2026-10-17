//! The layers the places are drawn with on the map. Widths are in metres on the ground, so the layers
//! depend on the latitude of the place; the colours are the page's own.

use serde_json::{Value, json};

/// The name of the GeoJSON source the layers draw.
pub const SOURCE: &str = "places";

/// What the colours say: a place that works, one that has been changed, one that needs attention.
pub const OK: &str = "#4b6b8a";
pub const CHANGED: &str = "#2563eb";
pub const BAD: &str = "#dc2626";
/// The mark down a changed street, or in a junction that needs attention: dark, so that it is told from the
/// status colours by its lightness and its shape, not by a hue (WCAG 1.4.1).
const MARK: &str = "#111827";
/// The place the pointer or the focus is on.
const HOT: &str = "#f59e0b";
/// The edge of the place the pointer or the focus is on: dark where the road's own edge is white, so the highlight
/// is not carried by its hue alone.
const HOT_OUTLINE: &str = "#111827";
/// How opaque a place is drawn when it is not editable (not wholly in the open view): a disabled control is exempt
/// from the contrast rule, and the cue is not carried by this alone (the pointer is not a hand over it, and a press
/// says why it opens nothing).
const MUTED: f64 = 0.35;
/// A street that has what the rule of an area it is inside does not allow. `web/app.css`'s `.dot-conflict`
/// repeats it.
pub const CONFLICT: &str = "#db2777";
/// The restrictions' source, and the outline being drawn's.
pub const AREAS: &str = "restrictions";
pub const DRAFT: &str = "draft";
/// The layers `web/basemap.js` asks what the pointer is on: keep these names in step with it.
pub const AREAS_FILL: &str = "areas-fill";
pub const AREAS_VERTEX: &str = "areas-vertex";
pub const DRAFT_VERTEX: &str = "draft-vertex";
/// Each rule (by `Rule::id`): its colour, and how its outline is dashed (none: solid), so that a rule is not told
/// by its colour alone.
const RULES: [(&str, &str, Option<[f64; 4]>); 4] = [
    ("car_free", "#15803d", None),
    ("bus_and_bike_only", "#0e7490", Some([4.0, 2.0, 4.0, 2.0])),
    ("speed_limit", "#b45309", Some([1.0, 2.0, 1.0, 2.0])),
    ("no_parking_or_loading", "#6d28d9", Some([4.0, 2.0, 1.0, 2.0])),
];

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

/// The colour of a place: the highlight where the pointer is, else red where it needs attention, else the
/// conflict tint where it breaks an area's rule, else the one its status says.
fn colour() -> Value {
    json!([
        "case",
        ["boolean", ["feature-state", "hot"], false],
        HOT,
        ["==", ["get", "status"], "bad"],
        BAD,
        ["boolean", ["get", "conflict"], false],
        CONFLICT,
        ["match", ["get", "status"], "changed", CHANGED, OK]
    ])
}

/// `white`, or the dark outline where the place is the highlighted one.
fn edge() -> Value {
    json!(["case", ["boolean", ["feature-state", "hot"], false], HOT_OUTLINE, "#ffffff"])
}

/// `full` where the place is editable (also before the map has been told which are), else `MUTED`.
fn opacity(full: f64) -> Value {
    json!(["case", ["boolean", ["feature-state", "editable"], true], full, MUTED])
}

/// The layers, as JSON, for a place at this latitude.
pub fn layers(lat: f64) -> String {
    json!([
        {
            "id": "places-casing", "type": "line", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "LineString"],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": edge(), "line-opacity": opacity(0.9), "line-width": metres("width_m", lat, 3.0, 3.0) }
        },
        {
            "id": "places-street", "type": "line", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "LineString"],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": colour(), "line-opacity": opacity(1.0), "line-width": metres("width_m", lat, 2.5, 0.0) }
        },
        {
            "id": "places-pedestrian", "type": "line", "source": SOURCE,
            "filter": ["all", ["==", ["geometry-type"], "LineString"], ["==", ["get", "class"], "pedestrian"]],
            "layout": { "line-cap": "round", "line-join": "round" },
            "paint": { "line-color": "#ffffff", "line-opacity": opacity(1.0), "line-width": 2, "line-dasharray": [0.1, 2.5] }
        },
        {
            "id": "places-conflict", "type": "line", "source": SOURCE,
            "filter": ["all", ["==", ["geometry-type"], "LineString"], ["boolean", ["get", "conflict"], false]],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": "#ffffff", "line-opacity": opacity(1.0), "line-width": 2, "line-dasharray": [1, 2] }
        },
        {
            "id": "places-junction", "type": "circle", "source": SOURCE,
            "filter": ["==", ["geometry-type"], "Point"],
            "paint": {
                "circle-color": colour(), "circle-opacity": opacity(1.0), "circle-radius": metres("radius_m", lat, 5.0, 0.0),
                "circle-stroke-color": edge(), "circle-stroke-opacity": opacity(1.0), "circle-stroke-width": 2
            }
        },
        // What is not told by the colour alone: a changed street has a thin dark line down it, one needing attention
        // dark dashes, a changed junction a light dot and one needing attention a dark one.
        {
            "id": "places-changed", "type": "line", "source": SOURCE,
            "filter": ["all", ["==", ["geometry-type"], "LineString"], ["==", ["get", "status"], "changed"]],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": MARK, "line-opacity": opacity(0.8), "line-width": 1.2 }
        },
        {
            "id": "places-bad", "type": "line", "source": SOURCE,
            "filter": ["all", ["==", ["geometry-type"], "LineString"], ["==", ["get", "status"], "bad"]],
            "layout": { "line-cap": "butt", "line-join": "round" },
            "paint": { "line-color": MARK, "line-opacity": opacity(0.9), "line-width": 1.6, "line-dasharray": [2, 1.5] }
        },
        {
            "id": "places-changed-dot", "type": "circle", "source": SOURCE,
            "filter": ["all", ["==", ["geometry-type"], "Point"], ["==", ["get", "status"], "changed"]],
            "paint": { "circle-color": "#ffffff", "circle-opacity": opacity(1.0), "circle-radius": 2.2 }
        },
        {
            "id": "places-bad-dot", "type": "circle", "source": SOURCE,
            "filter": ["all", ["==", ["geometry-type"], "Point"], ["==", ["get", "status"], "bad"]],
            "paint": { "circle-color": MARK, "circle-opacity": opacity(1.0), "circle-radius": 2.4 }
        }
    ])
    .to_string()
}

/// The colour of an area, by its rule.
fn rule_colour() -> Value {
    let mut m = vec![json!("match"), json!(["get", "rule"])];
    for (id, colour, _) in RULES {
        m.push(json!(id));
        m.push(json!(colour));
    }
    m.push(json!(OK));
    Value::Array(m)
}

/// The restrictions' layers: a fill tinted by rule (stronger where selected), an outline a rule with that rule's
/// dash, and the selected one's corners to drag.
pub fn area_layers() -> String {
    let selected = json!(["boolean", ["get", "selected"], false]);
    let mut layers = vec![json!({
        "id": AREAS_FILL, "type": "fill", "source": AREAS,
        "filter": ["==", ["geometry-type"], "Polygon"],
        "paint": { "fill-color": rule_colour(), "fill-opacity": ["case", selected, 0.3, 0.15] }
    })];
    for (id, colour, dash) in RULES {
        let mut paint = json!({ "line-color": colour, "line-width": ["case", selected, 3.5, 2] });
        if let Some(d) = dash {
            paint["line-dasharray"] = json!(d);
        }
        layers.push(json!({
            "id": format!("areas-line-{id}"), "type": "line", "source": AREAS,
            "filter": ["all", ["==", ["geometry-type"], "Polygon"], ["==", ["get", "rule"], id]],
            "paint": paint
        }));
    }
    layers.push(json!({
        "id": AREAS_VERTEX, "type": "circle", "source": AREAS,
        "filter": ["==", ["geometry-type"], "Point"],
        "paint": { "circle-radius": 6, "circle-color": "#ffffff", "circle-stroke-color": HOT_OUTLINE, "circle-stroke-width": 2 }
    }));
    Value::Array(layers).to_string()
}

/// The outline being drawn: its area once it can close, its dashed path, and its corners (the first larger).
pub fn draft_layers() -> String {
    json!([
        { "id": "draft-fill", "type": "fill", "source": DRAFT, "filter": ["==", ["get", "part"], "fill"], "paint": { "fill-color": HOT, "fill-opacity": 0.15 } },
        { "id": "draft-line", "type": "line", "source": DRAFT, "filter": ["==", ["get", "part"], "path"], "paint": { "line-color": HOT_OUTLINE, "line-width": 2, "line-dasharray": [2, 1] } },
        {
            "id": DRAFT_VERTEX, "type": "circle", "source": DRAFT, "filter": ["==", ["geometry-type"], "Point"],
            "paint": { "circle-radius": ["case", ["boolean", ["get", "first"], false], 8, 5], "circle-color": "#ffffff", "circle-stroke-color": HOT_OUTLINE, "circle-stroke-width": 2 }
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
    fn the_places_are_drawn_in_layers_on_one_source() {
        let l = layers(45.5);
        let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids[..5], ["places-casing", "places-street", "places-pedestrian", "places-conflict", "places-junction"]);
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
    fn the_highlighted_place_is_outlined_dark_so_it_is_not_told_by_its_colour_alone() {
        let l = layers(45.5);
        for (layer, property) in [(&l[0], "line-color"), (&l[4], "circle-stroke-color")] {
            let outline = layer["paint"][property].to_string();
            assert!(outline.contains("feature-state") && outline.contains(HOT_OUTLINE) && outline.contains("#ffffff"), "{outline}");
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
        let junction = &layers(45.5)[4];
        assert_eq!(junction["type"], "circle");
        let r = junction["paint"]["circle-radius"].to_string();
        assert!(r.contains("\"radius_m\"") && r.contains("\"max\""), "{r}");
    }

    #[test]
    fn a_street_in_conflict_is_tinted_and_dashed_so_it_is_not_told_by_colour_alone() {
        let l = layers(45.5);
        assert!(l[1]["paint"]["line-color"].to_string().contains(CONFLICT));
        assert_eq!(l[3]["filter"].to_string(), r#"["all",["==",["geometry-type"],"LineString"],["boolean",["get","conflict"],false]]"#);
        assert!(l[3]["paint"]["line-dasharray"].is_array());
    }

    #[test]
    fn a_pedestrian_street_has_a_dotted_white_line_over_its_status_colour() {
        let l = layers(45.5);
        let ped = &l[2];
        assert_eq!(ped["id"], "places-pedestrian");
        assert_eq!(ped["source"], SOURCE);
        assert_eq!(ped["filter"].to_string(), r#"["all",["==",["geometry-type"],"LineString"],["==",["get","class"],"pedestrian"]]"#);
        assert_eq!(ped["paint"]["line-color"], "#ffffff");
        assert!(ped["paint"]["line-dasharray"].is_array());
        assert_eq!(ped["layout"]["line-cap"], "round", "a dash of almost no length is a dot");
    }

    #[test]
    fn a_changed_place_and_one_needing_attention_each_carry_a_mark_that_is_not_their_hue() {
        let l = layers(45.5);
        let by_id = |id: &str| l.iter().find(|l| l["id"] == id).unwrap_or_else(|| panic!("{id}"));
        let status = |layer: &Value| layer["filter"].to_string();
        // A street: a thin dark line down a changed one, dark dashes down one needing attention.
        let (changed, bad) = (by_id("places-changed"), by_id("places-bad"));
        assert!(status(changed).contains("\"changed\"") && status(bad).contains("\"bad\""));
        assert!(changed["paint"]["line-dasharray"].is_null() && bad["paint"]["line-dasharray"].is_array());
        assert_eq!(changed["paint"]["line-color"], MARK);
        assert_eq!(bad["paint"]["line-color"], MARK);
        // A junction: a light dot in a changed one, a dark dot in one needing attention.
        let (changed, bad) = (by_id("places-changed-dot"), by_id("places-bad-dot"));
        assert_eq!((changed["type"].as_str(), bad["type"].as_str()), (Some("circle"), Some("circle")));
        assert_eq!((changed["paint"]["circle-color"].as_str(), bad["paint"]["circle-color"].as_str()), (Some("#ffffff"), Some(MARK)));
        // The ones the places already had keep their order and come first.
        let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids[..5], ["places-casing", "places-street", "places-pedestrian", "places-conflict", "places-junction"]);
    }

    #[test]
    fn every_layer_of_the_places_is_muted_where_the_place_is_not_editable_and_not_before_it_is_told() {
        for layer in layers(45.5) {
            let id = layer["id"].as_str().unwrap();
            let paint = layer["paint"].as_object().unwrap();
            let opacities: Vec<String> = paint.iter().filter(|(k, _)| k.ends_with("opacity")).map(|(_, v)| v.to_string()).collect();
            assert!(!opacities.is_empty(), "{id} has an opacity");
            for o in opacities {
                assert!(o.contains("\"editable\"") && o.contains(&MUTED.to_string()), "{id}: {o}");
                assert!(o.contains("true"), "{id}: a place not told about is editable: {o}");
            }
        }
    }

    fn area_layers() -> Vec<Value> {
        serde_json::from_str(&super::area_layers()).unwrap()
    }

    #[test]
    fn the_areas_are_filled_by_rule_outlined_with_a_dash_of_their_rule_and_the_selected_one_has_corners() {
        let l = area_layers();
        let ids: Vec<&str> = l.iter().map(|l| l["id"].as_str().unwrap()).collect();
        assert_eq!(ids[0], AREAS_FILL);
        assert_eq!(*ids.last().unwrap(), AREAS_VERTEX);
        assert_eq!(ids.len(), 1 + 4 + 1, "a fill, an outline a rule, the corners");
        assert!(l.iter().all(|l| l["source"] == AREAS));
        let dashed = l[1..5].iter().filter(|l| l["paint"]["line-dasharray"].is_array()).count();
        assert_eq!(dashed, 3, "car-free is solid; the other three each have a dash of their own");
        for id in crate::restriction::model::Rule::IDS {
            assert!(l[0]["paint"]["fill-color"].to_string().contains(id), "{id}");
        }
    }

    #[test]
    fn the_outline_being_drawn_has_its_corners_to_press() {
        let l: Vec<Value> = serde_json::from_str(&draft_layers()).unwrap();
        assert!(l.iter().all(|l| l["source"] == DRAFT));
        assert_eq!(l.last().unwrap()["id"], DRAFT_VERTEX);
        assert_eq!(
            (AREAS, DRAFT, AREAS_FILL, AREAS_VERTEX, DRAFT_VERTEX),
            ("restrictions", "draft", "areas-fill", "areas-vertex", "draft-vertex"),
            "the adapter's names"
        );
    }
}
