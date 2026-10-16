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

const RENE_LEVESQUE: &[u8] = include_bytes!("rene_levesque.osm");

#[test]
fn a_bus_lane_tagged_with_hours_keeps_them() {
    use osm_import::LaneKind;
    let net = import(RENE_LEVESQUE).expect("the boulevard reads");
    assert!(!net.roads.is_empty());
    for road in &net.roads {
        let bus: Vec<_> = road.lanes.iter().filter(|l| l.kind == LaneKind::Bus).collect();
        assert_eq!(bus.len(), 1, "{:?}", road.lanes);
        assert_eq!(bus[0].hours.as_deref(), Some("Mo-Fr 06:00-10:00,14:30-19:00"), "{:?}", road.osm_ways);
        assert!(road.lanes.iter().filter(|l| l.kind != LaneKind::Bus).all(|l| l.hours.is_none()));
    }
}

#[test]
fn a_road_and_a_node_remember_the_version_of_each_osm_way_and_node_they_were_made_from() {
    let net = import(RENE_LEVESQUE).unwrap();
    for road in &net.roads {
        let want = if road.osm_ways == [4687530] { 49 } else { 26 };
        assert_eq!(road.osm_versions.get(&road.osm_ways[0]), Some(&want), "{road:?}");
        assert_eq!(road.osm_versions.len(), road.osm_ways.len());
    }
    let version = |osm: i64| net.nodes.iter().find(|n| n.osm_nodes == [osm]).map(|n| n.osm_versions.get(&osm).copied());
    assert_eq!(version(29796354), Some(Some(5)));
    assert_eq!(version(9041670850), Some(Some(1)));
}

const SIDEWALKS: &[u8] = include_bytes!("sidewalks.osm");

/// Which sides of a road have a sidewalk: (left, right), looking from its first end to its last.
fn sidewalks_of(net: &osm_import::Network, name: &str) -> (bool, bool) {
    use osm_import::LaneKind;
    let road = net.roads.iter().find(|r| r.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no road {name}"));
    let walk = |l: Option<&osm_import::Lane>| l.is_some_and(|l| l.kind == LaneKind::Sidewalk);
    (walk(road.lanes.first()), walk(road.lanes.last()))
}

#[test]
fn a_side_tagged_with_a_sidewalk_has_one_and_a_side_not_tagged_has_none() {
    let net = import(SIDEWALKS).unwrap();
    assert_eq!(sidewalks_of(&net, "left yes right no"), (true, false));
    assert_eq!(sidewalks_of(&net, "left yes only"), (true, false));
    assert_eq!(sidewalks_of(&net, "right yes only"), (false, true));
    assert_eq!(sidewalks_of(&net, "left no only"), (false, false));
    assert_eq!(sidewalks_of(&net, "left no right no"), (false, false));
}

#[test]
fn a_sidewalk_mapped_separately_is_drawn_beside_the_road() {
    let net = import(SIDEWALKS).unwrap();
    assert_eq!(sidewalks_of(&net, "left no right separate"), (false, true));
    assert_eq!(sidewalks_of(&net, "left separate right yes"), (true, true));
}

#[test]
fn a_road_with_neither_side_tagged_is_given_sidewalks_as_before() {
    let net = import(SIDEWALKS).unwrap();
    assert_eq!(sidewalks_of(&net, "nothing tagged"), (true, true), "a street with no tags is given sidewalks as before");
}

const TRACKS: &[u8] = include_bytes!("tracks.osm");

/// Whether the outermost lane on each side of a road, leaving sidewalks out, is a bike lane: (left, right).
fn bike_sides_of(net: &osm_import::Network, name: &str) -> (bool, bool) {
    use osm_import::LaneKind;
    let road = net.roads.iter().find(|r| r.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no road {name}"));
    let lanes: Vec<_> = road.lanes.iter().filter(|l| l.kind != LaneKind::Sidewalk).collect();
    (lanes.first().is_some_and(|l| l.kind == LaneKind::Bike), lanes.last().is_some_and(|l| l.kind == LaneKind::Bike))
}

#[test]
fn a_bike_track_mapped_separately_is_a_bike_lane_beside_the_road() {
    let net = import(TRACKS).unwrap();
    assert_eq!(bike_sides_of(&net, "both separate"), (true, true));
    assert_eq!(bike_sides_of(&net, "track left only"), (true, false));
    assert_eq!(bike_sides_of(&net, "track right only"), (false, true));
    // a plain `cycleway` tag is a track on a one-way road's right side, as osm2streets reads `cycleway=track` ...
    assert_eq!(bike_sides_of(&net, "cycleway separate"), (false, true));
    // ... and on both sides of a two-way road
    assert_eq!(bike_sides_of(&net, "two-way both separate"), (true, true));
    assert_eq!(bike_sides_of(&net, "two-way cycleway separate"), (true, true));
    assert_eq!(bike_sides_of(&net, "nothing tagged"), (false, false));
}

#[test]
fn a_road_with_separate_sidewalks_and_a_separate_track_has_both_on_both_sides_in_order() {
    use osm_import::LaneKind::{Bike, Buffer, Driving, Sidewalk};
    let net = import(TRACKS).unwrap();
    let road = net.roads.iter().find(|r| r.name.as_deref() == Some("both separate")).unwrap();
    let kinds: Vec<_> = road.lanes.iter().map(|l| l.kind).collect();
    assert_eq!(kinds, [Sidewalk, Bike, Buffer, Driving, Driving, Buffer, Bike, Sidewalk]);
}

#[test]
fn the_sidewalk_both_shorthand_is_read_as_the_two_sides() {
    let net = import(TRACKS).unwrap();
    assert_eq!(sidewalks_of(&net, "both separate"), (true, true));
    assert_eq!(sidewalks_of(&net, "sidewalk both no"), (false, false));
    assert_eq!(sidewalks_of(&net, "sidewalk both yes"), (true, true));
    assert_eq!(sidewalks_of(&net, "sidewalk both yes left no"), (false, true));
}

#[test]
fn a_plain_separate_sidewalk_tag_is_a_sidewalk_on_each_side() {
    let net = import(TRACKS).unwrap();
    assert_eq!(sidewalks_of(&net, "sidewalk plain separate"), (true, true));
}

const TWO_WAY_TRACKS: &[u8] = include_bytes!("two_way_tracks.osm");

/// The ways of the bike lanes of the road of that name, left to right.
fn bike_ways(net: &osm_import::Network, name: &str) -> Vec<osm_import::Way> {
    let road = net.roads.iter().find(|r| r.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no road {name}"));
    road.lanes.iter().filter(|l| l.kind == osm_import::LaneKind::Bike).map(|l| l.way).collect()
}

#[test]
fn a_track_mapped_as_two_way_cycleways_is_a_two_way_bike_lane() {
    use osm_import::Way::Both;
    let net = import(TWO_WAY_TRACKS).unwrap();
    assert_eq!(bike_ways(&net, "two-way separate left"), [Both]);
    assert_eq!(bike_ways(&net, "separate left default"), [Both], "a cycleway is two-way unless it says otherwise");
    // only the side whose cycleway is two-way
    let both_sides = bike_ways(&net, "separate both sides");
    assert_eq!((both_sides.len(), both_sides[0] == Both, both_sides[1] == Both), (2, true, false));
}

#[test]
fn a_track_mapped_as_one_way_cycleways_keeps_its_direction() {
    let net = import(TWO_WAY_TRACKS).unwrap();
    assert_eq!(bike_ways(&net, "one-way separate left").len(), 1);
    assert_ne!(bike_ways(&net, "one-way separate left"), [osm_import::Way::Both]);
}

#[test]
fn a_cycleway_that_is_not_beside_the_road_on_that_side_does_not_make_it_two_way() {
    use osm_import::Way::Both;
    let net = import(TWO_WAY_TRACKS).unwrap();
    for name in ["separate left track on the right", "separate left crossing", "separate left far away"] {
        assert!(!bike_ways(&net, name).contains(&Both), "{name}");
    }
}

#[test]
fn a_bike_lane_tagged_oneway_no_on_its_side_is_two_way() {
    use osm_import::Way::Both;
    let net = import(TWO_WAY_TRACKS).unwrap();
    assert_eq!(bike_ways(&net, "tagged two-way left"), [Both]);
    assert_eq!(bike_ways(&net, "tagged two-way both"), [Both, Both]);
    assert!(!bike_ways(&net, "tagged one-way left").contains(&Both));
}

#[test]
fn a_two_way_cycleway_beside_another_street_is_not_the_roads_track() {
    use osm_import::Way::Both;
    let net = import(TWO_WAY_TRACKS).unwrap();
    assert!(!bike_ways(&net, "no track of its own").contains(&Both));
    // the street that has the cycleway beside it gets the track as it has always had it: none, as it is not tagged
    assert!(bike_ways(&net, "neighbouring street").is_empty());
}

#[test]
fn a_path_or_footway_designated_for_bicycles_is_a_track_and_is_two_way_unless_it_says_otherwise() {
    assert_eq!(bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "designated footway"), [osm_import::Way::Both]);
}

#[test]
fn a_road_drawn_east_to_west_has_its_track_on_the_side_it_is_on() {
    assert_eq!(bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "westbound"), [osm_import::Way::Both]);
}

#[test]
fn a_track_that_says_oneway_no_for_bicycles_is_two_way_whatever_oneway_says_and_the_other_way_round() {
    assert_eq!(bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "bicycle oneway no"), [osm_import::Way::Both]);
}

#[test]
fn one_way_tracks_on_both_sides_stay_one_way() {
    let ways = bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "one-way both sides");
    assert_eq!(ways.len(), 2);
    assert!(!ways.contains(&osm_import::Way::Both), "{ways:?}");
}

#[test]
fn a_track_at_an_angle_to_the_road_is_not_its_track_and_one_covering_part_of_it_is() {
    let net = import(TWO_WAY_TRACKS).unwrap();
    assert!(!bike_ways(&net, "tilted track").contains(&osm_import::Way::Both));
    assert_eq!(bike_ways(&net, "partial track"), [osm_import::Way::Both]);
}

#[test]
fn a_two_way_road_with_oneway_no_on_a_side_has_a_two_way_lane_there() {
    assert_eq!(bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "two-way road side"), [osm_import::Way::Both]);
}

#[test]
fn a_two_way_track_beside_a_road_in_left_hand_traffic_is_a_two_way_lane() {
    let net = import(include_bytes!("left_hand_track.osm")).unwrap();
    assert!(net.left_hand);
    assert_eq!(bike_ways(&net, "left hand"), [osm_import::Way::Both]);
}

#[test]
fn a_service_road_nearer_to_the_track_does_not_take_it_from_the_street_it_is_mapped_for() {
    assert_eq!(bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "service nearer"), [osm_import::Way::Both]);
}

#[test]
fn a_parking_aisle_beside_the_track_does_not_matter() {
    assert_eq!(bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "aisle beside"), [osm_import::Way::Both]);
}

#[test]
fn a_track_beyond_the_middle_of_a_wide_road_is_still_its_track() {
    assert_eq!(bike_ways(&import(TWO_WAY_TRACKS).unwrap(), "wide road"), [osm_import::Way::Both]);
}
