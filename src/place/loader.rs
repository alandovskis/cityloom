//! Finding a place by name, and making a city of the roads around it.

use std::rc::Rc;

use super::area::Area;
use super::nominatim::{self, Place};
use super::overpass;
use super::tiles;
use crate::city::store::CityStore;
use crate::shared::ports::Ports;
use crate::shared::said::{Arg, Said};

/// What the browser or the OpenStreetMap reader said went wrong, passed on in its own words.
fn problem(e: String) -> Said {
    Said::new("place-problem").with("e", Arg::Text(e))
}

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
    pub fn search(&self, text: &str, done: impl FnOnce(Result<Vec<Place>, Said>) + 'static) {
        self.ports.fetcher.fetch(&nominatim::search_url(text), None, Box::new(move |r| done(r.map_err(problem).and_then(|body| nominatim::parse(&body)))));
    }

    /// Makes sure the store holds the street network of its area: the roads
    /// are fetched, read and kept, unless they already are.
    ///
    /// Where the roads come from: a tile of the metropolitan area's data if one holds the place, and
    /// Overpass if not (or if the tile cannot be had).
    ///
    /// Roads that are kept stand, unless the tile index gives a checksum for the tile that holds the place
    /// and it is not the one they were made from: they are then read from the tile again. A tile that cannot
    /// be had, or read, leaves the roads that are kept as they are.
    pub fn load(&self, store: &CityStore, done: impl FnOnce(Result<(), Said>) + 'static) {
        let area = store.area().clone();
        let kept = store.has_network();
        let done = Rc::new(std::cell::RefCell::new(Some(done)));
        let finish: Rc<dyn Fn(Result<(), Said>)> = Rc::new(move |r| {
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
                let index = index.ok().and_then(|body| tiles::Index::parse(&body).ok());
                let tile = index.as_ref().and_then(|i| i.tile_at(area.lat, area.lon));
                let checksum = index.as_ref().and_then(|i| i.checksum_at(area.lat, area.lon));
                if kept {
                    let stale = checksum.is_some() && store.checksum() != checksum;
                    let Some(url) = tile.filter(|_| stale) else { return finish(Ok(())) };
                    // whatever comes of it, there are roads
                    let (importer, kept_anyway) = (this.clone(), Rc::new(move |_: Result<(), Said>| finish(Ok(()))));
                    let (area, store, after) = (area.clone(), store.clone(), kept_anyway.clone());
                    this.ports.fetcher.fetch(
                        &url,
                        None,
                        Box::new(move |fetched| match fetched {
                            Ok(osm) => importer.import(osm, area, store, checksum, after),
                            Err(_) => after(Ok(())),
                        }),
                    );
                    return;
                }
                match tile {
                    Some(url) => {
                        let (importer, after) = (this.clone(), overpass.clone());
                        this.ports.fetcher.fetch(
                            &url,
                            None,
                            Box::new(move |fetched| match fetched {
                                Ok(osm) => importer.import(osm, area, store, checksum, finish),
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
    fn read(&self, url: &str, body: Option<String>, area: Area, store: CityStore, finish: Rc<dyn Fn(Result<(), Said>)>) {
        let this = self.clone();
        self.ports.fetcher.fetch(
            url,
            body.as_deref(),
            Box::new(move |fetched| match fetched {
                Err(e) => finish(Err(Said::new("place-fetch-failed").with("area", label(&area)).with("e", Arg::Text(e)))),
                Ok(osm) => this.import(osm, area, store, None, finish),
            }),
        );
    }

    /// Reads the OpenStreetMap data, keeping what lies in the area, and keeps the network that makes, with
    /// the checksum of the tile it came from, if it came from one.
    fn import(&self, osm: Vec<u8>, area: Area, store: CityStore, checksum: Option<String>, finish: Rc<dyn Fn(Result<(), Said>)>) {
        let b = area.bounds();
        self.ports.importer.import(
            osm,
            Some([b.south, b.west, b.north, b.east]),
            Box::new(move |read| {
                finish(
                    read.map_err(problem)
                        .and_then(|json| serde_json::from_str(&json).map_err(|e| Said::new("place-roads-not-understood").with("e", Arg::Text(e.to_string()))))
                        .and_then(|network: osm_network::Network| {
                            if network.roads.is_empty() {
                                return Err(Said::new("place-no-streets").with("area", label(&area)));
                            }
                            if store.keep_network_from(&network, checksum.as_deref()) { Ok(()) } else { Err(Said::new("place-roads-not-kept")) }
                        }),
                )
            }),
        );
    }
}

/// What an area is called in a message: its name, or where it is when it has none. Data, not words of the app.
fn label(area: &Area) -> Arg {
    Arg::Text(if area.name.is_empty() { area.key() } else { area.name.clone() })
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

    /// What a loader said, in English.
    fn en(said: &Said) -> String {
        crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), crate::shared::units::Units::Metres, said)
    }

    /// What a loader said, in French.
    fn fr(said: &Said) -> String {
        crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::FrCa), crate::shared::units::Units::Metres, said)
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
        assert_eq!(en(&found.borrow_mut().take().unwrap().unwrap_err()), "the server answered 429");
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

    /// A tile of Testville with a checksum, and the index that names it.
    const SUMMED: &str = r#"{"lon0":1.9,"lat0":0.9,"dlon":0.0257,"dlat":0.018,"tiles":["3_5"],"checksums":{"3_5":"new"}}"#;

    /// Testville with its network kept as made from a tile that had `checksum`, and its load begun.
    fn kept_from(
        checksum: Option<&str>,
    ) -> (crate::shared::ports::Ports, Rc<crate::shared::ports::FakeFetcher>, Rc<crate::shared::ports::FakeImporter>, CityStore, Seen<Result<(), Said>>) {
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        store.keep_network_from(&serde_json::from_str(NETWORK).unwrap(), checksum);
        let result = seen();
        let r = result.clone();
        Loader::new(ports.clone()).load(&store, move |x| *r.borrow_mut() = Some(x));
        (ports, fetcher, importer, store, result)
    }

    #[test]
    fn an_area_already_kept_is_not_read_again_while_its_tile_is_the_same() {
        let (_, fetcher, _, store, result) = kept_from(Some("new"));
        assert_eq!(fetcher.asked(), vec![("data/metro/index.json".to_string(), None)]);
        fetcher.answer(Ok(SUMMED.as_bytes().to_vec()));
        assert!(fetcher.asked().is_empty(), "the tile is not fetched again");
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert_eq!(store.checksum().as_deref(), Some("new"));
    }

    #[test]
    fn an_area_whose_tile_has_changed_is_read_again_and_keeps_the_new_checksum() {
        let (_, fetcher, importer, store, result) = kept_from(Some("old"));
        fetcher.answer(Ok(SUMMED.as_bytes().to_vec()));
        assert_eq!(fetcher.asked(), vec![("data/metro/3_5.osm.pbf".to_string(), None)]);
        assert!(result.borrow().is_none(), "the page waits for the new roads");
        fetcher.answer(Ok(b"TILE".to_vec()));
        assert_eq!(importer.asked(), vec![b"TILE".to_vec()]);
        let two_roads = NETWORK.replace(r#""name":"Only Street""#, r#""name":"Changed Street""#);
        importer.answer(Ok(two_roads));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert_eq!(store.checksum().as_deref(), Some("new"));
        assert_eq!(
            crate::shared::said::say_now(
                &crate::i18n_for(crate::shared::i18n::Locale::En),
                crate::shared::units::Units::Metres,
                &store.open().view(0).edges[0].called
            ),
            "Changed Street"
        );
    }

    #[test]
    fn an_area_kept_before_checksums_were_is_read_again_once() {
        let (_, fetcher, importer, store, _) = kept_from(None);
        fetcher.answer(Ok(SUMMED.as_bytes().to_vec()));
        fetcher.answer(Ok(b"TILE".to_vec()));
        importer.answer(Ok(NETWORK.to_string()));
        assert_eq!(store.checksum().as_deref(), Some("new"));
    }

    #[test]
    fn an_area_is_kept_as_it_is_when_nothing_says_its_data_changed_or_it_cannot_be_checked() {
        // no index, an index that is not one, a place no tile holds, and an index with no checksums
        let answers: [Result<Vec<u8>, String>; 4] = [
            Err("offline".into()),
            Ok(b"<html>".to_vec()),
            Ok(br#"{"lon0":50.0,"lat0":50.0,"dlon":0.0257,"dlat":0.018,"tiles":["0_0"],"checksums":{"0_0":"x"}}"#.to_vec()),
            Ok(INDEX.as_bytes().to_vec()),
        ];
        for (i, answer) in answers.into_iter().enumerate() {
            let (_, fetcher, _, store, result) = kept_from(Some("old"));
            fetcher.answer(answer);
            assert!(fetcher.asked().is_empty(), "case {i}: nothing more is asked");
            assert_eq!(result.borrow_mut().take(), Some(Ok(())), "case {i}");
            assert_eq!(store.checksum().as_deref(), Some("old"), "case {i}");
        }
    }

    #[test]
    fn a_refresh_that_fails_leaves_the_roads_that_are_kept() {
        for step in 0..3 {
            let (_, fetcher, importer, store, result) = kept_from(Some("old"));
            fetcher.answer(Ok(SUMMED.as_bytes().to_vec()));
            match step {
                0 => assert!(fetcher.answer(Err("the server answered 404".into()))),
                1 => {
                    fetcher.answer(Ok(b"TILE".to_vec()));
                    importer.answer(Err("bad data".into()));
                }
                _ => {
                    fetcher.answer(Ok(b"TILE".to_vec()));
                    importer.answer(Ok(r#"{"left_hand":false,"nodes":[],"roads":[]}"#.to_string()));
                }
            }
            assert!(fetcher.asked().is_empty(), "step {step}: Overpass is not asked");
            assert_eq!(result.borrow_mut().take(), Some(Ok(())), "step {step}");
            assert_eq!(store.checksum().as_deref(), Some("old"), "step {step}");
            assert_eq!(
                crate::shared::said::say_now(
                    &crate::i18n_for(crate::shared::i18n::Locale::En),
                    crate::shared::units::Units::Metres,
                    &store.open().view(0).edges[0].called
                ),
                "Only Street",
                "step {step}"
            );
        }
    }

    #[test]
    fn an_area_read_from_a_tile_keeps_the_tile_s_checksum_and_one_read_from_overpass_has_none() {
        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        Loader::new(ports).load(&store, |_| {});
        fetcher.answer(Ok(SUMMED.as_bytes().to_vec()));
        fetcher.answer(Ok(b"TILE".to_vec()));
        importer.answer(Ok(NETWORK.to_string()));
        assert_eq!(store.checksum().as_deref(), Some("new"));

        let (ports, fetcher, importer) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        Loader::new(ports).load(&store, |_| {});
        without_tiles(&fetcher);
        fetcher.answer(Ok(b"<osm/>".to_vec()));
        importer.answer(Ok(NETWORK.to_string()));
        assert!(store.has_network() && store.checksum().is_none());
    }

    #[test]
    fn each_thing_that_goes_wrong_is_said_and_nothing_is_kept() {
        let english = ["the roads of Testville could not be fetched (offline)", "bad data", "the roads were not understood ("];
        let french = ["les rues de Testville n’ont pas pu être obtenues (offline)", "bad data", "les rues n’ont pas été comprises ("];
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
            assert!(en(&message).starts_with(english[step]), "step {step}: {}", en(&message));
            assert!(fr(&message).starts_with(french[step]), "step {step}: {}", fr(&message));
            assert!(!store.has_network(), "step {step}: {message:?}");
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
        assert_eq!(en(&message), "OpenStreetMap has no streets around Mid-ocean");
        assert_eq!(fr(&message), "OpenStreetMap n’a aucune rue autour de Mid-ocean");
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
        assert_eq!(en(&result.borrow_mut().take().unwrap().unwrap_err()), "the roads could not be kept: storage is blocked or full");
    }
}
