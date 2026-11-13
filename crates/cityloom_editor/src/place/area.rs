//! The area of the world a city is made from: where it is, and what to call it.

use super::Bounds;
use crate::shared::said::Said;

/// How far either way of its centre an area reaches, in metres.
pub const HALF_M: f64 = 400.0;

/// The catalogue id of the metropolitan area of Montréal.
pub const MONTREAL: &str = "montreal";

/// A metropolitan area a project can be made in: an entry of the catalogue. A project chooses one when it is made and
/// keeps it. The editor works on one place about 800 m across (`HALF_M`), not on a whole region at once, so a
/// metropolitan area is the box of degrees it covers and the place inside it that a new project's map opens on.
#[derive(Debug, PartialEq)]
pub struct Metro {
    /// Stable: the value of the create form's option.
    pub id: &'static str,
    /// The message that names it (`place/i18n`).
    name: &'static str,
    /// What it covers, in degrees: an area whose centre lies in it belongs to it.
    south: f64,
    west: f64,
    north: f64,
    east: f64,
    /// The place a new project's map opens on: its name (data, as an area's name is) and centre.
    start: (&'static str, f64, f64),
}

/// The metropolitan areas there are, the default first. Adding one is an entry here, the message naming it in both
/// languages and its tiles in the tile index.
const METROS: &[Metro] = &[Metro {
    id: MONTREAL,
    name: "place-metro-montreal",
    // the island and its surroundings, from Saint-Jérôme to Saint-Jean-sur-Richelieu
    south: 45.2,
    west: -74.3,
    north: 45.8,
    east: -73.3,
    start: ("Plateau Mont-Royal, Montréal", 45.5261, -73.5978),
}];

impl Metro {
    /// Its name, in the page's language.
    pub fn name(&self) -> Said {
        Said::new(self.name)
    }

    /// The place a new project's map opens on.
    pub fn start(&self) -> Area {
        let (name, lat, lon) = self.start;
        Area::new(name, lat, lon)
    }

    /// Whether `area`'s centre lies in it.
    pub fn holds(&self, area: &Area) -> bool {
        (self.south..=self.north).contains(&area.lat) && (self.west..=self.east).contains(&area.lon)
    }
}

/// Every metropolitan area of the catalogue, the default first.
pub fn metros() -> &'static [Metro] {
    METROS
}

/// The metropolitan area a new project is offered first, and the one that stands in where none can be told.
pub fn default_metro() -> &'static Metro {
    &METROS[0]
}

/// The metropolitan area the catalogue lists as `id`.
pub fn metro(id: &str) -> Option<&'static Metro> {
    METROS.iter().find(|m| m.id == id)
}

/// The metropolitan area `area` lies in, if the catalogue has one.
pub fn metro_of(area: &Area) -> Option<&'static Metro> {
    METROS.iter().find(|m| m.holds(area))
}

/// The area a visit begins with where no project names one: the starting place of the default metropolitan area. It
/// is made like any other: from the metro tiles that hold it.
pub fn default_area() -> Area {
    default_metro().start()
}

/// Whether `area` lies in the metropolitan area of Montréal.
pub fn is_montreal(area: &Area) -> bool {
    metro(MONTREAL).is_some_and(|m| m.holds(area))
}

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
    fn the_default_area_is_the_plateau() {
        assert_eq!(default_area().key(), "45.5261,-73.5978");
        assert_eq!(default_area(), Area::from_query(&default_area().to_query()).unwrap());
    }

    #[test]
    fn the_catalogue_holds_montreal_the_default_metropolitan_area() {
        assert_eq!(metros().iter().map(|m| m.id).collect::<Vec<_>>(), [MONTREAL]);
        let montreal = metro(MONTREAL).expect("Montréal is in the catalogue");
        assert_eq!(default_metro(), montreal);
        assert_eq!(metro("toronto"), None);
        assert_eq!(metro(""), None);
    }

    #[test]
    fn a_metropolitan_area_opens_on_its_starting_place_which_is_the_default_area() {
        let montreal = metro(MONTREAL).unwrap();
        assert_eq!(montreal.start(), default_area());
        assert_eq!(montreal.start().key(), "45.5261,-73.5978", "the Plateau, as before: kept roads and edits stand");
        assert_eq!(metro_of(&montreal.start()), Some(montreal), "a metropolitan area holds its own starting place");
    }

    #[test]
    fn montreal_is_named_in_both_languages() {
        use crate::shared::i18n::Locale;
        let name = |locale| crate::place::say_now(&crate::i18n_for(locale), &metro(MONTREAL).unwrap().name());
        assert_eq!(name(Locale::En), "Montréal");
        assert_eq!(name(Locale::FrCa), "Montréal");
    }

    #[test]
    fn an_area_belongs_to_the_metropolitan_area_that_holds_its_centre() {
        let montreal = metro(MONTREAL);
        for (name, lat, lon) in
            [("Mile End", 45.5246, -73.6011), ("Laval", 45.5699, -73.7500), ("Longueuil", 45.5312, -73.5181), ("Saint-Jérôme", 45.7800, -74.0030)]
        {
            let area = Area::new(name, lat, lon);
            assert_eq!(metro_of(&area), montreal, "{name}");
            assert!(is_montreal(&area), "{name}");
        }
        for (name, lat, lon) in [("Berlin", 52.5, 13.4), ("Québec", 46.8139, -71.2080), ("Sherbrooke", 45.4042, -71.8929)] {
            let area = Area::new(name, lat, lon);
            assert_eq!(metro_of(&area), None, "{name}");
            assert!(!is_montreal(&area), "{name}");
        }
    }

    #[test]
    fn the_bounds_reach_a_fixed_distance_either_way() {
        let b = Area::new("x", 0.0, 0.0).bounds();
        assert!(((b.north - b.south) * 111_320.0 - 2.0 * HALF_M).abs() < 1.0);
    }
}
