//! Transit signal priority (CLM-14): a street with transit and a phase of its own is given no priority (the default),
//! an early green, a green extension, or both, for the buses approaching it. A seeded generator spreads each street's
//! buses over one hour, and the controller (`control::run`) serves them with the pedestrian calls, the actuated streets'
//! vehicles and the cyclists. Deterministic: the same junction, plan and rates always give the same hour. CityLoom
//! models the behaviour to compare strategies; it never controls a signal or predicts real-world delay or safety. No
//! DOM, no signals, no I/O, no wording.
//!
//! The rule of a bus (a street's buses are served by the phase that is the street's own, `actuation::own_phase`):
//! - A bus that comes while its street's green runs (from the phase's start, its transit lead included, to the end of
//!   its green) goes on arrival.
//! - Early green: a bus that comes while its street is red cuts short the green of the phases that run before its own
//!   (the one running, if its green has not ended, then each in turn), so its green comes sooner, by at most
//!   `EARLY_GREEN_MAX_S` in all for that green. A green is never ended before the bus came, nor cut below its minimum
//!   (a fixed phase's: the timing's least green, `GREEN_MIN_S`; an actuated phase's: its minimum green), nor below the
//!   walk and clearance of a crossing on recall it walks, the bicycle minimum green and clearance of a bike movement on
//!   recall it runs, an actuated phase's floor, or what a demand served in it holds (a pedestrian call, a cyclist): those
//!   are never shortened. When one of them keeps a green longer than the bus asked, the bus is given what can be given
//!   and the seconds kept are a finding (`Finding::ClearanceHeld`), never shortened silently. A phase a bus of its own
//!   street is due to use (coming before its green and an extension could end) is never cut. When buses of several
//!   streets wait, the phase is cut for the one whose own phase comes next.
//! - Green extension: a bus that comes within `EXTENSION_MAX_S` after its street's green would end holds the green
//!   until it has passed (the second after it came), at most `EXTENSION_MAX_S`.
//! - The yellow and the all-red are never shortened: only the green is cut or held, so the phases after a held green
//!   start later, and those after a cut one sooner.
//! - Not in two cycles running: a street given priority (an early green or an extension) for one green of its phase is
//!   given none for the next one; a bus coming then goes on arrival or waits (delayed). Its phase's turns are counted as
//!   they run (an actuated phase skipped does not count).
//! - A bus calls its street's phase when it is actuated, as a vehicle does.
//! - What it costs: the seconds of green the other phases lost to an early green, and the seconds an extended green
//!   made each other approach's next green start later, added up for each approach (`Moved`). Only delay an approach
//!   bore counts: an extension's seconds go to each other approach once, when its next green runs before the street's
//!   own runs again (an actuated phase skipped, or a green the hour never reaches, bore none).
//! - The street's transit lead (`Timing::transit_lead_s`) is unchanged: a fixed early green for the bus lane every
//!   cycle, whether a bus comes or not. Priority is on demand, on top of it.
//!
//! A street has transit when it has a bus lane on its way in, a queue jump or bus gate (`Arm.approach`), a bus stop
//! measure (`Arm.stop`), a transit modal filter (`Arm.filter`) or a transit stop of its own in its end zone
//! (`Arm.stops`): `has_transit`.
//!
//! Every figure here is a placeholder (synthetic, like the catalogue's rules and rates): the longest early green and
//! extension, the default and the bound of the buses an hour.

use serde::{Deserialize, Serialize};

use crate::junction::model::actuation::own_phase;
use crate::junction::model::control::{self, Rule, Turn};
use crate::junction::model::signal::Plan;
use crate::junction::model::{Arm, GREEN_MIN_S, Refusal, SIGNAL, State, bicycles, calls, in_force};

/// The most an early green brings a street's green forward, in seconds, for one green. Placeholder.
pub const EARLY_GREEN_MAX_S: u32 = 10;
/// The most a green is held for a bus, in seconds. Placeholder.
pub const EXTENSION_MAX_S: u32 = 10;
/// The buses an hour a street is given when its priority is turned on. Placeholder.
pub const DEFAULT_BUSES_PER_HOUR: u32 = 12;
/// The most buses an hour a street may be given. Placeholder.
pub const MAX_BUSES_PER_HOUR: u32 = 60;

/// What a street's buses are given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The phases before its own are cut short when a bus comes during red.
    EarlyGreen,
    /// Its green is held for a bus that comes just after it would end.
    Extension,
    /// Both.
    Both,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::EarlyGreen, Mode::Extension, Mode::Both];

    /// Whether it gives an early green.
    pub fn early_green(self) -> bool {
        matches!(self, Mode::EarlyGreen | Mode::Both)
    }

    /// Whether it gives a green extension.
    pub fn extends(self) -> bool {
        matches!(self, Mode::Extension | Mode::Both)
    }
}

/// A street's transit priority and the buses an hour its approach is given (the scenario). Saved on the arm only once
/// set (`Arm.transit_priority`), so a junction saved before has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Priority {
    pub mode: Mode,
    pub buses: u32,
}

/// Arm `a`'s priority, if it has one, its buses never past `MAX_BUSES_PER_HOUR`: a document from a sync or written by
/// hand can hold any number. Read it here, never from `Arm.transit_priority`.
pub fn priority_of(a: &Arm) -> Option<Priority> {
    a.transit_priority.map(|p| Priority { buses: p.buses.min(MAX_BUSES_PER_HOUR), ..p })
}

/// Whether street `a` has transit: a bus lane on its way in, a queue jump or bus gate, a bus stop measure, a transit
/// modal filter, or a transit stop of its own in its end zone.
pub fn has_transit(a: &Arm) -> bool {
    a.bus_lane || a.approach != 0 || a.stop != 0 || a.filter || !a.stops.is_empty()
}

/// Why street `uid` cannot be given priority, if it cannot: the junction has to be a signal, the street has to have
/// transit (`has_transit`), and the plan has to give it a phase of its own.
pub fn refusal(s: &State, region: usize, uid: u32) -> Option<Refusal> {
    let Some(arm) = s.arms.iter().find(|a| a.uid == uid) else { return Some(Refusal::DoesNotFit) };
    let s = in_force(s);
    if s.control != SIGNAL {
        return Some(Refusal::NotSignal);
    }
    if !has_transit(arm) {
        return Some(Refusal::PriorityNoTransit);
    }
    let Some(plan) = crate::junction::model::signal::plan(&s, region) else { return Some(Refusal::NotSignal) };
    own_phase(&plan, uid).is_none().then_some(Refusal::PriorityNoPhase)
}

/// The seconds into the hour at which buses come on the approach of street `uid`, `rate` an hour: the hour is cut into
/// `rate` equal slots, one bus in each, at a point the generator picks. Seeded from the street and the rate on a stream
/// of its own (`BUS_STREAM`), so it never repeats the calls, vehicles or cyclists of the same street. In order, each
/// second at most once. A rate past `MAX_BUSES_PER_HOUR` is read as that.
pub fn arrivals(uid: u32, rate: u32) -> Vec<u32> {
    let rate = rate.min(MAX_BUSES_PER_HOUR);
    control::spread(((u64::from(uid) << 32) | u64::from(rate)) ^ BUS_STREAM, rate)
}

/// What sets the buses' seeds apart from the other demands' ("BUSES" in ASCII).
const BUS_STREAM: u64 = 0x0000_0042_5553_4553;

/// What happened to one bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Outcome {
    /// Came while its street's green ran.
    OnArrival,
    /// Came during red, and its street's green was brought forward for it (or for a bus before it in the same red).
    EarlyGreen,
    /// Came just after its street's green would have ended, and the green was held for it.
    Extended,
    /// Came during red and waited for its street's next green, with no priority (none it may have, none left, or none
    /// that cycle).
    Delayed,
    /// Still waiting when the run stopped.
    NotServed,
}

/// One bus: its street, when it came (seconds into the hour), what happened, how long it waited for its green, how many
/// seconds its green was brought forward (early green) or held for it (extended).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Bus {
    pub arm: u32,
    pub at_s: u32,
    pub outcome: Outcome,
    pub wait_s: u32,
    pub early_s: u32,
    pub extended_s: u32,
}

/// The delay priority for street `from` moved to approach `to`: the seconds of green its phase lost to an early green,
/// and the seconds its phase started later after an extended green, added up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Moved {
    pub from: u32,
    pub to: u32,
    pub seconds: u32,
}

/// The buses of one street with priority, added up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct Totals {
    pub arm: u32,
    pub mode: Mode,
    /// Its buses an hour.
    pub rate: u32,
    pub buses: u32,
    pub on_arrival: u32,
    pub early_green: u32,
    pub extended: u32,
    pub delayed: u32,
    pub not_served: u32,
    /// The longest a bus waited for its green; None when no bus came.
    pub longest_wait_s: Option<u32>,
    /// The delay its priority moved to the other approaches (`Moved`), in seconds.
    pub moved_s: u32,
}

/// Something about a street with priority the person should act on. By street uid; no wording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Finding {
    /// The street has priority, but no transit any more (an edit took it away): it is kept, and gives nothing until the
    /// street has transit again.
    NoTransit { arm: u32 },
    /// The street has priority, but no phase of its own any more: it is kept, and gives nothing until it has one again.
    NoPhase { arm: u32 },
    /// An early green for the street's buses was kept shorter `times` times, by `held_s` seconds in all, because the
    /// phase of street `phase` was serving a walk and its clearance or a bicycle minimum and clearance, which are never
    /// shortened.
    ClearanceHeld { arm: u32, phase: u32, times: u32, held_s: u32 },
}

impl Finding {
    /// What the finding is about, whatever its figures: its kind and its streets.
    pub fn about(&self) -> (u8, u32, u32) {
        match *self {
            Finding::NoTransit { arm } => (30, arm, 0),
            Finding::NoPhase { arm } => (31, arm, 0),
            Finding::ClearanceHeld { arm, phase, .. } => (32, arm, phase),
        }
    }
}

/// How the plan served the buses of one hour.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Summary {
    /// Every bus, in the order they came.
    pub buses: Vec<Bus>,
    /// One for each street with priority that has transit and a phase of its own, in the order of the bearings.
    pub totals: Vec<Totals>,
    /// The delay moved to each approach, by the street whose priority moved it, in the order it was first moved.
    pub moved: Vec<Moved>,
    pub findings: Vec<Finding>,
}

/// The buses of one street with priority: its street, what it is given, its own phase (by its place in the plan) and
/// when each bus comes.
pub(crate) struct Street {
    pub(crate) arm: u32,
    pub(crate) mode: Mode,
    pub(crate) phase: usize,
    pub(crate) at_s: Vec<u32>,
}

/// The streets of `s` with priority (in the order of the bearings) that have transit and a phase of their own, each
/// with its buses, with `out` given a row of totals for each and a finding for each left out.
pub(crate) fn streets(s: &State, plan: &Plan, out: &mut Summary) -> Vec<Street> {
    let mut arms: Vec<&Arm> = s.arms.iter().collect();
    arms.sort_by_key(|a| a.bearing);
    let mut streets = Vec::new();
    for (arm, p) in arms.into_iter().filter_map(|a| priority_of(a).map(|p| (a, p))) {
        if !has_transit(arm) {
            out.findings.push(Finding::NoTransit { arm: arm.uid });
            continue;
        }
        let Some(phase) = own_phase(plan, arm.uid) else {
            out.findings.push(Finding::NoPhase { arm: arm.uid });
            continue;
        };
        out.totals.push(Totals {
            arm: arm.uid,
            mode: p.mode,
            rate: p.buses,
            buses: 0,
            on_arrival: 0,
            early_green: 0,
            extended: 0,
            delayed: 0,
            not_served: 0,
            longest_wait_s: None,
            moved_s: 0,
        });
        streets.push(Street { arm: arm.uid, mode: p.mode, phase, at_s: arrivals(arm.uid, p.buses) });
    }
    streets
}

/// The least green each phase of `plan` keeps whatever comes, besides what a demand holds: an actuated phase's floor,
/// the walk and clearance of each crossing on recall it walks (`distances`, `read_model::crossing_distances`) and the
/// bicycle minimum green and clearance of each bike movement on recall it runs (`lengths`, `read_model::ride_lengths`).
pub(crate) fn kept(s: &State, plan: &Plan, rules: &[Rule], distances: &[(u32, i32)], lengths: &[(u32, i32)]) -> Vec<u32> {
    let arm = |uid: u32| s.arms.iter().find(|a| a.uid == uid);
    let of = |list: &[(u32, i32)], uid: u32| list.iter().find(|(u, _)| *u == uid).map(|(_, d)| *d);
    plan.phases
        .iter()
        .zip(rules)
        .map(|(p, rule)| {
            let floor = match rule {
                Rule::Fixed => 0,
                Rule::Actuated { floor_s, .. } => *floor_s,
            };
            let walks = p
                .walks
                .iter()
                .filter(|&&w| arm(w).is_some_and(|a| a.calls.is_none() && a.crossing.is_some()))
                .filter_map(|&w| of(distances, w))
                .map(calls::need_s)
                .max()
                .unwrap_or(0);
            let rides = p
                .rides
                .iter()
                .filter(|&&r| arm(r).is_some_and(|a| a.cyclists.is_none()))
                .filter_map(|&r| of(lengths, r))
                .map(bicycles::need_s)
                .max()
                .unwrap_or(0);
            floor.max(walks).max(rides)
        })
        .collect()
}

/// The buses as a demand on the controller: who waits for which street's phase, the early greens and extensions given,
/// and what happened to every bus.
pub(crate) struct Buses<'a> {
    streets: &'a [Street],
    /// Each phase's street, and the least green its rule gives (`GREEN_MIN_S` fixed, the minimum green actuated) and
    /// what it keeps besides (`kept`).
    uids: Vec<u32>,
    least: Vec<u32>,
    kept: Vec<u32>,
    /// Each street's first bus not yet served (its place in `at_s`), and every bus as it was served.
    next: Vec<usize>,
    rows: Vec<Vec<Bus>>,
    /// How many greens of its phase each street has had, the last one given priority (by that count), and the seconds
    /// of early green given toward its next green.
    greens: Vec<u32>,
    given: Vec<Option<u32>>,
    early: Vec<u32>,
    /// The seconds each street's last green was extended, and the approaches whose next green has borne them since.
    shift: Vec<u32>,
    charged: Vec<Vec<u32>>,
    moved: Vec<Moved>,
    held: Vec<Finding>,
}

/// What an early green does to one phase's green: whose bus it is for (its place in the streets), the green it is
/// given, and the green the bus asked for before what is never shortened (equal, or shorter when a walk or a bicycle
/// clearance kept the green).
struct Cut {
    street: usize,
    green_s: u32,
    asked_s: u32,
}

impl<'a> Buses<'a> {
    /// `rules` the phases' rules, `kept` what each keeps whatever comes (`kept`).
    pub(crate) fn new(plan: &Plan, rules: &[Rule], streets: &'a [Street], kept: Vec<u32>) -> Buses<'a> {
        let least = rules
            .iter()
            .map(|r| match r {
                Rule::Fixed => GREEN_MIN_S as u32,
                Rule::Actuated { bounds, .. } => bounds.min_green_s,
            })
            .collect();
        let n = streets.len();
        let rows = streets
            .iter()
            .map(|st| st.at_s.iter().map(|&at_s| Bus { arm: st.arm, at_s, outcome: Outcome::NotServed, wait_s: 0, early_s: 0, extended_s: 0 }).collect())
            .collect();
        Buses {
            streets,
            uids: plan.phases.iter().map(|p| p.uid).collect(),
            least,
            kept,
            next: vec![0; n],
            rows,
            greens: vec![0; n],
            given: vec![None; n],
            early: vec![0; n],
            shift: vec![0; n],
            charged: vec![Vec::new(); n],
            moved: Vec::new(),
            held: Vec::new(),
        }
    }

    /// When street `x`'s first bus not yet served comes.
    fn first(&self, x: usize) -> Option<u64> {
        self.streets[x].at_s.get(self.next[x]).map(|&t| u64::from(t))
    }

    /// Whether street `x` may be given priority for its next green: not when it was given priority for the last one.
    fn allowed(&self, x: usize) -> bool {
        !(self.greens[x] > 0 && self.given[x] == Some(self.greens[x] - 1))
    }

    /// The early green phase `k` gives, its green starting at `green_start` and running `green_s` with `held_s` held by
    /// the demands served in it, if a bus asks for one: the street whose own phase comes next after `k` and has a bus
    /// waiting, or coming before the green would end, that may be given an early green.
    fn cut_for(&self, k: usize, green_start: u64, green_s: u32, held_s: u32) -> Option<Cut> {
        let end = green_start + u64::from(green_s);
        // A phase a bus of its own street is due to use is not cut.
        if (0..self.streets.len()).any(|y| self.streets[y].phase == k && self.first(y).is_some_and(|t| t < end + u64::from(EXTENSION_MAX_S))) {
            return None;
        }
        let n = self.uids.len();
        let x = (1..n).map(|d| (k + d) % n).find_map(|j| {
            (0..self.streets.len()).find(|&x| {
                let st = &self.streets[x];
                st.phase == j && st.mode.early_green() && self.allowed(x) && self.early[x] < EARLY_GREEN_MAX_S && self.first(x).is_some_and(|t| t < end)
            })
        })?;
        let came = u32::try_from(self.first(x)?.saturating_sub(green_start)).unwrap_or(u32::MAX);
        let asked_s = came.max(self.least[k]).max(green_s.saturating_sub(EARLY_GREEN_MAX_S - self.early[x])).min(green_s);
        let green = asked_s.max(self.least[k]).max(self.kept[k]).max(held_s).min(green_s);
        (green < green_s || green > asked_s).then_some(Cut { street: x, green_s: green, asked_s })
    }

    /// Adds `seconds` to the delay street `from`'s priority moved to approach `to`.
    fn move_delay(&mut self, from: usize, to: u32, seconds: u32) {
        let from = self.streets[from].arm;
        if seconds == 0 || from == to {
            return;
        }
        match self.moved.iter_mut().find(|m| m.from == from && m.to == to) {
            Some(m) => m.seconds = m.seconds.saturating_add(seconds),
            None => self.moved.push(Moved { from, to, seconds }),
        }
    }

    /// Every bus (in the order they came), the delay moved, and the findings about greens kept for a clearance.
    pub(crate) fn finish(self) -> (Vec<Bus>, Vec<Moved>, Vec<Finding>) {
        let mut buses: Vec<(usize, Bus)> = self.rows.into_iter().enumerate().flat_map(|(x, r)| r.into_iter().map(move |b| (x, b))).collect();
        buses.sort_by_key(|(x, b)| (b.at_s, *x));
        (buses.into_iter().map(|(_, b)| b).collect(), self.moved, self.held)
    }
}

impl control::Demand for Buses<'_> {
    /// A bus waiting calls its street's phase.
    fn calls(&self, k: usize, at: u64) -> bool {
        (0..self.streets.len()).any(|x| self.streets[x].phase == k && self.first(x).is_some_and(|t| t <= at))
    }

    fn hold(&self, _k: usize, _start: u64) -> u32 {
        0
    }

    fn cut(&self, k: usize, green_start: u64, green_s: u32, held_s: u32) -> u32 {
        self.cut_for(k, green_start, green_s, held_s).map_or(green_s, |c| c.green_s)
    }

    /// The seconds the green of a street's phase is held for its first bus coming within `EXTENSION_MAX_S` after it
    /// would end, until that bus has passed.
    fn prolong(&self, k: usize, _start: u64, green_end: u64) -> u32 {
        (0..self.streets.len())
            .filter(|&x| self.streets[x].phase == k && self.streets[x].mode.extends() && self.allowed(x))
            .filter_map(|x| self.streets[x].at_s[self.next[x]..].iter().map(|&t| u64::from(t)).find(|&t| t >= green_end))
            .filter(|&t| t < green_end + u64::from(EXTENSION_MAX_S))
            .map(|t| u32::try_from(t + 1 - green_end).unwrap_or(u32::MAX))
            .max()
            .unwrap_or(0)
    }

    /// Records the early green the phase gave and any clearance that kept it, the delay moved to it, and serves the
    /// buses of the street whose phase it is.
    fn ran(&mut self, turn: &Turn) {
        if turn.skipped {
            return;
        }
        let (k, uid) = (turn.phase, self.uids[turn.phase]);
        let before = turn.green_s() - turn.prolonged_s + turn.cut_s;
        if let Some(cut) = self.cut_for(k, turn.green_start, before, turn.held_s) {
            let x = cut.street;
            let seconds = before - cut.green_s;
            if seconds > 0 {
                self.early[x] += seconds;
                self.given[x] = Some(self.greens[x]);
                self.move_delay(x, uid, seconds);
            }
            if cut.green_s > cut.asked_s {
                let (arm, more) = (self.streets[x].arm, cut.green_s - cut.asked_s);
                match self.held.iter_mut().find(|f| matches!(f, Finding::ClearanceHeld { arm: a, phase, .. } if *a == arm && *phase == uid)) {
                    Some(Finding::ClearanceHeld { times, held_s, .. }) => {
                        *times += 1;
                        *held_s += more;
                    }
                    _ => self.held.push(Finding::ClearanceHeld { arm, phase: uid, times: 1, held_s: more }),
                }
            }
        }
        // A green that runs after another street's extended one started that much later: it bears the delay, once.
        for x in (0..self.streets.len()).filter(|&x| self.streets[x].phase != k) {
            if self.shift[x] > 0 && !self.charged[x].contains(&uid) {
                self.charged[x].push(uid);
                let shift = self.shift[x];
                self.move_delay(x, uid, shift);
            }
        }
        for x in (0..self.streets.len()).filter(|&x| self.streets[x].phase == k) {
            let base_end = turn.green_end - u64::from(turn.prolonged_s);
            while let Some(t) = self.first(x).filter(|&t| t < turn.green_end) {
                let bus = &mut self.rows[x][self.next[x]];
                if t >= turn.start {
                    bus.outcome = if t < base_end { Outcome::OnArrival } else { Outcome::Extended };
                    bus.extended_s = if t < base_end { 0 } else { turn.prolonged_s };
                } else {
                    bus.wait_s = u32::try_from(turn.start - t).unwrap_or(u32::MAX);
                    bus.outcome = if self.early[x] > 0 { Outcome::EarlyGreen } else { Outcome::Delayed };
                    bus.early_s = self.early[x];
                }
                self.next[x] += 1;
            }
            if turn.prolonged_s > 0 {
                self.given[x] = Some(self.greens[x]);
            }
            // The other approaches' next greens start that much later; each bears it as it runs.
            self.shift[x] = turn.prolonged_s;
            self.charged[x].clear();
            self.greens[x] += 1;
            self.early[x] = 0;
        }
    }

    fn pending(&self) -> bool {
        (0..self.streets.len()).any(|x| self.first(x).is_some())
    }

    fn next_after(&self, at: u64) -> Option<u64> {
        (0..self.streets.len()).filter_map(|x| self.streets[x].at_s[self.next[x]..].iter().map(|&t| u64::from(t)).find(|&t| t > at)).min()
    }

    fn last(&self) -> u64 {
        self.streets.iter().filter_map(|st| st.at_s.last()).map(|&t| u64::from(t)).max().unwrap_or(0)
    }

    fn most_extension(&self) -> u32 {
        if self.streets.is_empty() { 0 } else { EXTENSION_MAX_S }
    }

    fn most_prolong(&self) -> u32 {
        self.most_extension()
    }
}

/// `out`'s buses, delay moved, findings and totals from what the run did.
pub(crate) fn tally(out: &mut Summary, (buses, moved, held): (Vec<Bus>, Vec<Moved>, Vec<Finding>)) {
    out.buses = buses;
    out.moved = moved;
    out.findings.extend(held);
    for t in &mut out.totals {
        for b in out.buses.iter().filter(|b| b.arm == t.arm) {
            t.buses += 1;
            match b.outcome {
                Outcome::OnArrival => t.on_arrival += 1,
                Outcome::EarlyGreen => t.early_green += 1,
                Outcome::Extended => t.extended += 1,
                Outcome::Delayed => t.delayed += 1,
                Outcome::NotServed => t.not_served += 1,
            }
            if b.outcome != Outcome::NotServed {
                t.longest_wait_s = Some(t.longest_wait_s.unwrap_or(0).max(b.wait_s));
            }
        }
        t.moved_s = out.moved.iter().filter(|m| m.from == t.arm).map(|m| m.seconds).sum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::calls::{Asked, Walks};
    use crate::junction::model::control::{Bounds, Demand, run};
    use crate::junction::model::signal::{self, Move, Phase, SignalPlan, StoredPhase};
    use crate::junction::model::{Junction, PRIORITY, Timing, actuation, junction_biked};

    fn at(s: &State, bearing: i32) -> u32 {
        s.arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    fn arm_mut(s: &mut State, uid: u32) -> &mut Arm {
        s.arms.iter_mut().find(|a| a.uid == uid).unwrap()
    }

    /// The four-way of `junction_biked` (a signal, every street's traffic straight on, a phase each) with a bus lane on
    /// its north street, and that street.
    fn with_bus_lane() -> (Junction, u32) {
        let mut j = junction_biked();
        let n = at(j.current(), 0);
        assert!(j.set_bus_lane(n, true));
        (j, n)
    }

    /// A phase of street `uid` with a `green_s` green, yellow 3 s and all-red 2 s, walking `walks`.
    fn phase(uid: u32, green_s: u32, walks: &[u32]) -> Phase {
        let timing = Timing { green_s, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        Phase { uid, timing, length_s: timing.length_s(), walks: walks.to_vec(), rides: vec![], moves: vec![Move { from: uid, to: 99 }] }
    }

    fn plan_of(phases: Vec<Phase>) -> Plan {
        let cycle_s = phases.iter().map(|p| p.length_s).sum();
        Plan { phases, cycle_s, suggested: false }
    }

    fn street(arm: u32, phase: usize, mode: Mode, at_s: Vec<u32>) -> Street {
        Street { arm, mode, phase, at_s }
    }

    /// The buses of `streets` served by `plan` on `rules`, each phase keeping `kept`, with the pedestrian calls `asked`:
    /// the turns that ran as (phase, start, green, end), and what the buses' demand gave.
    fn serve(
        plan: &Plan,
        rules: &[Rule],
        streets: &[Street],
        kept: Vec<u32>,
        asked: &[Asked],
    ) -> (Vec<(usize, u64, u32, u64)>, Vec<Bus>, Vec<Moved>, Vec<Finding>) {
        let mut walks = Walks::new(plan, rules, asked);
        let mut buses = Buses::new(plan, rules, streets, kept);
        let turns = run(plan, rules, &mut [&mut walks as &mut dyn Demand, &mut buses], 100_000);
        let ran = turns.iter().filter(|t| !t.skipped).map(|t| (t.phase, t.start, t.green_s(), t.end)).collect();
        let (b, m, f) = buses.finish();
        (ran, b, m, f)
    }

    fn bus(arm: u32, at_s: u32, outcome: Outcome, wait_s: u32, early_s: u32, extended_s: u32) -> Bus {
        Bus { arm, at_s, outcome, wait_s, early_s, extended_s }
    }

    /// Two phases of 25 s green, 30 s each: street 1's first, street 2's second.
    fn two() -> Plan {
        plan_of(vec![phase(1, 25, &[]), phase(2, 25, &[])])
    }

    #[test]
    fn the_placeholders_are_the_ones_the_issue_settled() {
        assert_eq!((EARLY_GREEN_MAX_S, EXTENSION_MAX_S), (10, 10));
        assert_eq!((DEFAULT_BUSES_PER_HOUR, MAX_BUSES_PER_HOUR), (12, 60));
    }

    #[test]
    fn the_buses_of_an_hour_are_the_same_every_time_spread_over_the_hour_and_unlike_the_street_s_other_demands() {
        let buses = arrivals(7, 12);
        assert_eq!(buses, arrivals(7, 12), "seeded");
        assert_eq!(buses.len(), 12);
        for (k, t) in buses.iter().enumerate() {
            assert!((k as u32 * 300..(k as u32 + 1) * 300).contains(t), "one in each slot of 300 s: {buses:?}");
        }
        assert_ne!(arrivals(7, 12), calls::arrivals(7, 12), "a stream of its own, not the pedestrian calls'");
        assert_ne!(arrivals(7, 12), actuation::arrivals(7, 12), "nor the vehicles'");
        assert_ne!(arrivals(7, 12), bicycles::arrivals(7, 12), "nor the cyclists'");
        assert_eq!(arrivals(7, u32::MAX).len(), 60, "read as the bound");
        assert!(arrivals(7, 0).is_empty());
        // Pinned: a change of generator or seed changes every summary, so it has to be a decision.
        assert_eq!(arrivals(1, 4), PINNED_BUSES);
    }

    const PINNED_BUSES: [u32; 4] = [622, 978, 2516, 3271];

    #[test]
    fn a_bus_coming_during_red_cuts_the_phases_before_its_own_by_at_most_10_s_keeping_the_yellow_and_all_red() {
        // Street 2's bus comes at 3 s, during street 1's green (0 to 25 s): that green is cut to 15 s (10 s at most), so
        // street 2's phase starts at 20 s, not 30 s.
        let plan = two();
        let (ran, buses, moved, found) = serve(&plan, &control::fixed(&plan), &[street(2, 1, Mode::EarlyGreen, vec![3])], vec![0, 0], &[]);
        assert_eq!(ran, vec![(0, 0, 15, 20), (1, 20, 25, 50)], "the 3 s yellow and 2 s all-red kept");
        assert_eq!(buses, vec![bus(2, 3, Outcome::EarlyGreen, 17, 10, 0)]);
        assert_eq!(moved, vec![Moved { from: 2, to: 1, seconds: 10 }], "street 1's green lost 10 s");
        assert!(found.is_empty());
        // Coming late in the green: never ended before the bus came (at 20 s, so 5 s cut).
        let (ran, buses, _, _) = serve(&plan, &control::fixed(&plan), &[street(2, 1, Mode::EarlyGreen, vec![20])], vec![0, 0], &[]);
        assert_eq!(ran[0], (0, 0, 20, 25));
        assert_eq!(buses, vec![bus(2, 20, Outcome::EarlyGreen, 5, 5, 0)]);
        // Over three phases the 10 s are spread: the first is cut to its minimum's 5 s (7 s), the second by the 3 s left.
        let plan = plan_of(vec![phase(1, 12, &[]), phase(3, 25, &[]), phase(2, 25, &[])]);
        let (ran, buses, moved, _) = serve(&plan, &control::fixed(&plan), &[street(2, 2, Mode::EarlyGreen, vec![0])], vec![0; 3], &[]);
        assert_eq!(&ran[..2], &[(0, 0, 5, 10), (1, 10, 22, 37)], "never below the 5 s minimum green");
        assert_eq!(buses, vec![bus(2, 0, Outcome::EarlyGreen, 37, 10, 0)]);
        assert_eq!(moved, vec![Moved { from: 2, to: 1, seconds: 7 }, Moved { from: 2, to: 3, seconds: 3 }]);
        // Extension only: no early green, the bus waits.
        let plan = two();
        let (ran, buses, moved, _) = serve(&plan, &control::fixed(&plan), &[street(2, 1, Mode::Extension, vec![3])], vec![0, 0], &[]);
        assert_eq!(ran[0], (0, 0, 25, 30));
        assert_eq!(buses, vec![bus(2, 3, Outcome::Delayed, 27, 0, 0)]);
        assert!(moved.is_empty());
    }

    #[test]
    fn a_bus_coming_just_after_its_green_would_end_holds_it_until_it_has_passed_at_most_10_s() {
        let plan = two();
        // Street 1's green ends at 25 s; its bus comes at 27 s: the green is held 3 s, to 28 s, and street 2's next green,
        // which runs (a second bus keeps the signal running), starts 3 s later.
        let (ran, buses, moved, _) = serve(&plan, &control::fixed(&plan), &[street(1, 0, Mode::Extension, vec![27, 70])], vec![0, 0], &[]);
        assert_eq!(&ran[..2], &[(0, 0, 28, 33), (1, 33, 25, 63)]);
        assert_eq!(buses[0], bus(1, 27, Outcome::Extended, 0, 0, 3));
        assert_eq!(moved, vec![Moved { from: 1, to: 2, seconds: 3 }]);
        // With nobody left to serve the signal stops after the held green: no other green ran later, so none bore delay.
        let (_, _, moved, _) = serve(&plan, &control::fixed(&plan), &[street(1, 0, Mode::Extension, vec![27])], vec![0, 0], &[]);
        assert!(moved.is_empty(), "{moved:?}");
        // At 35 s it is past the 10 s: no extension, it waits for the next green (at 60 s).
        let (ran, buses, _, _) = serve(&plan, &control::fixed(&plan), &[street(1, 0, Mode::Extension, vec![35])], vec![0, 0], &[]);
        assert_eq!(ran[0], (0, 0, 25, 30));
        assert_eq!(buses, vec![bus(1, 35, Outcome::Delayed, 25, 0, 0)]);
        // During the green: on arrival, nothing held.
        let (ran, buses, moved, _) = serve(&plan, &control::fixed(&plan), &[street(1, 0, Mode::Both, vec![10])], vec![0, 0], &[]);
        assert_eq!(ran[0], (0, 0, 25, 30));
        assert_eq!(buses, vec![bus(1, 10, Outcome::OnArrival, 0, 0, 0)]);
        assert!(moved.is_empty());
        // Early green only: no extension.
        let (_, buses, _, _) = serve(&plan, &control::fixed(&plan), &[street(1, 0, Mode::EarlyGreen, vec![27])], vec![0, 0], &[]);
        assert_eq!(buses[0].outcome, Outcome::EarlyGreen, "it came during red, and street 2's green was cut for it");
    }

    #[test]
    fn an_extension_moves_delay_only_to_the_approaches_whose_next_green_runs_each_once() {
        // Street 1 fixed, street 2 actuated and called by nobody (skipped, taking no time), street 3 fixed. The bus at
        // 27 s holds street 1's green 3 s: street 3's next green starts 3 s later; street 2's is skipped and bears none.
        let plan = plan_of(vec![phase(1, 25, &[]), phase(2, 25, &[]), phase(3, 25, &[])]);
        let bounds = Bounds { min_green_s: 7, gap_s: 3, max_green_s: 45 };
        let rules = vec![Rule::Fixed, Rule::Actuated { bounds, floor_s: 7 }, Rule::Fixed];
        let (ran, buses, moved, _) = serve(&plan, &rules, &[street(1, 0, Mode::Extension, vec![27, 100])], vec![0, 7, 0], &[]);
        assert_eq!(&ran[..2], &[(0, 0, 28, 33), (2, 33, 25, 63)], "street 2 skipped");
        assert_eq!(buses[0], bus(1, 27, Outcome::Extended, 0, 0, 3));
        assert_eq!(moved, vec![Moved { from: 1, to: 3, seconds: 3 }]);
    }

    #[test]
    fn a_street_given_priority_for_one_green_is_given_none_for_the_next() {
        let plan = two();
        // Cycle 1: the bus at 3 s gets an early green (street 1's green cut to 15 s, street 2's phase from 20 to 50 s).
        // Cycle 2: street 1 runs 50 to 80 s; the bus at 55 s may have none, so street 1's green is not cut and it waits
        // to 80 s. Cycle 3 (street 1 from 110 s): the bus at 115 s gets an early green again.
        let rules = control::fixed(&plan);
        let (ran, buses, moved, _) = serve(&plan, &rules, &[street(2, 1, Mode::EarlyGreen, vec![3, 55, 115])], vec![0, 0], &[]);
        assert_eq!(&ran[..5], &[(0, 0, 15, 20), (1, 20, 25, 50), (0, 50, 25, 80), (1, 80, 25, 110), (0, 110, 15, 130)]);
        assert_eq!(buses, vec![bus(2, 3, Outcome::EarlyGreen, 17, 10, 0), bus(2, 55, Outcome::Delayed, 25, 0, 0), bus(2, 115, Outcome::EarlyGreen, 15, 10, 0)]);
        assert_eq!(moved, vec![Moved { from: 2, to: 1, seconds: 20 }]);
        // The same for an extension: held at 27 s, not again in the next cycle (its green 60 to 85 s, a bus at 86 s).
        let (ran, buses, _, _) = serve(&plan, &rules, &[street(1, 0, Mode::Extension, vec![27, 89])], vec![0, 0], &[]);
        assert_eq!(&ran[..3], &[(0, 0, 28, 33), (1, 33, 25, 63), (0, 63, 25, 93)]);
        assert_eq!(buses, vec![bus(1, 27, Outcome::Extended, 0, 0, 3), bus(1, 89, Outcome::Delayed, 34, 0, 0)]);
        // Both in one green are one priority: the early green and then the extension of the same green.
        let (ran, buses, _, _) = serve(&plan, &rules, &[street(2, 1, Mode::Both, vec![3, 46])], vec![0, 0], &[]);
        assert_eq!(&ran[..2], &[(0, 0, 15, 20), (1, 20, 27, 52)]);
        assert_eq!(buses, vec![bus(2, 3, Outcome::EarlyGreen, 17, 10, 0), bus(2, 46, Outcome::Extended, 0, 0, 2)]);
    }

    #[test]
    fn an_early_green_never_shortens_a_walk_or_a_bicycle_clearance_and_says_so_as_a_finding() {
        let plan = two();
        let rules = control::fixed(&plan);
        // Street 1's phase walks a crossing on recall needing 20 s (or runs a bike movement needing it): the bus asks for
        // 15 s, and is given 20 s; the 5 s kept are a finding, never applied silently.
        let (ran, buses, moved, found) = serve(&plan, &rules, &[street(2, 1, Mode::EarlyGreen, vec![3])], vec![20, 0], &[]);
        assert_eq!(ran[0], (0, 0, 20, 25));
        assert_eq!(buses, vec![bus(2, 3, Outcome::EarlyGreen, 22, 5, 0)]);
        assert_eq!(moved, vec![Moved { from: 2, to: 1, seconds: 5 }]);
        assert_eq!(found, vec![Finding::ClearanceHeld { arm: 2, phase: 1, times: 1, held_s: 5 }]);
        // A pedestrian call on call being served there holds 22 s: the green is not cut below it.
        let plan = plan_of(vec![phase(1, 25, &[8]), phase(2, 25, &[])]);
        let walk = [Asked { arm: 8, need_s: 22, at_s: vec![0], passive: false }];
        let (ran, _, _, found) = serve(&plan, &rules, &[street(2, 1, Mode::EarlyGreen, vec![3])], vec![0, 0], &walk);
        assert_eq!(ran[0], (0, 0, 22, 27));
        assert_eq!(found, vec![Finding::ClearanceHeld { arm: 2, phase: 1, times: 1, held_s: 7 }]);
        // Kept past the whole green: nothing cut, the bus delayed, and still a finding.
        let (ran, buses, moved, found) = serve(&two(), &rules, &[street(2, 1, Mode::EarlyGreen, vec![3])], vec![30, 0], &[]);
        assert_eq!(ran[0], (0, 0, 25, 30));
        assert_eq!(buses, vec![bus(2, 3, Outcome::Delayed, 27, 0, 0)]);
        assert!(moved.is_empty());
        assert_eq!(found, vec![Finding::ClearanceHeld { arm: 2, phase: 1, times: 1, held_s: 10 }]);
    }

    #[test]
    fn a_phase_a_bus_of_its_own_street_is_due_to_use_is_not_cut_for_another() {
        let plan = two();
        // Street 2's bus at 3 s would cut street 1's green, but street 1's own bus comes at 30 s, within its extension.
        let streets = [street(1, 0, Mode::Both, vec![30]), street(2, 1, Mode::EarlyGreen, vec![3])];
        let (ran, buses, _, _) = serve(&plan, &control::fixed(&plan), &streets, vec![0, 0], &[]);
        assert_eq!(ran[0], (0, 0, 31, 36), "held 6 s for street 1's bus, not cut for street 2's");
        assert_eq!(buses, vec![bus(2, 3, Outcome::Delayed, 33, 0, 0), bus(1, 30, Outcome::Extended, 0, 0, 6)]);
    }

    #[test]
    fn a_bus_calls_an_actuated_phase_of_its_street() {
        let plan = two();
        let rules = vec![Rule::Fixed, Rule::Actuated { bounds: Bounds { min_green_s: 7, gap_s: 3, max_green_s: 45 }, floor_s: 7 }];
        let (ran, buses, _, _) = serve(&plan, &rules, &[street(2, 1, Mode::Extension, vec![100])], vec![0, 7], &[]);
        assert!(ran.iter().any(|r| r.0 == 1), "{ran:?}");
        assert_eq!(buses[0].outcome, Outcome::Delayed);
        // An actuated phase cut for a bus: never below its minimum green.
        let rules = vec![Rule::Actuated { bounds: Bounds { min_green_s: 12, gap_s: 3, max_green_s: 45 }, floor_s: 12 }, Rule::Fixed];
        let mut walks = Walks::new(&plan, &rules, &[]);
        let streets = [street(2, 1, Mode::EarlyGreen, vec![1])];
        let approach = [actuation::Approach { arm: 1, rate: 3, at_s: vec![0, 10, 20] }];
        let mut vehicles = actuation::Vehicles::new(&plan, &approach);
        let mut buses = Buses::new(&plan, &rules, &streets, vec![12, 0]);
        let turns = run(&plan, &rules, &mut [&mut walks as &mut dyn Demand, &mut vehicles, &mut buses], 100_000);
        let first = turns.iter().find(|t| t.phase == 0 && !t.skipped).unwrap();
        assert_eq!((first.base_s, first.green_s(), first.cut_s), (13, 12, 1), "the vehicles kept it to 13 s; the bus cut it to its 12 s minimum");
    }

    #[test]
    fn priority_is_allowed_only_on_a_street_with_transit_and_a_phase_of_its_own() {
        let (j, n) = with_bus_lane();
        let s = j.current().clone();
        let e = at(&s, 90);
        assert_eq!(refusal(&s, 0, n), None);
        assert_eq!(refusal(&s, 0, e), Some(Refusal::PriorityNoTransit), "east has no transit");
        // Each transit measure counts.
        for give in [|a: &mut Arm| a.approach = 1, |a: &mut Arm| a.stop = 1, |a: &mut Arm| a.filter = true] {
            let mut t = s.clone();
            give(arm_mut(&mut t, e));
            assert!(has_transit(t.arms.iter().find(|a| a.uid == e).unwrap()));
        }
        // A plan with no phase of north's own.
        let mut no_phase = s.clone();
        let so = at(&s, 180);
        no_phase.signal_plan =
            Some(SignalPlan { phases: vec![StoredPhase { moves: vec![Move { from: so, to: n }], walks: vec![], rides: vec![], arm: None }] });
        assert_eq!(refusal(&no_phase, 0, n), Some(Refusal::PriorityNoPhase));
        let mut not_signal = s.clone();
        not_signal.control = PRIORITY;
        assert_eq!(refusal(&not_signal, 0, n), Some(Refusal::NotSignal));
        assert_eq!(refusal(&s, 0, 12_345), Some(Refusal::DoesNotFit));
    }

    #[test]
    fn priority_is_set_its_buses_given_within_the_bound_cleared_and_undone_like_any_edit() {
        let (mut j, n) = with_bus_lane();
        let e = at(j.current(), 90);
        assert!(!j.set_buses(n, 10));
        assert_eq!(j.refusal(), Some(Refusal::NotOnPriority));
        assert!(!j.set_transit_priority(e, Some(Mode::EarlyGreen)));
        assert_eq!(j.refusal(), Some(Refusal::PriorityNoTransit));
        assert!(j.set_transit_priority(n, Some(Mode::EarlyGreen)));
        assert_eq!(j.arm(n).unwrap().transit_priority, Some(Priority { mode: Mode::EarlyGreen, buses: DEFAULT_BUSES_PER_HOUR }));
        assert_eq!(j.current().label.key, "jn-rev-priority-early-green");
        assert!(j.set_buses(n, 60));
        assert_eq!(j.current().label.key, "jn-rev-buses");
        for bad in [-1, 61] {
            assert!(!j.set_buses(n, bad), "{bad}");
            assert_eq!(j.refusal(), Some(Refusal::BusesOutOfRange));
        }
        let edits = j.revisions().count();
        assert!(j.set_buses(n, 60), "the rate it has: taken");
        assert!(j.set_transit_priority(n, Some(Mode::EarlyGreen)), "the mode it has: taken");
        assert_eq!(j.revisions().count(), edits, "and no edit");
        assert!(j.set_transit_priority(n, Some(Mode::Both)));
        assert_eq!(j.arm(n).unwrap().transit_priority, Some(Priority { mode: Mode::Both, buses: 60 }), "the rate kept");
        assert_eq!(j.current().label.key, "jn-rev-priority-both");
        assert!(j.set_transit_priority(n, Some(Mode::Extension)));
        assert_eq!(j.current().label.key, "jn-rev-priority-extension");
        assert!(j.set_transit_priority(n, None));
        assert_eq!((j.arm(n).unwrap().transit_priority, j.current().label.key), (None, "jn-rev-priority-off"));
        assert!(j.undo() && j.arm(n).unwrap().transit_priority.is_some());
        // The bus lane taken away: kept (a finding says so), and none is always allowed.
        assert!(j.set_bus_lane(n, false));
        assert!(j.arm(n).unwrap().transit_priority.is_some());
        assert!(!j.set_transit_priority(n, Some(Mode::Both)));
        assert_eq!(j.refusal(), Some(Refusal::PriorityNoTransit));
        assert!(j.undo());
        // Off a signal it goes, as the timing does, and the revision says the timing went.
        assert!(j.set_control(PRIORITY));
        assert_eq!(j.arm(n).unwrap().transit_priority, None);
        assert_eq!(j.current().label.key, "jn-rev-control-timing");
        assert!(!j.set_transit_priority(n, Some(Mode::Both)));
        assert_eq!(j.refusal(), Some(Refusal::NotSignal));
    }

    #[test]
    fn a_junction_with_no_street_given_priority_is_written_as_before_and_its_priority_only_once_set() {
        let (j, n) = with_bus_lane();
        let mut s = j.current().clone();
        let text = serde_json::to_string(&s).unwrap();
        assert!(!text.contains("transit_priority"), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        arm_mut(&mut s, n).transit_priority = Some(Priority { mode: Mode::Both, buses: 12 });
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""transit_priority":{"mode":"both","buses":12}"#), "{text}");
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.arms.iter().find(|a| a.uid == n).unwrap().transit_priority, Some(Priority { mode: Mode::Both, buses: 12 }));
        // A document's rate past the bound is read as the bound.
        arm_mut(&mut s, n).transit_priority = Some(Priority { mode: Mode::Both, buses: 5_000 });
        assert_eq!(priority_of(s.arms.iter().find(|a| a.uid == n).unwrap()).unwrap().buses, MAX_BUSES_PER_HOUR);
    }

    #[test]
    fn a_street_kept_with_priority_but_no_transit_or_phase_is_a_finding_and_gives_nothing() {
        let (j, n) = with_bus_lane();
        let mut s = j.current().clone();
        arm_mut(&mut s, n).transit_priority = Some(Priority { mode: Mode::Both, buses: 12 });
        arm_mut(&mut s, n).bus_lane = false;
        let plan = signal::plan(&s, 0).unwrap();
        let h = control::hour(&s, &plan, &[], &[]);
        assert_eq!(h.transit.findings, vec![Finding::NoTransit { arm: n }]);
        assert!(h.transit.totals.is_empty() && h.transit.buses.is_empty());
        arm_mut(&mut s, n).bus_lane = true;
        let so = at(&s, 180);
        s.signal_plan = Some(SignalPlan { phases: vec![StoredPhase { moves: vec![Move { from: so, to: n }], walks: vec![], rides: vec![], arm: None }] });
        let plan = signal::plan(&s, 0).unwrap();
        assert_eq!(control::hour(&s, &plan, &[], &[]).transit.findings, vec![Finding::NoPhase { arm: n }]);
    }

    #[test]
    fn with_no_bus_the_hour_is_as_with_no_priority() {
        let (j, n) = with_bus_lane();
        let mut s = j.current().clone();
        let e = at(&s, 90);
        arm_mut(&mut s, e).calls = Some(12);
        arm_mut(&mut s, n).cyclists = Some(30);
        let plan = signal::plan(&s, 0).unwrap();
        let (distances, lengths) = ([(e, 9_000)], [(n, 26_000)]);
        let before = control::hour(&s, &plan, &distances, &lengths);
        assert_eq!(before.transit, Summary::default());
        arm_mut(&mut s, n).transit_priority = Some(Priority { mode: Mode::Both, buses: 0 });
        let after = control::hour(&s, &plan, &distances, &lengths);
        assert_eq!((&after.calls, &after.actuation, &after.bicycles), (&before.calls, &before.actuation, &before.bicycles));
        assert_eq!(after.transit.totals.len(), 1);
        assert_eq!(after.transit.totals[0].buses, 0);
    }

    #[test]
    fn fixed_inputs_give_a_fixed_summary() {
        let (j, n) = with_bus_lane();
        let mut s = j.current().clone();
        let (e, so, w) = (at(&s, 90), at(&s, 180), at(&s, 270));
        // North's buses given both, 30 an hour; east's crossing on call.
        arm_mut(&mut s, n).transit_priority = Some(Priority { mode: Mode::Both, buses: 30 });
        arm_mut(&mut s, e).calls = Some(6);
        let plan = signal::plan(&s, 0).unwrap();
        let distances = [(n, 9_000), (e, 9_000), (so, 9_000), (w, 9_000)];
        let lengths = [(n, 26_000), (e, 26_000), (so, 26_000), (w, 26_000)];
        let h = control::hour(&s, &plan, &distances, &lengths);
        assert_eq!(h, control::hour(&s, &plan, &distances, &lengths), "deterministic");
        assert_eq!(h.transit.buses.len(), 30);
        let t = h.transit.totals[0];
        assert_eq!((t.arm, t.mode, t.rate, t.buses), (n, Mode::Both, 30, 30));
        assert_eq!(t.on_arrival + t.early_green + t.extended + t.delayed + t.not_served, 30);
        assert_eq!(h.transit.totals, vec![PINNED_TOTALS(n)], "{:#?}", h.transit.totals);
        // Each phase it cuts walks a 9 m crossing on recall (7 s + 9 s = 16 s), so a 10 s cut of a 25 s green is kept to 9 s.
        assert_eq!(
            h.transit.findings,
            vec![
                Finding::ClearanceHeld { arm: n, phase: w, times: 3, held_s: 3 },
                Finding::ClearanceHeld { arm: n, phase: so, times: 1, held_s: 1 },
                Finding::ClearanceHeld { arm: n, phase: e, times: 3, held_s: 3 },
            ],
            "{:#?}",
            h.transit.findings
        );
        assert_eq!(
            &h.transit.buses[..4],
            &[
                bus(n, 87, Outcome::EarlyGreen, 24, 9, 0),
                bus(n, 238, Outcome::OnArrival, 0, 0, 0),
                bus(n, 301, Outcome::EarlyGreen, 40, 10, 0),
                bus(n, 397, Outcome::Delayed, 64, 0, 0), // the cycle after one given priority
            ]
        );
        assert_eq!(h.transit.moved, vec![Moved { from: n, to: w, seconds: 49 }, Moved { from: n, to: so, seconds: 48 }, Moved { from: n, to: e, seconds: 44 }]);
    }

    #[allow(non_snake_case)]
    fn PINNED_TOTALS(n: u32) -> Totals {
        Totals {
            arm: n,
            mode: Mode::Both,
            rate: 30,
            buses: 30,
            on_arrival: 7,
            early_green: 13,
            extended: 2,
            delayed: 8,
            not_served: 0,
            longest_wait_s: Some(74),
            moved_s: 141,
        }
    }
}
