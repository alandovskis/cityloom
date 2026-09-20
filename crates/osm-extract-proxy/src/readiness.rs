//! `readiness` — the start-up sequence (RD-1, BR9.1): load the manifest,
//! open the store, verify every listed cell against its digest. The
//! service serves extracts only after all three succeed; until then, and
//! whenever a later reload fails, it is Unready with the check that failed
//! and, for a digest failure, how many cells failed it (OD-4).

use std::fs;
use std::path::Path;

use crate::manifest::Manifest;
use crate::store::Store;

/// Why the service is Unready (BR9.1): the check that failed, and — for a
/// digest failure — how many cells failed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnreadyReason {
    pub check: &'static str,
    pub failing_cells: usize,
}

/// Run the start-up sequence: read the manifest, open the store, verify
/// every cell. `Ok` carries what the service needs to serve requests.
pub fn start_up(
    manifest_path: impl AsRef<Path>,
    store_path: impl AsRef<Path>,
) -> Result<(Manifest, Store), UnreadyReason> {
    let text = fs::read_to_string(manifest_path).map_err(|_| UnreadyReason {
        check: "manifest",
        failing_cells: 0,
    })?;
    let manifest = Manifest::from_json(&text).map_err(|_| UnreadyReason {
        check: "manifest",
        failing_cells: 0,
    })?;
    let store = Store::open(store_path, &manifest.cells).map_err(|_| UnreadyReason {
        check: "store",
        failing_cells: manifest.cells.len(),
    })?;
    let failing = store.verify_all(&manifest.cells);
    if !failing.is_empty() {
        return Err(UnreadyReason {
            check: "digest",
            failing_cells: failing.len(),
        });
    }
    Ok((manifest, store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::{CellId, GridSpec};
    use crate::manifest::{CellEntry, RegionRecord};
    use crate::osm::Clip;
    use crate::store::StoreWriter;

    fn write_good_build(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let manifest_path = dir.join("manifest.json");
        let store_path = dir.join("store.bin");

        let mut writer = StoreWriter::create(&store_path).unwrap();
        let (offset, length, digest) = writer
            .write_cell(&crate::encode::encode(&Clip::default()))
            .unwrap();
        writer.flush().unwrap();

        let cell = CellEntry {
            cell_id: CellId::new(0, 0),
            offset,
            length,
            digest,
            way_count: 0,
            node_count: 0,
        };
        let grid = GridSpec::default_cells();
        let manifest = Manifest {
            build_id: Manifest::derive_build_id(&[digest], &grid),
            built_at: "2026-09-15T00:00:00Z".to_string(),
            grid_spec: grid,
            regions: vec![RegionRecord {
                source_path: "test-region".to_string(),
                source_published_at: "2026-09-14T00:00:00Z".to_string(),
                source_digest: digest,
                source_bytes: 0,
                bounds: "0,0,0.01,0.01".to_string(),
            }],
            filter_profile: "highway-v1".to_string(),
            cell_count: 1,
            total_bytes: length,
            coverage: Vec::new(),
            cells: vec![cell],
        };
        fs::write(&manifest_path, manifest.to_json()).unwrap();
        (manifest_path, store_path)
    }

    // BR9.1 — Ready on a good build.
    #[test]
    fn ready_on_a_good_build() {
        let dir = tempfile::tempdir().unwrap();
        let (manifest_path, store_path) = write_good_build(dir.path());
        assert!(start_up(&manifest_path, &store_path).is_ok());
    }

    // BR9.1 — Unready on a missing manifest.
    #[test]
    fn unready_on_a_missing_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let (_, store_path) = write_good_build(dir.path());
        let err = start_up(dir.path().join("no-such-manifest.json"), &store_path).unwrap_err();
        assert_eq!(err.check, "manifest");
    }

    // BR9.1 — Unready on an unreadable (malformed) manifest.
    #[test]
    fn unready_on_an_unreadable_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let (manifest_path, store_path) = write_good_build(dir.path());
        fs::write(&manifest_path, "not json").unwrap();
        let err = start_up(&manifest_path, &store_path).unwrap_err();
        assert_eq!(err.check, "manifest");
    }

    // BR9.1 — Unready on a missing store.
    #[test]
    fn unready_on_a_missing_store() {
        let dir = tempfile::tempdir().unwrap();
        let (manifest_path, store_path) = write_good_build(dir.path());
        fs::remove_file(&store_path).unwrap();
        let err = start_up(&manifest_path, &store_path).unwrap_err();
        assert_eq!(err.check, "store");
        assert_eq!(err.failing_cells, 1);
    }

    // BR9.1 — Unready on a digest mismatch, with the failing-cell count.
    #[test]
    fn unready_on_a_digest_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let (manifest_path, store_path) = write_good_build(dir.path());
        {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&store_path)
                .unwrap();
            file.write_all(b"corrupted!!").unwrap();
        }
        let err = start_up(&manifest_path, &store_path).unwrap_err();
        assert_eq!(err.check, "digest");
        assert_eq!(err.failing_cells, 1);
    }
}
