//! The street network of a whole metropolitan area, cut into tiles that are fetched one at a time.
//!
//! A tile is a rectangle of the earth (its core) and holds every road that reaches into the core
//! widened by a margin, whole, in the frame of the whole area (`cityloom_model::NetworkTile`). A margin
//! wider than half an area makes one tile normally enough for an area whose centre lies in the tile's
//! core; an area whose box reaches other tiles' cores is read from those too, merged by id, so that it
//! does not rest on the margin alone.
//! `citymodel_builder` makes the tiles and writes the index that names the grid and the tiles there are.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use cityloom_model::with_importer;

use super::Bounds;
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
        self.name_at(lat, lon).map(|name| format!("{DIR}/{name}.json"))
    }

    /// Every tile there is whose core meets the box (a core that only touches its edge does not), as the names of
    /// their files, the one holding the box's centre first and the others by row, south to north, then west to east.
    pub fn tiles_touching(&self, b: Bounds) -> Vec<String> {
        // the first and last column and row a box from `low` to `high` reaches, along an axis from `origin` in steps of `size`
        let span = |low: f64, high: f64, origin: f64, size: f64| ((low - origin) / size).floor() as i64..=(((high - origin) / size).ceil() as i64 - 1);
        let centre = self.name_at((b.south + b.north) / 2.0, (b.west + b.east) / 2.0);
        let mut names: Vec<String> = centre.iter().cloned().collect();
        for y in span(b.south, b.north, self.lat0, self.dlat) {
            for x in span(b.west, b.east, self.lon0, self.dlon) {
                let name = format!("{x}_{y}");
                if self.tiles.contains(&name) && centre.as_ref() != Some(&name) {
                    names.push(name);
                }
            }
        }
        names.into_iter().map(|name| format!("{DIR}/{name}.json")).collect()
    }

    /// What an area read from that tile is kept as made from: the checksum of the tile's file, if the index gives
    /// one, and the version of the importer that read it. An area kept by an earlier importer therefore reads as
    /// stale and is read again, once.
    pub fn checksum_at(&self, lat: f64, lon: f64) -> Option<String> {
        self.checksums.get(&self.name_at(lat, lon)?).map(|sum| with_importer(sum))
    }
}

/// Small hand-made tiles and the indexes that name them, for the tests of what reads them.
#[cfg(test)]
pub mod testing {
    use cityloom_model::Frame;

    /// The frame of a whole area that holds tiles 12_9 and 12_10 of `INDEX`, about 2.7 km by 3.3 km.
    pub const AREA: Frame = Frame { south: 45.36, west: -74.095, north: 45.39, east: -74.06 };

    /// An index of the grid round Montreal naming tiles 12_9 (core: lon -74.0916 .. -74.0659, lat 45.362 .. 45.38)
    /// and 12_10 (lat 45.38 .. 45.398), each with a checksum.
    pub const INDEX: &str = r#"{"lon0":-74.40,"lat0":45.20,"dlon":0.0257,"dlat":0.018,"tiles":["12_9","12_10"],"checksums":{"12_9":"new","12_10":"north"}}"#;

    /// A road of a tile: its id, its name and its points as (lat, lon). It runs from junction `2 * id` to `2 * id + 1`.
    pub type Road<'a> = (u32, &'a str, &'a [(f64, f64)]);

    /// A tile of the whole area in `frame` holding `roads`, as the builder writes it.
    pub fn tile(frame: &Frame, roads: &[Road]) -> Vec<u8> {
        let at = |&(lat, lon): &(f64, f64)| {
            let [x, y] = frame.local(lon, lat);
            format!("[{x},{y}]")
        };
        let junction = |id: u32, p: &(f64, f64)| {
            format!(r#"{{"id":{id},"provenance":[{{"node":{{"id":{id}}}}}],"position":{},"junction":false,"control":"none"}}"#, at(p))
        };
        let junctions: Vec<String> =
            roads.iter().flat_map(|(id, _, points)| [junction(2 * id, &points[0]), junction(2 * id + 1, points.last().unwrap())]).collect();
        let roads: Vec<String> = roads
            .iter()
            .map(|(id, name, points)| {
                format!(
                    r#"{{"edge":{{"id":{id},"from":{},"to":{}}},"provenance":[{{"way":{{"id":{id}}}}}],"name":"{name}","road_type":"local","zones":[{{"from":0.0,"lanes":[{{"kind":"driving","direction":"forward","width_m":3.0}},{{"kind":"driving","direction":"backward","width_m":3.0}}]}}],"points":[{}]}}"#,
                    2 * id,
                    2 * id + 1,
                    points.iter().map(at).collect::<Vec<_>>().join(",")
                )
            })
            .collect();
        format!(
            r#"{{"frame":{},"network":{{"left_hand":false,"junctions":[{}],"roads":[{}]}}}}"#,
            serde_json::to_string(frame).unwrap(),
            junctions.join(","),
            roads.join(",")
        )
        .into_bytes()
    }

    /// An index whose one tile, `data/metro/0_0.json`, holds the point, with `checksum` if there is one, and that
    /// tile, holding one short road named `name` through the point.
    pub fn covering(lat: f64, lon: f64, checksum: Option<&str>, name: &str) -> (Vec<u8>, Vec<u8>) {
        let sums = checksum.map(|c| format!(r#","checksums":{{"0_0":"{c}"}}"#)).unwrap_or_default();
        let index = format!(r#"{{"lon0":{},"lat0":{},"dlon":0.0257,"dlat":0.018,"tiles":["0_0"]{sums}}}"#, lon - 0.01, lat - 0.009);
        let frame = Frame { south: lat - 0.015, west: lon - 0.02, north: lat + 0.015, east: lon + 0.02 };
        (index.into_bytes(), tile(&frame, &[(1, name, &[(lat, lon - 0.001), (lat, lon + 0.001)])]))
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
        assert_eq!(index.tile_at(45.37, -74.08), Some("data/metro/12_9.json".to_string()));
        assert_eq!(index.tile_at(45.39, -74.08), Some("data/metro/12_10.json".to_string()));
        assert_eq!(index.tile_at(45.201, -74.399), Some("data/metro/0_0.json".to_string()));
    }

    /// A place's box, 800 m across, round a point.
    fn place(lat: f64, lon: f64) -> Bounds {
        Bounds::around(lat, lon, 400.0)
    }

    #[test]
    fn a_box_inside_one_core_touches_that_tile_only() {
        let index = Index::parse(INDEX).unwrap();
        // 12_9's core: lon -74.0916 .. -74.0659, lat 45.362 .. 45.38; the box is about 0.0102 by 0.0072 degrees
        assert_eq!(index.tiles_touching(place(45.371, -74.078)), vec!["data/metro/12_9.json".to_string()]);
    }

    #[test]
    fn a_box_reaching_other_listed_cores_touches_those_too_with_the_tile_holding_its_centre_first() {
        let index = Index::parse(INDEX).unwrap();
        let (t12_9, t12_10) = ("data/metro/12_9.json".to_string(), "data/metro/12_10.json".to_string());
        assert_eq!(index.tiles_touching(place(45.378, -74.078)), vec![t12_9.clone(), t12_10.clone()], "reaching north into 12_10");
        assert_eq!(index.tiles_touching(place(45.382, -74.078)), vec![t12_10.clone(), t12_9.clone()], "centred in 12_10, reaching south into 12_9");
        assert_eq!(index.tiles_touching(place(45.378, -74.068)), vec![t12_9, t12_10], "also reaching 13_9 and 13_10, which are not listed");
    }

    #[test]
    fn a_box_whose_centre_no_listed_tile_holds_touches_only_listed_tiles() {
        let index = Index::parse(INDEX).unwrap();
        assert_eq!(index.tiles_touching(place(45.371, -74.063)), vec!["data/metro/12_9.json".to_string()], "centred in 13_9, reaching west into 12_9");
        assert_eq!(index.tiles_touching(place(48.85, 2.35)), Vec::<String>::new(), "Paris");
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
        assert_eq!(index.checksum_at(45.37, -74.08), Some(with_importer("ab12")), "with the importer's version");
        assert!(with_importer("ab12").starts_with("ab12+i"));
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
