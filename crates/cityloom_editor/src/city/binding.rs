//! A place in the city that a page edits, and how what it makes is written back.

use crate::city::scenario::Action;
use crate::city::store::{CityStore, ScenarioFailure};
use crate::junction::model::State;
use crate::shared::project_id::wire_key;
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

    /// The street editor on this page's street with scenario `scenario`'s design (`None`: the baseline), as the city
    /// keeps it now. None where this page's place is no street or has no such scenario.
    pub fn street_editor(&self, scenario: Option<u32>) -> Option<crate::street::model::Editor> {
        let Place::Street(edge) = self.place else { return None };
        self.store.open().street_editor_in(edge, self.store.region(), scenario)
    }

    /// The same for the junction.
    pub fn junction_editor(&self, scenario: Option<u32>) -> Option<crate::junction::model::Junction> {
        let Place::Junction(node) = self.place else { return None };
        self.store.open().junction_editor_in(node, self.store.region(), scenario)
    }

    /// Writes a street as it now is back to the city, as scenario `scenario`'s design (`None`: the baseline, as
    /// `keep_street`). Says whether it was kept.
    pub fn keep_street_in(&self, street: Street, scenario: Option<u32>) -> bool {
        let Place::Street(edge) = self.place else { return false };
        match scenario {
            None => self.store.write_street(edge, street),
            Some(id) => self.store.write_street_scenario(edge, id, street),
        }
    }

    /// The same for a junction.
    pub fn keep_junction_in(&self, state: State, scenario: Option<u32>) -> bool {
        let Place::Junction(node) = self.place else { return false };
        match scenario {
            None => self.store.write_junction(node, state),
            Some(id) => self.store.write_junction_scenario(node, id, state),
        }
    }

    /// The scenarios of this page's place, by id and name, in the order they were made.
    pub fn scenarios(&self) -> Vec<(u32, String)> {
        let city = self.store.open();
        match self.place {
            Place::Street(edge) => city.street_scenarios(edge).map(|s| s.iter().map(|s| (s.id, s.name.clone())).collect()),
            Place::Junction(node) => city.junction_scenarios(node).map(|s| s.iter().map(|s| (s.id, s.name.clone())).collect()),
        }
        .unwrap_or_default()
    }

    /// Does a scenario action on this page's place and keeps it. The id of the scenario it made, if it made one.
    pub fn act(&self, action: Action) -> Result<Option<u32>, ScenarioFailure> {
        match self.place {
            Place::Street(edge) => self.store.change_street_scenarios(edge, |s, baseline| s.act(action, baseline)),
            Place::Junction(node) => self.store.change_junction_scenarios(node, |s, baseline| s.act(action, baseline)),
        }
    }

    /// The sync's name for this page's place: its kind and wire key (the project's prefix added, as `mark` adds it).
    /// None where the place has no key of its own.
    pub fn record(&self) -> Option<Id> {
        let city = self.store.open();
        let (kind, key) = match self.place {
            Place::Street(edge) => (Kind::Street, city.street_key(edge)?),
            Place::Junction(node) => (Kind::Junction, city.junction_key(node)?),
        };
        let key = self.store.project().map_or_else(|| key.clone(), |project| wire_key(&project, &key));
        Some(Id { kind, key })
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
    fn in_another_project_than_the_first_a_binding_names_its_place_by_its_wire_key() {
        let (ports, _, root) = test_ports();
        let storage: std::rc::Rc<dyn crate::shared::ports::Storage> = std::rc::Rc::new(crate::shared::scoped::ScopedStorage::new(root.clone(), "p1"));
        let store = CityStore::sample(storage);
        let edge = store.open().view(0).edges[0].uid;
        let bare = store.open().street_record(edge).unwrap().0;
        let b = CityBinding { store, place: Place::Street(edge) };
        assert_eq!(b.record(), Some(Id { kind: Kind::Street, key: format!("p1:{bare}") }));
        let first = CityBinding { store: CityStore::sample(crate::shared::scoped::in_project(ports, "first").storage), place: Place::Street(edge) };
        assert_eq!(first.record(), Some(Id { kind: Kind::Street, key: bare }), "the first project's keys are bare");
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
    fn a_scenario_is_made_edited_and_listed_through_the_binding_and_the_baseline_stays_as_it_was() {
        for street in [true, false] {
            let b = if street {
                binding(|v| Place::Street(v.edges[0].uid))
            } else {
                binding(|v| Place::Junction(v.nodes.iter().find(|n| n.junction).unwrap().uid))
            };
            assert_eq!(b.scenarios(), Vec::<(u32, String)>::new());
            let a = b.act(Action::Create("Wider bike lane".into())).unwrap().unwrap();
            let p = b.act(Action::Create("Parking kept".into())).unwrap().unwrap();
            assert_eq!(b.scenarios(), [(a, "Wider bike lane".to_string()), (p, "Parking kept".to_string())]);
            assert_eq!(b.act(Action::Create("parking KEPT".into())), Err(ScenarioFailure::Refused(crate::city::scenario::Refusal::NameTaken)));
            if street {
                let mut e = b.street_editor(Some(a)).unwrap();
                let u = e.view().segments[0].uid;
                e.nudge_width(u, -100);
                assert!(b.keep_street_in(e.snapshot(), Some(a)));
                assert_eq!(b.street_editor(Some(a)).unwrap().snapshot(), e.snapshot());
                assert_ne!(b.street_editor(None).unwrap().snapshot(), e.snapshot(), "the baseline is as it was");
                assert!(b.junction_editor(None).is_none() && !b.keep_junction_in(crate::junction::model::Junction::new(0).snapshot(), Some(a)));
            } else {
                let mut j = b.junction_editor(Some(a)).unwrap();
                assert!(j.set_corner(j.current().arms[0].uid, 7_000));
                assert!(b.keep_junction_in(j.snapshot(), Some(a)));
                assert!(b.junction_editor(Some(a)).unwrap().drawn_as(&j));
                assert!(!b.junction_editor(None).unwrap().drawn_as(&j), "the baseline is as it was");
                assert!(b.street_editor(None).is_none());
            }
            assert_eq!(b.store.open().view(0).edited, 0, "the map reads the baselines");
        }
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
