//! Place search by Nominatim, the geocoder of OpenStreetMap.

use serde::Deserialize;

use super::{Bounds, encode};
use crate::shared::said::{Arg, Said};

const ENDPOINT: &str = "https://nominatim.openstreetmap.org/search";

/// The address that asks for up to five places matching `text`.
pub fn search_url(text: &str) -> String {
    format!("{ENDPOINT}?q={}&format=jsonv2&limit=5", encode(text.trim()))
}

/// A place found by name.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub bounds: Bounds,
}

#[derive(Deserialize)]
struct Hit {
    display_name: String,
    lat: String,
    lon: String,
    boundingbox: Vec<String>,
}

/// The places in a Nominatim answer, best first. An answer that is not what
/// Nominatim sends is an error; a hit that lacks a position is left out.
pub fn parse(body: &[u8]) -> Result<Vec<Place>, Said> {
    let hits: Vec<Hit> = serde_json::from_slice(body).map_err(|e| Said::new("place-search-unexpected").with("e", Arg::Text(e.to_string())))?;
    Ok(hits.into_iter().filter_map(place).collect())
}

fn place(h: Hit) -> Option<Place> {
    let number = |s: &String| s.parse::<f64>().ok();
    let b: Vec<f64> = h.boundingbox.iter().filter_map(number).collect();
    // Nominatim orders a box south, north, west, east.
    let [south, north, west, east] = b[..] else { return None };
    Some(Place { name: h.display_name, lat: number(&h.lat)?, lon: number(&h.lon)?, bounds: Bounds { south, west, north, east } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_asks_for_a_few_places_by_the_text() {
        assert_eq!(search_url("  Kreuzberg, Berlin "), "https://nominatim.openstreetmap.org/search?q=Kreuzberg%2C%20Berlin&format=jsonv2&limit=5");
    }

    #[test]
    fn the_places_in_an_answer_are_read_with_their_boxes() {
        let body = br#"[{"display_name":"Kreuzberg, Berlin, Germany","lat":"52.4990","lon":"13.4030","boundingbox":["52.48","52.51","13.38","13.43"]},
                       {"display_name":"no box","lat":"1","lon":"2","boundingbox":[]}]"#;
        let found = parse(body).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Kreuzberg, Berlin, Germany");
        assert_eq!((found[0].lat, found[0].lon), (52.499, 13.403));
        assert_eq!(found[0].bounds, Bounds { south: 52.48, west: 13.38, north: 52.51, east: 13.43 });
    }

    #[test]
    fn nothing_found_is_an_empty_list_and_a_wrong_answer_is_an_error() {
        assert_eq!(parse(b"[]").unwrap(), vec![]);
        let wrong = parse(b"<html>rate limited</html>").unwrap_err();
        let en = crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), crate::shared::units::Units::Metres, &wrong);
        assert!(en.starts_with("the place search answered something unexpected (expected value"), "{en}");
    }
}
