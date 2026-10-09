//! The OpenStreetMap data of a whole metropolitan area, cut into tiles that are fetched one at a time.
//!
//! A tile is a rectangle of the earth (its core) and holds every road that reaches into the core
//! widened by a margin, whole. A margin wider than half an area makes one tile enough for an
//! area whose centre lies in the tile's core, so a place is read from a single file.
//! `scripts/metro-tiles.sh` cuts the tiles and writes the index that names the grid and the tiles there are.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::shared::said::{Arg, Said};

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
    /// A checksum of each tile's file, by name, so that a tile that has changed can be told from one that has not.
    /// An index made before the checksums were has none.
    #[serde(default)]
    checksums: HashMap<String, String>,
}

impl Index {
    pub fn parse(json: &[u8]) -> Result<Index, Said> {
        let index: Index = serde_json::from_slice(json).map_err(|e| Said::new("place-index-not-understood").with("e", Arg::Text(e.to_string())))?;
        if index.dlon <= 0.0 || index.dlat <= 0.0 {
            return Err(Said::new("place-index-no-size"));
        }
        Ok(index)
    }

    /// The name of the tile whose core holds the point, if there is such a tile.
    fn name_at(&self, lat: f64, lon: f64) -> Option<String> {
        let (x, y) = ((lon - self.lon0) / self.dlon, (lat - self.lat0) / self.dlat);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let name = format!("{}_{}", x.floor() as i64, y.floor() as i64);
        self.tiles.contains(&name).then_some(name)
    }

    /// The tile whose core holds the point, as the name of its file, if there is such a tile.
    pub fn tile_at(&self, lat: f64, lon: f64) -> Option<String> {
        self.name_at(lat, lon).map(|name| format!("{DIR}/{name}.osm.pbf"))
    }

    /// The checksum of that tile's file, if the index gives one.
    pub fn checksum_at(&self, lat: f64, lon: f64) -> Option<String> {
        self.checksums.get(&self.name_at(lat, lon)?).cloned()
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
    fn a_tile_has_the_checksum_the_index_gives_it_if_it_gives_one() {
        let index = Index::parse(br#"{"lon0":-74.40,"lat0":45.20,"dlon":0.0257,"dlat":0.018,"tiles":["12_9","12_10"],"checksums":{"12_9":"ab12"}}"#).unwrap();
        assert_eq!(index.checksum_at(45.37, -74.08).as_deref(), Some("ab12"));
        assert_eq!(index.checksum_at(45.39, -74.08), None, "that tile has none");
        assert_eq!(index.checksum_at(48.85, 2.35), None, "no tile");
        assert_eq!(Index::parse(INDEX).unwrap().checksum_at(45.37, -74.08), None, "an index with none");
    }

    #[test]
    fn an_index_that_is_not_one_is_an_error() {
        let en = |s: Said| crate::shared::said::say_now(&crate::i18n_for(crate::shared::i18n::Locale::En), crate::shared::units::Units::Metres, &s);
        assert!(en(Index::parse(b"<html>").unwrap_err()).starts_with("the tile index was not understood (expected value"));
        assert_eq!(en(Index::parse(br#"{"lon0":0,"lat0":0,"dlon":0,"dlat":0.1,"tiles":[]}"#).unwrap_err()), "the tile index has no tile size");
    }
}
