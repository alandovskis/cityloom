//! Where a box of the earth lies, and how a `Network`'s local metres map onto it: linearly over the
//! box, scaled to its size on the ground, the way osm2streets/`geom` lays it out.

use serde::{Deserialize, Serialize};

/// The earth's radius `geom` uses for distances, in metres.
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// The distance between two points on the earth, in metres, by the haversine formula, as `geom`
/// computes it (its `Pt2D` then keeps a box's size to a tenth of a millimetre, so this does too).
fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let (dp, dl) = ((lat2 - lat1).to_radians(), (lon2 - lon1).to_radians());
    let a = (dp / 2.0).sin().powi(2) + (dl / 2.0).sin().powi(2) * p1.cos() * p2.cos();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    (EARTH_RADIUS_M * c * 10_000.0).round() / 10_000.0
}

/// A box on the earth, in degrees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

impl Frame {
    /// The box's size on the ground along its south edge, in metres.
    pub fn width_m(&self) -> f64 {
        haversine_m(self.south, self.west, self.south, self.east)
    }

    /// The box's size on the ground along its west edge, in metres.
    pub fn height_m(&self) -> f64 {
        haversine_m(self.south, self.west, self.north, self.west)
    }

    /// `[lon, lat]` of a position in metres east and north of the box's south-west corner.
    pub fn lon_lat(&self, x_m: f64, y_m: f64) -> [f64; 2] {
        [self.west + x_m / self.width_m() * (self.east - self.west), self.south + y_m / self.height_m() * (self.north - self.south)]
    }

    /// Metres east and north of the box's south-west corner of a `[lon, lat]`; the inverse of `lon_lat`.
    pub fn local(&self, lon: f64, lat: f64) -> [f64; 2] {
        [(lon - self.west) / (self.east - self.west) * self.width_m(), (lat - self.south) / (self.north - self.south) * self.height_m()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plateau() -> Frame {
        Frame { south: 45.5221, west: -73.6038, north: 45.5301, east: -73.5918 }
    }

    #[test]
    fn local_inverts_lon_lat() {
        let f = plateau();
        for (x, y) in [(0.0, 0.0), (10.5, 700.25), (400.0, 400.0), (799.9, 3.0), (123.456, 654.321)] {
            let [lon, lat] = f.lon_lat(x, y);
            let [x2, y2] = f.local(lon, lat);
            assert!((x - x2).abs() < 1e-9 && (y - y2).abs() < 1e-9, "{x},{y} -> {x2},{y2}");
        }
    }

    #[test]
    fn the_south_west_corner_is_the_origin() {
        let f = plateau();
        assert_eq!(f.lon_lat(0.0, 0.0), [f.west, f.south]);
    }

    #[test]
    fn size_on_the_ground() {
        let f = Frame { south: 45.5, west: -73.6, north: 45.518, east: -73.5743 };
        assert!((f.width_m() - 2005.0).abs() < 5.0, "{}", f.width_m());
        assert!((f.height_m() - 2002.0).abs() < 5.0, "{}", f.height_m());
    }
}
