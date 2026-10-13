//! The cycle of a traffic signal and what runs with each phase. Each street that has entering lanes is a phase of
//! its own, taken in turn in the order of the streets' bearings. A pedestrian crossing or a bike movement runs in every
//! phase where it does not conflict with that phase's vehicle movements. No DOM, no signals, no I/O.

use crate::junction::model::{Arm, SIGNAL, State, Timing};

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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub phases: Vec<Phase>,
    pub cycle_s: u32,
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

/// Whether the crossing of arm `b` is free while arm `i`'s traffic goes: nothing of it enters from `b` or leaves
/// into `b`. This is the one rule about pedestrians; change it here.
fn crossing_clear(moves: &[Vec<usize>], i: usize, b: usize) -> bool {
    i != b && !moves[i].contains(&b)
}

pub fn plan(s: &State, region: usize) -> Option<Plan> {
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
            let walks = (0..arms.len()).filter(|&b| arms[b].crossing.is_some() && crossing_clear(&moves, i, b)).map(|b| arms[b].uid).collect();
            let rides = (0..arms.len())
                .filter(|&b| bikes[b] && (b == i || moves[b].iter().all(|&x| moves[i].iter().all(|&y| !crosses(b, x, i, y)))))
                .map(|b| arms[b].uid)
                .collect();
            Phase { uid: arms[i].uid, timing, length_s: timing.length_s(), walks, rides }
        })
        .collect();
    let cycle_s = phases.iter().map(|p| p.length_s).sum();
    Some(Plan { phases, cycle_s })
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
    fn a_crossing_is_walked_while_the_traffic_beside_it_goes_straight_and_not_when_it_turns_into_it() {
        let mut s = through_only();
        let (n, e, so, w) = (at(&s, 0), at(&s, 90), at(&s, 180), at(&s, 270));
        let p = plan(&s, 0).unwrap();
        assert_eq!(phase(&p, n).walks, vec![e, w], "north goes to south: east's and west's crossings are free");
        send(&mut s, n, &[so, e]);
        let p = plan(&s, 0).unwrap();
        assert_eq!(phase(&p, n).walks, vec![w], "north now turns into east");
        assert!(!phase(&p, n).walks.contains(&n), "a street's own crossing is never free in its own phase");
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
    fn a_t_junction_whose_every_street_turns_into_the_others_walks_nowhere_and_has_no_window() {
        let mut j = Junction::new(1);
        assert!(j.set_control(crate::junction::model::SIGNAL));
        let mut s = j.current().clone();
        let (e, so, w) = (at(&s, 90), at(&s, 180), at(&s, 270));
        send(&mut s, e, &[so, w]);
        send(&mut s, so, &[e, w]);
        send(&mut s, w, &[e, so]);
        let p = plan(&s, 0).unwrap();
        assert!(p.phases.iter().all(|p| p.walks.is_empty()));
        assert!([e, so, w].iter().all(|&u| walk_window_s(&p, u) == 0));
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
