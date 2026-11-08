use serde::{Deserialize, Serialize};

use crate::{Hours, Point, Street, Window};

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

/// A lane of a road, left to right facing from its `from` end to its `to` end.
///
/// Kept as it always was, but for the hours of a bus lane: the old `hours` text, on a bus lane, still reads as a bus
/// lane that is parking the rest of the time, which is what the editor made of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "LaneWire", into = "LaneWire")]
pub struct Lane {
    /// What the lane is, except during its `windows`.
    pub kind: LaneKind,
    pub direction: Direction,
    pub width_m: f64,
    /// What a `Buffer` lane is made of, when it is known.
    pub buffer_type: Option<BufferType>,
    /// What else the lane is at certain hours, in order. A lane with none is of its `kind` all day.
    pub windows: Vec<Window>,
    /// The lane's centre line in the road's own coordinates: `x` along the road, `y` across it from its centre line,
    /// left positive. Empty where it has not been laid out; see `Road::lay_out_lanes`.
    pub shape: Vec<Point<Street>>,
}

impl Lane {
    /// Makes the lane what it is now only during `hours` (OSM opening hours, `Mo-Fr 06:00-10:00`), and `outside` the
    /// rest of the time. Returns false, and leaves the lane as it is, if the hours cannot be read.
    pub fn only_during(&mut self, hours: &str, outside: LaneKind) -> bool {
        let Some(hours) = Hours::parse(hours) else { return false };
        let kind = self.kind;
        self.windows = hours.into_iter().map(|hours| Window { hours, kind }).collect();
        self.kind = outside;
        true
    }
}

#[derive(Serialize, Deserialize)]
struct LaneWire {
    kind: LaneKind,
    way: Direction,
    width_m: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    buffer_type: Option<BufferType>,
    /// Read only: what an earlier build kept for a bus lane that runs at certain hours.
    #[serde(default, skip_serializing)]
    hours: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    windows: Vec<Window>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    shape: Vec<Point<Street>>,
}

impl From<LaneWire> for Lane {
    fn from(w: LaneWire) -> Lane {
        let mut lane = Lane { kind: w.kind, direction: w.way, width_m: w.width_m, buffer_type: w.buffer_type, windows: w.windows, shape: w.shape };
        if let (LaneKind::Bus, Some(hours)) = (lane.kind, w.hours.as_deref()) {
            lane.only_during(hours, LaneKind::Parking);
        }
        lane
    }
}

impl From<Lane> for LaneWire {
    fn from(l: Lane) -> LaneWire {
        LaneWire { kind: l.kind, way: l.direction, width_m: l.width_m, buffer_type: l.buffer_type, hours: None, windows: l.windows, shape: l.shape }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bus_lane_kept_with_hours_reads_as_parking_with_a_bus_window() {
        let json = r#"{"kind":"bus","way":"forward","width_m":3.2,"hours":"Mo-Fr 06:00-10:00,14:30-19:00"}"#;
        let lane: Lane = serde_json::from_str(json).unwrap();
        assert_eq!(lane.kind, LaneKind::Parking);
        let hours: Vec<_> = lane.windows.iter().map(|w| (w.kind, w.hours.from_min, w.hours.to_min)).collect();
        assert_eq!(hours, [(LaneKind::Bus, 360, 600), (LaneKind::Bus, 870, 1140)]);
        let kept = serde_json::to_string(&lane).unwrap();
        let top: serde_json::Value = serde_json::from_str(&kept).unwrap();
        assert!(top.get("hours").is_none() && top.get("windows").is_some(), "the old text is not written back: {kept}");
        assert_eq!(serde_json::from_str::<Lane>(&kept).unwrap(), lane);
    }

    #[test]
    fn hours_that_cannot_be_read_or_are_not_a_bus_lanes_leave_the_lane_alone() {
        for json in [
            r#"{"kind":"bus","way":"forward","width_m":3.2,"hours":"sunrise-sunset"}"#,
            r#"{"kind":"driving","way":"forward","width_m":3.2,"hours":"Mo-Fr 06:00-10:00"}"#,
            r#"{"kind":"bus","way":"forward","width_m":3.2}"#,
        ] {
            let lane: Lane = serde_json::from_str(json).unwrap();
            assert!(lane.windows.is_empty() && lane.kind != LaneKind::Parking, "{json}");
        }
    }
}
