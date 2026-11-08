use serde::{Deserialize, Serialize};

/// One position along an axis in the network's only coordinate system, local metres: metres from the
/// south-west corner of the data, `x` east and `y` north. It is not longitude or latitude, and not layout millimetres.
/// The field is private so a value is made, and read, by naming that system.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Coordinate(f64);

impl Coordinate {
    pub fn local_metres(metres: f64) -> Coordinate {
        Coordinate(metres)
    }

    pub fn as_local_metres(self) -> f64 {
        self.0
    }
}

/// A position in local metres.
///
/// Kept as a two-element array, `[x, y]`, as the pairs it replaces were, so a network kept by an earlier build still reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "[Coordinate; 2]", into = "[Coordinate; 2]")]
pub struct Point {
    pub x: Coordinate,
    pub y: Coordinate,
}

impl Point {
    /// The point at `x_m` east and `y_m` north, in local metres.
    pub fn new(x_m: f64, y_m: f64) -> Point {
        Point { x: Coordinate::local_metres(x_m), y: Coordinate::local_metres(y_m) }
    }

    pub fn x_m(self) -> f64 {
        self.x.as_local_metres()
    }

    pub fn y_m(self) -> f64 {
        self.y.as_local_metres()
    }

    /// The distance to `other`, in metres.
    pub fn distance(self, other: Point) -> f64 {
        (self.x_m() - other.x_m()).hypot(self.y_m() - other.y_m())
    }
}

impl From<[Coordinate; 2]> for Point {
    fn from([x, y]: [Coordinate; 2]) -> Self {
        Point { x, y }
    }
}

impl From<Point> for [Coordinate; 2] {
    fn from(p: Point) -> Self {
        [p.x, p.y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_is_kept_as_the_pair_it_replaces() {
        let p = Point::new(1.5, -2.0);
        assert_eq!(serde_json::to_string(&p).unwrap(), "[1.5,-2.0]");
        assert_eq!(serde_json::from_str::<Point>("[1.5,-2.0]").unwrap(), p);
    }

    #[test]
    fn distance_is_euclidean() {
        assert_eq!(Point::new(0.0, 0.0).distance(Point::new(3.0, 4.0)), 5.0);
    }
}
