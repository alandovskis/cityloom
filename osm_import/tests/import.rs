use osm_import::{Control, import};

const CROSSING: &[u8] = include_bytes!("crossing.osm");

#[test]
fn a_crossing_of_two_streets_is_one_signalled_junction_with_four_roads() {
    let net = import(CROSSING).expect("the crossing reads");
    let junctions: Vec<_> = net.nodes.iter().filter(|n| n.junction).collect();
    assert_eq!(junctions.len(), 1, "{:?}", net.nodes);
    assert_eq!(junctions[0].control, Control::Signals);
    assert_eq!(junctions[0].osm_nodes, vec![2]);
    assert_eq!(net.roads.len(), 4);
    let mut names: Vec<_> = net.roads.iter().filter_map(|r| r.name.clone()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names, ["Main Street", "Side Road"]);
}

#[test]
fn a_road_keeps_its_lanes_and_the_osm_way_it_came_from() {
    let net = import(CROSSING).unwrap();
    let main = net.roads.iter().find(|r| r.name.as_deref() == Some("Main Street")).unwrap();
    assert_eq!(main.osm_ways, vec![10]);
    assert_eq!(main.highway, "secondary");
    assert!(main.lanes.iter().filter(|l| l.kind == osm_import::LaneKind::Driving).count() >= 2);
    assert!(main.lanes.iter().all(|l| l.width_m > 0.0));
    assert!(main.points.len() >= 2);
}

#[test]
fn a_crossing_in_london_keeps_to_the_left() {
    assert!(import(CROSSING).unwrap().left_hand);
}
