use serde::{Deserialize, Serialize};

use crate::{Lane, LaneKind, Network, RoadId};

/// Who a corridor serves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserClass {
    Pedestrian,
    Cyclist,
    Transit,
    Driver,
}

impl UserClass {
    pub const ALL: [UserClass; 4] = [UserClass::Pedestrian, UserClass::Cyclist, UserClass::Transit, UserClass::Driver];

    /// Whether this kind of lane is for the user class.
    pub fn uses(self, kind: LaneKind) -> bool {
        matches!(
            (self, kind),
            (UserClass::Pedestrian, LaneKind::Sidewalk)
                | (UserClass::Cyclist, LaneKind::Bike)
                | (UserClass::Transit, LaneKind::Bus)
                | (UserClass::Driver, LaneKind::Driving)
        )
    }

    /// Whether the lane is for the user class at any hour: by its own kind or in one of its windows.
    pub fn serves(self, lane: &Lane) -> bool {
        self.uses(lane.kind) || lane.windows.iter().any(|w| self.uses(w.kind))
    }
}

/// A collection of roads that serve one class of user, joined end to end: from any of its roads to any other there
/// is a way along its roads, meeting at their nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Corridor {
    pub user: UserClass,
    /// Its roads, in order, none twice; at least one.
    pub roads: Vec<RoadId>,
}

impl Network {
    /// Replaces the network's corridors by the ones its lanes make: for each user class, the roads with a lane for
    /// that class, in groups that meet at nodes. Ordered by user class, then by the lowest road id of each.
    /// A road a class has no lane on is in no corridor of that class, even where the class could go along it.
    pub fn derive_corridors(&mut self) {
        let mut corridors = Vec::new();
        for user in UserClass::ALL {
            let serving: Vec<&crate::Road> = self.roads.iter().filter(|r| r.zones.iter().flat_map(|z| &z.lanes).any(|l| user.serves(l))).collect();
            let edges: Vec<crate::Edge> = serving.iter().map(|r| r.edge()).collect();
            let found = crate::connected(&edges).into_iter().map(|roads| Corridor { user, roads });
            corridors.extend(found);
        }
        self.corridors = corridors;
    }

    /// The corridors that serve `user`.
    pub fn corridors_of(&self, user: UserClass) -> impl Iterator<Item = &Corridor> {
        self.corridors.iter().filter(move |c| c.user == user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Control, Direction, Junction, NodeId, Point, Provenance, Road, RoadType, Zone};

    fn lane(kind: LaneKind) -> Lane {
        Lane { kind, direction: Direction::Forward, width_m: 2.0, buffer_type: None, windows: vec![], shape: vec![] }
    }

    fn road(id: u32, from: u32, to: u32, lanes: &[LaneKind]) -> Road {
        Road {
            id: RoadId(id),
            provenance: Provenance::default(),
            name: "Street".into(),
            road_type: RoadType::Local,
            from: NodeId(from),
            to: NodeId(to),
            zones: vec![Zone::whole(lanes.iter().map(|&k| lane(k)).collect())],
            speed_kmh: None,
            points: vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
        }
    }

    fn network(roads: Vec<Road>) -> Network {
        let nodes = (1..=6)
            .map(|id| Junction { id: NodeId(id), provenance: Provenance::default(), x_m: 0.0, y_m: 0.0, junction: false, control: Control::None })
            .collect();
        Network { left_hand: false, junctions: nodes, roads, corridors: vec![] }
    }

    fn roads_of(net: &Network, user: UserClass) -> Vec<Vec<u32>> {
        net.corridors_of(user).map(|c| c.roads.iter().map(|r| r.0).collect()).collect()
    }

    #[test]
    fn roads_that_meet_and_serve_a_class_are_one_corridor_and_others_are_apart() {
        use LaneKind::*;
        // 1-2-3 in a row with sidewalks, 4-5 apart with sidewalks, 6 (to node 3) with none; a bike lane on 2 and 6
        let mut net = network(vec![
            road(1, 1, 2, &[Sidewalk, Driving, Sidewalk]),
            road(2, 2, 3, &[Sidewalk, Bike, Driving, Sidewalk]),
            road(3, 4, 5, &[Sidewalk, Driving]),
            road(6, 3, 6, &[Bike, Driving]),
        ]);
        net.derive_corridors();
        assert_eq!(roads_of(&net, UserClass::Pedestrian), [vec![1, 2], vec![3]]);
        assert_eq!(roads_of(&net, UserClass::Cyclist), [vec![2, 6]]);
        assert_eq!(roads_of(&net, UserClass::Driver), [vec![1, 2, 6], vec![3]]);
        assert_eq!(roads_of(&net, UserClass::Transit), Vec::<Vec<u32>>::new());
    }

    #[test]
    fn a_road_that_joins_two_groups_makes_them_one() {
        use LaneKind::*;
        let mut net = network(vec![road(1, 1, 2, &[Driving]), road(2, 3, 4, &[Driving]), road(3, 2, 3, &[Driving])]);
        net.derive_corridors();
        assert_eq!(roads_of(&net, UserClass::Driver), [vec![1, 2, 3]]);
    }

    #[test]
    fn a_lane_that_is_a_bus_lane_at_some_hours_serves_transit() {
        let mut timed = lane(LaneKind::Bus);
        assert!(timed.only_during("Mo-Fr 06:00-10:00", LaneKind::Parking));
        let mut r = road(1, 1, 2, &[]);
        r.zones[0].lanes = vec![timed];
        let mut net = network(vec![r]);
        net.derive_corridors();
        assert_eq!(roads_of(&net, UserClass::Transit), [vec![1]]);
        assert_eq!(roads_of(&net, UserClass::Driver), Vec::<Vec<u32>>::new());
    }

    #[test]
    fn corridors_are_kept_only_when_there_are_some_and_lose_roads_that_are_gone() {
        let mut net = network(vec![road(1, 1, 2, &[LaneKind::Driving])]);
        let json = serde_json::to_string(&net).unwrap();
        assert!(!json.contains("corridors"), "{json}");
        net.derive_corridors();
        let json = serde_json::to_string(&net).unwrap();
        assert!(json.contains(r#""corridors":[{"user":"driver","roads":[1]}]"#), "{json}");
        assert_eq!(serde_json::from_str::<Network>(&json).unwrap(), net);
        // a corridor naming a road the network does not have loses it, and one left with none is dropped
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut value = value;
        value["corridors"] = serde_json::json!([{"user": "driver", "roads": [1, 9]}, {"user": "cyclist", "roads": [9]}]);
        let read: Network = serde_json::from_value(value).unwrap();
        assert_eq!(read.corridors, [Corridor { user: UserClass::Driver, roads: vec![RoadId(1)] }]);
    }
}
