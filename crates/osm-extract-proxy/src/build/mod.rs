//! `build` — the offline regional-data build pipeline (W1, BR8.x): verify
//! each configured region's checksum, filter to what osm2streets reads,
//! slice into the grid, and publish a manifest and a packed store, moved
//! into place only once every region has succeeded (RD-8).

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use md5::Digest as _;
use thiserror::Error;

use crate::decode::decode;
use crate::encode::encode;
use crate::geo::CanonicalBox;
use crate::grid::GridSpec;
use crate::manifest::{BuildId, CellEntry, Digest, Manifest, RegionRecord};
use crate::osm::Clip;
use crate::store::StoreWriter;

/// One configured source region (`data/regions.toml`): where to fetch it
/// from and the area it covers (BR8.4's `Region.bounds`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionConfig {
    pub name: String,
    pub pbf_url: String,
    pub md5_url: String,
    pub bounds: String,
}

/// Fetches one region's raw PBF bytes and the publisher's claimed MD5 text
/// (BR8.1): `reqwest` in production, a local-file stand-in in tests, so the
/// checksum and decode logic run unchanged against a small synthetic file.
pub trait RegionSource {
    fn fetch(&self, region: &RegionConfig) -> Result<(Vec<u8>, String), BuildError>;
}

/// Why the build failed; a build failure publishes nothing (BR8.1, BR8.4).
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("region {0} could not be fetched")]
    FetchFailed(String),
    #[error("region {0} failed its published checksum")]
    ChecksumMismatch(String),
    #[error("region {0} could not be decoded as PBF")]
    DecodeFailed(String),
    #[error("region {0} has an invalid bounds bbox")]
    InvalidBounds(String),
    #[error("writing the build output failed: {0}")]
    Io(#[from] std::io::Error),
}

/// What a successful build produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildOutput {
    pub build_id: BuildId,
    pub cell_count: u64,
}

/// Run the full pipeline: verify, filter (BR8.2), slice (BR8.3), write
/// (BR8.4, RD-8). `built_at` is supplied by the caller (RFC 3339 text) so
/// the pure-enough parts of this function stay deterministic in tests;
/// the binary passes the real time.
pub fn build(
    regions: &[RegionConfig],
    source: &dyn RegionSource,
    grid: GridSpec,
    out_dir: &Path,
    built_at: &str,
) -> Result<BuildOutput, BuildError> {
    fs::create_dir_all(out_dir)?;
    let tmp_dir = out_dir.join(".build-tmp");
    fs::create_dir_all(&tmp_dir)?;

    let mut region_digests = Vec::with_capacity(regions.len());
    let mut region_records = Vec::with_capacity(regions.len());
    let mut coverage = Vec::with_capacity(regions.len());
    let mut cells: BTreeMap<crate::grid::CellId, Clip> = BTreeMap::new();

    for region in regions {
        let (bytes, expected_md5) = source.fetch(region)?;

        // BR8.1 — verify against the publisher's checksum before use.
        let actual_md5: String = md5::Md5::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if !actual_md5.eq_ignore_ascii_case(expected_md5.trim()) {
            return Err(BuildError::ChecksumMismatch(region.name.clone()));
        }

        let mut clip = decode(&bytes).map_err(|_| BuildError::DecodeFailed(region.name.clone()))?;

        // BR8.2 — keep every way carrying a highway tag and the nodes it
        // references; record the profile in the manifest.
        clip.ways
            .retain(|way| way.tags.iter().any(|(k, _)| k == "highway"));
        let kept_node_ids: HashSet<i64> = clip
            .ways
            .iter()
            .flat_map(|way| way.refs.iter().copied())
            .collect();
        clip.nodes.retain(|node| kept_node_ids.contains(&node.id));
        clip.normalise();

        let bounds = CanonicalBox::parse(&region.bounds)
            .map_err(|_| BuildError::InvalidBounds(region.name.clone()))?;
        coverage.push(grid.touched(&bounds).rect());

        // BR8.3 — a way goes into every cell its geometry touches, with
        // every node it references.
        for way in &clip.ways {
            let Some(extent) = clip.way_extent(way) else {
                continue;
            };
            for cell_id in grid.touched_by_extent(&extent).iter() {
                let cell = cells.entry(cell_id).or_default();
                cell.ways.push(way.clone());
                for node_id in &way.refs {
                    if let Some(node) = clip.nodes.iter().find(|n| n.id == *node_id) {
                        cell.nodes.push(node.clone());
                    }
                }
            }
        }

        let digest = Digest::from_bytes(*blake3::hash(&bytes).as_bytes());
        region_digests.push(digest);
        region_records.push(RegionRecord {
            source_path: region.name.clone(),
            source_published_at: built_at.to_string(),
            source_digest: digest,
            source_bytes: bytes.len() as u64,
            bounds: bounds.key_text(),
        });
    }

    let build_id = Manifest::derive_build_id(&region_digests, &grid);

    let store_tmp = tmp_dir.join("store.bin");
    let mut writer = StoreWriter::create(&store_tmp)?;
    let mut cell_entries = Vec::with_capacity(cells.len());
    let mut total_bytes = 0u64;
    for (cell_id, mut clip) in cells {
        clip.normalise();
        let way_count = clip.way_count() as u32;
        let node_count = clip.node_count() as u32;
        let encoded = encode(&clip);
        let (offset, length, digest) = writer.write_cell(&encoded)?;
        total_bytes += length;
        cell_entries.push(CellEntry {
            cell_id,
            offset,
            length,
            digest,
            way_count,
            node_count,
        });
    }
    writer.flush()?;

    let manifest = Manifest {
        build_id: build_id.clone(),
        built_at: built_at.to_string(),
        grid_spec: grid,
        regions: region_records,
        filter_profile: "highway-v1".to_string(),
        cell_count: cell_entries.len() as u64,
        total_bytes,
        coverage,
        cells: cell_entries,
    };
    let cell_count = manifest.cell_count;
    let manifest_tmp = tmp_dir.join("manifest.json");
    fs::write(&manifest_tmp, manifest.to_json())?;

    // RD-8 — move into place only once everything else has succeeded.
    fs::rename(&store_tmp, out_dir.join("store.bin"))?;
    fs::rename(&manifest_tmp, out_dir.join("manifest.json"))?;
    let _ = fs::remove_dir(&tmp_dir);

    Ok(BuildOutput {
        build_id,
        cell_count,
    })
}

/// A tiny synthetic build, entirely in memory: one region, one way, used by
/// `region-build --synthetic` so the service can be run locally without a
/// real download (`README.md`).
pub fn synthetic_region_bytes() -> Vec<u8> {
    let clip = Clip {
        nodes: vec![
            crate::osm::Node {
                id: 1,
                lat: 436_500_000,
                lon: -796_300_000,
                tags: Vec::new(),
            },
            crate::osm::Node {
                id: 2,
                lat: 436_600_000,
                lon: -796_200_000,
                tags: Vec::new(),
            },
        ],
        ways: vec![crate::osm::Way {
            id: 1,
            refs: vec![1, 2],
            tags: vec![("highway".to_string(), "residential".to_string())],
        }],
    };
    encode(&clip)
}

/// The current time as `YYYY-MM-DDTHH:MM:SSZ` (RFC 3339, UTC, whole
/// seconds) without a `chrono` dependency: Howard Hinnant's public-domain
/// `civil_from_days` algorithm converts days-since-epoch to a Gregorian
/// date. Used for `Manifest.builtAt` and `RegionRecord.sourcePublishedAt`.
pub fn now_rfc3339() -> String {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    rfc3339_from_unix_secs(since_epoch.as_secs())
}

fn rfc3339_from_unix_secs(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;
    let (h, m, s) = (
        time_of_day / 3_600,
        (time_of_day / 60) % 60,
        time_of_day % 60,
    );
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::CoverageIndex;
    use std::sync::Mutex;

    /// A `RegionSource` backed by an in-memory table, keyed by region name:
    /// exactly the pipeline's checksum and decode logic, without touching
    /// the network or a temp file (`unit-test-instructions.md`).
    struct FixtureSource {
        table: Mutex<std::collections::HashMap<String, (Vec<u8>, String)>>,
    }

    impl FixtureSource {
        fn new() -> FixtureSource {
            FixtureSource {
                table: Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn set(&self, name: &str, bytes: Vec<u8>, claimed_md5: Option<String>) {
            let md5 = claimed_md5.unwrap_or_else(|| {
                md5::Md5::digest(&bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect()
            });
            self.table
                .lock()
                .unwrap()
                .insert(name.to_string(), (bytes, md5));
        }
    }

    impl RegionSource for FixtureSource {
        fn fetch(&self, region: &RegionConfig) -> Result<(Vec<u8>, String), BuildError> {
            self.table
                .lock()
                .unwrap()
                .get(&region.name)
                .cloned()
                .ok_or_else(|| BuildError::FetchFailed(region.name.clone()))
        }
    }

    fn way_clip(way_id: i64, node_ids: &[i64], highway: bool, span_two_cells: bool) -> Vec<u8> {
        // A grid cell is 100,000 raw units wide (1,000 units of 1e-5 degree
        // at 100 raw units per unit); an offset comfortably past that lands
        // the second node in the neighbouring cell.
        let mut nodes = Vec::new();
        for (i, id) in node_ids.iter().enumerate() {
            let offset = if span_two_cells && i == 1 { 150_000 } else { 0 };
            nodes.push(crate::osm::Node {
                id: *id,
                lat: 1_000,
                lon: 1_000 + offset,
                tags: Vec::new(),
            });
        }
        let tags = if highway {
            vec![("highway".to_string(), "residential".to_string())]
        } else {
            vec![("landuse".to_string(), "residential".to_string())]
        };
        let clip = Clip {
            nodes,
            ways: vec![crate::osm::Way {
                id: way_id,
                refs: node_ids.to_vec(),
                tags,
            }],
        };
        encode(&clip)
    }

    fn region(name: &str, bounds: &str) -> RegionConfig {
        RegionConfig {
            name: name.to_string(),
            pbf_url: format!("https://example.invalid/{name}.osm.pbf"),
            md5_url: format!("https://example.invalid/{name}.osm.pbf.md5"),
            bounds: bounds.to_string(),
        }
    }

    // BR8.1 — a corrupted download or a mismatched checksum both fail the
    // build, with nothing written.
    #[test]
    fn a_mismatched_checksum_fails_the_build_with_nothing_written() {
        let dir = tempfile::tempdir().unwrap();
        let source = FixtureSource::new();
        source.set("a", way_clip(1, &[1, 2], true, false), Some("0".repeat(32)));
        let err = build(
            &[region("a", "0,0,0.01,0.01")],
            &source,
            GridSpec::default_cells(),
            dir.path(),
            "2026-09-15T00:00:00Z",
        )
        .unwrap_err();
        assert!(matches!(err, BuildError::ChecksumMismatch(_)));
        assert!(!dir.path().join("store.bin").exists());
        assert!(!dir.path().join("manifest.json").exists());
    }

    // BR8.2 — highway ways and their nodes are kept; the profile is recorded.
    #[test]
    fn keeps_highway_ways_and_records_the_profile() {
        let dir = tempfile::tempdir().unwrap();
        let source = FixtureSource::new();
        source.set("a", way_clip(1, &[1, 2], false, false), None);
        let output = build(
            &[region("a", "0,0,0.01,0.01")],
            &source,
            GridSpec::default_cells(),
            dir.path(),
            "2026-09-15T00:00:00Z",
        )
        .unwrap();
        let manifest =
            Manifest::from_json(&fs::read_to_string(dir.path().join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest.filter_profile, "highway-v1");
        assert_eq!(manifest.cell_count, 0, "the only way was not a highway");
        assert_eq!(output.cell_count, 0);
    }

    // BR8.3 — a way is written into every cell its geometry touches.
    #[test]
    fn writes_a_way_into_every_cell_it_touches() {
        let dir = tempfile::tempdir().unwrap();
        let source = FixtureSource::new();
        source.set("a", way_clip(1, &[1, 2], true, true), None);
        build(
            &[region("a", "0,0,0.02,0.01")],
            &source,
            GridSpec::default_cells(),
            dir.path(),
            "2026-09-15T00:00:00Z",
        )
        .unwrap();
        let manifest =
            Manifest::from_json(&fs::read_to_string(dir.path().join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest.cell_count, 2, "the way spans two cells");
        for cell in &manifest.cells {
            assert_eq!(cell.way_count, 1);
        }
    }

    // BR8.4 / RD-8 — store and manifest move into place only at the end;
    // running twice with the same inputs yields the same buildId and
    // byte-identical output.
    #[test]
    fn the_same_inputs_yield_the_same_build_id_and_byte_identical_output() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let bytes = way_clip(1, &[1, 2], true, false);
        let source1 = FixtureSource::new();
        source1.set("a", bytes.clone(), None);
        let source2 = FixtureSource::new();
        source2.set("a", bytes, None);
        let grid = GridSpec::default_cells();
        let out1 = build(
            &[region("a", "0,0,0.01,0.01")],
            &source1,
            grid,
            dir1.path(),
            "2026-09-15T00:00:00Z",
        )
        .unwrap();
        let out2 = build(
            &[region("a", "0,0,0.01,0.01")],
            &source2,
            grid,
            dir2.path(),
            "2026-09-15T00:00:00Z",
        )
        .unwrap();
        assert_eq!(out1.build_id, out2.build_id);
        assert_eq!(
            fs::read(dir1.path().join("store.bin")).unwrap(),
            fs::read(dir2.path().join("store.bin")).unwrap()
        );
        assert_eq!(
            fs::read(dir1.path().join("manifest.json")).unwrap(),
            fs::read(dir2.path().join("manifest.json")).unwrap()
        );
        assert!(!dir1.path().join(".build-tmp").exists());
    }

    // NFR2.1.4 — a box in each region is covered and a box between them is not.
    #[test]
    fn a_box_in_each_region_is_covered_and_a_box_between_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let source = FixtureSource::new();
        source.set("a", way_clip(1, &[1, 2], true, false), None);
        source.set("b", way_clip(2, &[3, 4], true, false), None);
        let manifest_regions = [
            region("a", "-63.20,46.20,-63.10,46.30"),
            region("b", "-63.00,46.20,-62.90,46.30"),
        ];
        build(
            &manifest_regions,
            &source,
            GridSpec::default_cells(),
            dir.path(),
            "2026-09-15T00:00:00Z",
        )
        .unwrap();
        let manifest =
            Manifest::from_json(&fs::read_to_string(dir.path().join("manifest.json")).unwrap())
                .unwrap();
        let coverage = CoverageIndex::from_rectangles(&manifest.coverage);
        let grid = GridSpec::default_cells();
        let inside_a = grid.touched(&CanonicalBox::parse("-63.195,46.205,-63.185,46.215").unwrap());
        assert!(coverage.covers_all(&inside_a));
        let inside_b = grid.touched(&CanonicalBox::parse("-62.995,46.205,-62.985,46.215").unwrap());
        assert!(coverage.covers_all(&inside_b));
        let between = grid.touched(&CanonicalBox::parse("-63.05,46.205,-63.04,46.215").unwrap());
        assert!(!coverage.covers_all(&between));
    }

    // now_rfc3339's civil-date conversion against known epoch instants.
    #[test]
    fn rfc3339_from_unix_secs_matches_known_instants() {
        assert_eq!(rfc3339_from_unix_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            rfc3339_from_unix_secs(1_577_836_800),
            "2020-01-01T00:00:00Z"
        );
        assert_eq!(
            rfc3339_from_unix_secs(1_893_456_000),
            "2030-01-01T00:00:00Z"
        );
        assert_eq!(
            rfc3339_from_unix_secs(86_400 + 3_661),
            "1970-01-02T01:01:01Z"
        );
    }
}
