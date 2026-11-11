//! Layout millimetres to longitude and latitude, the way osm2streets lays a box of the earth out in
//! metres: linearly over the box, scaled to its size on the ground.

use crate::city::store::CityStore;
use crate::place::Bounds;
use cityloom_model::Frame;

/// Where a layout lies on the earth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    frame: Frame,
    width_m: f64,
    height_m: f64,
    /// Network metres (east, north of the box's south-west corner) of the layout's origin.
    origin_m: (f64, f64),
}

impl Projection {
    /// The projection of a layout whose origin is `origin_m` in a network read over `bounds`.
    pub fn new(bounds: Bounds, origin_m: [f64; 2]) -> Projection {
        let frame = bounds.frame();
        let (width_m, height_m) = (frame.width_m(), frame.height_m());
        Projection { frame, width_m, height_m, origin_m: (origin_m[0], origin_m[1]) }
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
        self.frame.lon_lat_sized(self.width_m, self.height_m, x_m, y_m)
    }

    /// `[lon, lat]` of a position of the layout, in millimetres (x east, y south).
    pub fn lon_lat(&self, x_mm: i32, y_mm: i32) -> [f64; 2] {
        self.network_lon_lat(self.origin_m.0 + x_mm as f64 / 1000.0, self.origin_m.1 - y_mm as f64 / 1000.0)
    }
}

/// Where the city kept in `store` lies: known once its area's roads are kept and its layout says where its
/// origin is (`CityView::origin_m`).
pub fn for_store(store: &CityStore, origin_m: Option<[f64; 2]>) -> Option<Projection> {
    if !store.has_network() {
        return None;
    }
    Some(Projection::new(store.area().bounds(), origin_m?))
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
    fn network_positions_are_the_frames_own() {
        let b = plateau();
        let p = Projection::new(b, [0.0, 0.0]);
        for (x, y) in [(0.0, 0.0), (10.5, 700.25), (799.0, 3.0)] {
            assert_eq!(p.network_lon_lat(x, y), b.frame().lon_lat(x, y));
        }
    }

    #[test]
    fn layout_positions_are_offset_by_the_origin_and_run_south_as_y_grows() {
        let p = Projection::new(plateau(), [10.0, 20.0]);
        assert_eq!(p.lon_lat(0, 0), p.network_lon_lat(10.0, 20.0));
        assert_eq!(p.lon_lat(2_000, 5_000), p.network_lon_lat(12.0, 15.0));
    }

    #[test]
    fn a_city_lies_on_the_earth_once_its_roads_are_kept_and_its_layout_says_where() {
        use crate::city::store::CityStore;
        let (ports, ..) = crate::shared::ports::test_ports();
        let area = Area::new("Testville", 45.5, -73.6);
        let store = CityStore::for_area(ports.storage.clone(), area.clone());
        assert_eq!(for_store(&store, Some([0.0, 0.0])), None, "no roads kept: nowhere");
        assert!(store.keep_network(&cityloom_model::Network::default()));
        assert_eq!(for_store(&store, None), None, "a layout that does not say where its origin is");
        assert_eq!(for_store(&store, Some([1.0, 2.0])), Some(Projection::new(area.bounds(), [1.0, 2.0])));
    }
}
