//! Emergency vehicle pre-emption (CLM-15): a street with a phase of its own has pre-emption off (the default) or on. An
//! emergency call on a street with it on takes the signal to the street's phase, after a safe clearance of the phase
//! running, above every other policy (pedestrian and bicycle calls, vehicle actuation, transit priority). A seeded
//! generator spreads each street's emergency calls over one hour, and the controller (`control::run`) serves them with
//! every other demand. Deterministic: the same junction, plan and rates always give the same hour. CityLoom models the
//! behaviour to compare strategies; it never controls a signal or predicts real-world delay or safety. No DOM, no
//! signals, no I/O, no wording.
//!
//! The rule of an emergency call (a street's calls are served by the phase that is the street's own,
//! `actuation::own_phase`):
//! - Entry: a call that comes while another phase's green runs ends that green, but never before a clearance in
//!   progress is done. A walk still showing is cut short and its pedestrian clearance then runs in full; a clearance
//!   under way runs out; the same for a bike movement's minimum green and its bicycle clearance. Those served on call
//!   and those on recall alike. Then the phase's yellow and all-red run in full: they are never shortened. A call that
//!   comes during a yellow or an all-red lets it run out. The street's phase then runs out of turn, at once.
//! - The street's phase already green: it stays green (a time to green of 0).
//! - Dwell: the street's green runs at least `DWELL_S` from when it turns green for the call (or from the call, when it
//!   came while the green ran), else as the plan gives it; a further call of the same street during it holds it
//!   `DWELL_S` past that call. The vehicle is taken to have passed then: placeholder, no vehicle is tracked.
//! - Return: after the emergency green (its yellow and all-red in full), the plan goes on with the phase after the one
//!   that was interrupted (the one whose green was cut short or ran its yellow or all-red when the call came), not back
//!   to it. When the street's phase is the next anyway, it is run once and the plan goes on after it.
//! - Several calls: served in the order they come. A call that comes while another street's pre-emption runs (from that
//!   call until the end of its green) waits for it to end, and then takes the signal; the phase displaced by the first
//!   still runs after them.
//! - Above every policy: an interrupted green is cut whatever transit priority, an actuated phase or a call holds for
//!   it, but never through a clearance. Each call counts, by kind of road user (pedestrian, cyclist, bus, vehicle):
//!   those pre-empted (waiting for the interrupted phase, its green cut short under them) and those delayed (waiting
//!   for any other phase when the call came, theirs put back by the emergency green), when the call changed anything
//!   (it interrupted a green, ran its phase out of turn or held a green longer).
//!
//! The safe entry (`entry_s`): the longest a call can take to reach its green when no other pre-emption is running: the
//! longest clearance, yellow and all-red of the phases it can interrupt, and its own phase's transit lead.
//!
//! Every figure here is a placeholder (synthetic, like the catalogue's rules and rates): the dwell, the default and the
//! bound of the calls an hour.

use serde::Serialize;

use crate::junction::model::actuation::own_phase;
use crate::junction::model::calls::clear_at;
use crate::junction::model::control::{self, Kind, Turn};
use crate::junction::model::signal::Plan;
use crate::junction::model::{Arm, Refusal, SIGNAL, State, bicycles, calls, in_force};

/// The least time an emergency green runs for its vehicle, in seconds. Placeholder.
pub const DWELL_S: u32 = 20;
/// The emergency calls an hour a street is given when its pre-emption is turned on. Placeholder.
pub const DEFAULT_CALLS_PER_HOUR: u32 = 2;
/// The most emergency calls an hour a street may be given. Placeholder.
pub const MAX_CALLS_PER_HOUR: u32 = 12;

/// The emergency calls an hour on arm `a`'s street, if its pre-emption is on, never past `MAX_CALLS_PER_HOUR`: a document
/// from a sync or written by hand can hold any number. Read it here, never from `Arm.preemption`.
pub fn rate_of(a: &Arm) -> Option<u32> {
    a.preemption.map(|r| r.min(MAX_CALLS_PER_HOUR))
}

/// Why street `uid` cannot be given pre-emption, if it cannot: the junction has to be a signal, and the plan has to give
/// the street a phase of its own.
pub fn refusal(s: &State, region: usize, uid: u32) -> Option<Refusal> {
    if !s.arms.iter().any(|a| a.uid == uid) {
        return Some(Refusal::DoesNotFit);
    }
    let s = in_force(s);
    if s.control != SIGNAL {
        return Some(Refusal::NotSignal);
    }
    let Some(plan) = crate::junction::model::signal::plan(&s, region) else { return Some(Refusal::NotSignal) };
    own_phase(&plan, uid).is_none().then_some(Refusal::PreemptionNoPhase)
}

/// The seconds into the hour at which emergency calls come on the approach of street `uid`, `rate` an hour: the hour is
/// cut into `rate` equal slots, one call in each, at a point the generator picks. Seeded from the street and the rate on
/// a stream of its own (`EMERGENCY_STREAM`), so it never repeats another demand of the same street. In order, each
/// second at most once. A rate past `MAX_CALLS_PER_HOUR` is read as that.
pub fn arrivals(uid: u32, rate: u32) -> Vec<u32> {
    let rate = rate.min(MAX_CALLS_PER_HOUR);
    control::spread(((u64::from(uid) << 32) | u64::from(rate)) ^ EMERGENCY_STREAM, rate)
}

/// What sets the emergency calls' seeds apart from the other demands' ("PREEMPT" in ASCII).
const EMERGENCY_STREAM: u64 = 0x0050_5245_454D_5054;

/// The longest an emergency call on the street whose own phase is `phase` (by its place in `plan`) takes to reach its
/// green, in seconds, when no other pre-emption runs: over every other phase it can interrupt, the longest clearance it
/// may have under way (a pedestrian clearance of a crossing it walks, `distances`; a bicycle clearance of a movement it
/// runs, `rides`) and then its yellow and all-red; then the phase's own transit lead.
pub fn entry_s(plan: &Plan, phase: usize, distances: &[(u32, i32)], rides: &[(u32, i32)]) -> u32 {
    let lead = plan.phases.get(phase).map_or(0, |p| p.timing.transit_lead_s);
    let worst = plan
        .phases
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != phase)
        .map(|(_, p)| {
            let walks = p.walks.iter().filter_map(|&w| length_of(distances, w)).map(calls::clearance_s);
            let ride = p.rides.iter().filter_map(|&r| length_of(rides, r)).map(bicycles::clearance_s);
            walks.chain(ride).max().unwrap_or(0) + p.timing.yellow_s + p.timing.all_red_s
        })
        .max()
        .unwrap_or(0);
    worst + lead
}

/// The length `list` gives street `uid` (a crossing's, `distances`; a ride's, `rides`), if it gives one: how the safe
/// entry (`entry_s`) and what is cleared on an interruption (`recall`) both read them.
fn length_of(list: &[(u32, i32)], uid: u32) -> Option<i32> {
    list.iter().find(|(u, _)| *u == uid).map(|(_, d)| *d)
}

/// How many road users of each kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct Counts {
    pub pedestrians: u32,
    pub cyclists: u32,
    pub buses: u32,
    pub vehicles: u32,
}

impl Counts {
    /// How many of `kind`.
    pub fn of(&self, kind: Kind) -> u32 {
        match kind {
            Kind::Pedestrian => self.pedestrians,
            Kind::Bicycle => self.cyclists,
            Kind::Bus => self.buses,
            Kind::Vehicle => self.vehicles,
        }
    }

    fn set(&mut self, kind: Kind, n: u32) {
        match kind {
            Kind::Pedestrian => self.pedestrians = n,
            Kind::Bicycle => self.cyclists = n,
            Kind::Bus => self.buses = n,
            Kind::Vehicle => self.vehicles = n,
        }
    }

    /// Both added up.
    pub fn plus(self, other: Counts) -> Counts {
        let mut out = self;
        for kind in Kind::ALL {
            out.set(kind, self.of(kind).saturating_add(other.of(kind)));
        }
        out
    }

    /// Whether there is nobody.
    pub fn is_empty(&self) -> bool {
        Kind::ALL.iter().all(|&k| self.of(k) == 0)
    }
}

/// What happened to one emergency call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Outcome {
    /// Its street's green came for it.
    Served,
    /// Still waiting when the run stopped.
    NotServed,
}

/// One emergency call: its street, when it came (seconds into the hour), what happened, how long it took its street's
/// green to come (0: it was green), the street whose phase's green it cut short (if it did), and who it pre-empted and
/// delayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Call {
    pub arm: u32,
    pub at_s: u32,
    pub outcome: Outcome,
    pub to_green_s: u32,
    pub interrupted: Option<u32>,
    pub preempted: Counts,
    pub delayed: Counts,
}

/// The emergency calls of one street with pre-emption, added up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Totals {
    pub arm: u32,
    /// Its calls an hour.
    pub rate: u32,
    /// The longest a call takes to reach its green when no other pre-emption runs (`entry_s`).
    pub entry_s: u32,
    pub calls: u32,
    pub served: u32,
    pub not_served: u32,
    /// The longest a served call took to reach its green; None when none was served.
    pub longest_to_green_s: Option<u32>,
    pub preempted: Counts,
    pub delayed: Counts,
}

/// Something about a street with pre-emption the person should act on. By street uid; no wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Finding {
    /// The street has pre-emption on, but no phase of its own any more (an edit took it away): it is kept, and its
    /// calls take the signal nowhere until it has one again.
    NoPhase { arm: u32 },
}

impl Finding {
    /// What the finding is about, whatever its figures: its kind and its street.
    pub fn about(&self) -> (u8, u32) {
        match *self {
            Finding::NoPhase { arm } => (40, arm),
        }
    }
}

/// How the plan served the emergency calls of one hour.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Summary {
    /// Every call, in the order they came.
    pub calls: Vec<Call>,
    /// One for each street with pre-emption on and a phase of its own, in the order of the bearings.
    pub totals: Vec<Totals>,
    pub findings: Vec<Finding>,
}

/// The emergency calls of one street with pre-emption: its street, its own phase (by its place in the plan) and when
/// each call comes.
pub(crate) struct Street {
    pub(crate) arm: u32,
    pub(crate) phase: usize,
    pub(crate) at_s: Vec<u32>,
}

/// The streets of `s` with pre-emption on (in the order of the bearings) that have a phase of their own, each with its
/// calls, with `out` given a row of totals for each (its safe entry from `distances` and `rides`) and a finding for each
/// left out.
pub(crate) fn streets(s: &State, plan: &Plan, distances: &[(u32, i32)], rides: &[(u32, i32)], out: &mut Summary) -> Vec<Street> {
    let mut arms: Vec<&Arm> = s.arms.iter().collect();
    arms.sort_by_key(|a| a.bearing);
    let mut streets = Vec::new();
    for (arm, rate) in arms.into_iter().filter_map(|a| rate_of(a).map(|r| (a, r))) {
        let Some(phase) = own_phase(plan, arm.uid) else {
            out.findings.push(Finding::NoPhase { arm: arm.uid });
            continue;
        };
        out.totals.push(Totals {
            arm: arm.uid,
            rate,
            entry_s: entry_s(plan, phase, distances, rides),
            calls: 0,
            served: 0,
            not_served: 0,
            longest_to_green_s: None,
            preempted: Counts::default(),
            delayed: Counts::default(),
        });
        streets.push(Street { arm: arm.uid, phase, at_s: arrivals(arm.uid, rate) });
    }
    streets
}

/// What each phase of `plan` runs every time it is green whatever is called, and so has under way when interrupted: the
/// walk and clearance of each crossing on recall it walks (`distances`), and the bicycle minimum green and clearance of
/// each bike movement on recall it runs (`rides`), each as (the part that may end early, all it needs).
pub(crate) fn recall(s: &State, plan: &Plan, distances: &[(u32, i32)], rides: &[(u32, i32)]) -> Vec<Vec<(u32, u32)>> {
    let arm = |uid: u32| s.arms.iter().find(|a| a.uid == uid);
    plan.phases
        .iter()
        .map(|p| {
            let walks = p
                .walks
                .iter()
                .filter(|&&w| arm(w).is_some_and(|a| a.calls.is_none() && a.crossing.is_some()))
                .filter_map(|&w| length_of(distances, w))
                .map(|d| (calls::WALK_S, calls::need_s(d)));
            let ridden = p
                .rides
                .iter()
                .filter(|&&r| arm(r).is_some_and(|a| a.cyclists.is_none()))
                .filter_map(|&r| length_of(rides, r))
                .map(|d| (bicycles::MIN_GREEN_S, bicycles::need_s(d)));
            walks.chain(ridden).collect()
        })
        .collect()
}

/// The emergency calls as a demand on the controller, above every other: which call comes next, the signal it takes,
/// and what happened to every call.
pub(crate) struct Emergencies<'a> {
    streets: &'a [Street],
    /// Every call as (its street's place, when), in the order they come, and the first not yet served.
    queue: Vec<(usize, u64)>,
    next: usize,
    rows: Vec<Call>,
    /// Whether each call changed anything (interrupted a green, ran its phase out of turn, held a green longer), and
    /// the calls whose delayed road users are counted.
    changed: Vec<bool>,
    counted: usize,
    /// Each phase's street, transit lead and what it runs on recall (`recall`); the phase the plan runs next.
    uids: Vec<u32>,
    leads: Vec<u64>,
    recall: Vec<Vec<(u32, u32)>>,
    expected: usize,
}

impl<'a> Emergencies<'a> {
    /// `recall` what each phase runs whatever is called (`recall`).
    pub(crate) fn new(plan: &Plan, streets: &'a [Street], recall: Vec<Vec<(u32, u32)>>) -> Emergencies<'a> {
        let mut queue: Vec<(usize, u64)> = streets.iter().enumerate().flat_map(|(x, st)| st.at_s.iter().map(move |&t| (x, u64::from(t)))).collect();
        queue.sort_by_key(|&(x, t)| (t, x));
        let rows = queue
            .iter()
            .map(|&(x, t)| Call {
                arm: streets[x].arm,
                at_s: u32::try_from(t).unwrap_or(u32::MAX),
                outcome: Outcome::NotServed,
                to_green_s: 0,
                interrupted: None,
                preempted: Counts::default(),
                delayed: Counts::default(),
            })
            .collect();
        Emergencies {
            streets,
            changed: vec![false; queue.len()],
            queue,
            next: 0,
            rows,
            counted: 0,
            uids: plan.phases.iter().map(|p| p.uid).collect(),
            leads: plan.phases.iter().map(|p| u64::from(p.timing.transit_lead_s)).collect(),
            recall,
            expected: 0,
        }
    }

    /// The first call not yet served, as (the phase it takes the signal to, when it came).
    fn head(&self) -> Option<(usize, u64)> {
        self.queue.get(self.next).map(|&(x, t)| (self.streets[x].phase, t))
    }

    /// Where the green of phase `k`, from `green_start` and so far ending at `end`, is to end for the calls of its own
    /// street, from the first not yet served, that come while it runs: each holds it `DWELL_S` past its green (or past
    /// the call, when it came while the green ran). A call of another street stops the reckoning: it waits.
    fn dwell_end(&self, k: usize, green_start: u64, mut end: u64) -> u64 {
        for &(x, t) in &self.queue[self.next..] {
            if self.streets[x].phase != k || t >= end {
                break;
            }
            end = end.max(t.max(green_start) + u64::from(DWELL_S));
        }
        end
    }

    /// Every call, what happened to it; who it delayed only when it changed anything.
    pub(crate) fn finish(self) -> Vec<Call> {
        self.rows.into_iter().zip(self.changed).map(|(c, changed)| if changed { c } else { Call { delayed: Counts::default(), ..c } }).collect()
    }
}

impl control::Demand for Emergencies<'_> {
    /// The first call waiting calls its street's phase.
    fn calls(&self, k: usize, at: u64) -> bool {
        self.head().is_some_and(|(phase, t)| phase == k && t <= at)
    }

    /// The first call waiting since the phase started holds its green `DWELL_S`.
    fn hold(&self, k: usize, start: u64) -> u32 {
        if self.calls(k, start) { DWELL_S } else { 0 }
    }

    /// The first call waiting takes the signal to its street's phase.
    fn jump(&self, planned: usize, at: u64) -> Option<usize> {
        self.head().filter(|&(phase, t)| phase != planned && t <= at).map(|(phase, _)| phase)
    }

    /// The first call not yet served, of another street, coming before the green ends, ends it.
    fn interrupt(&self, k: usize, _start: u64, green_start: u64, green_end: u64) -> Option<u64> {
        self.head().filter(|&(phase, t)| phase != k && t < green_end).map(|(_, t)| t.max(green_start))
    }

    /// What the phase runs on recall runs every green from its start: a walk or bicycle minimum green still showing
    /// ends, its clearance runs out.
    fn clear_by(&self, k: usize, _start: u64, green_start: u64, at: u64) -> u64 {
        self.recall[k].iter().map(|&(first, need)| clear_at(green_start, first, need, at)).max().unwrap_or(at)
    }

    /// The green of the street of the first call not yet served, waiting or coming before it ends, runs for that call:
    /// no lower policy cuts it.
    fn keeps(&self, k: usize, _start: u64, _green_start: u64, green_end: u64) -> bool {
        self.head().is_some_and(|(phase, t)| phase == k && t < green_end)
    }

    /// The calls of the phase's own street coming while its green runs hold it to their dwell.
    fn prolong(&self, k: usize, start: u64, green_end: u64) -> u32 {
        let green_start = start + self.leads[k];
        u32::try_from(self.dwell_end(k, green_start, green_end) - green_end).unwrap_or(u32::MAX)
    }

    /// Who waited at each call's second: before the turn is told, the road users an interrupted green had (pre-empted)
    /// and, for a call that came by the turn's start (in a rest gap, say), who waited for any other phase (delayed),
    /// before the turn serves anyone; after it is told, the same for a call that came while the turn ran, those it served
    /// before the call gone.
    fn census(&mut self, turn: &Turn, told: bool, others: &dyn Fn(Kind, u64, &[usize]) -> u32) {
        if !told
            && turn.preempted_s > 0
            && let Some(&(_, t)) = self.queue.get(self.next)
        {
            let row = &mut self.rows[self.next];
            row.interrupted = Some(self.uids[turn.phase]);
            for kind in Kind::ALL {
                row.preempted.set(kind, others(kind, t, &[turn.phase]));
            }
        }
        let by = if told { turn.end } else { turn.start + 1 };
        let all: Vec<usize> = (0..self.uids.len()).collect();
        while let Some(&(x, t)) = self.queue.get(self.counted).filter(|&&(_, t)| t < by) {
            let own = [self.streets[x].phase];
            let row = &mut self.rows[self.counted];
            for kind in Kind::ALL {
                row.delayed.set(kind, others(kind, t, &all).saturating_sub(others(kind, t, &own)));
            }
            self.counted += 1;
        }
    }

    fn counts(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Follows the plan's order (a phase run out of turn leaves it where it was), and serves the calls of the street
    /// whose phase it is that came before its green ended, in order.
    fn ran(&mut self, turn: &Turn) {
        let out_of_turn = turn.phase != self.expected;
        if !out_of_turn {
            self.expected = (self.expected + 1) % self.uids.len().max(1);
        }
        if turn.skipped {
            return;
        }
        // Whether the calls served here held the green past what its rule gave.
        let base_end = turn.green_start + u64::from(turn.base_s);
        let lengthened = self.dwell_end(turn.phase, turn.green_start, base_end) > base_end;
        while let Some((_, t)) = self.head().filter(|&(phase, t)| phase == turn.phase && t < turn.green_end) {
            let i = self.next;
            let row = &mut self.rows[i];
            row.outcome = Outcome::Served;
            row.to_green_s = u32::try_from(turn.green_start.saturating_sub(t)).unwrap_or(u32::MAX);
            self.changed[i] = row.interrupted.is_some() || out_of_turn || lengthened;
            self.next += 1;
        }
    }

    fn pending(&self) -> bool {
        self.next < self.queue.len()
    }

    fn next_after(&self, at: u64) -> Option<u64> {
        self.queue[self.next..].iter().map(|&(_, t)| t).find(|&t| t > at)
    }

    fn last(&self) -> u64 {
        self.queue.last().map_or(0, |&(_, t)| t)
    }

    fn longest_hold(&self, k: usize) -> u32 {
        if self.streets.iter().any(|st| st.phase == k) { DWELL_S } else { 0 }
    }

    fn most_prolong(&self) -> u32 {
        if self.queue.is_empty() { 0 } else { DWELL_S }
    }

    fn out_of_turn(&self) -> u32 {
        u32::from(!self.queue.is_empty())
    }
}

/// `out`'s calls and totals from what the run did.
pub(crate) fn tally(out: &mut Summary, calls: Vec<Call>) {
    out.calls = calls;
    for t in &mut out.totals {
        for c in out.calls.iter().filter(|c| c.arm == t.arm) {
            t.calls += 1;
            match c.outcome {
                Outcome::Served => {
                    t.served += 1;
                    t.longest_to_green_s = Some(t.longest_to_green_s.unwrap_or(0).max(c.to_green_s));
                }
                Outcome::NotServed => t.not_served += 1,
            }
            t.preempted = t.preempted.plus(c.preempted);
            t.delayed = t.delayed.plus(c.delayed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::calls::{Asked, Walks};
    use crate::junction::model::control::{Demand, Rule, run};
    use crate::junction::model::signal::{self, Move, Phase, SignalPlan, StoredPhase};
    use crate::junction::model::{PRIORITY, Timing, actuation, junction_biked, transit};

    fn at(s: &State, bearing: i32) -> u32 {
        s.arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    fn arm_mut(s: &mut State, uid: u32) -> &mut Arm {
        s.arms.iter_mut().find(|a| a.uid == uid).unwrap()
    }

    /// A phase of street `uid` with a `green_s` green, yellow 3 s and all-red 2 s, walking `walks` and running the bike
    /// movements of `rides`.
    fn phase(uid: u32, green_s: u32, walks: &[u32], rides: &[u32]) -> Phase {
        let timing = Timing { green_s, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        Phase { uid, timing, length_s: timing.length_s(), walks: walks.to_vec(), rides: rides.to_vec(), moves: vec![Move { from: uid, to: 99 }] }
    }

    fn plan_of(phases: Vec<Phase>) -> Plan {
        let cycle_s = phases.iter().map(|p| p.length_s).sum();
        Plan { phases, cycle_s, suggested: false }
    }

    /// Three phases of 25 s green, 30 s each: streets 1, 2 and 3 in turn; street 1's walks crossing 8 and runs street
    /// 1's bike movement, street 2's walks crossing 9.
    fn three() -> Plan {
        plan_of(vec![phase(1, 25, &[8], &[1]), phase(2, 25, &[9], &[]), phase(3, 25, &[], &[])])
    }

    fn street(arm: u32, phase: usize, at_s: Vec<u32>) -> Street {
        Street { arm, phase, at_s }
    }

    /// A pedestrian call at crossing `arm` needing `need_s`, made at each of `at_s`.
    fn walk(arm: u32, need_s: u32, at_s: Vec<u32>) -> Asked {
        Asked { arm, need_s, at_s, passive: false }
    }

    /// The emergency calls of `streets` served by `plan` (every phase fixed), each phase running `recall` whatever is
    /// called, with the pedestrian calls `walks` and the bicycle calls `bikes`: the turns that ran as (phase, start,
    /// green, end), and every emergency call.
    fn serve(plan: &Plan, streets: &[Street], recall: Vec<Vec<(u32, u32)>>, walks: &[Asked], bikes: &[Asked]) -> (Vec<(usize, u64, u32, u64)>, Vec<Call>) {
        let rules = control::fixed(plan);
        let mut walking = Walks::new(plan, &rules, walks);
        let mut riding = bicycles::riders(plan, &rules, bikes);
        let mut emergencies = Emergencies::new(plan, streets, recall);
        let turns = run(plan, &rules, &mut [&mut walking as &mut dyn Demand, &mut riding, &mut emergencies], 100_000);
        (turns.iter().filter(|t| !t.skipped).map(|t| (t.phase, t.start, t.green_s(), t.end)).collect(), emergencies.finish())
    }

    fn none() -> Vec<Vec<(u32, u32)>> {
        vec![vec![]; 3]
    }

    /// A pedestrian call far off, so the signal keeps running after the emergency calls.
    fn later() -> Vec<Asked> {
        vec![walk(8, 10, vec![400])]
    }

    fn call(arm: u32, at_s: u32, to_green_s: u32, interrupted: Option<u32>) -> Call {
        Call { arm, at_s, outcome: Outcome::Served, to_green_s, interrupted, preempted: Counts::default(), delayed: Counts::default() }
    }

    #[test]
    fn the_placeholders_are_the_ones_chosen() {
        assert_eq!((DWELL_S, DEFAULT_CALLS_PER_HOUR, MAX_CALLS_PER_HOUR), (20, 2, 12));
    }

    #[test]
    fn the_emergency_calls_of_an_hour_are_the_same_every_time_spread_over_the_hour_and_unlike_the_street_s_other_demands() {
        let calls = arrivals(7, 12);
        assert_eq!(calls, arrivals(7, 12), "seeded");
        assert_eq!(calls.len(), 12);
        for (k, t) in calls.iter().enumerate() {
            assert!((k as u32 * 300..(k as u32 + 1) * 300).contains(t), "one in each slot of 300 s: {calls:?}");
        }
        assert_ne!(arrivals(7, 12), calls::arrivals(7, 12), "a stream of its own, not the pedestrian calls'");
        assert_ne!(arrivals(7, 12), actuation::arrivals(7, 12), "nor the vehicles'");
        assert_ne!(arrivals(7, 12), bicycles::arrivals(7, 12), "nor the cyclists'");
        assert_ne!(arrivals(7, 12), transit::arrivals(7, 12), "nor the buses'");
        assert_eq!(arrivals(7, u32::MAX).len(), 12, "read as the bound");
        assert!(arrivals(7, 0).is_empty());
        // Pinned: a change of generator or seed changes every summary, so it has to be a decision.
        assert_eq!(arrivals(1, 2), PINNED_CALLS);
    }

    const PINNED_CALLS: [u32; 2] = [686, 2626];

    #[test]
    fn a_call_ends_the_green_running_keeps_its_yellow_and_all_red_runs_its_phase_out_of_turn_and_the_plan_goes_on_after_the_one_interrupted() {
        // Street 3's call at 10 s, during street 1's green: that green ends at 10 s, its 3 s yellow and 2 s all-red run,
        // street 3's phase runs at once (from 15 s), and then street 2's, the phase after the one interrupted.
        let plan = three();
        let (ran, calls) = serve(&plan, &[street(3, 2, vec![10])], none(), &later(), &[]);
        assert_eq!(&ran[..4], &[(0, 0, 10, 15), (2, 15, 25, 45), (1, 45, 25, 75), (2, 75, 25, 105)]);
        assert_eq!(calls, vec![call(3, 10, 5, Some(1))], "5 s to green: the yellow and the all-red");
        // During the yellow or the all-red: they run out, nothing is cut, and the phase after it is displaced.
        let (ran, calls) = serve(&plan, &[street(3, 2, vec![27])], none(), &later(), &[]);
        assert_eq!(&ran[..3], &[(0, 0, 25, 30), (2, 30, 25, 60), (1, 60, 25, 90)]);
        assert_eq!(calls, vec![call(3, 27, 3, None)]);
        // Its phase the next anyway: run once, and the plan goes on after it.
        let (ran, calls) = serve(&plan, &[street(2, 1, vec![27])], none(), &later(), &[]);
        assert_eq!(&ran[..3], &[(0, 0, 25, 30), (1, 30, 25, 60), (2, 60, 25, 90)]);
        assert_eq!(calls, vec![call(2, 27, 3, None)]);
    }

    #[test]
    fn a_call_never_shortens_a_clearance_in_progress_nor_the_yellow_and_all_red() {
        let plan = three();
        // Street 1's phase walks crossing 8 on recall: a 7 s walk and a 9 s clearance from its green's start.
        let recall = || vec![vec![(calls::WALK_S, 16)], vec![], vec![]];
        // During the walk (3 s): the walk ends, the clearance runs in full, to 12 s.
        let (ran, _) = serve(&plan, &[street(3, 2, vec![3])], recall(), &later(), &[]);
        assert_eq!(ran[0], (0, 0, 12, 17));
        // During the clearance (10 s): it runs out, to 16 s.
        let (ran, calls) = serve(&plan, &[street(3, 2, vec![10])], recall(), &later(), &[]);
        assert_eq!(ran[0], (0, 0, 16, 21));
        assert_eq!(calls[0].to_green_s, 11);
        // Once it is done (20 s): nothing more is kept.
        let (ran, _) = serve(&plan, &[street(3, 2, vec![20])], recall(), &later(), &[]);
        assert_eq!(ran[0], (0, 0, 20, 25));
        // A bike movement on recall (5 s minimum green, 7 s clearance): during the clearance (8 s), it runs out to 12 s;
        // during the minimum green (2 s), the clearance follows in full, to 9 s.
        let bikes = || vec![vec![(bicycles::MIN_GREEN_S, 12)], vec![], vec![]];
        assert_eq!(serve(&plan, &[street(3, 2, vec![8])], bikes(), &later(), &[]).0[0], (0, 0, 12, 17));
        assert_eq!(serve(&plan, &[street(3, 2, vec![2])], bikes(), &later(), &[]).0[0], (0, 0, 9, 14));
        // A pedestrian call on call served in the green (22 s: a 7 s walk, a 15 s clearance), the call at 2 s: to 17 s.
        let walks = [walk(8, 22, vec![0]), walk(8, 10, vec![400])];
        assert_eq!(serve(&plan, &[street(3, 2, vec![2])], none(), &walks, &[]).0[0], (0, 0, 17, 22));
        // A bicycle call on detection served in the green (12 s: 5 s and a 7 s clearance), the call at 8 s: to 12 s.
        let bikes = [walk(1, 12, vec![0])];
        assert_eq!(serve(&plan, &[street(3, 2, vec![8])], none(), &later(), &bikes).0[0], (0, 0, 12, 17));
        // Whatever ends the green, the yellow and the all-red after it run in full.
        for t in 0..40 {
            for (_, start, green, end) in serve(&plan, &[street(3, 2, vec![t])], recall(), &walks, &bikes).0 {
                assert_eq!(end - start - u64::from(green), 5, "a call at {t} s");
            }
        }
    }

    #[test]
    fn a_call_on_a_street_whose_green_runs_keeps_it_green_and_holds_it_for_the_dwell() {
        let plan = three();
        // Street 1's call at 20 s in its own green (to 25 s): green 0 s away, held to 40 s.
        let (ran, calls) = serve(&plan, &[street(1, 0, vec![20])], none(), &later(), &[]);
        assert_eq!(ran[0], (0, 0, 40, 45));
        assert_eq!(calls, vec![call(1, 20, 0, None)]);
        // Early in it (2 s): its 20 s end before the green's 25 s, which runs as planned.
        let (ran, calls) = serve(&plan, &[street(1, 0, vec![2])], none(), &later(), &[]);
        assert_eq!(ran[0], (0, 0, 25, 30));
        assert_eq!(calls, vec![call(1, 2, 0, None)]);
    }

    #[test]
    fn calls_are_served_in_the_order_they_come_one_during_another_s_pre_emption_waiting_for_it() {
        // Four phases, streets 1 to 4. Street 3's call at 5 s ends street 1's green; street 4's at 8 s waits for street
        // 3's green to end (it is not cut), then runs; then street 2's phase, displaced by the first, and the plan goes on.
        let plan = plan_of(vec![phase(1, 25, &[8], &[]), phase(2, 25, &[], &[]), phase(3, 25, &[], &[]), phase(4, 25, &[], &[])]);
        let streets = [street(3, 2, vec![5]), street(4, 3, vec![8])];
        let (ran, calls) = serve(&plan, &streets, vec![vec![]; 4], &later(), &[]);
        assert_eq!(&ran[..5], &[(0, 0, 5, 10), (2, 10, 25, 40), (3, 40, 25, 70), (1, 70, 25, 100), (2, 100, 25, 130)]);
        assert_eq!(calls, vec![call(3, 5, 5, Some(1)), call(4, 8, 32, None)]);
        // The second street's phase the next after the first's: no phase displaced, the plan goes on after it.
        let plan = three();
        let (ran, _) = serve(&plan, &[street(2, 1, vec![5]), street(3, 2, vec![8])], none(), &later(), &[]);
        assert_eq!(&ran[..4], &[(0, 0, 5, 10), (1, 10, 25, 40), (2, 40, 25, 70), (0, 70, 25, 100)]);
        // A second call of the same street during its emergency green is served there and holds it 20 s past it.
        let (ran, calls) = serve(&plan, &[street(2, 1, vec![5, 30])], none(), &later(), &[]);
        assert_eq!(&ran[..2], &[(0, 0, 5, 10), (1, 10, 40, 55)]);
        assert_eq!(calls, vec![call(2, 5, 5, Some(1)), call(2, 30, 0, None)]);
    }

    #[test]
    fn a_call_counts_who_it_pre_empted_and_who_it_delayed_by_kind() {
        let plan = three();
        // Crossing 8's walk is served in street 1's green from 0 s (7 s walk, 9 s clearance); a pedestrian waits from 3 s
        // at crossing 9, walked in street 2's phase. Street 3's call at 10 s ends street 1's green after the clearance,
        // at 16 s: the walk at crossing 8 was pre-empted, the one waiting at crossing 9 delayed.
        let walks = [walk(8, 16, vec![0]), walk(9, 16, vec![3]), walk(8, 10, vec![400])];
        let (ran, calls) = serve(&plan, &[street(3, 2, vec![10])], none(), &walks, &[]);
        assert_eq!(&ran[..3], &[(0, 0, 16, 21), (2, 21, 25, 51), (1, 51, 25, 81)]);
        let c = calls[0];
        assert_eq!((c.to_green_s, c.interrupted), (11, Some(1)));
        assert_eq!(c.preempted, Counts { pedestrians: 1, ..Counts::default() });
        assert_eq!(c.delayed, Counts { pedestrians: 1, ..Counts::default() });
        // A call that changes nothing (its own green runs long enough) delays nobody.
        let walks = [walk(9, 16, vec![1]), walk(8, 10, vec![400])];
        let (_, calls) = serve(&plan, &[street(1, 0, vec![2])], none(), &walks, &[]);
        assert_eq!((calls[0].preempted, calls[0].delayed), (Counts::default(), Counts::default()));
    }

    #[test]
    fn no_lower_policy_shortens_a_green_running_for_a_pre_emption_call() {
        let plan = three();
        let rules = control::fixed(&plan);
        let run_with = |emergency: Vec<Street>, buses: Vec<transit::Street>| {
            let mut emergencies = Emergencies::new(&plan, &emergency, none());
            let mut bus = transit::Buses::new(&plan, &rules, &buses, vec![0; 3]);
            let turns = run(&plan, &rules, &mut [&mut bus as &mut dyn Demand, &mut emergencies], 100_000);
            (turns.iter().filter(|t| !t.skipped).map(|t| (t.phase, t.start, t.green_s(), t.end)).collect::<Vec<_>>(), emergencies.finish())
        };
        let early = |arm, phase, at_s| transit::Street { arm, mode: transit::Mode::EarlyGreen, phase, at_s };
        // Street 2's call at 5 s takes the signal to its phase from 10 s; street 3's bus at 12 s, during that emergency
        // green, would cut it (its phase is next) by 10 s: not at all, the green runs its whole 25 s.
        let (ran, calls) = run_with(vec![street(2, 1, vec![5])], vec![early(3, 2, vec![12])]);
        assert_eq!(&ran[..2], &[(0, 0, 5, 10), (1, 10, 25, 40)]);
        assert_eq!(calls, vec![call(2, 5, 5, Some(1))]);
        // Street 1's call at 20 s in its own green: street 2's bus at 3 s would cut that green to 15 s, ending it before
        // the call; it is not cut, and held for the dwell, so the bus waiting since 3 s is delayed by the call.
        let (ran, calls) = run_with(vec![street(1, 0, vec![20])], vec![early(2, 1, vec![3])]);
        assert_eq!(ran[0], (0, 0, 40, 45));
        assert_eq!(calls, vec![Call { delayed: Counts { buses: 1, ..Counts::default() }, ..call(1, 20, 0, None) }]);
    }

    /// The emergency calls of `emergency` and the buses of `buses` served by `plan` (every phase fixed): the turns that
    /// ran as (phase, start, green, end), every emergency call, every bus and the delay transit priority moved.
    #[allow(clippy::type_complexity)]
    fn with_buses(
        plan: &Plan,
        emergency: &[Street],
        buses: &[transit::Street],
    ) -> (Vec<(usize, u64, u32, u64)>, Vec<Call>, Vec<transit::Bus>, Vec<transit::Moved>) {
        let rules = control::fixed(plan);
        let mut emergencies = Emergencies::new(plan, emergency, none());
        let mut bus = transit::Buses::new(plan, &rules, buses, vec![0; plan.phases.len()]);
        let turns = run(plan, &rules, &mut [&mut bus as &mut dyn Demand, &mut emergencies], 100_000);
        let (served, moved, _) = bus.finish();
        (turns.iter().filter(|t| !t.skipped).map(|t| (t.phase, t.start, t.green_s(), t.end)).collect(), emergencies.finish(), served, moved)
    }

    #[test]
    fn transit_counts_no_early_green_or_extension_a_green_kept_for_a_call_never_gave() {
        let plan = three();
        let bus = |arm, mode, phase, at_s| transit::Street { arm, mode, phase, at_s };
        // Street 2's call at 5 s takes the signal to its phase from 10 s, its green not cut for street 3's bus at 12 s:
        // that bus waits to 40 s with no early green, and no delay is moved for it.
        let (_, _, served, moved) = with_buses(&plan, &[street(2, 1, vec![5])], &[bus(3, transit::Mode::EarlyGreen, 2, vec![12])]);
        assert_eq!((served[0].outcome, served[0].wait_s, served[0].early_s), (transit::Outcome::Delayed, 28, 0));
        assert!(moved.is_empty(), "{moved:?}");
        // Street 1's call at 20 s holds its green to 40 s for the dwell; its bus at 30 s comes in that green, which ran
        // for the call: on arrival, not an extension, and the next phases bear no delay of transit's.
        let (ran, _, served, moved) = with_buses(&plan, &[street(1, 0, vec![20])], &[bus(1, transit::Mode::Extension, 0, vec![30])]);
        assert_eq!(ran[0], (0, 0, 40, 45));
        assert_eq!((served[0].outcome, served[0].extended_s), (transit::Outcome::OnArrival, 0));
        assert!(moved.is_empty(), "{moved:?}");
    }

    #[test]
    fn a_call_ends_a_green_a_bus_holds_longer_and_never_lengthens_one_a_bus_cut_short() {
        let plan = three();
        let bus = |arm, mode, phase, at_s| transit::Street { arm, mode, phase, at_s };
        // Street 1's bus at 27 s would hold its green from 25 s to 28 s; street 2's call at 26 s ends it there, no
        // transit extension holding the emergency vehicle: its green at 31 s, after the yellow and all-red.
        let (ran, calls, _, _) = with_buses(&plan, &[street(2, 1, vec![26])], &[bus(1, transit::Mode::Extension, 0, vec![27])]);
        assert_eq!(&ran[..2], &[(0, 0, 26, 31), (1, 31, 25, 61)]);
        assert_eq!(calls, vec![call(2, 26, 5, Some(1))]);
        // Street 2's bus at 3 s cuts street 1's green to 15 s; street 3's call at 18 s comes in its yellow: the green
        // still ends at 15 s, the call interrupting nothing, and runs street 3's phase out of turn at 20 s, putting back
        // the bus's.
        let (ran, calls, served, _) = with_buses(&plan, &[street(3, 2, vec![18])], &[bus(2, transit::Mode::EarlyGreen, 1, vec![3])]);
        assert_eq!(&ran[..3], &[(0, 0, 15, 20), (2, 20, 25, 50), (1, 50, 25, 80)]);
        assert_eq!(calls, vec![Call { delayed: Counts { buses: 1, ..Counts::default() }, ..call(3, 18, 2, None) }]);
        assert_eq!((served[0].outcome, served[0].early_s), (transit::Outcome::EarlyGreen, 10), "the bus's early green is kept");
    }

    #[test]
    fn a_call_in_a_rest_gap_counts_who_waited_at_its_second_before_the_next_turn_serves_anyone() {
        // Every phase actuated and nobody coming: the signal rests. At 100 s street 3's call comes, and at the same
        // second a pedestrian at crossing 8 (street 1's phase) and one at crossing 9 (street 2's phase). The call takes the
        // signal to street 3's phase at once; both pedestrians were waiting at its second, for other phases: delayed.
        let plan = three();
        let bounds = control::Bounds { min_green_s: 7, gap_s: 3, max_green_s: 45 };
        let rules = vec![Rule::Actuated { bounds, floor_s: 7 }; 3];
        let walks = [walk(8, 16, vec![100]), walk(9, 16, vec![100])];
        let mut walking = Walks::new(&plan, &rules, &walks);
        let streets = [street(3, 2, vec![100])];
        let mut emergencies = Emergencies::new(&plan, &streets, none());
        let turns = run(&plan, &rules, &mut [&mut walking as &mut dyn Demand, &mut emergencies], 100_000);
        let first = turns.iter().find(|t| !t.skipped).unwrap();
        assert_eq!((first.phase, first.start), (2, 100), "out of the rest, straight to street 3's phase");
        let c = emergencies.finish()[0];
        assert_eq!((c.to_green_s, c.interrupted, c.preempted), (0, None, Counts::default()));
        assert_eq!(c.delayed, Counts { pedestrians: 2, ..Counts::default() });
        // A pedestrian the emergency phase itself serves (crossing 10, walked in street 3's phase) is not delayed by it.
        let plan = plan_of(vec![phase(1, 25, &[8], &[]), phase(2, 25, &[9], &[]), phase(3, 25, &[10], &[])]);
        let walks = [walk(8, 16, vec![100]), walk(10, 16, vec![100])];
        let mut walking = Walks::new(&plan, &rules, &walks);
        let mut emergencies = Emergencies::new(&plan, &streets, none());
        run(&plan, &rules, &mut [&mut walking as &mut dyn Demand, &mut emergencies], 100_000);
        assert_eq!(emergencies.finish()[0].delayed, Counts { pedestrians: 1, ..Counts::default() });
    }

    #[test]
    fn the_safe_entry_is_the_longest_clearance_yellow_and_all_red_of_the_phases_a_call_can_interrupt() {
        let plan = three();
        // Crossing 8 (9 m: a 9 s clearance) in street 1's phase; street 1's bike movement (26 m: 7 s) too.
        let (distances, rides) = ([(8, 9_000), (9, 4_000)], [(1, 26_000)]);
        assert_eq!(entry_s(&plan, 2, &distances, &rides), 14, "9 s, then 3 s and 2 s");
        assert_eq!(entry_s(&plan, 0, &distances, &rides), 9, "street 2's crossing: 4 s, then 5 s");
        assert_eq!(entry_s(&plan, 0, &[], &[]), 5);
    }

    #[test]
    fn pre_emption_overrides_an_actuated_phase_and_calls_one() {
        let plan = three();
        let bounds = control::Bounds { min_green_s: 7, gap_s: 3, max_green_s: 45 };
        let rules = vec![Rule::Fixed, Rule::Fixed, Rule::Actuated { bounds, floor_s: 7 }];
        let streets = [street(3, 2, vec![100])];
        let mut emergencies = Emergencies::new(&plan, &streets, none());
        let turns = run(&plan, &rules, &mut [&mut emergencies as &mut dyn Demand], 100_000);
        let served = turns.iter().find(|t| t.phase == 2 && !t.skipped).unwrap();
        assert_eq!(served.green_s(), DWELL_S, "called by the emergency, held for its dwell");
        assert_eq!(emergencies.finish()[0].outcome, Outcome::Served);
    }

    #[test]
    fn pre_emption_is_allowed_only_on_a_street_the_plan_gives_a_phase_of_its_own() {
        let j = junction_biked();
        let s = j.current().clone();
        let n = at(&s, 0);
        assert_eq!(refusal(&s, 0, n), None);
        let mut no_phase = s.clone();
        let so = at(&s, 180);
        no_phase.signal_plan =
            Some(SignalPlan { phases: vec![StoredPhase { moves: vec![Move { from: so, to: n }], walks: vec![], rides: vec![], arm: None }] });
        assert_eq!(refusal(&no_phase, 0, n), Some(Refusal::PreemptionNoPhase));
        let mut not_signal = s.clone();
        not_signal.control = PRIORITY;
        assert_eq!(refusal(&not_signal, 0, n), Some(Refusal::NotSignal));
        assert_eq!(refusal(&s, 0, 12_345), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn pre_emption_is_turned_on_its_calls_given_within_the_bound_cleared_and_undone_like_any_edit() {
        let mut j = junction_biked();
        let n = at(j.current(), 0);
        assert!(!j.set_emergency_calls(n, 4));
        assert_eq!(j.refusal(), Some(Refusal::NotOnPreemption));
        assert!(j.set_preemption(n, true));
        assert_eq!(j.arm(n).unwrap().preemption, Some(DEFAULT_CALLS_PER_HOUR));
        assert_eq!(j.current().label.key, "jn-rev-preemption-on");
        assert!(j.set_emergency_calls(n, 12));
        assert_eq!(j.current().label.key, "jn-rev-emergency-calls");
        for bad in [-1, 13] {
            assert!(!j.set_emergency_calls(n, bad), "{bad}");
            assert_eq!(j.refusal(), Some(Refusal::EmergencyCallsOutOfRange));
        }
        let edits = j.revisions().count();
        assert!(j.set_emergency_calls(n, 12), "the rate it has: taken");
        assert!(j.set_preemption(n, true), "the setting it has: taken");
        assert_eq!(j.revisions().count(), edits, "and no edit");
        assert!(j.set_preemption(n, false));
        assert_eq!((j.arm(n).unwrap().preemption, j.current().label.key), (None, "jn-rev-preemption-off"));
        assert!(j.undo() && j.arm(n).unwrap().preemption == Some(12));
        // Off a signal it goes, as the timing does, and the revision says the timing went.
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.arm(n).unwrap().preemption, None);
        assert_eq!(j.current().label.key, "jn-rev-control-timing");
        assert!(!j.set_preemption(n, true));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
    }

    #[test]
    fn a_junction_with_no_street_given_pre_emption_is_written_as_before_and_its_pre_emption_only_once_set() {
        let j = junction_biked();
        let mut s = j.current().clone();
        let n = at(&s, 0);
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("preemption"), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        arm_mut(&mut s, n).preemption = Some(3);
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""preemption":3"#), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.arms.iter().find(|a| a.uid == n).unwrap().preemption, Some(3));
        // A document's rate past the bound is read as the bound.
        arm_mut(&mut s, n).preemption = Some(5_000);
        assert_eq!(rate_of(s.arms.iter().find(|a| a.uid == n).unwrap()), Some(MAX_CALLS_PER_HOUR));
    }

    #[test]
    fn a_street_kept_with_pre_emption_but_no_phase_is_a_finding_and_takes_the_signal_nowhere() {
        let j = junction_biked();
        let mut s = j.current().clone();
        let (n, so) = (at(&s, 0), at(&s, 180));
        arm_mut(&mut s, n).preemption = Some(12);
        s.signal_plan = Some(SignalPlan { phases: vec![StoredPhase { moves: vec![Move { from: so, to: n }], walks: vec![], rides: vec![], arm: None }] });
        let plan = signal::plan(&s, 0).unwrap();
        let h = control::hour(&s, &plan, &[], &[]);
        assert_eq!(h.preemption.findings, vec![Finding::NoPhase { arm: n }]);
        assert!(h.preemption.totals.is_empty() && h.preemption.calls.is_empty());
    }

    #[test]
    fn with_no_call_the_hour_is_as_with_pre_emption_off() {
        let j = junction_biked();
        let mut s = j.current().clone();
        let (n, e) = (at(&s, 0), at(&s, 90));
        arm_mut(&mut s, e).calls = Some(12);
        arm_mut(&mut s, n).cyclists = Some(30);
        let plan = signal::plan(&s, 0).unwrap();
        let (distances, lengths) = ([(e, 9_000)], [(n, 26_000)]);
        let before = control::hour(&s, &plan, &distances, &lengths);
        assert_eq!(before.preemption, Summary::default());
        arm_mut(&mut s, n).preemption = Some(0);
        let after = control::hour(&s, &plan, &distances, &lengths);
        assert_eq!((&after.calls, &after.actuation, &after.bicycles, &after.transit), (&before.calls, &before.actuation, &before.bicycles, &before.transit));
        assert_eq!(after.preemption.totals.len(), 1);
        assert_eq!(after.preemption.totals[0].calls, 0);
    }

    #[test]
    fn fixed_inputs_give_a_fixed_summary() {
        let mut j = junction_biked();
        let n = at(j.current(), 0);
        assert!(j.set_bus_lane(n, true));
        let mut s = j.current().clone();
        let (e, so, w) = (at(&s, 90), at(&s, 180), at(&s, 270));
        // East's emergency calls pre-empt, 12 an hour; north's buses given both, 30 an hour; east's crossing on call;
        // west's cyclists on detection; south actuated.
        arm_mut(&mut s, e).preemption = Some(12);
        arm_mut(&mut s, n).transit_priority = Some(transit::Priority { mode: transit::Mode::Both, buses: 30 });
        arm_mut(&mut s, e).calls = Some(6);
        arm_mut(&mut s, w).cyclists = Some(30);
        arm_mut(&mut s, so).actuation = Some(actuation::Actuation::default());
        let plan = signal::plan(&s, 0).unwrap();
        let distances = [(n, 9_000), (e, 9_000), (so, 9_000), (w, 9_000)];
        let lengths = [(n, 26_000), (e, 26_000), (so, 26_000), (w, 26_000)];
        let h = control::hour(&s, &plan, &distances, &lengths);
        assert_eq!(h, control::hour(&s, &plan, &distances, &lengths), "deterministic");
        assert_eq!(h.preemption.calls.len(), 12);
        assert!(h.preemption.calls.iter().all(|c| c.outcome == Outcome::Served));
        let t = h.preemption.totals[0];
        assert_eq!((t.arm, t.rate, t.calls, t.served), (e, 12, 12, 12));
        assert!(t.longest_to_green_s.unwrap() <= t.entry_s, "{t:?}: no other pre-emption runs, so none waits past the safe entry");
        assert_eq!(h.preemption.totals, vec![PINNED_TOTALS(e)], "{:#?}", h.preemption.totals);
        assert_eq!(&h.preemption.calls[..3], &PINNED_CALLS_OF(e, n, w)[..], "{:#?}", &h.preemption.calls[..3]);
    }

    #[allow(non_snake_case)]
    fn PINNED_TOTALS(e: u32) -> Totals {
        Totals {
            arm: e,
            rate: 12,
            entry_s: 14,
            calls: 12,
            served: 12,
            not_served: 0,
            longest_to_green_s: Some(14),
            preempted: Counts { pedestrians: 0, cyclists: 1, buses: 1, vehicles: 0 },
            delayed: Counts { pedestrians: 1, cyclists: 0, buses: 2, vehicles: 67 },
        }
    }

    #[allow(non_snake_case)]
    fn PINNED_CALLS_OF(e: u32, n: u32, w: u32) -> Vec<Call> {
        let vehicles = |vehicles| Counts { vehicles, ..Counts::default() };
        vec![
            // During north's clearance (its 9 m crossing on recall, from 7 s to 16 s): it runs out, then the yellow and
            // the all-red.
            Call { delayed: vehicles(1), ..call(e, 8, 13, Some(n)) },
            // East's own green: held for the dwell.
            Call { delayed: vehicles(7), ..call(e, 456, 0, None) },
            // During west's green, nothing under way: the yellow and the all-red.
            Call { delayed: vehicles(3), ..call(e, 637, 5, Some(w)) },
        ]
    }
}
