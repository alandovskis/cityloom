use serde::{Deserialize, Serialize};

/// A road of the network, by its id in it. Not an OSM way: see `WayRef`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoadId(pub u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_is_kept_as_the_bare_number() {
        assert_eq!(serde_json::to_string(&RoadId(4)).unwrap(), "4");
        assert_eq!(serde_json::from_str::<RoadId>("4").unwrap(), RoadId(4));
    }
}
