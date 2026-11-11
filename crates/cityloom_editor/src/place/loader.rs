//! Finding a place by name, and making a city of the roads around it.

use std::cell::RefCell;
use std::rc::Rc;

use cityloom_model::NetworkTile;

use super::area::Area;
use super::nominatim::{self, Place};
use super::tiles;
use crate::city::store::CityStore;
use crate::shared::ports::{Fetched, Ports};
use crate::shared::said::{Arg, Said};

/// What the browser said went wrong, passed on in its own words.
fn problem(e: String) -> Said {
    Said::new("place-problem").with("e", Arg::Text(e))
}

/// The roads that were fetched could not be read; `e` says what was wrong with them.
fn not_understood(e: String) -> Said {
    Said::new("place-roads-not-understood").with("e", Arg::Text(e))
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

    /// Makes sure the store holds the street network of its area: the roads are fetched, cut out and kept, unless
    /// they already are.
    ///
    /// Where the roads come from: the tile of the metropolitan area's data whose core holds the place, and every
    /// other tile there is whose core the place's box reaches. A place no tile holds is not covered.
    ///
    /// Roads that are kept stand, unless the tile index gives a checksum for the tile that holds the place
    /// and it is not the one they were made from: they are then read from the tiles again. A tile that cannot
    /// be had, or read, leaves the roads that are kept as they are.
    pub fn load(&self, store: &CityStore, done: impl FnOnce(Result<(), Said>) + 'static) {
        let area = store.area().clone();
        let kept = store.has_network();
        let (this, store) = (self.clone(), store.clone());
        self.ports.fetcher.fetch(
            tiles::INDEX_URL,
            None,
            Box::new(move |index| {
                let not_covered = || Said::new("place-not-covered").with("area", label(&area));
                let index = match index.map(|body| tiles::Index::parse(&body)) {
                    Ok(Ok(index)) => index,
                    // whatever comes of it, there are roads
                    _ if kept => return done(Ok(())),
                    Ok(Err(why)) => return done(Err(why)),
                    Err(_) => return done(Err(not_covered())),
                };
                if index.tile_at(area.lat, area.lon).is_none() {
                    return done(if kept { Ok(()) } else { Err(not_covered()) });
                }
                let checksum = index.checksum_at(area.lat, area.lon);
                if !kept {
                    return this.read(&index, area, store, checksum, done);
                }
                if checksum.is_none() || store.checksum() == checksum {
                    return done(Ok(()));
                }
                // whatever comes of it, there are roads
                this.read(&index, area, store, checksum, move |_| done(Ok(())));
            }),
        );
    }

    /// Fetches every tile the area's box reaches, the one holding the place first, and keeps what lies in the box,
    /// with `checksum`, once they have all come.
    fn read(&self, index: &tiles::Index, area: Area, store: CityStore, checksum: Option<String>, done: impl FnOnce(Result<(), Said>) + 'static) {
        let urls = index.tiles_touching(area.bounds());
        let answers: Rc<RefCell<Vec<Option<Fetched>>>> = Rc::new(RefCell::new(vec![None; urls.len()]));
        let done = Rc::new(RefCell::new(Some(done)));
        for (i, url) in urls.iter().enumerate() {
            let (answers, done, urls, area, store, checksum) = (answers.clone(), done.clone(), urls.clone(), area.clone(), store.clone(), checksum.clone());
            self.ports.fetcher.fetch(
                url,
                None,
                Box::new(move |fetched| {
                    let all = {
                        let mut answers = answers.borrow_mut();
                        answers[i] = Some(fetched);
                        if answers.iter().any(Option::is_none) {
                            return;
                        }
                        answers.drain(..).flatten().collect::<Vec<_>>()
                    };
                    let Some(done) = done.borrow_mut().take() else { return };
                    done(keep(&urls, all, &area, &store, checksum.as_deref()));
                }),
            );
        }
    }
}

/// Reads the tiles fetched from `urls`, merges them by id, and keeps what lies in the area's box with `checksum`.
fn keep(urls: &[String], fetched: Vec<Fetched>, area: &Area, store: &CityStore, checksum: Option<&str>) -> Result<(), Said> {
    let mut merged: Option<NetworkTile> = None;
    for (url, fetched) in urls.iter().zip(fetched) {
        let bytes = fetched.map_err(|e| Said::new("place-fetch-failed").with("area", label(area)).with("e", Arg::Text(e)))?;
        let tile: NetworkTile = serde_json::from_slice(&bytes).map_err(|e| not_understood(e.to_string()))?;
        match &mut merged {
            None => merged = Some(tile),
            // the tiles of one area share its frame: one that does not is not of the same data
            Some(m) if m.frame != tile.frame => return Err(not_understood(url.clone())),
            Some(m) => m.network.absorb(&tile.network),
        }
    }
    let Some(tile) = merged else { return Err(Said::new("place-not-covered").with("area", label(area))) };
    let network = tile.network.clip_to(&tile.frame, &area.bounds().frame());
    if network.roads.is_empty() {
        return Err(Said::new("place-no-streets").with("area", label(area)));
    }
    if store.keep_network_from(&network, checksum) { Ok(()) } else { Err(Said::new("place-roads-not-kept")) }
}

/// What an area is called in a message: its name, or where it is when it has none. Data, not words of the app.
fn label(area: &Area) -> Arg {
    Arg::Text(if area.name.is_empty() { area.key() } else { area.name.clone() })
}

#[cfg(test)]
mod tests {
    use cityloom_model::{Network, with_importer};

    use super::*;
    use crate::city::store::NETWORK_KEY;
    use crate::place::area::default_area;
    use crate::place::tiles::testing::{AREA, INDEX, covering, tile};
    use crate::shared::ports::{FakeFetcher, MemoryStorage, Ports, test_ports_with_fetcher};

    type Seen<T> = Rc<RefCell<Option<T>>>;

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

    /// The URLs asked and not yet answered, oldest first.
    fn urls(fetcher: &FakeFetcher) -> Vec<String> {
        fetcher.asked().into_iter().map(|(url, _)| url).collect()
    }

    /// The network kept for `area`, if one is.
    fn kept_network(ports: &Ports, area: &Area) -> Option<Network> {
        ports.storage.recall(&format!("{NETWORK_KEY}:{}", area.key())).map(|json| serde_json::from_str(&json).unwrap())
    }

    /// The name of each street of the city kept for `store`, in English.
    fn names(store: &CityStore) -> Vec<String> {
        let mut names: Vec<String> = store.open().view(0).edges.iter().map(|e| en(&e.called)).collect();
        names.sort();
        names.dedup();
        names
    }

    /// The places of tile 12_9 and of tiles 12_9 and 12_10, whose boxes reach 0.0036 degrees north and south.
    fn in_one_tile() -> Area {
        Area::new("Inland", 45.371, -74.078)
    }

    fn on_the_edge() -> Area {
        Area::new("Edgeton", 45.378, -74.078)
    }

    /// Tile 12_9: a street through the middle of `in_one_tile`, one across its box, one south of it, and one running
    /// north into 12_10, which that tile holds too.
    fn south() -> Vec<u8> {
        tile(
            &AREA,
            &[
                (1, "Inside Street", &[(45.371, -74.080), (45.371, -74.076)]),
                (2, "Across Street", &[(45.372, -74.090), (45.372, -74.070)]),
                (3, "Far Street", &[(45.364, -74.080), (45.364, -74.076)]),
                (4, "Shared Street", &[(45.373, -74.076), (45.385, -74.076)]),
            ],
        )
    }

    /// Tile 12_10: the street both tiles hold, and one in the north of `on_the_edge`'s box.
    fn north() -> Vec<u8> {
        tile(&AREA, &[(4, "Shared Street", &[(45.373, -74.076), (45.385, -74.076)]), (5, "North Street", &[(45.381, -74.080), (45.381, -74.076)])])
    }

    fn parsed(bytes: &[u8]) -> NetworkTile {
        serde_json::from_slice(bytes).unwrap()
    }

    /// A load of `area` begun on fresh ports: the ports, the fetcher, the store and what the load said.
    fn loading(area: Area) -> (Ports, Rc<FakeFetcher>, CityStore, Seen<Result<(), Said>>) {
        let (ports, fetcher, ..) = test_ports_with_fetcher();
        let store = CityStore::for_area(ports.storage.clone(), area);
        let result = seen();
        let r = result.clone();
        Loader::new(ports.clone()).load(&store, move |x| *r.borrow_mut() = Some(x));
        (ports, fetcher, store, result)
    }

    #[test]
    fn a_search_asks_nominatim_and_hands_on_the_places() {
        let (ports, fetcher, ..) = test_ports_with_fetcher();
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
        let (ports, fetcher, ..) = test_ports_with_fetcher();
        let found = seen();
        let f = found.clone();
        Loader::new(ports).search("x", move |r| *f.borrow_mut() = Some(r));
        fetcher.answer(Err("the server answered 429".into()));
        assert_eq!(en(&found.borrow_mut().take().unwrap().unwrap_err()), "the server answered 429");
    }

    #[test]
    fn a_place_in_a_tile_is_cut_out_of_that_tile_and_kept_with_its_checksum() {
        let area = in_one_tile();
        let (ports, fetcher, store, result) = loading(area.clone());
        assert_eq!(urls(&fetcher), vec!["data/metro/index.json"]);
        fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
        assert_eq!(urls(&fetcher), vec!["data/metro/12_9.json"]);
        assert!(result.borrow().is_none(), "nothing is done before the roads arrive");
        fetcher.answer(Ok(south()));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert!(fetcher.asked().is_empty(), "nothing more is asked: the roads come from the tile alone");
        let t = parsed(&south());
        assert_eq!(kept_network(&ports, &area), Some(t.network.clip_to(&t.frame, &area.bounds().frame())));
        assert_eq!(store.checksum(), Some(with_importer("new")));
        assert_eq!(names(&store), vec!["Across Street", "Inside Street", "Shared Street"], "Far Street is south of the box");
        assert_eq!(store.open().view(0).name, "Inland", "the city opens on the area");
    }

    #[test]
    fn a_place_whose_box_reaches_another_tile_is_read_from_both_with_each_road_once() {
        let area = on_the_edge();
        let (ports, fetcher, store, result) = loading(area.clone());
        fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
        assert_eq!(urls(&fetcher), vec!["data/metro/12_9.json", "data/metro/12_10.json"], "the tile holding the place first");
        fetcher.answer(Ok(south()));
        assert!(result.borrow().is_none(), "the page waits for every tile");
        fetcher.answer(Ok(north()));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        let mut merged = parsed(&south());
        merged.network.absorb(&parsed(&north()).network);
        let kept = kept_network(&ports, &area).unwrap();
        assert_eq!(kept, merged.network.clip_to(&AREA, &area.bounds().frame()));
        let shared = kept.roads.iter().filter(|r| r.name == "Shared Street").count();
        assert_eq!(shared, 1, "the road both tiles hold is kept once");
        assert_eq!(names(&store), vec!["North Street", "Shared Street"], "Inside Street and Across Street are south of the box");
        assert_eq!(store.checksum(), Some(with_importer("new")), "the checksum of the tile holding the place");
    }

    #[test]
    fn a_place_no_tile_holds_is_not_covered_and_nothing_else_is_asked() {
        // no index, as where the tiles are not deployed; an index that does not list the place's tile
        let answers: [Result<Vec<u8>, String>; 2] = [Err("the server answered 404".into()), Ok(INDEX.as_bytes().to_vec())];
        for (i, answer) in answers.into_iter().enumerate() {
            let (_, fetcher, store, result) = loading(Area::new("Paris", 48.85, 2.35));
            fetcher.answer(answer);
            assert!(fetcher.asked().is_empty(), "case {i}: nothing more is asked");
            let message = result.borrow_mut().take().unwrap().unwrap_err();
            assert_eq!(en(&message), "CityLoom has no street data for Paris.", "case {i}");
            assert_eq!(fr(&message), "CityLoom n’a pas de données de rues pour Paris.", "case {i}");
            assert!(!store.has_network(), "case {i}");
        }
    }

    #[test]
    fn an_index_that_is_not_one_is_said() {
        let (_, fetcher, store, result) = loading(in_one_tile());
        fetcher.answer(Ok(b"<html>".to_vec()));
        assert!(fetcher.asked().is_empty());
        assert!(en(&result.borrow_mut().take().unwrap().unwrap_err()).starts_with("the tile index was not understood ("));
        assert!(!store.has_network());
    }

    #[test]
    fn each_thing_that_goes_wrong_with_a_tile_is_said_and_nothing_is_kept() {
        let english =
            ["the roads of Edgeton could not be fetched (offline)", "the roads were not understood (", "the roads were not understood (data/metro/12_10.json)"];
        let french = [
            "les rues de Edgeton n’ont pas pu être obtenues (offline)",
            "les rues n’ont pas été comprises (",
            "les rues n’ont pas été comprises (data/metro/12_10.json)",
        ];
        let elsewhere = cityloom_model::Frame { south: 45.0, ..AREA };
        for step in 0..3 {
            let (_, fetcher, store, result) = loading(on_the_edge());
            fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
            fetcher.answer(Ok(south()));
            match step {
                0 => fetcher.answer(Err("offline".into())),
                1 => fetcher.answer(Ok(b"{not json".to_vec())),
                _ => fetcher.answer(Ok(tile(&elsewhere, &[(5, "North Street", &[(45.381, -74.080), (45.381, -74.076)])]))),
            };
            assert!(fetcher.asked().is_empty(), "step {step}: nothing more is asked");
            let message = result.borrow_mut().take().unwrap().unwrap_err();
            assert!(en(&message).starts_with(english[step]), "step {step}: {}", en(&message));
            assert!(fr(&message).starts_with(french[step]), "step {step}: {}", fr(&message));
            assert!(!store.has_network(), "step {step}: {message:?}");
        }
    }

    #[test]
    fn a_place_whose_tile_has_no_roads_in_its_box_is_said_and_not_kept() {
        let (_, fetcher, store, result) = loading(Area::new("Mid-field", 45.371, -74.078));
        fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
        fetcher.answer(Ok(tile(&AREA, &[(3, "Far Street", &[(45.364, -74.080), (45.364, -74.076)])])));
        let message = result.borrow_mut().take().unwrap().unwrap_err();
        assert_eq!(en(&message), "OpenStreetMap has no streets around Mid-field");
        assert_eq!(fr(&message), "OpenStreetMap n’a aucune rue autour de Mid-field");
        assert!(!store.has_network());
    }

    #[test]
    fn blocked_storage_is_said_and_not_taken_for_success() {
        let (mut ports, fetcher, ..) = test_ports_with_fetcher();
        let storage = Rc::new(MemoryStorage::default());
        ports.storage = storage.clone();
        let store = CityStore::for_area(ports.storage.clone(), in_one_tile());
        storage.blocked(true);
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        fetcher.answer(Ok(INDEX.as_bytes().to_vec()));
        fetcher.answer(Ok(south()));
        assert_eq!(en(&result.borrow_mut().take().unwrap().unwrap_err()), "the roads could not be kept: storage is blocked or full");
    }

    #[test]
    fn the_default_area_is_read_like_any_other_from_its_tile() {
        let d = default_area();
        let (index, tile) = covering(d.lat, d.lon, None, "Rue Rachel");
        let (_, fetcher, store, result) = loading(d);
        fetcher.answer(Ok(index));
        assert_eq!(urls(&fetcher), vec!["data/metro/0_0.json"]);
        fetcher.answer(Ok(tile));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert_eq!(names(&store), vec!["Rue Rachel"]);
        assert_eq!(store.checksum(), None, "the index gave no checksum");
    }

    /// The network an area was kept with before the load: one street, Only Street.
    fn kept_before() -> Vec<u8> {
        let (_, tile) = covering(1.0, 2.0, None, "Only Street");
        tile
    }

    /// What `kept_from` gives back: the fetcher, the store and what the load said.
    type Kept = (Rc<FakeFetcher>, CityStore, Seen<Result<(), Said>>);

    /// Testville with its network kept as made from a tile that had `checksum`, and its load begun.
    fn kept_from(checksum: Option<&str>) -> Kept {
        let (ports, fetcher, ..) = test_ports_with_fetcher();
        let area = Area::new("Testville", 1.0, 2.0);
        let store = CityStore::for_area(ports.storage.clone(), area.clone());
        let t = parsed(&kept_before());
        assert!(store.keep_network_from(&t.network.clip_to(&t.frame, &area.bounds().frame()), checksum));
        let result = seen();
        let r = result.clone();
        Loader::new(ports).load(&store, move |x| *r.borrow_mut() = Some(x));
        (fetcher, store, result)
    }

    /// The index of Testville's tile, whose checksum is `new`, and that tile, whose street is now `name`.
    fn testville_now(name: &str) -> (Vec<u8>, Vec<u8>) {
        covering(1.0, 2.0, Some("new"), name)
    }

    #[test]
    fn an_area_already_kept_is_not_read_again_while_its_tile_is_the_same() {
        let (fetcher, store, result) = kept_from(Some(&with_importer("new")));
        assert_eq!(urls(&fetcher), vec!["data/metro/index.json"]);
        fetcher.answer(Ok(testville_now("Changed Street").0));
        assert!(fetcher.asked().is_empty(), "only the index is asked: the tile is not fetched again");
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert_eq!(store.checksum(), Some(with_importer("new")));
        assert_eq!(names(&store), vec!["Only Street"]);
    }

    #[test]
    fn an_area_whose_tile_has_changed_is_read_again_and_keeps_the_new_checksum() {
        let (fetcher, store, result) = kept_from(Some("old"));
        let (index, tile) = testville_now("Changed Street");
        fetcher.answer(Ok(index));
        assert_eq!(urls(&fetcher), vec!["data/metro/0_0.json"]);
        assert!(result.borrow().is_none(), "the page waits for the new roads");
        fetcher.answer(Ok(tile));
        assert_eq!(result.borrow_mut().take(), Some(Ok(())));
        assert_eq!(store.checksum(), Some(with_importer("new")));
        assert_eq!(names(&store), vec!["Changed Street"]);
    }

    #[test]
    fn an_area_made_by_an_earlier_importer_or_before_checksums_were_is_read_again_once() {
        // what an earlier version kept: the tile's checksum as the index gives it, with no importer version; and nothing
        for checksum in [Some("new"), None] {
            let (fetcher, store, result) = kept_from(checksum);
            let (index, tile) = testville_now("Only Street");
            fetcher.answer(Ok(index));
            assert_eq!(urls(&fetcher), vec!["data/metro/0_0.json"], "{checksum:?}");
            fetcher.answer(Ok(tile));
            assert_eq!(result.borrow_mut().take(), Some(Ok(())), "{checksum:?}");
            assert_eq!(store.checksum(), Some(with_importer("new")), "{checksum:?}: kept as made by this importer, not read again");
        }
    }

    #[test]
    fn an_area_is_kept_as_it_is_when_nothing_says_its_data_changed_or_it_cannot_be_checked() {
        // no index, an index that is not one, a place no tile holds, and an index with no checksums
        let answers: [Result<Vec<u8>, String>; 4] = [
            Err("offline".into()),
            Ok(b"<html>".to_vec()),
            Ok(br#"{"lon0":50.0,"lat0":50.0,"dlon":0.0257,"dlat":0.018,"tiles":["0_0"],"checksums":{"0_0":"x"}}"#.to_vec()),
            Ok(covering(1.0, 2.0, None, "Changed Street").0),
        ];
        for (i, answer) in answers.into_iter().enumerate() {
            let (fetcher, store, result) = kept_from(Some("old"));
            fetcher.answer(answer);
            assert!(fetcher.asked().is_empty(), "case {i}: nothing more is asked");
            assert_eq!(result.borrow_mut().take(), Some(Ok(())), "case {i}");
            assert_eq!(store.checksum().as_deref(), Some("old"), "case {i}");
        }
    }

    #[test]
    fn a_refresh_that_fails_leaves_the_roads_that_are_kept() {
        // a tile that cannot be had, one that is not a tile, and one with no roads in the box
        let empty = tile(&AREA, &[]);
        let answers: [Result<Vec<u8>, String>; 3] = [Err("the server answered 404".into()), Ok(b"{not json".to_vec()), Ok(empty)];
        for (step, answer) in answers.into_iter().enumerate() {
            let (fetcher, store, result) = kept_from(Some("old"));
            fetcher.answer(Ok(testville_now("Changed Street").0));
            fetcher.answer(answer);
            assert!(fetcher.asked().is_empty(), "step {step}: nothing more is asked");
            assert_eq!(result.borrow_mut().take(), Some(Ok(())), "step {step}");
            assert_eq!(store.checksum().as_deref(), Some("old"), "step {step}");
            assert_eq!(names(&store), vec!["Only Street"], "step {step}");
        }
    }
}
