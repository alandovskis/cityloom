use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::{Lane, Point, RoadType, WayRef};

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
    /// Left to right when facing from `from` to `to`.
    pub lanes: Vec<Lane>,
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
    lanes: Vec<Lane>,
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
        let osm_ways = w.osm_ways.iter().map(|&id| WayRef { id, version: w.osm_versions.get(&id).copied() }).collect();
        Ok(Road { id: w.id, osm_ways, name, road_type: w.road_type, from: w.from, to: w.to, lanes: w.lanes, speed_kmh: w.speed_kmh, points: w.points })
    }
}

impl From<Road> for RoadWire {
    fn from(r: Road) -> RoadWire {
        let osm_versions = r.osm_ways.iter().filter_map(|w| Some((w.id, w.version?))).collect();
        RoadWire {
            id: r.id,
            osm_ways: r.osm_ways.iter().map(|w| w.id).collect(),
            osm_versions,
            name: Some(r.name),
            road_type: r.road_type,
            from: r.from,
            to: r.to,
            lanes: r.lanes,
            speed_kmh: r.speed_kmh,
            points: r.points,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_road_is_kept_with_its_ways_and_their_versions_apart_as_it_always_was() {
        let road = Road {
            id: 1,
            osm_ways: vec![WayRef::with_version(5, 49), WayRef::new(7)],
            name: "Main".into(),
            road_type: RoadType::Local,
            from: 1,
            to: 2,
            lanes: vec![],
            speed_kmh: None,
            points: vec![Point::new(0.0, 1.0)],
        };
        let json = serde_json::to_string(&road).unwrap();
        assert!(json.contains(r#""osm_ways":[5,7]"#) && json.contains(r#""osm_versions":{"5":49}"#), "{json}");
        assert_eq!(serde_json::from_str::<Road>(&json).unwrap(), road);
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
