//! The cycle of a traffic signal and what runs with each phase. Each street that has entering lanes is a phase of
//! its own, taken in turn in the order of the streets' bearings. A pedestrian crossing or a bike movement runs in every
//! phase where it does not conflict with that phase's vehicle movements. No DOM, no signals, no I/O.

use serde::{Deserialize, Serialize};

use crate::junction::model::{Arm, LEFT, SIGNAL, State, THROUGH, Timing, turn_class};

/// One vehicle movement a phase serves: the traffic entering from arm `from`, going to arm `to`. It is the movement,
/// not a lane, that a phase serves: a lane's number changes when the street's lanes are made again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    pub from: u32,
    pub to: u32,
}

/// A phase as it is kept: the movements it serves and the crossings (`walks`) and bike movements (`rides`) that run
/// with it, all by street uid. Its timing is not kept here: it is the timing of its primary arm, the arm of its first
/// movement (`Arm.timing`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPhase {
    pub moves: Vec<Move>,
    pub walks: Vec<u32>,
    pub rides: Vec<u32>,
    /// The street whose timing a phase with no movement takes (a pedestrian scramble); written only then. A phase
    /// that serves movements takes its timing from the street of its first movement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arm: Option<u32>,
}

/// A signal plan the person has edited: its phases in the order they run. A junction that has none shows the
/// suggestion (`plan`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalPlan {
    pub phases: Vec<StoredPhase>,
}

/// One street's turn of the signal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phase {
    pub uid: u32,
    pub timing: Timing,
    pub length_s: u32,
    /// The streets whose crossing is walked during this phase, in the order of the phases.
    pub walks: Vec<u32>,
    /// The streets whose bike movements run during this phase (this street's own among them).
    pub rides: Vec<u32>,
    /// The movements the phase serves.
    pub moves: Vec<Move>,
}

impl Phase {
    /// The streets the phase sends traffic from, each once, in the order the moves first name them.
    pub fn sources(&self) -> Vec<u32> {
        let mut from: Vec<u32> = Vec::new();
        for m in &self.moves {
            if !from.contains(&m.from) {
                from.push(m.from);
            }
        }
        from
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub phases: Vec<Phase>,
    pub cycle_s: u32,
    /// The plan is the suggestion: the person has not edited it.
    pub suggested: bool,
}

impl Plan {
    /// The plan as it is kept.
    pub fn to_stored(&self) -> SignalPlan {
        SignalPlan {
            phases: self
                .phases
                .iter()
                .map(|p| StoredPhase { moves: p.moves.clone(), walks: p.walks.clone(), rides: p.rides.clone(), arm: p.moves.is_empty().then_some(p.uid) })
                .collect(),
        }
    }
}

/// Where the traffic entering arm `i` goes, as places in `arms`.
fn destinations(arms: &[&Arm], i: usize) -> Vec<usize> {
    let mut to: Vec<usize> = arms[i].lanes.iter().flat_map(|l| l.to.iter()).filter_map(|u| arms.iter().position(|a| a.uid == *u)).filter(|&x| x != i).collect();
    to.sort_unstable();
    to.dedup();
    to
}

/// Whether movement `a -> x` crosses movement `p -> y`: four different streets, their places round the junction
/// interleaved. Sharing a street (leaving it, entering it) is not crossing.
fn crosses(a: usize, x: usize, p: usize, y: usize) -> bool {
    if a == p || a == y || x == p || x == y {
        return false;
    }
    let (lo, hi) = (a.min(x), a.max(x));
    let inside = |v: usize| lo < v && v < hi;
    inside(p) != inside(y)
}

/// Whether the crossing of arm `b` is free while arm `i`'s traffic goes: nothing enters from `b`, and nothing goes
/// straight on into `b`. A turn into `b` gives way to people crossing, as it does at any signal, so it does not block
/// the crossing. This is the one rule about pedestrians; change it here.
fn crossing_clear(arms: &[&Arm], moves: &[Vec<usize>], i: usize, b: usize) -> bool {
    i != b && !(moves[i].contains(&b) && turn_class(arms[i].bearing, arms[b].bearing) == THROUGH)
}

/// The plan the person sees: the one they edited, else the suggestion. None unless the control is a signal.
pub fn plan(s: &State, region: usize) -> Option<Plan> {
    let suggestion = suggestion(s, region)?;
    let Some(stored) = s.signal_plan.as_ref().and_then(|p| settled(s, p)) else { return Some(suggestion) };
    let phases: Vec<Phase> = stored
        .phases
        .into_iter()
        .filter_map(|p| {
            let uid = p.moves.first().map(|m| m.from).or(p.arm)?;
            let timing = s.arms.iter().find(|a| a.uid == uid)?.timing.unwrap_or_default();
            // A scramble walks every crossing there is, as the junction has them now.
            let walks = if p.moves.is_empty() { scramble_walks(s) } else { p.walks };
            Some(Phase { uid, timing, length_s: timing.length_s(), walks, rides: p.rides, moves: p.moves })
        })
        .collect();
    let cycle_s = phases.iter().map(|p| p.length_s).sum();
    Some(Plan { phases, cycle_s, suggested: false })
}

/// The stored plan with whatever no longer exists taken out (a street, a lane, a destination), and a phase that serves
/// nothing any more dropped. None when nothing is left.
fn settled(s: &State, plan: &SignalPlan) -> Option<SignalPlan> {
    let has = |uid: u32| s.arms.iter().any(|a| a.uid == uid);
    let phases: Vec<StoredPhase> = plan
        .phases
        .iter()
        .map(|p| StoredPhase {
            moves: p
                .moves
                .iter()
                .copied()
                .filter(|m| has(m.to) && s.arms.iter().any(|a| a.uid == m.from && a.lanes.iter().any(|l| l.to.contains(&m.to))))
                .collect(),
            walks: p.walks.iter().copied().filter(|&u| has(u)).collect(),
            rides: p.rides.iter().copied().filter(|&u| has(u)).collect(),
            arm: p.arm,
        })
        .filter(|p| !p.moves.is_empty() || p.arm.is_some_and(has))
        .collect();
    (!phases.is_empty()).then_some(SignalPlan { phases })
}

/// Keeps the stored plan true to the junction after an edit: gone unless the control is a signal, and without what no
/// longer exists.
pub fn settle(s: &mut State) {
    s.signal_plan = if s.control == SIGNAL { s.signal_plan.as_ref().and_then(|p| settled(s, p)) } else { None };
}

/// A phase for arm `uid` as the suggestion would make it, to add to a stored plan. None when nothing enters from it.
pub fn phase_for(s: &State, region: usize, uid: u32) -> Option<StoredPhase> {
    let suggestion = suggestion(s, region)?;
    let p = suggestion.phases.into_iter().find(|p| p.uid == uid)?;
    (!p.moves.is_empty()).then_some(StoredPhase { moves: p.moves, walks: p.walks, rides: p.rides, arm: None })
}

/// The movements of the traffic entering `arms[i]`, in the order of its lanes.
fn movements(arms: &[&Arm], i: usize) -> Vec<Move> {
    let from = arms[i].uid;
    let mut moves: Vec<Move> = Vec::new();
    for m in arms[i].lanes.iter().flat_map(|l| l.to.iter().map(move |&to| Move { from, to })) {
        if m.to != m.from && arms.iter().any(|a| a.uid == m.to) && !moves.contains(&m) {
            moves.push(m);
        }
    }
    moves
}

/// The streets' arms in the order of the bearings, and where the traffic entering each goes (places in that order).
fn ordered(s: &State) -> (Vec<&Arm>, Vec<Vec<usize>>) {
    let mut arms: Vec<&Arm> = s.arms.iter().collect();
    arms.sort_by_key(|a| a.bearing);
    let moves = (0..arms.len()).map(|i| destinations(&arms, i)).collect();
    (arms, moves)
}

/// The crossings walked and the bike movements run while arm `i` sends `going` (places in `arms`) and nothing else:
/// the one place the pedestrian and bike rules meet a phase.
fn walks_and_rides(arms: &[&Arm], moves: &[Vec<usize>], bikes: &[bool], i: usize, going: &[usize]) -> (Vec<u32>, Vec<u32>) {
    let mut moves = moves.to_vec();
    moves[i] = going.to_vec();
    let walks = (0..arms.len()).filter(|&b| arms[b].crossing.is_some() && crossing_clear(arms, &moves, i, b)).map(|b| arms[b].uid).collect();
    let rides = (0..arms.len())
        .filter(|&b| bikes[b] && (b == i || moves[b].iter().all(|&x| moves[i].iter().all(|&y| !crosses(b, x, i, y)))))
        .map(|b| arms[b].uid)
        .collect();
    (walks, rides)
}

fn bikes_of(arms: &[&Arm], region: usize) -> Vec<bool> {
    arms.iter().map(|a| !a.profile(region).outer_bike().is_empty()).collect()
}

/// The suggestion: one phase per street that has entering lanes, in the order of the bearings.
fn suggestion(s: &State, region: usize) -> Option<Plan> {
    if s.control != SIGNAL {
        return None;
    }
    let (arms, moves) = ordered(s);
    let bikes = bikes_of(&arms, region);
    let phases: Vec<Phase> = (0..arms.len())
        .filter(|&i| !arms[i].lanes.is_empty())
        .map(|i| {
            let timing = arms[i].timing.unwrap_or_default();
            let (walks, rides) = walks_and_rides(&arms, &moves, &bikes, i, &moves[i]);
            Phase { uid: arms[i].uid, timing, length_s: timing.length_s(), walks, rides, moves: movements(&arms, i) }
        })
        .collect();
    let cycle_s = phases.iter().map(|p| p.length_s).sum();
    Some(Plan { phases, cycle_s, suggested: true })
}

/// A ready-made choice of what a phase allows. The figures behind them (which turns, whom they give way to) are
/// UNVERIFIED placeholders, not a standard's rules: only the rules in this file about who crosses whom are used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Pattern {
    /// Only the traffic going straight on.
    ThroughOnly,
    /// Every movement of the street, turns giving way to the people crossing.
    All,
    /// The left turns first, with no one crossing where they turn, then the straight on and right movements.
    ProtectedLeftThenThrough,
    /// No vehicle moves; every crossing is walked.
    Scramble,
    /// Whatever matches none of the above. It is shown, never chosen.
    Custom,
}

impl Pattern {
    /// The patterns a person can choose, in the order they are offered.
    pub const CHOICES: [Pattern; 4] = [Pattern::ThroughOnly, Pattern::All, Pattern::ProtectedLeftThenThrough, Pattern::Scramble];

    pub fn name_key(self) -> &'static str {
        match self {
            Pattern::ThroughOnly => "jn-pattern-through",
            Pattern::All => "jn-pattern-all",
            Pattern::ProtectedLeftThenThrough => "jn-pattern-protected-left",
            Pattern::Scramble => "jn-pattern-scramble",
            Pattern::Custom => "jn-pattern-custom",
        }
    }

    pub fn description_key(self) -> &'static str {
        match self {
            Pattern::ThroughOnly => "jn-pattern-through-desc",
            Pattern::All => "jn-pattern-all-desc",
            Pattern::ProtectedLeftThenThrough => "jn-pattern-protected-left-desc",
            Pattern::Scramble => "jn-pattern-scramble-desc",
            Pattern::Custom => "jn-pattern-custom-desc",
        }
    }
}

/// The phase for street `uid` as `pattern` says it, to put in a stored plan. None when the pattern does not fit
/// this junction (nothing for it to serve) or is Custom.
pub fn pattern_phase(s: &State, region: usize, uid: u32, pattern: Pattern) -> Option<StoredPhase> {
    if s.control != SIGNAL {
        return None;
    }
    let (arms, moves) = ordered(s);
    let bikes = bikes_of(&arms, region);
    let i = arms.iter().position(|a| a.uid == uid)?;
    if pattern == Pattern::Scramble {
        return arms.iter().any(|a| a.crossing.is_some()).then(|| StoredPhase {
            moves: Vec::new(),
            walks: scramble_walks(s),
            rides: Vec::new(),
            arm: Some(uid),
        });
    }
    let class_of = |m: &Move| arms.iter().find(|a| a.uid == m.to).map_or(0, |to| turn_class(arms[i].bearing, to.bearing));
    let all = movements(&arms, i);
    let chosen: Vec<Move> = match pattern {
        Pattern::ThroughOnly => all.iter().filter(|m| class_of(m) == THROUGH).copied().collect(),
        Pattern::All => all,
        Pattern::ProtectedLeftThenThrough => {
            let (left, rest): (Vec<Move>, Vec<Move>) = all.into_iter().partition(|m| class_of(m) == LEFT);
            if left.is_empty() {
                return None;
            }
            left.into_iter().chain(rest).collect()
        }
        Pattern::Scramble | Pattern::Custom => return None,
    };
    if chosen.is_empty() {
        return None;
    }
    let going: Vec<usize> = chosen.iter().filter_map(|m| arms.iter().position(|a| a.uid == m.to)).collect();
    let (mut walks, rides) = walks_and_rides(&arms, &moves, &bikes, i, &going);
    if pattern == Pattern::ProtectedLeftThenThrough {
        // Protected: the crossing of a street a left turn goes into is not walked, whoever gives way elsewhere.
        let lefts: Vec<u32> = chosen.iter().filter(|m| class_of(m) == LEFT).map(|m| m.to).collect();
        walks.retain(|u| !lefts.contains(u));
    }
    Some(StoredPhase { moves: chosen, walks, rides, arm: None })
}

/// Every street with a crossing, in the order of the bearings: who walks in a scramble.
fn scramble_walks(s: &State) -> Vec<u32> {
    let (arms, _) = ordered(s);
    arms.iter().filter(|a| a.crossing.is_some()).map(|a| a.uid).collect()
}

/// The patterns that fit the phase of street `phase.uid`, in the order they are offered. A pattern that would make
/// the same phase as one before it is left out, so the list never holds two names for one thing.
pub fn patterns_for(s: &State, region: usize, phase: &Phase) -> Vec<Pattern> {
    let mut seen: Vec<StoredPhase> = Vec::new();
    Pattern::CHOICES
        .into_iter()
        .filter(|&p| {
            let Some(made) = pattern_phase(s, region, phase.uid, p) else { return false };
            let new = !seen.contains(&made);
            seen.push(made);
            new
        })
        .collect()
}

/// Which pattern a phase is: the first that makes exactly this phase, else Custom.
pub fn pattern_of(s: &State, region: usize, phase: &Phase) -> Pattern {
    Pattern::CHOICES
        .into_iter()
        .find(|&p| {
            pattern_phase(s, region, phase.uid, p).is_some_and(|made| made.moves == phase.moves && made.walks == phase.walks && made.rides == phase.rides)
        })
        .unwrap_or(Pattern::Custom)
}

/// The longest time in a row, round the cycle, that the crossing of arm `uid` is walked.
pub fn walk_window_s(plan: &Plan, uid: u32) -> u32 {
    let runs: Vec<Option<u32>> = plan.phases.iter().map(|p| p.walks.contains(&uid).then_some(p.length_s)).collect();
    if runs.iter().all(Option::is_some) {
        return runs.iter().flatten().sum();
    }
    // Going round twice finds a run that wraps past the end; there is a break somewhere, so no run is counted twice.
    let (mut best, mut run) = (0, 0);
    for k in 0..2 * runs.len() {
        match runs[k % runs.len()] {
            Some(s) => {
                run += s;
                best = best.max(run);
            }
            None => run = 0,
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{Junction, Lane, PRIORITY, junction_of};

    /// The avenue-and-street four-way on a signal: streets at 0, 90, 180 and 270 degrees.
    fn cross() -> State {
        Junction::new(0).current().clone()
    }

    fn at(s: &State, bearing: i32) -> u32 {
        s.arms.iter().find(|a| a.bearing == bearing).unwrap().uid
    }

    /// Makes the traffic entering `from` go to `to`, in one lane.
    fn send(s: &mut State, from: u32, to: &[u32]) {
        s.arms.iter_mut().find(|a| a.uid == from).unwrap().lanes = vec![Lane { to: to.to_vec() }];
    }

    /// Everything goes straight on: north to south, east to west, and back.
    fn through_only() -> State {
        let mut s = cross();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        send(&mut s, n, &[so]);
        send(&mut s, so, &[n]);
        send(&mut s, e, &[w]);
        send(&mut s, w, &[e]);
        s
    }

    fn phase(p: &Plan, uid: u32) -> &Phase {
        p.phases.iter().find(|p| p.uid == uid).unwrap()
    }

    #[test]
    fn the_streets_a_phase_sends_traffic_from_are_listed_once_in_the_order_first_seen() {
        let mut p = plan(&cross(), 0).unwrap().phases[0].clone();
        let (a, b, c) = (p.uid, at(&cross(), 90), at(&cross(), 180));
        p.moves = vec![Move { from: b, to: a }, Move { from: c, to: a }, Move { from: b, to: c }];
        assert_eq!(p.sources(), vec![b, c]);
        p.moves.clear();
        assert!(p.sources().is_empty());
    }

    #[test]
    fn there_is_no_plan_unless_the_control_is_a_signal() {
        assert!(plan(Junction::new(1).current(), 0).is_none());
        assert!(plan(&cross(), 0).is_some());
    }

    #[test]
    fn the_phases_take_turns_in_the_order_of_the_bearings_and_the_cycle_is_their_sum() {
        let mut s = through_only();
        let n = at(&s, 0);
        s.arms.iter_mut().find(|a| a.uid == n).unwrap().timing = Some(Timing { green_s: 30, yellow_s: 3, all_red_s: 2, transit_lead_s: 5 });
        // Out of order in the state: the plan sorts by bearing.
        s.arms.reverse();
        let p = plan(&s, 0).unwrap();
        assert_eq!(p.phases.iter().map(|p| p.uid).collect::<Vec<_>>(), vec![n, at(&s, 90), at(&s, 180), at(&s, 270)]);
        assert_eq!(phase(&p, n).length_s, 40, "the early green lengthens its own phase");
        assert_eq!(p.cycle_s, 40 + 3 * 30);
    }

    #[test]
    fn a_street_nothing_enters_from_has_no_phase_but_its_crossing_is_still_walked() {
        let mut s = through_only();
        let (n, e, w) = (at(&s, 0), at(&s, 90), at(&s, 270));
        send(&mut s, w, &[n]); // the west street no longer touches the east one
        s.arms.iter_mut().find(|a| a.uid == e).unwrap().lanes.clear();
        let p = plan(&s, 0).unwrap();
        assert_eq!(p.phases.len(), 3);
        assert!(p.phases.iter().all(|p| p.uid != e));
        assert_eq!(p.cycle_s, 90);
        assert_eq!(walk_window_s(&p, e), 90, "walked in every phase: the whole cycle");
    }

    #[test]
    fn a_crossing_is_walked_while_the_traffic_beside_it_goes_straight_on_but_not_while_traffic_goes_straight_into_it() {
        let s = through_only();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        let p = plan(&s, 0).unwrap();
        assert_eq!(phase(&p, n).walks, vec![e, w], "north goes straight into south: south's crossing is not free, east's and west's are");
        assert!(!phase(&p, n).walks.contains(&n) && !phase(&p, n).walks.contains(&so));
    }

    #[test]
    fn a_turn_into_a_street_gives_way_to_its_crossing() {
        let mut s = through_only();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        send(&mut s, n, &[so, e, w]); // straight on, and both turns
        let p = plan(&s, 0).unwrap();
        assert_eq!(phase(&p, n).walks, vec![e, w], "turning drivers give way: east's and west's crossings are still walked");
    }

    #[test]
    fn a_bike_movement_runs_unless_it_crosses_the_phases_traffic() {
        let mut j = junction_of(&[("sidewalk", 2000), ("bike", 1500), ("travel", 3200), ("travel", 3200), ("bike", 1500), ("sidewalk", 2000)]);
        let mut s = j.current().clone();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        send(&mut s, n, &[so]);
        send(&mut s, so, &[n]);
        send(&mut s, e, &[w]);
        send(&mut s, w, &[e]);
        let p = plan(&s, 0).unwrap();
        // North's phase: south's bikes go the other way along the same line, east's and west's cross it.
        assert_eq!(phase(&p, n).rides, vec![n, so]);
        assert_eq!(phase(&p, e).rides, vec![e, w]);
        // Turning left from north crosses south's straight bikes' path? It shares no street with them: it does not.
        send(&mut s, n, &[so, w]);
        let p = plan(&s, 0).unwrap();
        assert_eq!(phase(&p, n).rides, vec![n, so]);
        j.set_control(PRIORITY);
        assert!(plan(j.current(), 0).is_none());
    }

    #[test]
    fn at_a_t_junction_a_crossing_is_walked_in_the_phases_that_send_nothing_straight_into_it() {
        let mut j = Junction::new(1);
        assert!(j.set_control(crate::junction::model::SIGNAL));
        let mut s = j.current().clone();
        let (e, so, w) = (at(&s, 90), at(&s, 180), at(&s, 270));
        send(&mut s, e, &[so, w]);
        send(&mut s, so, &[e, w]);
        send(&mut s, w, &[e, so]);
        let p = plan(&s, 0).unwrap();
        // East's traffic goes straight into west: west's crossing is not free then; south's is (a turn gives way).
        assert_eq!(phase(&p, e).walks, vec![so]);
        assert_eq!(phase(&p, so).walks, vec![e, w], "south's traffic only turns");
        assert_eq!(phase(&p, w).walks, vec![so]);
        // The phases run east, south, west: south's crossing is walked in east's and west's, which are next to each
        // other going round; east's only in south's.
        assert_eq!(walk_window_s(&p, so), 60);
        assert_eq!(walk_window_s(&p, e), 30);
    }

    #[test]
    fn five_streets_make_five_phases() {
        let mut j = Junction::new(3); // five ways, all-way stop
        assert!(j.set_control(crate::junction::model::SIGNAL));
        let p = plan(j.current(), 0).unwrap();
        assert_eq!(p.phases.len(), 5);
        assert_eq!(p.cycle_s, 5 * 30);
    }

    #[test]
    fn the_walk_window_runs_on_across_the_end_of_the_cycle() {
        // North, east, south, west in turn. East's crossing is walked in north's, south's and west's phases: south, west
        // and north are in a row going round the cycle (30 s each), though north is first and south third.
        let mut s = through_only();
        let (n, e, w) = (at(&s, 0), at(&s, 90), at(&s, 270));
        send(&mut s, w, &[n]);
        let p = plan(&s, 0).unwrap();
        assert_eq!(walk_window_s(&p, e), 90);
    }

    #[test]
    fn a_window_is_the_longest_unbroken_run_not_the_total() {
        let s = through_only(); // east's crossing: free in north's and south's phases, which are not next to each other
        let p = plan(&s, 0).unwrap();
        assert_eq!(walk_window_s(&p, at(&s, 90)), 30);
    }

    // ---- ready-made patterns --------------------------------------------------------------------------------------

    /// The north street sends traffic every way: south (straight on), east (a left) and west (a right).
    fn north_every_way() -> State {
        let mut s = through_only();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        send(&mut s, n, &[e, so, w]);
        s
    }

    fn stored(s: &State, uid: u32, pattern: Pattern) -> StoredPhase {
        pattern_phase(s, 0, uid, pattern).unwrap()
    }

    #[test]
    fn through_only_serves_the_streets_straight_on_movements_and_nothing_else() {
        let s = north_every_way();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        let p = stored(&s, n, Pattern::ThroughOnly);
        assert_eq!(p.moves, vec![Move { from: n, to: so }]);
        assert_eq!(p.walks, vec![e, w], "nobody turns, so the crossings beside it are walked");
        assert!(p.arm.is_none());
    }

    #[test]
    fn a_protected_left_goes_first_and_no_one_crosses_where_it_turns() {
        let s = north_every_way();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        let p = stored(&s, n, Pattern::ProtectedLeftThenThrough);
        assert_eq!(p.moves, vec![Move { from: n, to: e }, Move { from: n, to: so }, Move { from: n, to: w }], "the left first, then the rest");
        assert_eq!(p.walks, vec![w], "east's crossing is where the left turns into: not walked, nor south's where traffic goes straight on");
        let all = stored(&s, n, Pattern::All);
        assert_eq!(all.walks, vec![e, w], "unprotected: the left gives way to the people crossing");
    }

    #[test]
    fn a_pedestrian_scramble_serves_no_vehicle_and_every_crossing_walks() {
        let s = north_every_way();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        let p = stored(&s, n, Pattern::Scramble);
        assert!(p.moves.is_empty() && p.rides.is_empty());
        assert_eq!(p.arm, Some(n));
        let mut shown = s.clone();
        shown.signal_plan = Some(SignalPlan { phases: vec![p] });
        let plan = plan(&shown, 0).unwrap();
        assert_eq!(plan.phases.len(), 1, "a phase with no movement is kept");
        assert_eq!((plan.phases[0].uid, plan.phases[0].walks.clone()), (n, vec![n, e, so, w]));
        assert!(plan.phases[0].sources().is_empty());
    }

    #[test]
    fn a_scramble_takes_the_timing_of_its_street() {
        let mut s = north_every_way();
        let n = at(&s, 0);
        let t = Timing { green_s: 40, yellow_s: 3, all_red_s: 2, transit_lead_s: 0 };
        s.arms.iter_mut().find(|a| a.uid == n).unwrap().timing = Some(t);
        s.signal_plan = Some(SignalPlan { phases: vec![stored(&s, n, Pattern::Scramble)] });
        let p = plan(&s, 0).unwrap();
        assert_eq!((p.phases[0].timing, p.cycle_s), (t, t.length_s()));
        assert_eq!(p.to_stored().phases[0].arm, Some(n), "kept as a scramble when written back");
    }

    #[test]
    fn only_the_patterns_the_lanes_support_are_offered() {
        let s = through_only();
        let n = at(&s, 0);
        let p = plan(&s, 0).unwrap();
        let offered = patterns_for(&s, 0, phase(&p, n));
        assert_eq!(offered, vec![Pattern::ThroughOnly, Pattern::Scramble], "no left to protect, and all movements is the through movement");
        assert!(pattern_phase(&s, 0, n, Pattern::ProtectedLeftThenThrough).is_none());
        let s = north_every_way();
        let p = plan(&s, 0).unwrap();
        let offered = patterns_for(&s, 0, phase(&p, at(&s, 0)));
        assert_eq!(offered, vec![Pattern::ThroughOnly, Pattern::All, Pattern::ProtectedLeftThenThrough, Pattern::Scramble]);
    }

    #[test]
    fn a_phase_is_the_pattern_it_matches_and_custom_when_it_matches_none() {
        let s = north_every_way();
        let n = at(&s, 0);
        let p = plan(&s, 0).unwrap();
        assert_eq!(pattern_of(&s, 0, phase(&p, n)), Pattern::All, "the suggestion serves every movement");
        for pat in [Pattern::ThroughOnly, Pattern::ProtectedLeftThenThrough, Pattern::Scramble] {
            let mut t = s.clone();
            t.signal_plan = Some(SignalPlan { phases: vec![stored(&s, n, pat)] });
            let p = plan(&t, 0).unwrap();
            assert_eq!(pattern_of(&t, 0, &p.phases[0]), pat);
        }
        let mut custom = stored(&s, n, Pattern::All);
        custom.moves.pop();
        let mut t = s.clone();
        t.signal_plan = Some(SignalPlan { phases: vec![custom] });
        let p = plan(&t, 0).unwrap();
        assert_eq!(pattern_of(&t, 0, &p.phases[0]), Pattern::Custom);
        assert!(Pattern::Custom.name_key() != Pattern::All.name_key());
    }

    #[test]
    fn a_plan_kept_before_patterns_still_loads_and_a_scramble_is_the_only_phase_that_writes_the_marker() {
        let old = r#"{"phases":[{"moves":[{"from":1,"to":2}],"walks":[3],"rides":[]}]}"#;
        let plan: SignalPlan = serde_json::from_str(old).unwrap();
        assert_eq!(plan.phases[0].arm, None);
        assert_eq!(serde_json::to_string(&plan).unwrap(), old, "nothing new is written for a phase with movements");
        let scramble = StoredPhase { arm: Some(4), ..Default::default() };
        assert!(serde_json::to_string(&scramble).unwrap().contains(r#""arm":4"#));
    }
}
