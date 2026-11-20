//! A city made from a street network read from OpenStreetMap.

use std::collections::BTreeMap;

use cityloom_model::street::{class_of, legacy_pieces, one_way_bike_lanes, pieces, refs};
use cityloom_model::{Control, Network, NodeId, Point, Road};

use super::model::{City, EdgeDef, Layout, NodeDef};
use crate::junction::model::{ALL_WAY_STOP, MAX_ARMS, MIN_ARMS, PRIORITY, SIGNAL};
use crate::shared::catalogue::Side;
use crate::street::model::StreetExt;
use crate::street::model::{Piece, Speed, Street, Zone, ZoneEnd, ZoneFrame};

/// How wide a corner is rounded where nothing in the data says.
const CORNER_MM: i32 = 4_000;

fn control_of(c: &Control) -> usize {
    match c {
        Control::Signals => SIGNAL,
        Control::Signs => ALL_WAY_STOP,
        Control::None | Control::Priority(_) => PRIORITY,
    }
}

/// How far along a road to look for the way it leaves a junction: past the rounding of its corner.
const LOOK_M: f64 = 20.0;

/// The way a road leaves its first point and its last, in degrees clockwise from north, from a point
/// about `LOOK_M` along it (or its far end, if it is shorter).
fn headings(points: &[Point]) -> Option<(f64, f64)> {
    let along = |line: &[Point]| -> Option<f64> {
        let origin = *line.first()?;
        let mut run = 0.0;
        let mut at = origin;
        for &p in &line[1..] {
            run += p.distance(at);
            at = p;
            if run >= LOOK_M {
                break;
            }
        }
        let (dx, dy) = (at.x_m() - origin.x_m(), at.y_m() - origin.y_m());
        (dx != 0.0 || dy != 0.0).then(|| dx.atan2(dy).to_degrees().rem_euclid(360.0))
    };
    let reversed: Vec<Point> = points.iter().rev().copied().collect();
    Some((along(points)?, along(&reversed)?))
}

/// A road's centreline in layout millimetres with its two ends set to where its nodes are, so that a street
/// meets its junctions; None for a road with fewer than two points.
fn shape_mm(points: &[Point], from: (i32, i32), to: (i32, i32), mm_of: &impl Fn(f64, f64) -> (i32, i32)) -> Option<Vec<(i32, i32)>> {
    if points.len() < 2 {
        return None;
    }
    let mut line: Vec<(i32, i32)> = points.iter().map(|p| mm_of(p.x_m(), p.y_m())).collect();
    let last = line.len() - 1;
    line[0] = from;
    line[last] = to;
    Some(line)
}

/// Gives `street`, made from the first zone of `road`, a zone for each of the road's, measured along it as `frame`
/// says: each with its own pieces (uids the street issues) and speed limit, then settled, so a zone like the first
/// is the street's base. From the last end the zones are listed from that end, the road's last piece first.
/// Zones that would make the street unsound (a piece with no lanes, starts the same once rounded) are not opened:
/// the street keeps its first zone only, since an unsound street is never kept, so an edit to it would be lost.
fn open_zones(street: &mut Street, road: &Road, frame: ZoneFrame) {
    let mut zoned = street.clone();
    fill_zones(&mut zoned, road, frame);
    if zoned.is_sound() {
        *street = zoned;
    }
}

/// `open_zones` without the check that the zoned street is sound.
fn fill_zones(street: &mut Street, road: &Road, frame: ZoneFrame) {
    let length_m = road.length_m();
    if road.zones.len() < 2 || length_m <= 0.0 {
        return;
    }
    let at_mm = |m: f64| (m / length_m * frame.length_mm as f64).round() as i32;
    let ends = road.zones.iter().skip(1).map(|z| z.from.metres()).chain([length_m]);
    let mut zones = Vec::with_capacity(road.zones.len());
    for (zone, end) in road.zones.iter().zip(ends) {
        let mut segments = Street::imported(street.class, street.side, &pieces(&zone.lanes)).segments;
        for s in &mut segments {
            s.uid = street.next_uid;
            street.next_uid += 1;
        }
        let from_mm = match frame.from_end {
            ZoneEnd::First => at_mm(zone.from.metres()),
            ZoneEnd::Last => at_mm(length_m - end),
        };
        zones.push(Zone { from_mm, segments: Some(segments), name: None, speed: Speed::from_kmh(zone.speed_kmh) });
    }
    if frame.from_end == ZoneEnd::Last {
        zones.reverse();
    }
    street.zones = zones;
    street.from_end = frame.from_end;
    street.length_mm = Some(frame.length_mm);
    street.settle_zones();
    street.settle_class();
}

impl City {
    /// The city for `network`, every street and junction as OpenStreetMap has it.
    pub fn from_network(network: &Network, name: &str) -> City {
        City::on(Layout::from_network(network, name))
    }
}

impl Layout {
    /// The layout of a network: a node for each end or meeting of streets, a street for each road, with a zone for each
    /// of its zones.
    /// Roads that start and end at one node, that have no lanes, that have no name, or that start or end at a node the
    /// network lacks are left out, and so is a node nothing leads to.
    pub fn from_network(network: &Network, name: &str) -> Layout {
        let roads: Vec<_> = network.roads.iter().filter(|r| r.edge.from != r.edge.to && !r.base_lanes().is_empty()).collect();
        let mut degree: BTreeMap<NodeId, usize> = BTreeMap::new();
        for r in &roads {
            *degree.entry(r.edge.from).or_default() += 1;
            *degree.entry(r.edge.to).or_default() += 1;
        }
        let nodes: Vec<_> = network.junctions.iter().filter(|n| degree.contains_key(&n.id)).collect();
        let index: BTreeMap<NodeId, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let roads: Vec<_> = roads.into_iter().filter(|r| index.contains_key(&r.edge.from) && index.contains_key(&r.edge.to)).collect();
        let (min_x, max_y) = nodes.iter().fold((f64::MAX, f64::MIN), |(x, y), n| (x.min(n.position.x_m()), y.max(n.position.y_m())));
        let origin_m = (!nodes.is_empty()).then_some((min_x, max_y));
        let mm_of = |x_m: f64, y_m: f64| (((x_m - min_x) * 1000.0).round() as i32, ((max_y - y_m) * 1000.0).round() as i32);
        let node_mm = |id: NodeId| {
            let n = nodes[index[&id]];
            mm_of(n.position.x_m(), n.position.y_m())
        };
        let edges: Vec<EdgeDef> = roads
            .iter()
            .map(|r| {
                let class = class_of(r.road_type);
                let pieces = pieces(r.base_lanes());
                let side = if network.left_hand { Side::Left } else { Side::Right };
                let section = Street {
                    name: Some(r.name.clone()),
                    source: refs(&r.provenance),
                    speed: Speed::from_kmh(r.base_speed_kmh()),
                    ..Street::imported(class, side, &pieces)
                };
                let row_of = |pieces: &[Piece]| Street::imported(class, side, pieces).row_mm;
                let one_way = one_way_bike_lanes(r.base_lanes());
                let mut legacy_rows_mm: Vec<i32> =
                    [row_of(&legacy_pieces(r.base_lanes())), row_of(&self::pieces(&one_way)), row_of(&legacy_pieces(&one_way))].into();
                legacy_rows_mm.sort_unstable();
                legacy_rows_mm.dedup();
                legacy_rows_mm.retain(|&row| row != section.row_mm);
                EdgeDef {
                    a: index[&r.edge.from],
                    b: index[&r.edge.to],
                    class,
                    name: Some(r.name.clone()),
                    headings: headings(&r.points),
                    shape: shape_mm(&r.points, node_mm(r.edge.from), node_mm(r.edge.to), &mm_of),
                    legacy_rows_mm,
                    section,
                }
            })
            .collect();

        let nodes = nodes
            .iter()
            .map(|n| {
                let junction = n.junction && (MIN_ARMS..=MAX_ARMS).contains(&degree[&n.id]);
                NodeDef {
                    x_mm: ((n.position.x_m() - min_x) * 1000.0).round() as i32,
                    y_mm: ((max_y - n.position.y_m()) * 1000.0).round() as i32,
                    junction,
                    control: if junction { control_of(&n.control) } else { 0 },
                    corner_mm: if junction { CORNER_MM } else { 0 },
                    source: refs(&n.provenance),
                }
            })
            .collect();
        let mut layout = Layout { name: name.to_string(), side: if network.left_hand { Side::Left } else { Side::Right }, nodes, edges, origin_m };
        // zones are measured along the street as laid out, from the end the layout anchors them on; a road cut at
        // both ends has none, and opens with its first zone only
        for (edge, road) in roads.iter().enumerate().filter(|(_, r)| r.zones.len() > 1) {
            if let Some(frame) = layout.zone_frame(edge) {
                open_zones(&mut layout.edges[edge].section, road, frame);
            }
        }
        layout
    }
}

#[cfg(test)]
mod tests {
    use cityloom_model::{
        BufferType, Control, Direction, Edge, Junction, Lane, LaneKind, NodeId, NodeRef, Point, Provenance, Road, RoadId, RoadType, WayRef, Zone,
    };

    use super::*;
    use crate::junction::model::CONTROLS;
    use crate::junction::model::SIGNAL;
    use crate::shared::catalogue::{KINDS, StreetClass};
    use crate::shared::said::{Arg, Said};
    use crate::shared::testing::en;

    fn lane(kind: LaneKind, way: Direction, width_m: f64) -> Lane {
        Lane { kind, direction: way, width_m, buffer_type: None, windows: vec![], shape: vec![] }
    }

    fn buffer(buffer_type: Option<BufferType>) -> Lane {
        Lane { buffer_type, ..lane(LaneKind::Buffer, Direction::Forward, 0.6) }
    }

    fn ids(lanes: &[Lane]) -> Vec<&'static str> {
        pieces(lanes).iter().map(|p| KINDS[p.kind].id).collect()
    }

    fn widths(lanes: &[Lane]) -> Vec<i32> {
        pieces(lanes).iter().map(|p| p.width_mm).collect()
    }

    fn curb() -> Lane {
        buffer(Some(BufferType::Curb))
    }

    trait WithWidth {
        fn with_width(self, width_m: f64) -> Lane;
    }

    impl WithWidth for Lane {
        fn with_width(self, width_m: f64) -> Lane {
            Lane { width_m, ..self }
        }
    }

    fn bike() -> Lane {
        lane(LaneKind::Bike, Direction::Forward, 1.8)
    }

    fn driving() -> Lane {
        lane(LaneKind::Driving, Direction::Forward, 3.3)
    }

    /// The edges of the piece `i` that carry a curb in the street the lanes make, each true for its right edge.
    fn curb_sides(lanes: &[Lane], i: usize) -> Vec<bool> {
        let street = Street::imported(StreetClass::Local, Side::Right, &pieces(lanes));
        let editor = crate::street::model::Editor::from_street(&street, &street, 0);
        crate::street::model::pieces::curb_edges(&editor.view().segments, i)
    }

    #[test]
    fn a_curb_buffer_between_a_bike_lane_and_the_road_is_the_bike_lanes_own_curb_on_that_side() {
        // the bike lane to the left of the curb, then to its right: its width grows by the curb's and the curb is on the road side
        let left = [bike(), curb(), driving()];
        assert_eq!(ids(&left), ["bike", "travel"]);
        assert_eq!(widths(&left), [2400, 3300]);
        assert_eq!(curb_sides(&left, 0), [true]);
        let right = [driving(), curb(), bike()];
        assert_eq!(ids(&right), ["travel", "bike"]);
        assert_eq!(widths(&right), [3300, 2400]);
        assert_eq!(curb_sides(&right, 1), [false]);
        // beside parking and a transit lane too: they are road pieces
        for road in [LaneKind::Parking, LaneKind::Bus] {
            let lanes = [bike(), curb(), lane(road, Direction::Forward, 2.4)];
            assert_eq!(ids(&lanes).len(), 2, "{road:?}");
            assert_eq!(widths(&lanes)[0], 2400, "{road:?}");
            assert_eq!(curb_sides(&lanes, 0), [true], "{road:?}");
        }
        // a curb on each side of the bike lane
        let both = [driving(), curb(), bike(), curb(), driving()];
        assert_eq!(ids(&both), ["travel", "bike", "travel"]);
        assert_eq!(widths(&both), [3300, 3000, 3300]);
        assert_eq!(curb_sides(&both, 1), [false, true]);
        // the bike lane keeps its direction and the curb is the catalogue's default one
        let p = pieces(&left);
        assert_eq!(p[0].direction, Some(0));
        let street = Street::imported(StreetClass::Local, Side::Right, &p);
        assert_eq!(street.segments[0].curb, Some(crate::shared::catalogue::DEFAULT_CURB));
    }

    #[test]
    fn a_curb_buffer_between_a_sidewalk_and_a_road_lane_is_the_sidewalks_own_curb() {
        let sidewalk = || lane(LaneKind::Sidewalk, Direction::Forward, 2.0);
        let left = [sidewalk(), curb(), driving()];
        assert_eq!(ids(&left), ["sidewalk", "travel"]);
        assert_eq!(widths(&left), [2600, 3300]);
        assert_eq!(curb_sides(&left, 0), [true]);
        let right = [driving(), curb(), sidewalk()];
        assert_eq!(ids(&right), ["travel", "sidewalk"]);
        assert_eq!(widths(&right), [3300, 2600]);
        assert_eq!(curb_sides(&right, 1), [false]);
        let parked = [sidewalk(), curb(), lane(LaneKind::Parking, Direction::Forward, 2.4)];
        assert_eq!(ids(&parked), ["sidewalk", "parking"]);
        assert_eq!(widths(&parked), [2600, 2400]);
        // the sidewalk's own maximum holds
        let wide = [lane(LaneKind::Sidewalk, Direction::Forward, 7.8), curb(), driving()];
        assert_eq!(widths(&wide), [8000, 400, 3300]);
    }

    #[test]
    fn what_a_curb_adds_past_the_maximum_of_the_piece_it_joins_stays_as_a_buffer_so_no_width_is_lost() {
        let wide_bike = [lane(LaneKind::Bike, Direction::Forward, 2.8), Lane { width_m: 0.5, ..curb() }, driving()];
        assert_eq!(ids(&wide_bike), ["bike", "buffer", "travel"]);
        assert_eq!(widths(&wide_bike), [3000, 300, 3300]);
        let street = Street::imported(StreetClass::Local, Side::Right, &pieces(&wide_bike));
        assert_eq!(street.row_mm, 6600, "the street is as wide as its lanes");
        // a curb on each side of a lane with room for part of one: the lanes' widths are all kept
        let both = [driving(), curb(), lane(LaneKind::Bike, Direction::Forward, 2.5), curb(), driving()];
        let w = widths(&both);
        assert_eq!(w.iter().sum::<i32>(), 3300 + 600 + 2500 + 600 + 3300);
        assert!(w.contains(&3000), "the lane is at its maximum: {w:?}");
    }

    #[test]
    fn a_remainder_under_the_buffers_minimum_goes_to_the_road_lane_beside_the_curb_and_the_street_keeps_its_width() {
        // 2.95 m of bike lane and a 0.1 m curb: 50 mm too much for the lane, too little for a buffer
        let lanes = [lane(LaneKind::Bike, Direction::Forward, 2.95), curb().with_width(0.1), driving()];
        assert_eq!(ids(&lanes), ["bike", "travel"]);
        assert_eq!(widths(&lanes), [3000, 3350]);
        let street = Street::imported(StreetClass::Local, Side::Right, &pieces(&lanes));
        assert_eq!(street.row_mm, 6350, "exactly the lanes' width");
        // the other order: the road lane is before the curb
        let lanes = [driving(), curb().with_width(0.1), lane(LaneKind::Bike, Direction::Forward, 2.95)];
        assert_eq!(widths(&lanes), [3350, 3000]);
        // 150 mm and more stays a buffer
        let lanes = [lane(LaneKind::Bike, Direction::Forward, 2.95), curb().with_width(0.2), driving()];
        assert_eq!(widths(&lanes), [3000, 150, 3300]);
    }

    #[test]
    fn a_street_and_a_junction_carry_the_osm_ways_and_nodes_they_were_made_from_with_their_versions() {
        use crate::shared::provenance::OsmRef;
        let mut net = crossing();
        net.roads[1].provenance = Provenance::of_ways([WayRef::with_version(200, 12)]);
        net.junctions[0].provenance = Provenance::of_nodes([NodeRef::with_version(10, 4)]);
        let city = City::from_network(&net, "Testville");
        let v = city.view(0);
        let street = |i: usize| city.street_editor(v.edges[i].uid, 0).unwrap().view();
        assert_eq!(street(1).source, [OsmRef { id: 200, version: Some(12) }]);
        assert_eq!(street(0).source, [OsmRef { id: 100, version: None }], "a way with no version is still named");
        let junction = v.nodes.iter().find(|n| n.junction).unwrap().uid;
        let editor = city.junction_editor(junction, 0).unwrap();
        assert_eq!(editor.view().source, [OsmRef { id: 10, version: Some(4) }]);
        assert_eq!(editor.snapshot().source, [OsmRef { id: 10, version: Some(4) }]);
    }

    /// Two streets crossing at a signalled junction, with a dead end on each arm.
    fn crossing() -> Network {
        let node = |id: u32, x_m, y_m, junction, control| Junction {
            id: NodeId(id),
            provenance: Provenance::of_nodes([NodeRef::new(id as i64 * 10)]),
            position: Point::new(x_m, y_m),
            junction,
            control,
        };
        let road = |id: u32, name: &str, to: u32, points| Road {
            edge: Edge { id: RoadId(id), from: NodeId(1), to: NodeId(to) },
            provenance: Provenance::of_ways([WayRef::new(id as i64 * 100)]),
            name: name.to_string(),
            road_type: RoadType::Local,
            zones: vec![Zone::whole(vec![
                lane(LaneKind::Sidewalk, Direction::Forward, 2.0),
                lane(LaneKind::Driving, Direction::Backward, 3.0),
                lane(LaneKind::Driving, Direction::Forward, 3.0),
                lane(LaneKind::Sidewalk, Direction::Forward, 2.0),
            ])],
            points,
        };
        Network {
            left_hand: false,
            corridors: vec![],
            junctions: vec![
                node(1, 0.0, 0.0, true, Control::Signals),
                node(2, 0.0, 200.0, false, Control::None),
                node(3, 200.0, 0.0, false, Control::None),
                node(4, 0.0, -200.0, false, Control::None),
                node(5, -200.0, 0.0, false, Control::None),
            ],
            roads: vec![
                road(1, "Side Road", 2, vec![Point::new(0.0, 0.0), Point::new(0.0, 200.0)]),
                road(2, "Main Street", 3, vec![Point::new(0.0, 0.0), Point::new(200.0, 0.0)]),
                road(3, "Side Road", 4, vec![Point::new(0.0, 0.0), Point::new(0.0, -200.0)]),
                road(4, "Main Street", 5, vec![Point::new(0.0, 0.0), Point::new(-200.0, 0.0)]),
            ],
        }
    }

    #[test]
    fn a_street_takes_the_speed_of_its_road() {
        use crate::street::model::Speed;
        for (road_speed, speed, kmh) in [(Some(40), Speed::Kmh(40), Some(40)), (None, Speed::NotSet, None)] {
            let mut network = crossing();
            network.roads[0].zones[0].speed_kmh = road_speed;
            let layout = Layout::from_network(&network, "x");
            let street = &layout.edges.iter().find(|e| e.section.source == refs(&network.roads[0].provenance)).unwrap().section;
            assert_eq!((street.speed, street.speed_kmh()), (speed, kmh));
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
        assert_eq!((junctions[0].arms, junctions[0].control), (4, Some(CONTROLS[SIGNAL].key)));
        assert!(city.junction_editor(junctions[0].uid, 0).is_some());
    }

    #[test]
    fn a_junction_with_priority_roads_is_run_with_priority() {
        let mut net = crossing();
        net.junctions[0].control = Control::Priority(vec![net.roads[0].edge.id, net.roads[2].edge.id]);
        let city = City::from_network(&net, "Testville");
        let v = city.view(0);
        let junction = v.nodes.iter().find(|n| n.junction).unwrap();
        assert_eq!(junction.control, Some(CONTROLS[PRIORITY].key));
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
    fn a_network_without_roads_is_a_city_of_no_places_whose_bounds_are_a_point() {
        let v = City::from_network(&Network::default(), "Nowhere").view(0);
        assert_eq!((v.name.as_str(), v.places, v.nodes.len(), v.edges.len()), ("Nowhere", 0, 0, 0));
        assert_eq!(v.bounds_mm, [0; 4], "not the empty fold's sentinels, which make an inside-out box");
        assert_eq!(v.origin_m, None);
    }

    #[test]
    fn a_street_has_the_lanes_and_name_of_its_road() {
        let city = City::from_network(&crossing(), "Testville");
        let v = city.view(0);
        let main = v.edges.iter().find(|e| en(&e.called) == "Main Street").expect("named for the road");
        assert_eq!(main.row_mm, 2000 + 3000 + 3000 + 2000);
        assert!(city.street_editor(main.uid, 0).is_some());
        let junction = en(&v.nodes.iter().find(|n| n.junction).unwrap().name);
        assert!(junction.contains("Main Street") && junction.contains("Side Road"), "{junction}");
        let ends: Vec<String> = v.nodes.iter().filter(|n| !n.junction).map(|n| en(&n.name)).collect();
        assert!(ends.iter().all(|n| n.starts_with("End of ")), "{ends:?}");
    }

    /// A node at the origin with a road to each of the given points, all of them junction-like in the data.
    fn star(ends: &[(f64, f64)]) -> Network {
        let mut net = crossing();
        net.junctions.truncate(1);
        net.roads.clear();
        for (i, &(x, y)) in ends.iter().enumerate() {
            let id = i as u32 + 2;
            net.junctions.push(Junction {
                id: NodeId(id),
                provenance: Provenance::of_nodes([NodeRef::new(id as i64)]),
                position: Point::new(x, y),
                junction: false,
                control: Control::None,
            });
            let mut road = crossing().roads[0].clone();
            road.edge.id = RoadId(id);
            road.edge.to = NodeId(id);
            road.points = vec![Point::new(0.0, 0.0), Point::new(x, y)];
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

    #[test]
    fn a_places_title_follows_the_names_of_the_streets_that_meet_there() {
        let mut layout = Layout::from_network(&crossing(), "Testville");
        let junction = layout.nodes.iter().position(|n| n.junction).unwrap();
        assert_eq!(en(&layout.node_name(junction)), "Side Road and Main Street");
        assert_eq!(en(&layout.end_name(junction)), "Side Road and Main Street");
        let gate = layout.nodes.iter().position(|n| !n.junction).unwrap();
        let street = layout.edges[layout.edges_at(gate)[0]].name.clone().unwrap();
        assert_eq!(en(&layout.node_name(gate)), format!("End of {street}"));
        assert_eq!(en(&layout.end_name(gate)), format!("the end of {street}"));
        // the title is not kept: renaming a street changes it
        for e in &mut layout.edges {
            if e.name.as_deref() == Some("Side Road") {
                e.name = Some("Renamed Road".into());
            }
        }
        assert_eq!(en(&layout.node_name(junction)), "Renamed Road and Main Street");
    }

    #[test]
    fn a_meeting_that_cannot_be_drawn_is_titled_as_a_connection_on_its_street() {
        let net = star(&[(-50.0, 200.0), (0.0, 200.0), (50.0, 200.0)]);
        let v = City::from_network(&net, "Fan").view(0);
        let meeting = v.nodes.iter().find(|n| n.uid == 1).unwrap();
        assert!(!meeting.junction);
        assert_eq!(en(&meeting.name), "Connection on Side Road");
    }

    #[test]
    fn a_road_with_no_lanes_is_not_a_street() {
        let mut net = crossing();
        net.roads[0].base_lanes_mut().clear();
        let v = City::from_network(&net, "Bare").view(0);
        assert_eq!(v.edges.len(), 3, "the lane-less road is left out");
    }

    #[test]
    fn a_motorway_is_a_freeway_and_a_residential_street_is_not() {
        let mut net = crossing();
        net.roads[1].road_type = RoadType::Motorway;
        let v = City::from_network(&net, "Fast").view(0);
        assert_eq!(v.edges.iter().filter(|e| e.freeway).count(), 1);
    }

    fn bare_street() -> Network {
        let mut net = star(&[(100.0, 0.0)]);
        *net.roads[0].base_lanes_mut() = vec![lane(LaneKind::Driving, Direction::Backward, 3.0), lane(LaneKind::Driving, Direction::Forward, 3.0)];
        net
    }

    #[test]
    fn what_a_real_street_already_lacks_is_not_flagged_until_a_change_makes_it_worse() {
        let mut city = City::from_network(&bare_street(), "Bare");
        let v = city.view(0);
        // two driving lanes and nothing else: no sidewalks, no room for emergency vehicles
        let editor = city.street_editor(v.edges[0].uid, 0).unwrap();
        assert!(editor.view().checks.iter().any(|c| !c.ok), "the street does fall short of the rules");
        assert!(v.edges[0].ok && v.edges[0].failing.is_empty() && v.failing == 0, "but it is as it is, not a problem to fix");
        // making it wider than its room is a new failure, and is flagged, alone
        let mut e = editor;
        let uid = e.view().segments[0].uid;
        e.nudge_width(uid, 100);
        assert!(city.keep_street(v.edges[0].uid, e.snapshot()));
        let v = city.view(0);
        assert_eq!(v.edges[0].failing.iter().map(en).collect::<Vec<_>>(), ["Fits the street width"]);
        assert_eq!(v.failing, 1);
    }

    #[test]
    fn the_editors_call_a_street_by_its_own_name_and_not_by_its_kind() {
        let city = City::from_network(&crossing(), "Testville");
        let v = city.view(0);
        let main = v.edges.iter().find(|e| en(&e.called) == "Main Street").unwrap();
        assert_eq!(main.called, Said::new("city-name").with("name", Arg::Text("Main Street".into())));
        assert_eq!(city.street_editor(main.uid, 0).unwrap().view().name, Street::imported(StreetClass::Local, Side::Right, &[]).named("Main Street").title());
        let junction = v.nodes.iter().find(|n| n.junction).unwrap();
        let arms: Vec<String> = city.junction_editor(junction.uid, 0).unwrap().view().arms.iter().map(|a| en(&a.street)).collect();
        assert_eq!(arms.iter().filter(|n| *n == "Main Street").count(), 2);
        assert_eq!(arms.iter().filter(|n| *n == "Side Road").count(), 2);
        // and a name survives being kept and opened again
        assert_eq!(Street::imported(StreetClass::Local, Side::Right, &[]).named("X").name.as_deref(), Some("X"));
    }

    #[test]
    fn a_junction_is_numbered_by_its_place_among_the_junctions_and_a_gate_has_no_number() {
        let v = City::from_network(&crossing(), "Testville").view(0);
        let numbers: Vec<u32> = v.nodes.iter().map(|n| n.number).collect();
        assert_eq!(numbers, vec![1, 0, 0, 0, 0]);
        let sample = City::new().view(0);
        let mut j: Vec<u32> = sample.nodes.iter().filter(|n| n.junction).map(|n| n.number).collect();
        j.sort_unstable();
        assert_eq!(j, (1..=9).collect::<Vec<u32>>());
    }

    /// A T whose three streets all curve away to the north, so that the straight line from the junction to the
    /// far end of each lies in one half of the plane, though they leave the junction west, east and south.
    fn curving_t() -> Network {
        let mut net = star(&[(-100.0, 100.0), (100.0, 100.0), (60.0, 50.0)]);
        net.roads[0].points = vec![Point::new(0.0, 0.0), Point::new(-30.0, 0.0), Point::new(-100.0, 100.0)];
        net.roads[1].points = vec![Point::new(0.0, 0.0), Point::new(30.0, 0.0), Point::new(100.0, 100.0)];
        net.roads[2].points = vec![Point::new(0.0, 0.0), Point::new(0.0, -30.0), Point::new(60.0, -30.0), Point::new(60.0, 50.0)];
        net
    }

    #[test]
    fn a_junction_leaves_the_way_its_roads_do_and_not_the_way_their_far_ends_lie() {
        let city = City::from_network(&curving_t(), "Curves");
        let v = city.view(0);
        let junction = v.nodes.iter().find(|n| n.junction).expect("a T, though the far ends all lie to the north");
        let arms = city.junction_editor(junction.uid, 0).unwrap().view().arms;
        let mut bearings: Vec<i32> = arms.iter().map(|a| a.bearing).collect();
        bearings.sort_unstable();
        assert_eq!(bearings, vec![90, 180, 270]);
    }

    #[test]
    fn a_street_keeps_its_roads_shape_with_its_ends_on_its_nodes() {
        let v = City::from_network(&curving_t(), "Curves").view(0);
        let node = |uid: u32| v.nodes.iter().find(|n| n.uid == uid).unwrap();
        let mut lens: Vec<usize> = v.edges.iter().map(|e| e.shape_mm.len()).collect();
        lens.sort_unstable();
        assert_eq!(lens, vec![3, 3, 4], "a point for each point the road has");
        for e in &v.edges {
            assert_eq!(e.shape_mm[0], [node(e.a).x_mm, node(e.a).y_mm], "starts on node a");
            assert_eq!(*e.shape_mm.last().unwrap(), [node(e.b).x_mm, node(e.b).y_mm], "ends on node b");
        }
    }

    #[test]
    fn a_curved_street_is_longer_than_the_line_between_its_ends() {
        let v = City::from_network(&curving_t(), "Curves").view(0);
        let e = &v.edges[0]; // (0,0) -> (-30,0) -> (-100,100): 30 m, then about 122.07 m
        let (a, b) = (e.shape_mm[0], e.shape_mm[2]);
        let chord = ((a[0] - b[0]) as f64).hypot((a[1] - b[1]) as f64);
        assert!((e.length_mm - 152_066).abs() <= 2, "{}", e.length_mm);
        assert!(e.length_mm as f64 > chord + 5_000.0);
    }

    #[test]
    fn a_road_with_no_centreline_or_one_point_is_the_straight_line_between_its_nodes() {
        for points in [vec![], vec![Point::new(0.0, 0.0)]] {
            let mut net = curving_t();
            net.roads[0].points = points;
            let v = City::from_network(&net, "Bare").view(0);
            let e = &v.edges[0];
            assert_eq!(e.shape_mm.len(), 2);
            let node = |uid: u32| v.nodes.iter().find(|n| n.uid == uid).unwrap();
            assert_eq!(e.shape_mm[0], [node(e.a).x_mm, node(e.a).y_mm]);
            assert_eq!(e.shape_mm[1], [node(e.b).x_mm, node(e.b).y_mm]);
        }
    }

    fn two_lanes() -> Vec<Lane> {
        vec![lane(LaneKind::Driving, Direction::Backward, 3.0), lane(LaneKind::Driving, Direction::Forward, 3.0)]
    }

    fn four_lanes() -> Vec<Lane> {
        let mut lanes = two_lanes();
        lanes.insert(0, lane(LaneKind::Driving, Direction::Backward, 3.0));
        lanes.push(lane(LaneKind::Driving, Direction::Forward, 3.0));
        lanes
    }

    /// A 200 m road east from one OSM node to another: two lanes at 30 km/h, then four at 40 from `second_m`.
    fn zoned_road(second_m: f64) -> Network {
        let mut net = star(&[(200.0, 0.0)]);
        net.roads[0].zones = vec![
            Zone { speed_kmh: Some(30), ..Zone::whole(two_lanes()) },
            Zone { from: cityloom_model::Coordinate::new(second_m), speed_kmh: Some(40), lanes: four_lanes() },
        ];
        net
    }

    /// The pieces of a section, without their uids.
    fn kinds_and_widths(section: &[crate::street::model::Segment]) -> Vec<(usize, i32, Option<usize>)> {
        section.iter().map(|s| (s.kind, s.width_mm, s.direction)).collect()
    }

    /// The pieces the editor makes of `lanes`, without their uids.
    fn imported(lanes: &[Lane]) -> Vec<(usize, i32, Option<usize>)> {
        kinds_and_widths(&Street::imported(StreetClass::Local, Side::Right, &pieces(lanes)).segments)
    }

    #[test]
    fn a_road_of_two_zones_opens_as_one_street_of_two_zones() {
        use crate::street::model::ZoneEnd;
        let layout = Layout::from_network(&zoned_road(80.0), "x");
        let street = &layout.edges[0].section;
        let length_mm = street.length_mm.expect("measured");
        assert_eq!(street.from_end, ZoneEnd::First);
        assert_eq!(street.zones.len(), 2);
        let expected = (80_000.0 * length_mm as f64 / 200_000.0).round() as i32;
        assert!((street.zones[1].from_mm - expected).abs() <= 1, "{} against {expected}", street.zones[1].from_mm);
        assert_eq!(kinds_and_widths(&street.segments), imported(&two_lanes()), "the base is the first piece's");
        assert_eq!(street.zones[0].segments, None, "the first zone is the base");
        assert_eq!(kinds_and_widths(street.zones[1].segments.as_deref().expect("its own section")), imported(&four_lanes()));
        assert_eq!((street.speed, street.zones[0].speed, street.zones[1].speed), (Speed::Kmh(30), Speed::Undecided, Speed::Kmh(40)));
        assert!(street.zones.iter().all(|z| z.name.is_none()));
        assert!(street.is_sound(), "every uid is one the street has issued: {street:?}");
        let own = street.zones[1].segments.as_deref().unwrap();
        assert!(own.iter().all(|s| street.segments.iter().all(|b| b.uid != s.uid)), "fresh uids");
    }

    #[test]
    fn a_road_of_one_zone_opens_as_before() {
        let mut net = zoned_road(80.0);
        net.roads[0].zones.truncate(1);
        let street = &Layout::from_network(&net, "x").edges[0].section;
        assert!(street.zones.is_empty());
        assert_eq!(street.length_mm, None);
        assert_eq!(street.next_uid, 3, "no uid spent on zones");
    }

    #[test]
    fn a_zoned_road_whose_first_end_is_cut_is_measured_from_its_last_end() {
        use crate::street::model::ZoneEnd;
        let mut net = zoned_road(80.0);
        net.junctions[0].provenance = Provenance::default();
        let street = &Layout::from_network(&net, "x").edges[0].section;
        let length_mm = street.length_mm.expect("measured");
        assert_eq!(street.from_end, ZoneEnd::Last);
        assert_eq!(street.zones.len(), 2);
        assert_eq!(kinds_and_widths(street.zones[0].segments.as_deref().expect("the road's last piece")), imported(&four_lanes()));
        assert_eq!(street.zones[0].speed, Speed::Kmh(40));
        assert_eq!((street.zones[1].segments.as_ref(), street.zones[1].speed), (None, Speed::Undecided), "the base: the road's first piece");
        let expected = (120_000.0 * length_mm as f64 / 200_000.0).round() as i32;
        assert!((street.zones[1].from_mm - expected).abs() <= 1, "{} against {expected}", street.zones[1].from_mm);
        assert!(street.is_sound());
    }

    #[test]
    fn a_road_cut_at_both_ends_opens_with_its_first_zone() {
        let mut net = zoned_road(80.0);
        net.junctions[0].provenance = Provenance::default();
        net.junctions[1].provenance = Provenance::default();
        let street = &Layout::from_network(&net, "x").edges[0].section;
        assert!(street.zones.is_empty());
        assert_eq!(kinds_and_widths(&street.segments), imported(&two_lanes()));
    }

    #[test]
    fn a_zone_shorter_than_the_editor_minimum_is_kept() {
        let street = &Layout::from_network(&zoned_road(197.0), "x").edges[0].section;
        assert_eq!(street.zones.len(), 2);
        assert!(street.is_sound());
    }

    #[test]
    fn zones_that_would_make_an_unsound_street_leave_it_with_its_first_zone() {
        // a later piece with no lanes: an empty section
        let mut lane_less = zoned_road(80.0);
        lane_less.roads[0].zones[1].lanes.clear();
        // two starts under a millimetre apart: the same start once rounded
        let mut crowded = zoned_road(80.0);
        crowded.roads[0].zones.push(Zone { from: cityloom_model::Coordinate::new(80.0002), speed_kmh: Some(50), lanes: two_lanes() });
        for net in [lane_less, crowded] {
            let street = &Layout::from_network(&net, "x").edges[0].section;
            assert!(street.zones.is_empty(), "{:?}", street.zones);
            assert_eq!((street.length_mm, street.next_uid), (None, 3), "as a road of one zone opens");
            assert_eq!(kinds_and_widths(&street.segments), imported(&two_lanes()));
            assert!(street.is_sound());
        }
    }

    #[test]
    fn an_unchanged_zoned_street_is_not_saved() {
        use crate::city::store::CityStore;
        use crate::place::area::Area;
        use crate::shared::ports::test_ports;
        let (ports, ..) = test_ports();
        let store = CityStore::for_area(ports.storage, Area::new("Testville", 1.0, 2.0));
        assert!(store.keep_network(&zoned_road(80.0)));
        let city = store.open();
        let v = city.view(0);
        assert_eq!((v.edges[0].zones, v.edited), (2, 0));
        let editor = city.street_editor(v.edges[0].uid, 0).unwrap();
        assert_eq!(editor.snapshot().zones.len(), 2, "the editor opens the zones as imported");
        assert!(editor.structure_blocked().is_none(), "and may rearrange them");
    }

    /// A merged block (a zone per OSM piece) as the street page shows it: neighbouring pieces that look the same are one
    /// run of the plan, an edit over a pick is made again on each run it covers, and the edited street is kept stamped
    /// with the importer, so clearing the edits made on the pieces before the merge leaves it.
    #[test]
    fn a_merged_block_shows_its_pieces_as_runs_takes_an_edit_on_each_and_is_kept_stamped() {
        use crate::city::store::CityStore;
        use crate::place::area::Area;
        use crate::shared::catalogue::kind_index;
        use crate::shared::ports::test_ports;
        // Three pieces: two lanes at 30 km/h, the same again from 60 m (a way of its own in OSM), four lanes at 40 from 120 m.
        let mut net = zoned_road(120.0);
        net.roads[0].zones.insert(1, Zone { from: cityloom_model::Coordinate::new(60.0), speed_kmh: Some(30), lanes: two_lanes() });
        let (ports, ..) = test_ports();
        let store = CityStore::for_area(ports.storage, Area::new("Testville", 1.0, 2.0));
        assert!(store.keep_network_from(&net, Some(&cityloom_model::with_importer("ab12"))));
        let edge = store.open().view(0).edges[0].uid;
        let mut e = store.open().street_editor(edge, 0).unwrap();
        assert_eq!(e.snapshot().zones.len(), 3, "a zone per piece, as imported");
        let plan = e.view().plan;
        assert_eq!(plan.runs.len(), 2, "the two pieces alike are one run: {:?}", plan.runs);
        assert_eq!(plan.boundaries.len(), 1, "where the street changes, not where OSM split it");

        // Made pedestrian over the whole street: each run's motor pieces, at their own widths.
        e.pick_whole();
        let sidewalk = kind_index("sidewalk").unwrap();
        assert!(e.make_pedestrian(sidewalk));
        assert!(e.missed().is_empty(), "every run took it");
        let plan = e.view().plan;
        assert_eq!(plan.runs.len(), 2, "the runs keep their widths apart");
        assert!(plan.runs.iter().all(|r| r.bands.iter().all(|b| b.kind == sidewalk)), "{:?}", plan.runs);
        let pieces: Vec<usize> = plan.runs.iter().map(|r| r.bands.len()).collect();
        assert_eq!(pieces, [2, 4], "two lanes, then four, each now a sidewalk");

        // Kept with the importer's stamp: clearing the stale edits removes nothing, and the edit is there on reopening.
        assert!(store.write_street(edge, e.snapshot()));
        assert_eq!(store.discard_stale(), 0, "an edit made on the merged block is not stale");
        let reopened = store.open().street_editor(edge, 0).unwrap();
        assert_eq!(reopened.snapshot(), e.snapshot());
    }

    #[test]
    fn the_layout_remembers_where_its_origin_lies_in_the_network() {
        assert_eq!(City::from_network(&curving_t(), "Curves").view(0).origin_m, Some([-100.0, 100.0]));
        let sample = City::new().view(0);
        assert_eq!(sample.origin_m, None);
        assert!(sample.edges.iter().all(|e| e.shape_mm.len() == 2), "the sample's streets are straight");
    }
}
