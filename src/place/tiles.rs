//! The OpenStreetMap data of a whole metropolitan area, cut into tiles that are fetched one at a time.
//!
//! A tile is a rectangle of the earth (its core) and holds every road that reaches into the core
//! widened by a margin, whole. A margin wider than half an area makes one tile enough for an
//! area whose centre lies in the tile's core, so a place is read from a single file.
//! `scripts/metro-tiles.sh` cuts the tiles and writes the index that names the grid and the tiles there are.

use std::collections::HashSet;

use serde::Deserialize;

/// Where the tiles are, relative to the pages.
pub const DIR: &str = "data/metro";

/// The index of the tiles there are, `data/metro/index.json`.
pub const INDEX_URL: &str = "data/metro/index.json";

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Index {
    /// The south-west corner of tile 0_0, in degrees.
    pub lon0: f64,
    pub lat0: f64,
    /// The size of a tile's core, in degrees.
    pub dlon: f64,
    pub dlat: f64,
    /// The tiles there are, as `x_y`.
    tiles: HashSet<String>,
}

impl Index {
    pub fn parse(json: &[u8]) -> Result<Index, String> {
        let index: Index = serde_json::from_slice(json).map_err(|e| format!("the tile index was not understood ({e})"))?;
        if index.dlon <= 0.0 || index.dlat <= 0.0 {
            return Err("the tile index has no tile size".to_string());
        }
        Ok(index)
    }

    /// The tile whose core holds the point, as the name of its file, if there is such a tile.
    pub fn tile_at(&self, lat: f64, lon: f64) -> Option<String> {
        let (x, y) = ((lon - self.lon0) / self.dlon, (lat - self.lat0) / self.dlat);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let name = format!("{}_{}", x.floor() as i64, y.floor() as i64);
        self.tiles.contains(&name).then(|| format!("{DIR}/{name}.osm.pbf"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &[u8] = br#"{"lon0":-74.40,"lat0":45.20,"dlon":0.0257,"dlat":0.018,"tiles":["0_0","12_9","12_10"]}"#;

    #[test]
    fn a_point_is_in_the_tile_whose_core_holds_it() {
        let index = Index::parse(INDEX).unwrap();
        // x: (-74.40 + 12 * 0.0257 = -74.0916) .. -74.0659; y: 45.20 + 9 * 0.018 = 45.362 .. 45.38
        assert_eq!(index.tile_at(45.37, -74.08), Some("data/metro/12_9.osm.pbf".to_string()));
        assert_eq!(index.tile_at(45.39, -74.08), Some("data/metro/12_10.osm.pbf".to_string()));
        assert_eq!(index.tile_at(45.201, -74.399), Some("data/metro/0_0.osm.pbf".to_string()));
    }

    #[test]
    fn a_point_outside_the_tiles_there_are_has_none() {
        let index = Index::parse(INDEX).unwrap();
        assert_eq!(index.tile_at(45.7, -73.5), None, "no such tile in the index");
        assert_eq!(index.tile_at(45.0, -74.5), None, "south-west of the grid");
        assert_eq!(index.tile_at(48.85, 2.35), None, "Paris");
    }

    #[test]
    fn an_index_that_is_not_one_is_an_error() {
        assert!(Index::parse(b"<html>").unwrap_err().contains("not understood"));
        assert!(Index::parse(br#"{"lon0":0,"lat0":0,"dlon":0,"dlat":0.1,"tiles":[]}"#).unwrap_err().contains("tile size"));
    }
}
