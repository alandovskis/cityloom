//! Roads divided by a median: two one-way carriageways of one street, running side by side
//! in opposite directions, are one street with a median between its two halves.

use std::collections::HashMap;

use crate::importer::{Direction, Lane, LaneKind, Local, Network, NodeId, Point, Road, RoadId, Zone};

/// The widest a median can be, from centre line to centre line of the carriageways, for them to be one street.
const MAX_APART_M: f64 = 60.0;
/// A median narrower than this is drawn at least this wide.
const MIN_MEDIAN_M: f64 = 1.0;

fn dist(a: Point, b: Point) -> f64 {
    a.distance(b)
}

/// The nearest point of the polyline `line` to `p`.
fn nearest(line: &[Point], p: Point) -> Point {
    let mut best = (f64::MAX, p);
    for w in line.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (dx, dy) = (b.x_m() - a.x_m(), b.y_m() - a.y_m());
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 { 0.0 } else { (((p.x_m() - a.x_m()) * dx + (p.y_m() - a.y_m()) * dy) / len2).clamp(0.0, 1.0) };
        let q = Point::new(a.x_m() + t * dx, a.y_m() + t * dy);
        let d = dist(p, q);
        if d < best.0 {
            best = (d, q);
        }
    }
    best.1
}

/// The road turned round (its lanes seen from the other end) if its driving lanes run against it, so
/// that they run along it; None where it is not one-way, has no driving lane, or is divided into zones (turning a
/// zoned road round would move its zones' starts, and a dual carriageway is one cross-section).
fn along(road: &Road) -> Option<Road> {
    if road.zones.len() != 1 {
        return None;
    }
    let driving: Vec<Direction> = road.base_lanes().iter().filter(|l| l.kind == LaneKind::Driving).map(|l| l.direction).collect();
    let first = *driving.first()?;
    if driving.iter().any(|w| *w != first) {
        return None;
    }
    Some(if first == Direction::Forward { road.clone() } else { reversed(road) })
}

fn reversed(road: &Road) -> Road {
    let mut r = road.clone();
    std::mem::swap(&mut r.edge.from, &mut r.edge.to);
    r.points.reverse();
    r.base_lanes_mut().reverse();
    for l in r.base_lanes_mut() {
        l.direction = match l.direction {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
            Direction::Both => Direction::Both,
        };
    }
    r
}

fn width(road: &Road) -> f64 {
    road.base_lanes().iter().map(|l| l.width_m).sum()
}

fn length(line: &[Point]) -> f64 {
    line.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// How far apart two carriageways run, on average, measured from `a`'s points; None when they do not run beside each other.
fn apart(a: &[Point], b: &[Point]) -> Option<f64> {
    let d: Vec<f64> = a.iter().map(|&p| dist(p, nearest(b, p))).collect();
    let mean = d.iter().sum::<f64>() / d.len() as f64;
    (mean <= MAX_APART_M && d.iter().all(|x| *x <= MAX_APART_M * 1.5)).then_some(mean)
}

/// Which side of `a`'s direction of travel `b` lies on: positive for the left.
fn side_of(a: &[Point], b: &[Point]) -> f64 {
    let m = (a.len() - 1) / 2;
    let (p, q) = (a[m], a[m + 1]);
    let n = nearest(b, p);
    (q.x_m() - p.x_m()) * (n.y_m() - p.y_m()) - (q.y_m() - p.y_m()) * (n.x_m() - p.x_m())
}

/// Joins each pair of one-way carriageways of one street into a single road with a median.
///
/// A pair is two one-way roads of the same name and kind of street that run in opposite directions, begin
/// and end near each other (at one junction or at two close together, the crossing street's stub between them)
/// and run beside each other. Their ends become one node each, in the middle; a road left joining a node to itself
/// by that (the crossing street's stub) is dropped.
pub fn merge_dual_carriageways(net: &mut Network) {
    // the first junction of an id wins, as a linear search would find it
    let mut index: HashMap<NodeId, usize> = HashMap::with_capacity(net.junctions.len());
    for (k, n) in net.junctions.iter().enumerate() {
        index.entry(n.id).or_insert(k);
    }
    let positions: HashMap<NodeId, Point<Local>> = index.iter().map(|(id, &k)| (*id, net.junctions[k].position)).collect();
    let at = |id: NodeId| positions.get(&id).copied();
    let oriented: Vec<Option<Road>> = net.roads.iter().map(along).collect();
    let mut taken = vec![false; net.roads.len()];
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    // the roads that can be merged, by name, each group in ascending index order
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (k, o) in oriented.iter().enumerate() {
        if let Some(r) = o {
            by_name.entry(r.name.as_str()).or_default().push(k);
        }
    }
    for i in 0..net.roads.len() {
        let Some(a) = &oriented[i] else { continue };
        let mut best: Option<(f64, usize)> = None;
        for &j in &by_name[a.name.as_str()] {
            let Some(b) = &oriented[j] else { continue };
            if i == j || taken[j] || a.road_type != b.road_type {
                continue;
            }
            let (Some(a0), Some(a1), Some(b0), Some(b1)) = (at(a.edge.from), at(a.edge.to), at(b.edge.from), at(b.edge.to)) else {
                continue;
            };
            // b runs the other way: it starts where a ends and ends where a starts
            if dist(a1, b0) > MAX_APART_M || dist(a0, b1) > MAX_APART_M {
                continue;
            }
            if length(&a.points) < 1.0 || b.points.len() < 2 || a.points.len() < 2 {
                continue;
            }
            if let Some(d) = apart(&a.points, &b.points)
                && best.is_none_or(|(bd, _)| d < bd)
            {
                best = Some((d, j));
            }
        }
        if let Some((_, j)) = best
            && !taken[i]
            && oriented[j].as_ref().is_some()
        {
            taken[i] = true;
            taken[j] = true;
            pairs.push((i, j));
        }
    }
    if pairs.is_empty() {
        return;
    }

    let mut moved: Vec<(NodeId, NodeId)> = Vec::new(); // (node, the node it is merged into)
    let mut kept_as: Vec<(RoadId, RoadId)> = Vec::new(); // (a road that is gone, the road it became)
    let mut merged: Vec<Road> = Vec::new();
    let mut gone = vec![false; net.roads.len()];
    for (i, j) in pairs {
        let (a, b) = (oriented[i].clone().unwrap(), oriented[j].clone().unwrap());
        // `b` seen along `a`: driving lanes run against it
        kept_as.push((b.edge.id, a.edge.id));
        let b_here = reversed(&b);
        // where the halves are furthest apart, which is their spacing; at the ends they meet
        let d = a.points.iter().map(|&p| dist(p, nearest(&b.points, p))).fold(0.0, f64::max);
        let median = (d - width(&a) / 2.0 - width(&b) / 2.0).max(MIN_MEDIAN_M);
        let gap = Lane { kind: LaneKind::Other, direction: Direction::Forward, width_m: median, buffer_type: None, windows: vec![], shape: vec![] };
        let b_on_left = side_of(&a.points, &b.points) > 0.0;
        let lanes: Vec<Lane> = if b_on_left {
            b_here.base_lanes().iter().cloned().chain([gap]).chain(a.base_lanes().iter().cloned()).collect()
        } else {
            a.base_lanes().iter().cloned().chain([gap]).chain(b_here.base_lanes().iter().cloned()).collect()
        };
        let points: Vec<Point> = a
            .points
            .iter()
            .map(|&p| {
                let q = nearest(&b.points, p);
                Point::new((p.x_m() + q.x_m()) / 2.0, (p.y_m() + q.y_m()) / 2.0)
            })
            .collect();
        moved.push((b.edge.to, a.edge.from));
        moved.push((b.edge.from, a.edge.to));
        let mut provenance = a.provenance.clone();
        provenance.extend(b.provenance.clone());
        merged.push(Road {
            edge: a.edge,
            provenance,
            name: a.name.clone(),
            road_type: a.road_type,
            zones: vec![Zone { speed_kmh: lower(a.base_speed_kmh(), b.base_speed_kmh()), ..Zone::whole(lanes) }],
            points,
        });
        gone[i] = true;
        gone[j] = true;
    }

    // each merged pair's ends become one node, in the middle; a node merged away is dropped and
    // the roads' ends that named it are rewritten once, after all the moves
    let mut removed = vec![false; net.junctions.len()];
    let mut into_of: HashMap<NodeId, NodeId> = HashMap::new(); // (a node merged away, the node it went into)
    for (from, into) in moved {
        let (Some(&fi), Some(&ti)) = (index.get(&from), index.get(&into)) else { continue };
        if removed[fi] || removed[ti] {
            continue;
        }
        let (f, t) = (net.junctions[fi].position, net.junctions[ti].position);
        net.junctions[ti].position = Point::<Local>::new((f.x_m() + t.x_m()) / 2.0, (f.y_m() + t.y_m()) / 2.0);
        if from != into {
            removed[fi] = true;
            let g = net.junctions[fi].clone();
            let n = &mut net.junctions[ti];
            n.provenance.extend(g.provenance);
            n.junction |= g.junction;
            if n.control == crate::importer::Control::None {
                n.control = g.control;
            }
            into_of.insert(from, into);
        }
    }
    let mut k = 0;
    net.junctions.retain(|_| {
        k += 1;
        !removed[k - 1]
    });
    if !into_of.is_empty() {
        // a node can be merged into one that is itself merged away later: follow the chain (it has no cycle)
        let resolve = |mut id: NodeId| {
            while let Some(&next) = into_of.get(&id) {
                id = next;
            }
            id
        };
        for r in net.roads.iter_mut().chain(merged.iter_mut()) {
            r.edge.from = resolve(r.edge.from);
            r.edge.to = resolve(r.edge.to);
        }
    }
    let mut roads: Vec<Road> = net.roads.iter().enumerate().filter(|(i, _)| !gone[*i]).map(|(_, r)| r.clone()).collect();
    roads.extend(merged);
    // a stub of the crossing street that ran between the two carriageways now joins a node to itself
    roads.retain(|r| r.edge.from != r.edge.to);
    roads.sort_by_key(|r| r.edge.id);
    net.roads = roads;
    // a priority road that was merged is the road it became; one that is gone is dropped
    for j in &mut net.junctions {
        if let crate::importer::Control::Priority(ids) = &mut j.control {
            let mut kept: Vec<RoadId> = Vec::new();
            for id in ids.iter().map(|id| kept_as.iter().find(|(gone, _)| gone == id).map_or(*id, |(_, now)| *now)) {
                if !kept.contains(&id) {
                    kept.push(id);
                }
            }
            *ids = kept;
        }
    }
    drop_unknown_priority(net);
}

/// Takes from every priority control the roads the network does not have, so it never names one that is gone.
fn drop_unknown_priority(net: &mut Network) {
    let roads = &net.roads;
    for j in &mut net.junctions {
        if let crate::importer::Control::Priority(ids) = &mut j.control {
            ids.retain(|id| roads.iter().any(|r| r.edge.id == *id));
        }
    }
}

/// The lower of two known speeds, else the one known, else none.
fn lower(a: Option<u16>, b: Option<u16>) -> Option<u16> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importer::{Control, Coordinate, Edge, Junction, NodeRef, Provenance, RoadType, WayRef};

    fn node(id: u32, x_m: f64, y_m: f64) -> Junction {
        Junction {
            id: NodeId(id),
            provenance: Provenance::of_nodes([NodeRef::with_version(id as i64, id as i32 + 100)]),
            position: Point::new(x_m, y_m),
            junction: false,
            control: Control::None,
        }
    }

    #[test]
    fn merged_carriageways_keep_the_lower_known_speed() {
        assert_eq!(lower(Some(50), Some(40)), Some(40));
        assert_eq!(lower(None, Some(40)), Some(40));
        assert_eq!(lower(Some(50), None), Some(50));
        assert_eq!(lower(None, None), None);
    }

    fn lane(kind: LaneKind, way: Direction, width_m: f64) -> Lane {
        Lane { kind, direction: way, width_m, buffer_type: None, windows: vec![], shape: vec![] }
    }

    /// A one-way carriageway of two lanes, with a sidewalk on its right: lanes left to right as seen along the road.
    fn oneway(id: u32, name: &str, from: u32, to: u32, points: Vec<Point>) -> Road {
        Road {
            edge: Edge { id: RoadId(id), from: NodeId(from), to: NodeId(to) },
            provenance: Provenance::of_ways([WayRef::with_version(id as i64 * 10, id as i32 + 50)]),
            name: name.into(),
            road_type: RoadType::Arterial,
            zones: vec![Zone::whole(vec![
                lane(LaneKind::Driving, Direction::Forward, 3.0),
                lane(LaneKind::Driving, Direction::Forward, 3.0),
                lane(LaneKind::Sidewalk, Direction::Forward, 2.0),
            ])],
            points,
        }
    }

    /// Boulevard: eastbound one way at y = -10 (the south), westbound at y = +10, both between x = 0 and x = 300,
    /// meeting at node 1 and node 2, with a street across at each end.
    fn shared_ends() -> Network {
        let mut west = oneway(2, "Boulevard", 2, 1, vec![Point::new(300.0, 0.0), Point::new(150.0, 10.0), Point::new(0.0, 0.0)]);
        west.points = vec![Point::new(300.0, 0.0), Point::new(150.0, 10.0), Point::new(0.0, 0.0)];
        let east = oneway(1, "Boulevard", 1, 2, vec![Point::new(0.0, 0.0), Point::new(150.0, -10.0), Point::new(300.0, 0.0)]);
        let cross = |id: u32, from: u32, to: u32, y| Road {
            edge: Edge { id: RoadId(id), from: NodeId(from), to: NodeId(to) },
            provenance: Provenance::of_ways([WayRef::new(id as i64)]),
            name: "Cross".into(),
            road_type: RoadType::Local,
            zones: vec![Zone::whole(vec![lane(LaneKind::Driving, Direction::Forward, 3.0), lane(LaneKind::Driving, Direction::Backward, 3.0)])],
            points: vec![Point::new(0.0, 0.0), Point::new(0.0, y)],
        };
        Network {
            left_hand: false,
            corridors: vec![],
            junctions: vec![node(1, 0.0, 0.0), node(2, 300.0, 0.0), node(3, 0.0, 100.0), node(4, 0.0, -100.0)],
            roads: vec![east, west, cross(3, 1, 3, 100.0), cross(4, 1, 4, -100.0)],
        }
    }

    #[test]
    fn a_merged_road_keeps_the_versions_of_the_ways_it_was_made_from() {
        let mut net = shared_ends();
        merge_dual_carriageways(&mut net);
        let b = net.roads.iter().find(|r| r.name == "Boulevard").unwrap();
        assert_eq!(b.provenance.ways().collect::<Vec<_>>(), [WayRef::with_version(10, 51), WayRef::with_version(20, 52)]);
    }

    #[test]
    fn a_priority_road_that_was_merged_is_the_road_it_became() {
        let mut net = shared_ends();
        net.junctions[2].control = Control::Priority(vec![RoadId(2), RoadId(1), RoadId(3), RoadId(99)]);
        merge_dual_carriageways(&mut net);
        let merged = net.roads.iter().find(|r| r.name == "Boulevard").unwrap().edge.id;
        assert_eq!(net.junctions[2].control, Control::Priority(vec![merged, RoadId(3)]), "the halves are one road, and one that is not there is dropped");
    }

    #[test]
    fn a_road_divided_into_zones_is_not_merged() {
        let mut net = shared_ends();
        let east = net.roads.iter_mut().find(|r| r.edge.id == RoadId(1)).unwrap();
        let lanes = east.base_lanes().to_vec();
        east.zones.push(Zone { from: Coordinate::new(100.0), speed_kmh: None, lanes });
        let before = net.clone();
        merge_dual_carriageways(&mut net);
        assert_eq!(net, before);
    }

    #[test]
    fn two_opposite_carriageways_of_one_name_between_the_same_junctions_become_one_road() {
        let mut net = shared_ends();
        merge_dual_carriageways(&mut net);
        let boulevards: Vec<&Road> = net.roads.iter().filter(|r| r.name == "Boulevard").collect();
        assert_eq!(boulevards.len(), 1);
        let b = boulevards[0];
        assert_eq!((b.edge.from, b.edge.to), (NodeId(1), NodeId(2)));
        assert!(b.has_way(10) && b.has_way(20));
        assert_eq!(net.roads.len(), 3, "the two cross roads stay");
    }

    #[test]
    fn the_merged_road_has_the_median_between_the_carriageways_with_traffic_on_the_right_of_each() {
        let mut net = shared_ends();
        merge_dual_carriageways(&mut net);
        let b = net.roads.iter().find(|r| r.name == "Boulevard").unwrap();
        // Along the merged road (west to east): the westbound half is on the left (north), the eastbound on the right (south).
        let kinds: Vec<(LaneKind, Direction)> = b.base_lanes().iter().map(|l| (l.kind, l.direction)).collect();
        use Direction::*;
        use LaneKind::*;
        assert_eq!(
            kinds,
            vec![(Sidewalk, Backward), (Driving, Backward), (Driving, Backward), (Other, Forward), (Driving, Forward), (Driving, Forward), (Sidewalk, Forward)]
        );
        // centre lines 20 m apart at the middle; carriageways 8 m wide each: 20 - 4 - 4 = 12 m of median
        let median = b.base_lanes().iter().find(|l| l.kind == LaneKind::Other).unwrap();
        assert!((median.width_m - 12.0).abs() < 1.5, "{}", median.width_m);
        // the centre line runs between the two carriageways
        assert!(b.points.iter().all(|p| p.y_m().abs() < 10.5));
        assert!(b.points[1].y_m().abs() < 1.0, "{:?}", b.points);
    }

    #[test]
    fn on_the_other_side_of_the_map_the_halves_swap_sides() {
        // Left-hand traffic: the eastbound carriageway lies to the north.
        let mut net = shared_ends();
        for r in &mut net.roads {
            for p in &mut r.points {
                p.y = Coordinate::new(-p.y_m());
            }
        }
        merge_dual_carriageways(&mut net);
        let b = net.roads.iter().find(|r| r.name == "Boulevard").unwrap();
        let first_driving = b.base_lanes().iter().find(|l| l.kind == LaneKind::Driving).unwrap();
        assert_eq!(first_driving.direction, Direction::Forward, "traffic going east is now on the left");
    }

    #[test]
    fn carriageways_that_end_at_two_junctions_close_together_are_joined_and_the_street_between_goes() {
        // the crossing street meets the eastbound half at node 1 and the westbound half at node 5, 20 m north;
        // the stub between them is road 9
        let mut net = shared_ends();
        net.junctions.push(node(5, 0.0, 20.0));
        net.junctions.push(node(6, 300.0, 20.0));
        net.junctions.push(node(7, 300.0, 0.0));
        net.roads.iter_mut().find(|r| r.edge.id == RoadId(2)).unwrap().edge.to = NodeId(5);
        net.roads.iter_mut().find(|r| r.edge.id == RoadId(2)).unwrap().edge.from = NodeId(6);
        net.roads.iter_mut().find(|r| r.edge.id == RoadId(2)).unwrap().points = vec![Point::new(300.0, 20.0), Point::new(150.0, 30.0), Point::new(0.0, 20.0)];
        net.roads.iter_mut().find(|r| r.edge.id == RoadId(1)).unwrap().points = vec![Point::new(0.0, 0.0), Point::new(150.0, -10.0), Point::new(300.0, 0.0)];
        net.roads.push(Road {
            edge: Edge { id: RoadId(9), from: NodeId(1), to: NodeId(5) },
            provenance: Provenance::of_ways([WayRef::new(9)]),
            name: "Cross".into(),
            road_type: RoadType::Local,
            zones: vec![Zone::whole(vec![lane(LaneKind::Driving, Direction::Forward, 3.0), lane(LaneKind::Driving, Direction::Backward, 3.0)])],
            points: vec![Point::new(0.0, 0.0), Point::new(0.0, 20.0)],
        });
        merge_dual_carriageways(&mut net);
        assert_eq!(net.roads.iter().filter(|r| r.name == "Boulevard").count(), 1);
        assert!(net.roads.iter().all(|r| r.edge.id != RoadId(9)), "the stub between the halves is gone");
        // the node the two ends became keeps the version of each OSM node it stands for
        let joined = net.junctions.iter().find(|n| n.provenance.has_node(1)).unwrap();
        assert!(joined.provenance.has_node(5), "{joined:?}");
        assert_eq!(joined.provenance.nodes().collect::<Vec<_>>(), [NodeRef::with_version(1, 101), NodeRef::with_version(5, 105)]);
        assert!(net.roads.iter().all(|r| r.edge.from != r.edge.to));
        // and no road points at a node that is no longer there
        assert!(net.roads.iter().all(|r| net.junctions.iter().any(|n| n.id == r.edge.from) && net.junctions.iter().any(|n| n.id == r.edge.to)));
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
        same(&|n| n.roads[1].name = "Other".into());
        // one of them two-way
        same(&|n| *n.roads[1].base_lanes_mut() = vec![lane(LaneKind::Driving, Direction::Forward, 3.0), lane(LaneKind::Driving, Direction::Backward, 3.0)]);
        // both the same way
        same(&|n| {
            n.roads[1].edge.from = NodeId(1);
            n.roads[1].edge.to = NodeId(2);
            n.roads[1].points.reverse();
        });
        // too far apart to be one street
        same(&|n| n.roads[1].points = vec![Point::new(300.0, 200.0), Point::new(150.0, 200.0), Point::new(0.0, 200.0)]);
    }

    #[test]
    fn merging_twenty_thousand_roads_with_their_own_names_takes_under_two_seconds() {
        const N: u32 = 20_000;
        let mut net = Network::default();
        for i in 0..N {
            let x = i as f64 * 1000.0;
            net.junctions.push(node(2 * i, x, 0.0));
            net.junctions.push(node(2 * i + 1, x + 100.0, 0.0));
            net.roads.push(oneway(i, &format!("Street {i}"), 2 * i, 2 * i + 1, vec![Point::new(x, 0.0), Point::new(x + 100.0, 0.0)]));
        }
        let start = std::time::Instant::now();
        merge_dual_carriageways(&mut net);
        let took = start.elapsed();
        assert_eq!(net.roads.len(), N as usize);
        assert!(took < std::time::Duration::from_secs(2), "took {took:?}");
    }

    #[test]
    fn a_network_with_nothing_to_merge_is_untouched() {
        let mut net = Network::default();
        merge_dual_carriageways(&mut net);
        assert_eq!(net, Network::default());
    }
}
