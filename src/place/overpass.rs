//! The roads in a box, from the Overpass API.

use super::Bounds;

pub const ENDPOINT: &str = "https://overpass-api.de/api/interpreter";

/// What a person can drive, ride or walk along as a street. Left out: tracks, paths, steps and the like.
const HIGHWAYS: &str = "motorway|motorway_link|trunk|trunk_link|primary|primary_link|secondary|secondary_link|tertiary|tertiary_link|unclassified|residential|living_street|service|pedestrian";

/// The Overpass query for every road in `b`, with the nodes they run through, as OSM XML.
pub fn query(b: Bounds) -> String {
    format!("[out:xml][timeout:25];(way[\"highway\"~\"^({HIGHWAYS})$\"]({:.6},{:.6},{:.6},{:.6});>;);out;", b.south, b.west, b.north, b.east)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_asks_for_streets_in_the_box_with_their_nodes() {
        let q = query(Bounds { south: 51.4990, west: -0.1290, north: 51.5010, east: -0.1260 });
        assert!(q.starts_with("[out:xml]"));
        assert!(q.contains("(51.499000,-0.129000,51.501000,-0.126000)"));
        assert!(q.contains(">;") && q.ends_with("out;"));
        assert!(q.contains("residential") && !q.contains("footway") && !q.contains("track"));
    }
}
