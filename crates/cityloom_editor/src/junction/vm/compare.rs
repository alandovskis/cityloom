//! The junction half of the comparison page: two designs of one junction (the baseline or a scenario each), opened
//! read-only, their views for the drawings, and what differs between them as `Said` data.
//!
//! The view-model never writes: it reads the designs as the city keeps them and rewrites the page's address (the
//! picks), nothing else. It takes ports already scoped to the project (`project::context::peek`).

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::junction::model::State;
use crate::junction::model::compare::{arms, checks, control, others, signal};
use crate::junction::model::read_model::JView;
use crate::shared::compare::{CompareHost, CompareState, DiffGroup, Pick, Summary, initial_picks, with_picks};
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;
use crate::shared::said::{Arg, Said, say_now};
use crate::shared::units::Units;

/// One side's design: the junction as kept, and what the drawing and the summary read of it.
struct Side {
    state: State,
    view: Rc<JView>,
}

pub struct JunctionCompareVm {
    ports: Ports,
    i18n: Rc<I18n>,
    binding: CityBinding,
    node: u32,
    state: CompareState,
    sides: RefCell<(Side, Side)>,
    /// Changes whenever a side is opened again.
    version: ArcRwSignal<u32>,
    units: ArcRwSignal<Units>,
}

impl JunctionCompareVm {
    /// The comparison of the junction `binding` names, on the designs the page's address picks (`a=`, `b=`).
    /// None where the baseline cannot be opened (not a junction, or not in the city, or not drawable).
    pub fn new(ports: Ports, i18n: Rc<I18n>, binding: CityBinding) -> Option<Rc<Self>> {
        let Place::Junction(node) = binding.place else { return None };
        let items = binding.scenarios();
        let ids: Vec<u32> = items.iter().map(|(id, _)| *id).collect();
        let (a, b, missing) = initial_picks(&ports.page.path(), &ids);
        let state = CompareState::default();
        state.list(items);
        let open = |id: Option<u32>| Self::open_side(&binding, id);
        // A listed design that will not open is the baseline.
        let (a, side_a) = open(a).map(|s| (a, s)).or_else(|| open(None).map(|s| (None, s)))?;
        let (b, side_b) = open(b).map(|s| (b, s)).or_else(|| open(None).map(|s| (None, s)))?;
        state.set(Pick::A, a);
        state.set(Pick::B, b);
        if missing {
            state.refuse(Said::new("scenario-missing"));
        }
        Some(Rc::new(JunctionCompareVm {
            ports,
            i18n,
            binding,
            node,
            state,
            sides: RefCell::new((side_a, side_b)),
            version: ArcRwSignal::new(0),
            units: ArcRwSignal::new(Units::default()),
        }))
    }

    fn open_side(binding: &CityBinding, id: Option<u32>) -> Option<Side> {
        let junction = binding.junction_editor(id)?;
        Some(Side { state: junction.current().clone(), view: Rc::new(junction.view()) })
    }

    /// The two designs' views, for the drawings (watched: they change when a side is chosen again).
    pub fn views(&self) -> (Rc<JView>, Rc<JView>) {
        self.version.track();
        let sides = self.sides.borrow();
        (sides.0.view.clone(), sides.1.view.clone())
    }

    /// The units lengths are shown in (watched).
    pub fn units(&self) -> Units {
        self.units.get()
    }

    pub fn set_units(&self, units: Units) {
        self.units.set(units);
    }

    fn pick_names(&self) -> (String, String) {
        let baseline = self.i18n.tr_now("cmp-baseline", &Args::new());
        (self.state.name_of(self.state.pick_now(Pick::A), &baseline), self.state.name_of(self.state.pick_now(Pick::B), &baseline))
    }

    fn summary_now(&self) -> Summary {
        let sides = self.sides.borrow();
        let (a, b) = (&sides.0, &sides.1);
        let arm_lines = arms(&a.view, &b.view).iter().flat_map(|c| c.lines()).collect();
        let (passes, fails) = checks(&a.view, &b.view);
        let check = |key: &'static str, label: Said| Said::new(key).with("check", Arg::Said(Box::new(label)));
        let mut check_lines: Vec<Said> = passes.into_iter().map(|l| check("cmp-only-passes", l)).collect();
        check_lines.extend(fails.into_iter().map(|l| check("cmp-only-fails", l)));
        Summary {
            groups: vec![
                DiffGroup { heading: "cmp-h-control", lines: control(&a.view, &b.view) },
                DiffGroup { heading: "cmp-h-arms", lines: arm_lines },
                DiffGroup { heading: "cmp-h-signal", lines: signal(&a.view, &b.view) },
                DiffGroup { heading: "cmp-h-checks", lines: check_lines },
            ],
            also: others(&a.view, &b.view, &a.state, &b.state).into_iter().map(|o| Said::new(o.key())).collect(),
        }
    }
}

impl CompareHost for JunctionCompareVm {
    fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    fn compare_state(&self) -> &CompareState {
        &self.state
    }

    fn choose(&self, side: Pick, id: Option<u32>) {
        self.state.list(self.binding.scenarios());
        // A design deleted meanwhile opens nothing: the side shows the baseline, and it is said.
        let scenario = id.and_then(|id| Self::open_side(&self.binding, Some(id)).map(|opened| (id, opened)));
        let missing = id.is_some() && scenario.is_none();
        let (id, opened) = match scenario {
            Some((id, opened)) => (Some(id), Some(opened)),
            None => (None, Self::open_side(&self.binding, None)),
        };
        self.state.set(side, id);
        if let Some(opened) = opened {
            let mut sides = self.sides.borrow_mut();
            match side {
                Pick::A => sides.0 = opened,
                Pick::B => sides.1 = opened,
            }
        }
        if missing {
            self.state.refuse(Said::new("scenario-missing"));
        }
        self.version.update(|v| *v += 1);
        let (a, b) = (self.state.pick_now(Pick::A), self.state.pick_now(Pick::B));
        self.ports.page.replace_path(&with_picks(&self.ports.page.path(), a, b));
        let (name_a, name_b) = self.pick_names();
        self.ports.announcer.say(&say_now(&self.i18n, self.units.get_untracked(), &self.summary_now().headline(&name_a, &name_b)));
    }

    fn summary(&self) -> Summary {
        self.version.track();
        self.summary_now()
    }

    fn back_href(&self) -> String {
        crate::shared::project_id::link_for(&format!("intersection.html?junction={}", self.node), &*self.ports.storage)
    }

    fn back_key(&self) -> &'static str {
        "cmp-back-junction"
    }

    fn place_name(&self) -> Said {
        self.sides.borrow().0.view.name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::scenario::Action;
    use crate::city::store::CityStore;
    use crate::junction::model::{PRIORITY, SIGNAL};
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{FakePage, MemoryStorage, Page, RecordingAnnouncer, Storage, test_ports_with_time};
    use crate::shared::scoped::ScopedStorage;

    struct Rig {
        vm: Option<Rc<JunctionCompareVm>>,
        binding: CityBinding,
        page: Rc<FakePage>,
        said: Rc<RecordingAnnouncer>,
        storage: Rc<MemoryStorage>,
        ports: Ports,
    }

    impl Rig {
        fn vm(&self) -> &Rc<JunctionCompareVm> {
            self.vm.as_ref().unwrap()
        }
    }

    /// A control other than the one the junction has.
    fn other_control(view: &JView) -> usize {
        if view.control_index == PRIORITY { SIGNAL } else { PRIORITY }
    }

    /// A city with one junction and `scenarios` scenarios, each changing the control; the page at
    /// `/compare.html?junction=N` plus `query`.
    fn rig(scenarios: usize, query: &str) -> Rig {
        let (mut ports, said, storage, _) = test_ports_with_time();
        let page = Rc::new(FakePage::default());
        ports.page = page.clone();
        let own: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), "p1"));
        let store = CityStore::sample(own.clone());
        let node = store.open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let binding = CityBinding { store, place: Place::Junction(node) };
        for i in 0..scenarios {
            let id = binding.act(Action::Create(format!("Scenario {i}"))).unwrap().unwrap();
            let mut j = binding.junction_editor(Some(id)).unwrap();
            let to = other_control(&j.view());
            assert!(j.set_control(to));
            binding.keep_junction_in(j.snapshot(), Some(id)).unwrap();
        }
        *page.path.borrow_mut() = format!("/compare.html?junction={node}{query}");
        ports.storage = own.clone();
        let binding = CityBinding { store: CityStore::sample(own), place: Place::Junction(node) };
        let vm = JunctionCompareVm::new(ports.clone(), crate::i18n_for(Locale::En), binding.clone());
        Rig { vm, binding, page, said, storage, ports }
    }

    fn stored(storage: &MemoryStorage) -> Vec<(String, Option<String>)> {
        let mut keys = storage.keys();
        keys.sort();
        keys.into_iter().map(|k| (k.clone(), storage.recall(&k))).collect()
    }

    fn group<'a>(summary: &'a Summary, heading: &str) -> &'a DiffGroup {
        summary.groups.iter().find(|g| g.heading == heading).unwrap()
    }

    #[test]
    fn it_opens_on_the_baseline_and_the_first_scenario() {
        let r = rig(2, "");
        let first = r.binding.scenarios()[0].0;
        let state = r.vm().compare_state();
        assert_eq!((state.pick_now(Pick::A), state.pick_now(Pick::B)), (None, Some(first)));
        assert!(state.refusal().is_none());
        let (a, b) = r.vm().views();
        assert_ne!(a.control_index, b.control_index);
    }

    #[test]
    fn a_changed_control_is_a_line_of_the_control_group() {
        let r = rig(1, "");
        let summary = r.vm().summary();
        let control = group(&summary, "cmp-h-control");
        assert!(control.lines.iter().any(|l| l.key == "cmp-jn-control"), "{summary:?}");
        assert!(!summary.is_same());
        let headings: Vec<_> = summary.groups.iter().map(|g| g.heading).collect();
        assert_eq!(headings, ["cmp-h-control", "cmp-h-arms", "cmp-h-signal", "cmp-h-checks"]);
    }

    #[test]
    fn a_place_with_no_scenarios_compares_the_baseline_with_itself() {
        let r = rig(0, "");
        let state = r.vm().compare_state();
        assert_eq!((state.pick_now(Pick::A), state.pick_now(Pick::B)), (None, None));
        assert!(r.vm().summary().is_same());
    }

    #[test]
    fn the_same_design_twice_is_the_same() {
        let r = rig(2, "");
        let id = r.binding.scenarios()[1].0;
        r.vm().choose(Pick::A, Some(id));
        r.vm().choose(Pick::B, Some(id));
        let summary = r.vm().summary();
        assert!(summary.is_same(), "{summary:?}");
        assert!(summary.also.is_empty());
    }

    #[test]
    fn a_deleted_scenario_in_the_address_falls_back_and_is_said() {
        let r = rig(1, "&b=99");
        assert_eq!(r.vm().compare_state().pick_now(Pick::B), None);
        assert_eq!(r.vm().compare_state().refusal(), Some(Said::new("scenario-missing")));
    }

    #[test]
    fn choosing_a_deleted_scenario_falls_back_to_the_baseline_and_says_so() {
        let r = rig(1, "");
        r.vm().choose(Pick::B, Some(99));
        assert_eq!(r.vm().compare_state().pick_now(Pick::B), None);
        assert_eq!(r.vm().compare_state().refusal(), Some(Said::new("scenario-missing")));
        r.vm().choose(Pick::B, None);
        assert!(r.vm().compare_state().refusal().is_none());
    }

    #[test]
    fn choosing_rewrites_the_address_and_says_the_headline() {
        let r = rig(2, "");
        let second = r.binding.scenarios()[1].0;
        r.vm().choose(Pick::B, Some(second));
        let path = r.page.path();
        assert!(path.ends_with(&format!("a=base&b={second}")), "{path}");
        let said = r.said.take();
        let summary = r.vm().summary();
        let headline = summary.headline("Baseline", "Scenario 1");
        assert_eq!(headline.key, "cmp-differs");
        assert_eq!(said, [say_now(&r.vm().i18n(), Units::default(), &headline)]);
        assert!(said[0].starts_with("Scenario 1 differs from Baseline in "), "{said:?}");
    }

    #[test]
    fn opening_and_choosing_write_nothing() {
        let r = rig(2, "");
        let before = stored(&r.storage);
        let again = JunctionCompareVm::new(r.ports.clone(), crate::i18n_for(Locale::En), r.binding.clone()).unwrap();
        let ids: Vec<u32> = r.binding.scenarios().iter().map(|(id, _)| *id).collect();
        again.choose(Pick::A, Some(ids[1]));
        again.choose(Pick::B, None);
        assert_eq!(stored(&r.storage), before);
    }

    #[test]
    fn the_back_link_names_the_junction_and_keeps_view() {
        let r = rig(1, "&view=1");
        let vm = r.vm();
        let Place::Junction(node) = r.binding.place else { unreachable!() };
        assert!(vm.back_href().contains(&format!("intersection.html?junction={node}")), "{}", vm.back_href());
        assert_eq!(vm.back_key(), "cmp-back-junction");
    }

    #[test]
    fn a_street_is_not_a_junction() {
        let (ports, ..) = test_ports_with_time();
        let store = CityStore::sample(ports.storage.clone());
        let edge = store.open().view(0).edges[0].uid;
        let binding = CityBinding { store, place: Place::Street(edge) };
        assert!(JunctionCompareVm::new(ports, crate::i18n_for(Locale::En), binding).is_none());
    }

    #[test]
    fn the_baseline_is_named_in_the_language_of_the_page() {
        let r = rig(0, "");
        r.vm().i18n().set(Locale::FrCa);
        r.vm().choose(Pick::B, None);
        assert_eq!(r.said.take(), ["Référence et Référence sont identiques."]);
    }
}
