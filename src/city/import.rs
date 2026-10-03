//! A city made from a street network read from OpenStreetMap.

use std::collections::BTreeMap;

use osm_network::{Control, Lane, LaneKind, Network, Way};

use super::model::{City, EdgeDef, Layout, Names, NodeDef};
use crate::junction::model::{ALL_WAY_STOP, MAX_ARMS, MIN_ARMS, PRIORITY, SIGNAL};
use crate::shared::catalogue::{Side, kind_index};
use crate::street::model::{Piece, Street};

/// How wide a corner is rounded where nothing in the data says.
const CORNER_MM: i32 = 4_000;

/// Which sort of sample street a road is like, by OpenStreetMap's `highway`.
fn class_of(highway: &str) -> usize {
    const STREET: usize = 0;
    const AVENUE: usize = 1;
    const LANE: usize = 2;
    const FREEWAY: usize = 3;
    match highway {
        "motorway" | "motorway_link" => FREEWAY,
        "trunk" | "trunk_link" | "primary" | "primary_link" | "secondary" | "secondary_link" | "tertiary" | "tertiary_link" => AVENUE,
        "service" => LANE,
        _ => STREET,
    }
}

/// The catalogue kind that stands for a kind of lane. A buffer is planting and anything else (a turn lane, a
/// rail track) is a median: what is left of the road when no lane uses it.
fn kind_of(kind: LaneKind) -> &'static str {
    match kind {
        LaneKind::Driving => "travel",
        LaneKind::Parking => "parking",
        LaneKind::Sidewalk => "sidewalk",
        LaneKind::Bike => "bike",
        LaneKind::Bus => "bus",
        LaneKind::Buffer => "planting",
        LaneKind::Other => "median",
    }
}

fn piece(lane: &Lane) -> Piece {
    Piece {
        kind: kind_index(kind_of(lane.kind)).expect("the catalogue has every kind of lane"),
        width_mm: (lane.width_m * 1000.0).round() as i32,
        direction: Some(usize::from(lane.way == Way::Backward)),
    }
}

fn control_of(c: Control) -> usize {
    match c {
        Control::Signals => SIGNAL,
        Control::Signs => ALL_WAY_STOP,
        Control::None => PRIORITY,
    }
}

fn join(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => format!("{one} junction"),
        [a, b] => format!("{a} and {b}"),
        [a, b, c] => format!("{a}, {b} and {c}"),
        [a, b, rest @ ..] => format!("{a}, {b} and {} more", rest.len()),
    }
}

impl City {
    /// The city for `network`, every street and junction as OpenStreetMap has it.
    pub fn from_network(network: &Network, name: &str) -> City {
        City::on(Layout::from_network(network, name))
    }
}

impl Layout {
    /// The layout of a network: a node for each end or meeting of streets, a street for each road.
    /// Roads that start and end at one node, or at a node the network lacks, are left out, and so is a
    /// node nothing leads to.
    pub fn from_network(network: &Network, name: &str) -> Layout {
        let roads: Vec<_> = network.roads.iter().filter(|r| r.from != r.to).collect();
        let mut degree: BTreeMap<u32, usize> = BTreeMap::new();
        for r in &roads {
            *degree.entry(r.from).or_default() += 1;
            *degree.entry(r.to).or_default() += 1;
        }
        let nodes: Vec<_> = network.nodes.iter().filter(|n| degree.contains_key(&n.id)).collect();
        let index: BTreeMap<u32, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let roads: Vec<_> = roads.into_iter().filter(|r| index.contains_key(&r.from) && index.contains_key(&r.to)).collect();
        let (min_x, max_y) = nodes.iter().fold((f64::MAX, f64::MIN), |(x, y), n| (x.min(n.x_m), y.max(n.y_m)));
        let street_name = |r: &osm_network::Road| r.name.clone().unwrap_or_else(|| format!("Unnamed {}", r.highway.replace('_', " ")));

        let edges: Vec<EdgeDef> = roads
            .iter()
            .map(|r| {
                let class = class_of(&r.highway);
                let pieces: Vec<Piece> = r.lanes.iter().map(piece).collect();
                let side = if network.left_hand { Side::Left } else { Side::Right };
                EdgeDef {
                    a: index[&r.from],
                    b: index[&r.to],
                    street: class,
                    name: Some(street_name(r)),
                    section: (!pieces.is_empty()).then(|| Street::imported(class, side, &pieces)),
                }
            })
            .collect();

        let nodes = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let incident: Vec<&String> = edges.iter().filter(|e| e.a == i || e.b == i).filter_map(|e| e.name.as_ref()).collect();
                let mut distinct: Vec<String> = Vec::new();
                for n in incident {
                    if !distinct.contains(n) {
                        distinct.push(n.clone());
                    }
                }
                let junction = n.junction && (MIN_ARMS..=MAX_ARMS).contains(&degree[&n.id]);
                let names = if junction {
                    let title = join(&distinct);
                    Names { end: title.clone(), name: title }
                } else {
                    let street = distinct.first().cloned().unwrap_or_default();
                    if degree[&n.id] == 1 {
                        Names { name: format!("End of {street}"), end: format!("the end of {street}") }
                    } else {
                        Names { name: format!("Connection on {street}"), end: format!("a connection on {street}") }
                    }
                };
                NodeDef {
                    x_mm: ((n.x_m - min_x) * 1000.0).round() as i32,
                    y_mm: ((max_y - n.y_m) * 1000.0).round() as i32,
                    junction,
                    control: if junction { control_of(n.control) } else { 0 },
                    corner_mm: if junction { CORNER_MM } else { 0 },
                    names: Some(names),
                }
            })
            .collect();
        Layout { name: name.to_string(), side: if network.left_hand { Side::Left } else { Side::Right }, nodes, edges }
    }
}

#[cfg(test)]
mod tests {
    use osm_network::{Control, Lane, LaneKind, Node, Road, Way};

    use super::*;
    use crate::junction::model::CONTROLS;
    use crate::junction::model::SIGNAL;

    fn lane(kind: LaneKind, way: Way, width_m: f64) -> Lane {
        Lane { kind, way, width_m }
    }

    /// Two streets crossing at a signalled junction, with a dead end on each arm.
    fn crossing() -> Network {
        let node = |id, x_m, y_m, junction, control| Node { id, osm_nodes: vec![id as i64 * 10], x_m, y_m, junction, control };
        let road = |id, name: &str, to, points| Road {
            id,
            osm_ways: vec![id as i64 * 100],
            name: Some(name.to_string()),
            highway: "residential".into(),
            from: 1,
            to,
            lanes: vec![
                lane(LaneKind::Sidewalk, Way::Forward, 2.0),
                lane(LaneKind::Driving, Way::Backward, 3.0),
                lane(LaneKind::Driving, Way::Forward, 3.0),
                lane(LaneKind::Sidewalk, Way::Forward, 2.0),
            ],
            points,
        };
        Network {
            left_hand: false,
            nodes: vec![
                node(1, 0.0, 0.0, true, Control::Signals),
                node(2, 0.0, 200.0, false, Control::None),
                node(3, 200.0, 0.0, false, Control::None),
                node(4, 0.0, -200.0, false, Control::None),
                node(5, -200.0, 0.0, false, Control::None),
            ],
            roads: vec![
                road(1, "Side Road", 2, vec![(0.0, 0.0), (0.0, 200.0)]),
                road(2, "Main Street", 3, vec![(0.0, 0.0), (200.0, 0.0)]),
                road(3, "Side Road", 4, vec![(0.0, 0.0), (0.0, -200.0)]),
                road(4, "Main Street", 5, vec![(0.0, 0.0), (-200.0, 0.0)]),
            ],
        }
    }

    #[test]
    fn a_crossing_becomes_a_junction_with_a_street_on_each_arm() {
        let city = City::from_network(&crossing(), "Testville");
        let v = city.view(0);
        assert_eq!(v.name, "Testville");
        assert_eq!((v.nodes.len(), v.edges.len()), (5, 4));
        let junctions: Vec<_> = v.nodes.iter().filter(|n| n.junction).collect();
        assert_eq!(junctions.len(), 1);
        assert_eq!((junctions[0].arms, junctions[0].control), (4, Some(CONTROLS[SIGNAL].name)));
        assert!(city.junction_editor(junctions[0].uid, 0).is_some());
    }

    #[test]
    fn places_lie_where_the_data_puts_them_with_north_at_the_top() {
        let v = City::from_network(&crossing(), "Testville").view(0);
        let at = |osm_id: i64| v.nodes.iter().find(|n| n.uid as i64 == osm_id).unwrap();
        let (middle, north, east, south, west) = (at(1), at(2), at(3), at(4), at(5));
        assert!(north.y_mm < middle.y_mm && middle.y_mm < south.y_mm);
        assert!(west.x_mm < middle.x_mm && middle.x_mm < east.x_mm);
        assert_eq!(east.x_mm - middle.x_mm, 200_000);
        assert_eq!(v.bounds_mm, [0, 0, 400_000, 400_000]);
    }

    #[test]
    fn a_street_has_the_lanes_and_name_of_its_road() {
        let city = City::from_network(&crossing(), "Testville");
        let v = city.view(0);
        let main = v.edges.iter().find(|e| e.name.starts_with("Main Street")).expect("named for the road");
        assert_eq!(main.row_mm, 2000 + 3000 + 3000 + 2000);
        assert!(city.street_editor(main.uid, 0).is_some());
        let junction = v.nodes.iter().find(|n| n.junction).unwrap();
        assert!(junction.name.contains("Main Street") && junction.name.contains("Side Road"), "{}", junction.name);
        assert!(v.nodes.iter().filter(|n| !n.junction).all(|n| n.name.starts_with("End of ")), "{:?}", v.nodes.iter().map(|n| &n.name).collect::<Vec<_>>());
    }

    /// A node at the origin with a road to each of the given points, all of them junction-like in the data.
    fn star(ends: &[(f64, f64)]) -> Network {
        let mut net = crossing();
        net.nodes.truncate(1);
        net.roads.clear();
        for (i, &(x, y)) in ends.iter().enumerate() {
            let id = i as u32 + 2;
            net.nodes.push(Node { id, osm_nodes: vec![id as i64], x_m: x, y_m: y, junction: false, control: Control::None });
            let mut road = crossing().roads[0].clone();
            road.id = id;
            road.to = id;
            road.points = vec![(0.0, 0.0), (x, y)];
            net.roads.push(road);
        }
        net
    }

    #[test]
    fn arms_that_leave_almost_together_are_drawn_apart() {
        // two roads 10 degrees apart, and two more to make a crossing
        let net = star(&[(0.0, 200.0), (35.0, 197.0), (200.0, 0.0), (0.0, -200.0)]);
        let city = City::from_network(&net, "Fork");
        let v = city.view(0);
        let junction = v.nodes.iter().find(|n| n.junction).expect("still a junction");
        assert_eq!(junction.arms, 4);
        assert!(city.junction_editor(junction.uid, 0).is_some());
    }

    #[test]
    fn a_meeting_that_cannot_be_drawn_as_a_junction_is_not_offered_as_one() {
        // three roads all heading north: no junction editor can draw that
        let net = star(&[(-50.0, 200.0), (0.0, 200.0), (50.0, 200.0)]);
        let city = City::from_network(&net, "Fan");
        let v = city.view(0);
        assert_eq!(v.edges.len(), 3);
        assert!(v.nodes.iter().all(|n| !n.junction));
        assert!(v.nodes.iter().all(|n| n.ok), "nothing to fail where there is no junction");
    }
}
