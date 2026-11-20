//! Roads split at a plain connection: the pieces of one block are one road, each piece a zone.

use std::collections::{HashMap, HashSet};

use super::merge::{drop_unknown_priority, reversed};
use crate::importer::{Control, Coordinate, Edge, Network, NodeId, Point, Provenance, Road, RoadId, Zone};

/// Joins each run of roads that meet end to end at plain connections into one road, each piece a zone.
///
/// A node joins two roads when it is one of `through` (the nodes where only two ways of the data meet, with no
/// signals or signs) and is the end of exactly two roads of the network, of the same name and the same type, each of
/// one zone. A run of roads joined that way becomes one road between the run's two outer nodes, with the id and
/// direction of its lowest id; the nodes it joined are dropped, and a priority that named a piece names the road. A
/// run that closes on itself, or whose two outer nodes are one node, is left alone.
pub fn merge_blocks(net: &mut Network, through: &HashSet<NodeId>) {
    let joins = joins(net, through);
    if joins.is_empty() {
        return;
    }
    let roads = &net.roads;
    let mut seen = vec![false; roads.len()];
    let mut runs: Vec<Vec<(usize, bool)>> = Vec::new();
    for start in 0..roads.len() {
        let edge = roads[start].edge;
        if seen[start] || !(joins.contains_key(&edge.from) || joins.contains_key(&edge.to)) {
            continue;
        }
        seen[start] = true;
        // onwards from its `to` end, then back from its `from` end; a ring comes back to it
        let Some(after) = walk(&joins, roads, start, edge.to, &mut seen) else { continue };
        let Some(before) = walk(&joins, roads, start, edge.from, &mut seen) else { continue };
        let mut run: Vec<(usize, bool)> = before.into_iter().rev().map(|(k, entered_at_from)| (k, !entered_at_from)).collect();
        run.push((start, true));
        run.extend(after);
        let (first, last) = (run[0], run[run.len() - 1]);
        if start_of(&roads[first.0], first.1) == start_of(&roads[last.0], !last.1) {
            continue;
        }
        // the road keeps the direction of its lowest id
        let lowest = run.iter().min_by_key(|(k, _)| roads[*k].edge.id).unwrap();
        if !lowest.1 {
            run.reverse();
            run.iter_mut().for_each(|(_, along)| *along = !*along);
        }
        runs.push(run);
    }
    if runs.is_empty() {
        return;
    }

    let mut slots: Vec<Option<Road>> = std::mem::take(&mut net.roads).into_iter().map(Some).collect();
    let mut gone_nodes: HashSet<NodeId> = HashSet::new();
    let mut kept_as: HashMap<RoadId, RoadId> = HashMap::new(); // (a piece, the road it is part of)
    let mut merged: Vec<Road> = Vec::with_capacity(runs.len());
    for run in runs {
        let pieces: Vec<Road> = run
            .iter()
            .map(|&(k, along)| {
                let piece = slots[k].take().expect("a road is in one run");
                if along { piece } else { reversed(&piece) }
            })
            .collect();
        gone_nodes.extend(pieces.iter().skip(1).map(|p| p.edge.from));
        let ids: Vec<RoadId> = pieces.iter().map(|p| p.edge.id).collect();
        let road = joined(pieces);
        kept_as.extend(ids.into_iter().map(|id| (id, road.edge.id)));
        merged.push(road);
    }
    net.roads = slots.into_iter().flatten().chain(merged).collect();
    net.roads.sort_by_key(|r| r.edge.id);
    net.junctions.retain(|j| !gone_nodes.contains(&j.id));
    for j in &mut net.junctions {
        if let Control::Priority(ids) = &mut j.control {
            let mut kept: Vec<RoadId> = Vec::with_capacity(ids.len());
            for id in ids.iter().map(|id| kept_as.get(id).copied().unwrap_or(*id)) {
                if !kept.contains(&id) {
                    kept.push(id);
                }
            }
            *ids = kept;
        }
    }
    drop_unknown_priority(net);
}

/// The nodes of `through` that join two roads, each with the two roads (by index) whose ends meet there.
fn joins(net: &Network, through: &HashSet<NodeId>) -> HashMap<NodeId, [usize; 2]> {
    let mut ends: HashMap<NodeId, Vec<usize>> = HashMap::new();
    for (k, r) in net.roads.iter().enumerate() {
        ends.entry(r.edge.from).or_default().push(k);
        ends.entry(r.edge.to).or_default().push(k);
    }
    ends.into_iter()
        .filter_map(|(id, at)| {
            let &[a, b] = at.as_slice() else { return None };
            let (ra, rb) = (&net.roads[a], &net.roads[b]);
            let alike = a != b && ra.name == rb.name && ra.road_type == rb.road_type && ra.zones.len() == 1 && rb.zones.len() == 1;
            (through.contains(&id) && alike).then_some((id, [a, b]))
        })
        .collect()
}

/// The roads met going out from road `start` through its end `at`, each with whether it was entered at its `from`
/// end; None when the way comes back to `start`, a ring.
fn walk(joins: &HashMap<NodeId, [usize; 2]>, roads: &[Road], start: usize, mut at: NodeId, seen: &mut [bool]) -> Option<Vec<(usize, bool)>> {
    let mut found = Vec::new();
    let mut last = start;
    while let Some(&[a, b]) = joins.get(&at) {
        let next = if a == last { b } else { a };
        if next == start {
            return None;
        }
        seen[next] = true;
        let edge = roads[next].edge;
        let entered_at_from = edge.from == at;
        found.push((next, entered_at_from));
        at = if entered_at_from { edge.to } else { edge.from };
        last = next;
    }
    Some(found)
}

/// The node a road starts at when it is taken `along` its own direction, or against it.
fn start_of(road: &Road, along: bool) -> NodeId {
    if along { road.edge.from } else { road.edge.to }
}

/// The one road the pieces of a run make, each piece already turned to run along it: their lines in order, a zone
/// for each (one for neighbours of the same lanes and speed), and the ways of all of them.
fn joined(pieces: Vec<Road>) -> Road {
    let lowest = pieces.iter().min_by_key(|p| p.edge.id).expect("a run has a piece");
    let edge = Edge { id: lowest.edge.id, from: pieces[0].edge.from, to: pieces[pieces.len() - 1].edge.to };
    let (name, road_type) = (lowest.name.clone(), lowest.road_type);
    let mut points: Vec<Point> = Vec::new();
    let mut zones: Vec<Zone> = Vec::with_capacity(pieces.len());
    let mut provenance = Provenance::default();
    let mut length = 0.0;
    for piece in pieces {
        let mut line = piece.points.as_slice();
        if let (Some(&end), Some(&first)) = (points.last(), line.first()) {
            if end == first {
                line = &line[1..];
            } else {
                length += end.distance(first);
            }
        }
        let from = length;
        length += piece.length_m();
        points.extend_from_slice(line);
        let zone = piece.zones.into_iter().next().expect("a piece has one zone");
        if !zones.last().is_some_and(|z| z.lanes == zone.lanes && z.speed_kmh == zone.speed_kmh) {
            zones.push(Zone { from: Coordinate::new(from), ..zone });
        }
        let new: Vec<_> = piece.provenance.refs.into_iter().filter(|r| !provenance.refs.contains(r)).collect();
        provenance.extend(Provenance { refs: new });
    }
    Road { edge, provenance, name, road_type, zones, points }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::importer::{
        Control, Direction, Edge, Junction, Lane, LaneKind, Network, NodeId, NodeRef, Point, Provenance, Road, RoadId, RoadType, WayRef, Zone,
    };

    fn node(id: u32, x_m: f64, y_m: f64) -> Junction {
        Junction {
            id: NodeId(id),
            provenance: Provenance::of_nodes([NodeRef::with_version(id as i64, id as i32 + 100)]),
            position: Point::new(x_m, y_m),
            junction: false,
            control: Control::None,
        }
    }

    /// The nodes the data says only two ways meet at, with no signals or signs.
    fn through(ids: &[u32]) -> HashSet<NodeId> {
        ids.iter().map(|&id| NodeId(id)).collect()
    }

    fn lane(kind: LaneKind, way: Direction) -> Lane {
        Lane { kind, direction: way, width_m: 3.0, buffer_type: None, windows: vec![], shape: vec![] }
    }

    /// `n` driving lanes, half each way, between two sidewalks: left to right as seen along the road.
    fn driving(n: usize) -> Vec<Lane> {
        let mut lanes = vec![lane(LaneKind::Sidewalk, Direction::Backward)];
        lanes.extend((0..n / 2).map(|_| lane(LaneKind::Driving, Direction::Backward)));
        lanes.extend((0..n - n / 2).map(|_| lane(LaneKind::Driving, Direction::Forward)));
        lanes.push(lane(LaneKind::Sidewalk, Direction::Forward));
        lanes
    }

    fn two_way(id: u32, name: &str, from: u32, to: u32, points: &[(f64, f64)], lanes: Vec<Lane>, speed: Option<u16>) -> Road {
        Road {
            edge: Edge { id: RoadId(id), from: NodeId(from), to: NodeId(to) },
            provenance: Provenance::of_ways([WayRef::with_version(id as i64 * 10, id as i32 + 50)]),
            name: name.into(),
            road_type: RoadType::Local,
            zones: vec![Zone { speed_kmh: speed, ..Zone::whole(lanes) }],
            points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        }
    }

    /// A street across at `node`, from `x_m`, ending at `end` to the north.
    fn cross(id: u32, node: u32, end: u32, x_m: f64) -> Road {
        two_way(id, "Cross", node, end, &[(x_m, 0.0), (x_m, 100.0)], driving(2), None)
    }

    /// Main street from node 1 (x 0) to node 3 (x 250), in two pieces that meet at node 2 (x 100), a plain
    /// connection (`through(&[2])`); Cross streets at 1 and at 3 make those junctions. Road 10 has 2 driving lanes at 30 km/h, road 11
    /// has 4 at 40.
    fn block() -> Network {
        Network {
            left_hand: false,
            corridors: vec![],
            junctions: vec![node(1, 0.0, 0.0), node(2, 100.0, 0.0), node(3, 250.0, 0.0), node(4, 0.0, 100.0), node(5, 250.0, 100.0)],
            roads: vec![
                two_way(10, "Main", 1, 2, &[(0.0, 0.0), (100.0, 0.0)], driving(2), Some(30)),
                two_way(11, "Main", 2, 3, &[(100.0, 0.0), (250.0, 0.0)], driving(4), Some(40)),
                cross(20, 1, 4, 0.0),
                cross(21, 3, 5, 250.0),
            ],
        }
    }

    fn main_roads(net: &Network) -> Vec<&Road> {
        net.roads.iter().filter(|r| r.name == "Main").collect()
    }

    fn starts(road: &Road) -> Vec<f64> {
        road.zones.iter().map(|z| z.from.metres()).collect()
    }

    #[test]
    fn two_pieces_of_one_block_are_one_road_of_two_zones() {
        let mut net = block();
        merge_blocks(&mut net, &through(&[2]));
        let main = main_roads(&net);
        assert_eq!(main.len(), 1, "{:?}", net.roads);
        let m = main[0];
        assert_eq!(m.edge, Edge { id: RoadId(10), from: NodeId(1), to: NodeId(3) });
        assert_eq!(m.zones.len(), 2);
        assert_eq!(m.zones[1].from.metres(), 100.0);
        assert_eq!((m.zones[0].speed_kmh, m.zones[1].speed_kmh), (Some(30), Some(40)));
        assert_eq!((m.zones[0].lanes.clone(), m.zones[1].lanes.clone()), (driving(2), driving(4)));
        assert_eq!(m.points, [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(250.0, 0.0)]);
        assert!(net.junctions.iter().all(|j| j.id != NodeId(2)), "the connection is gone");
        assert_eq!(m.provenance.ways().collect::<Vec<_>>(), [WayRef::with_version(100, 60), WayRef::with_version(110, 61)]);
        assert_eq!(net.roads.iter().map(|r| r.edge.id.0).collect::<Vec<_>>(), [10, 20, 21], "sorted by id");
    }

    #[test]
    fn a_chain_of_three_pieces_is_one_road_of_three_zones() {
        let mut net = block();
        // a third piece, from 3 on to node 6 (x 400), where another Cross street makes a junction; 3 is a connection
        net.roads.retain(|r| r.edge.id != RoadId(21));
        net.junctions.push(node(6, 400.0, 0.0));
        net.junctions.push(node(7, 400.0, 100.0));
        net.roads.push(two_way(12, "Main", 3, 6, &[(250.0, 0.0), (400.0, 0.0)], driving(2), Some(50)));
        net.roads.push(cross(22, 6, 7, 400.0));
        merge_blocks(&mut net, &through(&[2, 3]));
        let main = main_roads(&net);
        assert_eq!(main.len(), 1, "{:?}", net.roads);
        assert_eq!((main[0].edge.from, main[0].edge.to), (NodeId(1), NodeId(6)));
        assert_eq!(starts(main[0]), [0.0, 100.0, 250.0]);
        assert!(net.junctions.iter().all(|j| j.id != NodeId(2) && j.id != NodeId(3)));
    }

    #[test]
    fn a_piece_drawn_the_other_way_is_turned_round() {
        let mut net = block();
        let lanes =
            vec![lane(LaneKind::Sidewalk, Direction::Forward), lane(LaneKind::Driving, Direction::Backward), lane(LaneKind::Driving, Direction::Forward)];
        net.roads[1] = two_way(11, "Main", 3, 2, &[(250.0, 0.0), (100.0, 0.0)], lanes, Some(40));
        merge_blocks(&mut net, &through(&[2]));
        let m = main_roads(&net)[0];
        assert_eq!((m.edge.from, m.edge.to), (NodeId(1), NodeId(3)), "the direction of the lowest id");
        assert_eq!(m.points, [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(250.0, 0.0)]);
        assert_eq!(m.zones[1].from.metres(), 100.0);
        let seen: Vec<(LaneKind, Direction)> = m.zones[1].lanes.iter().map(|l| (l.kind, l.direction)).collect();
        assert_eq!(seen, [(LaneKind::Driving, Direction::Backward), (LaneKind::Driving, Direction::Forward), (LaneKind::Sidewalk, Direction::Backward)]);
    }

    #[test]
    fn neighbouring_pieces_with_the_same_lanes_and_speed_are_one_zone() {
        let mut net = block();
        net.roads[1].zones = net.roads[0].zones.clone();
        merge_blocks(&mut net, &through(&[2]));
        let m = main_roads(&net)[0];
        assert_eq!(m.zones.len(), 1, "{:?}", m.zones);
        assert_eq!(m.points, [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(250.0, 0.0)], "the whole line");
    }

    #[test]
    fn pieces_whose_ends_do_not_touch_keep_both_points() {
        let mut net = block();
        net.roads[1].points = vec![Point::new(100.0, 1.0), Point::new(250.0, 0.0)];
        merge_blocks(&mut net, &through(&[2]));
        let m = main_roads(&net)[0];
        assert_eq!(m.points.len(), 4);
        assert_eq!(m.zones[1].from.metres(), 100.0 + 1.0, "the length up to (100, 1)");
    }

    /// Asserts that `merge_blocks`, through the nodes `ids`, leaves the network that `change` makes of `block()` as
    /// it was.
    fn left_alone(ids: &[u32], change: impl Fn(&mut Network)) {
        let mut net = block();
        change(&mut net);
        let before = net.clone();
        merge_blocks(&mut net, &through(ids));
        assert_eq!(net, before);
    }

    #[test]
    fn a_change_of_name_is_two_roads() {
        left_alone(&[2], |n| n.roads[1].name = "Other".into());
    }

    #[test]
    fn a_change_of_road_type_is_two_roads() {
        left_alone(&[2], |n| n.roads[1].road_type = RoadType::Collector);
    }

    #[test]
    fn a_node_the_data_does_not_pass_through_is_kept() {
        left_alone(&[], |_| {});
    }

    #[test]
    fn a_node_where_three_road_ends_meet_is_kept() {
        left_alone(&[2], |n| {
            n.junctions.push(node(6, 100.0, 100.0));
            n.roads.push(two_way(12, "Main", 2, 6, &[(100.0, 0.0), (100.0, 100.0)], driving(2), Some(30)));
        });
    }

    #[test]
    fn a_ring_of_connections_is_left_alone() {
        let mut net = Network {
            left_hand: false,
            corridors: vec![],
            junctions: vec![node(1, 0.0, 0.0), node(2, 100.0, 0.0), node(3, 50.0, 100.0)],
            roads: vec![
                two_way(10, "Main", 1, 2, &[(0.0, 0.0), (100.0, 0.0)], driving(2), Some(30)),
                two_way(11, "Main", 2, 3, &[(100.0, 0.0), (50.0, 100.0)], driving(4), Some(40)),
                two_way(12, "Main", 3, 1, &[(50.0, 100.0), (0.0, 0.0)], driving(2), Some(30)),
            ],
        };
        let before = net.clone();
        merge_blocks(&mut net, &through(&[1, 2, 3]));
        assert_eq!(net, before);
    }

    #[test]
    fn a_block_that_comes_back_to_its_own_end_is_left_alone() {
        left_alone(&[2, 3, 6], |n| {
            // Main goes on from 3 round to 1 through node 6; 3 is now a connection, 1 still meets Cross
            n.roads.retain(|r| r.edge.id != RoadId(21));
            n.junctions.push(node(6, 125.0, -100.0));
            n.roads.push(two_way(12, "Main", 3, 6, &[(250.0, 0.0), (125.0, -100.0)], driving(2), Some(30)));
            n.roads.push(two_way(13, "Main", 6, 1, &[(125.0, -100.0), (0.0, 0.0)], driving(2), Some(30)));
        });
    }

    #[test]
    fn a_priority_naming_a_piece_names_the_merged_road() {
        let mut net = block();
        net.junctions[2].control = Control::Priority(vec![RoadId(11), RoadId(21)]);
        merge_blocks(&mut net, &through(&[2]));
        assert_eq!(net.junctions.iter().find(|j| j.id == NodeId(3)).unwrap().control, Control::Priority(vec![RoadId(10), RoadId(21)]));
    }

    #[test]
    fn a_priority_naming_two_pieces_names_the_merged_road_once() {
        let mut net = block();
        net.junctions[2].control = Control::Priority(vec![RoadId(10), RoadId(11), RoadId(21)]);
        merge_blocks(&mut net, &through(&[2]));
        assert_eq!(net.junctions.iter().find(|j| j.id == NodeId(3)).unwrap().control, Control::Priority(vec![RoadId(10), RoadId(21)]));
    }

    #[test]
    fn merging_twenty_thousand_roads_takes_under_two_seconds() {
        const N: u32 = 10_000;
        let mut net = Network::default();
        for i in 0..N {
            let x = i as f64 * 1000.0;
            net.junctions.push(node(3 * i, x, 0.0));
            net.junctions.push(node(3 * i + 1, x + 100.0, 0.0));
            net.junctions.push(node(3 * i + 2, x + 250.0, 0.0));
            let name = format!("Street {i}");
            net.roads.push(two_way(2 * i, &name, 3 * i, 3 * i + 1, &[(x, 0.0), (x + 100.0, 0.0)], driving(2), Some(30)));
            net.roads.push(two_way(2 * i + 1, &name, 3 * i + 1, 3 * i + 2, &[(x + 100.0, 0.0), (x + 250.0, 0.0)], driving(4), Some(40)));
        }
        let connections: HashSet<NodeId> = (0..N).map(|i| NodeId(3 * i + 1)).collect();
        let start = std::time::Instant::now();
        merge_blocks(&mut net, &connections);
        let took = start.elapsed();
        assert_eq!(net.roads.len(), N as usize);
        assert_eq!(net.junctions.len(), 2 * N as usize);
        assert!(took < std::time::Duration::from_secs(2), "took {took:?}");
    }
}
