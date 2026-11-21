//! Bicycle detection (CLM-13): a street's bike movement (its cyclists entering the junction, going where its traffic
//! goes, `Phase.rides`) is on recall (it runs with every phase that lets it go, the default) or on detection (a cyclist
//! detected on the approach calls it, and the phase that serves the call gives it a bicycle minimum green and a
//! clearance for cyclists). A seeded generator spreads each detected movement's cyclists over one hour, and the
//! controller (`control::run`) serves them with the pedestrian calls and the actuated streets' vehicles. Deterministic:
//! the same junction, plan and rates always give the same hour. CityLoom models the behaviour to compare strategies; it
//! never controls a signal or predicts real-world delay or safety. No DOM, no signals, no I/O, no wording.
//!
//! The rule of a bicycle call (the pedestrian calls' rule, `calls::Walks` over the phases that run the movement):
//! - A cyclist is served at the start of the first phase, at or after they came, that runs their street's bike
//!   movement and can give it the bicycle minimum green and the clearance (`need_s`); one who came while such a phase
//!   ran missed it and is delayed to the next.
//! - A fixed phase whose green is shorter than that is lengthened by the shortfall, at most `MAX_EXTENSION_S` (the
//!   pedestrian calls' bound), its yellow and all-red kept; a fixed phase that cannot fit it even so does not serve the
//!   movement. A movement no phase can serve is a finding, and its calls are not served.
//! - With the pedestrian calls: when both are served in the same phase each holds the green for what it needs, so the
//!   longer requirement is kept; neither the walk and its clearance nor the bicycle minimum and its clearance is ever
//!   shortened.
//! - With vehicle actuation: a cyclist waiting calls an actuated phase that runs their movement as a vehicle or a
//!   pedestrian call does, and a served bicycle minimum and clearance hold its green past a gap-out and past the
//!   maximum, as a walk does. An actuated phase can always serve the movement (it holds its green for it).
//! - On recall nothing changes: the movement runs with its phases as before, and sets no actuated phase's floor.
//!
//! Every figure here is a placeholder (synthetic, like the catalogue's rules and rates): the minimum green, the
//! cycling speed, the longest extension, the default and the bound of the cyclists an hour.

use crate::junction::model::calls::{self, Asked, Walks};
use crate::junction::model::control;
use crate::junction::model::signal::{Phase, Plan};
use crate::junction::model::{Arm, Refusal, SIGNAL, State, in_force};

/// The least green a phase gives a bike movement it serves on detection, in seconds. Placeholder.
pub const MIN_GREEN_S: u32 = 5;
/// How fast a cyclist is taken to ride while clearing the junction, in millimetres a second: 4 m/s. Placeholder.
pub const SPEED_MM_PER_S: u32 = 4_000;
/// The most a phase's green is lengthened to serve a bicycle call, in seconds: the pedestrian calls' bound. Placeholder.
pub use crate::junction::model::calls::MAX_EXTENSION_S;
/// The cyclists an hour a bike movement is given when it is put on detection. Placeholder.
pub const DEFAULT_CYCLISTS_PER_HOUR: u32 = 30;
/// The most cyclists an hour a bike movement may be given. Placeholder.
pub const MAX_CYCLISTS_PER_HOUR: u32 = 120;

/// The bicycle clearance over a crossing of the junction `length_mm` long: its length at the cycling speed, rounded up
/// to the next whole second.
pub fn clearance_s(length_mm: i32) -> u32 {
    (length_mm.max(0) as u32).div_ceil(SPEED_MM_PER_S)
}

/// What a bicycle call needs of the green that serves it: the bicycle minimum green and then the clearance, never
/// shortened.
pub fn need_s(length_mm: i32) -> u32 {
    MIN_GREEN_S + clearance_s(length_mm)
}

/// The cyclists an hour of arm `a`'s bike movement, if it is on detection, never past `MAX_CYCLISTS_PER_HOUR`: a document
/// from a sync or written by hand can hold any number. Read it here, never from `Arm.cyclists`.
pub fn rate_of(a: &Arm) -> Option<u32> {
    a.cyclists.map(|r| r.min(MAX_CYCLISTS_PER_HOUR))
}

/// Whether phase `p` runs the bike movement of street `uid`: who a bicycle call waits for.
pub(crate) fn rides_it(p: &Phase, uid: u32) -> bool {
    p.rides.contains(&uid)
}

/// Whether some phase of `plan` runs the bike movement of street `uid`.
pub fn ridden(plan: &Plan, uid: u32) -> bool {
    plan.phases.iter().any(|p| rides_it(p, uid))
}

/// Why the bike movement of street `uid` cannot be put on detection, if it cannot: the junction has to be a signal, and
/// the plan has to run the movement in at least one phase (a street with no bike lane has none to run).
pub fn refusal(s: &State, region: usize, uid: u32) -> Option<Refusal> {
    if !s.arms.iter().any(|a| a.uid == uid) {
        return Some(Refusal::DoesNotFit);
    }
    let s = in_force(s);
    if s.control != SIGNAL {
        return Some(Refusal::NotSignal);
    }
    let Some(plan) = crate::junction::model::signal::plan(&s, region) else { return Some(Refusal::NotSignal) };
    (!ridden(&plan, uid)).then_some(Refusal::BikeNotRidden)
}

/// The seconds into the hour at which cyclists are detected on the approach of street `uid`, `rate` an hour: the hour is
/// cut into `rate` equal slots, one cyclist in each, at a point the generator picks. Seeded from the street and the rate
/// on a stream of its own (`BIKE_STREAM`), so it never repeats the pedestrian calls or the vehicles of the same street.
/// In order, each second at most once. A rate past `MAX_CYCLISTS_PER_HOUR` is read as that.
pub fn arrivals(uid: u32, rate: u32) -> Vec<u32> {
    let rate = rate.min(MAX_CYCLISTS_PER_HOUR);
    control::spread(((u64::from(uid) << 32) | u64::from(rate)) ^ BIKE_STREAM, rate)
}

/// What sets the cyclists' seeds apart from the pedestrian calls' and the vehicles' ("BICYCLES" in ASCII).
const BIKE_STREAM: u64 = 0x4249_4359_434C_4553;

/// What happened to one cyclist's call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Outcome {
    /// Served at the start of the first phase that runs the movement after they came.
    Served,
    /// Came after a phase that runs the movement had started: waited for the next one.
    Delayed,
    /// No phase can serve the movement (a `Finding::CannotServe`), or the hour ended first.
    NotServed,
}

/// One cyclist's call: the street whose bike movement it is, when it came (seconds into the hour), what happened, how
/// long it waited for its green, and how many seconds the green that served it was lengthened for it (0: it fitted).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Ride {
    pub arm: u32,
    pub at_s: u32,
    pub outcome: Outcome,
    pub wait_s: u32,
    pub extended_s: u32,
}

/// The calls of one bike movement on detection, added up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Totals {
    pub arm: u32,
    /// Its cyclists an hour.
    pub rate: u32,
    /// How far a cyclist rides to clear the junction, the clearance that takes, and the minimum green plus it.
    pub length_mm: i32,
    pub clearance_s: u32,
    pub need_s: u32,
    /// Every call of the hour: served, delayed and not served.
    pub calls: u32,
    pub served: u32,
    pub delayed: u32,
    pub not_served: u32,
    /// The calls whose green was lengthened, and the seconds it was lengthened for them, added up.
    pub extended: u32,
    pub extended_s: u32,
    /// The longest a call waited for its green; None when none was served or delayed.
    pub longest_wait_s: Option<u32>,
}

/// Something about a bike movement on detection the person should act on. By street uid; no wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Finding {
    /// No phase that runs the movement can give its minimum green and clearance (`need_s`), even with its green
    /// lengthened by `MAX_EXTENSION_S`: `most_s` is the longest green it could have. Its calls are not served.
    CannotServe { arm: u32, need_s: u32, most_s: u32 },
    /// The movement is on detection, but no phase runs it any more (an edit took it or the bike lane away): it is kept on
    /// detection, and serves nobody until a phase runs it again.
    NotRidden { arm: u32 },
    /// An actuated phase of street `phase` runs the movement, and its minimum green and clearance (`need_s`) are longer
    /// than the phase's maximum green (`max_s`): a call served there holds the green past the maximum (as a walk does,
    /// actuation's `WalkPastMax`).
    PastMax { arm: u32, phase: u32, need_s: u32, max_s: u32 },
}

impl Finding {
    /// What the finding is about, whatever its figures: its kind and its street.
    pub fn about(&self) -> (u8, u32, u32) {
        match *self {
            Finding::CannotServe { arm, .. } => (20, arm, 0),
            Finding::NotRidden { arm } => (21, arm, 0),
            Finding::PastMax { arm, phase, .. } => (22, arm, phase),
        }
    }
}

/// How the plan serves the bicycle calls of one hour.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Summary {
    /// Every call, in the order they came.
    pub rides: Vec<Ride>,
    /// One for each movement on detection that some phase runs, in the order of the bearings.
    pub totals: Vec<Totals>,
    pub findings: Vec<Finding>,
}

/// The calls of the hour of the bike movements of `s` on detection, one `Asked` a movement, with `out` given a row of
/// totals for each and a finding for each no phase runs (left out). `lengths` is how far each movement's cyclists ride
/// to clear the junction, by street uid (`read_model::ride_lengths`).
pub(crate) fn asked(s: &State, plan: &Plan, lengths: &[(u32, i32)], out: &mut Summary) -> Vec<Asked> {
    let mut arms: Vec<&Arm> = s.arms.iter().collect();
    arms.sort_by_key(|a| a.bearing);
    let mut asked = Vec::new();
    for (arm, rate) in arms.into_iter().filter_map(|a| rate_of(a).map(|r| (a, r))) {
        let length = lengths.iter().find(|(u, _)| *u == arm.uid).map(|(_, l)| *l);
        let (Some(length_mm), true) = (length, ridden(plan, arm.uid)) else {
            out.findings.push(Finding::NotRidden { arm: arm.uid });
            continue;
        };
        let need = need_s(length_mm);
        out.totals.push(Totals { arm: arm.uid, rate, length_mm, clearance_s: clearance_s(length_mm), need_s: need, ..Totals::default() });
        asked.push(Asked { arm: arm.uid, need_s: need, at_s: arrivals(arm.uid, rate), passive: false });
    }
    asked
}

/// A finding for each movement that no phase running it can serve by `rules` (the pedestrian calls' rule).
pub(crate) fn cannot_serve(plan: &Plan, rules: &[control::Rule], asked: &[Asked]) -> Vec<Finding> {
    calls::unserved(plan, rules, asked, rides_it).into_iter().map(|(arm, need_s, most_s)| Finding::CannotServe { arm, need_s, most_s }).collect()
}

/// A finding for each movement of `asked` that an actuated phase runs whose maximum green is shorter than what a call
/// needs: the green is held for it all the same, past the maximum.
pub(crate) fn past_max(plan: &Plan, rules: &[control::Rule], asked: &[Asked]) -> Vec<Finding> {
    let mut out = Vec::new();
    for a in asked {
        for (p, rule) in plan.phases.iter().zip(rules) {
            let control::Rule::Actuated { bounds, .. } = rule else { continue };
            let found = Finding::PastMax { arm: a.arm, phase: p.uid, need_s: a.need_s, max_s: bounds.max_green_s };
            if rides_it(p, a.arm) && a.need_s > bounds.max_green_s && !out.contains(&found) {
                out.push(found);
            }
        }
    }
    out
}

/// The bicycle calls as a demand on the controller: `calls::Walks` over the phases that run each movement.
pub(crate) fn riders<'a>(plan: &Plan, rules: &[control::Rule], asked: &'a [Asked]) -> Walks<'a> {
    Walks::over(plan, rules, asked, rides_it)
}

/// `out`'s calls and totals from what the run did to every call.
pub(crate) fn tally(out: &mut Summary, run: calls::Run) {
    out.rides = run
        .calls
        .into_iter()
        .map(|c| Ride {
            arm: c.arm,
            at_s: c.at_s,
            outcome: match c.outcome {
                calls::Outcome::Served => Outcome::Served,
                calls::Outcome::Delayed => Outcome::Delayed,
                // A cyclist never leaves (no passive cancelling): what is left is not served.
                calls::Outcome::NotServed | calls::Outcome::Cancelled => Outcome::NotServed,
            },
            wait_s: c.wait_s,
            extended_s: c.extended_s,
        })
        .collect();
    for t in &mut out.totals {
        for r in out.rides.iter().filter(|r| r.arm == t.arm) {
            match r.outcome {
                Outcome::Served => t.served += 1,
                Outcome::Delayed => t.delayed += 1,
                Outcome::NotServed => t.not_served += 1,
            }
            t.calls += 1;
            if r.outcome != Outcome::NotServed {
                t.extended += u32::from(r.extended_s > 0);
                t.extended_s = t.extended_s.saturating_add(r.extended_s);
                t.longest_wait_s = Some(t.longest_wait_s.unwrap_or(0).max(r.wait_s));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::actuation::{self, Actuation};
    use crate::junction::model::control::{Bounds, Demand, Rule, run};
    use crate::junction::model::signal::{self, Move, SignalPlan, StoredPhase};
    use crate::junction::model::{Junction, PRIORITY, Timing, junction_biked};

    fn at(s: &State, bearing: i32) -> u32 {
        s.arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    fn arm_mut(s: &mut State, uid: u32) -> &mut Arm {
        s.arms.iter_mut().find(|a| a.uid == uid).unwrap()
    }

    /// The four-way on a signal whose every street has a bike lane each side and its traffic going straight on: north's
    /// phase runs north's and south's bike movements, east's runs east's and west's.
    fn biked() -> State {
        junction_biked().current().clone()
    }

    /// A phase of street `uid` with a `green_s` green, yellow 3 s and all-red 2 s, walking `walks` and riding `rides`.
    fn phase(uid: u32, green_s: u32, walks: &[u32], rides: &[u32]) -> Phase {
        let timing = Timing { green_s, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        Phase { uid, timing, length_s: timing.length_s(), walks: walks.to_vec(), rides: rides.to_vec(), moves: vec![Move { from: uid, to: 99 }] }
    }

    fn plan_of(phases: Vec<Phase>) -> Plan {
        let cycle_s = phases.iter().map(|p| p.length_s).sum();
        Plan { phases, cycle_s, suggested: false }
    }

    fn cyclists(arm: u32, need_s: u32, at_s: Vec<u32>) -> Asked {
        Asked { arm, need_s, at_s, passive: false }
    }

    /// The calls of `asked` (pedestrian) and `bikes` served by `plan` on `rules`: the turns that ran, as (phase, start,
    /// green, end), each call and each ride.
    fn serve(plan: &Plan, rules: &[Rule], asked: &[Asked], bikes: &[Asked]) -> (Vec<(usize, u64, u32, u64)>, Vec<calls::Call>, Vec<calls::Call>) {
        let mut walks = Walks::new(plan, rules, asked);
        let mut rides = riders(plan, rules, bikes);
        let turns = run(plan, rules, &mut [&mut walks as &mut dyn Demand, &mut rides], 100_000);
        let ran = turns.iter().filter(|t| !t.skipped).map(|t| (t.phase, t.start, t.green_s(), t.end)).collect();
        (ran, walks.finish().calls, rides.finish().calls)
    }

    fn call(arm: u32, at_s: u32, outcome: calls::Outcome, wait_s: u32, extended_s: u32) -> calls::Call {
        calls::Call { arm, at_s, outcome, wait_s, extended_s }
    }

    #[test]
    fn the_placeholders_are_the_ones_the_issue_settled_and_the_clearance_is_the_length_at_4_m_a_second_rounded_up() {
        assert_eq!((MIN_GREEN_S, SPEED_MM_PER_S, MAX_EXTENSION_S), (5, 4_000, 15));
        assert_eq!((DEFAULT_CYCLISTS_PER_HOUR, MAX_CYCLISTS_PER_HOUR), (30, 120));
        assert_eq!(clearance_s(20_000), 5);
        assert_eq!(clearance_s(20_001), 6, "rounded up to the next second");
        assert_eq!(clearance_s(-5), 0);
        assert_eq!(need_s(26_000), 5 + 7);
    }

    #[test]
    fn the_cyclists_of_an_hour_are_the_same_every_time_spread_over_the_hour_and_unlike_the_calls_and_vehicles_of_the_street() {
        let bikes = arrivals(7, 30);
        assert_eq!(bikes, arrivals(7, 30), "seeded");
        assert_eq!(bikes.len(), 30);
        for (k, t) in bikes.iter().enumerate() {
            assert!((k as u32 * 120..(k as u32 + 1) * 120).contains(t), "one in each slot of 120 s: {bikes:?}");
        }
        assert_ne!(arrivals(7, 12), calls::arrivals(7, 12), "a stream of its own, not the pedestrian calls'");
        assert_ne!(arrivals(7, 12), actuation::arrivals(7, 12), "nor the vehicles'");
        assert_eq!(arrivals(7, u32::MAX).len(), 120, "read as the bound");
        assert!(arrivals(7, 0).is_empty());
        // Pinned: a change of generator or seed changes every summary, so it has to be a decision.
        assert_eq!(arrivals(1, 4), PINNED_CYCLISTS);
    }

    const PINNED_CYCLISTS: [u32; 4] = [269, 981, 2149, 2845];

    #[test]
    fn a_cyclist_is_served_when_the_next_phase_that_runs_the_movement_starts_and_delayed_if_they_missed_one() {
        // Street 9's bike movement runs in the first phase (0 to 30 s of each 60 s cycle), not in the second.
        let plan = plan_of(vec![phase(1, 25, &[], &[9]), phase(2, 25, &[], &[])]);
        let (_, _, rides) = serve(&plan, &control::fixed(&plan), &[], &[cyclists(9, 12, vec![0, 10, 40])]);
        assert_eq!(
            rides,
            vec![
                call(9, 0, calls::Outcome::Served, 0, 0),
                call(9, 10, calls::Outcome::Delayed, 50, 0), // its green had started: the next is at 60 s
                call(9, 40, calls::Outcome::Served, 20, 0),
            ]
        );
    }

    #[test]
    fn a_short_green_is_lengthened_by_the_shortfall_within_the_bound_and_a_phase_that_cannot_fit_it_does_not_serve() {
        // 14 s needed and a 10 s green: lengthened by 4 s, the yellow and all-red kept, so the next phase starts later
        // and a cyclist served by it waits that much longer.
        let plan = plan_of(vec![phase(1, 10, &[], &[9]), phase(2, 25, &[], &[8])]);
        let (ran, _, rides) = serve(&plan, &control::fixed(&plan), &[], &[cyclists(9, 14, vec![0]), cyclists(8, 6, vec![0])]);
        assert_eq!(ran[0], (0, 0, 14, 19));
        assert_eq!(ran[1], (1, 19, 25, 49));
        assert_eq!(rides, vec![call(9, 0, calls::Outcome::Served, 0, 4), call(8, 0, calls::Outcome::Served, 19, 0)]);
        // 26 s needed: 16 s past the 10 s green, more than the 15 s bound. Only the third phase (25 s) serves it.
        let plan = plan_of(vec![phase(1, 10, &[], &[9]), phase(2, 25, &[], &[]), phase(3, 25, &[], &[9])]);
        let (_, _, rides) = serve(&plan, &control::fixed(&plan), &[], &[cyclists(9, 26, vec![0])]);
        assert_eq!(rides, vec![call(9, 0, calls::Outcome::Served, 45, 1)]);
        // No phase at all: a finding, and not served.
        let plan = plan_of(vec![phase(1, 10, &[], &[9]), phase(2, 25, &[], &[])]);
        let asked = [cyclists(9, 26, vec![0])];
        assert_eq!(cannot_serve(&plan, &control::fixed(&plan), &asked), vec![Finding::CannotServe { arm: 9, need_s: 26, most_s: 25 }]);
        let (_, _, rides) = serve(&plan, &control::fixed(&plan), &[], &asked);
        assert_eq!(rides, vec![call(9, 0, calls::Outcome::NotServed, 0, 0)]);
    }

    #[test]
    fn with_a_pedestrian_call_in_the_same_phase_the_longer_requirement_is_kept_and_neither_clearance_is_shortened() {
        // One phase walks street 8's crossing and runs street 9's bikes; its green is 10 s. The walk needs 19 s, the
        // bikes 22 s: the green runs 22 s, the longer of the two, and each is served whole.
        let plan = plan_of(vec![phase(1, 10, &[8], &[9]), phase(2, 25, &[], &[])]);
        let rules = control::fixed(&plan);
        let walk = [Asked { arm: 8, need_s: 19, at_s: vec![0], passive: false }];
        let (ran, walks, rides) = serve(&plan, &rules, &walk, &[cyclists(9, 22, vec![0])]);
        assert_eq!(ran[0], (0, 0, 22, 27));
        assert_eq!(walks, vec![call(8, 0, calls::Outcome::Served, 0, 9)], "the walk asked 9 s more of the 10 s green");
        assert_eq!(rides, vec![call(9, 0, calls::Outcome::Served, 0, 12)], "the bikes 12 s");
        // The walk the longer: it is kept, and the bikes' requirement is inside it.
        let walk = [Asked { arm: 8, need_s: 24, at_s: vec![0], passive: false }];
        let (ran, _, rides) = serve(&plan, &rules, &walk, &[cyclists(9, 13, vec![0])]);
        assert_eq!(ran[0].2, 24);
        assert_eq!(rides, vec![call(9, 0, calls::Outcome::Served, 0, 3)]);
        // Neither alone shortens the other: with no cyclist the green is the walk's, with no walk the bikes'.
        assert_eq!(serve(&plan, &rules, &walk, &[]).0[0].2, 24);
        assert_eq!(serve(&plan, &rules, &[], &[cyclists(9, 13, vec![0])]).0[0].2, 13);
    }

    #[test]
    fn a_cyclist_calls_an_actuated_phase_and_a_served_minimum_and_clearance_hold_its_green_past_its_maximum() {
        let bounds = Bounds { min_green_s: 7, gap_s: 3, max_green_s: 10 };
        let plan = plan_of(vec![phase(1, 25, &[], &[9]), phase(2, 25, &[], &[])]);
        let rules = vec![Rule::Actuated { bounds, floor_s: 7 }, Rule::Fixed];
        // Nobody else: the cyclist at 40 s alone calls the actuated phase, which runs its 14 s, past the 10 s maximum.
        let (ran, _, rides) = serve(&plan, &rules, &[], &[cyclists(9, 14, vec![40])]);
        let given: Vec<_> = ran.iter().filter(|r| r.0 == 0).collect();
        assert_eq!(given.len(), 1, "given its green once, for the cyclist: {ran:?}");
        assert_eq!(given[0].2, 14);
        assert_eq!(rides[0].outcome, calls::Outcome::Served);
        assert_eq!(rides[0].extended_s, 7, "7 s past the minimum the rule gave");
        // A need past every bound is still served there: an actuated phase holds its green for it, so no finding.
        assert!(cannot_serve(&plan, &rules, &[cyclists(9, 60, vec![0])]).is_empty());
        // With the vehicles too: the green holds for the cyclist past a gap-out.
        let approach = [actuation::Approach { arm: 1, rate: 1, at_s: vec![40] }];
        let mut vehicles = actuation::Vehicles::new(&plan, &approach);
        let bikes = [cyclists(9, 14, vec![40])];
        let mut rides = riders(&plan, &rules, &bikes);
        let turns = run(&plan, &rules, &mut [&mut vehicles as &mut dyn Demand, &mut rides], 100_000);
        let green = turns.iter().find(|t| t.phase == 0 && !t.skipped).unwrap();
        assert_eq!((green.base_s, green.green_s()), (7, 14), "the vehicle alone gave the minimum; the cyclist held it to 14 s");
    }

    #[test]
    fn on_detection_is_allowed_only_where_the_plan_runs_the_movement() {
        let mut s = biked();
        let (n, e, so) = (at(&s, 0), at(&s, 90), at(&s, 180));
        assert_eq!(refusal(&s, 0, n), None);
        // A plan of one phase that runs north's bikes only: east's are run by no phase.
        s.signal_plan = Some(SignalPlan { phases: vec![StoredPhase { moves: vec![Move { from: n, to: so }], walks: vec![], rides: vec![n], arm: None }] });
        assert_eq!(refusal(&s, 0, n), None);
        assert_eq!(refusal(&s, 0, e), Some(Refusal::BikeNotRidden));
        let mut not_signal = biked();
        not_signal.control = PRIORITY;
        assert_eq!(refusal(&not_signal, 0, n), Some(Refusal::NotSignal));
        assert_eq!(refusal(&biked(), 0, 12_345), Some(Refusal::DoesNotFit));
        // The sample's streets have no bike lane: nothing to run, so nothing to detect.
        let sample = Junction::new(0);
        let uid = sample.current().arms[0].uid;
        assert_eq!(refusal(sample.current(), 0, uid), Some(Refusal::BikeNotRidden));
    }

    #[test]
    fn a_movement_is_put_on_detection_given_its_cyclists_within_the_bound_taken_off_and_undone_like_any_edit() {
        let mut j = junction_biked();
        let n = at(j.current(), 0);
        assert!(!j.set_cyclists(n, 10));
        assert_eq!(j.refusal(), Some(Refusal::NotOnBikeDetection));
        assert!(j.set_bike_detection(n, true));
        assert_eq!((j.arm(n).unwrap().cyclists, j.current().label.key), (Some(DEFAULT_CYCLISTS_PER_HOUR), "jn-rev-bike-detection-on"));
        assert!(!j.set_bike_detection(n, true));
        assert_eq!(j.refusal(), Some(Refusal::BikeDetectionAlready));
        assert!(j.set_cyclists(n, 0), "no cyclists at all is allowed");
        assert!(j.set_cyclists(n, 120));
        assert_eq!((j.arm(n).unwrap().cyclists, j.current().label.key), (Some(120), "jn-rev-cyclists"));
        for bad in [-1, 121] {
            assert!(!j.set_cyclists(n, bad), "{bad}");
            assert_eq!(j.refusal(), Some(Refusal::CyclistsOutOfRange));
        }
        let edits = j.revisions().count();
        assert!(j.set_cyclists(n, 120), "the rate it has: taken");
        assert_eq!(j.revisions().count(), edits, "and no edit");
        assert!(j.set_bike_detection(n, false));
        assert_eq!((j.arm(n).unwrap().cyclists, j.current().label.key), (None, "jn-rev-bike-detection-off"));
        assert!(j.undo() && j.arm(n).unwrap().cyclists == Some(120));
        // Off a signal the detection goes, as the timing does, and the revision says the timing went.
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.arm(n).unwrap().cyclists, None);
        assert_eq!(j.current().label.key, "jn-rev-control-timing");
        assert!(!j.set_bike_detection(n, true));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
        // On the sample, which runs no bikes, it is refused with that reason.
        let mut sample = Junction::new(0);
        let uid = sample.current().arms[0].uid;
        assert!(!sample.set_bike_detection(uid, true));
        assert_eq!(sample.refusal(), Some(Refusal::BikeNotRidden));
    }

    #[test]
    fn a_junction_with_no_movement_on_detection_is_written_as_before_and_its_cyclists_only_once_set() {
        let mut s = biked();
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("cyclists"), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert!(back.arms.iter().all(|a| a.cyclists.is_none()));
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        s.arms[0].cyclists = Some(30);
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""cyclists":30"#), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.arms[0].cyclists, Some(30));
        // A document's rate past the bound is read as the bound.
        s.arms[0].cyclists = Some(5_000);
        assert_eq!(rate_of(&s.arms[0]), Some(MAX_CYCLISTS_PER_HOUR));
    }

    #[test]
    fn a_cyclist_clears_the_junction_from_their_street_s_mouth_to_the_farthest_mouth_their_traffic_goes_to() {
        use crate::junction::model::geometry::dist;
        use crate::junction::model::read_model::ride_lengths;
        let j = junction_biked();
        let (s, v) = (j.current().clone(), j.view());
        let mouth = |uid: u32| v.arms.iter().find(|a| a.uid == uid).unwrap().mouth_at;
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        let lengths = ride_lengths(&s, &v.arms);
        assert_eq!(lengths.len(), 4);
        let of = |uid: u32| lengths.iter().find(|(u, _)| *u == uid).unwrap().1;
        assert_eq!(of(n), dist(mouth(n), mouth(so)).round() as i32, "north's traffic goes south");
        assert!(of(n) > 10_000, "across the junction: {}", of(n));
        // A street that sends nothing: the farthest other mouth.
        let mut quiet = s.clone();
        arm_mut(&mut quiet, e).lanes.clear();
        let far = [n, so, w].map(|u| dist(mouth(e), mouth(u))).into_iter().fold(0.0, f64::max);
        assert_eq!(ride_lengths(&quiet, &v.arms).iter().find(|(u, _)| *u == e).unwrap().1, far.round() as i32);
    }

    #[test]
    fn a_movement_whose_need_outlasts_an_actuated_phase_s_maximum_is_a_finding_and_its_green_still_held() {
        let mut s = biked();
        let n = at(&s, 0);
        arm_mut(&mut s, n).cyclists = Some(30);
        arm_mut(&mut s, n).actuation = Some(Actuation { max_green_s: 10, ..Actuation::default() });
        let plan = signal::plan(&s, 0).unwrap();
        // 40 m to clear: 5 s + 10 s = 15 s, past north's 10 s maximum.
        let h = control::hour(&s, &plan, &[], &[(n, 40_000)]);
        assert_eq!(h.bicycles.findings, vec![Finding::PastMax { arm: n, phase: n, need_s: 15, max_s: 10 }]);
        assert_eq!(h.bicycles.totals[0].not_served, 0, "served all the same");
        assert!(h.bicycles.rides.iter().all(|r| r.outcome != Outcome::NotServed));
        // Within the maximum: no finding.
        assert!(control::hour(&s, &plan, &[], &[(n, 20_000)]).bicycles.findings.is_empty());
        // A fixed phase has no maximum to outlast.
        arm_mut(&mut s, n).actuation = None;
        assert!(control::hour(&s, &plan, &[], &[(n, 40_000)]).bicycles.findings.is_empty());
    }

    #[test]
    fn with_no_movement_on_detection_the_hour_is_as_before() {
        let mut s = biked();
        let e = at(&s, 90);
        arm_mut(&mut s, e).calls = Some(12);
        let plan = signal::plan(&s, 0).unwrap();
        let h = control::hour(&s, &plan, &[(e, 9_000)], &[(e, 26_000)]);
        assert_eq!(h.bicycles, Summary::default());
        assert_eq!(h.calls, control::hour(&s, &plan, &[(e, 9_000)], &[]).calls);
        assert_eq!(h.calls.calls.len(), 12);
    }

    #[test]
    fn a_movement_on_detection_that_no_phase_runs_any_more_is_a_finding_and_kept() {
        let mut s = biked();
        let (n, so) = (at(&s, 0), at(&s, 180));
        arm_mut(&mut s, n).cyclists = Some(30);
        s.signal_plan = Some(SignalPlan { phases: vec![StoredPhase { moves: vec![Move { from: so, to: n }], walks: vec![], rides: vec![so], arm: None }] });
        let plan = signal::plan(&s, 0).unwrap();
        let h = control::hour(&s, &plan, &[], &[(n, 26_000)]);
        assert_eq!(h.bicycles.findings, vec![Finding::NotRidden { arm: n }]);
        assert!(h.bicycles.totals.is_empty() && h.bicycles.rides.is_empty());
        assert_eq!(s.arms.iter().find(|a| a.uid == n).unwrap().cyclists, Some(30), "kept on detection");
    }

    #[test]
    fn fixed_inputs_give_a_fixed_summary() {
        let mut s = biked();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        // North's bikes on detection (30 an hour, 120 m to clear: 5 s + 30 s, past every green of the plan, so its greens
        // are lengthened); east's crossing on call and east actuated.
        arm_mut(&mut s, n).cyclists = Some(30);
        arm_mut(&mut s, e).calls = Some(6);
        arm_mut(&mut s, e).actuation = Some(Actuation { vehicles: 60, ..Actuation::default() });
        let plan = signal::plan(&s, 0).unwrap();
        let distances = [(n, 9_000), (e, 9_000), (so, 9_000), (w, 9_000)];
        let lengths = [(n, 120_000), (e, 26_000), (so, 26_000), (w, 26_000)];
        let h = control::hour(&s, &plan, &distances, &lengths);
        assert_eq!(h, control::hour(&s, &plan, &distances, &lengths), "deterministic");
        let pinned = Totals { arm: n, rate: 30, length_mm: 120_000, clearance_s: 30, need_s: 35, ..Totals::default() };
        // Every call's green lengthened by 10 s (north's 25 s green to the 35 s its cyclists need).
        let pinned = Totals { calls: 30, served: 14, delayed: 16, not_served: 0, extended: 30, extended_s: 300, longest_wait_s: Some(60), ..pinned };
        assert_eq!(h.bicycles.totals, vec![pinned], "{:#?}", h.bicycles.totals);
        assert!(h.bicycles.findings.is_empty());
        assert_eq!(h.bicycles.rides.len(), 30);
        let ride = |at_s, outcome, wait_s| Ride { arm: n, at_s, outcome, wait_s, extended_s: 10 };
        assert_eq!(&h.bicycles.rides[..3], &[ride(105, Outcome::Served, 6), ride(181, Outcome::Delayed, 51), ride(281, Outcome::Served, 12)]);
    }
}
