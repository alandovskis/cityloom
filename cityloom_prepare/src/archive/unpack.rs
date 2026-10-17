//! Verifies a `.cityloom` archive whole and unpacks it: the index first, then every member hashed as it is written
//! into a staging directory, renamed into place only when everything checks out. An archive is untrusted input:
//! no path of a member is ever used, only the one the verified index gives for it. `tar` ignores bytes after its
//! end-of-archive marker, so a file with trailing junk still unpacks (every byte served was verified).
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use super::index::{INDEX_LIMIT, INDEX_NAME, check_entries, destination, member, read_index};
use super::{ArchiveError, Entry, Index, Kind};
use crate::metro_index::{hex, index_json, short_checksum};

const MARKER: &str = ".index-sha256";

#[derive(Debug, Clone, PartialEq)]
pub struct Unpacked {
    pub area: String,
    pub dir: PathBuf,
    pub reused: bool,
    pub tiles: usize,
}

fn io_err(what: impl Into<String>) -> impl FnOnce(io::Error) -> ArchiveError {
    let what = what.into();
    move |source| ArchiveError::Io { what, source }
}

/// A directory made for one unpack, removed on drop unless it was renamed into place.
struct Staging {
    dir: PathBuf,
    armed: bool,
}

impl Drop for Staging {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

/// A name no other start or thread has: the process, the time and a counter.
fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{prefix}{}-{nanos}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Earlier starts that ended before they renamed or removed their directories.
fn remove_leftovers(data_root: &Path) {
    let Ok(entries) = fs::read_dir(data_root) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if (name.starts_with(".unpack-") || name.starts_with(".old-")) && entry.path().is_dir() {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

fn first_member<R: Read>(members: &mut tar::Entries<'_, R>) -> Result<Index, ArchiveError> {
    let mut first = members.next().ok_or(ArchiveError::IndexNotFirst)?.map_err(io_err("cannot read the archive's first member"))?;
    let named = first.path().ok().and_then(|p| p.to_str().map(str::to_owned));
    if named.as_deref() != Some(INDEX_NAME) || first.header().entry_type() != tar::EntryType::Regular {
        return Err(ArchiveError::IndexNotFirst);
    }
    let mut bytes = Vec::new();
    (&mut first).take(INDEX_LIMIT + 1).read_to_end(&mut bytes).map_err(io_err("cannot read the index"))?;
    if bytes.len() as u64 > INDEX_LIMIT {
        return Err(ArchiveError::IndexTooBig);
    }
    let index = read_index(&bytes)?;
    check_entries(&index)?;
    Ok(index)
}

/// The tile index the app reads, from the archive's index.
fn metro_index(index: &Index) -> String {
    let tiles: Vec<(String, String)> =
        index.entries.iter().filter(|e| e.kind == Kind::Road).map(|e| (format!("{}_{}", e.x, e.y), short_checksum(&e.sha256))).collect();
    index_json(&index.header.grid, &tiles)
}

/// Whether `dir` holds exactly this archive's unpacking: the marker, the tile index and every file at its size.
/// (The bytes are not hashed again: they were verified when they were written.)
fn is_current(dir: &Path, index: &Index) -> bool {
    fs::read_to_string(dir.join(MARKER)).is_ok_and(|marker| marker == hex(&index.trailer))
        && dir.join("metro/index.json").is_file()
        && index.entries.iter().all(|e| fs::metadata(dir.join(destination(e))).is_ok_and(|m| m.is_file() && m.len() == e.size))
}

fn copy_verified<R: Read>(from: &mut R, dest: &Path, entry: &Entry) -> Result<(), ArchiveError> {
    let file = File::options().write(true).create_new(true).open(dest).map_err(io_err(format!("cannot write {}", dest.display())))?;
    let mut out = BufWriter::new(file);
    let (mut hasher, mut total, mut buffer) = (Sha256::new(), 0u64, vec![0u8; 64 * 1024]);
    loop {
        let n = from.read(&mut buffer).map_err(io_err(format!("cannot read member {:?}", entry.path)))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
        out.write_all(&buffer[..n]).map_err(io_err(format!("cannot write {}", dest.display())))?;
        total += n as u64;
    }
    out.flush().map_err(io_err(format!("cannot write {}", dest.display())))?;
    if total != entry.size {
        return Err(member(&entry.path, "has another size than the index says"));
    }
    if hasher.finalize().as_slice() != entry.sha256 {
        return Err(member(&entry.path, "does not match its checksum in the index"));
    }
    Ok(())
}

/// Unpacks `archive` into `data_root/<area>`. Nothing reaches that directory unless the whole archive verified.
///
/// A restart trusts the files it unpacked and verified earlier: an archive with the same index trailer as the
/// marker, whose unpacked files are all present at their sizes, is reused without hashing again (nothing
/// unverified is served: every byte there was checked when it was written).
pub fn unpack(archive: &Path, data_root: &Path) -> Result<Unpacked, ArchiveError> {
    let file = File::open(archive).map_err(io_err(format!("cannot open {}", archive.display())))?;
    let mut tar = tar::Archive::new(BufReader::new(file));
    let mut members = tar.entries().map_err(io_err("cannot read the archive"))?;
    let index = first_member(&mut members)?;
    let area = index.header.area.clone();
    let tiles = index.entries.iter().filter(|e| e.kind == Kind::Road).count();
    fs::create_dir_all(data_root).map_err(io_err(format!("cannot create {}", data_root.display())))?;
    remove_leftovers(data_root);
    let target = data_root.join(&area);
    if is_current(&target, &index) {
        return Ok(Unpacked { area, dir: target, reused: true, tiles });
    }
    let mut staging = Staging { dir: data_root.join(unique(".unpack-")), armed: true };
    for sub in ["metro", "basemap"] {
        fs::create_dir_all(staging.dir.join(sub)).map_err(io_err(format!("cannot create a directory in {}", data_root.display())))?;
    }
    let mut seen = vec![false; index.entries.len()];
    for member_result in members {
        let mut in_tar = member_result.map_err(io_err("cannot read a member of the archive"))?;
        let path = in_tar.path().ok().and_then(|p| p.to_str().map(str::to_owned)).ok_or_else(|| member("?", "has a path that cannot be read"))?;
        if in_tar.header().entry_type() != tar::EntryType::Regular {
            return Err(member(&path, "is not a regular file"));
        }
        let at = index.entries.iter().position(|e| e.path == path).ok_or_else(|| member(&path, "is not in the index"))?;
        if std::mem::replace(&mut seen[at], true) {
            return Err(member(&path, "appears twice in the archive"));
        }
        let entry = &index.entries[at];
        if in_tar.size() != entry.size {
            return Err(member(&path, "has another size than the index says"));
        }
        copy_verified(&mut in_tar, &staging.dir.join(destination(entry)), entry)?;
    }
    if let Some(at) = seen.iter().position(|seen| !seen) {
        return Err(ArchiveError::Missing { path: index.entries[at].path.clone() });
    }
    fs::write(staging.dir.join("metro/index.json"), metro_index(&index)).map_err(io_err("cannot write metro/index.json"))?;
    fs::write(staging.dir.join(MARKER), hex(&index.trailer)).map_err(io_err(format!("cannot write {MARKER}")))?;
    install(&staging.dir, &target, data_root)?;
    staging.armed = false;
    Ok(Unpacked { area, dir: target, reused: false, tiles })
}

fn install(staged: &Path, target: &Path, data_root: &Path) -> Result<(), ArchiveError> {
    let aside = data_root.join(unique(".old-"));
    let had = target.exists();
    if had {
        fs::rename(target, &aside).map_err(io_err(format!("cannot move {} aside", target.display())))?;
    }
    if let Err(source) = fs::rename(staged, target) {
        if had {
            let _ = fs::rename(&aside, target);
        }
        return Err(ArchiveError::Io { what: format!("cannot move the unpacked area to {}", target.display()), source });
    }
    if had {
        let _ = fs::remove_dir_all(&aside);
    }
    Ok(())
}

/// Unpacks the one `.cityloom` file of `folder`.
pub fn unpack_folder(folder: &Path, data_root: &Path) -> Result<Unpacked, ArchiveError> {
    let mut found = Vec::new();
    for entry in fs::read_dir(folder).map_err(io_err(format!("cannot read {}", folder.display())))? {
        let path = entry.map_err(io_err(format!("cannot read {}", folder.display())))?.path();
        if path.extension().is_some_and(|e| e == "cityloom") && path.is_file() {
            found.push(path);
        }
    }
    match found.as_slice() {
        [] => Err(ArchiveError::NoArchive(folder.to_path_buf())),
        [one] => unpack(one, data_root),
        _ => Err(ArchiveError::TooMany(folder.to_path_buf())),
    }
}
