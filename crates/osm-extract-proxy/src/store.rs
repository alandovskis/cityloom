//! `store` — the packed cell store: a writer that appends cell payloads
//! back to back in cell-id order (used by `build/`) and a reader with a
//! sorted, binary-searchable index and BLAKE3 verification (PD-2, PD-7).
//!
//! A cell absent from the manifest's `cells` list is "covered but empty"
//! (NFR3.1.10): [`StoreIndex::find`] returning `None` is not an error, it is
//! the shape of a covered cell that happened to hold nothing.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;

use thiserror::Error;

use crate::grid::CellId;
use crate::manifest::{CellEntry, Digest};

/// One entry of the runtime index: a cell id and where its bytes live.
/// Exactly three `u64`s (24 bytes), sorted by `cell_id` for binary search
/// (PD-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IndexEntry {
    cell_id: CellId,
    offset: u64,
    length: u64,
}

/// The store's runtime index: sorted `(CellId, offset, length)` triples
/// built from the manifest's `cells`, searched by binary search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreIndex {
    entries: Vec<IndexEntry>,
}

impl StoreIndex {
    /// Build the index from the manifest's cells. The manifest's own
    /// `from_json` already rejects unsorted or duplicate cells, so this
    /// never needs to re-sort; it re-validates defensively regardless.
    pub fn from_cells(cells: &[CellEntry]) -> StoreIndex {
        let mut entries: Vec<IndexEntry> = cells
            .iter()
            .map(|c| IndexEntry {
                cell_id: c.cell_id,
                offset: c.offset,
                length: c.length,
            })
            .collect();
        entries.sort_by_key(|e| e.cell_id);
        StoreIndex { entries }
    }

    /// The `(offset, length)` of a cell, or `None` if the cell holds no
    /// entry at all (covered but empty, NFR3.1.10).
    fn find(&self, id: CellId) -> Option<(u64, u64)> {
        self.entries
            .binary_search_by_key(&id, |e| e.cell_id)
            .ok()
            .map(|i| (self.entries[i].offset, self.entries[i].length))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A store file could not be opened, read or verified.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("the store file could not be opened or read")]
    Io(#[from] io::Error),
    #[error("the store is missing a cell the manifest lists")]
    MissingCell,
    #[error("a cell's bytes do not match the manifest's digest")]
    DigestMismatch,
}

/// A read-only handle on the packed store file (PD-2).
#[derive(Debug)]
pub struct Store {
    file: File,
    index: StoreIndex,
}

impl Store {
    pub fn open(path: impl AsRef<Path>, cells: &[CellEntry]) -> Result<Store, StoreError> {
        let file = File::open(path)?;
        Ok(Store {
            file,
            index: StoreIndex::from_cells(cells),
        })
    }

    /// The exact bytes of one cell, or `None` if the cell holds no entry
    /// (covered but empty).
    pub fn read_cell(&self, id: CellId) -> Result<Option<Vec<u8>>, StoreError> {
        let Some((offset, length)) = self.index.find(id) else {
            return Ok(None);
        };
        let mut buf = vec![0u8; length as usize];
        self.file
            .read_exact_at(&mut buf, offset)
            .map_err(|_| StoreError::MissingCell)?;
        Ok(Some(buf))
    }

    /// Verify every listed cell's bytes against its manifest digest
    /// (NFR3.2.4). Returns the ids of every cell that failed to read or
    /// whose digest did not match; an empty result is a good store.
    pub fn verify_all(&self, cells: &[CellEntry]) -> Vec<CellId> {
        let mut failing = Vec::new();
        for cell in cells {
            let mut buf = vec![0u8; cell.length as usize];
            let ok = self.file.read_exact_at(&mut buf, cell.offset).is_ok()
                && Digest::from_bytes(*blake3::hash(&buf).as_bytes()) == cell.digest;
            if !ok {
                failing.push(cell.cell_id);
            }
        }
        failing
    }
}

/// A sequential writer: cells are appended in the order they are written
/// (the build pipeline writes them in cell-id order, PD-7), and each write
/// returns exactly what the manifest needs to record for that cell.
pub struct StoreWriter {
    file: File,
    offset: u64,
}

impl StoreWriter {
    pub fn create(path: impl AsRef<Path>) -> io::Result<StoreWriter> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        Ok(StoreWriter { file, offset: 0 })
    }

    /// Append one cell's bytes; returns `(offset, length, digest)` for the
    /// manifest's `CellEntry`.
    pub fn write_cell(&mut self, bytes: &[u8]) -> io::Result<(u64, u64, Digest)> {
        use std::io::Write;
        self.file.write_all(bytes)?;
        let offset = self.offset;
        let length = bytes.len() as u64;
        self.offset += length;
        let digest = Digest::from_bytes(*blake3::hash(bytes).as_bytes());
        Ok((offset, length, digest))
    }

    pub fn flush(&mut self) -> io::Result<()> {
        use std::io::Write;
        self.file.flush()?;
        self.file.sync_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::CellId;

    fn entry(cell_id: CellId, offset: u64, length: u64, byte: u8) -> CellEntry {
        CellEntry {
            cell_id,
            offset,
            length,
            digest: Digest::from_bytes([byte; 32]),
            way_count: 0,
            node_count: 0,
        }
    }

    fn write_sample(path: &Path) -> Vec<CellEntry> {
        let mut writer = StoreWriter::create(path).unwrap();
        let payloads: [(CellId, &[u8]); 3] = [
            (CellId::new(0, 0), b"aaaa"),
            (CellId::new(1, 0), b"bbbbbbbb"),
            (CellId::new(2, 0), b"cc"),
        ];
        let mut cells = Vec::new();
        for (id, bytes) in payloads {
            let (offset, length, digest) = writer.write_cell(bytes).unwrap();
            cells.push(CellEntry {
                cell_id: id,
                offset,
                length,
                digest,
                way_count: 1,
                node_count: 1,
            });
        }
        writer.flush().unwrap();
        cells
    }

    // PD-2/PD-7 — the writer packs cells back to back in cell-id order and
    // the reader's index finds them; a positional read returns exactly the
    // cell bytes.
    #[test]
    fn writer_packs_cells_and_reader_index_finds_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.bin");
        let cells = write_sample(&path);
        assert_eq!(cells[0].offset, 0);
        assert_eq!(cells[1].offset, 4);
        assert_eq!(cells[2].offset, 12);

        let store = Store::open(&path, &cells).unwrap();
        assert_eq!(
            store.read_cell(CellId::new(0, 0)).unwrap(),
            Some(b"aaaa".to_vec())
        );
        assert_eq!(
            store.read_cell(CellId::new(1, 0)).unwrap(),
            Some(b"bbbbbbbb".to_vec())
        );
        assert_eq!(
            store.read_cell(CellId::new(2, 0)).unwrap(),
            Some(b"cc".to_vec())
        );
    }

    // NFR3.1.10 — an absent entry is "covered but empty", not an error.
    #[test]
    fn an_absent_entry_is_covered_but_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.bin");
        let cells = write_sample(&path);
        let store = Store::open(&path, &cells).unwrap();
        assert_eq!(store.read_cell(CellId::new(99, 99)).unwrap(), None);
    }

    // NFR3.2.4 — verify_all passes on a good store.
    #[test]
    fn verify_all_passes_on_a_good_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.bin");
        let cells = write_sample(&path);
        let store = Store::open(&path, &cells).unwrap();
        assert!(store.verify_all(&cells).is_empty());
    }

    // NFR3.2.4 — verify_all fails on one flipped byte.
    #[test]
    fn verify_all_fails_on_one_flipped_byte() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.bin");
        let cells = write_sample(&path);
        {
            let file = OpenOptions::new().write(true).open(&path).unwrap();
            file.write_at(b"X", 0).unwrap();
        }
        let store = Store::open(&path, &cells).unwrap();
        let failing = store.verify_all(&cells);
        assert_eq!(failing, vec![cells[0].cell_id]);
    }

    // NFR3.2.4 — verify_all fails on a missing cell (an entry the manifest
    // lists but the store has no bytes for at all — simulated here by an
    // entry pointing past the end of an otherwise-good file).
    #[test]
    fn verify_all_fails_on_a_missing_cell() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.bin");
        let mut cells = write_sample(&path);
        cells.push(entry(CellId::new(9, 9), 1_000, 4, 1));
        let store = Store::open(&path, &cells).unwrap();
        let failing = store.verify_all(&cells);
        assert_eq!(failing, vec![CellId::new(9, 9)]);
    }

    // NFR3.2.4 — verify_all fails on a truncated file.
    #[test]
    fn verify_all_fails_on_a_truncated_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.bin");
        let cells = write_sample(&path);
        {
            let file = OpenOptions::new().write(true).open(&path).unwrap();
            file.set_len(6).unwrap();
        }
        let store = Store::open(&path, &cells).unwrap();
        let failing = store.verify_all(&cells);
        assert!(failing.contains(&cells[1].cell_id));
        assert!(failing.contains(&cells[2].cell_id));
        assert!(!failing.contains(&cells[0].cell_id));
    }
}
