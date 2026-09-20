//! `manifest` — the build manifest (`RegionalDataBuild`, `Region`, `Cell` of
//! `entities.md`; BR8.4) and the content-derived `buildId` (TS-5). Pure.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::grid::{CellId, CellRect, GridSpec};

/// A 32-byte BLAKE3 digest, serialised as lowercase hex (64 characters).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Digest([u8; 32]);

impl Digest {
    pub fn from_bytes(bytes: [u8; 32]) -> Digest {
        Digest(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for b in self.0 {
            out.push_str(&format!("{b:02x}"));
        }
        out
    }

    pub fn from_hex(text: &str) -> Result<Digest, ManifestError> {
        if text.len() != 64
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ManifestError::InvalidDigest);
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
                .map_err(|_| ManifestError::InvalidDigest)?;
        }
        Ok(Digest(bytes))
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({})", self.to_hex())
    }
}

impl Serialize for Digest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Digest, D::Error> {
        let text = String::deserialize(deserializer)?;
        Digest::from_hex(&text).map_err(serde::de::Error::custom)
    }
}

/// The build manifest could not be read (BR9.1, unready with `check:
/// "manifest"`).
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ManifestError {
    #[error("the manifest is not valid JSON or is missing a required field")]
    Malformed,
    #[error("a digest is not 64 lowercase hex characters")]
    InvalidDigest,
    #[error("the cells are not sorted by id or contain a duplicate")]
    UnsortedOrDuplicateCells,
}

/// A validated build identifier: 64 lowercase hex characters (TS-5).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BuildId(String);

/// A `buildId` text is not 64 lowercase hex characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("a buildId must be 64 lowercase hex characters")]
pub struct InvalidBuildId;

impl BuildId {
    pub fn parse(text: &str) -> Result<BuildId, InvalidBuildId> {
        if text.len() == 64
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(BuildId(text.to_string()))
        } else {
            Err(InvalidBuildId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for BuildId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for BuildId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<BuildId, D::Error> {
        let text = String::deserialize(deserializer)?;
        BuildId::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// One configured source region's provenance (`entities.md` `Region`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionRecord {
    pub source_path: String,
    pub source_published_at: String,
    pub source_digest: Digest,
    pub source_bytes: u64,
    pub bounds: String,
}

/// One cell's location in the packed store (`entities.md` `CellEntry`).
/// Kept at or under 80 bytes (NFR2.1.5) so `cellCount` cells cost a bounded
/// amount of memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellEntry {
    pub cell_id: CellId,
    pub offset: u64,
    pub length: u64,
    pub digest: Digest,
    pub way_count: u32,
    pub node_count: u32,
}

/// The build manifest: everything the service needs to answer a request
/// without touching the regional source data again (BR8.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub build_id: BuildId,
    pub built_at: String,
    pub grid_spec: GridSpec,
    pub regions: Vec<RegionRecord>,
    pub filter_profile: String,
    pub cell_count: u64,
    pub total_bytes: u64,
    pub coverage: Vec<CellRect>,
    pub cells: Vec<CellEntry>,
}

impl Manifest {
    /// The content-derived `buildId`: BLAKE3 over the region digests in
    /// configured order, then the grid spec (BR8.4, TS-5).
    pub fn derive_build_id(region_digests: &[Digest], grid: &GridSpec) -> BuildId {
        let mut hasher = blake3::Hasher::new();
        for digest in region_digests {
            hasher.update(digest.as_bytes());
            hasher.update(b"\n");
        }
        hasher.update(grid.cell_size_text().as_bytes());
        BuildId(hasher.finalize().to_hex().to_string())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Manifest always serialises")
    }

    /// Parse and validate: the cells must be sorted by id and unique
    /// (the reader's binary search over `store.rs` relies on this).
    pub fn from_json(text: &str) -> Result<Manifest, ManifestError> {
        let manifest: Manifest =
            serde_json::from_str(text).map_err(|_| ManifestError::Malformed)?;
        let sorted = manifest
            .cells
            .windows(2)
            .all(|pair| pair[0].cell_id < pair[1].cell_id);
        if !sorted {
            return Err(ManifestError::UnsortedOrDuplicateCells);
        }
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{CellId, CellRect, GridSpec};

    fn digest(byte: u8) -> Digest {
        Digest::from_bytes([byte; 32])
    }

    fn sample() -> Manifest {
        Manifest {
            build_id: Manifest::derive_build_id(
                &[digest(1), digest(2)],
                &GridSpec::default_cells(),
            ),
            built_at: "2026-09-15T12:00:00Z".to_string(),
            grid_spec: GridSpec::default_cells(),
            regions: vec![RegionRecord {
                source_path: "north-america/canada/prince-edward-island".to_string(),
                source_published_at: "2026-09-14T20:21:00Z".to_string(),
                source_digest: digest(1),
                source_bytes: 10_800_000,
                bounds: "-64.50000,45.90000,-61.90000,47.10000".to_string(),
            }],
            filter_profile: "highway-v1".to_string(),
            cell_count: 2,
            total_bytes: 300,
            coverage: vec![CellRect {
                col_min: -6450,
                col_max: -6191,
                row_min: 4590,
                row_max: 4709,
            }],
            cells: vec![
                CellEntry {
                    cell_id: CellId::new(-6320, 4620),
                    offset: 12,
                    length: 100,
                    digest: digest(7),
                    way_count: 3,
                    node_count: 9,
                },
                CellEntry {
                    cell_id: CellId::new(-6295, 4625),
                    offset: 112,
                    length: 200,
                    digest: digest(8),
                    way_count: 1,
                    node_count: 2,
                },
            ],
        }
    }

    // BR8.4 — the manifest round-trips through JSON with its wire spelling.
    #[test]
    fn json_round_trip_preserves_every_field() {
        let m = sample();
        let json = m.to_json();
        let back = Manifest::from_json(&json).expect("parses");
        assert_eq!(back, m);
        for key in [
            "\"buildId\"",
            "\"builtAt\"",
            "\"gridSpec\"",
            "\"cellSizeDegrees\":\"0.01\"",
            "\"coordinateSystem\":\"WGS84\"",
            "\"regions\"",
            "\"sourcePath\"",
            "\"sourcePublishedAt\"",
            "\"sourceDigest\"",
            "\"sourceBytes\"",
            "\"bounds\"",
            "\"filterProfile\"",
            "\"cellCount\"",
            "\"totalBytes\"",
            "\"coverage\"",
            "\"cells\"",
            "\"cellId\"",
            "\"offset\"",
            "\"length\"",
            "\"digest\"",
            "\"wayCount\"",
            "\"nodeCount\"",
        ] {
            assert!(json.contains(key), "missing {key} in {json}");
        }
        // Digests are lowercase hex on the wire.
        assert!(json.contains(&"07".repeat(32)));
    }

    #[test]
    fn malformed_json_is_a_typed_error() {
        assert!(Manifest::from_json("").is_err());
        assert!(Manifest::from_json("{\"buildId\": 1}").is_err());
        assert!(Manifest::from_json("{}").is_err());
        let bad_digest = sample().to_json().replace(&"07".repeat(32), "zz");
        assert!(Manifest::from_json(&bad_digest).is_err());
    }

    // BR8.4 / TS-5 — buildId is BLAKE3 over the region digests in configured
    // order plus the GridSpec: stable across runs, changed by any input.
    #[test]
    fn build_id_is_stable_and_content_derived() {
        let grid = GridSpec::default_cells();
        let a = Manifest::derive_build_id(&[digest(1), digest(2)], &grid);
        let b = Manifest::derive_build_id(&[digest(1), digest(2)], &grid);
        assert_eq!(a, b);
        assert_eq!(a.as_str().len(), 64);
        assert!(
            a.as_str()
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_ne!(
            a,
            Manifest::derive_build_id(&[digest(2), digest(1)], &grid),
            "order matters"
        );
        assert_ne!(
            a,
            Manifest::derive_build_id(&[digest(1), digest(3)], &grid),
            "a digest matters"
        );
        assert_ne!(
            a,
            Manifest::derive_build_id(&[digest(1)], &grid),
            "the region count matters"
        );
        assert_ne!(
            a,
            Manifest::derive_build_id(
                &[digest(1), digest(2)],
                &GridSpec::from_cell_size("0.02").unwrap()
            ),
            "the grid matters"
        );
    }

    #[test]
    fn build_id_text_is_validated() {
        assert!(BuildId::parse(&"ab".repeat(32)).is_ok());
        assert!(BuildId::parse(&"AB".repeat(32)).is_err(), "lowercase only");
        assert!(BuildId::parse("abc").is_err(), "64 hex characters");
        assert!(BuildId::parse(&"zz".repeat(32)).is_err());
    }

    // NFR2.1.5 — the in-memory manifest is proportional to cellCount at
    // no more than 80 bytes per cell.
    #[test]
    fn a_cell_entry_is_at_most_eighty_bytes() {
        assert!(std::mem::size_of::<CellEntry>() <= 80);
        let mut m = sample();
        m.cells = (0..10_000)
            .map(|i| CellEntry {
                cell_id: CellId::new(i % 100, i / 100),
                offset: u64::from(i as u32) * 10,
                length: 10,
                digest: digest(1),
                way_count: 1,
                node_count: 1,
            })
            .collect();
        m.cells.shrink_to_fit();
        assert!(m.cells.capacity() * std::mem::size_of::<CellEntry>() <= 80 * 10_000);
    }

    #[test]
    fn cells_must_be_sorted_by_id_and_unique() {
        let mut m = sample();
        m.cells.swap(0, 1);
        assert!(
            Manifest::from_json(&m.to_json()).is_err(),
            "unsorted cells are rejected"
        );
        let mut m = sample();
        m.cells[1].cell_id = m.cells[0].cell_id;
        assert!(
            Manifest::from_json(&m.to_json()).is_err(),
            "duplicate cells are rejected"
        );
    }
}
