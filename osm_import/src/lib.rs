//! OpenStreetMap to a plain street network.
//!
//! osm2streets does the hard part (splitting ways at junctions, collapsing the
//! tangle around a big crossing, reading lane tags). This crate runs it and
//! hands the result on as a small, stable shape of its own, so the editor
//! depends on that shape and not on osm2streets. It is built as a separate
//! WebAssembly module, loaded when a place is opened.
//!
//! Positions are metres from the south-west corner of the data: `x` east, `y` north.

use abstutil::Timer;
use osm2streets::{Direction, IntersectionControl, IntersectionKind, LaneType, MapConfig, StreetNetwork, Transformation};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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

/// Which way a lane runs along its road, from the road's first point to its last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Way {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    pub kind: LaneKind,
    pub way: Way,
    pub width_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: u32,
    /// The OSM nodes it stands for; the key an edit is kept under.
    pub osm_nodes: Vec<i64>,
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
    pub name: Option<String>,
    /// OSM's `highway` value.
    pub highway: String,
    pub from: u32,
    pub to: u32,
    /// Left to right when facing from `from` to `to`.
    pub lanes: Vec<Lane>,
    /// The road's centre line.
    pub points: Vec<(f64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Network {
    pub nodes: Vec<Node>,
    pub roads: Vec<Road>,
}

/// Reads OSM XML (or PBF) into a network.
pub fn import(osm: &[u8]) -> Result<Network, String> {
    let mut timer = Timer::throwaway();
    let (mut streets, _doc) = streets_reader::osm_to_street_network(osm, None, MapConfig::default(), &mut timer).map_err(|e| e.to_string())?;
    streets.apply_transformations(Transformation::standard_for_clipped_areas(), &mut timer);
    Ok(convert(&streets))
}

fn convert(streets: &StreetNetwork) -> Network {
    let bounds = streets.gps_bounds.to_bounds();
    let height = bounds.max_y;
    let mut net = Network::default();
    for (id, i) in &streets.intersections {
        let c = i.polygon.center();
        net.nodes.push(Node {
            id: id.0 as u32,
            osm_nodes: i.osm_ids.iter().map(|n| n.0).collect(),
            x_m: c.x(),
            y_m: height - c.y(),
            junction: i.kind == IntersectionKind::Intersection,
            control: match i.control {
                IntersectionControl::Signalled => Control::Signals,
                IntersectionControl::Signed => Control::Signs,
                _ => Control::None,
            },
        });
    }
    for (id, r) in &streets.roads {
        net.roads.push(Road {
            id: id.0 as u32,
            osm_ways: r.osm_ids.iter().map(|w| w.0).collect(),
            name: r.name.clone(),
            highway: r.highway_type.clone(),
            from: r.src_i.0 as u32,
            to: r.dst_i.0 as u32,
            lanes: r
                .lane_specs_ltr
                .iter()
                .map(|l| Lane {
                    kind: match l.lt {
                        LaneType::Driving => LaneKind::Driving,
                        LaneType::Parking(_) => LaneKind::Parking,
                        LaneType::Sidewalk | LaneType::Shoulder | LaneType::Footway => LaneKind::Sidewalk,
                        LaneType::Biking => LaneKind::Bike,
                        LaneType::Bus => LaneKind::Bus,
                        LaneType::Buffer(_) => LaneKind::Buffer,
                        _ => LaneKind::Other,
                    },
                    way: if l.dir == Direction::Forward { Way::Forward } else { Way::Backward },
                    width_m: l.width.inner_meters(),
                })
                .collect(),
            points: r.center_line.points().iter().map(|p| (p.x(), height - p.y())).collect(),
        });
    }
    net
}

/// For the browser: OSM XML in, the network as JSON out (or the error text, as a thrown string).
#[wasm_bindgen]
pub fn osm_to_network(osm: &[u8]) -> Result<String, String> {
    serde_json::to_string(&import(osm)?).map_err(|e| e.to_string())
}
