//! The tile index the app reads (`src/place/tiles.rs`): the grid, the tiles there are and a checksum of each.
//! One writer for `cityloom_prepare tiles` and for the server's unpack, so the two files are the same bytes.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// The south-west corner of tile 0_0, in degrees.
    pub lon0: f64,
    pub lat0: f64,
    /// The size of a tile's core, in degrees.
    pub dlon: f64,
    pub dlat: f64,
    /// The grid's size in tiles (kept in the archive's index, not in `index.json`).
    pub nx: u16,
    pub ny: u16,
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The checksum `index.json` keeps for a tile: the first 16 hex digits of its SHA-256.
pub fn short_checksum(sha256: &[u8; 32]) -> String {
    hex(&sha256[..8])
}

/// `{"lon0":..,"lat0":..,"dlon":..,"dlat":..,"tiles":[..],"checksums":{..}}` and a newline, as Python's
/// `json.dumps(.., separators=(",", ":"))` wrote it. A float is printed in Rust's shortest form that reads back
/// the same, which is Python's `repr` for every value the grid holds; a tile name is `<x>_<y>`, so nothing is escaped.
pub fn index_json(grid: &Grid, tiles: &[(String, String)]) -> String {
    debug_assert!(tiles.iter().all(|(name, _)| name.bytes().all(|b| b.is_ascii_digit() || b == b'_')));
    let names: Vec<String> = tiles.iter().map(|(name, _)| format!("\"{name}\"")).collect();
    let sums: Vec<String> = tiles.iter().map(|(name, sum)| format!("\"{name}\":\"{sum}\"")).collect();
    format!(
        "{{\"lon0\":{},\"lat0\":{},\"dlon\":{},\"dlat\":{},\"tiles\":[{}],\"checksums\":{{{}}}}}\n",
        grid.lon0,
        grid.lat0,
        grid.dlon,
        grid.dlat,
        names.join(","),
        sums.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid { lon0: -74.4, lat0: 45.2, dlon: 0.0257, dlat: 0.018, nx: 51, ny: 34 }
    }

    #[test]
    fn the_index_is_the_json_the_python_script_wrote() {
        // python3: json.dumps({"lon0": -74.40, "lat0": 45.20, "dlon": 0.0257, "dlat": 0.018, "tiles": ["0_0", "12_9"],
        //   "checksums": {"0_0": "ab12", "12_9": "cd34"}}, separators=(",", ":")) + "\n"
        let tiles = vec![("0_0".to_string(), "ab12".to_string()), ("12_9".to_string(), "cd34".to_string())];
        assert_eq!(
            index_json(&grid(), &tiles),
            "{\"lon0\":-74.4,\"lat0\":45.2,\"dlon\":0.0257,\"dlat\":0.018,\"tiles\":[\"0_0\",\"12_9\"],\"checksums\":{\"0_0\":\"ab12\",\"12_9\":\"cd34\"}}\n"
        );
    }

    #[test]
    fn no_tiles_is_empty_lists() {
        assert!(index_json(&grid(), &[]).ends_with("\"tiles\":[],\"checksums\":{}}\n"));
    }

    #[test]
    fn the_short_checksum_is_the_first_sixteen_hex_digits() {
        let mut sha = [0u8; 32];
        sha[0] = 0xab;
        sha[7] = 0x01;
        sha[8] = 0xff;
        assert_eq!(short_checksum(&sha), "ab00000000000001");
        assert_eq!(hex(&[0x00, 0x0f, 0xa0]), "000fa0");
    }
}
