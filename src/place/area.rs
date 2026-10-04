//! The area of the world a city is made from: where it is, and what to call it.

use super::Bounds;

/// How far either way of its centre an area reaches, in metres.
pub const HALF_M: f64 = 400.0;

/// The area a visit begins with, before anyone has chosen one. Its OpenStreetMap data is shipped
/// with the app (`web/data/default.osm.pbf`), so it opens without Overpass.
pub fn default_area() -> Area {
    Area::new("Plateau Mont-Royal, Montréal", 45.5261, -73.5978)
}

/// Where the shipped OpenStreetMap data of the default area is, relative to the pages.
pub const DEFAULT_DATA_URL: &str = "data/default.osm.pbf";

#[derive(Clone, Debug, PartialEq)]
pub struct Area {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

impl Area {
    pub fn new(name: &str, lat: f64, lon: f64) -> Area {
        // A position is kept to the metre or so: four places of a degree. The same place found twice is one area.
        let round = |d: f64| (d * 1e4).round() / 1e4;
        Area { name: name.trim().to_string(), lat: round(lat), lon: round(lon) }
    }

    /// What the area is kept under: its position, which names it better than its name does.
    pub fn key(&self) -> String {
        format!("{:.4},{:.4}", self.lat, self.lon)
    }

    pub fn bounds(&self) -> Bounds {
        Bounds::around(self.lat, self.lon, HALF_M)
    }

    /// The query string of a page that opens this area: `area=52.5010,13.4235&name=Kreuzberg`.
    pub fn to_query(&self) -> String {
        format!("area={}&name={}", self.key(), super::encode(&self.name))
    }

    /// The area a query string names, if it names one. A name is optional.
    pub fn from_query(query: &str) -> Option<Area> {
        let mut at = None;
        let mut name = String::new();
        for pair in query.trim_start_matches('?').split('&') {
            match pair.split_once('=') {
                Some(("area", v)) => at = v.split_once(',').and_then(|(a, o)| Some((a.parse::<f64>().ok()?, o.parse::<f64>().ok()?))),
                Some(("name", v)) => name = super::decode(v),
                _ => {}
            }
        }
        let (lat, lon) = at.filter(|(a, o)| (-90.0..=90.0).contains(a) && (-180.0..=180.0).contains(o))?;
        Some(Area::new(&name, lat, lon))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_area_is_kept_under_its_position_to_the_metre() {
        let a = Area::new(" Kreuzberg ", 52.501_04, 13.423_46);
        assert_eq!((a.name.as_str(), a.key().as_str()), ("Kreuzberg", "52.5010,13.4235"));
        assert_eq!(Area::new("again", 52.50099, 13.42351).key(), a.key());
    }

    #[test]
    fn an_area_goes_into_a_query_and_comes_back() {
        let a = Area::new("Rue de l'Église, Paris", 48.8566, 2.3522);
        let q = a.to_query();
        assert_eq!(q, "area=48.8566,2.3522&name=Rue%20de%20l%27%C3%89glise%2C%20Paris");
        assert_eq!(Area::from_query(&format!("?{q}")), Some(a));
    }

    #[test]
    fn a_query_without_a_usable_position_names_no_area() {
        assert_eq!(Area::from_query(""), None);
        assert_eq!(Area::from_query("name=Nowhere"), None);
        assert_eq!(Area::from_query("area=abc,def"), None);
        assert_eq!(Area::from_query("area=95,0"), None);
        assert_eq!(Area::from_query("area=1,2").unwrap().name, "");
    }

    #[test]
    fn the_default_area_is_the_one_whose_data_ships() {
        assert_eq!(default_area().key(), "45.5261,-73.5978");
        assert_eq!(default_area(), Area::from_query(&default_area().to_query()).unwrap());
    }

    #[test]
    fn the_bounds_reach_a_fixed_distance_either_way() {
        let b = Area::new("x", 0.0, 0.0).bounds();
        assert!(((b.north - b.south) * 111_320.0 - 2.0 * HALF_M).abs() < 1.0);
    }
}
