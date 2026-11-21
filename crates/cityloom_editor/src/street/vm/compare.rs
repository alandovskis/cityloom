//! The street half of the comparison page: two designs of one street (the baseline or a scenario each), opened
//! read-only, their views for the drawings, and what differs between them as `Said` data.
//!
//! The view-model never writes: it reads the designs as the city keeps them and rewrites the page's address (the
//! picks), nothing else. It takes ports already scoped to the project (`project::context::peek`).

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::restriction::model::{Finding, Restriction};
use crate::shared::compare::{CompareHost, CompareState, DiffGroup, Pick, Summary, initial_picks, with_picks};
use crate::shared::i18n::{Args, I18n};
use crate::shared::ports::Ports;
use crate::shared::said::{Arg, Said, say_now};
use crate::shared::units::Units;
use crate::street::model::compare::{lanes, others, pieces};
use crate::street::model::{Street, StreetExt, View};
use crate::street::vm::impact::{Cause, Input, diff};

/// One side's design: the street as kept, and what the drawings and the summary read of it.
struct Side {
    street: Street,
    view: Rc<View>,
    findings: Vec<Finding>,
}

pub struct StreetCompareVm {
    ports: Ports,
    i18n: Rc<I18n>,
    binding: CityBinding,
    edge: u32,
    /// The areas the street lies in, read once when the page opens.
    areas: Vec<Restriction>,
    state: CompareState,
    sides: RefCell<(Side, Side)>,
    /// Changes whenever a side is opened again.
    version: ArcRwSignal<u32>,
    units: ArcRwSignal<Units>,
}

impl StreetCompareVm {
    /// The comparison of the street `binding` names, on the designs the page's address picks (`a=`, `b=`).
    /// None where the baseline cannot be opened (not a street, or not in the city).
    pub fn new(ports: Ports, i18n: Rc<I18n>, binding: CityBinding) -> Option<Rc<Self>> {
        let Place::Street(edge) = binding.place else { return None };
        let areas = super::areas_holding(&ports, Some(&binding));
        let items = binding.scenarios();
        let ids: Vec<u32> = items.iter().map(|(id, _)| *id).collect();
        let (a, b, missing) = initial_picks(&ports.page.path(), &ids);
        let state = CompareState::default();
        state.list(items);
        let open = |id: Option<u32>| Self::open_side(&binding, &areas, id);
        // A listed design that will not open is the baseline.
        let (a, side_a) = open(a).map(|s| (a, s)).or_else(|| open(None).map(|s| (None, s)))?;
        let (b, side_b) = open(b).map(|s| (b, s)).or_else(|| open(None).map(|s| (None, s)))?;
        state.set(Pick::A, a);
        state.set(Pick::B, b);
        if missing {
            state.refuse(Said::new("scenario-missing"));
        }
        Some(Rc::new(StreetCompareVm {
            ports,
            i18n,
            binding,
            edge,
            areas,
            state,
            sides: RefCell::new((side_a, side_b)),
            version: ArcRwSignal::new(0),
            units: ArcRwSignal::new(Units::default()),
        }))
    }

    fn open_side(binding: &CityBinding, areas: &[Restriction], id: Option<u32>) -> Option<Side> {
        let editor = binding.street_editor(id)?;
        let street = editor.snapshot();
        let findings = areas.iter().map(|r| Finding::of(r, &street)).collect();
        Some(Side { street, view: Rc::new(editor.view()), findings })
    }

    /// The two designs' views, for the drawings (watched: they change when a side is chosen again).
    pub fn views(&self) -> (Rc<View>, Rc<View>) {
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

    fn baseline_name(&self) -> String {
        self.i18n.tr_now("cmp-baseline", &Args::new())
    }

    fn pick_names(&self) -> (String, String) {
        let baseline = self.baseline_name();
        (self.state.name_of(self.state.pick_now(Pick::A), &baseline), self.state.name_of(self.state.pick_now(Pick::B), &baseline))
    }

    fn summary_now(&self) -> Summary {
        let sides = self.sides.borrow();
        let (a, b) = (&sides.0, &sides.1);
        let impact = diff(Cause::Edit, &Input::of(&a.view, &a.findings), &Input::of(&b.view, &b.findings));
        let mut space: Vec<Said> = impact.uses.iter().map(|u| u.said()).collect();
        space.extend(impact.class.clone());
        let lane_lines = lanes(&a.street, &b.street).iter().map(|l| l.said()).collect();
        let (stretches, changes) = pieces(&a.street, &b.street);
        let mut piece_lines: Vec<Said> = stretches.then(|| Said::new("cmp-stretches-differ")).into_iter().collect();
        piece_lines.extend(changes.iter().map(|c| c.said()));
        let check = |key: &'static str, label: &Said| Said::new(key).with("check", Arg::Said(Box::new(label.clone())));
        let mut checks: Vec<Said> = impact.now_passing.iter().map(|l| check("cmp-only-passes", l)).collect();
        checks.extend(impact.now_failing.iter().map(|l| check("cmp-only-fails", l)));
        Summary {
            groups: vec![
                DiffGroup { heading: "cmp-h-space", lines: space },
                DiffGroup { heading: "cmp-h-lanes", lines: lane_lines },
                DiffGroup { heading: "cmp-h-pieces", lines: piece_lines },
                DiffGroup { heading: "cmp-h-checks", lines: checks },
            ],
            also: others(&a.street, &b.street).into_iter().map(|o| Said::new(o.key())).collect(),
        }
    }
}

impl CompareHost for StreetCompareVm {
    fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    fn compare_state(&self) -> &CompareState {
        &self.state
    }

    fn choose(&self, side: Pick, id: Option<u32>) {
        self.state.list(self.binding.scenarios());
        // A design deleted meanwhile opens nothing: the side shows the baseline, and it is said.
        let scenario = id.and_then(|id| Self::open_side(&self.binding, &self.areas, Some(id)).map(|opened| (id, opened)));
        let missing = id.is_some() && scenario.is_none();
        let (id, opened) = match scenario {
            Some((id, opened)) => (Some(id), Some(opened)),
            None => (None, Self::open_side(&self.binding, &self.areas, None)),
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
        crate::shared::project_id::link_for(&format!("street.html?street={}", self.edge), &*self.ports.storage)
    }

    fn back_key(&self) -> &'static str {
        "cmp-back-street"
    }

    fn place_name(&self) -> Said {
        self.sides.borrow().0.street.title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::scenario::Action;
    use crate::city::store::CityStore;
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{FakePage, MemoryStorage, Page, RecordingAnnouncer, Storage, test_ports_with_time};
    use crate::shared::scoped::ScopedStorage;

    struct Rig {
        vm: Option<Rc<StreetCompareVm>>,
        binding: CityBinding,
        page: Rc<FakePage>,
        said: Rc<RecordingAnnouncer>,
        storage: Rc<MemoryStorage>,
        ports: Ports,
    }

    impl Rig {
        fn vm(&self) -> &Rc<StreetCompareVm> {
            self.vm.as_ref().unwrap()
        }
    }

    /// A city with one street and the given scenarios, each widening the first piece by its amount; the page at
    /// `/compare.html?street=E` plus `query`. `read_only` opens it as a read-only project view.
    fn rig(widths: &[i32], query: &str, read_only: bool) -> Rig {
        let (mut ports, said, storage, _) = test_ports_with_time();
        let page = Rc::new(FakePage::default());
        ports.page = page.clone();
        let own: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), "p1"));
        let store = CityStore::sample(own.clone());
        let edge = store.open().view(0).edges[0].uid;
        let binding = CityBinding { store, place: Place::Street(edge) };
        for (i, w) in widths.iter().enumerate() {
            let id = binding.act(Action::Create(format!("Scenario {i}"))).unwrap().unwrap();
            let mut s = binding.street_editor(Some(id)).unwrap().snapshot();
            s.segments[0].width_mm += w;
            binding.keep_street_in(s, Some(id)).unwrap();
        }
        *page.path.borrow_mut() = format!("/compare.html?street={edge}{query}");
        let scoped: Rc<dyn Storage> = if read_only { Rc::new(ScopedStorage::new(storage.clone(), "p1").only_to_read()) } else { own };
        ports.storage = scoped.clone();
        let binding = CityBinding { store: CityStore::sample(scoped), place: Place::Street(edge) };
        let vm = StreetCompareVm::new(ports.clone(), crate::i18n_for(Locale::En), binding.clone());
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
        let r = rig(&[500, 900], "", false);
        let first = r.binding.scenarios()[0].0;
        let state = r.vm().compare_state();
        assert_eq!((state.pick_now(Pick::A), state.pick_now(Pick::B)), (None, Some(first)));
        assert!(state.refusal().is_none());
    }

    #[test]
    fn a_wider_piece_in_b_is_in_space_pieces_and_checks() {
        let r = rig(&[3000], "", false);
        let summary = r.vm().summary();
        assert!(!group(&summary, "cmp-h-space").lines.is_empty(), "{summary:?}");
        assert!(group(&summary, "cmp-h-pieces").lines.iter().any(|l| l.key == "cmp-piece-width"), "{summary:?}");
        let checks = group(&summary, "cmp-h-checks");
        assert!(!checks.lines.is_empty(), "a piece 3 m wider flips a check: {summary:?}");
        assert!(checks.lines.iter().all(|l| l.key == "cmp-only-passes" || l.key == "cmp-only-fails"));
        assert!(!summary.is_same());
    }

    #[test]
    fn a_place_with_no_scenarios_compares_the_baseline_with_itself() {
        let r = rig(&[], "", false);
        let state = r.vm().compare_state();
        assert_eq!((state.pick_now(Pick::A), state.pick_now(Pick::B)), (None, None));
        assert!(r.vm().summary().is_same());
    }

    #[test]
    fn the_same_design_twice_is_the_same() {
        let r = rig(&[500, 900], "", false);
        let id = r.binding.scenarios()[1].0;
        r.vm().choose(Pick::A, Some(id));
        r.vm().choose(Pick::B, Some(id));
        let summary = r.vm().summary();
        assert!(summary.is_same(), "{summary:?}");
        assert!(summary.also.is_empty());
    }

    #[test]
    fn a_deleted_scenario_in_the_address_falls_back_and_is_said() {
        let r = rig(&[500], "&b=99", false);
        assert_eq!(r.vm().compare_state().pick_now(Pick::B), None);
        assert_eq!(r.vm().compare_state().refusal(), Some(Said::new("scenario-missing")));
    }

    #[test]
    fn choosing_a_deleted_scenario_falls_back_to_the_baseline_and_says_so() {
        let r = rig(&[500], "", false);
        r.vm().choose(Pick::B, Some(99));
        assert_eq!(r.vm().compare_state().pick_now(Pick::B), None);
        assert_eq!(r.vm().compare_state().refusal(), Some(Said::new("scenario-missing")));
        r.vm().choose(Pick::B, None);
        assert!(r.vm().compare_state().refusal().is_none());
    }

    #[test]
    fn choosing_rewrites_the_address_and_says_the_headline() {
        let r = rig(&[500, 900], "", false);
        let second = r.binding.scenarios()[1].0;
        r.vm().choose(Pick::B, Some(second));
        let path = r.page.path();
        assert!(path.ends_with(&format!("a=base&b={second}")), "{path}");
        let said = r.said.take();
        let n = r.vm().summary().groups.iter().map(|g| g.lines.len()).sum::<usize>() + r.vm().summary().also.len();
        assert_eq!(said, [format!("Scenario 1 differs from Baseline in {n} ways.")]);
    }

    #[test]
    fn opening_and_choosing_write_nothing() {
        let r = rig(&[500, 900], "", false);
        let before = stored(&r.storage);
        let again = StreetCompareVm::new(r.ports.clone(), crate::i18n_for(Locale::En), r.binding.clone()).unwrap();
        let ids: Vec<u32> = r.binding.scenarios().iter().map(|(id, _)| *id).collect();
        again.choose(Pick::A, Some(ids[1]));
        again.choose(Pick::B, None);
        assert_eq!(stored(&r.storage), before);
    }

    #[test]
    fn read_only_back_link_keeps_view() {
        let r = rig(&[500], "&view=1", true);
        let vm = r.vm();
        assert!(vm.back_href().contains("view=1"), "{}", vm.back_href());
        assert!(vm.back_href().contains("street.html?street="));
        assert_eq!(vm.back_key(), "cmp-back-street");
    }

    #[test]
    fn the_baseline_is_named_in_the_language_of_the_page() {
        let r = rig(&[], "", false);
        r.vm().i18n().set(Locale::FrCa);
        r.vm().choose(Pick::B, None);
        assert_eq!(r.said.take(), ["Référence et Référence sont identiques."]);
    }
}
