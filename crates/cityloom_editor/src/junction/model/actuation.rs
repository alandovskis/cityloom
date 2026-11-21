//! Vehicle demand-responsive actuation (CLM-12): a street with entering lanes is on recall (its phase runs every cycle at
//! its timing's green, the default) or actuated (detectors on its approach call and extend its phase). A seeded
//! generator spreads each actuated approach's vehicles over one hour, and the controller (`control::run`) runs the plan
//! over them and the pedestrian calls together. Deterministic: the same junction, plan and settings always give the same
//! hour. CityLoom models the behaviour to compare strategies; it never controls a signal. No DOM, no signals, no I/O, no
//! wording.
//!
//! The rule of an actuated phase (the phase whose own street, `Phase.uid`, is actuated, and which serves movements):
//! - A vehicle arriving while no phase serving its approach is green waits for the next one that is.
//! - The phase is skipped (takes no time) when nobody calls it: no vehicle waiting on an approach it serves and no
//!   pedestrian call it would serve. A crossing on recall walked in it does not call it.
//! - Once green it runs at least its floor: the minimum green, or the walk and clearance of a crossing on recall walked
//!   in it when longer. Each vehicle arriving while it is green holds it for the passage (gap) time after that arrival
//!   (timed even while the floor runs), never past the maximum green: once the floor is done it ends at the first gap
//!   with no arrival ("gap-out", ended early against the maximum) or at the maximum ("max-out").
//! - A walk and its clearance being served hold the green as long as they need, past a gap-out and past the maximum,
//!   the arrivals still extending it within the maximum: actuation never shortens them, nor the yellow or the all-red.
//!   A walk longer than the maximum is a finding.
//! - Not modelled: a queue's discharge (the vehicles waiting at the start go as it starts and hold it no longer than its
//!   floor), lanes and detector placement, the phase resting in green.
//!
//! Every figure here is a placeholder (synthetic, like the catalogue's rules and rates): the defaults, the bounds and
//! the vehicles an hour.

use serde::{Deserialize, Serialize};

use crate::junction::model::control::{self, Bounds, Rule, Turn};
use crate::junction::model::signal::Plan;
use crate::junction::model::{Arm, Refusal, SIGNAL, State, in_force};

/// The least green an actuated phase runs, as a street is made actuated, in seconds. Placeholder.
pub const MIN_GREEN_DEFAULT_S: u32 = 7;
/// How long a gap with no arrival ends an actuated green, as a street is made actuated, in seconds. Placeholder.
pub const GAP_DEFAULT_S: u32 = 3;
/// The most an actuated green runs for its vehicles, as a street is made actuated, in seconds. Placeholder.
pub const MAX_GREEN_DEFAULT_S: u32 = 45;
/// The vehicles an hour an approach is given as it is made actuated. Placeholder.
pub const VEHICLES_DEFAULT_PER_HOUR: u32 = 300;
/// The bounds each setting is edited within, in seconds or vehicles an hour. Placeholders.
pub const MIN_GREEN_RANGE_S: (u32, u32) = (5, 30);
pub const GAP_RANGE_S: (u32, u32) = (1, 10);
pub const MAX_GREEN_RANGE_S: (u32, u32) = (10, 120);
pub const VEHICLES_RANGE_PER_HOUR: (u32, u32) = (0, 1_800);

/// How an actuated street's phase is run, and the vehicles an hour its approach is given (the scenario). Saved on the
/// arm only once the street is actuated (`Arm.actuation`), so a junction saved before reads as on recall.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Actuation {
    pub min_green_s: u32,
    pub gap_s: u32,
    pub max_green_s: u32,
    pub vehicles: u32,
}

impl Default for Actuation {
    fn default() -> Actuation {
        Actuation { min_green_s: MIN_GREEN_DEFAULT_S, gap_s: GAP_DEFAULT_S, max_green_s: MAX_GREEN_DEFAULT_S, vehicles: VEHICLES_DEFAULT_PER_HOUR }
    }
}

impl Actuation {
    /// Each setting within its range, and the minimum green no longer than the maximum (the maximum raised to it): a
    /// document from a sync or written by hand can hold anything.
    pub fn settled(mut self) -> Actuation {
        for f in Field::ALL {
            let (lo, hi) = f.range();
            let v = f.get(&self).clamp(lo, hi);
            f.put(&mut self, v);
        }
        self.max_green_s = self.max_green_s.max(self.min_green_s);
        self
    }

    /// What bounds the green it runs.
    pub fn bounds(&self) -> Bounds {
        Bounds { min_green_s: self.min_green_s, gap_s: self.gap_s, max_green_s: self.max_green_s }
    }
}

/// One of the four settings of an `Actuation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    MinGreen,
    Gap,
    MaxGreen,
    Vehicles,
}

impl Field {
    pub const ALL: [Field; 4] = [Field::MinGreen, Field::Gap, Field::MaxGreen, Field::Vehicles];

    /// The least and the most it may be.
    pub fn range(self) -> (u32, u32) {
        match self {
            Field::MinGreen => MIN_GREEN_RANGE_S,
            Field::Gap => GAP_RANGE_S,
            Field::MaxGreen => MAX_GREEN_RANGE_S,
            Field::Vehicles => VEHICLES_RANGE_PER_HOUR,
        }
    }

    /// The message that names it.
    pub fn key(self) -> &'static str {
        match self {
            Field::MinGreen => "jn-act-min-green",
            Field::Gap => "jn-act-gap",
            Field::MaxGreen => "jn-act-max-green",
            Field::Vehicles => "jn-act-vehicles",
        }
    }

    /// How much a stepper or an arrow key changes it: a second, or ten vehicles an hour.
    pub fn step(self) -> u32 {
        match self {
            Field::Vehicles => 10,
            _ => 1,
        }
    }

    /// The part of an element id that names it.
    pub fn id(self) -> &'static str {
        match self {
            Field::MinGreen => "min",
            Field::Gap => "gap",
            Field::MaxGreen => "max",
            Field::Vehicles => "rate",
        }
    }

    pub fn get(self, a: &Actuation) -> u32 {
        match self {
            Field::MinGreen => a.min_green_s,
            Field::Gap => a.gap_s,
            Field::MaxGreen => a.max_green_s,
            Field::Vehicles => a.vehicles,
        }
    }

    pub(crate) fn put(self, a: &mut Actuation, value: u32) {
        match self {
            Field::MinGreen => a.min_green_s = value,
            Field::Gap => a.gap_s = value,
            Field::MaxGreen => a.max_green_s = value,
            Field::Vehicles => a.vehicles = value,
        }
    }
}

/// How arm `a`'s phase is run, if it is actuated, every setting within its bounds: a document from a sync or written by
/// hand can hold any number. Read it here, never from `Arm.actuation`.
pub fn actuation_of(a: &Arm) -> Option<Actuation> {
    a.actuation.map(Actuation::settled)
}

/// The phase of `plan` that is street `uid`'s own and serves movements, if it has one (the first, when it has several).
pub fn own_phase(plan: &Plan, uid: u32) -> Option<usize> {
    plan.phases.iter().position(|p| p.uid == uid && !p.moves.is_empty())
}

/// Why street `uid` cannot be made actuated, if it cannot: the junction has to be a signal, and the plan has to give the
/// street a phase of its own (a pedestrian street, or one with no entering lanes, has none).
pub fn refusal(s: &State, region: usize, uid: u32) -> Option<Refusal> {
    if !s.arms.iter().any(|a| a.uid == uid) {
        return Some(Refusal::DoesNotFit);
    }
    let s = in_force(s);
    if s.control != SIGNAL {
        return Some(Refusal::NotSignal);
    }
    let Some(plan) = crate::junction::model::signal::plan(&s, region) else { return Some(Refusal::NotSignal) };
    own_phase(&plan, uid).is_none().then_some(Refusal::ActuationNoPhase)
}

/// The seconds into the hour at which vehicles arrive on the approach of street `uid`, `rate` an hour: the hour is cut
/// into `rate` equal slots, one vehicle in each, at a point the generator picks. Seeded from the street and the rate on
/// a stream of its own (`VEHICLE_STREAM`), so it never repeats the pedestrian calls of the same street. In order, each
/// second at most once. A rate past the bound is read as the bound.
pub fn arrivals(uid: u32, rate: u32) -> Vec<u32> {
    let rate = rate.min(VEHICLES_RANGE_PER_HOUR.1);
    control::spread(((u64::from(uid) << 32) | u64::from(rate)) ^ VEHICLE_STREAM, rate)
}

/// What sets the vehicles' seeds apart from the pedestrian calls' ("VEHICLES" in ASCII).
const VEHICLE_STREAM: u64 = 0x5645_4849_434C_4553;

/// The rules the phases of `plan` run by: an actuated phase for each phase whose own street of `s` is actuated and which
/// serves movements, its floor the minimum green or, when longer, the walk and clearance of a crossing on recall walked
/// in it (`distances` by street uid, `read_model::crossing_distances`); every other phase fixed.
pub fn rules(s: &State, plan: &Plan, distances: &[(u32, i32)]) -> Vec<Rule> {
    plan.phases
        .iter()
        .map(|p| {
            let Some(a) = s.arms.iter().find(|a| a.uid == p.uid).and_then(actuation_of).filter(|_| !p.moves.is_empty()) else { return Rule::Fixed };
            let recall = p.walks.iter().filter(|w| s.arms.iter().any(|a| a.uid == **w && a.calls.is_none() && a.crossing.is_some()));
            let walk = recall.filter_map(|w| distance(distances, *w)).map(crate::junction::model::calls::need_s).max().unwrap_or(0);
            Rule::Actuated { bounds: a.bounds(), floor_s: a.min_green_s.max(walk) }
        })
        .collect()
}

fn distance(distances: &[(u32, i32)], uid: u32) -> Option<i32> {
    distances.iter().find(|(u, _)| *u == uid).map(|(_, d)| *d)
}

/// One actuated phase over the hour: how often it was given its green or skipped, how often its green ran past its floor
/// and by how much in all, and how often it ended before its maximum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct PhaseTotals {
    /// Its place in the plan, and its street.
    pub phase: usize,
    pub arm: u32,
    pub min_green_s: u32,
    pub gap_s: u32,
    pub max_green_s: u32,
    /// The least green it runs: the minimum, or a walk on recall's walk and clearance when longer.
    pub floor_s: u32,
    pub served: u32,
    pub skipped: u32,
    /// The greens that ran past the floor, and the seconds past it, added up.
    pub extended: u32,
    pub extended_s: u32,
    /// The greens that ended before the maximum (a gap with no arrival).
    pub ended_early: u32,
}

/// The vehicles of one actuated approach, added up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct ApproachTotals {
    pub arm: u32,
    /// Its vehicles an hour.
    pub rate: u32,
    pub vehicles: u32,
    pub served: u32,
    /// Still waiting when the run stopped.
    pub not_served: u32,
    /// The mean and the longest wait for a green, rounded to the second; None when no vehicle was served.
    pub average_wait_s: Option<u32>,
    pub longest_wait_s: Option<u32>,
}

/// Something about an actuated street the person should act on. By street uid; no wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Finding {
    /// The street is actuated, but no phase of the plan is its own any more (an edit took it away): it is kept actuated,
    /// and its vehicles call nothing until it has one again.
    NoPhase { arm: u32 },
    /// An actuated phase of street `arm` walks the crossing of street `crossing`, whose walk and clearance (`need_s`) are
    /// longer than the maximum green (`max_s`): the walk holds the green past the maximum whenever it is served.
    WalkPastMax { arm: u32, crossing: u32, need_s: u32, max_s: u32 },
}

impl Finding {
    /// What the finding is about, whatever its figures: its kind and its streets.
    pub fn about(&self) -> (u8, u32, u32) {
        match *self {
            Finding::NoPhase { arm } => (10, arm, 0),
            Finding::WalkPastMax { arm, crossing, .. } => (11, arm, crossing),
        }
    }
}

/// How the actuated phases ran over the hour, and the vehicles they served.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Summary {
    /// One for each actuated phase, in the order of the plan.
    pub phases: Vec<PhaseTotals>,
    /// One for each actuated street that has a phase of its own, in the order of the bearings.
    pub approaches: Vec<ApproachTotals>,
    pub findings: Vec<Finding>,
}

/// The vehicles arriving on one approach over the hour.
pub(crate) struct Approach {
    pub arm: u32,
    pub rate: u32,
    pub at_s: Vec<u32>,
}

/// The actuated approaches of `s` (in the order of the bearings) that have a phase of their own in `plan`, each with its
/// vehicles, and the findings about those that have none or whose phase walks a crossing longer than its maximum.
pub(crate) fn approaches(s: &State, plan: &Plan, rules: &[Rule], distances: &[(u32, i32)]) -> (Vec<Approach>, Vec<Finding>) {
    let mut arms: Vec<&Arm> = s.arms.iter().collect();
    arms.sort_by_key(|a| a.bearing);
    let (mut out, mut findings) = (Vec::new(), Vec::new());
    for (arm, act) in arms.into_iter().filter_map(|a| actuation_of(a).map(|x| (a, x))) {
        if own_phase(plan, arm.uid).is_none() {
            findings.push(Finding::NoPhase { arm: arm.uid });
            continue;
        }
        out.push(Approach { arm: arm.uid, rate: act.vehicles, at_s: arrivals(arm.uid, act.vehicles) });
    }
    for (p, rule) in plan.phases.iter().zip(rules) {
        let Rule::Actuated { bounds, .. } = rule else { continue };
        for &w in &p.walks {
            let need_s = distance(distances, w).map_or(0, crate::junction::model::calls::need_s);
            let found = Finding::WalkPastMax { arm: p.uid, crossing: w, need_s, max_s: bounds.max_green_s };
            if need_s > bounds.max_green_s && !findings.contains(&found) {
                findings.push(found);
            }
        }
    }
    (out, findings)
}

/// The vehicles as a demand on the controller: each approach's arrivals, which phases serve it, and how long each
/// vehicle served waited. Vehicles are served in the order they came, so each approach keeps the first not yet served.
pub(crate) struct Vehicles<'a> {
    approaches: &'a [Approach],
    /// Whether each phase (by its place in the plan) moves each approach's traffic.
    serves: Vec<Vec<bool>>,
    next: Vec<usize>,
    waits: Vec<Vec<u32>>,
}

impl<'a> Vehicles<'a> {
    pub(crate) fn new(plan: &Plan, approaches: &'a [Approach]) -> Vehicles<'a> {
        let serves = approaches.iter().map(|a| plan.phases.iter().map(|p| p.moves.iter().any(|m| m.from == a.arm)).collect()).collect();
        Vehicles { approaches, serves, next: vec![0; approaches.len()], waits: vec![Vec::new(); approaches.len()] }
    }

    /// The approaches phase `k` serves, by their place.
    fn served_by(&self, k: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.approaches.len()).filter(move |&x| self.serves[x][k])
    }

    /// The vehicles of approach `x` not yet served.
    fn unserved(&self, x: usize) -> &[u32] {
        &self.approaches[x].at_s[self.next[x]..]
    }

    /// Each approach's vehicles added up.
    pub(crate) fn finish(self) -> Vec<ApproachTotals> {
        self.approaches
            .iter()
            .zip(&self.waits)
            .map(|(a, waits)| {
                let served = waits.len() as u32;
                let sum: u64 = waits.iter().map(|&w| u64::from(w)).sum();
                ApproachTotals {
                    arm: a.arm,
                    rate: a.rate,
                    vehicles: a.at_s.len() as u32,
                    served,
                    not_served: a.at_s.len() as u32 - served,
                    average_wait_s: (served > 0).then(|| u32::try_from((sum + u64::from(served) / 2) / u64::from(served)).unwrap_or(u32::MAX)),
                    longest_wait_s: waits.iter().copied().max(),
                }
            })
            .collect()
    }
}

impl control::Demand for Vehicles<'_> {
    /// A vehicle waiting on an approach the phase serves calls it.
    fn calls(&self, k: usize, at: u64) -> bool {
        self.served_by(k).any(|x| self.unserved(x).first().is_some_and(|&t| u64::from(t) <= at))
    }

    /// The floor (at least the minimum green), then each arrival while green holds it a gap longer, the gap timed from
    /// the arrival even while the floor runs, never past the maximum.
    fn extend(&self, k: usize, green_start: u64, b: Bounds, floor_s: u32) -> u32 {
        if self.served_by(k).next().is_none() {
            return 0;
        }
        let (floor, most) = (green_start + u64::from(floor_s), green_start + u64::from(b.max_green_s));
        let mut coming: Vec<u64> =
            self.served_by(k).flat_map(|x| self.unserved(x).iter().map(|&t| u64::from(t))).filter(|&t| t >= green_start && t < floor.max(most)).collect();
        coming.sort_unstable();
        let mut end = floor;
        for t in coming {
            if t >= end {
                break; // a gap with no arrival once the floor is done: gap-out
            }
            end = end.max((t + u64::from(b.gap_s)).min(most));
        }
        u32::try_from(end - green_start).unwrap_or(u32::MAX)
    }

    fn hold(&self, _k: usize, _start: u64) -> u32 {
        0
    }

    /// Every vehicle waiting, or arriving before the green ends, on an approach the phase serves goes.
    fn ran(&mut self, turn: &Turn) {
        if turn.skipped {
            return;
        }
        for x in 0..self.approaches.len() {
            if !self.serves[x][turn.phase] {
                continue;
            }
            while let Some(&t) = self.approaches[x].at_s.get(self.next[x]) {
                if u64::from(t) >= turn.green_end {
                    break;
                }
                self.waits[x].push(u32::try_from(turn.green_start.saturating_sub(u64::from(t))).unwrap_or(u32::MAX));
                self.next[x] += 1;
            }
        }
    }

    fn pending(&self) -> bool {
        (0..self.approaches.len()).any(|x| !self.unserved(x).is_empty())
    }

    fn next_after(&self, at: u64) -> Option<u64> {
        (0..self.approaches.len()).filter_map(|x| self.unserved(x).iter().map(|&t| u64::from(t)).find(|&t| t > at)).min()
    }

    fn last(&self) -> u64 {
        self.approaches.iter().filter_map(|a| a.at_s.last()).map(|&t| u64::from(t)).max().unwrap_or(0)
    }
}

/// Each actuated phase's turns added up, in the order of the plan.
pub(crate) fn phase_totals(plan: &Plan, rules: &[Rule], turns: &[Turn]) -> Vec<PhaseTotals> {
    plan.phases
        .iter()
        .zip(rules)
        .enumerate()
        .filter_map(|(k, (p, rule))| {
            let Rule::Actuated { bounds, floor_s } = *rule else { return None };
            let mut t = PhaseTotals {
                phase: k,
                arm: p.uid,
                min_green_s: bounds.min_green_s,
                gap_s: bounds.gap_s,
                max_green_s: bounds.max_green_s,
                floor_s,
                ..PhaseTotals::default()
            };
            for turn in turns.iter().filter(|t| t.phase == k) {
                if turn.skipped {
                    t.skipped += 1;
                    continue;
                }
                let green = turn.green_s();
                t.served += 1;
                t.extended += u32::from(green > floor_s);
                t.extended_s = t.extended_s.saturating_add(green.saturating_sub(floor_s));
                t.ended_early += u32::from(green < bounds.max_green_s);
            }
            Some(t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::calls::{Asked, Walks};
    use crate::junction::model::control::{Demand, run};
    use crate::junction::model::signal::{self, Move, Phase, SignalPlan, StoredPhase};
    use crate::junction::model::{Junction, Lane, PRIORITY, Timing};

    fn at(s: &State, bearing: i32) -> u32 {
        s.arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    /// The four-way on a signal, every street's traffic going straight on (as `calls`' tests have it): north and south's
    /// phases walk east and west, east and west's walk north and south; each green 25 s, yellow 3 s, all-red 2 s.
    fn cross() -> State {
        let mut s = Junction::new(0).current().clone();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        for (from, to) in [(n, so), (so, n), (e, w), (w, e)] {
            s.arms.iter_mut().find(|a| a.uid == from).unwrap().lanes = vec![Lane { to: vec![to] }];
        }
        s
    }

    fn arm_mut(s: &mut State, uid: u32) -> &mut Arm {
        s.arms.iter_mut().find(|a| a.uid == uid).unwrap()
    }

    /// A phase of street `uid`, its traffic going to street 99, with a `green_s` green, yellow 3 s and all-red 2 s,
    /// walking `walks`.
    fn phase(uid: u32, green_s: u32, walks: &[u32]) -> Phase {
        let timing = Timing { green_s, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        Phase { uid, timing, length_s: timing.length_s(), walks: walks.to_vec(), rides: vec![], moves: vec![Move { from: uid, to: 99 }] }
    }

    fn plan_of(phases: Vec<Phase>) -> Plan {
        let cycle_s = phases.iter().map(|p| p.length_s).sum();
        Plan { phases, cycle_s, suggested: false }
    }

    fn actuated(min: u32, gap: u32, max: u32) -> Rule {
        Rule::Actuated { bounds: Bounds { min_green_s: min, gap_s: gap, max_green_s: max }, floor_s: min }
    }

    /// The turns of `plan` by `rules` with the vehicles of `approaches` and the pedestrian calls of `asked`, and each
    /// approach's totals: (phase, start, green, end) for each turn that ran, skipped ones left out.
    fn hour_of(plan: &Plan, rules: &[Rule], approaches: &[Approach], asked: &[Asked]) -> (Vec<(usize, u64, u32, u64)>, Vec<ApproachTotals>, Vec<Turn>) {
        let mut walks = Walks::new(plan, rules, asked);
        let mut vehicles = Vehicles::new(plan, approaches);
        let turns = run(plan, rules, &mut [&mut walks as &mut dyn Demand, &mut vehicles], 100_000);
        let ran = turns.iter().filter(|t| !t.skipped).map(|t| (t.phase, t.start, t.green_s(), t.end)).collect();
        (ran, vehicles.finish(), turns)
    }

    fn approach(arm: u32, at_s: Vec<u32>) -> Approach {
        Approach { arm, rate: at_s.len() as u32, at_s }
    }

    #[test]
    fn the_defaults_and_bounds_are_the_placeholders_the_issue_settled() {
        let a = Actuation::default();
        assert_eq!((a.min_green_s, a.gap_s, a.max_green_s, a.vehicles), (7, 3, 45, 300));
        assert_eq!((MIN_GREEN_RANGE_S, GAP_RANGE_S, MAX_GREEN_RANGE_S, VEHICLES_RANGE_PER_HOUR), ((5, 30), (1, 10), (10, 120), (0, 1_800)));
        for f in Field::ALL {
            let (lo, hi) = f.range();
            assert!((lo..=hi).contains(&f.get(&a)), "{f:?}: the default is within its bounds");
        }
    }

    #[test]
    fn a_document_s_settings_are_read_within_their_bounds_the_minimum_never_past_the_maximum() {
        let wild = Actuation { min_green_s: 0, gap_s: 99, max_green_s: u32::MAX, vehicles: u32::MAX };
        assert_eq!(wild.settled(), Actuation { min_green_s: 5, gap_s: 10, max_green_s: 120, vehicles: 1_800 });
        let crossed = Actuation { min_green_s: 30, gap_s: 3, max_green_s: 10, vehicles: 0 };
        assert_eq!(crossed.settled().max_green_s, 30, "the maximum raised to the minimum");
    }

    #[test]
    fn the_vehicles_of_an_hour_are_the_same_every_time_spread_over_the_hour_and_unlike_the_calls_of_the_same_street() {
        let cars = arrivals(7, 300);
        assert_eq!(cars, arrivals(7, 300), "seeded");
        assert_eq!(cars.len(), 300);
        for (k, t) in cars.iter().enumerate() {
            assert!((k as u32 * 12..(k as u32 + 1) * 12).contains(t), "one in each slot of 12 s: {cars:?}");
        }
        assert_ne!(arrivals(7, 12), crate::junction::model::calls::arrivals(7, 12), "a stream of its own");
        assert_eq!(arrivals(7, u32::MAX).len(), 1_800, "read as the bound");
        let most = arrivals(7, 1_800);
        assert!(most.windows(2).all(|w| w[0] < w[1]) && most.iter().all(|&t| t < 3_600));
        assert!(arrivals(7, 0).is_empty());
        // Pinned: a change of generator or seed changes every summary, so it has to be a decision.
        assert_eq!(arrivals(1, 4), PINNED_VEHICLES);
    }

    const PINNED_VEHICLES: [u32; 4] = [705, 1142, 2547, 3351];

    #[test]
    fn an_actuated_phase_with_no_vehicle_waiting_is_skipped_and_the_signal_rests_until_one_comes() {
        let plan = plan_of(vec![phase(1, 25, &[]), phase(2, 25, &[])]);
        let rules = vec![actuated(7, 3, 20), actuated(7, 3, 20)];
        let (ran, totals, turns) = hour_of(&plan, &rules, &[approach(1, vec![100])], &[]);
        // Nothing until 100 s: the first phase runs its minimum, 7 s, its yellow and all-red kept (12 s in all).
        assert_eq!(ran, vec![(0, 100, 7, 112)]);
        assert!(turns.iter().filter(|t| t.skipped).all(|t| t.end == t.start), "a skipped phase takes no time");
        assert_eq!(turns.iter().filter(|t| t.skipped).count(), 4, "both at 0 s and both at 112 s; then rest");
        assert_eq!((totals[0].served, totals[0].longest_wait_s), (1, Some(0)));
    }

    #[test]
    fn each_arrival_in_the_green_holds_it_a_gap_longer_until_a_gap_with_none_ends_it_early() {
        let plan = plan_of(vec![phase(1, 25, &[]), phase(2, 25, &[])]);
        let rules = vec![actuated(7, 3, 20), Rule::Fixed];
        // Green from 0 s: the minimum to 7 s; the arrival at 5 s holds it to 8 s, the one at 7 s to 10 s; nothing comes
        // from 10 s to 13 s, so it ends at 10 s (gap-out). The one at 13 s waits for the next green.
        let (ran, totals, _) = hour_of(&plan, &rules, &[approach(1, vec![0, 5, 7, 13])], &[]);
        assert_eq!(ran[0], (0, 0, 10, 15));
        assert_eq!(ran[1], (1, 15, 25, 45), "the fixed phase runs at its timing");
        assert_eq!(ran[2], (0, 45, 7, 57));
        assert!(ran[3..].iter().all(|r| r.0 == 1), "then only the fixed phase runs, to the end of the hour");
        assert_eq!(totals, vec![ApproachTotals { arm: 1, rate: 4, vehicles: 4, served: 4, not_served: 0, average_wait_s: Some(8), longest_wait_s: Some(32) }]);
    }

    #[test]
    fn arrivals_closer_than_the_gap_run_the_green_to_its_maximum_and_no_further() {
        let plan = plan_of(vec![phase(1, 25, &[]), phase(2, 25, &[])]);
        let rules = vec![actuated(7, 3, 20), Rule::Fixed];
        let every_two: Vec<u32> = (0..30).map(|k| k * 2).collect();
        let (ran, _, turns) = hour_of(&plan, &rules, &[approach(1, every_two)], &[]);
        assert_eq!(ran[0], (0, 0, 20, 25), "max-out at 20 s");
        // The vehicles from 20 s wait through the fixed phase (25 to 55 s) and go as the next green starts: a queue at
        // the start holds it no longer than the minimum, and the arrivals at 56 and 58 s only to 61 s, inside it.
        assert_eq!(ran[2], (0, 55, 7, 67));
        let totals = phase_totals(&plan, &rules, &turns);
        assert_eq!((totals[0].served, totals[0].extended, totals[0].extended_s, totals[0].ended_early), (2, 1, 13, 1));
    }

    #[test]
    fn a_served_walk_holds_the_green_past_a_gap_out_and_past_the_maximum_and_the_yellow_and_all_red_are_kept() {
        let plan = plan_of(vec![phase(1, 25, &[9]), phase(2, 25, &[])]);
        let rules = vec![actuated(7, 3, 10), Rule::Fixed];
        // One vehicle and a call at 0 s; the crossing needs 30 s, more than the 10 s maximum: the green runs 30 s.
        let asked = [Asked { arm: 9, need_s: 30, at_s: vec![0], passive: false }];
        let (ran, _, turns) = hour_of(&plan, &rules, &[approach(1, vec![0])], &asked);
        assert_eq!(ran[0], (0, 0, 30, 35), "walk and clearance whole, then the 3 s yellow and 2 s all-red");
        assert_eq!(turns[0].base_s, 7, "the vehicles alone gave the minimum");
        // A call alone calls the phase, and is served with its whole walk.
        let (ran, _, _) = hour_of(&plan, &rules, &[], &[Asked { arm: 9, need_s: 16, at_s: vec![40], passive: false }]);
        assert_eq!(ran.iter().find(|r| r.0 == 0).map(|r| r.2), Some(16));
    }

    #[test]
    fn a_walk_on_recall_in_an_actuated_phase_is_its_floor_and_does_not_call_it() {
        let mut s = cross();
        let (n, e) = (at(&s, 0), at(&s, 90));
        arm_mut(&mut s, n).actuation = Some(Actuation::default());
        let plan = signal::plan(&s, 0).unwrap();
        // North's phase walks east's crossing (on recall), 12 m: 7 s walk + 12 s clearance = 19 s, past the 7 s minimum.
        let rules = rules(&s, &plan, &[(e, 12_000)]);
        let k = own_phase(&plan, n).unwrap();
        assert!(plan.phases[k].walks.contains(&e));
        assert_eq!(rules[k], Rule::Actuated { bounds: Actuation::default().bounds(), floor_s: 19 });
        assert!(rules.iter().enumerate().all(|(i, r)| i == k || *r == Rule::Fixed), "only the actuated street's own phase");
        // On call, the crossing no longer sets the floor: it holds the green only when someone has called it.
        arm_mut(&mut s, e).calls = Some(12);
        assert_eq!(super::rules(&s, &plan, &[(e, 12_000)])[k], Rule::Actuated { bounds: Actuation::default().bounds(), floor_s: 7 });
    }

    #[test]
    fn a_street_can_be_actuated_only_where_the_plan_gives_it_a_phase_of_its_own() {
        let mut s = cross();
        let (n, e, so) = (at(&s, 0), at(&s, 90), at(&s, 180));
        assert_eq!(refusal(&s, 0, n), None);
        let only = StoredPhase { moves: vec![Move { from: n, to: so }], walks: vec![e], rides: vec![], arm: None };
        s.signal_plan = Some(SignalPlan { phases: vec![only] });
        assert_eq!(refusal(&s, 0, n), None);
        assert_eq!(refusal(&s, 0, e), Some(Refusal::ActuationNoPhase), "east's traffic has no phase of its own");
        let mut not_signal = cross();
        not_signal.control = PRIORITY;
        assert_eq!(refusal(&not_signal, 0, n), Some(Refusal::NotSignal));
        assert_eq!(refusal(&cross(), 0, 12_345), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn actuation_is_set_within_its_bounds_and_taken_off_and_undone_like_any_edit() {
        let mut j = Junction::new(0);
        let n = at(j.current(), 0);
        assert!(!j.set_actuation(n, Field::MinGreen, 10));
        assert_eq!(j.refusal(), Some(Refusal::NotActuated));
        assert!(j.set_actuated(n, true));
        assert_eq!((j.arm(n).unwrap().actuation, j.current().label.key), (Some(Actuation::default()), "jn-rev-actuated-on"));
        assert!(!j.set_actuated(n, true));
        assert_eq!(j.refusal(), Some(Refusal::ActuationAlready));
        assert!(j.set_actuation(n, Field::MaxGreen, 60));
        assert_eq!((j.arm(n).unwrap().actuation.unwrap().max_green_s, j.current().label.key), (60, "jn-rev-actuation"));
        assert!(j.set_actuation(n, Field::Vehicles, 0), "no vehicles at all is allowed");
        assert_eq!((j.arm(n).unwrap().actuation.unwrap().vehicles, j.current().label.key), (0, "jn-rev-actuation-vehicles"));
        for f in Field::ALL {
            let (lo, hi) = f.range();
            for bad in [lo as i32 - 1, hi as i32 + 1] {
                assert!(!j.set_actuation(n, f, bad), "{f:?} {bad}");
                assert_eq!(j.refusal(), Some(Refusal::ActuationOutOfRange));
            }
        }
        assert!(j.set_actuation(n, Field::MaxGreen, 20) && j.set_actuation(n, Field::MinGreen, 20), "equal is allowed");
        assert!(!j.set_actuation(n, Field::MinGreen, 21));
        assert_eq!(j.refusal(), Some(Refusal::MinAboveMax));
        assert!(!j.set_actuation(n, Field::MaxGreen, 19));
        assert_eq!(j.refusal(), Some(Refusal::MinAboveMax));
        let edits = j.revisions().count();
        assert!(j.set_actuation(n, Field::Gap, 3), "the value it has: taken");
        assert_eq!(j.revisions().count(), edits, "and no edit");
        assert!(j.set_actuated(n, false));
        assert_eq!((j.arm(n).unwrap().actuation, j.current().label.key), (None, "jn-rev-actuated-off"));
        assert!(j.undo() && j.arm(n).unwrap().actuation.is_some_and(|a| a.min_green_s == 20));
        // Off a signal the settings go, as the timing does, and the revision says the timing went.
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.arm(n).unwrap().actuation, None);
        assert_eq!(j.current().label.key, "jn-rev-control-timing");
        assert!(!j.set_actuated(n, true));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
    }

    #[test]
    fn a_junction_with_no_street_actuated_is_written_as_before_and_actuation_only_once_set() {
        let mut s = Junction::new(0).current().clone();
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("actuation"), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert!(back.arms.iter().all(|a| a.actuation.is_none()));
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        s.arms[0].actuation = Some(Actuation::default());
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""actuation":{"min_green_s":7,"gap_s":3,"max_green_s":45,"vehicles":300}"#), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.arms[0].actuation, Some(Actuation::default()));
        // A document's settings past their bounds open within them.
        let mut wild = Junction::new(0).current().clone();
        wild.arms[0].actuation = Some(Actuation { min_green_s: 99, gap_s: 0, max_green_s: 1, vehicles: 5_000 });
        let opened = Junction::from_city(crate::street::model::named_said("J"), &wild, &wild, 0).unwrap();
        assert_eq!(opened.current().arms[0].actuation, Some(Actuation { min_green_s: 30, gap_s: 1, max_green_s: 30, vehicles: 1_800 }));
    }

    #[test]
    fn with_no_street_actuated_the_hour_is_the_calls_alone_and_every_phase_fixed() {
        let mut s = cross();
        let e = at(&s, 90);
        arm_mut(&mut s, e).calls = Some(12);
        let plan = signal::plan(&s, 0).unwrap();
        let h = control::hour(&s, &plan, &[(e, 9_000)]);
        assert_eq!(h.actuation, Summary::default());
        assert!(rules(&s, &plan, &[(e, 9_000)]).iter().all(|r| *r == Rule::Fixed));
        assert_eq!(h.calls.calls.len(), 12);
    }

    #[test]
    fn an_actuated_street_that_loses_its_phase_or_whose_walk_outlasts_its_maximum_is_a_finding() {
        let mut s = cross();
        let (n, e, so) = (at(&s, 0), at(&s, 90), at(&s, 180));
        arm_mut(&mut s, n).actuation = Some(Actuation { max_green_s: 10, ..Actuation::default() });
        let plan = signal::plan(&s, 0).unwrap();
        // North's phase walks east's crossing, 12 m: 19 s, past its 10 s maximum.
        let h = control::hour(&s, &plan, &[(e, 12_000)]);
        assert!(h.actuation.findings.contains(&Finding::WalkPastMax { arm: n, crossing: e, need_s: 19, max_s: 10 }), "{:?}", h.actuation.findings);
        // A plan with no phase of north's own: kept actuated, and a finding.
        s.signal_plan = Some(SignalPlan { phases: vec![StoredPhase { moves: vec![Move { from: so, to: n }], walks: vec![], rides: vec![], arm: None }] });
        let plan = signal::plan(&s, 0).unwrap();
        let h = control::hour(&s, &plan, &[]);
        assert_eq!(h.actuation.findings, vec![Finding::NoPhase { arm: n }]);
        assert!(h.actuation.approaches.is_empty() && h.actuation.phases.is_empty());
    }

    #[test]
    fn fixed_inputs_give_a_fixed_summary() {
        let mut s = cross();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        // North busy (600 an hour, one every 6 s), east quiet (20 an hour), east's crossing on call; every crossing 9 m.
        arm_mut(&mut s, n).actuation = Some(Actuation { vehicles: 600, ..Actuation::default() });
        arm_mut(&mut s, e).actuation = Some(Actuation { vehicles: 20, ..Actuation::default() });
        arm_mut(&mut s, e).calls = Some(3);
        let plan = signal::plan(&s, 0).unwrap();
        let distances = [(n, 9_000), (e, 9_000), (so, 9_000), (w, 9_000)];
        let h = control::hour(&s, &plan, &distances);
        assert_eq!(h, control::hour(&s, &plan, &distances), "deterministic");
        let phase = |phase: usize, arm: u32| PhaseTotals { phase, arm, min_green_s: 7, gap_s: 3, max_green_s: 45, floor_s: 16, ..PhaseTotals::default() };
        // North's floor is the 16 s walk across west's crossing (on recall); it was called every turn, and 13 of its
        // greens ran past the floor. East's lone vehicles each called a green of their own, and its phase was skipped
        // as often. East's crossing (on call) is walked in north's and south's phases: it holds no east green.
        let pinned = Summary {
            phases: vec![
                PhaseTotals { served: 41, skipped: 0, extended: 13, extended_s: 23, ended_early: 41, ..phase(0, n) },
                PhaseTotals { served: 20, skipped: 20, extended: 0, extended_s: 0, ended_early: 20, ..phase(1, e) },
            ],
            approaches: vec![
                ApproachTotals { arm: n, rate: 600, vehicles: 600, served: 600, not_served: 0, average_wait_s: Some(33), longest_wait_s: Some(86) },
                ApproachTotals { arm: e, rate: 20, vehicles: 20, served: 20, not_served: 0, average_wait_s: Some(43), longest_wait_s: Some(78) },
            ],
            findings: vec![],
        };
        assert_eq!(h.actuation, pinned, "{:#?}", h.actuation);
    }
}
