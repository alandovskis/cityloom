//! Choosing the place a city is made from: a search by name, the places it finds, and the
//! opening of the one chosen. The view binds to this and decides nothing.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;

use super::area::Area;
use super::loader::Loader;
use super::nominatim::Place;
use crate::city::store::CityStore;
use crate::shared::ports::Ports;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Searching,
    Found,
    Nothing,
    /// The streets of the named place are being got.
    Loading(String),
    Failed(String),
}

/// A place found, as a row of the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub name: String,
    pub detail: String,
}

pub struct AreaVm {
    ports: Ports,
    loader: Loader,
    text: ArcRwSignal<String>,
    found: ArcRwSignal<Vec<Place>>,
    status: ArcRwSignal<Status>,
    active: ArcRwSignal<Option<usize>>,
    /// Which search the answer to is wanted; an older one that arrives late is dropped.
    turn: Cell<u32>,
}

/// The name an area is called: where it is, as far as the second place name.
fn area_name(display: &str) -> String {
    display.split(',').map(str::trim).filter(|p| !p.is_empty()).take(2).collect::<Vec<_>>().join(", ")
}

impl AreaVm {
    /// How many places the list shows.
    pub const SHOWN: usize = 5;

    pub fn new(ports: Ports) -> Rc<AreaVm> {
        Rc::new(AreaVm {
            loader: Loader::new(ports.clone()),
            ports,
            text: ArcRwSignal::new(String::new()),
            found: ArcRwSignal::new(Vec::new()),
            status: ArcRwSignal::new(Status::Idle),
            active: ArcRwSignal::new(None),
            turn: Cell::new(0),
        })
    }

    pub fn text(&self) -> String {
        self.text.get()
    }

    /// Typing changes the text and clears what an earlier search found; nothing is asked of the
    /// server until the person asks (`search`), as its usage policy has it.
    pub fn set_text(&self, text: &str) {
        self.text.set(text.to_string());
        if !matches!(self.status.get_untracked(), Status::Loading(_)) {
            self.found.set(Vec::new());
            self.active.set(None);
            self.status.set(Status::Idle);
        }
    }

    pub fn status(&self) -> Status {
        self.status.get()
    }

    /// Whether the streets of a place are being got.
    pub fn busy(&self) -> bool {
        matches!(self.status.get(), Status::Loading(_) | Status::Searching)
    }

    pub fn rows(&self) -> Vec<Row> {
        self.found
            .get()
            .iter()
            .take(Self::SHOWN)
            .map(|p| {
                let name = area_name(&p.name);
                let detail = p.name.strip_prefix(&name).unwrap_or(&p.name).trim_start_matches([',', ' ']).to_string();
                Row { name, detail }
            })
            .collect()
    }

    pub fn active(&self) -> Option<usize> {
        self.active.get()
    }

    /// Moves the chosen row down (`1`) or up (`-1`), staying in the list.
    pub fn move_active(&self, by: i32) {
        let n = self.rows().len() as i32;
        if n == 0 {
            return;
        }
        let next = match self.active.get_untracked() {
            None => {
                if by > 0 {
                    0
                } else {
                    n - 1
                }
            }
            Some(i) => (i as i32 + by).clamp(0, n - 1),
        };
        self.active.set(Some(next as usize));
    }

    /// What the list says about itself, for the person and a screen reader; None when it has nothing to say.
    pub fn note(&self) -> Option<String> {
        match self.status.get() {
            Status::Idle => None,
            Status::Searching => Some("Searching…".to_string()),
            Status::Found => {
                let n = self.rows().len();
                Some(format!("{n} {} found. Use the arrow keys, then Enter.", if n == 1 { "place" } else { "places" }))
            }
            Status::Nothing => Some("No place found. Try a city, a neighbourhood or an address.".to_string()),
            Status::Loading(name) => Some(format!("Getting the streets of {name}…")),
            Status::Failed(why) => Some(why),
        }
    }

    fn say(&self) {
        if let Some(note) = self.note() {
            self.ports.announcer.say(&note);
        }
    }

    /// Looks the text up by name.
    pub fn search(self: &Rc<Self>) {
        let text = self.text.get_untracked();
        if text.trim().is_empty() || matches!(self.status.get_untracked(), Status::Loading(_)) {
            return;
        }
        let turn = self.turn.get() + 1;
        self.turn.set(turn);
        self.status.set(Status::Searching);
        self.say();
        let vm = self.clone();
        self.loader.search(&text, move |result| {
            if vm.turn.get() != turn {
                return;
            }
            match result {
                Ok(places) if places.is_empty() => vm.status.set(Status::Nothing),
                Ok(places) => {
                    vm.found.set(places);
                    vm.active.set(Some(0));
                    vm.status.set(Status::Found);
                }
                Err(why) => vm.status.set(Status::Failed(why)),
            }
            vm.say();
        });
    }

    /// Opens the place the arrow keys are on, or the first.
    pub fn choose_active(self: &Rc<Self>) {
        let i = self.active.get_untracked().unwrap_or(0);
        self.choose(i);
    }

    /// Makes a city of the streets round the place in row `i` and opens its map. If the streets cannot
    /// be got, the page stays as it is and says why.
    pub fn choose(self: &Rc<Self>, i: usize) {
        let Some(place) = self.found.get_untracked().get(i).cloned() else { return };
        if matches!(self.status.get_untracked(), Status::Loading(_)) {
            return;
        }
        let area = Area::new(&area_name(&place.name), place.lat, place.lon);
        self.status.set(Status::Loading(area.name.clone()));
        self.say();
        let store = CityStore::for_area(self.ports.storage.clone(), area.clone());
        let vm = self.clone();
        self.loader.load(&store, move |result| {
            match result {
                Ok(()) if CityStore::choose(&*vm.ports.storage, &area) => vm.ports.navigator.go("map.html"),
                Ok(()) => vm.status.set(Status::Failed("The place could not be kept: storage is blocked.".to_string())),
                Err(why) => vm.status.set(Status::Failed(why)),
            }
            if matches!(vm.status.get_untracked(), Status::Failed(_)) {
                vm.say();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::{MemoryStorage, RecordingAnnouncer, test_ports_with_services};

    const BERLIN: &[u8] = br#"[{"display_name":"Kreuzberg, Friedrichshain-Kreuzberg, Berlin, 10999, Germany","lat":"52.4990","lon":"13.4030","boundingbox":["52.48","52.51","13.38","13.43"]},
        {"display_name":"Kreuzberg, Bavaria, Germany","lat":"50.0","lon":"10.0","boundingbox":["49.9","50.1","9.9","10.1"]}]"#;

    const NETWORK: &str = r#"{"left_hand":false,"nodes":[{"id":1,"osm_nodes":[1],"x_m":0.0,"y_m":0.0,"junction":false,"control":"none"},
        {"id":2,"osm_nodes":[2],"x_m":90.0,"y_m":0.0,"junction":false,"control":"none"}],
        "roads":[{"id":1,"osm_ways":[5],"name":"Only Street","highway":"residential","from":1,"to":2,
        "lanes":[{"kind":"driving","way":"forward","width_m":3.0}],"points":[[0.0,0.0],[90.0,0.0]]}]}"#;

    struct Fixture {
        vm: Rc<AreaVm>,
        fetcher: Rc<crate::shared::ports::FakeFetcher>,
        importer: Rc<crate::shared::ports::FakeImporter>,
        navigator: Rc<crate::shared::ports::RecordingNavigator>,
        storage: Rc<MemoryStorage>,
        said: Rc<RecordingAnnouncer>,
    }

    fn fixture() -> Fixture {
        let (mut ports, fetcher, importer, navigator) = test_ports_with_services();
        let storage = Rc::new(MemoryStorage::default());
        let said = Rc::new(RecordingAnnouncer::default());
        ports.storage = storage.clone();
        ports.announcer = said.clone();
        Fixture { vm: AreaVm::new(ports), fetcher, importer, navigator, storage, said }
    }

    fn found(f: &Fixture) {
        f.vm.set_text("Kreuzberg");
        f.vm.search();
        f.fetcher.answer(Ok(BERLIN.to_vec()));
    }

    #[test]
    fn nothing_is_asked_until_the_person_searches_and_an_empty_text_asks_nothing() {
        let f = fixture();
        f.vm.set_text("Kreuz");
        f.vm.set_text("Kreuzberg");
        assert!(f.fetcher.asked().is_empty());
        f.vm.set_text("   ");
        f.vm.search();
        assert!(f.fetcher.asked().is_empty());
        assert_eq!(f.vm.status(), Status::Idle);
    }

    #[test]
    fn a_search_says_it_is_searching_then_lists_what_it_found_with_the_first_chosen() {
        let f = fixture();
        f.vm.set_text("Kreuzberg");
        f.vm.search();
        assert_eq!(f.vm.status(), Status::Searching);
        assert!(f.vm.busy());
        f.fetcher.answer(Ok(BERLIN.to_vec()));
        assert_eq!(f.vm.status(), Status::Found);
        let rows = f.vm.rows();
        assert_eq!(rows[0], Row { name: "Kreuzberg, Friedrichshain-Kreuzberg".into(), detail: "Berlin, 10999, Germany".into() });
        assert_eq!(f.vm.active(), Some(0));
        assert_eq!(f.vm.note().unwrap(), "2 places found. Use the arrow keys, then Enter.");
        assert_eq!(f.said.take(), vec!["Searching…", "2 places found. Use the arrow keys, then Enter."]);
    }

    #[test]
    fn the_arrow_keys_stay_inside_the_list() {
        let f = fixture();
        found(&f);
        f.vm.move_active(1);
        assert_eq!(f.vm.active(), Some(1));
        f.vm.move_active(1);
        assert_eq!(f.vm.active(), Some(1));
        f.vm.move_active(-5);
        assert_eq!(f.vm.active(), Some(0));
    }

    #[test]
    fn a_search_with_no_result_or_a_failure_says_so_and_typing_again_clears_it() {
        let f = fixture();
        f.vm.set_text("zzzz");
        f.vm.search();
        f.fetcher.answer(Ok(b"[]".to_vec()));
        assert_eq!(f.vm.status(), Status::Nothing);
        assert!(f.vm.note().unwrap().starts_with("No place found"));
        f.vm.set_text("Berlin");
        assert_eq!(f.vm.status(), Status::Idle);
        f.vm.search();
        f.fetcher.answer(Err("the server answered 429".into()));
        assert_eq!(f.vm.status(), Status::Failed("the server answered 429".into()));
        assert_eq!(f.vm.note().unwrap(), "the server answered 429");
    }

    #[test]
    fn the_answer_to_an_older_search_does_not_replace_a_newer() {
        let f = fixture();
        f.vm.set_text("Kreuzberg");
        f.vm.search();
        f.vm.set_text("Mitte");
        f.vm.search();
        f.fetcher.answer(Ok(BERLIN.to_vec()));
        f.fetcher.answer(Ok(b"[]".to_vec()));
        assert_eq!(f.vm.status(), Status::Nothing);
        assert!(f.vm.rows().is_empty());
    }

    #[test]
    fn choosing_a_place_gets_its_streets_then_makes_it_the_area_and_opens_the_map() {
        let f = fixture();
        found(&f);
        f.vm.choose(0);
        assert_eq!(f.vm.status(), Status::Loading("Kreuzberg, Friedrichshain-Kreuzberg".into()));
        assert_eq!(f.vm.note().unwrap(), "Getting the streets of Kreuzberg, Friedrichshain-Kreuzberg…");
        assert!(f.navigator.take().is_empty(), "not before the streets are here");
        f.fetcher.answer(Err("no tiles".into())); // the index of the metro tiles: none
        f.fetcher.answer(Ok(b"<osm/>".to_vec()));
        f.importer.answer(Ok(NETWORK.to_string()));
        assert_eq!(f.navigator.take(), vec!["map.html"]);
        let area = CityStore::current_area(&*f.storage).unwrap();
        assert_eq!((area.name.as_str(), area.key().as_str()), ("Kreuzberg, Friedrichshain-Kreuzberg", "52.4990,13.4030"));
        assert_eq!(CityStore::current(f.storage.clone()).open().view(0).name, "Kreuzberg, Friedrichshain-Kreuzberg");
    }

    #[test]
    fn a_place_whose_streets_cannot_be_got_is_not_made_the_area_and_says_why() {
        let f = fixture();
        found(&f);
        f.vm.choose(1);
        f.fetcher.answer(Err("no tiles".into()));
        f.fetcher.answer(Err("the server answered 504".into()));
        assert!(matches!(f.vm.status(), Status::Failed(m) if m.contains("504")));
        assert!(f.navigator.take().is_empty());
        assert_eq!(CityStore::current_area(&*f.storage), Some(crate::place::area::default_area()), "still the default");
    }

    #[test]
    fn a_second_choice_while_one_is_loading_is_ignored() {
        let f = fixture();
        found(&f);
        f.vm.choose(0);
        f.vm.choose(1);
        assert_eq!(f.fetcher.asked().len(), 1);
    }
}
