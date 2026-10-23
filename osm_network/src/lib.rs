//! The plain street network that `osm_import` produces and the editor reads.
//! It stands between them so that the editor does not depend on osm2streets.
//!
//! Positions are metres from the south-west corner of the data: `x` east, `y` north.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    None,
    Signs,
    Signals,
}

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
pub enum Way {
    Forward,
    Backward,
    Both,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    pub kind: LaneKind,
    pub way: Way,
    pub width_m: f64,
    /// What a `Buffer` lane is made of, when it is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer_type: Option<BufferType>,
    /// When the lane is a bus lane, if only at certain hours: OSM's opening-hours text, as tagged
    /// (`Mo-Fr 06:00-10:00,14:30-19:00`). Without it a bus lane is one all day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hours: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: u32,
    /// The OSM nodes it stands for; the key an edit is kept under.
    pub osm_nodes: Vec<i64>,
    /// The version OpenStreetMap had of each of them when the data was read.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub osm_versions: BTreeMap<i64, i32>,
    pub x_m: f64,
    pub y_m: f64,
    /// Where three or more roads meet and traffic gives way, as opposed to a street running off the data or ending.
    pub junction: bool,
    pub control: Control,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Road {
    pub id: u32,
    /// The OSM ways it was made from; the key an edit is kept under.
    pub osm_ways: Vec<i64>,
    /// The version OpenStreetMap had of each of them when the data was read.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub osm_versions: BTreeMap<i64, i32>,
    pub name: Option<String>,
    /// OSM's `highway` value.
    pub highway: String,
    pub from: u32,
    pub to: u32,
    /// Left to right when facing from `from` to `to`.
    pub lanes: Vec<Lane>,
    /// OSM's `maxspeed` in km/h; none when the way gives none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_kmh: Option<u16>,
    /// The road's centre line.
    pub points: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Network {
    /// Whether traffic keeps to the left.
    pub left_hand: bool,
    pub nodes: Vec<Node>,
    pub roads: Vec<Road>,
}

pub mod merge;
