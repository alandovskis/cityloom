//! The roads in a box, from the Overpass API.

use super::Bounds;

pub const ENDPOINT: &str = "https://overpass-api.de/api/interpreter";

/// What a person can drive, ride or walk along as a street. Left out: tracks, paths, steps and the like.
///
/// The bike tracks mapped as ways of their own are fetched as well (`TRACKS`), but they are not streets: the importer
/// only reads them to tell whether the track beside a road runs both ways, and leaves them out of the network.
const HIGHWAYS: &str = "motorway|motorway_link|trunk|trunk_link|primary|primary_link|secondary|secondary_link|tertiary|tertiary_link|unclassified|residential|living_street|service|pedestrian";

/// The ways the importer reads as a bike track of their own (`osm_import`'s `is_track`): a cycleway, or a path or
/// footway designated for bicycles.
const TRACKS: &str = "[\"highway\"=\"cycleway\"]";
const DESIGNATED_TRACKS: &str = "[\"highway\"~\"^(path|footway)$\"][\"bicycle\"=\"designated\"]";

/// The Overpass query for every road and bike track in `b`, with the nodes they run through, as OSM XML. The nodes are
/// asked for after the union: inside it, `>;` would recurse only from the statement before it.
pub fn query(b: Bounds) -> String {
    let area = format!("({:.6},{:.6},{:.6},{:.6})", b.south, b.west, b.north, b.east);
    format!("[out:xml][timeout:25];(way[\"highway\"~\"^({HIGHWAYS})$\"]{area};way{TRACKS}{area};way{DESIGNATED_TRACKS}{area};);(._;>;);out;")
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
        assert!(q.contains("residential") && !q.contains("steps"));
    }

    #[test]
    fn the_query_also_asks_for_bike_tracks_mapped_as_ways_of_their_own() {
        let q = query(Bounds { south: 51.4990, west: -0.1290, north: 51.5010, east: -0.1260 });
        let area = "(51.499000,-0.129000,51.501000,-0.126000)";
        assert!(q.contains(&format!("way[\"highway\"=\"cycleway\"]{area};")));
        assert!(q.contains(&format!("way[\"highway\"~\"^(path|footway)$\"][\"bicycle\"=\"designated\"]{area};")));
        // Inside a union, `>;` recurses only from the statement before it: the nodes are asked for after the union,
        // from all of its ways.
        assert!(q.contains(&format!("{area};);(._;>;);out;")), "{q}");
    }
}
