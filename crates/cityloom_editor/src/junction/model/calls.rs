//! Pedestrian calls (#60), the first responsive behaviour of a signal: a crossing on call is walked only when someone
//! has pushed the button, and the plan's fixed-time cycle serves the calls. A seeded generator spreads each on-call
//! crossing's calls over one hour, and `summary` runs the cycle over them: when each call is served, how long it
//! waited, and how far a phase's green was lengthened to fit the walk and its clearance. Deterministic: the same
//! junction, plan and rates always give the same calls and the same summary. CityLoom models the behaviour to compare
//! strategies; it never controls a signal. No DOM, no signals, no I/O, no wording.
//!
//! A crossing on call detects the people waiting by a push button (as in #60) or by passive kerbside detection (radar,
//! infrared or video, CLM-17): a pedestrian waiting at the kerb places the call without pressing anything, and one who
//! leaves before the walk cancels it, so no walk is given to an empty crossing. A push-button call stays placed after
//! the person leaves.
//!
//! Every figure here is a placeholder (synthetic, like the catalogue's rules and rates): the walk, the clearance speed,
//! the longest extension, the bound on the call rate and the waiting limit.

use crate::junction::model::control;
use crate::junction::model::signal::{Phase, Plan};
use crate::junction::model::{Arm, Refusal, SIGNAL, State, in_force};

/// The least time the walk is shown, in seconds. Placeholder.
pub const WALK_S: u32 = 7;
/// How fast a person is taken to walk while the don't-walk flashes (the clearance), in millimetres a second: 1.0 m/s.
/// Placeholder.
pub const CLEARANCE_MM_PER_S: u32 = 1_000;
/// The most a phase's green is lengthened to serve a call, in seconds. Placeholder.
pub const MAX_EXTENSION_S: u32 = 15;
/// The most calls an hour a crossing may be given. Placeholder.
pub const MAX_CALLS_PER_HOUR: u32 = 60;
/// The calls an hour a crossing is given when it is put on call. Placeholder.
pub const DEFAULT_CALLS_PER_HOUR: u32 = 12;
/// The span the calls are spread over.
pub use crate::junction::model::control::HOUR_S;
/// How long a pedestrian waits at a crossing with passive detection before leaving, if the walk has not started, in
/// seconds; one figure for the whole junction. Placeholder.
pub const WAIT_LIMIT_S: u32 = 30;

/// How an on-call crossing knows someone is waiting (CLM-17). Saved on the arm only once passive
/// (`Arm.detection`), so a junction saved before reads as push buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Detection {
    /// The pedestrian pushes a button; the call stays placed after they leave (#60).
    #[default]
    PushButton,
    /// Kerbside detection places the call while someone waits, and cancels it when they leave.
    Passive,
}

impl Detection {
    /// Whether it is the push button, the detection a crossing has unless told otherwise (what is not saved).
    pub fn is_push_button(&self) -> bool {
        *self == Detection::PushButton
    }
}

/// How arm `a`'s crossing detects people: what it was given while it is on call, else the push button (a crossing on
/// recall has none to speak of). Read it here, never from `Arm.detection`.
pub fn detection_of(a: &Arm) -> Detection {
    if a.calls.is_some() { a.detection } else { Detection::PushButton }
}

/// The pedestrian clearance of a crossing `distance_mm` long: its length at the clearance speed, rounded up to the
/// next whole second.
pub fn clearance_s(distance_mm: i32) -> u32 {
    (distance_mm.max(0) as u32).div_ceil(CLEARANCE_MM_PER_S)
}

/// What a call needs of the green that serves it: the walk and then the clearance, never shortened.
pub fn need_s(distance_mm: i32) -> u32 {
    WALK_S + clearance_s(distance_mm)
}

/// The seconds into the hour at which the crossing of street `uid` is called, `rate` times an hour: the hour is cut
/// into `rate` equal slots and each slot gets one call at a point the generator, seeded from the street and the rate,
/// picks. In order, each second at most once. A rate past `MAX_CALLS_PER_HOUR` is read as that.
pub fn arrivals(uid: u32, rate: u32) -> Vec<u32> {
    let rate = rate.min(MAX_CALLS_PER_HOUR);
    control::spread((u64::from(uid) << 32) | u64::from(rate), rate)
}

/// Whether some phase of `plan` walks the crossing of street `uid`.
pub fn walked(plan: &Plan, uid: u32) -> bool {
    plan.phases.iter().any(|p| p.walks.contains(&uid))
}

/// Why the crossing of street `uid` cannot be put on call, if it cannot: the junction has to be a signal, the street
/// needs a crossing (a pedestrian street has none to call), and the plan has to walk it in at least one phase.
pub fn refusal(s: &State, region: usize, uid: u32) -> Option<Refusal> {
    let Some(arm) = s.arms.iter().find(|a| a.uid == uid) else { return Some(Refusal::DoesNotFit) };
    let s = in_force(s);
    if s.control != SIGNAL {
        return Some(Refusal::NotSignal);
    }
    if arm.is_pedestrian() {
        return Some(Refusal::CrossingOnPedestrian);
    }
    if arm.crossing.is_none() {
        return Some(Refusal::NoCrossing);
    }
    let Some(plan) = crate::junction::model::signal::plan(&s, region) else { return Some(Refusal::NotSignal) };
    (!walked(&plan, uid)).then_some(Refusal::CallNotWalked)
}

/// What happened to one call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Outcome {
    /// Served at the start of the first phase that walks its crossing after it was made.
    Served,
    /// Made after a phase that walks its crossing had started: it waited for the next one.
    Delayed,
    /// No phase can fit the walk and its clearance, even lengthened by `MAX_EXTENSION_S` (a `Finding::CannotServe`).
    NotServed,
    /// Passive detection: the pedestrian left after waiting `WAIT_LIMIT_S`, before their walk, and so cancelled the call.
    Cancelled,
}

/// One call: the street whose crossing was called, when (seconds into the hour), what happened, how long it waited for
/// its walk, and how many seconds the green that served it was lengthened for it (0: it fitted).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Call {
    pub arm: u32,
    pub at_s: u32,
    pub outcome: Outcome,
    pub wait_s: u32,
    pub extended_s: u32,
}

/// The calls of one on-call crossing, added up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Totals {
    pub arm: u32,
    /// Its calls an hour.
    pub rate: u32,
    /// The clearance and the walk plus the clearance its length needs (0 when it has no crossing).
    pub clearance_s: u32,
    pub need_s: u32,
    /// Every call of the hour: served, delayed, not served and cancelled.
    pub calls: u32,
    pub served: u32,
    pub delayed: u32,
    pub not_served: u32,
    /// The calls cancelled by a pedestrian who left before the walk (passive detection only).
    pub cancelled: u32,
    /// The calls whose green was lengthened, served or delayed.
    pub extended: u32,
    /// The walks not given because everyone waiting for them had left (passive detection only).
    pub walks_skipped: u32,
    /// The longest a call waited for its walk; None when none was served or delayed, so there is no wait to give.
    pub longest_wait_s: Option<u32>,
}

/// Something about an on-call crossing the person should act on. By street uid; no wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Finding {
    /// No phase that walks the crossing can fit its walk and clearance (`need_s`), even with its green lengthened by
    /// `MAX_EXTENSION_S`: `most_s` is the longest green it could have.
    CannotServe { arm: u32, need_s: u32, most_s: u32 },
    /// The crossing is on call, but no phase walks it any more (an edit took the walk or the crossing away): it is kept
    /// on call, and serves nobody until a phase walks it again.
    NotWalked { arm: u32 },
}

impl Finding {
    /// What the finding is about, whatever its figures: its kind and its street. An edit that changes a crossing's
    /// name or what it needs leaves this alone, so a finding that was there is not taken for a new one.
    pub fn about(&self) -> (u8, u32) {
        match *self {
            Finding::CannotServe { arm, .. } => (0, arm),
            Finding::NotWalked { arm } => (1, arm),
        }
    }
}

/// How the plan serves the calls of one hour.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Summary {
    /// Every call, in the order they were made.
    pub calls: Vec<Call>,
    /// One for each crossing on call that some phase walks, in the order of the bearings.
    pub totals: Vec<Totals>,
    pub findings: Vec<Finding>,
}

/// The calls an hour of arm `a`'s crossing, if it is on call, never past `MAX_CALLS_PER_HOUR`: a document from a sync
/// or written by hand can hold any number. Read it here, never from `Arm.calls`.
pub fn rate_of(a: &Arm) -> Option<u32> {
    a.calls.map(|r| r.min(MAX_CALLS_PER_HOUR))
}

/// The on-call crossings of `s` in the order of the bearings, each with its rate.
fn on_call(s: &State) -> Vec<(&Arm, u32)> {
    let mut arms: Vec<&Arm> = s.arms.iter().collect();
    arms.sort_by_key(|a| a.bearing);
    arms.into_iter().filter_map(|a| rate_of(a).map(|r| (a, r))).collect()
}

/// Runs the cycle of `plan` over an hour of calls at the crossings of `s` that are on call, with the vehicles of its
/// actuated streets (`control::hour`; no bike movement is given a length, so it serves none). `distances` is each crossing's length by street uid
/// (`read_model::crossing_distances`). A crossing no phase walks is a finding and is left out; one no phase can serve
/// (`serve`) is a finding and its calls are not served.
pub fn summary(s: &State, plan: &Plan, distances: &[(u32, i32)]) -> Summary {
    control::hour(s, plan, distances, &[]).calls
}

/// The calls of the hour at the crossings of `s` on call, one `Asked` a crossing, with `out` given a row of totals for
/// each and a finding for each no phase walks (left out).
pub(crate) fn asked(s: &State, plan: &Plan, distances: &[(u32, i32)], out: &mut Summary) -> Vec<Asked> {
    let distance = |uid: u32| distances.iter().find(|(u, _)| *u == uid).map(|(_, d)| *d);
    let mut asked: Vec<Asked> = Vec::new();
    for (arm, rate) in on_call(s) {
        let (Some(d), true) = (distance(arm.uid).filter(|_| arm.crossing.is_some()), walked(plan, arm.uid)) else {
            out.findings.push(Finding::NotWalked { arm: arm.uid });
            continue;
        };
        let need = need_s(d);
        out.totals.push(Totals { arm: arm.uid, rate, clearance_s: clearance_s(d), need_s: need, ..Totals::default() });
        let passive = detection_of(arm) == Detection::Passive;
        asked.push(Asked { arm: arm.uid, need_s: need, at_s: arrivals(arm.uid, rate), passive });
    }
    asked
}

/// A finding for each crossing that no phase walking it can serve by `rules`: a fixed phase needs its green lengthened
/// by at most `MAX_EXTENSION_S` to fit the walk and clearance; an actuated phase holds its green for them.
pub(crate) fn cannot_serve(plan: &Plan, rules: &[control::Rule], asked: &[Asked]) -> Vec<Finding> {
    unserved(plan, rules, asked, walks_it).into_iter().map(|(arm, need_s, most_s)| Finding::CannotServe { arm, need_s, most_s }).collect()
}

/// Whether phase `p` walks the crossing of street `uid`: who a pedestrian call waits for.
pub(crate) fn walks_it(p: &Phase, uid: u32) -> bool {
    p.walks.contains(&uid)
}

/// Each of `asked` that no phase running it (`runs`) can serve by `rules`, as (street, what it needs, the longest green
/// a fixed phase running it could give): a fixed phase needs its green lengthened by at most `MAX_EXTENSION_S` to fit
/// what it needs; an actuated phase holds its green for it. The pedestrian calls' rule, which the bicycle calls share.
pub(crate) fn unserved(plan: &Plan, rules: &[control::Rule], asked: &[Asked], runs: fn(&Phase, u32) -> bool) -> Vec<(u32, u32, u32)> {
    let mut out = Vec::new();
    for a in asked {
        let running = || plan.phases.iter().zip(rules).filter(|(p, _)| runs(p, a.arm));
        let fits =
            |p: &Phase, rule: &control::Rule| matches!(rule, control::Rule::Actuated { .. }) || p.timing.green_s.saturating_add(MAX_EXTENSION_S) >= a.need_s;
        if !running().any(|(p, rule)| fits(p, rule)) {
            let most_s = running().map(|(p, _)| p.timing.green_s).max().unwrap_or(0).saturating_add(MAX_EXTENSION_S);
            out.push((a.arm, a.need_s, most_s));
        }
    }
    out
}

/// `out`'s totals added up from what the run did to every call.
pub(crate) fn tally(out: &mut Summary, asked: &[Asked], run: Run) {
    out.calls = run.calls;
    for t in &mut out.totals {
        for c in out.calls.iter().filter(|c| c.arm == t.arm) {
            match c.outcome {
                Outcome::Served => t.served += 1,
                Outcome::Delayed => t.delayed += 1,
                Outcome::NotServed => t.not_served += 1,
                Outcome::Cancelled => t.cancelled += 1,
            }
            t.calls += 1;
            t.extended += u32::from(c.extended_s > 0);
            if matches!(c.outcome, Outcome::Served | Outcome::Delayed) {
                t.longest_wait_s = Some(t.longest_wait_s.unwrap_or(0).max(c.wait_s));
            }
        }
        t.walks_skipped = asked.iter().zip(&run.skipped).filter(|(a, _)| a.arm == t.arm).map(|(_, n)| n).sum();
    }
}

/// The calls made at one crossing: its street, what a call needs of the green (`need_s`), when each was made, and
/// whether the crossing has passive detection (a pedestrian who leaves cancels their call).
pub(crate) struct Asked {
    pub(crate) arm: u32,
    pub(crate) need_s: u32,
    pub(crate) at_s: Vec<u32>,
    pub(crate) passive: bool,
}

/// What `serve` gives: every call, and the walks each crossing skipped (by its place in the `asked` it was given).
pub(crate) struct Run {
    pub(crate) calls: Vec<Call>,
    skipped: Vec<u32>,
}

/// The phases of `plan` run in turn from second 0, round and round, until every call that can be served is.
///
/// A call is served at the start of the first phase, at or after it was made, that walks its crossing and can fit the
/// walk and clearance; made while such a phase was running, it missed that walk and is delayed. A phase serving a call
/// whose crossing needs more than its green has the green lengthened by the shortfall, at most `MAX_EXTENSION_S`, its
/// yellow and all-red kept, so every phase after it starts that much later. A phase that cannot fit the walk and
/// clearance even so does not serve that crossing: they are never shortened. A crossing no phase can serve has its
/// calls not served. The calls come back in the order they were made (crossings in the order given at the same second).
///
/// Passive detection (`Asked.passive`): who leaves follows from the seeded arrivals alone. A pedestrian waits
/// `WAIT_LIMIT_S` at most; one whose walk would start later than that after they arrived has left by then and cancelled
/// their call (`Outcome::Cancelled`, waited `WAIT_LIMIT_S`). A walk everyone waiting for has left is skipped: counted in
/// `Run.skipped`, it lengthens no green. Whoever is still there is given the whole walk and clearance as above. A
/// crossing no phase can serve keeps its calls not served: there is no walk to leave before.
///
/// The run is the controller's (`control::run`), with every phase fixed: a phase takes at least a second, the clock
/// counts in u64, and the run stops a cycle at its longest (each phase extended by at most `MAX_EXTENSION_S`,
/// `stop_at`) past the last call, whatever is still waiting then not served. The hour runs the calls with the vehicles
/// through `control::hour`; this runs them alone on a fixed plan, as the tests pin them.
#[cfg(test)]
fn serve(plan: &Plan, asked: &[Asked]) -> Run {
    serve_turns(plan, asked, MAX_TURNS)
}

/// A bound on the phases `serve` runs, only a guard: the run stops a cycle at its longest past the last call of the hour
/// (`stop_at`), and a phase lasts a second or more, so it takes at most about an hour and a cycle of one-second phases.
pub(crate) const MAX_TURNS: usize = 100_000;

/// When the run stops: a cycle at its longest past the `last` call. Each phase takes a second or more, and its green
/// may be extended by at most `MAX_EXTENSION_S`, so a call that can be served is, at the latest, a cycle that long after
/// it was made (one that misses an extended walk waits for the next): the bound is a guard, and cuts off no call.
#[cfg(test)]
fn stop_at(plan: &Plan, last: u64) -> u64 {
    let rules = control::fixed(plan);
    let mut walks = Walks::new(plan, &rules, &[]);
    let demand: &mut dyn control::Demand = &mut walks;
    control::stop_at(plan, &rules, last, &[demand])
}

/// `serve`, run for at most `turns` phases. A call still waiting when it stops is not served.
#[cfg(test)]
fn serve_turns(plan: &Plan, asked: &[Asked], turns: usize) -> Run {
    let rules = control::fixed(plan);
    let mut walks = Walks::new(plan, &rules, asked);
    control::run(plan, &rules, &mut [&mut walks], turns);
    walks.finish()
}

/// The pedestrian calls as a demand on the controller: who is waiting at which crossing, which phases can serve each
/// crossing, and what happened to every call so far. The bicycle calls (`bicycles.rs`, CLM-13) are the same demand over
/// the phases that run a street's bike movement (`Walks::over`): a cyclist waits as a pedestrian on a push button does.
pub(crate) struct Walks<'a> {
    asked: &'a [Asked],
    /// Whether each phase (by its place in the plan) can serve each crossing: looked up once a turn, never searched. A
    /// fixed phase can when it walks the crossing and its green lengthened by `MAX_EXTENSION_S` fits the walk and
    /// clearance; an actuated phase holds its green for them, so it can whenever it walks the crossing.
    serving: Vec<Vec<bool>>,
    /// Every call with the crossing it is for (its place in `asked`), in the order they were made.
    calls: Vec<(usize, Call)>,
    /// The calls still waiting for their walk (places in `calls`), and whether each has missed one.
    waiting: Vec<usize>,
    missed: Vec<bool>,
    /// The walks each crossing skipped because everyone waiting for them had left.
    skipped: Vec<u32>,
    /// The longest walk and clearance each phase can be asked to hold its green for.
    longest: Vec<u32>,
    /// Who waits (pedestrians, or cyclists for the bicycle calls), and the part of what a call needs that may end early
    /// when the green is interrupted (the walk, or the bicycle minimum green): the rest, the clearance, never does.
    kind: control::Kind,
    first_s: u32,
}

impl<'a> Walks<'a> {
    pub(crate) fn new(plan: &Plan, rules: &[control::Rule], asked: &'a [Asked]) -> Walks<'a> {
        Walks::over(plan, rules, asked, walks_it)
    }

    /// The calls of `asked`, each served by the phases that run its street's crossing or movement (`runs`).
    pub(crate) fn over(plan: &Plan, rules: &[control::Rule], asked: &'a [Asked], runs: fn(&Phase, u32) -> bool) -> Walks<'a> {
        let serving: Vec<Vec<bool>> = asked
            .iter()
            .map(|a| {
                plan.phases
                    .iter()
                    .zip(rules)
                    .map(|(p, rule)| {
                        runs(p, a.arm)
                            && match rule {
                                control::Rule::Fixed => p.timing.green_s.saturating_add(MAX_EXTENSION_S) >= a.need_s,
                                control::Rule::Actuated { .. } => true,
                            }
                    })
                    .collect()
            })
            .collect();
        let mut calls: Vec<(usize, Call)> = asked
            .iter()
            .enumerate()
            .flat_map(|(x, a)| {
                let outcome = if serving[x].contains(&true) { Outcome::Served } else { Outcome::NotServed };
                a.at_s.iter().map(move |&at_s| (x, Call { arm: a.arm, at_s, outcome, wait_s: 0, extended_s: 0 }))
            })
            .collect();
        calls.sort_by_key(|(x, c)| (c.at_s, *x));
        let waiting: Vec<usize> = (0..calls.len()).filter(|&i| calls[i].1.outcome != Outcome::NotServed).collect();
        let longest = (0..plan.phases.len()).map(|k| asked.iter().zip(&serving).filter(|(_, s)| s[k]).map(|(a, _)| a.need_s).max().unwrap_or(0)).collect();
        let missed = vec![false; calls.len()];
        Walks { asked, serving, calls, waiting, missed, skipped: vec![0; asked.len()], longest, kind: control::Kind::Pedestrian, first_s: WALK_S }
    }

    /// The same calls standing for `kind` of road user, the first `first_s` of what each needs ending early when the green
    /// is interrupted (the bicycle calls: cyclists, their minimum green).
    pub(crate) fn of(self, kind: control::Kind, first_s: u32) -> Walks<'a> {
        Walks { kind, first_s, ..self }
    }

    /// Whether phase `k` serves call `i`'s crossing.
    fn serves(&self, k: usize, i: usize) -> bool {
        self.serving[self.calls[i].0][k]
    }

    /// The calls waiting at `at` whose crossing phase `k` serves (places in `calls`).
    fn present(&self, k: usize, at: u64) -> impl Iterator<Item = usize> + '_ {
        self.waiting.iter().copied().filter(move |&i| u64::from(self.calls[i].1.at_s) <= at && self.serves(k, i))
    }

    /// Whether the pedestrian of call `i` has left by `at`: passive detection, waited past `WAIT_LIMIT_S`.
    fn left(&self, i: usize, at: u64) -> bool {
        let (x, c) = &self.calls[i];
        self.asked[*x].passive && at - u64::from(c.at_s) > u64::from(WAIT_LIMIT_S)
    }

    /// Every call, what happened to it, and the walks each crossing skipped. A call still waiting is not served.
    pub(crate) fn finish(mut self) -> Run {
        for &i in &self.waiting {
            self.calls[i].1.outcome = Outcome::NotServed;
        }
        Run { calls: self.calls.into_iter().map(|(_, c)| c).collect(), skipped: self.skipped }
    }
}

impl control::Demand for Walks<'_> {
    /// Someone still at the kerb (not left) waits for a walk the phase gives.
    fn calls(&self, k: usize, at: u64) -> bool {
        self.present(k, at).any(|i| !self.left(i, at))
    }

    /// The walk and clearance of everyone the phase's walk serves, never shortened.
    fn hold(&self, k: usize, start: u64) -> u32 {
        self.present(k, start).filter(|&i| !self.left(i, start)).map(|i| self.asked[self.calls[i].0].need_s).max().unwrap_or(0)
    }

    /// Serves the calls waiting when the phase started, cancels those whose pedestrian left, counts a walk nobody was
    /// left for as skipped, and marks the calls made while it walked their crossing as having missed that walk.
    fn ran(&mut self, turn: &control::Turn) {
        let (k, start) = (turn.phase, turn.start);
        let now: Vec<usize> = self.present(k, start).collect();
        // Which crossings this walk serves someone at, and which had someone leave before it.
        let (mut walked, mut left) = (vec![false; self.asked.len()], vec![false; self.asked.len()]);
        for &i in &now {
            let x = self.calls[i].0;
            let wait = start - u64::from(self.calls[i].1.at_s);
            if self.left(i, start) {
                let c = &mut self.calls[i].1;
                *c = Call { outcome: Outcome::Cancelled, wait_s: WAIT_LIMIT_S, extended_s: 0, ..*c };
                left[x] = true;
                continue;
            }
            let short = self.asked[x].need_s.saturating_sub(turn.base_s);
            let missed = self.missed[i];
            let c = &mut self.calls[i].1;
            c.wait_s = u32::try_from(wait).unwrap_or(u32::MAX);
            c.extended_s = short;
            c.outcome = if missed { Outcome::Delayed } else { Outcome::Served };
            walked[x] = true;
        }
        for (x, n) in self.skipped.iter_mut().enumerate() {
            *n += u32::from(left[x] && !walked[x]);
        }
        self.waiting.retain(|i| !now.contains(i));
        // A call made while this phase walks its crossing has missed that walk.
        if !turn.skipped {
            for w in 0..self.waiting.len() {
                let i = self.waiting[w];
                let at = u64::from(self.calls[i].1.at_s);
                if self.serves(k, i) && at > start && at < turn.end {
                    self.missed[i] = true;
                }
            }
        }
    }

    fn pending(&self) -> bool {
        !self.waiting.is_empty()
    }

    fn next_after(&self, at: u64) -> Option<u64> {
        self.waiting.iter().map(|&i| u64::from(self.calls[i].1.at_s)).filter(|&t| t > at).min()
    }

    fn last(&self) -> u64 {
        self.calls.iter().map(|(_, c)| u64::from(c.at_s)).max().unwrap_or(0)
    }

    fn most_extension(&self) -> u32 {
        MAX_EXTENSION_S
    }

    fn longest_hold(&self, k: usize) -> u32 {
        self.longest[k]
    }

    fn kind(&self) -> Option<control::Kind> {
        Some(self.kind)
    }

    /// The calls made by `at` and still waiting (whoever has not left) that one of `phases` serves.
    fn waiting(&self, at: u64, phases: &[usize]) -> u32 {
        self.waiting.iter().filter(|&&i| u64::from(self.calls[i].1.at_s) <= at && !self.left(i, at) && phases.iter().any(|&k| self.serves(k, i))).count() as u32
    }

    /// The walks served in the phase's green (those waiting at its start) run from `green_start`: one still showing at
    /// `at` ends there and its clearance runs in full; a clearance under way runs out.
    fn clear_by(&self, k: usize, start: u64, green_start: u64, at: u64) -> u64 {
        self.present(k, start)
            .filter(|&i| !self.left(i, start))
            .map(|i| clear_at(green_start, self.first_s, self.asked[self.calls[i].0].need_s, at))
            .max()
            .unwrap_or(at)
    }
}

/// When what needs `need_s` of a green from `green_start`, its first `first_s` a walk (or a bicycle minimum green) and the
/// rest its clearance, can stop for an interruption at `at`: a walk still showing ends at `at` and the clearance follows
/// in full; a clearance under way runs out; one done, or not begun, keeps nothing.
pub(crate) fn clear_at(green_start: u64, first_s: u32, need_s: u32, at: u64) -> u64 {
    let (walk_end, end) = (green_start + u64::from(first_s.min(need_s)), green_start + u64::from(need_s));
    if at < walk_end { at + u64::from(need_s - first_s.min(need_s)) } else { at.max(end) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::signal::{self, Phase, SignalPlan, StoredPhase};
    use crate::junction::model::{Junction, Lane, PRIORITY, Timing};

    fn at(s: &State, bearing: i32) -> u32 {
        s.arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    fn send(s: &mut State, from: u32, to: &[u32]) {
        s.arms.iter_mut().find(|a| a.uid == from).unwrap().lanes = vec![Lane { to: to.to_vec() }];
    }

    /// The four-way on a signal, every street's traffic going straight on: north and south's phases walk east and west,
    /// east and west's walk north and south. Each phase is green 25 s, yellow 3 s, all-red 2 s: 30 s, and a 120 s cycle.
    fn cross() -> State {
        let mut s = Junction::new(0).current().clone();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        send(&mut s, n, &[so]);
        send(&mut s, so, &[n]);
        send(&mut s, e, &[w]);
        send(&mut s, w, &[e]);
        s
    }

    fn arm_mut(s: &mut State, uid: u32) -> &mut Arm {
        s.arms.iter_mut().find(|a| a.uid == uid).unwrap()
    }

    fn summary_of(s: &State, distances: &[(u32, i32)]) -> Summary {
        summary(s, &signal::plan(s, 0).unwrap(), distances)
    }

    /// A phase of street `uid` with a `green_s` green, yellow 3 s and all-red 2 s, walking `walks`.
    fn phase(uid: u32, green_s: u32, walks: &[u32]) -> Phase {
        let timing = Timing { green_s, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        Phase { uid, timing, length_s: timing.length_s(), walks: walks.to_vec(), rides: vec![], moves: vec![] }
    }

    fn plan_of(phases: Vec<Phase>) -> Plan {
        let cycle_s = phases.iter().fold(0u32, |sum, p| sum.saturating_add(p.length_s));
        Plan { phases, cycle_s, suggested: false }
    }

    fn call(arm: u32, at_s: u32, outcome: Outcome, wait_s: u32, extended_s: u32) -> Call {
        Call { arm, at_s, outcome, wait_s, extended_s }
    }

    #[test]
    fn the_clearance_is_the_length_at_one_metre_a_second_rounded_up_and_the_walk_comes_first() {
        assert_eq!((WALK_S, CLEARANCE_MM_PER_S, MAX_EXTENSION_S), (7, 1_000, 15), "the placeholder figures the issue settled");
        assert_eq!(clearance_s(9_000), 9);
        assert_eq!(clearance_s(9_001), 10, "rounded up to the next second");
        assert_eq!(clearance_s(0), 0);
        assert_eq!(need_s(12_400), 7 + 13);
    }

    #[test]
    fn the_calls_of_an_hour_are_the_same_every_time_spread_over_the_hour_in_order() {
        let calls = arrivals(7, 12);
        assert_eq!(calls, arrivals(7, 12), "seeded: the same street and rate give the same calls");
        assert_eq!(calls.len(), 12);
        for (k, at) in calls.iter().enumerate() {
            assert!((k as u32 * 300..(k as u32 + 1) * 300).contains(at), "one call in each twelfth of the hour: {calls:?}");
        }
        assert_ne!(arrivals(8, 12), calls, "another street is called at other times");
        assert!(arrivals(7, 0).is_empty());
        let most = arrivals(7, MAX_CALLS_PER_HOUR);
        assert_eq!(most.len(), MAX_CALLS_PER_HOUR as usize);
        assert!(most.windows(2).all(|w| w[0] < w[1]) && most.iter().all(|&t| t < HOUR_S), "in order, within the hour: {most:?}");
        // Pinned: a change of generator changes every summary, so it has to be a decision, not an accident.
        assert_eq!(arrivals(1, 4), PINNED_ARRIVALS);
    }

    const PINNED_ARRIVALS: [u32; 4] = [769, 1725, 2634, 3541];

    #[test]
    fn a_call_is_served_when_the_next_phase_that_walks_its_crossing_starts_and_delayed_if_it_missed_one() {
        // Street 9's crossing is walked in the first phase (0 to 30 s of each 60 s cycle), not in the second.
        let plan = plan_of(vec![phase(1, 25, &[9]), phase(2, 25, &[])]);
        let asked = [Asked { arm: 9, need_s: 16, at_s: vec![0, 10, 40], passive: false }];
        assert_eq!(
            serve(&plan, &asked).calls,
            vec![
                call(9, 0, Outcome::Served, 0, 0),
                call(9, 10, Outcome::Delayed, 50, 0), // its walk had started: the next is at 60 s
                call(9, 40, Outcome::Served, 20, 0),  // made while nothing walks it: served at the first chance
            ]
        );
    }

    #[test]
    fn a_short_green_is_lengthened_by_the_shortfall_and_every_phase_after_it_starts_later() {
        // Street 9 needs 19 s and its phase has a 10 s green: lengthened by 9 s, to 24 s in all; the second phase then
        // runs 24 to 54 s and the first again from 54 s. Street 8's crossing, walked in the second phase, waits 9 s
        // longer than it would have.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 25, &[8])]);
        let asked = [Asked { arm: 9, need_s: 19, at_s: vec![0, 5, 30], passive: false }, Asked { arm: 8, need_s: 10, at_s: vec![0], passive: false }];
        assert_eq!(
            serve(&plan, &asked).calls,
            vec![
                call(9, 0, Outcome::Served, 0, 9),
                call(8, 0, Outcome::Served, 24, 0),
                call(9, 5, Outcome::Delayed, 49, 9),
                call(9, 30, Outcome::Served, 24, 9),
            ]
        );
        // With no call at street 9, its phase keeps its 15 s and street 8 waits only that long.
        assert_eq!(serve(&plan, &asked[1..]).calls, vec![call(8, 0, Outcome::Served, 15, 0)]);
    }

    #[test]
    fn a_phase_that_cannot_fit_the_walk_and_clearance_even_lengthened_does_not_serve_the_call() {
        // 26 s needed: 16 s more than the first phase's 10 s green, past the 15 s bound, so only the third (25 s) serves.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 25, &[]), phase(3, 25, &[9])]);
        let asked = [Asked { arm: 9, need_s: 26, at_s: vec![0, 5], passive: false }];
        assert_eq!(
            serve(&plan, &asked).calls,
            vec![call(9, 0, Outcome::Served, 45, 1), call(9, 5, Outcome::Served, 40, 1)],
            "the first phase is no walk to miss"
        );
        // No phase at all: not served, and nothing waits for it.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 25, &[])]);
        assert_eq!(serve(&plan, &asked).calls, vec![call(9, 0, Outcome::NotServed, 0, 0), call(9, 5, Outcome::NotServed, 0, 0)]);
    }

    /// A phase of street `uid` that takes no time at all (a document's timing is not clamped when it is read).
    fn empty(uid: u32, walks: &[u32]) -> Phase {
        let timing = Timing { green_s: 0, yellow_s: 0, all_red_s: 0, transit_lead_s: 0 };
        Phase { uid, timing, length_s: 0, walks: walks.to_vec(), rides: vec![], moves: vec![] }
    }

    #[test]
    fn a_plan_whose_phases_take_no_time_still_ends_at_once_and_serves_its_calls() {
        let plan = plan_of(vec![empty(1, &[9]), empty(2, &[])]);
        let asked = [Asked { arm: 9, need_s: 10, at_s: vec![0, 100, 3_599], passive: false }];
        let started = std::time::Instant::now();
        let calls = serve(&plan, &asked).calls;
        assert!(started.elapsed() < std::time::Duration::from_secs(1), "it took {:?}", started.elapsed());
        // The first phase's green of 0 s is lengthened by the 10 s the crossing needs; a phase that takes no time
        // takes a second in the run, so the walk comes round again.
        assert_eq!(calls[0], call(9, 0, Outcome::Served, 0, 10));
        assert!(calls.iter().all(|c| c.outcome != Outcome::NotServed && c.wait_s < 60 && c.extended_s == 10), "{calls:?}");
    }

    /// A phase of street `uid` with any green, yellow 3 s and all-red 2 s, walking `walks`: a document's timing is not
    /// bounded until it is settled, so the run must hold whatever it is given.
    fn huge(uid: u32, green_s: u32, walks: &[u32]) -> Phase {
        let timing = Timing { green_s, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        Phase { uid, timing, length_s: green_s.saturating_add(5), walks: walks.to_vec(), rides: vec![], moves: vec![] }
    }

    #[test]
    fn timings_too_long_for_the_clock_end_the_run_at_once_without_wrapping() {
        // Four phases of 2^30 s: a cycle of exactly 2^32 s, which a u32 clock wraps back to 0.
        let g = (1u32 << 30) - 5;
        let plan = plan_of(vec![huge(1, g, &[9]), huge(2, g, &[]), huge(3, g, &[]), huge(4, g, &[])]);
        let asked = [Asked { arm: 9, need_s: 16, at_s: vec![0, 100], passive: false }];
        let started = std::time::Instant::now();
        let calls = serve(&plan, &asked).calls;
        assert!(started.elapsed() < std::time::Duration::from_secs(1), "it took {:?}", started.elapsed());
        assert_eq!(calls[0], call(9, 0, Outcome::Served, 0, 0));
        // The second call missed the first walk, and the next starts a whole 2^32 s cycle later: counted, not wrapped.
        assert_eq!(calls[1], call(9, 100, Outcome::Delayed, u32::MAX - 99, 0));
        // A green so long that adding the extension to it would overflow.
        let plan = plan_of(vec![huge(1, u32::MAX - 3, &[9]), phase(2, 25, &[])]);
        let calls = serve(&plan, &[Asked { arm: 9, need_s: 16, at_s: vec![0, 50], passive: false }]).calls;
        assert_eq!(calls[0], call(9, 0, Outcome::Served, 0, 0));
        // The first phase lasts as long as a u32 goes: its next walk is a cycle on, counted without wrapping.
        assert_eq!(calls[1], call(9, 50, Outcome::Delayed, u32::MAX - 20, 0));
        // Stopped by its turn limit before the walk comes round, the call is not served (never served with no wait).
        let plan = plan_of(vec![huge(1, 25, &[]), huge(2, 25, &[9])]);
        let calls = serve_turns(&plan, &[Asked { arm: 9, need_s: 16, at_s: vec![0], passive: false }], 1).calls;
        assert_eq!(calls, vec![call(9, 0, Outcome::NotServed, 0, 0)], "still waiting when the run stops");
    }

    #[test]
    fn a_document_with_timings_past_their_range_reads_them_within_it() {
        use crate::junction::model::TimingField;
        let mut now = Junction::new(0).current().clone();
        now.arms[0].timing = Some(Timing { green_s: (1 << 30) - 5, yellow_s: u32::MAX, all_red_s: 0, transit_lead_s: 99 });
        let j = Junction::from_city(crate::street::model::named_said("J"), Junction::new(0).current(), &now, 0).unwrap();
        let t = j.current().arms.iter().find(|a| a.uid == now.arms[0].uid).unwrap().timing.unwrap();
        for f in TimingField::ALL {
            let (lo, hi) = f.range();
            assert!((lo as u32..=hi as u32).contains(&f.get(&t)), "{f:?} is {} on load", f.get(&t));
        }
        assert_eq!(t.transit_lead_s, 0, "no transit lane: no early green, as an edit settles it");
    }

    #[test]
    fn a_call_that_misses_an_extended_walk_is_delayed_to_the_next_however_long_the_extension() {
        // The first phase walks street 9's crossing, green 25 s, 30 s in all; the second runs 30 s. The crossing needs
        // 35 s, so the green is extended by 10 s: the first phase runs 0 to 40 s, the second 40 to 70 s. The call at 5 s
        // missed that walk; the next starts at 70 s, past the last call and a cycle of the phases unextended (65 s).
        let plan = plan_of(vec![phase(1, 25, &[9]), phase(2, 25, &[])]);
        let asked = [Asked { arm: 9, need_s: 35, at_s: vec![0, 5], passive: false }];
        assert_eq!(serve(&plan, &asked).calls, vec![call(9, 0, Outcome::Served, 0, 10), call(9, 5, Outcome::Delayed, 65, 10)]);
    }

    #[test]
    fn the_run_stops_a_longest_cycle_past_the_last_call_and_the_latest_walk_falls_within_it() {
        // Two phases of 15 s, each walking a crossing that needs 25 s: each green, 10 s, is extended by the most, 15 s.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 10, &[8])]);
        assert_eq!(stop_at(&plan, 1), 1 + 2 * (15 + MAX_EXTENSION_S as u64), "the last call, then a cycle at its longest");
        // The worst a call can wait: made a second into an extended walk it missed, while the other phase is extended
        // too. Its walk comes round at 60 s, a second inside the bound (61 s): delayed, not cut off.
        let asked = [Asked { arm: 9, need_s: 25, at_s: vec![0, 1], passive: false }, Asked { arm: 8, need_s: 25, at_s: vec![0], passive: false }];
        assert_eq!(
            serve(&plan, &asked).calls,
            vec![call(9, 0, Outcome::Served, 0, 15), call(8, 0, Outcome::Served, 30, 15), call(9, 1, Outcome::Delayed, 59, 15)]
        );
    }

    #[test]
    fn a_stored_plan_of_endless_phases_is_read_as_many_as_an_edit_allows_and_served_at_once() {
        use crate::junction::model::MAX_ARMS;
        let mut s = cross();
        let (n, east, so) = (at(&s, 0), at(&s, 90), at(&s, 180));
        let phase = StoredPhase { moves: vec![signal::Move { from: n, to: so }], walks: vec![east], rides: vec![], arm: None };
        s.signal_plan = Some(SignalPlan { phases: vec![phase; 100_000] });
        arm_mut(&mut s, east).calls = Some(MAX_CALLS_PER_HOUR);
        let started = std::time::Instant::now();
        let plan = signal::plan(&s, 0).unwrap();
        assert_eq!(plan.phases.len(), MAX_ARMS * 2, "no more phases than an edit can make");
        let sum = summary(&s, &plan, &[(east, 9_000)]);
        assert_eq!(sum.calls.len(), MAX_CALLS_PER_HOUR as usize);
        assert!(started.elapsed() < std::time::Duration::from_secs(1), "it took {:?}", started.elapsed());
    }

    #[test]
    fn a_call_still_waiting_when_the_run_stops_is_not_served_never_served_with_no_wait() {
        let plan = plan_of(vec![phase(1, 25, &[9]), phase(2, 25, &[])]);
        let asked = [Asked { arm: 9, need_s: 16, at_s: vec![0, 10, 40], passive: false }];
        // Two turns: the first phase (serving the call at 0 s), then the second; the others are still waiting.
        assert_eq!(
            serve_turns(&plan, &asked, 2).calls,
            vec![call(9, 0, Outcome::Served, 0, 0), call(9, 10, Outcome::NotServed, 0, 0), call(9, 40, Outcome::NotServed, 0, 0)]
        );
    }

    #[test]
    fn a_rate_past_the_bound_in_a_document_is_read_as_the_bound() {
        assert_eq!(arrivals(7, u32::MAX).len(), MAX_CALLS_PER_HOUR as usize);
        assert_eq!(arrivals(7, u32::MAX), arrivals(7, MAX_CALLS_PER_HOUR), "the same calls as at the bound");
        let mut s = cross();
        let e = at(&s, 90);
        let mut text = serde_json::to_value(&s).unwrap();
        let arm = text["arms"].as_array_mut().unwrap().iter_mut().find(|a| a["uid"] == e).unwrap();
        arm["calls"] = serde_json::json!(4_294_967_295u32);
        let mut loaded: State = serde_json::from_value(text).unwrap();
        // The streets are the city's, never saved with the junction: put back as the city does when it opens it.
        for (a, b) in loaded.arms.iter_mut().zip(&s.arms) {
            a.section = b.section.clone();
        }
        s = loaded;
        assert_eq!(rate_of(s.arms.iter().find(|a| a.uid == e).unwrap()), Some(MAX_CALLS_PER_HOUR));
        let sum = summary_of(&s, &[(e, 9_000)]);
        assert_eq!((sum.totals[0].rate, sum.calls.len()), (MAX_CALLS_PER_HOUR, MAX_CALLS_PER_HOUR as usize));
    }

    #[test]
    fn a_junction_that_falls_back_on_how_it_was_laid_out_is_not_said_to_have_lost_its_calls_to_a_control() {
        // Today a signal with no call; now its crossing is on call, but two streets on one bearing cannot be drawn,
        // even eased: it opens as it was laid out. The call went with the drawing, not because it is no signal.
        let today = Junction::new(0).current().clone();
        let mut now = today.clone();
        now.arms[0].calls = Some(12);
        now.arms[1].bearing = now.arms[0].bearing;
        let opened = Junction::from_city(crate::street::model::named_said("J"), &today, &now, 0).unwrap();
        assert_eq!(opened.current().control, crate::junction::model::SIGNAL);
        assert!(opened.current().arms.iter().all(|a| a.calls.is_none()), "it opened on today");
        assert_eq!(opened.calls_dropped(), 0, "not said to be off call for want of a signal");
    }

    #[test]
    fn a_stored_plan_longer_than_an_edit_can_make_says_how_many_phases_it_left_out_until_edited() {
        use crate::junction::model::MAX_ARMS;
        let today = cross();
        let mut now = today.clone();
        let phase = signal::plan(&today, 0).unwrap().to_stored().phases[0].clone();
        now.signal_plan = Some(SignalPlan { phases: vec![phase; MAX_ARMS * 2 + 2] });
        let opened = Junction::from_city(crate::street::model::named_said("J"), &today, &now, 0).unwrap();
        assert_eq!(signal::plan(opened.current(), 0).unwrap().phases.len(), MAX_ARMS * 2);
        assert_eq!((opened.phases_cut(), opened.view().phases_cut), (2, 2), "the two past the bound");
        let mut edited = opened;
        let uid = edited.current().arms[0].uid;
        assert!(edited.step_corner(uid, 1));
        assert_eq!(edited.view().phases_cut, 0, "said until the first edit, which writes the plan as it is read");
        let plain = Junction::from_city(crate::street::model::named_said("J"), &today, &today, 0).unwrap();
        assert_eq!(plain.phases_cut(), 0);
    }

    #[test]
    fn a_junction_that_stops_being_a_signal_takes_its_crossings_off_call_and_says_so() {
        let mut j = Junction::new(0);
        let e = at(j.current(), 90);
        assert!(j.set_on_call(e, true));
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.arm(e).unwrap().calls, None, "no signal, no calls: nothing is left on call where no section shows it");
        let label = &j.current().label;
        assert_eq!(label.key, "jn-rev-calls-lost", "the revision says the calls went: {label:?}");
        assert!(matches!(&label.args[0].1, crate::shared::said::Arg::Said(inner) if inner.key.starts_with("jn-rev-control")));
        // A document saved with calls on a junction that is not a signal opens without them.
        let mut kept = Junction::new(0).current().clone();
        kept.control = PRIORITY;
        kept.arms[0].calls = Some(12);
        let opened = Junction::from_city(crate::street::model::named_said("J"), &kept, &kept, 0).unwrap();
        assert!(opened.current().arms.iter().all(|a| a.calls.is_none()));
        // ...and says so, without a step to undo: nobody has edited it.
        assert!(!opened.changed() && !opened.can_undo(), "opening is no edit");
        assert_eq!(opened.view().calls_dropped, 1, "the junction's level says how many were taken off");
        // Its first edit is what writes the record without them: the note has been said by then.
        let mut edited = opened;
        let uid = edited.current().arms[0].uid;
        assert!(edited.step_corner(uid, 1));
        assert_eq!(edited.view().calls_dropped, 0);
        // When the junction opens on changes of its own, its state says it as an edit that took them off would.
        let mut changed = kept.clone();
        changed.arms[1].corner_mm += 1_000;
        let opened = Junction::from_city(crate::street::model::named_said("J"), &kept, &changed, 0).unwrap();
        let label = &opened.current().label;
        assert_eq!(label.key, "jn-rev-calls-lost", "{label:?}");
        assert_eq!(label.args[1], ("n", crate::shared::said::Arg::Num(1)));
        assert!(matches!(&label.args[0].1, crate::shared::said::Arg::Said(inner) if inner.key == crate::junction::model::earlier().key));
        // A signal keeps its calls, and opens as it did.
        let mut signal = Junction::new(0).current().clone();
        signal.arms[0].calls = Some(12);
        let opened = Junction::from_city(crate::street::model::named_said("J"), Junction::new(0).current(), &signal, 0).unwrap();
        assert_eq!((opened.current().arms[0].calls, opened.current().label.key), (Some(12), crate::junction::model::earlier().key));
    }

    #[test]
    fn a_crossing_can_go_on_call_only_where_the_plan_walks_it() {
        let mut s = cross();
        let (n, e) = (at(&s, 0), at(&s, 90));
        assert_eq!(refusal(&s, 0, e), None, "north and south's phases walk east");
        // An edited plan that walks only east: north's crossing is walked by no phase.
        let only = StoredPhase { moves: vec![signal::Move { from: n, to: at(&s, 180) }], walks: vec![e], rides: vec![], arm: None };
        s.signal_plan = Some(SignalPlan { phases: vec![only] });
        assert_eq!(refusal(&s, 0, n), Some(Refusal::CallNotWalked));
        assert_eq!(refusal(&s, 0, e), None);
        arm_mut(&mut s, e).crossing = None;
        assert_eq!(refusal(&s, 0, e), Some(Refusal::NoCrossing));
        let mut not_signal = cross();
        not_signal.control = PRIORITY;
        assert_eq!(refusal(&not_signal, 0, e), Some(Refusal::NotSignal));
    }

    #[test]
    fn no_crossing_on_call_means_no_calls_and_no_findings() {
        assert_eq!(summary_of(&cross(), &[]), Summary::default());
    }

    #[test]
    fn a_crossing_that_no_phase_can_serve_is_a_finding_and_its_calls_are_counted_as_not_served() {
        let mut s = cross();
        let (n, so, e) = (at(&s, 0), at(&s, 180), at(&s, 90));
        for uid in [n, so] {
            arm_mut(&mut s, uid).timing = Some(Timing { green_s: 10, ..Timing::default() });
        }
        arm_mut(&mut s, e).calls = Some(6);
        // 7 s walk + 19 s clearance = 26 s: 16 s more than the 10 s greens that walk it.
        let sum = summary_of(&s, &[(e, 18_500)]);
        assert_eq!(sum.findings, vec![Finding::CannotServe { arm: e, need_s: 26, most_s: 25 }]);
        assert!(sum.calls.iter().all(|c| c.outcome == Outcome::NotServed), "{sum:?}");
        assert_eq!((sum.totals[0].not_served, sum.totals[0].served, sum.totals[0].clearance_s), (6, 0, 19));
        // Every call is counted, and with none served or delayed there is no wait to give: none, not 0 s.
        assert_eq!((sum.totals[0].calls, sum.totals[0].longest_wait_s), (6, None));
        // 7 s + 12 s = 19 s fits with 9 s more: every call is served lengthened.
        let sum = summary_of(&s, &[(e, 11_500)]);
        assert!(sum.findings.is_empty());
        assert!(sum.calls.iter().all(|c| c.extended_s == 9 && c.outcome != Outcome::NotServed), "{sum:?}");
        assert_eq!(sum.totals[0].extended, 6);
        assert_eq!(sum.totals[0].calls, 6);
        assert!(sum.totals[0].longest_wait_s.is_some(), "calls served: the longest wait is given");
    }

    #[test]
    fn a_crossing_left_on_call_that_no_phase_walks_is_a_finding_and_is_kept() {
        let mut s = cross();
        let (n, so, e) = (at(&s, 0), at(&s, 180), at(&s, 90));
        arm_mut(&mut s, e).calls = Some(12);
        s.signal_plan =
            Some(SignalPlan { phases: vec![StoredPhase { moves: vec![signal::Move { from: n, to: so }], walks: vec![], rides: vec![], arm: None }] });
        let sum = summary_of(&s, &[(e, 9_000)]);
        assert_eq!(sum.findings, vec![Finding::NotWalked { arm: e }]);
        assert!(sum.calls.is_empty() && sum.totals.is_empty());
        // The crossing taken away altogether: still a finding.
        let mut gone = cross();
        arm_mut(&mut gone, e).calls = Some(12);
        arm_mut(&mut gone, e).crossing = None;
        assert_eq!(summary_of(&gone, &[]).findings, vec![Finding::NotWalked { arm: e }]);
    }

    #[test]
    fn fixed_inputs_give_a_fixed_summary() {
        let mut s = cross();
        let (e, w) = (at(&s, 90), at(&s, 270));
        arm_mut(&mut s, e).calls = Some(3);
        arm_mut(&mut s, w).calls = Some(2);
        let sum = summary_of(&s, &[(e, 9_000), (w, 21_000)]);
        assert_eq!(sum, summary_of(&s, &[(e, 9_000), (w, 21_000)]), "deterministic");
        // Phases north, east, south, west, 30 s each; east's crossing (9 m: 7 + 9 = 16 s) is walked in north's and
        // south's, west's (21 m: 7 + 21 = 28 s, 3 s more than the 25 s green) in the same two.
        let pinned = Summary {
            calls: vec![
                call(e, 455, Outcome::Served, 25, 0),
                call(w, 479, Outcome::Served, 1, 3),
                call(e, 1612, Outcome::Served, 11, 0),
                call(w, 1881, Outcome::Delayed, 42, 3),
                call(e, 3067, Outcome::Delayed, 59, 0),
            ],
            totals: vec![
                Totals {
                    arm: e,
                    rate: 3,
                    clearance_s: 9,
                    need_s: 16,
                    calls: 3,
                    served: 2,
                    delayed: 1,
                    not_served: 0,
                    cancelled: 0,
                    extended: 0,
                    walks_skipped: 0,
                    longest_wait_s: Some(59),
                },
                Totals {
                    arm: w,
                    rate: 2,
                    clearance_s: 21,
                    need_s: 28,
                    calls: 2,
                    served: 1,
                    delayed: 1,
                    not_served: 0,
                    cancelled: 0,
                    extended: 2,
                    walks_skipped: 0,
                    longest_wait_s: Some(42),
                },
            ],
            findings: vec![],
        };
        assert_eq!(sum, pinned, "{sum:#?}");
    }

    // ---- passive detection (CLM-17) -------------------------------------------------------------------------------------

    fn passive(arm: u32, need_s: u32, at_s: Vec<u32>) -> Asked {
        Asked { arm, need_s, at_s, passive: true }
    }

    #[test]
    fn the_waiting_limit_is_the_placeholder_the_issue_settled() {
        assert_eq!(WAIT_LIMIT_S, 30);
        assert_eq!(Detection::default(), Detection::PushButton, "a crossing on call has a push button until told otherwise");
    }

    #[test]
    fn a_passive_call_whose_walk_is_past_the_waiting_limit_is_cancelled_and_a_walk_with_no_one_left_is_skipped() {
        // Street 9's crossing is walked in the first phase, from 0, 60, 120 s.
        let plan = plan_of(vec![phase(1, 25, &[9]), phase(2, 25, &[])]);
        let at = vec![0, 10, 30, 40, 61, 75];
        let run = serve(&plan, &[passive(9, 16, at.clone())]);
        assert_eq!(
            run.calls,
            vec![
                call(9, 0, Outcome::Served, 0, 0),
                call(9, 10, Outcome::Cancelled, WAIT_LIMIT_S, 0), // its walk would have been at 60 s, 50 s on: it left at 40 s
                call(9, 30, Outcome::Served, 30, 0),              // exactly the limit: still there when the walk starts
                call(9, 40, Outcome::Served, 20, 0),
                call(9, 61, Outcome::Cancelled, WAIT_LIMIT_S, 0), // missed the walk at 60 s; the next, at 120 s, is 59 s on
                call(9, 75, Outcome::Cancelled, WAIT_LIMIT_S, 0),
            ]
        );
        // The walk at 60 s still had people waiting; the one at 120 s had nobody left: skipped.
        assert_eq!(run.skipped, vec![1]);
        // With a push button the same people are served or delayed: a call stays placed after the person leaves.
        let pushed = serve(&plan, &[Asked { arm: 9, need_s: 16, at_s: at, passive: false }]);
        assert_eq!(
            pushed.calls,
            vec![
                call(9, 0, Outcome::Served, 0, 0),
                call(9, 10, Outcome::Delayed, 50, 0),
                call(9, 30, Outcome::Served, 30, 0),
                call(9, 40, Outcome::Served, 20, 0),
                call(9, 61, Outcome::Delayed, 59, 0),
                call(9, 75, Outcome::Delayed, 45, 0),
            ]
        );
        assert_eq!(pushed.skipped, vec![0]);
    }

    #[test]
    fn a_skipped_walk_lengthens_no_green_and_walk_and_clearance_are_never_shortened() {
        // Street 9 needs 19 s and its phase has a 10 s green; street 8's crossing is walked in the second phase. The call
        // at 5 s missed the first walk (0 to 15 s); the next starts at 45 s, 40 s on.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 25, &[8])]);
        let pushed = serve(&plan, &[Asked { arm: 9, need_s: 19, at_s: vec![5], passive: false }, Asked { arm: 8, need_s: 10, at_s: vec![50], passive: false }]);
        // With a push button the walk is given at 45 s with its green lengthened by 9 s, so street 8's walk is at 69 s.
        assert_eq!(pushed.calls, vec![call(9, 5, Outcome::Delayed, 40, 9), call(8, 50, Outcome::Served, 19, 0)]);
        // Passive: the person left at 35 s, the walk at 45 s is skipped and its green kept at 10 s, so street 8's walk
        // comes at 60 s.
        let run = serve(&plan, &[passive(9, 19, vec![5]), Asked { arm: 8, need_s: 10, at_s: vec![50], passive: false }]);
        assert_eq!(run.calls, vec![call(9, 5, Outcome::Cancelled, WAIT_LIMIT_S, 0), call(8, 50, Outcome::Served, 10, 0)]);
        assert_eq!(run.skipped, vec![1, 0]);
        // Someone still waiting at that walk is given the whole walk and clearance, the green lengthened as before.
        let run = serve(&plan, &[passive(9, 19, vec![5, 30]), Asked { arm: 8, need_s: 10, at_s: vec![50], passive: false }]);
        assert_eq!(run.calls[..2], [call(9, 5, Outcome::Cancelled, WAIT_LIMIT_S, 0), call(9, 30, Outcome::Served, 15, 9)]);
        assert_eq!(run.skipped, vec![0, 0]);
    }

    #[test]
    fn a_passive_crossing_no_phase_can_serve_keeps_its_calls_not_served() {
        // No walk to leave before: the finding says why, as with a push button.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 25, &[])]);
        let run = serve(&plan, &[passive(9, 26, vec![0, 5])]);
        assert_eq!(run.calls, vec![call(9, 0, Outcome::NotServed, 0, 0), call(9, 5, Outcome::NotServed, 0, 0)]);
        assert_eq!(run.skipped, vec![0]);
    }

    /// `s` with the crossings of `uids` on call at `rate` an hour, each detected by `detection`.
    fn on_call_with(mut s: State, uids: &[u32], rate: u32, detection: Detection) -> State {
        for &uid in uids {
            let a = arm_mut(&mut s, uid);
            a.calls = Some(rate);
            a.detection = detection;
        }
        s
    }

    #[test]
    fn the_same_arrivals_with_push_buttons_and_passive_detection_differ_only_by_the_cancelled_calls_and_the_walks_they_skip() {
        let s = cross();
        let (n, e) = (at(&s, 0), at(&s, 90));
        // Both crossings 9 m: 7 s + 9 s = 16 s fits every 25 s green, so no green is lengthened and every phase runs as
        // planned whichever way the crossings detect people.
        let distances = [(n, 9_000), (e, 9_000)];
        let pushed = summary_of(&on_call_with(s.clone(), &[n, e], MAX_CALLS_PER_HOUR, Detection::PushButton), &distances);
        let sensed = summary_of(&on_call_with(s, &[n, e], MAX_CALLS_PER_HOUR, Detection::Passive), &distances);
        assert_eq!(pushed.findings, sensed.findings);
        assert_eq!(pushed.calls.len(), sensed.calls.len(), "the same arrivals");
        let left = |c: &Call| c.outcome != Outcome::NotServed && c.wait_s > WAIT_LIMIT_S;
        for (p, q) in pushed.calls.iter().zip(&sensed.calls) {
            if left(p) {
                assert_eq!(*q, Call { outcome: Outcome::Cancelled, wait_s: WAIT_LIMIT_S, extended_s: 0, ..*p }, "{p:?}");
            } else {
                assert_eq!(q, p, "a call whose walk came in time is served as with a push button");
            }
        }
        assert_eq!(pushed.totals.len(), 2);
        for (p, q) in pushed.totals.iter().zip(&sensed.totals) {
            let mine: Vec<&Call> = pushed.calls.iter().filter(|c| c.arm == p.arm).collect();
            let cancelled = |o: Outcome| mine.iter().filter(|c| c.outcome == o && left(c)).count() as u32;
            // The walks skipped: those at which every call the push button served had left.
            let mut walks: Vec<(u32, bool)> = Vec::new();
            for c in &mine {
                let walk = c.at_s + c.wait_s;
                match walks.iter_mut().find(|(w, _)| *w == walk) {
                    Some((_, all_left)) => *all_left &= left(c),
                    None => walks.push((walk, left(c))),
                }
            }
            let skipped = walks.iter().filter(|(_, all_left)| *all_left).count() as u32;
            let longest = mine.iter().filter(|c| !left(c) && c.outcome != Outcome::NotServed).map(|c| c.wait_s).max();
            let expected = Totals {
                served: p.served - cancelled(Outcome::Served),
                delayed: p.delayed - cancelled(Outcome::Delayed),
                cancelled: cancelled(Outcome::Served) + cancelled(Outcome::Delayed),
                walks_skipped: skipped,
                longest_wait_s: longest,
                ..*p
            };
            assert_eq!(*q, expected);
            assert!(q.cancelled > 0 && q.walks_skipped > 0 && q.served > 0, "the hour shows each: {q:?}");
            assert_eq!(p.cancelled + p.walks_skipped, 0, "a push button cancels nothing");
        }
    }

    #[test]
    fn fixed_inputs_give_a_fixed_summary_with_passive_detection() {
        let mut s = cross();
        let (e, w) = (at(&s, 90), at(&s, 270));
        arm_mut(&mut s, e).calls = Some(3);
        arm_mut(&mut s, e).detection = Detection::Passive;
        arm_mut(&mut s, w).calls = Some(2);
        let sum = summary_of(&s, &[(e, 9_000), (w, 21_000)]);
        assert_eq!(sum, summary_of(&s, &[(e, 9_000), (w, 21_000)]), "deterministic");
        // As `fixed_inputs_give_a_fixed_summary`, but east's call at 3067 s would have waited 59 s: it left after 30 s,
        // and nobody else waited for that walk.
        let pinned = Summary {
            calls: vec![
                call(e, 455, Outcome::Served, 25, 0),
                call(w, 479, Outcome::Served, 1, 3),
                call(e, 1612, Outcome::Served, 11, 0),
                call(w, 1881, Outcome::Delayed, 42, 3),
                call(e, 3067, Outcome::Cancelled, 30, 0),
            ],
            totals: vec![
                Totals {
                    arm: e,
                    rate: 3,
                    clearance_s: 9,
                    need_s: 16,
                    calls: 3,
                    served: 2,
                    delayed: 0,
                    not_served: 0,
                    cancelled: 1,
                    extended: 0,
                    walks_skipped: 1,
                    longest_wait_s: Some(25),
                },
                Totals {
                    arm: w,
                    rate: 2,
                    clearance_s: 21,
                    need_s: 28,
                    calls: 2,
                    served: 1,
                    delayed: 1,
                    not_served: 0,
                    cancelled: 0,
                    extended: 2,
                    walks_skipped: 0,
                    longest_wait_s: Some(42),
                },
            ],
            findings: vec![],
        };
        assert_eq!(sum, pinned, "{sum:#?}");
    }

    #[test]
    fn passive_detection_is_set_only_on_a_crossing_on_call_that_a_phase_walks_and_is_undone_like_any_edit() {
        let mut j = Junction::new(0);
        let (n, so, e) = (at(j.current(), 0), at(j.current(), 180), at(j.current(), 90));
        // On recall: nothing to detect.
        assert!(!j.set_detection(e, Detection::Passive));
        assert_eq!(j.refusal(), Some(Refusal::DetectionNeedsCall));
        assert!(!j.set_detection(e, Detection::PushButton));
        assert_eq!(j.refusal(), Some(Refusal::DetectionNeedsCall));
        assert!(j.set_on_call(e, true));
        assert_eq!(detection_of(j.arm(e).unwrap()), Detection::PushButton);
        assert!(j.set_detection(e, Detection::Passive));
        assert_eq!((j.arm(e).unwrap().detection, j.current().label.key), (Detection::Passive, "jn-rev-call-passive"));
        let edits = j.revisions().count();
        assert!(j.set_detection(e, Detection::Passive), "the detection it has: taken, as a radio chosen again");
        assert_eq!(j.revisions().count(), edits, "and no edit");
        assert!(j.set_detection(e, Detection::PushButton));
        assert_eq!((j.arm(e).unwrap().detection, j.current().label.key), (Detection::PushButton, "jn-rev-call-push-button"));
        assert!(j.undo() && j.arm(e).unwrap().detection == Detection::Passive);
        // Back on recall, the crossing has no detection; undone, it has its own again.
        assert!(j.set_on_call(e, false));
        assert_eq!(j.arm(e).unwrap().detection, Detection::PushButton);
        assert!(j.undo() && j.arm(e).unwrap().detection == Detection::Passive);
        // A plan whose only phase walks east alone: north is walked by no phase, so passive is refused with why.
        let walk_east = StoredPhase { moves: vec![signal::Move { from: n, to: so }], walks: vec![e], rides: vec![], arm: None };
        j.set_plan_for_test(SignalPlan { phases: vec![walk_east] });
        assert!(!j.set_detection(n, Detection::Passive));
        assert_eq!(j.refusal(), Some(Refusal::DetectionNotWalked));
        // East left on call with its walk taken away: kept on call, and passive is refused with why.
        assert!(j.set_detection(e, Detection::PushButton));
        assert!(j.set_phase_walk(0, e, false));
        assert_eq!(j.arm(e).unwrap().calls, Some(DEFAULT_CALLS_PER_HOUR));
        assert!(!j.set_detection(e, Detection::Passive));
        assert_eq!(j.refusal(), Some(Refusal::DetectionNotWalked));
        assert_eq!(j.arm(e).unwrap().detection, Detection::PushButton);
        // Off a signal the calls go, and the detection with them.
        let mut j = Junction::new(0);
        assert!(j.set_on_call(e, true) && j.set_detection(e, Detection::Passive));
        assert!(j.set_control(PRIORITY));
        assert_eq!((j.arm(e).unwrap().calls, j.arm(e).unwrap().detection), (None, Detection::PushButton));
        assert!(!j.set_detection(e, Detection::Passive));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
    }

    #[test]
    fn a_junction_saved_with_push_buttons_loads_unchanged_and_passive_detection_is_written_only_once_chosen() {
        let mut s = Junction::new(0).current().clone();
        s.arms[0].calls = Some(12);
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("detection"), "a crossing on call with a push button is written as #60 wrote it: {text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.arms[0].detection, Detection::PushButton);
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        s.arms[0].detection = Detection::Passive;
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""detection":"passive""#), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.arms[0].detection, Detection::Passive);
        // A document holding a detection on a crossing on recall opens with none: only a crossing on call has one.
        let mut stray = Junction::new(0).current().clone();
        stray.arms[0].detection = Detection::Passive;
        let opened = Junction::from_city(crate::street::model::named_said("J"), &stray, &stray, 0).unwrap();
        assert!(opened.current().arms.iter().all(|a| a.detection == Detection::PushButton));
    }

    // ---- the edits ----------------------------------------------------------------------------------------------------

    #[test]
    fn on_call_is_set_and_taken_off_and_the_rate_is_kept_within_its_bound() {
        let mut j = Junction::new(0);
        let e = at(j.current(), 90);
        assert!(j.set_on_call(e, true));
        assert_eq!(j.arm(e).unwrap().calls, Some(DEFAULT_CALLS_PER_HOUR));
        assert_eq!(j.current().label.key, "jn-rev-call-on");
        assert!(!j.set_on_call(e, true));
        assert_eq!(j.refusal(), Some(Refusal::CallAlready));
        assert!(j.set_call_rate(e, 30));
        assert_eq!((j.arm(e).unwrap().calls, j.current().label.key), (Some(30), "jn-rev-call-rate"));
        for bad in [-1, MAX_CALLS_PER_HOUR as i32 + 1] {
            assert!(!j.set_call_rate(e, bad));
            assert_eq!(j.refusal(), Some(Refusal::CallRateOutOfRange));
        }
        assert!(j.set_call_rate(e, MAX_CALLS_PER_HOUR as i32) && j.set_call_rate(e, 0), "both ends of the bound are allowed");
        assert!(j.set_on_call(e, false));
        assert_eq!((j.arm(e).unwrap().calls, j.current().label.key), (None, "jn-rev-call-off"));
        assert!(!j.set_call_rate(e, 5));
        assert_eq!(j.refusal(), Some(Refusal::NotOnCall));
        assert!(j.undo() && j.arm(e).unwrap().calls == Some(0), "an edit of the history like any other");
    }

    #[test]
    fn on_call_is_refused_with_its_reason_where_no_phase_walks_the_crossing() {
        let mut j = Junction::new(0);
        let (n, so, e) = (at(j.current(), 0), at(j.current(), 180), at(j.current(), 90));
        // A plan whose only phase walks east alone.
        let walk_east = StoredPhase { moves: vec![signal::Move { from: n, to: so }], walks: vec![e], rides: vec![], arm: None };
        j.set_plan_for_test(SignalPlan { phases: vec![walk_east] });
        assert!(!j.set_on_call(n, true));
        assert_eq!(j.refusal(), Some(Refusal::CallNotWalked));
        assert!(j.arm(n).unwrap().calls.is_none());
        assert!(j.set_on_call(e, true));
        assert!(j.set_control(PRIORITY));
        assert!(!j.set_on_call(n, true));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
    }

    #[test]
    fn an_edit_that_takes_the_walk_away_keeps_the_crossing_on_call_as_a_finding() {
        let mut j = Junction::new(0);
        let (n, e) = (at(j.current(), 0), at(j.current(), 90));
        assert!(j.set_on_call(e, true));
        let phases = signal::plan(j.current(), 0).unwrap().phases;
        for (i, p) in phases.iter().enumerate() {
            if p.walks.contains(&e) {
                assert!(j.set_phase_walk(i, e, false));
            }
        }
        assert_eq!(j.arm(e).unwrap().calls, Some(DEFAULT_CALLS_PER_HOUR), "kept, never silently dropped");
        let sum = summary(j.current(), &signal::plan(j.current(), 0).unwrap(), &crate::junction::model::read_model::crossing_distances(&j.view().arms));
        assert_eq!(sum.findings, vec![Finding::NotWalked { arm: e }]);
        assert!(j.set_on_call(e, false), "recall is always allowed: the finding can be cleared");
        assert!(!j.set_on_call(n, false) && j.refusal() == Some(Refusal::CallAlready));
    }

    #[test]
    fn a_junction_saved_before_calls_loads_unchanged_and_one_with_none_on_call_is_written_as_before() {
        let s = Junction::new(0).current().clone();
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("calls"), "nothing new is written while no crossing is on call");
        let back: State = serde_json::from_str(&text).unwrap();
        assert!(back.arms.iter().all(|a| a.calls.is_none()));
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        let mut on = s.clone();
        on.arms[0].calls = Some(20);
        let text = serde_json::to_string(&on).unwrap();
        assert!(text.contains(r#""calls":20"#), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.arms[0].calls, Some(20));
    }
}
