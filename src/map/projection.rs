//! Layout millimetres to longitude and latitude, the way osm2streets lays a box of the earth out in
//! metres: linearly over the box, scaled to its size on the ground.

use crate::place::Bounds;

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

/// Where a layout lies on the earth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    bounds: Bounds,
    width_m: f64,
    height_m: f64,
    /// Network metres (east, north of the box's south-west corner) of the layout's origin.
    origin_m: (f64, f64),
}

impl Projection {
    /// The projection of a layout whose origin is `origin_m` in a network read over `bounds`.
    pub fn new(bounds: Bounds, origin_m: [f64; 2]) -> Projection {
        let width_m = haversine_m(bounds.south, bounds.west, bounds.south, bounds.east);
        let height_m = haversine_m(bounds.south, bounds.west, bounds.north, bounds.west);
        Projection { bounds, width_m, height_m, origin_m: (origin_m[0], origin_m[1]) }
    }

    /// The box's size on the ground: along its south edge, and along its west edge.
    pub fn width_m(&self) -> f64 {
        self.width_m
    }

    pub fn height_m(&self) -> f64 {
        self.height_m
    }

    /// `[lon, lat]` of a position in metres east and north of the box's south-west corner.
    pub fn network_lon_lat(&self, x_m: f64, y_m: f64) -> [f64; 2] {
        let b = &self.bounds;
        [b.west + x_m / self.width_m * (b.east - b.west), b.south + y_m / self.height_m * (b.north - b.south)]
    }

    /// `[lon, lat]` of a position of the layout, in millimetres (x east, y south).
    pub fn lon_lat(&self, x_mm: i32, y_mm: i32) -> [f64; 2] {
        self.network_lon_lat(self.origin_m.0 + x_mm as f64 / 1000.0, self.origin_m.1 - y_mm as f64 / 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::area::Area;

    fn plateau() -> Bounds {
        Area::new("Plateau", 45.5261, -73.5978).bounds()
    }

    #[test]
    fn the_box_is_as_big_on_the_ground_as_the_area_says() {
        let p = Projection::new(plateau(), [0.0, 0.0]);
        assert!((p.width_m() - 800.0).abs() < 2.0, "{}", p.width_m());
        assert!((p.height_m() - 800.0).abs() < 2.0, "{}", p.height_m());
    }

    #[test]
    fn the_south_west_corner_is_the_network_origin_and_the_far_corner_is_the_north_east() {
        let b = plateau();
        let p = Projection::new(b, [0.0, 0.0]);
        assert_eq!(p.network_lon_lat(0.0, 0.0), [b.west, b.south]);
        let [lon, lat] = p.network_lon_lat(p.width_m(), p.height_m());
        assert!((lon - b.east).abs() < 1e-12 && (lat - b.north).abs() < 1e-12);
    }

    #[test]
    fn layout_positions_are_offset_by_the_origin_and_run_south_as_y_grows() {
        let p = Projection::new(plateau(), [10.0, 20.0]);
        assert_eq!(p.lon_lat(0, 0), p.network_lon_lat(10.0, 20.0));
        assert_eq!(p.lon_lat(2_000, 5_000), p.network_lon_lat(12.0, 15.0));
    }
}
