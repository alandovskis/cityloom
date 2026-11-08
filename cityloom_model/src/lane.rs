use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneKind {
    Driving,
    Parking,
    Sidewalk,
    Bike,
    Bus,
    Buffer,
    Other,
}

/// What a `Buffer` lane is made of, osm2streets' own types. A buffer lane without one (a network kept by an earlier
/// build) is of no known type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BufferType {
    /// Just paint.
    Stripes,
    /// Flex posts, wands, cones.
    FlexPosts,
    /// Planters, with gaps.
    Planters,
    /// A solid barrier.
    JerseyBarrier,
    /// A raised curb.
    Curb,
    /// Non-road surface, between the road and a footpath or within a median: grass.
    Verge,
}

/// Which way a lane runs along its road, from the road's first point to its last. A bike lane can run both ways;
/// a network kept before `Both` existed has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Backward,
    Both,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    pub kind: LaneKind,
    pub way: Direction,
    pub width_m: f64,
    /// What a `Buffer` lane is made of, when it is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer_type: Option<BufferType>,
    /// When the lane is a bus lane, if only at certain hours: OSM's opening-hours text, as tagged
    /// (`Mo-Fr 06:00-10:00,14:30-19:00`). Without it a bus lane is one all day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<String>,
}
