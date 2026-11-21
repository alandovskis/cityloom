//! The junction page's view-model: the junction being edited, what is said when
//! it changes, and keeping what is made in the city. The views read its view and
//! send it what the person does.

pub mod compare;
pub mod keys;
pub mod text;
pub mod watch;

use std::rc::Rc;

use leptos::prelude::*;

use crate::city::binding::{CityBinding, Place, ScenarioPage, scenario_actions};
use crate::junction::model::read_model::JView;
use crate::junction::model::signal;
use crate::junction::model::{Junction, Refusal, State, Target};
use crate::shared::core::{Core, Presents};
use crate::shared::i18n::{Args, I18n};
use crate::shared::keeper::Keeper;
use crate::shared::ports::Ports;
use crate::shared::said::Said;
use crate::shared::said::say_now;
use crate::shared::scenarios::{ScenarioHost, ScenarioState};
use crate::shared::units::Units;
use crate::sync::model::{Change, Kind, Origin};

impl Presents for Junction {
    type View = JView;
    fn present(&self) -> JView {
        self.view()
    }
}

pub struct JunctionVm {
    core: Core<Junction>,
    ports: Ports,
    i18n: Rc<I18n>,
    keeper: Option<Rc<Keeper>>,
    binding: Option<CityBinding>,
    goal: crate::shared::goals::GoalChoice,
    /// Whether the details show the signal plan, a level of its own under the junction's (nothing chosen).
    plan: ArcRwSignal<bool>,
    /// Whether the Checks tab shows its passing checks.
    passes: crate::shared::check_fold::PassFold,
    /// The junction's scenarios and which design is shown (the baseline when the page opens).
    scenarios: ScenarioState,
    /// Whether the junction keeps scenarios: it is a junction of the city with a key of its own.
    offers_scenarios: bool,
    /// The design shown as last opened or kept (as `StreetVm`'s): a write of the same design changes nothing.
    kept_design: std::cell::RefCell<State>,
}

/// A conflict of the signal plan: its phase's place, the phase's street, and what meets.
type Meeting = (usize, u32, crate::junction::model::signal::Conflict);

/// What an edit's announcement compares with afterwards, taken before it: what met in the plan and the calls' findings.
struct Before {
    meetings: Vec<Meeting>,
    /// What each finding was about (`calls::Finding::about`), not its words: a renamed street is no new finding.
    findings: Vec<(u8, u32)>,
    /// The crossings kept on call while the signal was suspended.
    kept: usize,
    /// The crossings taken off call as the junction opened (until its first edit): a sync says only more of them.
    dropped: usize,
}

/// Whether an edit says which phases it took off their ready-made pattern.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PatternNote {
    Report,
    Skip,
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
            let goal = crate::shared::goals::GoalChoice::new(ports.storage.clone());
            let offers_scenarios = binding.as_ref().is_some_and(|b| b.record().is_some());
            let scenarios = ScenarioState::default();
            if let Some(b) = &binding {
                scenarios.list(b.scenarios());
            }
            JunctionVm {
                kept_design: std::cell::RefCell::new(junction.snapshot()),
                core: Core::new(junction),
                ports,
                i18n,
                keeper,
                binding,
                goal,
                plan: ArcRwSignal::new(false),
                passes: Default::default(),
                scenarios,
                offers_scenarios,
            }
        })
    }

    // ---- what the views read ----

    /// The planning goal chosen (#72): it marks checks and lists measures, and changes nothing in the junction.
    pub fn goal(&self) -> &crate::shared::goals::GoalChoice {
        &self.goal
    }

    /// Whether the Checks tab shows its passing checks (folded at first, not remembered).
    pub fn passes(&self) -> &crate::shared::check_fold::PassFold {
        &self.passes
    }

    /// The words of the page, in its language.
    pub fn i18n(&self) -> &Rc<I18n> {
        &self.i18n
    }

    /// The project the junction is edited in; none outside a project.
    pub fn project(&self) -> Option<String> {
        self.ports.storage.project()
    }

    /// `href` to another page of the junction's project (`shared::project_id::link_for`): read-only where this one is.
    pub fn link(&self, href: &str) -> String {
        crate::shared::project_id::link_for(href, &*self.ports.storage)
    }

    /// Whether the junction is shown read-only (`Storage::read_only`): every edit is refused and said, and nothing is
    /// kept; choosing and looking go on. The page offers no edit then.
    pub fn read_only(&self) -> bool {
        self.ports.storage.read_only()
    }

    /// Says so, and is true, when the junction is shown read-only: the command that asks makes no edit.
    fn refuse_read_only(&self) -> bool {
        let read_only = self.read_only();
        if read_only {
            self.ports.announcer.say(&self.i18n.tr_now("read-only-refused", &Args::new()));
        }
        read_only
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

    // ---- what the views do ----

    pub fn set_units(&self, units: Units) {
        self.core.set_units(units);
    }

    /// Changes the junction, tells whatever watches it, and sees that it is kept. Shown read-only, whatever `f` did to
    /// the junction's history is put back (what is chosen stays), the refusal is said, `R::default()` (false, none) is
    /// returned and nothing is kept: the commands refuse first, and this holds for any change that reaches the junction
    /// some other way.
    pub fn edit<R: Default>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        if self.read_only() {
            let (r, put_back) = self.core.edit(|j| {
                let kept = j.clone();
                let r = f(j);
                let put_back = !j.same_history(&kept);
                if put_back {
                    *j = kept;
                }
                (r, put_back)
            });
            if put_back {
                self.refuse_read_only();
                return R::default();
            }
            return r;
        }
        let r = self.core.edit(f);
        if let Some(k) = &self.keeper {
            k.touch();
        }
        r
    }

    /// An edit the person made: it is announced by what it was, and a refusal by
    /// its reason. A phase it takes off its ready-made pattern is said too. Shown read-only, it is refused and said.
    pub fn apply<R: Into<bool> + Copy + Default>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        self.apply_with(f, PatternNote::Report)
    }

    /// `apply` for a choice of pattern, which says nothing of the phases' patterns: changing one is what it is for.
    fn apply_choosing_pattern<R: Into<bool> + Copy + Default>(&self, f: impl FnOnce(&mut Junction) -> R) -> R {
        self.apply_with(f, PatternNote::Skip)
    }

    fn apply_with<R: Into<bool> + Copy + Default>(&self, f: impl FnOnce(&mut Junction) -> R, note: PatternNote) -> R {
        if self.refuse_read_only() {
            return R::default();
        }
        let before = (note == PatternNote::Report).then(|| self.pattern_state()).flatten();
        let meeting = self.before();
        let r = self.edit(f);
        if r.into() {
            let lost = self.lost_since(before);
            self.say_edit_with(&lost, &meeting);
        } else {
            self.say_refusal();
        }
        r
    }

    /// The state and region, to compare patterns against later; None when the junction has no plan of the person's own
    /// (nothing there can be lost).
    fn pattern_state(&self) -> Option<(State, usize)> {
        self.read(|j| j.current().signal_plan.is_some().then(|| (j.current().clone(), j.region)))
    }

    /// The phases (from 0) that were on a ready-made pattern in `before` and are on none now.
    fn lost_since(&self, before: Option<(State, usize)>) -> Vec<usize> {
        let Some((state, region)) = before else { return Vec::new() };
        self.read(|j| crate::junction::model::signal::patterns_lost_across(&state, region, j.current(), j.region))
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
    pub fn choose_pattern(&self, index: usize, pattern: crate::junction::model::signal::Pattern) -> bool {
        let chosen = || {
            self.read(|j| {
                let phase = crate::junction::model::signal::plan(j.current(), j.region).and_then(|p| p.phases.get(index).cloned());
                phase.is_some_and(|ph| crate::junction::model::signal::pattern_of(j.current(), j.region, &ph) == pattern)
            })
        };
        if !chosen() {
            self.apply_choosing_pattern(|j| j.set_phase_pattern(index, pattern));
        }
        chosen()
    }

    /// The person switches the crossing of street `uid` in phase `index`: on if it is off there, else off. Announced by what
    /// was done, with a sentence when that takes the phase off its ready-made pattern; a refusal is said with its reason.
    pub fn toggle_phase_walk(&self, index: usize, uid: u32) -> bool {
        self.toggle_phase_movement(index, uid, false)
    }

    /// `toggle_phase_walk` for the bike movements of street `uid`.
    pub fn toggle_phase_ride(&self, index: usize, uid: u32) -> bool {
        self.toggle_phase_movement(index, uid, true)
    }

    fn toggle_phase_movement(&self, index: usize, uid: u32, ride: bool) -> bool {
        if self.refuse_read_only() {
            return false;
        }
        // The pattern of the phase, as the suggestion or the person's plan has it.
        let pattern = || {
            self.read(|j| {
                let phase = signal::plan(j.current(), j.region).and_then(|p| p.phases.get(index).cloned());
                phase.map(|ph| (signal::pattern_of(j.current(), j.region, &ph), if ride { ph.rides.contains(&uid) } else { ph.walks.contains(&uid) }))
            })
        };
        let Some((before, on)) = pattern() else {
            self.edit(|j| if ride { j.set_phase_ride(index, uid, true) } else { j.set_phase_walk(index, uid, true) });
            self.say_refusal();
            return false;
        };
        let meeting = self.before();
        let done = self.edit(|j| if ride { j.set_phase_ride(index, uid, !on) } else { j.set_phase_walk(index, uid, !on) });
        if done {
            let lost: Vec<usize> = match pattern() {
                Some((after, _)) if before != signal::Pattern::Custom && after == signal::Pattern::Custom => vec![index],
                _ => Vec::new(),
            };
            self.say_edit_with(&lost, &meeting);
        } else {
            self.say_refusal();
        }
        done
    }

    /// The person puts the crossing of street `uid` on call (`on`), or back on recall. Announced by what was done and
    /// how the plan then serves the crossing's calls; a refusal is said with its reason (no phase walks it, say).
    pub fn set_on_call(&self, uid: u32, on: bool) -> bool {
        self.call_edit(uid, |j| j.set_on_call(uid, on))
    }

    /// The person sets how many times an hour the crossing of street `uid`, on call, is called. Announced as
    /// `set_on_call` is.
    pub fn set_call_rate(&self, uid: u32, rate: i32) -> bool {
        self.call_edit(uid, |j| j.set_call_rate(uid, rate))
    }

    /// An edit of a crossing's calls: the edit's text, the crossing's calls added up and any calls finding that arose,
    /// or the refusal. It changes no phase, so nothing about the plan's conflicts or patterns is said. A rate the crossing
    /// had already is said not at all: `set_call_rate` takes it (returns true) without adding a revision, and only an
    /// edit that added one is announced, or the last revision, some earlier edit, would be read out.
    fn call_edit(&self, uid: u32, f: impl FnOnce(&mut Junction) -> bool) -> bool {
        // Shown read-only, refused first, as every edit command is: the read-only sentence and nothing else is said.
        if self.refuse_read_only() {
            return false;
        }
        // Only the findings are compared: a crossing's calls move no phase, so nothing about meetings can change.
        let (findings, edits) = (self.findings(), self.read(|j| j.revisions().count()));
        let done = self.edit(f);
        // A rate it had already is no edit: nothing was done, so nothing is said (the last edit is some other one).
        if done && self.read(|j| j.revisions().count()) != edits {
            let v = self.view_now();
            let edit = text::edit_text(&v, &self.i18n, self.units_now());
            self.say_parts(edit.into_iter().chain(text::calls_text(&v, &self.i18n, uid)).chain(self.findings_note(&findings)));
        } else if !done {
            self.say_refusal();
        }
        done
    }

    /// A selection the person made, announced by what is selected.
    pub fn select(&self, target: Target) {
        self.plan.set(false);
        self.edit(|j| j.select(target));
        self.say_selection();
    }

    /// Shows the signal plan as the details' level: nothing is chosen, and the plan is the junction's.
    pub fn show_plan(&self) {
        self.edit(|j| j.select(Target::None));
        self.plan.set(true);
    }

    /// Whether the details show the signal plan: asked for, nothing chosen since, and a signal to plan.
    pub fn plan_shown(&self) -> bool {
        let v = self.view();
        self.plan.get() && v.selected.kind.is_none() && v.signal.is_some()
    }

    /// Moves the selection to the next thing round the junction, and says what. Like any choice, it leaves the
    /// signal plan.
    pub fn select_relative(&self, dir: i32) {
        self.plan.set(false);
        self.edit(|j| j.select_relative(dir));
        self.say_selection();
    }

    pub fn undo(&self) -> bool {
        if self.refuse_read_only() {
            return false;
        }
        let meeting = self.before();
        let done = self.edit(|j| j.undo());
        if done {
            self.say_with_meetings(self.i18n.tr_now("jn-undone", &Args::new()), &meeting);
        }
        done
    }

    pub fn redo(&self) -> bool {
        if self.refuse_read_only() {
            return false;
        }
        let meeting = self.before();
        let done = self.edit(|j| j.redo());
        if done {
            self.say_edit_with(&[], &meeting);
        }
        done
    }

    pub fn reset(&self) -> bool {
        if self.refuse_read_only() {
            return false;
        }
        let meeting = self.before();
        let done = self.edit(|j| j.reset());
        if done {
            self.say_with_meetings(self.i18n.tr_now("jn-started-over", &Args::new()), &meeting);
        }
        done
    }

    #[cfg(test)]
    pub fn load_sample(&self, sample: usize) {
        if self.refuse_read_only() {
            return;
        }
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
        if !matches!(binding.place, Place::Junction(_)) {
            return;
        }
        if !changes.iter().any(|c| matches!(c.id.kind, Kind::Junction | Kind::Street)) {
            return;
        }
        // As the street's: a later edit still waiting in the keeper is written first and wins.
        self.flush();
        scenario_actions::refresh(self);
        // The scenario shown, as the city now keeps it; the baseline where it is gone (deleted elsewhere).
        let shown = self.scenarios.active_now();
        let Some(mut fresh) = self.open_design(shown) else {
            if shown.is_some() {
                // the scenario shown was deleted elsewhere: the baseline, said as that rather than as an update
                scenario_actions::gone(self, false);
            }
            return;
        };
        if self.core.read(|j| j.drawn_as(&fresh)) {
            // The state as drawn, the arms' street sections too (a snapshot leaves them out: they are the streets').
            return;
        }
        fresh.select(self.core.read(|j| j.selected));
        let own = binding.record();
        let replaced = changes.iter().any(|c| Some(&c.id) == own.as_ref() && c.origin == Origin::Replaced);
        let before = self.before();
        *self.kept_design.borrow_mut() = fresh.snapshot();
        self.core.edit(|j| *j = fresh);
        // The sync's sentence, then what it did to the calls, as an edit's announcement says it: crossings it took off
        // call (the junction is not a signal), a finding that arose, crossings kept on call as the signal is suspended.
        let synced = self.i18n.tr_now(if replaced { "sync-replaced" } else { "sync-updated" }, &Args::new());
        // Until its first edit the record still holds the calls, so each sync opens the junction without them again:
        // said only when this one took more off than were off before it.
        let dropped = self.view_now().calls_dropped;
        let dropped = (dropped > before.dropped).then(|| self.i18n.tr_now("jn-calls-dropped", &Args::new().num("n", dropped as i64)));
        self.say_parts(std::iter::once(synced).chain(dropped).chain(self.findings_note(&before.findings)).chain(self.kept_note(&before)));
    }

    /// Design `id` (None: the baseline) as the city keeps it now, its history begun there (`Junction::begin_history`).
    /// What both a switch and a sync reload show.
    fn open_design(&self, id: Option<u32>) -> Option<Junction> {
        let mut fresh = self.binding.as_ref()?.junction_editor(id)?;
        fresh.begin_history();
        Some(fresh)
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
        let state = self.core.read(|j| j.snapshot());
        // the same design, whatever step it is labelled as
        let changed = !crate::junction::model::same_design(&self.kept_design.borrow(), &state);
        let kept = binding.keep_junction_in(state.clone(), self.scenarios.active_now());
        if kept.is_ok() {
            *self.kept_design.borrow_mut() = state;
        }
        scenario_actions::kept(self, kept, changed)
    }

    /// The edit's text, then a sentence for the movements that now meet and did not before (`meeting`), then one for each
    /// phase (from 0) the edit took off its ready-made pattern.
    fn say_edit_with(&self, lost: &[usize], meeting: &Before) {
        if let Some(t) = text::edit_text(&self.view_now(), &self.i18n, self.units_now()) {
            self.say_parts(std::iter::once(t).chain(self.notes(meeting, lost)));
        }
    }

    /// Says `text`, then the sentence for the movements that meet now and did not before.
    fn say_with_meetings(&self, text: String, meeting: &Before) {
        self.say_parts(std::iter::once(text).chain(self.meeting_note(meeting)).chain(self.findings_note(&meeting.findings)).chain(self.kept_note(meeting)));
    }

    /// The sentences that follow an edit's text: the movements that now meet, the crossings on call it left unserved,
    /// then the phases off their pattern.
    fn notes(&self, meeting: &Before, lost: &[usize]) -> Vec<String> {
        self.meeting_note(meeting).into_iter().chain(self.findings_note(&meeting.findings)).chain(self.kept_note(meeting)).chain(self.lost_note(lost)).collect()
    }

    /// What an edit is compared against afterwards: the movements that met and the calls' findings before it.
    fn before(&self) -> Before {
        Before { meetings: self.meetings(), findings: self.findings(), kept: self.view_now().calls_kept, dropped: self.view_now().calls_dropped }
    }

    /// What the findings of the pedestrian calls are about now, each its kind and street.
    fn findings(&self) -> Vec<(u8, u32)> {
        self.view_now().signal.as_ref().map(|s| s.calls.found.iter().map(|f| f.about()).collect()).unwrap_or_default()
    }

    /// The crossings that stay on call now that the signal is suspended, when it was not before: they show nowhere
    /// else. None otherwise.
    fn kept_note(&self, before: &Before) -> Option<String> {
        let n = self.view_now().calls_kept;
        (n > 0 && before.kept == 0).then(|| self.i18n.tr_now("jn-calls-kept", &Args::new().num("n", n as i64)))
    }

    /// The calls' findings that arose since `before`, each its own sentence, as the plan shows them; one that was there
    /// already (the same kind at the same street, `calls::Finding::about`, whatever its words now) is not said again.
    /// Every announcement of a change says it: an edit of the calls or of the signal, an undo, a redo, a reset and a
    /// change of region. None when none arose.
    fn findings_note(&self, before: &[(u8, u32)]) -> Option<String> {
        let v = self.view_now();
        let calls = &v.signal.as_ref()?.calls;
        let new: Vec<String> = calls
            .found
            .iter()
            .zip(&calls.findings)
            .filter(|(f, _)| !before.contains(&f.about()))
            .map(|(_, said)| say_now(&self.i18n, self.units_now(), said))
            .collect();
        (!new.is_empty()).then(|| new.join(" "))
    }

    /// Announces the parts as one text.
    fn say_parts(&self, parts: impl Iterator<Item = String>) {
        self.ports.announcer.say(&parts.collect::<Vec<String>>().join(" "));
    }

    /// The movements that meet in the signal plan now, each with its phase's street.
    fn meetings(&self) -> Vec<Meeting> {
        self.read(|j| signal::meetings(j.current(), j.region))
    }

    /// One sentence for the conflicts that arose since `before`: the warning itself when there is one, a count and the
    /// phases (and the first of them) when there are several. None when none arose (one that was there already is not said again).
    fn meeting_note(&self, before: &Before) -> Option<String> {
        let v = self.view_now();
        let new: Vec<Meeting> = self.meetings().into_iter().filter(|(_, uid, c)| !before.meetings.iter().any(|(_, u, b)| u == uid && b == c)).collect();
        let (first, rest) = new.split_first()?;
        let first_said = say_now(&self.i18n, self.units_now(), &crate::junction::model::read_model::conflict_said(first.0, &first.2, &v.arms));
        if rest.is_empty() {
            return Some(self.i18n.tr_now("jn-conflict-heard", &Args::new().str("conflict", first_said)));
        }
        let mut phases: Vec<usize> = new.iter().map(|m| m.0).collect();
        phases.dedup();
        let list = self.number_list(&phases);
        let args = Args::new().num("count", new.len() as i64).num("phases", phases.len() as i64).str("list", list).str("first", first_said);
        Some(self.i18n.tr_now("jn-conflict-heard-many", &args))
    }

    /// "1", "1 and 2", "1, 2 and 3": the places (from 0) as phase numbers.
    fn number_list(&self, places: &[usize]) -> String {
        let numbers: Vec<String> = places.iter().map(|&i| (i + 1).to_string()).collect();
        let Some((last, rest)) = numbers.split_last() else { return String::new() };
        if rest.is_empty() {
            return last.clone();
        }
        let head =
            rest.iter().cloned().reduce(|head, tail| self.i18n.tr_now("jn-list-comma", &Args::new().str("head", head).str("tail", tail))).unwrap_or_default();
        self.i18n.tr_now("jn-ph-and", &Args::new().str("head", head).str("tail", last.clone()))
    }

    /// One sentence for the phases (from 0) that came off their ready-made pattern: "Phase 2 no longer follows ...", or
    /// "Phases 1, 2 and 3 no longer follow ..." for several. None when there are none.
    pub(super) fn lost_note(&self, lost: &[usize]) -> Option<String> {
        match lost {
            [] => None,
            [one] => Some(self.i18n.tr_now("jn-pattern-lost", &Args::new().str("n", (one + 1).to_string()))),
            _ => Some(self.i18n.tr_now("jn-pattern-lost-many", &Args::new().str("list", self.number_list(lost)))),
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
    /// The drive side is the design's: a page shown read-only cannot choose it.
    fn region_is_design(&self) -> bool {
        true
    }

    fn set_units(&self, units: Units) {
        JunctionVm::set_units(self, units);
    }
    fn project(&self) -> Option<String> {
        JunctionVm::project(self)
    }

    /// Shown read-only the junction keeps its own region (its drive side is the design): not taken and nothing said,
    /// so the region the shell gives on load, which may be the other side's, makes no edit and no refusal.
    fn apply_region(&self, region: usize) -> bool {
        if self.read_only() {
            return false;
        }
        let before = self.pattern_state();
        let meeting = self.before();
        let changed = self.edit(|j| j.set_region(region));
        if changed {
            let notes = self.notes(&meeting, &self.lost_since(before));
            if !notes.is_empty() {
                self.say_parts(notes.into_iter());
            }
        }
        changed
    }

    fn region_id(&self) -> String {
        self.view_now().region.to_string()
    }
}

impl ScenarioHost for JunctionVm {
    fn i18n(&self) -> Rc<I18n> {
        self.i18n.clone()
    }

    fn scenario_state(&self) -> &ScenarioState {
        &self.scenarios
    }

    fn scenarios_offered(&self) -> bool {
        self.offers_scenarios
    }

    fn read_only(&self) -> bool {
        JunctionVm::read_only(self)
    }

    fn switch_scenario(&self, id: Option<u32>) -> bool {
        scenario_actions::switch(self, id)
    }

    fn create_scenario(&self, name: &str) -> bool {
        scenario_actions::create(self, name)
    }

    fn duplicate_scenario(&self, name: &str) -> bool {
        scenario_actions::duplicate(self, name)
    }

    fn rename_scenario(&self, name: &str) -> bool {
        scenario_actions::rename(self, name)
    }

    fn delete_scenario(&self) -> bool {
        scenario_actions::delete(self)
    }
}

impl ScenarioPage for JunctionVm {
    fn binding(&self) -> Option<&CityBinding> {
        self.binding.as_ref().filter(|_| self.offers_scenarios)
    }

    fn flush(&self) {
        JunctionVm::flush(self);
    }

    fn show_design(&self, id: Option<u32>, keep_choices: bool) -> bool {
        let Some(mut fresh) = self.open_design(id) else { return false };
        if keep_choices {
            // as a sync reload does: the part chosen stays chosen where the junction shown has it
            fresh.select(self.core.read(|j| j.selected));
        } else {
            // another design the person asked for starts as a page opens: nothing chosen, the junction's own level
            self.plan.set(false);
        }
        *self.kept_design.borrow_mut() = fresh.snapshot();
        self.core.edit(|j| *j = fresh);
        true
    }

    fn tell(&self, said: &Said) {
        self.ports.announcer.say(&text::scenario_text(said, &self.i18n, self.units_now()));
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
    fn a_sync_reloads_the_scenario_shown_and_falls_back_to_the_baseline_when_it_is_gone() {
        use crate::city::scenario::Action;
        use crate::shared::scenarios::ScenarioHost;
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let imported = corner(&r);
        assert!(r.vm.create_scenario("Raised"));
        let id = r.vm.scenario_state().active_now().unwrap();
        r.vm.plan.set(true);
        // another tab edits the scenario
        let mut j = binding.junction_editor(Some(id)).unwrap();
        assert!(j.set_corner(j.current().arms[0].uid, imported + 1_000));
        assert_eq!(binding.keep_junction_in(j.snapshot(), Some(id)), Ok(()));
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(corner(&r), imported + 1_000, "the scenario shown, as the other tab left it");
        assert!(!r.vm.view_now().can_undo && !r.vm.undo(), "its history starts at the design reloaded, not at the import");
        assert_eq!(corner(&r), imported + 1_000);
        // (a refused undo still asks the keeper to write: that write is done before the other tab acts)
        r.vm.flush();
        // and then deletes it
        binding.act(Action::Delete(id)).unwrap();
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.vm.scenario_state().active_now(), None);
        assert_eq!(r.said.take().last().map(String::as_str), Some("That scenario is no longer here: it was deleted elsewhere. Now showing the baseline."));
        assert_eq!(corner(&r), imported);
        assert!(!r.vm.view_now().can_undo);
        assert!(r.vm.plan.get_untracked(), "a fallback the person did not ask for keeps what they were looking at, as it keeps the part chosen");
        hub::reset();
    }

    #[test]
    fn an_edit_of_a_scenario_deleted_elsewhere_shows_the_baseline_says_so_and_leaves_the_storage_warning_for_real_failures() {
        use crate::city::scenario::Action;
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        let imported = corner(&r);
        assert!(r.vm.create_scenario("Raised"));
        let id = r.vm.scenario_state().active_now().unwrap();
        binding.act(Action::Delete(id)).unwrap();
        let uid = r.vm.read(|j| j.current().arms[0].uid);
        assert!(r.vm.apply(|j| j.set_corner(uid, imported + 1_000)));
        r.said.take();
        r.time.advance(1_000);
        assert_eq!(
            r.said.take(),
            ["That scenario is no longer here: it was deleted elsewhere, so your last change to it was not kept. Now showing the baseline."]
        );
        assert_eq!(r.vm.scenario_state().active_now(), None);
        assert_eq!((corner(&r), kept_corner(&binding, None)), (imported, imported));
        r.storage.refuse(crate::city::edits::EDITS_KEY);
        assert!(r.vm.apply(|j| j.set_corner(uid, imported + 500)));
        r.time.advance(1_000);
        assert_eq!(r.said.take().last().map(String::as_str), Some("This browser is not keeping your changes, so other pages will not see them."));
    }

    #[test]
    fn an_action_whose_flush_finds_the_scenario_gone_stops_there_with_the_baseline_shown() {
        use crate::city::scenario::Action;
        use crate::shared::scenarios::ScenarioHost;
        let actions: [(&str, fn(&JunctionVm) -> bool); 5] = [
            ("switch", |vm| vm.switch_scenario(None)),
            ("create", |vm| vm.create_scenario("Protected")),
            ("duplicate", |vm| vm.duplicate_scenario("Copy")),
            ("rename", |vm| vm.rename_scenario("Renamed")),
            ("delete", |vm| vm.delete_scenario()),
        ];
        for (what, action) in actions {
            let (r, binding) = city_rig();
            let imported = corner(&r);
            assert!(r.vm.create_scenario("Raised"));
            let id = r.vm.scenario_state().active_now().unwrap();
            let uid = r.vm.read(|j| j.current().arms[0].uid);
            assert!(r.vm.apply(|j| j.set_corner(uid, imported + 1_000)));
            binding.act(Action::Delete(id)).unwrap();
            r.said.take();
            assert!(!action(&r.vm), "{what}: it stops once the flush found the scenario gone");
            assert_eq!(
                r.said.take(),
                ["That scenario is no longer here: it was deleted elsewhere, so your last change to it was not kept. Now showing the baseline."],
                "{what}"
            );
            assert_eq!(r.vm.scenario_state().refusal(), None, "{what}");
            assert_eq!(r.vm.scenario_state().active_now(), None, "{what}");
            assert!(binding.scenarios().is_empty(), "{what}");
            assert_eq!((corner(&r), kept_corner(&binding, None)), (imported, imported), "{what}");
        }
    }

    #[test]
    fn a_scenario_deleted_elsewhere_while_only_a_part_was_chosen_in_it_says_no_change_was_lost() {
        use crate::city::scenario::Action;
        use crate::junction::model::Target;
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        assert!(r.vm.create_scenario("Raised"));
        let id = r.vm.scenario_state().active_now().unwrap();
        let uid = r.vm.read(|j| j.current().arms[0].uid);
        r.vm.select(Target::Arm(uid));
        r.vm.select_relative(1);
        assert!(!r.vm.redo());
        binding.act(Action::Delete(id)).unwrap();
        r.said.take();
        r.time.advance(1_000);
        assert_eq!(r.said.take(), ["That scenario is no longer here: it was deleted elsewhere. Now showing the baseline."]);
        assert_eq!(r.vm.scenario_state().active_now(), None);
    }

    #[test]
    fn the_baseline_shown_for_a_scenario_deleted_elsewhere_keeps_the_part_chosen() {
        use crate::city::scenario::Action;
        use crate::junction::model::Target;
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        assert!(r.vm.create_scenario("Raised"));
        let id = r.vm.scenario_state().active_now().unwrap();
        let uid = r.vm.read(|j| j.current().arms[1].uid);
        r.vm.select(Target::Corner(uid));
        let corner_mm = r.vm.read(|j| j.current().arms[1].corner_mm);
        assert!(r.vm.apply(|j| j.set_corner(uid, corner_mm + 1_000)));
        binding.act(Action::Delete(id)).unwrap();
        r.time.advance(1_000);
        assert_eq!(r.vm.scenario_state().active_now(), None);
        assert_eq!(r.vm.read(|j| j.selected), Target::Corner(uid), "the corner is still chosen in the baseline, which has it");
    }

    #[test]
    fn shown_read_only_the_scenarios_can_be_looked_at_and_no_action_on_them_is_taken() {
        use crate::shared::ports::{Ports, Storage, test_ports};
        use crate::shared::scenarios::ScenarioHost;
        use crate::shared::scoped::ScopedStorage;
        let (ports, said, root) = test_ports();
        let writable: Rc<dyn Storage> = Rc::new(ScopedStorage::new(root.clone(), "p1"));
        let node = CityStore::sample(writable.clone()).open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let id = CityStore::sample(writable).change_junction_scenarios(node, |s, base| s.create("Raised", base.clone())).unwrap();
        let storage: Rc<dyn Storage> = Rc::new(ScopedStorage::new(root.clone(), "p1").only_to_read());
        let store = CityStore::sample(storage.clone());
        let junction = store.open().junction_editor(node, 0).unwrap();
        let binding = CityBinding { store, place: Place::Junction(node) };
        let vm = JunctionVm::new(Ports { storage, ..ports }, crate::i18n_for(crate::shared::i18n::Locale::En), junction, Some(binding.clone()));
        assert!(vm.switch_scenario(Some(id)), "looking at a scenario is choosing, which goes on");
        said.take();
        let actions: [(&str, &dyn Fn() -> bool); 3] =
            [("create", &|| vm.create_scenario("Protected")), ("rename", &|| vm.rename_scenario("Renamed")), ("delete", &|| vm.delete_scenario())];
        for (what, action) in actions {
            assert!(!action(), "{what}");
            assert_eq!(said.take(), ["This design is open read-only: nothing can be changed."], "{what}");
        }
        assert_eq!(binding.scenarios(), [(id, "Raised".to_string())]);
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

    /// The first arm's corner radius as the editor holds it, and as the city keeps design `scenario`.
    fn corner(r: &Rig) -> i32 {
        r.vm.read(|j| j.current().arms[0].corner_mm)
    }

    fn kept_corner(binding: &CityBinding, scenario: Option<u32>) -> i32 {
        binding.junction_editor(scenario).unwrap().current().arms[0].corner_mm
    }

    fn last_said(r: &Rig) -> String {
        r.said.take().pop().unwrap_or_default()
    }

    #[test]
    fn two_named_scenarios_are_made_and_switching_between_them_and_the_baseline_loses_nothing() {
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        assert!(r.vm.scenarios_offered());
        assert_eq!(r.vm.scenario_state().active_now(), None, "the page opens on the baseline");
        let baseline = corner(&r);
        assert!(r.vm.create_scenario("Wider bike lane"));
        assert_eq!(last_said(&r), "Scenario “Wider bike lane” made from the baseline. Now editing it.");
        let wider = r.vm.scenario_state().active_now().unwrap();
        let uid = r.vm.read(|j| j.current().arms[0].uid);
        assert!(r.vm.apply(|j| j.set_corner(uid, baseline + 1_000)));
        assert!(r.vm.create_scenario("Parking kept"));
        let parking = r.vm.scenario_state().active_now().unwrap();
        assert_eq!(corner(&r), baseline, "a new scenario starts as the baseline");
        assert!(r.vm.switch_scenario(Some(wider)));
        assert_eq!(last_said(&r), "Now showing scenario “Wider bike lane”.");
        assert_eq!(corner(&r), baseline + 1_000);
        assert!(r.vm.switch_scenario(None));
        assert_eq!(corner(&r), baseline);
        assert!(r.vm.switch_scenario(Some(parking)));
        r.vm.flush();
        assert_eq!(
            (kept_corner(&binding, None), kept_corner(&binding, Some(wider)), kept_corner(&binding, Some(parking))),
            (baseline, baseline + 1_000, baseline)
        );
        assert_eq!(binding.store.open().view(0).edited, 0, "the map reads the baseline");
    }

    #[test]
    fn right_after_a_switch_undo_has_nothing_to_undo_and_start_over_touches_only_the_design_shown() {
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        let uid = r.vm.read(|j| j.current().arms[0].uid);
        let imported = corner(&r);
        assert!(r.vm.apply(|j| j.set_corner(uid, imported + 500)), "an edit of the baseline");
        assert!(r.vm.create_scenario("Raised"));
        let raised = r.vm.scenario_state().active_now().unwrap();
        assert!(r.vm.apply(|j| j.set_corner(uid, imported + 1_000)));
        assert!(r.vm.create_scenario("Protected"));
        let protected = r.vm.scenario_state().active_now().unwrap();
        assert!(r.vm.apply(|j| j.set_corner(uid, imported + 1_500)));
        let (ports, _, _, _) = test_ports_with_time();
        let again = JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::En), binding.junction_editor(None).unwrap(), Some(binding.clone()));
        for vm in [&r.vm, &again] {
            assert!(vm.switch_scenario(Some(raised)));
            assert!(!vm.view_now().can_undo, "nothing was done in it since it was shown");
            assert!(!vm.undo());
            assert_eq!(vm.read(|j| j.current().arms[0].corner_mm), imported + 1_000);
            vm.flush();
        }
        let kept = |s| kept_corner(&binding, s);
        assert_eq!((kept(None), kept(Some(raised)), kept(Some(protected))), (imported + 500, imported + 1_000, imported + 1_500));
        assert!(r.vm.reset());
        r.vm.flush();
        assert_eq!(
            (kept(None), kept(Some(raised)), kept(Some(protected))),
            (imported + 500, imported, imported + 1_500),
            "start over: the scenario shown only"
        );
        assert!(r.vm.undo(), "and it can be undone");
        r.vm.flush();
        assert_eq!(kept(Some(raised)), imported + 1_000);
        assert!(!r.vm.undo());
    }

    #[test]
    fn start_over_in_a_scenario_made_from_a_junction_never_edited_can_be_undone() {
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        let imported = corner(&r);
        assert!(r.vm.create_scenario("Raised"));
        let id = r.vm.scenario_state().active_now().unwrap();
        let uid = r.vm.read(|j| j.current().arms[0].uid);
        assert!(r.vm.apply(|j| j.set_corner(uid, imported + 1_000)));
        assert!(r.vm.reset());
        assert_eq!(corner(&r), imported);
        assert!(r.vm.undo(), "the edit comes back");
        r.vm.flush();
        assert_eq!(kept_corner(&binding, Some(id)), imported + 1_000);
    }

    #[test]
    fn an_edit_in_a_scenario_changes_only_it_and_can_be_undone() {
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        let baseline = corner(&r);
        assert!(r.vm.create_scenario("Raised"));
        let id = r.vm.scenario_state().active_now().unwrap();
        let uid = r.vm.read(|j| j.current().arms[0].uid);
        assert!(r.vm.apply(|j| j.set_corner(uid, baseline + 1_000)));
        r.time.advance(1_000);
        assert_eq!((kept_corner(&binding, Some(id)), kept_corner(&binding, None)), (baseline + 1_000, baseline));
        assert!(r.vm.undo());
        r.vm.flush();
        assert_eq!(kept_corner(&binding, Some(id)), baseline);
        assert!(r.vm.redo());
        r.vm.flush();
        assert_eq!((kept_corner(&binding, Some(id)), kept_corner(&binding, None)), (baseline + 1_000, baseline));
    }

    #[test]
    fn a_junction_scenario_is_duplicated_renamed_and_deleted_and_a_refusal_is_said_in_the_language_of_the_page() {
        use crate::shared::scenarios::ScenarioHost;
        let (r, binding) = city_rig();
        assert!(r.vm.create_scenario("Raised"));
        assert!(r.vm.duplicate_scenario("Raised, protected"));
        assert!(r.vm.rename_scenario("Protected"));
        assert_eq!(last_said(&r), "Scenario “Raised, protected” renamed “Protected”.");
        r.vm.i18n().set(crate::shared::i18n::Locale::FrCa);
        assert!(!r.vm.rename_scenario("raised"));
        assert_eq!(last_said(&r), "Un autre scénario s’appelle déjà «\u{a0}raised\u{a0}».");
        assert!(!r.vm.delete_scenario());
        assert_eq!(last_said(&r), "Supprimer le scénario «\u{a0}Protected\u{a0}»? Appuyez sur Confirmer la suppression pour le supprimer.");
        assert!(r.vm.delete_scenario());
        assert_eq!(last_said(&r), "Scénario «\u{a0}Protected\u{a0}» supprimé. La référence est maintenant affichée.");
        assert_eq!(binding.scenarios().into_iter().map(|(_, n)| n).collect::<Vec<_>>(), ["Raised"]);
        assert!(!rig(0).vm.scenarios_offered(), "a sample junction outside the city keeps none");
    }

    #[test]
    fn a_sync_that_leaves_a_crossing_on_call_unwalked_says_so_after_the_update() {
        use crate::junction::model::SIGNAL;
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let Place::Junction(node) = binding.place else { unreachable!() };
        // Another device makes it a signal, puts a crossing on call and walks it in no phase.
        let mut j = binding.store.open().junction_editor(node, 0).unwrap();
        let _ = j.set_control(SIGNAL);
        let arm = j.current().arms.iter().find(|a| a.crossing.is_some()).unwrap().uid;
        assert!(j.set_on_call(arm, true));
        let plan = crate::junction::model::signal::plan(j.current(), 0).unwrap();
        for i in (0..plan.phases.len()).filter(|&i| plan.phases[i].walks.contains(&arm)) {
            assert!(j.set_phase_walk(i, arm, false));
        }
        assert!(binding.store.write_junction(node, j.snapshot()));
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        let t = r.said.take();
        assert_eq!(t.len(), 1);
        assert!(t[0].starts_with("Updated from another device. The crossing of "), "{t:?}");
        assert!(t[0].ends_with("is on call, but no phase walks it, so its calls are not served. Walk it in a phase, or put it back on recall."), "{t:?}");
    }

    /// The junction of `city_rig`'s record as another device writes it: a signal, its first crossing put on call.
    fn on_call_record(binding: &CityBinding) -> (u32, State) {
        use crate::junction::model::SIGNAL;
        let Place::Junction(node) = binding.place else { unreachable!() };
        let mut j = binding.store.open().junction_editor(node, 0).unwrap();
        let _ = j.set_control(SIGNAL);
        let arm = j.current().arms.iter().find(|a| a.crossing.is_some()).unwrap().uid;
        assert!(j.set_on_call(arm, true));
        (node, j.snapshot())
    }

    #[test]
    fn a_sync_that_opens_the_junction_off_a_signal_says_the_calls_it_took_off_once() {
        use crate::junction::model::PRIORITY;
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        // The record holds a crossing on call on a junction that is no signal (an older build's, say).
        let (node, mut state) = on_call_record(&binding);
        state.control = PRIORITY;
        assert!(binding.store.write_junction(node, state.clone()));
        r.said.take();
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.said.take(), ["Updated from another device. 1 crossing was taken off call as the junction opened: it is not run by a signal."]);
        // Another change comes, the record still holding the call: the sync took nothing off this time.
        state.arms[1].corner_mm += 1_000;
        assert!(binding.store.write_junction(node, state));
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        assert_eq!(r.said.take(), ["Updated from another device."]);
    }

    #[test]
    fn a_sync_that_suspends_the_signal_says_its_crossings_stay_on_call() {
        use crate::shared::catalogue::StreetClass;
        use crate::sync::hub;
        use crate::sync::model::{Change, Origin};
        hub::reset();
        let (r, binding) = city_rig();
        r.vm.watch_sync();
        let (node, state) = on_call_record(&binding);
        assert!(binding.store.write_junction(node, state));
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        r.said.take();
        // Every street but the one on call is made a walkway elsewhere: one street of traffic, the signal suspended.
        let opened = binding.store.open().junction_editor(node, 0).unwrap();
        let on_call = opened.current().arms.iter().find(|a| a.calls.is_some()).unwrap().uid;
        for a in opened.current().arms.iter().filter(|a| a.uid != on_call) {
            let mut walkway = a.section.clone().unwrap();
            walkway.class = StreetClass::Pedestrian;
            assert!(binding.store.write_street(a.edge, walkway));
        }
        hub::publish(&[Change { id: binding.record().unwrap(), origin: Origin::Remote }]);
        let t = r.said.take();
        assert!(r.vm.view_now().calls_kept == 1, "suspended, the call kept");
        assert_eq!(t, ["Updated from another device. 1 crossing stays on call for when the signal runs again."]);
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
    fn a_crossing_put_on_call_is_announced_with_how_its_calls_are_served_and_a_refusal_says_why_in_both_languages() {
        let r = rig(0);
        let (n, e) = r.vm.read(|j| {
            let at = |b: i32| j.current().arms.iter().find(|a| a.bearing == b).unwrap().uid;
            (at(0), at(90))
        });
        assert!(r.vm.set_on_call(e, true));
        let t = r.said.take();
        assert_eq!(t.len(), 1);
        assert!(t[0].starts_with("Crossing of Sample Street 1 (east) on call."), "{t:?}");
        assert!(t[0].contains("Crossing of Sample Street 1 (east): 12 calls in the hour; served "), "{t:?}");
        assert!(r.vm.set_call_rate(e, 1));
        assert!(r.said.take()[0].contains(": 1 call in the hour;"));
        assert!(!r.vm.set_call_rate(e, 61));
        assert_eq!(r.said.take(), vec!["That many calls an hour is outside the placeholder bound shown."]);
        // A plan with no phase that walks the north street's crossing: on call is refused, and says why.
        let only_east = crate::junction::model::signal::SignalPlan {
            phases: vec![crate::junction::model::signal::StoredPhase {
                moves: vec![crate::junction::model::signal::Move { from: e, to: n }],
                walks: vec![],
                rides: vec![],
                arm: None,
            }],
        };
        r.vm.edit(|j| j.set_plan_for_test(only_east.clone()));
        assert!(!r.vm.set_on_call(n, true));
        assert_eq!(r.said.take(), vec!["No phase of the plan walks this crossing, so it cannot be on call: walk it in a phase first."]);
        // The east crossing, left on call with no walk, is a finding of the plan.
        let findings = r.vm.view_now().signal.as_ref().unwrap().calls.findings.clone();
        assert_eq!(findings.iter().map(|f| f.key).collect::<Vec<_>>(), vec!["jn-calls-finding-unwalked"]);

        let (ports, said, _, _) = test_ports_with_time();
        let fr = JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::FrCa), Junction::new(0), None);
        assert!(fr.set_on_call(e, true));
        let t = said.take();
        assert!(t[0].starts_with("Passage de Sample Street 1 (est) sur appel."), "{t:?}");
        assert!(t[0].contains("12\u{a0}appels dans l’heure") || t[0].contains("12 appels dans l’heure"), "{t:?}");
        fr.edit(|j| j.set_plan_for_test(only_east));
        assert!(!fr.set_on_call(n, true));
        assert_eq!(said.take(), vec!["Aucune phase du plan n’ouvre ce passage, il ne peut donc pas être sur appel\u{a0}: ouvrez-le d’abord dans une phase."]);
    }

    #[test]
    fn a_control_change_that_takes_crossings_off_call_counts_them_in_both_languages() {
        use crate::junction::model::PRIORITY;
        for (locale, one, two) in [
            (crate::shared::i18n::Locale::En, "; 1 crossing no longer on call", "; 2 crossings no longer on call"),
            (crate::shared::i18n::Locale::FrCa, "; 1 passage qui n’est plus sur appel", "; 2 passages qui ne sont plus sur appel"),
        ] {
            for (count, words) in [(1, one), (2, two)] {
                let (ports, said, _, _) = test_ports_with_time();
                let vm = JunctionVm::new(ports, crate::i18n_for(locale), Junction::new(0), None);
                let (e, w) = vm.read(|j| {
                    let at = |b: i32| j.current().arms.iter().find(|a| a.bearing == b).unwrap().uid;
                    (at(90), at(270))
                });
                for &u in &[e, w][..count] {
                    assert!(vm.set_on_call(u, true));
                }
                said.take();
                assert!(vm.apply(|j| j.set_control(PRIORITY)));
                let t = said.take();
                assert!(t[0].contains(words), "{t:?}");
            }
        }
    }

    #[test]
    fn a_change_that_suspends_the_signal_says_its_crossings_stay_on_call() {
        let (suspended, today) = crate::junction::view::tests::suspended_with_a_call();
        // Today the junction is suspended; now its streets carry traffic again. Undo goes back to today.
        let mut now = Junction::new(0).current().clone();
        for (a, b) in now.arms.iter_mut().zip(&today.arms) {
            a.edge = b.edge;
            a.calls = b.calls;
        }
        let j = Junction::from_city(crate::street::model::named_said("Test"), &today, &now, 0).unwrap();
        drop(suspended);
        let (ports, said, _, _) = test_ports_with_time();
        let vm = JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::En), j, None);
        assert_eq!(vm.view_now().calls_kept, 0);
        assert!(vm.undo());
        let t = said.take();
        assert!(t[0].contains("1 crossing stays on call for when the signal runs again."), "{t:?}");
    }

    #[test]
    fn a_crossing_whose_calls_are_not_served_is_said_to_have_none_served_not_a_wait_of_0_s() {
        use crate::junction::model::{TimingField, junction_of};
        for (locale, words, not) in [
            (crate::shared::i18n::Locale::En, "not served 12; no call was served.", "longest wait"),
            (crate::shared::i18n::Locale::FrCa, "non servis 12; aucun appel n’a été servi.", "attente la plus longue"),
        ] {
            // Five lanes of 3.5 m and 5 s greens: no phase can fit the walk and clearance.
            let mut j = junction_of(&[
                ("sidewalk", 2000),
                ("travel", 3500),
                ("travel", 3500),
                ("travel", 3500),
                ("travel", 3500),
                ("travel", 3500),
                ("sidewalk", 2000),
            ]);
            for u in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
                assert!(j.set_timing(u, TimingField::Green, 5));
            }
            let (ports, said, _, _) = test_ports_with_time();
            let vm = JunctionVm::new(ports, crate::i18n_for(locale), j, None);
            let e = vm.read(|j| j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid);
            assert!(vm.set_on_call(e, true));
            let t = said.take();
            assert!(t[0].contains(words) && !t[0].contains(not), "{t:?}");
        }
    }

    #[test]
    fn a_finding_already_there_is_not_said_again_when_an_edit_renames_its_street() {
        use crate::junction::model::{TimingField, junction_of};
        // Five lanes of 3.5 m and 5 s greens: the north crossing on call cannot be served, a finding from the start.
        let mut j =
            junction_of(&[("sidewalk", 2000), ("travel", 3500), ("travel", 3500), ("travel", 3500), ("travel", 3500), ("travel", 3500), ("sidewalk", 2000)]);
        for u in j.current().arms.iter().map(|a| a.uid).collect::<Vec<_>>() {
            assert!(j.set_timing(u, TimingField::Green, 5));
        }
        let (ports, said, _, _) = test_ports_with_time();
        let vm = JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::En), j, None);
        let n = vm.read(|j| j.current().arms.iter().find(|a| a.bearing == 0).unwrap().uid);
        assert!(vm.set_on_call(n, true));
        assert!(said.take()[0].contains("cannot be served safely"), "said when it arises");
        // Turned into the north-east: the street's name changes, the finding does not.
        assert!(vm.apply(|j| j.set_bearing(n, 30)));
        let t = said.take();
        assert!(t[0].contains("(north-east)") || t[0].contains("bearing"), "{t:?}");
        assert!(!t[0].contains("cannot be served"), "the same finding, not said again: {t:?}");
    }

    #[test]
    fn no_calls_an_hour_are_said_as_none_in_french_never_as_one() {
        let (ports, said, _, _) = test_ports_with_time();
        let fr = JunctionVm::new(ports, crate::i18n_for(crate::shared::i18n::Locale::FrCa), Junction::new(0), None);
        let e = fr.read(|j| j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid);
        assert!(fr.set_on_call(e, true));
        said.take();
        assert!(fr.set_call_rate(e, 0));
        let t = said.take();
        // French counts 0 with the singular: "0 appel", which [one] must say with the number, not "1 appel".
        assert!(t[0].contains("0\u{a0}appel dans l’heure") || t[0].contains(" 0 appel dans l’heure"), "{t:?}");
        assert!(!t[0].contains("1 appel"), "{t:?}");
    }

    #[test]
    fn a_rate_typed_again_unchanged_says_nothing_rather_than_an_earlier_edit() {
        let r = rig(0);
        let e = r.vm.read(|j| j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid);
        assert!(r.vm.set_on_call(e, true));
        r.said.take();
        assert!(r.vm.set_call_rate(e, 12), "the rate it has: not refused");
        assert!(r.said.take().is_empty(), "no revision was added, so nothing is said");
    }

    #[test]
    fn a_signal_edit_that_leaves_a_crossing_on_call_unwalked_says_so_in_both_languages() {
        for (locale, words) in [
            (crate::shared::i18n::Locale::En, "The crossing of Sample Street 1 (east) is on call, but no phase walks it"),
            (crate::shared::i18n::Locale::FrCa, "Le passage de Sample Street 1 (est) est sur appel, mais aucune phase ne l’ouvre"),
        ] {
            let (ports, said, _, _) = test_ports_with_time();
            let vm = JunctionVm::new(ports, crate::i18n_for(locale), Junction::new(0), None);
            let e = vm.read(|j| j.current().arms.iter().find(|a| a.bearing == 90).unwrap().uid);
            assert!(vm.set_on_call(e, true));
            let walking: Vec<usize> = vm.read(|j| {
                let plan = crate::junction::model::signal::plan(j.current(), j.region).unwrap();
                (0..plan.phases.len()).filter(|&i| plan.phases[i].walks.contains(&e)).collect()
            });
            let (last, rest) = walking.split_last().unwrap();
            for &i in rest {
                assert!(vm.toggle_phase_walk(i, e));
            }
            said.take();
            assert!(vm.toggle_phase_walk(*last, e));
            let t = said.take();
            assert!(t[0].contains(words), "{t:?}");
            // Walked again: no finding, so nothing of it is said.
            assert!(vm.toggle_phase_walk(*last, e) && !said.take()[0].contains(words));
        }
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

/// A junction shown read-only (`Storage::read_only`, a page opened with `project_id::VIEW`): every edit is refused, by
/// the page or by a command called as it is, and nothing reaches the city or the sync; choosing and looking still work.
#[cfg(test)]
mod read_only_tests {
    use super::*;
    use crate::city::store::CityStore;
    use crate::junction::model::signal::Pattern;
    use crate::junction::model::{SIGNAL, Target};
    use crate::shared::i18n::Locale;
    use crate::shared::ports::{ManualScheduler, MemoryStorage, RecordingAnnouncer, Storage, test_ports_with_time};
    use crate::shared::scoped::ScopedStorage;

    struct Rig {
        vm: Rc<JunctionVm>,
        said: Rc<RecordingAnnouncer>,
        time: Rc<ManualScheduler>,
        storage: Rc<MemoryStorage>,
    }

    fn stored(storage: &MemoryStorage) -> Vec<(String, Option<String>)> {
        let mut keys = storage.keys();
        keys.sort();
        keys.into_iter().map(|k| (k.clone(), storage.recall(&k))).collect()
    }

    /// A junction of project `p1` signalled and with a corner changed before (so there is something to undo and start
    /// over from), opened on a page that views the project read-only, in `locale`.
    fn rig_in(locale: Locale) -> Rig {
        let (ports, said, storage, time) = test_ports_with_time();
        let own: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), "p1"));
        let store = CityStore::sample(own);
        let node = store.open().view(0).nodes.iter().find(|n| n.junction).unwrap().uid;
        let mut j = store.open().junction_editor(node, 0).unwrap();
        let arm = j.current().arms[0].uid;
        j.set_control(SIGNAL);
        assert!(j.set_corner(arm, 7_000));
        assert!(store.write_junction(node, j.snapshot()));
        let view: Rc<dyn Storage> = Rc::new(ScopedStorage::new(storage.clone(), "p1").only_to_read());
        let store = CityStore::sample(view.clone());
        let junction = store.open().junction_editor(node, 0).unwrap();
        let binding = CityBinding { store, place: Place::Junction(node) };
        let vm = JunctionVm::new(Ports { storage: view, ..ports }, crate::i18n_for(locale), junction, Some(binding));
        Rig { vm, said, time, storage }
    }

    fn rig() -> Rig {
        rig_in(Locale::En)
    }

    const REFUSED: &str = "This design is open read-only: nothing can be changed.";

    /// The region (the shell gives the one kept, on load and when chosen) changes the junction's drive side: shown
    /// read-only it is not taken, and nothing is said, so a page opened with the other side kept says nothing on load.
    #[test]
    fn a_region_is_not_taken_and_nothing_is_said() {
        use crate::shell::Target;
        let r = rig();
        assert!(r.vm.region_is_design(), "the drive side is the design's");
        let (junction, region) = (r.vm.read(|j| j.snapshot()), r.vm.region_id());
        let left = crate::shared::catalogue::REGIONS.iter().position(|g| g.id == "united-kingdom").unwrap();
        assert_ne!(region, "united-kingdom");
        assert!(!r.vm.apply_region(left));
        assert_eq!((r.vm.read(|j| j.snapshot()), r.vm.region_id()), (junction, region));
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn the_junction_says_it_is_read_only_and_one_opened_to_edit_is_not() {
        assert!(rig().vm.read_only());
        let (ports, ..) = test_ports_with_time();
        assert!(!JunctionVm::new(ports, crate::i18n_for(Locale::En), Junction::new(0), None).read_only());
    }

    #[test]
    fn every_edit_command_is_refused_and_said_and_leaves_the_junction_and_the_storage_as_they_were() {
        let r = rig();
        let before = stored(&r.storage);
        let junction = r.vm.read(|j| j.snapshot());
        let arm = r.vm.view_now().arms[0].uid;
        assert!(r.vm.view_now().can_undo && r.vm.view_now().changed, "the earlier edit can be undone and started over from");
        assert!(r.vm.view_now().signal.is_some(), "a signal to plan");
        let refused: Vec<(&str, Box<dyn Fn(&JunctionVm) -> bool>)> = vec![
            ("apply", Box::new(move |vm| vm.apply(|j| j.set_corner(arm, 9_000)))),
            ("undo", Box::new(|vm| vm.undo())),
            ("redo", Box::new(|vm| vm.redo())),
            ("reset", Box::new(|vm| vm.reset())),
            ("turbo", Box::new(|vm| vm.choose_turbo(Some(0)))),
            ("pattern", Box::new(|vm| vm.choose_pattern(0, Pattern::Scramble))),
            ("walk", Box::new(move |vm| vm.toggle_phase_walk(0, arm))),
            ("ride", Box::new(move |vm| vm.toggle_phase_ride(0, arm))),
        ];
        for (what, command) in &refused {
            assert!(!command(&r.vm), "{what} is refused");
            assert_eq!(r.said.take(), [REFUSED], "{what} says why");
            assert_eq!(r.vm.read(|j| j.snapshot()), junction, "{what} changes nothing");
        }
        r.vm.load_sample(1);
        assert_eq!(r.said.take(), [REFUSED]);
        r.vm.flush();
        r.time.advance(1_000);
        assert_eq!(r.vm.read(|j| j.snapshot()), junction);
        assert_eq!(stored(&r.storage), before, "nothing written, nothing marked for the sync");
    }

    #[test]
    fn a_calls_edit_is_refused_and_said_once_and_changes_nothing() {
        let r = rig();
        let before = stored(&r.storage);
        let junction = r.vm.read(|j| j.snapshot());
        let arm = r.vm.view_now().arms.iter().find(|a| a.crossing.is_some()).unwrap().uid;
        assert!(!r.vm.set_on_call(arm, true), "on call is refused");
        assert_eq!(r.said.take(), [REFUSED], "said once, the read-only sentence and no other reason");
        assert!(!r.vm.set_call_rate(arm, 20), "a rate is refused");
        assert_eq!(r.said.take(), [REFUSED]);
        assert_eq!(r.vm.read(|j| j.snapshot()), junction, "nothing changed");
        r.vm.flush();
        r.time.advance(1_000);
        assert_eq!(stored(&r.storage), before, "nothing written, nothing marked for the sync");
    }

    #[test]
    fn a_change_made_by_any_other_way_is_put_back_and_nothing_is_kept() {
        let r = rig();
        let before = stored(&r.storage);
        let junction = r.vm.read(|j| j.snapshot());
        let arm = r.vm.view_now().arms[0].uid;
        assert!(!r.vm.edit(|j| j.set_corner(arm, 9_000)), "the model took it, and the page refuses it: no edit to announce");
        assert_eq!(r.vm.read(|j| j.snapshot()), junction, "and it was put back");
        assert_eq!(r.said.take(), ["This design is open read-only: nothing can be changed."]);
        r.vm.flush();
        r.time.advance(1_000);
        assert_eq!(stored(&r.storage), before);
        assert!(r.said.take().is_empty());
    }

    #[test]
    fn choosing_and_looking_still_work() {
        let r = rig();
        let before = stored(&r.storage);
        let arm = r.vm.view_now().arms[0].uid;
        r.vm.select(Target::Arm(arm));
        assert_eq!(r.vm.view_now().selected.uid, arm);
        assert!(!r.said.take().is_empty(), "the choice is said");
        let chosen = |vm: &JunctionVm| (vm.view_now().selected.kind, vm.view_now().selected.uid);
        let first = chosen(&r.vm);
        r.vm.select_relative(1);
        assert_ne!(chosen(&r.vm), first, "the next thing round the junction");
        r.vm.show_plan();
        assert!(r.vm.plan_shown(), "the signal plan can be looked at");
        r.vm.flush();
        r.time.advance(1_000);
        assert_eq!(stored(&r.storage), before);
    }

    #[test]
    fn the_refusal_is_said_in_the_language_of_the_page() {
        let r = rig_in(Locale::FrCa);
        assert!(!r.vm.undo());
        assert_eq!(r.said.take(), ["Cet aménagement est ouvert en lecture seule\u{a0}: rien ne peut y être modifié."]);
    }

    #[test]
    fn a_link_from_the_junction_keeps_the_project_read_only() {
        assert_eq!(rig().vm.link("street.html?street=7"), "street.html?street=7&project=p1&view=1");
    }
}
