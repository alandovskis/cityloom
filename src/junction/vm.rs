//! The junction page's view-model: the junction being edited, what is said when
//! it changes, and keeping what is made in the city. The views read its view and
//! send it what the person does.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::city::binding::{CityBinding, Place};
use crate::junction::frame::Frame;
use crate::junction::model::{Junction, Refusal, Target};
use crate::junction::read_model::JView;
use crate::junction::text;
use crate::shared::core::{Core, Presents};
use crate::shared::i18n::{Args, I18n};
use crate::shared::keeper::Keeper;
use crate::shared::ports::Ports;
use crate::shared::said::say_now;
use crate::shared::units::Units;
use crate::sync::model::{Change, Kind, Origin};

impl Presents for Junction {
    type View = JView;
    fn present(&self) -> JView {
        self.view()
    }
}

pub const VIEW_KEY: &str = "cityloom-junction-view";

/// How the junction is drawn: the plan, which is edited, or the 2.5D picture, which is only looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawMode {
    Plan,
    Iso,
}

impl DrawMode {
    pub fn key(self) -> &'static str {
        match self {
            DrawMode::Plan => "plan",
            DrawMode::Iso => "iso",
        }
    }

    pub fn parse(s: &str) -> Option<DrawMode> {
        match s {
            "plan" => Some(DrawMode::Plan),
            "iso" => Some(DrawMode::Iso),
            _ => None,
        }
    }
}

pub struct JunctionVm {
    core: Core<Junction>,
    mode: RwSignal<DrawMode>,
    ports: Ports,
    i18n: Rc<I18n>,
    keeper: Option<Rc<Keeper>>,
    binding: Option<CityBinding>,
    /// How the plan was last drawn, for turning a pointer position into a point on it.
    frame: Cell<Option<Frame>>,
}

impl JunctionVm {
    /// A junction to edit, saying what it says in the language of `i18n`. Bound to a place in the
    /// city, what is made is written back to it; otherwise it is a sandbox.
    pub fn new(ports: Ports, i18n: Rc<I18n>, junction: Junction, binding: Option<CityBinding>) -> Rc<JunctionVm> {
        Rc::new_cyclic(|me: &std::rc::Weak<JunctionVm>| {
            let keeper = binding.is_some().then(|| {
                let (keep, told) = (me.clone(), me.clone());
                Keeper::new(
                    ports.scheduler.clone(),
                    move || keep.upgrade().is_some_and(|vm| vm.keep_now()),
                    move || {
                        if let Some(vm) = told.upgrade() {
                            vm.ports.announcer.say(&vm.i18n.tr_now("not-kept", &Args::new()));
                        }
                    },
                )
            });
            let mode = ports.storage.recall(VIEW_KEY).as_deref().and_then(DrawMode::parse).unwrap_or(DrawMode::Plan);
            JunctionVm { core: Core::new(junction), mode: RwSignal::new(mode), ports, i18n, keeper, binding, frame: Cell::new(None) }
        })
    }

    // ---- what the views read ----

    /// The words of the page, in its language.
    pub fn i18n(&self) -> &Rc<I18n> {
        &self.i18n
    }

    /// The view, which a view that reads it draws again when the junction changes.
    pub fn view(&self) -> Rc<JView> {
        self.core.view()
    }

    /// The same, for a command: nothing is watched.
    pub fn view_now(&self) -> Rc<JView> {
        self.core.view_now()
    }

    pub fn units(&self) -> Units {
        self.core.units()
    }

    pub fn units_now(&self) -> Units {
        self.core.units_now()
    }

    pub fn version(&self) -> RwSignal<u32> {
        self.core.version()
    }

    pub fn read<R>(&self, f: impl FnOnce(&Junction) -> R) -> R {
        self.core.read(f)
    }

    pub fn frame(&self) -> Option<Frame> {
        self.frame.get()
    }

    /// How the junction is drawn, which a view that reads it draws again when it changes.
    pub fn mode(&self) -> DrawMode {
        self.mode.get()
    }

    /// The same, for a command: nothing is watched.
    pub fn mode_now(&self) -> DrawMode {
        self.mode.get_untracked()
    }

    /// Draws the junction in `mode`, keeps the choice and says it. Choosing the mode it is already in says nothing,
    /// whether or not the browser kept the choice.
    pub fn set_mode(&self, mode: DrawMode) {
        if self.mode.get_untracked() == mode {
            return;
        }
        self.mode.set(mode);
        self.ports.storage.remember(VIEW_KEY, mode.key());
        let key = match mode {
            DrawMode::Plan => "jn-view-shown-plan",
            DrawMode::Iso => "jn-view-shown-iso",
        };
        self.ports.announcer.say(&self.i18n.tr_now(key, &Args::new()));
    }

    pub fn toggle_mode(&self) {
        self.set_mode(if self.mode_now() == DrawMode::Plan { DrawMode::Iso } else { DrawMode::Plan });
    }

    // ---- what the views do ----

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
    }

    pub fn set_frame(&self, frame: Frame) {
        self.frame.set(Some(frame));
    }

    /// Changes the junction, tells whatever watches it, and sees that it is kept.
    pub fn edit<R>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        let r = self.core.edit(f);
        if let Some(k) = &self.keeper {
            k.touch();
        }
        r
    }

    /// An edit the person made: it is announced by what it was, and a refusal by
    /// its reason.
    pub fn apply<R: Into<bool> + Copy>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        self.apply_noting(f, true)
    }

    /// `apply`; `note_patterns` adds a sentence for each phase the edit took off its ready-made pattern (a choice of
    /// pattern does not: that is what it is for).
    fn apply_noting<R: Into<bool> + Copy>(&self, f: impl FnOnce(&mut Junction) -> R, note_patterns: bool) -> R {
        let before = (note_patterns).then(|| self.read(|j| j.current().signal_plan.is_some().then(|| j.current().clone()))).flatten();
        let r = self.edit(f);
        if r.into() {
            let lost = before.map(|b| self.read(|j| crate::junction::signal::patterns_lost(&b, j.current(), j.region))).unwrap_or_default();
            self.say_edit_with(&lost);
        } else {
            self.say_refusal();
        }
        r
    }

    /// The person chooses a turbo layout (`None`: the single lane). Choosing the one already chosen changes nothing, so it
    /// is neither refused nor announced and leaves no history.
    /// Whether the layout is the one chosen afterwards (it is not when the choice was refused).
    pub fn choose_turbo(&self, layout: Option<usize>) -> bool {
        let chosen = || self.read(|j| j.current().turbo.as_ref().map(|t| t.layout));
        if chosen() != layout {
            self.apply(|j| j.set_turbo(layout));
        }
        chosen() == layout
    }

    /// The person gives phase `index` a ready-made pattern. Choosing the pattern it is already changes nothing, so it
    /// is neither refused nor announced and leaves no history. Whether the phase is that pattern afterwards.
    pub fn choose_pattern(&self, index: usize, pattern: crate::junction::signal::Pattern) -> bool {
        let chosen = || {
            self.read(|j| {
                let phase = crate::junction::signal::plan(j.current(), j.region).and_then(|p| p.phases.get(index).cloned());
                phase.is_some_and(|ph| crate::junction::signal::pattern_of(j.current(), j.region, &ph) == pattern)
            })
        };
        if !chosen() {
            self.apply_noting(|j| j.set_phase_pattern(index, pattern), false);
        }
        chosen()
    }

    /// A change that is not announced, as each step of a drag is.
    pub fn quiet<R>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        self.edit(f)
    }

    /// A selection the person made, announced by what is selected.
    pub fn select(&self, target: Target) {
        self.edit(|j| j.select(target));
        self.say_selection();
    }

    /// Moves the selection to the next thing round the junction, and says what.
    pub fn select_relative(&self, dir: i32) {
        self.edit(|j| j.select_relative(dir));
        self.say_selection();
    }

    /// The end of a gesture: kept when `commit`, and announced as an edit if it
    /// changed anything, or else by what is selected.
    pub fn end_gesture(&self, commit: bool) {
        if commit {
            if self.edit(|j| j.end_gesture()) {
                self.say_edit();
            } else {
                self.say_selection();
            }
        } else {
            self.edit(|j| j.cancel_gesture());
        }
    }

    pub fn undo(&self) -> bool {
        let done = self.edit(|j| j.undo());
        if done {
            self.ports.announcer.say(&self.i18n.tr_now("jn-undone", &Args::new()));
        }
        done
    }

    pub fn redo(&self) -> bool {
        let done = self.edit(|j| j.redo());
        if done {
            self.say_edit();
        }
        done
    }

    pub fn reset(&self) -> bool {
        let done = self.edit(|j| j.reset());
        if done {
            self.ports.announcer.say(&self.i18n.tr_now("jn-started-over", &Args::new()));
        }
        done
    }

    #[cfg(test)]
    pub fn load_sample(&self, sample: usize) {
        self.edit(|j| j.load_sample(sample));
        self.ports.announcer.say(&text::sample_text(&self.view_now(), &self.i18n));
    }

    /// As `StreetVm::watch_sync`.
    pub fn watch_sync(self: &Rc<Self>) {
        crate::sync::hub::watch(self, Self::flush, Self::on_synced);
    }

    /// A sync changed junctions or streets: the junction is drawn from both, so it is read again, and shown if it
    /// differs (the selection stays if it is still there).
    fn on_synced(&self, changes: &[Change]) {
        let Some(binding) = &self.binding else { return };
        let Place::Junction(node) = binding.place else { return };
        if !changes.iter().any(|c| matches!(c.id.kind, Kind::Junction | Kind::Street)) {
            return;
        }
        // As the street's: a later edit still waiting in the keeper is written first and wins.
        self.flush();
        let Some(mut fresh) = binding.store.open().junction_editor(node, binding.store.region()) else { return };
        // The state as drawn, the arms' street sections too (a snapshot leaves them out: they are the streets').
        if self.core.read(|j| j.drawn_as(&fresh)) {
            return;
        }
        fresh.select(self.core.read(|j| j.selected));
        let own = binding.record();
        let replaced = changes.iter().any(|c| Some(&c.id) == own.as_ref() && c.origin == Origin::Replaced);
        self.core.edit(|j| *j = fresh);
        self.ports.announcer.say(&self.i18n.tr_now(if replaced { "sync-replaced" } else { "sync-updated" }, &Args::new()));
    }

    /// The page is being left: what is waiting to be kept is kept now.
    pub fn flush(&self) {
        if let Some(k) = &self.keeper {
            k.flush();
        }
    }

    /// Writes the junction as it now is back to its place in the city.
    fn keep_now(&self) -> bool {
        let Some(binding) = &self.binding else { return true };
        binding.keep_junction(self.core.read(|j| j.snapshot()))
    }

    fn say_edit(&self) {
        self.say_edit_with(&[]);
    }

    /// The edit's text, then a sentence for each phase (from 0) the edit took off its ready-made pattern.
    fn say_edit_with(&self, lost: &[usize]) {
        if let Some(mut t) = text::edit_text(&self.view_now(), &self.i18n, self.units_now()) {
            for &i in lost {
                t = format!("{t} {}", self.i18n.tr_now("jn-pattern-lost", &Args::new().num("n", i as i64 + 1)));
            }
            self.ports.announcer.say(&t);
        }
    }

    fn say_refusal(&self) {
        let (why, arm) = self.core.read(|j| (j.refusal().unwrap_or(Refusal::DoesNotFit), j.refusal_arm().cloned()));
        let mut said = self.i18n.tr_now(why.key(), &Args::new());
        // A turbo refusal that is about one street (off square, or too narrow to widen) names it in a sentence of its own.
        let about_the_arm = match why {
            Refusal::TurboArmSkewed => Some("jn-refusal-turbo-skewed-arm"),
            Refusal::TurboEntryTooNarrow => Some("jn-refusal-turbo-entry-too-narrow-arm"),
            _ => None,
        };
        if let (Some(key), Some(arm)) = (about_the_arm, arm) {
            let arm = say_now(&self.i18n, self.units_now(), &arm);
            said = format!("{said} {}", self.i18n.tr_now(key, &Args::new().str("arm", arm)));
        }
        self.ports.announcer.say(&said);
    }

    fn say_selection(&self) {
        if let Some(t) = text::selection_text(&self.view_now(), &self.i18n, self.units_now()) {
            self.ports.announcer.say(&t);
        }
    }
}

impl crate::shell::Target for JunctionVm {
    fn set_units(&self, units: Units) {
        JunctionVm::set_units(self, units);
    }

    fn apply_region(&self, region: usize) -> bool {
        self.edit(|j| j.set_region(region))
    }

    fn region_id(&self) -> String {
        self.view_now().region.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::binding::Place;
    use crate::city::store::CityStore;
    use crate::shared::ports::{ManualScheduler, MemoryStorage, RecordingAnnouncer, Storage, test_ports_with_time};

    struct Rig {
        vm: Rc<JunctionVm>,
        said: Rc<RecordingAnnouncer>,
        time: Rc<ManualScheduler>,
        storage: Rc<MemoryStorage>,
    }

    fn rig(sample: usize) -> Rig {
        let (ports, said, storage, time) = test_ports_with_time();
        Rig { vm: JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::En), Junction::new(sample), None), said, time, storage }
    }

    #[test]
    fn a_junction_changed_by_a_sync_is_reloaded_and_announced() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let Place::Junction(node) = binding.place else { unreachable!() };
        let mut j = binding.store.open().junction_editor(node, 0).unwrap();
        let arm = j.current().arms[0].uid;
        assert!(j.set_corner(arm, 7_000));
        assert!(binding.store.write_junction(node, j.snapshot()));
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.vm.core.read(|j| j.current().arms[0].corner_mm), 7_000);
        assert_eq!(r.said.take(), ["Updated from another device."]);
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert!(r.said.take().is_empty(), "nothing differs now");
    }

    #[test]
    fn a_junction_edit_made_a_moment_before_a_sync_is_not_lost() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let arm = r.vm.edit(|j| j.current().arms[0].uid);
        assert!(r.vm.edit(|j| j.set_corner(arm, 7_000)));
        assert_eq!(binding.store.open().view(0).edited, 0, "still waiting in the keeper");
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.vm.core.read(|j| j.current().arms[0].corner_mm), 7_000);
        assert!(r.said.take().is_empty());
        assert_eq!(binding.store.open().view(0).edited, 1);
    }

    #[test]
    fn a_junction_whose_own_change_lost_says_so_and_keeps_its_selection() {
        use crate::junction::model::Target;
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let Place::Junction(node) = binding.place else { unreachable!() };
        let arm = r.vm.edit(|j| j.current().arms[0].uid);
        r.vm.edit(|j| j.select(Target::Arm(arm)));
        r.time.advance(1_000);
        let mut j = binding.store.open().junction_editor(node, 0).unwrap();
        assert!(j.set_corner(arm, 7_000));
        assert!(binding.store.write_junction(node, j.snapshot()));
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Replaced }]);
        assert_eq!(r.said.take(), ["Your change to this place was replaced by a later one."]);
        assert_eq!(r.vm.core.read(|j| j.selected), Target::Arm(arm), "the selection survives the reload");
    }

    #[test]
    fn a_change_to_an_arms_street_reloads_the_junction() {
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let Place::Junction(node) = binding.place else { unreachable!() };
        let city = binding.store.open();
        let edge = city.view(0).edges.iter().find(|e| e.a == node || e.b == node).expect("an arm's street").uid;
        let mut s = city.street_editor(edge, 0).unwrap().snapshot();
        s.segments[0].width_mm -= 100;
        s.segments[1].width_mm += 100;
        let sections = |r: &Rig| r.vm.core.read(|j| j.current().arms.iter().map(|a| a.section.clone()).collect::<Vec<_>>());
        let before = sections(&r);
        assert!(binding.store.write_street(edge, s));
        r.said.take();
        let key = binding.store.open().street_key(edge).unwrap();
        hub::publish(&[Change { id: crate::sync::model::Id::new(crate::sync::model::Kind::Street, &key), origin: Origin::Remote }]);
        assert_ne!(sections(&r), before, "the junction read its arm's street again");
        assert_eq!(r.said.take(), ["Updated from another device."]);
    }

    fn city_rig() -> (Rig, CityBinding) {
        let (ports, said, storage, time) = test_ports_with_time();
        let store = CityStore::sample(ports.storage.clone());
        let node = store.open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let junction = store.open().junction_editor(node, 0).unwrap();
        let binding = CityBinding { store, place: Place::Junction(node) };
        (Rig { vm: JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::En), junction, Some(binding.clone())), said, time, storage }, binding)
    }

    #[test]
    fn a_refusal_for_an_arm_off_square_names_the_arm_in_a_second_sentence() {
        use crate::junction::model::ROUNDABOUT;
        let r = rig(0);
        assert!(r.vm.apply(|j| j.set_control(ROUNDABOUT)));
        assert!(r.vm.apply(|j| j.set_turbo(Some(0))));
        r.said.take();
        let east = arm(&r.vm, 90);
        assert!(!r.vm.apply(|j| j.set_bearing(east, 115)));
        let said = r.said.take();
        assert_eq!(said.len(), 1);
        assert!(said[0].starts_with("A street would be more than 20° off the square a turbo roundabout needs. Sample Street 1 ("), "{said:?}");
        assert!(said[0].ends_with(") is the one that would be off."), "{said:?}");
    }

    #[test]
    fn a_refusal_for_an_entry_too_narrow_to_widen_names_the_street_in_a_second_sentence() {
        use crate::junction::model::ROUNDABOUT;
        let r = rig(0);
        assert!(r.vm.apply(|j| j.set_control(ROUNDABOUT)));
        r.said.take();
        assert!(!r.vm.apply(|j| j.set_turbo(Some(4))), "a rotor wants three entry lanes on Sample Street 1, which has not the room");
        let said = r.said.take();
        assert_eq!(said.len(), 1);
        assert!(said[0].starts_with("This street is too narrow to widen for the entry lanes this layout needs. Sample Street 1 ("), "{said:?}");
        assert!(said[0].ends_with(") is the one that is too narrow."), "{said:?}");
    }

    #[test]
    fn choosing_the_turbo_layout_already_chosen_changes_nothing_and_says_nothing() {
        use crate::junction::model::ROUNDABOUT;
        let r = rig(0);
        assert!(r.vm.apply(|j| j.set_control(ROUNDABOUT)));
        r.said.take();
        r.vm.choose_turbo(Some(0));
        let (said, version) = (r.said.take(), r.vm.version().get_untracked());
        assert_eq!(said.len(), 1, "the real choice is announced: {said:?}");
        r.vm.choose_turbo(Some(0));
        assert!(r.said.take().is_empty(), "no refusal and no edit is announced");
        assert_eq!(r.vm.version().get_untracked(), version, "nothing changed");
        assert!(r.vm.undo() && r.vm.read(|j| j.current().turbo.is_none()), "one undo takes the real choice back, so the repeat added no entry");
        r.said.take();
        r.vm.choose_turbo(None);
        assert!(r.said.take().is_empty(), "the single lane, chosen again, is the same");
    }

    fn arm(vm: &JunctionVm, bearing: i32) -> u32 {
        vm.read(|j| j.current().arms.iter().find(|a| a.bearing == bearing).unwrap().uid)
    }

    #[test]
    fn an_edit_changes_the_view_and_the_version_and_a_view_is_built_once_per_edit() {
        let r = rig(0);
        let (a, version) = (r.vm.view_now(), r.vm.version());
        assert!(Rc::ptr_eq(&a, &r.vm.view_now()));
        assert_eq!(version.get_untracked(), 0);
        let n = arm(&r.vm, 0);
        assert!(r.vm.edit(|j| j.set_corner(n, 7_000)));
        assert_eq!(version.get_untracked(), 1);
        let b = r.vm.view_now();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(b.arms.iter().find(|x| x.uid == n).unwrap().corner_mm, 7_000);
    }

    #[test]
    fn the_mode_starts_as_the_plan_toggles_and_is_announced() {
        let r = rig(0);
        assert_eq!(r.vm.mode_now(), DrawMode::Plan);
        r.vm.toggle_mode();
        assert_eq!(r.vm.mode_now(), DrawMode::Iso);
        r.vm.set_mode(DrawMode::Iso);
        r.vm.toggle_mode();
        assert_eq!(r.said.take(), vec!["Showing the junction in 2.5D.", "Showing the plan."]);
    }

    #[test]
    fn choosing_the_view_already_shown_says_nothing_even_before_anything_is_kept() {
        let r = rig(0);
        r.vm.set_mode(DrawMode::Plan);
        assert_eq!(r.said.take(), Vec::<String>::new());
        assert_eq!(r.storage.recall(VIEW_KEY), None);
    }

    #[test]
    fn the_mode_is_kept_across_a_new_view_model_and_garbage_falls_back_to_the_plan() {
        let (ports, _, storage) = crate::shared::ports::test_ports();
        let en = crate::i18n_for(crate::shared::i18n::Locale::En);
        JunctionVm::new(ports.clone(), en.clone(), Junction::new(0), None).set_mode(DrawMode::Iso);
        assert_eq!(storage.recall(VIEW_KEY).as_deref(), Some("iso"));
        assert_eq!(JunctionVm::new(ports.clone(), en.clone(), Junction::new(0), None).mode_now(), DrawMode::Iso);
        storage.remember(VIEW_KEY, "sideways");
        assert_eq!(JunctionVm::new(ports, en, Junction::new(0), None).mode_now(), DrawMode::Plan);
    }

    #[test]
    fn the_mode_speaks_french() {
        let (ports, said, _) = crate::shared::ports::test_ports();
        let vm = JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::FrCa), Junction::new(0), None);
        vm.set_mode(DrawMode::Iso);
        assert_eq!(said.take(), vec!["Jonction affichée en 2,5D."]);
    }

    #[test]
    fn the_units_start_in_metres_and_changing_them_is_not_an_edit() {
        let r = rig(0);
        assert_eq!(r.vm.units_now(), Units::Metres);
        let a = r.vm.view_now();
        r.vm.set_units(Units::Feet);
        assert_eq!(r.vm.units_now(), Units::Feet);
        assert_eq!(r.vm.version().get_untracked(), 0);
        assert!(Rc::ptr_eq(&a, &r.vm.view_now()));
    }

    #[test]
    fn the_plan_s_frame_is_kept_for_whoever_needs_to_read_a_point_from_it() {
        let r = rig(0);
        assert_eq!(r.vm.frame(), None);
        let f = Frame::fit([-10.0, -10.0, 10.0, 10.0], 1000.0, 800.0);
        r.vm.set_frame(f);
        assert_eq!(r.vm.frame(), Some(f));
    }

    #[test]
    fn an_edit_is_announced_by_its_label_and_a_refusal_by_its_reason() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        assert!(r.vm.apply(|j| j.set_corner(n, 7_000)));
        assert_eq!(r.said.take(), vec!["Corner after Sample Avenue 2 (north): 7.0 m radius. 4 streets, traffic signal. Every check passes."]);
        assert!(!r.vm.apply(|j| j.set_corner(n, 99_000)));
        assert_eq!(r.said.take(), vec!["That change does not fit."]);
        assert_eq!(r.vm.version().get_untracked(), 2, "a refused edit still asks the views to look again");
    }

    #[test]
    fn the_phase_commands_are_announced_and_refused_in_english_and_in_french() {
        let r = rig(0);
        assert!(r.vm.apply(|j| j.remove_phase(0)));
        assert!(r.said.take()[0].starts_with("Phase 1 for Sample Avenue 2 (north) removed."), "english");
        assert!(r.vm.apply(|j| j.restore_suggestion()));
        assert!(r.said.take()[0].starts_with("Signal plan restored to the suggestion."));
        assert!(!r.vm.apply(|j| j.restore_suggestion()));
        assert_eq!(r.said.take(), vec!["The signal plan is already the suggested one."]);
        assert!(!r.vm.apply(|j| j.move_phase(0, -1)));
        assert_eq!(r.said.take(), vec!["That phase cannot move any further that way."]);

        let (ports, said, _, _) = test_ports_with_time();
        let fr = JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::FrCa), Junction::new(0), None);
        assert!(fr.apply(|j| j.move_phase(0, 1)));
        let t = said.take();
        assert!(t[0].contains("reculée, maintenant 2 sur 4"), "{t:?}");
        assert!(fr.apply(|j| j.restore_suggestion()));
        assert!(said.take()[0].starts_with("Plan des feux rétabli à la suggestion."));
        assert!(!fr.apply(|j| j.move_phase(0, -1)));
        assert_eq!(said.take(), vec!["Cette phase ne peut pas aller plus loin dans ce sens."]);
    }

    #[test]
    fn a_selection_is_announced_and_clearing_it_says_nothing() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.select(Target::Arm(n));
        assert_eq!(r.said.take(), vec!["Sample Avenue 2 (north), 0 degrees"]);
        assert_eq!(r.vm.read(|j| j.selected), Target::Arm(n));
        r.vm.select(Target::None);
        assert!(r.said.take().is_empty());
        r.vm.select_relative(1);
        assert_eq!(r.said.take().len(), 1);
    }

    #[test]
    fn a_quiet_change_says_nothing() {
        let r = rig(0);
        r.vm.quiet(|j| j.set_control(2));
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn a_gesture_is_one_revision_announced_when_it_ends_and_none_when_cancelled_or_unchanged() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.select(Target::Arm(n));
        r.said.take();
        r.vm.quiet(|j| j.begin_gesture());
        assert!(r.vm.quiet(|j| j.drag_arm_to(n, 3_000.0, -10_000.0)));
        r.vm.quiet(|j| j.drag_arm_to(n, 5_000.0, -10_000.0));
        assert_eq!(r.vm.view_now().revisions.len(), 0);
        r.vm.end_gesture(true);
        assert_eq!(r.vm.view_now().revisions.len(), 1);
        assert!(r.said.take()[0].starts_with("Sample Avenue 2 bearing: 25°."));
        // Nothing changed: the selection is said instead.
        r.vm.quiet(|j| j.begin_gesture());
        r.vm.end_gesture(true);
        assert_eq!(r.said.take(), vec!["Sample Avenue 2 (north-east), 25 degrees"]);
        r.vm.quiet(|j| j.begin_gesture());
        r.vm.quiet(|j| j.drag_arm_to(n, 9_000.0, -10_000.0));
        r.vm.end_gesture(false);
        assert_eq!((r.vm.view_now().revisions.len(), r.vm.read(|j| j.arm(n).unwrap().bearing)), (1, 25));
    }

    #[test]
    fn undo_redo_start_over_and_a_sample_are_announced() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.edit(|j| j.set_corner(n, 7_000));
        assert!(r.vm.undo());
        assert_eq!(r.said.take(), vec!["Undone."]);
        assert!(r.vm.redo());
        assert!(r.said.take()[0].starts_with("Corner after"));
        assert!(r.vm.reset());
        assert_eq!(r.said.take(), vec!["Started over from the junction as it is today."]);
        assert!(!r.vm.undo());
        assert!(r.said.take().is_empty());
        r.vm.load_sample(1);
        assert_eq!(r.said.take(), vec!["Street and lane. 3 streets, side streets stop. Every check passes."]);
    }

    #[test]
    fn a_sandbox_keeps_nothing_in_storage() {
        let r = rig(0);
        let n = arm(&r.vm, 0);
        r.vm.edit(|j| j.set_corner(n, 7_000));
        r.time.advance(1_000);
        assert_eq!(r.time.pending(), 0);
        assert_eq!(r.storage.recall(crate::city::edits::EDITS_KEY), None);
    }

    #[test]
    fn what_is_made_of_a_city_junction_is_kept_in_the_city_a_moment_after_the_last_change() {
        let (r, binding) = city_rig();
        let n = r.vm.view_now().arms[0].uid;
        assert!(r.vm.edit(|j| j.set_corner(n, 7_000)));
        r.time.advance(200);
        assert_eq!(binding.store.open().view(0).edited, 0, "not yet");
        assert!(r.vm.edit(|j| j.set_corner(n, 7_500)));
        r.time.advance(200);
        assert_eq!(binding.store.open().view(0).edited, 0);
        r.time.advance(100);
        assert_eq!(binding.store.open().view(0).edited, 1);
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn leaving_the_page_keeps_what_is_waiting_at_once() {
        let (r, binding) = city_rig();
        let n = r.vm.view_now().arms[0].uid;
        r.vm.edit(|j| j.set_corner(n, 7_000));
        r.vm.flush();
        assert_eq!(binding.store.open().view(0).edited, 1);
    }

    #[test]
    fn a_city_that_cannot_be_written_is_said_once_and_the_page_goes_on() {
        let (r, _) = city_rig();
        r.storage.blocked(true);
        let n = r.vm.view_now().arms[0].uid;
        r.vm.edit(|j| j.set_corner(n, 7_000));
        r.time.advance(300);
        r.vm.edit(|j| j.set_corner(n, 7_500));
        r.time.advance(300);
        assert_eq!(r.said.take(), vec![crate::i18n_for(crate::shared::i18n::Locale::En).tr_now("not-kept", &crate::shared::i18n::Args::new())]);
        assert_eq!(r.vm.read(|j| j.arm(n).unwrap().corner_mm), 7_500);
    }

    #[test]
    fn only_selecting_on_a_place_never_edited_marks_nothing_for_the_sync() {
        let (r, _) = city_rig();
        let arm = r.vm.read(|j| j.current().arms[0].uid);
        r.vm.select(crate::junction::model::Target::Arm(arm));
        r.vm.flush();
        r.time.advance(1_000);
        assert_eq!(r.storage.recall(crate::sync::model::SYNC_KEY), None, "looking is not editing");
    }
}
