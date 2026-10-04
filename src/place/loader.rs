//! Finding a place by name, and making a city of the roads around it.

use std::rc::Rc;

use super::area::{Area, DEFAULT_DATA_URL, default_area};
use super::nominatim::{self, Place};
use super::overpass;
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
    /// are fetched, read and kept, unless they already are. A store of no area
    /// has the sample city and is done at once.
    pub fn load(&self, store: &CityStore, done: impl FnOnce(Result<(), String>) + 'static) {
        let Some(area) = store.area().cloned() else { return done(Ok(())) };
        if store.has_network() {
            return done(Ok(()));
        }
        let bounds = area.bounds();
        let area_name = area.clone();
        let (store, importer) = (store.clone(), self.ports.importer.clone());
        let done = Rc::new(std::cell::RefCell::new(Some(done)));
        let finish = move |r: Result<(), String>| {
            if let Some(done) = done.borrow_mut().take() {
                done(r)
            }
        };
        let finish_import = finish.clone();
        // The default area's data comes with the app, as PBF; any other is asked of Overpass, as XML.
        let (url, query) = if area == default_area() { (DEFAULT_DATA_URL, None) } else { (overpass::ENDPOINT, Some(overpass::query(area.bounds()))) };
        self.ports.fetcher.fetch(
            url,
            query.as_deref(),
            Box::new(move |fetched| match fetched {
                Err(e) => finish(Err(format!("the roads of {} could not be fetched ({e})", label(&area)))),
                Ok(osm) => importer.import(
                    osm,
                    Some([bounds.south, bounds.west, bounds.north, bounds.east]),
                    Box::new(move |read| {
                        finish_import(read.and_then(|json| serde_json::from_str(&json).map_err(|e| format!("the roads were not understood ({e})"))).and_then(
                            |network: osm_network::Network| {
                                if network.roads.is_empty() {
                                    return Err(format!("OpenStreetMap has no streets around {}", label(&area_name)));
                                }
                                if store.keep_network(&network) { Ok(()) } else { Err("the roads could not be kept: storage is blocked or full".to_string()) }
                            },
                        ))
                    }),
                ),
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
    use crate::shared::ports::{MemoryStorage, test_ports_with_fetcher};

    type Seen<T> = Rc<RefCell<Option<T>>>;

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
    fn the_default_area_is_read_from_the_pbf_that_ships_not_from_overpass() {
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), default_area());
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        assert_eq!(fetcher.asked(), vec![("data/default.osm.pbf".to_string(), None)]);
        fetcher.answer(Ok(b"PBF".to_vec()));
        assert_eq!(importer.asked(), vec![b"PBF".to_vec()], "read like any other data");
        importer.answer(Ok(NETWORK.to_string()));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert!(store.has_network());
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
    fn the_sample_city_needs_nothing_fetched() {
        let (ports, fetcher, _) = test_ports_with_fetcher();
        let store = CityStore::new(ports.storage.clone());
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

    #[test]
    fn a_place_with_no_streets_round_it_is_said_and_not_kept() {
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Mid-ocean", 1.0, 2.0));
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
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
        fetcher.answer(Ok(vec![]));
        importer.answer(Ok(NETWORK.to_string()));
        assert!(result.borrow_mut().take().unwrap().unwrap_err().contains("storage"));
    }
}
