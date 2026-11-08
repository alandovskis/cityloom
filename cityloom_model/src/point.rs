use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// The coordinate system of a `Coordinate`: local metres, measured from the south-west corner of the data, `x` east
/// and `y` north. It is not longitude or latitude, and not layout millimetres.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Local;

/// The coordinate system of a position along a road: metres from the road's `from` end, towards its `to` end.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Street;

/// One position along an axis, in metres, in the coordinate system `S`. A coordinate in one system is not one in
/// another, so the two cannot be mixed up: a `Coordinate<Street>` is not a `Coordinate<Local>`.
/// Kept as the bare number, whatever the system.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent, bound = "")]
pub struct Coordinate<S> {
    metres: f64,
    #[serde(skip)]
    system: PhantomData<S>,
}

impl<S> Coordinate<S> {
    pub fn new(metres: f64) -> Coordinate<S> {
        Coordinate { metres, system: PhantomData }
    }

    pub fn metres(self) -> f64 {
        self.metres
    }
}

/// A position in local metres, `x` east and `y` north.
///
/// Kept as a two-element array, `[x, y]`, as the pairs it replaces were, so a network kept by an earlier build still reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "[Coordinate<Local>; 2]", into = "[Coordinate<Local>; 2]")]
pub struct Point {
    pub x: Coordinate<Local>,
    pub y: Coordinate<Local>,
}

impl Point {
    /// The point at `x_m` east and `y_m` north, in local metres.
    pub fn new(x_m: f64, y_m: f64) -> Point {
        Point { x: Coordinate::new(x_m), y: Coordinate::new(y_m) }
    }

    pub fn x_m(self) -> f64 {
        self.x.metres()
    }

    pub fn y_m(self) -> f64 {
        self.y.metres()
    }

    /// The distance to `other`, in metres.
    pub fn distance(self, other: Point) -> f64 {
        (self.x_m() - other.x_m()).hypot(self.y_m() - other.y_m())
    }
}

impl From<[Coordinate<Local>; 2]> for Point {
    fn from([x, y]: [Coordinate<Local>; 2]) -> Self {
        Point { x, y }
    }
}

impl From<Point> for [Coordinate<Local>; 2] {
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
    fn a_coordinate_is_kept_as_the_bare_number_in_any_system() {
        assert_eq!(serde_json::to_string(&Coordinate::<Street>::new(4.5)).unwrap(), "4.5");
        assert_eq!(serde_json::from_str::<Coordinate<Street>>("4.5").unwrap().metres(), 4.5);
    }

    #[test]
    fn distance_is_euclidean() {
        assert_eq!(Point::new(0.0, 0.0).distance(Point::new(3.0, 4.0)), 5.0);
    }
}
