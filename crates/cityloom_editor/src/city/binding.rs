//! A place in the city that a page edits, and how what it makes is written back.

use crate::city::scenario::Action;
use crate::city::store::{CityStore, ScenarioFailure};
use crate::junction::model::State;
use crate::shared::project_id::wire_key;
use crate::shared::said::Said;
use crate::shared::scenarios::ScenarioHost;
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
    /// `keep_street`). Why it was not kept: `Refused(Missing)` where the scenario is gone (deleted elsewhere), which is
    /// no fault of the storage; `NotKept` otherwise.
    pub fn keep_street_in(&self, street: Street, scenario: Option<u32>) -> Result<(), ScenarioFailure> {
        let Place::Street(edge) = self.place else { return Err(ScenarioFailure::NotKept) };
        match scenario {
            None => self.store.write_street(edge, street).then_some(()).ok_or(ScenarioFailure::NotKept),
            Some(id) => self.store.write_street_scenario(edge, id, street),
        }
    }

    /// The same for a junction.
    pub fn keep_junction_in(&self, state: State, scenario: Option<u32>) -> Result<(), ScenarioFailure> {
        let Place::Junction(node) = self.place else { return Err(ScenarioFailure::NotKept) };
        match scenario {
            None => self.store.write_junction(node, state).then_some(()).ok_or(ScenarioFailure::NotKept),
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

/// A page that edits a place's scenarios (the street's and the junction's view-models): what the scenario actions
/// (`scenario_actions`) need of it.
pub trait ScenarioPage: ScenarioHost {
    /// The place in the city the page edits; none for a sandbox, which keeps no scenarios.
    fn binding(&self) -> Option<&CityBinding>;
    /// Writes what waits to be kept, now (to the design shown). Where the scenario shown has been deleted elsewhere,
    /// that write finds it gone and the page then shows the baseline and says why (`gone`): what is shown may change.
    fn flush(&self);
    /// Shows design `id` (None: the baseline) as the city keeps it, starting a history of its own: what could be
    /// undone in the design shown before is not carried over. Whether it could be shown.
    fn show_design(&self, id: Option<u32>) -> bool;
    /// Tells the person (the live region).
    fn tell(&self, said: &Said);
}

/// The scenario actions of a page, whichever kind of place it edits. Each keeps what the page shows before it reads
/// it or acts (`kept_first`), so no edit is lost, and tells the person what it did or why it did nothing.
pub mod scenario_actions {
    use super::{Action, ScenarioFailure, ScenarioPage};
    use crate::city::scenario::{News, Refusal};
    use crate::shared::said::Said;

    /// The scenario shown is gone (deleted elsewhere, found by a write or a sync): the baseline is shown, with a
    /// history of its own, and the person is told, `edit_lost` where a change made to it could not be kept.
    pub fn gone(page: &impl ScenarioPage, edit_lost: bool) {
        refresh(page);
        page.show_design(None);
        page.scenario_state().show(None);
        page.tell(&News::Gone { edit_lost }.said());
    }

    /// What a write of the design shown came to, for the page's keeper: false only where the storage failed (which
    /// the keeper says once). A scenario gone since it was shown is no storage failure: the page shows the baseline and
    /// says why (`gone`), and the keeper's one warning is kept for a real failure. `changed`: the design written
    /// differs from the one last opened or kept, so a change to it is lost (a write also follows a mere choice of a
    /// piece, which loses nothing).
    pub fn kept(page: &impl ScenarioPage, outcome: Result<(), ScenarioFailure>, changed: bool) -> bool {
        match outcome {
            Ok(()) => true,
            Err(ScenarioFailure::NotKept) => false,
            Err(ScenarioFailure::Refused(_)) => {
                gone(page, changed);
                true
            }
        }
    }

    /// Lists the place's scenarios as kept now.
    pub fn refresh(page: &impl ScenarioPage) {
        if let Some(binding) = page.binding() {
            page.scenario_state().list(binding.scenarios());
        }
    }

    /// Shows scenario `id`, or the baseline.
    /// Writes what waits to be kept before an action reads what is shown. False where the action is to stop: the page
    /// edits no place, or the write found the scenario shown gone, so the page already shows the baseline and has
    /// said why (anything said after it would only replace that in the live region).
    fn kept_first(page: &impl ScenarioPage) -> bool {
        if page.binding().is_none() {
            return false;
        }
        let shown = page.scenario_state().active_now();
        page.flush();
        !(shown.is_some() && page.scenario_state().active_now().is_none())
    }

    pub fn switch(page: &impl ScenarioPage, id: Option<u32>) -> bool {
        if !kept_first(page) {
            return false;
        }
        let state = page.scenario_state();
        if !page.show_design(id) {
            // gone since it was listed (deleted in another tab, say)
            refresh(page);
            let why = Refusal::Missing.said("");
            state.refuse(why.clone());
            page.tell(&why);
            return false;
        }
        state.show(id);
        page.tell(&News::Showing(id.and_then(|id| state.name_now(id))).said());
        true
    }

    /// Does `action` on the place, `name` the name it was given (as typed): what it made, or None where it was
    /// refused or not kept (and the person told why). What was waiting is kept first (`kept_first`).
    fn act(page: &impl ScenarioPage, action: Action, name: &str) -> Option<Option<u32>> {
        let binding = page.binding()?;
        let state = page.scenario_state();
        match binding.act(action) {
            Ok(made) => {
                refresh(page);
                Some(made)
            }
            Err(ScenarioFailure::Refused(why)) => {
                let why = why.said(name);
                state.refuse(why.clone());
                page.tell(&why);
                None
            }
            Err(ScenarioFailure::NotKept) => {
                state.settle();
                page.tell(&Said::new("not-kept"));
                None
            }
        }
    }

    /// Shows scenario `made`, just made, and says `news` of it.
    fn show_made(page: &impl ScenarioPage, made: u32, news: impl FnOnce(String) -> News) -> bool {
        if !page.show_design(Some(made)) {
            return false;
        }
        let state = page.scenario_state();
        state.show(Some(made));
        page.tell(&news(state.name_now(made).unwrap_or_default()).said());
        true
    }

    pub fn create(page: &impl ScenarioPage, name: &str) -> bool {
        if !kept_first(page) {
            return false;
        }
        let Some(Some(made)) = act(page, Action::Create(name.to_string()), name) else { return false };
        show_made(page, made, News::Created)
    }

    /// Duplicates the scenario shown; nothing on the baseline (a new scenario is made from it).
    pub fn duplicate(page: &impl ScenarioPage, name: &str) -> bool {
        if !kept_first(page) {
            return false;
        }
        let state = page.scenario_state();
        let Some(from) = state.active_now() else { return false };
        let from_name = state.name_now(from).unwrap_or_default();
        let Some(Some(made)) = act(page, Action::Duplicate(from, name.to_string()), name) else { return false };
        show_made(page, made, |name| News::Duplicated { from: from_name, name })
    }

    /// Renames the scenario shown; the baseline has no name of its own.
    pub fn rename(page: &impl ScenarioPage, name: &str) -> bool {
        if !kept_first(page) {
            return false;
        }
        let state = page.scenario_state();
        let Some(id) = state.active_now() else { return false };
        let from = state.name_now(id).unwrap_or_default();
        if act(page, Action::Rename(id, name.to_string()), name).is_none() {
            return false;
        }
        state.settle();
        page.tell(&News::Renamed { from, name: state.name_now(id).unwrap_or_default() }.said());
        true
    }

    /// Deletes the scenario shown, the second time it is asked with nothing done in between, and shows the
    /// baseline; the baseline cannot be deleted.
    pub fn delete(page: &impl ScenarioPage) -> bool {
        if !kept_first(page) {
            return false;
        }
        let state = page.scenario_state();
        let Some(id) = state.active_now() else { return false };
        let name = state.name_now(id).unwrap_or_default();
        if !state.asking_delete_now() {
            state.ask_delete();
            page.tell(&News::AskDelete(name).said());
            return false;
        }
        if act(page, Action::Delete(id), "").is_none() {
            return false;
        }
        page.show_design(None);
        state.show(None);
        page.tell(&News::Deleted(name).said());
        true
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
                assert_eq!(b.keep_street_in(e.snapshot(), Some(a)), Ok(()));
                assert_eq!(b.street_editor(Some(a)).unwrap().snapshot(), e.snapshot());
                assert_ne!(b.street_editor(None).unwrap().snapshot(), e.snapshot(), "the baseline is as it was");
                assert!(b.junction_editor(None).is_none() && b.keep_junction_in(crate::junction::model::Junction::new(0).snapshot(), Some(a)).is_err());
            } else {
                let mut j = b.junction_editor(Some(a)).unwrap();
                assert!(j.set_corner(j.current().arms[0].uid, 7_000));
                assert_eq!(b.keep_junction_in(j.snapshot(), Some(a)), Ok(()));
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
