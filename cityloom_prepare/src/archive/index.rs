//! The binary index of a `.cityloom` archive (its first member, `cityloom.idx`).
use std::collections::HashSet;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

use super::ArchiveError;
use crate::metro_index::Grid;

pub const INDEX_NAME: &str = "cityloom.idx";
pub const INDEX_LIMIT: u64 = 1 << 20;
const MAGIC: &[u8; 8] = b"CLOOMIDX";
const VERSION: u16 = 1;
const TRAILER: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Road,
    Basemap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub area: String,
    pub created: i64,
    pub source: String,
    pub source_time: i64,
    pub grid: Grid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    pub x: u16,
    pub y: u16,
    pub size: u64,
    pub sha256: [u8; 32],
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub header: Header,
    pub entries: Vec<Entry>,
    pub trailer: [u8; 32],
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum IndexError {
    #[error("the index is too short")]
    Short,
    #[error("the index does not start with the magic CLOOMIDX")]
    Magic,
    #[error("the index is version {0}; this server reads version 1")]
    Version(u16),
    #[error("the index has flags {0:#06x} that this server does not know")]
    Flags(u16),
    #[error("the index's checksum is wrong")]
    Trailer,
    #[error("the index holds text that is not UTF-8")]
    Utf8,
    #[error("an entry has the kind {0}, which is not known")]
    UnknownKind(u8),
    #[error("the index has bytes after its last entry")]
    Trailing,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], IndexError> {
        let end = self.at.checked_add(n).filter(|end| *end <= self.bytes.len()).ok_or(IndexError::Short)?;
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], IndexError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }
    fn u16(&mut self) -> Result<u16, IndexError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32, IndexError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64, IndexError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn i64(&mut self) -> Result<i64, IndexError> {
        Ok(i64::from_le_bytes(self.array()?))
    }
    fn f64(&mut self) -> Result<f64, IndexError> {
        Ok(f64::from_le_bytes(self.array()?))
    }
    fn string(&mut self) -> Result<String, IndexError> {
        let n = self.u16()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| IndexError::Utf8)
    }
}

/// Reads and verifies an index: the magic, the version, the flags, the trailer over everything before it, then the body.
pub fn read_index(bytes: &[u8]) -> Result<Index, IndexError> {
    let mut head = Cursor { bytes, at: 0 };
    if head.take(MAGIC.len())? != MAGIC {
        return Err(IndexError::Magic);
    }
    let version = head.u16()?;
    if version != VERSION {
        return Err(IndexError::Version(version));
    }
    let flags = head.u16()?;
    if flags != 0 {
        return Err(IndexError::Flags(flags));
    }
    if bytes.len() < head.at + TRAILER {
        return Err(IndexError::Short);
    }
    let (body, trailer) = bytes.split_at(bytes.len() - TRAILER);
    if Sha256::digest(body).as_slice() != trailer {
        return Err(IndexError::Trailer);
    }
    let mut c = Cursor { bytes: body, at: head.at };
    let area = c.string()?;
    let created = c.i64()?;
    let source = c.string()?;
    let source_time = c.i64()?;
    let (lon0, lat0, dlon, dlat) = (c.f64()?, c.f64()?, c.f64()?, c.f64()?);
    let (nx, ny, count) = (c.u16()?, c.u16()?, c.u32()?);
    // A count is trusted only as far as the bytes go: an entry takes at least 47.
    let mut entries = Vec::with_capacity((count as usize).min(body.len() / 47));
    for _ in 0..count {
        let kind = match c.take(1)?[0] {
            1 => Kind::Road,
            2 => Kind::Basemap,
            other => return Err(IndexError::UnknownKind(other)),
        };
        let (x, y, size) = (c.u16()?, c.u16()?, c.u64()?);
        let sha256 = c.array()?;
        entries.push(Entry { kind, x, y, size, sha256, path: c.string()? });
    }
    if c.at != body.len() {
        return Err(IndexError::Trailing);
    }
    let mut trailer_copy = [0u8; 32];
    trailer_copy.copy_from_slice(trailer);
    Ok(Index { header: Header { area, created, source, source_time, grid: Grid { lon0, lat0, dlon, dlat, nx, ny } }, entries, trailer: trailer_copy })
}

/// The bytes of an index (the writer `pack` uses).
pub fn write_index(header: &Header, entries: &[Entry]) -> Vec<u8> {
    fn string(out: &mut Vec<u8>, text: &str) {
        out.extend((text.len() as u16).to_le_bytes());
        out.extend(text.as_bytes());
    }
    let g = &header.grid;
    let mut out = MAGIC.to_vec();
    out.extend(VERSION.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    string(&mut out, &header.area);
    out.extend(header.created.to_le_bytes());
    string(&mut out, &header.source);
    out.extend(header.source_time.to_le_bytes());
    for v in [g.lon0, g.lat0, g.dlon, g.dlat] {
        out.extend(v.to_le_bytes());
    }
    out.extend(g.nx.to_le_bytes());
    out.extend(g.ny.to_le_bytes());
    out.extend((entries.len() as u32).to_le_bytes());
    for e in entries {
        out.push(match e.kind {
            Kind::Road => 1,
            Kind::Basemap => 2,
        });
        out.extend(e.x.to_le_bytes());
        out.extend(e.y.to_le_bytes());
        out.extend(e.size.to_le_bytes());
        out.extend(e.sha256);
        string(&mut out, &e.path);
    }
    let digest = Sha256::digest(&out);
    out.extend(digest);
    out
}

fn lower_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub(crate) fn basemap_name(path: &str) -> Option<&str> {
    path.strip_prefix("basemap/").and_then(|rest| rest.strip_suffix(".pmtiles")).filter(|name| lower_name(name))
}

pub(crate) fn member(path: &str, problem: &'static str) -> ArchiveError {
    ArchiveError::Member { path: path.to_string(), problem }
}

/// The area is a plain name (it becomes a directory of `DATA_ROOT`) and every entry has the path its kind, `x` and `y`
/// call for, inside the grid, once. Called before any byte of the archive is trusted or written.
pub fn check_entries(index: &Index) -> Result<(), ArchiveError> {
    let area = &index.header.area;
    if area.len() > 64 || !lower_name(area) {
        return Err(ArchiveError::Area(area.clone()));
    }
    let grid = &index.header.grid;
    let mut seen = HashSet::new();
    for entry in &index.entries {
        let path = entry.path.as_str();
        match entry.kind {
            Kind::Road => {
                if path != format!("roads/{}_{}.osm.pbf", entry.x, entry.y) {
                    return Err(member(path, "has a path that is not roads/<x>_<y>.osm.pbf for its x and y"));
                }
                if entry.x >= grid.nx || entry.y >= grid.ny {
                    return Err(member(path, "is outside the grid"));
                }
            }
            Kind::Basemap => {
                if basemap_name(path).is_none() {
                    return Err(member(path, "has a path that is not basemap/<name>.pmtiles"));
                }
                if entry.x != 0 || entry.y != 0 {
                    return Err(member(path, "is a basemap with a place in the grid"));
                }
            }
        }
        if !seen.insert(path) {
            return Err(member(path, "is listed twice"));
        }
    }
    Ok(())
}

/// Where an entry is unpacked, under `DATA_ROOT/<area>/`: the tiles in `metro/` (the app's name for them), the basemap in `basemap/`.
pub fn destination(entry: &Entry) -> PathBuf {
    match entry.kind {
        Kind::Road => PathBuf::from(format!("metro/{}_{}.osm.pbf", entry.x, entry.y)),
        Kind::Basemap => PathBuf::from(format!("basemap/{}.pmtiles", basemap_name(&entry.path).unwrap_or("invalid"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn s(out: &mut Vec<u8>, text: &str) {
        out.extend((text.len() as u16).to_le_bytes());
        out.extend(text.as_bytes());
    }

    fn resign(mut b: Vec<u8>) -> Vec<u8> {
        let digest = Sha256::digest(&b);
        b.extend(digest);
        b
    }

    /// The same bytes with the trailer made right again for what comes before it.
    fn re_sign(bytes: &[u8]) -> Vec<u8> {
        resign(bytes[..bytes.len() - 32].to_vec())
    }

    /// The spec's layout for `area`, written by hand: two road tiles and a basemap. `kind0` is the first entry's
    /// kind byte and `count` the number the header claims.
    fn hand_made_with(area: &str, kind0: u8, count: u32) -> Vec<u8> {
        let mut b = b"CLOOMIDX".to_vec();
        b.extend(1u16.to_le_bytes());
        b.extend(0u16.to_le_bytes());
        s(&mut b, area);
        b.extend(1_760_000_000i64.to_le_bytes());
        s(&mut b, "quebec-latest.osm.pbf");
        b.extend(1_759_900_000i64.to_le_bytes());
        for v in [-74.4f64, 45.2, 0.0257, 0.018] {
            b.extend(v.to_le_bytes());
        }
        b.extend(51u16.to_le_bytes());
        b.extend(34u16.to_le_bytes());
        b.extend(count.to_le_bytes());
        for (kind, x, y, size, fill, path) in [
            (kind0, 0u16, 0u16, 10u64, 0xAAu8, "roads/0_0.osm.pbf"),
            (1, 12, 9, 20, 0xBB, "roads/12_9.osm.pbf"),
            (2, 0, 0, 30, 0xCC, "basemap/montreal.pmtiles"),
        ] {
            b.push(kind);
            b.extend(x.to_le_bytes());
            b.extend(y.to_le_bytes());
            b.extend(size.to_le_bytes());
            b.extend([fill; 32]);
            s(&mut b, path);
        }
        resign(b)
    }

    fn hand_made(area: &str) -> Vec<u8> {
        hand_made_with(area, 1, 3)
    }

    #[test]
    fn reads_every_field_of_the_hand_made_index() {
        let bytes = hand_made("montreal");
        let index = read_index(&bytes).unwrap();
        let h = &index.header;
        assert_eq!((h.area.as_str(), h.created, h.source.as_str(), h.source_time), ("montreal", 1_760_000_000, "quebec-latest.osm.pbf", 1_759_900_000));
        assert_eq!((h.grid.lon0, h.grid.lat0, h.grid.dlon, h.grid.dlat, h.grid.nx, h.grid.ny), (-74.4, 45.2, 0.0257, 0.018, 51, 34));
        assert_eq!(index.entries.len(), 3);
        let e = &index.entries[1];
        assert_eq!((e.kind, e.x, e.y, e.size, e.sha256, e.path.as_str()), (Kind::Road, 12, 9, 20, [0xBB; 32], "roads/12_9.osm.pbf"));
        assert_eq!(index.entries[2].kind, Kind::Basemap);
        assert_eq!(index.trailer[..], bytes[bytes.len() - 32..]);
    }

    #[test]
    fn write_index_makes_the_hand_made_bytes() {
        let index = read_index(&hand_made("montreal")).unwrap();
        assert_eq!(write_index(&index.header, &index.entries), hand_made("montreal"));
    }

    #[test]
    fn refuses_a_wrong_magic_an_unknown_version_and_an_unknown_flag() {
        let mut b = hand_made("montreal");
        b[0] = b'X';
        assert_eq!(read_index(&b), Err(IndexError::Magic));
        let mut b = hand_made("montreal");
        b[8..10].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(read_index(&re_sign(&b)), Err(IndexError::Version(2)));
        let mut b = hand_made("montreal");
        b[10..12].copy_from_slice(&4u16.to_le_bytes());
        assert_eq!(read_index(&re_sign(&b)), Err(IndexError::Flags(4)));
    }

    #[test]
    fn refuses_short_input_and_every_flipped_byte() {
        let b = hand_made("montreal");
        for n in [0, 5, 11, 20] {
            assert_eq!(read_index(&b[..n]), Err(IndexError::Short), "{n} bytes");
        }
        assert_eq!(read_index(&b[..b.len() - 1]), Err(IndexError::Trailer));
        for i in 0..b.len() {
            let mut bad = b.clone();
            bad[i] ^= 1;
            assert!(read_index(&bad).is_err(), "byte {i}");
        }
    }

    #[test]
    fn refuses_a_signed_body_that_is_cut_short_goes_on_or_is_not_utf8() {
        let b = hand_made("montreal");
        assert_eq!(read_index(&resign(b[..b.len() - 35].to_vec())), Err(IndexError::Short));
        let mut more = b[..b.len() - 32].to_vec();
        more.push(b'x');
        assert_eq!(read_index(&resign(more)), Err(IndexError::Trailing));
        let mut bad = b.clone();
        bad[14] = 0xFF; // inside the area name
        assert_eq!(read_index(&re_sign(&bad)), Err(IndexError::Utf8));
    }

    #[test]
    fn refuses_an_unknown_kind_and_a_count_the_bytes_cannot_hold() {
        assert_eq!(read_index(&hand_made_with("montreal", 9, 3)), Err(IndexError::UnknownKind(9)));
        // a hostile count is trusted only as far as the bytes go: no allocation for four billion entries
        assert_eq!(read_index(&hand_made_with("montreal", 1, u32::MAX)), Err(IndexError::Short));
    }

    fn entry(kind: Kind, x: u16, y: u16, path: &str) -> Entry {
        Entry { kind, x, y, size: 1, sha256: [0; 32], path: path.into() }
    }

    fn index_with(area: &str, entries: Vec<Entry>) -> Index {
        let mut index = read_index(&hand_made("montreal")).unwrap();
        index.header.area = area.into();
        index.entries = entries;
        index
    }

    #[test]
    fn accepts_the_hand_made_entries() {
        check_entries(&read_index(&hand_made("montreal")).unwrap()).unwrap();
    }

    #[test]
    fn refuses_an_area_that_is_a_path_or_not_lower_case() {
        for area in ["../x", "a/b", "", "Montreal", ".", "..", "a b", &"a".repeat(65)] {
            assert!(matches!(check_entries(&index_with(area, vec![])), Err(ArchiveError::Area(_))), "{area:?}");
        }
    }

    #[test]
    fn refuses_paths_outside_the_two_patterns_and_disagreeing_entries() {
        let road = |x, y, path: &str| index_with("montreal", vec![entry(Kind::Road, x, y, path)]);
        for (x, y, path) in [
            (0, 0, "roads/../0_0.osm.pbf"),
            (0, 0, "../roads/0_0.osm.pbf"),
            (0, 0, "/roads/0_0.osm.pbf"),
            (0, 0, "roads/0_0.osm.pbf/"),
            (0, 0, "roads/00_0.osm.pbf"),
            (0, 0, "roads/0_0.osm.pbf.x"),
            (0, 0, "roads/a_0.osm.pbf"),
            (0, 0, "other/0_0.osm.pbf"),
            (1, 0, "roads/0_0.osm.pbf"),
            (51, 0, "roads/51_0.osm.pbf"),
            (0, 34, "roads/0_34.osm.pbf"),
            (0, 0, "roads/0_0.osm.pbf\0"),
        ] {
            assert!(matches!(check_entries(&road(x, y, path)), Err(ArchiveError::Member { .. })), "{path:?}");
        }
        for path in
            ["basemap/Montreal.pmtiles", "basemap/a/b.pmtiles", "basemap/.pmtiles", "basemap/a.pmtiles.x", "basemap/../a.pmtiles", "roads/montreal.pmtiles"]
        {
            let index = index_with("montreal", vec![entry(Kind::Basemap, 0, 0, path)]);
            assert!(matches!(check_entries(&index), Err(ArchiveError::Member { .. })), "{path:?}");
        }
        let placed = index_with("montreal", vec![entry(Kind::Basemap, 1, 0, "basemap/m.pmtiles")]);
        assert!(check_entries(&placed).unwrap_err().to_string().contains("basemap/m.pmtiles"));
    }

    #[test]
    fn refuses_a_path_listed_twice() {
        let twice = index_with("montreal", vec![entry(Kind::Road, 0, 0, "roads/0_0.osm.pbf"), entry(Kind::Road, 0, 0, "roads/0_0.osm.pbf")]);
        assert!(check_entries(&twice).unwrap_err().to_string().contains("twice"));
    }

    #[test]
    fn says_where_each_entry_is_unpacked() {
        assert_eq!(destination(&entry(Kind::Road, 12, 9, "roads/12_9.osm.pbf")), std::path::PathBuf::from("metro/12_9.osm.pbf"));
        assert_eq!(destination(&entry(Kind::Basemap, 0, 0, "basemap/montreal.pmtiles")), std::path::PathBuf::from("basemap/montreal.pmtiles"));
    }
}
