//! Writes a `.cityloom` archive: a deterministic ustar tar (mtime 0, owner 0, mode 0644) whose first member is the index.
use std::fs::{self, File};
use std::io::{self, BufWriter, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::index::{INDEX_NAME, basemap_name, write_index};
use super::{Entry, Header, Kind};
use crate::metro_index::Grid;

pub struct PackInputs<'a> {
    pub area: &'a str,
    pub source: &'a str,
    pub source_time: i64,
    pub created: i64,
    pub grid: Grid,
    pub roads: &'a [(u16, u16, PathBuf)],
    pub basemaps: &'a [PathBuf],
}

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("the area name {0:?} is not allowed (lower-case letters, digits and hyphens, at most 64)")]
    Area(String),
    #[error("{0:?} is not a basemap name (lower-case letters, digits and hyphens, then .pmtiles)")]
    Name(String),
    #[error("road tile {0} is outside the grid")]
    OutsideGrid(String),
    #[error("{0}")]
    Empty(&'static str),
    #[error("{what}: {source}")]
    Io { what: String, source: io::Error },
}

struct Piece {
    entry: Entry,
    file: PathBuf,
}

fn io_err(what: String) -> impl FnOnce(io::Error) -> PackError {
    move |source| PackError::Io { what, source }
}

/// The bytes done so far of the two passes, and who is told.
struct Tally<'a> {
    done: u64,
    total: u64,
    progress: &'a mut dyn FnMut(u64, u64),
}

impl Tally<'_> {
    fn add(&mut self, n: u64) {
        self.done += n;
        (self.progress)(self.done, self.total);
    }
}

/// Hashes and counts what is read through it, and counts it in the progress.
struct Reading<'a, 'b> {
    file: File,
    hasher: Sha256,
    size: u64,
    tally: &'a mut Tally<'b>,
}

impl Read for Reading<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.file.read(buf)?;
        self.hasher.update(&buf[..n]);
        self.size += n as u64;
        self.tally.add(n as u64);
        Ok(n)
    }
}

fn open(path: &Path) -> Result<File, PackError> {
    File::open(path).map_err(io_err(format!("cannot read {}", path.display())))
}

/// Reads `path` to its end through a `Reading`; returns its SHA-256 and size.
fn hash_file(path: &Path, tally: &mut Tally) -> Result<([u8; 32], u64), PackError> {
    let mut reading = Reading { file: open(path)?, hasher: Sha256::new(), size: 0, tally };
    io::copy(&mut reading, &mut io::sink()).map_err(io_err(format!("cannot read {}", path.display())))?;
    Ok((reading.hasher.finalize().into(), reading.size))
}

fn header(path: &str, size: u64) -> io::Result<tar::Header> {
    let mut h = tar::Header::new_ustar();
    h.set_path(path)?;
    h.set_size(size);
    h.set_mtime(0);
    h.set_uid(0);
    h.set_gid(0);
    h.set_mode(0o644);
    h.set_entry_type(tar::EntryType::Regular);
    h.set_cksum();
    Ok(h)
}

fn changed(path: &Path) -> PackError {
    PackError::Io {
        what: format!("{} changed while it was being packed", path.display()),
        source: io::Error::new(io::ErrorKind::InvalidData, "the file's size or content is not what was measured"),
    }
}

fn is_area(area: &str) -> bool {
    !area.is_empty() && area.len() <= 64 && area.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Packs `inputs` into `out` (through `out.part`, renamed when whole); returns how many members besides the index.
/// `progress(done, total)` counts the bytes read to hash and then to write.
pub fn pack(inputs: &PackInputs, out: &Path, progress: &mut dyn FnMut(u64, u64)) -> Result<usize, PackError> {
    if !is_area(inputs.area) {
        return Err(PackError::Area(inputs.area.to_string()));
    }
    let blank = |kind, x, y, path: String| Entry { kind, x, y, size: 0, sha256: [0; 32], path };
    let mut roads = Vec::new();
    for (x, y, file) in inputs.roads {
        if *x >= inputs.grid.nx || *y >= inputs.grid.ny {
            return Err(PackError::OutsideGrid(format!("{x}_{y}")));
        }
        roads.push(Piece { entry: blank(Kind::Road, *x, *y, format!("roads/{x}_{y}.osm.pbf")), file: file.clone() });
    }
    if roads.is_empty() {
        return Err(PackError::Empty("no road tiles"));
    }
    let mut maps = Vec::new();
    for file in inputs.basemaps {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let path = format!("basemap/{name}");
        if basemap_name(&path).is_none() {
            return Err(PackError::Name(name.to_string()));
        }
        maps.push(Piece { entry: blank(Kind::Basemap, 0, 0, path), file: file.clone() });
    }
    if maps.is_empty() {
        return Err(PackError::Empty("no basemap"));
    }
    roads.sort_by(|a, b| a.entry.path.cmp(&b.entry.path));
    maps.sort_by(|a, b| a.entry.path.cmp(&b.entry.path));
    let mut pieces = roads;
    pieces.extend(maps);

    let mut total = 0;
    for piece in &pieces {
        total += fs::metadata(&piece.file).map_err(io_err(format!("cannot read {}", piece.file.display())))?.len();
    }
    let mut tally = Tally { done: 0, total: total * 2, progress };
    for piece in &mut pieces {
        let (sha, size) = hash_file(&piece.file, &mut tally)?;
        piece.entry.sha256 = sha;
        piece.entry.size = size;
    }
    let head = Header {
        area: inputs.area.to_string(),
        created: inputs.created,
        source: inputs.source.to_string(),
        source_time: inputs.source_time,
        grid: inputs.grid,
    };
    let entries: Vec<Entry> = pieces.iter().map(|p| p.entry.clone()).collect();
    let index = write_index(&head, &entries);

    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(io_err(format!("cannot create {}", parent.display())))?;
    }
    let mut part_name = out.file_name().unwrap_or_default().to_os_string();
    part_name.push(".part");
    let part = out.with_file_name(part_name);
    let written = write_tar(&part, &index, &pieces, &mut tally)
        .and_then(|()| fs::rename(&part, out).map_err(io_err(format!("cannot move {} to {}", part.display(), out.display()))));
    if written.is_err() {
        let _ = fs::remove_file(&part);
    }
    written.map(|()| pieces.len())
}

fn write_tar(part: &Path, index: &[u8], pieces: &[Piece], tally: &mut Tally) -> Result<(), PackError> {
    let writing = || format!("cannot write {}", part.display());
    let file = File::create(part).map_err(io_err(writing()))?;
    let mut builder = tar::Builder::new(BufWriter::new(file));
    let head = header(INDEX_NAME, index.len() as u64).map_err(io_err("cannot build the index header".into()))?;
    builder.append(&head, index).map_err(io_err(writing()))?;
    for piece in pieces {
        let head = header(&piece.entry.path, piece.entry.size).map_err(io_err(format!("cannot build the header of {}", piece.entry.path)))?;
        // `Builder::append` does not compare what it copies with the size in the header, so the count and the hash are checked here.
        let mut reading = Reading { file: open(&piece.file)?, hasher: Sha256::new(), size: 0, tally };
        builder.append(&head, &mut reading).map_err(io_err(writing()))?;
        let (size, sha): (u64, [u8; 32]) = (reading.size, reading.hasher.finalize().into());
        if size != piece.entry.size || sha != piece.entry.sha256 {
            return Err(changed(&piece.file));
        }
    }
    let mut inner = builder.into_inner().map_err(io_err(writing()))?;
    io::Write::flush(&mut inner).map_err(io_err(writing()))
}
