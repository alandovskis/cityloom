//! A city made from a street network read from OpenStreetMap.

use std::collections::BTreeMap;

use osm_network::{Control, Lane, LaneKind, Network, Way};

use super::model::{City, EdgeDef, Layout, NodeDef};
use crate::junction::model::{ALL_WAY_STOP, MAX_ARMS, MIN_ARMS, PRIORITY, SIGNAL};
use crate::shared::catalogue::{Side, StreetClass, kind_index};
use crate::shared::provenance::OsmRef;
use crate::street::model::{Piece, Street, Window};

/// How wide a corner is rounded where nothing in the data says.
const CORNER_MM: i32 = 4_000;

/// The class of a road, by OpenStreetMap's `highway`.
fn class_of(highway: &str) -> StreetClass {
    match highway {
        "motorway" | "motorway_link" => StreetClass::Motorway,
        "trunk" | "trunk_link" | "primary" | "primary_link" => StreetClass::Arterial,
        "secondary" | "secondary_link" | "tertiary" | "tertiary_link" => StreetClass::Collector,
        _ => StreetClass::Local,
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
    let kind = |id| kind_index(id).expect("the catalogue has every kind of lane");
    let (mut base, mut variants) = (kind(kind_of(lane.kind)), Vec::new());
    // A bus lane at some hours is parking the rest of the time
    if let Some(windows) = lane.hours.as_deref().filter(|_| lane.kind == LaneKind::Bus).and_then(windows_of) {
        base = kind("parking");
        let days = days_of(lane.hours.as_deref().unwrap_or_default());
        variants = windows.into_iter().map(|(from_min, to_min)| Window { kind: kind("bus"), from_min, to_min, days: days.clone() }).collect();
    }
    Piece { kind: base, width_mm: (lane.width_m * 1000.0).round() as i32, direction: Some(usize::from(lane.way == Way::Backward)), variants }
}

const SLOT_MIN: i32 = 15;

/// What opening hours say before their times, as written: `Mo-Fr`, `Sa,Su`; nothing for every day.
fn days_of(hours: &str) -> Option<String> {
    let days: Vec<&str> = hours.split_whitespace().filter(|t| !t.contains(':')).collect();
    (!days.is_empty()).then(|| days.join(" "))
}

/// The windows of the day, in minutes (from, to) with midnight at the end of a window as 0, that OSM
/// opening hours name: `Mo-Fr 06:00-10:00,14:30-19:00`. The days are for `days_of`; a time to the minute:
/// a window starts on the quarter hour before and ends on the one after. Hours with several rules, an `off`,
/// a window past midnight or two that overlap are not read.
fn windows_of(hours: &str) -> Option<Vec<(i32, i32)>> {
    let time = |t: &str| {
        let (h, m) = t.split_once(':')?;
        let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
        (h < 24 && m < 60 || (h, m) == (24, 0)).then_some(h * 60 + m)
    };
    if hours.contains(';') || hours.split_whitespace().any(|t| t.eq_ignore_ascii_case("off")) {
        return None;
    }
    let mut windows = Vec::new();
    for range in hours.split(|c: char| c.is_whitespace() || c == ',').filter(|r| r.contains(':')) {
        let (from, to) = range.split_once('-')?;
        let (from, to) = (time(from)? / SLOT_MIN * SLOT_MIN, (time(to)? + SLOT_MIN - 1) / SLOT_MIN * SLOT_MIN);
        if to <= from {
            return None;
        }
        windows.push((from, to));
    }
    windows.sort();
    if windows.is_empty() || windows.windows(2).any(|w| w[1].0 < w[0].1) {
        return None;
    }
    Some(windows.into_iter().map(|(from, to)| (from, to % (24 * 60))).collect())
}

/// The OSM ways or nodes with the version each had.
fn refs(ids: &[i64], versions: &BTreeMap<i64, i32>) -> Vec<OsmRef> {
    ids.iter().map(|&id| OsmRef { id, version: versions.get(&id).copied() }).collect()
}

fn control_of(c: Control) -> usize {
    match c {
        Control::Signals => SIGNAL,
        Control::Signs => ALL_WAY_STOP,
        Control::None => PRIORITY,
    }
}

/// How far along a road to look for the way it leaves a junction: past the rounding of its corner.
const LOOK_M: f64 = 20.0;

/// The way a road leaves its first point and its last, in degrees clockwise from north, from a point
/// about `LOOK_M` along it (or its far end, if it is shorter).
fn headings(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    let along = |line: &[(f64, f64)]| -> Option<f64> {
        let origin = *line.first()?;
        let mut run = 0.0;
        let mut at = origin;
        for &p in &line[1..] {
            run += (p.0 - at.0).hypot(p.1 - at.1);
            at = p;
            if run >= LOOK_M {
                break;
            }
        }
        let (dx, dy) = (at.0 - origin.0, at.1 - origin.1);
        (dx != 0.0 || dy != 0.0).then(|| dx.atan2(dy).to_degrees().rem_euclid(360.0))
    };
    let reversed: Vec<(f64, f64)> = points.iter().rev().copied().collect();
    Some((along(points)?, along(&reversed)?))
}

/// A road's centreline in layout millimetres with its two ends set to where its nodes are, so that a street
/// meets its junctions; None for a road with fewer than two points.
fn shape_mm(points: &[(f64, f64)], from: (i32, i32), to: (i32, i32), mm_of: &impl Fn(f64, f64) -> (i32, i32)) -> Option<Vec<(i32, i32)>> {
    if points.len() < 2 {
        return None;
    }
    let mut line: Vec<(i32, i32)> = points.iter().map(|&(x, y)| mm_of(x, y)).collect();
    let last = line.len() - 1;
    line[0] = from;
    line[last] = to;
    Some(line)
}

impl City {
    /// The city for `network`, every street and junction as OpenStreetMap has it.
    pub fn from_network(network: &Network, name: &str) -> City {
        City::on(Layout::from_network(network, name))
    }
}

impl Layout {
    /// The layout of a network: a node for each end or meeting of streets, a street for each road.
    /// Roads that start and end at one node, that have no lanes, or that start or end at a node the
    /// network lacks are left out, and so is a node nothing leads to.
    pub fn from_network(network: &Network, name: &str) -> Layout {
        let roads: Vec<_> = network.roads.iter().filter(|r| r.from != r.to && !r.lanes.is_empty()).collect();
        let mut degree: BTreeMap<u32, usize> = BTreeMap::new();
        for r in &roads {
            *degree.entry(r.from).or_default() += 1;
            *degree.entry(r.to).or_default() += 1;
        }
        let nodes: Vec<_> = network.nodes.iter().filter(|n| degree.contains_key(&n.id)).collect();
        let index: BTreeMap<u32, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let roads: Vec<_> = roads.into_iter().filter(|r| index.contains_key(&r.from) && index.contains_key(&r.to)).collect();
        let (min_x, max_y) = nodes.iter().fold((f64::MAX, f64::MIN), |(x, y), n| (x.min(n.x_m), y.max(n.y_m)));
        let origin_m = (!nodes.is_empty()).then_some((min_x, max_y));
        let mm_of = |x_m: f64, y_m: f64| (((x_m - min_x) * 1000.0).round() as i32, ((max_y - y_m) * 1000.0).round() as i32);
        let node_mm = |id: u32| {
            let n = nodes[index[&id]];
            mm_of(n.x_m, n.y_m)
        };
        let edges: Vec<EdgeDef> = roads
            .iter()
            .map(|r| {
                let class = class_of(&r.highway);
                let pieces: Vec<Piece> = r.lanes.iter().map(piece).collect();
                let side = if network.left_hand { Side::Left } else { Side::Right };
                EdgeDef {
                    a: index[&r.from],
                    b: index[&r.to],
                    class,
                    name: r.name.clone(),
                    headings: headings(&r.points),
                    shape: shape_mm(&r.points, node_mm(r.from), node_mm(r.to), &mm_of),
                    section: Street { name: r.name.clone(), source: refs(&r.osm_ways, &r.osm_versions), ..Street::imported(class, side, &pieces) },
                }
            })
            .collect();

        let nodes = nodes
            .iter()
            .map(|n| {
                let junction = n.junction && (MIN_ARMS..=MAX_ARMS).contains(&degree[&n.id]);
                NodeDef {
                    x_mm: ((n.x_m - min_x) * 1000.0).round() as i32,
                    y_mm: ((max_y - n.y_m) * 1000.0).round() as i32,
                    junction,
                    control: if junction { control_of(n.control) } else { 0 },
                    corner_mm: if junction { CORNER_MM } else { 0 },
                    source: refs(&n.osm_nodes, &n.osm_versions),
                }
            })
            .collect();
        Layout { name: name.to_string(), side: if network.left_hand { Side::Left } else { Side::Right }, nodes, edges, origin_m }
    }
}

#[cfg(test)]
mod tests {
    use osm_network::{Control, Lane, LaneKind, Node, Road, Way};

    use super::*;
    use crate::junction::model::CONTROLS;
    use crate::junction::model::SIGNAL;
    use crate::shared::catalogue::KINDS;
    use crate::shared::i18n::Locale;
    use crate::shared::said::{Arg, Said, say_now};
    use crate::shared::units::Units;

    fn en(said: &Said) -> String {
        say_now(&crate::i18n_for(Locale::En), Units::Metres, said)
    }

    fn fr(said: &Said) -> String {
        say_now(&crate::i18n_for(Locale::FrCa), Units::Metres, said)
    }

    fn lane(kind: LaneKind, way: Way, width_m: f64) -> Lane {
        Lane { kind, way, width_m, hours: None }
    }

    fn bus_lane(hours: Option<&str>) -> Lane {
        Lane { hours: hours.map(str::to_string), ..lane(LaneKind::Bus, Way::Forward, 3.2) }
    }

    fn windows(p: &Piece) -> Vec<(&'static str, i32, i32)> {
        p.variants.iter().map(|w| (KINDS[w.kind].id, w.from_min, w.to_min)).collect()
    }

    #[test]
    fn the_days_of_the_hours_go_with_each_window() {
        let days = |hours| piece(&bus_lane(Some(hours))).variants.iter().map(|w| w.days.clone()).collect::<Vec<_>>();
        assert_eq!(days("Mo-Fr 06:00-10:00,14:30-19:00"), [Some("Mo-Fr".to_string()), Some("Mo-Fr".to_string())]);
        assert_eq!(days("Sa,Su 08:00-12:00"), [Some("Sa,Su".to_string())]);
        assert_eq!(days("06:00-10:00"), [None]);
    }

    #[test]
    fn a_street_and_a_junction_carry_the_osm_ways_and_nodes_they_were_made_from_with_their_versions() {
        use crate::shared::provenance::OsmRef;
        let mut net = crossing();
        net.roads[1].osm_versions = [(200, 12)].into();
        net.nodes[0].osm_versions = [(10, 4)].into();
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

    #[test]
    fn a_city_saved_before_places_carried_their_source_has_only_what_was_changed_marked_changed() {
        let net = crossing();
        let mut city = City::from_network(&net, "Testville");
        let edge = city.view(0).edges[0].uid;
        let mut e = city.street_editor(edge, 0).unwrap();
        let u = e.view().segments[1].uid;
        assert!(e.nudge_width(u, 100));
        assert!(city.keep_street(edge, e.snapshot()));
        // the save as an older version wrote it: none of its places has a source
        let mut saved: serde_json::Value = serde_json::from_str(&city.save()).unwrap();
        fn strip(v: &mut serde_json::Value) {
            match v {
                serde_json::Value::Object(m) => {
                    m.remove("source");
                    m.values_mut().for_each(strip);
                }
                serde_json::Value::Array(a) => a.iter_mut().for_each(strip),
                _ => {}
            }
        }
        strip(&mut saved);
        let loaded = City::load_on(Layout::from_network(&net, "Testville"), &saved.to_string());
        let v = loaded.view(0);
        assert_eq!((v.edited, v.edges[0].edited), (1, true));
        let source = loaded.street_editor(edge, 0).unwrap().view().source;
        assert_eq!(source.len(), 1);
    }

    #[test]
    fn a_bus_lane_with_hours_is_parking_outside_them() {
        let p = piece(&bus_lane(Some("Mo-Fr 06:00-10:00,14:30-19:00")));
        assert_eq!(KINDS[p.kind].id, "parking");
        assert_eq!(windows(&p), [("bus", 360, 600), ("bus", 870, 1140)]);
    }

    #[test]
    fn a_bus_lane_without_hours_or_with_hours_that_cannot_be_read_stays_a_bus_lane() {
        for hours in [None, Some("Sa,Su 07:00-09:00; Mo-Fr 06:00-10:00"), Some("sunrise-sunset"), Some("Mo-Fr")] {
            let p = piece(&bus_lane(hours));
            assert_eq!((KINDS[p.kind].id, p.variants.len()), ("bus", 0), "{hours:?}");
        }
    }

    #[test]
    fn hours_are_read_as_windows_of_quarter_hours_and_a_day_ends_at_midnight() {
        assert_eq!(windows_of("07:05-09:50"), Some(vec![(420, 600)]));
        assert_eq!(windows_of("Mo-Fr 16:00-24:00"), Some(vec![(960, 0)]));
        assert_eq!(windows_of("Mo-Fr 10:00-12:00,06:00-08:00"), Some(vec![(360, 480), (600, 720)]));
    }

    #[test]
    fn hours_that_overlap_or_run_past_midnight_or_say_off_are_not_read() {
        for hours in ["06:00-10:00,09:00-12:00", "22:00-02:00", "Mo-Fr off", "10:00-10:00", ""] {
            assert_eq!(windows_of(hours), None, "{hours:?}");
        }
    }

    /// Two streets crossing at a signalled junction, with a dead end on each arm.
    fn crossing() -> Network {
        let node =
            |id, x_m, y_m, junction, control| Node { id, osm_nodes: vec![id as i64 * 10], osm_versions: Default::default(), x_m, y_m, junction, control };
        let road = |id, name: &str, to, points| Road {
            id,
            osm_ways: vec![id as i64 * 100],
            osm_versions: Default::default(),
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
        let main = v.edges.iter().find(|e| en(&e.kind) == "Main Street").expect("named for the road");
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
        net.nodes.truncate(1);
        net.roads.clear();
        for (i, &(x, y)) in ends.iter().enumerate() {
            let id = i as u32 + 2;
            net.nodes.push(Node { id, osm_nodes: vec![id as i64], osm_versions: Default::default(), x_m: x, y_m: y, junction: false, control: Control::None });
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
    fn a_road_is_classed_by_its_highway() {
        use crate::shared::catalogue::StreetClass::*;
        for (highway, class) in [
            ("motorway", Motorway),
            ("motorway_link", Motorway),
            ("trunk", Arterial),
            ("primary_link", Arterial),
            ("secondary", Collector),
            ("tertiary_link", Collector),
            ("residential", Local),
            ("service", Local),
            ("footway", Local),
        ] {
            assert_eq!(class_of(highway), class, "{highway}");
        }
    }

    #[test]
    fn a_road_without_a_name_keeps_none_and_is_titled_by_its_class_in_each_language() {
        let mut net = crossing();
        net.roads[0].name = None;
        net.roads[1].name = None;
        net.roads[1].highway = "motorway".into();
        let layout = Layout::from_network(&net, "Nameless");
        let city = City::on(Layout::from_network(&net, "Nameless"));
        let edge = |road: u32| layout.edges.iter().position(|e| e.section.source.first().is_some_and(|s| s.id == road as i64 * 100)).unwrap();
        let (residential, motorway) = (edge(1), edge(2));
        assert_eq!(layout.edges[residential].name, None);
        assert_eq!(layout.edges[residential].section.name, None, "the stored network holds no wording");
        let kind = &city.view(0).edges[residential].kind;
        assert_eq!((en(kind), fr(kind)), ("Unnamed local street".into(), "Rue locale sans nom".into()));
        let kind = &city.view(0).edges[motorway].kind;
        assert_eq!((en(kind), fr(kind)), ("Unnamed motorway".into(), "Autoroute sans nom".into()));
        assert!(!city.save().contains("Unnamed"));
        // a save from before kept the English it named an unnamed street with; the name comes from the network
        let mut city = City::on(Layout::from_network(&net, "Nameless"));
        let mut e = city.street_editor(residential as u32 + 1, 0).unwrap();
        let piece = e.view().segments[0].uid;
        e.nudge_width(piece, -100);
        assert!(city.keep_street(residential as u32 + 1, e.snapshot()));
        let old = city.save().replacen("\"name\":null", "\"name\":\"Unnamed residential\"", 1);
        assert!(old.contains("Unnamed residential"));
        let back = City::load_on(Layout::from_network(&net, "Nameless"), &old);
        assert!(back.view(0).edges[residential].edited, "the edit is kept");
        assert_eq!(en(&back.street_editor(residential as u32 + 1, 0).unwrap().view().name), "Unnamed local street");
        // a street without a name is no part of a place's title
        let ends: Vec<String> = city.view(0).nodes.iter().filter(|n| !n.junction).map(|n| en(&n.name)).collect();
        assert!(ends.contains(&"Edge of the map".to_string()) && ends.contains(&"End of Side Road".to_string()), "{ends:?}");
    }

    #[test]
    fn a_road_with_no_lanes_is_not_a_street() {
        let mut net = crossing();
        net.roads[0].lanes.clear();
        let v = City::from_network(&net, "Bare").view(0);
        assert_eq!(v.edges.len(), 3, "the lane-less road is left out");
    }

    #[test]
    fn a_motorway_is_a_freeway_and_a_residential_street_is_not() {
        let mut net = crossing();
        net.roads[1].highway = "motorway".into();
        let v = City::from_network(&net, "Fast").view(0);
        assert_eq!(v.edges.iter().filter(|e| e.freeway).count(), 1);
    }

    fn bare_street() -> Network {
        let mut net = star(&[(100.0, 0.0)]);
        net.roads[0].lanes = vec![lane(LaneKind::Driving, Way::Backward, 3.0), lane(LaneKind::Driving, Way::Forward, 3.0)];
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
        let main = v.edges.iter().find(|e| en(&e.kind) == "Main Street").unwrap();
        assert_eq!(main.kind, Said::new("city-name").with("name", Arg::Text("Main Street".into())));
        assert_eq!(city.street_editor(main.uid, 0).unwrap().view().name, Street::imported(StreetClass::Local, Side::Right, &[]).named("Main Street").title());
        let junction = v.nodes.iter().find(|n| n.junction).unwrap();
        let arms: Vec<String> = city.junction_editor(junction.uid, 0).unwrap().view().arms.iter().map(|a| a.street.clone()).collect();
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
        net.roads[0].points = vec![(0.0, 0.0), (-30.0, 0.0), (-100.0, 100.0)];
        net.roads[1].points = vec![(0.0, 0.0), (30.0, 0.0), (100.0, 100.0)];
        net.roads[2].points = vec![(0.0, 0.0), (0.0, -30.0), (60.0, -30.0), (60.0, 50.0)];
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
        for points in [vec![], vec![(0.0, 0.0)]] {
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

    #[test]
    fn the_layout_remembers_where_its_origin_lies_in_the_network() {
        assert_eq!(City::from_network(&curving_t(), "Curves").view(0).origin_m, Some([-100.0, 100.0]));
        let sample = City::new().view(0);
        assert_eq!(sample.origin_m, None);
        assert!(sample.edges.iter().all(|e| e.shape_mm.len() == 2), "the sample's streets are straight");
    }
}
