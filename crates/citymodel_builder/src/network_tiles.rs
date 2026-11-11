//! The whole area's network cut along the grid into tiles, and the tiles written as JSON files.
//!
//! A tile holds every road whose centre line reaches into its core widened by the margin, whole, with the junctions
//! it uses (`Network::reaching`), so a place whose centre is in the core is read from that one file. A road that
//! reaches into several tiles is in each, with the same ids. Positions stay in the whole area's frame.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use cityloom_model::{Frame, Network, NetworkTile, NodeId};
use sha2::{Digest, Sha256};

use crate::metro_index::{Grid, short_checksum};
use crate::progress::StepError;
use crate::tiles::{MARGIN_LAT, MARGIN_LON};

/// The box a road is looked for in, widened by this much (degrees, about a tenth of a millimetre), so that rounding
/// cannot leave out a road `reaching` would keep: the box only chooses the roads `reaching` is asked about.
const SLACK: f64 = 1e-9;

/// Tile `x`, `y`'s core widened by the margin, as a frame.
fn rect(grid: &Grid, x: u16, y: u16) -> Frame {
    let (x, y) = (f64::from(x), f64::from(y));
    Frame {
        south: grid.lat0 + y * grid.dlat - MARGIN_LAT,
        west: grid.lon0 + x * grid.dlon - MARGIN_LON,
        north: grid.lat0 + (y + 1.0) * grid.dlat + MARGIN_LAT,
        east: grid.lon0 + (x + 1.0) * grid.dlon + MARGIN_LON,
    }
}

/// The tiles `from` to `to` (degrees) along one axis can reach: a range wide enough by one tile each way, which the
/// boxes then decide.
fn span(from: f64, to: f64, origin: f64, size: f64, margin: f64, count: u16) -> std::ops::RangeInclusive<u16> {
    let index = |v: f64| ((v - origin) / size).floor();
    let low = (index(from - margin) - 1.0).clamp(0.0, f64::from(count - 1)) as u16;
    let high = (index(to + margin) + 1.0).clamp(0.0, f64::from(count - 1)) as u16;
    low..=high
}

/// The tiles of the grid that hold a road, `x` outer and `y` inner; each tile's frame is `frame`, the whole area's.
pub fn cut(network: &Network, frame: &Frame, grid: &Grid) -> Vec<(u16, u16, NetworkTile)> {
    let (width_m, height_m) = (frame.width_m(), frame.height_m());
    // Which roads each tile's box may hold, in the network's order: a road is looked at for the few tiles its own
    // box comes near, not for every tile.
    let mut candidates: Vec<Vec<usize>> = vec![Vec::new(); usize::from(grid.nx) * usize::from(grid.ny)];
    for (i, road) in network.roads.iter().enumerate() {
        let mut points = road.points.iter().map(|p| frame.lon_lat_sized(width_m, height_m, p.x_m(), p.y_m()));
        let Some([lon, lat]) = points.next() else { continue };
        let (mut west, mut south, mut east, mut north) = (lon, lat, lon, lat);
        for [lon, lat] in points {
            (west, east, south, north) = (west.min(lon), east.max(lon), south.min(lat), north.max(lat));
        }
        if grid.nx == 0 || grid.ny == 0 || !(west.is_finite() && east.is_finite() && south.is_finite() && north.is_finite()) {
            continue;
        }
        for x in span(west, east, grid.lon0, grid.dlon, MARGIN_LON, grid.nx) {
            for y in span(south, north, grid.lat0, grid.dlat, MARGIN_LAT, grid.ny) {
                let r = rect(grid, x, y);
                if west <= r.east + SLACK && east >= r.west - SLACK && south <= r.north + SLACK && north >= r.south - SLACK {
                    candidates[usize::from(x) * usize::from(grid.ny) + usize::from(y)].push(i);
                }
            }
        }
    }

    let junction_at: HashMap<NodeId, usize> = network.junctions.iter().enumerate().map(|(i, j)| (j.id, i)).collect();
    let mut tiles = Vec::new();
    for x in 0..grid.nx {
        for y in 0..grid.ny {
            let roads = &candidates[usize::from(x) * usize::from(grid.ny) + usize::from(y)];
            if roads.is_empty() {
                continue;
            }
            // The candidates as a network of their own, in the whole network's order, so that `reaching` gives
            // what it would give from the whole.
            let mut junctions: Vec<usize> =
                roads.iter().flat_map(|&i| [network.roads[i].edge.from, network.roads[i].edge.to]).filter_map(|id| junction_at.get(&id).copied()).collect();
            junctions.sort_unstable();
            junctions.dedup();
            let near = Network {
                left_hand: network.left_hand,
                junctions: junctions.iter().map(|&i| network.junctions[i].clone()).collect(),
                roads: roads.iter().map(|&i| network.roads[i].clone()).collect(),
                corridors: Vec::new(),
            };
            let held = near.reaching(frame, &rect(grid, x, y));
            if !held.roads.is_empty() {
                tiles.push((x, y, NetworkTile { frame: *frame, network: held }));
            }
        }
    }
    tiles
}

/// Writes each tile to `out/<x>_<y>.json`, compact, one at a time; returns each tile's name and short checksum, in
/// the tiles' order, as `index_json` takes them.
pub fn write(out: &Path, tiles: &[(u16, u16, NetworkTile)]) -> Result<Vec<(String, String)>, StepError> {
    let mut named = Vec::with_capacity(tiles.len());
    for (x, y, tile) in tiles {
        let name = format!("{x}_{y}");
        let bytes = serde_json::to_vec(tile).map_err(|e| StepError::new([format!("cannot write tile {name}: {e}")]))?;
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        fs::write(out.join(format!("{name}.json")), &bytes)?;
        named.push((name, short_checksum(&digest)));
    }
    Ok(named)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metro_index::index_json;
    use cityloom_model::{
        Control, Corridor, Direction, Edge, Junction, Lane, LaneKind, NodeRef, Point, Provenance, Road, RoadId, RoadType, UserClass, WayRef, Zone,
    };

    /// About 6 km by 4 km in Montreal, with a 3 by 2 grid of the city's tile size on it.
    fn frame() -> Frame {
        Frame { south: 45.5, west: -73.6, north: 45.536, east: -73.523 }
    }

    fn small_grid() -> Grid {
        Grid { lon0: -73.6, lat0: 45.5, dlon: 0.0257, dlat: 0.018, nx: 3, ny: 2 }
    }

    fn at(lon: f64, lat: f64) -> Point {
        let [x, y] = frame().local(lon, lat);
        Point::new(x, y)
    }

    fn junction(id: u32, p: Point) -> Junction {
        Junction {
            id: NodeId(id),
            provenance: Provenance::of_nodes([NodeRef::with_version(1000 + i64::from(id), 1)]),
            position: p,
            junction: true,
            control: Control::Signs,
        }
    }

    fn road(id: u32, from: u32, to: u32, points: &[Point]) -> Road {
        let lane = Lane { kind: LaneKind::Driving, direction: Direction::Forward, width_m: 3.0, buffer_type: None, windows: vec![], shape: vec![] };
        Road {
            edge: Edge { id: RoadId(id), from: NodeId(from), to: NodeId(to) },
            provenance: Provenance::of_ways([WayRef::with_version(500 + i64::from(id), 1)]),
            name: format!("Road {id}"),
            road_type: RoadType::Local,
            zones: vec![Zone::whole(vec![lane])],
            points: points.to_vec(),
        }
    }

    /// Road 1 lies well inside tile 0_0; road 2 crosses the edge between tiles 0_0 and 1_0 (at -73.5743); the other
    /// four tiles hold nothing. The whole network has a corridor.
    fn network() -> Network {
        let (a, b) = (at(-73.59, 45.505), at(-73.585, 45.505));
        let (c, d) = (at(-73.578, 45.506), at(-73.570, 45.506));
        Network {
            left_hand: false,
            junctions: vec![junction(1, a), junction(2, b), junction(3, c), junction(4, d)],
            roads: vec![road(1, 1, 2, &[a, b]), road(2, 3, 4, &[c, d])],
            corridors: vec![Corridor { user: UserClass::Driver, roads: vec![RoadId(1), RoadId(2)] }],
        }
    }

    fn ids(tile: &NetworkTile) -> (Vec<u32>, Vec<u32>) {
        (tile.network.roads.iter().map(|r| r.edge.id.0).collect(), tile.network.junctions.iter().map(|j| j.id.0).collect())
    }

    #[test]
    fn a_road_is_in_every_tile_whose_widened_core_it_reaches_and_empty_tiles_are_left_out() {
        let whole = network();
        let tiles = cut(&whole, &frame(), &small_grid());
        let at: Vec<(u16, u16)> = tiles.iter().map(|(x, y, _)| (*x, *y)).collect();
        assert_eq!(at, [(0, 0), (1, 0)]);
        assert_eq!(ids(&tiles[0].2), (vec![1, 2], vec![1, 2, 3, 4]));
        assert_eq!(ids(&tiles[1].2), (vec![2], vec![3, 4]));
        // the same road, the same ids, the whole area's positions
        assert_eq!(tiles[0].2.network.roads[1], whole.roads[1]);
        assert_eq!(tiles[1].2.network.roads[0], whole.roads[1]);
        for (_, _, tile) in &tiles {
            assert_eq!(tile.frame, frame());
            assert!(tile.network.corridors.is_empty());
        }
    }

    #[test]
    fn the_margin_takes_in_a_road_just_past_the_core() {
        // 300 m east of tile 0_0's core (its margin is about 500 m) and inside 1_0's core
        let (a, b) = (at(-73.5705, 45.51), at(-73.5700, 45.51));
        let whole = Network { left_hand: false, junctions: vec![junction(1, a), junction(2, b)], roads: vec![road(7, 1, 2, &[a, b])], corridors: vec![] };
        let at: Vec<(u16, u16)> = cut(&whole, &frame(), &small_grid()).iter().map(|(x, y, _)| (*x, *y)).collect();
        assert_eq!(at, [(0, 0), (1, 0)]);
    }

    #[test]
    fn the_tiles_are_what_reaching_gives_from_the_whole_network() {
        let osm = fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../osm_import/tests/data/plateau.osm")).unwrap();
        let (whole, frame) = osm_import::import_whole(&osm).unwrap();
        let grid = crate::tiles::grid();
        let tiles = cut(&whole, &frame, &grid);
        let mut expected = Vec::new();
        for x in 0..grid.nx {
            for y in 0..grid.ny {
                let held = whole.reaching(&frame, &rect(&grid, x, y));
                if !held.roads.is_empty() {
                    expected.push((x, y, NetworkTile { frame, network: held }));
                }
            }
        }
        assert!(expected.len() >= 2, "{} tiles", expected.len());
        assert_eq!(tiles, expected);
    }

    #[test]
    fn written_tiles_parse_back_and_their_checksums_are_the_files() {
        let tiles = cut(&network(), &frame(), &small_grid());
        let dir = tempfile::tempdir().unwrap();
        let named = write(dir.path(), &tiles).unwrap();
        let names: Vec<&str> = named.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["0_0", "1_0"]);
        for ((name, sum), (_, _, tile)) in named.iter().zip(&tiles) {
            let bytes = fs::read(dir.path().join(format!("{name}.json"))).unwrap();
            let digest: [u8; 32] = Sha256::digest(&bytes).into();
            assert_eq!(*sum, short_checksum(&digest));
            assert_eq!(serde_json::from_slice::<NetworkTile>(&bytes).unwrap(), *tile);
            assert!(!bytes.contains(&b'\n'), "compact JSON");
        }
        let index = index_json(&small_grid(), &named);
        assert!(index.contains("\"tiles\":[\"0_0\",\"1_0\"]"), "{index}");
    }

    #[test]
    fn writing_twice_gives_the_same_bytes() {
        let tiles = cut(&network(), &frame(), &small_grid());
        let (one, two) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        assert_eq!(write(one.path(), &tiles).unwrap(), write(two.path(), &tiles).unwrap());
        for name in ["0_0", "1_0"] {
            let file = format!("{name}.json");
            assert_eq!(fs::read(one.path().join(&file)).unwrap(), fs::read(two.path().join(&file)).unwrap());
        }
    }
}
