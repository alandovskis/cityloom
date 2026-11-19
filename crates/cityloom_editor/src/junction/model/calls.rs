//! Pedestrian calls (#60), the first responsive behaviour of a signal: a crossing on call is walked only when someone
//! has pushed the button, and the plan's fixed-time cycle serves the calls. A seeded generator spreads each on-call
//! crossing's calls over one hour, and `summary` runs the cycle over them: when each call is served, how long it
//! waited, and how far a phase's green was lengthened to fit the walk and its clearance. Deterministic: the same
//! junction, plan and rates always give the same calls and the same summary. CityLoom models the behaviour to compare
//! strategies; it never controls a signal. No DOM, no signals, no I/O, no wording.
//!
//! Every figure here is a placeholder (synthetic, like the catalogue's rules and rates): the walk, the clearance speed,
//! the longest extension and the bound on the call rate.

use crate::junction::model::signal::Plan;
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
pub const HOUR_S: u32 = 3_600;

/// The pedestrian clearance of a crossing `distance_mm` long: its length at the clearance speed, rounded up to the
/// next whole second.
pub fn clearance_s(distance_mm: i32) -> u32 {
    (distance_mm.max(0) as u32).div_ceil(CLEARANCE_MM_PER_S)
}

/// What a call needs of the green that serves it: the walk and then the clearance, never shortened.
pub fn need_s(distance_mm: i32) -> u32 {
    WALK_S + clearance_s(distance_mm)
}

/// One step of SplitMix64: a small fixed generator, so the calls are the same on every machine and in every build.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The seconds into the hour at which the crossing of street `uid` is called, `rate` times an hour: the hour is cut
/// into `rate` equal slots and each slot gets one call at a point the generator, seeded from the street and the rate,
/// picks. In order, each second at most once. A rate past `MAX_CALLS_PER_HOUR` is read as that.
pub fn arrivals(uid: u32, rate: u32) -> Vec<u32> {
    let rate = rate.min(MAX_CALLS_PER_HOUR);
    let mut seed = (u64::from(uid) << 32) | u64::from(rate);
    (0..rate)
        .map(|k| {
            let (lo, hi) = (k * HOUR_S / rate, (k + 1) * HOUR_S / rate);
            lo + (splitmix(&mut seed) % u64::from(hi - lo)) as u32
        })
        .collect()
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
    pub served: u32,
    pub delayed: u32,
    pub not_served: u32,
    /// The calls whose green was lengthened, served or delayed.
    pub extended: u32,
    pub longest_wait_s: u32,
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

/// Runs the cycle of `plan` over an hour of calls at the crossings of `s` that are on call. `distances` is each
/// crossing's length by street uid (`read_model::crossing_distances`). A crossing no phase walks is a finding and is
/// left out; one no phase can serve (`serve`) is a finding and its calls are not served.
pub fn summary(s: &State, plan: &Plan, distances: &[(u32, i32)]) -> Summary {
    let mut out = Summary::default();
    let distance = |uid: u32| distances.iter().find(|(u, _)| *u == uid).map(|(_, d)| *d);
    let mut asked: Vec<Asked> = Vec::new();
    for (arm, rate) in on_call(s) {
        let (Some(d), true) = (distance(arm.uid).filter(|_| arm.crossing.is_some()), walked(plan, arm.uid)) else {
            out.findings.push(Finding::NotWalked { arm: arm.uid });
            continue;
        };
        let need = need_s(d);
        out.totals.push(Totals { arm: arm.uid, rate, clearance_s: clearance_s(d), need_s: need, ..Totals::default() });
        asked.push(Asked { arm: arm.uid, need_s: need, at_s: arrivals(arm.uid, rate) });
    }
    for a in &asked {
        let walking = || plan.phases.iter().filter(|p| p.walks.contains(&a.arm));
        if !walking().any(|p| p.timing.green_s + MAX_EXTENSION_S >= a.need_s) {
            let most_s = walking().map(|p| p.timing.green_s).max().unwrap_or(0) + MAX_EXTENSION_S;
            out.findings.push(Finding::CannotServe { arm: a.arm, need_s: a.need_s, most_s });
        }
    }
    out.calls = serve(plan, &asked);
    for t in &mut out.totals {
        for c in out.calls.iter().filter(|c| c.arm == t.arm) {
            match c.outcome {
                Outcome::Served => t.served += 1,
                Outcome::Delayed => t.delayed += 1,
                Outcome::NotServed => t.not_served += 1,
            }
            t.extended += u32::from(c.extended_s > 0);
            t.longest_wait_s = t.longest_wait_s.max(c.wait_s);
        }
    }
    out
}

/// The calls made at one crossing: its street, what a call needs of the green (`need_s`), and when each was made.
struct Asked {
    arm: u32,
    need_s: u32,
    at_s: Vec<u32>,
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
/// A phase takes at least a second in the run, so time moves on whatever a document's timing says (it is not clamped
/// when read): every call that can be served is, within a cycle of being made.
fn serve(plan: &Plan, asked: &[Asked]) -> Vec<Call> {
    serve_turns(plan, asked, MAX_TURNS)
}

/// A bound on the phases `serve` runs, only a guard: a phase lasts a second or more, and the plan at most a few dozen,
/// so every call is served long before.
const MAX_TURNS: usize = 1_000_000;

/// `serve`, run for at most `turns` phases. A call still waiting when it stops is not served.
fn serve_turns(plan: &Plan, asked: &[Asked], turns: usize) -> Vec<Call> {
    // The phases (places in the plan) that can serve each crossing.
    let serving: Vec<Vec<usize>> = asked
        .iter()
        .map(|a| {
            (0..plan.phases.len()).filter(|&k| plan.phases[k].walks.contains(&a.arm) && plan.phases[k].timing.green_s + MAX_EXTENSION_S >= a.need_s).collect()
        })
        .collect();
    let mut calls: Vec<(usize, Call)> = asked
        .iter()
        .enumerate()
        .flat_map(|(x, a)| {
            let outcome = if serving[x].is_empty() { Outcome::NotServed } else { Outcome::Served };
            a.at_s.iter().map(move |&at_s| (x, Call { arm: a.arm, at_s, outcome, wait_s: 0, extended_s: 0 }))
        })
        .collect();
    calls.sort_by_key(|(x, c)| (c.at_s, *x));
    // The calls still waiting for their walk (places in `calls`), and whether each has missed one.
    let mut waiting: Vec<usize> = (0..calls.len()).filter(|&i| calls[i].1.outcome != Outcome::NotServed).collect();
    let mut missed = vec![false; calls.len()];
    let (mut start, mut turn) = (0u32, 0usize);
    while !waiting.is_empty() && !plan.phases.is_empty() && turn < turns {
        let k = turn % plan.phases.len();
        let phase = &plan.phases[k];
        let serves = |x: usize| serving[x].contains(&k);
        let (now, later): (Vec<usize>, Vec<usize>) = waiting.iter().partition(|&&i| calls[i].1.at_s <= start && serves(calls[i].0));
        let mut extension = 0;
        for &i in &now {
            let short = asked[calls[i].0].need_s.saturating_sub(phase.timing.green_s);
            extension = extension.max(short);
            let c = &mut calls[i].1;
            c.wait_s = start - c.at_s;
            c.extended_s = short;
            c.outcome = if missed[i] { Outcome::Delayed } else { Outcome::Served };
        }
        let end = start + (phase.length_s + extension).max(1);
        // A call made while this phase walks its crossing has missed that walk.
        for &i in &later {
            if serves(calls[i].0) && calls[i].1.at_s > start && calls[i].1.at_s < end {
                missed[i] = true;
            }
        }
        waiting = later;
        start = end;
        turn += 1;
    }
    for i in waiting {
        calls[i].1.outcome = Outcome::NotServed;
    }
    calls.into_iter().map(|(_, c)| c).collect()
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
        let cycle_s = phases.iter().map(|p| p.length_s).sum();
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
        let asked = [Asked { arm: 9, need_s: 16, at_s: vec![0, 10, 40] }];
        assert_eq!(
            serve(&plan, &asked),
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
        let asked = [Asked { arm: 9, need_s: 19, at_s: vec![0, 5, 30] }, Asked { arm: 8, need_s: 10, at_s: vec![0] }];
        assert_eq!(
            serve(&plan, &asked),
            vec![
                call(9, 0, Outcome::Served, 0, 9),
                call(8, 0, Outcome::Served, 24, 0),
                call(9, 5, Outcome::Delayed, 49, 9),
                call(9, 30, Outcome::Served, 24, 9),
            ]
        );
        // With no call at street 9, its phase keeps its 15 s and street 8 waits only that long.
        assert_eq!(serve(&plan, &asked[1..]), vec![call(8, 0, Outcome::Served, 15, 0)]);
    }

    #[test]
    fn a_phase_that_cannot_fit_the_walk_and_clearance_even_lengthened_does_not_serve_the_call() {
        // 26 s needed: 16 s more than the first phase's 10 s green, past the 15 s bound, so only the third (25 s) serves.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 25, &[]), phase(3, 25, &[9])]);
        let asked = [Asked { arm: 9, need_s: 26, at_s: vec![0, 5] }];
        assert_eq!(serve(&plan, &asked), vec![call(9, 0, Outcome::Served, 45, 1), call(9, 5, Outcome::Served, 40, 1)], "the first phase is no walk to miss");
        // No phase at all: not served, and nothing waits for it.
        let plan = plan_of(vec![phase(1, 10, &[9]), phase(2, 25, &[])]);
        assert_eq!(serve(&plan, &asked), vec![call(9, 0, Outcome::NotServed, 0, 0), call(9, 5, Outcome::NotServed, 0, 0)]);
    }

    /// A phase of street `uid` that takes no time at all (a document's timing is not clamped when it is read).
    fn empty(uid: u32, walks: &[u32]) -> Phase {
        let timing = Timing { green_s: 0, yellow_s: 0, all_red_s: 0, transit_lead_s: 0 };
        Phase { uid, timing, length_s: 0, walks: walks.to_vec(), rides: vec![], moves: vec![] }
    }

    #[test]
    fn a_plan_whose_phases_take_no_time_still_ends_at_once_and_serves_its_calls() {
        let plan = plan_of(vec![empty(1, &[9]), empty(2, &[])]);
        let asked = [Asked { arm: 9, need_s: 10, at_s: vec![0, 100, 3_599] }];
        let started = std::time::Instant::now();
        let calls = serve(&plan, &asked);
        assert!(started.elapsed() < std::time::Duration::from_secs(1), "it took {:?}", started.elapsed());
        // The first phase's green of 0 s is lengthened by the 10 s the crossing needs; a phase that takes no time
        // takes a second in the run, so the walk comes round again.
        assert_eq!(calls[0], call(9, 0, Outcome::Served, 0, 10));
        assert!(calls.iter().all(|c| c.outcome != Outcome::NotServed && c.wait_s < 60 && c.extended_s == 10), "{calls:?}");
    }

    #[test]
    fn a_call_still_waiting_when_the_run_stops_is_not_served_never_served_with_no_wait() {
        let plan = plan_of(vec![phase(1, 25, &[9]), phase(2, 25, &[])]);
        let asked = [Asked { arm: 9, need_s: 16, at_s: vec![0, 10, 40] }];
        // Two turns: the first phase (serving the call at 0 s), then the second; the others are still waiting.
        assert_eq!(
            serve_turns(&plan, &asked, 2),
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
    fn a_junction_that_stops_being_a_signal_takes_its_crossings_off_call_and_says_so() {
        let mut j = Junction::new(0);
        let e = at(j.current(), 90);
        assert!(j.set_on_call(e, true));
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.arm(e).unwrap().calls, None, "no signal, no calls: nothing is left on call where no section shows it");
        let label = &j.current().label;
        assert_eq!(label.key, "jn-rev-calls-lost", "the revision says the calls went: {label:?}");
        assert!(matches!(&label.args[0].1, crate::shared::said::Arg::Said(inner) if inner.key.starts_with("jn-rev-control")));
        // A document saved with calls on a junction that is not a signal reads without them.
        let mut kept = Junction::new(0).current().clone();
        kept.control = PRIORITY;
        kept.arms[0].calls = Some(12);
        let mut back = kept.clone();
        crate::junction::model::settle_for_test(&mut back, 0);
        assert!(back.arms.iter().all(|a| a.calls.is_none()));
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
        // 7 s + 12 s = 19 s fits with 9 s more: every call is served lengthened.
        let sum = summary_of(&s, &[(e, 11_500)]);
        assert!(sum.findings.is_empty());
        assert!(sum.calls.iter().all(|c| c.extended_s == 9 && c.outcome != Outcome::NotServed), "{sum:?}");
        assert_eq!(sum.totals[0].extended, 6);
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
                Totals { arm: e, rate: 3, clearance_s: 9, need_s: 16, served: 2, delayed: 1, not_served: 0, extended: 0, longest_wait_s: 59 },
                Totals { arm: w, rate: 2, clearance_s: 21, need_s: 28, served: 1, delayed: 1, not_served: 0, extended: 2, longest_wait_s: 42 },
            ],
            findings: vec![],
        };
        assert_eq!(sum, pinned, "{sum:#?}");
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
