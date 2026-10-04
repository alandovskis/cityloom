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

const BOULEVARD: &[u8] = include_bytes!("boulevard.osm");

#[test]
fn two_carriageways_with_a_median_between_them_are_one_street_of_both_directions() {
    let net = import(BOULEVARD).expect("the boulevard reads");
    let between: Vec<_> = net
        .roads
        .iter()
        .filter(|r| r.name.as_deref() == Some("Boulevard") && r.osm_ways.len() <= 2 && (r.osm_ways.contains(&100) || r.osm_ways.contains(&101)))
        .collect();
    assert_eq!(between.len(), 1, "{:#?}", net.roads.iter().map(|r| (&r.name, &r.osm_ways, r.from, r.to)).collect::<Vec<_>>());
    let b = between[0];
    assert!(b.osm_ways.contains(&100) && b.osm_ways.contains(&101));
    use osm_import::{LaneKind, Way};
    let driving = |way| b.lanes.iter().filter(|l| l.kind == LaneKind::Driving && l.way == way).count();
    assert!(driving(Way::Forward) >= 2 && driving(Way::Backward) >= 2, "{:?}", b.lanes);
}

const PLATEAU: &[u8] = include_bytes!("data/plateau.osm");

#[test]
fn an_import_can_be_clipped_to_a_box_and_keeps_only_what_lies_in_it() {
    let all = import(PLATEAU).unwrap();
    // 150 m either way of the middle of the extract (45.5261, -73.5978)
    let (lat, lon): (f64, f64) = (45.5261, -73.5978);
    let (dlat, dlon) = (150.0 / 111_320.0, 150.0 / (111_320.0 * lat.to_radians().cos()));
    let clipped = osm_import::import_in(PLATEAU, Some([lat - dlat, lon - dlon, lat + dlat, lon + dlon])).unwrap();
    assert!(clipped.roads.len() > 10, "{} roads", clipped.roads.len());
    assert!(clipped.roads.len() < all.roads.len() / 2, "{} of {}", clipped.roads.len(), all.roads.len());
    // nothing lies far outside the box: the data's own corner is 400 m away, the box 150 m
    let (min_x, max_x) = clipped.nodes.iter().fold((f64::MAX, f64::MIN), |(a, b), n| (a.min(n.x_m), b.max(n.x_m)));
    assert!(max_x - min_x < 400.0, "{} m wide", max_x - min_x);
    // no box is the same as `import`
    assert_eq!(osm_import::import_in(PLATEAU, None).unwrap(), all);
}

#[test]
fn a_street_with_no_sidewalk_tags_gets_a_sidewalk_on_each_side() {
    use osm_import::{LaneKind, Way};
    let net = import(CROSSING).unwrap();
    let side = net.roads.iter().find(|r| r.name.as_deref() == Some("Side Road")).unwrap();
    let sidewalks = |way| side.lanes.iter().filter(|l| l.kind == LaneKind::Sidewalk && l.way == way).count();
    assert_eq!((sidewalks(Way::Forward), sidewalks(Way::Backward)), (1, 1), "{:?}", side.lanes);
}
