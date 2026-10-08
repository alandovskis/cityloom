//! Roads divided by a median: two one-way carriageways of one street, running side by side
//! in opposite directions, are one street with a median between its two halves.

use crate::{Lane, LaneKind, Network, Road, Way};

/// The widest a median can be, from centre line to centre line of the carriageways, for them to be one street.
const MAX_APART_M: f64 = 60.0;
/// A median narrower than this is drawn at least this wide.
const MIN_MEDIAN_M: f64 = 1.0;

type P = (f64, f64);

fn dist(a: P, b: P) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The nearest point of the polyline `line` to `p`.
fn nearest(line: &[P], p: P) -> P {
    let mut best = (f64::MAX, p);
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) };
        let q = (a.0 + t * dx, a.1 + t * dy);
        let d = dist(p, q);
        if d < best.0 {
            best = (d, q);
        }
    }
    best.1
}

/// The road turned round (its lanes seen from the other end) if its driving lanes run against it, so
/// that they run along it; None where it is not one-way, or has no driving lane.
fn along(road: &Road) -> Option<Road> {
    let driving: Vec<Way> = road.lanes.iter().filter(|l| l.kind == LaneKind::Driving).map(|l| l.way).collect();
    let first = *driving.first()?;
    if driving.iter().any(|w| *w != first) {
        return None;
    }
    Some(if first == Way::Forward { road.clone() } else { reversed(road) })
}

fn reversed(road: &Road) -> Road {
    let mut r = road.clone();
    std::mem::swap(&mut r.from, &mut r.to);
    r.points.reverse();
    r.lanes.reverse();
    for l in &mut r.lanes {
        l.way = if l.way == Way::Forward { Way::Backward } else { Way::Forward };
    }
    r
}

fn width(road: &Road) -> f64 {
    road.lanes.iter().map(|l| l.width_m).sum()
}

fn length(line: &[P]) -> f64 {
    line.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// How far apart two carriageways run, on average, measured from `a`'s points; None when they do not run beside each other.
fn apart(a: &[P], b: &[P]) -> Option<f64> {
    let d: Vec<f64> = a.iter().map(|&p| dist(p, nearest(b, p))).collect();
    let mean = d.iter().sum::<f64>() / d.len() as f64;
    (mean <= MAX_APART_M && d.iter().all(|x| *x <= MAX_APART_M * 1.5)).then_some(mean)
}

/// Which side of `a`'s direction of travel `b` lies on: positive for the left.
fn side_of(a: &[P], b: &[P]) -> f64 {
    let m = (a.len() - 1) / 2;
    let (p, q) = (a[m], a[m + 1]);
    let n = nearest(b, p);
    (q.0 - p.0) * (n.1 - p.1) - (q.1 - p.1) * (n.0 - p.0)
}

/// Joins each pair of one-way carriageways of one street into a single road with a median.
///
/// A pair is two one-way roads of the same name and kind of street that run in opposite directions, begin
/// and end near each other (at one junction or at two close together, the crossing street's stub between them)
/// and run beside each other. Their ends become one node each, in the middle; a road left joining a node to itself
/// by that (the crossing street's stub) is dropped.
pub fn merge_dual_carriageways(net: &mut Network) {
    let at = |net: &Network, id: u32| net.nodes.iter().find(|n| n.id == id).map(|n| (n.x_m, n.y_m));
    let oriented: Vec<Option<Road>> = net.roads.iter().map(along).collect();
    let mut taken = vec![false; net.roads.len()];
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..net.roads.len() {
        let Some(a) = &oriented[i] else { continue };
        let mut best: Option<(f64, usize)> = None;
        for j in 0..net.roads.len() {
            let Some(b) = &oriented[j] else { continue };
            if i == j || taken[j] || a.name.is_none() || a.name != b.name || a.highway != b.highway {
                continue;
            }
            let (Some(a0), Some(a1), Some(b0), Some(b1)) = (at(net, a.from), at(net, a.to), at(net, b.from), at(net, b.to)) else { continue };
            // b runs the other way: it starts where a ends and ends where a starts
            if dist(a1, b0) > MAX_APART_M || dist(a0, b1) > MAX_APART_M {
                continue;
            }
            if length(&a.points) < 1.0 || b.points.len() < 2 || a.points.len() < 2 {
                continue;
            }
            if let Some(d) = apart(&a.points, &b.points) {
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, j));
                }
            }
        }
        if let Some((_, j)) = best {
            if !taken[i] && oriented[j].as_ref().is_some() {
                taken[i] = true;
                taken[j] = true;
                pairs.push((i, j));
            }
        }
    }
    if pairs.is_empty() {
        return;
    }

    let mut moved: Vec<(u32, u32)> = Vec::new(); // (node, the node it is merged into)
    let mut merged: Vec<Road> = Vec::new();
    let mut gone = vec![false; net.roads.len()];
    for (i, j) in pairs {
        let (a, b) = (oriented[i].clone().unwrap(), oriented[j].clone().unwrap());
        // `b` seen along `a`: driving lanes run against it
        let b_here = reversed(&b);
        // where the halves are furthest apart, which is their spacing; at the ends they meet
        let d = a.points.iter().map(|&p| dist(p, nearest(&b.points, p))).fold(0.0, f64::max);
        let median = (d - width(&a) / 2.0 - width(&b) / 2.0).max(MIN_MEDIAN_M);
        let gap = Lane { kind: LaneKind::Other, way: Way::Forward, width_m: median, hours: None };
        let b_on_left = side_of(&a.points, &b.points) > 0.0;
        let lanes: Vec<Lane> = if b_on_left {
            b_here.lanes.iter().cloned().chain([gap]).chain(a.lanes.iter().cloned()).collect()
        } else {
            a.lanes.iter().cloned().chain([gap]).chain(b_here.lanes.iter().cloned()).collect()
        };
        let points: Vec<P> = a
            .points
            .iter()
            .map(|&p| {
                let q = nearest(&b.points, p);
                ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0)
            })
            .collect();
        moved.push((b.to, a.from));
        moved.push((b.from, a.to));
        let mut osm_ways = a.osm_ways.clone();
        osm_ways.extend(b.osm_ways.iter().copied());
        merged.push(Road { id: a.id, osm_ways, name: a.name.clone(), highway: a.highway.clone(), from: a.from, to: a.to, lanes, points });
        gone[i] = true;
        gone[j] = true;
    }

    // each merged pair's ends become one node, in the middle
    for (from, into) in moved {
        let (Some(f), Some(t)) = (at(net, from), at(net, into)) else { continue };
        let mid = ((f.0 + t.0) / 2.0, (f.1 + t.1) / 2.0);
        for n in &mut net.nodes {
            if n.id == into {
                (n.x_m, n.y_m) = mid;
            }
        }
        if from != into {
            let gone_node = net.nodes.iter().position(|n| n.id == from);
            if let Some(g) = gone_node {
                let g = net.nodes.remove(g);
                if let Some(n) = net.nodes.iter_mut().find(|n| n.id == into) {
                    n.osm_nodes.extend(g.osm_nodes);
                    n.junction |= g.junction;
                    if n.control == crate::Control::None {
                        n.control = g.control;
                    }
                }
            }
            for r in &mut net.roads {
                if r.from == from {
                    r.from = into;
                }
                if r.to == from {
                    r.to = into;
                }
            }
            for r in &mut merged {
                if r.from == from {
                    r.from = into;
                }
                if r.to == from {
                    r.to = into;
                }
            }
        }
    }
    let mut roads: Vec<Road> = net.roads.iter().enumerate().filter(|(i, _)| !gone[*i]).map(|(_, r)| r.clone()).collect();
    roads.extend(merged);
    // a stub of the crossing street that ran between the two carriageways now joins a node to itself
    roads.retain(|r| r.from != r.to);
    roads.sort_by_key(|r| r.id);
    net.roads = roads;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Control, Node};

    fn node(id: u32, x_m: f64, y_m: f64) -> Node {
        Node { id, osm_nodes: vec![id as i64], x_m, y_m, junction: false, control: Control::None }
    }

    fn lane(kind: LaneKind, way: Way, width_m: f64) -> Lane {
        Lane { kind, way, width_m, hours: None }
    }

    /// A one-way carriageway of two lanes, with a sidewalk on its right: lanes left to right as seen along the road.
    fn oneway(id: u32, name: &str, from: u32, to: u32, points: Vec<P>) -> Road {
        Road {
            id,
            osm_ways: vec![id as i64 * 10],
            name: Some(name.into()),
            highway: "primary".into(),
            from,
            to,
            lanes: vec![lane(LaneKind::Driving, Way::Forward, 3.0), lane(LaneKind::Driving, Way::Forward, 3.0), lane(LaneKind::Sidewalk, Way::Forward, 2.0)],
            points,
        }
    }

    /// Boulevard: eastbound one way at y = -10 (the south), westbound at y = +10, both between x = 0 and x = 300,
    /// meeting at node 1 and node 2, with a street across at each end.
    fn shared_ends() -> Network {
        let mut west = oneway(2, "Boulevard", 2, 1, vec![(300.0, 0.0), (150.0, 10.0), (0.0, 0.0)]);
        west.points = vec![(300.0, 0.0), (150.0, 10.0), (0.0, 0.0)];
        let east = oneway(1, "Boulevard", 1, 2, vec![(0.0, 0.0), (150.0, -10.0), (300.0, 0.0)]);
        let cross = |id, from, to, y| Road {
            id,
            osm_ways: vec![id as i64],
            name: Some("Cross".into()),
            highway: "residential".into(),
            from,
            to,
            lanes: vec![lane(LaneKind::Driving, Way::Forward, 3.0), lane(LaneKind::Driving, Way::Backward, 3.0)],
            points: vec![(0.0, 0.0), (0.0, y)],
        };
        Network {
            left_hand: false,
            nodes: vec![node(1, 0.0, 0.0), node(2, 300.0, 0.0), node(3, 0.0, 100.0), node(4, 0.0, -100.0)],
            roads: vec![east, west, cross(3, 1, 3, 100.0), cross(4, 1, 4, -100.0)],
        }
    }

    #[test]
    fn two_opposite_carriageways_of_one_name_between_the_same_junctions_become_one_road() {
        let mut net = shared_ends();
        merge_dual_carriageways(&mut net);
        let boulevards: Vec<&Road> = net.roads.iter().filter(|r| r.name.as_deref() == Some("Boulevard")).collect();
        assert_eq!(boulevards.len(), 1);
        let b = boulevards[0];
        assert_eq!((b.from, b.to), (1, 2));
        assert_eq!(b.osm_ways, vec![10, 20]);
        assert_eq!(net.roads.len(), 3, "the two cross roads stay");
    }

    #[test]
    fn the_merged_road_has_the_median_between_the_carriageways_with_traffic_on_the_right_of_each() {
        let mut net = shared_ends();
        merge_dual_carriageways(&mut net);
        let b = net.roads.iter().find(|r| r.name.as_deref() == Some("Boulevard")).unwrap();
        // Along the merged road (west to east): the westbound half is on the left (north), the eastbound on the right (south).
        let kinds: Vec<(LaneKind, Way)> = b.lanes.iter().map(|l| (l.kind, l.way)).collect();
        use LaneKind::*;
        use Way::*;
        assert_eq!(
            kinds,
            vec![(Sidewalk, Backward), (Driving, Backward), (Driving, Backward), (Other, Forward), (Driving, Forward), (Driving, Forward), (Sidewalk, Forward)]
        );
        // centre lines 20 m apart at the middle; carriageways 8 m wide each: 20 - 4 - 4 = 12 m of median
        let median = b.lanes.iter().find(|l| l.kind == LaneKind::Other).unwrap();
        assert!((median.width_m - 12.0).abs() < 1.5, "{}", median.width_m);
        // the centre line runs between the two carriageways
        assert!(b.points.iter().all(|p| p.1.abs() < 10.5));
        assert!((b.points[1].1).abs() < 1.0, "{:?}", b.points);
    }

    #[test]
    fn on_the_other_side_of_the_map_the_halves_swap_sides() {
        // Left-hand traffic: the eastbound carriageway lies to the north.
        let mut net = shared_ends();
        for r in &mut net.roads {
            for p in &mut r.points {
                p.1 = -p.1;
            }
        }
        merge_dual_carriageways(&mut net);
        let b = net.roads.iter().find(|r| r.name.as_deref() == Some("Boulevard")).unwrap();
        let first_driving = b.lanes.iter().find(|l| l.kind == LaneKind::Driving).unwrap();
        assert_eq!(first_driving.way, Way::Forward, "traffic going east is now on the left");
    }

    #[test]
    fn carriageways_that_end_at_two_junctions_close_together_are_joined_and_the_street_between_goes() {
        // the crossing street meets the eastbound half at node 1 and the westbound half at node 5, 20 m north;
        // the stub between them is road 9
        let mut net = shared_ends();
        net.nodes.push(node(5, 0.0, 20.0));
        net.nodes.push(node(6, 300.0, 20.0));
        net.nodes.push(node(7, 300.0, 0.0));
        net.roads.iter_mut().find(|r| r.id == 2).unwrap().to = 5;
        net.roads.iter_mut().find(|r| r.id == 2).unwrap().from = 6;
        net.roads.iter_mut().find(|r| r.id == 2).unwrap().points = vec![(300.0, 20.0), (150.0, 30.0), (0.0, 20.0)];
        net.roads.iter_mut().find(|r| r.id == 1).unwrap().points = vec![(0.0, 0.0), (150.0, -10.0), (300.0, 0.0)];
        net.roads.push(Road {
            id: 9,
            osm_ways: vec![9],
            name: Some("Cross".into()),
            highway: "residential".into(),
            from: 1,
            to: 5,
            lanes: vec![lane(LaneKind::Driving, Way::Forward, 3.0), lane(LaneKind::Driving, Way::Backward, 3.0)],
            points: vec![(0.0, 0.0), (0.0, 20.0)],
        });
        merge_dual_carriageways(&mut net);
        assert_eq!(net.roads.iter().filter(|r| r.name.as_deref() == Some("Boulevard")).count(), 1);
        assert!(net.roads.iter().all(|r| r.id != 9), "the stub between the halves is gone");
        assert!(net.roads.iter().all(|r| r.from != r.to));
        // and no road points at a node that is no longer there
        assert!(net.roads.iter().all(|r| net.nodes.iter().any(|n| n.id == r.from) && net.nodes.iter().any(|n| n.id == r.to)));
    }

    #[test]
    fn roads_that_are_not_a_divided_street_are_left_alone() {
        let same = |f: &dyn Fn(&mut Network)| {
            let mut net = shared_ends();
            f(&mut net);
            let before = net.clone();
            merge_dual_carriageways(&mut net);
            assert_eq!(net, before);
        };
        // another name
        same(&|n| n.roads[1].name = Some("Other".into()));
        // one of them two-way
        same(&|n| n.roads[1].lanes = vec![lane(LaneKind::Driving, Way::Forward, 3.0), lane(LaneKind::Driving, Way::Backward, 3.0)]);
        // both the same way
        same(&|n| {
            n.roads[1].from = 1;
            n.roads[1].to = 2;
            n.roads[1].points.reverse();
        });
        // a street with no name
        same(&|n| {
            n.roads[0].name = None;
            n.roads[1].name = None;
        });
        // too far apart to be one street
        same(&|n| n.roads[1].points = vec![(300.0, 200.0), (150.0, 200.0), (0.0, 200.0)]);
    }

    #[test]
    fn a_network_with_nothing_to_merge_is_untouched() {
        let mut net = Network::default();
        merge_dual_carriageways(&mut net);
        assert_eq!(net, Network::default());
    }
}
