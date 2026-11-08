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
#[derive(Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent, bound = "")]
pub struct Coordinate<S> {
    metres: f64,
    #[serde(skip)]
    system: PhantomData<S>,
}

// not derived: a derive would ask the system, a marker that is never held, to be `Clone` too
impl<S> Clone for Coordinate<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Coordinate<S> {}

impl<S> Clone for Point<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Point<S> {}

impl<S> Coordinate<S> {
    pub fn new(metres: f64) -> Coordinate<S> {
        Coordinate { metres, system: PhantomData }
    }

    pub fn metres(self) -> f64 {
        self.metres
    }
}

/// A position in the coordinate system `S`, in metres; `Local` unless said otherwise.
///
/// In `Local` it is `x` east and `y` north. In `Street` it is `x` along the road from its `from` end and `y` across
/// it from its centre line, positive to the left facing from `from` to `to`.
///
/// Kept as a two-element array, `[x, y]`, as the pairs it replaces were, so a network kept by an earlier build still reads.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "[Coordinate<S>; 2]", into = "[Coordinate<S>; 2]", bound = "")]
pub struct Point<S = Local> {
    pub x: Coordinate<S>,
    pub y: Coordinate<S>,
}

impl<S> Point<S> {
    /// The point at `x_m` and `y_m`, in metres in the system `S`.
    pub fn new(x_m: f64, y_m: f64) -> Point<S> {
        Point { x: Coordinate::new(x_m), y: Coordinate::new(y_m) }
    }

    pub fn x_m(self) -> f64 {
        self.x.metres()
    }

    pub fn y_m(self) -> f64 {
        self.y.metres()
    }

    /// The distance to `other`, in metres.
    pub fn distance(self, other: Point<S>) -> f64 {
        (self.x_m() - other.x_m()).hypot(self.y_m() - other.y_m())
    }
}

impl<S> From<[Coordinate<S>; 2]> for Point<S> {
    fn from([x, y]: [Coordinate<S>; 2]) -> Self {
        Point { x, y }
    }
}

impl<S> From<Point<S>> for [Coordinate<S>; 2] {
    fn from(p: Point<S>) -> Self {
        [p.x, p.y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_is_kept_as_the_pair_it_replaces() {
        let p = Point::<Local>::new(1.5, -2.0);
        assert_eq!(serde_json::to_string(&p).unwrap(), "[1.5,-2.0]");
        assert_eq!(serde_json::from_str::<Point<Local>>("[1.5,-2.0]").unwrap(), p);
    }

    #[test]
    fn a_coordinate_is_kept_as_the_bare_number_in_any_system() {
        assert_eq!(serde_json::to_string(&Coordinate::<Street>::new(4.5)).unwrap(), "4.5");
        assert_eq!(serde_json::from_str::<Coordinate<Street>>("4.5").unwrap().metres(), 4.5);
    }

    #[test]
    fn distance_is_euclidean() {
        assert_eq!(Point::<Local>::new(0.0, 0.0).distance(Point::new(3.0, 4.0)), 5.0);
    }
}
