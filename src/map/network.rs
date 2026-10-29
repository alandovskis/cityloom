//! The network-level design checks: findings that span the loaded streets and no single place can show, read from
//! `NetworkRead` (plain data about each street and junction) and judged here with no DOM, signals or I/O. Every
//! finding says which rule it failed (as `Said` data, worded by the view), which places it concerns and how serious it is.
//!
//! Every figure is a placeholder, UNVERIFIED against a design manual (TAC GDG, MTQ Tome I, NACTO).

use std::collections::BTreeMap;

use crate::city::model::{EndRead, NetworkRead};
use crate::shared::said::{Arg, Said};
use crate::street::model::ACCESS_LANE_MM;

/// A lane count that changes by more than this many lanes where two streets join is abrupt. UNVERIFIED: a lane
/// added or dropped on its own is the usual way a street changes.
pub const MAX_LANE_STEP: usize = 1;
/// A pedestrian crossing wider than this is unsuitable for the people crossing. UNVERIFIED (the junction editor allows
/// up to 6 m).
pub const MAX_CROSSING_MM: i32 = 4_500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    Cycling,
    Access,
    Crossings,
    Lanes,
    Transit,
}

impl Check {
    pub const ALL: [Check; 5] = [Check::Cycling, Check::Access, Check::Crossings, Check::Lanes, Check::Transit];

    /// The rule's name, for an element id; and the message `net-rule-<id>` that words it.
    pub fn id(self) -> &'static str {
        match self {
            Check::Cycling => "cycling",
            Check::Access => "access",
            Check::Crossings => "crossings",
            Check::Lanes => "lanes",
            Check::Transit => "transit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The network breaks the rule.
    Problem,
    /// Worth a look: the rule may be met some other way.
    Notice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceKind {
    Street,
    Junction,
}

/// A place a finding concerns, named as the notes name it.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub kind: PlaceKind,
    pub uid: u32,
    pub name: Said,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkFinding {
    pub rule: Check,
    pub severity: Severity,
    /// Why the rule fails here.
    pub said: Said,
    pub places: Vec<Place>,
}

/// Every finding of the five checks over the network, by rule and then by place.
pub fn check(read: &NetworkRead) -> Vec<NetworkFinding> {
    let ctx = Ctx::new(read);
    let mut found = cycling(&ctx);
    found.extend(access(&ctx));
    found.extend(crossings(&ctx));
    found.extend(lanes(&ctx));
    found.extend(transit(&ctx));
    found
}

/// The network indexed for the rules.
struct Ctx<'a> {
    read: &'a NetworkRead,
    /// Per node uid: the street ends that meet there, as (index into `read.streets`, 0 for the street's `a` end or 1 for `b`).
    at: BTreeMap<u32, Vec<(usize, usize)>>,
}

impl<'a> Ctx<'a> {
    fn new(read: &'a NetworkRead) -> Ctx<'a> {
        let mut at: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
        for (i, s) in read.streets.iter().enumerate() {
            at.entry(s.a).or_default().push((i, 0));
            at.entry(s.b).or_default().push((i, 1));
        }
        Ctx { read, at }
    }

    fn street_place(&self, i: usize) -> Place {
        let s = &self.read.streets[i];
        Place { kind: PlaceKind::Street, uid: s.uid, name: s.name.clone() }
    }

    /// The junction at `node` as a place, when it is one.
    fn junction_place(&self, node: u32) -> Option<Place> {
        self.read.nodes.iter().find(|n| n.uid == node && n.junction).map(|n| Place { kind: PlaceKind::Junction, uid: n.uid, name: n.name.clone() })
    }

    fn end(&self, (i, e): (usize, usize)) -> &EndRead {
        &self.read.streets[i].ends[e]
    }
}

fn count(key: &'static str, n: usize) -> Arg {
    Arg::Said(Box::new(Said::new(key).with("n", Arg::Num(n as i64))))
}

/// The street ends where something runs (`has` is true of its end) and nothing that meets it there does: the
/// lane the street has stops at the node. Plain connections and junctions alike; where a street leaves the map
/// (no other street) there is nothing to join.
fn ends_where_it_stops(ctx: &Ctx, has: impl Fn(&EndRead) -> bool, rule: Check, key: &'static str) -> Vec<NetworkFinding> {
    let mut found = Vec::new();
    for (&node, ends) in &ctx.at {
        if ends.len() < 2 {
            continue;
        }
        for &here in ends {
            if !has(ctx.end(here)) || ends.iter().any(|&other| other != here && has(ctx.end(other))) {
                continue;
            }
            let mut places = vec![ctx.street_place(here.0)];
            places.extend(ctx.junction_place(node));
            found.push(NetworkFinding { rule, severity: Severity::Notice, said: Said::new(key), places });
        }
    }
    found
}

/// Cycling links cut off from the rest of the bike network, and bike lanes that stop at a node.
fn cycling(ctx: &Ctx) -> Vec<NetworkFinding> {
    let streets = &ctx.read.streets;
    // the streets with a bike lane, joined where they share a node
    let bikes: Vec<usize> = (0..streets.len()).filter(|&i| streets[i].bike).collect();
    let mut component: BTreeMap<usize, usize> = BTreeMap::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for &start in &bikes {
        if component.contains_key(&start) {
            continue;
        }
        let id = groups.len();
        let mut group = Vec::new();
        let mut todo = vec![start];
        while let Some(i) = todo.pop() {
            if component.insert(i, id).is_some() {
                continue;
            }
            group.push(i);
            for node in [streets[i].a, streets[i].b] {
                todo.extend(ctx.at[&node].iter().map(|&(j, _)| j).filter(|&j| streets[j].bike && !component.contains_key(&j)));
            }
        }
        group.sort_unstable();
        groups.push(group);
    }
    // the biggest link is the network; the others are cut off from it (the first, on a tie)
    let main = (0..groups.len()).max_by_key(|&g| (groups[g].len(), std::cmp::Reverse(groups[g][0])));
    let mut found: Vec<NetworkFinding> = (0..groups.len())
        .filter(|&g| Some(g) != main)
        .map(|g| NetworkFinding {
            rule: Check::Cycling,
            severity: Severity::Problem,
            said: Said::new("net-cycling-island").with("n", Arg::Num(groups[g].len() as i64)),
            places: groups[g].iter().map(|&i| ctx.street_place(i)).collect(),
        })
        .collect();
    found.extend(ends_where_it_stops(ctx, |e| e.bike, Check::Cycling, "net-cycling-ends"));
    found
}

/// Pedestrian streets that have no unbroken sidewalk run for emergency access.
fn access(ctx: &Ctx) -> Vec<NetworkFinding> {
    ctx.read
        .streets
        .iter()
        .enumerate()
        .filter(|(_, s)| s.pedestrian && !s.walkway_ok)
        .map(|(i, _)| NetworkFinding {
            rule: Check::Access,
            severity: Severity::Problem,
            said: Said::new("net-access-broken").with("need", Arg::Length(ACCESS_LANE_MM)),
            places: vec![ctx.street_place(i)],
        })
        .collect()
}

/// Arms of a junction that take traffic and have no crossing, or one too wide.
fn crossings(ctx: &Ctx) -> Vec<NetworkFinding> {
    let mut found = Vec::new();
    for node in ctx.read.nodes.iter().filter(|n| n.junction) {
        for arm in &node.arms {
            let Some(i) = ctx.read.streets.iter().position(|s| s.uid == arm.street) else { continue };
            let s = &ctx.read.streets[i];
            if s.pedestrian || s.freeway {
                continue;
            }
            let (severity, said) = match arm.crossing_mm {
                None => (Severity::Problem, Said::new("net-crossing-missing")),
                Some(w) if w > MAX_CROSSING_MM => {
                    (Severity::Notice, Said::new("net-crossing-wide").with("width", Arg::Length(w)).with("max", Arg::Length(MAX_CROSSING_MM)))
                }
                Some(_) => continue,
            };
            found.push(NetworkFinding {
                rule: Check::Crossings,
                severity,
                said,
                places: vec![ctx.street_place(i), Place { kind: PlaceKind::Junction, uid: node.uid, name: node.name.clone() }],
            });
        }
    }
    found
}

/// Two streets that join at a plain connection with lane counts that differ by more than a lane.
fn lanes(ctx: &Ctx) -> Vec<NetworkFinding> {
    let mut found = Vec::new();
    for (&node, ends) in &ctx.at {
        if ctx.read.nodes.iter().any(|n| n.uid == node && n.junction) {
            continue;
        }
        let [x, y] = ends.as_slice() else { continue };
        let (sx, sy) = (&ctx.read.streets[x.0], &ctx.read.streets[y.0]);
        if x.0 == y.0 || sx.pedestrian || sy.pedestrian {
            continue;
        }
        let (from, to) = (ctx.end(*x).lanes, ctx.end(*y).lanes);
        if from.abs_diff(to) > MAX_LANE_STEP {
            found.push(NetworkFinding {
                rule: Check::Lanes,
                severity: Severity::Problem,
                said: Said::new("net-lanes-change")
                    .with("from", count("net-lanes", from))
                    .with("to", count("net-lanes", to))
                    .with("step", count("net-lanes", MAX_LANE_STEP)),
                places: vec![ctx.street_place(x.0), ctx.street_place(y.0)],
            });
        }
    }
    found
}

/// Transit lanes that stop at a node where no other street carries one on.
fn transit(ctx: &Ctx) -> Vec<NetworkFinding> {
    ends_where_it_stops(ctx, |e| e.bus, Check::Transit, "net-transit-break")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::model::{ArmRead, EndRead, NodeRead, StreetRead};

    fn end(lanes: usize, bike: bool, bus: bool) -> EndRead {
        EndRead { lanes, bike, bus }
    }

    fn street(uid: u32, a: u32, b: u32) -> StreetRead {
        StreetRead {
            uid,
            a,
            b,
            name: Said::new("test-street"),
            pedestrian: false,
            freeway: false,
            bike: false,
            walkway_ok: true,
            ends: [end(2, false, false), end(2, false, false)],
        }
    }

    fn node(uid: u32, arms: &[(u32, Option<i32>)]) -> NodeRead {
        NodeRead {
            uid,
            name: Said::new("test-node"),
            junction: !arms.is_empty(),
            arms: arms.iter().map(|&(street, crossing_mm)| ArmRead { street, crossing_mm }).collect(),
        }
    }

    fn with_bike(mut s: StreetRead) -> StreetRead {
        s.bike = true;
        s.ends = [end(2, true, false), end(2, true, false)];
        s
    }

    fn of(read: &NetworkRead, rule: Check) -> Vec<NetworkFinding> {
        check(read).into_iter().filter(|f| f.rule == rule).collect()
    }

    fn uids(f: &NetworkFinding) -> Vec<(PlaceKind, u32)> {
        f.places.iter().map(|p| (p.kind, p.uid)).collect()
    }

    /// A line of streets 1..=4 over nodes 1..=5.
    fn line() -> NetworkRead {
        NetworkRead { streets: (1..=4).map(|i| street(i, i, i + 1)).collect(), nodes: (1..=5).map(|i| node(i, &[])).collect() }
    }

    #[test]
    fn a_bike_link_cut_off_from_the_rest_of_the_bike_network_is_found_and_joining_it_clears_it() {
        let mut r = line();
        r.streets[0] = with_bike(r.streets[0].clone());
        r.streets[1] = with_bike(r.streets[1].clone());
        r.streets[3] = with_bike(r.streets[3].clone());
        // streets 1 and 2 are joined, street 4 is alone; street 3 has no bike lane between them
        let islands: Vec<_> = of(&r, Check::Cycling).into_iter().filter(|f| f.said.key == "net-cycling-island").collect();
        assert_eq!(islands.len(), 1, "{islands:?}");
        assert_eq!(uids(&islands[0]), vec![(PlaceKind::Street, 4)], "the larger link is not the one cut off");
        r.streets[2] = with_bike(r.streets[2].clone());
        assert!(of(&r, Check::Cycling).iter().all(|f| f.said.key != "net-cycling-island"));
    }

    #[test]
    fn a_bike_lane_that_ends_where_no_other_street_has_one_is_found_at_its_junction() {
        let mut r = line();
        r.streets[0] = with_bike(r.streets[0].clone());
        r.nodes[1] = node(2, &[(1, None), (2, None), (5, None)]);
        let ends: Vec<_> = of(&r, Check::Cycling).into_iter().filter(|f| f.said.key == "net-cycling-ends").collect();
        assert_eq!(ends.len(), 1);
        assert_eq!(uids(&ends[0]), vec![(PlaceKind::Street, 1), (PlaceKind::Junction, 2)]);
        // the lane goes on along the next street
        r.streets[1] = with_bike(r.streets[1].clone());
        let still: Vec<_> = of(&r, Check::Cycling).into_iter().filter(|f| f.places.iter().any(|p| p.kind == PlaceKind::Junction && p.uid == 2)).collect();
        assert!(still.is_empty(), "{still:?}");
    }

    #[test]
    fn a_bike_lane_that_runs_to_the_edge_of_the_map_is_not_a_break() {
        let mut r = line();
        r.streets = r.streets.iter().cloned().map(with_bike).collect();
        assert!(of(&r, Check::Cycling).is_empty(), "{:?}", of(&r, Check::Cycling));
    }

    #[test]
    fn a_pedestrian_street_without_an_unbroken_sidewalk_run_is_found() {
        let mut r = line();
        r.streets[1].pedestrian = true;
        r.streets[1].walkway_ok = false;
        let f = of(&r, Check::Access);
        assert_eq!(f.len(), 1);
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 2)]);
        assert_eq!(f[0].severity, Severity::Problem);
        r.streets[1].walkway_ok = true;
        assert!(of(&r, Check::Access).is_empty());
        // a street with traffic is judged by its lanes, not by this rule
        r.streets[1].pedestrian = false;
        r.streets[1].walkway_ok = false;
        assert!(of(&r, Check::Access).is_empty());
    }

    #[test]
    fn an_arm_that_takes_traffic_without_a_crossing_or_with_too_wide_a_one_is_found() {
        let mut r = line();
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, None), (5, Some(MAX_CROSSING_MM + 100))]);
        r.streets.push(street(5, 2, 6));
        let f = of(&r, Check::Crossings);
        assert_eq!(f.len(), 2, "{f:?}");
        let missing = f.iter().find(|f| f.said.key == "net-crossing-missing").unwrap();
        assert_eq!(uids(missing), vec![(PlaceKind::Street, 2), (PlaceKind::Junction, 2)]);
        let wide = f.iter().find(|f| f.said.key == "net-crossing-wide").unwrap();
        assert_eq!(uids(wide), vec![(PlaceKind::Street, 5), (PlaceKind::Junction, 2)]);
        assert_eq!(wide.severity, Severity::Notice);
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, Some(3_000)), (5, Some(MAX_CROSSING_MM))]);
        assert!(of(&r, Check::Crossings).is_empty());
    }

    #[test]
    fn a_pedestrian_street_or_a_freeway_needs_no_crossing() {
        let mut r = line();
        r.streets[0].pedestrian = true;
        r.streets[1].freeway = true;
        r.nodes[1] = node(2, &[(1, None), (2, None)]);
        assert!(of(&r, Check::Crossings).is_empty());
    }

    #[test]
    fn lanes_that_jump_where_two_streets_join_are_found_and_one_lane_more_is_not() {
        let mut r = line();
        r.streets[0].ends[1] = end(4, false, false);
        let f = of(&r, Check::Lanes);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 1), (PlaceKind::Street, 2)]);
        r.streets[0].ends[1] = end(3, false, false);
        assert!(of(&r, Check::Lanes).is_empty(), "a lane added or dropped is the usual way a street changes");
    }

    #[test]
    fn lanes_are_not_compared_at_a_junction_or_with_a_pedestrian_street() {
        let mut r = line();
        r.streets[0].ends[1] = end(5, false, false);
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, Some(3_000)), (3, Some(3_000))]);
        assert!(of(&r, Check::Lanes).is_empty());
        let mut r = line();
        r.streets[0].ends[1] = end(0, false, false);
        r.streets[0].pedestrian = true;
        r.streets[1].ends[0] = end(4, false, false);
        assert!(of(&r, Check::Lanes).is_empty());
    }

    #[test]
    fn a_transit_lane_that_breaks_at_a_junction_is_found() {
        let mut r = line();
        r.streets[0].ends[1] = end(2, false, true);
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, Some(3_000)), (5, Some(3_000))]);
        let f = of(&r, Check::Transit);
        assert_eq!(f.len(), 1);
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 1), (PlaceKind::Junction, 2)]);
        r.streets[1].ends[0] = end(2, false, true);
        assert!(of(&r, Check::Transit).is_empty());
    }
}
