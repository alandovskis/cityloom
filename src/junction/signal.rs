//! The cycle of a traffic signal and what runs with each phase. Each street that has entering lanes is a phase of
//! its own, taken in turn in the order of the streets' bearings. A pedestrian crossing or a bike movement runs in every
//! phase where it does not conflict with that phase's vehicle movements. No DOM, no signals, no I/O.

use serde::{Deserialize, Serialize};

use crate::junction::model::{Arm, SIGNAL, State, THROUGH, Timing, turn_class};

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
        SignalPlan { phases: self.phases.iter().map(|p| StoredPhase { moves: p.moves.clone(), walks: p.walks.clone(), rides: p.rides.clone() }).collect() }
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
            let uid = p.moves.first()?.from;
            let timing = s.arms.iter().find(|a| a.uid == uid)?.timing.unwrap_or_default();
            Some(Phase { uid, timing, length_s: timing.length_s(), walks: p.walks, rides: p.rides, moves: p.moves })
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
        })
        .filter(|p| !p.moves.is_empty())
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
    (!p.moves.is_empty()).then_some(StoredPhase { moves: p.moves, walks: p.walks, rides: p.rides })
}

/// The suggestion: one phase per street that has entering lanes, in the order of the bearings.
fn suggestion(s: &State, region: usize) -> Option<Plan> {
    if s.control != SIGNAL {
        return None;
    }
    let mut arms: Vec<&Arm> = s.arms.iter().collect();
    arms.sort_by_key(|a| a.bearing);
    let moves: Vec<Vec<usize>> = (0..arms.len()).map(|i| destinations(&arms, i)).collect();
    let bikes: Vec<bool> = arms.iter().map(|a| !a.profile(region).outer_bike().is_empty()).collect();
    let phases: Vec<Phase> = (0..arms.len())
        .filter(|&i| !arms[i].lanes.is_empty())
        .map(|i| {
            let timing = arms[i].timing.unwrap_or_default();
            let walks = (0..arms.len()).filter(|&b| arms[b].crossing.is_some() && crossing_clear(&arms, &moves, i, b)).map(|b| arms[b].uid).collect();
            let rides = (0..arms.len())
                .filter(|&b| bikes[b] && (b == i || moves[b].iter().all(|&x| moves[i].iter().all(|&y| !crosses(b, x, i, y)))))
                .map(|b| arms[b].uid)
                .collect();
            let from = arms[i].uid;
            let mut moves: Vec<Move> = Vec::new();
            for m in arms[i].lanes.iter().flat_map(|l| l.to.iter().map(move |&to| Move { from, to })) {
                if m.to != m.from && arms.iter().any(|a| a.uid == m.to) && !moves.contains(&m) {
                    moves.push(m);
                }
            }
            Phase { uid: arms[i].uid, timing, length_s: timing.length_s(), walks, rides, moves }
        })
        .collect();
    let cycle_s = phases.iter().map(|p| p.length_s).sum();
    Some(Plan { phases, cycle_s, suggested: true })
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
}
