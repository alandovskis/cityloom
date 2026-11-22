//! The signal controller over one hour: one deterministic loop that runs the plan's phases in turn and decides each
//! phase's green from what is waiting for it. What waits is a `Demand`: the pedestrian calls (`calls.rs`, #60 and
//! CLM-17) are one, vehicles detected on an actuated approach (`actuation.rs`, CLM-12) another, cyclists detected on a
//! bike movement's approach (`bicycles.rs`, CLM-13) a third, buses given transit priority (`transit.rs`, CLM-14) a
//! fourth, emergency vehicle pre-emption (`preemption.rs`, CLM-15) a fifth, each keeping its own rules and adding
//! nothing to the loop but its answers. CityLoom models the behaviour to compare strategies; it never controls a signal.
//! No DOM, no signals, no I/O, no wording.
//!
//! A phase is `Fixed` (it runs every time, at its timing's green) or `Actuated` (it runs only when a demand calls it,
//! its green decided between bounds by the demands that extend it). Either way the yellow and the all-red are the
//! timing's and are never shortened, and a demand that holds the green (a walk and its clearance being served) is
//! given the whole of it. A demand may cut a green short (an early green for a bus whose phase comes later), never below
//! what is held, or hold it a little longer for one coming just after it would end (a green extension). Above them all,
//! a demand may interrupt a green (an emergency call: it ends once every clearance in progress is done, no other demand
//! cutting or holding it) and run a phase out of turn, the phase it displaced running after it (pre-emption).

use crate::junction::model::signal::Plan;
use crate::junction::model::{State, actuation, bicycles, calls, preemption, transit};

/// The span the demands are spread over: one hour.
pub const HOUR_S: u32 = 3_600;

/// One step of SplitMix64: a small fixed generator, so the arrivals are the same on every machine and in every build.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `rate` arrivals spread over the hour (the seeded generator of #60): the hour is cut into `rate` equal slots and each
/// slot gets one arrival at a point the generator, seeded with `seed`, picks. In order, each second at most once, so
/// `rate` is at most `HOUR_S`. Each demand seeds it on a stream of its own.
pub fn spread(mut seed: u64, rate: u32) -> Vec<u32> {
    let rate = rate.min(HOUR_S);
    (0..rate)
        .map(|k| {
            let (lo, hi) = (k * HOUR_S / rate, (k + 1) * HOUR_S / rate);
            lo + (splitmix(&mut seed) % u64::from(hi - lo)) as u32
        })
        .collect()
}

/// The hour of a signal: how its plan serves the pedestrian calls, the actuated streets' vehicles, the bicycle calls,
/// the buses of the streets with transit priority and the emergency calls of the streets with pre-emption together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hour {
    pub calls: calls::Summary,
    pub actuation: actuation::Summary,
    pub bicycles: bicycles::Summary,
    pub transit: transit::Summary,
    pub preemption: preemption::Summary,
}

/// The kind of road user a demand stands for: what a pre-emption counts the calls it pre-empts or delays by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Kind {
    Pedestrian,
    Bicycle,
    Bus,
    Vehicle,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Pedestrian, Kind::Bicycle, Kind::Bus, Kind::Vehicle];
}

/// Runs `plan` on `s` over an hour: the phases by their rules (`actuation::rules`), the pedestrian calls at the crossings
/// on call, the vehicles of the actuated streets, the cyclists of the bike movements on detection and the buses of the
/// streets with transit priority as its demands, and the emergency calls of the streets with pre-emption above them all.
/// `distances` is each crossing's length by street uid (`read_model::crossing_distances`), `rides` how far each street's
/// cyclists ride to clear the junction (`read_model::ride_lengths`). With no street actuated every phase is fixed and the
/// calls are served as #60 and CLM-17 serve them; with no bike movement on detection the cyclists change nothing, and
/// with no street given priority the buses change nothing, and with no street given pre-emption nothing is pre-empted.
pub fn hour(s: &State, plan: &Plan, distances: &[(u32, i32)], rides: &[(u32, i32)]) -> Hour {
    let rules = actuation::rules(s, plan, distances);
    let mut out = Hour::default();
    let asked = calls::asked(s, plan, distances, &mut out.calls);
    out.calls.findings.extend(calls::cannot_serve(plan, &rules, &asked));
    let (approaches, found) = actuation::approaches(s, plan, &rules, distances);
    out.actuation.findings = found;
    let ridden = bicycles::asked(s, plan, rides, &mut out.bicycles);
    out.bicycles.findings.extend(bicycles::cannot_serve(plan, &rules, &ridden));
    out.bicycles.findings.extend(bicycles::past_max(plan, &rules, &ridden));
    let mut walks = calls::Walks::new(plan, &rules, &asked);
    let mut vehicles = actuation::Vehicles::new(plan, &approaches);
    let mut riders = bicycles::riders(plan, &rules, &ridden);
    let streets = transit::streets(s, plan, &mut out.transit);
    let mut buses = transit::Buses::new(plan, &rules, &streets, transit::kept(s, plan, &rules, distances, rides));
    let emergencies = preemption::streets(s, plan, distances, rides, &mut out.preemption);
    let mut emergency = preemption::Emergencies::new(plan, &emergencies, preemption::recall(s, plan, distances, rides));
    let turns = run(plan, &rules, &mut [&mut walks, &mut vehicles, &mut riders, &mut buses, &mut emergency], calls::MAX_TURNS);
    calls::tally(&mut out.calls, &asked, walks.finish());
    bicycles::tally(&mut out.bicycles, riders.finish());
    transit::tally(&mut out.transit, buses.finish());
    preemption::tally(&mut out.preemption, emergency.finish());
    out.actuation.approaches = vehicles.finish();
    out.actuation.phases = actuation::phase_totals(plan, &rules, &turns);
    out
}

/// What bounds an actuated phase's green, in seconds: the least it runs, how long a gap with no arrival ends it, and
/// the most it runs for the vehicles alone (a walk being served still holds it longer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub min_green_s: u32,
    pub gap_s: u32,
    pub max_green_s: u32,
}

/// How a phase decides its green.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Runs every time its turn comes, at its timing's green, lengthened only to hold what a demand is served.
    Fixed,
    /// Runs only when some demand calls it, else it is skipped (taking no time). Its green runs at least `floor_s` (the
    /// minimum green, or more when a walk on recall runs with it), then as long as the demands extend it, never past the
    /// maximum green for them; a demand that holds it (a walk served) lengthens it past that.
    Actuated { bounds: Bounds, floor_s: u32 },
}

/// One turn of a phase, as the controller ran it: when it started, its green (from `green_start` to `green_end`, after
/// the transit lead), when the next phase starts, and the green its rule gave before any demand held it (`base_s`); the
/// longest any demand held it for (`held_s`), the seconds a demand cut it short (`cut_s`) and then held it longer
/// (`prolonged_s`), and the seconds an interruption took off it (`preempted_s`, pre-emption), and whether it ran for a
/// demand above every other (`kept`, a pre-emption call: no other demand cut it, and what held it longer was that
/// call's). A skipped phase ends where it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turn {
    pub phase: usize,
    pub start: u64,
    pub green_start: u64,
    pub green_end: u64,
    pub end: u64,
    pub base_s: u32,
    pub held_s: u32,
    pub cut_s: u32,
    pub prolonged_s: u32,
    pub preempted_s: u32,
    pub kept: bool,
    pub skipped: bool,
}

impl Turn {
    /// The green the phase was given, in seconds.
    pub fn green_s(&self) -> u32 {
        u32::try_from(self.green_end - self.green_start).unwrap_or(u32::MAX)
    }

    /// The green its rule and every hold gave it, before a demand cut it short, held it longer or interrupted it: the
    /// green that ran is always this `- cut_s + prolonged_s - preempted_s`.
    pub fn given_s(&self) -> u32 {
        self.green_s().saturating_add(self.cut_s).saturating_add(self.preempted_s).saturating_sub(self.prolonged_s)
    }
}

/// Something that waits for phases: it says whether it calls a phase, how long it extends or holds its green, and is
/// told each turn as it ran, so it can record who was served. Times are seconds into the hour on the run's u64 clock.
pub trait Demand {
    /// Whether someone waiting at `at` calls phase `phase` (an actuated phase no demand calls is skipped).
    fn calls(&self, phase: usize, at: u64) -> bool;
    /// The green, from `green_start`, that this demand's arrivals keep actuated phase `phase` in within `bounds`, the
    /// green running at least `floor_s` whatever comes: once that is done a gap with no arrival ends it, and arrivals
    /// never hold it past the maximum. 0 when it extends none.
    fn extend(&self, _phase: usize, _green_start: u64, _bounds: Bounds, _floor_s: u32) -> u32 {
        0
    }
    /// The least green phase `phase`, starting at `start`, must give for what this demand is served in it: never
    /// shortened, whatever the phase's rule. 0 when it holds nothing.
    fn hold(&self, phase: usize, start: u64) -> u32;
    /// The phase ran (or was skipped) as `turn` says.
    fn ran(&mut self, turn: &Turn);
    /// Whether anyone is still waiting, or still to come.
    fn pending(&self) -> bool;
    /// When the next one comes after `at`, if anyone does: the controller rests until then when nothing calls a phase.
    fn next_after(&self, at: u64) -> Option<u64>;
    /// When the last one comes.
    fn last(&self) -> u64;
    /// The most it lengthens a fixed phase's green past the timing's.
    fn most_extension(&self) -> u32 {
        0
    }
    /// The longest green it can hold actuated phase `phase` for.
    fn longest_hold(&self, _phase: usize) -> u32 {
        0
    }
    /// The green phase `phase`, its green starting at `green_start` and given `green_s` (every hold in it, `held_s` the
    /// longest), is to have instead: shorter to cut it short, never below `held_s` (the loop keeps that whatever is
    /// asked). `green_s` when it cuts nothing.
    fn cut(&self, _phase: usize, _green_start: u64, green_s: u32, _held_s: u32) -> u32 {
        green_s
    }
    /// Whether phase `phase`'s green (the phase started at `start`, its green running from `green_start` to `green_end`)
    /// runs for this demand above every other (a pre-emption call): then no demand cuts it short or interrupts it.
    fn keeps(&self, _phase: usize, _start: u64, _green_start: u64, _green_end: u64) -> bool {
        false
    }
    /// The seconds phase `phase`, started at `start`, holds its green past `green_end`, where it would end. 0 when it
    /// holds it no longer.
    fn prolong(&self, _phase: usize, _start: u64, _green_end: u64) -> u32 {
        0
    }
    /// The most it holds any phase's green past where it would end (`prolong`).
    fn most_prolong(&self) -> u32 {
        0
    }
    /// What kind of road user it stands for, if any: who a pre-emption counts as pre-empted or delayed.
    fn kind(&self) -> Option<Kind> {
        None
    }
    /// How many of its arrivals have come by `at` and are not yet served that one of `phases` serves.
    fn waiting(&self, _at: u64, _phases: &[usize]) -> u32 {
        0
    }
    /// The phase to run now, out of turn, in place of `planned` (which then runs after it), if this demand takes the
    /// signal there (pre-emption). It calls the phase it names.
    fn jump(&self, _planned: usize, _at: u64) -> Option<usize> {
        None
    }
    /// The second at which phase `phase`'s green (the phase started at `start`, its green running from `green_start` to
    /// `green_end`, where it would end once cut short or held longer) is to end for a call above every other demand
    /// (pre-emption), if one comes before `green_end`: the green then ends there, or once every clearance in progress
    /// then is done (`clear_by`), and no other demand cuts or holds it past that.
    fn interrupt(&self, _phase: usize, _start: u64, _green_start: u64, _green_end: u64) -> Option<u64> {
        None
    }
    /// When what this demand is served in phase `phase`'s green (the phase started at `start`, its green from
    /// `green_start`) can stop at `at` without cutting a clearance in progress: `at` when none is. A walk (or a bicycle
    /// minimum green) still showing at `at` ends there and its clearance runs in full; a clearance under way runs out.
    fn clear_by(&self, _phase: usize, _start: u64, _green_start: u64, at: u64) -> u64 {
        at
    }
    /// Whether it counts who else waits as the turns run (`census`).
    fn counts(&self) -> bool {
        false
    }
    /// Told before (`told` false) and after (`told` true) every demand is told `turn`, with `others(kind, at, phases)`:
    /// how many road users of that kind the other demands have waiting at `at` for one of `phases`.
    fn census(&mut self, _turn: &Turn, _told: bool, _others: &dyn Fn(Kind, u64, &[usize]) -> u32) {}
    /// How many phases it may run out of turn after the last arrival (`stop_at`).
    fn out_of_turn(&self) -> u32 {
        0
    }
}

/// Every phase of `plan` fixed: the plan as a fixed-time signal runs it.
pub fn fixed(plan: &Plan) -> Vec<Rule> {
    vec![Rule::Fixed; plan.phases.len()]
}

/// The phases of `plan` run in turn from second 0, round and round, by their `rules`, until no demand has anyone waiting
/// or still to come (and, when a phase is actuated, the hour is over), the run stops a cycle at its longest past the
/// last arrival or the hour (`stop_at`), or `turns` phases have run.
///
/// Each turn: an actuated phase that no demand calls is skipped, taking no time; when every phase in a row of the plan's
/// length is skipped, nothing is waiting, and the controller rests (all red) until the next arrival. A phase that runs
/// is given the green its rule gives (a fixed phase its timing's; an actuated one its floor, extended by the demands up
/// to its maximum), lengthened to the longest any demand holds it for (an actuated phase's arrivals still extending it
/// past that hold, within the maximum), then cut short as far as a demand asks (never below that hold) and held longer
/// as far as one asks; its transit lead, yellow and all-red are its timing's. A phase takes at least a second, and the clock counts in u64 (a settled timing is within its range, but
/// the run holds any), so time moves on. Every turn, run or skipped, is told to every demand and returned.
///
/// Above that, a demand may take the signal (pre-emption): it runs a phase out of turn (`jump`; the phase it displaced
/// runs next, and the plan goes on from there), and it may interrupt a green (`interrupt`), which then ends at the
/// interruption or once every clearance in progress is done (`clear_by`), whatever any other demand asks; its yellow
/// and all-red still run in full.
pub fn run(plan: &Plan, rules: &[Rule], demands: &mut [&mut dyn Demand], turns: usize) -> Vec<Turn> {
    let n = plan.phases.len();
    let mut out = Vec::new();
    if n == 0 || rules.len() != n {
        return out;
    }
    // With a phase actuated the run covers the hour, so its skips are counted to the end of it, not only until the
    // last arrival is served; a plan of fixed phases stops as soon as nobody is waiting or still to come.
    let cover = if rules.iter().any(|r| matches!(r, Rule::Actuated { .. })) { u64::from(HOUR_S) } else { 0 };
    let last = demands.iter().map(|d| d.last()).max().unwrap_or(0).max(cover);
    let stop = stop_at(plan, rules, last, demands);
    let (mut start, mut turn, mut ran, mut idle) = (0u64, 0usize, 0usize, 0usize);
    while (start < cover || demands.iter().any(|d| d.pending())) && ran < turns && start <= stop {
        // A demand that takes the signal runs its phase out of turn; the phase it displaced runs after it.
        let planned = turn % n;
        let k = demands.iter().find_map(|d| d.jump(planned, start)).filter(|&j| j < n).unwrap_or(planned);
        if k == planned {
            turn += 1;
        }
        let phase = &plan.phases[k];
        let lead = u64::from(phase.timing.transit_lead_s);
        if let Rule::Actuated { .. } = rules[k]
            && !demands.iter().any(|d| d.calls(k, start))
        {
            let skipped = Turn {
                phase: k,
                start,
                green_start: start,
                green_end: start,
                end: start,
                base_s: 0,
                held_s: 0,
                cut_s: 0,
                prolonged_s: 0,
                preempted_s: 0,
                kept: false,
                skipped: true,
            };
            for d in demands.iter_mut() {
                d.ran(&skipped);
            }
            out.push(skipped);
            idle += 1;
            if idle >= n {
                // Nothing calls any phase: rest until someone comes, or stop when nobody will.
                let Some(next) = demands.iter().filter_map(|d| d.next_after(start)).min() else { break };
                start = next;
                idle = 0;
            }
            continue;
        }
        idle = 0;
        let green_start = start + lead;
        let hold = demands.iter().map(|d| d.hold(k, start)).max().unwrap_or(0);
        // The green the phase's rule gives (`base_s`), and the green once every hold is given in full: a fixed phase's
        // timing, lengthened to the longest hold; an actuated phase's floor, extended by the arrivals, and when a hold is
        // longer, that hold extended by the arrivals still coming within a gap of its end.
        let (base_s, green_s) = match rules[k] {
            Rule::Fixed => (phase.timing.green_s, phase.timing.green_s.max(hold)),
            Rule::Actuated { bounds, floor_s } => {
                let extended = |floor: u32| demands.iter().map(|d| d.extend(k, green_start, bounds, floor)).max().unwrap_or(0).max(floor);
                let base = extended(floor_s);
                (base, if hold > base { extended(hold) } else { base })
            }
        };
        let (green_s, cut_s, prolonged_s, preempted_s, kept) = settle(demands, k, (start, green_start), green_s, hold);
        // The phase's length with this green in place of the timing's: the transit lead, yellow and all-red kept.
        let length = u64::from(phase.length_s).saturating_sub(u64::from(phase.timing.green_s)) + u64::from(green_s);
        let end = start + length.max(1);
        let green_end = green_start + u64::from(green_s);
        let done = Turn { phase: k, start, green_start, green_end, end, base_s, held_s: hold, cut_s, prolonged_s, preempted_s, kept, skipped: false };
        census(demands, &done, false);
        for d in demands.iter_mut() {
            d.ran(&done);
        }
        census(demands, &done, true);
        out.push(done);
        start = end;
        ran += 1;
    }
    out
}

/// How phase `k`'s green of `green_s` (the phase started at `start`, its green from `green_start`, `hold` of it held by
/// a demand) ends once the demands have their say: its green, the seconds cut short, held longer and interrupted, and
/// whether it was kept (`(green_s, cut_s, prolonged_s, preempted_s, kept)`; the green is always `green_s - cut_s +
/// prolonged_s - preempted_s`). A green running for a demand above every other (pre-emption) is kept: no other cuts,
/// interrupts or holds it, only the demand it runs for holding it longer. Otherwise it is cut short for a demand (transit's early green for a later phase), never below what is
/// held; then any green is held longer for one coming just after it would end (transit's green extension,
/// pre-emption's dwell). A green not kept is then interrupted by a call coming before that end: it ends at the call, or
/// once every clearance in progress is done; ended before the cut, nothing else cut it, and ended in a green held
/// longer, it is held only to there.
fn settle(demands: &[&mut dyn Demand], k: usize, (start, green_start): (u64, u64), green_s: u32, hold: u32) -> (u32, u32, u32, u32, bool) {
    let keeps = |d: &&&mut dyn Demand| d.keeps(k, start, green_start, green_start + u64::from(green_s));
    let kept = demands.iter().any(|d| keeps(&d));
    let cut_to =
        if kept { green_s } else { demands.iter().map(|d| d.cut(k, green_start, green_s, hold)).min().unwrap_or(green_s).max(hold.min(green_s)).min(green_s) };
    let prolonged_s = demands.iter().filter(|d| !kept || keeps(d)).map(|d| d.prolong(k, start, green_start + u64::from(cut_to))).max().unwrap_or(0);
    let planned = cut_to + prolonged_s;
    let interrupted = if kept { None } else { demands.iter().filter_map(|d| d.interrupt(k, start, green_start, green_start + u64::from(planned))).min() };
    if let Some(at) = interrupted {
        let at = at.max(green_start);
        let clear = demands.iter().map(|d| d.clear_by(k, start, green_start, at)).max().unwrap_or(at).max(at);
        let to = u32::try_from(clear - green_start).unwrap_or(u32::MAX).min(planned);
        if to < planned {
            return if to <= cut_to { (to, 0, 0, green_s - to, false) } else { (to, green_s - cut_to, prolonged_s, planned - to, false) };
        }
    }
    (planned, green_s - cut_to, prolonged_s, 0, kept)
}

/// Tells each demand that counts (`Demand::counts`) who the others have waiting, before (`told` false) or after `turn`
/// is told to them all.
fn census(demands: &mut [&mut dyn Demand], turn: &Turn, told: bool) {
    for i in 0..demands.len() {
        if !demands[i].counts() {
            continue;
        }
        let (before, rest) = demands.split_at_mut(i);
        let Some((me, after)) = rest.split_first_mut() else { continue };
        let others = |kind: Kind, at: u64, phases: &[usize]| -> u32 {
            before.iter().chain(after.iter()).filter(|d| d.kind() == Some(kind)).map(|d| d.waiting(at, phases)).sum()
        };
        me.census(turn, told, &others);
    }
}

/// When the run stops: a cycle at its longest past the `last` arrival. A fixed phase takes a second or more, and its
/// green is lengthened by at most the most any demand lengthens one; an actuated phase runs at most its transit lead,
/// the longest of its maximum, its floor and any hold, its yellow and its all-red; and any phase's green may be held
/// longer by the most any demand prolongs one; a phase run out of turn (pre-emption) adds the longest phase once for
/// each the demands may run after the last arrival. So whoever can be served is, at the
/// latest, a cycle that long after they came (one who misses a green waits for the next): the bound is a guard, and
/// cuts off nobody.
pub fn stop_at(plan: &Plan, rules: &[Rule], last: u64, demands: &[&mut dyn Demand]) -> u64 {
    let slack = u64::from(demands.iter().map(|d| d.most_extension()).max().unwrap_or(0));
    let prolong = u64::from(demands.iter().map(|d| d.most_prolong()).max().unwrap_or(0));
    let longest = |k: usize| {
        let p = &plan.phases[k];
        prolong
            + match rules[k] {
                Rule::Fixed => u64::from(p.length_s).max(1) + slack,
                Rule::Actuated { bounds, floor_s } => {
                    let hold = demands.iter().map(|d| d.longest_hold(k)).max().unwrap_or(0);
                    let green = bounds.max_green_s.max(floor_s).max(hold);
                    (u64::from(p.length_s).saturating_sub(u64::from(p.timing.green_s)) + u64::from(green)).max(1)
                }
            }
    };
    let out_of_turn = u64::from(demands.iter().map(|d| d.out_of_turn()).sum::<u32>());
    last + (0..plan.phases.len()).map(longest).sum::<u64>() + out_of_turn * (0..plan.phases.len()).map(longest).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::Timing;
    use crate::junction::model::signal::Phase;

    fn phase(uid: u32, green_s: u32) -> Phase {
        let timing = Timing { green_s, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        Phase { uid, timing, length_s: timing.length_s(), walks: vec![], rides: vec![], moves: vec![] }
    }

    fn plan_of(phases: Vec<Phase>) -> Plan {
        let cycle_s = phases.iter().map(|p| p.length_s).sum();
        Plan { phases, cycle_s, suggested: false }
    }

    /// A demand that calls the phases in `wants` at the seconds in `at`, holds none and is served at its phase's start.
    struct Someone {
        wants: Vec<usize>,
        at: Vec<u64>,
        served: Vec<(u64, u64)>,
    }

    impl Demand for Someone {
        fn calls(&self, phase: usize, at: u64) -> bool {
            self.wants.contains(&phase) && self.at.iter().any(|&t| t <= at)
        }
        fn hold(&self, _phase: usize, _start: u64) -> u32 {
            0
        }
        fn ran(&mut self, turn: &Turn) {
            if !turn.skipped && self.wants.contains(&turn.phase) {
                let (now, later): (Vec<u64>, Vec<u64>) = self.at.iter().partition(|&&t| t <= turn.green_end);
                self.served.extend(now.into_iter().map(|t| (t, turn.green_start)));
                self.at = later;
            }
        }
        fn pending(&self) -> bool {
            !self.at.is_empty()
        }
        fn next_after(&self, at: u64) -> Option<u64> {
            self.at.iter().copied().filter(|&t| t > at).min()
        }
        fn last(&self) -> u64 {
            self.at.iter().copied().max().unwrap_or(0)
        }
    }

    const BOUNDS: Bounds = Bounds { min_green_s: 7, gap_s: 3, max_green_s: 45 };

    #[test]
    fn fixed_phases_run_every_turn_at_their_timing_and_keep_their_yellow_and_all_red() {
        let plan = plan_of(vec![phase(1, 25), phase(2, 10)]);
        let mut d = Someone { wants: vec![1], at: vec![50], served: vec![] };
        let turns = run(&plan, &fixed(&plan), &mut [&mut d], 100);
        assert_eq!(turns.iter().map(|t| (t.start, t.green_s(), t.end)).collect::<Vec<_>>(), vec![(0, 25, 30), (30, 10, 45), (45, 25, 75), (75, 10, 90)]);
        assert_eq!(d.served, vec![(50, 75)]);
    }

    #[test]
    fn an_actuated_phase_nobody_calls_is_skipped_and_the_controller_rests_until_someone_comes() {
        let plan = plan_of(vec![phase(1, 25), phase(2, 25)]);
        let rules = vec![Rule::Actuated { bounds: BOUNDS, floor_s: 7 }; 2];
        let mut d = Someone { wants: vec![1], at: vec![100], served: vec![] };
        let turns = run(&plan, &rules, &mut [&mut d], 100);
        // Both skipped at 0 s, then rest until 100 s; the first is skipped again, the second runs its 7 s minimum; then
        // both are skipped and, nobody else coming, the signal rests the hour out.
        assert_eq!(
            turns.iter().map(|t| (t.phase, t.start, t.skipped, t.green_s())).collect::<Vec<_>>(),
            vec![(0, 0, true, 0), (1, 0, true, 0), (0, 100, true, 0), (1, 100, false, 7), (0, 112, true, 0), (1, 112, true, 0)]
        );
        assert_eq!(turns[3].end, 112, "7 s of green, the yellow and the all-red kept");
        assert_eq!(d.served, vec![(100, 100)]);
    }

    #[test]
    fn with_a_phase_actuated_the_run_covers_the_hour_and_counts_each_skip_even_with_nobody_coming() {
        // An actuated phase nobody calls beside a fixed one of 30 s: the fixed one runs every 30 s through the hour, and
        // the actuated one is skipped as often.
        let plan = plan_of(vec![phase(1, 25), phase(2, 25)]);
        let rules = vec![Rule::Actuated { bounds: BOUNDS, floor_s: 7 }, Rule::Fixed];
        let mut d = Someone { wants: vec![], at: vec![], served: vec![] };
        let turns = run(&plan, &rules, &mut [&mut d], 100_000);
        assert_eq!(turns.iter().filter(|t| t.skipped).count(), 120);
        assert_eq!(turns.iter().filter(|t| !t.skipped).count(), 120);
        assert!(turns.iter().all(|t| t.start < u64::from(HOUR_S)));
        // Every phase actuated and nobody coming: each is skipped once, and the signal rests the hour out.
        let rules = vec![Rule::Actuated { bounds: BOUNDS, floor_s: 7 }; 2];
        assert_eq!(run(&plan, &rules, &mut [&mut d], 100_000).len(), 2);
    }

    #[test]
    fn with_nothing_waiting_or_to_come_the_run_stops() {
        let plan = plan_of(vec![phase(1, 25)]);
        let mut d = Someone { wants: vec![0], at: vec![], served: vec![] };
        assert!(run(&plan, &fixed(&plan), &mut [&mut d], 100).is_empty());
        assert!(run(&plan_of(vec![]), &[], &mut [&mut d], 100).is_empty());
    }
}
