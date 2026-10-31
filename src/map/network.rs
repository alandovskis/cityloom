//! The network-level design checks: findings that span the loaded streets and no single place can show, read from
//! `NetworkRead` (plain data about each street and junction) and judged here with no DOM, signals or I/O. Every
//! finding says which rule it failed (as `Said` data, worded by the view), which places it concerns and how serious it is.
//!
//! Every figure is a placeholder, UNVERIFIED against a design manual (TAC GDG, MTQ Tome I, NACTO).

use std::collections::{BTreeMap, HashMap};

use crate::city::model::{EndRead, NetworkRead, NodeRead};
use crate::shared::said::{Arg, Said};
use crate::street::model::ACCESS_LANE_MM;

/// A lane count that changes by more than this many lanes where two streets join is abrupt. UNVERIFIED: a lane
/// added or dropped on its own is the usual way a street changes.
pub const MAX_LANE_STEP: usize = 1;
/// A pedestrian crossing wider than this is unsuitable for the people crossing. UNVERIFIED (the junction editor allows
/// up to 6 m).
pub const MAX_CROSSING_MM: i32 = 4_500;
/// A street longer than this between its ends is a block too long to walk round. UNVERIFIED: a placeholder, since the
/// figure a design manual gives depends on the setting.
pub const MAX_BLOCK_MM: i32 = 250_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    Cycling,
    Access,
    Crossings,
    Lanes,
    Transit,
    Walking,
}

impl Check {
    pub const ALL: [Check; 6] = [Check::Cycling, Check::Access, Check::Crossings, Check::Lanes, Check::Transit, Check::Walking];

    /// The rule's name, for an element id; and the message `net-rule-<id>` that words it.
    pub fn id(self) -> &'static str {
        match self {
            Check::Cycling => "cycling",
            Check::Access => "access",
            Check::Crossings => "crossings",
            Check::Lanes => "lanes",
            Check::Transit => "transit",
            Check::Walking => "walking",
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

/// What the checks found, and what they could not look at.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkReport {
    /// Every finding, by check and then by place.
    pub findings: Vec<NetworkFinding>,
    /// How many junctions could not be read (the editor cannot draw them) or have an arm whose street is not in the
    /// network: their crossings are not checked, and that is not the same as nothing found.
    pub unreadable: usize,
}

/// Every finding of the six checks over the network, by check and then by place.
pub fn check(read: &NetworkRead) -> NetworkReport {
    let ctx = Ctx::new(read);
    let mut findings = cycling(&ctx);
    findings.extend(access(&ctx));
    let (crossing_findings, unreadable) = crossings(&ctx);
    findings.extend(crossing_findings);
    findings.extend(lanes(&ctx));
    findings.extend(transit(&ctx));
    findings.extend(walking(&ctx));
    NetworkReport { findings, unreadable }
}

/// The network indexed for the rules.
struct Ctx<'a> {
    read: &'a NetworkRead,
    /// Per node uid: the street ends that meet there, as (index into `read.streets`, 0 for the street's `a` end or 1 for `b`).
    at: BTreeMap<u32, Vec<(usize, usize)>>,
    /// Index into `read.streets` by street uid.
    street_by_uid: HashMap<u32, usize>,
    /// Index into `read.nodes` by node uid.
    node_by_uid: HashMap<u32, usize>,
}

impl<'a> Ctx<'a> {
    fn new(read: &'a NetworkRead) -> Ctx<'a> {
        let mut at: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
        for (i, s) in read.streets.iter().enumerate() {
            at.entry(s.a).or_default().push((i, 0));
            at.entry(s.b).or_default().push((i, 1));
        }
        let street_by_uid = read.streets.iter().enumerate().map(|(i, s)| (s.uid, i)).collect();
        let node_by_uid = read.nodes.iter().enumerate().map(|(i, n)| (n.uid, i)).collect();
        Ctx { read, at, street_by_uid, node_by_uid }
    }

    fn street_place(&self, i: usize) -> Place {
        let s = &self.read.streets[i];
        Place { kind: PlaceKind::Street, uid: s.uid, name: s.name.clone() }
    }

    fn node(&self, uid: u32) -> Option<&NodeRead> {
        self.node_by_uid.get(&uid).map(|&i| &self.read.nodes[i])
    }

    /// The junction at `node` as a place, when it is one.
    fn junction_place(&self, node: u32) -> Option<Place> {
        self.node(node).filter(|n| n.junction).map(|n| Place { kind: PlaceKind::Junction, uid: n.uid, name: n.name.clone() })
    }

    fn end(&self, (i, e): (usize, usize)) -> &EndRead {
        &self.read.streets[i].ends[e]
    }
}

fn count(key: &'static str, n: usize) -> Arg {
    Arg::Said(Box::new(Said::new(key).with("n", Arg::Num(n as i64))))
}

fn name(s: &Said) -> Arg {
    Arg::Said(Box::new(s.clone()))
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
            let street = &ctx.read.streets[here.0].called;
            let at = ctx.node(node).map_or(street, |n| &n.name);
            let mut places = vec![ctx.street_place(here.0)];
            places.extend(ctx.junction_place(node));
            found.push(NetworkFinding { rule, severity: Severity::Notice, said: Said::new(key).with("street", name(street)).with("at", name(at)), places });
        }
    }
    found
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Cycling links cut off from the rest of the bike network, and bike lanes that stop at a node.
fn cycling(ctx: &Ctx) -> Vec<NetworkFinding> {
    let streets = &ctx.read.streets;
    let has_bike = |i: usize| streets[i].bike || streets[i].ends.iter().any(|e| e.bike);
    // the streets with a bike lane, joined where both have one at the node they share (so the link and the ends agree)
    let mut parent: Vec<usize> = (0..streets.len()).collect();
    for ends in ctx.at.values() {
        let mut bikes = ends.iter().filter(|&&e| ctx.end(e).bike).map(|&(i, _)| i);
        if let Some(first) = bikes.next() {
            for i in bikes {
                let (a, b) = (root(&mut parent, first), root(&mut parent, i));
                parent[b] = a;
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in (0..streets.len()).filter(|&i| has_bike(i)) {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let groups: Vec<Vec<usize>> = groups.into_values().collect();
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

/// Pedestrian streets that have no unbroken sidewalk run for emergency access. The rule is absolute: a street is
/// judged as it stands, not against how it was first laid out (the places' own checks do that), so an imported street
/// that was never wide enough stays a notice until it is changed.
fn access(ctx: &Ctx) -> Vec<NetworkFinding> {
    ctx.read
        .streets
        .iter()
        .enumerate()
        .filter(|(_, s)| s.pedestrian && !s.walkway_ok)
        .map(|(i, _)| NetworkFinding {
            rule: Check::Access,
            severity: Severity::Notice,
            said: Said::new("net-access-broken").with("need", Arg::Length(ACCESS_LANE_MM)),
            places: vec![ctx.street_place(i)],
        })
        .collect()
}

/// Arms of a junction that take traffic and have no crossing, or one too wide; and how many junctions could not be
/// looked at.
fn crossings(ctx: &Ctx) -> (Vec<NetworkFinding>, usize) {
    let mut found = Vec::new();
    let mut unreadable = 0;
    for node in ctx.read.nodes.iter().filter(|n| n.junction) {
        let mut unknown = node.unreadable;
        for arm in &node.arms {
            let Some(&i) = ctx.street_by_uid.get(&arm.street) else {
                unknown = true;
                continue;
            };
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
        unreadable += usize::from(unknown);
    }
    (found, unreadable)
}

/// Two streets that join at a plain connection with lane counts that differ by more than a lane. A freeway or a
/// pedestrian street is not compared: its lanes are not a choice of the street it joins.
fn lanes(ctx: &Ctx) -> Vec<NetworkFinding> {
    let mut found = Vec::new();
    for (&node, ends) in &ctx.at {
        if ctx.node(node).is_some_and(|n| n.junction) {
            continue;
        }
        let [x, y] = ends.as_slice() else { continue };
        let (sx, sy) = (&ctx.read.streets[x.0], &ctx.read.streets[y.0]);
        if x.0 == y.0 || sx.pedestrian || sy.pedestrian || sx.freeway || sy.freeway {
            continue;
        }
        let (from, to) = (ctx.end(*x).lanes, ctx.end(*y).lanes);
        if from.abs_diff(to) > MAX_LANE_STEP {
            found.push(NetworkFinding {
                rule: Check::Lanes,
                severity: Severity::Problem,
                said: Said::new("net-lanes-change")
                    .with("a", name(&sx.called))
                    .with("from", count("net-lanes", from))
                    .with("b", name(&sy.called))
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

/// Walking routes, read as routes across the network and not as single crossings or sections: a sidewalk that stops
/// where the next street has none, a part of the network people cannot walk to from the rest, and a block too long
/// to walk round. Accessible curb transitions are not modelled (the editors hold no curb ramps), so they are covered
/// only as far as sidewalk continuity and the crossings go: a sidewalk that stops at a node is a break in the route,
/// and an arm with no crossing does not join the walking network (the Crossings check names it too).
fn walking(ctx: &Ctx) -> Vec<NetworkFinding> {
    let mut found = walking_ends(ctx);
    found.extend(walking_islands(ctx));
    found.extend(long_blocks(ctx));
    found
}

/// Sidewalks that stop at a node where another street goes on without one. Where the only other streets are
/// freeways the walkway is cut by a barrier, which the islands report.
fn walking_ends(ctx: &Ctx) -> Vec<NetworkFinding> {
    let mut found = Vec::new();
    for (&node, ends) in &ctx.at {
        for &here in ends {
            if !ctx.end(here).sidewalk {
                continue;
            }
            let others: Vec<(usize, usize)> = ends.iter().copied().filter(|o| o.0 != here.0).collect();
            let goes_on = others.iter().any(|&o| !ctx.read.streets[o.0].freeway);
            if !goes_on || others.iter().any(|&o| ctx.end(o).sidewalk) {
                continue;
            }
            let street = &ctx.read.streets[here.0].called;
            let at = ctx.node(node).map_or(street, |n| &n.name);
            let mut places = vec![ctx.street_place(here.0)];
            places.extend(ctx.junction_place(node));
            found.push(NetworkFinding {
                rule: Check::Walking,
                severity: Severity::Notice,
                said: Said::new("net-walking-ends").with("street", name(street)).with("at", name(at)),
                places,
            });
        }
    }
    found
}

/// Streets people can walk, joined into routes: at a plain connection every walkable street end joins; at a junction
/// only the arms that can be crossed (a pedestrian street, or an arm with a pedestrian crossing) do. A junction that
/// could not be read, or an arm of no known street, is given the benefit of the doubt. A route that is not the biggest
/// is cut off from the rest, by a freeway, a street with no sidewalk or an arm with no crossing.
fn walking_islands(ctx: &Ctx) -> Vec<NetworkFinding> {
    let streets = &ctx.read.streets;
    let mut parent: Vec<usize> = (0..streets.len()).collect();
    // junction uid -> the walkable streets that end there and cannot be crossed to
    let mut cut_at: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (&node_uid, ends) in &ctx.at {
        let walkable: Vec<(usize, usize)> = ends.iter().copied().filter(|&(i, _)| streets[i].walkable).collect();
        let junction = ctx.node(node_uid).filter(|n| n.junction && !n.unreadable && !n.arms.is_empty());
        let mut joined = Vec::new();
        for &(i, _) in &walkable {
            let passable = match junction {
                None => true,
                Some(n) => match n.arms.iter().find(|a| a.street == streets[i].uid) {
                    None => true,
                    Some(arm) => streets[i].pedestrian || arm.crossing_mm.is_some(),
                },
            };
            if passable {
                joined.push(i);
            } else {
                cut_at.entry(node_uid).or_default().push(i);
            }
        }
        if let Some((&first, rest)) = joined.split_first() {
            for &i in rest {
                let (a, b) = (root(&mut parent, first), root(&mut parent, i));
                parent[b] = a;
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in (0..streets.len()).filter(|&i| streets[i].walkable) {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let groups: Vec<Vec<usize>> = groups.into_values().collect();
    let main = (0..groups.len()).max_by_key(|&g| (groups[g].len(), std::cmp::Reverse(groups[g][0])));
    (0..groups.len())
        .filter(|&g| Some(g) != main)
        .map(|g| {
            let mut places: Vec<Place> = groups[g].iter().map(|&i| ctx.street_place(i)).collect();
            // the junctions where the route ends against an arm with no crossing: its own, or a neighbour's
            // (or against a freeway or a street with no sidewalk)
            for (&node, ends) in &ctx.at {
                let cuts = cut_at.contains_key(&node) || ends.iter().any(|&(i, _)| !streets[i].walkable);
                if cuts && ends.iter().any(|(i, _)| groups[g].contains(i)) {
                    places.extend(ctx.junction_place(node));
                }
            }
            NetworkFinding {
                rule: Check::Walking,
                severity: Severity::Problem,
                said: Said::new("net-walking-island").with("n", Arg::Num(groups[g].len() as i64)),
                places,
            }
        })
        .collect()
}

/// Streets longer than a block should be, with the junctions at their ends. A freeway is not a block.
fn long_blocks(ctx: &Ctx) -> Vec<NetworkFinding> {
    ctx.read
        .streets
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.freeway && s.length_mm > MAX_BLOCK_MM)
        .map(|(i, s)| {
            let mut places = vec![ctx.street_place(i)];
            for node in [s.a, s.b] {
                places.extend(ctx.junction_place(node));
            }
            NetworkFinding {
                rule: Check::Walking,
                severity: Severity::Notice,
                said: Said::new("net-walking-block")
                    .with("street", name(&s.called))
                    .with("length", Arg::Length(s.length_mm))
                    .with("max", Arg::Length(MAX_BLOCK_MM)),
                places,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::model::{ArmRead, EndRead, NodeRead, StreetRead};

    fn end(lanes: usize, bike: bool, bus: bool) -> EndRead {
        EndRead { lanes, bike, bus, sidewalk: true }
    }

    fn street(uid: u32, a: u32, b: u32) -> StreetRead {
        StreetRead {
            uid,
            a,
            b,
            name: Said::new("test-street"),
            called: Said::new("test-called"),
            pedestrian: false,
            freeway: false,
            length_mm: 80_000,
            walkable: true,
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
            unreadable: false,
        }
    }

    fn with_bike(mut s: StreetRead) -> StreetRead {
        s.bike = true;
        s.ends = [end(2, true, false), end(2, true, false)];
        s
    }

    fn of(read: &NetworkRead, rule: Check) -> Vec<NetworkFinding> {
        check(read).findings.into_iter().filter(|f| f.rule == rule).collect()
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
        assert_eq!(f[0].severity, Severity::Notice, "the rule is absolute, not against how the street was first laid out");
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

    #[test]
    fn two_bike_streets_join_only_where_both_have_a_bike_lane_at_the_node() {
        let mut r = line();
        r.streets[0] = with_bike(r.streets[0].clone());
        r.streets[1] = with_bike(r.streets[1].clone());
        // street 2 has a lane somewhere along it but none at the end it meets street 1 by
        r.streets[1].ends[0] = end(2, false, false);
        let islands = |r: &NetworkRead| of(r, Check::Cycling).into_iter().filter(|f| f.said.key == "net-cycling-island").count();
        assert_eq!(islands(&r), 1, "the two are not joined where the lane does not meet");
        r.streets[1].ends[0] = end(2, true, false);
        assert_eq!(islands(&r), 0);
    }

    #[test]
    fn lanes_are_not_compared_with_a_freeway() {
        let mut r = line();
        r.streets[0].ends[1] = end(5, false, false);
        r.streets[0].freeway = true;
        assert!(of(&r, Check::Lanes).is_empty());
    }

    #[test]
    fn a_finding_names_the_streets_and_the_place_it_is_about() {
        let mut r = line();
        r.streets[0].ends[1] = end(4, false, false);
        let f = &of(&r, Check::Lanes)[0];
        let arg = |k: &str| f.said.args.iter().find(|(n, _)| *n == k).map(|(_, a)| a.clone());
        assert!(matches!(arg("a"), Some(Arg::Said(_))) && matches!(arg("b"), Some(Arg::Said(_))), "{:?}", f.said);
        let mut r = line();
        r.streets[0] = with_bike(r.streets[0].clone());
        r.nodes[1] = node(2, &[(1, None), (2, None)]);
        let f = of(&r, Check::Cycling).into_iter().find(|f| f.said.key == "net-cycling-ends").unwrap();
        assert!(f.said.args.iter().any(|(n, _)| *n == "street") && f.said.args.iter().any(|(n, _)| *n == "at"), "{:?}", f.said);
    }

    #[test]
    fn a_junction_that_could_not_be_read_or_has_an_arm_of_no_known_street_is_counted_not_passed() {
        let mut r = line();
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, Some(3_000))]);
        assert_eq!(check(&r).unreadable, 0);
        r.nodes[1].unreadable = true;
        assert_eq!(check(&r).unreadable, 1);
        r.nodes[1].unreadable = false;
        r.nodes[1].arms.push(ArmRead { street: 99, crossing_mm: None });
        let report = check(&r);
        assert_eq!(report.unreadable, 1);
        assert!(report.findings.is_empty(), "an arm that cannot be found is not a missing crossing: {:?}", report.findings);
    }

    fn walking_of(r: &NetworkRead, key: &str) -> Vec<NetworkFinding> {
        of(r, Check::Walking).into_iter().filter(|f| f.said.key == key).collect()
    }

    /// Street `s` as a street with no sidewalk (and so not walkable).
    fn without_sidewalk(mut s: StreetRead) -> StreetRead {
        s.walkable = false;
        s.ends[0].sidewalk = false;
        s.ends[1].sidewalk = false;
        s
    }

    #[test]
    fn a_sidewalk_that_stops_where_the_next_street_has_none_is_found_with_its_junction() {
        let mut r = line();
        r.streets[1] = without_sidewalk(r.streets[1].clone());
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, Some(3_000)), (5, Some(3_000))]);
        let f = walking_of(&r, "net-walking-ends");
        // street 1 stops at node 2, and street 3 stops at node 3 (a plain connection: no junction to name)
        assert_eq!(f.len(), 2, "{f:?}");
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 1), (PlaceKind::Junction, 2)]);
        assert_eq!(uids(&f[1]), vec![(PlaceKind::Street, 3)]);
        assert_eq!(f[0].severity, Severity::Notice);
        // the sidewalk goes on along the next street
        r.streets[1] = line().streets[1].clone();
        assert!(walking_of(&r, "net-walking-ends").is_empty());
    }

    #[test]
    fn a_sidewalk_that_meets_only_a_freeway_or_the_edge_of_the_map_is_not_an_end_to_report() {
        let mut r = line();
        r.streets[1].freeway = true;
        r.streets[1] = without_sidewalk(r.streets[1].clone());
        // street 1's sidewalk meets a freeway at node 2, and street 4 is left to run off the map
        assert!(walking_of(&r, "net-walking-ends").is_empty(), "{:?}", walking_of(&r, "net-walking-ends"));
    }

    #[test]
    fn a_street_cut_off_from_the_walking_network_by_a_street_without_a_sidewalk_is_an_island_and_a_sidewalk_clears_it() {
        let mut r = line();
        r.streets[2] = without_sidewalk(r.streets[2].clone());
        // streets 1 and 2 are joined (the bigger route), street 4 is alone beyond street 3
        let f = walking_of(&r, "net-walking-island");
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 4)]);
        assert_eq!(f[0].severity, Severity::Problem);
        assert_eq!(f[0].said.args[0].1, Arg::Num(1));
        r.streets[2] = line().streets[2].clone();
        assert!(walking_of(&r, "net-walking-island").is_empty());
    }

    #[test]
    fn a_freeway_cuts_the_walking_network() {
        let mut r = line();
        r.streets[1].freeway = true;
        r.streets[1].walkable = false;
        let f = walking_of(&r, "net-walking-island");
        assert_eq!(f.len(), 1, "{f:?}");
        // the one street on its own side is the island; the two on the other side are the bigger route
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 1)]);
        assert_eq!(f[0].said.args[0].1, Arg::Num(1));
    }

    #[test]
    fn an_arm_with_no_crossing_does_not_join_the_walking_network_and_the_junction_is_named() {
        let mut r = line();
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, None)]);
        // street 1 is alone on one side of node 2, streets 2..4 are the bigger route
        let f = walking_of(&r, "net-walking-island");
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 1), (PlaceKind::Junction, 2)]);
        // with the crossing, the route is whole
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, Some(3_000))]);
        assert!(walking_of(&r, "net-walking-island").is_empty());
        // a pedestrian street needs no crossing to be walked on to
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, None)]);
        r.streets[1].pedestrian = true;
        assert!(walking_of(&r, "net-walking-island").is_empty());
    }

    #[test]
    fn a_junction_that_could_not_be_read_does_not_cut_the_walking_network() {
        let mut r = line();
        r.nodes[1] = node(2, &[(1, None), (2, None)]);
        r.nodes[1].unreadable = true;
        assert!(walking_of(&r, "net-walking-island").is_empty());
    }

    #[test]
    fn a_block_longer_than_the_maximum_is_found_with_the_junctions_at_its_ends_and_a_freeway_is_not_a_block() {
        let mut r = line();
        r.nodes[1] = node(2, &[(1, Some(3_000)), (2, Some(3_000))]);
        r.streets[1].length_mm = MAX_BLOCK_MM + 1;
        let f = walking_of(&r, "net-walking-block");
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(uids(&f[0]), vec![(PlaceKind::Street, 2), (PlaceKind::Junction, 2)]);
        r.streets[1].length_mm = MAX_BLOCK_MM;
        assert!(walking_of(&r, "net-walking-block").is_empty());
        r.streets[1].length_mm = MAX_BLOCK_MM * 3;
        r.streets[1].freeway = true;
        assert!(walking_of(&r, "net-walking-block").is_empty());
    }
}
