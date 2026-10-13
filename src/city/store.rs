//! The city the street, junction and map pages share, as it is kept between
//! visits. A page opens it, takes out what it edits, and writes back what it
//! made, against the city as it is in storage now so that a change made in
//! another tab is not written over.

use std::rc::Rc;

use osm_network::Network;

use crate::city::edits::{self, DAMAGED_KEY, Document, Draft, EDITS_KEY, Edits};
use crate::city::model::{City, Layout};
use crate::junction::model::State;
use crate::place::area::{Area, default_area};
use crate::shared::ports::Storage;
use crate::street::model::Street;

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
    /// In the tests: the hand-made city with none of the OpenStreetMap ids its places are made from.
    #[cfg(test)]
    keyless: bool,
}

impl CityStore {
    /// The store of the hand-made city the tests use. It is not part of the app.
    #[cfg(test)]
    pub fn sample(storage: Rc<dyn Storage>) -> CityStore {
        CityStore { storage, area: default_area(), sample: true, keyless: false }
    }

    /// The same, with every place stripped of the OpenStreetMap ids its key is made from, so that none has a key.
    #[cfg(test)]
    pub fn sample_without_osm_ids(storage: Rc<dyn Storage>) -> CityStore {
        CityStore { keyless: true, ..CityStore::sample(storage) }
    }

    /// The store of the city made from `area`. Its street network has to be
    /// kept first (`keep_network`); until it is, the city has no places.
    pub fn for_area(storage: Rc<dyn Storage>, area: Area) -> CityStore {
        CityStore {
            storage,
            area,
            #[cfg(test)]
            sample: false,
            #[cfg(test)]
            keyless: false,
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

    /// The same for a network made from a metro tile that had `checksum`, which is kept with it so that a later
    /// visit can tell whether the tile has changed.
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
            let mut layout = Layout::sample();
            if self.keyless {
                layout.nodes.iter_mut().for_each(|n| n.source.clear());
                layout.edges.iter_mut().for_each(|e| e.section.source.clear());
            }
            return layout;
        }
        Layout::from_network(&self.network().unwrap_or_default(), &self.area.name)
    }

    fn document(&self) -> Document {
        edits::parse(self.storage.recall(EDITS_KEY).as_deref())
    }

    /// The edits as kept: none when there are none, they are damaged or they come from a newer version.
    fn edits(&self) -> Edits {
        match self.document() {
            Document::Current(e) => e,
            _ => Edits::new(),
        }
    }

    /// The city as it is kept, or as first laid out when nothing usable is.
    pub fn open(&self) -> City {
        City::load_on(self.layout(), &self.edits())
    }

    /// Takes `street` as street `edge`'s state, as the city as kept now allows, and keeps that one record (in place
    /// of the record the street took its edit from, where that was kept under another key). Says
    /// whether it was taken and kept (not where the street has no key or the document is a newer version's).
    pub fn write_street(&self, edge: u32, street: Street) -> bool {
        let mut city = self.open();
        if !city.keep_street(edge, street) {
            return false;
        }
        let Some((key, record)) = city.street_record(edge) else { return false };
        // the record the street took, under another key, is replaced: left, it would bring the older edit back
        let took = city.took_street(edge).filter(|&k| k != key).map(str::to_string);
        self.upsert(|draft| {
            if let Some(old) = took {
                draft.remove_street(&old);
            }
            draft.put_street(&key, &record)
        })
    }

    /// The same for the junction at `node`.
    pub fn write_junction(&self, node: u32, state: State) -> bool {
        let mut city = self.open();
        if !city.keep_junction(node, state) {
            return false;
        }
        let Some((key, record)) = city.junction_record(node) else { return false };
        let took = city.took_junction(node).filter(|&k| k != key).map(str::to_string);
        self.upsert(|draft| {
            if let Some(old) = took {
                draft.remove_junction(&old);
            }
            draft.put_junction(&key, &record)
        })
    }

    /// Changes the document as it is kept now and keeps it, every record `change` leaves alone exactly as it was
    /// kept (one this build cannot read too). A damaged document is copied aside first, unless a copy is already
    /// kept (the first damage is the one worth keeping); one from a newer version is not written over.
    fn upsert(&self, change: impl FnOnce(&mut Draft) -> bool) -> bool {
        let mut draft = match edits::draft(self.storage.recall(EDITS_KEY).as_deref()) {
            Ok(draft) => draft,
            Err(Document::Future) => return false,
            Err(Document::Damaged(raw)) => {
                if self.storage.recall(DAMAGED_KEY).is_none() && !self.storage.remember(DAMAGED_KEY, &raw) {
                    return false;
                }
                Draft::new()
            }
            Err(_) => Draft::new(),
        };
        change(&mut draft) && self.storage.remember(EDITS_KEY, &draft.to_json())
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
    use crate::city::edits::{self, Document, Edits, StreetRecord};
    use crate::shared::ports::{MemoryStorage, test_ports};
    use crate::street::model::Street;

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

    /// Street `uid` of the sample city with 100 mm moved between its first two pieces (the class and total width
    /// are kept).
    fn narrowed(store: &CityStore, uid: u32) -> Street {
        let mut s = store.open().street_editor(uid, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        s
    }

    /// The edits document as kept.
    fn document(storage: &MemoryStorage) -> Edits {
        let Document::Current(e) = edits::parse(storage.recall(EDITS_KEY).as_deref()) else { panic!("no current document") };
        e
    }

    #[test]
    fn an_edit_is_kept_in_the_edits_document_under_its_key() {
        let (store, storage) = store();
        let edited = narrowed(&store, 1);
        assert!(store.write_street(1, edited.clone()));
        let e = document(&storage);
        assert_eq!(e.streets.len(), 1);
        assert!(e.streets.keys().next().unwrap().starts_with("w2000~"), "the key is built from the OSM ids");
        assert_eq!(store.open().street_editor(1, 0).unwrap().snapshot(), edited, "and it is what the store opens");
        assert_eq!(store.open().view(0).edited, 1);
    }

    #[test]
    fn a_junction_is_kept_in_the_edits_document_under_its_key() {
        let (store, storage) = store();
        let node = store.open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let mut j = store.open().junction_editor(node, 0).unwrap();
        let arm = j.current().arms[0].uid;
        assert!(j.set_corner(arm, 7_000));
        assert!(store.write_junction(node, j.snapshot()));
        assert_eq!(document(&storage).junctions.len(), 1);
        assert_eq!(store.open().view(0).edited, 1);
    }

    #[test]
    fn writing_one_place_leaves_every_other_record_as_it_was() {
        let (store, storage) = store();
        assert!(store.write_street(1, narrowed(&store, 1)));
        let first = document(&storage).streets.clone();
        assert!(store.write_street(2, narrowed(&store, 2)));
        let after = document(&storage);
        assert_eq!(after.streets.len(), 2);
        for (key, record) in &first {
            assert_eq!(&after.streets[key], record, "the record of {key} was rewritten");
        }
    }

    #[test]
    fn a_record_that_matches_no_place_survives_a_write_of_another_place() {
        let (store, storage) = store();
        let mut stranger = Edits::new();
        let record = StreetRecord { ways: [77].into(), nodes: [1, 2].into(), leaving: [(1, 90)].into(), street: narrowed(&store, 3) };
        stranger.streets.insert("w77~n1~n2".into(), record.clone());
        storage.remember(EDITS_KEY, &stranger.to_json());
        assert!(store.write_street(1, narrowed(&store, 1)));
        assert_eq!(document(&storage).streets["w77~n1~n2"], record);
    }

    #[test]
    fn an_area_s_edit_does_not_remove_the_records_of_other_areas() {
        let (ports, _, storage) = test_ports();
        let area = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        area.keep_network(&tiny_network());
        let mut elsewhere = Edits::new();
        let record = StreetRecord {
            ways: [500].into(),
            nodes: [8, 9].into(),
            leaving: [(8, 90)].into(),
            street: CityStore::sample(ports.storage).open().street_editor(1, 0).unwrap().snapshot(),
        };
        elsewhere.streets.insert("w500~n8~n9".into(), record.clone());
        storage.remember(EDITS_KEY, &elsewhere.to_json());
        let street = area.open().view(0).edges[0].uid;
        let mut s = area.open().street_editor(street, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(area.write_street(street, s));
        let e = document(&storage);
        assert_eq!(e.streets.len(), 2);
        assert_eq!(e.streets["w500~n8~n9"], record);
    }

    #[test]
    fn a_damaged_document_is_backed_up_before_the_first_write_replaces_it() {
        let (store, storage) = store();
        storage.remember(EDITS_KEY, "{broken");
        assert!(store.write_street(1, narrowed(&store, 1)));
        assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("{broken"));
        assert_eq!(document(&storage).streets.len(), 1);
    }

    #[test]
    fn a_copy_of_a_damaged_document_already_kept_is_not_written_over() {
        let (store, storage) = store();
        storage.remember(DAMAGED_KEY, "{first damage");
        storage.remember(EDITS_KEY, "{second damage");
        assert!(store.write_street(1, narrowed(&store, 1)));
        assert_eq!(storage.recall(DAMAGED_KEY).as_deref(), Some("{first damage"));
        assert_eq!(document(&storage).streets.len(), 1);
    }

    #[test]
    fn a_record_this_build_cannot_read_survives_a_write_of_another_place_unchanged() {
        let (store, storage) = store();
        let newer = r#"{"version":3,"streets":{"w9~n1~n2":{"street":{"class":"Hovercraft"}}},"junctions":{}}"#;
        storage.remember(EDITS_KEY, newer);
        assert!(store.write_street(1, narrowed(&store, 1)));
        let kept: serde_json::Value = serde_json::from_str(&storage.recall(EDITS_KEY).unwrap()).unwrap();
        assert_eq!(kept["streets"]["w9~n1~n2"], serde_json::json!({"street": {"class": "Hovercraft"}}));
        assert_eq!(document(&storage).streets.len(), 1, "the one record this build reads is the new one");
    }

    #[test]
    fn a_damaged_document_that_cannot_be_backed_up_is_not_replaced() {
        let (store, storage) = store();
        storage.remember(EDITS_KEY, "{broken");
        storage.blocked(true);
        assert!(!store.write_street(1, narrowed(&store, 1)));
        storage.blocked(false);
        assert_eq!(storage.recall(EDITS_KEY).as_deref(), Some("{broken"));
    }

    #[test]
    fn a_document_from_a_newer_version_is_left_alone_and_the_write_says_it_was_not_kept() {
        let (store, storage) = store();
        let newer = r#"{"version":4,"streets":{},"junctions":{},"future":true}"#;
        storage.remember(EDITS_KEY, newer);
        assert!(!store.write_street(1, narrowed(&store, 1)));
        assert_eq!(storage.recall(EDITS_KEY).as_deref(), Some(newer));
        assert_eq!(storage.recall(DAMAGED_KEY), None, "a newer document is not a damaged one");
        assert_eq!(store.open().view(0).edited, 0, "and it is not read");
    }

    #[test]
    fn a_place_with_no_key_is_not_written() {
        let (ports, _, storage) = test_ports();
        let store = CityStore::sample_without_osm_ids(ports.storage);
        let s = narrowed(&store, 1);
        assert!(!store.write_street(1, s));
        assert!(storage.recall(EDITS_KEY).is_none_or(|_| document(&storage).streets.is_empty()));
    }

    #[test]
    fn a_change_that_is_not_taken_is_not_kept() {
        let (store, storage) = store();
        let mut wrong = narrowed(&store, 1);
        wrong.row_mm += 100; // the street no longer has the room it had
        assert!(!store.write_street(1, wrong));
        assert!(!store.write_street(999, narrowed(&store, 1)), "no such street");
        assert_eq!(storage.recall(EDITS_KEY), None, "nothing is written for a change that is not taken");
    }

    #[test]
    fn a_blocked_storage_says_the_change_was_not_kept() {
        let (store, storage) = store();
        let edited = narrowed(&store, 1);
        storage.blocked(true);
        assert!(!store.write_street(1, edited));
    }

    #[test]
    fn another_tabs_change_to_a_different_place_is_not_written_over() {
        let (ports, _, storage) = test_ports();
        let (tab_a, tab_b) = (CityStore::sample(ports.storage.clone()), CityStore::sample(ports.storage));
        // both tabs opened the city before either wrote
        let (a, b) = (narrowed(&tab_a, 1), narrowed(&tab_b, 2));
        assert!(tab_a.write_street(1, a));
        assert!(tab_b.write_street(2, b));
        assert_eq!(document(&storage).streets.len(), 2);
        assert_eq!(tab_a.open().view(0).edited, 2);
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
            crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), crate::shared::units::Units::Metres, &v.edges[0].called),
            "Only Street"
        );
    }

    #[test]
    fn what_is_edited_in_an_area_is_found_again_there_and_is_not_taken_by_another_city() {
        let (ports, _, storage) = test_ports();
        let area = CityStore::for_area(ports.storage.clone(), Area::new("Testville", 1.0, 2.0));
        area.keep_network(&tiny_network());
        let street = area.open().view(0).edges[0].uid;
        let mut s = area.open().street_editor(street, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(area.write_street(street, s));
        assert_eq!(area.open().view(0).edited, 1);
        assert_eq!(document(&storage).streets.keys().collect::<Vec<_>>(), ["w7~n1~n2"]);
        assert_eq!(CityStore::sample(ports.storage.clone()).open().view(0).edited, 0, "the record matches no place of another city");
        let other = CityStore::for_area(ports.storage, Area::new("Elsewhere", 5.0, 6.0));
        assert!(!other.has_network());
    }

    #[test]
    fn a_street_that_leaves_the_area_is_kept_though_its_far_end_is_no_osm_node() {
        let (ports, _, storage) = test_ports();
        let area = CityStore::for_area(ports.storage, Area::new("Testville", 1.0, 2.0));
        let mut network = tiny_network();
        network.nodes[1].osm_nodes.clear(); // where the road is cut at the area's box
        area.keep_network(&network);
        let street = area.open().view(0).edges[0].uid;
        let mut s = area.open().street_editor(street, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(area.write_street(street, s));
        assert_eq!(document(&storage).streets.keys().collect::<Vec<_>>(), ["w7~n1~x~e"], "the street leaves its one real end eastward");
        assert_eq!(area.open().view(0).edited, 1);
    }

    /// The first street of `store` with `mm` moved from its first piece to its second.
    fn shifted_street(store: &CityStore, mm: i32) -> (u32, Street) {
        let street = store.open().view(0).edges[0].uid;
        let mut s = store.open().street_editor(street, 0).unwrap().snapshot();
        s.segments[0].width_mm -= mm;
        s.segments[1].width_mm += mm;
        (street, s)
    }

    #[test]
    fn a_street_that_took_an_edit_kept_under_another_key_replaces_it_when_written_so_the_older_edit_does_not_come_back() {
        let (ports, _, storage) = test_ports();
        // area A cuts the road at its box; area B holds the whole road
        let a = CityStore::for_area(ports.storage.clone(), Area::new("A", 1.0, 2.0));
        let mut cut = tiny_network();
        cut.nodes[1].osm_nodes.clear();
        a.keep_network(&cut);
        let b = CityStore::for_area(ports.storage, Area::new("B", 1.0, 3.0));
        b.keep_network(&tiny_network());
        let (street, first) = shifted_street(&a, 100);
        assert!(a.write_street(street, first.clone()));
        assert_eq!(b.open().street_editor(street, 0).unwrap().snapshot(), first, "B's street takes A's edit by the ways and nodes they share");
        let (street, second) = shifted_street(&b, 300);
        assert!(b.write_street(street, second.clone()));
        assert_eq!(document(&storage).streets.keys().collect::<Vec<_>>(), ["w7~n1~n2"], "the record it took is replaced, not left beside it");
        assert_eq!(a.open().street_editor(street, 0).unwrap().snapshot(), second, "and A shows the later edit");
    }

    #[test]
    fn a_junction_that_took_an_edit_kept_under_another_key_replaces_it_when_written() {
        let (ports, _, storage) = test_ports();
        let store = CityStore::sample(ports.storage);
        let node = store.open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let mut j = store.open().junction_editor(node, 0).unwrap();
        assert!(j.set_corner(j.current().arms[0].uid, 7_000));
        assert!(store.write_junction(node, j.snapshot()));
        // the same junction kept under a key of an older smallest node: it is found by the nodes it shares
        let mut e = document(&storage);
        let (key, record) = e.junctions.pop_first().unwrap();
        e.junctions.insert("n1".into(), record);
        storage.remember(EDITS_KEY, &e.to_json());
        let mut j = store.open().junction_editor(node, 0).unwrap();
        assert_eq!(j.current().arms[0].corner_mm, 7_000, "taken by overlap");
        assert!(j.set_corner(j.current().arms[0].uid, 6_000));
        assert!(store.write_junction(node, j.snapshot()));
        assert_eq!(document(&storage).junctions.keys().collect::<Vec<_>>(), [&key]);
    }

    #[test]
    fn a_street_whose_key_another_street_shares_is_not_written_and_the_write_says_so() {
        let (ports, _, storage) = test_ports();
        let area = CityStore::for_area(ports.storage, Area::new("Testville", 1.0, 2.0));
        let mut network = tiny_network();
        // a second road of the same way between the same two nodes, along the same line: nothing tells them apart
        let mut twin = network.roads[0].clone();
        twin.id = 2;
        network.roads.push(twin);
        area.keep_network(&network);
        let street = area.open().view(0).edges[0].uid;
        let mut s = area.open().street_editor(street, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        assert!(!area.write_street(street, s), "an edit that could not come back is refused");
        assert_eq!(storage.recall(EDITS_KEY), None, "nothing is written");
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
