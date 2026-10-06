//! Finding a place by name, and making a city of the roads around it.

use std::rc::Rc;

use super::area::Area;
use super::nominatim::{self, Place};
use super::overpass;
use super::tiles;
use crate::city::store::CityStore;
use crate::shared::ports::Ports;

/// What is asked of the world's services, one thing at a time, with the answer handed on.
#[derive(Clone)]
pub struct Loader {
    ports: Ports,
}

impl Loader {
    pub fn new(ports: Ports) -> Loader {
        Loader { ports }
    }

    /// The places called `text`, best first.
    pub fn search(&self, text: &str, done: impl FnOnce(Result<Vec<Place>, String>) + 'static) {
        self.ports.fetcher.fetch(&nominatim::search_url(text), None, Box::new(move |r| done(r.and_then(|body| nominatim::parse(&body)))));
    }

    /// Makes sure the store holds the street network of its area: the roads
    /// are fetched, read and kept, unless they already are.
    ///
    /// Where the roads come from: a tile of the metropolitan area's data if one holds the place, and
    /// Overpass if not (or if the tile cannot be had).
    pub fn load(&self, store: &CityStore, done: impl FnOnce(Result<(), String>) + 'static) {
        let area = store.area().clone();
        if store.has_network() {
            return done(Ok(()));
        }
        let done = Rc::new(std::cell::RefCell::new(Some(done)));
        let finish: Rc<dyn Fn(Result<(), String>)> = Rc::new(move |r| {
            if let Some(done) = done.borrow_mut().take() {
                done(r)
            }
        });
        let this = self.clone();
        let store = store.clone();
        let overpass = {
            let (this, area, store, finish) = (this.clone(), area.clone(), store.clone(), finish.clone());
            move || this.read(overpass::ENDPOINT, Some(overpass::query(area.bounds())), area, store, finish)
        };
        self.ports.fetcher.fetch(
            tiles::INDEX_URL,
            None,
            Box::new(move |index| {
                let tile = index.ok().and_then(|body| tiles::Index::parse(&body).ok()).and_then(|i| i.tile_at(area.lat, area.lon));
                match tile {
                    Some(url) => {
                        let (importer, after) = (this.clone(), overpass.clone());
                        this.ports.fetcher.fetch(
                            &url,
                            None,
                            Box::new(move |fetched| match fetched {
                                Ok(osm) => importer.import(osm, area, store, finish),
                                // a tile that is listed but cannot be had is no reason not to ask Overpass
                                Err(_) => after(),
                            }),
                        );
                    }
                    None => overpass(),
                }
            }),
        );
    }

    /// Fetches the data at `url` (by POST when there is a `body`) and imports it.
    fn read(&self, url: &str, body: Option<String>, area: Area, store: CityStore, finish: Rc<dyn Fn(Result<(), String>)>) {
        let this = self.clone();
        self.ports.fetcher.fetch(
            url,
            body.as_deref(),
            Box::new(move |fetched| match fetched {
                Err(e) => finish(Err(format!("the roads of {} could not be fetched ({e})", label(&area)))),
                Ok(osm) => this.import(osm, area, store, finish),
            }),
        );
    }

    /// Reads the OpenStreetMap data, keeping what lies in the area, and keeps the network that makes.
    fn import(&self, osm: Vec<u8>, area: Area, store: CityStore, finish: Rc<dyn Fn(Result<(), String>)>) {
        let b = area.bounds();
        self.ports.importer.import(
            osm,
            Some([b.south, b.west, b.north, b.east]),
            Box::new(move |read| {
                finish(read.and_then(|json| serde_json::from_str(&json).map_err(|e| format!("the roads were not understood ({e})"))).and_then(
                    |network: osm_network::Network| {
                        if network.roads.is_empty() {
                            return Err(format!("OpenStreetMap has no streets around {}", label(&area)));
                        }
                        if store.keep_network(&network) { Ok(()) } else { Err("the roads could not be kept: storage is blocked or full".to_string()) }
                    },
                ))
            }),
        );
    }
}

fn label(area: &Area) -> String {
    if area.name.is_empty() { area.key() } else { area.name.clone() }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::place::area::default_area;
    use crate::shared::ports::{MemoryStorage, test_ports_with_fetcher};

    type Seen<T> = Rc<RefCell<Option<T>>>;

    /// The metro tiles are not there: the index asked for first is not found.
    fn without_tiles(fetcher: &crate::shared::ports::FakeFetcher) {
        assert_eq!(fetcher.asked()[0].0, "data/metro/index.json");
        fetcher.answer(Err("the server answered 404".into()));
    }

    fn seen<T>() -> Seen<T> {
        Rc::new(RefCell::new(None))
    }

    const NETWORK: &str = r#"{"left_hand":false,"nodes":[{"id":1,"osm_nodes":[1],"x_m":0.0,"y_m":0.0,"junction":false,"control":"none"},
        {"id":2,"osm_nodes":[2],"x_m":90.0,"y_m":0.0,"junction":false,"control":"none"}],
        "roads":[{"id":1,"osm_ways":[5],"name":"Only Street","highway":"residential","from":1,"to":2,
        "lanes":[{"kind":"driving","way":"forward","width_m":3.0},{"kind":"driving","way":"backward","width_m":3.0}],"points":[[0.0,0.0],[90.0,0.0]]}]}"#;

    #[test]
    fn a_search_asks_nominatim_and_hands_on_the_places() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let found = seen();
        let f = found.clone();
        Loader::new(ports).search("Kreuzberg", move |r| *f.borrow_mut() = Some(r));
        assert_eq!(fetcher.asked()[0].0, "https://nominatim.openstreetmap.org/search?q=Kreuzberg&format=jsonv2&limit=5");
        fetcher.answer(Ok(br#"[{"display_name":"Kreuzberg","lat":"52.5","lon":"13.4","boundingbox":["52.4","52.6","13.3","13.5"]}]"#.to_vec()));
        let places = found.borrow_mut().take().unwrap().unwrap();
        assert_eq!(places[0].name, "Kreuzberg");
    }

    #[test]
    fn a_search_that_cannot_reach_nominatim_says_why() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let found = seen();
        let f = found.clone();
        Loader::new(ports).search("x", move |r| *f.borrow_mut() = Some(r));
        fetcher.answer(Err("the server answered 429".into()));
        assert_eq!(found.borrow_mut().take().unwrap().unwrap_err(), "the server answered 429");
    }

    #[test]
    fn an_area_is_fetched_read_and_kept_and_the_city_then_opens_on_it() {
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        without_tiles(&fetcher);
        let (url, body) = fetcher.asked().remove(0);
        assert_eq!(url, overpass::ENDPOINT);
        assert!(body.unwrap().starts_with("[out:xml]"));
        assert!(result.borrow().is_none(), "nothing is done before the roads arrive");
        fetcher.answer(Ok(b"<osm/>".to_vec()));
        assert_eq!(importer.asked(), vec![b"<osm/>".to_vec()]);
        let [south, west, north, east] = importer.bounds_asked()[0].expect("only the area is kept");
        let b = Area::new("Testville", 1.0, 2.0).bounds();
        assert_eq!([south, west, north, east], [b.south, b.west, b.north, b.east]);
        importer.answer(Ok(NETWORK.to_string()));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert_eq!(store.open().view(0).name, "Testville");
    }

    #[test]
    fn the_default_area_is_read_like_any_other_from_its_tile_or_from_overpass() {
        // from a tile, when the index has one for it
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), default_area());
        Loader::new(ports).load(&store, |_| {});
        let d = default_area();
        let index = format!(r#"{{"lon0":{},"lat0":{},"dlon":0.0257,"dlat":0.018,"tiles":["0_0"]}}"#, d.lon - 0.01, d.lat - 0.01);
        fetcher.answer(Ok(index.into_bytes()));
        assert_eq!(fetcher.asked(), vec![("data/metro/0_0.osm.pbf".to_string(), None)]);
        fetcher.answer(Ok(b"TILE".to_vec()));
        assert_eq!(importer.asked(), vec![b"TILE".to_vec()]);

        // and from Overpass when there are no tiles, as there are not where they are not deployed
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), default_area());
        Loader::new(ports).load(&store, |_| {});
        without_tiles(&fetcher);
        assert_eq!(fetcher.asked()[0].0, overpass::ENDPOINT);
    }

    #[test]
    fn an_area_already_kept_is_not_fetched_again() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        store.keep_network(&serde_json::from_str(NETWORK).unwrap());
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        assert!(fetcher.asked().is_empty());
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
    }

    #[test]
    fn each_thing_that_goes_wrong_is_said_and_nothing_is_kept() {
        for step in 0..3 {
            let (ports, fetcher, importer) = test_ports_with_fetcher();
            let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
            let result = seen();
            let r = result.clone();
            Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
            without_tiles(&fetcher);
            match step {
                0 => {
                    fetcher.answer(Err("offline".into()));
                }
                1 => {
                    fetcher.answer(Ok(vec![]));
                    importer.answer(Err("bad data".into()));
                }
                _ => {
                    fetcher.answer(Ok(vec![]));
                    importer.answer(Ok("{not json".into()));
                }
            }
            let message = result.borrow_mut().take().unwrap().unwrap_err();
            assert!(!message.is_empty(), "step {step}");
            assert!(!store.has_network(), "step {step}: {message}");
        }
    }

    const INDEX: &str = r#"{"lon0":1.9,"lat0":0.9,"dlon":0.0257,"dlat":0.018,"tiles":["3_5"]}"#;

    #[test]
    fn a_place_in_a_tile_is_read_from_that_tile_and_overpass_is_not_asked() {
        // 1.0, 2.0 is in tile ((2.0-1.9)/0.0257, (1.0-0.9)/0.018) = (3, 5)
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        assert_eq!(fetcher.asked(), vec![("data/metro/index.json".to_string(), None)]);
        fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
        assert_eq!(fetcher.asked(), vec![("data/metro/3_5.osm.pbf".to_string(), None)]);
        fetcher.answer(Ok(b"TILE".to_vec()));
        assert_eq!(importer.asked(), vec![b"TILE".to_vec()]);
        assert!(importer.bounds_asked()[0].is_some(), "only the area is kept of the whole tile");
        importer.answer(Ok(NETWORK.to_string()));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert!(fetcher.asked().is_empty(), "nothing was asked of Overpass");
        assert!(store.has_network());
    }

    #[test]
    fn a_place_no_tile_holds_is_asked_of_overpass() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        // the area is far from the tiles there are
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Paris", 48.85, 2.35));
        Loader::new(ports).load(&store, |_| {});
        fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
        assert_eq!(fetcher.asked()[0].0, overpass::ENDPOINT);
    }

    #[test]
    fn a_tile_that_cannot_be_had_is_no_reason_not_to_ask_overpass() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        Loader::new(ports).load(&store, |_| {});
        fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
        fetcher.answer(Err("the server answered 404".into()));
        assert_eq!(fetcher.asked()[0].0, overpass::ENDPOINT);
    }

    #[test]
    fn a_place_with_no_streets_round_it_is_said_and_not_kept() {
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Mid-ocean", 1.0, 2.0));
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        without_tiles(&fetcher);
        fetcher.answer(Ok(b"<osm/>".to_vec()));
        importer.answer(Ok(r#"{"left_hand":false,"nodes":[],"roads":[]}"#.to_string()));
        let message = result.borrow_mut().take().unwrap().unwrap_err();
        assert!(message.contains("no streets") && message.contains("Mid-ocean"), "{message}");
        assert!(!store.has_network());
    }

    #[test]
    fn blocked_storage_is_said_and_not_taken_for_success() {
        let (mut ports, fetcher, importer) = test_ports_with_fetcher();
        let storage = Rc::new(MemoryStorage::default());
        ports.storage = storage.clone();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        storage.blocked(true);
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        without_tiles(&fetcher);
        fetcher.answer(Ok(vec![]));
        importer.answer(Ok(NETWORK.to_string()));
        assert!(result.borrow_mut().take().unwrap().unwrap_err().contains("storage"));
    }
}
