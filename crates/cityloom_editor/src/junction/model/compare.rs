//! What differs between two designs of one junction: the control, the streets' lanes, crossings and bike boxes, the
//! signal's plan, the checks that pass or fail on one side only, and the other things that set the two apart (the
//! ring, the raised table, the bus lane, the corners).
//!
//! Pure: no wording but `Said`. The designs are the junction's own views (`JView`); nothing is read from the page.

use super::read_model::{ArmView, Check, JView};
use super::{Arm, State, control_key};
use crate::shared::said::{Arg, Said};

/// One street of the junction that is not the same in the two designs; the street is the same in both by its `uid`.
/// A street only one design has counts as having no lanes, no crossing and no bike box in the other.
#[derive(Clone, Debug, PartialEq)]
pub struct ArmChange {
    pub uid: u32,
    /// The street's name, as the junction says it.
    pub street: Said,
    /// Lanes entering the junction, as the inspector counts them (`ArmView::lanes`), in A and in B.
    pub entering: (usize, usize),
    /// Lanes leaving it (`ArmView::leave_arrows`), in A and in B.
    pub leaving: (usize, usize),
    pub crossing: (bool, bool),
    pub bike_box: (bool, bool),
}

impl ArmChange {
    /// One line for each thing about the street that differs, each naming it.
    pub fn lines(&self) -> Vec<Said> {
        let street = || Arg::Said(Box::new(self.street.clone()));
        let counts = |key: &'static str, (a, b): (usize, usize)| {
            (a != b).then(|| Said::new(key).with("street", street()).with("a", Arg::Num(a as i64)).with("b", Arg::Num(b as i64)))
        };
        let toggle = |added: &'static str, removed: &'static str, (a, b): (bool, bool)| {
            (a != b).then(|| Said::new(if b { added } else { removed }).with("street", street()))
        };
        [
            counts("cmp-jn-entering", self.entering),
            counts("cmp-jn-leaving", self.leaving),
            toggle("cmp-jn-crossing-added", "cmp-jn-crossing-removed", self.crossing),
            toggle("cmp-jn-bike-box-added", "cmp-jn-bike-box-removed", self.bike_box),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

/// What else can set two designs apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JOther {
    Ring,
    Raised,
    Bus,
    Corners,
    /// Something else about the streets at the junction as the design keeps them: where each lane may go, the turns
    /// banned, a crossing's width or island, the bulbs, the stop line, the bus lane, the approach, the street's pieces.
    ArmDetails,
}

impl JOther {
    /// The message that names it, which the comparison's frame owns.
    pub fn key(self) -> &'static str {
        match self {
            JOther::Ring => "cmp-other-ring",
            JOther::Raised => "cmp-other-raised",
            JOther::Bus => "cmp-other-bus",
            JOther::Corners => "cmp-other-corners",
            JOther::ArmDetails => "cmp-other-arm-details",
        }
    }
}

/// How the turbo layout of a junction is named: the layout and its size, or "none" without one.
fn turbo_name(v: &JView) -> Arg {
    match &v.turbo {
        Some(t) => Arg::Said(Box::new(Said::new(t.label_key).with("size", Arg::MsgLower(t.size_key)))),
        None => Arg::Msg("cmp-jn-turbo-none"),
    }
}

/// A changed control word, and a changed turbo layout (or size). A signal that appears or goes is the control's.
pub fn control(a: &JView, b: &JView) -> Vec<Said> {
    let mut lines = Vec::new();
    if a.control_index != b.control_index {
        lines.push(Said::new("cmp-jn-control").with("a", Arg::Msg(control_key(a.control_index))).with("b", Arg::Msg(control_key(b.control_index))));
    }
    if a.turbo.as_ref().map(|t| (t.layout, t.size)) != b.turbo.as_ref().map(|t| (t.layout, t.size)) {
        lines.push(Said::new("cmp-jn-turbo").with("a", turbo_name(a)).with("b", turbo_name(b)));
    }
    lines
}

/// The streets that are not the same in the two designs, in A's order and then B's own.
pub fn arms(a: &JView, b: &JView) -> Vec<ArmChange> {
    let facts = |x: Option<&ArmView>| {
        (
            x.map_or(0, |x| x.lanes.len()),
            x.map_or(0, |x| x.leave_arrows.len()),
            x.is_some_and(|x| x.crossing.is_some()),
            x.is_some_and(|x| x.bike_box.is_some()),
        )
    };
    let uids = a.arms.iter().map(|x| x.uid).chain(b.arms.iter().map(|x| x.uid).filter(|u| !a.arms.iter().any(|x| x.uid == *u)));
    uids.filter_map(|uid| {
        let (was, now) = (a.arms.iter().find(|x| x.uid == uid), b.arms.iter().find(|x| x.uid == uid));
        let street = now.or(was)?.street.clone();
        let (fa, fb) = (facts(was), facts(now));
        (fa != fb).then(|| ArmChange { uid, street, entering: (fa.0, fb.0), leaving: (fa.1, fb.1), crossing: (fa.2, fb.2), bike_box: (fa.3, fb.3) })
    })
    .collect()
}

/// The signal's plan, when both designs have a signal: how many phases and how long the cycle is.
pub fn signal(a: &JView, b: &JView) -> Vec<Said> {
    let (Some(was), Some(now)) = (&a.signal, &b.signal) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    if was.phases.len() != now.phases.len() {
        lines.push(Said::new("cmp-jn-phases").with("a", Arg::Num(was.phases.len() as i64)).with("b", Arg::Num(now.phases.len() as i64)));
    }
    if was.cycle_s != now.cycle_s {
        lines.push(Said::new("cmp-jn-cycle").with("a", Arg::Num(was.cycle_s as i64)).with("b", Arg::Num(now.cycle_s as i64)));
    }
    lines
}

/// The checks that pass only in B and those that fail only in B, by id, as `street::vm::impact::diff` has them: a
/// check that passes now and did not before, or that is gone and was failing, passes; one that fails now and did not
/// before, or is new and fails, fails. A check that appears or goes already passing is neither.
pub fn checks(a: &JView, b: &JView) -> (Vec<Said>, Vec<Said>) {
    let find = |list: &'_ [Check], id| list.iter().find(|c| c.id == id).map(|c| c.ok);
    let (mut passes, mut fails) = (Vec::new(), Vec::new());
    for now in &b.checks {
        match find(&a.checks, now.id) {
            // Passed in A, fails in B.
            Some(was) if was && !now.ok => fails.push(now.label.clone()),
            // Failed in A, passes in B.
            Some(was) if !was && now.ok => passes.push(now.label.clone()),
            // Not asked for in A and failing in B.
            None if !now.ok => fails.push(now.label.clone()),
            _ => {}
        }
    }
    for was in a.checks.iter().filter(|c| !c.ok && find(&b.checks, c.id).is_none()) {
        passes.push(was.label.clone());
    }
    (passes, fails)
}

/// An arm as far as nothing else reports it: what `arms`, `signal`, `Corners` and the streets' lane counts already say
/// is cleared, as are the uids of the street's pieces, which say nothing of what the design holds.
fn arm_details(arm: &Arm, other: &Arm, counts_differ: bool, plan_differs: bool) -> Arm {
    let mut x = arm.clone();
    x.edge = 0;
    x.corner_mm = 0;
    x.bike_box = false;
    if x.crossing.is_none() != other.crossing.is_none() {
        x.crossing = None;
    }
    if x.lanes.len() != other.lanes.len() {
        x.lanes.clear();
    }
    if plan_differs {
        x.timing = None;
        x.calls = None;
    }
    if counts_differ {
        x.section = None;
    }
    if let Some(street) = &mut x.section {
        street.segments.iter_mut().for_each(|p| p.uid = 0);
        street.zones.iter_mut().flat_map(|z| z.segments.iter_mut().flatten()).for_each(|p| p.uid = 0);
    }
    x
}

/// Whether any street of the junction, matched by uid, differs in what its design keeps beyond what `arms`,
/// `signal`, `Corners` and the control say. A street only one design has counts.
fn arm_details_differ(sa: &State, sb: &State, a: &JView, b: &JView) -> bool {
    let plan_differs = !signal(a, b).is_empty();
    let mut uids_a: Vec<u32> = sa.arms.iter().map(|x| x.uid).collect();
    let mut uids_b: Vec<u32> = sb.arms.iter().map(|x| x.uid).collect();
    uids_a.sort_unstable();
    uids_b.sort_unstable();
    if uids_a != uids_b {
        return true;
    }
    sa.arms.iter().any(|x| {
        let Some(y) = sb.arms.iter().find(|y| y.uid == x.uid) else {
            return true;
        };
        let counts = |v: &JView| v.arms.iter().find(|m| m.uid == x.uid).map(|m| (m.lanes.len(), m.leave_arrows.len()));
        let counts_differ = counts(a) != counts(b);
        arm_details(x, y, counts_differ, plan_differs) != arm_details(y, x, counts_differ, plan_differs)
    })
}

/// The things beyond the others that differ between the two designs, in the order of `JOther`. Only what the person
/// can see counts: the ring's presence and size (not when the control already says a roundabout came or went), a
/// raised table, the bus lane's streets and width, each corner's radius by its street, and the details of the
/// streets (`arm_details_differ`). `sa` and `sb` are the designs `a` and `b` were read from (`Junction::current`).
pub fn others(a: &JView, b: &JView, sa: &State, sb: &State) -> Vec<JOther> {
    let ring = |v: &JView| v.ring.as_ref().map(|r| (r.radius_mm, r.road_mm, r.island_mm, r.cycle_mm));
    let bus = |v: &JView| v.bus.as_ref().map(|x| (x.from.min(x.to), x.from.max(x.to), x.width_mm));
    let corners = |v: &JView| {
        let mut all: Vec<(u32, i32)> = v.corners.iter().map(|c| (c.uid, c.radius_mm)).collect();
        all.sort_unstable();
        all
    };
    [
        (JOther::Ring, a.control_index == b.control_index && ring(a) != ring(b)),
        (JOther::Raised, a.raised != b.raised),
        (JOther::Bus, bus(a) != bus(b)),
        (JOther::Corners, corners(a) != corners(b)),
        (JOther::ArmDetails, arm_details_differ(sa, sb, a, b)),
    ]
    .into_iter()
    .filter_map(|(other, differs)| differs.then_some(other))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::junction::model::{Junction, PRIORITY, ROUNDABOUT, SIGNAL};
    use crate::shared::testing::en;

    fn base() -> Junction {
        Junction::new(0)
    }

    fn others_of(a: &Junction, b: &Junction) -> Vec<JOther> {
        others(&a.view(), &b.view(), a.current(), b.current())
    }

    fn views(a: &Junction, b: &Junction) -> (JView, JView) {
        (a.view(), b.view())
    }

    #[test]
    fn identical_junctions_differ_in_nothing() {
        let (a, b) = views(&base(), &base());
        assert!(control(&a, &b).is_empty());
        assert!(arms(&a, &b).is_empty());
        assert!(signal(&a, &b).is_empty());
        assert_eq!(checks(&a, &b), (vec![], vec![]));
        assert!(others_of(&base(), &base()).is_empty());
    }

    #[test]
    fn a_signal_that_goes_is_a_control_line_and_has_no_signal_lines() {
        let mut j = base();
        assert_eq!(j.view().control_index, SIGNAL, "a new junction is run by a signal");
        assert!(j.set_control(PRIORITY));
        let (a, b) = views(&base(), &j);
        let lines = control(&a, &b);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].key, "cmp-jn-control");
        assert!(en(&lines[0]).contains("Traffic signal"), "{}", en(&lines[0]));
        // A signal that appears or goes is the control line's; the signal's own lines need one on both sides.
        assert!(signal(&a, &b).is_empty() && signal(&b, &a).is_empty());
    }

    #[test]
    fn a_signal_with_another_phase_count_or_cycle_is_said() {
        let (a, mut b) = (base().view(), base().view());
        b.signal.as_mut().unwrap().cycle_s += 10;
        b.signal.as_mut().unwrap().phases.pop();
        let keys: Vec<_> = signal(&a, &b).iter().map(|s| s.key).collect();
        assert_eq!(keys, ["cmp-jn-phases", "cmp-jn-cycle"]);
    }

    #[test]
    fn a_corner_radius_alone_is_an_other() {
        let a = base().view();
        let mut j = base();
        let uid = a.arms[0].uid;
        assert!(j.set_corner(uid, a.corners.iter().find(|c| c.uid == uid).unwrap().radius_mm + 1000));
        let b = j.view();
        assert_eq!(others_of(&base(), &j), vec![JOther::Corners]);
        assert!(arms(&a, &b).is_empty() && control(&a, &b).is_empty());
    }

    #[test]
    fn a_removed_crossing_is_one_arm_change() {
        let a = base().view();
        let uid = a.arms.iter().find(|x| x.crossing.is_some()).expect("a sample arm has a crossing").uid;
        let mut j = base();
        assert!(j.set_crossing(uid, false));
        let b = j.view();
        let changes = arms(&a, &b);
        assert_eq!(changes.len(), 1);
        assert_eq!((changes[0].uid, changes[0].crossing, changes[0].bike_box), (uid, (true, false), (false, false)));
        let lines = changes[0].lines();
        assert_eq!(lines.iter().map(|s| s.key).collect::<Vec<_>>(), ["cmp-jn-crossing-removed"]);
    }

    #[test]
    fn lanes_are_counted_as_the_inspector_counts_them() {
        let a = base().view();
        let mut b = base().view();
        let arm = &mut b.arms[0];
        let (entering, leaving) = (arm.lanes.len(), arm.leave_arrows.len());
        arm.lanes.pop();
        arm.leave_arrows.pop();
        let changes = arms(&a, &b);
        assert_eq!((changes[0].entering, changes[0].leaving), ((entering, entering - 1), (leaving, leaving - 1)));
        assert_eq!(changes[0].lines().iter().map(|s| s.key).collect::<Vec<_>>(), ["cmp-jn-entering", "cmp-jn-leaving"]);
    }

    #[test]
    fn a_failing_check_that_goes_passes_and_a_new_failing_one_fails() {
        let a = base().view();
        let mut b = base().view();
        let (first, last) = (a.checks.first().unwrap().id, a.checks.last().unwrap().id);
        assert_ne!(first, last);
        // A: the first fails; B: the first is gone, the last now fails.
        let mut a = a;
        a.checks.iter_mut().find(|c| c.id == first).unwrap().ok = false;
        b.checks.retain(|c| c.id != first);
        b.checks.iter_mut().find(|c| c.id == last).unwrap().ok = false;
        let (passes, fails) = checks(&a, &b);
        assert_eq!(passes.len(), 1);
        assert_eq!(fails.len(), 1);
        assert_eq!(passes[0], a.checks.iter().find(|c| c.id == first).unwrap().label);
    }

    #[test]
    fn a_ring_is_an_other_only_between_two_roundabouts() {
        let mut r = base();
        assert!(r.set_control(ROUNDABOUT));
        // A roundabout that comes is the control line's, not the ring's.
        assert!(!others_of(&base(), &r).contains(&JOther::Ring));
        assert!(!others_of(&r, &base()).contains(&JOther::Ring));
        assert_eq!(control(&base().view(), &r.view()).len(), 1);
        let mut bigger = r.clone();
        assert!(bigger.set_ring(2000));
        assert_eq!(others_of(&r, &bigger), vec![JOther::Ring]);
    }

    #[test]
    fn priority_to_signal_is_a_control_line_and_no_signal_lines() {
        let mut p = base();
        assert!(p.set_control(PRIORITY));
        let (a, b) = views(&p, &base());
        assert_eq!(control(&a, &b).iter().map(|s| s.key).collect::<Vec<_>>(), ["cmp-jn-control"]);
        assert!(signal(&a, &b).is_empty());
    }

    #[test]
    fn a_lane_destination_alone_is_an_arm_detail() {
        let a = base();
        let (uid, lane, to) = a.current().arms.iter().find_map(|x| x.lanes.first().and_then(|l| l.to.first().map(|t| (x.uid, 0, *t)))).unwrap();
        let mut b = base();
        assert!(b.set_lane_dest(uid, lane, to, false));
        assert_eq!(others_of(&a, &b), vec![JOther::ArmDetails]);
    }

    #[test]
    fn a_crossing_width_alone_is_an_arm_detail() {
        let a = base();
        let uid = a.current().arms.iter().find(|x| x.crossing.is_some()).unwrap().uid;
        let mut b = base();
        assert!(b.step_crossing_width(uid, 1));
        assert_eq!(others_of(&a, &b), vec![JOther::ArmDetails]);
        assert!(arms(&a.view(), &b.view()).is_empty());
    }

    #[test]
    fn the_ring_and_a_raised_table_are_named() {
        let mut t = base();
        assert!(t.set_raised(true));
        assert_eq!(others_of(&base(), &t), vec![JOther::Raised]);
        assert_eq!(JOther::Bus.key(), "cmp-other-bus");
        assert_eq!(JOther::Ring.key(), "cmp-other-ring");
        assert_eq!(JOther::ArmDetails.key(), "cmp-other-arm-details");
    }
}
