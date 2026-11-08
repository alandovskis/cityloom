use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// How a road ranks in the network, from the busiest to the least. OSM's `highway` tag is finer than the editor
/// needs; `RoadType::from_osm` folds it into these five.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoadType {
    /// A motorway: a road with grade-separated junctions and no access from the side.
    Motorway,
    /// A trunk road: a main road between towns that is not a motorway.
    Highway,
    /// A primary road.
    Arterial,
    /// A secondary or tertiary road, which collects traffic from the local streets.
    Collector,
    /// Every other road: residential, service, living street, unclassified.
    Local,
}

impl RoadType {
    pub const ALL: [RoadType; 5] = [RoadType::Motorway, RoadType::Highway, RoadType::Arterial, RoadType::Collector, RoadType::Local];

    /// The name the type is kept under.
    pub fn name(self) -> &'static str {
        match self {
            RoadType::Motorway => "motorway",
            RoadType::Highway => "highway",
            RoadType::Arterial => "arterial",
            RoadType::Collector => "collector",
            RoadType::Local => "local",
        }
    }

    /// The type of a way by OSM's `highway` value, its `_link` included. A value that is none of the ranked ones
    /// (residential, service, unclassified, a value this does not know) is local.
    pub fn from_osm(highway: &str) -> RoadType {
        match highway {
            "motorway" | "motorway_link" => RoadType::Motorway,
            "trunk" | "trunk_link" => RoadType::Highway,
            "primary" | "primary_link" => RoadType::Arterial,
            "secondary" | "secondary_link" | "tertiary" | "tertiary_link" => RoadType::Collector,
            _ => RoadType::Local,
        }
    }
}

impl Serialize for RoadType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

/// Reads a type by its own name, and otherwise an OSM `highway` value, which is what a network kept by an earlier
/// build holds in its place.
impl<'de> Deserialize<'de> for RoadType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<RoadType, D::Error> {
        let text = String::deserialize(d)?;
        Ok(RoadType::ALL.into_iter().find(|t| t.name() == text).unwrap_or_else(|| RoadType::from_osm(&text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osm_values_fold_into_the_five_types() {
        for (highway, want) in [
            ("motorway", RoadType::Motorway),
            ("motorway_link", RoadType::Motorway),
            ("trunk", RoadType::Highway),
            ("trunk_link", RoadType::Highway),
            ("primary", RoadType::Arterial),
            ("primary_link", RoadType::Arterial),
            ("secondary", RoadType::Collector),
            ("tertiary_link", RoadType::Collector),
            ("residential", RoadType::Local),
            ("service", RoadType::Local),
            ("something_new", RoadType::Local),
        ] {
            assert_eq!(RoadType::from_osm(highway), want, "{highway}");
        }
    }

    #[test]
    fn a_type_is_kept_by_its_name_and_a_kept_osm_value_still_reads() {
        for t in RoadType::ALL {
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(serde_json::from_str::<RoadType>(&json).unwrap(), t);
        }
        assert_eq!(serde_json::from_str::<RoadType>(r#""trunk""#).unwrap(), RoadType::Highway);
        assert_eq!(serde_json::from_str::<RoadType>(r#""residential""#).unwrap(), RoadType::Local);
    }
}
