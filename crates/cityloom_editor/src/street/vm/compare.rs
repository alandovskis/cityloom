//! The street half of the comparison page: two designs of one street (the baseline or a scenario each), opened
//! read-only, their views for the drawings, and what differs between them as `Said` data.
//!
//! The view-model never writes: it reads the designs as the city keeps them and rewrites the page's address (the
//! picks), nothing else. It takes ports already scoped to the project (`project::context::peek`).

use std::rc::Rc;

use crate::city::binding::{CityBinding, Place};
use crate::restriction::model::{Finding, Restriction};
use crate::shared::compare::{Compare, CompareHost, CompareState, DiffGroup, Pick, Summary};
use crate::shared::i18n::I18n;
use crate::shared::ports::Ports;
use crate::shared::said::{Arg, Said};
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
    shared: Compare<Side>,
    binding: CityBinding,
    edge: u32,
    /// The areas the street lies in, read once when the page opens.
    areas: Vec<Restriction>,
}

impl StreetCompareVm {
    /// The comparison of the street `binding` names, on the designs the page's address picks (`a=`, `b=`).
    /// None where the baseline cannot be opened (not a street, or not in the city).
    pub fn new(ports: Ports, i18n: Rc<I18n>, binding: CityBinding) -> Option<Rc<Self>> {
        let Place::Street(edge) = binding.place else { return None };
        let areas = super::areas_holding(&ports, Some(&binding));
        let shared = Compare::open(ports, i18n, binding.scenarios(), |id| Self::open_side(&binding, &areas, id))?;
        Some(Rc::new(StreetCompareVm { shared, binding, edge, areas }))
    }

    fn open_side(binding: &CityBinding, areas: &[Restriction], id: Option<u32>) -> Option<Side> {
        let editor = binding.street_editor(id)?;
        let street = editor.snapshot();
        let findings = areas.iter().map(|r| Finding::of(r, &street)).collect();
        Some(Side { street, view: Rc::new(editor.view()), findings })
    }

    /// The two designs' views, for the drawings (watched: they change when a side is chosen again).
    pub fn views(&self) -> (Rc<View>, Rc<View>) {
        self.shared.read(|a, b| (a.view.clone(), b.view.clone()))
    }

    fn summary_of(a: &Side, b: &Side) -> Summary {
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
        self.shared.i18n.clone()
    }

    fn compare_state(&self) -> &CompareState {
        &self.shared.state
    }

    fn choose(&self, side: Pick, id: Option<u32>) {
        self.shared.choose(side, id, self.binding.scenarios(), |id| Self::open_side(&self.binding, &self.areas, id), Self::summary_of);
    }

    fn summary(&self) -> Summary {
        self.shared.read(Self::summary_of)
    }

    fn back_href(&self) -> String {
        crate::shared::project_id::link_for(&format!("street.html?street={}", self.edge), &*self.shared.ports.storage)
    }

    fn back_key(&self) -> &'static str {
        "cmp-back-street"
    }

    fn place_name(&self) -> Said {
        self.shared.read_now(|a, _| a.street.title())
    }

    fn units(&self) -> Units {
        self.shared.units()
    }

    fn set_units(&self, units: Units) {
        self.shared.set_units(units);
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

    /// A city with one street whose baseline is the sample's street changed by `baseline`, and one scenario per edit,
    /// each made from that baseline and then changed by its edit; the page at `/compare.html?street=E`, comparing the
    /// baseline with the first scenario.
    fn rig_with(baseline: fn(&mut Street), edits: &[fn(&mut Street)]) -> Rig {
        let (mut ports, said, storage, _) = test_ports_with_time();
        let page = Rc::new(FakePage::default());
        ports.page = page.clone();
        let own: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), "p1"));
        let store = CityStore::sample(own.clone());
        let edge = store.open().view(0).edges[0].uid;
        let binding = CityBinding { store, place: Place::Street(edge) };
        let mut base = binding.street_editor(None).unwrap().snapshot();
        baseline(&mut base);
        binding.keep_street_in(base, None).unwrap();
        for (i, edit) in edits.iter().enumerate() {
            let id = binding.act(Action::Create(format!("Scenario {i}"))).unwrap().unwrap();
            let mut s = binding.street_editor(Some(id)).unwrap().snapshot();
            edit(&mut s);
            binding.keep_street_in(s, Some(id)).unwrap();
        }
        *page.path.borrow_mut() = format!("/compare.html?street={edge}");
        ports.storage = own.clone();
        let binding = CityBinding { store: CityStore::sample(own), place: Place::Street(edge) };
        let vm = StreetCompareVm::new(ports.clone(), crate::i18n_for(Locale::En), binding.clone());
        Rig { vm, binding, page, said, storage, ports }
    }

    /// The baseline as the city keeps it.
    fn baseline_of(r: &Rig) -> Street {
        r.binding.street_editor(None).unwrap().snapshot()
    }

    fn kind(id: &str) -> usize {
        crate::shared::catalogue::kind_index(id).unwrap()
    }

    fn piece_lines(r: &Rig) -> Vec<Said> {
        group(&r.vm().summary(), "cmp-h-pieces").lines.clone()
    }

    /// The lines as the page says them in English.
    fn english(lines: &[Said]) -> Vec<String> {
        lines.iter().map(crate::shared::testing::en).collect()
    }

    #[test]
    fn a_piece_added_in_b_is_listed_with_its_kind_and_width() {
        let r = rig_with(
            |_| {},
            &[|s| {
                s.segments.insert(1, crate::street::model::Segment::new(s.next_uid, kind("bike"), 1_500));
                s.next_uid += 1;
            }],
        );
        let lines = piece_lines(&r);
        let added = Said::new("cmp-piece-added").with("kind", Arg::KindLower("bike")).with("width", Arg::Length(1_500));
        assert_eq!(lines, [added], "the one piece added, and nothing else");
        assert_eq!(english(&lines), ["Added: bike lane, 1.5 m"]);
    }

    #[test]
    fn a_piece_removed_in_b_is_listed_with_its_kind_and_width() {
        let r = rig_with(
            |_| {},
            &[|s| {
                s.segments.pop();
            }],
        );
        let gone = baseline_of(&r).segments.last().unwrap().clone();
        assert_eq!((gone.kind, gone.width_mm), (kind("sidewalk"), 3_500), "the sample's last piece");
        let lines = piece_lines(&r);
        let removed = Said::new("cmp-piece-removed").with("kind", Arg::KindLower("sidewalk")).with("width", Arg::Length(3_500));
        assert_eq!(lines, [removed], "the one piece removed, and nothing else");
        assert_eq!(english(&lines), ["Removed: sidewalk, 3.5 m"]);
    }

    #[test]
    fn a_piece_of_another_kind_in_b_is_listed_as_becoming_it() {
        let r = rig_with(
            |_| {},
            &[|s| {
                let bike = kind("bike");
                s.segments[1].kind = bike;
                s.segments[1].material = crate::shared::catalogue::KINDS[bike].materials[0];
                s.segments[1].curb = None;
            }],
        );
        assert_eq!(baseline_of(&r).segments[1].kind, kind("planting"), "the sample's second piece");
        let lines = piece_lines(&r);
        let becomes = Said::new("cmp-piece-kind").with("from", Arg::Kind("planting")).with("to", Arg::KindLower("bike"));
        assert_eq!(lines, [becomes], "the one piece of another kind, neither added nor removed");
        assert_eq!(english(&lines), ["Planting strip becomes bike lane"]);
    }

    #[test]
    fn each_other_difference_is_named_in_also() {
        use crate::shared::catalogue::{KINDS, Side};
        use crate::street::model::{Calming, CalmingKind, Green, GreenKind, Instance, Speed, Stop, StopKind, TreeSize, Use, Zone};
        /// Two stretches, the second from 20 m on with its own section, its first piece widened by `widen_mm`.
        fn zones(s: &mut Street, widen_mm: i32) {
            s.length_mm = Some(60_000);
            let mut section = s.segments.clone();
            section[0].width_mm += widen_mm;
            let zone = |from_mm, segments| Zone { from_mm, segments, name: None, speed: Speed::Undecided };
            s.zones = vec![zone(0, None), zone(20_000, Some(section))];
        }
        let cases: [(&str, fn(&mut Street)); 7] = [
            ("cmp-other-speed", |s| s.speed = if s.speed == Speed::Kmh(30) { Speed::Kmh(40) } else { Speed::Kmh(30) }),
            ("cmp-other-stretches", |s| zones(s, 1_000)),
            ("cmp-other-piece-details", |s| {
                let p = s.segments.iter_mut().find(|p| KINDS[p.kind].materials.len() > 1).unwrap();
                p.material = KINDS[p.kind].materials.iter().copied().find(|&m| m != p.material).unwrap();
            }),
            ("cmp-other-curbside", |s| s.curb.push(Instance { side: Side::Right, from_mm: 1_000, length_mm: 5_000, use_: Use::Loading })),
            ("cmp-other-stops", |s| {
                s.stops.push(Stop { side: Side::Right, from_mm: 8_000, length_mm: 12_000, kind: StopKind::Curb, shelter: false, name: None })
            }),
            ("cmp-other-calming", |s| s.calming.push(Calming { from_mm: 30_000, length_mm: 7_000, kind: CalmingKind::Table, first: None })),
            ("cmp-other-green", |s| {
                s.green.push(Green { kind: GreenKind::Tree, from_mm: 2_000, length_mm: 500, offset_mm: 1_000, width_mm: 800, size: Some(TreeSize::M) })
            }),
        ];
        let edits: Vec<fn(&mut Street)> = cases.iter().map(|(_, edit)| *edit).collect();
        let r = rig_with(|s| zones(s, 500), &edits);
        let ids: Vec<u32> = r.binding.scenarios().iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), cases.len());
        for ((key, _), id) in cases.iter().zip(ids) {
            r.vm().choose(Pick::B, Some(id));
            let also: Vec<&str> = r.vm().summary().also.iter().map(|s| s.key).collect();
            assert_eq!(also, [*key], "the scenario for {key}");
        }
        r.vm().choose(Pick::B, None);
        assert!(r.vm().summary().also.is_empty(), "the baseline against itself differs in none of them");
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
