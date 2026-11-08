use serde::{Deserialize, Serialize};

use crate::{Edge, Lane, Point, Provenance, RoadType, Street, Zone};

/// A street between two nodes, made from one or more OpenStreetMap ways. A road has a name: the importer leaves out
/// one without, or with a blank one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    /// Its id and the nodes it joins, from the first to the last.
    pub edge: Edge,
    /// The OSM ways it was made from, each with its version when known; the key an edit is kept under.
    pub provenance: Provenance,
    pub name: String,
    /// How the road ranks, from OSM's `highway` value.
    pub road_type: RoadType,
    /// The road divided lengthways, in order from `edge.from`: at least one, the first starting at 0 and each one
    /// after further along than the one before. A road made from one OSM way has one.
    pub zones: Vec<Zone>,
    /// The road's centre line.
    pub points: Vec<Point>,
}

impl Road {
    /// Whether the road was made from the way with this OSM id, whatever version of it.
    /// Whether `name` can name a road: it is there and is not blank.
    pub fn is_name(name: Option<&str>) -> bool {
        name.is_some_and(|n| !n.trim().is_empty())
    }

    /// The lanes of the first zone: all of them for a road of one zone, which is what the importer makes.
    pub fn base_lanes(&self) -> &[Lane] {
        self.zones.first().map_or(&[], |z| &z.lanes)
    }

    /// The first zone's lanes, to change. A road has a first zone, so this panics for one built with none.
    pub fn base_lanes_mut(&mut self) -> &mut Vec<Lane> {
        &mut self.zones.first_mut().expect("a road has a zone").lanes
    }

    /// Whether `zones` is as a road needs: one at least, the first at 0, each after further along, all finite.
    pub fn is_zoned(zones: &[Zone]) -> bool {
        zones.first().is_some_and(|z| z.from.metres() == 0.0)
            && zones.windows(2).all(|w| w[1].from.metres() > w[0].from.metres())
            && zones.iter().all(|z| z.from.metres().is_finite())
    }

    /// The length of the centre line, in metres.
    pub fn length_m(&self) -> f64 {
        self.points.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// Gives every lane the shape it has by lying straight along its zone, at its place across the road: the lanes
    /// are side by side, centred on the centre line, the first at the left. The shape runs from the zone's start to
    /// where the next zone starts, or the road's end. Done again whenever the lanes change.
    pub fn lay_out_lanes(&mut self) {
        let length = self.length_m();
        let ends: Vec<f64> = self.zones.iter().skip(1).map(|z| z.from.metres()).chain([length]).collect();
        for (zone, end) in self.zones.iter_mut().zip(ends) {
            let (from, end) = (zone.from.metres(), end.max(zone.from.metres()));
            let mut left = zone.lanes.iter().map(|l| l.width_m).sum::<f64>() / 2.0;
            for lane in &mut zone.lanes {
                let across = left - lane.width_m / 2.0;
                left -= lane.width_m;
                lane.shape = vec![Point::<Street>::new(from, across), Point::new(end, across)];
            }
        }
    }

    /// The speed limit of the first zone, which is the whole road's for a road of one zone.
    pub fn base_speed_kmh(&self) -> Option<u16> {
        self.zones.first().and_then(|z| z.speed_kmh)
    }

    pub fn has_way(&self, id: i64) -> bool {
        self.provenance.has_way(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coordinate, NodeId, RoadId, WayRef};

    fn road_of(zones: Vec<Zone>) -> Road {
        Road {
            edge: Edge { id: RoadId(1), from: NodeId(1), to: NodeId(2) },
            provenance: Provenance::of_ways([WayRef::with_version(5, 49), WayRef::new(7)]),
            name: "Main".into(),
            road_type: RoadType::Local,
            zones,
            points: vec![Point::new(0.0, 1.0)],
        }
    }

    fn lane(width_m: f64) -> Lane {
        Lane { kind: crate::LaneKind::Driving, direction: crate::Direction::Forward, width_m, buffer_type: None, windows: vec![], shape: vec![] }
    }

    #[test]
    fn a_road_is_kept_with_its_edge_provenance_and_zones() {
        let mut road = road_of(vec![Zone::whole(vec![lane(3.0)]), Zone { from: Coordinate::new(40.0), speed_kmh: Some(30), lanes: vec![lane(3.0)] }]);
        road.zones[0].speed_kmh = Some(50);
        let json = serde_json::to_value(&road).unwrap();
        assert_eq!(json["edge"], serde_json::json!({"id": 1, "from": 1, "to": 2}));
        assert_eq!(json["provenance"], serde_json::json!([{"way": {"id": 5, "version": 49}}, {"way": {"id": 7}}]));
        assert_eq!(json["zones"][0]["speed_kmh"], 50);
        assert_eq!(serde_json::from_value::<Road>(json).unwrap(), road);
    }

    #[test]
    fn a_zone_with_no_speed_limit_says_none() {
        let json = serde_json::to_value(Zone::whole(vec![])).unwrap();
        assert!(json.get("speed_kmh").is_none(), "{json}");
    }

    #[test]
    fn the_speed_limit_of_a_road_is_its_first_zones() {
        let mut road = road_of(vec![Zone::whole(vec![]), Zone { from: Coordinate::new(10.0), speed_kmh: Some(30), lanes: vec![] }]);
        assert_eq!(road.base_speed_kmh(), None);
        road.zones[0].speed_kmh = Some(50);
        assert_eq!(road.base_speed_kmh(), Some(50));
    }

    #[test]
    fn a_road_whose_zones_do_not_start_at_0_and_run_on_in_order_is_not_zoned() {
        for zones in [
            vec![],
            vec![Zone { from: Coordinate::new(5.0), speed_kmh: None, lanes: vec![] }],
            vec![Zone::whole(vec![]), Zone { from: Coordinate::new(0.0), speed_kmh: None, lanes: vec![] }],
            vec![Zone::whole(vec![]), Zone { from: Coordinate::new(f64::NAN), speed_kmh: None, lanes: vec![] }],
        ] {
            assert!(!Road::is_zoned(&zones), "{zones:?}");
        }
    }

    #[test]
    fn lanes_are_laid_out_side_by_side_along_their_zone() {
        let mut road = road_of(vec![Zone::whole(vec![lane(3.0), lane(1.0)]), Zone { from: Coordinate::new(60.0), speed_kmh: None, lanes: vec![lane(2.0)] }]);
        road.points = vec![Point::new(0.0, 0.0), Point::new(0.0, 100.0)];
        road.lay_out_lanes();
        let shapes: Vec<Vec<(f64, f64)>> = road.zones.iter().flat_map(|z| &z.lanes).map(|l| l.shape.iter().map(|p| (p.x_m(), p.y_m())).collect()).collect();
        // 4 m wide: the 3 m lane is centred 0.5 m left of the centre line, the 1 m lane 1.5 m right of it
        assert_eq!(shapes, [vec![(0.0, 0.5), (60.0, 0.5)], vec![(0.0, -1.5), (60.0, -1.5)], vec![(60.0, 0.0), (100.0, 0.0)]]);
    }

    #[test]
    fn a_name_is_there_and_not_blank() {
        assert!(!Road::is_name(None));
        assert!(!Road::is_name(Some("  ")));
        assert!(Road::is_name(Some("Main")));
    }
}
