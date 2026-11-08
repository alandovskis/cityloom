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

/// What a `Buffer` lane is made of, osm2streets' own types.
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

/// Which way a lane runs along its road, from the road's first point to its last. A bike lane can run both ways.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    Backward,
    Both,
}

/// A lane of a road, left to right facing from its `from` end to its `to` end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    /// What the lane is, except during its `windows`.
    pub kind: LaneKind,
    pub direction: Direction,
    pub width_m: f64,
    /// What a `Buffer` lane is made of, when it is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buffer_type: Option<BufferType>,
    /// What else the lane is at certain hours, in order. A lane with none is of its `kind` all day.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<Window>,
    /// The lane's centre line in the road's own coordinates: `x` along the road, `y` across it from its centre line,
    /// left positive. Empty where it has not been laid out; see `Road::lay_out_lanes`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bus_lane() -> Lane {
        Lane { kind: LaneKind::Bus, direction: Direction::Forward, width_m: 3.2, buffer_type: None, windows: vec![], shape: vec![] }
    }

    #[test]
    fn a_lane_that_is_of_one_kind_all_day_and_not_laid_out_is_kept_as_its_kind_direction_and_width() {
        let json = serde_json::to_string(&bus_lane()).unwrap();
        assert_eq!(json, r#"{"kind":"bus","direction":"forward","width_m":3.2}"#);
        assert_eq!(serde_json::from_str::<Lane>(&json).unwrap(), bus_lane());
    }

    #[test]
    fn a_bus_lane_made_only_during_hours_is_parking_the_rest_of_the_time_and_is_kept_so() {
        let mut lane = bus_lane();
        assert!(lane.only_during("Mo-Fr 06:00-10:00,14:30-19:00", LaneKind::Parking));
        assert_eq!(lane.kind, LaneKind::Parking);
        let hours: Vec<_> = lane.windows.iter().map(|w| (w.kind, w.hours.from_min, w.hours.to_min)).collect();
        assert_eq!(hours, [(LaneKind::Bus, 360, 600), (LaneKind::Bus, 870, 1140)]);
        let kept = serde_json::to_string(&lane).unwrap();
        assert_eq!(serde_json::from_str::<Lane>(&kept).unwrap(), lane);
    }

    #[test]
    fn hours_that_cannot_be_read_leave_the_lane_alone() {
        let mut lane = bus_lane();
        assert!(!lane.only_during("sunrise-sunset", LaneKind::Parking));
        assert_eq!(lane, bus_lane());
    }
}
