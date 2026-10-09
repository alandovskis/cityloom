//! The city the street, junction and map pages share, as it is kept between
//! visits. A page opens it, takes out what it edits, and writes back what it
//! made, against the city as it is in storage now so that a change made in
//! another tab is not written over.

use std::rc::Rc;

use osm_network::Network;

use crate::city::model::{City, Layout};
use crate::place::area::{Area, default_area};
use crate::shared::ports::Storage;

pub const CITY_KEY: &str = "cityloom-city";
/// The area the person is working in, which every page opens its city from.
pub const AREA_KEY: &str = "cityloom-area";
/// The street network of an area, as `osm_import` made it; the area's key follows.
pub const NETWORK_KEY: &str = "cityloom-network";
/// The checksum of the metro tile an area's network was made from, where it was; the area's key follows.
pub const CHECKSUM_KEY: &str = "cityloom-checksum";
pub const REGION_KEY: &str = "cityloom-region";

#[derive(Clone)]
pub struct CityStore {
    storage: Rc<dyn Storage>,
    /// The part of the world the city is made from.
    area: Area,
    /// In the tests: the hand-made city they are written against, kept under the keys of no area.
    #[cfg(test)]
    sample: bool,
}

impl CityStore {
    /// The store of the hand-made city the tests use. It is not part of the app.
    #[cfg(test)]
    pub fn sample(storage: Rc<dyn Storage>) -> CityStore {
        CityStore { storage, area: default_area(), sample: true }
    }

    /// The store of the city made from `area`. Its street network has to be
    /// kept first (`keep_network`); until it is, the city has no places.
    pub fn for_area(storage: Rc<dyn Storage>, area: Area) -> CityStore {
        CityStore {
            storage,
            area,
            #[cfg(test)]
            sample: false,
        }
    }

    /// The store of the area the person last chose, or of the default area when none was. Until that
    /// area's roads are kept (the page loads them before it opens the city) its city has no places.
    pub fn current(storage: Rc<dyn Storage>) -> CityStore {
        let area = CityStore::current_area(&*storage);
        CityStore::for_area(storage, area)
    }

    /// The area `current` would open, whether or not its roads are kept yet: the one last chosen, and the
    /// default one when none was or what was kept is not an area.
    pub fn current_area(storage: &dyn Storage) -> Area {
        storage.recall(AREA_KEY).and_then(|v| Area::from_query(&v)).unwrap_or_else(default_area)
    }

    /// Makes `area` the one every page opens. Says whether that was kept.
    pub fn choose(storage: &dyn Storage, area: &Area) -> bool {
        storage.remember(AREA_KEY, &area.to_query())
    }

    pub fn area(&self) -> &Area {
        &self.area
    }

    fn key(&self, base: &str) -> String {
        #[cfg(test)]
        if self.sample {
            return base.to_string();
        }
        format!("{base}:{}", self.area.key())
    }

    /// Keeps the street network the area's city is made from. Says whether it was kept.
    pub fn keep_network(&self, network: &Network) -> bool {
        self.keep_network_from(network, None)
    }

    /// The same for a network made from a metro tile that had `checksum`, which is kept with it so that a
    /// later visit can tell whether the tile has changed.
    pub fn keep_network_from(&self, network: &Network, checksum: Option<&str>) -> bool {
        serde_json::to_string(network).is_ok_and(|json| self.storage.remember(&self.key(NETWORK_KEY), &json))
            && self.storage.remember(&self.key(CHECKSUM_KEY), checksum.unwrap_or_default())
    }

    /// The checksum of the tile the area's network was made from, if it was made from one that had it.
    pub fn checksum(&self) -> Option<String> {
        self.storage.recall(&self.key(CHECKSUM_KEY)).filter(|c| !c.is_empty())
    }

    /// Whether the area's street network is kept.
    pub fn has_network(&self) -> bool {
        #[cfg(test)]
        if self.sample {
            return true;
        }
        self.network().is_some()
    }

    fn network(&self) -> Option<Network> {
        serde_json::from_str(&self.storage.recall(&self.key(NETWORK_KEY))?).ok()
    }

    fn layout(&self) -> Layout {
        #[cfg(test)]
        if self.sample {
            return Layout::sample();
        }
        Layout::from_network(&self.network().unwrap_or_default(), &self.area.name)
    }

    /// The city as it is kept, or as first laid out when nothing usable is.
    pub fn open(&self) -> City {
        City::load_on(self.layout(), &self.storage.recall(&self.key(CITY_KEY)).unwrap_or_default())
    }

    /// Applies `change` to the city as it is kept now and keeps the result.
    /// Says whether the change was taken and kept.
    pub fn write(&self, change: impl FnOnce(&mut City) -> bool) -> bool {
        let mut city = self.open();
        change(&mut city) && self.storage.remember(&self.key(CITY_KEY), &city.save())
    }

    /// Puts every street and junction back as first laid out, and keeps that.
    pub fn reset(&self) -> bool {
        self.write(|city| {
            city.reset();
            true
        })
    }

    /// The index of the region the person chose, which the pages share; the
    /// first when none is chosen or it is unknown.
    pub fn region(&self) -> usize {
        let id = self.storage.recall(REGION_KEY);
        crate::shared::catalogue::REGIONS.iter().position(|r| Some(r.id) == id.as_deref()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::test_ports;

    fn store() -> (CityStore, Rc<crate::shared::ports::MemoryStorage>) {
        let (ports, _, storage) = test_ports();
        (CityStore::sample(ports.storage), storage)
    }

    #[test]
    fn the_checksum_of_the_data_an_area_was_made_from_is_kept_with_its_network_and_for_that_area_only() {
        let (ports, ..) = test_ports();
        let area = |name, lon| CityStore::for_area(ports.storage.clone(), Area::new(name, 1.0, lon));
        let (a, b) = (area("A", 2.0), area("B", 3.0));
        let network = Network::default();
        assert_eq!(a.checksum(), None);
        assert!(a.keep_network_from(&network, Some("ab12")));
        assert_eq!((a.checksum().as_deref(), b.checksum()), (Some("ab12"), None));
        assert!(a.keep_network_from(&network, None), "a network made from data with no checksum has none");
        assert_eq!(a.checksum(), None);
        assert!(b.keep_network(&network) && b.checksum().is_none());
    }

    #[test]
    fn a_new_visit_opens_the_city_as_first_laid_out() {
        let (s, _) = store();
        let v = s.open().view(0);
        assert_eq!(v.edited, 0);
        assert!(v.places > 0);
    }

    #[test]
    fn what_a_page_writes_is_kept_and_found_by_the_next() {
        let (s, _) = store();
        let street = s.open().view(0).edges[0].uid;
        assert!(s.write(|c| {
            let mut e = c.street_editor(street, 0).unwrap();
            let u = e.view().segments[0].uid;
            e.nudge_width(u, 100);
            c.keep_street(street, e.snapshot())
        }));
        assert_eq!(s.open().view(0).edited, 1);
    }

    #[test]
    fn a_change_that_is_not_taken_is_not_kept() {
        let (s, st) = store();
        assert!(!s.write(|_| false));
        assert_eq!(st.recall(CITY_KEY), None);
    }

    #[test]
    fn a_blocked_storage_says_the_change_was_not_kept() {
        let (s, st) = store();
        st.blocked(true);
        assert!(!s.write(|_| true));
    }

    #[test]
    fn a_page_writes_against_what_another_tab_kept_not_over_it() {
        let (s, _) = store();
        let v = s.open().view(0);
        let (a, b) = (v.edges[0].uid, v.edges[1].uid);
        let nudge = |street: u32| {
            move |c: &mut City| {
                let mut e = c.street_editor(street, 0).unwrap();
                let u = e.view().segments[0].uid;
                e.nudge_width(u, 100);
                c.keep_street(street, e.snapshot())
            }
        };
        // Two pages opened the city at the same time; each writes its own street.
        assert!(s.write(nudge(a)));
        assert!(s.write(nudge(b)));
        assert_eq!(s.open().view(0).edited, 2);
    }

    #[test]
    fn starting_over_puts_everything_back_and_keeps_that() {
        let (s, _) = store();
        let street = s.open().view(0).edges[0].uid;
        s.write(|c| {
            let mut e = c.street_editor(street, 0).unwrap();
            let u = e.view().segments[0].uid;
            e.nudge_width(u, 100);
            c.keep_street(street, e.snapshot())
        });
        assert_eq!(s.open().view(0).edited, 1);
        assert!(s.reset());
        assert_eq!(s.open().view(0).edited, 0);
    }

    fn tiny_network() -> Network {
        use osm_network::{Control, Lane, LaneKind, Node, Road, Way};
        let node =
            |id, x_m, junction| Node { id, osm_nodes: vec![id as i64], osm_versions: Default::default(), x_m, y_m: 0.0, junction, control: Control::None };
        let lane = |kind, way| Lane { kind, way, width_m: 3.0, hours: None };
        Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, false), node(2, 100.0, false)],
            roads: vec![Road {
                id: 1,
                osm_ways: vec![7],
                osm_versions: Default::default(),
                name: Some("Only Street".into()),
                highway: "residential".into(),
                from: 1,
                to: 2,
                lanes: vec![lane(LaneKind::Driving, Way::Forward), lane(LaneKind::Driving, Way::Backward)],
                points: vec![(0.0, 0.0), (100.0, 0.0)],
            }],
        }
    }

    #[test]
    fn an_area_without_its_network_is_an_empty_city_until_the_network_is_kept() {
        let (ports, ..) = test_ports();
        let s = CityStore::for_area(ports.storage, Area::new("Testville", 1.0, 2.0));
        assert!(!s.has_network());
        let v = s.open().view(0);
        assert_eq!((v.name.as_str(), v.places), ("Testville", 0));
        assert!(s.keep_network(&tiny_network()));
        assert!(s.has_network());
        let v = s.open().view(0);
        assert_eq!((v.name.as_str(), v.edges.len()), ("Testville", 1));
        assert_eq!(
            crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), crate::shared::units::Units::Metres, &v.edges[0].kind),
            "Only Street"
        );
    }

    #[test]
    fn what_is_edited_in_an_area_is_kept_under_that_area_and_not_under_the_test_city_s() {
        let (ports, _, storage) = test_ports();
        let area = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        area.keep_network(&tiny_network());
        let street = area.open().view(0).edges[0].uid;
        assert!(area.write(|c| {
            let mut e = c.street_editor(street, 0).unwrap();
            let u = e.view().segments[0].uid;
            e.nudge_width(u, 100);
            c.keep_street(street, e.snapshot())
        }));
        assert_eq!(area.open().view(0).edited, 1);
        assert_eq!(storage.recall(CITY_KEY), None, "the key of no area is untouched");
        assert_eq!(CityStore::sample(ports.storage.clone()).open().view(0).edited, 0);
        let other = CityStore::for_area(ports.storage, Area::new("Elsewhere", 5.0, 6.0));
        assert!(!other.has_network());
    }

    #[test]
    fn pages_open_the_area_that_was_chosen_and_the_default_before_one_is() {
        let (ports, _, storage) = test_ports();
        assert_eq!(CityStore::current_area(&*storage), default_area());
        let chosen = Area::new("Testville", 1.0, 2.0);
        assert!(CityStore::choose(&*storage, &chosen));
        assert_eq!(CityStore::current_area(&*storage), chosen);
        // its roads are not kept yet: the pages have its city, with no places in it
        let store = CityStore::current(ports.storage.clone());
        assert_eq!(store.area(), &chosen);
        assert!(!store.has_network());
        assert_eq!(store.open().view(0).places, 0);
        store.keep_network(&tiny_network());
        assert_eq!(CityStore::current(ports.storage.clone()).open().view(0).name, "Testville");
        storage.remember(AREA_KEY, "area=nonsense");
        assert_eq!(CityStore::current_area(&*storage), default_area());
    }

    #[test]
    fn what_an_older_visit_kept_as_the_sample_city_is_the_default_area_now() {
        let (ports, _, storage) = test_ports();
        storage.remember(AREA_KEY, "sample");
        assert_eq!(CityStore::current_area(&*storage), default_area());
        assert_eq!(CityStore::current(ports.storage).area(), &default_area());
    }

    #[test]
    fn a_damaged_network_is_not_used() {
        let (ports, _, storage) = test_ports();
        let s = CityStore::for_area(ports.storage, Area::new("Testville", 1.0, 2.0));
        storage.remember("cityloom-network:1.0000,2.0000", "{not json");
        assert!(!s.has_network());
        let v = s.open().view(0);
        assert_eq!((v.name.as_str(), v.places), ("Testville", 0));
    }

    #[test]
    fn the_chosen_region_is_read_from_storage_with_the_first_as_the_default() {
        let (s, st) = store();
        assert_eq!(s.region(), 0);
        st.remember(REGION_KEY, "united-kingdom");
        assert_eq!(s.region(), 3);
        st.remember(REGION_KEY, "nowhere");
        assert_eq!(s.region(), 0);
    }
}
