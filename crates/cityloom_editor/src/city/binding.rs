//! A place in the city that a page edits, and how what it makes is written back.

use crate::city::store::CityStore;
use crate::junction::model::State;
use crate::street::model::Street;
use crate::sync::model::{Id, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Junction(u32),
    Street(u32),
}

/// The page edits a junction or a street of the city kept in `store`.
#[derive(Clone)]
pub struct CityBinding {
    pub store: CityStore,
    pub place: Place,
}

impl CityBinding {
    /// Writes a junction as it now is back to the city. Says whether it was kept.
    pub fn keep_junction(&self, state: State) -> bool {
        let Place::Junction(node) = self.place else { return false };
        self.store.write_junction(node, state)
    }

    /// Writes a street as it now is back to the city. Says whether it was kept.
    pub fn keep_street(&self, street: Street) -> bool {
        let Place::Street(edge) = self.place else { return false };
        self.store.write_street(edge, street)
    }

    /// The sync's name for this page's place: its kind and key. None where the place has no key of its own.
    pub fn record(&self) -> Option<Id> {
        let city = self.store.open();
        match self.place {
            Place::Street(edge) => city.street_key(edge).map(|key| Id { kind: Kind::Street, key }),
            Place::Junction(node) => city.junction_key(node).map(|key| Id { kind: Kind::Junction, key }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ports::test_ports;

    fn binding(place: impl Fn(&crate::city::model::CityView) -> Place) -> CityBinding {
        let (ports, ..) = test_ports();
        let store = CityStore::sample(ports.storage);
        let place = place(&store.open().view(0));
        CityBinding { store, place }
    }

    #[test]
    fn a_binding_names_its_place_by_the_key_the_sync_carries() {
        let b = binding(|v| Place::Street(v.edges[0].uid));
        let Place::Street(edge) = b.place else { unreachable!() };
        assert_eq!(b.record(), Some(Id { kind: Kind::Street, key: b.store.open().street_record(edge).unwrap().0 }));
        let b = binding(|v| Place::Junction(v.nodes.iter().find(|n| n.junction).unwrap().uid));
        assert!(matches!(b.record(), Some(Id { kind: Kind::Junction, .. })));
        let (ports, ..) = test_ports();
        let keyless = CityStore::sample_without_osm_ids(ports.storage);
        let place = Place::Street(keyless.open().view(0).edges[0].uid);
        assert_eq!(CityBinding { store: keyless, place }.record(), None, "a place with no key of its own is not synced");
    }

    #[test]
    fn a_street_is_written_back_and_found_by_the_next_page() {
        let b = binding(|v| Place::Street(v.edges[0].uid));
        let Place::Street(edge) = b.place else { unreachable!() };
        let mut e = b.store.open().street_editor(edge, 0).unwrap();
        let u = e.view().segments[0].uid;
        e.nudge_width(u, -100);
        assert!(b.keep_street(e.snapshot()));
        assert_eq!(b.store.open().view(0).edited, 1);
        assert!(!b.keep_junction(crate::junction::model::Junction::new(0).snapshot()), "not a junction");
    }

    #[test]
    fn a_junction_is_written_back_and_a_street_is_not_taken_for_one() {
        let b = binding(|v| Place::Junction(v.nodes.iter().find(|n| n.junction).unwrap().uid));
        let Place::Junction(node) = b.place else { unreachable!() };
        let mut j = b.store.open().junction_editor(node, 0).unwrap();
        let uid = j.current().arms[0].uid;
        assert!(j.set_corner(uid, 7_000));
        assert!(b.keep_junction(j.snapshot()));
        assert_eq!(b.store.open().view(0).edited, 1);
        let mut e = crate::street::model::Editor::new(0);
        let u = e.view().segments[0].uid;
        e.nudge_width(u, 100);
        assert!(!b.keep_street(e.snapshot()));
    }

    #[test]
    fn a_junction_with_a_street_that_has_no_key_is_not_kept() {
        let (ports, ..) = test_ports();
        let store = CityStore::sample_without_osm_ids(ports.storage);
        let node = store.open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let mut j = store.open().junction_editor(node, 0).unwrap();
        let uid = j.current().arms[0].uid;
        assert!(j.set_corner(uid, 7_000));
        let b = CityBinding { store, place: Place::Junction(node) };
        assert!(!b.keep_junction(j.snapshot()));
    }
}
