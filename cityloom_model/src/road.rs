use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::{Lane, Point, RoadType, WayRef, Zone};

/// A street between two nodes, made from one or more OpenStreetMap ways.
///
/// Kept as a road always was: the ways as bare ids in `osm_ways` and their versions in `osm_versions`, only for
/// the ways whose version is known, so a network kept by an earlier build still reads. A road has a name: one
/// without, or with a blank one, is not a road (`Road::try_from` refuses it, and so does reading one by itself).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RoadWire", into = "RoadWire")]
pub struct Road {
    pub id: u32,
    /// The OSM ways it was made from, each with its version when known; the key an edit is kept under.
    pub osm_ways: Vec<WayRef>,
    pub name: String,
    /// How the road ranks, from OSM's `highway` value.
    pub road_type: RoadType,
    pub from: u32,
    pub to: u32,
    /// The road divided lengthways, in order from `from`: at least one, the first starting at 0 and each one after
    /// further along than the one before. A road made from one OSM way has one.
    pub zones: Vec<Zone>,
    /// OSM's `maxspeed` in km/h; none when the way gives none.
    pub speed_kmh: Option<u16>,
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

    pub fn has_way(&self, id: i64) -> bool {
        self.osm_ways.iter().any(|w| w.id == id)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RoadWire {
    id: u32,
    osm_ways: Vec<i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    osm_versions: BTreeMap<i64, i32>,
    name: Option<String>,
    #[serde(alias = "highway")]
    road_type: RoadType,
    from: u32,
    to: u32,
    /// Written for a road of one zone, as `lanes` always was.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    lanes: Vec<Lane>,
    /// Written only for a road of more than one zone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    zones: Vec<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speed_kmh: Option<u16>,
    points: Vec<Point>,
}

/// The roads of a kept network, leaving out any that have no name: such a network may have been kept before a road
/// needed one.
pub(crate) fn named_roads<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Road>, D::Error> {
    let wires = Vec::<RoadWire>::deserialize(d)?;
    Ok(wires.into_iter().filter_map(|w| Road::try_from(w).ok()).collect())
}

impl TryFrom<RoadWire> for Road {
    type Error = &'static str;

    fn try_from(w: RoadWire) -> Result<Road, Self::Error> {
        if !Road::is_name(w.name.as_deref()) {
            return Err("a road needs a name");
        }
        let name = w.name.unwrap_or_default();
        let zones = if w.zones.is_empty() { vec![Zone::whole(w.lanes)] } else { w.zones };
        if !Road::is_zoned(&zones) {
            return Err("a road's zones start at 0 and run on in order");
        }
        let osm_ways = w.osm_ways.iter().map(|&id| WayRef { id, version: w.osm_versions.get(&id).copied() }).collect();
        Ok(Road { id: w.id, osm_ways, name, road_type: w.road_type, from: w.from, to: w.to, zones, speed_kmh: w.speed_kmh, points: w.points })
    }
}

impl From<Road> for RoadWire {
    fn from(r: Road) -> RoadWire {
        let (lanes, zones) = match <[Zone; 1]>::try_from(r.zones) {
            Ok([whole]) => (whole.lanes, Vec::new()),
            Err(zones) => (Vec::new(), zones),
        };
        let osm_versions = r.osm_ways.iter().filter_map(|w| Some((w.id, w.version?))).collect();
        RoadWire {
            id: r.id,
            osm_ways: r.osm_ways.iter().map(|w| w.id).collect(),
            osm_versions,
            name: Some(r.name),
            road_type: r.road_type,
            from: r.from,
            to: r.to,
            lanes,
            zones,
            speed_kmh: r.speed_kmh,
            points: r.points,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Coordinate;

    #[test]
    fn a_road_is_kept_with_its_ways_and_their_versions_apart_as_it_always_was() {
        let road = Road {
            id: 1,
            osm_ways: vec![WayRef::with_version(5, 49), WayRef::new(7)],
            name: "Main".into(),
            road_type: RoadType::Local,
            from: 1,
            to: 2,
            zones: vec![Zone::whole(vec![])],
            speed_kmh: None,
            points: vec![Point::new(0.0, 1.0)],
        };
        let json = serde_json::to_string(&road).unwrap();
        assert!(json.contains(r#""osm_ways":[5,7]"#) && json.contains(r#""osm_versions":{"5":49}"#), "{json}");
        assert_eq!(serde_json::from_str::<Road>(&json).unwrap(), road);
    }

    fn road_of(zones: Vec<Zone>) -> Road {
        Road { id: 1, osm_ways: vec![WayRef::new(5)], name: "Main".into(), road_type: RoadType::Local, from: 1, to: 2, zones, speed_kmh: None, points: vec![] }
    }

    fn lane(width_m: f64) -> Lane {
        Lane { kind: crate::LaneKind::Driving, way: crate::Way::Forward, width_m, buffer_type: None, hours: None }
    }

    #[test]
    fn a_road_of_one_zone_is_kept_as_lanes_and_one_of_several_as_zones() {
        let one = road_of(vec![Zone::whole(vec![lane(3.0)])]);
        let json = serde_json::to_string(&one).unwrap();
        assert!(json.contains(r#""lanes":[{"#) && !json.contains("zones"), "{json}");
        assert_eq!(serde_json::from_str::<Road>(&json).unwrap(), one);

        let several = road_of(vec![Zone::whole(vec![lane(3.0)]), Zone { from: Coordinate::new(40.0), lanes: vec![lane(3.0), lane(3.0)] }]);
        let json = serde_json::to_string(&several).unwrap();
        let kept: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(kept.get("zones").is_some() && kept.get("lanes").is_none(), "{json}");
        assert_eq!(serde_json::from_str::<Road>(&json).unwrap(), several);
    }

    #[test]
    fn a_road_whose_zones_do_not_start_at_0_and_run_on_in_order_is_refused() {
        for zones in [
            vec![],
            vec![Zone { from: Coordinate::new(5.0), lanes: vec![] }],
            vec![Zone::whole(vec![]), Zone { from: Coordinate::new(0.0), lanes: vec![] }],
            vec![Zone::whole(vec![]), Zone { from: Coordinate::new(f64::NAN), lanes: vec![] }],
        ] {
            assert!(!Road::is_zoned(&zones), "{zones:?}");
        }
        let bad = r#"{"id":1,"osm_ways":[5],"name":"Main","road_type":"local","from":1,"to":2,"zones":[{"from":5.0,"lanes":[]}],"points":[]}"#;
        assert!(serde_json::from_str::<Road>(bad).is_err());
    }

    #[test]
    fn a_road_with_no_name_or_a_blank_one_is_refused() {
        let json = |name: &str| format!(r#"{{"id":1,"osm_ways":[5],"name":{name},"highway":"x","from":1,"to":2,"lanes":[],"points":[]}}"#);
        assert!(serde_json::from_str::<Road>(&json("null")).is_err());
        assert!(serde_json::from_str::<Road>(&json(r#""  ""#)).is_err());
        assert!(serde_json::from_str::<Road>(&json(r#""Main""#)).is_ok());
    }

    #[test]
    fn a_road_kept_before_versions_existed_reads_with_none() {
        let json = r#"{"id":1,"osm_ways":[5],"name":"Main","highway":"x","from":1,"to":2,"lanes":[],"points":[]}"#;
        assert_eq!(serde_json::from_str::<Road>(json).unwrap().osm_ways, [WayRef::new(5)]);
    }
}
