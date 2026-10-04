//! The editor's projection must agree with the one osm2streets laid the network out with.

use cityloom_editor::map::projection::Projection;
use cityloom_editor::place::area::Area;
use geom::{GPSBounds, LonLat};

#[test]
fn the_projection_is_the_inverse_of_geoms_over_the_plateau() {
    let b = Area::new("Plateau", 45.5261, -73.5978).bounds();
    let gps = GPSBounds::from(vec![LonLat::new(b.west, b.south), LonLat::new(b.east, b.north)]);
    let size = gps.get_max_world_pt();
    let (w, h) = (size.x(), size.y());
    let p = Projection::new(b, [0.0, 0.0]);
    assert!((p.width_m() - w).abs() < 1e-6 && (p.height_m() - h).abs() < 1e-6, "size {} x {} against {w} x {h}", p.width_m(), p.height_m());
    for (fx, fy) in [(0.0, 0.0), (1.0, 1.0), (0.5, 0.5), (0.2, 0.9), (0.9, 0.1)] {
        // the importer measures y up from the south; geom measures it down from the north
        let (x, y_north) = (fx * w, fy * h);
        let want = gps.convert_back_xy(x, h - y_north);
        let got = p.network_lon_lat(x, y_north);
        assert!((got[0] - want.x()).abs() < 1e-9 && (got[1] - want.y()).abs() < 1e-9, "{fx},{fy}: {got:?} against {} {}", want.x(), want.y());
    }
}
